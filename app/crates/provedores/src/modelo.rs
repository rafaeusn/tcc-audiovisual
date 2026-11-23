//! O que circula entre o Player e os provedores: faixas, álbuns, artistas, playlists e prateleiras, iguais para
//! todo serviço.

use serde::{Deserialize, Serialize};

/// Uma música (ou um vídeo com música) de um serviço. `servico` + `id` identificam a faixa; o ISRC, quando vem,
/// identifica a gravação em qualquer serviço.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Faixa {
    pub servico: String,
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub isrc: Option<String>,
    pub titulo: String,
    pub artistas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duracao_s: Option<f64>,
    /// Endereço da capa.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capa: Option<String>,
    /// Um vídeo (clipe, ao vivo gravado, envio de fã), e não a faixa do álbum.
    pub video: bool,
}

impl Faixa {
    /// `servico:id`, a chave da faixa em todo o app (fila, cache de análise, pares entre serviços).
    pub fn chave(&self) -> String {
        format!("{}:{}", self.servico, self.id)
    }

    /// Os artistas juntos, como se escreve: "A, B".
    pub fn artistas_juntos(&self) -> String {
        self.artistas.join(", ")
    }

    /// O ISRC só com letras e números, em maiúsculas (cada serviço escreve de um jeito).
    pub fn isrc_limpo(&self) -> Option<String> {
        let limpo: String =
            self.isrc.as_deref()?.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).collect();
        (limpo.len() == 12).then_some(limpo)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Album {
    pub servico: String,
    pub id: String,
    pub titulo: String,
    pub artistas: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ano: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capa: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Artista {
    pub servico: String,
    pub id: String,
    pub nome: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub foto: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Playlist {
    pub servico: String,
    pub id: String,
    pub titulo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dono: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capa: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantas: Option<u32>,
}

/// Um resultado de busca ou um cartão de prateleira.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "tipo", rename_all = "kebab-case")]
pub enum Item {
    Faixa(Faixa),
    Album(Album),
    Artista(Artista),
    Playlist(Playlist),
}

impl Item {
    pub fn servico(&self) -> &str {
        match self {
            Self::Faixa(f) => &f.servico,
            Self::Album(a) => &a.servico,
            Self::Artista(a) => &a.servico,
            Self::Playlist(p) => &p.servico,
        }
    }

    pub fn faixa(&self) -> Option<&Faixa> {
        match self {
            Self::Faixa(f) => Some(f),
            _ => None,
        }
    }
}

/// Uma fileira da página inicial do serviço ("Para você", "Lançamentos"...).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prateleira {
    pub titulo: String,
    pub itens: Vec<Item>,
}

/// O que um provedor sabe fazer. Uma capacidade nova que o Player ainda não conhece é ignorada.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capacidade {
    Busca,
    Inicio,
    Biblioteca,
    Playlists,
    EditarPlaylists,
    Curtir,
    Artista,
    Album,
    Radio,
    Letra,
    /// Entrega o áudio (por endereço ou tocando sozinho).
    Audio,
    Conta,
    Podcast,
    Video,
    Canvas,
    Reconhecer,
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn faixa_vai_e_volta_em_json_e_aceita_campos_faltando() {
        let f = Faixa {
            servico: "exemplo".into(),
            id: "1".into(),
            isrc: Some("br-x12-26-00001".into()),
            titulo: "Nome".into(),
            artistas: vec!["A".into(), "B".into()],
            duracao_s: Some(200.0),
            ..Default::default()
        };
        let item = Item::Faixa(f.clone());
        let json = serde_json::to_string(&item).expect("json");
        assert!(json.contains(r#""tipo":"faixa""#), "{json}");
        assert!(!json.contains("capa"), "campos vazios não vão: {json}");
        assert_eq!(serde_json::from_str::<Item>(&json).expect("volta"), item);
        assert_eq!(f.chave(), "exemplo:1");
        assert_eq!(f.artistas_juntos(), "A, B");
        assert_eq!(f.isrc_limpo().as_deref(), Some("BRX122600001"));
        assert_eq!(Faixa { isrc: Some("curto".into()), ..f }.isrc_limpo(), None);
        // Um provedor que manda só o essencial (ou campos a mais) ainda é entendido.
        let minimo: Item =
            serde_json::from_str(r#"{"tipo":"faixa","servico":"s","id":"9","titulo":"T","novo":1}"#).expect("mínimo");
        assert_eq!(minimo.faixa().map(|f| f.titulo.as_str()), Some("T"));
        assert_eq!(minimo.servico(), "s");
    }

    #[test]
    fn capacidades_em_kebab_case() {
        let c: Vec<Capacidade> = serde_json::from_str(r#"["busca","editar-playlists","audio"]"#).expect("lista");
        assert_eq!(c, vec![Capacidade::Busca, Capacidade::EditarPlaylists, Capacidade::Audio]);
    }
}
