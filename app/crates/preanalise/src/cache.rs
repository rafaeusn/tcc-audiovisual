//! O cache das análises (`analises.db`): cada música é analisada uma vez.
//!
//! A análise vale enquanto o arquivo tem o mesmo tamanho e a mesma data de modificação, e foi feita pela mesma
//! [`VERSAO`](crate::VERSAO) da análise. Mudou qualquer um, a música é analisada de novo.

use crate::{Analise, VERSAO};
use nucleo::{Erro, Resultado};
use rusqlite::{Connection, OptionalExtension, params};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

fn erro(e: rusqlite::Error) -> Erro {
    Erro::Dados(e.to_string())
}

/// Tamanho e data de modificação (segundos) do arquivo: a "impressão digital" barata dele.
pub fn impressao(caminho: &Path) -> Option<(i64, i64)> {
    let m = std::fs::metadata(caminho).ok()?;
    let modificado = m.modified().ok()?.duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    Some((m.len() as i64, modificado))
}

pub struct Cache {
    con: Connection,
}

impl Cache {
    /// Abre (ou cria). Um arquivo estragado é guardado como `.corrompido` e o cache recomeça vazio (as músicas
    /// são analisadas de novo, nada se perde).
    pub fn abrir(caminho: &Path) -> Resultado<Self> {
        match Self::preparar(Connection::open(caminho).map_err(erro)?) {
            Ok(c) => Ok(c),
            Err(e) => {
                log::warn!("cache de análises ilegível ({e}); começando de novo");
                let _ = std::fs::rename(caminho, caminho.with_extension("db.corrompido"));
                Self::preparar(Connection::open(caminho).map_err(erro)?)
            }
        }
    }

    pub fn em_memoria() -> Resultado<Self> {
        Self::preparar(Connection::open_in_memory().map_err(erro)?)
    }

    fn preparar(con: Connection) -> Resultado<Self> {
        let _ = con.pragma_update(None, "journal_mode", "WAL");
        let _ = con.pragma_update(None, "synchronous", "NORMAL");
        con.execute_batch(
            "CREATE TABLE IF NOT EXISTS analises (
                 caminho    TEXT PRIMARY KEY,
                 tamanho    INTEGER NOT NULL,
                 modificado INTEGER NOT NULL,
                 versao     INTEGER NOT NULL,
                 lufs       REAL,
                 pico       REAL NOT NULL,
                 dados      TEXT NOT NULL
             );",
        )
        .map_err(erro)?;
        con.query_row("SELECT COUNT(*) FROM analises", [], |l| l.get::<_, i64>(0)).map_err(erro)?;
        Ok(Self { con })
    }

    /// A análise guardada, se ainda vale para o arquivo como ele está agora.
    pub fn ler(&self, caminho: &Path) -> Option<Analise> {
        let (tamanho, modificado) = impressao(caminho)?;
        let dados: Option<String> = self
            .con
            .query_row(
                "SELECT dados FROM analises WHERE caminho = ?1 AND tamanho = ?2 AND modificado = ?3 AND versao = ?4",
                params![caminho.to_string_lossy(), tamanho, modificado, VERSAO],
                |l| l.get(0),
            )
            .optional()
            .ok()
            .flatten();
        dados.and_then(|d| serde_json::from_str(&d).ok())
    }

    /// Já há análise válida para o arquivo.
    pub fn tem(&self, caminho: &Path) -> bool {
        let Some((tamanho, modificado)) = impressao(caminho) else { return false };
        self.con
            .query_row(
                "SELECT 1 FROM analises WHERE caminho = ?1 AND tamanho = ?2 AND modificado = ?3 AND versao = ?4",
                params![caminho.to_string_lossy(), tamanho, modificado, VERSAO],
                |_| Ok(()),
            )
            .optional()
            .ok()
            .flatten()
            .is_some()
    }

    pub fn guardar(&self, caminho: &Path, a: &Analise) -> Resultado<()> {
        let Some((tamanho, modificado)) = impressao(caminho) else {
            return Err(Erro::Dados("o arquivo sumiu antes de guardar a análise".into()));
        };
        let dados = serde_json::to_string(a).map_err(|e| Erro::Dados(e.to_string()))?;
        self.con
            .execute(
                "INSERT OR REPLACE INTO analises (caminho, tamanho, modificado, versao, lufs, pico, dados)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![caminho.to_string_lossy(), tamanho, modificado, VERSAO, a.lufs, a.pico, dados],
            )
            .map_err(erro)?;
        Ok(())
    }

    /// A sonoridade e o pico de todas as músicas já analisadas (para igualar o volume sem abrir cada análise).
    pub fn sonoridades(&self) -> HashMap<PathBuf, (f32, f32)> {
        let Ok(mut c) =
            self.con.prepare("SELECT caminho, lufs, pico FROM analises WHERE versao = ?1 AND lufs IS NOT NULL")
        else {
            return HashMap::new();
        };
        let linhas = c.query_map(params![VERSAO], |l| {
            Ok((PathBuf::from(l.get::<_, String>(0)?), (l.get::<_, f64>(1)? as f32, l.get::<_, f64>(2)? as f32)))
        });
        linhas.map(|ls| ls.filter_map(Result::ok).collect()).unwrap_or_default()
    }

    /// Quantas análises válidas (desta versão) existem.
    pub fn quantas(&self) -> u32 {
        self.con
            .query_row("SELECT COUNT(*) FROM analises WHERE versao = ?1", params![VERSAO], |l| l.get(0))
            .unwrap_or(0)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn guarda_le_e_refaz_quando_o_arquivo_muda() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-cache-analise-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).expect("pasta");
        let arquivo = pasta.join("m.wav");
        std::fs::write(&arquivo, b"conteudo").expect("gravar");
        let c = Cache::em_memoria().expect("cache");
        assert!(c.ler(&arquivo).is_none());
        let a = Analise { versao: VERSAO, duracao_s: 10.0, lufs: Some(-9.0), pico: 0.9, ..Default::default() };
        c.guardar(&arquivo, &a).expect("guardar");
        assert!(c.tem(&arquivo));
        assert_eq!(c.ler(&arquivo), Some(a));
        assert_eq!(c.sonoridades().get(&arquivo), Some(&(-9.0, 0.9)));
        assert_eq!(c.quantas(), 1);
        // O arquivo mudou de tamanho: a análise não vale mais.
        std::fs::write(&arquivo, b"conteudo maior agora").expect("gravar");
        assert!(c.ler(&arquivo).is_none());
        assert!(!c.tem(&arquivo));
        assert!(c.ler(&pasta.join("nao-existe.wav")).is_none());
        let _ = std::fs::remove_dir_all(pasta);
    }

    #[test]
    fn arquivo_estragado_recomeca_vazio() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-cache-estragado-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).expect("pasta");
        let db = pasta.join("analises.db");
        std::fs::write(&db, b"isto nao e um banco de dados, so texto qualquer para estragar o arquivo")
            .expect("gravar");
        let c = Cache::abrir(&db).expect("abre");
        assert_eq!(c.quantas(), 0);
        drop(c);
        let _ = std::fs::remove_dir_all(pasta);
    }
}
