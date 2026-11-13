//! A thread das letras: recebe "a faixa agora é esta" e responde com a letra, do banco local ou do LRCLIB.
//!
//! - Só o pedido mais novo interessa: pular várias faixas seguidas vira uma busca só.
//! - O que o LRCLIB responde fica guardado no banco: a letra para sempre, o "não tem" por um dia. Falha de rede
//!   não guarda nada, e a busca é refeita quando a interface pedir de novo.
//! - A pessoa também pode procurar à mão (quando a busca automática não acha, ou acha a música errada) e fixar
//!   um dos resultados na faixa que está tocando. A escolha fica guardada como qualquer outra letra.
//! - A interface só lê a última resposta. Nunca espera a rede.

use historico::{Banco, Guardada, LetraGuardada};
use letras::{Cliente, Linha, Pedido, Registro, ler_lrc};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Espera antes de buscar, para juntar pulos rápidos de faixa.
const ASSENTAR: Duration = Duration::from_millis(400);
/// Letra só de texto: limite de linhas mostradas.
const MAXIMO_DE_LINHAS: usize = 3000;

/// Uma letra pronta para a tela.
#[derive(Debug, Default, PartialEq)]
pub struct Letra {
    pub linhas: Vec<Linha>,
    /// As linhas têm tempo. Sem isso, a tela rola a letra na proporção da música.
    pub sincronizada: bool,
    pub instrumental: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Situacao {
    #[default]
    Parada,
    Buscando,
    Achou(Arc<Letra>),
    SemLetra,
    /// Não deu para perguntar (sem internet, por exemplo).
    SemRede,
}

/// A última resposta, com a chave da faixa a que ela se refere.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resposta {
    pub chave: String,
    pub situacao: Situacao,
}

/// O resultado da última busca feita à mão.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Achados {
    #[default]
    Nenhuma,
    Buscando,
    Prontos(Arc<Vec<Registro>>),
    SemRede,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Procura {
    /// O texto procurado.
    pub consulta: String,
    pub achados: Achados,
}

enum Recado {
    /// A letra da faixa que está tocando (chave, o que se sabe dela).
    Buscar(String, Pedido),
    /// Busca à mão, pelo texto digitado.
    Procurar(String),
    /// A pessoa escolheu esta letra para a faixa de chave dada.
    Fixar(String, Box<Registro>),
}

pub struct Letrista {
    recados: Option<Sender<Recado>>,
    resposta: Arc<Mutex<Resposta>>,
    procura: Arc<Mutex<Procura>>,
    versao: Arc<AtomicU64>,
}

fn trocar<T>(m: &Mutex<T>, valor: T) {
    match m.lock() {
        Ok(mut g) => *g = valor,
        Err(p) => *p.into_inner() = valor,
    }
}

fn ler<T: Clone>(m: &Mutex<T>) -> T {
    match m.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    }
}

/// Junta os recados que chegaram: das buscas automáticas e das buscas à mão só a mais nova de cada tipo vale;
/// as escolhas ("fixar") valem todas. A ordem de chegada é mantida.
fn enxugar(fila: &mut VecDeque<Recado>) {
    let ultima_busca = fila.iter().rposition(|r| matches!(r, Recado::Buscar(..)));
    let ultima_procura = fila.iter().rposition(|r| matches!(r, Recado::Procurar(..)));
    let mut i = 0;
    fila.retain(|r| {
        let fica = match r {
            Recado::Buscar(..) => Some(i) == ultima_busca,
            Recado::Procurar(..) => Some(i) == ultima_procura,
            Recado::Fixar(..) => true,
        };
        i += 1;
        fica
    });
}

/// Abre a ligação com a rede na primeira vez que ela é necessária.
fn garantir(cliente: &mut Option<Cliente>) {
    if cliente.is_none() {
        *cliente = Cliente::novo().map_err(|e| log::warn!("busca de letras indisponível: {e}")).ok();
    }
}

fn guardada_de(r: Registro) -> LetraGuardada {
    LetraGuardada { sincronizada: r.synced_lyrics, simples: r.plain_lyrics, instrumental: r.instrumental }
}

impl Letrista {
    /// `caminho` é o banco onde as letras ficam guardadas; sem ele, a busca funciona e só não lembra.
    pub fn iniciar(caminho: Option<PathBuf>) -> Self {
        let resposta = Arc::new(Mutex::new(Resposta::default()));
        let procura = Arc::new(Mutex::new(Procura::default()));
        let versao = Arc::new(AtomicU64::new(0));
        let (recados, chegada) = channel::<Recado>();
        let (r, p, v) = (resposta.clone(), procura.clone(), versao.clone());
        let criada = std::thread::Builder::new().name("letras".into()).spawn(move || {
            let publicar = |chave: &str, situacao: Situacao| {
                trocar(&r, Resposta { chave: chave.to_string(), situacao });
                v.fetch_add(1, Ordering::Release);
            };
            let publicar_procura = |consulta: &str, achados: Achados| {
                trocar(&p, Procura { consulta: consulta.to_string(), achados });
                v.fetch_add(1, Ordering::Release);
            };
            // O banco e a rede só são abertos no primeiro pedido: quem não usa o módulo Letra não paga por eles.
            let mut banco: Option<Banco> = None;
            let mut cliente: Option<Cliente> = None;
            let mut fila: VecDeque<Recado> = VecDeque::new();
            loop {
                if fila.is_empty() {
                    match chegada.recv() {
                        Ok(recado) => fila.push_back(recado),
                        Err(_) => return,
                    }
                    // Só a busca automática espera: quem digitou ou escolheu quer a resposta logo.
                    if matches!(fila.front(), Some(Recado::Buscar(..))) {
                        std::thread::sleep(ASSENTAR);
                    }
                }
                fila.extend(chegada.try_iter());
                enxugar(&mut fila);
                let Some(recado) = fila.pop_front() else { continue };

                if banco.is_none() {
                    banco = match caminho.as_deref().map(Banco::abrir).unwrap_or_else(Banco::em_memoria) {
                        Ok(b) => Some(b),
                        Err(e) => {
                            log::warn!("as letras não serão guardadas: {e}");
                            Banco::em_memoria().ok()
                        }
                    };
                }
                let agora = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
                let guardar = |chave: &str, letra: Option<&LetraGuardada>| {
                    if let Some(b) = banco.as_ref()
                        && let Err(e) = b.guardar_letra(chave, letra, agora)
                    {
                        log::debug!("não foi possível guardar a letra: {e}");
                    }
                };

                match recado {
                    Recado::Fixar(chave, registro) => {
                        let guardada = guardada_de(*registro);
                        guardar(&chave, Some(&guardada));
                        publicar(&chave, situacao_de(&guardada));
                    }
                    Recado::Procurar(consulta) => {
                        publicar_procura(&consulta, Achados::Buscando);
                        garantir(&mut cliente);
                        let achados = match cliente.as_ref().map(|c| c.procurar(&consulta)) {
                            Some(Ok(lista)) => Achados::Prontos(Arc::new(lista)),
                            Some(Err(e)) => {
                                log::debug!("busca manual de letra falhou: {e}");
                                Achados::SemRede
                            }
                            None => Achados::SemRede,
                        };
                        publicar_procura(&consulta, achados);
                    }
                    Recado::Buscar(chave, pedido) => {
                        publicar(&chave, Situacao::Buscando);
                        match banco.as_ref().map(|b| b.letra(&chave, agora)) {
                            Some(Ok(Guardada::Letra(guardada))) => {
                                publicar(&chave, situacao_de(&guardada));
                                continue;
                            }
                            Some(Ok(Guardada::SemLetra)) => {
                                publicar(&chave, Situacao::SemLetra);
                                continue;
                            }
                            Some(Err(e)) => log::debug!("leitura da letra guardada falhou: {e}"),
                            Some(Ok(Guardada::Nada)) | None => {}
                        }
                        garantir(&mut cliente);
                        let achado = match cliente.as_ref() {
                            Some(c) => c.buscar(&pedido),
                            None => Err(nucleo::Erro::Rede("sem cliente".into())),
                        };
                        let situacao = match achado {
                            Ok(registro) => {
                                let guardada = registro.map(guardada_de);
                                guardar(&chave, guardada.as_ref());
                                guardada.as_ref().map_or(Situacao::SemLetra, situacao_de)
                            }
                            Err(e) => {
                                log::debug!("busca da letra falhou: {e}");
                                Situacao::SemRede
                            }
                        };
                        publicar(&chave, situacao);
                    }
                }
            }
        });
        match criada {
            Ok(_) => Self { recados: Some(recados), resposta, procura, versao },
            Err(e) => {
                log::warn!("busca de letras desligada: {e}");
                Self { recados: None, resposta, procura, versao }
            }
        }
    }

    fn mandar(&self, recado: Recado) {
        if let Some(r) = &self.recados {
            let _ = r.send(recado);
        }
    }

    /// Pede a letra de uma faixa. Não espera: o resultado aparece em [`Self::resposta`].
    pub fn pedir(&self, chave: String, pedido: Pedido) {
        self.mandar(Recado::Buscar(chave, pedido));
    }

    /// Procura letras pelo texto digitado. O resultado aparece em [`Self::procura`].
    pub fn procurar(&self, consulta: String) {
        self.mandar(Recado::Procurar(consulta));
    }

    /// Usa `registro` como a letra da faixa de chave `chave`, e guarda a escolha.
    pub fn fixar(&self, chave: String, registro: Registro) {
        self.mandar(Recado::Fixar(chave, Box::new(registro)));
    }

    pub fn versao(&self) -> u64 {
        self.versao.load(Ordering::Acquire)
    }

    pub fn resposta(&self) -> Resposta {
        ler(&self.resposta)
    }

    pub fn procura(&self) -> Procura {
        ler(&self.procura)
    }
}

/// Transforma o que está guardado na letra que a tela mostra: a sincronizada, se houver; senão, só o texto.
fn situacao_de(guardada: &LetraGuardada) -> Situacao {
    if let Some(linhas) = guardada.sincronizada.as_deref().map(ler_lrc).filter(|l| !l.is_empty()) {
        return Situacao::Achou(Arc::new(Letra { linhas, sincronizada: true, instrumental: false }));
    }
    let linhas: Vec<Linha> = guardada
        .simples
        .as_deref()
        .unwrap_or_default()
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|l| !l.is_empty())
        .take(MAXIMO_DE_LINHAS)
        .map(|texto| Linha { t: 0.0, texto: texto.chars().filter(|c| !c.is_control()).take(300).collect() })
        .collect();
    if !linhas.is_empty() {
        Situacao::Achou(Arc::new(Letra { linhas, sincronizada: false, instrumental: false }))
    } else if guardada.instrumental {
        Situacao::Achou(Arc::new(Letra { instrumental: true, ..Default::default() }))
    } else {
        Situacao::SemLetra
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod testes {
    use super::*;

    #[test]
    fn sincronizada_ganha_do_texto() {
        let g = LetraGuardada {
            sincronizada: Some("[00:02.00]linha um\n[00:04.00]linha dois".into()),
            simples: Some("linha um\nlinha dois".into()),
            instrumental: false,
        };
        let Situacao::Achou(letra) = situacao_de(&g) else { panic!("devia achar") };
        assert!(letra.sincronizada);
        assert_eq!(letra.linhas.len(), 2);
        assert_eq!(letra.linhas[1].t, 4.0);
    }

    #[test]
    fn so_texto_e_instrumental() {
        let texto = LetraGuardada { simples: Some("linha um\n\n  linha   dois \n".into()), ..Default::default() };
        let Situacao::Achou(letra) = situacao_de(&texto) else { panic!("devia achar") };
        assert!(!letra.sincronizada);
        assert_eq!(letra.linhas.iter().map(|l| l.texto.as_str()).collect::<Vec<_>>(), ["linha um", "linha dois"]);

        let instrumental = LetraGuardada { instrumental: true, ..Default::default() };
        let Situacao::Achou(letra) = situacao_de(&instrumental) else { panic!("devia achar") };
        assert!(letra.instrumental && letra.linhas.is_empty());

        // Sincronizada que não é LRC de verdade cai para o texto; sem nada, não há letra.
        let quebrada = LetraGuardada { sincronizada: Some("sem tempos".into()), ..Default::default() };
        assert_eq!(situacao_de(&quebrada), Situacao::SemLetra);
        assert_eq!(situacao_de(&LetraGuardada::default()), Situacao::SemLetra);
    }

    #[test]
    fn so_o_recado_mais_novo_de_cada_tipo() {
        let buscar = |c: &str| Recado::Buscar(c.into(), Pedido::default());
        let fixar = |c: &str| Recado::Fixar(c.into(), Box::default());
        let mut fila: VecDeque<Recado> = VecDeque::from([
            buscar("a"),
            Recado::Procurar("um".into()),
            fixar("a"),
            buscar("b"),
            Recado::Procurar("dois".into()),
            fixar("b"),
        ]);
        enxugar(&mut fila);
        let resumo: Vec<String> = fila
            .iter()
            .map(|r| match r {
                Recado::Buscar(c, _) => format!("buscar {c}"),
                Recado::Procurar(c) => format!("procurar {c}"),
                Recado::Fixar(c, _) => format!("fixar {c}"),
            })
            .collect();
        assert_eq!(resumo, ["fixar a", "buscar b", "procurar dois", "fixar b"]);
    }
}
