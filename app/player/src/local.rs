//! O player de arquivos locais: a lista de músicas, a fila e o tocador, vistos pela interface.
//!
//! - Uma thread ("biblioteca") cuida do acervo: varre as pastas, lê as etiquetas, responde às buscas e guarda
//!   as playlists. A interface pede e lê a última lista pronta.
//! - O tocador (crate `tocador`) tem a thread dele. A fila ([`Fila`]) fica aqui: a música de agora é pedida
//!   com `Tocar`, e a seguinte já vai aberta com `Seguinte`, para entrar sem intervalo (ou com crossfade).
//! - As faixas dos provedores ([`crate::remoto`]) entram na mesma fila: antes de tocar, o endereço é pedido ao
//!   provedor; a de agora toca quando ele chega, e a seguinte já é resolvida antes, para entrar sem intervalo.
//! - A faixa é publicada no controle de mídia do Windows (a mesma caixinha onde aparecem o Spotify e o
//!   navegador), com a capa. Com isso o histórico, a letra, o Last.fm e o Discord enxergam a música local do
//!   mesmo jeito que enxergam a de qualquer app, e as teclas de mídia do teclado funcionam.

use crate::externo::Lido;
use crate::fila::Fila;
use crate::pares::QuemToca;
use biblioteca::{Acervo, Filtro, Grupo, Musica, Origem, Playlist};
use nucleo::Repetir;
use provedores::{Acao, EstadoDoTocador, SituacaoDaFaixa, SituacaoDoTocador};
use sessao::{Capa, Estado, Faixa, Posicao};
use souvlaki::{MediaControlEvent, MediaControls, MediaMetadata, MediaPlayback, MediaPosition, PlatformConfig};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tocador::{Comando, Situacao, Tocador};

/// Quantas músicas a lista mostra de uma vez (a busca estreita o resto).
const MAXIMO_NA_LISTA: u32 = 5000;
/// "Anterior" volta ao começo da música se ela já passou disto; antes disso, vai para a música anterior.
const VOLTA_AO_COMECO_S: f64 = 3.0;
/// De quanto em quanto tempo a posição é repassada ao Windows enquanto toca.
const REPUBLICAR: Duration = Duration::from_secs(5);
/// Como o player local aparece no controle de mídia do Windows (e é reconhecido de volta).
pub const NOME_NO_SISTEMA: &str = "Player";

/// Como a lista de músicas é vista.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Visao {
    #[default]
    Musicas,
    Albuns,
    Artistas,
}

/// O que a tela Músicas mostra.
#[derive(Clone, Default)]
pub struct Lista {
    pub filtro: Filtro,
    pub visao: Visao,
    pub musicas: Arc<Vec<Musica>>,
    /// Os álbuns ou artistas, quando a visão é uma dessas.
    pub grupos: Arc<Vec<Grupo>>,
    /// Todos os arquivos do acervo (a pré-análise percorre).
    pub caminhos: Arc<Vec<PathBuf>>,
    /// Quantas músicas há em "Todas", sem repetir.
    pub total: u32,
    /// As pastas, com quantas músicas cada uma tem.
    pub pastas: Vec<(PathBuf, u32)>,
    pub playlists: Vec<Playlist>,
    /// A pasta que está sendo varrida agora.
    pub varrendo: Option<String>,
    /// O acervo abriu (há pasta de dados).
    pub disponivel: bool,
    /// O que a pessoa curtiu e salvou dos serviços (os fixados primeiro), com o item já lido (a tela não lê JSON a
    /// cada quadro; `None` quando os dados estragaram).
    pub salvos: Arc<Vec<(biblioteca::Salvo, Option<provedores::Item>)>>,
}

enum Pedido {
    AdicionarPasta(PathBuf),
    RemoverPasta(PathBuf),
    /// Varre de novo todas as pastas (arquivos novos entram, os que sumiram saem).
    Atualizar,
    Buscar(String),
    /// Muda de onde vêm as músicas (tira os filtros de álbum e artista).
    Origem(Origem),
    Album(Option<String>),
    Artista(Option<String>),
    Visao(Visao),
    /// Cria uma playlist com estas músicas; com `abrir`, ela passa a ser a mostrada.
    CriarPlaylist {
        nome: String,
        musicas: Vec<Musica>,
        abrir: bool,
    },
    ApagarPlaylist(i64),
    /// As músicas entram no fim (as de serviço também: o que mostrar delas fica guardado no acervo).
    AdicionarAPlaylist(i64, Vec<Musica>),
    TirarDaPlaylist(i64, PathBuf),
    /// Desfaz o último "apagar playlist" ou "tirar da playlist".
    Desfazer,
    Salvar(biblioteca::Salvo),
    TirarSalvo(String, biblioteca::TipoSalvo, String),
    Fixar(String, biblioteca::TipoSalvo, String, bool),
}

/// O que a thread da biblioteca sabe desfazer.
enum Volta {
    PlaylistApagada { nome: String, caminhos: Vec<PathBuf>, estava_aberta: bool },
    TiradaDaPlaylist { id: i64, posicao: i64, caminho: PathBuf },
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

/// O estado da thread da biblioteca: o acervo e o que a tela pediu para ver.
struct Biblio<'a> {
    acervo: Acervo,
    filtro: Filtro,
    visao: Visao,
    lista: &'a Mutex<Lista>,
    versao: &'a AtomicU64,
    parar: &'a AtomicBool,
    /// O último apagar/tirar, para o "Desfazer" do aviso.
    volta: Option<Volta>,
}

impl Biblio<'_> {
    fn publicar(&self, varrendo: Option<String>) {
        let grupos = match self.visao {
            Visao::Musicas => Vec::new(),
            Visao::Albuns => self.acervo.albuns(&self.filtro).unwrap_or_default(),
            Visao::Artistas => self.acervo.artistas(&self.filtro).unwrap_or_default(),
        };
        let nova = Lista {
            filtro: self.filtro.clone(),
            visao: self.visao,
            musicas: Arc::new(self.acervo.buscar(&self.filtro, MAXIMO_NA_LISTA).unwrap_or_default()),
            grupos: Arc::new(grupos),
            total: self.acervo.total().unwrap_or(0),
            pastas: self.acervo.contagens().unwrap_or_default(),
            playlists: self.acervo.playlists().unwrap_or_default(),
            caminhos: Arc::new(self.acervo.caminhos().unwrap_or_default()),
            varrendo,
            disponivel: true,
            salvos: Arc::new(
                self.acervo
                    .salvos(None, None)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| {
                        let item = crate::paginas::item_do_salvo(&s);
                        (s, item)
                    })
                    .collect(),
            ),
        };
        trocar(self.lista, nova);
        self.versao.fetch_add(1, Ordering::Release);
    }

    /// Põe as músicas no fim da playlist. As de serviço guardam antes o que mostrar delas.
    fn por_na_playlist(&mut self, id: i64, musicas: &[Musica]) {
        if let Err(e) = self.acervo.guardar_remotas(musicas) {
            log::warn!("não foi possível guardar as faixas de serviço da playlist: {e}");
        }
        let caminhos: Vec<PathBuf> = musicas.iter().map(|m| m.caminho.clone()).collect();
        if let Err(e) = self.acervo.adicionar_a_playlist(id, &caminhos) {
            log::warn!("não foi possível pôr as músicas na playlist: {e}");
        }
    }

    fn varrer(&mut self, pasta: &Path) {
        // Pasta fora do ar (pendrive ou HD externo desligado, rede caída): o que estava guardado fica. Varrer
        // agora apagaria as músicas dela do acervo, e elas sumiriam das playlists.
        if !pasta.is_dir() {
            log::warn!("pasta indisponível, mantendo o que já estava guardado: {}", pasta.display());
            return;
        }
        self.publicar(Some(pasta.to_string_lossy().into_owned()));
        let mut achadas = Vec::new();
        let parar = self.parar;
        biblioteca::varrer(pasta, &mut |arquivo| achadas.push(biblioteca::ler(arquivo)), &|| {
            !parar.load(Ordering::Acquire)
        });
        // Interrompida no meio (o app está fechando): o que já estava guardado fica como estava.
        if !parar.load(Ordering::Acquire)
            && let Err(e) = self.acervo.guardar_varredura(pasta, &achadas)
        {
            log::warn!("não foi possível guardar as músicas de {}: {e}", pasta.display());
        }
    }

    fn mostrar(&mut self, origem: Origem) {
        self.filtro.origem = origem;
        self.filtro.album = None;
        self.filtro.artista = None;
    }

    fn atender(&mut self, pedido: Pedido) {
        let avisar = |r: nucleo::Resultado<()>, o_que: &str| {
            if let Err(e) = r {
                log::warn!("não foi possível {o_que}: {e}");
            }
        };
        match pedido {
            Pedido::Buscar(nova) => self.filtro.consulta = nova,
            Pedido::Origem(origem) => self.mostrar(origem),
            Pedido::Album(album) => self.filtro.album = album,
            Pedido::Artista(artista) => self.filtro.artista = artista,
            Pedido::Visao(visao) => self.visao = visao,
            Pedido::AdicionarPasta(pasta) => {
                avisar(self.acervo.adicionar_pasta(&pasta), "adicionar a pasta");
                // A pasta nova já aparece escolhida.
                self.mostrar(Origem::Pasta(pasta.clone()));
                self.varrer(&pasta);
            }
            Pedido::RemoverPasta(pasta) => {
                avisar(self.acervo.remover_pasta(&pasta), "tirar a pasta");
                if self.filtro.origem == Origem::Pasta(pasta) {
                    self.mostrar(Origem::Todas);
                }
            }
            Pedido::Atualizar => {
                for pasta in self.acervo.pastas().unwrap_or_default() {
                    self.varrer(&pasta);
                }
            }
            Pedido::CriarPlaylist { nome, musicas, abrir } => match self.acervo.criar_playlist(&nome) {
                Ok(id) => {
                    self.por_na_playlist(id, &musicas);
                    if abrir {
                        self.mostrar(Origem::Playlist(id));
                    }
                }
                Err(e) => log::warn!("não foi possível criar a playlist: {e}"),
            },
            Pedido::ApagarPlaylist(id) => {
                let estava_aberta = self.filtro.origem == Origem::Playlist(id);
                match self.acervo.apagar_playlist(id) {
                    Ok((nome, caminhos)) => self.volta = Some(Volta::PlaylistApagada { nome, caminhos, estava_aberta }),
                    Err(e) => log::warn!("não foi possível apagar a playlist: {e}"),
                }
                if estava_aberta {
                    self.mostrar(Origem::Todas);
                }
            }
            Pedido::AdicionarAPlaylist(id, musicas) => self.por_na_playlist(id, &musicas),
            Pedido::TirarDaPlaylist(id, caminho) => match self.acervo.tirar_da_playlist(id, &caminho) {
                Ok(Some(posicao)) => self.volta = Some(Volta::TiradaDaPlaylist { id, posicao, caminho }),
                Ok(None) => {}
                Err(e) => log::warn!("não foi possível tirar a música da playlist: {e}"),
            },
            Pedido::Salvar(s) => avisar(self.acervo.salvar(&s), "salvar"),
            Pedido::TirarSalvo(servico, tipo, id) => {
                avisar(self.acervo.tirar_salvo(&servico, tipo, &id).map(|_| ()), "tirar dos salvos");
            }
            Pedido::Fixar(servico, tipo, id, fixado) => {
                avisar(self.acervo.fixar(&servico, tipo, &id, fixado).map(|_| ()), "fixar");
            }
            Pedido::Desfazer => match self.volta.take() {
                Some(Volta::PlaylistApagada { nome, caminhos, estava_aberta }) => {
                    match self.acervo.criar_playlist(&nome) {
                        Ok(id) => {
                            if let Err(e) = self.acervo.adicionar_a_playlist(id, &caminhos) {
                                log::warn!("não foi possível devolver as músicas da playlist: {e}");
                            }
                            if estava_aberta {
                                self.mostrar(Origem::Playlist(id));
                            }
                        }
                        Err(e) => log::warn!("não foi possível recriar a playlist: {e}"),
                    }
                }
                Some(Volta::TiradaDaPlaylist { id, posicao, caminho }) => {
                    avisar(self.acervo.inserir_na_playlist(id, posicao, &caminho), "devolver a música à playlist");
                }
                None => {}
            },
        }
    }
}

fn rodar_biblioteca(
    caminho: &Path,
    fila: &Receiver<Pedido>,
    lista: &Mutex<Lista>,
    versao: &AtomicU64,
    parar: &AtomicBool,
) {
    let acervo = match Acervo::abrir(caminho) {
        Ok(a) => a,
        Err(e) => {
            log::warn!("biblioteca desligada: {e}");
            return;
        }
    };
    let mut b =
        Biblio { acervo, filtro: Filtro::default(), visao: Visao::default(), lista, versao, parar, volta: None };
    b.publicar(None);
    // Pastas sem nada guardado (acabaram de entrar, ou o acervo veio de uma versão anterior): varre já.
    for pasta in b.acervo.pastas_vazias().unwrap_or_default() {
        b.varrer(&pasta);
        b.publicar(None);
    }
    while let Ok(primeiro) = fila.recv() {
        // Vários pedidos na fila (digitação rápida na busca): atende todos e publica uma lista só no fim.
        let mut pedidos = vec![primeiro];
        pedidos.extend(fila.try_iter());
        for pedido in pedidos {
            b.atender(pedido);
        }
        b.publicar(None);
    }
}

/// A capa lida do arquivo: de qual música, a imagem decodificada e o arquivo de imagem gravado para o Windows.
type CapaLida = Arc<Mutex<Option<(PathBuf, Arc<Capa>, Option<PathBuf>)>>>;

/// Numera os arquivos de capa: cada música grava um nome novo (o Windows guarda a imagem pelo nome).
static CAPAS_GRAVADAS: AtomicU64 = AtomicU64::new(0);

/// Grava os bytes da capa numa pasta temporária, para o controle de mídia do Windows ler.
fn gravar_capa(bytes: &[u8]) -> Option<PathBuf> {
    let extensao = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { "png" } else { "jpg" };
    let n = CAPAS_GRAVADAS.fetch_add(1, Ordering::Relaxed);
    let pasta = std::env::temp_dir().join("TccPlayer");
    std::fs::create_dir_all(&pasta).ok()?;
    let caminho = pasta.join(format!("capa-{}-{n}.{extensao}", std::process::id()));
    std::fs::write(&caminho, bytes).ok()?;
    // A capa de duas músicas atrás já não é usada.
    if n >= 2 {
        for ext in ["png", "jpg"] {
            let _ = std::fs::remove_file(pasta.join(format!("capa-{}-{}.{ext}", std::process::id(), n - 2)));
        }
    }
    Some(caminho)
}

/// A faixa de agora toca no tocador do próprio serviço (o Soloist).
struct Externo {
    caminho: PathBuf,
    servico: String,
    /// A ordem de tocar: só vale o estado lido depois dela.
    ordem: u64,
    /// Já avisou que o serviço passou a tocar em outro aparelho.
    avisou_outro_lugar: bool,
    /// O que a pessoa acabou de mandar: fica na tela até a leitura confirmar, ou até esta hora (o tocador do serviço
    /// demora a obedecer, e a leitura de antes faria o botão voltar atrás).
    espera: Option<(Esperado, Instant)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Esperado {
    Tocando(bool),
    /// Para onde pulou, e quando.
    Posicao(f64, Instant),
}

/// Quanto o pedido de pausar, continuar ou pular vale sem confirmação; e o de tocar (o tocador do serviço abre a faixa).
const PEDIDO_VALE: Duration = Duration::from_millis(1500);
const TOCAR_VALE: Duration = Duration::from_secs(16);
/// Quão perto do pedido a posição lida confirma o pulo.
const PULO_CONFIRMADO_S: f64 = 2.5;

/// A situação lida com o que a pessoa mandou por cima, enquanto não confirma. `None` na espera: confirmou ou venceu.
fn com_o_pedido(
    lida: Situacao,
    espera: Option<(Esperado, Instant)>,
    agora: Instant,
) -> (Situacao, Option<(Esperado, Instant)>) {
    let Some((esperado, ate)) = espera.filter(|(_, ate)| agora < *ate) else { return (lida, None) };
    match esperado {
        Esperado::Tocando(t) if lida.tocando == t => (lida, None),
        Esperado::Tocando(tocando) => (Situacao { tocando, ..lida }, Some((esperado, ate))),
        Esperado::Posicao(p, quando) => {
            let anda = if lida.tocando { agora.saturating_duration_since(quando).as_secs_f64() } else { 0.0 };
            if (lida.posicao_s - (p + anda)).abs() < PULO_CONFIRMADO_S {
                return (lida, None);
            }
            let posicao_s = if lida.duracao_s > 0.0 { (p + anda).min(lida.duracao_s) } else { p + anda };
            (Situacao { posicao_s, ..lida }, Some((esperado, ate)))
        }
    }
}

/// O que fazer com a faixa do tocador do serviço.
#[derive(Debug, PartialEq)]
enum DoExterno {
    Segue,
    Acabou,
    EmOutroLugar,
    /// Não tocou (o motivo): vai pelo Match.
    Falhou(String),
}

/// O que o estado do tocador do serviço quer dizer para a faixa `id`.
fn do_externo(estado: &EstadoDoTocador, id: &str) -> DoExterno {
    let motivo = |padrao: &str| estado.erro.clone().unwrap_or_else(|| padrao.to_string());
    match estado.situacao {
        SituacaoDoTocador::Pronto | SituacaoDoTocador::Abrindo => {}
        SituacaoDoTocador::Pareando => return DoExterno::Falhou("falta parear".into()),
        SituacaoDoTocador::Desligado => return DoExterno::Falhou(motivo("foi desligado")),
        SituacaoDoTocador::Baixando => return DoExterno::Falhou("está sendo atualizado".into()),
        SituacaoDoTocador::Falhou => return DoExterno::Falhou(motivo("parou")),
    }
    match estado.tocando.as_ref().filter(|t| t.faixa == id) {
        None => DoExterno::Falhou("parou".into()),
        Some(t) => match t.situacao {
            SituacaoDaFaixa::Acabou => DoExterno::Acabou,
            SituacaoDaFaixa::EmOutroLugar => DoExterno::EmOutroLugar,
            SituacaoDaFaixa::Falhou => {
                DoExterno::Falhou(t.erro.clone().unwrap_or_else(|| "não tocou esta faixa".into()))
            }
            SituacaoDaFaixa::Carregando | SituacaoDaFaixa::Tocando | SituacaoDaFaixa::Pausada => DoExterno::Segue,
        },
    }
}

/// A faixa que toca no tocador do serviço, no formato da situação do tocador daqui (nunca é aberta).
fn origem_externa(caminho: &Path) -> tocador::Origem {
    tocador::Origem::Endereco(tocador::Endereco::novo(format!("tocador-do-servico:{}", caminho.display())))
}

pub struct Local {
    tocador: Tocador,
    preanalista: crate::preanalista::Preanalista,
    pedidos: Option<Sender<Pedido>>,
    lista_viva: Arc<Mutex<Lista>>,
    versao_viva: Arc<AtomicU64>,
    versao_vista: u64,
    parar: Arc<AtomicBool>,
    /// A última lista que a thread da biblioteca entregou.
    pub lista: Lista,
    pub fila: Fila,
    /// A situação do tocador neste quadro.
    pub situacao: Situacao,
    terminadas: u64,
    avancou: u64,
    /// O que foi pedido ao tocador como "a seguinte": a posição na fila e a origem.
    seguinte_pedida: Option<(usize, tocador::Origem)>,
    /// As músicas dos provedores.
    pub remoto: crate::remoto::Remoto,
    /// A música de agora é de um provedor e o endereço dela ainda não chegou (a chave).
    esperando_endereco: Option<PathBuf>,
    /// A música de agora toca no tocador do próprio serviço.
    externo: Option<Externo>,
    /// Um aviso para a pessoa (uma faixa de provedor que não tocou): o app mostra e limpa.
    pub aviso: Option<String>,
    /// Arquivos que não abriram, seguidos: com a fila toda ruim, para em vez de rodar sem fim.
    falhas_seguidas: usize,
    /// Igualar o volume pelo ReplayGain (vale a partir da próxima música).
    pub volume_igualado: bool,
    controles: Option<MediaControls>,
    eventos: Receiver<MediaControlEvent>,
    /// Para mandar comandos ao player local de fora da interface (bandeja, botões da barra de tarefas).
    envio_eventos: Sender<MediaControlEvent>,
    /// O que foi publicado no Windows por último: (arquivo, tocando, quando, com capa).
    publicado: Option<(PathBuf, bool, Instant, bool)>,
    capa: CapaLida,
}

impl Local {
    /// `pasta` é a pasta de dados (sem ela não há biblioteca); `hwnd` é a janela, para o controle de mídia do
    /// Windows; `mudo` toca sem som (para testar).
    pub fn iniciar(
        pasta: Option<PathBuf>,
        hwnd: Option<isize>,
        mudo: bool,
        aleatorio: bool,
        repetir: Repetir,
        analisar_biblioteca: bool,
        provedores_desligados: &[String],
    ) -> Self {
        let preanalista = crate::preanalista::Preanalista::iniciar(pasta.clone(), analisar_biblioteca);
        let remoto =
            crate::remoto::Remoto::iniciar(pasta.as_ref().map(|p| p.join("provedores")), provedores_desligados);
        let lista_viva = Arc::new(Mutex::new(Lista::default()));
        let versao_viva = Arc::new(AtomicU64::new(0));
        let parar = Arc::new(AtomicBool::new(false));
        let mut pedidos = None;
        if let Some(pasta) = pasta {
            let (envio, fila) = channel::<Pedido>();
            let (l, v, p) = (lista_viva.clone(), versao_viva.clone(), parar.clone());
            let criada = std::thread::Builder::new()
                .name("biblioteca".into())
                .spawn(move || rodar_biblioteca(&pasta.join("biblioteca.db"), &fila, &l, &v, &p));
            match criada {
                Ok(_) => pedidos = Some(envio),
                Err(e) => log::warn!("biblioteca desligada: {e}"),
            }
        }

        let (envio, eventos) = channel::<MediaControlEvent>();
        let envio_eventos = envio.clone();
        let controles = hwnd.and_then(|hwnd| {
            let config =
                PlatformConfig { display_name: NOME_NO_SISTEMA, dbus_name: "tccplayer", hwnd: Some(hwnd as *mut _) };
            let mut controles = MediaControls::new(config)
                .map_err(|e| log::warn!("o controle de mídia do sistema não está disponível: {e:?}"))
                .ok()?;
            controles
                .attach(move |evento| {
                    let _ = envio.send(evento);
                })
                .map_err(|e| log::warn!("o controle de mídia do sistema não aceitou a ligação: {e:?}"))
                .ok()?;
            Some(controles)
        });

        Self {
            tocador: Tocador::iniciar(mudo),
            preanalista,
            pedidos,
            lista_viva,
            versao_viva,
            versao_vista: u64::MAX,
            parar,
            lista: Lista::default(),
            fila: Fila::nova(aleatorio, repetir),
            situacao: Situacao::default(),
            terminadas: 0,
            avancou: 0,
            seguinte_pedida: None,
            remoto,
            esperando_endereco: None,
            externo: None,
            aviso: None,
            falhas_seguidas: 0,
            volume_igualado: true,
            controles,
            eventos,
            envio_eventos,
            publicado: None,
            capa: Arc::default(),
        }
    }

    /// Um canal para comandar o player local de outra parte do app (os comandos chegam como as teclas de
    /// mídia e são atendidos no próximo `quadro`).
    pub fn controle(&self) -> Sender<MediaControlEvent> {
        self.envio_eventos.clone()
    }

    fn pedir(&self, pedido: Pedido) {
        if let Some(p) = &self.pedidos {
            let _ = p.send(pedido);
        }
    }

    // ------------------------------------------------------------------ a lista

    pub fn buscar(&self, consulta: &str) {
        self.pedir(Pedido::Buscar(consulta.to_string()));
    }

    /// Mostra as músicas de uma origem: todas, uma pasta ou uma playlist.
    pub fn mostrar(&mut self, origem: Origem) {
        // A escolha aparece na hora; a lista chega logo depois, da thread da biblioteca.
        self.lista.filtro.origem = origem.clone();
        self.lista.filtro.album = None;
        self.lista.filtro.artista = None;
        self.pedir(Pedido::Origem(origem));
    }

    pub fn filtrar_album(&mut self, album: Option<String>) {
        self.lista.filtro.album.clone_from(&album);
        if album.is_some() {
            self.ver(Visao::Musicas);
        }
        self.pedir(Pedido::Album(album));
    }

    pub fn filtrar_artista(&mut self, artista: Option<String>) {
        self.lista.filtro.artista.clone_from(&artista);
        if artista.is_some() {
            self.ver(Visao::Musicas);
        }
        self.pedir(Pedido::Artista(artista));
    }

    pub fn ver(&mut self, visao: Visao) {
        self.lista.visao = visao;
        self.pedir(Pedido::Visao(visao));
    }

    pub fn adicionar_pasta(&self, pasta: PathBuf) {
        self.pedir(Pedido::AdicionarPasta(pasta));
    }

    pub fn remover_pasta(&self, pasta: PathBuf) {
        self.pedir(Pedido::RemoverPasta(pasta));
    }

    pub fn atualizar(&self) {
        self.pedir(Pedido::Atualizar);
    }

    pub fn criar_playlist(&self, nome: String, musicas: Vec<Musica>, abrir: bool) {
        self.pedir(Pedido::CriarPlaylist { nome, musicas, abrir });
    }

    pub fn apagar_playlist(&self, id: i64) {
        self.pedir(Pedido::ApagarPlaylist(id));
    }

    /// Curte ou salva um item de serviço (vai para a Biblioteca do serviço).
    pub fn salvar(&self, item: &provedores::Item) {
        let agora = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
        self.pedir(Pedido::Salvar(crate::paginas::salvo_de(item, agora)));
    }

    pub fn tirar_salvo(&self, item: &provedores::Item) {
        let (servico, tipo, id) = crate::paginas::chave_do_salvo(item);
        self.pedir(Pedido::TirarSalvo(servico, tipo, id));
    }

    pub fn fixar(&self, item: &provedores::Item, fixado: bool) {
        let (servico, tipo, id) = crate::paginas::chave_do_salvo(item);
        self.pedir(Pedido::Fixar(servico, tipo, id, fixado));
    }

    /// O salvo do item, se ele está salvo (na lista publicada; o pedido de agora aparece no quadro seguinte).
    pub fn salvo(&self, item: &provedores::Item) -> Option<&biblioteca::Salvo> {
        let (servico, tipo, id) = crate::paginas::chave_do_salvo(item);
        self.lista.salvos.iter().map(|(s, _)| s).find(|s| s.servico == servico && s.tipo == tipo && s.id == id)
    }

    pub fn adicionar_a_playlist(&self, id: i64, musicas: Vec<Musica>) {
        self.pedir(Pedido::AdicionarAPlaylist(id, musicas));
    }

    pub fn tirar_da_playlist(&self, id: i64, caminho: PathBuf) {
        self.pedir(Pedido::TirarDaPlaylist(id, caminho));
    }

    /// Desfaz o último "apagar playlist" ou "tirar da playlist".
    pub fn desfazer_na_biblioteca(&self) {
        self.pedir(Pedido::Desfazer);
    }

    // ------------------------------------------------------------------ tocar e a fila

    /// Há uma música local carregada (tocando ou pausada): os controles do app valem para ela.
    pub fn ativo(&self) -> bool {
        self.fila.atual().is_some()
    }

    pub fn atual(&self) -> Option<&Musica> {
        self.fila.musica_atual()
    }

    /// O ganho para tocar `m`: o ReplayGain do arquivo; sem ele, a sonoridade medida pela pré-análise (quando
    /// ela já passou pela música). Sem nenhum dos dois, como está.
    fn ganho(&self, m: &Musica) -> f32 {
        if !self.volume_igualado {
            return 1.0;
        }
        if m.ganho_db.is_some() {
            return m.ganho_igualado(0.0);
        }
        match self.preanalista.sonoridade(&m.caminho) {
            Some((lufs, pico)) => {
                let ganho = 10f32.powf(preanalise::sonoridade::ganho_db(lufs) / 20.0);
                if pico > 0.0 { ganho.min(1.0 / pico) } else { ganho }
            }
            None => 1.0,
        }
    }

    /// A pré-análise da música de agora, se já está pronta.
    pub fn analise_atual(&self) -> Option<Arc<preanalise::Analise>> {
        self.atual().and_then(|m| self.preanalista.analise(&m.caminho))
    }

    pub fn preanalista(&self) -> &crate::preanalista::Preanalista {
        &self.preanalista
    }

    /// De onde o tocador toca esta música: o arquivo, ou o endereço que o provedor deu (`None` enquanto não deu).
    fn origem_de(&self, m: &Musica) -> Option<tocador::Origem> {
        if self.externo.as_ref().is_some_and(|e| e.caminho == m.caminho) {
            return Some(origem_externa(&m.caminho));
        }
        if m.e_remota() {
            self.remoto.endereco(m).and_then(Result::ok).map(tocador::Origem::Endereco)
        } else {
            Some(tocador::Origem::Arquivo(m.caminho.clone()))
        }
    }

    /// Diz à pré-análise o que analisar primeiro (a de agora e as duas seguintes) e a biblioteca inteira.
    fn guiar_a_preanalise(&mut self) {
        use crate::preanalista::{Alvo, AlvoRemoto};
        let mut prioridade = Vec::new();
        if let Some(i) = self.fila.atual() {
            prioridade.extend(self.fila.itens().iter().skip(i).take(3).enumerate().map(|(n, m)| match &m.remota {
                None => Alvo::arquivo(m.caminho.clone()),
                Some(r) => {
                    // No tocador do serviço, nenhuma gravação daqui é a que soa.
                    let (par, endereco) = match &self.externo {
                        Some(e) if e.caminho == m.caminho => (None, None),
                        _ => self.remoto.alvo_da_analise(m, n == 0),
                    };
                    Alvo {
                        caminho: m.caminho.clone(),
                        remota: Some(AlvoRemoto { isrc: r.isrc.clone(), par, endereco }),
                    }
                }
            }));
        }
        self.preanalista.priorizar(prioridade);
        let caminhos = self.lista.caminhos.clone();
        self.preanalista.biblioteca(&caminhos);
    }

    /// Toca a música de agora da fila (do começo). De um provedor, sem o endereço ainda: pede e toca quando chegar.
    fn tocar_atual(&mut self) {
        let Some(musica) = self.fila.musica_atual().cloned() else {
            self.parar();
            return;
        };
        self.esperando_endereco = None;
        if let Some(r) = musica.remota.clone()
            && self.remoto.quem_toca_esta(&musica) == QuemToca::Sozinho
        {
            // Outro serviço que tocava sozinho (ou este, com outra faixa) é parado ou trocado por esta ordem.
            if self.externo.as_ref().is_some_and(|e| e.servico != r.servico) {
                self.parar_externo();
            }
            self.tocador.mandar(Comando::Parar);
            let ordem = self.remoto.controlar(&r.servico, Acao::Tocar { faixa: r.id });
            // Na tela já toca (dá para pausar antes de o som começar).
            self.externo = Some(Externo {
                caminho: musica.caminho.clone(),
                servico: r.servico,
                ordem,
                avisou_outro_lugar: false,
                espera: Some((Esperado::Tocando(true), Instant::now() + TOCAR_VALE)),
            });
            self.situacao = Situacao { tocando: true, ..self.situacao_externa(None) };
            self.carregar_capa(&musica);
            (self.publicado, self.seguinte_pedida) = (None, None);
            return;
        }
        self.parar_externo();
        if musica.e_remota() {
            match self.remoto.endereco(&musica) {
                Some(Ok(_)) => {}
                Some(Err(motivo)) => {
                    self.nao_tocou(&musica, &motivo);
                    return;
                }
                None => {
                    // A anterior para já: a pessoa vê que a troca aconteceu enquanto o endereço chega.
                    self.remoto.resolver(&musica);
                    self.tocador.mandar(Comando::Parar);
                    self.esperando_endereco = Some(musica.caminho.clone());
                    self.carregar_capa(&musica);
                    self.seguinte_pedida = None;
                    return;
                }
            }
        }
        let Some(origem) = self.origem_de(&musica) else { return };
        let ganho = self.ganho(&musica);
        self.tocador.mandar(Comando::Tocar { origem, ganho });
        self.carregar_capa(&musica);
        self.publicado = None;
        // A seguinte é pedida de novo no próximo quadro (o tocador esquece a anterior ao trocar de música).
        self.seguinte_pedida = None;
    }

    /// Toca a música `indice` da lista mostrada; a lista inteira vira a fila. `caminho` é o arquivo que a
    /// pessoa viu na linha: se a lista mudou entre o desenho e o clique (busca, varredura), vale o arquivo.
    pub fn tocar(&mut self, indice: usize, caminho: &Path) {
        let musicas = self.lista.musicas.as_ref().clone();
        let indice = match musicas.get(indice) {
            Some(m) if m.caminho == caminho => indice,
            _ => match musicas.iter().position(|m| m.caminho == caminho) {
                Some(i) => i,
                None => return,
            },
        };
        self.tocar_lista(musicas, indice);
    }

    /// Troca a fila por `musicas` e toca a `indice`.
    pub fn tocar_lista(&mut self, musicas: Vec<Musica>, indice: usize) {
        self.falhas_seguidas = 0;
        if self.fila.tocar(musicas, indice).is_some() {
            self.tocar_atual();
        }
    }

    /// Pula para a música `indice` da fila.
    pub fn ir_para_na_fila(&mut self, indice: usize) {
        if self.fila.ir_para(indice) {
            self.tocar_atual();
        }
    }

    /// Põe a música logo depois da de agora. Sem nada tocando, ela começa a tocar.
    pub fn tocar_a_seguir(&mut self, musica: Musica) {
        if self.ativo() {
            self.fila.tocar_a_seguir(musica);
        } else {
            self.tocar_lista(vec![musica], 0);
        }
    }

    /// Põe as músicas no fim da fila. Sem nada tocando, a primeira começa a tocar.
    pub fn adicionar_a_fila(&mut self, musicas: Vec<Musica>) {
        if self.ativo() {
            self.fila.adicionar(musicas);
        } else if !musicas.is_empty() {
            self.tocar_lista(musicas, 0);
        }
    }

    pub fn alternar_aleatorio(&mut self) {
        self.fila.alternar_aleatorio();
    }

    pub fn alternar_repetir(&mut self) {
        self.fila.repetir = self.fila.repetir.proximo();
    }

    /// Crossfade e equalizador do tocador.
    pub fn mixagem(&self, crossfade_s: f32, eq: Vec<f32>) {
        self.tocador.mandar(Comando::Mixagem { crossfade_s, eq });
    }

    pub fn alternar(&mut self) {
        if !self.ativo() {
            return;
        }
        if let Some(e) = self.externo.as_mut() {
            let acao = if self.situacao.tocando { Acao::Pausar } else { Acao::Continuar };
            let voltando = acao == Acao::Continuar;
            e.ordem = self.remoto.controlar(&e.servico, acao);
            // (De outro aparelho, a faixa volta do começo do carregamento.)
            let vale = if voltando && e.avisou_outro_lugar { TOCAR_VALE } else { PEDIDO_VALE };
            e.espera = Some((Esperado::Tocando(voltando), Instant::now() + vale));
            // Na hora na tela; a próxima leitura confirma.
            self.situacao.tocando = voltando;
            self.publicado = None;
            return;
        }
        self.tocador.mandar(if self.situacao.tocando { Comando::Pausar } else { Comando::Continuar });
    }

    /// "Próxima" apertada pela pessoa: passa adiante mesmo com "repetir a música".
    pub fn proxima(&mut self) {
        if !self.ativo() {
            return;
        }
        match self.fila.indice_seguinte(false) {
            Some(i) => self.ir_para_na_fila(i),
            // Fim da fila: para, e o app volta a acompanhar o que os outros apps tocam.
            None => self.parar(),
        }
    }

    pub fn anterior(&mut self) {
        if !self.ativo() {
            return;
        }
        match self.fila.indice_anterior() {
            Some(i) if self.situacao.posicao_s < VOLTA_AO_COMECO_S => self.ir_para_na_fila(i),
            _ => self.ir(0.0),
        }
    }

    pub fn ir(&mut self, segundos: f64) {
        if self.ativo() && segundos.is_finite() {
            let segundos = segundos.max(0.0);
            match self.externo.as_mut() {
                Some(e) => {
                    e.ordem = self.remoto.controlar(&e.servico, Acao::Ir { segundos });
                    let agora = Instant::now();
                    e.espera = Some((Esperado::Posicao(segundos, agora), agora + PEDIDO_VALE));
                    self.situacao.posicao_s = segundos;
                }
                None => self.tocador.mandar(Comando::Ir(segundos)),
            }
            self.publicado = None;
        }
    }

    /// Uma faixa de provedor não tocou (sem endereço, provedor fechado): avisa e segue para a próxima, como um
    /// arquivo que não abre.
    fn nao_tocou(&mut self, musica: &Musica, motivo: &str) {
        log::warn!("\"{}\" não tocou: {motivo}", musica.titulo);
        self.aviso = Some(format!("\"{}\" não tocou ({motivo})", musica.titulo));
        self.remoto.esquecer(musica);
        self.falhas_seguidas += 1;
        match self.fila.indice_seguinte(false) {
            Some(i) if self.falhas_seguidas < self.fila.itens().len() => self.ir_para_na_fila(i),
            _ => self.parar(),
        }
    }

    /// Procura outra gravação para uma faixa que toca pelo Match; se é a de agora, ela recomeça com a nova.
    pub fn outro_par(&mut self, m: &Musica) {
        self.remoto.outro_par(m);
        if self.fila.musica_atual().is_some_and(|a| a.caminho == m.caminho) {
            self.tocar_atual();
        }
    }

    /// O programa que faz o som da música de agora, quando não é o Player (para o visual ouvir só ele).
    pub fn processo_externo(&self) -> Option<String> {
        let e = self.externo.as_ref()?;
        self.remoto.tocador(&e.servico)?.estado.processo
    }

    /// Para o que toca no tocador do serviço.
    fn parar_externo(&mut self) {
        if let Some(e) = self.externo.take() {
            self.remoto.controlar(&e.servico, Acao::Parar);
        }
    }

    /// A situação da música de agora no tocador do serviço (`None`: a ordem de tocar ainda não chegou lá).
    fn situacao_externa(&self, lido: Option<&Lido>) -> Situacao {
        let Some(e) = &self.externo else { return self.situacao.clone() };
        let da_faixa = lido.and_then(|l| l.estado.tocando.as_ref().map(|t| (l, t)));
        let carregando = da_faixa.is_none_or(|(_, t)| t.situacao == SituacaoDaFaixa::Carregando && t.duracao_s <= 0.0);
        Situacao {
            origem: (!carregando).then(|| origem_externa(&e.caminho)),
            tocando: da_faixa.is_some_and(|(_, t)| t.situacao == SituacaoDaFaixa::Tocando),
            posicao_s: da_faixa.map_or(0.0, |(l, t)| l.posicao_s(t, Instant::now())),
            duracao_s: da_faixa.map_or(0.0, |(_, t)| t.duracao_s),
            seguinte: None,
            erro: None,
            ..self.situacao.clone()
        }
    }

    /// Acompanha a música que toca no tocador do serviço: a posição, o fim e a falha (que vai para o Match).
    fn acompanhar_externo(&mut self) {
        let Some(e) = &self.externo else { return };
        let Some(m) = self.fila.musica_atual().cloned().filter(|m| m.caminho == e.caminho) else {
            self.parar_externo();
            return;
        };
        // Até a ordem chegar lá, vale o que já está na tela.
        let Some(lido) = self.remoto.tocador_depois(&e.servico, e.ordem) else {
            return;
        };
        let id = m.remota.as_ref().map(|r| r.id.clone()).unwrap_or_default();
        match do_externo(&lido.estado, &id) {
            DoExterno::Segue => {
                let carregando =
                    lido.estado.tocando.as_ref().is_some_and(|t| t.situacao == SituacaoDaFaixa::Carregando);
                let espera = self.externo.as_ref().and_then(|e| e.espera);
                let (situacao, espera) = com_o_pedido(self.situacao_externa(Some(&lido)), espera, Instant::now());
                if let Some(e) = self.externo.as_mut() {
                    // Pausada antes de começar: "não tocando" ainda não confirma nada.
                    e.espera = if carregando { e.espera } else { espera };
                    e.avisou_outro_lugar &= !situacao.tocando;
                }
                self.situacao = situacao;
            }
            DoExterno::EmOutroLugar => {
                // Enquanto a pessoa traz a faixa de volta, a leitura de antes ainda diz que ela está lá.
                let espera = self.externo.as_ref().and_then(|e| e.espera);
                let voltando = matches!(espera, Some((Esperado::Tocando(true), ate)) if Instant::now() < ate)
                    && self.externo.as_ref().is_some_and(|e| e.avisou_outro_lugar);
                let lida = self.situacao_externa(Some(&lido));
                self.situacao = Situacao { tocando: voltando, ..lida };
                if let Some(e) = self.externo.as_mut().filter(|e| !e.avisou_outro_lugar) {
                    e.espera = None;
                    e.avisou_outro_lugar = true;
                    let fonte = m.remota.as_ref().map(|r| r.fonte.clone()).unwrap_or_default();
                    self.aviso = Some(format!("O {fonte} passou a tocar em outro aparelho"));
                }
            }
            DoExterno::Acabou => match self.fila.indice_seguinte(true) {
                Some(i) => self.ir_para_na_fila(i),
                None => self.parar(),
            },
            DoExterno::Falhou(motivo) => {
                let nome = m.remota.as_ref().map(|r| r.fonte.clone()).unwrap_or_default();
                log::warn!("\"{}\" não tocou no tocador do {nome}: {motivo}", m.titulo);
                self.remoto.mandar_pelo_match(&m);
                // Sem outro serviço, quem avisa é a falha da própria faixa.
                if matches!(self.remoto.quem_toca_esta(&m), QuemToca::Outro(_)) {
                    self.aviso = Some(format!("O player do {nome} não tocou ({motivo}): tocando por outro serviço"));
                }
                self.tocar_atual();
            }
        }
    }

    pub fn parar(&mut self) {
        self.parar_externo();
        self.tocador.mandar(Comando::Parar);
        self.fila.limpar();
        self.esperando_endereco = None;
        self.seguinte_pedida = None;
        self.publicado = None;
        if let Some(c) = self.controles.as_mut() {
            let _ = c.set_playback(MediaPlayback::Stopped);
        }
    }

    /// Pede ao tocador a música que vem depois, se ela mudou (a fila foi mexida, o repetir mudou...).
    fn preparar_seguinte(&mut self) {
        if !self.ativo() {
            return;
        }
        let seguinte = self.fila.indice_seguinte(true).and_then(|i| self.fila.itens().get(i).cloned().map(|m| (i, m)));
        // A seguinte é de um provedor e o endereço ainda não veio: pede agora (ela entra quando chegar).
        if let Some((_, m)) = &seguinte
            && m.e_remota()
            && self.remoto.quem_toca_esta(m) != QuemToca::Sozinho
            && self.remoto.endereco(m).is_none()
        {
            self.remoto.resolver(m);
            return;
        }
        // Com a de agora no tocador do serviço, o daqui não tem o que emendar (o endereço da seguinte já fica pronto).
        if self.externo.is_some() {
            return;
        }
        // A que toca no tocador do serviço não é preparada aqui: quando esta acabar, a fila anda até ela.
        let quer = seguinte
            .as_ref()
            .filter(|(_, m)| !m.e_remota() || self.remoto.quem_toca_esta(m) != QuemToca::Sozinho)
            .and_then(|(i, m)| self.origem_de(m).map(|o| (*i, o)));
        if quer == self.seguinte_pedida {
            return;
        }
        let pedido = match (&quer, &seguinte) {
            (Some((_, origem)), Some((_, m))) => Some((origem.clone(), self.ganho(m))),
            _ => None,
        };
        self.tocador.mandar(Comando::Seguinte(pedido));
        self.seguinte_pedida = quer;
    }

    /// Lê a capa (embutida no arquivo, ou baixada do endereço que o provedor deu) fora da thread da interface, e
    /// grava uma cópia para o Windows.
    fn carregar_capa(&self, musica: &Musica) {
        let destino = self.capa.clone();
        let arquivo = musica.caminho.clone();
        let endereco = musica.remota.as_ref().map(|r| r.capa.clone());
        let lendo = std::thread::Builder::new().name("capa".into()).spawn(move || {
            let bytes = match endereco {
                None => biblioteca::capa(&arquivo),
                Some(Some(url)) => tocador::rede::baixar_pequeno(&url, 8 * 1024 * 1024)
                    .map_err(|e| log::debug!("capa não baixada: {e}"))
                    .ok(),
                Some(None) => None,
            };
            let lida = bytes.and_then(|bytes| {
                let capa = Capa::decodificar(&bytes)?;
                Some((Arc::new(capa), gravar_capa(&bytes)))
            });
            trocar(&destino, lida.map(|(capa, imagem)| (arquivo, capa, imagem)));
        });
        if let Err(e) = lendo {
            log::debug!("capa não lida: {e}");
        }
    }

    /// Um quadro: lê a lista e o tocador, acompanha a fila quando uma música acaba ou a seguinte entra,
    /// atende as teclas de mídia e mantém o Windows informado.
    pub fn quadro(&mut self) {
        self.remoto.quadro();
        // O endereço da faixa de agora chegou (ou não deu).
        if let Some(chave) = self.esperando_endereco.clone() {
            match self.fila.musica_atual().cloned() {
                Some(m) if m.caminho == chave => {
                    // (Quem toca podia estar abrindo: o pedido é refeito, ou a faixa vai para o tocador do serviço.)
                    self.remoto.resolver(&m);
                    if self.remoto.endereco(&m).is_some() || self.remoto.quem_toca_esta(&m) == QuemToca::Sozinho {
                        self.tocar_atual();
                    }
                }
                _ => self.esperando_endereco = None,
            }
        }
        let versao = self.versao_viva.load(Ordering::Acquire);
        if versao != self.versao_vista {
            self.versao_vista = versao;
            self.lista = ler(&self.lista_viva);
        }
        let do_tocador = self.tocador.situacao();
        if self.externo.is_some() {
            // O tocador daqui está parado: os contadores dele não falam desta música.
            (self.avancou, self.terminadas) = (do_tocador.avancou, do_tocador.terminadas);
            self.situacao = Situacao { avancou: self.avancou, terminadas: self.terminadas, ..self.situacao.clone() };
            self.acompanhar_externo();
        }
        // (A música pode ter acabado de sair do tocador do serviço.)
        if self.externo.is_none() {
            self.situacao = self.tocador.situacao();
        }
        if self.situacao.tocando {
            self.falhas_seguidas = 0;
        }
        // A seguinte entrou sozinha (sem intervalo ou por crossfade): a fila anda junto.
        if self.situacao.avancou != self.avancou {
            self.avancou = self.situacao.avancou;
            let origem = self.situacao.origem.clone();
            let indice = match self.seguinte_pedida.take() {
                Some((i, o)) if Some(&o) == origem.as_ref() => Some(i),
                _ => origem
                    .as_ref()
                    .and_then(|o| self.fila.itens().iter().position(|m| self.origem_de(m).as_ref() == Some(o))),
            };
            if let Some(i) = indice {
                self.fila.ir_para(i);
            }
            if let Some(m) = self.fila.musica_atual().cloned() {
                self.carregar_capa(&m);
            }
            self.publicado = None;
        }
        // Acabou sem nada depois (fim da fila, ou um arquivo que não abriu).
        if self.situacao.terminadas != self.terminadas {
            self.terminadas = self.situacao.terminadas;
            if self.situacao.erro.is_some() {
                // Uma faixa de provedor que não abriu pode ter o endereço vencido: da próxima vez, pede de novo.
                if let Some(m) = self.fila.musica_atual().cloned().filter(Musica::e_remota) {
                    self.remoto.esquecer(&m);
                }
                self.falhas_seguidas += 1;
                match self.fila.indice_seguinte(false) {
                    Some(i) if self.falhas_seguidas < self.fila.itens().len() => self.ir_para_na_fila(i),
                    _ => self.parar(),
                }
            } else if self.ativo() {
                match self.fila.indice_seguinte(true) {
                    Some(i) => self.ir_para_na_fila(i),
                    None => self.parar(),
                }
            }
        }
        self.preparar_seguinte();
        self.guiar_a_preanalise();
        while let Ok(evento) = self.eventos.try_recv() {
            match evento {
                MediaControlEvent::Toggle => self.alternar(),
                MediaControlEvent::Play if !self.situacao.tocando => self.alternar(),
                MediaControlEvent::Pause if self.situacao.tocando => self.alternar(),
                MediaControlEvent::Next => self.proxima(),
                MediaControlEvent::Previous => self.anterior(),
                MediaControlEvent::Stop => self.parar(),
                MediaControlEvent::SetPosition(MediaPosition(onde)) => self.ir(onde.as_secs_f64()),
                _ => {}
            }
        }
        self.publicar();
    }

    /// Conta ao Windows o que está tocando: a faixa (e a capa, quando fica pronta) e a posição quando o estado
    /// muda (e de tempos em tempos).
    fn publicar(&mut self) {
        let Some(musica) = self.fila.musica_atual().cloned() else { return };
        // Só depois que o tocador carregou esta música: antes disso a posição ainda é a da anterior.
        match self.origem_de(&musica) {
            Some(o) if self.situacao.origem.as_ref() == Some(&o) => {}
            _ => return,
        }
        let Some(controles) = self.controles.as_mut() else { return };
        let tocando = self.situacao.tocando;
        let imagem = match ler(&self.capa) {
            Some((arquivo, _, imagem)) if arquivo == musica.caminho => imagem,
            _ => None,
        };
        let mesma_faixa = self.publicado.as_ref().is_some_and(|(arquivo, ..)| *arquivo == musica.caminho);
        let capa_em_dia = self.publicado.as_ref().is_some_and(|(.., com_capa)| *com_capa == imagem.is_some());
        let em_dia = self.publicado.as_ref().is_some_and(|(_, estava, quando, _)| {
            mesma_faixa && capa_em_dia && *estava == tocando && quando.elapsed() < REPUBLICAR
        });
        if em_dia {
            return;
        }
        if !mesma_faixa || !capa_em_dia {
            let duracao = Some(self.situacao.duracao_s).filter(|d| *d > 0.0).map(Duration::from_secs_f64);
            let capa_url = imagem.as_ref().map(|p| format!("file://{}", p.display()));
            let _ = controles.set_metadata(MediaMetadata {
                title: Some(&musica.titulo),
                artist: Some(&musica.artista).filter(|a| !a.is_empty()).map(String::as_str),
                album: Some(&musica.album).filter(|a| !a.is_empty()).map(String::as_str),
                duration: duracao,
                cover_url: capa_url.as_deref(),
            });
        }
        let progresso = Some(MediaPosition(Duration::from_secs_f64(self.situacao.posicao_s.max(0.0))));
        let _ = controles.set_playback(if tocando {
            MediaPlayback::Playing { progress: progresso }
        } else {
            MediaPlayback::Paused { progress: progresso }
        });
        self.publicado = Some((musica.caminho, tocando, Instant::now(), imagem.is_some()));
    }

    /// O "tocando agora" da música local, no mesmo formato do que vem dos outros apps. `None` quando não há
    /// música local carregada.
    pub fn estado_de_midia(&self) -> Option<Estado> {
        let musica = self.atual()?;
        let capa = match ler(&self.capa) {
            Some((arquivo, capa, _)) if arquivo == musica.caminho => Some(capa),
            _ => None,
        };
        // Durante o crossfade, a fila já anda quando a seguinte começa a soar; antes disso, o tocador ainda
        // informa a anterior.
        let carregada = self.origem_de(musica).is_some_and(|o| self.situacao.origem.as_ref() == Some(&o));
        let duracao = if carregada { self.situacao.duracao_s.max(musica.duracao_s) } else { musica.duracao_s };
        Some(Estado {
            faixa: Some(Faixa {
                titulo: musica.titulo.clone(),
                artista: musica.artista.clone(),
                album: musica.album.clone(),
                app: musica.remota.as_ref().map_or_else(|| "Neste computador".to_string(), |r| r.fonte.clone()),
            }),
            capa,
            tocando: self.situacao.tocando,
            posicao: Some(duracao).filter(|d| *d > 1.0).map(|duracao_s| Posicao {
                posicao_s: self.situacao.posicao_s,
                duracao_s,
                lida_em: Instant::now(),
            }),
            disponivel: true,
        })
    }
}

impl Drop for Local {
    fn drop(&mut self) {
        // Uma varredura longa não segura o fechamento do app.
        self.parar.store(true, Ordering::Release);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use provedores::Tocando;

    #[test]
    fn o_estado_do_tocador_diz_o_que_fazer_com_a_faixa() {
        let com = |situacao, faixa: &str, da_faixa| EstadoDoTocador {
            situacao,
            tocando: Some(Tocando { faixa: faixa.into(), situacao: da_faixa, ..Default::default() }),
            ..Default::default()
        };
        use SituacaoDaFaixa as F;
        use SituacaoDoTocador as T;
        for f in [F::Carregando, F::Tocando, F::Pausada] {
            assert_eq!(do_externo(&com(T::Pronto, "a", f), "a"), DoExterno::Segue);
        }
        assert_eq!(do_externo(&com(T::Abrindo, "a", F::Carregando), "a"), DoExterno::Segue, "abrindo, a faixa espera");
        assert_eq!(do_externo(&com(T::Pronto, "a", F::Acabou), "a"), DoExterno::Acabou);
        assert_eq!(do_externo(&com(T::Pronto, "a", F::EmOutroLugar), "a"), DoExterno::EmOutroLugar);
        assert!(matches!(do_externo(&com(T::Pronto, "a", F::Falhou), "a"), DoExterno::Falhou(_)));
        assert!(matches!(do_externo(&com(T::Pronto, "b", F::Tocando), "a"), DoExterno::Falhou(_)), "outra faixa");
        for t in [T::Desligado, T::Baixando, T::Pareando, T::Falhou] {
            assert!(matches!(do_externo(&com(t, "a", F::Tocando), "a"), DoExterno::Falhou(_)), "{t:?}");
        }
    }

    #[test]
    fn o_que_a_pessoa_mandou_fica_ate_confirmar() {
        let agora = Instant::now();
        let lida = |tocando, posicao_s| Situacao { tocando, posicao_s, duracao_s: 200.0, ..Default::default() };
        let pausar = Some((Esperado::Tocando(false), agora + PEDIDO_VALE));
        let (s, e) = com_o_pedido(lida(true, 10.0), pausar, agora);
        assert!(!s.tocando && e == pausar, "a leitura de antes não volta o botão");
        assert_eq!(com_o_pedido(lida(false, 10.0), pausar, agora).1, None, "confirmou");
        let (s, e) = com_o_pedido(lida(true, 10.0), pausar, agora + PEDIDO_VALE);
        assert!(s.tocando && e.is_none(), "não obedeceu: a verdade volta");
        let pular = Some((Esperado::Posicao(100.0, agora), agora + PEDIDO_VALE));
        let depois = agora + Duration::from_secs(1);
        let (s, e) = com_o_pedido(lida(true, 11.0), pular, depois);
        assert!((s.posicao_s - 101.0).abs() < 0.01 && e == pular);
        assert_eq!(com_o_pedido(lida(true, 100.8), pular, depois).1, None);
        let fim = Some((Esperado::Posicao(199.9, agora), agora + PEDIDO_VALE));
        assert_eq!(com_o_pedido(lida(true, 10.0), fim, depois).0.posicao_s, 200.0);
    }

    /// Um player local mudo, com o provedor-exemplo que toca sozinho já ligado.
    fn com_o_sozinho(teste: &str) -> Option<(Local, PathBuf)> {
        let exe = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent()?.parent().map(|p| p.join("provedor-exemplo.exe")))
            .filter(|e| e.is_file());
        let Some(exe) = exe else {
            eprintln!("{teste}: sem o provedor-exemplo compilado (rode cargo test --workspace)");
            return None;
        };
        let raiz = std::env::temp_dir().join(format!("tccplayer-local-{teste}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let p = raiz.join("provedores").join("exemplo");
        std::fs::create_dir_all(&p).expect("pasta");
        std::fs::copy(&exe, p.join("provedor-exemplo.exe")).expect("copiar o exemplo");
        // (Como o de verdade: só obedece um pouco depois de responder.)
        let manifesto = "nome = \"Exemplo\"\nservico = \"exemplo\"\nexecutavel = \"provedor-exemplo.exe\"\nargumentos = [\"--sozinho\", \"--atrasa\"]";
        std::fs::write(p.join("provedor.toml"), manifesto).expect("manifesto");
        let mut l = Local::iniciar(Some(raiz.clone()), None, true, false, Repetir::Nada, false, &[]);
        assert!(esperar(&mut l, |l| l.remoto.tocador("exemplo").is_some()));
        l.remoto.ligar_tocador("exemplo", None);
        let pronto =
            |l: &Local| l.remoto.tocador("exemplo").is_some_and(|t| t.estado.situacao == SituacaoDoTocador::Pronto);
        assert!(esperar(&mut l, pronto));
        Some((l, raiz))
    }

    fn esperar(l: &mut Local, f: impl Fn(&Local) -> bool) -> bool {
        let ate = Instant::now() + Duration::from_secs(8);
        while Instant::now() < ate {
            l.quadro();
            if f(l) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    fn musica(id: &str) -> Musica {
        let f = provedores::Faixa {
            servico: "exemplo".into(),
            id: id.into(),
            titulo: format!("Faixa {id}"),
            ..Default::default()
        };
        crate::remoto::musica_de(&f, "Exemplo")
    }

    #[test]
    fn a_fila_anda_com_o_tocador_do_servico() {
        let Some((mut l, raiz)) = com_o_sozinho("fila") else { return };
        l.tocar_lista(vec![musica("1"), musica("1b")], 0);
        assert!(l.situacao.tocando, "na tela já toca");
        assert!(esperar(&mut l, |l| l.situacao.tocando && l.situacao.posicao_s > 0.3), "{:?}", l.situacao);
        assert_eq!(l.situacao.duracao_s, 4.0);
        assert!(l.estado_de_midia().is_some_and(|e| e.tocando && e.posicao.is_some()));
        l.alternar();
        assert!(!l.situacao.tocando, "na hora");
        for _ in 0..80 {
            l.quadro();
            assert!(!l.situacao.tocando, "o botão não volta atrás");
            std::thread::sleep(Duration::from_millis(10));
        }
        l.alternar();
        for _ in 0..60 {
            l.quadro();
            assert!(l.situacao.tocando);
            std::thread::sleep(Duration::from_millis(10));
        }
        l.ir(3.4);
        assert!((l.situacao.posicao_s - 3.4).abs() < 0.01);
        assert!(esperar(&mut l, |l| l.fila.atual() == Some(1) && l.situacao.tocando), "a próxima entra");
        assert!(l.situacao.posicao_s < 2.0, "{}", l.situacao.posicao_s);
        l.parar();
        assert!(esperar(&mut l, |l| l.remoto.tocador("exemplo").is_some_and(|t| t.estado.tocando.is_none())));
        assert!(!l.ativo() && !l.situacao.tocando);
        drop(l);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn a_que_o_tocador_do_servico_nao_toca_e_a_tomada_por_outro_aparelho() {
        let Some((mut l, raiz)) = com_o_sozinho("falha") else { return };
        // Sem outro serviço para o Match, ela é pulada com o aviso.
        l.tocar_lista(vec![musica("3"), musica("2")], 0);
        assert!(esperar(&mut l, |l| l.fila.atual() == Some(1) && l.situacao.tocando), "{:?}", l.aviso);
        assert!(l.aviso.take().is_some_and(|a| a.contains("Faixa 3")));
        // Outro aparelho toma a vez: pausa, avisa uma vez, e a faixa continua na fila.
        assert!(esperar(&mut l, |l| l.aviso.is_some()));
        assert!(l.aviso.take().is_some_and(|a| a.contains("outro aparelho")));
        assert!(!l.situacao.tocando && l.fila.atual() == Some(1));
        for _ in 0..30 {
            l.quadro();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert_eq!(l.aviso, None);
        // Desligado no meio: a faixa não fica presa.
        l.remoto.desligar_tocador("exemplo");
        assert!(esperar(&mut l, |l| !l.ativo()));
        drop(l);
        let _ = std::fs::remove_dir_all(&raiz);
    }
}
