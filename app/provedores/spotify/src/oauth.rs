//! O login do Spotify pelo navegador: o fluxo Authorization Code com PKCE, escrito aqui (a ideia veio da branch
//! `feat/spotify-audio-e-investigacao`). O `librespot-oauth` esperava o navegador para sempre, aceitava a primeira conexão
//! que chegasse à porta (uma ociosa do navegador prendia o login), dizia os erros em inglês e escrevia na saída padrão,
//! que é o canal do protocolo.
//!
//! A senha é digitada no navegador: este programa só vê o código de uma vez que o Spotify devolve a `127.0.0.1`.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Sender, channel};
use std::time::{Duration, Instant};

/// O Client ID público do librespot (o do aplicativo de computador do Spotify). Não é segredo: está no librespot.
const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
/// A porta do retorno, a única que esse Client ID aceita (a mesma do Spotifast).
pub const PORTA: u16 = 8898;
const CAMINHO: &str = "/login";
/// O que o login pede: a sessão (`streaming`), a conta, as playlists, a coleção, quem a pessoa segue e o estado do Connect.
const ESCOPOS: &str = "streaming user-read-private playlist-read-private playlist-read-collaborative user-library-read \
                       user-follow-read user-read-playback-state user-modify-playback-state";
const AUTORIZAR: &str = "https://accounts.spotify.com/authorize";
const TOKEN: &str = "https://accounts.spotify.com/api/token";
/// Um navegador manda a linha do pedido na hora.
const PRAZO_DO_PEDIDO: Duration = Duration::from_secs(5);
/// O maior começo de pedido que se lê.
const MAXIMO_DA_LINHA: u64 = 8 * 1024;
const PASSO: Duration = Duration::from_millis(50);

/// Uma tentativa de login.
#[derive(Debug)]
pub struct Pedido {
    pub url: String,
    /// O segredo do PKCE: só sai daqui na troca do código.
    pub verificador: String,
    pub estado: String,
}

/// O que o Spotify devolve.
#[derive(Clone, PartialEq, Eq)]
pub struct Tokens {
    pub access: String,
    pub refresh: Option<String>,
}

impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Tokens(…)")
    }
}

/// Por que o Spotify não deu os tokens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FalhaDoToken {
    /// O código do OAuth (`invalid_grant`), quando ele mandou um.
    pub codigo: Option<String>,
    pub motivo: String,
}

/// Texto sorteado pelo sistema, para o PKCE e o `state`: se desse para adivinhar, outro programa do computador poderia
/// trocar o código no lugar deste.
fn sorteio(bytes: usize) -> Result<String, String> {
    let mut sorteados = vec![0u8; bytes];
    getrandom::fill(&mut sorteados).map_err(|e| format!("o sistema não deu números sorteados ({e})"))?;
    Ok(URL_SAFE_NO_PAD.encode(sorteados))
}

/// Começa uma tentativa: o endereço para abrir no navegador, e o que confere o retorno.
pub fn comecar() -> Result<Pedido, String> {
    let (verificador, estado) = (sorteio(48)?, sorteio(18)?);
    let desafio = URL_SAFE_NO_PAD.encode(Sha256::digest(verificador.as_bytes()));
    let retorno = percent(&format!("http://127.0.0.1:{PORTA}{CAMINHO}"));
    // `show_dialog`: a pessoa vê com que conta vai entrar (e pode trocar), em vez de passar direto.
    let url = format!(
        "{AUTORIZAR}?client_id={CLIENT_ID}&response_type=code&redirect_uri={retorno}&code_challenge_method=S256\
         &code_challenge={desafio}&state={estado}&scope={}&show_dialog=true",
        percent(ESCOPOS),
    );
    Ok(Pedido { url, verificador, estado })
}

/// Abre a porta do retorno (antes do navegador: ocupada, é um erro claro).
pub fn escutar() -> std::io::Result<TcpListener> {
    TcpListener::bind((Ipv4Addr::LOCALHOST, PORTA))
}

/// O que um pedido na porta do retorno quer dizer.
#[derive(Debug, PartialEq, Eq)]
enum Retorno {
    Codigo(String),
    /// A pessoa negou, no navegador.
    Negado,
    /// Outra coisa (o ícone da página, uma aba velha, outro programa): a espera continua.
    Outro,
}

/// Espera o navegador voltar, até `prazo` ou `parar` (a pessoa cancelou). Cada conexão é lida à parte: uma ociosa, que o
/// navegador abre de reserva, não prende o login.
pub fn esperar_o_codigo(
    ouvinte: &TcpListener,
    estado: &str,
    prazo: Duration,
    parar: &AtomicBool,
) -> Result<String, String> {
    ouvinte.set_nonblocking(true).map_err(|e| format!("a porta do login não abriu ({e})"))?;
    let (avisar, retornos) = channel();
    let fim = Instant::now() + prazo;
    loop {
        if parar.load(Ordering::Acquire) {
            return Err("O login foi cancelado.".into());
        }
        if Instant::now() >= fim {
            return Err("Ninguém aprovou no navegador em 5 minutos. Tente de novo.".into());
        }
        match retornos.try_recv() {
            Ok(Retorno::Codigo(codigo)) => return Ok(codigo),
            Ok(Retorno::Negado) => return Err("O Spotify não autorizou: o acesso foi negado no navegador.".into()),
            _ => {}
        }
        match ouvinte.accept() {
            Ok((conexao, _)) => {
                let (estado, avisar) = (estado.to_string(), avisar.clone());
                let _ = std::thread::Builder::new()
                    .name("spotify-login".into())
                    .spawn(move || atender(conexao, &estado, &avisar));
            }
            Err(_) => std::thread::sleep(PASSO),
        }
    }
}

fn atender(mut conexao: TcpStream, estado: &str, avisar: &Sender<Retorno>) {
    let _ = conexao.set_nonblocking(false);
    let _ = conexao.set_read_timeout(Some(PRAZO_DO_PEDIDO));
    let _ = conexao.set_write_timeout(Some(PRAZO_DO_PEDIDO));
    let mut linha = String::new();
    if BufReader::new((&conexao).take(MAXIMO_DA_LINHA)).read_line(&mut linha).is_err() {
        return;
    }
    let retorno = ler_o_retorno(&linha, estado);
    let (status, titulo, texto) = match &retorno {
        Retorno::Codigo(_) => ("200 OK", "Pronto", "O Spotify autorizou. Pode fechar esta aba e voltar ao Player."),
        Retorno::Negado => ("200 OK", "Não entrou", "O acesso foi negado. Volte ao Player para tentar de novo."),
        Retorno::Outro => ("404 Not Found", "Nada aqui", "Esta página é só do login do Player."),
    };
    let pagina = format!(
        "<!doctype html><html lang=\"pt-BR\"><meta charset=\"utf-8\"><title>Player</title>\
         <body style=\"font-family:system-ui,sans-serif;background:#060606;color:#f2f2f2;display:grid;\
         place-items:center;height:100vh;margin:0\"><div style=\"text-align:center\">\
         <h1 style=\"font-weight:600\">{titulo}</h1><p>{texto}</p></div></body></html>"
    );
    let resposta = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-store\r\n\
         Connection: close\r\nContent-Length: {}\r\n\r\n{pagina}",
        pagina.len()
    );
    let _ = conexao.write_all(resposta.as_bytes());
    let _ = avisar.send(retorno);
}

/// Lê a linha do pedido (`GET /login?code=...&state=... HTTP/1.1`). Só vale com o `state` desta tentativa.
fn ler_o_retorno(linha: &str, estado: &str) -> Retorno {
    let mut partes = linha.split_whitespace();
    let (Some("GET"), Some(alvo)) = (partes.next(), partes.next()) else { return Retorno::Outro };
    let Some((CAMINHO, consulta)) = alvo.split_once('?') else { return Retorno::Outro };
    let campo = |nome: &str| consulta.split('&').filter_map(|p| p.split_once('=')).find(|(c, _)| *c == nome);
    if campo("state").map(|(_, v)| v) != Some(estado) {
        return Retorno::Outro;
    }
    match (campo("code"), campo("error")) {
        (Some((_, codigo)), None) if !codigo.is_empty() => Retorno::Codigo(despercent(codigo)),
        (_, Some(_)) => Retorno::Negado,
        _ => Retorno::Outro,
    }
}

/// Troca o código do navegador pelos tokens.
pub async fn trocar_o_codigo(http: &reqwest::Client, codigo: &str, verificador: &str) -> Result<Tokens, FalhaDoToken> {
    let retorno = format!("http://127.0.0.1:{PORTA}{CAMINHO}");
    let campos = [
        ("client_id", CLIENT_ID),
        ("grant_type", "authorization_code"),
        ("code", codigo),
        ("redirect_uri", retorno.as_str()),
        ("code_verifier", verificador),
    ];
    pedir(http, &campos).await
}

/// Troca o refresh token guardado por um access token novo.
pub async fn renovar(http: &reqwest::Client, refresh: &str) -> Result<Tokens, FalhaDoToken> {
    pedir(http, &[("client_id", CLIENT_ID), ("grant_type", "refresh_token"), ("refresh_token", refresh)]).await
}

async fn pedir(http: &reqwest::Client, campos: &[(&str, &str)]) -> Result<Tokens, FalhaDoToken> {
    let sem_resposta = |e: reqwest::Error| FalhaDoToken { codigo: None, motivo: e.without_url().to_string() };
    let resposta = http.post(TOKEN).form(campos).send().await.map_err(sem_resposta)?;
    let status = resposta.status().as_u16();
    let corpo = resposta.bytes().await.map_err(sem_resposta)?;
    ler_os_tokens(status, &corpo)
}

/// A resposta do Spotify. (O corpo nunca vai para um erro ou um registro: ele pode ter os tokens.)
fn ler_os_tokens(status: u16, corpo: &[u8]) -> Result<Tokens, FalhaDoToken> {
    let json: serde_json::Value = serde_json::from_slice(corpo).unwrap_or_default();
    let texto = |campo: &str| json.get(campo).and_then(|v| v.as_str()).filter(|t| !t.is_empty()).map(str::to_string);
    match texto("access_token").filter(|_| (200..300).contains(&status)) {
        Some(access) => Ok(Tokens { access, refresh: texto("refresh_token") }),
        None => {
            let codigo = texto("error");
            let dito = texto("error_description").or_else(|| codigo.clone());
            let motivo = dito.map_or_else(|| format!("resposta {status}"), |d| format!("{status}: {d}"));
            Err(FalhaDoToken { codigo, motivo })
        }
    }
}

/// Um pedaço de endereço, escapado.
fn percent(texto: &str) -> String {
    texto
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => char::from(b).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Desfaz o `%XX` de um valor (um `%` que não é código fica como está).
fn despercent(texto: &str) -> String {
    let bytes = texto.as_bytes();
    let mut saida = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        let hex = (bytes[i] == b'%')
            .then(|| bytes.get(i + 1..i + 3))
            .flatten()
            .and_then(|h| std::str::from_utf8(h).ok())
            .and_then(|h| u8::from_str_radix(h, 16).ok());
        saida.push(hex.unwrap_or(bytes[i]));
        i += if hex.is_some() { 3 } else { 1 };
    }
    String::from_utf8_lossy(&saida).into_owned()
}

/// Abre o navegador padrão. Nada de `cmd /C start`: o `&` do endereço separa comandos para o `cmd`, e o navegador receberia
/// só o pedaço de antes dele ("response_type must be code").
pub fn abrir_o_navegador(url: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    let mut comando = {
        let mut c = std::process::Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler");
        c
    };
    #[cfg(not(windows))]
    let mut comando = std::process::Command::new("xdg-open");
    // (Sem herdar a entrada e a saída: elas são o canal do protocolo com o Player.)
    let nada = std::process::Stdio::null;
    comando.arg(url).stdin(nada()).stdout(nada()).stderr(nada()).spawn().map(|_| ())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn a_tentativa_tem_pkce_e_estado_proprios() {
        let (a, b) = (comecar().expect("a"), comecar().expect("b"));
        assert!(a.verificador != b.verificador && a.estado != b.estado, "sorteados a cada tentativa");
        assert!(a.verificador.len() >= 43 && a.estado.len() >= 20, "o tamanho que o PKCE pede");
        let desafio = URL_SAFE_NO_PAD.encode(Sha256::digest(a.verificador.as_bytes()));
        assert!(a.url.contains(&format!("code_challenge={desafio}")) && a.url.contains("code_challenge_method=S256"));
        assert!(!a.url.contains(&a.verificador), "o segredo não vai no endereço");
        assert!(a.url.contains("&response_type=code&"), "{}", a.url);
        assert!(a.url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A8898%2Flogin&"), "{}", a.url);
        assert!(a.url.contains("scope=streaming%20user-read-private%20"), "{}", a.url);
        assert!(!a.url.contains([' ', '"', '\n']), "{}", a.url);
    }

    #[test]
    fn o_retorno_so_vale_com_o_estado_desta_tentativa() {
        let ler = |linha: &str| ler_o_retorno(linha, "abc");
        assert_eq!(ler("GET /login?code=x%2Fy&state=abc HTTP/1.1\r\n"), Retorno::Codigo("x/y".into()));
        assert_eq!(ler("GET /login?state=abc&error=access_denied HTTP/1.1"), Retorno::Negado);
        for outro in [
            "GET /login?code=x&state=outro HTTP/1.1",
            "GET /login?code=x HTTP/1.1",
            "GET /login?error=access_denied HTTP/1.1",
            "GET /login?code=&state=abc HTTP/1.1",
            "GET /favicon.ico HTTP/1.1",
            "GET /login HTTP/1.1",
            "POST /login?code=x&state=abc HTTP/1.1",
            "",
        ] {
            assert_eq!(ler(outro), Retorno::Outro, "{outro}");
        }
    }

    /// Uma espera numa porta qualquer.
    fn esperando(prazo: Duration) -> (u16, Arc<AtomicBool>, std::thread::JoinHandle<Result<String, String>>) {
        let ouvinte = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("porta");
        let porta = ouvinte.local_addr().expect("endereço").port();
        let parar = Arc::new(AtomicBool::new(false));
        let p = parar.clone();
        (porta, parar, std::thread::spawn(move || esperar_o_codigo(&ouvinte, "abc", prazo, &p)))
    }

    fn mandar(porta: u16, linha: &str) -> String {
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, porta)).expect("conecta");
        c.write_all(format!("{linha}\r\nHost: 127.0.0.1\r\n\r\n").as_bytes()).expect("manda");
        let mut resposta = String::new();
        let _ = c.read_to_string(&mut resposta);
        resposta
    }

    #[test]
    fn uma_conexao_ociosa_e_o_icone_da_pagina_nao_atrapalham_o_login() {
        let (porta, _, espera) = esperando(Duration::from_secs(10));
        let _ociosa = TcpStream::connect((Ipv4Addr::LOCALHOST, porta)).expect("o navegador abre uma de reserva");
        assert!(mandar(porta, "GET /favicon.ico HTTP/1.1").starts_with("HTTP/1.1 404"));
        let comecou = Instant::now();
        let pagina = mandar(porta, "GET /login?code=xyz&state=abc HTTP/1.1");
        assert!(pagina.contains("Pronto") && pagina.contains("charset=utf-8"), "{pagina}");
        assert_eq!(espera.join().expect("thread"), Ok("xyz".into()));
        assert!(comecou.elapsed() < Duration::from_secs(3), "sem esperar a ociosa");
    }

    #[test]
    fn negar_cancelar_e_o_prazo_acabam_com_a_espera() {
        let (porta, _, espera) = esperando(Duration::from_secs(10));
        // Outro programa do computador não derruba o login sem o estado certo.
        mandar(porta, "GET /login?error=access_denied HTTP/1.1");
        assert!(!espera.is_finished());
        assert!(mandar(porta, "GET /login?error=access_denied&state=abc HTTP/1.1").contains("negado"));
        assert!(espera.join().expect("thread").is_err_and(|m| m.contains("negado")));
        let (_, parar, espera) = esperando(Duration::from_secs(10));
        parar.store(true, Ordering::Release);
        assert!(espera.join().expect("thread").is_err_and(|m| m.contains("cancelado")));
        let (_, _, espera) = esperando(Duration::from_millis(150));
        assert!(espera.join().expect("thread").is_err_and(|m| m.contains("Ninguém aprovou")));
    }

    #[test]
    fn os_tokens_e_as_recusas_do_spotify() {
        let com = |status, corpo: &str| ler_os_tokens(status, corpo.as_bytes());
        let t = com(200, r#"{"access_token":"A","refresh_token":"R","expires_in":3600}"#).expect("tokens");
        assert_eq!((t.access.as_str(), t.refresh.as_deref()), ("A", Some("R")));
        // O Spotify pode não trocar o refresh token.
        assert!(com(200, r#"{"access_token":"A"}"#).is_ok_and(|t| t.refresh.is_none()));
        let revogado = com(400, r#"{"error":"invalid_grant","error_description":"Refresh token revoked"}"#);
        let falha = revogado.expect_err("recusa");
        assert_eq!(falha.codigo.as_deref(), Some("invalid_grant"));
        assert!(falha.motivo.contains("revoked"), "{}", falha.motivo);
        // Uma página de erro no meio do caminho não é uma recusa, e o corpo não vai para a mensagem.
        let falha = com(503, "<html>access_token=segredo</html>").expect_err("fora do ar");
        assert_eq!(falha.codigo, None);
        assert!(!falha.motivo.contains("segredo"), "{}", falha.motivo);
        assert!(com(400, r#"{"access_token":"A"}"#).is_err(), "só com sucesso");
        assert!(!format!("{t:?}").contains('A'), "os tokens não vão para registro");
    }

    #[test]
    fn escapar_e_desescapar() {
        assert_eq!(percent("a b&c=d/é"), "a%20b%26c%3Dd%2F%C3%A9");
        assert_eq!(despercent("a%20b%2Fc%C3%A9"), "a b/cé");
        assert_eq!(despercent("sem%ZZ%2"), "sem%ZZ%2");
    }
}
