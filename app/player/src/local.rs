//! O player de arquivos locais: a lista de músicas, a fila e o tocador, vistos pela interface.
//!
//! - Uma thread ("biblioteca") cuida do acervo: varre as pastas, lê as etiquetas e responde às buscas. A
//!   interface pede e lê a última lista pronta.
//! - O tocador (crate `tocador`) tem a thread dele. Aqui fica a fila: qual música toca, a próxima quando uma
//!   acaba, voltar e avançar.
//! - A faixa é publicada no controle de mídia do Windows (a mesma caixinha onde aparecem o Spotify e o
//!   navegador). Com isso o histórico, a letra, o Last.fm e o Discord enxergam a música local do mesmo jeito
//!   que enxergam a de qualquer app, e as teclas de mídia do teclado funcionam.

use biblioteca::{Acervo, Musica};
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
const NOME_NO_SISTEMA: &str = "Player";

/// O que a tela Músicas mostra.
#[derive(Clone, Default)]
pub struct Lista {
    pub consulta: String,
    /// A pasta (categoria) mostrada; `None` = todas, cada música uma vez.
    pub pasta: Option<PathBuf>,
    pub musicas: Arc<Vec<Musica>>,
    /// Quantas músicas há em "Todas", sem repetir (a lista pode ser um recorte da busca ou de uma pasta).
    pub total: u32,
    /// As pastas, com quantas músicas cada uma tem.
    pub pastas: Vec<(PathBuf, u32)>,
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
    /// Mostra só as músicas de uma pasta (`None` = todas).
    Mostrar(Option<PathBuf>),
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

fn rodar_biblioteca(
    caminho: &Path,
    fila: &Receiver<Pedido>,
    lista: &Mutex<Lista>,
    versao: &AtomicU64,
    parar: &AtomicBool,
) {
    let mut acervo = match Acervo::abrir(caminho) {
        Ok(a) => a,
        Err(e) => {
            log::warn!("biblioteca desligada: {e}");
            return;
        }
    };
    let mut consulta = String::new();
    let mut mostrada: Option<PathBuf> = None;
    let publicar = |acervo: &Acervo, consulta: &str, mostrada: &Option<PathBuf>, varrendo: Option<String>| {
        let nova = Lista {
            consulta: consulta.to_string(),
            pasta: mostrada.clone(),
            musicas: Arc::new(acervo.buscar(consulta, mostrada.as_deref(), MAXIMO_NA_LISTA).unwrap_or_default()),
            total: acervo.total().unwrap_or(0),
            pastas: acervo.contagens().unwrap_or_default(),
            varrendo,
            disponivel: true,
        };
        trocar(lista, nova);
        versao.fetch_add(1, Ordering::Release);
    };
    let varrer = |acervo: &mut Acervo, pasta: &Path, consulta: &str, mostrada: &Option<PathBuf>| {
        publicar(acervo, consulta, mostrada, Some(pasta.to_string_lossy().into_owned()));
        let mut achadas = Vec::new();
        biblioteca::varrer(pasta, &mut |arquivo| achadas.push(biblioteca::ler(arquivo)), &|| {
            !parar.load(Ordering::Acquire)
        });
        // Interrompida no meio (o app está fechando): o que já estava guardado fica como estava.
        if !parar.load(Ordering::Acquire)
            && let Err(e) = acervo.guardar_varredura(pasta, &achadas)
        {
            log::warn!("não foi possível guardar as músicas de {}: {e}", pasta.display());
        }
    };
    publicar(&acervo, &consulta, &mostrada, None);
    // Pastas sem nada guardado (acabaram de entrar, ou o acervo veio da versão anterior): varre já.
    for pasta in acervo.pastas_vazias().unwrap_or_default() {
        varrer(&mut acervo, &pasta, &consulta, &mostrada);
        publicar(&acervo, &consulta, &mostrada, None);
    }
    while let Ok(primeiro) = fila.recv() {
        // Vários pedidos na fila (digitação rápida na busca): atende todos e publica uma lista só no fim.
        let mut pedidos = vec![primeiro];
        pedidos.extend(fila.try_iter());
        for pedido in pedidos {
            match pedido {
                Pedido::Buscar(nova) => consulta = nova,
                Pedido::Mostrar(pasta) => mostrada = pasta,
                Pedido::AdicionarPasta(pasta) => {
                    if let Err(e) = acervo.adicionar_pasta(&pasta) {
                        log::warn!("não foi possível adicionar a pasta: {e}");
                    }
                    // A pasta nova já aparece escolhida.
                    mostrada = Some(pasta.clone());
                    varrer(&mut acervo, &pasta, &consulta, &mostrada);
                }
                Pedido::RemoverPasta(pasta) => {
                    if let Err(e) = acervo.remover_pasta(&pasta) {
                        log::warn!("não foi possível tirar a pasta: {e}");
                    }
                    if mostrada.as_ref() == Some(&pasta) {
                        mostrada = None;
                    }
                }
                Pedido::Atualizar => {
                    for pasta in acervo.pastas().unwrap_or_default() {
                        varrer(&mut acervo, &pasta, &consulta, &mostrada);
                    }
                }
            }
        }
        publicar(&acervo, &consulta, &mostrada, None);
    }
}

/// A capa lida do arquivo, com o arquivo de onde veio (a leitura acontece em outra thread).
type CapaLida = Arc<Mutex<Option<(PathBuf, Arc<Capa>)>>>;

pub struct Local {
    tocador: Tocador,
    pedidos: Option<Sender<Pedido>>,
    lista_viva: Arc<Mutex<Lista>>,
    versao_viva: Arc<AtomicU64>,
    versao_vista: u64,
    parar: Arc<AtomicBool>,
    /// A última lista que a thread da biblioteca entregou.
    pub lista: Lista,
    /// As músicas da fila e qual delas está carregada.
    fila: Vec<Musica>,
    na_fila: Option<usize>,
    /// A situação do tocador neste quadro.
    pub situacao: Situacao,
    terminadas: u64,
    controles: Option<MediaControls>,
    eventos: Receiver<MediaControlEvent>,
    /// O que foi publicado no Windows por último: (arquivo, tocando) e quando.
    publicado: Option<(PathBuf, bool, Instant)>,
    capa: CapaLida,
    /// O título da última música que a fila tocou antes de parar.
    ultimo_titulo: Option<String>,
}

impl Local {
    /// `pasta` é a pasta de dados (sem ela não há biblioteca); `hwnd` é a janela, para o controle de mídia do
    /// Windows; `mudo` toca sem som (para testar).
    pub fn iniciar(pasta: Option<PathBuf>, hwnd: Option<isize>, mudo: bool) -> Self {
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
            pedidos,
            lista_viva,
            versao_viva,
            versao_vista: u64::MAX,
            parar,
            lista: Lista::default(),
            fila: Vec::new(),
            na_fila: None,
            situacao: Situacao::default(),
            terminadas: 0,
            controles,
            eventos,
            publicado: None,
            capa: Arc::default(),
            ultimo_titulo: None,
        }
    }

    /// Depois que a fila acaba, o Windows ainda guarda a última faixa do Player como "parada". Isto diz se uma
    /// faixa lida de lá é esse resto (e não algo que outro app está tocando).
    pub fn e_resto(&self, faixa: &Faixa) -> bool {
        !self.ativo() && faixa.app == NOME_NO_SISTEMA && self.ultimo_titulo.as_deref() == Some(faixa.titulo.as_str())
    }

    fn pedir(&self, pedido: Pedido) {
        if let Some(p) = &self.pedidos {
            let _ = p.send(pedido);
        }
    }

    pub fn buscar(&self, consulta: &str) {
        self.pedir(Pedido::Buscar(consulta.to_string()));
    }

    /// Mostra só as músicas de `pasta` (`None` = todas).
    pub fn mostrar(&mut self, pasta: Option<PathBuf>) {
        // A escolha aparece na hora; a lista chega logo depois, da thread da biblioteca.
        self.lista.pasta.clone_from(&pasta);
        self.pedir(Pedido::Mostrar(pasta));
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

    /// Há uma música local carregada (tocando ou pausada): os controles do app valem para ela.
    pub fn ativo(&self) -> bool {
        self.na_fila.is_some()
    }

    pub fn atual(&self) -> Option<&Musica> {
        self.na_fila.and_then(|i| self.fila.get(i))
    }

    fn tocar_da_fila(&mut self, indice: usize) {
        let Some(musica) = self.fila.get(indice) else {
            self.parar();
            return;
        };
        self.na_fila = Some(indice);
        self.tocador.mandar(Comando::Tocar(musica.caminho.clone()));
        self.carregar_capa(musica.caminho.clone());
        self.publicado = None;
    }

    /// Toca a música `indice` da lista mostrada; as seguintes viram a fila.
    pub fn tocar(&mut self, indice: usize) {
        self.fila = self.lista.musicas.as_ref().clone();
        self.tocar_da_fila(indice);
    }

    pub fn alternar(&mut self) {
        if self.ativo() {
            self.tocador.mandar(if self.situacao.tocando { Comando::Pausar } else { Comando::Continuar });
        }
    }

    pub fn proxima(&mut self) {
        match self.na_fila {
            Some(i) if i + 1 < self.fila.len() => self.tocar_da_fila(i + 1),
            // Fim da fila: para, e o app volta a acompanhar o que os outros apps tocam.
            Some(_) => self.parar(),
            None => {}
        }
    }

    pub fn anterior(&mut self) {
        match self.na_fila {
            Some(i) if i > 0 && self.situacao.posicao_s < VOLTA_AO_COMECO_S => self.tocar_da_fila(i - 1),
            Some(_) => self.ir(0.0),
            None => {}
        }
    }

    pub fn ir(&mut self, segundos: f64) {
        if self.ativo() && segundos.is_finite() {
            self.tocador.mandar(Comando::Ir(segundos.max(0.0)));
            self.publicado = None;
        }
    }

    pub fn parar(&mut self) {
        self.ultimo_titulo = self.atual().map(|m| m.titulo.clone()).or(self.ultimo_titulo.take());
        self.tocador.mandar(Comando::Parar);
        self.na_fila = None;
        self.fila.clear();
        self.publicado = None;
        if let Some(c) = self.controles.as_mut() {
            let _ = c.set_playback(MediaPlayback::Stopped);
        }
    }

    /// Lê a capa embutida no arquivo fora da thread da interface.
    fn carregar_capa(&self, arquivo: PathBuf) {
        let destino = self.capa.clone();
        let lendo = std::thread::Builder::new().name("capa".into()).spawn(move || {
            let capa = biblioteca::capa(&arquivo).and_then(|bytes| Capa::decodificar(&bytes)).map(Arc::new);
            trocar(&destino, capa.map(|c| (arquivo, c)));
        });
        if let Err(e) = lendo {
            log::debug!("capa não lida: {e}");
        }
    }

    /// Um quadro: lê a lista e o tocador, passa para a próxima música quando uma acaba, atende as teclas de
    /// mídia e mantém o Windows informado.
    pub fn quadro(&mut self) {
        let versao = self.versao_viva.load(Ordering::Acquire);
        if versao != self.versao_vista {
            self.versao_vista = versao;
            self.lista = ler(&self.lista_viva);
        }
        self.situacao = self.tocador.situacao();
        if self.situacao.terminadas != self.terminadas {
            self.terminadas = self.situacao.terminadas;
            self.proxima();
        }
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

    /// Conta ao Windows o que está tocando: a faixa quando ela muda, e a posição quando o estado muda (e de
    /// tempos em tempos).
    fn publicar(&mut self) {
        let (Some(musica), Some(controles)) = (self.na_fila.and_then(|i| self.fila.get(i)), self.controles.as_mut())
        else {
            return;
        };
        let tocando = self.situacao.tocando;
        // Só depois que o tocador carregou este arquivo: antes disso a posição ainda é a da música anterior.
        if self.situacao.arquivo.as_deref() != Some(musica.caminho.as_path()) {
            return;
        }
        let mesma_faixa = self.publicado.as_ref().is_some_and(|(arquivo, ..)| *arquivo == musica.caminho);
        let em_dia = self
            .publicado
            .as_ref()
            .is_some_and(|(_, estava, quando)| mesma_faixa && *estava == tocando && quando.elapsed() < REPUBLICAR);
        if em_dia {
            return;
        }
        if !mesma_faixa {
            let duracao = Some(self.situacao.duracao_s).filter(|d| *d > 0.0).map(Duration::from_secs_f64);
            let _ = controles.set_metadata(MediaMetadata {
                title: Some(&musica.titulo),
                artist: Some(&musica.artista).filter(|a| !a.is_empty()).map(String::as_str),
                album: Some(&musica.album).filter(|a| !a.is_empty()).map(String::as_str),
                duration: duracao,
                cover_url: None,
            });
        }
        let progresso = Some(MediaPosition(Duration::from_secs_f64(self.situacao.posicao_s.max(0.0))));
        let _ = controles.set_playback(if tocando {
            MediaPlayback::Playing { progress: progresso }
        } else {
            MediaPlayback::Paused { progress: progresso }
        });
        self.publicado = Some((musica.caminho.clone(), tocando, Instant::now()));
    }

    /// O "tocando agora" da música local, no mesmo formato do que vem dos outros apps. `None` quando não há
    /// música local carregada.
    pub fn estado_de_midia(&self) -> Option<Estado> {
        let musica = self.atual()?;
        let capa = match ler(&self.capa) {
            Some((arquivo, capa)) if arquivo == musica.caminho => Some(capa),
            _ => None,
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
            posicao: Some(self.situacao.duracao_s.max(musica.duracao_s))
                .filter(|d| *d > 1.0)
                .map(|duracao_s| Posicao { posicao_s: self.situacao.posicao_s, duracao_s, lida_em: Instant::now() }),
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
