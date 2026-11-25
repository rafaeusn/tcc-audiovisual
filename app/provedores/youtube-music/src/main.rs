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

use conversao::{album_item, artista_item, faixa, playlist_item};
use provedores::{
    Apresentacao, Capacidade, Causa, CorpoDaResposta, CorpoDoPedido, Fonte, Identidade, Item, Prateleira, Servico,
    VERSAO_DO_PROTOCOLO, servir,
};
use rustypipe::client::RustyPipe;
use rustypipe::error::Error;
use std::future::Future;

/// Quanto o áudio de uma faixa pode demorar ao todo (o Player espera 20 s: o motivo tem de chegar antes).
const PRAZO_DO_AUDIO: std::time::Duration = std::time::Duration::from_secs(12);
/// Quanto cada uma das buscas de uma pesquisa pode demorar (o Player espera 20 s pela resposta inteira).
const PRAZO_DE_CADA_BUSCA: std::time::Duration = std::time::Duration::from_secs(8);

struct YoutubeMusic {
    rt: tokio::runtime::Runtime,
    rp: RustyPipe,
    direto: visionos::Direto,
}

/// Um item da busca principal (os perfis de usuário ficam de fora).
fn item_da_busca(i: &rustypipe::model::MusicItem) -> Option<Item> {
    use rustypipe::model::MusicItem;
    match i {
        MusicItem::Track(t) => Some(Item::Faixa(faixa(t))),
        MusicItem::Album(a) => Some(Item::Album(album_item(a))),
        MusicItem::Artist(a) => Some(Item::Artista(artista_item(a))),
        MusicItem::Playlist(p) => Some(Item::Playlist(playlist_item(p))),
        MusicItem::User(_) => None,
    }
}

fn erro(e: impl std::fmt::Display) -> CorpoDaResposta {
    CorpoDaResposta::erro(e.to_string())
}

/// O erro do rustypipe, com a causa que ele deixa ver.
fn erro_do_youtube(e: &Error) -> CorpoDaResposta {
    CorpoDaResposta::Erro { mensagem: e.to_string(), causa: causa_de(e) }
}

/// De que tipo foi o erro: a rede e o YouTube pedindo calma são passageiros; o que não existe mais é do item.
fn causa_de(e: &Error) -> Option<Causa> {
    use rustypipe::error::ExtractionError as E;
    match e {
        Error::Http(_) => Some(Causa::Rede),
        Error::HttpStatus(status, _) => visionos::falha_do_status(*status).causa,
        Error::Extraction(E::NotFound { .. } | E::Unavailable { .. }) => Some(Causa::Indisponivel),
        Error::Extraction(E::InvalidData(_)) => Some(Causa::Quebrado),
        _ => None,
    }
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

    /// A busca de todos os tipos: a principal (que traz o melhor resultado primeiro) e uma por tipo, ao mesmo tempo.
    /// A que falha, ou passa de [`PRAZO_DE_CADA_BUSCA`], só fica de fora (uma lenta não segura as outras nem estoura
    /// o prazo do Player); erro só se todas falharem.
    fn buscar(&self, texto: &str, limite: u32) -> CorpoDaResposta {
        use tokio::time::timeout;
        let q = self.rp.query();
        let (principal, faixas, albuns, artistas, playlists) = self.esperar(async {
            tokio::join!(
                timeout(PRAZO_DE_CADA_BUSCA, q.music_search_main(texto)),
                timeout(PRAZO_DE_CADA_BUSCA, q.music_search_tracks(texto)),
                timeout(PRAZO_DE_CADA_BUSCA, q.music_search_albums(texto)),
                timeout(PRAZO_DE_CADA_BUSCA, q.music_search_artists(texto)),
                timeout(PRAZO_DE_CADA_BUSCA, q.music_search_playlists(texto, false)),
            )
        });
        let mut falhas = Vec::new();
        let mut causa = None;
        let mut lista = |r: Result<Result<Vec<Item>, Error>, tokio::time::error::Elapsed>| {
            let (motivo, de) = match r {
                Ok(Ok(itens)) => return itens,
                Ok(Err(e)) => (e.to_string(), causa_de(&e)),
                Err(_) => ("demorou demais".to_string(), Some(Causa::Rede)),
            };
            falhas.push(motivo);
            causa = causa.or(de);
            Vec::new()
        };
        let principal = lista(principal.map(|r| r.map(|r| r.items.items.iter().filter_map(item_da_busca).collect())));
        let por_tipo = vec![
            lista(faixas.map(|r| r.map(|r| r.items.items.iter().map(|t| Item::Faixa(faixa(t))).collect()))),
            lista(albuns.map(|r| r.map(|r| r.items.items.iter().map(|a| Item::Album(album_item(a))).collect()))),
            lista(artistas.map(|r| r.map(|r| r.items.items.iter().map(|a| Item::Artista(artista_item(a))).collect()))),
            lista(
                playlists.map(|r| r.map(|r| r.items.items.iter().map(|p| Item::Playlist(playlist_item(p))).collect())),
            ),
        ];
        if falhas.len() == 5 {
            return CorpoDaResposta::Erro { mensagem: falhas.join("; "), causa };
        }
        CorpoDaResposta::Itens { itens: conversao::juntar_busca(principal, por_tipo, limite as usize) }
    }

    /// O início: lançamentos, clipes novos e as paradas (faixas, artistas e playlists). Cada fonte é pedida à
    /// parte; a que falha ou vem vazia só fica de fora (as paradas, por exemplo, às vezes vêm vazias).
    fn inicio(&self) -> CorpoDaResposta {
        let fileira = |titulo: &str, itens: Vec<Item>| Prateleira { titulo: titulo.into(), itens };
        let mut prateleiras = Vec::new();
        let mut falhas = Vec::new();
        match self.esperar(self.rp.query().music_new_albums()) {
            Ok(a) => prateleiras.push(fileira("Lançamentos", a.iter().map(|x| Item::Album(album_item(x))).collect())),
            Err(e) => falhas.push(e.to_string()),
        }
        match self.esperar(self.rp.query().music_new_videos()) {
            Ok(v) => prateleiras.push(fileira("Clipes novos", v.iter().map(|t| Item::Faixa(faixa(t))).collect())),
            Err(e) => falhas.push(e.to_string()),
        }
        match self.esperar(self.rp.query().music_charts(None)) {
            Ok(c) => {
                let faixas = |v: &[rustypipe::model::TrackItem]| v.iter().map(|t| Item::Faixa(faixa(t))).collect();
                prateleiras.push(fileira("Mais tocadas", faixas(&c.top_tracks)));
                prateleiras.push(fileira("Em alta", faixas(&c.trending_tracks)));
                prateleiras.push(fileira(
                    "Artistas em alta",
                    c.artists.iter().map(|a| Item::Artista(artista_item(a))).collect(),
                ));
                prateleiras
                    .push(fileira("Paradas", c.playlists.iter().map(|p| Item::Playlist(playlist_item(p))).collect()));
            }
            Err(e) => falhas.push(e.to_string()),
        }
        prateleiras.retain(|p| !p.itens.is_empty());
        if prateleiras.is_empty() && !falhas.is_empty() {
            return erro(falhas.join("; "));
        }
        CorpoDaResposta::Prateleiras { prateleiras }
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
                        f.album_id.get_or_insert_with(|| id.to_string());
                        if f.capa.is_none() {
                            f.capa.clone_from(&capa);
                        }
                        Item::Faixa(f)
                    })
                    .collect();
                CorpoDaResposta::Itens { itens }
            }
            Err(e) => erro_do_youtube(&e),
        }
    }

    fn playlist(&self, id: &str) -> CorpoDaResposta {
        match self.esperar(self.rp.query().music_playlist(id)) {
            Ok(p) => CorpoDaResposta::Itens { itens: p.tracks.items.iter().map(|t| Item::Faixa(faixa(t))).collect() },
            Err(e) => erro_do_youtube(&e),
        }
    }

    fn artista(&self, id: &str) -> CorpoDaResposta {
        match self.esperar(self.rp.query().music_artist(id, false)) {
            Ok(a) => {
                let mut itens: Vec<Item> = a.tracks.iter().map(|t| Item::Faixa(faixa(t))).collect();
                itens.extend(a.albums.iter().map(|al| Item::Album(album_item(al))));
                CorpoDaResposta::Itens { itens }
            }
            Err(e) => erro_do_youtube(&e),
        }
    }

    /// O endereço do áudio, pela rota direta. Uma faixa sem endereço direto (com DRM, por exemplo) não toca.
    fn resolver(&mut self, id: &str) -> CorpoDaResposta {
        let audio = self.rt.block_on(async {
            let demorou = |_| visionos::Falha::nova(Causa::Rede, "o YouTube demorou demais");
            tokio::time::timeout(PRAZO_DO_AUDIO, self.direto.audio(id)).await.map_err(demorou).and_then(|r| r)
        });
        match audio {
            // O googlevideo serve os endereços desse cliente para o agente dele.
            Ok(a) => CorpoDaResposta::Fonte(Fonte::Endereco {
                url: a.url,
                cabecalhos: vec![("User-Agent".into(), visionos::AGENTE.into())],
                formato: Some("m4a".into()),
                duracao_s: a.duracao_s,
            }),
            Err(f) => CorpoDaResposta::Erro {
                mensagem: format!("o YouTube Music não deu o áudio desta faixa: {}", f.motivo),
                causa: f.causa,
            },
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
            identidade: Some(Identidade {
                cor: Some(conversao::COR.into()),
                logo: Some(include_str!("logo.svg").into()),
            }),
            conta: None,
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
        match i {
            Item::Faixa(f) => {
                println!("faixa    {}  {} — {}  ({:?} s)", f.id, f.titulo, f.artistas_juntos(), f.duracao_s)
            }
            Item::Album(a) => println!("álbum    {}  {} — {}", a.id, a.titulo, a.artistas.join(", ")),
            Item::Artista(a) => println!("artista  {}  {}", a.id, a.nome),
            Item::Playlist(p) => println!("playlist {}  {}", p.id, p.titulo),
        }
    }
    let faixas: Vec<_> = itens.iter().filter_map(Item::faixa).collect();
    let com_album = faixas.iter().filter(|f| f.album_id.is_some()).count();
    let com_artista = faixas.iter().filter(|f| f.id_do_artista(0).is_some()).count();
    println!("ids: {com_album} de {} com álbum, {com_artista} com artista", faixas.len());
    // A primeira faixa (o melhor resultado, o primeiro item, pode ser um artista ou um álbum).
    let Some(primeira) = itens.iter().find_map(Item::faixa) else { return };
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
    if let Err(e) = servir(std::io::stdin().lock(), std::io::stdout(), &mut yt) {
        eprintln!("youtube-music: {e}");
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use rustypipe::error::ExtractionError;

    #[test]
    fn a_causa_dos_erros_do_youtube() {
        assert_eq!(causa_de(&Error::Http("sem conexão".into())), Some(Causa::Rede));
        assert_eq!(causa_de(&Error::HttpStatus(429, "".into())), Some(Causa::Limite));
        assert_eq!(causa_de(&Error::HttpStatus(502, "".into())), Some(Causa::Rede));
        let sumiu = ExtractionError::NotFound { id: "x".into(), msg: "".into() };
        assert_eq!(causa_de(&Error::Extraction(sumiu)), Some(Causa::Indisponivel));
        assert_eq!(causa_de(&Error::Other("?".into())), None);
    }
}
