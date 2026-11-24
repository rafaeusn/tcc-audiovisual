//! Do formato do `rustypipe` para o formato comum dos provedores. Só contas, testadas sem internet.

use provedores::{Album, Artista, Faixa, Item, Playlist};
use rustypipe::model::{AlbumItem, ArtistId, ArtistItem, MusicPlaylistItem, Thumbnail, TrackItem, TrackType};

/// O serviço, como ele vai em toda faixa.
pub const SERVICO: &str = "youtube-music";
/// A cor da marca, para o espaço do serviço no Player.
pub const COR: &str = "#FF0000";

/// O id de cada artista, na ordem dos nomes (`""` sem id). Nenhum id: vetor vazio (nada vai no JSON).
pub fn ids(artistas: &[(Option<&str>, &str)]) -> Vec<String> {
    if artistas.iter().all(|(id, _)| id.is_none_or(str::is_empty)) {
        return Vec::new();
    }
    artistas.iter().map(|(id, _)| id.unwrap_or_default().to_string()).collect()
}

/// Os artistas com nome (os sem nome ficam de fora, como em `artistas`), com o id de cada um.
fn com_nome(artistas: &[ArtistId]) -> Vec<(Option<&str>, &str)> {
    artistas.iter().filter(|a| !a.name.is_empty()).map(|a| (a.id.as_deref(), a.name.as_str())).collect()
}

/// A maior capa da lista.
pub fn capa(miniaturas: &[Thumbnail]) -> Option<String> {
    maior(miniaturas.iter().map(|t| (u64::from(t.width) * u64::from(t.height), t.url.as_str())))
}

/// O endereço da maior imagem (pela área).
fn maior<'a>(imagens: impl Iterator<Item = (u64, &'a str)>) -> Option<String> {
    imagens.max_by_key(|(area, _)| *area).map(|(_, url)| url.to_string())
}

pub fn faixa(t: &TrackItem) -> Faixa {
    Faixa {
        servico: SERVICO.into(),
        id: t.id.clone(),
        isrc: None,
        titulo: t.name.clone(),
        artistas: t.artists.iter().map(|a| a.name.clone()).filter(|n| !n.is_empty()).collect(),
        artistas_ids: ids(&com_nome(&t.artists)),
        album: t.album.as_ref().map(|a| a.name.clone()),
        album_id: t.album.as_ref().map(|a| a.id.clone()).filter(|id| !id.is_empty()),
        duracao_s: t.duration.map(f64::from),
        capa: capa(&t.cover),
        video: t.track_type != TrackType::Track,
    }
}

pub fn album_item(a: &AlbumItem) -> Album {
    Album {
        servico: SERVICO.into(),
        id: a.id.clone(),
        titulo: a.name.clone(),
        artistas: a.artists.iter().map(|x| x.name.clone()).collect(),
        artistas_ids: ids(&a.artists.iter().map(|x| (x.id.as_deref(), x.name.as_str())).collect::<Vec<_>>()),
        ano: a.year,
        capa: capa(&a.cover),
    }
}

pub fn artista_item(a: &ArtistItem) -> Artista {
    Artista { servico: SERVICO.into(), id: a.id.clone(), nome: a.name.clone(), foto: capa(&a.avatar) }
}

pub fn playlist_item(p: &MusicPlaylistItem) -> Playlist {
    Playlist {
        servico: SERVICO.into(),
        id: p.id.clone(),
        titulo: p.name.clone(),
        dono: p.channel.as_ref().map(|c| c.name.clone()),
        capa: capa(&p.thumbnail),
        quantas: p.track_count.and_then(|n| u32::try_from(n).ok()),
    }
}

/// Junta a busca principal (de todos os tipos, o melhor resultado primeiro) com as buscas por tipo: o melhor fica
/// na frente (é o que o Player mostra em destaque); depois faixas, álbuns, artistas e playlists, sem repetir e no
/// máximo `limite` de cada tipo.
pub fn juntar_busca(principal: Vec<Item>, por_tipo: Vec<Vec<Item>>, limite: usize) -> Vec<Item> {
    fn tipo(i: &Item) -> usize {
        match i {
            Item::Faixa(_) => 0,
            Item::Album(_) => 1,
            Item::Artista(_) => 2,
            Item::Playlist(_) => 3,
        }
    }
    fn id(i: &Item) -> &str {
        match i {
            Item::Faixa(f) => &f.id,
            Item::Album(a) => &a.id,
            Item::Artista(a) => &a.id,
            Item::Playlist(p) => &p.id,
        }
    }
    // A busca por tipo traz cada item mais completo que a principal: quando as duas têm o mesmo, vale o dela.
    let por_tipo: Vec<Item> = por_tipo.into_iter().flatten().collect();
    let melhor_versao = |item: Item| -> Item {
        por_tipo.iter().find(|x| tipo(x) == tipo(&item) && id(x) == id(&item)).cloned().unwrap_or(item)
    };
    let mut vistos = std::collections::HashSet::new();
    let mut quantos = [0usize; 4];
    let mut juntos = Vec::new();
    let mut por = |item: Item, juntos: &mut Vec<Item>| {
        let t = tipo(&item);
        if quantos[t] < limite && vistos.insert((t, id(&item).to_string())) {
            quantos[t] += 1;
            juntos.push(item);
        }
    };
    let mut principal = principal.into_iter();
    if let Some(melhor) = principal.next() {
        por(melhor_versao(melhor), &mut juntos);
    }
    let resto: Vec<Item> = por_tipo.iter().cloned().chain(principal).collect();
    for t in 0..4 {
        for item in resto.iter().filter(|i| tipo(i) == t) {
            por(item.clone(), &mut juntos);
        }
    }
    juntos
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_maior_capa_e_nenhuma_sem_lista() {
        assert_eq!(maior([(3_600, "p"), (295_936, "g"), (14_400, "m")].into_iter()).as_deref(), Some("g"));
        assert_eq!(maior(std::iter::empty()), None);
        assert_eq!(capa(&[]), None);
    }

    #[test]
    fn ids_dos_artistas_na_mesma_ordem() {
        assert_eq!(ids(&[(Some("UC1"), "A"), (None, "B")]), vec!["UC1".to_string(), String::new()]);
        assert!(ids(&[(None, "A"), (None, "B")]).is_empty(), "nenhum id: vetor vazio");
    }

    #[test]
    fn juntar_busca_poe_o_melhor_na_frente_e_nao_repete() {
        let al = |id: &str| Item::Album(Album { servico: SERVICO.into(), id: id.into(), ..Default::default() });
        let fx = |id: &str| Item::Faixa(Faixa { servico: SERVICO.into(), id: id.into(), ..Default::default() });
        let ar = |id: &str| Item::Artista(Artista { servico: SERVICO.into(), id: id.into(), ..Default::default() });
        let principal = vec![ar("melhor"), fx("f1"), al("a1")];
        let por_tipo = vec![vec![fx("f1"), fx("f2"), fx("f3")], vec![al("a1"), al("a2")], vec![ar("melhor")]];
        let juntos = juntar_busca(principal, por_tipo, 2);
        let chaves: Vec<String> = juntos
            .iter()
            .map(|i| match i {
                Item::Faixa(f) => format!("f:{}", f.id),
                Item::Album(a) => format!("a:{}", a.id),
                Item::Artista(a) => format!("r:{}", a.id),
                Item::Playlist(p) => format!("p:{}", p.id),
            })
            .collect();
        assert_eq!(chaves, ["r:melhor", "f:f1", "f:f2", "a:a1", "a:a2"], "o melhor na frente, sem repetir, 2 por tipo");
        assert!(juntar_busca(Vec::new(), Vec::new(), 5).is_empty());
        // A principal traz as faixas pobres (sem duração, "Song" no lugar do artista): vale a da busca por tipo.
        let pobre = Item::Faixa(Faixa {
            titulo: "pobre".into(),
            ..Faixa { servico: SERVICO.into(), id: "f1".into(), ..Default::default() }
        });
        let boa = Item::Faixa(Faixa {
            titulo: "boa".into(),
            ..Faixa { servico: SERVICO.into(), id: "f1".into(), ..Default::default() }
        });
        let juntos = juntar_busca(vec![pobre.clone(), pobre], vec![vec![boa.clone()]], 5);
        assert_eq!(juntos, vec![boa], "o melhor e o resto saem da busca por tipo quando ela tem o item");
    }

    #[test]
    fn a_logo_e_a_cor_que_vao_ao_player_sao_validas() {
        let i = provedores::Identidade { cor: Some(COR.into()), logo: Some(include_str!("logo.svg").into()) };
        assert_eq!(i.cor_rgb(), Some(0xFF0000));
        assert!(i.logo_valida().is_some(), "o Player recusaria a logo");
    }
}
