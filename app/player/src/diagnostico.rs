//! O relatório de diagnóstico: o texto que a pessoa cola ao relatar um problema.
//!
//! Junta o que sempre se pergunta (versão, sistema, placa de vídeo, saída de som, ajustes) e o fim do registro.
//! O texto vai para um lugar público, então o nome do usuário do Windows é apagado de tudo, inclusive dos
//! caminhos de arquivo. Na dúvida apaga a mais: perder um detalhe custa uma pergunta, vazar não tem volta.

use std::path::Path;

/// Quantas linhas do fim do registro entram no relatório.
const LINHAS_DO_REGISTRO: usize = 80;
/// Só o fim do arquivo é lido: o registro pode ter meio megabyte.
const BYTES_LIDOS: u64 = 64 * 1024;

pub struct Dados {
    pub versao: String,
    pub sistema: String,
    pub placa: String,
    pub som: String,
    pub ajustes: String,
    pub historico: bool,
}

/// Troca o nome do usuário por `<usuário>`, sem diferenciar maiúsculas.
pub fn redigir(texto: &str, usuario: &str) -> String {
    let usuario = usuario.trim();
    // Um nome de uma letra apagaria o texto inteiro.
    if usuario.chars().count() < 2 {
        return texto.to_string();
    }
    let (baixo, alvo) = (texto.to_lowercase(), usuario.to_lowercase());
    // Só dá para cortar pelo índice se minúsculas e original tiverem o mesmo tamanho (quase sempre têm).
    if baixo.len() != texto.len() {
        return texto.replace(usuario, "<usuário>");
    }
    let mut saida = String::with_capacity(texto.len());
    let mut resto = 0;
    while let Some(i) = baixo[resto..].find(&alvo) {
        saida.push_str(&texto[resto..resto + i]);
        saida.push_str("<usuário>");
        resto += i + alvo.len();
    }
    saida.push_str(&texto[resto..]);
    saida
}

/// As últimas linhas do arquivo de registro (vazio se não der para ler).
pub fn fim_do_registro(caminho: &Path) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let ler = || -> std::io::Result<String> {
        let mut arquivo = std::fs::File::open(caminho)?;
        let tamanho = arquivo.metadata()?.len();
        arquivo.seek(SeekFrom::Start(tamanho.saturating_sub(BYTES_LIDOS)))?;
        let mut bytes = Vec::new();
        arquivo.read_to_end(&mut bytes)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned())
    };
    let texto = ler().unwrap_or_default();
    let linhas: Vec<&str> = texto.lines().collect();
    linhas[linhas.len().saturating_sub(LINHAS_DO_REGISTRO)..].join("\n")
}

pub fn montar(d: &Dados, registro: &str, usuario: &str) -> String {
    let texto = format!(
        "Relatório de diagnóstico do Player\n\
         Versão: {}\n\
         Sistema: {}\n\
         Placa de vídeo: {}\n\
         Som: {}\n\
         Histórico: {}\n\
         Ajustes: {}\n\
         \n\
         Fim do registro:\n{}\n",
        d.versao,
        d.sistema,
        if d.placa.is_empty() { "indisponível" } else { &d.placa },
        d.som,
        if d.historico { "ligado" } else { "desligado" },
        d.ajustes,
        if registro.trim().is_empty() { "(vazio)" } else { registro.trim_end() },
    );
    redigir(&texto, usuario)
}

/// Versão do Windows ("Windows 10.0.26200, x86_64").
pub fn sistema() -> String {
    #[cfg(windows)]
    {
        use windows::Win32::System::SystemInformation::OSVERSIONINFOW;
        let mut v = OSVERSIONINFOW { dwOSVersionInfoSize: size_of::<OSVERSIONINFOW>() as u32, ..Default::default() };
        // SAFETY: a estrutura é local, do tamanho informado, e a função só a preenche.
        let _ = unsafe { windows::Wdk::System::SystemServices::RtlGetVersion(&mut v) };
        format!("Windows {}.{}.{}, {}", v.dwMajorVersion, v.dwMinorVersion, v.dwBuildNumber, std::env::consts::ARCH)
    }
    #[cfg(not(windows))]
    {
        format!("{}, {}", std::env::consts::OS, std::env::consts::ARCH)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn apaga_o_nome_do_usuario() {
        let texto = r"erro em C:\Users\Fulano\AppData\Roaming\TccPlayer e em c:\users\fulano\x; FULANO-PC";
        assert_eq!(
            redigir(texto, "Fulano"),
            r"erro em C:\Users\<usuário>\AppData\Roaming\TccPlayer e em c:\users\<usuário>\x; <usuário>-PC"
        );
        assert_eq!(redigir("nada aqui", "Fulano"), "nada aqui");
        assert_eq!(redigir("abc", "a"), "abc", "nome curto demais não apaga nada");
        assert_eq!(redigir("abc", ""), "abc");
    }

    #[test]
    fn relatorio_completo_e_sem_o_nome() {
        let d = Dados {
            versao: "0.1.0".into(),
            sistema: "Windows 10.0.26200, x86_64".into(),
            placa: String::new(),
            som: "Ouvindo Fones (Fulano)".into(),
            ajustes: "tema Noite".into(),
            historico: true,
        };
        let r = montar(&d, "linha 1\nlinha 2 de fulano\n", "Fulano");
        assert!(r.starts_with("Relatório de diagnóstico do Player\nVersão: 0.1.0\n"));
        assert!(r.contains("Placa de vídeo: indisponível"));
        assert!(r.contains("Histórico: ligado"));
        assert!(r.contains("linha 2 de <usuário>"));
        assert!(!r.to_lowercase().contains("fulano"));
        assert!(montar(&d, "  ", "").contains("(vazio)"));
    }

    #[test]
    fn so_o_fim_do_registro() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-teste-diag-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&pasta);
        let arquivo = pasta.join("player.log");
        let texto: String = (0..500).map(|i| format!("linha {i}\n")).collect();
        let _ = std::fs::write(&arquivo, texto);
        let fim = fim_do_registro(&arquivo);
        assert_eq!(fim.lines().count(), LINHAS_DO_REGISTRO);
        assert!(fim.ends_with("linha 499"));
        assert!(fim.starts_with("linha 420"));
        assert_eq!(fim_do_registro(&pasta.join("nao-existe.log")), "");
        let _ = std::fs::remove_dir_all(pasta);
    }
}
