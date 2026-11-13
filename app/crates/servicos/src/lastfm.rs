//! Last.fm (<https://www.last.fm/api>): autorização pelo navegador e scrobble.
//!
//! A autorização é a de programas de desktop: o app pede um código (`auth.getToken`), abre a página do Last.fm
//! para a pessoa aprovar e depois troca o código por uma chave de sessão (`auth.getSession`), que não expira. A
//! senha nunca passa pelo app.
//!
//! Toda chamada que fala em nome da pessoa é assinada: MD5 dos parâmetros em ordem alfabética, seguidos do
//! segredo do app. É o que o Last.fm exige; o MD5 aqui não protege nada, só identifica o app.

use crate::Faixa;
use md5::{Digest, Md5};
use nucleo::{Erro, Resultado};
use std::io::Read;
use std::time::Duration;

const RAIZ: &str = "https://ws.audioscrobbler.com/2.0/";
const PAGINA_DE_AUTORIZACAO: &str = "https://www.last.fm/api/auth/";
const MAXIMO_DA_RESPOSTA: u64 = 256 * 1024;
/// Códigos de erro do Last.fm que interessam.
const ERRO_NAO_AUTORIZADO_AINDA: i64 = 14;
const ERRO_SESSAO_INVALIDA: i64 = 9;

/// Uma falha ao falar com o Last.fm.
#[derive(Debug, PartialEq)]
pub enum Falha {
    /// A pessoa ainda não aprovou na página (só durante a autorização).
    Aguardando,
    /// A chave de sessão não vale mais: é preciso autorizar de novo.
    SessaoInvalida,
    /// Sem internet, ou o Last.fm respondeu um erro.
    Rede(String),
}

impl From<Falha> for Erro {
    fn from(f: Falha) -> Self {
        Erro::Rede(match f {
            Falha::Aguardando => "aguardando a autorização no Last.fm".into(),
            Falha::SessaoInvalida => "a autorização do Last.fm não vale mais".into(),
            Falha::Rede(m) => m,
        })
    }
}

/// A conta autorizada.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conta {
    pub usuario: String,
    pub sessao: String,
}

impl Conta {
    /// Uma linha de texto para o cofre.
    pub fn para_texto(&self) -> String {
        format!("{}\n{}", self.usuario, self.sessao)
    }

    pub fn de_texto(texto: &str) -> Option<Self> {
        let (usuario, sessao) = texto.split_once('\n')?;
        (!usuario.is_empty() && !sessao.trim().is_empty())
            .then(|| Self { usuario: usuario.to_string(), sessao: sessao.trim().to_string() })
    }
}

/// A assinatura de uma chamada: MD5 de "chave1valor1chave2valor2..." (chaves em ordem) + o segredo do app.
pub fn assinatura(parametros: &[(&str, String)], segredo: &str) -> String {
    let mut ordenados: Vec<&(&str, String)> = parametros.iter().collect();
    ordenados.sort_by_key(|(chave, _)| *chave);
    let mut md5 = Md5::new();
    for (chave, valor) in ordenados {
        md5.update(chave.as_bytes());
        md5.update(valor.as_bytes());
    }
    md5.update(segredo.as_bytes());
    md5.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// A página onde a pessoa aprova o app.
pub fn pagina_de_autorizacao(chave: &str, codigo: &str) -> String {
    format!("{PAGINA_DE_AUTORIZACAO}?api_key={chave}&token={codigo}")
}

pub struct Cliente {
    chave: String,
    segredo: String,
    agente: ureq::Agent,
}

impl Cliente {
    pub fn novo(chave: &str, segredo: &str) -> Resultado<Self> {
        let tls = native_tls::TlsConnector::new().map_err(|e| Erro::Rede(e.to_string()))?;
        let agente = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(6))
            .timeout(Duration::from_secs(12))
            .user_agent(concat!("Player-Escuta/", env!("CARGO_PKG_VERSION")))
            .tls_connector(std::sync::Arc::new(tls))
            .build();
        Ok(Self { chave: chave.to_string(), segredo: segredo.to_string(), agente })
    }

    pub fn chave(&self) -> &str {
        &self.chave
    }

    /// Uma chamada assinada. Devolve o JSON da resposta.
    fn chamar(&self, metodo: &str, extras: Vec<(&str, String)>) -> Result<serde_json::Value, Falha> {
        let mut parametros = extras;
        parametros.push(("method", metodo.to_string()));
        parametros.push(("api_key", self.chave.clone()));
        let assinada = assinatura(&parametros, &self.segredo);
        parametros.push(("api_sig", assinada));
        parametros.push(("format", "json".to_string()));
        let formulario: Vec<(&str, &str)> = parametros.iter().map(|(c, v)| (*c, v.as_str())).collect();

        // O Last.fm devolve os erros dele com código HTTP de erro e o motivo no corpo: os dois casos são lidos.
        let resposta = match self.agente.post(RAIZ).send_form(&formulario) {
            Ok(r) | Err(ureq::Error::Status(_, r)) => r,
            Err(e) => return Err(Falha::Rede(e.to_string())),
        };
        let mut corpo = String::new();
        resposta
            .into_reader()
            .take(MAXIMO_DA_RESPOSTA)
            .read_to_string(&mut corpo)
            .map_err(|e| Falha::Rede(e.to_string()))?;
        let json: serde_json::Value =
            serde_json::from_str(&corpo).map_err(|e| Falha::Rede(format!("resposta inesperada do Last.fm: {e}")))?;
        interpretar(json)
    }

    /// Pede o código que a pessoa vai aprovar no navegador.
    pub fn codigo(&self) -> Result<String, Falha> {
        let json = self.chamar("auth.getToken", Vec::new())?;
        json["token"].as_str().map(str::to_string).ok_or_else(|| Falha::Rede("o Last.fm não devolveu o código".into()))
    }

    /// Troca o código aprovado pela conta. [`Falha::Aguardando`] enquanto a pessoa não aprova.
    pub fn conta(&self, codigo: &str) -> Result<Conta, Falha> {
        let json = self.chamar("auth.getSession", vec![("token", codigo.to_string())])?;
        let sessao = &json["session"];
        match (sessao["name"].as_str(), sessao["key"].as_str()) {
            (Some(usuario), Some(chave)) => Ok(Conta { usuario: usuario.to_string(), sessao: chave.to_string() }),
            _ => Err(Falha::Rede("o Last.fm não devolveu a conta".into())),
        }
    }

    fn parametros_da_faixa(conta: &Conta, f: &Faixa, duracao_s: Option<f64>) -> Vec<(&'static str, String)> {
        let mut p = vec![("sk", conta.sessao.clone()), ("track", f.titulo.clone()), ("artist", f.artista.clone())];
        if !f.album.is_empty() {
            p.push(("album", f.album.clone()));
        }
        if let Some(d) = duracao_s.filter(|d| d.is_finite() && *d >= 1.0) {
            p.push(("duration", format!("{}", d.round() as i64)));
        }
        p
    }

    /// Avisa que a faixa começou a tocar ("tocando agora" no perfil).
    pub fn tocando_agora(&self, conta: &Conta, f: &Faixa, duracao_s: Option<f64>) -> Result<(), Falha> {
        self.chamar("track.updateNowPlaying", Self::parametros_da_faixa(conta, f, duracao_s)).map(|_| ())
    }

    /// Conta a faixa no histórico. `inicio` é quando ela começou a tocar (segundos desde 1970).
    pub fn scrobble(&self, conta: &Conta, f: &Faixa, inicio: i64, duracao_s: Option<f64>) -> Result<(), Falha> {
        let mut p = Self::parametros_da_faixa(conta, f, duracao_s);
        p.push(("timestamp", inicio.to_string()));
        self.chamar("track.scrobble", p).map(|_| ())
    }
}

/// Separa uma resposta boa de um erro do Last.fm (`{"error": 9, "message": "..."}`).
fn interpretar(json: serde_json::Value) -> Result<serde_json::Value, Falha> {
    match json["error"].as_i64() {
        None => Ok(json),
        Some(ERRO_NAO_AUTORIZADO_AINDA) => Err(Falha::Aguardando),
        Some(ERRO_SESSAO_INVALIDA) => Err(Falha::SessaoInvalida),
        Some(codigo) => {
            Err(Falha::Rede(format!("Last.fm, erro {codigo}: {}", json["message"].as_str().unwrap_or("sem descrição"))))
        }
    }
}

/// O Last.fm precisa do artista: sem ele (título vindo do navegador, por exemplo) a faixa não é enviada.
pub fn pode_enviar(f: &Faixa) -> bool {
    !f.titulo.trim().is_empty() && !f.artista.trim().is_empty()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn assinatura_ordena_os_parametros() {
        // Exemplo conferido à mão: md5("api_keyxyzmethodauth.getTokensegredo").
        let p = vec![("method", "auth.getToken".to_string()), ("api_key", "xyz".to_string())];
        let a = assinatura(&p, "segredo");
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        let invertido = vec![("api_key", "xyz".to_string()), ("method", "auth.getToken".to_string())];
        assert_eq!(a, assinatura(&invertido, "segredo"), "a ordem de entrada não importa");
        assert_ne!(a, assinatura(&p, "outro"));
        // Valor conhecido do MD5 de texto vazio.
        assert_eq!(assinatura(&[], ""), "d41d8cd98f00b204e9800998ecf8427e");
        assert_eq!(assinatura(&[("a", "b".to_string())], "c"), "900150983cd24fb0d6963f7d28e17f72", "md5 de \"abc\"");
    }

    #[test]
    fn respostas_e_erros() {
        let ok = serde_json::json!({ "token": "abc" });
        assert_eq!(interpretar(ok.clone()), Ok(ok));
        assert_eq!(interpretar(serde_json::json!({ "error": 14, "message": "x" })), Err(Falha::Aguardando));
        assert_eq!(interpretar(serde_json::json!({ "error": 9, "message": "x" })), Err(Falha::SessaoInvalida));
        assert_eq!(
            interpretar(serde_json::json!({ "error": 29, "message": "Rate limit" })),
            Err(Falha::Rede("Last.fm, erro 29: Rate limit".into()))
        );
    }

    #[test]
    fn conta_vai_e_volta_do_cofre() {
        let c = Conta { usuario: "fulana".into(), sessao: "chave123".into() };
        assert_eq!(Conta::de_texto(&c.para_texto()), Some(c));
        assert_eq!(Conta::de_texto("sem quebra"), None);
        assert_eq!(Conta::de_texto("\nchave"), None);
        assert_eq!(Conta::de_texto("fulana\n "), None);
    }

    #[test]
    fn pagina_e_o_que_da_para_enviar() {
        assert_eq!(pagina_de_autorizacao("K", "T"), "https://www.last.fm/api/auth/?api_key=K&token=T");
        let f = |t: &str, a: &str| Faixa { titulo: t.into(), artista: a.into(), album: String::new() };
        assert!(pode_enviar(&f("Música", "Banda")));
        assert!(!pode_enviar(&f("Música", " ")));
        assert!(!pode_enviar(&f("", "Banda")));
    }
}
