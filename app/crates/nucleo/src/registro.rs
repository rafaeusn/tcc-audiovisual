//! Registro (log) em arquivo, com teto de tamanho. Sem dados pessoais: só eventos técnicos e erros.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

const TETO_BYTES: u64 = 512 * 1024;
const NOSSOS: [&str; 7] = ["player", "nucleo", "audio", "analise", "sessao", "visual", "janela"];

struct Registro {
    arquivo: Mutex<Option<File>>,
    nivel: log::LevelFilter,
}

impl log::Log for Registro {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= self.nivel
    }

    fn log(&self, r: &log::Record) {
        // Só as nossas partes, mais avisos e erros das bibliotecas: o wgpu e o GPUI falam muito em `info`.
        let nosso = r.target().split("::").next().is_some_and(|c| NOSSOS.contains(&c));
        if !self.enabled(r.metadata()) || !(nosso || r.level() <= log::Level::Warn) {
            return;
        }
        let seg = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let linha = format!("{seg} {:5} {}: {}\n", r.level(), r.target(), r.args());
        eprint!("{linha}");
        if let Ok(mut g) = self.arquivo.lock()
            && let Some(f) = g.as_mut()
        {
            let _ = f.write_all(linha.as_bytes());
        }
    }

    fn flush(&self) {
        if let Ok(mut g) = self.arquivo.lock()
            && let Some(f) = g.as_mut()
        {
            let _ = f.flush();
        }
    }
}

/// Liga o registro. Se o arquivo não puder ser aberto, o registro segue só no terminal.
pub fn iniciar(pasta: Option<&Path>) {
    let arquivo = pasta.and_then(|p| {
        let caminho = p.join("player.log");
        // Arquivo grande demais: guarda o anterior e começa outro.
        if std::fs::metadata(&caminho).is_ok_and(|m| m.len() > TETO_BYTES) {
            let _ = std::fs::rename(&caminho, p.join("player.anterior.log"));
        }
        OpenOptions::new().create(true).append(true).open(caminho).ok()
    });
    let nivel = if cfg!(debug_assertions) { log::LevelFilter::Debug } else { log::LevelFilter::Info };
    let registro = Registro { arquivo: Mutex::new(arquivo), nivel };
    if log::set_boxed_logger(Box::new(registro)).is_ok() {
        log::set_max_level(nivel);
    }
}
