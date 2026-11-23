//! Os provedores instalados: uma pasta por provedor dentro de `<pasta de dados>/provedores`, cada uma com o
//! `provedor.toml` e o executável. Instalar copia a pasta de um provedor para lá (com o nome do serviço); remover a
//! move para `provedores-removidos`, de onde ela volta com "Desfazer".

use serde::Deserialize;
use std::path::{Component, Path, PathBuf};

pub const MANIFESTO: &str = "provedor.toml";
/// Onde ficam os provedores removidos (ao lado da pasta dos provedores), para o "Desfazer".
pub const REMOVIDOS: &str = "provedores-removidos";
/// Limites da pasta de um provedor a instalar (um provedor de verdade tem uns poucos arquivos).
const MAXIMO_DE_ARQUIVOS: usize = 2_000;
const MAXIMO_DE_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Manifesto {
    pub nome: String,
    /// Identificador curto do serviço (letras minúsculas, números e hífen): vai em toda faixa (`servico:id`).
    pub servico: String,
    /// O programa, relativo à pasta do provedor.
    pub executavel: String,
    #[serde(default)]
    pub argumentos: Vec<String>,
    #[serde(default)]
    pub versao: String,
}

/// Um provedor instalado e válido.
#[derive(Clone, Debug, PartialEq)]
pub struct Instalado {
    pub manifesto: Manifesto,
    pub pasta: PathBuf,
}

impl Instalado {
    pub fn executavel(&self) -> PathBuf {
        self.pasta.join(&self.manifesto.executavel)
    }
}

/// Lê um manifesto e confere: serviço com nome válido e executável dentro da pasta, existindo.
pub fn ler(pasta: &Path) -> Result<Instalado, String> {
    let texto = std::fs::read_to_string(pasta.join(MANIFESTO)).map_err(|e| format!("sem {MANIFESTO}: {e}"))?;
    let manifesto: Manifesto = toml::from_str(&texto).map_err(|e| format!("{MANIFESTO} inválido: {e}"))?;
    let servico_valido = !manifesto.servico.is_empty()
        && manifesto.servico.len() <= 40
        && manifesto.servico.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if !servico_valido {
        return Err(format!("serviço inválido: {:?} (use letras minúsculas, números e hífen)", manifesto.servico));
    }
    if manifesto.nome.trim().is_empty() {
        return Err("o provedor não tem nome".into());
    }
    // O executável fica dentro da pasta do provedor: nada de caminho absoluto nem de "..".
    let relativo = Path::new(&manifesto.executavel);
    let dentro = !manifesto.executavel.is_empty() && relativo.components().all(|c| matches!(c, Component::Normal(_)));
    if !dentro {
        return Err(format!("executável fora da pasta do provedor: {}", manifesto.executavel));
    }
    let instalado = Instalado { manifesto, pasta: pasta.to_path_buf() };
    if !instalado.executavel().is_file() {
        return Err(format!("o executável não existe: {}", instalado.executavel().display()));
    }
    Ok(instalado)
}

/// Todos os provedores válidos de `pasta` (em ordem de nome). Os estragados vão para o registro e ficam de fora;
/// dois com o mesmo serviço, fica o primeiro.
pub fn instalados(pasta: &Path) -> Vec<Instalado> {
    let Ok(entradas) = std::fs::read_dir(pasta) else { return Vec::new() };
    let mut pastas: Vec<PathBuf> = entradas.filter_map(Result::ok).map(|e| e.path()).filter(|p| p.is_dir()).collect();
    pastas.sort();
    let mut saida: Vec<Instalado> = Vec::new();
    for p in pastas {
        match ler(&p) {
            Ok(i) if saida.iter().any(|j| j.manifesto.servico == i.manifesto.servico) => {
                log::warn!(
                    "provedor repetido para o serviço {} em {}; fica o primeiro",
                    i.manifesto.servico,
                    p.display()
                );
            }
            Ok(i) => saida.push(i),
            Err(motivo) => log::warn!("provedor em {} ignorado: {motivo}", p.display()),
        }
    }
    saida.sort_by_cached_key(|i| nucleo::texto::normalizar(&i.manifesto.nome));
    saida
}

/// Copia `de` para `para` (que não existe ainda), pasta por pasta, sem seguir atalhos. Conta arquivos e bytes.
fn copiar(de: &Path, para: &Path, arquivos: &mut usize, bytes: &mut u64) -> Result<(), String> {
    std::fs::create_dir_all(para).map_err(|e| format!("não deu para criar {}: {e}", para.display()))?;
    let entradas = std::fs::read_dir(de).map_err(|e| format!("não deu para ler {}: {e}", de.display()))?;
    for entrada in entradas {
        let entrada = entrada.map_err(|e| e.to_string())?;
        let tipo = entrada.file_type().map_err(|e| e.to_string())?;
        let destino = para.join(entrada.file_name());
        if tipo.is_dir() {
            copiar(&entrada.path(), &destino, arquivos, bytes)?;
        } else if tipo.is_file() {
            *arquivos += 1;
            *bytes += entrada.metadata().map(|m| m.len()).unwrap_or(0);
            if *arquivos > MAXIMO_DE_ARQUIVOS || *bytes > MAXIMO_DE_BYTES {
                return Err("a pasta é grande demais para ser um provedor".into());
            }
            std::fs::copy(entrada.path(), &destino).map_err(|e| format!("não deu para copiar: {e}"))?;
        }
    }
    Ok(())
}

fn canonico(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
}

/// A pasta dos removidos, ao lado da pasta dos provedores.
fn lixeira(pasta: &Path) -> PathBuf {
    pasta.parent().map_or_else(|| pasta.join(REMOVIDOS), |p| p.join(REMOVIDOS))
}

/// Instala o provedor que está em `origem` dentro de `pasta` (a pasta dos provedores): confere o manifesto e
/// copia a pasta inteira para `<pasta>/<servico>`. Um provedor do mesmo serviço já instalado é trocado (é uma
/// atualização). O programa do provedor antigo tem que estar fechado.
pub fn instalar(origem: &Path, pasta: &Path) -> Result<Instalado, String> {
    let novo = ler(origem)?;
    let servico = novo.manifesto.servico.clone();
    std::fs::create_dir_all(pasta).map_err(|e| format!("não deu para criar {}: {e}", pasta.display()))?;
    let destino = pasta.join(&servico);
    let (origem_c, destino_c, pasta_c) = (canonico(origem), canonico(&destino), canonico(pasta));
    if origem_c == destino_c {
        return Err("este provedor já está instalado (é a pasta dele)".into());
    }
    if pasta_c.starts_with(&origem_c) {
        return Err("a pasta escolhida contém a pasta dos provedores".into());
    }
    // Copia para uma pasta temporária e só troca no fim: uma cópia que falha no meio não estraga o instalado.
    let temporaria = pasta.join(format!(".{servico}-instalando"));
    let antiga = pasta.join(format!(".{servico}-antigo"));
    let _ = std::fs::remove_dir_all(&temporaria);
    let _ = std::fs::remove_dir_all(&antiga);
    let (mut arquivos, mut bytes) = (0, 0);
    if let Err(e) = copiar(origem, &temporaria, &mut arquivos, &mut bytes) {
        let _ = std::fs::remove_dir_all(&temporaria);
        return Err(e);
    }
    if destino.exists()
        && let Err(e) = std::fs::rename(&destino, &antiga)
    {
        let _ = std::fs::remove_dir_all(&temporaria);
        return Err(format!("o provedor antigo ainda está aberto ({e})"));
    }
    if let Err(e) = std::fs::rename(&temporaria, &destino) {
        // Devolve o antigo, se havia.
        let _ = std::fs::rename(&antiga, &destino);
        let _ = std::fs::remove_dir_all(&temporaria);
        return Err(format!("não deu para instalar: {e}"));
    }
    let _ = std::fs::remove_dir_all(&antiga);
    ler(&destino)
}

/// Tira o provedor de `pasta` (a dos provedores), movendo a pasta dele para [`REMOVIDOS`] (ao lado), de onde
/// [`restaurar`] o traz de volta. Só mexe numa pasta que esteja mesmo dentro da pasta dos provedores. O programa
/// tem que estar fechado.
pub fn remover(instalado: &Instalado, pasta: &Path) -> Result<(), String> {
    let pasta_c = canonico(pasta);
    if canonico(&instalado.pasta).parent() != Some(pasta_c.as_path()) {
        return Err("este provedor não está na pasta dos provedores".into());
    }
    let lixeira = lixeira(pasta);
    std::fs::create_dir_all(&lixeira).map_err(|e| format!("não deu para criar {}: {e}", lixeira.display()))?;
    let destino = lixeira.join(&instalado.manifesto.servico);
    // Só o último removido de cada serviço fica guardado.
    let _ = std::fs::remove_dir_all(&destino);
    std::fs::rename(&instalado.pasta, &destino).map_err(|e| format!("não deu para remover (está aberto?): {e}"))
}

/// Traz de volta o provedor do `servico` removido por último.
pub fn restaurar(servico: &str, pasta: &Path) -> Result<Instalado, String> {
    let guardado = lixeira(pasta).join(servico);
    let destino = pasta.join(servico);
    if destino.exists() {
        return Err("já há um provedor deste serviço instalado".into());
    }
    std::fs::create_dir_all(pasta).map_err(|e| e.to_string())?;
    std::fs::rename(&guardado, &destino).map_err(|e| format!("não deu para restaurar: {e}"))?;
    ler(&destino)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn provedor(raiz: &Path, pasta: &str, manifesto: &str, exe: Option<&str>) {
        let p = raiz.join(pasta);
        std::fs::create_dir_all(&p).expect("pasta");
        std::fs::write(p.join(MANIFESTO), manifesto).expect("manifesto");
        if let Some(exe) = exe {
            std::fs::write(p.join(exe), b"programa").expect("exe");
        }
    }

    #[test]
    fn le_os_validos_e_pula_os_estragados() {
        let raiz = std::env::temp_dir().join(format!("tccplayer-provedores-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        provedor(&raiz, "b", "nome = \"Bê\"\nservico = \"bee\"\nexecutavel = \"b.exe\"\nversao = \"1\"", Some("b.exe"));
        provedor(
            &raiz,
            "a",
            "nome = \"Á\"\nservico = \"aaa\"\nexecutavel = \"bin/a.exe\"\nargumentos = [\"-x\"]",
            None,
        );
        std::fs::create_dir_all(raiz.join("a/bin")).expect("bin");
        std::fs::write(raiz.join("a/bin/a.exe"), b"programa").expect("exe");
        provedor(&raiz, "c-repetido", "nome = \"C\"\nservico = \"bee\"\nexecutavel = \"c.exe\"", Some("c.exe"));
        provedor(&raiz, "d-fora", "nome = \"D\"\nservico = \"ddd\"\nexecutavel = \"../b/b.exe\"", None);
        provedor(&raiz, "e-sem-exe", "nome = \"E\"\nservico = \"eee\"\nexecutavel = \"e.exe\"", None);
        provedor(&raiz, "f-servico", "nome = \"F\"\nservico = \"Com Espaço\"\nexecutavel = \"f.exe\"", Some("f.exe"));
        provedor(&raiz, "g-lixo", "isto não é toml = = =", None);
        provedor(&raiz, "h-absoluto", "nome = \"H\"\nservico = \"hhh\"\nexecutavel = \"C:/Windows/notepad.exe\"", None);
        std::fs::write(raiz.join("solto.txt"), b"arquivo solto").expect("solto");

        let todos = instalados(&raiz);
        let servicos: Vec<&str> = todos.iter().map(|i| i.manifesto.servico.as_str()).collect();
        assert_eq!(servicos, vec!["aaa", "bee"], "em ordem de nome, sem os estragados nem o repetido");
        assert_eq!(todos[0].executavel(), raiz.join("a").join("bin/a.exe"));
        assert_eq!(todos[0].manifesto.argumentos, vec!["-x"]);
        assert_eq!(todos[1].pasta, raiz.join("b"), "do serviço repetido, fica o primeiro (pasta b)");
        assert!(ler(&raiz.join("d-fora")).is_err_and(|e| e.contains("fora da pasta")));
        assert!(ler(&raiz.join("h-absoluto")).is_err_and(|e| e.contains("fora da pasta")));
        assert!(ler(&raiz.join("e-sem-exe")).is_err_and(|e| e.contains("não existe")));
        assert!(ler(&raiz.join("f-servico")).is_err_and(|e| e.contains("serviço inválido")));
        assert!(instalados(&raiz.join("nao-existe")).is_empty());
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn instala_atualiza_remove_e_restaura() {
        let raiz = std::env::temp_dir().join(format!("tccplayer-instalar-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let pasta = raiz.join("dados").join("provedores");
        // Um provedor baixado numa pasta qualquer, com uma subpasta.
        let manifesto =
            |versao: &str| format!("nome = \"Ex\"\nservico = \"ex\"\nexecutavel = \"ex.exe\"\nversao = \"{versao}\"");
        provedor(&raiz, "baixado", &manifesto("1"), Some("ex.exe"));
        std::fs::create_dir_all(raiz.join("baixado/dados")).expect("sub");
        std::fs::write(raiz.join("baixado/dados/x.txt"), b"x").expect("x");
        let i = instalar(&raiz.join("baixado"), &pasta).expect("instala");
        assert_eq!(i.pasta, pasta.join("ex"));
        assert!(pasta.join("ex/dados/x.txt").is_file(), "a subpasta vem junto");
        assert_eq!(instalados(&pasta).len(), 1);
        // Instalar de novo (versão nova) troca o instalado.
        std::fs::write(raiz.join("baixado").join(MANIFESTO), manifesto("2")).expect("versão 2");
        let i = instalar(&raiz.join("baixado"), &pasta).expect("atualiza");
        assert_eq!(i.manifesto.versao, "2");
        assert!(!pasta.join(".ex-instalando").exists() && !pasta.join(".ex-antigo").exists(), "não sobra lixo");
        // A própria pasta instalada, ou uma pasta que contém a dos provedores, não serve.
        assert!(instalar(&pasta.join("ex"), &pasta).is_err());
        assert!(instalar(&raiz, &pasta).is_err(), "sem manifesto, e contém a pasta dos provedores");
        // Remover move para os removidos; restaurar traz de volta.
        remover(&i, &pasta).expect("remove");
        assert!(instalados(&pasta).is_empty());
        assert!(raiz.join("dados").join(REMOVIDOS).join("ex").join(MANIFESTO).is_file());
        let volta = restaurar("ex", &pasta).expect("restaura");
        assert_eq!(volta.manifesto.versao, "2");
        assert!(restaurar("ex", &pasta).is_err(), "já está instalado");
        // Remover só mexe em pasta de dentro da pasta dos provedores.
        let fora = Instalado { pasta: raiz.join("baixado"), ..volta };
        assert!(remover(&fora, &pasta).is_err());
        assert!(raiz.join("baixado").join(MANIFESTO).is_file(), "a pasta de fora ficou onde estava");
        let _ = std::fs::remove_dir_all(&raiz);
    }
}
