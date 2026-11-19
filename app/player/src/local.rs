//! O player de arquivos locais: a lista de músicas, a fila e o tocador, vistos pela interface.
//!
//! - Uma thread ("biblioteca") cuida do acervo: varre as pastas, lê as etiquetas, responde às buscas e guarda
//!   as playlists. A interface pede e lê a última lista pronta.
//! - O tocador (crate `tocador`) tem a thread dele. A fila ([`Fila`]) fica aqui: a música de agora é pedida
//!   com `Tocar`, e a seguinte já vai aberta com `Seguinte`, para entrar sem intervalo (ou com crossfade).
//! - A faixa é publicada no controle de mídia do Windows (a mesma caixinha onde aparecem o Spotify e o
//!   navegador), com a capa. Com isso o histórico, a letra, o Last.fm e o Discord enxergam a música local do
//!   mesmo jeito que enxergam a de qualquer app, e as teclas de mídia do teclado funcionam.

use crate::fila::Fila;
use biblioteca::{Acervo, Filtro, Grupo, Musica, Origem, Playlist};
use nucleo::Repetir;
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
        musicas: Vec<PathBuf>,
        abrir: bool,
    },
    ApagarPlaylist(i64),
    AdicionarAPlaylist(i64, Vec<PathBuf>),
    TirarDaPlaylist(i64, PathBuf),
    /// Desfaz o último "apagar playlist" ou "tirar da playlist".
    Desfazer,
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
        };
        trocar(self.lista, nova);
        self.versao.fetch_add(1, Ordering::Release);
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
                    if let Err(e) = self.acervo.adicionar_a_playlist(id, &musicas) {
                        log::warn!("não foi possível pôr as músicas na playlist: {e}");
                    }
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
            Pedido::AdicionarAPlaylist(id, musicas) => {
                if let Err(e) = self.acervo.adicionar_a_playlist(id, &musicas) {
                    log::warn!("não foi possível pôr as músicas na playlist: {e}");
                }
            }
            Pedido::TirarDaPlaylist(id, caminho) => match self.acervo.tirar_da_playlist(id, &caminho) {
                Ok(Some(posicao)) => self.volta = Some(Volta::TiradaDaPlaylist { id, posicao, caminho }),
                Ok(None) => {}
                Err(e) => log::warn!("não foi possível tirar a música da playlist: {e}"),
            },
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
    /// O que foi pedido ao tocador como "a seguinte": a posição na fila e o arquivo.
    seguinte_pedida: Option<(usize, PathBuf)>,
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
    ) -> Self {
        let preanalista = crate::preanalista::Preanalista::iniciar(pasta.clone(), analisar_biblioteca);
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

    pub fn criar_playlist(&self, nome: String, musicas: Vec<PathBuf>, abrir: bool) {
        self.pedir(Pedido::CriarPlaylist { nome, musicas, abrir });
    }

    pub fn apagar_playlist(&self, id: i64) {
        self.pedir(Pedido::ApagarPlaylist(id));
    }

    pub fn adicionar_a_playlist(&self, id: i64, musicas: Vec<PathBuf>) {
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

    /// Diz à pré-análise o que analisar primeiro (a de agora e as duas seguintes) e a biblioteca inteira.
    fn guiar_a_preanalise(&mut self) {
        let mut prioridade = Vec::new();
        if let Some(i) = self.fila.atual() {
            prioridade.extend(self.fila.itens().iter().skip(i).take(3).map(|m| m.caminho.clone()));
        }
        self.preanalista.priorizar(prioridade);
        let caminhos = self.lista.caminhos.clone();
        self.preanalista.biblioteca(&caminhos);
    }

    /// Toca a música de agora da fila (do começo).
    fn tocar_atual(&mut self) {
        let Some(musica) = self.fila.musica_atual().cloned() else {
            self.parar();
            return;
        };
        let ganho = self.ganho(&musica);
        self.tocador.mandar(Comando::Tocar { caminho: musica.caminho.clone(), ganho });
        self.carregar_capa(musica.caminho);
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
        if self.ativo() {
            self.tocador.mandar(if self.situacao.tocando { Comando::Pausar } else { Comando::Continuar });
        }
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
            self.tocador.mandar(Comando::Ir(segundos.max(0.0)));
            self.publicado = None;
        }
    }

    pub fn parar(&mut self) {
        self.tocador.mandar(Comando::Parar);
        self.fila.limpar();
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
        let quer =
            self.fila.indice_seguinte(true).and_then(|i| self.fila.itens().get(i).map(|m| (i, m.caminho.clone())));
        if quer == self.seguinte_pedida {
            return;
        }
        let pedido =
            quer.as_ref().and_then(|(i, caminho)| self.fila.itens().get(*i).map(|m| (caminho.clone(), self.ganho(m))));
        self.tocador.mandar(Comando::Seguinte(pedido));
        self.seguinte_pedida = quer;
    }

    /// Lê a capa embutida no arquivo fora da thread da interface, e grava uma cópia para o Windows.
    fn carregar_capa(&self, arquivo: PathBuf) {
        let destino = self.capa.clone();
        let lendo = std::thread::Builder::new().name("capa".into()).spawn(move || {
            let lida = biblioteca::capa(&arquivo).and_then(|bytes| {
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
        let versao = self.versao_viva.load(Ordering::Acquire);
        if versao != self.versao_vista {
            self.versao_vista = versao;
            self.lista = ler(&self.lista_viva);
        }
        self.situacao = self.tocador.situacao();
        if self.situacao.tocando {
            self.falhas_seguidas = 0;
        }
        // A seguinte entrou sozinha (sem intervalo ou por crossfade): a fila anda junto.
        if self.situacao.avancou != self.avancou {
            self.avancou = self.situacao.avancou;
            let arquivo = self.situacao.arquivo.clone();
            let indice = match self.seguinte_pedida.take() {
                Some((i, caminho)) if Some(&caminho) == arquivo.as_ref() => Some(i),
                _ => arquivo.as_ref().and_then(|a| self.fila.itens().iter().position(|m| &m.caminho == a)),
            };
            if let Some(i) = indice {
                self.fila.ir_para(i);
            }
            if let Some(a) = arquivo {
                self.carregar_capa(a);
            }
            self.publicado = None;
        }
        // Acabou sem nada depois (fim da fila, ou um arquivo que não abriu).
        if self.situacao.terminadas != self.terminadas {
            self.terminadas = self.situacao.terminadas;
            if self.situacao.erro.is_some() {
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
        let Some(controles) = self.controles.as_mut() else { return };
        let tocando = self.situacao.tocando;
        // Só depois que o tocador carregou este arquivo: antes disso a posição ainda é a da música anterior.
        if self.situacao.arquivo.as_deref() != Some(musica.caminho.as_path()) {
            return;
        }
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
        let duracao = if self.situacao.arquivo.as_deref() == Some(musica.caminho.as_path()) {
            self.situacao.duracao_s.max(musica.duracao_s)
        } else {
            musica.duracao_s
        };
        Some(Estado {
            faixa: Some(Faixa {
                titulo: musica.titulo.clone(),
                artista: musica.artista.clone(),
                album: musica.album.clone(),
                app: "Neste computador".to_string(),
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
