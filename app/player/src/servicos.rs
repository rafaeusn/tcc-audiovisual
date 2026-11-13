//! As threads dos módulos de serviço: Last.fm (scrobble) e Discord (cartão "ouvindo").
//!
//! Cada uma lê sozinha o estado da mídia e o retrato do som, uma ou duas vezes por segundo, e só age se o módulo
//! dela está instalado e ligado (a interface avisa por [`Servicos::ativar_lastfm`] e [`Servicos::ativar_discord`]).
//! Tudo é "melhor esforço": sem internet, sem conta ou sem o Discord aberto, o resto do app segue igual. A
//! interface só lê a situação de cada uma; nunca espera a rede.

use crate::escuta::LeitorDeRetrato;
use servicos::lastfm::{Cliente, Conta, Falha};
use servicos::marcador::{Amostra, Evento, Marcador};
use servicos::{Chaves, Faixa, cofre, discord, lastfm};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Quanto tempo a pessoa tem para aprovar o app na página do Last.fm.
const PRAZO_DA_AUTORIZACAO: Duration = Duration::from_secs(180);
const CONSULTA_DA_AUTORIZACAO: Duration = Duration::from_secs(3);
/// Scrobbles que não foram por falta de internet: quantos ficam guardados e de quanto em quanto se tenta.
const MAXIMO_DE_PENDENTES: usize = 50;
const NOVA_TENTATIVA: Duration = Duration::from_secs(60);
/// O Discord descarta atualizações muito próximas: o cartão é conferido neste ritmo.
const PASSO_DO_DISCORD: Duration = Duration::from_secs(2);
const NOVA_LIGACAO_AO_DISCORD: Duration = Duration::from_secs(10);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum SituacaoLastfm {
    /// O app foi compilado sem as chaves do Last.fm.
    SemChave,
    #[default]
    Desconectada,
    /// A página de autorização está aberta no navegador.
    Autorizando,
    Conectada(String),
    Falha(String),
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SituacaoDiscord {
    /// O app foi compilado sem o identificador do aplicativo no Discord.
    SemChave,
    #[default]
    Parado,
    /// O Discord não está aberto neste computador.
    Esperando,
    Ligado,
}

enum Pedido {
    Conectar,
    Desconectar,
}

pub struct Servicos {
    lastfm_ativo: Arc<AtomicBool>,
    discord_ativo: Arc<AtomicBool>,
    lastfm: Arc<Mutex<SituacaoLastfm>>,
    discord: Arc<Mutex<SituacaoDiscord>>,
    pedidos: Option<Sender<Pedido>>,
}

fn ler<T: Clone>(m: &Mutex<T>) -> T {
    match m.lock() {
        Ok(g) => g.clone(),
        Err(p) => p.into_inner().clone(),
    }
}

fn trocar<T>(m: &Mutex<T>, valor: T) {
    match m.lock() {
        Ok(mut g) => *g = valor,
        Err(p) => *p.into_inner() = valor,
    }
}

fn agora_ms() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0)
}

/// O que as duas threads precisam saber da reprodução.
fn amostra(midia: &sessao::Leitor, som: &LeitorDeRetrato) -> Amostra {
    let estado = midia.estado();
    Amostra {
        faixa: estado.faixa.as_ref().map(|f| Faixa {
            titulo: f.titulo.clone(),
            artista: f.artista.clone(),
            album: f.album.clone(),
        }),
        tocando: estado.tocando,
        com_som: som.retrato().gate > 0.5,
        posicao: estado.posicao.map(|p| (p.agora(estado.tocando), p.duracao_s)),
    }
}

/// Abre um endereço no navegador padrão.
fn abrir_no_navegador(endereco: &str) {
    #[cfg(windows)]
    let aberto = std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", endereco]).spawn();
    #[cfg(not(windows))]
    let aberto = std::process::Command::new("xdg-open").arg(endereco).spawn();
    if let Err(e) = aberto {
        log::warn!("não foi possível abrir o navegador: {e}");
    }
}

impl Servicos {
    /// `pasta` é a pasta de dados (onde a autorização do Last.fm fica guardada, cifrada). Sem ela o Last.fm
    /// funciona, mas pede a autorização de novo a cada abertura.
    pub fn iniciar(pasta: Option<PathBuf>, midia: sessao::Leitor, som: LeitorDeRetrato) -> Self {
        let chaves = Chaves::ler();
        let lastfm_ativo = Arc::new(AtomicBool::new(false));
        let discord_ativo = Arc::new(AtomicBool::new(false));
        let situacao_lastfm = Arc::new(Mutex::new(match chaves.lastfm {
            Some(_) => SituacaoLastfm::Desconectada,
            None => SituacaoLastfm::SemChave,
        }));
        let situacao_discord = Arc::new(Mutex::new(match chaves.discord {
            Some(_) => SituacaoDiscord::Parado,
            None => SituacaoDiscord::SemChave,
        }));

        let mut pedidos = None;
        if let Some((chave, segredo)) = chaves.lastfm {
            let (envio, fila) = channel::<Pedido>();
            let (ativo, situacao) = (lastfm_ativo.clone(), situacao_lastfm.clone());
            let (midia, som) = (midia.clone(), som.clone());
            let cofre = pasta.map(|p| p.join("lastfm.cofre"));
            let criada = std::thread::Builder::new().name("lastfm".into()).spawn(move || {
                rodar_lastfm(&chave, &segredo, cofre, &ativo, &situacao, &fila, &midia, &som);
            });
            match criada {
                Ok(_) => pedidos = Some(envio),
                Err(e) => log::warn!("Last.fm desligado: {e}"),
            }
        }
        if let Some(id) = chaves.discord {
            let (ativo, situacao) = (discord_ativo.clone(), situacao_discord.clone());
            let criada = std::thread::Builder::new()
                .name("discord".into())
                .spawn(move || rodar_discord(&id, &ativo, &situacao, &midia, &som));
            if let Err(e) = criada {
                log::warn!("Discord desligado: {e}");
            }
        }
        Self { lastfm_ativo, discord_ativo, lastfm: situacao_lastfm, discord: situacao_discord, pedidos }
    }

    pub fn ativar_lastfm(&self, ativo: bool) {
        self.lastfm_ativo.store(ativo, Ordering::Release);
    }

    pub fn ativar_discord(&self, ativo: bool) {
        self.discord_ativo.store(ativo, Ordering::Release);
    }

    pub fn lastfm(&self) -> SituacaoLastfm {
        ler(&self.lastfm)
    }

    pub fn discord(&self) -> SituacaoDiscord {
        ler(&self.discord)
    }

    /// Começa a autorização da conta do Last.fm (abre o navegador).
    pub fn conectar_lastfm(&self) {
        if let Some(p) = &self.pedidos {
            let _ = p.send(Pedido::Conectar);
        }
    }

    pub fn desconectar_lastfm(&self) {
        if let Some(p) = &self.pedidos {
            let _ = p.send(Pedido::Desconectar);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rodar_lastfm(
    chave: &str,
    segredo: &str,
    arquivo: Option<PathBuf>,
    ativo: &AtomicBool,
    situacao: &Mutex<SituacaoLastfm>,
    fila: &std::sync::mpsc::Receiver<Pedido>,
    midia: &sessao::Leitor,
    som: &LeitorDeRetrato,
) {
    let cliente = match Cliente::novo(chave, segredo) {
        Ok(c) => c,
        Err(e) => {
            log::warn!("Last.fm indisponível: {e}");
            trocar(situacao, SituacaoLastfm::Falha("não foi possível preparar a conexão".into()));
            return;
        }
    };
    let mut conta: Option<Conta> = arquivo.as_deref().and_then(cofre::ler).and_then(|t| Conta::de_texto(&t));
    if let Some(c) = &conta {
        trocar(situacao, SituacaoLastfm::Conectada(c.usuario.clone()));
    }
    // (código, prazo, próxima consulta) enquanto a pessoa aprova no navegador.
    let mut autorizando: Option<(String, Instant, Instant)> = None;
    let mut pendentes: VecDeque<(Faixa, i64, Option<f64>)> = VecDeque::new();
    let mut proxima_tentativa = Instant::now();
    let mut marcador = Marcador::default();
    let mut anterior = Instant::now();

    let esquecer = |conta: &mut Option<Conta>| {
        *conta = None;
        if let Some(a) = arquivo.as_deref() {
            cofre::apagar(a);
        }
    };

    loop {
        match fila.recv_timeout(Duration::from_secs(1)) {
            Ok(Pedido::Conectar) => match cliente.codigo() {
                Ok(codigo) => {
                    abrir_no_navegador(&lastfm::pagina_de_autorizacao(cliente.chave(), &codigo));
                    let agora = Instant::now();
                    autorizando = Some((codigo, agora + PRAZO_DA_AUTORIZACAO, agora + CONSULTA_DA_AUTORIZACAO));
                    trocar(situacao, SituacaoLastfm::Autorizando);
                }
                Err(e) => {
                    log::debug!("Last.fm: não foi possível começar a autorização: {e:?}");
                    trocar(situacao, SituacaoLastfm::Falha("sem conexão com o Last.fm".into()));
                }
            },
            Ok(Pedido::Desconectar) => {
                esquecer(&mut conta);
                autorizando = None;
                pendentes.clear();
                trocar(situacao, SituacaoLastfm::Desconectada);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }

        if let Some((codigo, prazo, proxima)) = autorizando.clone()
            && Instant::now() >= proxima
        {
            match cliente.conta(&codigo) {
                Ok(nova) => {
                    if let Some(a) = arquivo.as_deref()
                        && let Err(e) = cofre::guardar(a, &nova.para_texto())
                    {
                        log::warn!("a autorização do Last.fm não foi guardada: {e}");
                    }
                    trocar(situacao, SituacaoLastfm::Conectada(nova.usuario.clone()));
                    conta = Some(nova);
                    autorizando = None;
                }
                Err(Falha::Aguardando) if Instant::now() < prazo => {
                    autorizando = Some((codigo, prazo, Instant::now() + CONSULTA_DA_AUTORIZACAO));
                }
                Err(Falha::Aguardando) => {
                    autorizando = None;
                    trocar(situacao, SituacaoLastfm::Falha("a autorização não foi aprovada a tempo".into()));
                }
                Err(e) => {
                    log::debug!("Last.fm: autorização falhou: {e:?}");
                    autorizando = None;
                    trocar(situacao, SituacaoLastfm::Falha("o Last.fm não concluiu a autorização".into()));
                }
            }
        }

        let dt = anterior.elapsed().as_secs_f64().min(3.0);
        anterior = Instant::now();
        let agora = agora_ms() / 1000;
        let eventos = marcador.passo(&amostra(midia, som), dt, agora);
        let (true, Some(c)) = (ativo.load(Ordering::Acquire), conta.clone()) else { continue };

        let mut sessao_invalida = false;
        for evento in eventos {
            match evento {
                Evento::Comecou { faixa, duracao_s } if lastfm::pode_enviar(&faixa) => {
                    // O "tocando agora" não é guardado para depois: se falhar, a próxima música avisa de novo.
                    match cliente.tocando_agora(&c, &faixa, duracao_s) {
                        Err(Falha::SessaoInvalida) => sessao_invalida = true,
                        Err(e) => log::debug!("Last.fm: tocando agora falhou: {e:?}"),
                        Ok(()) => {}
                    }
                }
                Evento::Contou { faixa, inicio, duracao_s } if lastfm::pode_enviar(&faixa) => {
                    pendentes.push_back((faixa, inicio, duracao_s));
                    while pendentes.len() > MAXIMO_DE_PENDENTES {
                        pendentes.pop_front();
                    }
                    proxima_tentativa = Instant::now();
                }
                _ => {}
            }
        }
        // Os scrobbles saem em ordem; o que falha por rede espera a próxima tentativa.
        while !sessao_invalida && Instant::now() >= proxima_tentativa {
            let Some((faixa, inicio, duracao_s)) = pendentes.front().cloned() else { break };
            match cliente.scrobble(&c, &faixa, inicio, duracao_s) {
                Ok(()) => {
                    pendentes.pop_front();
                }
                Err(Falha::SessaoInvalida) => sessao_invalida = true,
                Err(e) => {
                    log::debug!("Last.fm: scrobble adiado: {e:?}");
                    proxima_tentativa = Instant::now() + NOVA_TENTATIVA;
                }
            }
        }
        if sessao_invalida {
            esquecer(&mut conta);
            trocar(situacao, SituacaoLastfm::Falha("a conta precisa ser conectada de novo".into()));
        }
    }
}

fn rodar_discord(
    id: &str,
    ativo: &AtomicBool,
    situacao: &Mutex<SituacaoDiscord>,
    midia: &sessao::Leitor,
    som: &LeitorDeRetrato,
) {
    let mut presenca = discord::Presenca::nova(id);
    let mut proxima_ligacao = Instant::now();
    loop {
        std::thread::sleep(PASSO_DO_DISCORD);
        if !ativo.load(Ordering::Acquire) {
            presenca.desligar();
            trocar(situacao, SituacaoDiscord::Parado);
            continue;
        }
        if !presenca.ligado() {
            if Instant::now() < proxima_ligacao {
                continue;
            }
            if !presenca.ligar() {
                proxima_ligacao = Instant::now() + NOVA_LIGACAO_AO_DISCORD;
                trocar(situacao, SituacaoDiscord::Esperando);
                continue;
            }
        }
        // O cartão aparece só enquanto toca de verdade: o Discord não tem estado de "pausado".
        let a = amostra(midia, som);
        let cartao = a.faixa.filter(|f| !f.titulo.is_empty() && a.tocando).map(|faixa| discord::Cartao {
            faixa,
            intervalo: a.posicao.and_then(|(posicao, duracao)| discord::intervalo(posicao, duracao, agora_ms())),
        });
        let de_pe = presenca.mostrar(cartao.as_ref());
        trocar(situacao, if de_pe { SituacaoDiscord::Ligado } else { SituacaoDiscord::Esperando });
        if !de_pe {
            proxima_ligacao = Instant::now() + NOVA_LIGACAO_AO_DISCORD;
        }
    }
}
