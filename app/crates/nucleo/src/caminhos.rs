//! Pastas do usuário: tudo fica em `%APPDATA%\<app>` (sem conta, sem nuvem).
//!
//! A variável de ambiente `TCCPLAYER_DADOS` troca essa pasta por outra. Serve para testar (e para um uso
//! "portátil"): com ela o app não toca nos ajustes nem no histórico de sempre, e pode rodar ao lado de outra
//! cópia que esteja aberta.

use crate::{Erro, NOME_APP, Resultado};
use std::path::{Path, PathBuf};

/// Nome da variável de ambiente que escolhe outra pasta de dados.
pub const VARIAVEL_DE_DADOS: &str = "TCCPLAYER_DADOS";

/// A pasta escolhida por [`VARIAVEL_DE_DADOS`], se houver.
pub fn pasta_alternativa() -> Option<PathBuf> {
    std::env::var_os(VARIAVEL_DE_DADOS).filter(|v| !v.is_empty()).map(PathBuf::from)
}

/// Pasta de dados do app, criada se não existir.
pub fn pasta_dados() -> Resultado<PathBuf> {
    if let Some(pasta) = pasta_alternativa() {
        std::fs::create_dir_all(&pasta).map_err(|e| Erro::arquivo(&pasta, e))?;
        return Ok(pasta);
    }
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(std::env::temp_dir);
    let pasta = base.join(NOME_APP);
    std::fs::create_dir_all(&pasta).map_err(|e| Erro::arquivo(&pasta, e))?;
    Ok(pasta)
}

/// Um número que identifica a pasta de dados (sem diferença de maiúsculas nem de barras): duas cópias do Player na
/// mesma pasta disputariam os bancos e o tocador do serviço, então a segunda chama a primeira e fecha.
pub fn chave_da_pasta(pasta: &Path) -> u32 {
    let texto = pasta.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase();
    // FNV-1a: o mesmo número em qualquer versão do app (o hash da biblioteca padrão pode mudar).
    let n = texto.bytes().fold(0x811c_9dc5u32, |h, b| (h ^ u32::from(b)).wrapping_mul(0x0100_0193));
    n.max(1)
}

pub fn arquivo_ajustes() -> Resultado<PathBuf> {
    Ok(pasta_dados()?.join("ajustes.toml"))
}

/// Banco local com o histórico do que foi ouvido e as letras já buscadas.
pub fn arquivo_banco() -> Resultado<PathBuf> {
    Ok(pasta_dados()?.join("historico.db"))
}

/// Lista dos módulos instalados e do que foi aceito de cada um.
pub fn arquivo_modulos() -> Resultado<PathBuf> {
    Ok(pasta_dados()?.join("modulos.toml"))
}

/// Onde ficam os provedores instalados (uma pasta por provedor, com o `provedor.toml`).
pub fn pasta_provedores() -> Resultado<PathBuf> {
    Ok(pasta_dados()?.join("provedores"))
}

pub fn pasta_registros() -> Resultado<PathBuf> {
    let pasta = pasta_dados()?.join("registros");
    std::fs::create_dir_all(&pasta).map_err(|e| Erro::arquivo(&pasta, e))?;
    Ok(pasta)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_mesma_pasta_tem_a_mesma_chave() {
        let a = chave_da_pasta(Path::new(r"C:\Users\ana\AppData\Roaming\TccPlayer"));
        assert_eq!(a, chave_da_pasta(Path::new("c:/users/ana/appdata/roaming/tccplayer/")));
        assert_ne!(a, chave_da_pasta(Path::new("D:/teste")));
        assert_ne!(chave_da_pasta(Path::new("")), 0, "zero quer dizer sem chave");
    }
}
