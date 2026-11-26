//! O Spotify de verdade: o login pelo navegador (`oauth.rs`, com o Client ID público do librespot, o mesmo do Spotifast, sem
//! app de desenvolvedor) e a sessão do `librespot-core`, usada para os dados.

use crate::conta::{Autorizador, DadosDaConta, Falha, Tokens};
use crate::dados::{Consultas, Pedido};
use crate::oauth::{self, PORTA};
use crate::pathfinder::{Acesso, Pathfinder};
use librespot_core::authentication::Credentials;
use librespot_core::error::ErrorKind;
use librespot_core::{Session, SessionConfig};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const NOSSO_PROGRAMA: &str = "provedor-spotify.exe";
/// Quanto esperar a pessoa aprovar no navegador.
pub const PRAZO_DO_NAVEGADOR: Duration = Duration::from_secs(5 * 60);
/// Quanto a sessão pode demorar para entrar.
const PRAZO_DA_SESSAO: Duration = Duration::from_secs(30);
/// Quanto o portador e o client token podem demorar.
const PRAZO_DO_ACESSO: Duration = Duration::from_secs(8);
/// Quantas consultas ao mesmo tempo.
const PEDIDOS_POR_VEZ: usize = 6;
/// Quanto esperar o plano (o atributo `type`, que chega logo depois de entrar) e o nome do perfil.
const PRAZO_DO_PERFIL: Duration = Duration::from_secs(5);
/// A frase para a porta do retorno ocupada, com o programa que a usa quando dá para saber.
pub fn mensagem_da_porta(dono: Option<&str>) -> String {
    if dono.is_some_and(|d| d.eq_ignore_ascii_case(NOSSO_PROGRAMA)) {
        // Uma espera anterior que não soltou a porta: ninguém fecha o provedor à mão.
        return format!("A porta {PORTA} ainda está com uma tentativa anterior. Desligue e ligue o Spotify na Loja.");
    }
    let quem = dono.map_or_else(|| "outro programa".to_string(), |d| format!("o {d}"));
    format!("A porta {PORTA}, que o login do Spotify usa, está com {quem}. Feche-o e tente de novo.")
}

/// Quanto o Spotify pode demorar para dar os tokens.
const PRAZO_DOS_TOKENS: Duration = Duration::from_secs(30);

/// O erro de renovar: `invalid_grant` (credencial revogada ou de outro app) apaga; o resto é passageiro.
fn falha_da_renovacao(falha: oauth::FalhaDoToken) -> Falha {
    if falha.codigo.as_deref() == Some("invalid_grant") {
        Falha::Recusada("O Spotify não aceita mais o login guardado. Entre de novo.".into())
    } else {
        Falha::Passageira(format!("Sem resposta do Spotify ({}). Tente de novo.", falha.motivo))
    }
}

/// O id deste aparelho no Spotify, o mesmo em todas as aberturas (um novo a cada vez pareceria um computador novo na conta).
fn id_do_aparelho(arquivo: &Path) -> Option<String> {
    let vale = |id: &str| id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-');
    if let Some(id) = std::fs::read_to_string(arquivo).ok().map(|t| t.trim().to_string()).filter(|id| vale(id)) {
        return Some(id);
    }
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).ok()?;
    // Um UUID sorteado (versão 4).
    (b[6], b[8]) = ((b[6] & 0x0f) | 0x40, (b[8] & 0x3f) | 0x80);
    let hex: String = b.iter().map(|b| format!("{b:02x}")).collect();
    let id = format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..]);
    if let Err(e) = std::fs::write(arquivo, &id) {
        eprintln!("spotify: o id do aparelho vale só desta vez ({e})");
    }
    Some(id)
}

pub struct Real {
    rt: tokio::runtime::Runtime,
    http: reqwest::Client,
    aparelho: Option<String>,
    /// A pasta temporária do librespot: limpa a cada abertura.
    tmp: PathBuf,
    sessao: Mutex<Option<Session>>,
    /// O navegador está sendo esperado (a porta é nossa): só assim [`Autorizador::cancelar`] mexe nela, para não
    /// bater na porta de outro programa.
    esperando: AtomicBool,
    /// A pessoa desistiu da espera do navegador.
    parar: AtomicBool,
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
        let http = reqwest::Client::builder().timeout(PRAZO_DOS_TOKENS).build().map_err(|e| e.to_string())?;
        Ok(Self {
            rt,
            http,
            aparelho: id_do_aparelho(&pasta.join("aparelho.txt")),
            tmp,
            sessao: Mutex::new(None),
            esperando: AtomicBool::new(false),
            parar: AtomicBool::new(false),
        })
    }

    /// A sessão de agora (os dados são pedidos por ela).
    pub fn sessao(&self) -> Option<Session> {
        self.sessao.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
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
        let prontas: std::sync::Mutex<Vec<Result<serde_json::Value, Falha>>> = std::sync::Mutex::default();
        let todas = async {
            let acesso = match self.acesso().await {
                Ok(a) => Arc::new(a),
                Err(f) => {
                    let mut p = prontas.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    p.extend(pedidos.iter().map(|_| Err(f.clone())));
                    return;
                }
            };
            // Sempre até PEDIDOS_POR_VEZ em andamento (um sai, o próximo entra), na ordem; depois de um erro, nenhum
            // pedido novo sai (um "pedidos demais" não é martelado).
            let mut andando = std::collections::VecDeque::new();
            let mut proximo = 0;
            let mut falhou = None;
            loop {
                while falhou.is_none() && proximo < pedidos.len() && andando.len() < PEDIDOS_POR_VEZ {
                    let (op, variaveis) = &pedidos[proximo];
                    let (pf, acesso) = (self.pathfinder.clone(), acesso.clone());
                    let (op, variaveis) = (*op, variaveis.clone());
                    andando.push_back(tokio::spawn(async move { pf.consultar(&acesso, op, &variaveis).await }));
                    proximo += 1;
                }
                let Some(tarefa) = andando.pop_front() else { break };
                let caiu = || Falha::Resposta("o pedido parou no meio".into());
                let resposta = tarefa.await.unwrap_or_else(|_| Err(caiu()));
                if let (Err(f), None) = (&resposta, &falhou) {
                    falhou = Some(f.clone());
                }
                prontas.lock().unwrap_or_else(std::sync::PoisonError::into_inner).push(resposta);
            }
        };
        // Passado o prazo, ninguém lê o resto: o que já saiu termina sozinho, e o pedido seguinte não espera. As
        // respostas que chegaram valem (as primeiras páginas de uma lista enorme); as outras são "demorou demais".
        self.real.rt.block_on(async {
            let _ = tokio::time::timeout_at(ate.into(), todas).await;
        });
        let mut respostas = prontas.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
        let falha = respostas.iter().find_map(|r| r.as_ref().err().cloned()).unwrap_or(Falha::Prazo);
        respostas.resize_with(pedidos.len(), || Err(falha.clone()));
        respostas
    }
}

impl Real {
    /// Abre a porta do retorno e o navegador, e espera a pessoa aprovar (a porta fecha ao voltar).
    fn codigo_do_navegador(&self, pedido: &oauth::Pedido) -> Result<String, String> {
        let ouvinte = oauth::escutar().map_err(|_| mensagem_da_porta(crate::porta::dono(PORTA).as_deref()))?;
        // Os testes escutam na porta de verdade, mas não abrem o navegador de quem os roda.
        if !cfg!(test) {
            oauth::abrir_o_navegador(&pedido.url).map_err(|e| format!("O navegador não abriu ({e})."))?;
        }
        oauth::esperar_o_codigo(&ouvinte, &pedido.estado, PRAZO_DO_NAVEGADOR, &self.parar)
    }
}

impl Autorizador for Real {
    fn autorizar(&self) -> Result<Tokens, String> {
        let pedido = oauth::comecar()?;
        // Antes de a porta abrir: cancelar daqui em diante já vale.
        self.parar.store(false, Ordering::Release);
        self.esperando.store(true, Ordering::Release);
        let codigo = self.codigo_do_navegador(&pedido);
        self.esperando.store(false, Ordering::Release);
        let codigo = codigo?;
        self.rt
            .block_on(oauth::trocar_o_codigo(&self.http, &codigo, &pedido.verificador))
            .map(|t| Tokens { access: t.access, refresh: t.refresh })
            .map_err(|f| format!("O Spotify não autorizou ({}). Tente de novo.", f.motivo))
    }

    fn renovar(&self, refresh: &str) -> Result<Tokens, Falha> {
        self.rt
            .block_on(oauth::renovar(&self.http, refresh))
            .map(|t| Tokens { access: t.access, refresh: t.refresh })
            .map_err(falha_da_renovacao)
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
        self.parar.store(true, Ordering::Release);
        // A porta solta logo depois; espera um pouco, para um `entrar` em seguida não a achar ocupada.
        for _ in 0..20 {
            if self.porta_ocupada().is_none() {
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn conectar(&self, access: &str) -> Result<DadosDaConta, Falha> {
        let mut config = SessionConfig { tmp_dir: self.tmp.clone(), ..SessionConfig::default() };
        if let Some(id) = &self.aparelho {
            config.device_id.clone_from(id);
        }
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
        let com = |codigo: Option<&str>, motivo: &str| {
            falha_da_renovacao(oauth::FalhaDoToken { codigo: codigo.map(str::to_string), motivo: motivo.into() })
        };
        assert!(matches!(com(Some("invalid_grant"), "400: Refresh token revoked"), Falha::Recusada(_)));
        assert!(matches!(com(None, "timed out"), Falha::Passageira(_)));
        assert!(matches!(com(Some("server_error"), "503: invalid_grant no texto"), Falha::Passageira(_)));
    }

    #[test]
    fn o_id_do_aparelho_e_o_mesmo_nas_proximas_aberturas() {
        let arquivo = std::env::temp_dir().join(format!("provedor-spotify-aparelho-{}.txt", std::process::id()));
        let _ = std::fs::remove_file(&arquivo);
        let id = id_do_aparelho(&arquivo).expect("id");
        assert_eq!((id.len(), id.matches('-').count(), &id[14..15]), (36, 4, "4"), "{id}");
        assert_eq!(id_do_aparelho(&arquivo).as_deref(), Some(id.as_str()));
        std::fs::write(&arquivo, "estragado").expect("estragar");
        let outro = id_do_aparelho(&arquivo).expect("outro");
        assert_ne!(outro, id, "um estragado é trocado");
        let _ = std::fs::remove_file(&arquivo);
    }
}
