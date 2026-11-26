//! O Pathfinder: o endereço que os apps do Spotify consultam (a busca, o início, as páginas, a biblioteca), com o
//! portador do Login5 e o client token da sessão do librespot, como o Spotifast faz. Só leituras.

use crate::codigos::{Codigos, codigo_no_texto, pacote_principal, pedacos_da_rota, rota_da_operacao};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Quanto uma consulta pode demorar.
const PRAZO: Duration = Duration::from_secs(8);
/// Depois de procurar um código no site e não achar, quanto esperar para procurar de novo.
const PROCURAR_DE_NOVO: Duration = Duration::from_secs(10 * 60);
/// A maior espera que um "pedidos demais" pode impor.
const MAIOR_ESPERA_S: u64 = 3600;
/// Quanto um arquivo do site pode demorar (a procura do código segue em segundo plano, sem prender o pedido).
const PRAZO_DO_SITE: Duration = Duration::from_secs(60);
/// O maior arquivo do site que se lê (o pacote principal tem uns 5 MB).
const MAIOR_PACOTE: usize = 16 * 1024 * 1024;

/// O que a sessão dá para cada pedido.
#[derive(Clone, Debug)]
pub struct Acesso {
    pub portador: String,
    pub cliente: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Falha {
    /// O portador venceu ou a sessão caiu: reentrar e tentar de novo.
    Sessao,
    /// O Spotify pediu para esperar (segundos).
    Limite(u64),
    /// O Spotify trocou o código da consulta, e o novo não foi achado (ou ainda está sendo procurado).
    Codigo,
    Rede(String),
    /// O prazo do pedido inteiro acabou antes desta resposta.
    Prazo,
    /// Uma resposta que não dá para usar.
    Resposta(String),
}

impl Falha {
    pub fn frase(&self) -> String {
        match self {
            Self::Sessao => "A sessão do Spotify caiu. Tente de novo.".into(),
            Self::Limite(s) => format!("O Spotify pediu para esperar {s} s."),
            Self::Codigo => {
                "O Spotify mudou este pedido, e o Player ainda não achou o novo. Tente de novo mais tarde.".into()
            }
            Self::Rede(m) => format!("Sem resposta do Spotify ({m})."),
            Self::Prazo => "Sem resposta do Spotify (demorou demais).".into(),
            Self::Resposta(m) => format!("O Spotify respondeu de um jeito inesperado ({m})."),
        }
    }
}

/// Onde ficam a consulta, a página do site e os pacotes dele (os testes trocam por um servidor local).
#[derive(Clone, Debug)]
pub struct Enderecos {
    pub consulta: String,
    pub site: String,
    pub pacotes: String,
}

impl Default for Enderecos {
    fn default() -> Self {
        Self {
            consulta: "https://api-partner.spotify.com/pathfinder/v2/query".into(),
            site: "https://open.spotify.com/".into(),
            pacotes: "https://open.spotifycdn.com/cdn/build/web-player/".into(),
        }
    }
}

pub struct Pathfinder {
    /// Para as consultas: sem seguir redirecionamento (o portador não vai a outro endereço).
    http: reqwest::Client,
    /// Para os arquivos públicos do site.
    site: reqwest::Client,
    enderecos: Enderecos,
    codigos: Codigos,
    /// Até quando o Spotify pediu para esperar.
    esperar_ate: Mutex<Option<Instant>>,
    /// Quando o código de cada operação foi procurado, com o site respondendo, e não apareceu.
    procurou_em_vao: Mutex<HashMap<String, Instant>>,
    /// As operações cujo código está sendo procurado agora.
    procurando: Mutex<HashSet<String>>,
}

/// O corpo de uma consulta.
fn corpo(op: &str, codigo: &str, variaveis: &Value) -> Value {
    json!({
        "variables": variaveis,
        "operationName": op,
        "extensions": {"persistedQuery": {"version": 1, "sha256Hash": codigo}},
    })
}

impl Pathfinder {
    pub fn novo(enderecos: Enderecos, codigos: Codigos) -> Result<Self, String> {
        let construtor = || reqwest::Client::builder();
        let http = construtor().timeout(PRAZO).redirect(reqwest::redirect::Policy::none());
        Ok(Self {
            http: http.build().map_err(|e| e.to_string())?,
            site: construtor().timeout(PRAZO_DO_SITE).build().map_err(|e| e.to_string())?,
            enderecos,
            codigos,
            esperar_ate: Mutex::new(None),
            procurou_em_vao: Mutex::default(),
            procurando: Mutex::default(),
        })
    }

    /// Faz uma consulta e devolve o `data` da resposta. Se o Spotify não conhece o código, tenta o embutido e depois
    /// procura o novo no site.
    pub async fn consultar(self: &Arc<Self>, acesso: &Acesso, op: &str, variaveis: &Value) -> Result<Value, Falha> {
        let candidatos = self.codigos.de(op);
        if candidatos.is_empty() {
            return Err(Falha::Resposta(format!("a operação {op} não é deste provedor")));
        }
        for (i, codigo) in candidatos.iter().enumerate() {
            match self.pedir(acesso, op, codigo, variaveis).await {
                Err(Falha::Codigo) => {}
                resposta => {
                    if i > 0 && resposta.is_ok() {
                        self.codigos.esquecer(op);
                    }
                    return resposta;
                }
            }
        }
        match self.novo_codigo(op).await.filter(|c| !candidatos.contains(c)) {
            Some(novo) => self.pedir(acesso, op, &novo, variaveis).await,
            None => Err(Falha::Codigo),
        }
    }

    /// Procura o código em segundo plano (quem espera pode desistir: a procura continua e fica guardada).
    async fn novo_codigo(self: &Arc<Self>, op: &str) -> Option<String> {
        let em_vao = self.procurou_em_vao.lock().unwrap_or_else(PoisonError::into_inner).get(op).copied();
        if em_vao.is_some_and(|quando| quando.elapsed() < PROCURAR_DE_NOVO) {
            return None;
        }
        if !self.procurando.lock().unwrap_or_else(PoisonError::into_inner).insert(op.to_string()) {
            return None;
        }
        let (eu, op) = (self.clone(), op.to_string());
        tokio::spawn(async move {
            let achado = eu.procurar_codigo(&op).await;
            eu.procurando.lock().unwrap_or_else(PoisonError::into_inner).remove(&op);
            achado
        })
        .await
        .ok()
        .flatten()
    }

    /// Quantos segundos faltam da espera que o Spotify pediu.
    fn falta_esperar(&self) -> Option<u64> {
        let ate = (*self.esperar_ate.lock().unwrap_or_else(PoisonError::into_inner))?;
        let falta = ate.saturating_duration_since(Instant::now());
        (!falta.is_zero()).then(|| falta.as_secs().max(1))
    }

    async fn pedir(&self, acesso: &Acesso, op: &str, codigo: &str, variaveis: &Value) -> Result<Value, Falha> {
        if let Some(s) = self.falta_esperar() {
            return Err(Falha::Limite(s));
        }
        let resposta = self
            .http
            .post(&self.enderecos.consulta)
            .bearer_auth(&acesso.portador)
            .header("client-token", &acesso.cliente)
            .header("app-platform", "WebPlayer")
            .header("accept-language", "pt-BR")
            .json(&corpo(op, codigo, variaveis))
            .send()
            .await
            .map_err(de_rede)?;
        match resposta.status().as_u16() {
            200 => {}
            401 => return Err(Falha::Sessao),
            412 => return Err(Falha::Codigo),
            429 => {
                let cabecalho = resposta.headers().get("retry-after").and_then(|v| v.to_str().ok());
                let s = cabecalho.and_then(|v| v.parse::<u64>().ok()).unwrap_or(5).clamp(1, MAIOR_ESPERA_S);
                let ate = Instant::now() + Duration::from_secs(s);
                *self.esperar_ate.lock().unwrap_or_else(PoisonError::into_inner) = Some(ate);
                return Err(Falha::Limite(s));
            }
            500.. => return Err(Falha::Rede(format!("erro {} do Spotify", resposta.status().as_u16()))),
            outro => return Err(Falha::Resposta(format!("código {outro}"))),
        }
        let texto = resposta.text().await.map_err(de_rede)?;
        let mut v: Value = serde_json::from_str(&texto).map_err(|_| Falha::Resposta("não é JSON".into()))?;
        let erro = v["errors"]
            .as_array()
            .and_then(|e| e.first())
            .map(|e| e["message"].as_str().unwrap_or("sem motivo").chars().take(120).collect::<String>());
        if erro.as_deref().is_some_and(|e| e.contains("PersistedQueryNotFound")) {
            return Err(Falha::Codigo);
        }
        match v["data"].take() {
            // Um erro de uma parte da página com o resto presente: vale o que veio (a conversão confere o que precisa).
            Value::Object(data) => Ok(Value::Object(data)),
            _ => Err(Falha::Resposta(erro.unwrap_or_else(|| "veio sem dados".into()))),
        }
    }

    /// Procura no texto do site do Spotify o código de agora de `op` e o guarda.
    async fn procurar_codigo(&self, op: &str) -> Option<String> {
        match self.codigo_do_site(op).await {
            // O site não respondeu: não diz nada sobre o código, e vale procurar no próximo pedido.
            Err(()) => None,
            Ok(Some(codigo)) => {
                eprintln!("spotify: o código de {op} mudou; relido do site");
                if self.codigos.de(op).contains(&codigo) {
                    // O site ainda diz o mesmo que foi recusado.
                    self.em_vao(op);
                    return None;
                }
                self.codigos.guardar(op, &codigo);
                Some(codigo)
            }
            Ok(None) => {
                eprintln!("spotify: o código de {op} mudou, e o site não disse o novo");
                self.em_vao(op);
                None
            }
        }
    }

    fn em_vao(&self, op: &str) {
        let mut em_vao = self.procurou_em_vao.lock().unwrap_or_else(PoisonError::into_inner);
        em_vao.insert(op.to_string(), Instant::now());
    }

    /// `Err` quando o site não respondeu; `Ok(None)` quando respondeu e o código não está lá.
    async fn codigo_do_site(&self, op: &str) -> Result<Option<String>, ()> {
        let pagina = self.texto(&self.enderecos.site).await?;
        let Some(pacote) = pacote_principal(&pagina) else { return Ok(None) };
        let pacote = self.texto(&format!("{}{pacote}", self.enderecos.pacotes)).await?;
        if let Some(codigo) = codigo_no_texto(&pacote, op) {
            return Ok(Some(codigo));
        }
        for arquivo in rota_da_operacao(op).map(|rota| pedacos_da_rota(&pacote, rota)).unwrap_or_default() {
            if let Ok(js) = self.texto(&format!("{}{arquivo}", self.enderecos.pacotes)).await
                && let Some(codigo) = codigo_no_texto(&js, op)
            {
                return Ok(Some(codigo));
            }
        }
        Ok(None)
    }

    /// O texto de um arquivo público do site (sem o portador nem nada da conta), até [`MAIOR_PACOTE`].
    async fn texto(&self, endereco: &str) -> Result<String, ()> {
        let mut resposta = self.site.get(endereco).header("user-agent", "Mozilla/5.0").send().await.map_err(|_| ())?;
        if !resposta.status().is_success() {
            return Err(());
        }
        let mut bytes = Vec::new();
        while let Some(pedaco) = resposta.chunk().await.map_err(|_| ())? {
            bytes.extend_from_slice(&pedaco);
            if bytes.len() > MAIOR_PACOTE {
                return Err(());
            }
        }
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    }
}

/// O erro de rede numa frase curta, sem o endereço.
fn de_rede(e: reqwest::Error) -> Falha {
    let motivo = if e.is_timeout() {
        "demorou demais"
    } else if e.is_connect() {
        "sem conexão"
    } else {
        "a conexão caiu"
    };
    Falha::Rede(motivo.into())
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod testes {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    const NOVO: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    /// Um pedido que chegou: caminho, cabeçalhos (em minúsculas) e corpo.
    #[derive(Clone, Debug)]
    struct Chegou {
        caminho: String,
        cabecalhos: String,
        corpo: String,
    }

    type Responder = Box<dyn Fn(&Chegou) -> (u16, String, String) + Send>;

    /// Um servidor HTTP que responde pelo roteiro (status, cabeçalhos a mais, corpo) e guarda o que chegou.
    fn servidor(responder: Responder) -> (String, Arc<Mutex<Vec<Chegou>>>) {
        let ouvinte = TcpListener::bind("127.0.0.1:0").expect("porta");
        let base = format!("http://{}", ouvinte.local_addr().expect("endereço"));
        let vistos: Arc<Mutex<Vec<Chegou>>> = Arc::default();
        let guarda = vistos.clone();
        std::thread::spawn(move || {
            for conexao in ouvinte.incoming().flatten() {
                let mut leitor = BufReader::new(conexao.try_clone().expect("clone"));
                let mut linha = String::new();
                let _ = leitor.read_line(&mut linha);
                let caminho = linha.split_whitespace().nth(1).unwrap_or("").to_string();
                let mut cabecalhos = String::new();
                let mut tamanho = 0;
                loop {
                    let mut l = String::new();
                    if leitor.read_line(&mut l).unwrap_or(0) == 0 || l.trim().is_empty() {
                        break;
                    }
                    let l = l.trim().to_string();
                    let (nome, valor) = l.split_once(':').unwrap_or((&l, ""));
                    if nome.eq_ignore_ascii_case("content-length") {
                        tamanho = valor.trim().parse().unwrap_or(0);
                    }
                    cabecalhos.push_str(&format!("{}:{}\n", nome.to_lowercase(), valor.trim()));
                }
                let mut corpo = vec![0; tamanho];
                let _ = leitor.read_exact(&mut corpo);
                let chegou = Chegou { caminho, cabecalhos, corpo: String::from_utf8_lossy(&corpo).into_owned() };
                let (status, extras, resposta) = responder(&chegou);
                guarda.lock().expect("vistos").push(chegou);
                let mut saida = conexao;
                let _ = write!(
                    saida,
                    "HTTP/1.1 {status} X\r\ncontent-length: {}\r\nconnection: close\r\n{extras}\r\n{resposta}",
                    resposta.len()
                );
            }
        });
        (base, vistos)
    }

    fn pathfinder(base: &str, teste: &str) -> Arc<Pathfinder> {
        let pasta = std::env::temp_dir().join(format!("provedor-spotify-pf-{teste}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        let enderecos = Enderecos {
            consulta: format!("{base}/consulta"),
            site: format!("{base}/site"),
            pacotes: format!("{base}/pacotes/"),
        };
        Arc::new(Pathfinder::novo(enderecos, Codigos::novo(pasta.join("codigos.json"))).expect("pathfinder"))
    }

    fn consultar(p: &Arc<Pathfinder>, op: &str) -> Result<Value, Falha> {
        let acesso = Acesso { portador: "portador-x".into(), cliente: "cliente-y".into() };
        tokio::runtime::Builder::new_current_thread().enable_all().build().expect("rt").block_on(p.consultar(
            &acesso,
            op,
            &json!({"uri": "u"}),
        ))
    }

    fn quantos(vistos: &Mutex<Vec<Chegou>>, caminho: &str) -> usize {
        vistos.lock().expect("vistos").iter().filter(|c| c.caminho == caminho).count()
    }

    #[test]
    fn consulta_manda_o_portador_e_devolve_o_data() {
        let (base, vistos) = servidor(Box::new(|_| (200, String::new(), r#"{"data":{"x":1}}"#.into())));
        let p = pathfinder(&base, "simples");
        assert_eq!(consultar(&p, "home"), Ok(json!({"x": 1})));
        let v = vistos.lock().expect("vistos");
        assert!(v[0].cabecalhos.contains("authorization:Bearer portador-x"));
        assert!(v[0].cabecalhos.contains("client-token:cliente-y"));
        let corpo: Value = serde_json::from_str(&v[0].corpo).expect("json");
        assert_eq!(corpo["operationName"], "home");
        assert_eq!(corpo["variables"], json!({"uri": "u"}));
        assert_eq!(corpo["extensions"]["persistedQuery"]["sha256Hash"].as_str().map(str::len), Some(64));
    }

    #[test]
    fn operacao_desconhecida_nem_sai() {
        let (base, vistos) = servidor(Box::new(|_| (200, String::new(), r#"{"data":{}}"#.into())));
        let p = pathfinder(&base, "desconhecida");
        assert!(consultar(&p, "addToPlaylist").is_err(), "só as leituras deste provedor");
        assert!(vistos.lock().expect("vistos").is_empty());
    }

    #[test]
    fn codigo_trocado_e_relido_do_site_e_guardado() {
        let (base, vistos) = servidor(Box::new(|c| match c.caminho.as_str() {
            "/consulta" if c.corpo.contains(NOVO) => (200, String::new(), r#"{"data":{"ok":true}}"#.into()),
            "/consulta" => (412, String::new(), "Invalid query hash".into()),
            "/site" => (200, String::new(), r#"<script src="x/web-player/web-player.abc123.js">"#.into()),
            "/pacotes/web-player.abc123.js" => (200, String::new(), format!(r#"("home","query","{NOVO}")"#)),
            _ => (404, String::new(), String::new()),
        }));
        let p = pathfinder(&base, "trocado");
        assert_eq!(consultar(&p, "home"), Ok(json!({"ok": true})));
        assert_eq!(consultar(&p, "home"), Ok(json!({"ok": true})));
        assert_eq!(quantos(&vistos, "/site"), 1, "a segunda já usa o novo");
        assert_eq!(quantos(&vistos, "/consulta"), 3);
    }

    #[test]
    fn codigo_que_vive_num_pedaco_e_achado() {
        let (base, _) = servidor(Box::new(|c| match c.caminho.as_str() {
            "/consulta" if c.corpo.contains(NOVO) => (200, String::new(), r#"{"data":{"ok":true}}"#.into()),
            "/consulta" => (412, String::new(), String::new()),
            "/site" => (200, String::new(), "x/web-player/web-player.abc123.js".into()),
            "/pacotes/web-player.abc123.js" => {
                (200, String::new(), r#"{7:"xpui-routes-search"}{7:"0000aaaa"}{7:"1111bbbb"}"#.into())
            }
            "/pacotes/xpui-routes-search.1111bbbb.js" => {
                (200, String::new(), format!(r#"("searchDesktop","query","{NOVO}")"#))
            }
            _ => (404, String::new(), String::new()),
        }));
        assert_eq!(consultar(&pathfinder(&base, "pedaco"), "searchDesktop"), Ok(json!({"ok": true})));
    }

    #[test]
    fn sem_codigo_novo_nao_le_o_site_de_novo() {
        let (base, vistos) = servidor(Box::new(|c| match c.caminho.as_str() {
            "/consulta" => (412, String::new(), String::new()),
            _ => (200, String::new(), "nada".into()),
        }));
        let p = pathfinder(&base, "em-vao");
        assert_eq!(consultar(&p, "home"), Err(Falha::Codigo));
        assert_eq!(consultar(&p, "home"), Err(Falha::Codigo));
        assert_eq!(quantos(&vistos, "/site"), 1);
        assert_eq!(consultar(&p, "getAlbum"), Err(Falha::Codigo));
        assert_eq!(quantos(&vistos, "/site"), 2, "uma operação sem código não bloqueia as outras");
    }

    #[test]
    fn site_fora_do_ar_nao_bloqueia_a_proxima_procura() {
        let (base, vistos) = servidor(Box::new(|c| match c.caminho.as_str() {
            "/consulta" => (412, String::new(), String::new()),
            _ => (503, String::new(), String::new()),
        }));
        let p = pathfinder(&base, "fora-do-ar");
        assert!(consultar(&p, "home").is_err());
        assert!(consultar(&p, "home").is_err());
        assert_eq!(quantos(&vistos, "/site"), 2, "o site nem respondeu: vale procurar de novo");
    }

    #[test]
    fn quem_desiste_de_esperar_nao_interrompe_a_procura() {
        let (base, _) = servidor(Box::new(|c| match c.caminho.as_str() {
            "/consulta" if c.corpo.contains(NOVO) => (200, String::new(), r#"{"data":{"ok":true}}"#.into()),
            "/consulta" => (412, String::new(), String::new()),
            "/site" => {
                std::thread::sleep(Duration::from_millis(400));
                (200, String::new(), "x/web-player/web-player.abc123.js".into())
            }
            _ => (200, String::new(), format!(r#"("home","query","{NOVO}")"#)),
        }));
        let p = pathfinder(&base, "desiste");
        let rt = tokio::runtime::Builder::new_multi_thread().worker_threads(2).enable_all().build().expect("rt");
        let acesso = Acesso { portador: "p".into(), cliente: "c".into() };
        let curto = Duration::from_millis(100);
        assert!(
            rt.block_on(async { tokio::time::timeout(curto, p.consultar(&acesso, "home", &json!({}))).await }).is_err()
        );
        let ate = Instant::now() + Duration::from_secs(5);
        while p.codigos.de("home").len() < 2 && Instant::now() < ate {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(rt.block_on(p.consultar(&acesso, "home", &json!({}))), Ok(json!({"ok": true})));
    }

    #[test]
    fn relido_recusado_volta_ao_embutido_antes_do_site() {
        // O Player foi atualizado: o embutido novo vale, e o relido de antes ficou velho.
        let (base, vistos) = servidor(Box::new(|c| match c.caminho.as_str() {
            "/consulta" if c.corpo.contains(NOVO) => (412, String::new(), String::new()),
            "/consulta" => (200, String::new(), r#"{"data":{"ok":true}}"#.into()),
            _ => (200, String::new(), "nada".into()),
        }));
        let p = pathfinder(&base, "relido-velho");
        p.codigos.guardar("home", NOVO);
        assert_eq!(consultar(&p, "home"), Ok(json!({"ok": true})));
        assert_eq!(consultar(&p, "home"), Ok(json!({"ok": true})));
        assert_eq!(quantos(&vistos, "/site"), 0);
        assert_eq!(quantos(&vistos, "/consulta"), 3, "o velho é esquecido");
    }

    #[test]
    fn codigo_trocado_avisado_no_corpo_tambem_vale() {
        let (base, vistos) = servidor(Box::new(|c| match c.caminho.as_str() {
            "/consulta" => (200, String::new(), r#"{"errors":[{"message":"PersistedQueryNotFound"}]}"#.into()),
            _ => (200, String::new(), "nada".into()),
        }));
        assert_eq!(consultar(&pathfinder(&base, "no-corpo"), "home"), Err(Falha::Codigo));
        assert_eq!(quantos(&vistos, "/site"), 1);
    }

    #[test]
    fn limite_espera_sem_bater_de_novo() {
        let (base, vistos) = servidor(Box::new(|_| (429, "retry-after: 7\r\n".into(), String::new())));
        let p = pathfinder(&base, "limite");
        assert_eq!(consultar(&p, "home"), Err(Falha::Limite(7)));
        assert!(matches!(consultar(&p, "home"), Err(Falha::Limite(1..=7))));
        assert_eq!(quantos(&vistos, "/consulta"), 1, "durante a espera, nem pede");
    }

    #[test]
    fn portador_vencido_e_sessao() {
        let (base, _) = servidor(Box::new(|_| (401, String::new(), String::new())));
        assert_eq!(consultar(&pathfinder(&base, "vencido"), "home"), Err(Falha::Sessao));
    }

    #[test]
    fn erro_no_corpo_nao_vira_vazio() {
        let (base, _) =
            servidor(Box::new(|_| (200, String::new(), r#"{"data":null,"errors":[{"message":"nao deu"}]}"#.into())));
        assert!(
            matches!(consultar(&pathfinder(&base, "corpo"), "home"), Err(Falha::Resposta(m)) if m.contains("nao deu"))
        );
        let (base, _) = servidor(Box::new(|_| (200, String::new(), "nao e json".into())));
        assert!(matches!(consultar(&pathfinder(&base, "texto"), "home"), Err(Falha::Resposta(_))));
        let (base, _) = servidor(Box::new(|_| (503, String::new(), String::new())));
        assert!(matches!(consultar(&pathfinder(&base, "fora"), "home"), Err(Falha::Rede(_))));
    }

    #[test]
    fn sem_rede_e_rede() {
        let p = pathfinder("http://127.0.0.1:1", "sem-rede");
        let Err(Falha::Rede(m)) = consultar(&p, "home") else { panic!("devia ser rede") };
        assert!(!m.contains("127.0.0.1"), "a frase não leva o endereço: {m}");
    }
}
