//! Letras que acompanham a música.
//!
//! - [`ler_lrc`] lê o formato LRC (uma linha de texto por tempo) e [`linha_em`] diz qual linha vale num instante.
//! - [`Cliente`] busca nas fontes em cadeia ([`fontes`]): Better Lyrics e Unison (palavra por palavra, em TTML,
//!   convertido por [`ttml`]) e o [LRCLIB](https://lrclib.net). Para achar a letra ele envia o título, o artista, o
//!   álbum e a duração da faixa (e, para a Unison, o id do vídeo, quando a faixa é do YouTube Music). Nada mais sai
//!   do computador, e nada é enviado se a letra não estiver à mostra.
//! - [`limpar`] arruma o que os apps publicam ("Artista - Música (Official Video)") antes de perguntar.
//!
//! Quem chama a rede é uma thread própria do app. Tudo aqui é "melhor esforço": sem letra ou sem internet, o
//! resto do app segue igual.

pub mod fontes;
mod lrc;
mod lrclib;
mod texto;
pub mod traducao;
pub mod ttml;

pub use fontes::{Achada, Fonte};
pub use lrc::{Linha, Palavra, cantado, ler_lrc, linha_em};
pub use lrclib::{Cliente, Pedido, Registro, escolher};
pub use texto::{chave, limpar, normalizar};
pub use traducao::{Traducao, romanizar};
