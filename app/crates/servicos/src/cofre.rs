//! Guarda um segredo pequeno (a autorização do Last.fm) cifrado pelo Windows para o usuário atual (DPAPI).
//!
//! O arquivo fica na pasta de dados do app. Copiado para outro computador ou lido por outro usuário, ele não
//! abre. Fora do Windows não há cofre: o segredo não é guardado e a pessoa autoriza de novo a cada abertura.

use nucleo::{Erro, Resultado};
use std::path::Path;

fn para_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn de_hex(texto: &str) -> Option<Vec<u8>> {
    let texto = texto.trim();
    if !texto.len().is_multiple_of(2) || !texto.is_ascii() {
        return None;
    }
    (0..texto.len()).step_by(2).map(|i| u8::from_str_radix(&texto[i..i + 2], 16).ok()).collect()
}

#[cfg(windows)]
fn cifrar(claro: &[u8], abrir: bool) -> Option<Vec<u8>> {
    use windows::Win32::Foundation::{HLOCAL, LocalFree};
    use windows::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CryptProtectData, CryptUnprotectData};
    let entrada = CRYPT_INTEGER_BLOB { cbData: u32::try_from(claro.len()).ok()?, pbData: claro.as_ptr().cast_mut() };
    let mut saida = CRYPT_INTEGER_BLOB::default();
    // SAFETY: `entrada` aponta para `claro`, vivo durante a chamada; `saida` é preenchida pelo Windows com um
    // bloco que copiamos e devolvemos com `LocalFree`.
    unsafe {
        let feito = if abrir {
            CryptUnprotectData(&entrada, None, None, None, None, 0, &mut saida)
        } else {
            CryptProtectData(&entrada, windows::core::PCWSTR::null(), None, None, None, 0, &mut saida)
        };
        if feito.is_err() || saida.pbData.is_null() {
            return None;
        }
        let bytes = std::slice::from_raw_parts(saida.pbData, saida.cbData as usize).to_vec();
        let _ = LocalFree(Some(HLOCAL(saida.pbData.cast())));
        Some(bytes)
    }
}

#[cfg(not(windows))]
fn cifrar(_: &[u8], _: bool) -> Option<Vec<u8>> {
    None
}

/// Guarda `segredo` em `caminho`.
pub fn guardar(caminho: &Path, segredo: &str) -> Resultado<()> {
    let cifrado =
        cifrar(segredo.as_bytes(), false).ok_or_else(|| Erro::Dados("o sistema não cifrou o segredo".into()))?;
    std::fs::write(caminho, para_hex(&cifrado)).map_err(|e| Erro::arquivo(caminho, e))
}

/// Lê o segredo guardado em `caminho`. `None` se não há, ou se não é deste usuário neste computador.
pub fn ler(caminho: &Path) -> Option<String> {
    let cifrado = de_hex(&std::fs::read_to_string(caminho).ok()?)?;
    String::from_utf8(cifrar(&cifrado, true)?).ok()
}

pub fn apagar(caminho: &Path) {
    let _ = std::fs::remove_file(caminho);
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn hex_ida_e_volta() {
        assert_eq!(para_hex(&[0, 15, 255]), "000fff");
        assert_eq!(de_hex("000fff\n"), Some(vec![0, 15, 255]));
        assert_eq!(de_hex("abc"), None);
        assert_eq!(de_hex("zz"), None);
        assert_eq!(de_hex(""), Some(vec![]));
    }

    #[cfg(windows)]
    #[test]
    fn guarda_cifrado_e_le_de_volta() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-teste-cofre-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).expect("pasta");
        let arquivo = pasta.join("conta.cofre");
        assert_eq!(ler(&arquivo), None);
        guardar(&arquivo, "fulana|chave-de-sessao-123").expect("guardar");
        let no_disco = std::fs::read_to_string(&arquivo).expect("ler arquivo");
        assert!(!no_disco.contains("chave-de-sessao"), "não fica em texto claro");
        assert_eq!(ler(&arquivo).as_deref(), Some("fulana|chave-de-sessao-123"));
        // Arquivo adulterado não abre.
        std::fs::write(&arquivo, "00ff00ff").expect("escrever");
        assert_eq!(ler(&arquivo), None);
        apagar(&arquivo);
        assert!(!arquivo.exists());
        let _ = std::fs::remove_dir_all(pasta);
    }
}
