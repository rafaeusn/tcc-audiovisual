//! O programa do Soloist: baixado do Spotify (nunca redistribuído) quando a pessoa o liga, e trocado antes de vencer (90
//! dias depois de compilado). Como o Spotifast (MIT) faz.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SERVIDOR: &str = "soloist-builds.spotifycdn.com";
const PACOTE: &str = "soloist_release_windows_x86_64.zip";
/// O programa.
pub const EXE: &str = "soloist.exe";
/// O que se aproveita do pacote (o resto é ignorado).
const DO_PACOTE: [&str; 3] = [EXE, "THIRD_PARTY_LICENSES.txt", "CHANGELOG.md"];
const MAXIMO_DO_PACOTE: u64 = 80 * 1024 * 1024;
const MAXIMO_ABERTO: u64 = 160 * 1024 * 1024;
const MAXIMO_DE_ITENS: usize = 64;
const DIA: u64 = 86_400;
/// Com quantos dias o programa vence, e com quantos se procura um novo (uma vez por dia).
const VENCE: u64 = 90 * DIA;
const PROCURAR_OUTRO: u64 = 60 * DIA;
/// Quantas quedas, sem nunca ter funcionado, fazem um Soloist quebrado.
const QUEDAS: u32 = 2;
const PRAZO_DO_DOWNLOAD: Duration = Duration::from_secs(180);

pub fn agora_s() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// Um Soloist instalado: a pasta `<raiz>/<quando foi compilado>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Instalado {
    pub pasta: PathBuf,
    compilado: u64,
}

impl Instalado {
    pub fn exe(&self) -> PathBuf {
        self.pasta.join(EXE)
    }

    pub fn vencido(&self, agora: u64) -> bool {
        agora >= self.compilado.saturating_add(VENCE)
    }

    /// Caiu sozinho antes de funcionar uma vez neste computador: na segunda, é um programa quebrado (o Spotify já publicou
    /// alguns) e não é mais aberto. Devolve se quebrou.
    pub fn caiu(&self) -> bool {
        if self.pasta.join("funciona").is_file() {
            return false;
        }
        let quedas = self.quedas() + 1;
        let _ = std::fs::write(self.pasta.join("quedas"), quedas.to_string());
        quedas >= QUEDAS
    }

    /// Tocou de verdade: uma queda depois disto não é do programa.
    pub fn funcionou(&self) {
        let _ = std::fs::write(self.pasta.join("funciona"), "");
    }

    fn quedas(&self) -> u32 {
        std::fs::read_to_string(self.pasta.join("quedas")).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0)
    }

    fn quebrado(&self) -> bool {
        self.quedas() >= QUEDAS && !self.pasta.join("funciona").is_file()
    }

    fn vale_procurar_outro(&self, agora: u64, procurado: Option<u64>) -> bool {
        (self.quebrado() || agora >= self.compilado.saturating_add(PROCURAR_OUTRO))
            && procurado.is_none_or(|p| agora >= p.saturating_add(DIA) || p > agora)
    }
}

/// O mais novo instalado em `raiz`.
pub fn instalado(raiz: &Path) -> Option<Instalado> {
    let pastas = std::fs::read_dir(raiz).ok()?;
    pastas
        .filter_map(Result::ok)
        .filter_map(|e| {
            let compilado = e.file_name().to_str()?.parse::<u64>().ok()?;
            let i = Instalado { pasta: e.path(), compilado };
            i.exe().is_file().then_some(i)
        })
        .max_by_key(|i| i.compilado)
}

/// Deixa um Soloist que valha em `raiz`, baixando se preciso (`de_novo`: o de agora venceu, baixa de qualquer jeito).
pub fn preparar(raiz: &Path, de_novo: bool, parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<Instalado, String> {
    let agora = agora_s();
    let marca = raiz.join("procurado");
    let procurado = std::fs::read_to_string(&marca).ok().and_then(|t| t.trim().parse::<u64>().ok());
    let atual = instalado(raiz).filter(|i| !i.vencido(agora) && !de_novo);
    if let Some(i) = &atual
        && !i.vale_procurar_outro(agora, procurado)
    {
        return inteiro(i.clone());
    }
    let _ = std::fs::create_dir_all(raiz);
    let _ = std::fs::write(&marca, agora.to_string());
    match (baixar(raiz, parar, progresso), atual) {
        (Ok(novo), _) => {
            limpar(raiz, &novo);
            inteiro(novo)
        }
        // O de agora ainda vale: fica, e amanhã se procura de novo.
        (Err(e), Some(i)) => {
            eprintln!("soloist: não deu para atualizar ({e}); fica o instalado");
            inteiro(i)
        }
        (Err(e), None) => Err(e),
    }
}

/// O instalado, se não for um que quebra aqui.
fn inteiro(i: Instalado) -> Result<Instalado, String> {
    if i.quebrado() { Err(super::QUEBRADO.into()) } else { Ok(i) }
}

/// Tira os outros instalados e as sobras de um download interrompido.
fn limpar(raiz: &Path, fica: &Instalado) {
    let Ok(pastas) = std::fs::read_dir(raiz) else { return };
    for e in pastas.filter_map(Result::ok) {
        let nome = e.file_name().to_string_lossy().into_owned();
        let nossa = nome.parse::<u64>().is_ok() || nome.starts_with("baixando-");
        if nossa && e.path() != fica.pasta && e.file_type().is_ok_and(|t| t.is_dir()) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

fn baixar(raiz: &Path, parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<Instalado, String> {
    let bytes = baixar_pacote(parar, progresso)?;
    instalar(raiz, &bytes, agora_s(), &compilado_de)
}

/// Abre o pacote numa pasta provisória, confere o programa e o põe no lugar.
fn instalar(
    raiz: &Path,
    bytes: &[u8],
    agora: u64,
    compilado_de: &dyn Fn(&Path) -> Result<u64, String>,
) -> Result<Instalado, String> {
    let provisoria = raiz.join(format!("baixando-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&provisoria);
    let feito = abrir_pacote(bytes, &provisoria).and_then(|()| {
        let compilado = compilado_de(&provisoria.join(EXE))?;
        let novo = Instalado { pasta: raiz.join(compilado.to_string()), compilado };
        if novo.vencido(agora) {
            return Err("o Soloist que o Spotify publica hoje já venceu (confira a data do computador)".into());
        }
        if !novo.exe().is_file() {
            let _ = std::fs::remove_dir_all(&novo.pasta);
            std::fs::rename(&provisoria, &novo.pasta).map_err(|e| format!("não deu para instalar o Soloist: {e}"))?;
        }
        Ok(novo)
    });
    let _ = std::fs::remove_dir_all(&provisoria);
    feito
}

/// Baixa o pacote do servidor do Spotify (só dele, sem redirecionar).
fn baixar_pacote(parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<Vec<u8>, String> {
    let nao_baixou = |e: reqwest::Error| format!("não deu para baixar o Soloist do Spotify: {}", e.without_url());
    let execucao = tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|e| e.to_string())?;
    let cancelado = async {
        while !parar.load(Ordering::Acquire) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err("cancelado".to_string())
    };
    let baixado = async {
        let cliente = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .https_only(true)
            .timeout(PRAZO_DO_DOWNLOAD)
            .build()
            .map_err(nao_baixou)?;
        let pedido = cliente.get(format!("https://{SERVIDOR}/{PACOTE}")).send().await;
        let mut resposta = pedido.and_then(reqwest::Response::error_for_status).map_err(nao_baixou)?;
        if !resposta.status().is_success() {
            return Err(format!("o servidor do Soloist respondeu {}", resposta.status().as_u16()));
        }
        let total = resposta.content_length().unwrap_or(0);
        let grande = || "o pacote do Soloist é grande demais".to_string();
        if total > MAXIMO_DO_PACOTE {
            return Err(grande());
        }
        let mut bytes = Vec::with_capacity(total as usize);
        let mut avisado = Instant::now();
        progresso(0.0);
        while let Some(pedaco) = resposta.chunk().await.map_err(nao_baixou)? {
            if (bytes.len() + pedaco.len()) as u64 > MAXIMO_DO_PACOTE {
                return Err(grande());
            }
            bytes.extend_from_slice(&pedaco);
            if total > 0 && avisado.elapsed() >= Duration::from_millis(150) {
                avisado = Instant::now();
                progresso(bytes.len() as f32 / total as f32);
            }
        }
        Ok(bytes)
    };
    execucao.block_on(async {
        tokio::select! { r = baixado => r, c = cancelado => c }
    })
}

/// O nome com que um item do pacote fica (`None`: não interessa). Caminho que sai da pasta é pacote estragado.
fn nome_aceito(item: &str) -> Result<Option<&'static str>, String> {
    let partes: Vec<&str> = item.split(['/', '\\']).filter(|p| !p.is_empty() && *p != ".").collect();
    if item.starts_with(['/', '\\']) || item.contains(':') || partes.contains(&"..") {
        return Err("o pacote do Soloist tem um caminho estranho".into());
    }
    Ok(partes.last().and_then(|n| DO_PACOTE.into_iter().find(|d| d == n)))
}

fn abrir_pacote(bytes: &[u8], destino: &Path) -> Result<(), String> {
    let estragado = |e: &dyn std::fmt::Display| format!("o pacote do Soloist veio estragado: {e}");
    let mut pacote = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| estragado(&e))?;
    if pacote.len() > MAXIMO_DE_ITENS {
        return Err(estragado(&"itens demais"));
    }
    std::fs::create_dir_all(destino).map_err(|e| estragado(&e))?;
    let mut total = 0u64;
    for i in 0..pacote.len() {
        let mut item = pacote.by_index(i).map_err(|e| estragado(&e))?;
        let nome = nome_aceito(item.name())?;
        let Some(nome) = nome.filter(|_| item.is_file()) else { continue };
        // (`create_new`: um nome repetido é pacote estragado.)
        let arquivo = std::fs::OpenOptions::new().write(true).create_new(true).open(destino.join(nome));
        let mut arquivo = arquivo.map_err(|e| estragado(&e))?;
        let cabe = MAXIMO_ABERTO.saturating_sub(total);
        total += std::io::copy(&mut (&mut item).take(cabe + 1), &mut arquivo).map_err(|e| estragado(&e))?;
        if total > MAXIMO_ABERTO {
            return Err(estragado(&"grande demais"));
        }
    }
    if destino.join(EXE).is_file() { Ok(()) } else { Err(estragado(&"sem o programa")) }
}

/// Quando o programa foi compilado, pelo `--version` dele ("soloist 1.3 build 1763000000 (...)").
fn compilado_de(exe: &Path) -> Result<u64, String> {
    let mut comando = Command::new(exe);
    comando.arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    super::sem_janela(&mut comando);
    let mut filho = comando.spawn().map_err(|e| super::nao_abriu(&e))?;
    let ate = Instant::now() + Duration::from_secs(10);
    while matches!(filho.try_wait(), Ok(None)) && Instant::now() < ate {
        std::thread::sleep(Duration::from_millis(30));
    }
    let _ = filho.kill();
    let mut texto = String::new();
    if let Some(saida) = filho.stdout.take() {
        let _ = saida.take(4096).read_to_string(&mut texto);
    }
    let _ = filho.wait();
    ler_versao(&texto).ok_or_else(|| "o Soloist baixado não abriu neste computador".to_string())
}

fn ler_versao(texto: &str) -> Option<u64> {
    let mut palavras = texto.split_whitespace();
    (palavras.next()? == "soloist").then_some(())?;
    palavras.find(|p| *p == "build")?;
    palavras.next()?.parse().ok()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use std::io::Write;

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("tccplayer-soloist-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("pasta");
        p
    }

    fn pacote(itens: &[(&str, &str)]) -> Vec<u8> {
        let mut z = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (nome, texto) in itens {
            z.start_file(*nome, zip::write::SimpleFileOptions::default()).expect("item");
            z.write_all(texto.as_bytes()).expect("escreve");
        }
        z.finish().expect("fecha").into_inner()
    }

    #[test]
    fn o_pacote_vira_o_programa_e_o_resto_fica_de_fora() {
        let raiz = pasta("pacote");
        let bytes =
            pacote(&[("soloist/soloist.exe", "programa"), ("soloist/CHANGELOG.md", "x"), ("soloist/outro.dll", "y")]);
        let agora = 1_800_000_000;
        let i = instalar(&raiz, &bytes, agora, &|_| Ok(agora - DIA)).expect("instala");
        assert_eq!(std::fs::read_to_string(i.exe()).expect("exe"), "programa");
        assert!(!i.pasta.join("outro.dll").exists());
        assert_eq!(instalado(&raiz), Some(i.clone()));
        // Um mais novo entra; o velho e as sobras saem.
        std::fs::create_dir_all(raiz.join("baixando-1")).expect("sobra");
        let novo = instalar(&raiz, &bytes, agora, &|_| Ok(agora)).expect("instala");
        limpar(&raiz, &novo);
        assert_eq!(instalado(&raiz), Some(novo));
        assert!(!i.pasta.exists() && !raiz.join("baixando-1").exists());
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn pacote_estragado_ou_vencido_nao_entra() {
        let raiz = pasta("estragado");
        let agora = 1_800_000_000;
        let certo = |_: &Path| Ok(agora);
        assert!(instalar(&raiz, b"nada", agora, &certo).is_err());
        assert!(
            instalar(&raiz, &pacote(&[("leia.txt", "x")]), agora, &certo).expect_err("sem exe").contains("programa")
        );
        assert!(instalar(&raiz, &pacote(&[("../soloist.exe", "x")]), agora, &certo).is_err());
        assert!(instalar(&raiz, &pacote(&[("C:/soloist.exe", "x")]), agora, &certo).is_err());
        let duplo = pacote(&[("a/soloist.exe", "1"), ("b/soloist.exe", "2")]);
        assert!(instalar(&raiz, &duplo, agora, &certo).is_err());
        let bom = pacote(&[("soloist.exe", "x")]);
        assert!(instalar(&raiz, &bom, agora, &|_| Ok(agora - VENCE)).expect_err("vencido").contains("venceu"));
        assert!(instalar(&raiz, &bom, agora, &|_| Err("não abre".into())).is_err());
        assert_eq!(instalado(&raiz), None);
        assert!(!raiz.join(format!("baixando-{}", std::process::id())).exists(), "sem sobras");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn procura_outro_so_no_fim_da_validade_e_uma_vez_por_dia() {
        let i = Instalado { pasta: PathBuf::new(), compilado: 1_000 * DIA };
        assert!(!i.vale_procurar_outro(1_059 * DIA, None));
        assert!(i.vale_procurar_outro(1_060 * DIA, None));
        assert!(!i.vale_procurar_outro(1_061 * DIA, Some(1_060 * DIA + DIA / 2)));
        assert!(i.vale_procurar_outro(1_062 * DIA, Some(1_060 * DIA)));
        assert!(!i.vencido(1_089 * DIA) && i.vencido(1_090 * DIA));
    }

    #[test]
    fn um_soloist_que_so_quebra_nao_e_aberto_e_procura_outro_por_dia() {
        let raiz = pasta("quebrado");
        let agora = agora_s();
        let bytes = pacote(&[("soloist.exe", "x")]);
        let i = instalar(&raiz, &bytes, agora, &|_| Ok(agora - DIA)).expect("instala");
        std::fs::write(raiz.join("procurado"), agora.to_string()).expect("marca");
        let nao_para = AtomicBool::new(false);
        assert!(preparar(&raiz, false, &nao_para, &|_| {}).is_ok());
        assert!(!i.caiu(), "uma queda acontece");
        assert!(preparar(&raiz, false, &nao_para, &|_| {}).is_ok());
        assert!(i.caiu(), "a segunda, sem nunca ter tocado");
        assert_eq!(preparar(&raiz, false, &nao_para, &|_| {}), Err(crate::soloist::QUEBRADO.to_string()));
        assert!(i.vale_procurar_outro(agora + DIA, Some(agora)), "mesmo novo, procura outro no dia seguinte");
        assert!(!i.vale_procurar_outro(agora + DIA / 2, Some(agora)));
        // Um que já tocou aqui pode cair sem virar quebrado.
        let outro = instalar(&raiz, &bytes, agora, &|_| Ok(agora)).expect("instala");
        outro.funcionou();
        assert!(!outro.caiu() && !outro.caiu() && !outro.caiu());
        assert_eq!(preparar(&raiz, false, &nao_para, &|_| {}), Ok(outro));
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn a_versao() {
        assert_eq!(ler_versao("soloist 1.3.0 build 1763000000 (2025-11-13)"), Some(1_763_000_000));
        assert_eq!(ler_versao("outro 1 build 3"), None);
        assert_eq!(ler_versao("soloist 1.3 build x"), None);
        assert_eq!(ler_versao(""), None);
    }
}
