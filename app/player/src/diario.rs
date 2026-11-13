//! A thread do histórico: acompanha o que está tocando e grava no banco local.
//!
//! Ela não depende da janela: duas vezes por segundo lê o estado da mídia e o retrato do som e soma o tempo
//! ouvido, então a música conta mesmo com o Player minimizado. Uma música entra no histórico depois de 30 s
//! ouvidos (ou da metade dela, se for curta) e a linha é atualizada a cada 10 s enquanto toca: fechar o app no
//! meio não perde quase nada.
//!
//! A interface só pede o resumo de um período e lê o último que ficou pronto. Nunca espera o disco.

use crate::escuta::LeitorDeRetrato;
use historico::{Banco, Consulta, Execucao, Resumo, inicio_do_dia};
use sessao::Faixa;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const PASSO: Duration = Duration::from_millis(500);
/// Tempo ouvido para a música entrar no histórico.
const MINIMO_S: f64 = 30.0;
/// Músicas curtas entram com metade da duração, desde que passe disto.
const MINIMO_CURTA_S: f64 = 10.0;
/// De quanto em quanto tempo ouvido a linha é regravada enquanto a música toca.
const INTERVALO_DE_GRAVACAO_S: f64 = 10.0;
const DIA_S: i64 = 86_400;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Periodo {
    Hoje,
    #[default]
    Semana,
    Mes,
    Tudo,
}

impl Periodo {
    pub const TODOS: [Periodo; 4] = [Periodo::Hoje, Periodo::Semana, Periodo::Mes, Periodo::Tudo];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Hoje => "Hoje",
            Self::Semana => "7 dias",
            Self::Mes => "30 dias",
            Self::Tudo => "Tudo",
        }
    }

    fn desde(self, agora: i64, fuso_s: i64) -> Option<i64> {
        match self {
            Self::Hoje => Some(inicio_do_dia(agora, fuso_s)),
            Self::Semana => Some(agora - 7 * DIA_S),
            Self::Mes => Some(agora - 30 * DIA_S),
            Self::Tudo => None,
        }
    }
}

/// O resumo de um período, pronto para a tela.
pub struct Relatorio {
    pub periodo: Periodo,
    pub resumo: Resumo,
    pub agora: i64,
    pub fuso_s: i64,
}

/// Ligação com a thread do histórico. Soltar o valor grava o que falta e encerra a thread.
pub struct Diario {
    pedidos: Option<Sender<Periodo>>,
    relatorio: Arc<Mutex<Option<Arc<Relatorio>>>>,
    versao: Arc<AtomicU64>,
    ativo: Arc<AtomicBool>,
}

impl Diario {
    /// `caminho = None` (sem pasta de dados) desliga o histórico: o app funciona, só não lembra o que tocou.
    pub fn iniciar(caminho: Option<PathBuf>, midia: sessao::Leitor, som: LeitorDeRetrato) -> Self {
        let relatorio = Arc::new(Mutex::new(None));
        let versao = Arc::new(AtomicU64::new(0));
        let ativo = Arc::new(AtomicBool::new(false));
        let desligado = |relatorio, versao, ativo| Self { pedidos: None, relatorio, versao, ativo };
        let Some(caminho) = caminho else { return desligado(relatorio, versao, ativo) };
        let (pedidos, fila) = channel::<Periodo>();
        let (r, v, a) = (relatorio.clone(), versao.clone(), ativo.clone());
        let criada = std::thread::Builder::new().name("historico".into()).spawn(move || {
            let banco = match Banco::abrir(&caminho) {
                Ok(b) => b,
                Err(e) => {
                    log::warn!("histórico desligado: {e}");
                    return;
                }
            };
            a.store(true, Ordering::Release);
            let mut gravar = |t: &mut Tocada| {
                let feito = match t.id {
                    Some(id) => banco.atualizar(id, &t.exec),
                    None => banco.registrar(&t.exec).map(|id| t.id = Some(id)),
                };
                if let Err(e) = feito {
                    log::warn!("não foi possível gravar no histórico: {e}");
                }
            };
            let mut acompanhador = Acompanhador::default();
            let mut anterior = Instant::now();
            loop {
                let pedido = match fila.recv_timeout(PASSO) {
                    Ok(p) => Some(p),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => {
                        acompanhador.fechar(&mut gravar);
                        return;
                    }
                };
                // Tempo real, não o do passo: um pedido acorda a thread antes da hora.
                let dt = anterior.elapsed().as_secs_f64().min(2.0);
                anterior = Instant::now();
                let (estado, retrato) = (midia.estado(), som.retrato());
                let amostra = Amostra {
                    faixa: estado.faixa.as_ref(),
                    tocando: estado.tocando,
                    posicao: estado.posicao.map(|p| (p.agora(estado.tocando), p.duracao_s)),
                    com_som: retrato.gate > 0.5,
                    energia: retrato.energia,
                    bpm: (retrato.conf_batida > 0.15).then(|| retrato.bpm_exibido()).filter(|b| *b >= 1.0),
                };
                acompanhador.passo(&amostra, dt, agora_unix(), &mut gravar);

                if let Some(periodo) = pedido {
                    // A música que está tocando aparece no resumo com o tempo de agora.
                    acompanhador.gravar_agora(&mut gravar);
                    let (agora, fuso_s) = (agora_unix(), fuso_local_s());
                    let consulta =
                        Consulta { desde: periodo.desde(agora, fuso_s), agora, fuso_s, tops: 5, recentes: 12 };
                    match banco.resumo(consulta) {
                        Ok(resumo) => {
                            let pronto = Arc::new(Relatorio { periodo, resumo, agora, fuso_s });
                            match r.lock() {
                                Ok(mut g) => *g = Some(pronto),
                                Err(p) => *p.into_inner() = Some(pronto),
                            }
                            v.fetch_add(1, Ordering::Release);
                        }
                        Err(e) => log::warn!("não foi possível ler o histórico: {e}"),
                    }
                }
            }
        });
        match criada {
            Ok(_) => Self { pedidos: Some(pedidos), relatorio, versao, ativo },
            Err(e) => {
                log::warn!("histórico desligado: {e}");
                desligado(relatorio, versao, ativo)
            }
        }
    }

    /// O banco abriu e o histórico está gravando.
    pub fn ativo(&self) -> bool {
        self.ativo.load(Ordering::Acquire)
    }

    /// Pede o resumo de um período. Não espera: o resultado aparece em [`Self::relatorio`].
    pub fn pedir(&self, periodo: Periodo) {
        if let Some(p) = &self.pedidos {
            let _ = p.send(periodo);
        }
    }

    pub fn versao(&self) -> u64 {
        self.versao.load(Ordering::Acquire)
    }

    pub fn relatorio(&self) -> Option<Arc<Relatorio>> {
        match self.relatorio.lock() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }
}

fn agora_unix() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Diferença da hora local para UTC, em segundos, a partir das duas horas lidas do sistema.
/// Cada hora vem como (ano, mês, dia, minutos do dia).
fn fuso_de(local: (u16, u16, u16, i64), utc: (u16, u16, u16, i64)) -> i64 {
    let dia = match (local.0, local.1, local.2).cmp(&(utc.0, utc.1, utc.2)) {
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
    };
    let minutos = dia * 1440 + local.3 - utc.3;
    // Os fusos são múltiplos de 15 min: o arredondamento cobre as duas leituras caírem em minutos diferentes.
    (minutos as f64 / 15.0).round() as i64 * 15 * 60
}

pub fn fuso_local_s() -> i64 {
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::{GetLocalTime, GetSystemTime};
        // SAFETY: funções do Windows sem parâmetros, que só devolvem a hora atual.
        let (l, u) = unsafe { (GetLocalTime(), GetSystemTime()) };
        let minutos = |h: u16, m: u16| i64::from(h) * 60 + i64::from(m);
        fuso_de(
            (l.wYear, l.wMonth, l.wDay, minutos(l.wHour, l.wMinute)),
            (u.wYear, u.wMonth, u.wDay, minutos(u.wHour, u.wMinute)),
        )
    }
    #[cfg(not(windows))]
    {
        0
    }
}

// ---------------------------------------------------------------------------------------- acompanhamento

/// O que se sabe do som e da mídia num instante.
struct Amostra<'a> {
    faixa: Option<&'a Faixa>,
    tocando: bool,
    /// (posição, duração), quando o app de origem informa.
    posicao: Option<(f64, f64)>,
    com_som: bool,
    energia: f32,
    bpm: Option<f32>,
}

/// A música que está tocando agora, com o que já foi somado dela.
struct Tocada {
    faixa: Faixa,
    exec: Execucao,
    /// Linha no banco, depois da primeira gravação.
    id: Option<i64>,
    /// Tempo ouvido que já está gravado.
    gravado_s: f64,
    soma_energia: f64,
    soma_bpm: f64,
    peso_bpm: f64,
    ultima_posicao: Option<f64>,
}

impl Tocada {
    fn nova(faixa: &Faixa, agora: i64) -> Self {
        Self {
            faixa: faixa.clone(),
            exec: Execucao {
                inicio: agora,
                titulo: faixa.titulo.clone(),
                artista: faixa.artista.clone(),
                album: faixa.album.clone(),
                app: faixa.app.clone(),
                ..Default::default()
            },
            id: None,
            gravado_s: 0.0,
            soma_energia: 0.0,
            soma_bpm: 0.0,
            peso_bpm: 0.0,
            ultima_posicao: None,
        }
    }

    fn vale(&self) -> bool {
        let (ouvido, duracao) = (self.exec.ouvido_s, self.exec.duracao_s);
        ouvido >= MINIMO_S || (duracao > 0.0 && ouvido >= (duracao * 0.5).max(MINIMO_CURTA_S))
    }
}

#[derive(Default)]
struct Acompanhador {
    atual: Option<Tocada>,
}

impl Acompanhador {
    fn passo(&mut self, a: &Amostra, dt: f64, agora: i64, gravar: &mut dyn FnMut(&mut Tocada)) {
        let faixa = a.faixa.filter(|f| !f.titulo.is_empty());
        let mudou = match (&self.atual, faixa) {
            (Some(t), Some(f)) => t.faixa.chave() != f.chave(),
            (None, None) => false,
            _ => true,
        };
        // A mesma música de novo (repetir uma): a posição volta ao começo depois de ter passado da metade.
        let repetiu = !mudou
            && self.atual.as_ref().zip(a.posicao).is_some_and(|(t, (posicao, duracao))| {
                posicao < 5.0
                    && t.ultima_posicao.is_some_and(|antes| antes - posicao > duracao * 0.5)
                    && t.exec.ouvido_s >= duracao * 0.5
            });
        if mudou || repetiu {
            self.fechar(gravar);
            self.atual = faixa.map(|f| Tocada::nova(f, agora));
        }
        let Some(t) = self.atual.as_mut() else { return };
        if let Some((posicao, duracao)) = a.posicao {
            t.exec.duracao_s = duracao;
            t.ultima_posicao = Some(posicao);
        }
        if a.tocando && a.com_som && dt > 0.0 {
            t.exec.ouvido_s += dt;
            t.soma_energia += f64::from(a.energia.clamp(0.0, 1.0)) * dt;
            t.exec.energia = (t.soma_energia / t.exec.ouvido_s) as f32;
            if let Some(bpm) = a.bpm.filter(|b| b.is_finite()) {
                t.soma_bpm += f64::from(bpm) * dt;
                t.peso_bpm += dt;
                t.exec.bpm = (t.soma_bpm / t.peso_bpm) as f32;
            }
        }
        if t.vale() && t.exec.ouvido_s - t.gravado_s >= INTERVALO_DE_GRAVACAO_S {
            gravar(t);
            t.gravado_s = t.exec.ouvido_s;
        }
    }

    /// Grava o que falta da música atual, se ela já vale e há algo novo.
    fn gravar_agora(&mut self, gravar: &mut dyn FnMut(&mut Tocada)) {
        if let Some(t) = self.atual.as_mut().filter(|t| t.vale() && t.exec.ouvido_s > t.gravado_s) {
            gravar(t);
            t.gravado_s = t.exec.ouvido_s;
        }
    }

    fn fechar(&mut self, gravar: &mut dyn FnMut(&mut Tocada)) {
        self.gravar_agora(gravar);
        self.atual = None;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn faixa(titulo: &str) -> Faixa {
        Faixa { titulo: titulo.into(), artista: "Banda".into(), album: String::new(), app: "Teste".into() }
    }

    /// Roda `segundos` de música tocando, um passo por segundo, e devolve o que foi gravado: (id, ouvido).
    fn tocar(
        ac: &mut Acompanhador,
        gravadas: &mut Vec<(i64, f64)>,
        f: Option<&Faixa>,
        segundos: u32,
        de: f64,
        duracao: Option<f64>,
        tocando: bool,
    ) {
        let mut proximo = gravadas.iter().map(|(id, _)| *id).max().unwrap_or(0) + 1;
        for i in 0..segundos {
            let amostra = Amostra {
                faixa: f,
                tocando,
                posicao: duracao.map(|d| (de + f64::from(i), d)),
                com_som: true,
                energia: 0.5,
                bpm: Some(120.0),
            };
            ac.passo(&amostra, 1.0, 1000 + i64::from(i), &mut |t: &mut Tocada| {
                let id = *t.id.get_or_insert_with(|| {
                    proximo += 1;
                    proximo - 1
                });
                gravadas.retain(|(g, _)| *g != id);
                gravadas.push((id, t.exec.ouvido_s));
            });
        }
    }

    #[test]
    fn so_entra_depois_de_meio_minuto() {
        let (mut ac, mut g) = (Acompanhador::default(), Vec::new());
        let a = faixa("A");
        tocar(&mut ac, &mut g, Some(&a), 29, 0.0, Some(200.0), true);
        assert!(g.is_empty(), "29 s ainda não contam");
        tocar(&mut ac, &mut g, Some(&a), 1, 29.0, Some(200.0), true);
        assert_eq!(g, vec![(1, 30.0)]);
        // Continua tocando: a mesma linha é regravada a cada 10 s.
        tocar(&mut ac, &mut g, Some(&a), 25, 30.0, Some(200.0), true);
        assert_eq!(g, vec![(1, 50.0)]);
        // Trocar de faixa grava o que faltava.
        let b = faixa("B");
        tocar(&mut ac, &mut g, Some(&b), 5, 0.0, Some(200.0), true);
        assert_eq!(g, vec![(1, 55.0)], "a B ainda não vale");
    }

    #[test]
    fn pausa_e_pulo_rapido_nao_contam() {
        let (mut ac, mut g) = (Acompanhador::default(), Vec::new());
        let a = faixa("A");
        tocar(&mut ac, &mut g, Some(&a), 20, 0.0, Some(200.0), true);
        tocar(&mut ac, &mut g, Some(&a), 300, 20.0, Some(200.0), false);
        let b = faixa("B");
        tocar(&mut ac, &mut g, Some(&b), 5, 0.0, Some(200.0), true);
        tocar(&mut ac, &mut g, None, 5, 0.0, None, true);
        assert!(g.is_empty(), "20 s ouvidos e depois pausa: nada entra");
    }

    #[test]
    fn musica_curta_entra_pela_metade() {
        let (mut ac, mut g) = (Acompanhador::default(), Vec::new());
        let a = faixa("Vinheta");
        tocar(&mut ac, &mut g, Some(&a), 12, 0.0, Some(24.0), true);
        assert_eq!(g, vec![(1, 12.0)]);
        // Sem duração informada, só o meio minuto vale.
        let (mut ac, mut g) = (Acompanhador::default(), Vec::new());
        tocar(&mut ac, &mut g, Some(&a), 20, 0.0, None, true);
        assert!(g.is_empty());
    }

    #[test]
    fn repetir_a_mesma_musica_conta_de_novo() {
        let (mut ac, mut g) = (Acompanhador::default(), Vec::new());
        let a = faixa("A");
        tocar(&mut ac, &mut g, Some(&a), 100, 0.0, Some(100.0), true);
        tocar(&mut ac, &mut g, Some(&a), 40, 0.0, Some(100.0), true);
        assert_eq!(g.len(), 2, "duas execuções: {g:?}");
        assert_eq!(g[0], (1, 100.0));
        // Voltar um pouco a música não cria outra execução.
        let (mut ac, mut g) = (Acompanhador::default(), Vec::new());
        tocar(&mut ac, &mut g, Some(&a), 60, 0.0, Some(100.0), true);
        tocar(&mut ac, &mut g, Some(&a), 20, 30.0, Some(100.0), true);
        assert_eq!(g, vec![(1, 80.0)]);
    }

    #[test]
    fn medias_de_bpm_e_energia() {
        let mut ac = Acompanhador::default();
        let a = faixa("A");
        let mut ultima = Execucao::default();
        for (energia, bpm) in [(0.2, Some(100.0)), (0.8, None), (0.5, Some(140.0))] {
            for _ in 0..20 {
                let amostra = Amostra { faixa: Some(&a), tocando: true, posicao: None, com_som: true, energia, bpm };
                ac.passo(&amostra, 1.0, 0, &mut |t: &mut Tocada| ultima = t.exec.clone());
            }
        }
        assert_eq!(ultima.ouvido_s, 60.0);
        assert!((ultima.energia - 0.5).abs() < 0.01);
        assert!((ultima.bpm - 120.0).abs() < 0.01, "sem pulso firme não entra na média");
    }

    #[test]
    fn fuso_a_partir_das_duas_horas() {
        // 15:00 em Brasília, 18:00 UTC.
        assert_eq!(fuso_de((2026, 11, 12, 15 * 60), (2026, 11, 12, 18 * 60)), -3 * 3600);
        // 22:30 do dia 11 em Brasília, 01:30 do dia 12 em UTC.
        assert_eq!(fuso_de((2026, 11, 11, 22 * 60 + 30), (2026, 11, 12, 90)), -3 * 3600);
        // Virada do ano em Tóquio (+9) e fuso de meia hora (Índia, +5:30).
        assert_eq!(fuso_de((2027, 1, 1, 8 * 60), (2026, 12, 31, 23 * 60)), 9 * 3600);
        assert_eq!(fuso_de((2026, 11, 12, 17 * 60 + 30), (2026, 11, 12, 12 * 60)), 19_800);
        // As duas leituras caíram em minutos diferentes.
        assert_eq!(fuso_de((2026, 11, 12, 15 * 60 + 1), (2026, 11, 12, 18 * 60)), -3 * 3600);
    }

    #[test]
    fn periodos() {
        let (agora, fuso) = (1_794_506_400, -3 * 3600);
        assert_eq!(Periodo::Hoje.desde(agora, fuso), Some(agora - 15 * 3600));
        assert_eq!(Periodo::Semana.desde(agora, fuso), Some(agora - 7 * DIA_S));
        assert_eq!(Periodo::Tudo.desde(agora, fuso), None);
    }
}
