//! Do formato do `rustypipe` para o formato comum dos provedores. Só contas, testadas sem internet.

use provedores::{Album, Faixa};
use rustypipe::model::{AlbumItem, Thumbnail, TrackItem, TrackType};

/// O serviço, como ele vai em toda faixa.
pub const SERVICO: &str = "youtube-music";

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
        album: t.album.as_ref().map(|a| a.name.clone()),
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
        ano: a.year,
        capa: capa(&a.cover),
    }
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
}
