//! A conta do Spotify: o estado do login (fora, entrando, dentro) e a credencial guardada.
//!
//! Quem fala com o Spotify é o [`Autorizador`] (o de verdade fica em `spotify.rs`; os testes usam um falso). O
//! trabalho lento (o navegador, a rede) corre numa thread por tentativa: o `servir` do protocolo atende um pedido
//! por vez e não pode esperar a pessoa aprovar. O que fica guardado é só o refresh token do OAuth, cifrado pelo
//! Windows (DPAPI); nunca a senha, que a pessoa digita no navegador.

use provedores::{Conta, SituacaoDaConta};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

/// O que o OAuth devolve.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tokens {
    pub access: String,
    /// O Spotify pode trocar o refresh token a cada renovação; sem um novo, o antigo continua valendo.
    pub refresh: Option<String>,
}

/// A conta, depois de a sessão entrar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DadosDaConta {
    pub nome: String,
    /// `None` quando o Spotify não disse o plano.
    pub premium: Option<bool>,
}

/// O plano da conta pelo atributo `type` da sessão.
pub fn plano(tipo: Option<&str>) -> Option<bool> {
    tipo.map(|t| t == "premium")
}

/// O que dizer quando a sessão anterior derrubou o programa no meio da entrada (ver [`Login::novo`]).
const A_SESSAO_CAIU: &str = "Da última vez, o Spotify derrubou a sessão ao entrar. Este modo precisa de conta Premium. Entre para tentar de novo.";

/// Por que renovar a credencial guardada falhou.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Falha {
    /// O Spotify não aceita mais a credencial (revogada, de outra conta): ela é apagada.
    Recusada(String),
    /// A rede, um prazo: a credencial continua guardada para a próxima tentativa.
    Passageira(String),
}

impl Falha {
    fn motivo(&self) -> &str {
        match self {
            Self::Recusada(m) | Self::Passageira(m) => m,
        }
    }
}

/// Quem fala com o Spotify. Cada método pode demorar (rede, navegador): só é chamado nas threads do [`Login`].
pub trait Autorizador: Send + Sync + 'static {
    /// Abre o navegador e espera a aprovação (até alguns minutos).
    fn autorizar(&self) -> Result<Tokens, String>;
    /// Troca o refresh token guardado por um access token novo.
    fn renovar(&self, refresh: &str) -> Result<Tokens, Falha>;
    /// Entra na sessão com o access token.
    fn conectar(&self, access: &str) -> Result<DadosDaConta, Falha>;
    /// Encerra a sessão, se há uma.
    fn desconectar(&self);
    /// Desiste do navegador que [`Self::autorizar`] está esperando, se está.
    fn cancelar(&self);
    /// `Some(frase)` quando a porta do retorno do login está ocupada (a frase diz por quem).
    fn porta_ocupada(&self) -> Option<String>;
}

/// O que ainda não foi aprovado: o navegador ou a credencial guardada.
enum Tentativa {
    Autorizar,
    Renovar(String),
}

#[derive(Default)]
struct Estado {
    conta: Conta,
    access: Option<String>,
    dados: Option<DadosDaConta>,
    /// Sobe a cada tentativa e a cada `sair`: o resultado de uma tentativa velha é ignorado.
    geracao: u64,
    /// Uma reentrada ([`Login::reconectar`]) está em curso, e quando a última começou.
    reentrando: bool,
    ultima_reentrada: Option<Instant>,
}

/// Quanto esperar entre uma reentrada e outra: uma que não resolveu não se repete a cada pedido.
const ENTRE_REENTRADAS: Duration = Duration::from_secs(30);
const A_SESSAO_ESTA_VOLTANDO: &str = "A sessão do Spotify está voltando. Tente de novo daqui a pouco.";

pub struct Login {
    autorizador: Arc<dyn Autorizador>,
    cofre: PathBuf,
    estado: Arc<Mutex<Estado>>,
    /// Só uma tentativa por vez entra na sessão: uma tentativa esquecida (por [`Login::sair`]) encerra a sessão dela
    /// antes de outra entrar, e as duas não disputam a marca.
    na_sessao: Arc<Mutex<()>>,
}

/// Onde fica a marca de "entrando na sessão", ao lado da credencial.
fn marca_de(cofre: &std::path::Path) -> PathBuf {
    cofre.with_file_name("conectando")
}

fn travar(m: &Mutex<Estado>) -> MutexGuard<'_, Estado> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl Login {
    /// Com uma credencial guardada em `cofre`, já começa entrando (renovando numa thread).
    ///
    /// O librespot fecha o programa quando a conta não é Premium. Uma marca no disco cerca a entrada na sessão: se
    /// ela sobrou da abertura anterior, não se tenta sozinho (o Player reabriria o provedor, que cairia de novo), e a
    /// credencial sai: o próximo `entrar` é pelo navegador, onde dá para usar outra conta.
    pub fn novo(autorizador: Arc<dyn Autorizador>, cofre: PathBuf) -> Self {
        let login = Self { autorizador, cofre, estado: Arc::default(), na_sessao: Arc::default() };
        if std::fs::remove_file(login.marca()).is_ok() {
            servicos::cofre::apagar(&login.cofre);
            travar(&login.estado).conta = Conta { erro: Some(A_SESSAO_CAIU.into()), ..Conta::default() };
            return login;
        }
        if let Some(refresh) = servicos::cofre::ler(&login.cofre) {
            login.comecar(Tentativa::Renovar(refresh));
        }
        login
    }

    fn marca(&self) -> PathBuf {
        marca_de(&self.cofre)
    }

    pub fn estado(&self) -> Conta {
        travar(&self.estado).conta.clone()
    }

    /// O access token da sessão de agora (para os pedidos das próximas partes).
    #[cfg_attr(not(test), expect(dead_code, reason = "a parte 2 (os dados) usa o token"))]
    pub fn access(&self) -> Option<String> {
        travar(&self.estado).access.clone()
    }

    /// A conta que entrou (nome e Premium), enquanto dentro.
    #[expect(dead_code, reason = "a parte 3 (tocar) usa o Premium daqui")]
    pub fn dados(&self) -> Option<DadosDaConta> {
        travar(&self.estado).dados.clone()
    }

    /// Começa o login. Com a credencial guardada, renova (sem navegador); senão abre o navegador, se a porta do
    /// retorno estiver livre. Já entrando ou dentro, não faz nada.
    pub fn entrar(&self) -> Result<(), String> {
        if travar(&self.estado).conta.situacao != SituacaoDaConta::Fora {
            return Ok(());
        }
        if let Some(refresh) = servicos::cofre::ler(&self.cofre) {
            self.comecar(Tentativa::Renovar(refresh));
            return Ok(());
        }
        if let Some(frase) = self.autorizador.porta_ocupada() {
            travar(&self.estado).conta = Conta { erro: Some(frase.clone()), ..Conta::default() };
            return Err(frase);
        }
        self.comecar(Tentativa::Autorizar);
        Ok(())
    }

    /// A sessão caiu com a conta dentro: renova a credencial e entra de novo, em segundo plano, e espera isso por até
    /// `espera` (a thread do protocolo não pode ficar presa). Sem rede, a conta continua dentro; recusada, sai com o motivo.
    pub fn reconectar(&self, espera: Duration) -> Result<(), String> {
        let (geracao, premium) = {
            let mut e = travar(&self.estado);
            if e.conta.situacao != SituacaoDaConta::Dentro {
                return Err("Entre na sua conta do Spotify.".into());
            }
            if e.reentrando || e.ultima_reentrada.is_some_and(|u| u.elapsed() < ENTRE_REENTRADAS) {
                return Err(A_SESSAO_ESTA_VOLTANDO.into());
            }
            e.reentrando = true;
            e.ultima_reentrada = Some(Instant::now());
            (e.geracao, e.conta.premium == Some(true))
        };
        let (autorizador, estado) = (self.autorizador.clone(), self.estado.clone());
        let (cofre, na_sessao) = (self.cofre.clone(), self.na_sessao.clone());
        let (avisar, resultado) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let fim = FimDaReentrada(estado.clone());
            let r = reentrar(&*autorizador, geracao, premium, &estado, &cofre, &na_sessao);
            drop(fim);
            let _ = avisar.send(r);
        });
        resultado.recv_timeout(espera).unwrap_or_else(|_| Err(A_SESSAO_ESTA_VOLTANDO.into()))
    }

    /// O Player vai fechar: a marca de "entrando na sessão" não fica para a próxima abertura.
    pub fn encerrar(&self) {
        // A geração anda com o estado travado: uma tentativa no meio não põe a marca de volta.
        let mut e = travar(&self.estado);
        e.geracao += 1;
        let _ = std::fs::remove_file(self.marca());
    }

    /// Sai da conta, encerra a sessão e apaga a credencial. Uma tentativa no meio do caminho é esquecida (e a espera
    /// do navegador acaba): ela não guarda nada nem fica com a sessão.
    pub fn sair(&self) {
        {
            let mut e = travar(&self.estado);
            e.geracao += 1;
            e.conta = Conta::default();
            e.access = None;
            e.dados = None;
            servicos::cofre::apagar(&self.cofre);
        }
        self.autorizador.cancelar();
        self.autorizador.desconectar();
    }

    fn comecar(&self, tentativa: Tentativa) {
        let geracao = {
            let mut e = travar(&self.estado);
            e.geracao += 1;
            e.ultima_reentrada = None;
            e.conta = Conta {
                situacao: SituacaoDaConta::Entrando,
                navegador: matches!(tentativa, Tentativa::Autorizar),
                ..Conta::default()
            };
            e.geracao
        };
        let autorizador = self.autorizador.clone();
        let estado = self.estado.clone();
        let cofre = self.cofre.clone();
        let na_sessao = self.na_sessao.clone();
        std::thread::spawn(move || {
            let resultado = tentar(&*autorizador, tentativa, true, geracao, &estado, &cofre, &na_sessao);
            let mut e = travar(&estado);
            if e.geracao != geracao {
                return;
            }
            match resultado {
                Ok((access, dados)) => {
                    e.conta = Conta {
                        situacao: SituacaoDaConta::Dentro,
                        nome: Some(dados.nome.clone()),
                        premium: dados.premium,
                        ..Conta::default()
                    };
                    e.access = Some(access);
                    e.dados = Some(dados);
                }
                Err(falha) => {
                    if matches!(falha, Falha::Recusada(_)) {
                        servicos::cofre::apagar(&cofre);
                    }
                    e.conta = Conta { erro: Some(falha.motivo().to_string()), ..Conta::default() };
                }
            }
        });
    }
}

/// Marca o fim da reentrada mesmo se a thread dela parar no meio.
struct FimDaReentrada(Arc<Mutex<Estado>>);

impl Drop for FimDaReentrada {
    fn drop(&mut self) {
        travar(&self.0).reentrando = false;
    }
}

/// Renova e entra de novo na sessão (ver [`Login::reconectar`]).
fn reentrar(
    autorizador: &dyn Autorizador,
    geracao: u64,
    premium: bool,
    estado: &Mutex<Estado>,
    cofre: &std::path::Path,
    na_sessao: &Mutex<()>,
) -> Result<(), String> {
    // A conta já entrou como Premium nesta abertura: sem a marca (fechar o Player aqui não pode apagar o login).
    let marcar = !premium;
    let resultado = match servicos::cofre::ler(cofre) {
        Some(refresh) => tentar(autorizador, Tentativa::Renovar(refresh), marcar, geracao, estado, cofre, na_sessao),
        None => Err(Falha::Recusada("O login do Spotify não está mais guardado. Entre de novo.".into())),
    };
    let mut e = travar(estado);
    if e.geracao != geracao {
        return Err("A conta mudou no meio do pedido.".into());
    }
    match resultado {
        Ok((access, dados)) => {
            e.access = Some(access);
            e.dados = Some(dados);
            Ok(())
        }
        Err(falha) => {
            let motivo = falha.motivo().to_string();
            if matches!(falha, Falha::Recusada(_)) {
                servicos::cofre::apagar(cofre);
                e.geracao += 1;
                e.conta = Conta { erro: Some(motivo.clone()), ..Conta::default() };
                e.access = None;
                e.dados = None;
                drop(e);
                autorizador.desconectar();
            }
            Err(motivo)
        }
    }
}

/// Uma tentativa inteira: aprovar ou renovar, guardar e entrar na sessão. Entre um passo e outro, confere se ela
/// ainda vale (a pessoa pode ter saído). O erro de uma tentativa esquecida não aparece para ninguém.
fn tentar(
    autorizador: &dyn Autorizador,
    tentativa: Tentativa,
    marcar: bool,
    geracao: u64,
    estado: &Mutex<Estado>,
    cofre: &std::path::Path,
    na_sessao: &Mutex<()>,
) -> Result<(String, DadosDaConta), Falha> {
    let esquecida = || Falha::Passageira("a tentativa foi cancelada".into());
    let tokens = match tentativa {
        Tentativa::Autorizar => autorizador.autorizar().map_err(Falha::Passageira),
        Tentativa::Renovar(refresh) => autorizador.renovar(&refresh),
    }?;
    let _vez = na_sessao.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let marca = marca_de(cofre);
    {
        // Guardar e pôr a marca com o estado travado: `sair` e `encerrar` não entram no meio.
        let e = travar(estado);
        if e.geracao != geracao {
            return Err(esquecida());
        }
        if let Some(refresh) = &tokens.refresh
            && let Err(e) = servicos::cofre::guardar(cofre, refresh)
        {
            // Sem guardar, a sessão de agora entra; só a próxima abertura vai pedir o navegador de novo.
            eprintln!("spotify: a credencial não foi guardada: {e}");
        }
        if marcar {
            let _ = std::fs::write(&marca, "");
        }
    }
    let dados = autorizador.conectar(&tokens.access);
    if marcar {
        let _ = std::fs::remove_file(&marca);
    }
    if travar(estado).geracao != geracao {
        // Saíram durante a entrada: a sessão que chegou depois não fica aberta.
        autorizador.desconectar();
        return Err(esquecida());
    }
    dados.map(|d| (tokens.access, d))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) mod falso {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// Um Spotify de mentira, com as respostas escolhidas pelo teste.
    pub(crate) struct Falso {
        pub porta: Option<String>,
        pub autorizar: Result<Tokens, String>,
        pub renovar: Result<Tokens, Falha>,
        pub conectar: Result<DadosDaConta, Falha>,
        pub demora_ao_renovar: Duration,
        pub demora_ao_conectar: Duration,
        pub autorizacoes: AtomicUsize,
        pub renovacoes: AtomicUsize,
        pub conexoes: AtomicUsize,
        pub desconexoes: AtomicUsize,
        pub cancelamentos: AtomicUsize,
    }

    impl Default for Falso {
        fn default() -> Self {
            Self {
                porta: None,
                autorizar: Ok(Tokens { access: "a".into(), refresh: Some("r".into()) }),
                renovar: Ok(Tokens { access: "a2".into(), refresh: None }),
                conectar: Ok(DadosDaConta { nome: "rafael".into(), premium: Some(true) }),
                demora_ao_renovar: Duration::ZERO,
                demora_ao_conectar: Duration::ZERO,
                autorizacoes: AtomicUsize::new(0),
                renovacoes: AtomicUsize::new(0),
                conexoes: AtomicUsize::new(0),
                desconexoes: AtomicUsize::new(0),
                cancelamentos: AtomicUsize::new(0),
            }
        }
    }

    impl Autorizador for Falso {
        fn autorizar(&self) -> Result<Tokens, String> {
            self.autorizacoes.fetch_add(1, Ordering::SeqCst);
            // Um pouco de demora, como o navegador: o segundo `entrar` chega com a tentativa no meio.
            std::thread::sleep(std::time::Duration::from_millis(100));
            self.autorizar.clone()
        }
        fn renovar(&self, _: &str) -> Result<Tokens, Falha> {
            self.renovacoes.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(self.demora_ao_renovar);
            self.renovar.clone()
        }
        fn conectar(&self, _: &str) -> Result<DadosDaConta, Falha> {
            self.conexoes.fetch_add(1, Ordering::SeqCst);
            std::thread::sleep(self.demora_ao_conectar);
            self.conectar.clone()
        }
        fn desconectar(&self) {
            self.desconexoes.fetch_add(1, Ordering::SeqCst);
        }
        fn cancelar(&self) {
            self.cancelamentos.fetch_add(1, Ordering::SeqCst);
        }
        fn porta_ocupada(&self) -> Option<String> {
            self.porta.clone()
        }
    }

    /// Uma pasta só deste teste, com o caminho do cofre dentro.
    pub(crate) fn cofre(nome: &str) -> PathBuf {
        let pasta = std::env::temp_dir().join(format!("provedor-spotify-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        pasta.join("conta.dpapi")
    }

    /// Espera até `cond` valer (no máximo 2 s).
    pub(crate) fn esperar(cond: impl Fn() -> bool) -> bool {
        let fim = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while std::time::Instant::now() < fim {
            if cond() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        cond()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::falso::{Falso, cofre, esperar};
    use super::*;
    use std::sync::atomic::Ordering;

    fn login(falso: Falso, cofre: PathBuf) -> (Login, Arc<Falso>) {
        let falso = Arc::new(falso);
        (Login::novo(falso.clone(), cofre), falso)
    }

    #[test]
    fn entrar_com_a_porta_ocupada_erra_na_hora() {
        let frase = "a porta 8898 está com o Spotifast.exe".to_string();
        let (l, f) = login(Falso { porta: Some(frase.clone()), ..Falso::default() }, cofre("porta"));
        assert_eq!(l.entrar(), Err(frase.clone()));
        let c = l.estado();
        assert_eq!(c.situacao, SituacaoDaConta::Fora, "não fica preso em entrando");
        assert_eq!(c.erro, Some(frase));
        assert_eq!(f.autorizacoes.load(Ordering::SeqCst), 0, "o navegador nem abre");
    }

    #[test]
    fn entrar_e_aprovar_fica_dentro_e_guarda() {
        let caminho = cofre("aprovar");
        let (l, _) = login(Falso::default(), caminho.clone());
        assert_eq!(l.estado().situacao, SituacaoDaConta::Fora);
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Dentro), "{:?}", l.estado());
        let c = l.estado();
        assert_eq!((c.nome.as_deref(), c.premium, c.erro), (Some("rafael"), Some(true), None));
        assert_eq!(l.access().as_deref(), Some("a"));
        if cfg!(windows) {
            assert_eq!(servicos::cofre::ler(&caminho).as_deref(), Some("r"), "o refresh token fica cifrado");
        }
    }

    #[test]
    fn navegador_fechado_volta_para_fora() {
        let (l, _) = login(Falso { autorizar: Err("desistiu".into()), ..Falso::default() }, cofre("desistiu"));
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Fora && l.estado().erro.is_some()));
        assert_eq!(l.estado().erro.as_deref(), Some("desistiu"));
    }

    #[test]
    fn com_a_credencial_guardada_entra_sozinho() {
        if !cfg!(windows) {
            return;
        }
        let caminho = cofre("guardada");
        servicos::cofre::guardar(&caminho, "velho").expect("guardar");
        let (l, f) = login(Falso::default(), caminho);
        assert_eq!(l.estado().situacao, SituacaoDaConta::Entrando, "já começa renovando");
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Dentro));
        assert_eq!(f.autorizacoes.load(Ordering::SeqCst), 0, "sem navegador");
        assert_eq!(l.access().as_deref(), Some("a2"));
    }

    #[test]
    fn refresh_recusado_apaga_e_fica_fora() {
        if !cfg!(windows) {
            return;
        }
        let caminho = cofre("recusado");
        servicos::cofre::guardar(&caminho, "velho").expect("guardar");
        let falso = Falso { renovar: Err(Falha::Recusada("revogado".into())), ..Falso::default() };
        let (l, f) = login(falso, caminho.clone());
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Fora));
        assert!(!caminho.exists(), "a credencial recusada é apagada");
        assert_eq!(l.estado().erro.as_deref(), Some("revogado"));
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(f.renovacoes.load(Ordering::SeqCst), 1, "sem laço");
    }

    #[test]
    fn refresh_sem_rede_guarda_e_tenta_de_novo_ao_entrar() {
        if !cfg!(windows) {
            return;
        }
        let caminho = cofre("sem-rede");
        servicos::cofre::guardar(&caminho, "velho").expect("guardar");
        let falso = Falso { renovar: Err(Falha::Passageira("sem conexão".into())), ..Falso::default() };
        let (l, f) = login(falso, caminho.clone());
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Fora));
        assert!(caminho.exists(), "a credencial continua");
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| f.renovacoes.load(Ordering::SeqCst) == 2), "entrar renova de novo, sem navegador");
        assert_eq!(f.autorizacoes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn entrar_duas_vezes_nao_abre_dois_navegadores() {
        let (l, f) = login(Falso::default(), cofre("duas"));
        assert_eq!(l.entrar(), Ok(()));
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Dentro));
        assert_eq!(f.autorizacoes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn se_a_ultima_sessao_derrubou_o_programa_nao_tenta_sozinho() {
        if !cfg!(windows) {
            return;
        }
        // O librespot fecha o programa quando a conta não é Premium: a marca que ficou conta isso à abertura seguinte.
        let caminho = cofre("derrubou");
        servicos::cofre::guardar(&caminho, "velho").expect("guardar");
        let marca = caminho.with_file_name("conectando");
        std::fs::write(&marca, "").expect("marca");
        let (l, f) = login(Falso::default(), caminho.clone());
        let c = l.estado();
        assert_eq!(c.situacao, SituacaoDaConta::Fora);
        assert!(c.erro.as_deref().is_some_and(|e| e.contains("Premium")), "{:?}", c.erro);
        assert_eq!(f.renovacoes.load(Ordering::SeqCst), 0, "sem laço de abrir e cair");
        assert!(!marca.exists(), "a marca some (a pessoa pode tentar de novo)");
        assert!(!caminho.exists(), "a credencial que derruba o programa sai: dá para entrar com outra conta");
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Dentro), "tentar de novo funciona");
        assert_eq!(f.autorizacoes.load(Ordering::SeqCst), 1, "pelo navegador");
        assert!(!marca.exists(), "depois de entrar, a marca some");
    }

    #[test]
    fn sessao_recusada_apaga_a_credencial_e_volta_ao_navegador() {
        if !cfg!(windows) {
            return;
        }
        let caminho = cofre("sessao-recusada");
        let falso = Falso { conectar: Err(Falha::Recusada("a conta não entrou".into())), ..Falso::default() };
        let (l, f) = login(falso, caminho.clone());
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().erro.is_some()));
        assert!(!caminho.exists(), "a credencial recusada não fica");
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| f.autorizacoes.load(Ordering::SeqCst) == 2), "de novo pelo navegador");
        assert_eq!(f.renovacoes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn sessao_sem_rede_guarda_a_credencial() {
        if !cfg!(windows) {
            return;
        }
        let caminho = cofre("sessao-sem-rede");
        let falso = Falso { conectar: Err(Falha::Passageira("sem conexão".into())), ..Falso::default() };
        let (l, _) = login(falso, caminho.clone());
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().erro.is_some()));
        assert_eq!(servicos::cofre::ler(&caminho).as_deref(), Some("r"), "sem rede, o login aprovado não se perde");
    }

    #[test]
    fn sair_com_o_navegador_aberto_esquece_a_tentativa() {
        let caminho = cofre("sair-navegador");
        let (l, f) = login(Falso::default(), caminho.clone());
        assert_eq!(l.entrar(), Ok(()));
        assert!(l.estado().navegador, "a tela sabe que é o navegador");
        l.sair();
        assert_eq!(f.cancelamentos.load(Ordering::SeqCst), 1, "a espera do navegador acaba");
        std::thread::sleep(Duration::from_millis(250));
        assert_eq!(l.estado(), Conta::default());
        assert!(!caminho.exists(), "a tentativa esquecida não guarda a credencial");
        assert_eq!(f.conexoes.load(Ordering::SeqCst), 0, "nem entra na sessão");
    }

    #[test]
    fn sair_no_meio_da_conexao_encerra_a_sessao_que_chega_depois() {
        let caminho = cofre("sair-conexao");
        let falso = Falso { demora_ao_conectar: Duration::from_millis(200), ..Falso::default() };
        let (l, f) = login(falso, caminho.clone());
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| f.conexoes.load(Ordering::SeqCst) == 1));
        l.sair();
        let ao_sair = f.desconexoes.load(Ordering::SeqCst);
        assert!(esperar(|| f.desconexoes.load(Ordering::SeqCst) > ao_sair), "a sessão atrasada é encerrada");
        assert_eq!(l.estado(), Conta::default());
        assert!(!caminho.exists());
    }

    #[test]
    fn sair_de_dentro_encerra_a_sessao() {
        let (l, f) = login(Falso::default(), cofre("sair-dentro"));
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Dentro));
        l.sair();
        assert_eq!(f.desconexoes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn uma_recusa_atrasada_nao_apaga_a_credencial_nova() {
        if !cfg!(windows) {
            return;
        }
        let caminho = cofre("recusa-atrasada");
        servicos::cofre::guardar(&caminho, "velho").expect("guardar");
        let falso = Falso {
            renovar: Err(Falha::Recusada("revogado".into())),
            demora_ao_renovar: Duration::from_millis(200),
            ..Falso::default()
        };
        let (l, _) = login(falso, caminho.clone());
        assert!(!l.estado().navegador, "renovar não abre o navegador");
        l.sair();
        servicos::cofre::guardar(&caminho, "novo").expect("guardar");
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(servicos::cofre::ler(&caminho).as_deref(), Some("novo"));
    }

    fn dentro(teste: &str, falso: Falso) -> (Arc<Login>, Arc<Falso>, PathBuf) {
        let caminho = cofre(teste);
        let (l, f) = login(falso, caminho.clone());
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Dentro));
        (Arc::new(l), f, caminho)
    }

    #[test]
    fn reconectar_troca_o_portador_e_continua_dentro() {
        if !cfg!(windows) {
            return;
        }
        let (l, f, _) = dentro("reconectar", Falso::default());
        assert_eq!(l.reconectar(Duration::from_secs(5)), Ok(()));
        assert_eq!(l.estado().situacao, SituacaoDaConta::Dentro);
        assert_eq!(l.access().as_deref(), Some("a2"));
        assert_eq!((f.renovacoes.load(Ordering::SeqCst), f.conexoes.load(Ordering::SeqCst)), (1, 2));
        assert_eq!(f.autorizacoes.load(Ordering::SeqCst), 1, "sem navegador");
    }

    #[test]
    fn reconectar_recusado_sai_com_o_motivo() {
        if !cfg!(windows) {
            return;
        }
        let falso = Falso { renovar: Err(Falha::Recusada("revogado".into())), ..Falso::default() };
        let (l, _, caminho) = dentro("reconectar-recusado", falso);
        assert!(l.reconectar(Duration::from_secs(5)).is_err());
        let c = l.estado();
        assert_eq!((c.situacao, c.erro.as_deref()), (SituacaoDaConta::Fora, Some("revogado")));
        assert!(!caminho.exists() && l.access().is_none());
    }

    #[test]
    fn reconectar_sem_rede_continua_dentro() {
        if !cfg!(windows) {
            return;
        }
        let falso = Falso { renovar: Err(Falha::Passageira("sem conexão".into())), ..Falso::default() };
        let (l, _, caminho) = dentro("reconectar-sem-rede", falso);
        assert_eq!(l.reconectar(Duration::from_secs(5)), Err("sem conexão".into()));
        assert_eq!(l.estado().situacao, SituacaoDaConta::Dentro);
        assert!(caminho.exists(), "o login não se perde por causa da rede");
    }

    #[test]
    fn reconectar_conta_premium_nao_poe_a_marca() {
        if !cfg!(windows) {
            return;
        }
        let falso = Falso { demora_ao_conectar: Duration::from_millis(200), ..Falso::default() };
        let (l, f, caminho) = dentro("reconectar-marca", falso);
        let outra = l.clone();
        let thread = std::thread::spawn(move || outra.reconectar(Duration::from_secs(5)));
        assert!(esperar(|| f.conexoes.load(Ordering::SeqCst) == 2));
        assert!(!marca_de(&caminho).exists(), "fechar o Player aqui não pode apagar o login");
        assert_eq!(thread.join().expect("thread"), Ok(()));
    }

    #[test]
    fn reconectar_lento_nao_prende_quem_pede() {
        if !cfg!(windows) {
            return;
        }
        let falso = Falso { demora_ao_renovar: Duration::from_millis(400), ..Falso::default() };
        let (l, f, _) = dentro("reconectar-lento", falso);
        let antes = Instant::now();
        assert!(l.reconectar(Duration::from_millis(50)).is_err_and(|e| e.contains("voltando")));
        assert!(l.reconectar(Duration::from_millis(50)).is_err(), "uma de cada vez");
        assert!(antes.elapsed() < Duration::from_millis(300));
        assert!(esperar(|| l.access().as_deref() == Some("a2")), "volta em segundo plano");
        assert_eq!(f.renovacoes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn reconectar_recusado_encerra_a_sessao() {
        if !cfg!(windows) {
            return;
        }
        let falso = Falso { renovar: Err(Falha::Recusada("revogado".into())), ..Falso::default() };
        let (l, f, _) = dentro("reconectar-encerra", falso);
        assert!(l.reconectar(Duration::from_secs(5)).is_err());
        assert_eq!(f.desconexoes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn reconectar_fora_da_conta_nao_faz_nada() {
        let (l, f) = login(Falso::default(), cofre("reconectar-fora"));
        assert!(l.reconectar(Duration::from_secs(5)).is_err());
        assert_eq!(f.renovacoes.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn sair_durante_o_reconectar_nao_volta() {
        if !cfg!(windows) {
            return;
        }
        let falso = Falso { demora_ao_renovar: Duration::from_millis(200), ..Falso::default() };
        let (l, f, caminho) = dentro("reconectar-sair", falso);
        let outra = l.clone();
        let thread = std::thread::spawn(move || outra.reconectar(Duration::from_secs(5)));
        assert!(esperar(|| f.renovacoes.load(Ordering::SeqCst) == 1));
        l.sair();
        assert!(thread.join().expect("thread").is_err());
        assert_eq!(l.estado(), Conta::default());
        assert!(!caminho.exists() && l.access().is_none());
    }

    #[test]
    fn fechar_o_player_no_meio_da_conexao_nao_deixa_a_marca() {
        let caminho = cofre("encerrar");
        let falso = Falso { demora_ao_conectar: Duration::from_millis(200), ..Falso::default() };
        let (l, f) = login(falso, caminho.clone());
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| f.conexoes.load(Ordering::SeqCst) == 1));
        l.encerrar();
        assert!(!marca_de(&caminho).exists(), "a próxima abertura não acusa a conta de não ser Premium");
    }

    #[test]
    fn plano_so_diz_premium_quando_o_spotify_disse() {
        assert_eq!(plano(Some("premium")), Some(true));
        assert_eq!(plano(Some("free")), Some(false));
        assert_eq!(plano(None), None, "ainda não chegou: não inventa gratuita");
    }

    #[test]
    fn sair_apaga_o_cofre() {
        let caminho = cofre("sair");
        let (l, _) = login(Falso::default(), caminho.clone());
        assert_eq!(l.entrar(), Ok(()));
        assert!(esperar(|| l.estado().situacao == SituacaoDaConta::Dentro));
        l.sair();
        assert_eq!(l.estado(), Conta::default());
        assert_eq!(l.access(), None);
        assert!(!caminho.exists());
    }
}
