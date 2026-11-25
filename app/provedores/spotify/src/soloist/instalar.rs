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
/// Quantas quedas seguidas, sem um trecho tocando bem entre elas, fazem um Soloist quebrado.
const QUEDAS: usize = 2;
/// Quantas quedas num dia fazem um quebrado, mesmo tocando bem entre elas (um que toca uma faixa e cai na seguinte).
const QUEDAS_NO_DIA: usize = 4;
/// Quantas linhas do histórico de quedas ficam guardadas.
const HISTORICO: usize = 20;
/// A linha do histórico de um trecho que tocou bem.
const TOCOU: &str = "tocou";
/// Os que caem ao tocar em qualquer Windows (visto aqui e em crmne/spotifast#1412): nem são abertos. O 1.4.0.7, de 23/11/2026.
const QUE_QUEBRAM: [u64; 1] = [1_795_456_923];
/// Com um quebrado, de quanto em quanto tempo a pessoa pode pedir para procurar outro.
const PEDIDO_DE_NOVO: u64 = 10 * 60;
const PRAZO_DO_DOWNLOAD: Duration = Duration::from_secs(180);
/// Já há um que serve: procurar outro não pode segurar a abertura dele.
const PRAZO_COM_UM_BOM: Duration = Duration::from_secs(30);

pub fn agora_s() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs()
}

/// A versão que o programa diz ter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Versao {
    /// `1.3.10.51`.
    pub texto: String,
    /// Quando foi compilado (a validade conta daí).
    pub compilado: u64,
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

    /// A versão dele, para a pessoa ver (`1.3.10.51`).
    pub fn versao(&self) -> Option<String> {
        std::fs::read_to_string(self.pasta.join("versao")).ok().map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
    }

    pub fn vencido(&self, agora: u64) -> bool {
        agora >= self.compilado.saturating_add(VENCE)
    }

    /// Caiu sozinho. Devolve se, com esta, ele é dos que só caem (o Spotify já publicou alguns): não é mais aberto.
    pub fn caiu(&self, agora: u64) -> bool {
        self.anotar(agora.to_string());
        self.quebrado(agora)
    }

    /// Tocou um bom trecho: as quedas de antes não são mais seguidas. (E não é para sempre: o 1.4.0.7 às vezes toca uma
    /// faixa e cai na seguinte.)
    pub fn funcionou(&self) {
        if self.historico().last().is_some_and(|l| l != TOCOU) {
            self.anotar(TOCOU.into());
        }
    }

    /// A pessoa pediu para tentar de novo: as quedas são esquecidas.
    fn perdoar(&self) {
        let _ = std::fs::remove_file(self.pasta.join("quedas"));
    }

    /// As quedas (a hora de cada uma) e os trechos que tocaram bem, em ordem.
    fn historico(&self) -> Vec<String> {
        let texto = std::fs::read_to_string(self.pasta.join("quedas")).unwrap_or_default();
        texto.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect()
    }

    fn anotar(&self, linha: String) {
        let mut linhas = self.historico();
        linhas.push(linha);
        let sobra = linhas.len().saturating_sub(HISTORICO);
        let _ = std::fs::write(self.pasta.join("quedas"), linhas[sobra..].join("\n"));
    }

    fn quebrado(&self, agora: u64) -> bool {
        let linhas = self.historico();
        let seguidas = linhas.iter().rev().take_while(|l| *l != TOCOU).count();
        let de_hoje = |l: &&String| l.parse::<u64>().is_ok_and(|q| q <= agora && agora - q < DIA);
        QUE_QUEBRAM.contains(&self.compilado)
            || seguidas >= QUEDAS
            || linhas.iter().filter(de_hoje).count() >= QUEDAS_NO_DIA
    }

    /// Perto de vencer: vale procurar outro.
    fn velho(&self, agora: u64) -> bool {
        agora >= self.compilado.saturating_add(PROCURAR_OUTRO)
    }

    fn serve(&self, agora: u64) -> bool {
        !self.vencido(agora) && !self.quebrado(agora)
    }
}

/// Os instalados em `raiz`, do mais novo ao mais velho.
pub fn instalados(raiz: &Path) -> Vec<Instalado> {
    let mut v: Vec<Instalado> = std::fs::read_dir(raiz)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|e| {
            let compilado = e.file_name().to_str()?.parse::<u64>().ok()?;
            let i = Instalado { pasta: e.path(), compilado };
            i.exe().is_file().then_some(i)
        })
        .collect();
    v.sort_by_key(|i| std::cmp::Reverse(i.compilado));
    v
}

/// O melhor dos instalados: o mais novo que não venceu nem quebra aqui. Se o último publicado quebra, vale o de antes
/// (como o Spotifast 0.17.2 faz).
pub fn escolher(raiz: &Path, agora: u64) -> Option<Instalado> {
    instalados(raiz).into_iter().find(|i| i.serve(agora))
}

/// Como preparar o programa.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Preparo {
    /// O de agora se disse vencido: baixa de qualquer jeito.
    pub de_novo: bool,
    /// A pessoa pediu agora (e não a abertura do Player): com um quebrado, procura outro sem esperar o dia seguinte.
    pub pedida: bool,
}

/// O que a procura no servidor deu.
enum Procura {
    /// O pacote é o mesmo que já está aqui.
    OMesmo,
    Novo(Instalado),
}

/// Deixa um Soloist que valha em `raiz`, baixando se preciso.
pub fn preparar(raiz: &Path, como: Preparo, parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<Instalado, String> {
    preparar_com(raiz, como, agora_s(), &|prazo| baixar(raiz, prazo, parar, progresso))
}

fn preparar_com(
    raiz: &Path,
    como: Preparo,
    agora: u64,
    baixar: &dyn Fn(Duration) -> Result<Procura, String>,
) -> Result<Instalado, String> {
    let marca = raiz.join("procurado");
    let procurado = std::fs::read_to_string(&marca).ok().and_then(|t| t.trim().parse::<u64>().ok());
    let todos = instalados(raiz);
    let bom = todos.iter().find(|i| i.serve(agora)).filter(|_| !como.de_novo).cloned();
    let quebra = |i: &Instalado| i.quebrado(agora) && !i.vencido(agora);
    let espera = if como.pedida { PEDIDO_DE_NOVO } else { DIA };
    let faz_tempo = procurado.is_none_or(|p| agora >= p.saturating_add(espera) || p > agora);
    let procurar = match &bom {
        // O mais novo quebra (pode ter saído outro), ou este está para vencer.
        Some(i) => faz_tempo && (todos.first().is_some_and(quebra) || i.velho(agora)),
        // Só há quebrados: não adianta baixar o mesmo a toda hora. Sem nenhum (ou vencido), é preciso.
        None => como.de_novo || faz_tempo || !todos.iter().any(quebra),
    };
    // Com um quebrado e a pessoa pedindo, ele ganha outra chance (menos os que quebram em todo Windows).
    let ultimo_recurso = || {
        if como.pedida && !como.de_novo {
            todos.iter().for_each(Instalado::perdoar);
            return escolher(raiz, agora);
        }
        None
    };
    if !procurar {
        return bom.or_else(ultimo_recurso).ok_or_else(|| super::QUEBRADO.into());
    }
    let _ = std::fs::create_dir_all(raiz);
    let prazo = if bom.is_some() { PRAZO_COM_UM_BOM } else { PRAZO_DO_DOWNLOAD };
    // (A marca só vale se o servidor respondeu: sem rede, a próxima abertura procura de novo.)
    let achado = baixar(prazo).inspect(|_| {
        let _ = std::fs::write(&marca, agora.to_string());
    });
    match achado {
        Ok(Procura::Novo(novo)) => {
            limpar(raiz, &novo, agora);
            escolher(raiz, agora).or_else(ultimo_recurso).ok_or_else(|| super::QUEBRADO.into())
        }
        Ok(Procura::OMesmo) => bom.or_else(ultimo_recurso).ok_or_else(|| super::QUEBRADO.into()),
        // Há um que vale: fica, e depois se procura de novo.
        Err(e) => bom
            .inspect(|_| eprintln!("soloist: não deu para procurar outro ({e}); fica o instalado"))
            .or_else(|| todos.iter().any(quebra).then(ultimo_recurso).flatten())
            .ok_or(e),
    }
}

/// Tira os outros instalados e as sobras de um download interrompido. Fica também o melhor dos outros: se o novo quebrar,
/// volta-se para ele.
fn limpar(raiz: &Path, novo: &Instalado, agora: u64) {
    let reserva = instalados(raiz).into_iter().find(|i| i.pasta != novo.pasta && i.serve(agora));
    let Ok(pastas) = std::fs::read_dir(raiz) else { return };
    for e in pastas.filter_map(Result::ok) {
        let nome = e.file_name().to_string_lossy().into_owned();
        let nossa = nome.parse::<u64>().is_ok() || nome.starts_with("baixando-");
        let fica = e.path() == novo.pasta || reserva.as_ref().is_some_and(|r| r.pasta == e.path());
        if nossa && !fica && e.file_type().is_ok_and(|t| t.is_dir()) {
            let _ = std::fs::remove_dir_all(e.path());
        }
    }
}

fn baixar(raiz: &Path, prazo: Duration, parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<Procura, String> {
    // O pacote do mais novo daqui: se o servidor ainda tem o mesmo, não há o que baixar.
    let conhecido = instalados(raiz).first().and_then(|i| std::fs::read_to_string(i.pasta.join("etag")).ok());
    let Some(pacote) = baixar_pacote(conhecido.as_deref(), prazo, parar, progresso)? else {
        return Ok(Procura::OMesmo);
    };
    let novo = instalar(raiz, agora_s(), &|d| abrir_pacote(&pacote.bytes, d), &|exe| conferir(exe, parar))?;
    if let Some(etag) = pacote.etag {
        let _ = std::fs::write(novo.pasta.join("etag"), etag);
    }
    Ok(Procura::Novo(novo))
}

struct Pacote {
    bytes: Vec<u8>,
    etag: Option<String>,
}

/// Um Soloist de outro programa deste computador (o Spotifast), que pode ser copiado para cá.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeOutroPrograma {
    pasta: PathBuf,
    pub versao: Versao,
}

/// Onde o Spotifast guarda os dele (uma pasta por programa baixado).
pub fn pasta_do_spotifast() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    Some(PathBuf::from(local).join(r"paolino\spotifast\data\soloist\builds"))
}

/// O melhor Soloist de outro programa em `base`: assinado pelo Spotify, que não venceu, não quebra e não está aqui.
pub fn de_outro_programa(raiz: &Path, base: &Path, agora: u64, parar: &AtomicBool) -> Option<DeOutroPrograma> {
    de_outro_com(raiz, base, agora, &|exe| conferir(exe, parar))
}

fn de_outro_com(
    raiz: &Path,
    base: &Path,
    agora: u64,
    conferir: &dyn Fn(&Path) -> Result<Versao, String>,
) -> Option<DeOutroPrograma> {
    let daqui = instalados(raiz);
    std::fs::read_dir(base)
        .ok()?
        .filter_map(Result::ok)
        .take(MAXIMO_DE_ITENS)
        .map(|e| e.path())
        .filter(|p| p.join(EXE).is_file())
        .filter_map(|pasta| Some(DeOutroPrograma { versao: conferir(&pasta.join(EXE)).ok()?, pasta }))
        .filter(|d| {
            let la = Instalado { pasta: d.pasta.clone(), compilado: d.versao.compilado };
            la.serve(agora) && !daqui.iter().any(|i| i.compilado == d.versao.compilado)
        })
        .max_by_key(|d| d.versao.compilado)
}

/// Copia para cá o Soloist de outro programa (a cópia é conferida de novo).
pub fn trazer(raiz: &Path, outro: &DeOutroPrograma, parar: &AtomicBool) -> Result<Instalado, String> {
    let _ = std::fs::create_dir_all(raiz);
    instalar(raiz, agora_s(), &|destino| copiar(&outro.pasta, destino), &|exe| conferir(exe, parar))
}

fn copiar(origem: &Path, destino: &Path) -> Result<(), String> {
    let nao_copiou = |e: std::io::Error| format!("não deu para copiar o Soloist: {e}");
    std::fs::create_dir_all(destino).map_err(nao_copiou)?;
    for nome in DO_PACOTE.iter().filter(|n| origem.join(n).is_file()) {
        std::fs::copy(origem.join(nome), destino.join(nome)).map_err(nao_copiou)?;
    }
    if destino.join(EXE).is_file() { Ok(()) } else { Err("o outro programa não tem mais o Soloist".into()) }
}

/// Põe o programa numa pasta provisória (`por`), confere e o põe no lugar.
fn instalar(
    raiz: &Path,
    agora: u64,
    por: &dyn Fn(&Path) -> Result<(), String>,
    conferir: &dyn Fn(&Path) -> Result<Versao, String>,
) -> Result<Instalado, String> {
    let provisoria = raiz.join(format!("baixando-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&provisoria);
    let feito = por(&provisoria).and_then(|()| {
        let versao = conferir(&provisoria.join(EXE))?;
        let novo = Instalado { pasta: raiz.join(versao.compilado.to_string()), compilado: versao.compilado };
        if novo.vencido(agora) {
            return Err("este Soloist já venceu (confira a data do computador)".into());
        }
        if !novo.exe().is_file() {
            let _ = std::fs::write(provisoria.join("versao"), &versao.texto);
            let _ = std::fs::remove_dir_all(&novo.pasta);
            std::fs::rename(&provisoria, &novo.pasta).map_err(|e| format!("não deu para instalar o Soloist: {e}"))?;
        }
        Ok(novo)
    });
    let _ = std::fs::remove_dir_all(&provisoria);
    feito
}

/// Baixa o pacote do servidor do Spotify (só dele, sem redirecionar). `None`: ainda é o `conhecido`.
fn baixar_pacote(
    conhecido: Option<&str>,
    prazo: Duration,
    parar: &AtomicBool,
    progresso: &dyn Fn(f32),
) -> Result<Option<Pacote>, String> {
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
            .timeout(prazo)
            .connect_timeout(Duration::from_secs(10))
            .build()
            .map_err(nao_baixou)?;
        let mut pedido = cliente.get(format!("https://{SERVIDOR}/{PACOTE}"));
        if let Some(etag) = conhecido {
            pedido = pedido.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        let resposta = pedido.send().await;
        let mut resposta = resposta.and_then(reqwest::Response::error_for_status).map_err(nao_baixou)?;
        if resposta.status() == reqwest::StatusCode::NOT_MODIFIED {
            return Ok(None);
        }
        let etag = resposta.headers().get(reqwest::header::ETAG).and_then(|e| e.to_str().ok()).map(str::to_string);
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
        Ok(Some(Pacote { bytes, etag }))
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

/// O programa é mesmo do Spotify e abre aqui: a versão dele, pelo `--version` ("soloist 1.3.10.51 build 1793988108 (...)").
fn conferir(exe: &Path, parar: &AtomicBool) -> Result<Versao, String> {
    if !super::assinatura::do_spotify(exe) {
        return Err("este Soloist não tem a assinatura do Spotify".into());
    }
    let mut comando = Command::new(exe);
    comando.arg("--version").stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    super::sem_janela(&mut comando);
    let mut filho = comando.spawn().map_err(|e| super::nao_abriu(&e))?;
    let ate = Instant::now() + Duration::from_secs(10);
    while matches!(filho.try_wait(), Ok(None)) && Instant::now() < ate && !parar.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(30));
    }
    let _ = filho.kill();
    let mut texto = String::new();
    if let Some(saida) = filho.stdout.take() {
        let _ = saida.take(4096).read_to_string(&mut texto);
    }
    let _ = filho.wait();
    ler_versao(&texto).ok_or_else(|| "este Soloist não abre neste computador".to_string())
}

fn ler_versao(texto: &str) -> Option<Versao> {
    let mut palavras = texto.split_whitespace();
    (palavras.next()? == "soloist").then_some(())?;
    let versao = palavras.next()?.to_string();
    palavras.find(|p| *p == "build")?;
    Some(Versao { texto: versao, compilado: palavras.next()?.parse().ok()? })
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
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

    fn de(compilado: u64) -> impl Fn(&Path) -> Result<Versao, String> {
        move |_| Ok(Versao { texto: format!("1.{compilado}"), compilado })
    }

    /// Instala um pacote com o programa, compilado em `compilado`.
    fn por(raiz: &Path, agora: u64, compilado: u64) -> Instalado {
        let bytes = pacote(&[("soloist.exe", "programa")]);
        instalar(raiz, agora, &|d| abrir_pacote(&bytes, d), &de(compilado)).expect("instala")
    }

    const AGORA: u64 = 1_800_000_000;

    #[test]
    fn o_pacote_vira_o_programa_e_o_resto_fica_de_fora() {
        let raiz = pasta("pacote");
        let bytes =
            pacote(&[("soloist/soloist.exe", "programa"), ("soloist/CHANGELOG.md", "x"), ("soloist/outro.dll", "y")]);
        let i = instalar(&raiz, AGORA, &|d| abrir_pacote(&bytes, d), &de(AGORA - DIA)).expect("instala");
        assert_eq!(std::fs::read_to_string(i.exe()).expect("exe"), "programa");
        assert_eq!(i.versao(), Some(format!("1.{}", AGORA - DIA)), "a versão fica para a pessoa ver");
        assert!(!i.pasta.join("outro.dll").exists());
        assert_eq!(escolher(&raiz, AGORA), Some(i));
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn pacote_estragado_ou_vencido_nao_entra() {
        let raiz = pasta("estragado");
        let com = |bytes: &[u8], conferir: &dyn Fn(&Path) -> Result<Versao, String>| {
            instalar(&raiz, AGORA, &|d| abrir_pacote(bytes, d), conferir)
        };
        assert!(com(b"nada", &de(AGORA)).is_err());
        assert!(com(&pacote(&[("leia.txt", "x")]), &de(AGORA)).expect_err("sem exe").contains("programa"));
        assert!(com(&pacote(&[("../soloist.exe", "x")]), &de(AGORA)).is_err());
        assert!(com(&pacote(&[("C:/soloist.exe", "x")]), &de(AGORA)).is_err());
        assert!(com(&pacote(&[("a/soloist.exe", "1"), ("b/soloist.exe", "2")]), &de(AGORA)).is_err());
        let bom = pacote(&[("soloist.exe", "x")]);
        assert!(com(&bom, &de(AGORA - VENCE)).expect_err("vencido").contains("venceu"));
        assert!(com(&bom, &|_| Err("sem assinatura".into())).is_err());
        assert!(instalados(&raiz).is_empty());
        assert!(!raiz.join(format!("baixando-{}", std::process::id())).exists(), "sem sobras");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn sem_assinatura_do_spotify_o_programa_nem_abre() {
        let raiz = pasta("assinatura");
        let bytes = pacote(&[("soloist.exe", "MZ qualquer coisa")]);
        let nao_para = AtomicBool::new(false);
        let conferir = |exe: &Path| conferir(exe, &nao_para);
        let erro = instalar(&raiz, AGORA, &|d| abrir_pacote(&bytes, d), &conferir).expect_err("sem assinatura");
        assert!(erro.contains("assinatura"), "{erro}");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn procura_outro_so_no_fim_da_validade_e_uma_vez_por_dia() {
        let raiz = pasta("validade");
        let i = por(&raiz, AGORA, AGORA);
        assert!(!i.vencido(AGORA + 89 * DIA) && i.vencido(AGORA + 90 * DIA));
        let (baixou, sem_rede) = (std::cell::Cell::new(0), std::cell::Cell::new(false));
        let preparar = |agora: u64| {
            preparar_com(&raiz, Preparo::default(), agora, &|prazo| {
                assert!(prazo <= PRAZO_DO_DOWNLOAD);
                baixou.set(baixou.get() + 1);
                if sem_rede.get() { Err("sem rede".into()) } else { Ok(Procura::OMesmo) }
            })
        };
        assert_eq!((preparar(AGORA + 59 * DIA), baixou.get()), (Ok(i.clone()), 0));
        sem_rede.set(true);
        assert_eq!((preparar(AGORA + 60 * DIA), baixou.get()), (Ok(i.clone()), 1), "sem rede, fica o instalado");
        sem_rede.set(false);
        assert_eq!((preparar(AGORA + 60 * DIA + 60), baixou.get()), (Ok(i.clone()), 2), "e procura na seguinte");
        assert_eq!((preparar(AGORA + 60 * DIA + DIA / 2), baixou.get()), (Ok(i.clone()), 2), "uma vez por dia");
        assert_eq!((preparar(AGORA + 61 * DIA + 60), baixou.get()), (Ok(i), 3));
        sem_rede.set(true);
        assert_eq!((preparar(AGORA + 90 * DIA).is_err(), baixou.get()), (true, 4), "vencido: é preciso outro");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn se_o_novo_so_quebra_volta_se_para_o_anterior() {
        let raiz = pasta("voltar");
        let velho = por(&raiz, AGORA, AGORA - 10 * DIA);
        let novo = por(&raiz, AGORA, AGORA - DIA);
        limpar(&raiz, &novo, AGORA);
        assert!(velho.exe().is_file(), "o anterior fica de reserva");
        std::fs::write(raiz.join("procurado"), AGORA.to_string()).expect("marca");
        let sem_baixar = |_: Duration| -> Result<Procura, String> { panic!("não é hora de baixar") };
        let agora = |como| preparar_com(&raiz, como, AGORA + 60, &sem_baixar);
        assert_eq!(agora(Preparo::default()), Ok(novo.clone()));
        assert!(!novo.caiu(AGORA), "uma queda acontece");
        assert_eq!(agora(Preparo::default()), Ok(novo.clone()));
        // Um bom trecho tocando zera as quedas seguidas, mas não vale para sempre.
        novo.funcionou();
        assert!(!novo.caiu(AGORA));
        assert!(novo.caiu(AGORA), "a segunda seguida");
        assert_eq!(agora(Preparo::default()), Ok(velho.clone()), "volta para o anterior");
        // Com o anterior também quebrado, não há o que abrir (nem baixar de novo na mesma hora).
        velho.caiu(AGORA);
        assert!(velho.caiu(AGORA));
        assert_eq!(agora(Preparo::default()), Err(crate::soloist::QUEBRADO.to_string()));
        // A pessoa pediu: eles ganham outra chance, do mais novo ao mais velho.
        assert_eq!(agora(Preparo { pedida: true, ..Default::default() }), Ok(novo));
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn com_um_quebrado_procura_outro_no_dia_seguinte_ou_quando_a_pessoa_pede() {
        let raiz = pasta("procura");
        let ruim = por(&raiz, AGORA, AGORA - DIA);
        let quebrar = || {
            ruim.caiu(AGORA);
            assert!(ruim.caiu(AGORA));
        };
        quebrar();
        std::fs::write(raiz.join("procurado"), AGORA.to_string()).expect("marca");
        let baixou = std::cell::Cell::new(0);
        let preparar = |agora: u64, pedida: bool, nasce: Option<u64>| {
            preparar_com(&raiz, Preparo { pedida, ..Default::default() }, agora, &|_| {
                baixou.set(baixou.get() + 1);
                Ok(nasce.map_or(Procura::OMesmo, |c| Procura::Novo(por(&raiz, agora, c))))
            })
        };
        assert!(preparar(AGORA + 3600, false, None).is_err());
        assert_eq!(baixou.get(), 0, "sozinho, só uma vez por dia");
        assert_eq!(preparar(AGORA + 60, true, None), Ok(ruim.clone()), "acabou de procurar: outra chance");
        assert_eq!(baixou.get(), 0);
        quebrar();
        assert_eq!(preparar(AGORA + 3600, true, None), Ok(ruim.clone()), "procurou, e o publicado é o mesmo");
        assert_eq!(baixou.get(), 1);
        quebrar();
        let novo = preparar(AGORA + 3600 + DIA, false, Some(AGORA)).expect("saiu outro");
        assert_eq!((novo.compilado, baixou.get()), (AGORA, 2));
        assert!(!ruim.pasta.exists(), "o quebrado sai quando há outro mais novo");
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn os_que_quebram_em_todo_windows_nem_sao_abertos() {
        let raiz = pasta("conhecido");
        let conhecido = por(&raiz, QUE_QUEBRAM[0] + DIA, QUE_QUEBRAM[0]);
        // O "funciona" de uma versão antiga do Player (dado depois de 2 s tocando) não vale.
        std::fs::write(conhecido.pasta.join("funciona"), "").expect("marca antiga");
        assert_eq!(escolher(&raiz, QUE_QUEBRAM[0] + DIA), None);
        let anterior = por(&raiz, QUE_QUEBRAM[0] + DIA, QUE_QUEBRAM[0] - 20 * DIA);
        assert_eq!(escolher(&raiz, QUE_QUEBRAM[0] + DIA), Some(anterior.clone()));
        // Nem pedindo: sem outro, o motivo é dito na hora.
        std::fs::remove_dir_all(&anterior.pasta).expect("tira o anterior");
        std::fs::write(raiz.join("procurado"), (QUE_QUEBRAM[0] + DIA).to_string()).expect("marca");
        let pedida = Preparo { pedida: true, ..Default::default() };
        let sem_baixar = |_: Duration| -> Result<Procura, String> { panic!("acabou de procurar") };
        let agora = QUE_QUEBRAM[0] + DIA + 60;
        assert_eq!(preparar_com(&raiz, pedida, agora, &sem_baixar), Err(crate::soloist::QUEBRADO.to_string()));
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn o_que_toca_uma_faixa_e_cai_na_seguinte_tambem_e_quebrado() {
        let raiz = pasta("alterna");
        let i = por(&raiz, AGORA, AGORA - DIA);
        for n in 0..3 {
            assert!(!i.caiu(AGORA + n), "queda {n}");
            i.funcionou();
            i.funcionou();
        }
        assert!(i.caiu(AGORA + 3), "quatro no mesmo dia");
        assert!(!i.quebrado(AGORA + 2 * DIA), "no outro dia ele pode tentar de novo");
        // Espalhadas pelos dias, com música entre elas, são só azar (e o histórico não cresce sem fim).
        i.funcionou();
        for d in 2..40 {
            assert!(!i.caiu(AGORA + d * DIA), "dia {d}");
            i.funcionou();
        }
        assert!(i.historico().len() <= HISTORICO);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn o_soloist_de_outro_programa_serve_se_for_bom_e_nao_estiver_aqui() {
        let (raiz, base) = (pasta("daqui"), pasta("spotifast"));
        let la = |nome: &str, compilado: u64| {
            std::fs::create_dir_all(base.join(nome)).expect("pasta");
            std::fs::write(base.join(nome).join(EXE), compilado.to_string()).expect("exe");
        };
        // O "programa" de mentira diz a versão no próprio conteúdo; o sem assinatura não é lido.
        let ler = |exe: &Path| {
            let texto = std::fs::read_to_string(exe).map_err(|e| e.to_string())?;
            let compilado = texto.parse::<u64>().map_err(|_| "sem assinatura".to_string())?;
            Ok(Versao { texto: "1.3.10.51".into(), compilado })
        };
        assert_eq!(de_outro_com(&raiz, &base, AGORA, &ler), None);
        la("velho", AGORA - 95 * DIA);
        la("bom", AGORA - 18 * DIA);
        la("mais-velho", AGORA - 40 * DIA);
        la("que-quebra", QUE_QUEBRAM[0]);
        std::fs::create_dir_all(base.join("falso")).expect("pasta");
        std::fs::write(base.join("falso").join(EXE), "programa qualquer").expect("exe");
        let achado = de_outro_com(&raiz, &base, AGORA, &ler).expect("um bom");
        assert_eq!((achado.versao.compilado, achado.versao.texto.as_str()), (AGORA - 18 * DIA, "1.3.10.51"));
        // Copiado para cá, ele passa a ser um dos instalados, e não é mais oferecido.
        let copiado = instalar(&raiz, AGORA, &|d| copiar(&achado.pasta, d), &ler).expect("copia");
        assert_eq!(escolher(&raiz, AGORA), Some(copiado));
        assert!(base.join("bom").join(EXE).is_file(), "o do outro programa fica onde está");
        assert_eq!(de_outro_com(&raiz, &base, AGORA, &ler).map(|d| d.versao.compilado), Some(AGORA - 40 * DIA));
        assert!(copiar(&base.join("nada"), &raiz.join("x")).is_err());
        let _ = (std::fs::remove_dir_all(&raiz), std::fs::remove_dir_all(&base));
    }

    /// `cargo test -p provedor-spotify -- --ignored soloist_de_verdade --nocapture`
    #[test]
    #[ignore = "precisa do Spotifast com um Soloist neste computador"]
    fn o_soloist_de_verdade_do_spotifast_e_conferido_e_copiado() {
        let raiz = pasta("de-verdade");
        let base = pasta_do_spotifast().expect("a pasta");
        let nao_para = AtomicBool::new(false);
        let outro = de_outro_programa(&raiz, &base, agora_s(), &nao_para).expect("um Soloist que serve");
        println!("achado: {outro:?}");
        let copiado = trazer(&raiz, &outro, &nao_para).expect("copia");
        assert_eq!(copiado.versao(), Some(outro.versao.texto.clone()));
        assert_eq!(escolher(&raiz, agora_s()), Some(copiado));
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn a_versao() {
        assert_eq!(
            ler_versao("soloist 1.3.10.51 build 1793988108 (20261106) (g19412df6ea) (windows/x86_64)"),
            Some(Versao { texto: "1.3.10.51".into(), compilado: 1_793_988_108 })
        );
        assert_eq!(ler_versao("outro 1 build 3"), None);
        assert_eq!(ler_versao("soloist 1.3 build x"), None);
        assert_eq!(ler_versao("soloist"), None);
        assert_eq!(ler_versao(""), None);
    }
}
