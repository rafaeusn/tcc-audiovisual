//! Provedor do YouTube Music (F3): conversa com o Player pelo protocolo de provedores e com o YouTube Music pela
//! API interna dele (InnerTube).
//!
//! - Busca, início (mais tocadas e em alta), álbuns, artistas e playlists: pelo `rustypipe` (GPL-3.0), como o
//!   cliente web do YouTube Music.
//! - O áudio: pela rota direta ([`visionos`]), o cliente do YouTube para visionOS, sem conta, como o MixMusic e o
//!   limusic fazem. Os endereços dele tocam como vêm (sem decifrar nada e sem PO token); o áudio é o AAC (`m4a`),
//!   que o motor do Player decodifica. Nada roda código do YouTube aqui.
//!
//! `provedor-youtube-music --teste "<busca>"` mostra os resultados e o endereço da primeira faixa, para conferir
//! sem o Player.

mod conversao;
mod visionos;

use conversao::{album_item, faixa};
use provedores::{
    Apresentacao, Capacidade, CorpoDaResposta, CorpoDoPedido, Fonte, Item, Prateleira, Servico, VERSAO_DO_PROTOCOLO,
    servir,
};
use rustypipe::client::RustyPipe;
use std::future::Future;

struct YoutubeMusic {
    rt: tokio::runtime::Runtime,
    rp: RustyPipe,
    direto: visionos::Direto,
}

fn erro(e: impl std::fmt::Display) -> CorpoDaResposta {
    CorpoDaResposta::Erro { mensagem: e.to_string() }
}

impl YoutubeMusic {
    fn novo() -> Result<Self, String> {
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
        // O Player passa uma pasta própria para cada provedor guardar o que quiser (aqui, o cache do rustypipe).
        let mut construtor = RustyPipe::builder();
        construtor = match std::env::var_os("TCCPLAYER_PROVEDOR_DADOS") {
            Some(pasta) => {
                let _ = std::fs::create_dir_all(&pasta);
                construtor.storage_dir(pasta)
            }
            None => construtor.no_storage(),
        };
        let rp = construtor.build().map_err(|e| e.to_string())?;
        Ok(Self { rt, rp, direto: visionos::Direto::novo()? })
    }

    fn esperar<T>(&self, f: impl Future<Output = T>) -> T {
        self.rt.block_on(f)
    }

    fn buscar(&self, texto: &str, limite: u32) -> CorpoDaResposta {
        match self.esperar(self.rp.query().music_search_tracks(texto)) {
            Ok(r) => CorpoDaResposta::Itens {
                itens: r.items.items.iter().take(limite as usize).map(|t| Item::Faixa(faixa(t))).collect(),
            },
            Err(e) => erro(e),
        }
    }

    fn inicio(&self) -> CorpoDaResposta {
        match self.esperar(self.rp.query().music_charts(None)) {
            Ok(c) => {
                let fileira = |titulo: &str, faixas: &[rustypipe::model::TrackItem]| Prateleira {
                    titulo: titulo.into(),
                    itens: faixas.iter().map(|t| Item::Faixa(faixa(t))).collect(),
                };
                let prateleiras = [fileira("Mais tocadas", &c.top_tracks), fileira("Em alta", &c.trending_tracks)]
                    .into_iter()
                    .filter(|p| !p.itens.is_empty())
                    .collect();
                CorpoDaResposta::Prateleiras { prateleiras }
            }
            Err(e) => erro(e),
        }
    }

    fn album(&self, id: &str) -> CorpoDaResposta {
        match self.esperar(self.rp.query().music_album(id)) {
            Ok(a) => {
                let capa = conversao::capa(&a.cover);
                let itens = a
                    .tracks
                    .iter()
                    .map(|t| {
                        let mut f = faixa(t);
                        // Dentro do álbum, as faixas vêm sem o álbum e às vezes sem capa: herdam as dele.
                        f.album.get_or_insert_with(|| a.name.clone());
                        if f.capa.is_none() {
                            f.capa.clone_from(&capa);
                        }
                        Item::Faixa(f)
                    })
                    .collect();
                CorpoDaResposta::Itens { itens }
            }
            Err(e) => erro(e),
        }
    }

    fn playlist(&self, id: &str) -> CorpoDaResposta {
        match self.esperar(self.rp.query().music_playlist(id)) {
            Ok(p) => CorpoDaResposta::Itens { itens: p.tracks.items.iter().map(|t| Item::Faixa(faixa(t))).collect() },
            Err(e) => erro(e),
        }
    }

    fn artista(&self, id: &str) -> CorpoDaResposta {
        match self.esperar(self.rp.query().music_artist(id, false)) {
            Ok(a) => {
                let mut itens: Vec<Item> = a.tracks.iter().map(|t| Item::Faixa(faixa(t))).collect();
                itens.extend(a.albums.iter().map(|al| Item::Album(album_item(al))));
                CorpoDaResposta::Itens { itens }
            }
            Err(e) => erro(e),
        }
    }

    /// O endereço do áudio, pela rota direta. Uma faixa sem endereço direto (com DRM, por exemplo) não toca.
    fn resolver(&mut self, id: &str) -> CorpoDaResposta {
        match self.rt.block_on(self.direto.audio(id)) {
            // O googlevideo serve os endereços desse cliente para o agente dele.
            Ok(a) => CorpoDaResposta::Fonte(Fonte::Endereco {
                url: a.url,
                cabecalhos: vec![("User-Agent".into(), visionos::AGENTE.into())],
                formato: Some("m4a".into()),
                duracao_s: a.duracao_s,
            }),
            Err(motivo) => erro(format!("o YouTube Music não deu o áudio desta faixa: {motivo}")),
        }
    }
}

impl Servico for YoutubeMusic {
    fn apresentacao(&self) -> Apresentacao {
        Apresentacao {
            nome: "YouTube Music".into(),
            servico: "youtube-music".into(),
            versao: env!("CARGO_PKG_VERSION").into(),
            protocolo: VERSAO_DO_PROTOCOLO,
            capacidades: vec![
                Capacidade::Busca,
                Capacidade::Inicio,
                Capacidade::Album,
                Capacidade::Artista,
                Capacidade::Playlists,
                Capacidade::Audio,
            ],
        }
    }

    fn atender(&mut self, pedido: CorpoDoPedido) -> CorpoDaResposta {
        match pedido {
            CorpoDoPedido::Buscar { texto, limite } => self.buscar(&texto, limite),
            CorpoDoPedido::Inicio => self.inicio(),
            CorpoDoPedido::Album { album } => self.album(&album),
            CorpoDoPedido::Playlist { playlist } => self.playlist(&playlist),
            CorpoDoPedido::Artista { artista } => self.artista(&artista),
            CorpoDoPedido::Resolver { faixa } => self.resolver(&faixa),
            outro => erro(format!("o provedor do YouTube Music não sabe responder {outro:?}")),
        }
    }
}

/// O modo de teste: busca, mostra os resultados e resolve a primeira faixa.
fn testar(yt: &mut YoutubeMusic, texto: &str) {
    let CorpoDaResposta::Itens { itens } = yt.buscar(texto, 5) else {
        eprintln!("a busca falhou: {:?}", yt.buscar(texto, 5));
        return;
    };
    for i in &itens {
        if let Some(f) = i.faixa() {
            println!("{}  {} — {}  ({:?} s)", f.id, f.titulo, f.artistas_juntos(), f.duracao_s);
        }
    }
    let Some(primeira) = itens.first().and_then(Item::faixa) else { return };
    match yt.resolver(&primeira.id) {
        CorpoDaResposta::Fonte(Fonte::Endereco { url, duracao_s, cabecalhos, .. }) => {
            for (nome, valor) in &cabecalhos {
                println!("cabeçalho {nome}: {valor}");
            }
            println!("endereço da primeira ({duracao_s:?} s):\n{url}");
        }
        outra => println!("não resolveu: {outra:?}"),
    }
}

fn main() {
    let mut yt = match YoutubeMusic::novo() {
        Ok(y) => y,
        Err(e) => {
            eprintln!("o provedor do YouTube Music não abriu: {e}");
            std::process::exit(1);
        }
    };
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--teste") {
        testar(&mut yt, args.get(i + 1).map_or("daft punk", String::as_str));
        return;
    }
    if let Err(e) = servir(std::io::stdin().lock(), std::io::stdout().lock(), &mut yt) {
        eprintln!("youtube-music: {e}");
    }
}
