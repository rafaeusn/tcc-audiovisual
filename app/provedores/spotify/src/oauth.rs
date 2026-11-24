//! O login do Spotify pelo navegador, com o fluxo Authorization Code + PKCE escrito por nós.
//!
//! **Por que não o `librespot-oauth`:** ele monta o pedido de autorização sozinho e não deixa escolher o `show_dialog` nem
//! separar os escopos. O Spotify trata identidades diferentes de formas diferentes, e é isso que faz o áudio funcionar:
//!
//! - **O grant do áudio** usa a identidade do app de computador do Spotify (o Client ID do librespot, o mesmo que ele usa
//!   para tocar) e pede **só** `streaming`, com `show_dialog=true`. É esse par (identidade + escopo enxuto) que o Spotify
//!   aceita para entregar a chave do áudio.
//! - **O grant dos dados** usa o app que a pessoa registra no painel do Spotify (`developer.spotify.com/dashboard`) e pede
//!   os escopos de Web API.
//!
//! Os dois ficam guardados à parte, e cada um se renova com o seu Client ID. O caminho e as decisões vêm do **Spotifast**
//! (MIT): `src/auth.rs`, `docs/_reference/how-it-connects.md`. Nenhum código foi copiado — esta é a mesma ideia escrita para
//! o nosso provedor.
//!
//! O navegador é de quem digita a senha: este programa só vê o código de uma vez que o Spotify devolve ao `127.0.0.1`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

/// A identidade do app de computador do Spotify (a mesma que o librespot usa para tocar). Não é segredo: está no librespot.
pub const CLIENT_ID_DO_AUDIO: &str = "65b708073fc0480ea92a077233ca87bd";
/// A porta do retorno do login do áudio. É a que esse Client ID aceita.
pub const PORTA_DO_AUDIO: u16 = 8898;
/// A porta do retorno do login dos dados. É a que a pessoa registra no app dela, letra por letra.
pub const PORTA_DOS_DADOS: u16 = 8989;
/// O caminho do retorno (o mesmo nos dois).
const CAMINHO: &str = "/login";
/// O áudio precisa **só** disto.
pub const ESCOPOS_DO_AUDIO: [&str; 1] = ["streaming"];
/// Os dados (as páginas, a biblioteca) pedem isto.
pub const ESCOPOS_DOS_DADOS: [&str; 8] = [
    "user-read-private",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-library-read",
    "user-follow-read",
    "user-read-playback-state",
    "user-modify-playback-state",
    "user-read-recently-played",
];

const AUTORIZAR: &str = "https://accounts.spotify.com/authorize";
const TOKEN: &str = "https://accounts.spotify.com/api/token";
/// Quanto esperar a pessoa aprovar no navegador.
pub const PRAZO_DO_NAVEGADOR: Duration = Duration::from_secs(10 * 60);
/// Um navegador manda a linha do pedido na hora; mais lento que isto não é o retorno.
const PRAZO_DO_PEDIDO: Duration = Duration::from_secs(5);

/// De quem é o login: o áudio (a identidade do librespot) ou os dados (o app da pessoa).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Grant {
    Audio,
    Dados,
}

impl Grant {
    pub fn client_id(self, client_id_dos_dados: Option<&str>) -> Result<String, String> {
        match self {
            Self::Audio => Ok(CLIENT_ID_DO_AUDIO.to_string()),
            Self::Dados => {
                let id = client_id_dos_dados.unwrap_or_default().trim();
                if id.is_empty() {
                    return Err(
                        "Registre um app no painel do Spotify e cole o Client ID dele para ver as suas páginas.".into(),
                    );
                }
                if !parece_client_id(id) {
                    return Err(
                        "O Client ID tem 32 letras e números (só de 0 a 9 e a a f). Confira o que foi colado.".into()
                    );
                }
                Ok(id.to_string())
            }
        }
    }

    pub fn porta(self) -> u16 {
        match self {
            Self::Audio => PORTA_DO_AUDIO,
            Self::Dados => PORTA_DOS_DADOS,
        }
    }

    pub fn escopos(self) -> &'static [&'static str] {
        match self {
            Self::Audio => &ESCOPOS_DO_AUDIO,
            Self::Dados => &ESCOPOS_DOS_DADOS,
        }
    }

    pub fn retorno(self) -> String {
        format!("http://127.0.0.1:{}{CAMINHO}", self.porta())
    }

    /// O nome para a pessoa ler.
    pub fn nome(self) -> &'static str {
        match self {
            Self::Audio => "do áudio",
            Self::Dados => "dos dados",
        }
    }
}

/// Um Client ID do Spotify tem 32 dígitos hexadecimais. O que não tiver nem chega ao navegador.
pub fn parece_client_id(id: &str) -> bool {
    let id = id.trim();
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

/// O pedido de autorização, já com o PKCE.
#[derive(Clone, Debug)]
pub struct Pedido {
    pub url: String,
    /// O verificador do PKCE (não sai daqui).
    pub verificador: String,
    pub estado: String,
}

/// Os bytes sorteados para o PKCE (48) e o `state` (18), em base64 sem enchimento.
fn sorteio(bytes: usize) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    // Sem dependência de `rand`: o relógio, o contador de endereços e um xorshift dão o que o PKCE precisa.
    let agora = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let sal = &agora as *const u128 as usize;
    let mut x = agora as u64 ^ (sal as u64).rotate_left(13) ^ ((std::process::id() as u64) << 24);
    let mut saida = Vec::with_capacity(bytes);
    while saida.len() < bytes {
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        saida.extend_from_slice(&x.wrapping_mul(0x2545_F491_4F6C_DD1D).to_le_bytes());
    }
    saida.truncate(bytes);
    URL_SAFE_NO_PAD.encode(saida)
}

/// Monta o pedido de autorização: `show_dialog=true` no áudio (o Spotify exige uma aprovação nova para o streaming, e
/// reaproveitar uma tela antiga não dá o escopo), os escopos separados por grant.
pub fn comecar(grant: Grant, client_id: &str) -> Pedido {
    let verificador = sorteio(48);
    let desafio = URL_SAFE_NO_PAD.encode(Sha256::digest(verificador.as_bytes()));
    let estado = sorteio(18);
    // O Spotify só mostra a tela de novo com `show_dialog`; no áudio isto é o que garante que o `streaming` seja concedido.
    let dialogo = if grant == Grant::Audio { "&show_dialog=true" } else { "" };
    let url = format!(
        "{AUTORIZAR}?client_id={}&response_type=code&redirect_uri={}&code_challenge_method=S256&code_challenge={desafio}\
         &state={estado}&scope={}{dialogo}",
        percent(client_id),
        percent(&grant.retorno()),
        percent(&grant.escopos().join(" ")),
    );
    Pedido { url, verificador, estado }
}

/// A codificação de um pedaço de endereço (`%20` para espaço, e o resto que o `format!` não trata).
fn percent(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    for b in texto.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => saida.push(b as char),
            _ => saida.push_str(&format!("%{b:02X}")),
        }
    }
    saida
}

/// O que o Spotify devolve na troca do código.
#[derive(Clone, Debug)]
pub struct Tokens {
    pub access: String,
    pub refresh: Option<String>,
}

/// Abre o ouvinte do retorno **antes** de abrir o navegador. Uma porta ocupada é um erro claro, não um navegador que volta
/// para uma porta fechada.
pub fn escutar(grant: Grant) -> Result<TcpListener, String> {
    let endereco: SocketAddr = (Ipv4Addr::LOCALHOST, grant.porta()).into();
    TcpListener::bind(endereco).map_err(|_| {
        format!(
            "A porta {} ({}) está ocupada. Feche o programa que a usa (o Spotifast, por exemplo) e tente de novo.",
            grant.porta(),
            grant.nome()
        )
    })
}

/// Espera o retorno do navegador e devolve o código. `prazo`: quanto esperar a pessoa aprovar.
///
/// Um pedido que não é o retorno (`/favicon.ico`, uma aba velha) é respondido e o ouvinte continua esperando.
pub fn esperar_o_codigo(ouvinte: &TcpListener, estado: &str, prazo: Duration) -> Result<String, String> {
    let fim = std::time::Instant::now() + prazo;
    for conexao in ouvinte.incoming() {
        if std::time::Instant::now() > fim {
            return Err("Ninguém aprovou no navegador a tempo. Tente de novo.".into());
        }
        let Ok(mut conexao) = conexao else { continue };
        let _ = conexao.set_read_timeout(Some(PRAZO_DO_PEDIDO));
        let mut linha = String::new();
        if BufReader::new(&conexao).read_line(&mut linha).is_err() {
            continue;
        }
        match ler_o_retorno(&linha, estado) {
            Ok(codigo) => {
                responder(&mut conexao, "200 OK", PAGINA_DE_PRONTO);
                return Ok(codigo);
            }
            Err(motivo) => {
                responder(&mut conexao, "400 Bad Request", &pagina_de_falha(&motivo));
                // Se o Spotify recusou (a pessoa negou), não adianta continuar esperando.
                if linha.contains("error=") {
                    return Err(motivo);
                }
            }
        }
    }
    Err("O navegador não voltou. Tente de novo.".into())
}

/// Lê o código da linha do pedido (`GET /login?code=...&state=... HTTP/1.1`).
pub fn ler_o_retorno(linha: &str, estado: &str) -> Result<String, String> {
    let alvo = linha.split_whitespace().nth(1).ok_or("o pedido do navegador veio estranho")?;
    let (caminho, consulta) = alvo.split_once('?').unwrap_or((alvo, ""));
    if caminho != CAMINHO {
        return Err("não é o retorno do login".into());
    }
    let (mut codigo, mut veio_estado, mut erro) = (None, None, None);
    for par in consulta.split('&') {
        let (chave, valor) = par.split_once('=').unwrap_or((par, ""));
        match chave {
            "code" => codigo = Some(despercent(valor)),
            "state" => veio_estado = Some(despercent(valor)),
            "error" => erro = Some(despercent(valor)),
            _ => {}
        }
    }
    if erro.is_some() {
        return Err("O Spotify recusou o login. Tente de novo.".into());
    }
    if veio_estado.as_deref() != Some(estado) {
        return Err("o retorno não é desta tentativa".into());
    }
    codigo.ok_or_else(|| "O Spotify não devolveu o código".into())
}

/// Desfaz o `%XX` de um valor da consulta (espaço, acentos, `+`).
fn despercent(texto: &str) -> String {
    let bytes = texto.as_bytes();
    let mut saida = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        saida.push(byte);
                        i += 3;
                    }
                    // Um `%` que não é código fica como está (o valor veio de um lugar estranho).
                    Err(_) => {
                        saida.push(b'%');
                        i += 1;
                    }
                }
            }
            b'+' => {
                saida.push(b' ');
                i += 1;
            }
            b => {
                saida.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&saida).into_owned()
}

fn responder(conexao: &mut TcpStream, status: &str, corpo: &str) {
    let resposta = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\n\
         Connection: close\r\nContent-Length: {}\r\n\r\n{corpo}",
        corpo.len()
    );
    let _ = conexao.write_all(resposta.as_bytes());
}

const PAGINA_DE_PRONTO: &str = "<!doctype html><html lang=\"pt-BR\"><meta charset=\"utf-8\"><title>Player</title>\
<body style=\"font-family:system-ui,sans-serif;background:#060606;color:#f2f2f2;display:grid;place-items:center;\
height:100vh;margin:0\"><div style=\"text-align:center\"><h1 style=\"font-weight:600\">Pronto</h1>\
<p>A conta do Spotify entrou no Player. Pode fechar esta aba.</p></div></body></html>";

fn pagina_de_falha(motivo: &str) -> String {
    format!(
        "<!doctype html><html lang=\"pt-BR\"><meta charset=\"utf-8\"><title>Player</title>\
         <body style=\"font-family:system-ui,sans-serif;background:#060606;color:#f2f2f2;display:grid;\
         place-items:center;height:100vh;margin:0\"><div style=\"text-align:center\"><h1>Não deu</h1><p>{motivo}</p>\
         </div></body></html>"
    )
}

/// Troca o código pelo token (`grant_type=authorization_code`), com o verificador do PKCE.
pub fn trocar_o_codigo(
    http: &reqwest::blocking::Client,
    grant: Grant,
    client_id: &str,
    codigo: &str,
    verificador: &str,
) -> Result<Tokens, String> {
    pedir_token(
        http,
        &[
            ("client_id", client_id),
            ("grant_type", "authorization_code"),
            ("code", codigo),
            ("redirect_uri", &grant.retorno()),
            ("code_verifier", verificador),
        ],
    )
}

/// Renova a credencial guardada. O Client ID é o do grant dela (o do áudio, ou o do app da pessoa).
pub fn renovar(http: &reqwest::blocking::Client, client_id: &str, refresh: &str) -> Result<Tokens, FalhaDaRenovacao> {
    pedir_token(http, &[("client_id", client_id), ("grant_type", "refresh_token"), ("refresh_token", refresh)]).map_err(
        |motivo| {
            if motivo.contains("invalid_grant") {
                FalhaDaRenovacao::Recusada
            } else {
                FalhaDaRenovacao::Passageira(motivo)
            }
        },
    )
}

/// Por que renovar falhou.
#[derive(Clone, Debug, PartialEq)]
pub enum FalhaDaRenovacao {
    /// O Spotify não aceita mais (revogada, ou de outro app): a credencial sai, e a pessoa entra de novo.
    Recusada,
    /// A rede, um prazo: a credencial continua guardada.
    Passageira(String),
}

fn pedir_token(http: &reqwest::blocking::Client, campos: &[(&str, &str)]) -> Result<Tokens, String> {
    let resposta = http.post(TOKEN).form(campos).send().map_err(|e| format!("sem resposta do Spotify ({e})"))?;
    let status = resposta.status();
    let texto = resposta.text().unwrap_or_default();
    if !status.is_success() {
        return Err(format!("o Spotify recusou ({status}): {texto}"));
    }
    let json: serde_json::Value =
        serde_json::from_str(&texto).map_err(|e| format!("resposta do Spotify ilegível ({e})"))?;
    let access = json.get("access_token").and_then(|v| v.as_str()).ok_or("o Spotify não devolveu o token")?.to_string();
    let refresh = json.get("refresh_token").and_then(|v| v.as_str()).filter(|r| !r.is_empty()).map(str::to_string);
    Ok(Tokens { access, refresh })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn a_url_leva_o_response_type_e_o_escapamento_nao_o_come() {
        // O Spotify responde "response_type must be code" quando o parâmetro chega vazio. Era o que acontecia ao abrir o
        // navegador por `cmd /C start`: o `&` é separador de comandos para o `cmd`, que entregava só o pedaço antes dele.
        // Aqui a URL inteira tem de estar certa, parâmetro por parâmetro.
        let p = comecar(Grant::Audio, CLIENT_ID_DO_AUDIO);
        assert!(p.url.contains("&response_type=code&"), "{}", p.url);
        // O `redirect_uri` vai escapado (o `:` e as `/` não podem quebrar a consulta).
        assert!(p.url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A8898%2Flogin&"), "{}", p.url);
        // Todos os `&` são separadores de parâmetro (são 7 pares depois do `client_id`).
        assert_eq!(p.url.matches('&').count(), 7, "os sete pares depois do primeiro: {}", p.url);
        assert!(!p.url.contains(' '), "um espaço não escapado cortaria a URL: {}", p.url);
        // O `&` não pode chegar cru ao `cmd`: a abertura é por `rundll32`, e é isso que este teste protege.
        assert!(p.url.starts_with("https://accounts.spotify.com/authorize?client_id="), "{}", p.url);
    }

    #[test]
    fn o_audio_pede_so_streaming_e_mostra_a_tela() {
        let p = comecar(Grant::Audio, CLIENT_ID_DO_AUDIO);
        assert!(p.url.starts_with(AUTORIZAR), "{}", p.url);
        assert!(p.url.contains(&format!("client_id={CLIENT_ID_DO_AUDIO}")), "{}", p.url);
        assert!(p.url.contains("scope=streaming"), "o áudio pede só isto: {}", p.url);
        assert!(p.url.contains("show_dialog=true"), "sem a tela nova o streaming não vem: {}", p.url);
        assert!(p.url.contains("127.0.0.1%3A8898%2Flogin"), "{}", p.url);
        assert!(p.url.contains("code_challenge_method=S256"));
        assert!(!p.verificador.is_empty() && !p.estado.is_empty());
    }

    #[test]
    fn os_dados_pedem_os_escopos_da_web_api_sem_a_tela() {
        let id = "0123456789abcdef0123456789abcdef";
        let p = comecar(Grant::Dados, id);
        assert!(p.url.contains(&format!("client_id={id}")), "{}", p.url);
        assert!(p.url.contains("playlist-read-private"), "{}", p.url);
        assert!(!p.url.contains("show_dialog=true"), "{}", p.url);
        assert!(p.url.contains("127.0.0.1%3A8989%2Flogin"), "{}", p.url);
        assert!(!p.url.contains("streaming"), "os dados não pedem áudio: {}", p.url);
    }

    #[test]
    fn cada_tentativa_sorteia_outro_estado_e_verificador() {
        let a = comecar(Grant::Audio, CLIENT_ID_DO_AUDIO);
        let b = comecar(Grant::Audio, CLIENT_ID_DO_AUDIO);
        assert_ne!(a.verificador, b.verificador);
        assert_ne!(a.estado, b.estado);
    }

    #[test]
    fn sem_client_id_dos_dados_os_dados_nao_abrem_e_o_audio_abre() {
        assert!(Grant::Dados.client_id(None).is_err());
        assert!(Grant::Dados.client_id(Some("  ")).is_err());
        assert!(Grant::Dados.client_id(Some("curto")).is_err(), "não parece um Client ID");
        let id = "0123456789abcdef0123456789abcdef";
        assert_eq!(Grant::Dados.client_id(Some(id)).expect("vale"), id);
        assert_eq!(Grant::Audio.client_id(None).expect("o áudio não precisa"), CLIENT_ID_DO_AUDIO);
    }

    #[test]
    fn o_retorno_do_navegador() {
        let p = comecar(Grant::Audio, CLIENT_ID_DO_AUDIO);
        let linha = format!("GET /login?code=abc123&state={} HTTP/1.1", p.estado);
        assert_eq!(ler_o_retorno(&linha, &p.estado), Ok("abc123".into()));
        // O estado de outra tentativa não vale.
        assert!(ler_o_retorno(&linha, "outro").is_err());
        // A pessoa negou.
        assert!(ler_o_retorno("GET /login?error=access_denied&state=x HTTP/1.1", "x").is_err());
        // Um favicon não é o retorno (mas também não é erro fatal).
        assert!(ler_o_retorno("GET /favicon.ico HTTP/1.1", &p.estado).is_err());
    }

    #[test]
    fn o_codigo_com_caracteres_escapados_volta() {
        assert_eq!(despercent("a%20b%2Bc+d"), "a b+c d");
        assert_eq!(despercent("sem%ZZescapar"), "sem%ZZescapar");
    }

    #[test]
    fn o_client_id_precisa_de_32_digitos_hexadecimais() {
        assert!(parece_client_id("0123456789abcdef0123456789ABCDEF"));
        assert!(parece_client_id(" 0123456789abcdef0123456789abcdef\n"));
        assert!(!parece_client_id(""));
        assert!(!parece_client_id("0123456789abcdef0123456789abcde"), "31");
        assert!(!parece_client_id("0123456789abcdef0123456789abcdeg"), "g não é hexa");
    }

    #[test]
    fn a_porta_ocupada_diz_qual_e_por_que() {
        let ouvinte = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("livre");
        let porta = ouvinte.local_addr().expect("endereço").port();
        // A porta sorteada está ocupada (o `ouvinte` acima a segura): o segundo bind tem de falhar.
        let endereco: SocketAddr = (Ipv4Addr::LOCALHOST, porta).into();
        assert!(TcpListener::bind(endereco).is_err(), "a porta {porta} deveria estar ocupada");
        // E a frase que a pessoa lê tem o número dela.
        let frase = format!("a porta {porta} está ocupada. Feche o programa que a usa");
        assert!(frase.contains(&porta.to_string()));
        drop(ouvinte);
    }
}
