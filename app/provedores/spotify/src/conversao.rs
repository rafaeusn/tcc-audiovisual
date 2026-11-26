//! Do JSON do Pathfinder (o `data` de cada consulta) para os modelos do Player. Só lógica, testada. Um item que não dá
//! para ler é pulado (um tipo novo, um campo nulo); um nó que a página precisa e não veio é erro, nunca lista vazia.

use provedores::{Album, Artista, Detalhe, Faixa, Item, Playlist, Prateleira, TipoDaPrateleira, TipoDeAlbum};
use serde_json::Value;
use std::collections::HashSet;

pub const SERVICO: &str = "spotify";
/// O id da "playlist" das Músicas Curtidas (o Spotify não dá um id de playlist a ela).
pub const CURTIDAS: &str = "curtidas";

type Resultado<T> = Result<T, String>;

/// Uma página de uma lista.
#[derive(Clone, Debug, PartialEq)]
pub struct Pagina<T> {
    pub itens: Vec<T>,
    /// Quantos a lista tem ao todo.
    pub total: u32,
    /// Quantos vieram nesta página, com os que o Player não mostra.
    pub vieram: u32,
}

fn objeto<'a>(data: &'a Value, caminho: &str, o_que: &str) -> Resultado<&'a Value> {
    data.pointer(caminho).filter(|v| v.is_object()).ok_or_else(|| format!("sem {o_que}"))
}

fn lista<'a>(v: &'a Value, campo: &str) -> &'a [Value] {
    v[campo].as_array().map_or(&[], Vec::as_slice)
}

/// A lista que a página precisa: uma resposta pela metade (`items` nulo ao lado do total) é erro, não lista vazia.
fn itens_de(v: &Value) -> Resultado<&[Value]> {
    v["items"].as_array().map(Vec::as_slice).ok_or_else(|| "a lista não veio".to_string())
}

fn pagina<T>(v: &Value, ler: impl Fn(&Value) -> Option<T>) -> Resultado<Pagina<T>> {
    let itens = itens_de(v)?;
    let lidos = itens.iter().filter_map(ler).collect();
    Ok(Pagina { itens: lidos, total: total(v, itens.len()), vieram: itens.len() as u32 })
}

fn texto(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|t| !t.is_empty()).map(str::to_string)
}

fn total(v: &Value, veio: usize) -> u32 {
    v["totalCount"].as_u64().map_or(veio, |t| t as usize).min(u32::MAX as usize) as u32
}

/// O id no endereço `spotify:<tipo>:<id>` (só do tipo pedido).
fn id(endereco: &Value, tipo: &str) -> Option<String> {
    let id = endereco.as_str()?.strip_prefix("spotify:")?.strip_prefix(tipo)?.strip_prefix(':')?;
    (!id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| id.to_string())
}

/// A imagem: a menor com 300 px ou mais (a dos cartões); sem nenhuma, a maior; sem medidas, a primeira.
pub fn capa(imagem: &Value) -> Option<String> {
    let fontes = lista(imagem, "sources");
    let altura = |f: &Value| f["height"].as_u64().unwrap_or(0);
    let serve = fontes.iter().filter(|f| altura(f) >= 300).min_by_key(|f| altura(f));
    let maior = || fontes.iter().rev().max_by_key(|f| altura(f));
    texto(&serve.or_else(maior)?["url"])
}

/// Os nomes e os ids dos artistas, na mesma ordem (`""` quando não há id).
fn artistas(v: &Value) -> (Vec<String>, Vec<String>) {
    lista(v, "items")
        .iter()
        .filter_map(|a| Some((texto(&a["profile"]["name"])?, id(&a["uri"], "artist").unwrap_or_default())))
        .unzip()
}

/// `endereco` é o do embrulho, para as respostas que não o repetem na faixa.
fn faixa(v: &Value, endereco: &Value) -> Option<Faixa> {
    let (artistas, artistas_ids) = artistas(&v["artists"]);
    let disco = &v["albumOfTrack"];
    let ms = v["duration"]["totalMilliseconds"].as_f64().or_else(|| v["trackDuration"]["totalMilliseconds"].as_f64());
    Some(Faixa {
        servico: SERVICO.into(),
        id: id(&v["uri"], "track").or_else(|| id(endereco, "track"))?,
        titulo: texto(&v["name"])?,
        artistas,
        artistas_ids,
        album: texto(&disco["name"]),
        album_id: id(&disco["uri"], "album"),
        duracao_s: ms.map(|ms| ms / 1000.0).filter(|s| *s > 0.0),
        capa: capa(&disco["coverArt"]),
        ouvidas: numero(&v["playcount"]),
        ..Default::default()
    })
}

/// Um número, que o Spotify às vezes manda como texto.
fn numero(v: &Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str()?.parse().ok())
}

/// A data do lançamento, só com o que se sabe dela (`2001`, `2001-03-12`).
fn data_de(v: &Value) -> Option<String> {
    let iso = v["isoString"]
        .as_str()
        .and_then(|t| t.get(..10))
        .filter(|t| t.bytes().all(|b| b.is_ascii_digit() || b == b'-'));
    match (iso, v["precision"].as_str()) {
        (Some(t), Some("DAY")) => Some(t.to_string()),
        (Some(t), Some("MONTH")) => Some(t[..7].to_string()),
        (Some(t), _) => Some(t[..4].to_string()),
        (None, _) => v["year"].as_u64().map(|a| a.to_string()),
    }
}

fn tipo_de_album(v: &Value) -> Option<TipoDeAlbum> {
    Some(match v.as_str()? {
        "ALBUM" => TipoDeAlbum::Album,
        "SINGLE" => TipoDeAlbum::Single,
        "EP" => TipoDeAlbum::Ep,
        "COMPILATION" => TipoDeAlbum::Coletanea,
        _ => TipoDeAlbum::Outro,
    })
}

/// A cor `#RRGGBB`, se for uma.
fn cor(v: &Value) -> Option<String> {
    let t = v["hex"].as_str()?;
    (t.len() == 7 && t.starts_with('#') && t[1..].bytes().all(|b| b.is_ascii_hexdigit())).then(|| t.to_string())
}

/// O texto sem HTML (as biografias e as descrições vêm com links).
pub fn sem_html(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(i) = resto.find('<') {
        saida.push_str(&resto[..i]);
        // Um "<" solto (sem letra nem barra depois) é texto.
        let marca = resto[i + 1..].starts_with(|c: char| c.is_ascii_alphabetic() || c == '/');
        match resto[i..].find('>').filter(|_| marca) {
            Some(f) => resto = &resto[i + f + 1..],
            None => {
                saida.push('<');
                resto = &resto[i + 1..];
            }
        }
    }
    saida.push_str(resto);
    [
        ("&quot;", "\""),
        ("&#34;", "\""),
        ("&#39;", "'"),
        ("&#x27;", "'"),
        ("&#x2F;", "/"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&nbsp;", " "),
        ("&amp;", "&"),
    ]
    .into_iter()
    .fold(saida, |s, (de, para)| s.replace(de, para))
    .trim()
    .to_string()
}

fn texto_puro(v: &Value) -> Option<String> {
    texto(v).map(|t| sem_html(&t)).filter(|t| !t.is_empty())
}

/// A imagem larga: a menor com 1280 px ou mais; sem nenhuma, a maior.
fn larga(imagem: &Value) -> Option<String> {
    let fontes = lista(imagem, "sources");
    let largura = |f: &Value| f["maxWidth"].as_u64().or_else(|| f["width"].as_u64()).unwrap_or(0);
    let serve = fontes.iter().filter(|f| largura(f) >= 1280).min_by_key(|f| largura(f));
    texto(&serve.or_else(|| fontes.iter().max_by_key(|f| largura(f)))?["url"])
}

fn secao(titulo: &str, tipo: TipoDaPrateleira, itens: Vec<Item>) -> Option<Prateleira> {
    (!itens.is_empty()).then(|| Prateleira { titulo: titulo.into(), tipo, itens })
}

fn album_de(v: &Value) -> Option<Album> {
    let (artistas, artistas_ids) = artistas(&v["artists"]);
    Some(Album {
        servico: SERVICO.into(),
        id: id(&v["uri"], "album")?,
        titulo: texto(&v["name"])?,
        artistas,
        artistas_ids,
        ano: data_de(&v["date"]).and_then(|d| d.get(..4)?.parse().ok()),
        capa: capa(&v["coverArt"]),
        tipo: tipo_de_album(&v["type"]),
    })
}

fn artista_de(v: &Value) -> Option<Artista> {
    Some(Artista {
        servico: SERVICO.into(),
        id: id(&v["uri"], "artist")?,
        nome: texto(&v["profile"]["name"])?,
        foto: capa(&v["visuals"]["avatarImage"]),
    })
}

fn playlist_de(v: &Value) -> Option<Playlist> {
    Some(Playlist {
        servico: SERVICO.into(),
        id: id(&v["uri"], "playlist")?,
        titulo: texto(&v["name"])?,
        dono: texto(&v["ownerV2"]["data"]["name"]),
        capa: lista(&v["images"], "items").first().and_then(capa),
        quantas: None,
    })
}

/// Um item embrulhado (`{"__typename": "...ResponseWrapper", "data": {...}}`).
fn item(embrulho: &Value) -> Option<Item> {
    let v = &embrulho["data"];
    match v["__typename"].as_str()? {
        "Track" => faixa(v, &embrulho["_uri"]).map(Item::Faixa),
        "Album" => album_de(v).map(Item::Album),
        "Artist" => artista_de(v).map(Item::Artista),
        "Playlist" => playlist_de(v).map(Item::Playlist),
        _ => None,
    }
}

/// O tipo e o id, para não repetir um item.
fn chave(i: &Item) -> (u8, String) {
    match i {
        Item::Faixa(f) => (0, f.id.clone()),
        Item::Album(a) => (1, a.id.clone()),
        Item::Artista(a) => (2, a.id.clone()),
        Item::Playlist(p) => (3, p.id.clone()),
    }
}

/// A busca: os melhores resultados primeiro, depois cada tipo na ordem do Spotify; `limite` vale para cada tipo.
pub fn busca(data: &Value, limite: u32) -> Resultado<Vec<Item>> {
    let b = objeto(data, "/searchV2", "o resultado da busca")?;
    let grupos = [
        ("topResultsV2", "itemsV2"),
        ("tracksV2", "items"),
        ("artists", "items"),
        ("albumsV2", "items"),
        ("playlists", "items"),
    ];
    if !grupos.into_iter().any(|(grupo, campo)| b[grupo][campo].is_array()) {
        return Err("a busca veio sem nenhum grupo".into());
    }
    let embrulhados =
        lista(&b["topResultsV2"], "itemsV2").iter().chain(lista(&b["tracksV2"], "items")).map(|i| &i["item"]);
    let soltos = ["artists", "albumsV2", "playlists"].into_iter().flat_map(|campo| lista(&b[campo], "items"));
    let mut vistos = HashSet::new();
    let mut quantos = [0u32; 4];
    Ok(embrulhados
        .chain(soltos)
        .filter_map(item)
        .filter(|i| {
            let (tipo, id) = chave(i);
            if quantos[usize::from(tipo)] >= limite || !vistos.insert((tipo, id)) {
                return false;
            }
            quantos[usize::from(tipo)] += 1;
            true
        })
        .collect())
}

/// As prateleiras do início. As seções de uma sugestão só (a "recomendação do dia" do app) ficam juntas no fim.
pub fn inicio(data: &Value) -> Resultado<Vec<Prateleira>> {
    let secoes = objeto(data, "/home/sectionContainer/sections", "as seções do início")?;
    let mut prateleiras: Vec<Prateleira> = Vec::new();
    let mut sugestoes = Vec::new();
    for s in lista(secoes, "items") {
        let mut itens: Vec<Item> =
            lista(&s["sectionItems"], "items").iter().filter_map(|i| item(&i["content"])).collect();
        sem_repetir(&mut itens);
        if s["data"]["__typename"] == "HomeFeedBaselineSectionData" {
            sugestoes.append(&mut itens);
        } else if !itens.is_empty() {
            let titulo = texto(&s["data"]["title"]["transformedLabel"]).unwrap_or_else(|| "Para começar".into());
            let tipo = tipo_da_secao(s["data"]["__typename"].as_str().unwrap_or_default(), &titulo, &itens);
            prateleiras.push(Prateleira { titulo, tipo, itens });
        }
    }
    sem_repetir(&mut sugestoes);
    if !sugestoes.is_empty() {
        let tipo = TipoDaPrateleira::ParaVoce;
        prateleiras.push(Prateleira { titulo: "Sugestões para você".into(), tipo, itens: sugestoes });
    }
    if prateleiras.is_empty() {
        return Err("o início veio sem nada que o Player mostre".into());
    }
    Ok(prateleiras)
}

/// O tipo de uma seção do início: pelo nome dela na API e, nas genéricas, pelo título (em português e em inglês) e pelo que traz.
fn tipo_da_secao(nome: &str, titulo: &str, itens: &[Item]) -> TipoDaPrateleira {
    use TipoDaPrateleira as T;
    match nome {
        "HomeShortsSectionData" => return T::Atalhos,
        "HomeRecentlyPlayedSectionData" => return T::Recentes,
        _ => {}
    }
    let titulo = provedores::casador::limpar_titulo(titulo);
    let tem = |palavras: &[&str]| palavras.iter().any(|p| titulo.contains(p));
    if tem(&["recente", "recently", "ouvidas ha pouco", "ouvir de novo", "jump back"]) {
        T::Recentes
    } else if tem(&["lancamento", "novidade", "novos lanc", "new release", "estreia"]) {
        T::Lancamentos
    } else if tem(&[
        "parada",
        "chart",
        "top 50",
        "mais tocad",
        "em alta",
        "populares",
        "sucessos",
        "maiores hits",
        "trending",
    ]) {
        T::Paradas
    } else if !itens.is_empty() && itens.iter().all(|i| matches!(i, Item::Artista(_))) {
        T::Artistas
    } else if tem(&[
        "para voce",
        "pra voce",
        "feito para",
        "feito pra",
        "made for",
        "for you",
        "seus mix",
        "sua cara",
        "recomend",
    ]) {
        T::ParaVoce
    } else {
        T::Outras
    }
}

fn sem_repetir(itens: &mut Vec<Item>) {
    let mut vistos = HashSet::new();
    itens.retain(|i| vistos.insert(chave(i)));
}

/// As faixas de um álbum; o disco, a capa e o id vêm do cabeçalho.
pub fn album(data: &Value) -> Resultado<Pagina<Faixa>> {
    let a = objeto(data, "/albumUnion", "o álbum")?;
    let (nome, id, capa) = (texto(&a["name"]), id(&a["uri"], "album"), capa(&a["coverArt"]));
    pagina(objeto(a, "/tracksV2", "as faixas do álbum")?, |i| {
        let f = faixa(&i["track"], &Value::Null)?;
        Some(Faixa { album: nome.clone(), album_id: id.clone(), capa: capa.clone(), ..f })
    })
}

/// O que mais se sabe de um álbum.
pub fn detalhe_do_album(data: &Value) -> Detalhe {
    let a = &data["albumUnion"];
    let este = id(&a["uri"], "album");
    let outros = lista(&a["moreAlbumsByArtist"], "items")
        .iter()
        .flat_map(|i| lista(&i["discography"]["popularReleasesAlbums"], "items"))
        .filter_map(album_de)
        .filter(|x| Some(&x.id) != este.as_ref())
        .map(Item::Album)
        .collect();
    Detalhe {
        item: album_de(a).map(Item::Album),
        cor: cor(&a["coverArt"]["extractedColors"]["colorDark"]),
        total: a["tracksV2"]["totalCount"].as_u64().and_then(|n| u32::try_from(n).ok()),
        data: data_de(&a["date"]),
        gravadora: texto(&a["label"]),
        tipo: tipo_de_album(&a["type"]),
        secoes: secao("Mais do artista", TipoDaPrateleira::MaisDoArtista, outros).into_iter().collect(),
        ..Default::default()
    }
}

/// O que mais se sabe de um artista.
pub fn detalhe_do_artista(data: &Value) -> Detalhe {
    let a = &data["artistUnion"];
    let relacionados = &a["relatedContent"];
    let ultimo = album_de(&a["discography"]["latest"]).map(Item::Album).into_iter().collect();
    let parecidos =
        lista(&relacionados["relatedArtists"], "items").iter().filter_map(artista_de).map(Item::Artista).collect();
    let playlists = lista(&relacionados["featuringV2"], "items").iter().filter_map(item).collect();
    Detalhe {
        item: artista_de(a).map(Item::Artista),
        imagem: larga(&a["headerImage"]["data"]),
        cor: cor(&a["visuals"]["avatarImage"]["extractedColors"]["colorRaw"]),
        descricao: texto_puro(&a["profile"]["biography"]["text"]),
        fotos: lista(&a["visuals"]["gallery"], "items").iter().filter_map(capa).take(8).collect(),
        seguidores: numero(&a["stats"]["followers"]),
        ouvintes: numero(&a["stats"]["monthlyListeners"]),
        secoes: [
            secao("Último lançamento", TipoDaPrateleira::UltimoLancamento, ultimo),
            secao("Quem ouve também curte", TipoDaPrateleira::Parecidos, parecidos),
            secao("Em playlists", TipoDaPrateleira::Outras, playlists),
        ]
        .into_iter()
        .flatten()
        .collect(),
        ..Default::default()
    }
}

/// O que mais se sabe de uma playlist.
pub fn detalhe_da_playlist(data: &Value) -> Detalhe {
    let p = &data["playlistV2"];
    let total = p["content"]["totalCount"].as_u64().and_then(|n| u32::try_from(n).ok());
    let capa = lista(&p["images"], "items").first();
    Detalhe {
        item: playlist_de(p).map(|x| Item::Playlist(Playlist { quantas: total, ..x })),
        cor: capa.and_then(|c| cor(&c["extractedColors"]["colorRaw"])),
        descricao: texto_puro(&p["description"]),
        seguidores: numero(&p["followers"]).or_else(|| numero(&p["followers"]["totalCount"])).filter(|n| *n > 0),
        total,
        dono: texto(&p["ownerV2"]["data"]["name"]),
        ..Default::default()
    }
}

/// As faixas populares de um artista (um artista pode não ter nenhuma).
pub fn populares(data: &Value) -> Resultado<Vec<Faixa>> {
    let d = objeto(data, "/artistUnion/discography", "o artista")?;
    Ok(lista(&d["topTracks"], "items").iter().filter_map(|i| faixa(&i["track"], &Value::Null)).collect())
}

/// Os lançamentos de um artista.
pub fn discografia(data: &Value) -> Resultado<Pagina<Album>> {
    let todos = objeto(data, "/artistUnion/discography/all", "a discografia")?;
    pagina(todos, |i| album_de(lista(&i["releases"], "items").first()?))
}

/// As faixas de uma playlist.
pub fn playlist(data: &Value) -> Resultado<Pagina<Faixa>> {
    pagina(objeto(data, "/playlistV2/content", "as faixas da playlist")?, |i| {
        let e = &i["itemV2"];
        (e["data"]["__typename"] == "Track").then(|| faixa(&e["data"], &e["_uri"]))?
    })
}

/// As Músicas Curtidas.
pub fn curtidas(data: &Value) -> Resultado<Pagina<Faixa>> {
    let t = objeto(data, "/me/library/tracks", "as Músicas Curtidas")?;
    pagina(t, |i| faixa(&i["track"]["data"], &i["track"]["_uri"]))
}

/// A biblioteca da conta: as Curtidas viram uma playlist; pastas, episódios e podcasts ficam de fora.
pub fn biblioteca(data: &Value) -> Resultado<Pagina<Item>> {
    let l = objeto(data, "/me/libraryV3", "a biblioteca")?;
    pagina(l, |i| curtidas_de(&i["item"]).or_else(|| item(&i["item"])))
}

fn curtidas_de(embrulho: &Value) -> Option<Item> {
    let v = &embrulho["data"];
    let sao = [&embrulho["_uri"], &v["uri"]].into_iter().any(|u| u == "spotify:collection:tracks");
    sao.then(|| {
        Item::Playlist(Playlist {
            servico: SERVICO.into(),
            id: CURTIDAS.into(),
            titulo: "Músicas Curtidas".into(),
            dono: None,
            capa: None,
            quantas: v["count"].as_u64().and_then(|n| u32::try_from(n).ok()),
        })
    })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod testes {
    use super::*;
    use serde_json::json;

    fn capas() -> Value {
        json!({"sources": [
            {"height": 64, "url": "c64", "width": 64},
            {"height": 640, "url": "c640", "width": 640},
            {"height": 300, "url": "c300", "width": 300}
        ]})
    }

    fn artista_de(id: &str, nome: &str) -> Value {
        json!({"profile": {"name": nome}, "uri": format!("spotify:artist:{id}")})
    }

    fn faixa_json(id: &str, nome: &str) -> Value {
        json!({"__typename": "Track", "uri": format!("spotify:track:{id}"), "name": nome,
            "artists": {"items": [artista_de("a1", "Ana"), artista_de("a2", "Bia")]},
            "albumOfTrack": {"uri": "spotify:album:al1", "name": "Disco", "coverArt": capas()},
            "duration": {"totalMilliseconds": 201_500}, "playability": {"playable": true}})
    }

    fn album_json(id: &str, nome: &str) -> Value {
        json!({"__typename": "Album", "uri": format!("spotify:album:{id}"), "name": nome, "date": {"year": 2013},
            "artists": {"items": [artista_de("a1", "Ana")]}, "coverArt": capas()})
    }

    fn artista_json(id: &str, nome: &str) -> Value {
        json!({"__typename": "Artist", "uri": format!("spotify:artist:{id}"), "profile": {"name": nome},
            "visuals": {"avatarImage": capas()}})
    }

    fn playlist_json(id: &str, nome: &str) -> Value {
        json!({"__typename": "Playlist", "uri": format!("spotify:playlist:{id}"), "name": nome,
            "ownerV2": {"data": {"name": "Dona"}},
            "images": {"items": [{"sources": [{"height": null, "url": "lista", "width": null}]}]}})
    }

    fn embrulho(dado: Value) -> Value {
        let tipo = format!("{}ResponseWrapper", dado["__typename"].as_str().unwrap_or("X"));
        json!({"__typename": tipo, "data": dado})
    }

    #[test]
    fn faixa_completa() {
        let data = json!({"searchV2": {
            "topResultsV2": {"itemsV2": []}, "tracksV2": {"items": [{"item": embrulho(faixa_json("f1", "Canção"))}]},
            "artists": {"items": []}, "albumsV2": {"items": []}, "playlists": {"items": []}
        }});
        let itens = busca(&data, 20).expect("busca");
        let f = itens[0].faixa().expect("faixa");
        assert_eq!((f.servico.as_str(), f.id.as_str(), f.titulo.as_str()), ("spotify", "f1", "Canção"));
        assert_eq!((f.artistas.join("|"), f.artistas_ids.join("|")), ("Ana|Bia".into(), "a1|a2".into()));
        assert_eq!((f.album.as_deref(), f.album_id.as_deref()), (Some("Disco"), Some("al1")));
        assert_eq!((f.duracao_s, f.capa.as_deref(), f.video), (Some(201.5), Some("c300"), false));
    }

    #[test]
    fn a_capa_e_a_menor_que_serve() {
        assert_eq!(capa(&capas()).as_deref(), Some("c300"));
        assert_eq!(
            capa(&json!({"sources": [{"height": 64, "url": "a"}, {"height": 200, "url": "b"}]})).as_deref(),
            Some("b")
        );
        assert_eq!(capa(&json!({"sources": [{"height": null, "url": "x"}]})).as_deref(), Some("x"));
        assert_eq!(capa(&json!({"sources": []})), None);
        assert_eq!(capa(&Value::Null), None);
    }

    #[test]
    fn busca_com_o_melhor_primeiro_e_sem_repetir() {
        let faixas: Vec<Value> =
            (0..5).map(|i| json!({"item": embrulho(faixa_json(&format!("f{i}"), &format!("F{i}")))})).collect();
        let data = json!({"searchV2": {
            "topResultsV2": {"itemsV2": [
                {"item": embrulho(artista_json("a9", "Ana"))},
                {"item": {"__typename": "SearchSectionEntity", "data": {}}},
                {"item": embrulho(faixa_json("f2", "F2"))}
            ]},
            "tracksV2": {"items": faixas},
            "artists": {"items": [embrulho(artista_json("a9", "Ana")), embrulho(artista_json("a8", "Outra"))]},
            "albumsV2": {"items": [embrulho(album_json("al1", "Disco")), embrulho(json!({"__typename": "PreRelease"}))]},
            "playlists": {"items": [embrulho(playlist_json("p1", "Lista"))]}
        }});
        let itens = busca(&data, 3).expect("busca");
        assert!(matches!(&itens[0], Item::Artista(a) if a.id == "a9" && a.foto.as_deref() == Some("c300")));
        let faixas: Vec<&str> = itens.iter().filter_map(Item::faixa).map(|f| f.id.as_str()).collect();
        assert_eq!(faixas, ["f2", "f0", "f1"], "o melhor primeiro, sem repetir, no limite de cada tipo");
        assert!(itens.iter().any(|i| matches!(i, Item::Album(a) if a.ano == Some(2013) && a.artistas_ids == ["a1"])));
        let lista =
            itens.iter().find_map(|i| if let Item::Playlist(p) = i { Some(p) } else { None }).expect("playlist");
        assert_eq!((lista.dono.as_deref(), lista.capa.as_deref()), (Some("Dona"), Some("lista")));
        assert_eq!(itens.iter().filter(|i| matches!(i, Item::Artista(_))).count(), 2);
    }

    #[test]
    fn busca_sem_nenhum_resultado_e_vazia_e_sem_o_no_e_erro() {
        let nada = json!({"searchV2": {"topResultsV2": {"itemsV2": []}, "tracksV2": {"items": []}, "artists": {"items": []},
            "albumsV2": {"items": []}, "playlists": {"items": []}}});
        assert_eq!(busca(&nada, 20), Ok(Vec::new()));
        assert!(busca(&json!({}), 20).is_err());
        assert!(busca(&json!({"searchV2": null}), 20).is_err());
    }

    fn secao(tipo: &str, titulo: Option<&str>, itens: Vec<Value>) -> Value {
        let itens: Vec<Value> = itens.into_iter().map(|c| json!({"content": c})).collect();
        json!({"data": {"__typename": tipo, "title": titulo.map(|t| json!({"transformedLabel": t}))},
            "sectionItems": {"items": itens}})
    }

    #[test]
    fn inicio_em_prateleiras() {
        let episodio = json!({"__typename": "EpisodeOrChapterResponseWrapper", "data": {"__typename": "Episode"}});
        let data = json!({"home": {"sectionContainer": {"sections": {"items": [
            secao("HomeShortsSectionData", None, vec![embrulho(playlist_json("p0", "Atalho")), json!({"__typename": "UnknownType"})]),
            secao("HomeGenericSectionData", Some("Feito para você"), vec![embrulho(playlist_json("p1", "Mix"))]),
            secao("HomeGenericSectionData", Some("Podcasts"), vec![episodio.clone()]),
            secao("HomeGenericSectionData", Some("Discos"), vec![embrulho(album_json("al1", "Disco")), embrulho(artista_json("a1", "Ana"))]),
            secao("HomeFeedBaselineSectionData", Some("Porque você ouviu Ana"), vec![embrulho(playlist_json("p2", "Um"))]),
            secao("HomeFeedBaselineSectionData", Some("Outro motivo"), vec![embrulho(album_json("al2", "Dois"))]),
            secao("HomeFeedBaselineSectionData", Some("Mais um"), vec![embrulho(playlist_json("p2", "Um"))]),
            secao("HomeFeedBaselineSectionData", None, vec![episodio])
        ]}}}});
        let p = inicio(&data).expect("início");
        let titulos: Vec<&str> = p.iter().map(|p| p.titulo.as_str()).collect();
        assert_eq!(titulos, ["Para começar", "Feito para você", "Discos", "Sugestões para você"]);
        use TipoDaPrateleira as T;
        let tipos: Vec<T> = p.iter().map(|p| p.tipo).collect();
        assert_eq!(tipos, [T::Atalhos, T::ParaVoce, T::Outras, T::ParaVoce]);
        assert_eq!(tipo_da_secao("HomeGenericSectionData", "Lançamentos pra você", &[]), T::Lancamentos);
        assert_eq!(tipo_da_secao("HomeGenericSectionData", "Paradas em destaque", &[]), T::Paradas);
        assert_eq!(tipo_da_secao("HomeGenericSectionData", "Tocados recentemente", &[]), T::Recentes);
        assert_eq!(tipo_da_secao("HomeGenericSectionData", "Seus artistas favoritos", &p[3].itens[..0]), T::Outras);
        assert_eq!(p[0].itens.len(), 1, "o tipo desconhecido é pulado");
        assert_eq!(p[2].itens.len(), 2);
        assert_eq!(p[3].itens.len(), 2, "as sugestões de um item só ficam juntas, sem repetir");
        assert!(inicio(&json!({"home": {}})).is_err());
        assert!(inicio(&json!({"home": {"sectionContainer": {"sections": {"items": []}}}})).is_err(), "nada é erro");
    }

    #[test]
    fn album_leva_o_disco_em_cada_faixa() {
        let mut sem_album = faixa_json("f1", "Um");
        sem_album.as_object_mut().expect("objeto").remove("albumOfTrack");
        let data = json!({"albumUnion": {"uri": "spotify:album:al1", "name": "Disco", "coverArt": capas(),
            "tracksV2": {"items": [{"track": sem_album}, {"track": null}], "totalCount": 14}}});
        let Pagina { itens: faixas, total, .. } = album(&data).expect("álbum");
        assert_eq!((faixas.len(), total), (1, 14));
        assert_eq!((faixas[0].album.as_deref(), faixas[0].album_id.as_deref()), (Some("Disco"), Some("al1")));
        assert_eq!(faixas[0].capa.as_deref(), Some("c300"));
        assert!(album(&json!({"albumUnion": {"__typename": "NotFound"}})).is_err());
    }

    #[test]
    fn artista_com_populares_e_discografia() {
        let data =
            json!({"artistUnion": {"discography": {"topTracks": {"items": [{"track": faixa_json("f1", "Um")}]}}}});
        assert_eq!(populares(&data).expect("populares")[0].id, "f1");
        let poucas = json!({"artistUnion": {"discography": {"topTracks": {"items": []}}}});
        assert_eq!(populares(&poucas), Ok(Vec::new()), "artista sem populares");
        assert!(populares(&json!({"artistUnion": {}})).is_err());
        let data = json!({"artistUnion": {"discography": {"all": {"totalCount": 36, "items": [
            {"releases": {"items": [{"uri": "spotify:album:al1", "id": "al1", "name": "Disco", "date": {"year": 2023}, "coverArt": capas()}]}},
            {"releases": {"items": []}}
        ]}}}});
        let Pagina { itens: albuns, total, .. } = discografia(&data).expect("discografia");
        assert_eq!((albuns.len(), total), (1, 36));
        assert_eq!(
            (albuns[0].id.as_str(), albuns[0].ano, albuns[0].capa.as_deref()),
            ("al1", Some(2023), Some("c300"))
        );
    }

    #[test]
    fn detalhe_do_artista_do_album_e_da_playlist() {
        let data = json!({"artistUnion": {
            "uri": "spotify:artist:a1", "profile": {"name": "Lume", "biography": {"text": "De <a href=\"x\">São Paulo</a> &amp; região."}},
            "headerImage": {"data": {"sources": [
                {"maxWidth": 640, "url": "p"}, {"maxWidth": 2660, "url": "g"}, {"maxWidth": 1494, "url": "m"}]}},
            "visuals": {"avatarImage": {"sources": [{"height": 320, "url": "foto"}], "extractedColors": {"colorRaw": {"hex": "#B0A840"}}},
                "gallery": {"items": [{"sources": [{"height": 640, "url": "g1"}]}]}},
            "stats": {"followers": 12, "monthlyListeners": 28289213},
            "discography": {"latest": album_json("al9", "Novo")},
            "relatedContent": {"relatedArtists": {"items": [artista_json("r1", "Parecido")]}, "featuringV2": {"items": [null]}},
        }});
        let d = detalhe_do_artista(&data);
        assert_eq!(d.imagem.as_deref(), Some("m"), "a menor que serve");
        assert_eq!(d.descricao.as_deref(), Some("De São Paulo & região."));
        assert_eq!((d.seguidores, d.ouvintes, d.cor.as_deref()), (Some(12), Some(28_289_213), Some("#B0A840")));
        assert!(matches!(&d.item, Some(Item::Artista(a)) if a.foto.as_deref() == Some("foto")));
        let tipos: Vec<_> = d.secoes.iter().map(|s| s.tipo).collect();
        assert_eq!(tipos, [TipoDaPrateleira::UltimoLancamento, TipoDaPrateleira::Parecidos], "a vazia não vem");
        // Um artista sem nada disso não quebra.
        assert_eq!(
            detalhe_do_artista(&json!({"artistUnion": {"headerImage": null, "discography": {"latest": null}}})),
            Detalhe::default()
        );

        let data = json!({"albumUnion": {
            "uri": "spotify:album:d1", "name": "Disco", "type": "EP", "label": "Selo",
            "date": {"isoString": "2001-03-12T00:00:00Z", "precision": "DAY"},
            "coverArt": {"sources": [{"height": 300, "url": "c"}], "extractedColors": {"colorDark": {"hex": "#302040"}}},
            "tracksV2": {"totalCount": 14},
            "moreAlbumsByArtist": {"items": [{"discography": {"popularReleasesAlbums": {"items": [
                album_json("d1", "Disco"), album_json("d2", "Outro")]}}}]},
        }});
        let d = detalhe_do_album(&data);
        assert_eq!((d.data.as_deref(), d.gravadora.as_deref(), d.total), (Some("2001-03-12"), Some("Selo"), Some(14)));
        assert_eq!(d.tipo, Some(TipoDeAlbum::Ep));
        assert!(matches!(&d.item, Some(Item::Album(a)) if a.ano == Some(2001) && a.tipo == Some(TipoDeAlbum::Ep)));
        assert_eq!(d.secoes[0].itens.len(), 1, "sem ele mesmo");

        let data = json!({"playlistV2": {"uri": "spotify:playlist:p1", "name": "Lista", "description": "", "followers": "169",
            "ownerV2": {"data": {"name": "Ana"}}, "content": {"totalCount": 50}}});
        let d = detalhe_da_playlist(&data);
        assert_eq!((d.seguidores, d.total, d.dono.as_deref(), d.descricao), (Some(169), Some(50), Some("Ana"), None));
    }

    #[test]
    fn html_vira_texto_puro() {
        assert_eq!(sem_html("a <b>b</b> &lt;3 e 2 < 3"), "a b <3 e 2 < 3");
        assert_eq!(sem_html("<a href='x'>sem fim"), "sem fim");
        assert_eq!(sem_html("quebrado <a href"), "quebrado <a href");
        assert_eq!(sem_html("&amp;lt;"), "&lt;", "uma vez só");
    }

    #[test]
    fn playlist_pula_o_que_nao_e_faixa() {
        let mut faixa = faixa_json("f1", "Um");
        let o = faixa.as_object_mut().expect("objeto");
        o.remove("duration");
        o.insert("trackDuration".into(), json!({"totalMilliseconds": 1000}));
        let mut local = faixa_json("x", "Local");
        local["uri"] = json!("spotify:local:Ana::Local:100");
        let data = json!({"playlistV2": {"content": {"totalCount": 250, "items": [
            {"itemV2": embrulho(faixa)},
            {"itemV2": embrulho(json!({"__typename": "Episode", "uri": "spotify:episode:e1", "name": "E"}))},
            {"itemV2": embrulho(local)},
            {"itemV2": {"__typename": "TrackResponseWrapper", "data": {"__typename": "NotFound"}}},
            {"itemV2": null}
        ]}}});
        let Pagina { itens: faixas, total, .. } = playlist(&data).expect("playlist");
        assert_eq!((faixas.len(), total), (1, 250));
        assert_eq!(faixas[0].duracao_s, Some(1.0));
        assert!(playlist(&json!({"playlistV2": {"__typename": "NotFound"}})).is_err(), "playlist que não existe");
    }

    #[test]
    fn curtidas_com_o_endereco_no_embrulho() {
        let mut faixa = faixa_json("f1", "Um");
        faixa.as_object_mut().expect("objeto").remove("uri");
        let data = json!({"me": {"library": {"tracks": {"totalCount": 900, "items": [
            {"track": {"_uri": "spotify:track:f1", "data": faixa}},
            {"track": {"_uri": "spotify:local:x", "data": faixa_json("l", "L")["x"]}}
        ]}}}});
        let Pagina { itens: faixas, total, .. } = curtidas(&data).expect("curtidas");
        assert_eq!((faixas.len(), faixas[0].id.as_str(), total), (1, "f1", 900));
        assert!(curtidas(&json!({"me": {}})).is_err());
    }

    #[test]
    fn biblioteca_com_as_curtidas_como_playlist() {
        let item = |dado: Value| json!({"item": embrulho(dado)});
        let data = json!({"me": {"libraryV3": {"totalCount": 5, "items": [
            {"item": {"__typename": "LibraryPseudoPlaylistResponseWrapper", "_uri": "spotify:collection:tracks",
                "data": {"__typename": "PseudoPlaylist", "uri": "spotify:collection:tracks", "name": "Liked Songs", "count": 900}}},
            {"item": {"__typename": "LibraryPseudoPlaylistResponseWrapper", "data": {"uri": "spotify:collection:your-episodes"}}},
            item(playlist_json("p1", "Lista")), item(album_json("al1", "Disco")), item(artista_json("a1", "Ana")),
            {"item": {"__typename": "LibraryFolderResponseWrapper", "data": {"name": "Pasta"}}}
        ]}}});
        let Pagina { itens, total, .. } = biblioteca(&data).expect("biblioteca");
        assert_eq!((itens.len(), total), (4, 5));
        let Item::Playlist(c) = &itens[0] else { panic!("as curtidas primeiro") };
        assert_eq!((c.id.as_str(), c.titulo.as_str(), c.quantas), (CURTIDAS, "Músicas Curtidas", Some(900)));
        let vazia = json!({"me": {"libraryV3": {"totalCount": 0, "items": []}}});
        assert!(biblioteca(&vazia).is_ok_and(|p| p.itens.is_empty()), "biblioteca vazia não é erro");
        assert!(biblioteca(&json!({"me": null})).is_err());
    }

    #[test]
    fn lista_nula_ao_lado_do_total_e_erro_e_nao_vazia() {
        let cortada = json!({"totalCount": 250, "items": null});
        assert!(playlist(&json!({"playlistV2": {"content": cortada}})).is_err());
        assert!(curtidas(&json!({"me": {"library": {"tracks": cortada}}})).is_err());
        assert!(biblioteca(&json!({"me": {"libraryV3": cortada}})).is_err());
        assert!(album(&json!({"albumUnion": {"tracksV2": cortada}})).is_err());
        assert!(discografia(&json!({"artistUnion": {"discography": {"all": cortada}}})).is_err());
        let nulos = json!({"searchV2": {"topResultsV2": null, "tracksV2": null, "artists": null, "albumsV2": null, "playlists": null}});
        assert!(busca(&nulos, 20).is_err(), "nenhum grupo veio");
        assert_eq!(populares(&json!({"artistUnion": {"discography": {"topTracks": null}}})), Ok(Vec::new()));
    }

    #[test]
    fn campo_nulo_nao_derruba() {
        let pelado = json!({"__typename": "Album", "uri": "spotify:album:al", "name": "Só o nome", "date": null, "artists": null, "coverArt": null});
        let data = json!({"searchV2": {"topResultsV2": null, "tracksV2": null, "artists": null, "playlists": null,
            "albumsV2": {"items": [embrulho(pelado), null, embrulho(json!({"__typename": "Album", "name": "Sem endereço"}))]}}});
        let itens = busca(&data, 20).expect("busca");
        assert!(
            matches!(&itens[..], [Item::Album(a)] if a.titulo == "Só o nome" && a.ano.is_none() && a.capa.is_none())
        );
    }
}
