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
    #[error("provedor: {0}")]
    Provedor(String),
    /// Um provedor respondeu com erro e disse de que tipo foi (F6): o Player escolhe se tenta de novo, troca de
    /// jeito de tocar ou desiste daquela faixa.
    #[error("provedor: {mensagem}")]
    ProvedorComCausa { mensagem: String, causa: CausaDoProvedor },
}

/// Por que o provedor falhou, do jeito que o Player entende. A lista acompanha o protocolo (uma causa nova entra nos
/// dois lados). `Outra` é o que o Player não conhece: vale como erro comum.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CausaDoProvedor {
    Rede,
    Limite,
    Indisponivel,
    SemConta,
    SemPremium,
    SessaoRecusada,
    ChaveDeAudio,
    Quebrado,
    Outra,
}

pub type Resultado<T> = Result<T, Erro>;

impl Erro {
    pub fn arquivo(caminho: impl Into<PathBuf>, fonte: std::io::Error) -> Self {
        Self::Arquivo { caminho: caminho.into(), fonte }
    }

    /// A causa de um erro de provedor, quando ele disse; `None` para os outros erros e para um erro sem causa.
    pub fn causa_do_provedor(&self) -> Option<CausaDoProvedor> {
        match self {
            Self::ProvedorComCausa { causa, .. } => Some(*causa),
            _ => None,
        }
    }

    /// O texto do erro sem o "provedor:" / "rede:" que o tipo põe na frente.
    pub fn mensagem(&self) -> String {
        match self {
            Self::Provedor(m) => m.clone(),
            Self::ProvedorComCausa { mensagem, .. } => mensagem.clone(),
            outro => outro.to_string(),
        }
    }
}
