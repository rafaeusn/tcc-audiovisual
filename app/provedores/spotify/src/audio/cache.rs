//! O áudio decifrado das faixas do Spotify, só na memória: o tocador e a pré-análise do Player pedem pedaços ao servidor
//! local ([`super::servidor`]), e é daqui que eles saem.
//!
//! **A regra 2 do `AGENTS.md` mora aqui.** Nada desta estrutura vai para arquivo: os pedaços vivem num `Arc<Vec<u8>>` que
//! morre com o provedor, e o teto de memória impede que uma fila longa cresça sem fim. Não existe caminho de código que
//! grave o áudio — não é uma promessa, é o tipo.
//!
//! Duas coisas ficam guardadas por faixa: o tamanho do arquivo e os pedaços já decifrados. O pedaço que falta é baixado do
//! Spotify na hora do pedido ([`Baixador`]) e guardado aqui, para o tocador (que lê do começo ao fim) e a pré-análise
//! (que lê as mesmas faixas) não baixarem duas vezes a mesma coisa.

use super::librespot::FalhaDeAudio;
use super::pedidos;
use provedores::Causa;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

/// O tamanho de cada pedaço guardado. Um pouco maior que o do tocador (64 KB), para o servidor não pedir ao Spotify a
/// cada leitura do decodificador.
pub const PEDACO: u64 = 128 * 1024;
/// O teto do áudio decifrado guardado ao mesmo tempo. Com Ogg 320, são uns 3 minutos de música: a faixa de agora, a
/// seguinte e a pré-análise cabem; uma fila inteira nunca.
pub const TETO_DA_MEMORIA: usize = 8 * 1024 * 1024;
/// Quantas faixas podem ficar abertas ao mesmo tempo (a de agora, as duas seguintes e a pré-análise).
pub const FAIXAS_ABERTAS: usize = 4;

/// Quem baixa e decifra um pedaço do Spotify. O de verdade fala com o librespot (`librespot.rs`); os testes usam um de
/// mentira, e é por isso que este módulo não sabe nada de rede.
pub trait Baixador: Send + Sync + 'static {
    /// O tamanho do arquivo inteiro de uma faixa, em bytes.
    fn tamanho(&self, id: &str) -> Result<u64, FalhaDeAudio>;
    /// Os bytes `[de, de+quantos)` do arquivo, **já decifrados**.
    fn pedaco(&self, id: &str, de: u64, quantos: u64) -> Result<Vec<u8>, FalhaDeAudio>;
    /// A extensão do formato que toca nesta conta (`ogg`, `mp3`), para o decodificador saber.
    ///
    /// É aqui que a faixa é aberta de verdade: um erro (conta sem Premium, chave recusada, faixa fora do catálogo) tem de
    /// sair antes de o Player receber um endereço.
    fn formato(&self, id: &str) -> Result<String, FalhaDeAudio>;
}

/// O estado de uma faixa aberta.
struct Faixa {
    tamanho: u64,
    pedacos: HashMap<u64, Arc<Vec<u8>>>,
    /// O último uso, para o teto esquecer a mais antiga primeiro.
    usado: u64,
}

#[derive(Default)]
struct Estado {
    faixas: HashMap<String, Faixa>,
    bytes: usize,
    relogio: u64,
}

/// Quem espera um pedaço que outro está baixando.
type Esperando = Vec<Sender<Result<Arc<Vec<u8>>, FalhaDeAudio>>>;

pub struct Cache {
    baixador: Arc<dyn Baixador>,
    estado: Mutex<Estado>,
    /// Os pedaços sendo baixados agora (`id:bloco`): quem pede o mesmo espera o primeiro terminar, em vez de baixar duas
    /// vezes (o tocador e a pré-análise abrem a mesma faixa).
    baixando: Mutex<HashMap<String, Esperando>>,
    teto: usize,
    abertas: usize,
}

fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Cache {
    pub fn novo(baixador: Arc<dyn Baixador>) -> Self {
        Self::com_limites(baixador, TETO_DA_MEMORIA, FAIXAS_ABERTAS)
    }

    /// Com tetos escolhidos (os testes usam um pequeno, para o esquecimento acontecer de propósito).
    pub fn com_limites(baixador: Arc<dyn Baixador>, teto: usize, abertas: usize) -> Self {
        Self { baixador, estado: Mutex::default(), baixando: Mutex::default(), teto, abertas: abertas.max(1) }
    }

    /// O tamanho do arquivo de uma faixa, abrindo-a se for a primeira vez.
    pub fn tamanho(&self, id: &str) -> Result<u64, FalhaDeAudio> {
        if let Some(t) = travar(&self.estado).faixas.get(id).map(|f| f.tamanho) {
            return Ok(t);
        }
        let tamanho = self.baixador.tamanho(id)?;
        let mut e = travar(&self.estado);
        e.relogio += 1;
        let relogio = e.relogio;
        let antiga = e.faixas.insert(id.to_string(), Faixa { tamanho, pedacos: HashMap::new(), usado: relogio });
        if let Some(antiga) = antiga {
            e.bytes = e.bytes.saturating_sub(antiga.pedacos.values().map(|p| p.len()).sum::<usize>());
        }
        self.esquecer_os_velhos(&mut e, id);
        Ok(tamanho)
    }

    /// Os bytes `[de, ate)` de uma faixa, já decifrados. Um erro no meio do caminho volta com o motivo (e o pedaço não
    /// fica guardado: a próxima tentativa baixa de novo).
    pub fn ler(&self, id: &str, faixa: pedidos::Faixa) -> Result<Vec<u8>, FalhaDeAudio> {
        let mut saida = Vec::with_capacity(faixa.quantos as usize);
        let mut de = faixa.de;
        while de < faixa.ate() {
            let bloco = de / PEDACO;
            let pedaco = self.pedaco(id, bloco)?;
            let dentro = (de - bloco * PEDACO) as usize;
            let quanto = (faixa.ate() - de).min((pedaco.len().saturating_sub(dentro)) as u64) as usize;
            if quanto == 0 {
                break;
            }
            saida.extend_from_slice(&pedaco[dentro..dentro + quanto]);
            de += quanto as u64;
        }
        Ok(saida)
    }

    /// O formato que toca nesta conta (para o `resolver` dizer ao Player).
    pub fn formato(&self, id: &str) -> Result<String, FalhaDeAudio> {
        self.baixador.formato(id)
    }

    /// O pedaço `bloco` de uma faixa: da memória, ou baixado agora (um só download por pedaço, mesmo com dois leitores).
    fn pedaco(&self, id: &str, bloco: u64) -> Result<Arc<Vec<u8>>, FalhaDeAudio> {
        {
            let mut e = travar(&self.estado);
            e.relogio += 1;
            let relogio = e.relogio;
            if let Some(f) = e.faixas.get_mut(id) {
                f.usado = relogio;
                if let Some(p) = f.pedacos.get(&bloco) {
                    return Ok(Arc::clone(p));
                }
            }
        }
        let chave = format!("{id}:{bloco}");
        let (avisar, espera) = std::sync::mpsc::channel();
        let sou_eu = {
            let mut b = travar(&self.baixando);
            match b.get_mut(&chave) {
                // Já tem alguém baixando este pedaço: espero a resposta dele.
                Some(esperando) => {
                    esperando.push(avisar);
                    false
                }
                None => {
                    b.insert(chave.clone(), vec![avisar]);
                    true
                }
            }
        };
        if !sou_eu {
            // O primeiro termina em segundos; se ele sumir sem avisar, cai no erro comum.
            return espera.recv().unwrap_or_else(|_| Err(FalhaDeAudio::nova(Causa::Rede, "o pedaço não chegou")));
        }
        let de = bloco * PEDACO;
        let resultado = self.tamanho(id).and_then(|t| {
            let quantos = t.saturating_sub(de).min(PEDACO);
            if quantos == 0 {
                return Err(FalhaDeAudio::nova(Causa::Quebrado, "o pedaço está depois do fim da faixa"));
            }
            self.baixador.pedaco(id, de, quantos).map(Arc::new)
        });
        let resultado = resultado.inspect(|p| {
            let mut e = travar(&self.estado);
            if let Some(f) = e.faixas.get_mut(id) {
                if f.pedacos.insert(bloco, Arc::clone(p)).is_none() {
                    e.bytes += p.len();
                }
                self.esquecer_os_velhos(&mut e, id);
            }
        });
        // Avisa quem esperava (antes de soltar a lista: ninguém fica sem resposta).
        for quem in travar(&self.baixando).remove(&chave).unwrap_or_default() {
            let _ = quem.send(resultado.clone());
        }
        resultado
    }

    /// Esquece as faixas mais antigas até caber no teto. `em_uso` nunca sai, e a que tiver menos pedaços sai antes das
    /// outras (esquecer uma que mal começou devolve mais memória por byte perdido).
    fn esquecer_os_velhos(&self, e: &mut Estado, em_uso: &str) {
        while e.bytes > self.teto || e.faixas.len() > self.abertas {
            let Some(velha) = e
                .faixas
                .iter()
                .filter(|(id, _)| id.as_str() != em_uso)
                .min_by_key(|(_, f)| (f.usado, f.pedacos.len()))
                .map(|(id, _)| id.clone())
            else {
                return;
            };
            if let Some(f) = e.faixas.remove(&velha) {
                e.bytes = e.bytes.saturating_sub(f.pedacos.values().map(|p| p.len()).sum::<usize>());
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Um Spotify de mentira: cada faixa é uma sequência previsível de bytes, e ele conta os downloads.
    struct Falso {
        tamanho: u64,
        downloads: AtomicUsize,
        falhar: Mutex<Option<String>>,
    }

    impl Falso {
        fn novo(tamanho: u64) -> Arc<Self> {
            Arc::new(Self { tamanho, downloads: AtomicUsize::new(0), falhar: Mutex::default() })
        }
    }

    impl Baixador for Falso {
        fn tamanho(&self, _: &str) -> Result<u64, FalhaDeAudio> {
            Ok(self.tamanho)
        }
        fn pedaco(&self, _: &str, de: u64, quantos: u64) -> Result<Vec<u8>, FalhaDeAudio> {
            if let Some(motivo) = travar(&self.falhar).clone() {
                return Err(FalhaDeAudio::nova(Causa::Rede, motivo));
            }
            self.downloads.fetch_add(1, Ordering::SeqCst);
            Ok((de..de + quantos).map(|i| (i % 251) as u8).collect())
        }
        fn formato(&self, _: &str) -> Result<String, FalhaDeAudio> {
            Ok("ogg".into())
        }
    }

    fn bytes(de: u64, quantos: u64) -> Vec<u8> {
        (de..de + quantos).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn le_a_faixa_inteira_em_pedacos_e_acerta_os_bytes() {
        let falso = Falso::novo(PEDACO * 2 + 10);
        let cache = Cache::novo(falso.clone());
        let todos = cache.ler("x", pedidos::Faixa::inteira(PEDACO * 2 + 10)).expect("lê");
        assert_eq!(todos, bytes(0, PEDACO * 2 + 10));
        assert_eq!(falso.downloads.load(Ordering::SeqCst), 3);
        // Ler de novo não baixa nada.
        let meio = cache.ler("x", pedidos::Faixa { de: 100, quantos: 200, parcial: true }).expect("lê");
        assert_eq!(meio, bytes(100, 200));
        assert_eq!(falso.downloads.load(Ordering::SeqCst), 3, "veio da memória");
        assert_eq!(cache.formato("x").as_deref(), Ok("ogg"));
    }

    #[test]
    fn um_pedaco_que_atravessa_dois_blocos_vem_certo() {
        let cache = Cache::novo(Falso::novo(PEDACO * 3));
        let de = PEDACO + 5;
        let quantos = PEDACO - 5;
        let lido = cache.ler("x", pedidos::Faixa { de, quantos, parcial: true }).expect("lê");
        assert_eq!(lido, bytes(de, quantos));
    }

    #[test]
    fn dois_leitores_do_mesmo_pedaco_baixam_uma_vez_so() {
        let falso = Falso::novo(PEDACO);
        let cache = Arc::new(Cache::novo(falso.clone()));
        let mut threads = Vec::new();
        for _ in 0..4 {
            let cache = Arc::clone(&cache);
            threads.push(std::thread::spawn(move || cache.ler("x", pedidos::Faixa::inteira(PEDACO)).expect("lê")));
        }
        for t in threads {
            assert_eq!(t.join().expect("thread"), bytes(0, PEDACO));
        }
        assert_eq!(falso.downloads.load(Ordering::SeqCst), 1, "um download para os quatro");
    }

    #[test]
    fn o_teto_da_memoria_esquece_a_faixa_mais_antiga() {
        let cache = Cache::com_limites(Falso::novo(PEDACO), (PEDACO as usize) * 3 / 2, FAIXAS_ABERTAS);
        for id in ["a", "b", "c"] {
            cache.ler(id, pedidos::Faixa::inteira(PEDACO)).expect("lê");
        }
        let e = travar(&cache.estado);
        assert!(e.bytes <= cache.teto, "coube no teto: {}", e.bytes);
        assert!(!e.faixas.contains_key("a"), "a mais antiga saiu");
        assert!(e.faixas.contains_key("c"), "a de agora fica");
    }

    #[test]
    fn erro_no_download_volta_e_nao_fica_guardado() {
        let falso = Falso::novo(PEDACO);
        *travar(&falso.falhar) = Some("sem conexão".into());
        let cache = Cache::novo(falso.clone());
        assert_eq!(
            cache.ler("x", pedidos::Faixa::inteira(PEDACO)),
            Err(FalhaDeAudio::nova(Causa::Rede, "sem conexão"))
        );
        assert!(travar(&cache.estado).faixas["x"].pedacos.is_empty(), "nada guardado");
        *travar(&falso.falhar) = None;
        assert_eq!(cache.ler("x", pedidos::Faixa::inteira(PEDACO)).expect("agora vai").len(), PEDACO as usize);
    }
}
