//! Erros do app. Cada parte converte a falha dela para um destes casos, com contexto legível em pt-BR.
//! Nenhuma falha de áudio, GPU ou sessão derruba o app: quem chama registra e segue com o que tem.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum Erro {
    #[error("arquivo {caminho}: {fonte}")]
    Arquivo {
        caminho: PathBuf,
        #[source]
        fonte: std::io::Error,
    },
    #[error("ajustes inválidos: {0}")]
    Ajustes(String),
    #[error("áudio: {0}")]
    Audio(String),
    #[error("vídeo: {0}")]
    Gpu(String),
    #[error("sessão de mídia: {0}")]
    Sessao(String),
    #[error("histórico: {0}")]
    Dados(String),
    #[error("rede: {0}")]
    Rede(String),
}

pub type Resultado<T> = Result<T, Erro>;

impl Erro {
    pub fn arquivo(caminho: impl Into<PathBuf>, fonte: std::io::Error) -> Self {
        Self::Arquivo { caminho: caminho.into(), fonte }
    }
}
