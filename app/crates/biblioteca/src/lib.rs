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
}

/// Forma de comparar e buscar: minúsculas, sem acento, um espaço entre as palavras.
pub fn normalizar(s: &str) -> String {
    let mut saida = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase) {
        let c = match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            outro => outro,
        };
        if c.is_alphanumeric() {
            saida.push(c);
        } else if !saida.ends_with(' ') && !saida.is_empty() {
            saida.push(' ');
        }
    }
    saida.trim_end().to_string()
}

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
