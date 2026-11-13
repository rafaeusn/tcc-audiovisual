//! Núcleo do Player: o que todas as outras partes compartilham e que não depende de áudio, GPU nem interface.

pub mod ajustes;
pub mod caminhos;
pub mod comando;
pub mod erro;
pub mod registro;

pub use ajustes::{Ajustes, Modelo, Modulo, Skin, Tema};
pub use comando::{Comando, Pagina};
pub use erro::{Erro, Resultado};

/// Nome provisório do app (pasta de dados, título da janela, instância única).
pub const NOME_APP: &str = "TccPlayer";
