//! O programa do Soloist de verdade: abrir com a chave, achar a porta da API local e fechar junto com o Player.

use super::instalar::{self, Instalado};
use super::{nao_abriu, sem_janela};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// O nome do aparelho no Spotify.
pub const APARELHO: &str = "Player (Soloist)";
const PRAZO_PARA_ABRIR: Duration = Duration::from_secs(20);
/// O cache de áudio do Soloist (dele, cifrado por ele), em MB.
const CACHE_MB: &str = "512";
/// Quantas linhas da saída dele ficam guardadas.
const LINHAS_DA_SAIDA: usize = 20_000;
/// A saída de um Soloist vencido.
const VENCEU: i32 = 10;

/// Por que o Soloist não abriu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Falha {
    Venceu,
    Motivo(String),
}

/// Quem abre o Soloist (os testes usam um falso).
pub trait Programa: Send + Sync + 'static {
    /// Deixa o programa pronto para abrir, baixando se preciso.
    /// `parar`: a pessoa desistiu (as duas voltam logo).
    fn preparar(&self, de_novo: bool, parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<(), String>;
    fn abrir(&self, chave: &str, parar: &AtomicBool) -> Result<Box<dyn Aberto>, Falha>;
    /// Caiu sozinho; devolve se este programa é dos que só caem.
    fn caiu(&self) -> bool;
    /// Tocou por um tempo sem cair.
    fn funcionou(&self);
}

/// Um Soloist aberto; soltar o valor o fecha.
pub trait Aberto: Send {
    /// A porta da API local.
    fn porta(&self) -> u16;
    /// Já fechou: com que código.
    fn saiu(&mut self) -> Option<i32>;
}

pub struct DoSpotify {
    pasta: PathBuf,
    instalado: Mutex<Option<Instalado>>,
}

impl DoSpotify {
    /// `pasta` é a dele, dentro da pasta de dados do provedor.
    pub fn novo(pasta: PathBuf) -> Self {
        Self { pasta, instalado: Mutex::default() }
    }
}

impl DoSpotify {
    fn o_instalado(&self) -> Option<Instalado> {
        self.instalado.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }
}

impl Programa for DoSpotify {
    fn caiu(&self) -> bool {
        self.o_instalado().is_some_and(|i| i.caiu())
    }

    fn funcionou(&self) {
        if let Some(i) = self.o_instalado() {
            i.funcionou();
        }
    }

    fn preparar(&self, de_novo: bool, parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<(), String> {
        if !cfg!(windows) {
            return Err("o Soloist só entra no Player do Windows por enquanto".into());
        }
        let i = instalar::preparar(&self.pasta.join("programa"), de_novo, parar, progresso)?;
        *self.instalado.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(i);
        Ok(())
    }

    fn abrir(&self, chave: &str, parar: &AtomicBool) -> Result<Box<dyn Aberto>, Falha> {
        let exe = self.o_instalado().ok_or_else(|| Falha::Motivo("o Soloist não está instalado".into()))?.exe();
        let dados = self.pasta.join("conta");
        std::fs::create_dir_all(&dados).map_err(|e| Falha::Motivo(format!("a pasta do Soloist não abriu: {e}")))?;
        for sobra in ["ws.addr", "ws.port"] {
            let _ = std::fs::remove_file(dados.join(sobra));
        }
        // (O modo mudo das cópias de teste vale para o Soloist também.)
        let mudo = std::env::var_os("TCCPLAYER_MUDO").is_some_and(|v| v == "1");
        let volume = if mudo { "0" } else { "100" };
        let mut comando = Command::new(exe);
        // A chave só entra pela linha de comando: é o que o Soloist aceita. Ela não vai para registro nenhum.
        comando
            .args(["--device-name", APARELHO, "--api-key", chave, "--ws", "127.0.0.1:0"])
            .args(["--initial-volume", volume, "--cache-size", CACHE_MB])
            .arg("--data-dir")
            .arg(&dados)
            .arg("--cache-dir")
            .arg(self.pasta.join("cache"))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        sem_janela(&mut comando);
        let mut filho = comando.spawn().map_err(|e| Falha::Motivo(nao_abriu(&e)))?;
        let grupo = preso_a_este(&filho);
        // A da abertura anterior fica: é nela que se vê por que caiu.
        let _ = std::fs::rename(self.pasta.join("saida.log"), self.pasta.join("saida-anterior.log"));
        guardar_saida(&self.pasta.join("saida.log"), chave, &mut filho);
        let mut processo = Processo { filho, porta: 0, _grupo: grupo };
        let ate = Instant::now() + PRAZO_PARA_ABRIR;
        loop {
            match processo.saiu() {
                Some(VENCEU) => return Err(Falha::Venceu),
                Some(_) => {
                    return Err(Falha::Motivo(
                        "o Soloist fechou ao abrir. Confira a chave (ela vale para quem a criou, com Premium) e se o Player não está aberto duas vezes".into(),
                    ));
                }
                None => {}
            }
            if let Some(porta) = porta_em(&dados) {
                processo.porta = porta?;
                return Ok(Box::new(processo));
            }
            if parar.load(Ordering::Acquire) {
                return Err(Falha::Motivo("cancelado".into()));
            }
            if Instant::now() > ate {
                return Err(Falha::Motivo("o Soloist demorou demais para abrir".into()));
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

/// A porta que o Soloist escolheu (`None`: ainda não). Só a deste computador.
fn porta_em(dados: &Path) -> Option<Result<u16, Falha>> {
    let porta = std::fs::read_to_string(dados.join("ws.port")).ok()?.trim().parse::<u16>().ok().filter(|p| *p != 0)?;
    let endereco = std::fs::read_to_string(dados.join("ws.addr")).ok()?;
    if endereco.trim().is_empty() {
        return None;
    }
    Some(if endereco.trim() == "127.0.0.1" {
        Ok(porta)
    } else {
        Err(Falha::Motivo("o Soloist abriu a API dele fora deste computador".into()))
    })
}

struct Processo {
    filho: Child,
    porta: u16,
    /// Fecha o Soloist se este programa morrer.
    _grupo: Option<Grupo>,
}

impl Aberto for Processo {
    fn porta(&self) -> u16 {
        self.porta
    }

    fn saiu(&mut self) -> Option<i32> {
        self.filho.try_wait().ok().flatten().map(|s| s.code().unwrap_or(1))
    }
}

impl Drop for Processo {
    fn drop(&mut self) {
        let _ = self.filho.kill();
        let _ = self.filho.wait();
    }
}

/// O que o Soloist escreve fica num arquivo só dele, sem a chave (não se sabe o que mais ele conta: fora do registro do
/// Player). Cada abertura começa de novo.
fn guardar_saida(arquivo: &Path, chave: &str, filho: &mut Child) {
    let saida = std::sync::Arc::new(Mutex::new(std::fs::File::create(arquivo).ok()));
    let saidas: [Option<Box<dyn Read + Send>>; 2] =
        [filho.stdout.take().map(|s| Box::new(s) as _), filho.stderr.take().map(|s| Box::new(s) as _)];
    for lida in saidas.into_iter().flatten() {
        let (saida, chave) = (saida.clone(), chave.to_string());
        let _ = std::thread::Builder::new().name("soloist-saida".into()).spawn(move || {
            for (n, linha) in BufReader::new(lida).split(b'\n').map_while(Result::ok).enumerate() {
                // Depois disto, só lê: parar de ler travaria (ou derrubaria) o Soloist.
                if n >= LINHAS_DA_SAIDA {
                    continue;
                }
                let linha = String::from_utf8_lossy(&linha).replace(&chave, "…");
                if let Some(f) = saida.lock().unwrap_or_else(std::sync::PoisonError::into_inner).as_mut() {
                    let _ = writeln!(f, "{}", linha.trim_end());
                }
            }
        });
    }
}

/// Um grupo do Windows que fecha os processos dele quando este programa acaba, de qualquer jeito.
#[cfg(windows)]
struct Grupo(windows::Win32::Foundation::HANDLE);

// SAFETY: o handle do grupo só é fechado, uma vez, no `Drop`; o Windows aceita isso de qualquer thread.
#[cfg(windows)]
unsafe impl Send for Grupo {}

#[cfg(windows)]
impl Drop for Grupo {
    fn drop(&mut self) {
        // SAFETY: o handle foi criado por este valor e só é fechado aqui.
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.0);
        }
    }
}

#[cfg(windows)]
fn preso_a_este(filho: &Child) -> Option<Grupo> {
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::HANDLE;
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
    };
    let mut limites = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limites.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: o grupo é novo e sem nome; a estrutura passada tem o tamanho informado; o handle do processo é do `Child`
    // vivo.
    let feito = unsafe {
        CreateJobObjectW(None, windows::core::PCWSTR::null()).and_then(|h| {
            let grupo = Grupo(h);
            SetInformationJobObject(
                h,
                JobObjectExtendedLimitInformation,
                (&raw const limites).cast(),
                std::mem::size_of_val(&limites) as u32,
            )?;
            AssignProcessToJobObject(h, HANDLE(filho.as_raw_handle()))?;
            Ok(grupo)
        })
    };
    feito.inspect_err(|e| eprintln!("soloist: fora do grupo deste programa: {e}")).ok()
}

#[cfg(not(windows))]
struct Grupo;

#[cfg(not(windows))]
fn preso_a_este(_: &Child) -> Option<Grupo> {
    None
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn a_porta_so_vale_neste_computador() {
        let p = std::env::temp_dir().join(format!("tccplayer-soloist-porta-{}", std::process::id()));
        std::fs::create_dir_all(&p).expect("pasta");
        assert_eq!(porta_em(&p), None);
        std::fs::write(p.join("ws.port"), "0").expect("porta");
        std::fs::write(p.join("ws.addr"), "127.0.0.1\n").expect("endereço");
        assert_eq!(porta_em(&p), None);
        std::fs::write(p.join("ws.port"), "4123\n").expect("porta");
        assert_eq!(porta_em(&p), Some(Ok(4123)));
        std::fs::write(p.join("ws.addr"), "0.0.0.0").expect("endereço");
        assert!(matches!(porta_em(&p), Some(Err(_))));
        let _ = std::fs::remove_dir_all(&p);
    }

    #[cfg(windows)]
    #[test]
    fn soltar_o_grupo_fecha_o_programa() {
        let mut filho =
            Command::new("ping").args(["-n", "60", "127.0.0.1"]).stdout(Stdio::null()).spawn().expect("ping");
        let grupo = preso_a_este(&filho).expect("grupo");
        assert!(matches!(filho.try_wait(), Ok(None)));
        drop(grupo);
        let ate = Instant::now() + Duration::from_secs(5);
        while matches!(filho.try_wait(), Ok(None)) && Instant::now() < ate {
            std::thread::sleep(Duration::from_millis(20));
        }
        assert!(matches!(filho.try_wait(), Ok(Some(_))), "o grupo fechou e o programa ficou");
    }
}
