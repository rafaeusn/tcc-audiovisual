//! Letras que acompanham a música.
//!
//! - [`ler_lrc`] lê o formato LRC (uma linha de texto por tempo) e [`linha_em`] diz qual linha vale num instante.
//! - [`Cliente`] busca no [LRCLIB](https://lrclib.net), um acervo aberto mantido pela comunidade: sem conta, sem
//!   chave. Para achar a letra ele envia o título, o artista, o álbum e a duração da faixa. Nada mais sai do
//!   computador, e nada é enviado se o módulo Letra não estiver no painel.
//! - [`limpar`] arruma o que os apps publicam ("Artista - Música (Official Video)") antes de perguntar.
//!
//! Quem chama a rede é uma thread própria do app. Tudo aqui é "melhor esforço": sem letra ou sem internet, o
//! resto do app segue igual.

mod lrc;
mod lrclib;
mod texto;

pub use lrc::{Linha, ler_lrc, linha_em};
pub use lrclib::{Cliente, Pedido, Registro, escolher};
pub use texto::{chave, limpar, normalizar};
