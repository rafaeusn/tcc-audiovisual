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

/// Uma música do acervo.
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
}

impl Musica {
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
    pub quantas: u32,
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
