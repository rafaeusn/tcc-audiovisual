//! O Spotify de verdade: o OAuth pelo navegador (`librespot-oauth`, com o Client ID público do librespot, o mesmo do
//! Spotifast, sem app de desenvolvedor) e a sessão do `librespot-core`, que as próximas partes usam para os dados e
//! o áudio.
//!
//! Duas manias do `librespot-oauth` que este arquivo contorna:
//! - ele espera o retorno do navegador para sempre, segurando a porta: depois de [`PRAZO_DO_NAVEGADOR`], este
//!   provedor chama a própria porta com um retorno sem código, e a espera acaba com erro;
//! - ele escreve o endereço do login na saída padrão, que é o canal do protocolo: o Player ignora a linha (não é
//!   JSON) e só a anota no registro. O endereço não dá acesso a nada sem o verificador do PKCE, que fica aqui.

use crate::conta::{Autorizador, DadosDaConta, Falha, Tokens};
use crate::dados::{Consultas, Pedido};
use crate::pathfinder::{Acesso, Pathfinder};
use librespot_core::authentication::Credentials;
use librespot_core::error::ErrorKind;
use librespot_core::{Session, SessionConfig};
use librespot_oauth::{OAuthClient, OAuthClientBuilder, OAuthError};
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// O Client ID público do librespot (o do aplicativo de computador do Spotify). Não é segredo: está no librespot.
pub const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";
/// A porta do retorno, a única que esse Client ID aceita (a mesma do Spotifast).
pub const PORTA: u16 = 8898;
const RETORNO: &str = "http://127.0.0.1:8898/login";
const NOSSO_PROGRAMA: &str = "provedor-spotify.exe";
/// O que as partes 1 a 5 usam (pedir agora evita um segundo login): tocar, a conta, as playlists, a coleção, quem
/// a pessoa segue e o estado do Connect.
const ESCOPOS: [&str; 8] = [
    "streaming",
    "user-read-private",
    "playlist-read-private",
    "playlist-read-collaborative",
    "user-library-read",
    "user-follow-read",
    "user-read-playback-state",
    "user-modify-playback-state",
];
/// Quanto esperar a pessoa aprovar no navegador.
pub const PRAZO_DO_NAVEGADOR: Duration = Duration::from_secs(5 * 60);
/// Quanto a sessão pode demorar para entrar.
const PRAZO_DA_SESSAO: Duration = Duration::from_secs(30);
/// Quanto o portador e o client token podem demorar.
const PRAZO_DO_ACESSO: Duration = Duration::from_secs(8);
/// Quantas consultas ao mesmo tempo.
const PEDIDOS_POR_VEZ: usize = 4;
/// Quanto esperar o plano (o atributo `type`, que chega logo depois de entrar) e o nome do perfil.
const PRAZO_DO_PERFIL: Duration = Duration::from_secs(5);
/// A página que o navegador mostra depois da aprovação.
const PAGINA_DE_PRONTO: &str = "<!doctype html><html lang=\"pt-BR\"><meta charset=\"utf-8\"><title>Player</title>\
<body style=\"font-family:system-ui,sans-serif;background:#060606;color:#f2f2f2;display:grid;place-items:center;\
height:100vh;margin:0\"><div style=\"text-align:center\"><h1 style=\"font-weight:600\">Pronto</h1>\
<p>A conta do Spotify entrou no Player. Pode fechar esta aba.</p></div></body></html>";

/// A frase para a porta do retorno ocupada, com o programa que a usa quando dá para saber.
pub fn mensagem_da_porta(dono: Option<&str>) -> String {
    if dono.is_some_and(|d| d.eq_ignore_ascii_case(NOSSO_PROGRAMA)) {
        // Uma espera anterior que não soltou a porta: ninguém fecha o provedor à mão.
        return format!("A porta {PORTA} ainda está com uma tentativa anterior. Desligue e ligue o Spotify na Loja.");
    }
    let quem = dono.map_or_else(|| "outro programa".to_string(), |d| format!("o {d}"));
    format!("A porta {PORTA}, que o login do Spotify usa, está com {quem}. Feche-o e tente de novo.")
}

/// O erro de renovar: `invalid_grant` (o OAuth para credencial revogada ou de outro app) apaga; o resto é passageiro.
pub fn falha_da_renovacao(motivo: &str) -> Falha {
    if motivo.contains("invalid_grant") {
        Falha::Recusada("O Spotify não aceita mais o login guardado. Entre de novo.".into())
    } else {
        Falha::Passageira(format!("Sem resposta do Spotify ({motivo}). Tente de novo."))
    }
}

pub struct Real {
    rt: tokio::runtime::Runtime,
    /// A pasta temporária do librespot (pedaços de áudio, ainda cifrados, na parte 3): limpa a cada abertura.
    tmp: PathBuf,
    sessao: Mutex<Option<Session>>,
    /// O navegador está sendo esperado (a porta é nossa): só assim [`Autorizador::cancelar`] mexe nela, para não
    /// bater na porta de outro programa.
    esperando: AtomicBool,
}

impl Real {
    pub fn novo(pasta: &Path) -> Result<Self, String> {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let tmp = pasta.join("librespot-tmp");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| format!("a pasta {} não foi criada: {e}", tmp.display()))?;
        Ok(Self { rt, tmp, sessao: Mutex::new(None), esperando: AtomicBool::new(false) })
    }

    /// A sessão de agora (os dados e, na parte 3, o áudio são pedidos por ela).
    pub fn sessao(&self) -> Option<Session> {
        self.sessao.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    fn cliente() -> Result<OAuthClient, String> {
        let mut construtor =
            OAuthClientBuilder::new(CLIENT_ID, RETORNO, ESCOPOS.to_vec()).with_custom_message(PAGINA_DE_PRONTO);
        // Os testes escutam na porta de verdade, mas não abrem o navegador de quem os roda.
        if !cfg!(test) {
            construtor = construtor.open_in_browser();
        }
        construtor.build().map_err(|e| e.to_string())
    }
}

/// As consultas do Pathfinder com a sessão de agora.
pub struct PelaSessao {
    real: Arc<Real>,
    pathfinder: Arc<Pathfinder>,
}

impl PelaSessao {
    pub fn novo(real: Arc<Real>, pathfinder: Pathfinder) -> Self {
        Self { real, pathfinder: Arc::new(pathfinder) }
    }

    /// O portador do Login5 e o client token da sessão. Sem eles, é a sessão que precisa voltar.
    async fn acesso(&self) -> Result<Acesso, crate::pathfinder::Falha> {
        use crate::pathfinder::Falha;
        let sessao = self.real.sessao().filter(|s| !s.is_invalid()).ok_or(Falha::Sessao)?;
        let tokens = async {
            let portador = sessao.login5().auth_token().await.map_err(|e| falha_do_acesso(e.kind))?.access_token;
            let cliente = sessao.spclient().client_token().await.map_err(|e| falha_do_acesso(e.kind))?;
            Ok(Acesso { portador, cliente })
        };
        match tokio::time::timeout(PRAZO_DO_ACESSO, tokens).await {
            Ok(acesso) => acesso,
            Err(_) => Err(Falha::Rede("demorou demais".into())),
        }
    }
}

/// O erro de conseguir o portador: a rede e os limites são passageiros; o resto pede a sessão de novo.
fn falha_do_acesso(tipo: ErrorKind) -> crate::pathfinder::Falha {
    use crate::pathfinder::Falha;
    match tipo {
        ErrorKind::DeadlineExceeded | ErrorKind::Cancelled | ErrorKind::Aborted => Falha::Rede("demorou demais".into()),
        ErrorKind::ResourceExhausted => Falha::Limite(30),
        _ => Falha::Sessao,
    }
}

impl Consultas for PelaSessao {
    fn consultar(&self, pedidos: &[Pedido], ate: Instant) -> Vec<Result<serde_json::Value, crate::pathfinder::Falha>> {
        use crate::pathfinder::Falha;
        let todas = async {
            let acesso = match self.acesso().await {
                Ok(a) => Arc::new(a),
                Err(f) => return pedidos.iter().map(|_| Err(f.clone())).collect(),
            };
            let mut respostas = Vec::with_capacity(pedidos.len());
            // Alguns por vez, e cada leva só começa se a anterior deu certo (um "pedidos demais" não é martelado).
            for leva in pedidos.chunks(PEDIDOS_POR_VEZ) {
                let tarefas: Vec<_> = leva
                    .iter()
                    .map(|(op, variaveis)| {
                        let (pf, acesso) = (self.pathfinder.clone(), acesso.clone());
                        let (op, variaveis) = (*op, variaveis.clone());
                        tokio::spawn(async move { pf.consultar(&acesso, op, &variaveis).await })
                    })
                    .collect();
                for t in tarefas {
                    let caiu = || Falha::Resposta("o pedido parou no meio".into());
                    respostas.push(t.await.unwrap_or_else(|_| Err(caiu())));
                }
                if let Some(f) = respostas.iter().find_map(|r| r.as_ref().err().cloned()) {
                    respostas.resize_with(pedidos.len(), || Err(f.clone()));
                    break;
                }
            }
            respostas
        };
        // Passado o prazo, ninguém lê a resposta: o que já saiu termina sozinho, e o pedido seguinte não espera.
        self.real.rt.block_on(async {
            match tokio::time::timeout_at(ate.into(), todas).await {
                Ok(respostas) => respostas,
                Err(_) => pedidos.iter().map(|_| Err(Falha::Rede("demorou demais".into()))).collect(),
            }
        })
    }
}

/// Acaba com a espera do `librespot-oauth`: um retorno sem código na própria porta.
fn destravar_a_porta() {
    let endereco = SocketAddr::from(([127, 0, 0, 1], PORTA));
    if let Ok(mut s) = TcpStream::connect_timeout(&endereco, Duration::from_secs(1)) {
        let _ = s.write_all(b"GET /login?error=prazo HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    }
}

impl Autorizador for Real {
    fn autorizar(&self) -> Result<Tokens, String> {
        let cliente = Self::cliente()?;
        let (avisar, resposta) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = avisar.send(cliente.get_access_token());
        });
        self.esperando.store(true, Ordering::Release);
        let resposta = resposta.recv_timeout(PRAZO_DO_NAVEGADOR);
        self.esperando.store(false, Ordering::Release);
        match resposta {
            Ok(Ok(t)) => {
                Ok(Tokens { access: t.access_token, refresh: Some(t.refresh_token).filter(|r| !r.is_empty()) })
            }
            Ok(Err(OAuthError::AuthCodeListenerBind { .. })) => {
                Err(mensagem_da_porta(crate::porta::dono(PORTA).as_deref()))
            }
            Ok(Err(e)) => Err(format!("O Spotify não autorizou ({e}). Tente de novo.")),
            Err(_) => {
                destravar_a_porta();
                Err("Ninguém aprovou no navegador em 5 minutos. Tente de novo.".into())
            }
        }
    }

    fn renovar(&self, refresh: &str) -> Result<Tokens, Falha> {
        let cliente = Self::cliente().map_err(Falha::Passageira)?;
        match cliente.refresh_token(refresh) {
            Ok(t) => Ok(Tokens { access: t.access_token, refresh: Some(t.refresh_token).filter(|r| !r.is_empty()) }),
            Err(e) => Err(falha_da_renovacao(&e.to_string())),
        }
    }

    fn desconectar(&self) {
        let sessao = self.sessao.lock().unwrap_or_else(std::sync::PoisonError::into_inner).take();
        if let Some(s) = sessao {
            s.shutdown();
        }
    }

    fn cancelar(&self) {
        if !self.esperando.load(Ordering::Acquire) {
            return;
        }
        destravar_a_porta();
        // A porta solta logo depois; espera um pouco, para um `entrar` em seguida não a achar ocupada.
        for _ in 0..20 {
            if self.porta_ocupada().is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn conectar(&self, access: &str) -> Result<DadosDaConta, Falha> {
        let config = SessionConfig { tmp_dir: self.tmp.clone(), ..SessionConfig::default() };
        let credenciais = Credentials::with_access_token(access);
        let sessao = self.rt.block_on(async {
            let sessao = Session::new(config, None);
            match tokio::time::timeout(PRAZO_DA_SESSAO, sessao.connect(credenciais, false)).await {
                Ok(Ok(())) => Ok(sessao),
                Ok(Err(e)) => Err(falha_da_sessao(&e.to_string())),
                Err(_) => Err(Falha::Passageira("A sessão do Spotify não respondeu em 30 s. Tente de novo.".into())),
            }
        })?;
        // O plano e o nome chegam depois de entrar: o plano num pacote da sessão, o nome no perfil. Sem eles no
        // prazo, a conta vale assim mesmo (o código da conta no lugar do nome, e o plano desconhecido).
        let dados = self.rt.block_on(async {
            let usuario = sessao.username();
            let perfil =
                tokio::time::timeout(PRAZO_DO_PERFIL, sessao.spclient().get_user_profile(&usuario, Some(0), Some(0)));
            let nome = match perfil.await {
                Ok(Ok(json)) => nome_do_perfil(&json),
                _ => None,
            };
            let ate = tokio::time::Instant::now() + PRAZO_DO_PERFIL;
            let mut tipo = sessao.get_user_attribute("type");
            while tipo.is_none() && tokio::time::Instant::now() < ate {
                tokio::time::sleep(Duration::from_millis(100)).await;
                tipo = sessao.get_user_attribute("type");
            }
            DadosDaConta { nome: nome.unwrap_or(usuario), premium: crate::conta::plano(tipo.as_deref()) }
        });
        let antiga = self.sessao.lock().unwrap_or_else(std::sync::PoisonError::into_inner).replace(sessao);
        if let Some(s) = antiga {
            s.shutdown();
        }
        Ok(dados)
    }

    fn porta_ocupada(&self) -> Option<String> {
        match TcpListener::bind(("127.0.0.1", PORTA)) {
            Ok(_) => None,
            Err(_) => Some(mensagem_da_porta(crate::porta::dono(PORTA).as_deref())),
        }
    }
}

/// O erro de entrar na sessão: só a recusa da conta apaga a credencial; a rede e o resto são passageiros.
pub fn falha_da_sessao(motivo: &str) -> Falha {
    if motivo.contains("Premium account required") {
        Falha::Recusada("Esta conta do Spotify não é Premium, e este modo precisa do Premium.".into())
    } else if motivo.contains("Bad credentials") {
        Falha::Recusada("O Spotify recusou esta conta. Entre de novo.".into())
    } else {
        Falha::Passageira(format!("A sessão do Spotify não entrou ({motivo}). Tente de novo."))
    }
}

/// O nome para mostrar, do perfil em JSON (`name` ou `display_name`).
pub fn nome_do_perfil(json: &[u8]) -> Option<String> {
    let perfil: serde_json::Value = serde_json::from_slice(json).ok()?;
    ["name", "display_name"]
        .iter()
        .filter_map(|campo| perfil.get(campo)?.as_str())
        .find(|nome| !nome.trim().is_empty())
        .map(str::to_string)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn nome_do_perfil_do_json_do_spotify() {
        let json = br#"{"uri":"spotify:user:31ax","name":"Rafael","image_url":"x"}"#;
        assert_eq!(nome_do_perfil(json).as_deref(), Some("Rafael"));
        assert_eq!(nome_do_perfil(br#"{"display_name":"R"}"#).as_deref(), Some("R"));
        assert_eq!(nome_do_perfil(br#"{"name":""}"#), None, "vazio não vale");
        assert_eq!(nome_do_perfil(b"nao e json"), None);
    }

    #[test]
    fn a_frase_da_porta_diz_quem_usa() {
        let f = mensagem_da_porta(Some("Spotifast.exe"));
        assert!(f.contains("8898") && f.contains("Spotifast.exe") && f.contains("Feche"), "{f}");
        assert!(mensagem_da_porta(None).contains("outro programa"));
    }

    #[test]
    fn o_portador_que_demora_nao_derruba_a_sessao() {
        use crate::pathfinder::Falha;
        assert!(matches!(falha_do_acesso(ErrorKind::DeadlineExceeded), Falha::Rede(_)));
        assert!(matches!(falha_do_acesso(ErrorKind::ResourceExhausted), Falha::Limite(_)));
        assert_eq!(falha_do_acesso(ErrorKind::Unauthenticated), Falha::Sessao);
    }

    #[test]
    fn so_a_recusa_da_conta_apaga_a_credencial() {
        for recusa in ["Bad credentials", "Premium account required"] {
            let motivo = format!("Permission denied {{ Login failed with reason: {recusa} }}");
            assert!(matches!(falha_da_sessao(&motivo), Falha::Recusada(_)), "{recusa}");
        }
        for passageiro in
            ["Service unavailable { transport returned no data }", "Login failed with reason: Try another access point"]
        {
            assert!(matches!(falha_da_sessao(passageiro), Falha::Passageira(_)), "{passageiro}");
        }
    }

    #[test]
    fn a_porta_presa_pelo_proprio_provedor_pede_para_religar() {
        let f = mensagem_da_porta(Some("provedor-spotify.exe"));
        assert!(f.contains("Loja") && !f.contains("Feche"), "a pessoa não fecha o provedor à mão: {f}");
    }

    #[test]
    fn cancelar_acaba_com_a_espera_do_navegador_e_solta_a_porta() {
        let pasta = std::env::temp_dir().join(format!("provedor-spotify-cancelar-{}", std::process::id()));
        let real = std::sync::Arc::new(Real::novo(&pasta).expect("abrir"));
        if let Some(frase) = real.porta_ocupada() {
            eprintln!("cancelar_acaba_com_a_espera: pulado ({frase})");
            return;
        }
        // Sem ninguém esperando, cancelar não mexe na porta (pode ser de outro programa).
        real.cancelar();
        let espera = {
            let real = real.clone();
            std::thread::spawn(move || real.autorizar())
        };
        let ate = std::time::Instant::now() + Duration::from_secs(5);
        while real.porta_ocupada().is_none() && std::time::Instant::now() < ate {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(real.porta_ocupada().is_some(), "o login escuta na porta");
        real.cancelar();
        assert!(real.porta_ocupada().is_none(), "a porta ficou livre para o próximo login");
        assert!(espera.join().expect("thread").is_err(), "a espera acabou com erro");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn so_o_invalid_grant_apaga_a_credencial() {
        let revogado = "Server returned error response: invalid_grant: Refresh token revoked";
        assert!(matches!(falha_da_renovacao(revogado), Falha::Recusada(_)));
        assert!(matches!(falha_da_renovacao("Request failed: timed out"), Falha::Passageira(_)));
    }
}
