//! Tocar por endereço: a música vem da internet (o endereço que um provedor entregou), aos pedaços.
//!
//! Uma thread baixa a música em pedaços de [`PEDACO`] bytes, em sequência, desde que o leitor é aberto; o
//! decodificador lê dos pedaços já baixados e, se pula para um ponto que ainda não chegou, a thread recomeça o
//! download dali (com `Range`, quando o servidor aceita). Os pedaços ficam na memória enquanto a música está
//! aberta, então voltar é instantâneo. Uma queda de conexão no meio é retomada do pedaço que faltava.

use std::collections::HashMap;
use std::io::{self, Read, Seek, SeekFrom};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// O tamanho de cada pedaço baixado.
pub const PEDACO: u64 = 64 * 1024;
/// Quanto o decodificador espera um pedaço antes de desistir (sem internet, por exemplo).
const ESPERA_MAXIMA: Duration = Duration::from_secs(30);
/// Quantas vezes seguidas a thread tenta retomar uma conexão que caiu.
const TENTATIVAS: u32 = 3;
/// Um pedido para mais longe que isto à frente do download recomeça o download no ponto pedido.
const SALTO: u64 = 8;

/// Um endereço de áudio, com os cabeçalhos que o serviço exige e, se o provedor disse, o formato.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Endereco {
    pub url: String,
    pub cabecalhos: Vec<(String, String)>,
    pub formato: Option<String>,
}

impl Endereco {
    pub fn novo(url: impl Into<String>) -> Self {
        Self { url: url.into(), cabecalhos: Vec::new(), formato: None }
    }

    /// A extensão do formato: a que o provedor deu ou a do fim do endereço (`.../faixa.mp3?x=1` → `mp3`).
    pub fn extensao(&self) -> Option<String> {
        if let Some(f) = self.formato.as_deref().filter(|f| !f.is_empty()) {
            return Some(f.trim_start_matches('.').to_ascii_lowercase());
        }
        let caminho = self.url.split(['?', '#']).next().unwrap_or("");
        let ultimo = caminho.rsplit('/').next().unwrap_or("");
        let (_, ext) = ultimo.rsplit_once('.')?;
        (!ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()))
            .then(|| ext.to_ascii_lowercase())
    }

    /// O endereço sem a parte depois do `?` (que costuma levar assinaturas e chaves): é o que vai para o registro.
    pub fn para_registro(&self) -> &str {
        self.url.split('?').next().unwrap_or(&self.url)
    }
}

#[derive(Default)]
struct Estado {
    /// O tamanho total, quando o servidor diz (ou quando o download chega ao fim).
    tamanho: Option<u64>,
    blocos: HashMap<u64, Vec<u8>>,
    /// O pedaço que o decodificador está esperando.
    quero: Option<u64>,
    erro: Option<String>,
    parar: bool,
    aceita_faixas: bool,
}

type Compartilhado = Arc<(Mutex<Estado>, Condvar)>;

fn travar(m: &Mutex<Estado>) -> MutexGuard<'_, Estado> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Lê uma música da internet como se fosse um arquivo (lê e pula para qualquer ponto).
pub struct LeitorHttp {
    comp: Compartilhado,
    pos: u64,
}

fn agente() -> io::Result<ureq::Agent> {
    let tls = native_tls::TlsConnector::new().map_err(io::Error::other)?;
    Ok(ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(20))
        .tls_connector(Arc::new(tls))
        .build())
}

/// Uma resposta aberta: o corpo, de que byte ele começa e o que o servidor disse do tamanho e das faixas.
struct Aberta {
    corpo: Box<dyn Read + Send + Sync>,
    comeca_em: u64,
    tamanho: Option<u64>,
    aceita_faixas: bool,
}

fn pedir(agente: &ureq::Agent, e: &Endereco, de: u64) -> io::Result<Aberta> {
    let mut pedido = agente.get(&e.url).set("Range", &format!("bytes={de}-"));
    for (nome, valor) in &e.cabecalhos {
        pedido = pedido.set(nome, valor);
    }
    let resposta = match pedido.call() {
        Ok(r) => r,
        Err(ureq::Error::Status(codigo, _)) => {
            return Err(io::Error::other(format!("o servidor respondeu {codigo} para {}", e.para_registro())));
        }
        Err(erro) => return Err(io::Error::other(erro.to_string())),
    };
    if resposta.status() == 206 {
        // Content-Range: bytes 100-999/1000
        let total = resposta
            .header("Content-Range")
            .and_then(|v| v.rsplit('/').next())
            .and_then(|t| t.trim().parse::<u64>().ok());
        let comeca = resposta
            .header("Content-Range")
            .and_then(|v| v.trim().strip_prefix("bytes "))
            .and_then(|v| v.split('-').next())
            .and_then(|v| v.trim().parse::<u64>().ok())
            .unwrap_or(de);
        Ok(Aberta { corpo: Box::new(resposta.into_reader()), comeca_em: comeca, tamanho: total, aceita_faixas: true })
    } else {
        // 200: o servidor ignorou o pedido de faixa e manda tudo desde o começo.
        let aceita = resposta.header("Accept-Ranges").is_some_and(|v| v.eq_ignore_ascii_case("bytes"));
        let total = resposta.header("Content-Length").and_then(|v| v.trim().parse::<u64>().ok());
        Ok(Aberta { corpo: Box::new(resposta.into_reader()), comeca_em: 0, tamanho: total, aceita_faixas: aceita })
    }
}

/// Baixa de uma vez um arquivo pequeno (a capa de uma faixa), até `limite` bytes.
pub fn baixar_pequeno(url: &str, limite: u64) -> io::Result<Vec<u8>> {
    let resposta = match agente()?.get(url).call() {
        Ok(r) => r,
        Err(e) => return Err(io::Error::other(e.to_string())),
    };
    let mut v = Vec::new();
    resposta.into_reader().take(limite).read_to_end(&mut v)?;
    Ok(v)
}

impl LeitorHttp {
    /// Abre o endereço (a primeira resposta chega antes de voltar) e começa a baixar em segundo plano.
    pub fn abrir(e: &Endereco) -> io::Result<Self> {
        let agente = agente()?;
        let aberta = pedir(&agente, e, 0)?;
        let comp: Compartilhado = Arc::new((
            Mutex::new(Estado { tamanho: aberta.tamanho, aceita_faixas: aberta.aceita_faixas, ..Default::default() }),
            Condvar::new(),
        ));
        let (c, e) = (comp.clone(), e.clone());
        std::thread::Builder::new().name("tocador-rede".into()).spawn(move || baixar(&agente, &e, aberta, &c))?;
        Ok(Self { comp, pos: 0 })
    }

    /// O tamanho total em bytes, se já se sabe.
    pub fn tamanho(&self) -> Option<u64> {
        travar(&self.comp.0).tamanho
    }
}

/// O primeiro pedaço que falta a partir de `de` (até o fim, quando o tamanho é conhecido).
fn proximo_que_falta(st: &Estado, de: u64) -> Option<u64> {
    let total = st.tamanho?.div_ceil(PEDACO);
    (de..total).find(|i| !st.blocos.contains_key(i))
}

/// A thread que baixa: enche um pedaço por vez e decide de onde continuar.
fn baixar(agente: &ureq::Agent, e: &Endereco, primeira: Aberta, comp: &Compartilhado) {
    let (m, cv) = &**comp;
    let mut corpo = primeira.corpo;
    let mut bloco = primeira.comeca_em / PEDACO;
    let mut falhas = 0u32;
    let mut ultimo_erro: Option<String> = None;
    loop {
        let mut buf = Vec::with_capacity(PEDACO as usize);
        let lido = (&mut corpo).take(PEDACO).read_to_end(&mut buf);
        let mut st = travar(m);
        if st.parar {
            return;
        }
        let fim_da_resposta = lido.is_err() || (buf.len() as u64) < PEDACO;
        let chegou_ao_fim = match st.tamanho {
            Some(t) => bloco * PEDACO + buf.len() as u64 >= t,
            None => lido.is_ok() && (buf.len() as u64) < PEDACO,
        };
        // Um pedaço curto só vale quando é o último; no meio, a conexão caiu e ele é baixado de novo.
        let pedaco_bom = !buf.is_empty() && (!fim_da_resposta || chegou_ao_fim);
        if pedaco_bom {
            st.blocos.entry(bloco).or_insert(buf);
            falhas = 0;
        }
        if chegou_ao_fim && st.tamanho.is_none() {
            st.tamanho = Some(bloco * PEDACO + st.blocos.get(&bloco).map_or(0, |b| b.len() as u64));
        }
        cv.notify_all();

        // De onde continuar: o pedaço que o decodificador espera (se está longe), o próximo que falta ou, sem
        // nada faltando, esperar um pedido.
        // Sem faixas (o servidor só manda do começo), não dá para pular: o download segue em sequência.
        let aceita = st.aceita_faixas;
        let quero = st.quero.filter(|q| !st.blocos.contains_key(q));
        let seguinte = if pedaco_bom && !fim_da_resposta { bloco + 1 } else { bloco };
        let recomecar = if aceita && !fim_da_resposta && quero.is_some_and(|q| q < seguinte || q > seguinte + SALTO) {
            quero
        } else if fim_da_resposta || (aceita && st.blocos.contains_key(&seguinte)) {
            if fim_da_resposta && !pedaco_bom && !chegou_ao_fim {
                falhas += 1;
                if let Err(err) = &lido {
                    ultimo_erro = Some(format!("a conexão caiu: {err}"));
                }
                if falhas > TENTATIVAS {
                    st.erro = Some(ultimo_erro.take().unwrap_or_else(|| "o servidor parou de mandar a música".into()));
                    cv.notify_all();
                    return;
                }
            }
            match quero.or_else(|| proximo_que_falta(&st, seguinte)).or_else(|| proximo_que_falta(&st, 0)) {
                Some(q) => Some(q),
                None if st.tamanho.is_some() => {
                    // Tudo baixado: espera um pedido (que só vem se algo sumir) ou o fim.
                    loop {
                        if st.parar {
                            return;
                        }
                        if let Some(q) = st.quero.filter(|q| !st.blocos.contains_key(q)) {
                            break Some(q);
                        }
                        st = cv.wait_timeout(st, Duration::from_secs(1)).unwrap_or_else(PoisonError::into_inner).0;
                    }
                }
                None => None,
            }
        } else {
            None
        };
        match recomecar {
            None => bloco = seguinte,
            Some(q) => {
                drop(st);
                // Sem faixas, só dá para recomeçar do começo (os pedaços já baixados ficam).
                let de = if aceita { q * PEDACO } else { 0 };
                match pedir(agente, e, de) {
                    Ok(nova) => {
                        if nova.comeca_em != de {
                            travar(m).aceita_faixas = false;
                        }
                        bloco = nova.comeca_em / PEDACO;
                        corpo = nova.corpo;
                    }
                    Err(err) => {
                        // Tenta de novo no próximo giro, do mesmo ponto (a resposta vazia conta como uma falha).
                        ultimo_erro = Some(err.to_string());
                        std::thread::sleep(Duration::from_millis(300 * u64::from(falhas + 1)));
                        corpo = Box::new(io::empty());
                        bloco = if aceita { q } else { 0 };
                    }
                }
            }
        }
    }
}

impl Read for LeitorHttp {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let (m, cv) = &*self.comp;
        let mut st = travar(m);
        let comeco = Instant::now();
        loop {
            if st.tamanho.is_some_and(|t| self.pos >= t) {
                return Ok(0);
            }
            let indice = self.pos / PEDACO;
            let dentro = (self.pos % PEDACO) as usize;
            if let Some(b) = st.blocos.get(&indice) {
                if dentro >= b.len() {
                    return Ok(0);
                }
                let n = (b.len() - dentro).min(buf.len());
                buf[..n].copy_from_slice(&b[dentro..dentro + n]);
                self.pos += n as u64;
                if st.quero == Some(indice) {
                    st.quero = None;
                }
                return Ok(n);
            }
            if let Some(e) = &st.erro {
                return Err(io::Error::other(e.clone()));
            }
            if comeco.elapsed() > ESPERA_MAXIMA {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "a música parou de chegar"));
            }
            st.quero = Some(indice);
            cv.notify_all();
            st = cv.wait_timeout(st, Duration::from_millis(500)).unwrap_or_else(PoisonError::into_inner).0;
        }
    }
}

impl Seek for LeitorHttp {
    fn seek(&mut self, de: SeekFrom) -> io::Result<u64> {
        let (m, cv) = &*self.comp;
        let mut st = travar(m);
        let novo = match de {
            SeekFrom::Start(n) => i128::from(n),
            SeekFrom::Current(d) => i128::from(self.pos) + i128::from(d),
            SeekFrom::End(d) => match st.tamanho {
                Some(t) => i128::from(t) + i128::from(d),
                None => return Err(io::Error::new(io::ErrorKind::Unsupported, "o tamanho ainda não é conhecido")),
            },
        };
        let novo = u64::try_from(novo).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "antes do começo"))?;
        self.pos = novo;
        // Já avisa a thread: o download vai para lá enquanto o decodificador se prepara.
        let indice = novo / PEDACO;
        if !st.blocos.contains_key(&indice) && st.tamanho.is_none_or(|t| novo < t) {
            st.quero = Some(indice);
            cv.notify_all();
        }
        Ok(novo)
    }
}

impl symphonia::core::io::MediaSource for LeitorHttp {
    fn is_seekable(&self) -> bool {
        self.tamanho().is_some()
    }

    fn byte_len(&self) -> Option<u64> {
        self.tamanho()
    }
}

impl Drop for LeitorHttp {
    fn drop(&mut self) {
        travar(&self.comp.0).parar = true;
        self.comp.1.notify_all();
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
pub(crate) mod testes {
    use super::*;
    use std::io::{BufRead, BufReader, Write};
    use std::net::TcpListener;

    /// Um servidor HTTP mínimo, numa thread, que serve `dados` em `/musica` (com faixas, se `faixas`) e 404 no
    /// resto. Devolve o endereço base.
    pub(crate) fn servidor(dados: Vec<u8>, faixas: bool) -> String {
        let ouvinte = TcpListener::bind("127.0.0.1:0").expect("porta");
        let base = format!("http://{}", ouvinte.local_addr().expect("endereço"));
        let dados = Arc::new(dados);
        std::thread::spawn(move || {
            for conexao in ouvinte.incoming().flatten() {
                let dados = dados.clone();
                std::thread::spawn(move || {
                    let mut leitor = BufReader::new(conexao.try_clone().expect("clone"));
                    let mut primeira = String::new();
                    if leitor.read_line(&mut primeira).is_err() {
                        return;
                    }
                    let mut de = 0u64;
                    loop {
                        let mut linha = String::new();
                        if leitor.read_line(&mut linha).unwrap_or(0) == 0 || linha.trim().is_empty() {
                            break;
                        }
                        if let Some(v) = linha.to_ascii_lowercase().strip_prefix("range: bytes=") {
                            de = v.trim().trim_end_matches('-').parse().unwrap_or(0);
                        }
                    }
                    let mut conexao = conexao;
                    if !primeira.contains("/musica") {
                        let _ = conexao.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
                        return;
                    }
                    let total = dados.len() as u64;
                    let de = if faixas { de.min(total) } else { 0 };
                    let cabeca = if faixas {
                        format!(
                            "HTTP/1.1 206 Partial Content\r\nContent-Range: bytes {de}-{}/{total}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            total.saturating_sub(1),
                            total - de
                        )
                    } else {
                        format!("HTTP/1.1 200 OK\r\nContent-Length: {total}\r\nConnection: close\r\n\r\n")
                    };
                    let _ = conexao.write_all(cabeca.as_bytes());
                    let _ = conexao.write_all(&dados[de as usize..]);
                });
            }
        });
        base
    }

    fn dados(n: usize) -> Vec<u8> {
        (0..n).map(|i| (i * 31 % 251) as u8).collect()
    }

    #[test]
    fn le_tudo_e_pula_para_qualquer_ponto() {
        for faixas in [true, false] {
            let original = dados(300_001);
            let base = servidor(original.clone(), faixas);
            let mut l = LeitorHttp::abrir(&Endereco::novo(format!("{base}/musica"))).expect("abre");
            assert_eq!(l.tamanho(), Some(300_001), "faixas={faixas}");
            // Pula para perto do fim (o download ainda não chegou lá) e lê.
            l.seek(SeekFrom::Start(280_000)).expect("pula");
            let mut fim = Vec::new();
            l.read_to_end(&mut fim).expect("fim");
            assert_eq!(fim, original[280_000..], "faixas={faixas}");
            // Volta ao começo e lê tudo.
            l.seek(SeekFrom::Start(0)).expect("volta");
            let mut tudo = Vec::new();
            l.read_to_end(&mut tudo).expect("tudo");
            assert_eq!(tudo.len(), original.len());
            assert!(tudo == original, "faixas={faixas}: o conteúdo bate");
            assert_eq!(l.seek(SeekFrom::End(-1)).expect("do fim"), 300_000);
            let mut um = [0u8; 4];
            assert_eq!(l.read(&mut um).expect("último"), 1);
            assert_eq!(um[0], original[300_000]);
            assert!(l.seek(SeekFrom::Current(-400_000)).is_err(), "antes do começo");
        }
    }

    #[test]
    fn endereco_que_nao_existe_da_erro() {
        let base = servidor(dados(10), true);
        let erro = LeitorHttp::abrir(&Endereco::novo(format!("{base}/outra"))).err().expect("404");
        assert!(erro.to_string().contains("404"), "{erro}");
        assert!(LeitorHttp::abrir(&Endereco::novo("http://127.0.0.1:9/nada")).is_err(), "ninguém escutando");
    }

    /// Um WAV de `segundos` com um tom, 16 bits, mono, na taxa `taxa`.
    fn wav(taxa: u32, segundos: f32) -> Vec<u8> {
        let n = (taxa as f32 * segundos) as u32;
        let mut v = Vec::new();
        v.extend_from_slice(b"RIFF");
        v.extend_from_slice(&(36 + n * 2).to_le_bytes());
        v.extend_from_slice(b"WAVEfmt ");
        v.extend_from_slice(&16u32.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&taxa.to_le_bytes());
        v.extend_from_slice(&(taxa * 2).to_le_bytes());
        v.extend_from_slice(&2u16.to_le_bytes());
        v.extend_from_slice(&16u16.to_le_bytes());
        v.extend_from_slice(b"data");
        v.extend_from_slice(&(n * 2).to_le_bytes());
        for i in 0..n {
            let x = (i as f32 * 440.0 * std::f32::consts::TAU / taxa as f32).sin() * 12_000.0;
            v.extend_from_slice(&(x as i16).to_le_bytes());
        }
        v
    }

    #[test]
    fn toca_um_wav_pela_rede() {
        let base = servidor(wav(8000, 3.0), true);
        let mut e = Endereco::novo(format!("{base}/musica"));
        e.cabecalhos = vec![("User-Agent".into(), "teste".into())];
        let mut fonte = crate::fonte::Fonte::abrir_endereco(&e, 8000).expect("abre e reconhece o WAV");
        assert!((fonte.duracao_s() - 3.0).abs() < 0.01, "{}", fonte.duracao_s());
        // Pula para a metade e lê até o fim: sobra metade do som.
        let onde = fonte.ir(1.5);
        assert!((onde - 1.5).abs() < 0.1, "{onde}");
        let mut som = Vec::new();
        while fonte.proximo(&mut som) {}
        let quadros = som.len() / 2;
        assert!((11_000..=13_000).contains(&quadros), "{quadros} quadros depois de 1,5 s");
        assert!(som.iter().any(|v| v.abs() > 0.2), "tem som de verdade");
        let origem = crate::Origem::Endereco(Endereco::novo(format!("{base}/musica?token=segredo")));
        assert!(!origem.to_string().contains("segredo"), "o registro não leva a chave");
        assert!(crate::fonte::Fonte::abrir_origem(&origem, 8000).is_ok());
    }

    #[test]
    fn extensao_e_registro() {
        let mut e = Endereco::novo("https://x.com/a/faixa.MP3?sig=segredo&k=1");
        assert_eq!(e.extensao().as_deref(), Some("mp3"));
        assert_eq!(e.para_registro(), "https://x.com/a/faixa.MP3");
        e.formato = Some(".m4a".into());
        assert_eq!(e.extensao().as_deref(), Some("m4a"));
        assert_eq!(Endereco::novo("https://x.com/videoplayback?id=1").extensao(), None);
        assert_eq!(Endereco::novo("https://x.com/").extensao(), None);
    }
}
