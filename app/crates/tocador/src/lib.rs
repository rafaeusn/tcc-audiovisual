//! Toca música: arquivos do computador e endereços na internet (os que os provedores entregam).
//!
//! Uma thread própria decodifica ([`fonte`]; da internet, aos pedaços: [`rede`]), mistura ([`mistura`]: troca sem
//! intervalo, crossfade, ganho por faixa, equalizador) e mantém cheia a fila da saída de som ([`saida`]). Quem usa
//! manda comandos ([`Comando`]) e lê a situação ([`Situacao`]); nunca espera o disco, a rede nem a placa de som.
//!
//! A fila de músicas não é daqui: quem usa diz qual toca agora ([`Comando::Tocar`]) e qual vem depois
//! ([`Comando::Seguinte`]). Quando a seguinte entra sozinha, [`Situacao::avancou`] muda.
//!
//! O volume ([`Comando::Volume`]) é só da música e vem por último, na saída. O visual ouve a música antes dele
//! ([`Tocador::derivacao`]): baixar o som não o enfraquece.

pub mod fonte;
pub mod mistura;
pub mod rede;
pub mod saida;

use fonte::Fonte;
use mistura::{Faixa, Marco, Mistura};
use saida::Saida;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub use mistura::{BANDAS, TETO_EQ_DB};
pub use rede::Endereco;

/// De onde vem uma música: um arquivo do computador ou um endereço na internet (de um provedor).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Origem {
    Arquivo(PathBuf),
    Endereco(Endereco),
}

impl Origem {
    /// O caminho, quando é um arquivo.
    pub fn caminho(&self) -> Option<&Path> {
        match self {
            Self::Arquivo(c) => Some(c),
            Self::Endereco(_) => None,
        }
    }
}

impl From<PathBuf> for Origem {
    fn from(c: PathBuf) -> Self {
        Self::Arquivo(c)
    }
}

impl From<&Path> for Origem {
    fn from(c: &Path) -> Self {
        Self::Arquivo(c.to_path_buf())
    }
}

impl From<&str> for Origem {
    fn from(c: &str) -> Self {
        Self::Arquivo(PathBuf::from(c))
    }
}

impl std::fmt::Display for Origem {
    /// Para o registro: o caminho do arquivo, ou o endereço sem a parte que leva assinaturas e chaves.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Arquivo(c) => write!(f, "{}", c.display()),
            Self::Endereco(e) => f.write_str(e.para_registro()),
        }
    }
}

/// Quadros que a mistura produz por vez.
const BLOCO: usize = 512;
/// Sem saída de som (fone desligado, dispositivo trocado), tenta reabrir de quanto em quanto tempo.
const NOVA_TENTATIVA: Duration = Duration::from_secs(1);

/// O que estava tocando quando a saída de som caiu: reaberto na saída nova, do mesmo ponto.
struct Retomar {
    atual: (Origem, f32),
    seguinte: Option<(Origem, f32)>,
    posicao_s: f64,
    pausado: bool,
    tentar_em: std::time::Instant,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Comando {
    /// Toca esta música do começo, com este ganho (1 = como está no arquivo).
    Tocar {
        origem: Origem,
        ganho: f32,
    },
    /// A música que entra quando a de agora acabar (`None` = nenhuma: a música para no fim). Da internet, ela
    /// abre em segundo plano, para a de agora não engasgar.
    Seguinte(Option<(Origem, f32)>),
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
    /// O ganho do volume da música (0 a 1).
    Volume(f32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Situacao {
    /// A música que está soando (tocando ou pausada).
    pub origem: Option<Origem>,
    pub tocando: bool,
    pub posicao_s: f64,
    /// 0 quando o arquivo não informa.
    pub duracao_s: f64,
    /// A música já aberta para entrar depois desta.
    pub seguinte: Option<Origem>,
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
    derivacao: Arc<audio::Derivacao>,
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
        let derivacao: Arc<audio::Derivacao> = Arc::default();
        let (s, d) = (situacao.clone(), derivacao.clone());
        let criada = std::thread::Builder::new().name("tocador".into()).spawn(move || rodar(&fila, &s, mudo, d));
        match criada {
            Ok(_) => Self { comandos: Some(comandos), situacao, derivacao },
            Err(e) => {
                log::error!("não foi possível iniciar o tocador: {e}");
                Self { comandos: None, situacao, derivacao }
            }
        }
    }

    /// O que está tocando, antes do volume: para o visual.
    pub fn derivacao(&self) -> Arc<audio::Derivacao> {
        self.derivacao.clone()
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

fn abrir_faixa(origem: &Origem, ganho: f32, taxa: u32) -> nucleo::Resultado<Faixa> {
    Fonte::abrir_origem(origem, taxa).map(|f| Faixa::nova(origem.clone(), Box::new(f), ganho, taxa))
}

/// O estado da thread do tocador.
struct Motor {
    saida: Option<Saida>,
    mistura: Option<Mistura>,
    mudo: bool,
    volume: f32,
    derivacao: Arc<audio::Derivacao>,
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
    /// A seguinte da internet, abrindo em segundo plano.
    abrindo: Option<(Origem, Receiver<nucleo::Resultado<Faixa>>)>,
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
                    s.origem = Some(r.atual.0.clone());
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
            self.saida = Saida::abrir(self.volume, Some(self.derivacao.clone()))
                .map_err(|e| log::warn!("sem saída de som: {e}"))
                .ok();
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

fn rodar(
    fila: &std::sync::mpsc::Receiver<Comando>,
    situacao: &Mutex<Situacao>,
    mudo: bool,
    derivacao: Arc<audio::Derivacao>,
) {
    let mut m = Motor {
        saida: None,
        mistura: None,
        mudo,
        volume: 1.0,
        derivacao,
        crossfade_s: 0.0,
        eq: Vec::new(),
        empurrados: 0,
        marcos: VecDeque::new(),
        inicio: 0,
        base_s: 0.0,
        pedaco: Vec::with_capacity(BLOCO * 2),
        aqui: Vec::new(),
        retomar: None,
        abrindo: None,
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
        receber_a_seguinte(&mut m, situacao);

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
                Marco::Comecou { origem, duracao_s } => {
                    m.inicio = q;
                    m.base_s = 0.0;
                    com(situacao, |e| {
                        e.origem = Some(origem);
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

/// A seguinte da internet terminou de abrir (ou falhou): entra na mistura.
fn receber_a_seguinte(m: &mut Motor, situacao: &Mutex<Situacao>) {
    let Some((origem, chegada)) = &m.abrindo else { return };
    let resultado = match chegada.try_recv() {
        Ok(r) => r,
        Err(TryRecvError::Empty) => return,
        Err(TryRecvError::Disconnected) => Err(nucleo::Erro::Audio("a abertura parou no meio".into())),
    };
    let origem = origem.clone();
    m.abrindo = None;
    let Some(mix) = m.mistura.as_mut() else { return };
    match resultado {
        Ok(faixa) => {
            mix.preparar(Some(faixa));
            com(situacao, |e| e.seguinte = Some(origem));
        }
        Err(e) => {
            log::warn!("a próxima música não abriu ({e}): {origem}");
            mix.preparar(None);
            com(situacao, |e| e.seguinte = None);
        }
    }
}

fn atender(m: &mut Motor, comando: Comando, situacao: &Mutex<Situacao>) {
    if matches!(comando, Comando::Tocar { .. } | Comando::Parar | Comando::Seguinte(_)) {
        // Um pedido novo vale mais que a seguinte que ainda estava abrindo.
        m.abrindo = None;
    }
    match comando {
        Comando::Tocar { origem, ganho } => {
            let aberta =
                if m.garantir_saida() { m.saida.as_ref().map(|s| abrir_faixa(&origem, ganho, s.taxa)) } else { None };
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
                            origem: Some(origem),
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
                    log::warn!("{motivo} {origem}");
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
                Some((origem, _)) if mix.seguinte() == Some(&origem) => {}
                Some((origem @ Origem::Endereco(_), ganho)) => {
                    // Da internet, abre em segundo plano: a de agora continua tocando enquanto isso.
                    let (avisar, chegada) = channel();
                    let (o, taxa) = (origem.clone(), mix.taxa());
                    let criada = std::thread::Builder::new()
                        .name("tocador-abre".into())
                        .spawn(move || drop(avisar.send(abrir_faixa(&o, ganho, taxa))));
                    match criada {
                        Ok(_) => m.abrindo = Some((origem, chegada)),
                        Err(e) => log::warn!("não foi possível abrir a próxima música: {e}"),
                    }
                    mix.preparar(None);
                    com(situacao, |e| e.seguinte = None);
                }
                Some((origem, ganho)) => match abrir_faixa(&origem, ganho, mix.taxa()) {
                    Ok(faixa) => {
                        mix.preparar(Some(faixa));
                        com(situacao, |e| e.seguinte = Some(origem));
                    }
                    Err(e) => {
                        // Fica sem seguinte: no fim desta, quem cuida da fila tenta a próxima e pula esta.
                        log::warn!("a próxima música não abriu ({e}): {origem}");
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
        Comando::Volume(v) => {
            m.volume = if v.is_finite() { v.clamp(0.0, 1.0) } else { 1.0 };
            if let Some(saida) = &m.saida {
                saida.painel.volume.store(m.volume.to_bits(), Ordering::Release);
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
