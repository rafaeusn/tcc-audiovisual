//! Toca arquivos de música.
//!
//! Uma thread própria decodifica os arquivos ([`fonte`]), mistura ([`mistura`]: troca sem intervalo, crossfade,
//! ganho por faixa, equalizador) e mantém cheia a fila da saída de som ([`saida`]). Quem usa manda comandos
//! ([`Comando`]) e lê a situação ([`Situacao`]); nunca espera o disco nem a placa de som.
//!
//! A fila de músicas não é daqui: quem usa diz qual toca agora ([`Comando::Tocar`]) e qual vem depois
//! ([`Comando::Seguinte`]). Quando a seguinte entra sozinha, [`Situacao::avancou`] muda.
//!
//! O volume geral é o do sistema, como no resto do app. O visual não precisa de nada especial: ele ouve o que
//! sai do computador, e o que o tocador toca sai por lá.

pub mod fonte;
pub mod mistura;
pub mod saida;

use fonte::Fonte;
use mistura::{Faixa, Marco, Mistura};
use saida::Saida;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use mistura::{BANDAS, TETO_EQ_DB};

/// Quadros que a mistura produz por vez.
const BLOCO: usize = 512;
/// Sem saída de som (fone desligado, dispositivo trocado), tenta reabrir de quanto em quanto tempo.
const NOVA_TENTATIVA: Duration = Duration::from_secs(1);

/// O que estava tocando quando a saída de som caiu: reaberto na saída nova, do mesmo ponto.
struct Retomar {
    atual: (PathBuf, f32),
    seguinte: Option<(PathBuf, f32)>,
    posicao_s: f64,
    pausado: bool,
    tentar_em: std::time::Instant,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Comando {
    /// Toca este arquivo do começo, com este ganho (1 = como está no arquivo).
    Tocar {
        caminho: PathBuf,
        ganho: f32,
    },
    /// O arquivo que entra quando o de agora acabar (`None` = nenhum: a música para no fim).
    Seguinte(Option<(PathBuf, f32)>),
    Pausar,
    Continuar,
    /// Para e esquece os arquivos.
    Parar,
    /// Vai para este ponto da música, em segundos.
    Ir(f64),
    /// Crossfade (segundos, 0 = sem) e equalizador (dB por banda, na ordem de [`BANDAS`]).
    Mixagem {
        crossfade_s: f32,
        eq: Vec<f32>,
    },
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Situacao {
    /// O arquivo que está soando (tocando ou pausado).
    pub arquivo: Option<PathBuf>,
    pub tocando: bool,
    pub posicao_s: f64,
    /// 0 quando o arquivo não informa.
    pub duracao_s: f64,
    /// O arquivo já aberto para entrar depois deste.
    pub seguinte: Option<PathBuf>,
    /// Quantas vezes a seguinte entrou sozinha (sem intervalo ou por crossfade).
    pub avancou: u64,
    /// Quantas vezes a música acabou sem nada depois (ou o arquivo não abriu). Quem cuida da fila decide o
    /// que fazer quando este número muda.
    pub terminadas: u64,
    /// A última falha (arquivo ilegível, sem saída de som), em texto para a pessoa.
    pub erro: Option<String>,
}

/// Ligação com a thread do tocador. Soltar o valor encerra a thread e o som.
pub struct Tocador {
    comandos: Option<Sender<Comando>>,
    situacao: Arc<Mutex<Situacao>>,
}

fn com<T>(m: &Mutex<Situacao>, f: impl FnOnce(&mut Situacao) -> T) -> T {
    match m.lock() {
        Ok(mut g) => f(&mut g),
        Err(p) => f(&mut p.into_inner()),
    }
}

impl Tocador {
    /// `mudo`: toca sem som (a música anda, a posição conta, nada sai pela placa). Serve para testar.
    pub fn iniciar(mudo: bool) -> Self {
        let situacao = Arc::new(Mutex::new(Situacao::default()));
        let (comandos, fila) = channel::<Comando>();
        let s = situacao.clone();
        let criada = std::thread::Builder::new().name("tocador".into()).spawn(move || rodar(&fila, &s, mudo));
        match criada {
            Ok(_) => Self { comandos: Some(comandos), situacao },
            Err(e) => {
                log::error!("não foi possível iniciar o tocador: {e}");
                Self { comandos: None, situacao }
            }
        }
    }

    pub fn mandar(&self, comando: Comando) {
        if let Some(c) = &self.comandos {
            let _ = c.send(comando);
        }
    }

    pub fn situacao(&self) -> Situacao {
        com(&self.situacao, |s| s.clone())
    }
}

fn abrir_faixa(caminho: &Path, ganho: f32, taxa: u32) -> nucleo::Resultado<Faixa> {
    Fonte::abrir(caminho, taxa).map(|f| Faixa::nova(caminho.to_path_buf(), Box::new(f), ganho, taxa))
}

/// O estado da thread do tocador.
struct Motor {
    saida: Option<Saida>,
    mistura: Option<Mistura>,
    mudo: bool,
    crossfade_s: f32,
    eq: Vec<f32>,
    /// Quadros postos na fila da placa desde o último descarte (mesmo marco zero de `painel.tocados`).
    empurrados: u64,
    /// Os marcos ainda não ouvidos, com o quadro (na contagem acima) em que acontecem.
    marcos: VecDeque<(u64, Marco)>,
    /// Onde a música de agora começou a soar (na mesma contagem) e de que segundo dela.
    inicio: u64,
    base_s: f64,
    pedaco: Vec<f32>,
    aqui: Vec<(usize, Marco)>,
    retomar: Option<Retomar>,
}

impl Motor {
    /// A saída caiu: guarda o que tocava para reabrir depois.
    fn saida_caiu(&mut self, situacao: &Mutex<Situacao>) {
        let pausado = self.saida.as_ref().is_some_and(|s| s.painel.pausado.load(Ordering::Acquire));
        let (atual, seguinte) = self.mistura.as_ref().map(Mistura::para_retomar).unwrap_or_default();
        self.saida = None;
        self.mistura = None;
        self.marcos.clear();
        let posicao_s = com(situacao, |s| s.posicao_s);
        self.retomar =
            atual.map(|atual| Retomar { atual, seguinte, posicao_s, pausado, tentar_em: std::time::Instant::now() });
        com(situacao, |s| {
            s.tocando = false;
            s.erro = Some("A saída de som mudou. Retomando na nova...".into());
        });
    }

    /// Tenta reabrir a saída e a música que tocava, do mesmo ponto. Sem sucesso, tenta de novo daqui a pouco.
    fn tentar_retomar(&mut self, situacao: &Mutex<Situacao>) {
        let Some(r) = self.retomar.as_mut() else { return };
        if std::time::Instant::now() < r.tentar_em {
            return;
        }
        r.tentar_em = std::time::Instant::now() + NOVA_TENTATIVA;
        if !self.garantir_saida() {
            return;
        }
        let Some(r) = self.retomar.take() else { return };
        let taxa = self.saida.as_ref().map_or(48_000, |s| s.taxa);
        match abrir_faixa(&r.atual.0, r.atual.1, taxa) {
            Ok(faixa) => {
                let duracao_s = faixa.duracao_s();
                if let Some(s) = &self.saida {
                    s.painel.mudo.store(self.mudo, Ordering::Release);
                    s.painel.pausado.store(r.pausado, Ordering::Release);
                }
                let mut onde = 0.0;
                if let Some(mix) = self.mistura.as_mut() {
                    mix.tocar(faixa);
                    onde = mix.ir(r.posicao_s).unwrap_or(0.0);
                    mix.preparar(r.seguinte.as_ref().and_then(|(c, g)| abrir_faixa(c, *g, taxa).ok()));
                }
                self.descartar(onde);
                log::info!("saída de som reaberta; a música continua em {onde:.1}s");
                com(situacao, |s| {
                    s.arquivo = Some(r.atual.0.clone());
                    s.duracao_s = duracao_s;
                    s.posicao_s = onde;
                    s.tocando = !r.pausado;
                    s.erro = None;
                });
            }
            Err(e) => {
                // O arquivo sumiu no meio do caminho: a fila segue para a próxima.
                log::warn!("não foi possível reabrir a música depois da troca de saída: {e}");
                com(situacao, |s| {
                    s.terminadas += 1;
                    s.erro = Some(format!("Não foi possível retomar a música ({e})."));
                });
            }
        }
    }

    /// Abre a saída de som (e a mistura na taxa dela) se ainda não estiver aberta.
    fn garantir_saida(&mut self) -> bool {
        if self.saida.is_none() {
            self.saida = Saida::abrir().map_err(|e| log::warn!("sem saída de som: {e}")).ok();
            self.mistura = self.saida.as_ref().map(|s| {
                let mut m = Mistura::nova(s.taxa);
                m.ajustar_crossfade(self.crossfade_s);
                m.eq.ajustar(&self.eq);
                m
            });
        }
        self.saida.is_some()
    }

    /// Joga fora o que está na fila da placa e zera as contagens.
    fn descartar(&mut self, base_s: f64) {
        if let Some(s) = &self.saida {
            s.painel.descartar.store(true, Ordering::Release);
            s.painel.tocados.store(0, Ordering::Release);
        }
        self.empurrados = 0;
        self.inicio = 0;
        self.base_s = base_s;
        self.marcos.clear();
    }
}

fn rodar(fila: &std::sync::mpsc::Receiver<Comando>, situacao: &Mutex<Situacao>, mudo: bool) {
    let mut m = Motor {
        saida: None,
        mistura: None,
        mudo,
        crossfade_s: 0.0,
        eq: Vec::new(),
        empurrados: 0,
        marcos: VecDeque::new(),
        inicio: 0,
        base_s: 0.0,
        pedaco: Vec::with_capacity(BLOCO * 2),
        aqui: Vec::new(),
        retomar: None,
    };

    loop {
        let comando = match fila.recv_timeout(Duration::from_millis(8)) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        // A saída caiu (fone desligado, dispositivo trocado): a música continua na saída nova, do mesmo ponto.
        if m.saida.as_ref().is_some_and(|s| s.painel.falhou.load(Ordering::Acquire)) {
            m.saida_caiu(situacao);
        }
        if let Some(c) = comando {
            // Um pedido novo vale mais que retomar o que tocava; pausar e continuar só mudam como volta.
            match &c {
                Comando::Pausar | Comando::Continuar if m.retomar.is_some() => {
                    if let Some(r) = m.retomar.as_mut() {
                        r.pausado = c == Comando::Pausar;
                    }
                }
                Comando::Seguinte(pedido) if m.retomar.is_some() => {
                    if let Some(r) = m.retomar.as_mut() {
                        r.seguinte.clone_from(pedido);
                    }
                }
                Comando::Ir(segundos) if m.retomar.is_some() => {
                    if let Some(r) = m.retomar.as_mut() {
                        r.posicao_s = *segundos;
                    }
                }
                Comando::Tocar { .. } | Comando::Parar => {
                    m.retomar = None;
                    atender(&mut m, c, situacao);
                }
                _ => atender(&mut m, c, situacao),
            }
        }
        m.tentar_retomar(situacao);

        let (Some(s), Some(mix)) = (m.saida.as_mut(), m.mistura.as_mut()) else { continue };
        // Mantém a fila da placa cheia. Enquanto um descarte está pendente, nada novo entra.
        while !s.painel.descartar.load(Ordering::Acquire) && !mix.vazia() && s.fila.slots() >= BLOCO * 2 {
            m.pedaco.clear();
            m.aqui.clear();
            let k = mix.produzir(BLOCO, &mut m.pedaco, &mut m.aqui);
            for v in m.pedaco.drain(..) {
                let _ = s.fila.push(v);
            }
            for (q, marco) in m.aqui.drain(..) {
                m.marcos.push_back((m.empurrados + q as u64, marco));
            }
            m.empurrados += k as u64;
        }
        let taxa = f64::from(s.taxa);
        let tocados = s.painel.tocados.load(Ordering::Acquire);
        // Os marcos que já soaram: a música mudou, ou tudo acabou.
        while m.marcos.front().is_some_and(|(q, _)| *q <= tocados) {
            let Some((q, marco)) = m.marcos.pop_front() else { break };
            match marco {
                Marco::Comecou { caminho, duracao_s } => {
                    m.inicio = q;
                    m.base_s = 0.0;
                    com(situacao, |e| {
                        e.arquivo = Some(caminho);
                        e.duracao_s = duracao_s;
                        e.posicao_s = 0.0;
                        e.seguinte = None;
                        e.avancou += 1;
                    });
                }
                Marco::Fim => com(situacao, |e| {
                    e.tocando = false;
                    e.posicao_s = e.duracao_s.max(e.posicao_s);
                    e.seguinte = None;
                    e.terminadas += 1;
                }),
            }
        }
        if !mix.vazia() {
            let posicao = m.base_s + tocados.saturating_sub(m.inicio) as f64 / taxa;
            com(situacao, |e| e.posicao_s = posicao);
        }
    }
}

fn atender(m: &mut Motor, comando: Comando, situacao: &Mutex<Situacao>) {
    match comando {
        Comando::Tocar { caminho, ganho } => {
            let aberta =
                if m.garantir_saida() { m.saida.as_ref().map(|s| abrir_faixa(&caminho, ganho, s.taxa)) } else { None };
            match aberta {
                Some(Ok(faixa)) => {
                    let duracao_s = faixa.duracao_s();
                    if let Some(s) = &m.saida {
                        s.painel.mudo.store(m.mudo, Ordering::Release);
                        s.painel.pausado.store(false, Ordering::Release);
                    }
                    m.descartar(0.0);
                    if let Some(mix) = m.mistura.as_mut() {
                        mix.tocar(faixa);
                    }
                    com(situacao, |e| {
                        *e = Situacao {
                            arquivo: Some(caminho),
                            tocando: true,
                            duracao_s,
                            avancou: e.avancou,
                            terminadas: e.terminadas,
                            ..Default::default()
                        };
                    });
                }
                falha => {
                    if let Some(mix) = m.mistura.as_mut() {
                        mix.parar();
                    }
                    m.descartar(0.0);
                    let motivo = match falha {
                        Some(Err(e)) => format!("Não foi possível tocar este arquivo ({e})."),
                        _ => "Não há saída de som para tocar.".to_string(),
                    };
                    log::warn!("{motivo} {}", caminho.display());
                    com(situacao, |e| {
                        // A fila segue para a próxima: um arquivo ruim não trava a lista.
                        *e = Situacao {
                            avancou: e.avancou,
                            terminadas: e.terminadas + 1,
                            erro: Some(motivo),
                            ..Default::default()
                        };
                    });
                }
            }
        }
        Comando::Seguinte(pedido) => {
            let Some(mix) = m.mistura.as_mut() else { return };
            match pedido {
                Some((caminho, _)) if mix.seguinte() == Some(&caminho) => {}
                Some((caminho, ganho)) => match abrir_faixa(&caminho, ganho, mix.taxa()) {
                    Ok(faixa) => {
                        mix.preparar(Some(faixa));
                        com(situacao, |e| e.seguinte = Some(caminho));
                    }
                    Err(e) => {
                        // Fica sem seguinte: no fim desta, quem cuida da fila tenta a próxima e pula esta.
                        log::warn!("a próxima música não abriu ({e}): {}", caminho.display());
                        mix.preparar(None);
                        com(situacao, |e| e.seguinte = None);
                    }
                },
                None => {
                    mix.preparar(None);
                    com(situacao, |e| e.seguinte = None);
                }
            }
        }
        Comando::Pausar | Comando::Continuar => {
            if m.mistura.as_ref().is_none_or(Mistura::vazia) {
                return;
            }
            let pausar = comando == Comando::Pausar;
            if let Some(s) = &m.saida {
                s.painel.pausado.store(pausar, Ordering::Release);
            }
            com(situacao, |e| e.tocando = !pausar);
        }
        Comando::Parar => {
            if let Some(mix) = m.mistura.as_mut() {
                mix.parar();
            }
            m.descartar(0.0);
            com(situacao, |e| *e = Situacao { avancou: e.avancou, terminadas: e.terminadas, ..Default::default() });
        }
        Comando::Ir(segundos) => {
            if let Some(onde) = m.mistura.as_mut().and_then(|mix| mix.ir(segundos)) {
                m.descartar(onde);
                com(situacao, |e| e.posicao_s = onde);
            }
        }
        Comando::Mixagem { crossfade_s, eq } => {
            m.crossfade_s = crossfade_s;
            m.eq = eq;
            if let Some(mix) = m.mistura.as_mut() {
                mix.ajustar_crossfade(m.crossfade_s);
                mix.eq.ajustar(&m.eq);
            }
        }
    }
}
