//! O servidor local do áudio: o provedor serve, só em `127.0.0.1` e com um token sorteado a cada abertura, os bytes
//! decifrados de uma faixa. É assim que o motor do Player (que já sabe tocar por endereço, com `Range`) toca o Spotify
//! sem saber que é Spotify: para ele, é um endereço como o do YouTube Music.
//!
//! Quatro decisões que valem a pena:
//! - **Só a máquina.** A porta escuta em `127.0.0.1` (nunca `0.0.0.0`): outro computador da rede não alcança o áudio.
//! - **Token por abertura.** Sem o token, o pedido recebe `404`; ele não dá acesso a nada além do áudio desta abertura, e
//!   morre com o provedor. Não é uma credencial do Spotify, e sim o que impede um site aberto no navegador de puxar o
//!   áudio por engano (o navegador não sabe o token).
//! - **Sem `CORS`.** Um site não consegue ler a resposta, mesmo sabendo o endereço.
//! - **Um pedido por thread, com prazo.** Um cliente que abre a conexão e não pede nada não trava o servidor.
//!
//! O HTTP é escrito à mão (é um `GET` com `Range` e nada mais): não vale trazer um servidor inteiro para isto.

use super::cache::Cache;
use super::pedidos;
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Quanto esperar o pedido chegar depois da conexão.
const PRAZO_DO_PEDIDO: Duration = Duration::from_secs(10);
/// Quanto esperar o cliente aceitar os bytes antes de desistir da conexão.
const PRAZO_DA_ESCRITA: Duration = Duration::from_secs(30);
/// O maior pedido que se lê (cabeçalhos de um `GET` não passam disso).
const MAXIMO_DO_PEDIDO: usize = 16 * 1024;

/// Um servidor de áudio aberto. Soltar o valor fecha a porta e o laço.
pub struct Servidor {
    endereco: SocketAddr,
    token: String,
    parar: Arc<AtomicBool>,
}

impl Servidor {
    /// Abre uma porta sorteada em `127.0.0.1` e começa a servir o `cache`.
    pub fn abrir(cache: Arc<Cache>) -> std::io::Result<Self> {
        // A porta 0 deixa o sistema escolher uma livre (nada de brigar com outro programa).
        let ouvinte = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let endereco = ouvinte.local_addr()?;
        let token = token_novo();
        let parar = Arc::new(AtomicBool::new(false));
        let (t, p) = (token.clone(), parar.clone());
        std::thread::Builder::new().name("spotify-audio".into()).spawn(move || atender(&ouvinte, &t, &cache, &p))?;
        Ok(Self { endereco, token, parar })
    }

    /// O endereço desta faixa (`http://127.0.0.1:<porta>/a/<token>/<id>.<ext>`).
    pub fn endereco(&self, id: &str, extensao: &str) -> String {
        format!("http://127.0.0.1:{}/a/{}/{id}.{extensao}", self.endereco.port(), self.token)
    }

    /// O caminho do endereço (o que o cliente manda na linha do `GET`), para os testes.
    #[cfg(test)]
    pub fn caminho(&self, id: &str, extensao: &str) -> String {
        format!("/a/{}/{id}.{extensao}", self.token)
    }

    pub fn porta(&self) -> u16 {
        self.endereco.port()
    }
}

impl Drop for Servidor {
    fn drop(&mut self) {
        self.parar.store(true, Ordering::Release);
        // Acorda o `accept` com uma conexão que morre na hora; o laço vê o `parar` e sai.
        let _ = TcpStream::connect_timeout(&self.endereco, Duration::from_millis(200));
    }
}

/// Um token de 32 letras e números, sorteado do relógio e do contador de endereços (não precisa ser criptográfico: ele
/// só evita que outro programa ou uma página adivinhem o endereço).
fn token_novo() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let agora = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let sal = &agora as *const u128 as usize;
    let mut x = agora as u64 ^ (sal as u64).rotate_left(17) ^ ((std::process::id() as u64) << 32);
    const ALFABETO: &[u8] = b"abcdefghijklmnopqrstuvwxyz0123456789";
    (0..32)
        .map(|_| {
            // xorshift64*, que basta para um token que só vive nesta abertura.
            x ^= x >> 12;
            x ^= x << 25;
            x ^= x >> 27;
            ALFABETO[(x.wrapping_mul(0x2545_F491_4F6C_DD1D) >> 33) as usize % ALFABETO.len()] as char
        })
        .collect()
}

/// O laço do servidor: uma thread por conexão, até o `parar`.
fn atender(ouvinte: &TcpListener, token: &str, cache: &Arc<Cache>, parar: &Arc<AtomicBool>) {
    for conexao in ouvinte.incoming() {
        if parar.load(Ordering::Acquire) {
            return;
        }
        let Ok(conexao) = conexao else { continue };
        let (token, cache) = (token.to_string(), Arc::clone(cache));
        let _ = std::thread::Builder::new().name("spotify-audio-pedido".into()).spawn(move || {
            let _ = servir(conexao, &token, &cache);
        });
    }
}

/// Um pedido lido dos cabeçalhos.
#[derive(Debug, PartialEq, Eq)]
struct Pedido {
    metodo: String,
    caminho: String,
    faixa: Option<String>,
}

/// Lê a primeira linha e o `Range` dos cabeçalhos.
fn ler_pedido(leitor: &mut impl BufRead) -> std::io::Result<Pedido> {
    let mut linha = String::new();
    leitor.read_line(&mut linha)?;
    let mut partes = linha.split_whitespace();
    let metodo = partes.next().unwrap_or_default().to_string();
    let caminho = partes.next().unwrap_or_default().to_string();
    let mut faixa = None;
    let mut lidos = linha.len();
    loop {
        let mut cabecalho = String::new();
        leitor.read_line(&mut cabecalho)?;
        lidos += cabecalho.len();
        if cabecalho.trim().is_empty() || lidos > MAXIMO_DO_PEDIDO {
            break;
        }
        if let Some((nome, valor)) = cabecalho.split_once(':') {
            let (nome, valor) = (nome.trim(), valor.trim());
            if nome.eq_ignore_ascii_case("range") {
                faixa = Some(valor.to_string());
            }
        }
    }
    Ok(Pedido { metodo, caminho, faixa })
}

/// Responde um pedido: o áudio (com `Range`), `404` para um caminho sem o token, `405` para outro método.
fn servir(mut conexao: TcpStream, token: &str, cache: &Cache) -> std::io::Result<()> {
    conexao.set_read_timeout(Some(PRAZO_DO_PEDIDO))?;
    conexao.set_write_timeout(Some(PRAZO_DA_ESCRITA))?;
    let mut leitor = BufReader::new(conexao.try_clone()?);
    let pedido = ler_pedido(&mut leitor)?;
    if pedido.metodo != "GET" && pedido.metodo != "HEAD" {
        return responder(&mut conexao, "405 Method Not Allowed", "text/plain", b"so GET");
    }
    let Some(id) = id_do_caminho(&pedido.caminho, token) else {
        return responder(&mut conexao, "404 Not Found", "text/plain", b"nao encontrado");
    };
    let tamanho = match cache.tamanho(&id) {
        Ok(t) => t,
        Err(falha) => {
            log::warn!("áudio: a faixa {id} não abriu para o pedido HTTP: {}", falha.mensagem);
            let texto = falha.mensagem.as_bytes().to_vec();
            return responder(&mut conexao, "503 Service Unavailable", "text/plain", &texto);
        }
    };
    let faixa = pedidos::faixa(pedido.faixa.as_deref(), tamanho);
    let tipo = match pedidos::extensao_do_caminho(&pedido.caminho).as_deref() {
        Some("mp3") => "audio/mpeg",
        _ => "audio/ogg",
    };
    if pedido.metodo == "HEAD" {
        let cabeca = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {tipo}\r\nContent-Length: {tamanho}\r\nAccept-Ranges: bytes\r\n\r\n"
        );
        conexao.write_all(cabeca.as_bytes())?;
        return Ok(());
    }
    let body = match cache.ler(&id, faixa) {
        Ok(b) => b,
        Err(falha) => {
            let texto = falha.mensagem.as_bytes().to_vec();
            return responder(&mut conexao, "503 Service Unavailable", "text/plain", &texto);
        }
    };
    if faixa.parcial {
        let cabeca = format!(
            "HTTP/1.1 206 Partial Content\r\nContent-Type: {tipo}\r\nContent-Range: bytes {}-{}/{tamanho}\r\n\
             Content-Length: {}\r\nAccept-Ranges: bytes\r\n\r\n",
            faixa.de,
            faixa.ate().saturating_sub(1),
            body.len()
        );
        conexao.write_all(cabeca.as_bytes())?;
        conexao.write_all(&body)?;
        Ok(())
    } else {
        let cabeca = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: {tipo}\r\nContent-Length: {}\r\nAccept-Ranges: bytes\r\n\r\n",
            body.len()
        );
        conexao.write_all(cabeca.as_bytes())?;
        conexao.write_all(&body)?;
        Ok(())
    }
}

/// O id da faixa no caminho (`/a/<token>/<id>.<ext>`), se o token confere.
fn id_do_caminho(caminho: &str, token: &str) -> Option<String> {
    let partes = pedidos::partes_do_caminho(caminho);
    let (raiz, do_pedido, arquivo) = (*partes.first()?, *partes.get(1)?, *partes.get(2)?);
    if raiz != "a" || do_pedido != token {
        return None;
    }
    // O nome do arquivo é `<id>.<ext>`; o id é o que vem antes do último ponto.
    let id = arquivo.rsplit_once('.').map_or(arquivo, |(id, _)| id);
    let vale = !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric());
    vale.then(|| id.to_string())
}

fn responder(conexao: &mut TcpStream, status: &str, tipo: &str, corpo: &[u8]) -> std::io::Result<()> {
    let cabeca = format!("HTTP/1.1 {status}\r\nContent-Type: {tipo}\r\nContent-Length: {}\r\n\r\n", corpo.len());
    conexao.write_all(cabeca.as_bytes())?;
    conexao.write_all(corpo)?;
    Ok(())
}

/// Um `Cache` só para os testes do servidor não precisa de rede: ele responde os bytes na hora.
#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) mod falso {
    use super::super::cache::Baixador;
    use super::super::librespot::FalhaDeAudio;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub(crate) struct Bytes(pub Arc<Vec<u8>>, pub AtomicUsize);

    impl Baixador for Bytes {
        fn tamanho(&self, _: &str) -> Result<u64, FalhaDeAudio> {
            Ok(self.0.len() as u64)
        }
        fn pedaco(&self, _: &str, de: u64, quantos: u64) -> Result<Vec<u8>, FalhaDeAudio> {
            self.1.fetch_add(1, Ordering::SeqCst);
            let de = de as usize;
            let ate = (de + quantos as usize).min(self.0.len());
            Ok(self.0[de..ate].to_vec())
        }
        fn formato(&self, _: &str) -> Result<String, FalhaDeAudio> {
            Ok("ogg".into())
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use std::io::Read as _;

    fn dados(n: usize) -> Arc<Vec<u8>> {
        Arc::new((0..n).map(|i| (i % 251) as u8).collect())
    }

    fn servidor(n: usize) -> (Servidor, Arc<Vec<u8>>) {
        let bytes = dados(n);
        let falso = falso::Bytes(Arc::clone(&bytes), std::sync::atomic::AtomicUsize::new(0));
        let cache = Arc::new(Cache::novo(Arc::new(falso)));
        (Servidor::abrir(cache).expect("abre"), bytes)
    }

    /// Um `GET` cru e a resposta inteira (cabeçalhos e corpo). O corpo é lido pelo `Content-Length`, porque o áudio é
    /// binário e pode conter a sequência `\r\n\r\n` por acaso.
    fn get(porta: u16, caminho: &str, range: Option<&str>) -> (String, Vec<u8>) {
        let mut s = TcpStream::connect((Ipv4Addr::LOCALHOST, porta)).expect("conecta");
        let range = range.map(|r| format!("Range: {r}\r\n")).unwrap_or_default();
        let pedido = format!("GET {caminho} HTTP/1.1\r\nHost: localhost\r\n{range}\r\n");
        s.write_all(pedido.as_bytes()).expect("escreve");
        let mut tudo = Vec::new();
        s.read_to_end(&mut tudo).expect("lê");
        let corte = tudo.windows(4).position(|w| w == b"\r\n\r\n").unwrap_or(tudo.len());
        let cabeca = String::from_utf8_lossy(&tudo[..corte]).to_string();
        let tamanho = cabeca
            .lines()
            .find_map(|l| l.strip_prefix("Content-Length: "))
            .and_then(|v| v.trim().parse::<usize>().ok())
            .unwrap_or(tudo.len() - corte - 4);
        let inicio = corte + 4;
        let corpo = tudo[inicio..(inicio + tamanho).min(tudo.len())].to_vec();
        (cabeca, corpo)
    }

    #[test]
    fn serve_a_faixa_inteira_e_uma_parte_dela() {
        let (s, bytes) = servidor(200_000);
        let caminho = s.caminho("abc123", "ogg");
        let (cabeca, corpo) = get(s.porta(), &caminho, None);
        assert!(cabeca.starts_with("HTTP/1.1 200 OK"), "{cabeca}");
        assert!(cabeca.contains("Accept-Ranges: bytes"), "{cabeca}");
        assert_eq!(corpo, *bytes);
        let (cabeca, corpo) = get(s.porta(), &caminho, Some("bytes=1000-1999"));
        assert!(cabeca.starts_with("HTTP/1.1 206 Partial Content"), "{cabeca}");
        assert!(cabeca.contains("Content-Range: bytes 1000-1999/200000"), "{cabeca}");
        assert_eq!(corpo, bytes[1000..2000].to_vec());
    }

    #[test]
    fn sem_o_token_nao_sai_audio() {
        let (s, _) = servidor(1000);
        let (cabeca, corpo) = get(s.porta(), "/a/errado/abc.ogg", None);
        assert!(cabeca.starts_with("HTTP/1.1 404"), "{cabeca}");
        assert_eq!(String::from_utf8_lossy(&corpo), "nao encontrado");
        let (cabeca, _) = get(s.porta(), "/abc.ogg", None);
        assert!(cabeca.starts_with("HTTP/1.1 404"), "{cabeca}");
    }

    #[test]
    fn um_metodo_que_nao_e_get_recebe_405() {
        let (s, _) = servidor(100);
        let mut c = TcpStream::connect((Ipv4Addr::LOCALHOST, s.porta())).expect("conecta");
        c.write_all(b"POST /a/x/y.ogg HTTP/1.1\r\n\r\n").expect("escreve");
        let mut resposta = String::new();
        c.read_to_string(&mut resposta).expect("lê");
        assert!(resposta.starts_with("HTTP/1.1 405"), "{resposta}");
    }

    #[test]
    fn o_token_so_vale_nesta_abertura() {
        let (a, _) = servidor(100);
        let (b, _) = servidor(100);
        assert_ne!(a.token, b.token);
        assert_eq!(a.token.len(), 32);
    }

    #[test]
    fn o_id_do_caminho() {
        assert_eq!(id_do_caminho("/a/t1/abc123.ogg", "t1"), Some("abc123".into()));
        assert_eq!(id_do_caminho("/a/t1/abc123", "t1"), Some("abc123".into()));
        assert_eq!(id_do_caminho("/a/t1/abc123.ogg", "t2"), None);
        assert_eq!(id_do_caminho("/b/t1/abc.ogg", "t1"), None);
        assert_eq!(id_do_caminho("/a/t1/../x.ogg", "t1"), None, "nada de caminho relativo");
        assert_eq!(id_do_caminho("/a/t1/.ogg", "t1"), None);
    }
}
