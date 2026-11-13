//! Pastas do usuário: tudo fica em `%APPDATA%\<app>` (sem conta, sem nuvem).
//!
//! A variável de ambiente `TCCPLAYER_DADOS` troca essa pasta por outra. Serve para testar (e para um uso
//! "portátil"): com ela o app não toca nos ajustes nem no histórico de sempre, e pode rodar ao lado de outra
//! cópia que esteja aberta.

use crate::{Erro, NOME_APP, Resultado};
use std::path::PathBuf;

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

pub fn pasta_registros() -> Resultado<PathBuf> {
    let pasta = pasta_dados()?.join("registros");
    std::fs::create_dir_all(&pasta).map_err(|e| Erro::arquivo(&pasta, e))?;
    Ok(pasta)
}
