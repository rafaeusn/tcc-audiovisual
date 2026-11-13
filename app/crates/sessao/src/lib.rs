//! "Tocando agora" e controles (tocar, pausar, próxima, anterior) pela sessão de mídia do Windows (SMTC).
//!
//! É a mesma informação que aparece na caixinha de mídia do Windows. O app lê o que o Spotify, o navegador ou
//! qualquer player publicam ali e envia os mesmos comandos das teclas de mídia. Nenhuma API de serviço é usada.
//!
//! Uma thread própria consulta o sistema e publica um [`Estado`]. A interface só lê: nunca espera o Windows.

mod capa;
#[cfg(windows)]
mod smtc;

pub use capa::Capa;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Controle {
    TocarPausar,
    Proxima,
    Anterior,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Faixa {
    pub titulo: String,
    pub artista: String,
    pub album: String,
    /// Nome legível do app de origem ("Spotify", "Chrome"...).
    pub app: String,
}

impl Faixa {
    /// Identidade da faixa para saber quando a música mudou.
    pub fn chave(&self) -> (&str, &str, &str) {
        (&self.titulo, &self.artista, &self.app)
    }
}

/// Posição informada pelo app de origem, com o instante em que foi lida.
#[derive(Clone, Copy, Debug)]
pub struct Posicao {
    pub posicao_s: f64,
    pub duracao_s: f64,
    pub lida_em: Instant,
}

impl Posicao {
    /// Posição estimada agora: os apps atualizam o valor só de vez em quando, então o relógio completa.
    pub fn agora(&self, tocando: bool) -> f64 {
        let extra = if tocando { self.lida_em.elapsed().as_secs_f64() } else { 0.0 };
        (self.posicao_s + extra).clamp(0.0, self.duracao_s.max(0.0))
    }
}

#[derive(Clone, Debug, Default)]
pub struct Estado {
    pub faixa: Option<Faixa>,
    pub capa: Option<Arc<Capa>>,
    pub tocando: bool,
    pub posicao: Option<Posicao>,
    /// Falso quando o sistema não oferece sessões de mídia (ou a leitura falhou).
    pub disponivel: bool,
}

/// Leitura do estado a partir de outra thread (a do histórico). Clonar é barato; não mantém a sessão viva.
#[derive(Clone)]
pub struct Leitor {
    estado: Arc<Mutex<Estado>>,
}

impl Leitor {
    pub fn estado(&self) -> Estado {
        match self.estado.lock() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }
}

/// Envia comandos ao app de origem a partir de outro lugar (os botões da barra de tarefas e da bandeja).
#[derive(Clone)]
pub struct Controlador(Sender<Controle>);

impl Controlador {
    pub fn controlar(&self, c: Controle) {
        let _ = self.0.send(c);
    }
}

/// Ligação com a thread da sessão. Soltar o valor encerra a thread.
pub struct Sessao {
    estado: Arc<Mutex<Estado>>,
    versao: Arc<AtomicU64>,
    controles: Sender<Controle>,
    /// Nome do app a seguir ("Spotify"); vazio = o app que o Windows considera atual.
    filtro: Arc<Mutex<String>>,
}

impl Sessao {
    pub fn iniciar() -> Self {
        let estado = Arc::new(Mutex::new(Estado::default()));
        let versao = Arc::new(AtomicU64::new(0));
        let (controles, fila) = channel();
        let filtro = Arc::new(Mutex::new(String::new()));
        #[cfg(windows)]
        {
            let (e, v, f) = (estado.clone(), versao.clone(), filtro.clone());
            let criada = std::thread::Builder::new().name("sessao".into()).spawn(move || smtc::rodar(e, v, fila, f));
            if let Err(e) = criada {
                log::error!("não foi possível iniciar a thread da sessão de mídia: {e}");
            }
        }
        #[cfg(not(windows))]
        drop(fila);
        Self { estado, versao, controles, filtro }
    }

    /// Passa a seguir só o app `nome` (como [`nome_do_app`] o devolve), mais o próprio Player. Vazio volta a
    /// seguir o app atual do Windows.
    pub fn filtrar(&self, nome: &str) {
        match self.filtro.lock() {
            Ok(mut g) => nome.clone_into(&mut g),
            Err(p) => nome.clone_into(&mut p.into_inner()),
        }
    }

    pub fn controlador(&self) -> Controlador {
        Controlador(self.controles.clone())
    }

    pub fn leitor(&self) -> Leitor {
        Leitor { estado: self.estado.clone() }
    }

    /// Número que muda sempre que o estado muda. Comparar é mais barato que copiar o estado a cada quadro.
    pub fn versao(&self) -> u64 {
        self.versao.load(Ordering::Acquire)
    }

    pub fn estado(&self) -> Estado {
        // Um `lock` envenenado só acontece se a thread da sessão entrou em pânico: o último estado ainda vale.
        match self.estado.lock() {
            Ok(g) => g.clone(),
            Err(p) => p.into_inner().clone(),
        }
    }

    /// Envia um comando ao app de origem. Não espera a resposta.
    pub fn controlar(&self, c: Controle) {
        if self.controles.send(c).is_err() {
            log::warn!("sessão de mídia encerrada: comando {c:?} ignorado");
        }
    }
}

/// Nome legível do app a partir do identificador que o Windows informa.
pub fn nome_do_app(id: &str) -> String {
    let baixo = id.to_lowercase();
    const CONHECIDOS: [(&str, &str); 14] = [
        ("spotify", "Spotify"),
        ("chrome", "Chrome"),
        ("msedge", "Edge"),
        ("firefox", "Firefox"),
        ("308046b0af4a39cb", "Firefox"),
        ("opera", "Opera"),
        ("brave", "Brave"),
        ("applemusic", "Apple Music"),
        ("itunes", "iTunes"),
        ("tidal", "TIDAL"),
        ("deezer", "Deezer"),
        ("vlc", "VLC"),
        ("foobar", "foobar2000"),
        ("zunemusic", "Reprodutor de Mídia"),
    ];
    if let Some((_, nome)) = CONHECIDOS.iter().find(|(chave, _)| baixo.contains(chave)) {
        return (*nome).to_string();
    }
    // Sem correspondência: usa o nome do executável sem a extensão, com a primeira letra maiúscula.
    let base = id.rsplit(['\\', '/', '!']).next().unwrap_or(id);
    let base = base.strip_suffix(".exe").or_else(|| base.strip_suffix(".EXE")).unwrap_or(base);
    let mut letras = base.chars();
    match letras.next() {
        Some(p) if !base.is_empty() => p.to_uppercase().chain(letras).take(40).collect(),
        _ => "App".to_string(),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use std::time::Duration;

    #[test]
    fn reconhece_apps_comuns() {
        assert_eq!(nome_do_app("Spotify.exe"), "Spotify");
        assert_eq!(nome_do_app("SpotifyAB.SpotifyMusic_zpdnekdrzrea0!Spotify"), "Spotify");
        assert_eq!(nome_do_app("chrome.exe"), "Chrome");
        assert_eq!(nome_do_app("MSEdge"), "Edge");
        assert_eq!(nome_do_app("308046B0AF4A39CB"), "Firefox");
        assert_eq!(nome_do_app("C:\\Apps\\meuplayer.exe"), "Meuplayer");
        assert_eq!(nome_do_app(""), "App");
    }

    #[test]
    fn posicao_anda_so_quando_toca() {
        let p = Posicao { posicao_s: 10.0, duracao_s: 12.0, lida_em: Instant::now() - Duration::from_secs(1) };
        assert!((p.agora(true) - 11.0).abs() < 0.2);
        assert_eq!(p.agora(false), 10.0);
        let fim = Posicao { posicao_s: 11.9, duracao_s: 12.0, lida_em: Instant::now() - Duration::from_secs(5) };
        assert_eq!(fim.agora(true), 12.0);
    }
}
