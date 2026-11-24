//! Quem está escutando numa porta TCP local (para dizer à pessoa qual programa fechar antes de entrar).

/// O nome do programa (`Spotifast.exe`) que escuta em `127.0.0.1:porta` ou `0.0.0.0:porta`. `None` sem dono, sem
/// permissão para ver o processo, ou fora do Windows.
#[cfg(windows)]
pub fn dono(porta: u16) -> Option<String> {
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    };
    use windows::Win32::Networking::WinSock::AF_INET;
    let familia = u32::from(AF_INET.0);
    let mut tamanho = 0u32;
    // SAFETY: a primeira chamada só pede o tamanho (sem tabela); a segunda escreve no `buf` desse tamanho, alinhado
    // a 4 bytes (um vetor de u32), como a tabela pede.
    let tabela = unsafe {
        let _ = GetExtendedTcpTable(None, &mut tamanho, false, familia, TCP_TABLE_OWNER_PID_LISTENER, 0);
        let mut buf = vec![0u32; (tamanho as usize).div_ceil(4) + 1];
        let r = GetExtendedTcpTable(
            Some(buf.as_mut_ptr().cast()),
            &mut tamanho,
            false,
            familia,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
        if r != 0 {
            return None;
        }
        let cabeca = &*buf.as_ptr().cast::<MIB_TCPTABLE_OWNER_PID>();
        let linhas = std::slice::from_raw_parts(cabeca.table.as_ptr(), cabeca.dwNumEntries as usize);
        linhas.to_vec()
    };
    // A porta vem na ordem da rede, nos 16 bits de baixo.
    let porta_de = |l: &MIB_TCPROW_OWNER_PID| u16::from_be((l.dwLocalPort & 0xFFFF) as u16);
    let pid = tabela.iter().find(|l| porta_de(l) == porta)?.dwOwningPid;
    nome_do_processo(pid)
}

#[cfg(windows)]
fn nome_do_processo(pid: u32) -> Option<String> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
    };
    let mut nome = [0u16; 1024];
    let mut tamanho = nome.len() as u32;
    // SAFETY: o handle aberto aqui é fechado logo depois; o Windows escreve no máximo `tamanho` caracteres em `nome`.
    let caminho = unsafe {
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let feito =
            QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, windows::core::PWSTR(nome.as_mut_ptr()), &mut tamanho);
        let _ = CloseHandle(h);
        feito.ok()?;
        String::from_utf16_lossy(&nome[..tamanho as usize])
    };
    std::path::Path::new(&caminho).file_name().map(|n| n.to_string_lossy().into_owned())
}

#[cfg(not(windows))]
pub fn dono(_porta: u16) -> Option<String> {
    None
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn porta_livre_nao_tem_dono() {
        if !cfg!(windows) {
            return;
        }
        let ouvinte = std::net::TcpListener::bind("127.0.0.1:0").expect("porta");
        let porta = ouvinte.local_addr().expect("endereço").port();
        let nome = dono(porta).expect("este teste escuta nela");
        assert!(nome.to_lowercase().ends_with(".exe"), "{nome}");
        assert!(nome.starts_with("provedor_spotify"), "o próprio exe do teste: {nome}");
        drop(ouvinte);
        assert_eq!(dono(porta), None, "fechada, ninguém");
    }
}
