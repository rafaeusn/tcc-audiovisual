//! Os dados do Spotify: cada página do Player é uma ou mais consultas ([`Consultas`]) e a conversão delas
//! (`conversao.rs`). Se a sessão caiu, reentra uma vez e tenta de novo.

use crate::conta::Login;
use crate::conversao::{self, Pagina};
use crate::pathfinder::Falha;
use provedores::{Detalhe, Faixa, Item, Prateleira};
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Quantas faixas se carregam no máximo numa página, e quantos lançamentos e itens da biblioteca.
pub const MAXIMO: u32 = 2000;
const MAXIMO_DE_LANCAMENTOS: u32 = 200;
const MAXIMO_DA_BIBLIOTECA: u32 = 1000;
/// Quanto um pedido inteiro pode demorar (o Player espera 20 s; depois disso, ninguém lê a resposta, e o pedido
/// seguinte espera à toa).
const PRAZO: Duration = Duration::from_secs(16);
/// Quanto disso esperar a sessão voltar (ela continua voltando em segundo plano).
const PRAZO_DA_REENTRADA: Duration = Duration::from_secs(8);
const POR_PAGINA: u32 = 100;
const POR_PAGINA_DE_ALBUM: u32 = 50;
const POR_PAGINA_DA_BIBLIOTECA: u32 = 50;
const POR_PAGINA_DA_DISCOGRAFIA: u32 = 50;

pub type Pedido = (&'static str, Value);

/// Os itens de uma página e o que mais se sabe dela.
pub type ComDetalhe = (Vec<Item>, Detalhe);

/// Quem responde as consultas: o Pathfinder com a sessão de verdade, ou um de mentira nos testes.
pub trait Consultas: Send + Sync {
    /// Faz os pedidos (alguns por vez), até `ate`, e devolve as respostas na mesma ordem.
    fn consultar(&self, pedidos: &[Pedido], ate: Instant) -> Vec<Result<Value, Falha>>;
}

pub struct Dados {
    fonte: Box<dyn Consultas>,
    login: Arc<Login>,
}

/// As posições das páginas depois da primeira (de `por_pagina` em `por_pagina`, até o total ou o máximo).
pub fn paginas_que_faltam(total: u32, por_pagina: u32, maximo: u32) -> Vec<u32> {
    if por_pagina == 0 {
        return Vec::new();
    }
    (por_pagina..total.min(maximo)).step_by(por_pagina as usize).collect()
}

/// Um id do Spotify (letras e números).
fn conferir(id: &str) -> Result<(), String> {
    let vale = !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric());
    if vale { Ok(()) } else { Err("este endereço não é do Spotify".into()) }
}

const CORTADA: &str = "A lista veio cortada do Spotify. Tente de novo.";

/// Uma lista em páginas: a operação, o tamanho da página e até quantos itens.
struct Lista {
    op: &'static str,
    por_pagina: u32,
    maximo: u32,
}

impl Dados {
    pub fn novo(fonte: Box<dyn Consultas>, login: Arc<Login>) -> Self {
        Self { fonte, login }
    }

    fn prazo() -> Instant {
        Instant::now() + PRAZO
    }

    /// Faz as consultas; se a sessão caiu, reentra uma vez e faz de novo.
    fn consultar(&self, pedidos: &[Pedido], ate: Instant) -> Vec<Result<Value, String>> {
        if pedidos.is_empty() {
            return Vec::new();
        }
        let mut respostas = self.fonte.consultar(pedidos, ate);
        if respostas.iter().any(|r| matches!(r, Err(Falha::Sessao))) {
            let espera = ate.saturating_duration_since(Instant::now()).min(PRAZO_DA_REENTRADA);
            if let Err(motivo) = self.login.reconectar(espera) {
                return pedidos.iter().map(|_| Err(motivo.clone())).collect();
            }
            respostas = self.fonte.consultar(pedidos, ate);
        }
        respostas.into_iter().map(|r| r.map_err(|f| f.frase())).collect()
    }

    fn uma(&self, op: &'static str, variaveis: Value, ate: Instant) -> Result<Value, String> {
        self.consultar(&[(op, variaveis)], ate).into_iter().next().unwrap_or_else(|| Err("sem resposta".into()))
    }

    /// Todas as páginas, em ordem, até `maximo` itens. Uma que falha (ou vem vazia no meio) derruba a lista: cortada,
    /// ela pareceria inteira.
    fn em_paginas<T>(
        &self,
        lista: &Lista,
        ate: Instant,
        variaveis: impl Fn(u32) -> Value,
        ler: impl Fn(&Value) -> Result<Pagina<T>, String>,
    ) -> Result<Vec<T>, String> {
        let primeira = ler(&self.uma(lista.op, variaveis(0), ate)?)?;
        if primeira.vieram == 0 && primeira.total > 0 {
            return Err(CORTADA.into());
        }
        // O Spotify pode mandar páginas menores do que as pedidas: o passo é o que veio.
        let passo = primeira.vieram.min(lista.por_pagina);
        let resto: Vec<Pedido> = paginas_que_faltam(primeira.total, passo, lista.maximo)
            .into_iter()
            .map(|de| (lista.op, variaveis(de)))
            .collect();
        let mut itens = primeira.itens;
        for resposta in self.consultar(&resto, ate) {
            let pagina = ler(&resposta?)?;
            if pagina.vieram == 0 {
                return Err(CORTADA.into());
            }
            itens.extend(pagina.itens);
        }
        itens.truncate(lista.maximo as usize);
        Ok(itens)
    }

    pub fn buscar(&self, texto: &str, limite: u32) -> Result<Vec<Item>, String> {
        let data = self.uma(
            "searchDesktop",
            json!({
                "searchTerm": texto, "offset": 0, "limit": limite.clamp(1, 50), "numberOfTopResults": 5,
                "includeAudiobooks": false, "includeArtistHasConcertsField": false, "includePreReleases": false,
                "includeLocalConcertsField": false, "includeAuthors": false,
            }),
            Self::prazo(),
        )?;
        conversao::busca(&data, limite)
    }

    pub fn inicio(&self) -> Result<Vec<Prateleira>, String> {
        let data = self.uma(
            "home",
            json!({
                "timeZone": "America/Sao_Paulo", "sp_t": "", "facet": "", "sectionItemsLimit": 20,
                "homeEndUserIntegration": "INTEGRATION_WEB_PLAYER",
            }),
            Self::prazo(),
        )?;
        conversao::inicio(&data)
    }

    pub fn album(&self, id: &str) -> Result<ComDetalhe, String> {
        conferir(id)?;
        let lista = Lista { op: "getAlbum", por_pagina: POR_PAGINA_DE_ALBUM, maximo: MAXIMO };
        let pagina = |de: u32| json!({"uri": format!("spotify:album:{id}"), "locale": "", "offset": de, "limit": POR_PAGINA_DE_ALBUM});
        self.com_detalhe(&lista, pagina, conversao::album, conversao::detalhe_do_album)
    }

    /// As faixas de todas as páginas, com o detalhe tirado da primeira.
    fn com_detalhe(
        &self,
        lista: &Lista,
        variaveis: impl Fn(u32) -> Value,
        ler: impl Fn(&Value) -> Result<Pagina<Faixa>, String>,
        detalhar: impl Fn(&Value) -> Detalhe,
    ) -> Result<ComDetalhe, String> {
        let detalhe = std::cell::OnceCell::new();
        let faixas = self.em_paginas(lista, Self::prazo(), variaveis, |v| {
            detalhe.get_or_init(|| detalhar(v));
            ler(v)
        })?;
        Ok((em_itens(faixas), detalhe.into_inner().unwrap_or_default()))
    }

    /// As populares e os lançamentos. Sem os lançamentos, as populares valem sozinhas.
    pub fn artista(&self, id: &str) -> Result<ComDetalhe, String> {
        conferir(id)?;
        let ate = Self::prazo();
        let endereco = format!("spotify:artist:{id}");
        let variaveis = json!({"uri": endereco, "locale": "", "preReleaseV2": false});
        let resumo = self.uma("queryArtistOverview", variaveis, ate)?;
        let mut itens = em_itens(conversao::populares(&resumo)?);
        let detalhe = conversao::detalhe_do_artista(&resumo);
        let lista = Lista {
            op: "queryArtistDiscographyAll",
            por_pagina: POR_PAGINA_DA_DISCOGRAFIA,
            maximo: MAXIMO_DE_LANCAMENTOS,
        };
        let pagina =
            |de: u32| json!({"uri": endereco, "offset": de, "limit": POR_PAGINA_DA_DISCOGRAFIA, "order": "DATE_DESC"});
        match self.em_paginas(&lista, ate, pagina, conversao::discografia) {
            Ok(albuns) => itens.extend(albuns.into_iter().map(Item::Album)),
            Err(motivo) if itens.is_empty() => return Err(motivo),
            Err(_) => eprintln!("spotify: a discografia de um artista não veio"),
        }
        Ok((itens, detalhe))
    }

    pub fn playlist(&self, id: &str) -> Result<ComDetalhe, String> {
        let ate = Self::prazo();
        if id == conversao::CURTIDAS {
            let lista = Lista { op: "fetchLibraryTracks", por_pagina: POR_PAGINA, maximo: MAXIMO };
            let pagina = |de: u32| json!({"offset": de, "limit": POR_PAGINA});
            let faixas = self.em_paginas(&lista, ate, pagina, conversao::curtidas)?;
            return Ok((em_itens(faixas), Detalhe::default()));
        }
        conferir(id)?;
        let lista = Lista { op: "fetchPlaylist", por_pagina: POR_PAGINA, maximo: MAXIMO };
        let pagina = |de: u32| {
            json!({"uri": format!("spotify:playlist:{id}"), "offset": de, "limit": POR_PAGINA,
                "enableWatchFeedEntrypoint": false})
        };
        self.com_detalhe(&lista, pagina, conversao::playlist, conversao::detalhe_da_playlist)
    }

    /// As playlists, os álbuns e os artistas da conta, na ordem do Spotify, com as Músicas Curtidas primeiro.
    pub fn biblioteca(&self) -> Result<Vec<Item>, String> {
        let lista = Lista { op: "libraryV3", por_pagina: POR_PAGINA_DA_BIBLIOTECA, maximo: MAXIMO_DA_BIBLIOTECA };
        let pagina = |de: u32| {
            json!({
                "filters": [], "order": null, "textFilter": "",
                "features": ["LIKED_SONGS", "YOUR_EPISODES_V2", "PRERELEASES", "EVENTS"],
                "limit": POR_PAGINA_DA_BIBLIOTECA, "offset": de, "flatten": true, "expandedFolders": [],
                "folderUri": null, "includeFoldersWhenFlattening": false,
            })
        };
        let mut itens = self.em_paginas(&lista, Self::prazo(), pagina, conversao::biblioteca)?;
        itens.sort_by_key(|i| !matches!(i, Item::Playlist(p) if p.id == conversao::CURTIDAS));
        Ok(itens)
    }
}

fn em_itens(faixas: Vec<Faixa>) -> Vec<Item> {
    faixas.into_iter().map(Item::Faixa).collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) mod falsas {
    use super::*;
    use std::sync::Mutex;

    pub(crate) type Responder = Box<dyn Fn(&str, &Value) -> Result<Value, Falha> + Send + Sync>;

    /// Responde pelo roteiro e guarda os pedidos.
    pub(crate) struct Falsas {
        pub responder: Responder,
        pub pedidos: Arc<Mutex<Vec<(String, Value)>>>,
    }

    impl Consultas for Falsas {
        fn consultar(&self, pedidos: &[Pedido], _: Instant) -> Vec<Result<Value, Falha>> {
            pedidos
                .iter()
                .map(|(op, v)| {
                    self.pedidos.lock().expect("pedidos").push(((*op).to_string(), v.clone()));
                    (self.responder)(op, v)
                })
                .collect()
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::falsas::Falsas;
    use super::*;
    use crate::conta::falso::{Falso, cofre, esperar};
    use provedores::SituacaoDaConta;
    use std::sync::Mutex;
    use std::sync::atomic::Ordering;

    type Pedidos = Arc<Mutex<Vec<(String, Value)>>>;

    fn dados(
        teste: &str,
        falso: Falso,
        responder: impl Fn(&str, &Value) -> Result<Value, Falha> + Send + Sync + 'static,
    ) -> (Dados, Arc<Falso>, Pedidos) {
        let falso = Arc::new(falso);
        let login = Arc::new(Login::novo(falso.clone(), cofre(teste)));
        login.entrar().expect("entrar");
        assert!(esperar(|| login.estado().situacao == SituacaoDaConta::Dentro));
        let pedidos: Pedidos = Arc::default();
        let fonte = Falsas { responder: Box::new(responder), pedidos: pedidos.clone() };
        (Dados::novo(Box::new(fonte), login), falso, pedidos)
    }

    fn faixa(id: usize) -> Value {
        json!({"__typename": "Track", "uri": format!("spotify:track:f{id}"), "name": format!("F{id}")})
    }

    /// Uma playlist de `total` faixas, servida em páginas.
    fn pagina_de_playlist(v: &Value, total: usize) -> Value {
        let (de, n) = (v["offset"].as_u64().unwrap_or(0) as usize, v["limit"].as_u64().unwrap_or(0) as usize);
        let itens: Vec<Value> = (de..total.min(de + n)).map(|i| json!({"itemV2": {"data": faixa(i)}})).collect();
        json!({"playlistV2": {"content": {"totalCount": total, "items": itens}}})
    }

    #[test]
    fn as_paginas_que_faltam() {
        assert!(paginas_que_faltam(0, 100, 2000).is_empty());
        assert!(paginas_que_faltam(100, 100, 2000).is_empty(), "coube na primeira");
        assert_eq!(paginas_que_faltam(101, 100, 2000), [100]);
        assert_eq!(paginas_que_faltam(350, 100, 2000), [100, 200, 300]);
        assert_eq!(paginas_que_faltam(90_000, 100, 300), [100, 200], "no teto");
        assert!(paginas_que_faltam(10, 0, 300).is_empty(), "página vazia não roda para sempre");
    }

    #[test]
    fn playlist_longa_vem_inteira_e_em_ordem() {
        let (d, _, pedidos) = dados("longa", Falso::default(), |_, v| Ok(pagina_de_playlist(v, 250)));
        let itens = d.playlist("37abc").expect("playlist").0;
        let ids: Vec<&str> = itens.iter().filter_map(Item::faixa).map(|f| f.id.as_str()).collect();
        assert_eq!(ids.len(), 250);
        assert_eq!((ids[0], ids[100], ids[249]), ("f0", "f100", "f249"));
        let p = pedidos.lock().expect("pedidos");
        assert!(p.iter().all(|(op, v)| op == "fetchPlaylist" && v["uri"] == "spotify:playlist:37abc"));
        assert_eq!(p.len(), 3);
    }

    #[test]
    fn playlist_enorme_para_no_teto() {
        let (d, _, _) = dados("enorme", Falso::default(), |_, v| Ok(pagina_de_playlist(v, 9000)));
        assert_eq!(d.playlist("x").expect("playlist").0.len(), MAXIMO as usize);
    }

    #[test]
    fn uma_pagina_que_falha_nao_vira_playlist_cortada() {
        let (d, _, _) = dados("cortada", Falso::default(), |_, v| match v["offset"].as_u64() {
            Some(100) => Err(Falha::Rede("a conexão caiu".into())),
            _ => Ok(pagina_de_playlist(v, 250)),
        });
        assert!(d.playlist("x").expect_err("erro").contains("a conexão caiu"));
    }

    #[test]
    fn pagina_vazia_no_meio_e_erro() {
        let (d, _, _) = dados("meio-vazio", Falso::default(), |_, v| match v["offset"].as_u64() {
            Some(100) => Ok(json!({"playlistV2": {"content": {"totalCount": 250, "items": []}}})),
            _ => Ok(pagina_de_playlist(v, 250)),
        });
        assert!(d.playlist("x").is_err(), "cem faixas do meio não somem caladas");
    }

    #[test]
    fn servidor_que_manda_paginas_menores_vem_inteiro() {
        let (d, _, _) = dados("menores", Falso::default(), |_, v| {
            let mut v = v.clone();
            v["limit"] = json!(v["limit"].as_u64().unwrap_or(0).min(50));
            Ok(pagina_de_playlist(&v, 170))
        });
        let itens = d.playlist("x").expect("playlist").0;
        let ids: Vec<&str> = itens.iter().filter_map(Item::faixa).map(|f| f.id.as_str()).collect();
        assert_eq!((ids.len(), ids[60], ids[169]), (170, "f60", "f169"));
    }

    #[test]
    fn artista_enorme_para_nos_200_lancamentos() {
        let (d, _, pedidos) = dados("muitos", Falso::default(), |op, v| {
            if op == "queryArtistOverview" {
                return Ok(json!({"artistUnion": {"discography": {"topTracks": {"items": []}}}}));
            }
            let de = v["offset"].as_u64().unwrap_or(0);
            let itens: Vec<Value> = (de..de + 50)
                .map(|i| json!({"releases": {"items": [{"uri": format!("spotify:album:al{i}"), "name": "D"}]}}))
                .collect();
            Ok(json!({"artistUnion": {"discography": {"all": {"totalCount": 1900, "items": itens}}}}))
        });
        assert_eq!(d.artista("a1").expect("artista").0.len(), 200);
        assert_eq!(pedidos.lock().expect("pedidos").len(), 5);
    }

    #[test]
    fn a_reentrada_tem_freio() {
        let (d, falso, _) = dados("freio", Falso::default(), |_, _| Err(Falha::Sessao));
        for _ in 0..4 {
            assert!(d.playlist("x").is_err());
        }
        if cfg!(windows) {
            assert_eq!(falso.renovacoes.load(Ordering::SeqCst), 1, "uma reentrada que não resolveu não se repete já");
        }
    }

    #[test]
    fn as_curtidas_sao_uma_playlist() {
        let (d, _, pedidos) = dados("curtidas", Falso::default(), |_, _| {
            Ok(json!({"me": {"library": {"tracks": {"totalCount": 1, "items": [{"track": {"data": faixa(1)}}]}}}}))
        });
        assert_eq!(d.playlist(conversao::CURTIDAS).expect("curtidas").0.len(), 1);
        assert_eq!(pedidos.lock().expect("pedidos")[0].0, "fetchLibraryTracks");
    }

    #[test]
    fn id_estranho_nao_vira_pedido() {
        let (d, _, pedidos) = dados("id", Falso::default(), |_, v| Ok(pagina_de_playlist(v, 1)));
        assert!(d.playlist("../x").is_err());
        assert!(d.album("").is_err());
        assert!(pedidos.lock().expect("pedidos").is_empty());
    }

    #[test]
    fn artista_com_populares_e_todos_os_lancamentos() {
        let (d, _, _) = dados("artista", Falso::default(), |op, v| {
            if op == "queryArtistOverview" {
                return Ok(json!({"artistUnion": {"discography": {"topTracks": {"items": [{"track": faixa(1)}]}}}}));
            }
            let de = v["offset"].as_u64().unwrap_or(0);
            let itens: Vec<Value> = (de..70.min(de + 50))
                .map(|i| json!({"releases": {"items": [{"uri": format!("spotify:album:al{i}"), "name": "D"}]}}))
                .collect();
            Ok(json!({"artistUnion": {"discography": {"all": {"totalCount": 70, "items": itens}}}}))
        });
        let itens = d.artista("a1").expect("artista").0;
        assert_eq!(itens.iter().filter(|i| i.faixa().is_some()).count(), 1);
        assert_eq!(itens.iter().filter(|i| matches!(i, Item::Album(_))).count(), 70);
    }

    #[test]
    fn artista_sem_a_discografia_ainda_mostra_as_populares() {
        let (d, _, _) = dados("sem-discografia", Falso::default(), |op, _| match op {
            "queryArtistOverview" => {
                Ok(json!({"artistUnion": {"discography": {"topTracks": {"items": [{"track": faixa(1)}]}}}}))
            }
            _ => Err(Falha::Codigo),
        });
        assert_eq!(d.artista("a1").expect("artista").0.len(), 1);
    }

    #[test]
    fn biblioteca_inteira_com_as_curtidas_primeiro() {
        let (d, _, _) = dados("biblioteca", Falso::default(), |_, v| {
            let de = v["offset"].as_u64().unwrap_or(0);
            let itens: Vec<Value> = (de..60.min(de + 50))
                .map(|i| {
                    if i == 30 {
                        return json!({"item": {"_uri": "spotify:collection:tracks", "data": {"count": 3}}});
                    }
                    json!({"item": {"data": {"__typename": "Playlist", "uri": format!("spotify:playlist:p{i}"), "name": "L"}}})
                })
                .collect();
            Ok(json!({"me": {"libraryV3": {"totalCount": 60, "items": itens}}}))
        });
        let itens = d.biblioteca().expect("biblioteca");
        assert_eq!(itens.len(), 60);
        assert!(matches!(&itens[0], Item::Playlist(p) if p.id == conversao::CURTIDAS));
    }

    #[test]
    fn sessao_caida_reentra_e_tenta_uma_vez() {
        let vez = Mutex::new(0);
        let (d, falso, _) = dados("reentra", Falso::default(), move |_, v| {
            let mut n = vez.lock().expect("vez");
            *n += 1;
            if *n == 1 { Err(Falha::Sessao) } else { Ok(pagina_de_playlist(v, 1)) }
        });
        assert_eq!(d.playlist("x").expect("playlist").0.len(), 1);
        if cfg!(windows) {
            assert_eq!(falso.renovacoes.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn sessao_que_continua_caida_vira_erro_sem_laco() {
        let (d, falso, pedidos) = dados("sem-laco", Falso::default(), |_, _| Err(Falha::Sessao));
        assert!(d.playlist("x").is_err());
        assert_eq!(pedidos.lock().expect("pedidos").len(), 2);
        if cfg!(windows) {
            assert_eq!(falso.renovacoes.load(Ordering::SeqCst), 1);
        }
    }

    #[test]
    fn o_limite_chega_com_a_frase() {
        let (d, _, _) = dados("limite", Falso::default(), |_, _| Err(Falha::Limite(7)));
        assert_eq!(d.inicio().expect_err("limite"), "O Spotify pediu para esperar 7 s.");
        assert!(d.buscar("x", 20).is_err());
    }
}
