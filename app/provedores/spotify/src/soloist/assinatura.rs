//! De quem é a assinatura de um programa: só se abre um Soloist assinado pelo Spotify, venha ele do servidor deles ou de
//! outro programa deste computador.

use std::path::Path;

/// Quem assina o Soloist.
const SPOTIFY: &str = "Spotify AB";

/// O programa tem uma assinatura válida do Spotify.
pub fn do_spotify(exe: &Path) -> bool {
    organizacao(exe).as_deref() == Some(SPOTIFY)
}

/// A organização da assinatura do arquivo, se ela vale (o arquivo não mudou e a cadeia do certificado é de confiança).
#[cfg(windows)]
pub fn organizacao(arquivo: &Path) -> Option<String> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Foundation::{HANDLE, HWND};
    use windows::Win32::Security::WinTrust::{
        WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0, WINTRUST_FILE_INFO, WTD_CHOICE_FILE,
        WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY, WTD_UI_NONE, WinVerifyTrust,
    };
    let caminho: Vec<u16> = arquivo.as_os_str().encode_wide().chain([0]).collect();
    let mut informacao = WINTRUST_FILE_INFO {
        cbStruct: size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: windows::core::PCWSTR(caminho.as_ptr()),
        hFile: HANDLE::default(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut dados = WINTRUST_DATA {
        cbStruct: size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_NONE,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &raw mut informacao },
        dwStateAction: WTD_STATEACTION_VERIFY,
        ..Default::default()
    };
    let mut acao = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    // SAFETY: as estruturas vivem até o fim desta função e levam o tamanho certo; o estado aberto pela conferência é
    // lido enquanto vale e fechado na segunda chamada, sempre.
    unsafe {
        let vale = WinVerifyTrust(HWND::default(), &raw mut acao, (&raw mut dados).cast()) == 0;
        let nome = vale.then(|| nome_do_assinante(dados.hWVTStateData)).flatten();
        dados.dwStateAction = WTD_STATEACTION_CLOSE;
        WinVerifyTrust(HWND::default(), &raw mut acao, (&raw mut dados).cast());
        nome
    }
}

/// O nome da organização no certificado de quem assinou.
///
/// # Safety
/// `estado` é o de uma conferência aberta agora por `WinVerifyTrust`.
#[cfg(windows)]
unsafe fn nome_do_assinante(estado: windows::Win32::Foundation::HANDLE) -> Option<String> {
    use windows::Win32::Security::Cryptography::{CERT_NAME_ATTR_TYPE, CertGetNameStringW, szOID_ORGANIZATION_NAME};
    use windows::Win32::Security::WinTrust::{WTHelperGetProvSignerFromChain, WTHelperProvDataFromStateData};
    // SAFETY: os ponteiros vêm do Windows para este estado aberto, e cada um é conferido antes de ser seguido.
    unsafe {
        let provedor = WTHelperProvDataFromStateData(estado);
        if provedor.is_null() {
            return None;
        }
        let assinante = WTHelperGetProvSignerFromChain(provedor, 0, false, 0);
        if assinante.is_null() || (*assinante).csCertChain == 0 || (*assinante).pasCertChain.is_null() {
            return None;
        }
        let certificado = (*(*assinante).pasCertChain).pCert;
        if certificado.is_null() {
            return None;
        }
        let mut nome = [0u16; 256];
        let tipo = Some(szOID_ORGANIZATION_NAME.0.cast());
        let n = CertGetNameStringW(certificado, CERT_NAME_ATTR_TYPE, 0, tipo, Some(&mut nome)) as usize;
        // (O tamanho conta o zero do fim.)
        (n > 1).then(|| String::from_utf16_lossy(&nome[..n - 1]))
    }
}

#[cfg(not(windows))]
pub fn organizacao(_: &Path) -> Option<String> {
    None
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn sem_assinatura_nao_e_do_spotify() {
        let arquivo = std::env::temp_dir().join(format!("tccplayer-assinatura-{}.exe", std::process::id()));
        std::fs::write(&arquivo, b"MZ nada").expect("arquivo");
        assert!(!do_spotify(&arquivo));
        let _ = std::fs::remove_file(&arquivo);
        assert!(!do_spotify(&arquivo), "nem o que não existe");
        assert!(!do_spotify(&std::env::current_exe().expect("este teste")), "nem um programa sem assinatura");
    }

    #[cfg(windows)]
    #[test]
    fn a_assinatura_de_outro_nao_passa_por_spotify() {
        // Um programa do Windows com a assinatura dentro do arquivo.
        let raiz = std::env::var_os("SystemRoot").unwrap_or_else(|| r"C:\Windows".into());
        let explorer = Path::new(&raiz).join("explorer.exe");
        let dono = organizacao(&explorer).expect("o explorer.exe é assinado");
        assert!(dono.contains("Microsoft"), "{dono}");
        assert!(!do_spotify(&explorer));
    }
}
