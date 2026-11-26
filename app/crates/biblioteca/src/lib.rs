//! As músicas do computador.
//!
//! A pessoa escolhe pastas; o app as varre, lê as etiquetas de cada arquivo (título, artista, álbum, número da
//! faixa, duração) e guarda num acervo local, de onde saem a lista e a busca. Nada é copiado nem alterado: os
//! arquivos ficam onde estão e o acervo só aponta para eles.
//!
//! Arquivo sem etiqueta entra com o nome do arquivo como título. Arquivo que a leitura não entende também
//! entra (só com o nome): se der para tocar, toca.

mod acervo;
mod leitura;

pub use acervo::Acervo;
pub use leitura::numero_da_etiqueta;
pub use leitura::{EXTENSOES, capa, ler, varrer};

use std::path::PathBuf;

/// De onde vem uma música que não é um arquivo: uma faixa de um provedor (YouTube Music, Spotify...).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Remota {
    pub servico: String,
    pub id: String,
    pub isrc: Option<String>,
    /// O endereço da capa.
    pub capa: Option<String>,
    /// O nome do provedor, para mostrar ("YouTube Music").
    pub fonte: String,
    /// O id do álbum e os artistas com o id de cada um (`""` quando não tem), para os links. Só na memória: uma faixa lida
    /// de uma playlist guardada não os tem.
    pub album_id: Option<String>,
    pub artistas: Vec<String>,
    pub artistas_ids: Vec<String>,
}

/// Uma música do acervo, ou uma faixa de provedor (com `remota`). Na faixa de provedor, `caminho` não é um
/// arquivo: é a chave `servico:id`, que a identifica na fila, na pré-análise e no resto do app.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Musica {
    pub caminho: PathBuf,
    pub titulo: String,
    pub artista: String,
    pub album: String,
    /// Número da faixa no álbum (0 quando a etiqueta não diz).
    pub numero: u32,
    /// 0 quando não deu para medir.
    pub duracao_s: f64,
    /// ReplayGain da faixa, em dB, quando o arquivo traz (igualar o volume entre músicas).
    pub ganho_db: Option<f32>,
    /// O pico da faixa (1 = o máximo sem distorcer), quando o arquivo traz.
    pub pico: Option<f32>,
    /// Faixa de provedor (não é um arquivo do computador).
    pub remota: Option<Remota>,
}

impl Musica {
    /// É uma faixa de provedor, e não um arquivo.
    pub fn e_remota(&self) -> bool {
        self.remota.is_some()
    }

    /// O ganho para tocar esta música com o volume igualado: o ReplayGain mais `preamp_db`, sem deixar o pico
    /// passar de 1. Sem ReplayGain no arquivo, 1 (como está).
    pub fn ganho_igualado(&self, preamp_db: f32) -> f32 {
        let Some(db) = self.ganho_db else { return 1.0 };
        let ganho = 10f32.powf((db + preamp_db) / 20.0);
        match self.pico.filter(|p| *p > 0.0) {
            Some(pico) => ganho.min(1.0 / pico),
            None => ganho,
        }
    }
}

/// O que a lista de músicas mostra: de onde vêm as músicas e os filtros por cima.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Filtro {
    pub consulta: String,
    pub origem: Origem,
    /// Só as deste álbum (nome como está nas etiquetas).
    pub album: Option<String>,
    /// Só as deste artista.
    pub artista: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Origem {
    /// Todas as pastas, cada música uma vez.
    #[default]
    Todas,
    Pasta(PathBuf),
    /// Uma playlist, na ordem dela.
    Playlist(i64),
}

/// Um álbum ou um artista, com quantas músicas tem e um arquivo de exemplo (para a capa).
#[derive(Clone, Debug, PartialEq)]
pub struct Grupo {
    pub nome: String,
    /// No álbum, o artista; no artista, vazio.
    pub detalhe: String,
    pub quantas: u32,
    pub exemplo: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Playlist {
    pub id: i64,
    pub nome: String,
    /// As músicas que dá para tocar: as do acervo e as dos serviços.
    pub quantas: u32,
    /// Quantas são faixas de serviço (com nenhuma, a playlist é só do computador).
    pub remotas: u32,
}

/// O que se salva de um serviço (ou do computador).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TipoSalvo {
    Faixa,
    Album,
    Artista,
    Playlist,
}

impl TipoSalvo {
    pub const TODOS: [TipoSalvo; 4] = [TipoSalvo::Faixa, TipoSalvo::Album, TipoSalvo::Artista, TipoSalvo::Playlist];

    /// Como o tipo fica no banco.
    pub fn chave(self) -> &'static str {
        match self {
            Self::Faixa => "faixa",
            Self::Album => "album",
            Self::Artista => "artista",
            Self::Playlist => "playlist",
        }
    }

    pub fn da_chave(s: &str) -> Option<Self> {
        Self::TODOS.into_iter().find(|t| t.chave() == s)
    }
}

/// Uma coisa que a pessoa curtiu ou salvou: faixa, álbum, artista ou playlist de um serviço. A faixa do computador
/// usa o serviço `computador` e o caminho como id. `dados` é o item como o Player o escreveu (JSON), para abrir sem
/// rede; o acervo não lê o que tem dentro. Nunca guarda áudio.
#[derive(Clone, Debug, PartialEq)]
pub struct Salvo {
    pub servico: String,
    pub tipo: TipoSalvo,
    pub id: String,
    pub dados: String,
    /// Quando foi salvo (segundos Unix).
    pub quando: i64,
    /// Fixado: vem primeiro na lista.
    pub fixado: bool,
}

/// Forma de comparar e buscar (a mesma das letras, no `nucleo`).
pub use nucleo::texto::normalizar;

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn normaliza_para_buscar() {
        assert_eq!(normalizar("  Coração (Ao Vivo) "), "coracao ao vivo");
        assert_eq!(normalizar("AC/DC"), "ac dc");
        assert_eq!(normalizar(""), "");
    }
}
