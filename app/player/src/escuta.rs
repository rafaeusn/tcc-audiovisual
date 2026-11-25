//! A thread do Modo Escuta: captura o som, roda a análise 60 vezes por segundo e publica um [`Retrato`]. A
//! interface só lê o retrato mais recente; nunca espera esta thread.
//!
//! A pessoa escolhe o que a thread ouve ([`Fonte`]): todo o som do PC ou só um app (o navegador passando um
//! anúncio não mexe no visual de quem escolheu o Spotify).
//!
//! A thread também cuida das falhas de áudio sozinha: sem dispositivo, dispositivo removido, saída trocada
//! (fone ↔ caixa) ou o app escolhido fechado, ela tenta de novo a cada 2 segundos e avisa a interface pela
//! [`Situacao`].

use analise::{FFT_PRINCIPAL, Motor, PASSO, Retrato};
use audio::{Anel, Captura, CapturaDeApp, Derivacao, Panorama, TAXA_DE_APP, nome_da_saida_padrao};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const NOVA_TENTATIVA: Duration = Duration::from_secs(2);
/// Sem amostras novas por este tempo, o passo é tratado como silêncio. O loopback do Windows para de entregar
/// amostras quando nada está tocando; sem isso, o último trecho ficaria "tocando" para sempre na análise.
const SEM_SOM: Duration = Duration::from_millis(80);

#[derive(Clone, Debug, PartialEq)]
pub enum Situacao {
    Iniciando,
    /// `dispositivo` é a saída de som (todo o PC) ou o nome do app ouvido.
    Ouvindo {
        dispositivo: String,
    },
    /// A pessoa escolheu ouvir um app que não está aberto.
    EsperandoApp {
        app: String,
    },
    SemAudio {
        motivo: String,
    },
}

/// O que o Modo Escuta ouve.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Fonte {
    /// Todo o som do PC, misturado.
    #[default]
    Tudo,
    /// Só o app deste executável (`spotify.exe`) e os processos que ele abre.
    App(String),
    /// Só a música que o próprio Player toca, antes do volume dela (a captura nem passa pelo Windows).
    Proprio,
    /// Só o programa que o Player abriu para tocar (o Soloist), desfazendo o volume dele.
    DoPlayer(String),
}

/// O que o Player empresta à escuta para ouvir a música dele.
struct DoPlayer {
    derivacao: Arc<Derivacao>,
    /// Quanto multiplicar o som do programa que toca por fora.
    ganho: Arc<AtomicU32>,
}

/// A captura em andamento.
enum Captando {
    Sistema(Captura),
    App(CapturaDeApp),
    Direto(Arc<Derivacao>),
}

impl Captando {
    fn falhou(&self) -> bool {
        match self {
            Self::Sistema(c) => c.falhou(),
            Self::App(c) => c.falhou(),
            Self::Direto(_) => false,
        }
    }
}

pub struct Escuta {
    retrato: Arc<Mutex<Retrato>>,
    situacao: Arc<Mutex<Situacao>>,
    fonte: Arc<Mutex<Fonte>>,
    /// Os apps (executáveis) que têm som aberto agora, para a pessoa escolher.
    apps: Arc<Mutex<Vec<String>>>,
    do_player: Arc<OnceLock<DoPlayer>>,
    parar: Arc<AtomicBool>,
    fio: Option<JoinHandle<()>>,
}

fn ler<T: Clone>(m: &Mutex<T>) -> T {
    // `lock` envenenado = a outra thread entrou em pânico. O último valor publicado continua válido.
    match m.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    }
}

fn publicar<T>(m: &Mutex<T>, valor: T) {
    match m.lock() {
        Ok(mut g) => *g = valor,
        Err(p) => *p.into_inner() = valor,
    }
}

/// Leitura do retrato a partir de outra thread (a do histórico).
#[derive(Clone)]
pub struct LeitorDeRetrato(Arc<Mutex<Retrato>>);

impl LeitorDeRetrato {
    pub fn retrato(&self) -> Retrato {
        ler(&self.0)
    }
}

/// O que a thread da escuta divide com o resto do app.
struct Ligacao {
    retrato: Arc<Mutex<Retrato>>,
    situacao: Arc<Mutex<Situacao>>,
    fonte: Arc<Mutex<Fonte>>,
    apps: Arc<Mutex<Vec<String>>>,
    do_player: Arc<OnceLock<DoPlayer>>,
    parar: Arc<AtomicBool>,
}

impl Escuta {
    pub fn leitor(&self) -> LeitorDeRetrato {
        LeitorDeRetrato(self.retrato.clone())
    }

    pub fn iniciar(fonte: Fonte) -> Self {
        let retrato = Arc::new(Mutex::new(Retrato::default()));
        let situacao = Arc::new(Mutex::new(Situacao::Iniciando));
        let fonte = Arc::new(Mutex::new(fonte));
        let apps = Arc::new(Mutex::new(Vec::new()));
        let parar = Arc::new(AtomicBool::new(false));
        let do_player = Arc::new(OnceLock::new());
        let ligacao = Ligacao {
            do_player: do_player.clone(),
            retrato: retrato.clone(),
            situacao: situacao.clone(),
            fonte: fonte.clone(),
            apps: apps.clone(),
            parar: parar.clone(),
        };
        let fio = match std::thread::Builder::new().name("escuta".into()).spawn(move || rodar(&ligacao)) {
            Ok(f) => Some(f),
            Err(e) => {
                log::error!("não foi possível iniciar a thread do Modo Escuta: {e}");
                publicar(&situacao, Situacao::SemAudio { motivo: "o sistema não permitiu iniciar a análise".into() });
                None
            }
        };
        Self { retrato, situacao, fonte, apps, do_player, parar, fio }
    }

    /// De onde vem a música do próprio Player ([`Fonte::Proprio`], [`Fonte::DoPlayer`]).
    pub fn ligar_ao_player(&self, derivacao: Arc<Derivacao>, ganho: Arc<AtomicU32>) {
        let _ = self.do_player.set(DoPlayer { derivacao, ganho });
    }

    pub fn retrato(&self) -> Retrato {
        ler(&self.retrato)
    }

    pub fn situacao(&self) -> Situacao {
        ler(&self.situacao)
    }

    /// Muda o que o Modo Escuta ouve. A troca acontece em poucos milissegundos, na thread da escuta.
    pub fn escolher(&self, fonte: Fonte) {
        publicar(&self.fonte, fonte);
    }

    pub fn apps(&self) -> Vec<String> {
        ler(&self.apps)
    }
}

impl Drop for Escuta {
    fn drop(&mut self) {
        self.parar.store(true, Ordering::Release);
        if let Some(f) = self.fio.take()
            && f.join().is_err()
        {
            log::error!("a thread do Modo Escuta terminou com erro");
        }
    }
}

/// O nome que aparece na tela para a fonte (`Chrome`, `o Player`).
fn nome_da_fonte(fonte: &Fonte) -> String {
    match fonte {
        Fonte::Tudo => "o PC".into(),
        Fonte::App(exe) | Fonte::DoPlayer(exe) => sessao::nome_do_app(exe),
        Fonte::Proprio => "o Player".into(),
    }
}

fn rodar(l: &Ligacao) {
    let (retrato, situacao, parar) = (&*l.retrato, &*l.situacao, &*l.parar);
    // Menos de meio segundo de som a 48 kHz: o bastante para a análise, nada que valha a pena guardar.
    let anel = Arc::new(Anel::new(16384));
    let mut captura: Option<Captando> = None;
    let mut motor = Motor::new(48000.0);
    let mut taxa = 48000u32;
    let mut amostras = vec![0.0f32; FFT_PRINCIPAL];
    let passo = Duration::from_secs_f32(PASSO);

    let mut proxima_tentativa = Instant::now();
    let mut proxima_checagem = Instant::now();
    let mut anterior = Instant::now();
    let (mut total_visto, mut ultima_chegada) = (0u64, Instant::now());
    let mut fonte_vista: Option<Fonte> = None;

    let mut trocar_taxa = |nova: u32, motor: &mut Motor| {
        if nova != taxa {
            taxa = nova;
            *motor = Motor::new(taxa as f32);
        }
    };

    while !parar.load(Ordering::Acquire) {
        let agora = Instant::now();
        let fonte = ler(&l.fonte);
        // A pessoa trocou o que quer ouvir: fecha a captura de agora e reabre na hora.
        if fonte_vista.as_ref() != Some(&fonte) {
            fonte_vista = Some(fonte.clone());
            captura = None;
            proxima_tentativa = agora;
            proxima_checagem = agora;
        }

        if captura.as_ref().is_some_and(Captando::falhou) {
            captura = None;
            let motivo = match fonte {
                Fonte::Tudo => "a saída de áudio foi desconectada",
                _ => "o app parou de entregar o som",
            };
            publicar(situacao, Situacao::SemAudio { motivo: motivo.into() });
            proxima_tentativa = agora + NOVA_TENTATIVA;
        }

        if agora >= proxima_checagem {
            proxima_checagem = agora + NOVA_TENTATIVA;
            let panorama = Panorama::ler();
            publicar(&l.apps, panorama.apps());
            match &fonte {
                // O usuário trocou a saída padrão (plugou um fone, por exemplo): passa a ouvir a nova.
                Fonte::Tudo => {
                    if let Some(Captando::Sistema(c)) = captura.as_ref()
                        && nome_da_saida_padrao().is_some_and(|nome| nome != c.dispositivo)
                    {
                        log::info!("saída padrão mudou; reabrindo a captura");
                        captura = None;
                        proxima_tentativa = agora;
                    }
                }
                Fonte::Proprio if l.do_player.get().is_some() => {}
                Fonte::App(_) | Fonte::Proprio | Fonte::DoPlayer(_) => {
                    let alvo = match &fonte {
                        Fonte::App(exe) => panorama.alvo(exe),
                        // (O do Player, e não o de outro programa que esteja aberto com o mesmo nome.)
                        Fonte::DoPlayer(exe) => panorama.do_player(exe).first().copied().or_else(|| panorama.alvo(exe)),
                        _ => Some(panorama.raiz_propria()),
                    };
                    let ganho = match (&fonte, l.do_player.get()) {
                        (Fonte::DoPlayer(_), Some(p)) => p.ganho.clone(),
                        _ => Arc::new(AtomicU32::new(1f32.to_bits())),
                    };
                    let nome = nome_da_fonte(&fonte);
                    match alvo {
                        None => {
                            captura = None;
                            publicar(situacao, Situacao::EsperandoApp { app: nome });
                        }
                        // Primeira vez, ou o app reabriu com outro número, ou passou a tocar noutro processo.
                        Some(pid)
                            if agora >= proxima_tentativa
                                && !matches!(&captura, Some(Captando::App(c)) if c.pid == pid) =>
                        {
                            captura = None;
                            match CapturaDeApp::abrir_com(anel.clone(), pid, ganho) {
                                Ok(c) => {
                                    trocar_taxa(TAXA_DE_APP, &mut motor);
                                    log::info!("Modo Escuta: ouvindo só {nome} (processo {pid})");
                                    publicar(situacao, Situacao::Ouvindo { dispositivo: nome });
                                    captura = Some(Captando::App(c));
                                }
                                Err(e) => {
                                    log::warn!("não foi possível ouvir só {nome}: {e}");
                                    publicar(
                                        situacao,
                                        Situacao::SemAudio {
                                            motivo: format!("o Windows não deixou ouvir só {nome}")
                                        },
                                    );
                                    proxima_tentativa = agora + NOVA_TENTATIVA;
                                }
                            }
                        }
                        Some(_) => {}
                    }
                }
            }
        }

        if fonte == Fonte::Tudo && captura.is_none() && agora >= proxima_tentativa {
            match Captura::abrir(anel.clone()) {
                Ok(c) => {
                    trocar_taxa(c.taxa, &mut motor);
                    publicar(situacao, Situacao::Ouvindo { dispositivo: c.dispositivo.clone() });
                    captura = Some(Captando::Sistema(c));
                }
                Err(e) => {
                    let nova = Situacao::SemAudio { motivo: e.to_string() };
                    if ler(situacao) != nova {
                        log::warn!("Modo Escuta indisponível: {e}");
                        publicar(situacao, nova);
                    }
                    proxima_tentativa = agora + NOVA_TENTATIVA;
                }
            }
        }

        if fonte == Fonte::Proprio
            && let Some(p) = l.do_player.get()
        {
            if !matches!(captura, Some(Captando::Direto(_))) {
                captura = Some(Captando::Direto(p.derivacao.clone()));
                publicar(situacao, Situacao::Ouvindo { dispositivo: nome_da_fonte(&fonte) });
            }
            trocar_taxa(p.derivacao.taxa(), &mut motor);
        }

        let anel = match &captura {
            Some(Captando::Direto(d)) => &d.anel,
            _ => &*anel,
        };
        let total = anel.total();
        if total != total_visto {
            total_visto = total;
            ultima_chegada = agora;
        }
        if captura.is_some() && agora.duration_since(ultima_chegada) < SEM_SOM {
            anel.ultimas(&mut amostras);
        } else {
            amostras.fill(0.0);
        }

        let dt = agora.duration_since(anterior).as_secs_f32();
        anterior = agora;
        publicar(retrato, motor.passo(dt, &amostras));

        if let Some(resto) = passo.checked_sub(agora.elapsed()) {
            std::thread::sleep(resto);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// A música do Player chega direto do tocador: nada do Windows no caminho, e o silêncio também é visto.
    #[test]
    fn a_musica_do_player_e_ouvida_direto() {
        let derivacao: Arc<Derivacao> = Arc::default();
        let escuta = Escuta::iniciar(Fonte::Proprio);
        escuta.ligar_ao_player(derivacao.clone(), Arc::new(AtomicU32::new(1f32.to_bits())));
        let mut n = 0u32;
        let mut tocar = |por: Duration, nivel: f32| {
            let ate = Instant::now() + por;
            while Instant::now() < ate {
                derivacao.anel.escrever((0..480).map(|_| {
                    n += 1;
                    (n as f32 * 0.02).sin() * nivel
                }));
                std::thread::sleep(Duration::from_millis(10));
            }
        };
        tocar(Duration::from_millis(1500), 0.5);
        assert_eq!(escuta.situacao(), Situacao::Ouvindo { dispositivo: "o Player".into() });
        let r = escuta.retrato();
        assert!(r.gate > 0.5 && r.espectro.iter().any(|v| *v > 60), "ouve: {} {:?}", r.gate, r.espectro.iter().max());
        // A saída parou de pedir som (a música foi parada): é silêncio, e não o último trecho para sempre.
        std::thread::sleep(Duration::from_millis(2500));
        let r = escuta.retrato();
        assert!(r.gate < 0.5, "silêncio: {}", r.gate);
    }
}
