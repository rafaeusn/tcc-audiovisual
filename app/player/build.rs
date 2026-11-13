//! Embute os ícones de `recursos/icones/*.svg` e as fontes de `recursos/fontes/*.ttf` no executável. Colocar um SVG novo na pasta já o torna disponível
//! como `icones/<nome>.svg`, sem mexer em código.

use std::fmt::Write as _;
use std::path::PathBuf;

fn main() {
    let raiz = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let pasta = raiz.join("../recursos/icones");
    println!("cargo:rerun-if-changed={}", pasta.display());

    let mut arquivos: Vec<PathBuf> = std::fs::read_dir(&pasta)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "svg")).collect())
        .unwrap_or_default();
    arquivos.sort();

    let mut codigo = String::from("pub fn icone(caminho: &str) -> Option<&'static [u8]> {\n    match caminho {\n");
    for arquivo in &arquivos {
        let Some(nome) = arquivo.file_stem().and_then(|n| n.to_str()) else { continue };
        let absoluto = arquivo.canonicalize().unwrap_or_else(|_| arquivo.clone());
        let _ = writeln!(
            codigo,
            "        {:?} => Some(include_bytes!({:?})),",
            format!("icones/{nome}.svg"),
            absoluto.display().to_string()
        );
    }
    codigo.push_str("        _ => None,\n    }\n}\n");

    // As fontes (`recursos/fontes/*.ttf`) entram do mesmo jeito.
    let pasta_fontes = raiz.join("../recursos/fontes");
    println!("cargo:rerun-if-changed={}", pasta_fontes.display());
    let mut fontes: Vec<PathBuf> = std::fs::read_dir(&pasta_fontes)
        .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "ttf")).collect())
        .unwrap_or_default();
    fontes.sort();
    codigo.push_str("pub fn fontes() -> Vec<&'static [u8]> {\n    vec![\n");
    for arquivo in &fontes {
        let absoluto = arquivo.canonicalize().unwrap_or_else(|_| arquivo.clone());
        let _ = writeln!(codigo, "        include_bytes!({:?}).as_slice(),", absoluto.display().to_string());
    }
    codigo.push_str("    ]\n}\n");

    let saida = PathBuf::from(std::env::var("OUT_DIR").unwrap_or_default()).join("icones.rs");
    if let Err(e) = std::fs::write(&saida, codigo) {
        println!("cargo:warning=não foi possível gerar {}: {e}", saida.display());
    }
}
