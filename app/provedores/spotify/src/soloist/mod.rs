//! Tocar pelo Soloist, o player oficial do Spotify sem janela (developer.spotify.com/documentation/soloist). O programa é
//! do Spotify, baixado quando a pessoa o liga, com a chave pessoal dela; o som sai por ele. Aqui só se abre, se pareia e
//! se manda nele pela API local (`--ws`). Nada do áudio passa pelo Player.

mod assinatura;
mod instalar;
mod leitura;
mod motor;
mod programa;

pub use motor::Soloist;
pub use programa::DoSpotify;

/// Um programa de console sem a janela preta.
fn sem_janela(comando: &mut std::process::Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW
        comando.creation_flags(0x0800_0000);
    }
    #[cfg(not(windows))]
    let _ = comando;
}

/// O Windows não deixou (erro 4551): tentar de novo sozinho só baixaria o programa outra vez.
pub(crate) const BLOQUEADO: &str = "o Controle Inteligente de Aplicativos do Windows bloqueou o Soloist";

/// O Soloist que o Spotify publica agora cai neste computador.
pub(crate) const QUEBRADO: &str = "o Soloist que o Spotify publica agora está quebrando neste computador";

/// Por que o programa não abriu, para a pessoa.
fn nao_abriu(e: &std::io::Error) -> String {
    match e.raw_os_error() {
        // ERROR_SYSTEM_INTEGRITY_POLICY_VIOLATION
        Some(4551) => BLOQUEADO.into(),
        _ => format!("o Soloist não abriu: {e}"),
    }
}
