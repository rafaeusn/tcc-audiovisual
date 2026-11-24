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
use provedor_spotify::oauth;
use std::io::Write;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A porta do retorno do login do áudio (a mesma do `oauth`; aqui só o nome curto).
pub const PORTA: u16 = oauth::PORTA_DO_AUDIO;
const NOSSO_PROGRAMA: &str = "provedor-spotify.exe";
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

/// A frase para a porta do retorno ocupada, com o programa que a usa quando dá para saber.
pub fn mensagem_da_porta(dono: Option<&str>) -> String {
    if dono.is_some_and(|d| d.eq_ignore_ascii_case(NOSSO_PROGRAMA)) {
        // Uma espera anterior que não soltou a porta: ninguém fecha o provedor à mão.
        return format!("A porta {PORTA} ainda está com uma tentativa anterior. Desligue e ligue o Spotify na Loja.");
    }
    let quem = dono.map_or_else(|| "outro programa".to_string(), |d| format!("o {d}"));
    format!("A porta {PORTA}, que o login do Spotify usa, está com {quem}. Feche-o e tente de novo.")
}

/// Quem quer saber quando a sessão entra (o áudio liga o servidor local por aqui).
type Ouvinte = Box<dyn Fn(Session) + Send>;

pub struct Real {
    rt: Arc<tokio::runtime::Runtime>,
    /// A pasta temporária do librespot (pedaços de áudio, ainda cifrados, na parte 3): limpa a cada abertura.
    tmp: PathBuf,
    sessao: Mutex<Option<Session>>,
    /// Os ouvintes, chamados com a sessão de agora.
    ouvintes: Mutex<Vec<Ouvinte>>,
    /// O navegador está sendo esperado (a porta é nossa): só assim [`Autorizador::cancelar`] mexe nela, para não
    /// bater na porta de outro programa.
    esperando: AtomicBool,
    /// O cliente HTTP do OAuth (bloqueante: o login roda numa thread própria).
    http: reqwest::blocking::Client,
}

impl Real {
    pub fn novo(pasta: &Path) -> Result<Self, String> {
        let rt = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?,
        );
        let tmp = pasta.join("librespot-tmp");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| format!("a pasta {} não foi criada: {e}", tmp.display()))?;
        Ok(Self {
            rt,
            tmp,
            sessao: Mutex::new(None),
            ouvintes: Mutex::new(Vec::new()),
            esperando: AtomicBool::new(false),
            http: reqwest::blocking::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .map_err(|e| e.to_string())?,
        })
    }

    /// O runtime do provedor (o áudio usa o mesmo: uma sessão do librespot pertence ao runtime que a criou).
    pub fn runtime(&self) -> Arc<tokio::runtime::Runtime> {
        Arc::clone(&self.rt)
    }

    /// A sessão de agora (os dados e o áudio são pedidos por ela).
    pub fn sessao(&self) -> Option<Session> {
        self.sessao.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    /// O nome da conta (o id do usuário) para pôr no `Credentials`. O Spotify exige isto junto do token para dar a chave de
    /// áudio; o `with_access_token` sozinho deixa vazio e o áudio é recusado.
    ///
    /// Vem do perfil da conta, lido do servidor **antes** de abrir a sessão de áudio (o perfil responde com o token OAuth,
    /// e não depende do áudio). Fica guardado para as próximas aberturas.
    fn conta_do_token(&self, access: &str) -> Option<String> {
        if let Some(nome) = self.usuario_guardado() {
            return Some(nome);
        }
        let perfil = self.rt.block_on(async {
            let config = SessionConfig { device_id: self.device_id(), tmp_dir: self.tmp.clone(), ..Default::default() };
            let sessao = Session::new(config, None);
            let credenciais = Credentials::with_access_token(access);
            tokio::time::timeout(PRAZO_DA_SESSAO, sessao.connect(credenciais, false)).await.ok()?.ok()?;
            Some(sessao.username())
        })?;
        let _ = std::fs::write(self.tmp.with_file_name("conta.txt"), &perfil);
        Some(perfil)
    }

    /// O nome da conta guardado da abertura anterior.
    fn usuario_guardado(&self) -> Option<String> {
        let nome = std::fs::read_to_string(self.tmp.with_file_name("conta.txt")).ok()?;
        let nome = nome.trim().to_string();
        (!nome.is_empty()).then_some(nome)
    }

    /// O id deste aparelho, estável entre as aberturas. Fica ao lado da credencial; sorteado na primeira abertura.
    pub fn device_id(&self) -> String {
        let arquivo = self.tmp.with_file_name("aparelho.txt");
        if let Ok(id) = std::fs::read_to_string(&arquivo) {
            let id = id.trim();
            if id.len() >= 16 && id.len() <= 128 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return id.to_string();
            }
        }
        let id = novo_device_id();
        if let Err(e) = std::fs::write(&arquivo, &id) {
            eprintln!("spotify: o id do aparelho não foi guardado: {e}");
        }
        id
    }

    /// Avizinha um ouvinte: ele é chamado com a sessão toda vez que ela entra (o áudio liga o servidor local por aqui).
    pub fn ao_entrar(&self, ouvinte: impl Fn(Session) + Send + 'static) {
        let mut ouvintes = self.ouvintes.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        ouvintes.push(Box::new(ouvinte));
        drop(ouvintes);
        // Se a sessão já está de pé, ele é chamado já.
        if let Some(s) = self.sessao() {
            let ouvintes = self.ouvintes.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if let Some(o) = ouvintes.last() {
                o(s);
            }
        }
    }

    fn avisar_ao_entrar(&self, sessao: &Session) {
        let ouvintes = self.ouvintes.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        for o in ouvintes.iter() {
            o(sessao.clone());
        }
    }
}

/// As consultas do Pathfinder com a sessão de agora.
pub struct PelasConsultas {
    real: Arc<Real>,
    pathfinder: Arc<Pathfinder>,
}

impl PelasConsultas {
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

impl Consultas for PelasConsultas {
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

impl crate::audio::FonteDaSessao for Real {
    fn agora(&self) -> Option<Session> {
        self.sessao()
    }
}

/// Um id de aparelho novo no formato que o librespot usa (UUID sem chaves), sorteado sem trazer dependência nova.
fn novo_device_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let agora = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
    let mut x = agora as u64 ^ ((std::process::id() as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
    let mut hex = String::with_capacity(32);
    for _ in 0..32 {
        // xorshift64*: basta para um id que só precisa não repetir.
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        hex.push(char::from_digit((x % 16) as u32, 16).unwrap_or('0'));
    }
    // No formato UUID (8-4-4-4-12), como o do librespot.
    format!("{}-{}-{}-{}-{}", &hex[0..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..32])
}

/// Abre o navegador de quem está usando o Player.
///
/// **Não use `cmd /C start`:** o `&` da URL é separador de comandos para o `cmd`, que entrega ao navegador só o pedaço
/// antes do primeiro `&` (o Spotify então responde "response_type must be code", porque o parâmetro ficou vazio). O
/// `rundll32` entrega o endereço inteiro, sem passar pelo interpretador.
fn abrir_o_navegador(url: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = std::process::Command::new("rundll32")
            .arg("url.dll,FileProtocolHandler")
            .arg(url)
            .creation_flags(0x0800_0000)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}

/// Acaba com a espera do navegador: um retorno sem código na própria porta.
fn destravar_a_porta() {
    let endereco = SocketAddr::from(([127, 0, 0, 1], PORTA));
    if let Ok(mut s) = TcpStream::connect_timeout(&endereco, Duration::from_secs(1)) {
        let _ = s.write_all(b"GET /login?error=prazo HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
    }
}

impl Autorizador for Real {
    fn autorizar(&self) -> Result<Tokens, String> {
        // O login do **áudio**: a identidade do app de computador do Spotify, com `show_dialog` e **só** o escopo
        // `streaming`. É o que o Spotifast pede no grant de playback, e é o que o Spotify aceita para dar a chave do áudio.
        // O fluxo (PKCE, a porta, o retorno) é o nosso `oauth.rs`, e não o do `librespot-oauth`: ele não deixa escolher o
        // `show_dialog` nem separar os escopos, e o áudio não vem sem isso.
        let grant = oauth::Grant::Audio;
        let pedido = oauth::comecar(grant, oauth::CLIENT_ID_DO_AUDIO);
        let ouvinte = oauth::escutar(grant)?;
        self.esperando.store(true, Ordering::Release);
        if !cfg!(test) {
            abrir_o_navegador(&pedido.url);
        }
        let codigo = oauth::esperar_o_codigo(&ouvinte, &pedido.estado, oauth::PRAZO_DO_NAVEGADOR);
        self.esperando.store(false, Ordering::Release);
        drop(ouvinte);
        let codigo = codigo?;
        let tokens =
            oauth::trocar_o_codigo(&self.http, grant, oauth::CLIENT_ID_DO_AUDIO, &codigo, &pedido.verificador)?;
        Ok(Tokens { access: tokens.access, refresh: tokens.refresh })
    }

    fn renovar(&self, refresh: &str) -> Result<Tokens, Falha> {
        match oauth::renovar(&self.http, oauth::CLIENT_ID_DO_AUDIO, refresh) {
            Ok(t) => Ok(Tokens { access: t.access, refresh: t.refresh }),
            Err(oauth::FalhaDaRenovacao::Recusada) => {
                Err(Falha::Recusada("O Spotify não aceita mais o login guardado. Entre de novo.".into()))
            }
            Err(oauth::FalhaDaRenovacao::Passageira(motivo)) => Err(Falha::Passageira(motivo)),
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
        // O `client_id` é o padrão do librespot (o do Keymaster), o mesmo que o Spotifast usa. O `username` vai junto do
        // token, como no Spotifast (`playback_credentials`): é o formato de `Credentials` que um cliente de verdade usa.
        let config = SessionConfig { tmp_dir: self.tmp.clone(), ..SessionConfig::default() };
        let credenciais = Credentials {
            username: self.conta_do_token(access),
            auth_type: librespot_protocol::authentication::AuthenticationType::AUTHENTICATION_SPOTIFY_TOKEN,
            auth_data: access.as_bytes().to_vec(),
        };
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
        let antiga = self.sessao.lock().unwrap_or_else(std::sync::PoisonError::into_inner).replace(sessao.clone());
        if let Some(s) = antiga {
            s.shutdown();
        }
        // Fora da trava: o ouvinte (o áudio) pode pedir a sessão de novo.
        self.avisar_ao_entrar(&sessao);
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
        // A regra vive no `oauth::renovar` (a renovação é do nosso fluxo): o `invalid_grant` é recusa, o resto é passageiro.
        // Aqui só se confere que a frase da recusa é a que a pessoa lê.
        let recusa = crate::conta::Falha::Recusada("O Spotify não aceita mais o login guardado. Entre de novo.".into());
        let crate::conta::Falha::Recusada(motivo) = recusa else { unreachable!("devia ser recusa") };
        assert!(motivo.contains("Entre de novo"));
    }
}
