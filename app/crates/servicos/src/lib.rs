//! Os serviços dos módulos da Loja.
//!
//! - [`lastfm`]: scrobble. Avisa o Last.fm quando uma música começa e a conta quando ela passa da metade (ou de
//!   4 minutos), que é a regra do próprio serviço. A conta é autorizada no navegador; o app nunca vê a senha.
//! - [`discord`]: o cartão "ouvindo" no perfil. Fala só com o Discord aberto neste computador, por um canal
//!   local. Não há login nem rede.
//! - [`marcador`]: a regra de quando uma música começou e quando ela conta, sem rede, para dar para testar.
//! - [`cofre`]: guarda a autorização do Last.fm cifrada pelo Windows para o usuário atual.
//!
//! As chaves que identificam o app nesses serviços não ficam no código: vêm de variáveis de ambiente, lidas na
//! compilação ou ao abrir ([`Chaves`]). Sem elas o módulo correspondente avisa que não está configurado.

pub mod cofre;
pub mod discord;
pub mod lastfm;
pub mod marcador;

/// Uma faixa, como os serviços a recebem.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Faixa {
    pub titulo: String,
    pub artista: String,
    pub album: String,
}

/// As chaves do app nos serviços. Cada uma vem da variável de ambiente de mesmo nome: a que existir ao abrir o
/// app vale; senão, a que existia na hora de compilar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Chaves {
    /// `LASTFM_API_KEY` e `LASTFM_API_SECRET`.
    pub lastfm: Option<(String, String)>,
    /// `DISCORD_APP_ID`.
    pub discord: Option<String>,
}

fn variavel(nome: &str, da_compilacao: Option<&'static str>) -> Option<String> {
    std::env::var(nome)
        .ok()
        .or_else(|| da_compilacao.map(str::to_string))
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

impl Chaves {
    pub fn ler() -> Self {
        let chave = variavel("LASTFM_API_KEY", option_env!("LASTFM_API_KEY"));
        let segredo = variavel("LASTFM_API_SECRET", option_env!("LASTFM_API_SECRET"));
        Self { lastfm: chave.zip(segredo), discord: variavel("DISCORD_APP_ID", option_env!("DISCORD_APP_ID")) }
    }
}
