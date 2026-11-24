//! Os códigos das consultas do Spotify ("persisted queries"): cada operação do Pathfinder tem um, e o Spotify troca
//! alguns de tempos em tempos. Os de agora vêm embutidos; um trocado é relido do texto do site do Spotify (a ideia é
//! do limusic, GPL-3) e fica guardado na pasta do provedor. Do site só se lê texto; nada é executado.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

/// As operações que este provedor usa (todas só leem) e o código de cada uma em 23/11/2026.
const EMBUTIDOS: [(&str, &str); 8] = [
    ("searchDesktop", "eef7cc54888d91bdd6802623477873caa3948ae173a0c34fd86827b267e94c03"),
    ("home", "5fb7da7a03ee2776c0670856609cb85a189a3cecc36cc160b91cde79063b9cf1"),
    ("libraryV3", "390c78e5b951029bad359785e69b07b536a509c581cbcd0aded5e5067f187455"),
    ("fetchLibraryTracks", "087278b20b743578a6262c2b0b4bcd20d879c503cc359a2285baf083ef944240"),
    ("fetchPlaylist", "21ea11a6ac75a5f3e01fd365d29c18e0a4a8b7dc09f89bbb5d73910190c6acaf"),
    ("getAlbum", "0b6ce3511e1b02e4a40fa95440f992acbe85080eb308af9194322d708fc1914f"),
    ("queryArtistOverview", "eafb1620d783ece118e72fbfede2b0064483e029d3c1aafafa1c2de0e25a89e7"),
    ("queryArtistDiscographyAll", "5e07d323febb57b4a56a42abbf781490e58764aa45feb6e3dc0591564fc56599"),
];

/// A parte do site em que a operação mora, quando não é o pacote principal.
pub fn rota_da_operacao(op: &str) -> Option<&'static str> {
    match op {
        "searchDesktop" => Some("xpui-routes-search"),
        "queryArtistDiscographyAll" => Some("xpui-routes-artist"),
        _ => None,
    }
}

pub struct Codigos {
    arquivo: PathBuf,
    relidos: Mutex<HashMap<String, String>>,
}

impl Codigos {
    pub fn novo(arquivo: PathBuf) -> Self {
        let relidos = std::fs::read_to_string(&arquivo)
            .ok()
            .and_then(|t| serde_json::from_str::<HashMap<String, String>>(&t).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|(op, c)| conhecida(op) && valido(c))
            .collect();
        Self { arquivo, relidos: Mutex::new(relidos) }
    }

    /// Os códigos a tentar, em ordem: o relido do site, se houve, e o embutido. Vazio para o que este provedor não usa.
    pub fn de(&self, op: &str) -> Vec<String> {
        let relidos = self.relidos.lock().unwrap_or_else(PoisonError::into_inner);
        let mut v: Vec<String> = relidos.get(op).cloned().into_iter().collect();
        v.extend(embutido(op).map(str::to_string).filter(|e| !v.contains(e)));
        v
    }

    /// Um código relido que o Spotify não aceita mais (o embutido de um Player novo vale).
    pub fn esquecer(&self, op: &str) {
        let mut relidos = self.relidos.lock().unwrap_or_else(PoisonError::into_inner);
        if relidos.remove(op).is_some() {
            self.gravar(&relidos);
        }
    }

    /// Guarda um código relido (só de uma operação conhecida e no formato certo).
    pub fn guardar(&self, op: &str, codigo: &str) {
        if !conhecida(op) || !valido(codigo) {
            return;
        }
        let mut relidos = self.relidos.lock().unwrap_or_else(PoisonError::into_inner);
        relidos.insert(op.to_string(), codigo.to_string());
        self.gravar(&relidos);
    }

    fn gravar(&self, relidos: &HashMap<String, String>) {
        if let Ok(texto) = serde_json::to_string(relidos) {
            let _ = std::fs::write(&self.arquivo, texto);
        }
    }
}

fn embutido(op: &str) -> Option<&'static str> {
    EMBUTIDOS.iter().find(|(nome, _)| *nome == op).map(|(_, c)| *c)
}

fn conhecida(op: &str) -> bool {
    embutido(op).is_some()
}

fn valido(codigo: &str) -> bool {
    codigo.len() == 64 && codigo.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// O nome do pacote principal (`web-player.<versão>.js`) na página do site.
pub fn pacote_principal(html: &str) -> Option<String> {
    const INICIO: &str = "web-player/web-player.";
    let depois = &html[html.find(INICIO)? + INICIO.len()..];
    let versao = &depois[..depois.find(".js")?];
    let limpa = !versao.is_empty() && versao.len() <= 40 && versao.bytes().all(|b| b.is_ascii_alphanumeric());
    limpa.then(|| format!("web-player.{versao}.js"))
}

/// O código de `op` no texto de um pacote (`"op","query","<64 hexadecimais>"`).
pub fn codigo_no_texto(js: &str, op: &str) -> Option<String> {
    let inicio = format!("\"{op}\",\"query\",\"");
    let depois = &js[js.find(&inicio)? + inicio.len()..];
    let codigo = &depois[..depois.find('"')?];
    valido(codigo).then(|| codigo.to_string())
}

/// Os arquivos possíveis do pedaço `rota` (o pacote principal diz o número dele e a versão; pode haver mais de uma
/// versão com o mesmo número, a do estilo e a do código).
pub fn pedacos_da_rota(js: &str, rota: &str) -> Vec<String> {
    let Some(onde) = js.find(&format!(":\"{rota}\"")) else { return Vec::new() };
    let antes = &js[..onde];
    let numero = &antes[antes.trim_end_matches(|c: char| c.is_ascii_digit()).len()..];
    if numero.is_empty() {
        return Vec::new();
    }
    let chave = format!("{numero}:\"");
    let mut achados = Vec::new();
    for (i, _) in js.match_indices(&chave) {
        // O número inteiro (o 4406, e não o fim do 14406).
        if js[..i].ends_with(|c: char| c.is_ascii_digit()) {
            continue;
        }
        let depois = &js[i + chave.len()..];
        let versao: String = depois.chars().take_while(|c| *c != '"').take(40).collect();
        if versao.len() == 8 && versao.bytes().all(|b| b.is_ascii_hexdigit()) {
            let nome = format!("{rota}.{versao}.js");
            if !achados.contains(&nome) {
                achados.push(nome);
            }
        }
    }
    achados
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    #[test]
    fn o_pacote_principal_sai_da_pagina() {
        let html = r#"<script src="https://open.spotifycdn.com/cdn/build/web-player/vendor~web-player.1a2b.js"></script>
            <script src="https://open.spotifycdn.com/cdn/build/web-player/web-player.d977a0e9.js"></script>"#;
        assert_eq!(pacote_principal(html).as_deref(), Some("web-player.d977a0e9.js"));
        assert_eq!(pacote_principal("<html>nada</html>"), None);
        assert_eq!(pacote_principal("web-player/web-player.../../x.js"), None, "só letras e números no nome");
    }

    #[test]
    fn o_codigo_sai_do_texto_do_pacote() {
        let js = format!(r#"x=new y("home","query","{A}",null),z=new y("home2","query","{}")"#, "b".repeat(64));
        assert_eq!(codigo_no_texto(&js, "home").as_deref(), Some(A));
        assert_eq!(codigo_no_texto(&js, "outra"), None);
        assert_eq!(codigo_no_texto(&format!(r#""home","query","{}""#, "a".repeat(63)), "home"), None, "curto");
        assert_eq!(codigo_no_texto(&format!(r#""home","query","{}""#, "g".repeat(64)), "home"), None, "não é hexa");
        assert_eq!(codigo_no_texto(&format!(r#""home","mutation","{A}""#), "home"), None, "só leituras");
    }

    #[test]
    fn os_pedacos_da_rota_saem_do_mapa_do_pacote() {
        let js = r#"{12:"outra",4406:"xpui-routes-search",4407:"xpui-routes-search-x"}[e]+"."+{12:"00000000",4406:"729334aa"}[e]+".css",{4406:"af326f70",14406:"11111111"}[e]+".js""#;
        let achados = pedacos_da_rota(js, "xpui-routes-search");
        assert_eq!(achados, ["xpui-routes-search.729334aa.js", "xpui-routes-search.af326f70.js"]);
        assert!(pedacos_da_rota(js, "nao-tem").is_empty());
        let acento = r#"é4406:"xpui-routes-search"}{4406:"af326f70""#;
        assert_eq!(pedacos_da_rota(acento, "xpui-routes-search"), ["xpui-routes-search.af326f70.js"]);
    }

    #[test]
    fn o_relido_vale_mais_e_fica_guardado() {
        let pasta = std::env::temp_dir().join(format!("provedor-spotify-codigos-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).expect("pasta");
        let arquivo = pasta.join("codigos.json");
        let _ = std::fs::remove_file(&arquivo);
        let c = Codigos::novo(arquivo.clone());
        assert_eq!(c.de("home").first().map(String::len), Some(64));
        assert!(c.de("apagarTudo").is_empty(), "só as operações deste provedor");
        c.guardar("home", A);
        c.guardar("apagarTudo", A);
        c.guardar("getAlbum", "curto");
        let outro = Codigos::novo(arquivo);
        assert_eq!(outro.de("home").len(), 2, "o relido e, se ele for recusado, o embutido");
        assert_eq!(outro.de("home")[0], A, "vale na próxima abertura");
        assert!(outro.de("apagarTudo").is_empty());
        assert_eq!(outro.de("getAlbum").len(), 1);
        outro.esquecer("home");
        assert_eq!(outro.de("home").len(), 1);
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
