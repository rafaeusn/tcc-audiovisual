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
    /// O id do álbum no serviço (para a página do álbum), quando o provedor sabe.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album_id: Option<String>,
    /// O id de cada artista, na mesma ordem de `artistas` (`""` quando não tem).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artistas_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duracao_s: Option<f64>,
    /// Endereço da capa.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capa: Option<String>,
    /// Um vídeo (clipe, ao vivo gravado, envio de fã), e não a faixa do álbum.
    pub video: bool,
    /// Quantas vezes foi ouvida no serviço.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ouvidas: Option<u64>,
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

    /// O id do artista `i` (o mesmo índice de `artistas`), quando o provedor mandou.
    pub fn id_do_artista(&self, i: usize) -> Option<&str> {
        id_do_artista(&self.artistas, &self.artistas_ids, i)
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
    /// O id de cada artista, na mesma ordem de `artistas` (`""` quando não tem).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub artistas_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ano: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capa: Option<String>,
    #[serde(rename = "tipo_de_album", skip_serializing_if = "Option::is_none")]
    pub tipo: Option<TipoDeAlbum>,
}

/// O que um lançamento é. Um tipo que o Player não conhece vale como `Outro`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TipoDeAlbum {
    Album,
    Single,
    Ep,
    Coletanea,
    #[serde(other)]
    Outro,
}

impl Album {
    /// O id do artista `i` (o mesmo índice de `artistas`), quando o provedor mandou.
    pub fn id_do_artista(&self, i: usize) -> Option<&str> {
        id_do_artista(&self.artistas, &self.artistas_ids, i)
    }
}

/// Só vale o id que tem um nome no mesmo lugar: listas de tamanhos diferentes não trocam um artista por outro.
fn id_do_artista<'a>(nomes: &[String], ids: &'a [String], i: usize) -> Option<&'a str> {
    nomes.get(i)?;
    ids.get(i).map(String::as_str).filter(|id| !id.is_empty())
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

/// O que uma prateleira traz: em Tudo, as do mesmo tipo de serviços diferentes aparecem juntas. Um tipo que o Player ainda
/// não conhece vale como `Outras`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TipoDaPrateleira {
    /// O que a pessoa mais abre (os botões do alto do app).
    Atalhos,
    ParaVoce,
    Lancamentos,
    /// O que tocou há pouco.
    Recentes,
    Artistas,
    Clipes,
    Paradas,
    /// Na página de um artista: o lançamento mais novo e os artistas parecidos.
    UltimoLancamento,
    Parecidos,
    /// Na página de um álbum: outros do mesmo artista.
    MaisDoArtista,
    #[default]
    #[serde(other)]
    Outras,
}

/// Uma fileira da página inicial do serviço ("Para você", "Lançamentos"...).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prateleira {
    pub titulo: String,
    #[serde(skip_serializing_if = "outras")]
    pub tipo: TipoDaPrateleira,
    pub itens: Vec<Item>,
}

fn outras(tipo: &TipoDaPrateleira) -> bool {
    *tipo == TipoDaPrateleira::Outras
}

/// O que se sabe de um álbum, artista ou playlist além das faixas. Tudo opcional: cada serviço manda o que tem.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Detalhe {
    /// O item completo (a página pode ter sido aberta por um link, só com o id e o nome).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<Item>,
    /// Uma imagem larga, para o fundo do cabeçalho.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imagem: Option<String>,
    /// A cor da imagem, `#RRGGBB`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cor: Option<String>,
    /// A biografia do artista, a descrição da playlist ou do álbum (texto puro).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descricao: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub fotos: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seguidores: Option<u64>,
    /// Ouvintes no último mês.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ouvintes: Option<u64>,
    /// Quantas faixas tem ao todo.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duracao_s: Option<f64>,
    /// `AAAA`, `AAAA-MM` ou `AAAA-MM-DD`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gravadora: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tipo: Option<TipoDeAlbum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dono: Option<String>,
    /// As outras prateleiras da página (o último lançamento, os parecidos, mais do artista).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub secoes: Vec<Prateleira>,
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
    /// Toca sozinho, num programa do próprio serviço que a pessoa liga (o Soloist do Spotify): ver `Tocador`.
    TocaSozinho,
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
        // A prateleira de um provedor antigo não tem tipo; a de um mais novo pode ter um que não conhecemos.
        let sem: Prateleira = serde_json::from_str(r#"{"titulo":"P","itens":[]}"#).expect("sem tipo");
        assert_eq!(sem.tipo, TipoDaPrateleira::Outras);
        assert!(!serde_json::to_string(&sem).expect("json").contains("tipo"));
        let novo: Prateleira = serde_json::from_str(r#"{"titulo":"P","tipo":"para-dormir","itens":[]}"#).expect("novo");
        assert_eq!(novo.tipo, TipoDaPrateleira::Outras);
        let conhecido: Prateleira =
            serde_json::from_str(r#"{"titulo":"P","tipo":"para-voce","itens":[]}"#).expect("tipo");
        assert_eq!(conhecido.tipo, TipoDaPrateleira::ParaVoce);
        assert_eq!(minimo.servico(), "s");
    }

    #[test]
    fn detalhe_e_tipo_de_album_opcionais() {
        let d: Detalhe = serde_json::from_str(r#"{"ouvintes":28289213,"novo":[1]}"#).expect("campos a mais");
        assert_eq!(d, Detalhe { ouvintes: Some(28_289_213), ..Default::default() });
        assert_eq!(serde_json::to_string(&Detalhe::default()).expect("json"), "{}", "vazio não pesa");
        let a: Item = serde_json::from_str(r#"{"tipo":"album","id":"1","tipo_de_album":"ep"}"#).expect("álbum");
        assert!(matches!(&a, Item::Album(a) if a.tipo == Some(TipoDeAlbum::Ep)));
        assert_eq!(serde_json::from_str::<Item>(&serde_json::to_string(&a).expect("vai")).expect("volta"), a);
        let novo: Item = serde_json::from_str(r#"{"tipo":"album","id":"1","tipo_de_album":"fita"}"#).expect("novo");
        assert!(matches!(novo, Item::Album(a) if a.tipo == Some(TipoDeAlbum::Outro)));
        let velho: Item = serde_json::from_str(r#"{"tipo":"album","id":"1"}"#).expect("velho");
        assert!(matches!(velho, Item::Album(a) if a.tipo.is_none()));
    }

    #[test]
    fn capacidades_em_kebab_case() {
        let c: Vec<Capacidade> = serde_json::from_str(r#"["busca","editar-playlists","audio"]"#).expect("lista");
        assert_eq!(c, vec![Capacidade::Busca, Capacidade::EditarPlaylists, Capacidade::Audio]);
    }

    #[test]
    fn ids_de_album_e_artista_opcionais() {
        let f: Faixa =
            serde_json::from_str(r#"{"servico":"s","id":"1","titulo":"T","artistas":["A","B"]}"#).expect("lê");
        assert_eq!(f.album_id, None);
        assert_eq!(f.id_do_artista(0), None);
        let f = Faixa { artistas_ids: vec!["a1".into(), String::new()], ..f };
        assert_eq!(f.id_do_artista(0), Some("a1"));
        assert_eq!(f.id_do_artista(1), None, "id vazio");
        assert_eq!(f.id_do_artista(5), None, "fora da lista");
        let json = serde_json::to_string(&Faixa { artistas_ids: Vec::new(), ..f.clone() }).expect("escreve");
        assert!(!json.contains("artistas_ids"), "vazio não vai no JSON");
        let a = Album { artistas: vec!["A".into()], artistas_ids: vec!["x".into(), "y".into()], ..Default::default() };
        assert_eq!(a.id_do_artista(1), None, "mais ids que nomes: o id sem nome não vale");
    }
}
