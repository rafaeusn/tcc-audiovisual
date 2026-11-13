//! O histórico do Player: cada música ouvida vira uma linha num arquivo SQLite local.
//!
//! - Nada sai do computador. O arquivo fica na pasta de dados do app.
//! - As estatísticas são calculadas na hora de mostrar, a partir das linhas: um ano de uso pesado são dezenas de
//!   milhares de linhas, que o SQLite agrupa em milissegundos. Não há contadores para manter em dia.
//! - O mesmo arquivo guarda as letras já buscadas, para não pedir de novo a cada vez que a música toca.
//!
//! Quem usa é uma thread própria do app: a interface nunca espera o disco.

mod data;

pub use data::{data_civil, dia_local};

use nucleo::{Erro, Resultado};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;

/// Uma "não tem letra" guardada vale por este tempo; depois o app pergunta de novo.
const VALIDADE_SEM_LETRA_S: i64 = 24 * 3600;
const SEGUNDOS_POR_DIA: i64 = 86_400;

/// Uma música ouvida. Os horários são segundos desde 1970 (UTC).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Execucao {
    pub inicio: i64,
    pub titulo: String,
    pub artista: String,
    pub album: String,
    /// App de origem ("Spotify", "Chrome"...).
    pub app: String,
    /// Duração da faixa; 0 quando o app de origem não informa.
    pub duracao_s: f64,
    /// Quanto foi ouvido de verdade (pausas e silêncio não contam).
    pub ouvido_s: f64,
    /// BPM médio medido enquanto tocava; 0 quando não houve pulso firme.
    pub bpm: f32,
    /// Energia média (0 a 1) medida enquanto tocava.
    pub energia: f32,
}

/// Um item de uma lista "mais ouvidos".
#[derive(Clone, Debug, PartialEq)]
pub struct Top {
    pub nome: String,
    /// O artista, nas músicas; vazio nos artistas.
    pub detalhe: String,
    pub segundos: f64,
    pub vezes: u32,
}

/// Os números de um período.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Resumo {
    pub segundos: f64,
    /// Tempo ouvido no período anterior, de mesmo tamanho (só quando o período tem começo).
    pub segundos_antes: Option<f64>,
    pub execucoes: u32,
    pub musicas: u32,
    pub artistas: u32,
    /// 0 quando não há medida.
    pub bpm_medio: f32,
    pub energia_media: f32,
    /// Segundos ouvidos por hora do dia (hora local em que a música começou).
    pub por_hora: [f64; 24],
    /// (dia local, contado desde 1970; segundos ouvidos), do mais antigo ao mais novo.
    pub por_dia: Vec<(i64, f64)>,
    pub top_musicas: Vec<Top>,
    pub top_artistas: Vec<Top>,
    /// Dias seguidos com música até hoje (ou até ontem, se hoje ainda não tocou nada). Conta o histórico todo.
    pub sequencia: u32,
    pub recentes: Vec<Execucao>,
}

/// O que se quer saber.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Consulta {
    /// Começo do período (`None` = desde sempre).
    pub desde: Option<i64>,
    pub agora: i64,
    /// Diferença da hora local para UTC, em segundos (Brasília: -10800).
    pub fuso_s: i64,
    pub tops: u32,
    pub recentes: u32,
}

/// Uma letra guardada, como veio da fonte.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LetraGuardada {
    /// Texto no formato LRC (com os tempos).
    pub sincronizada: Option<String>,
    pub simples: Option<String>,
    pub instrumental: bool,
}

/// O que o banco sabe sobre a letra de uma música.
#[derive(Clone, Debug, PartialEq)]
pub enum Guardada {
    /// Nunca buscada (ou a resposta "não tem" já venceu).
    Nada,
    /// Buscada há menos de um dia e não existe.
    SemLetra,
    Letra(LetraGuardada),
}

pub struct Banco {
    con: Connection,
}

fn erro(e: rusqlite::Error) -> Erro {
    Erro::Dados(e.to_string())
}

impl Banco {
    /// Abre (ou cria) o arquivo. Um arquivo estragado é guardado como `.corrompido` e o histórico recomeça vazio.
    pub fn abrir(caminho: &Path) -> Resultado<Self> {
        match Self::preparar(Connection::open(caminho).map_err(erro)?) {
            Ok(banco) => Ok(banco),
            Err(e) => {
                log::warn!("histórico ilegível ({e}); guardando como .corrompido e começando de novo");
                let _ = std::fs::rename(caminho, caminho.with_extension("db.corrompido"));
                Self::preparar(Connection::open(caminho).map_err(erro)?)
            }
        }
    }

    /// Banco só na memória (testes, ou quando não há pasta de dados).
    pub fn em_memoria() -> Resultado<Self> {
        Self::preparar(Connection::open_in_memory().map_err(erro)?)
    }

    fn preparar(con: Connection) -> Resultado<Self> {
        // WAL: gravar não bloqueia quem lê (a thread das letras usa o mesmo arquivo).
        let _ = con.pragma_update(None, "journal_mode", "WAL");
        let _ = con.pragma_update(None, "synchronous", "NORMAL");
        let _ = con.busy_timeout(std::time::Duration::from_secs(2));
        con.execute_batch(
            "CREATE TABLE IF NOT EXISTS execucoes (
                 id        INTEGER PRIMARY KEY,
                 inicio    INTEGER NOT NULL,
                 titulo    TEXT    NOT NULL,
                 artista   TEXT    NOT NULL,
                 album     TEXT    NOT NULL,
                 app       TEXT    NOT NULL,
                 duracao_s REAL    NOT NULL,
                 ouvido_s  REAL    NOT NULL,
                 bpm       REAL    NOT NULL,
                 energia   REAL    NOT NULL
             );
             CREATE INDEX IF NOT EXISTS execucoes_inicio ON execucoes(inicio);
             CREATE TABLE IF NOT EXISTS letras (
                 chave        TEXT PRIMARY KEY,
                 sincronizada TEXT,
                 simples      TEXT,
                 instrumental INTEGER NOT NULL,
                 buscada_em   INTEGER NOT NULL
             );",
        )
        .map_err(erro)?;
        // Uma consulta de verdade: um arquivo que não é um banco só falha aqui.
        con.query_row("SELECT COUNT(*) FROM execucoes", [], |l| l.get::<_, i64>(0)).map_err(erro)?;
        Ok(Self { con })
    }

    // ------------------------------------------------------------------ execuções

    /// Guarda uma execução nova e devolve o número dela (para atualizar depois, enquanto a música ainda toca).
    pub fn registrar(&self, e: &Execucao) -> Resultado<i64> {
        self.con
            .execute(
                "INSERT INTO execucoes (inicio, titulo, artista, album, app, duracao_s, ouvido_s, bpm, energia)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    e.inicio,
                    e.titulo,
                    e.artista,
                    e.album,
                    e.app,
                    finito(e.duracao_s),
                    finito(e.ouvido_s),
                    finito(f64::from(e.bpm)),
                    finito(f64::from(e.energia)),
                ],
            )
            .map_err(erro)?;
        Ok(self.con.last_insert_rowid())
    }

    /// Atualiza o que muda enquanto a música toca: o tempo ouvido, a duração (os apps informam depois) e as médias.
    pub fn atualizar(&self, id: i64, e: &Execucao) -> Resultado<()> {
        self.con
            .execute(
                "UPDATE execucoes SET duracao_s = ?2, ouvido_s = ?3, bpm = ?4, energia = ?5 WHERE id = ?1",
                params![
                    id,
                    finito(e.duracao_s),
                    finito(e.ouvido_s),
                    finito(f64::from(e.bpm)),
                    finito(f64::from(e.energia))
                ],
            )
            .map_err(erro)?;
        Ok(())
    }

    pub fn recentes(&self, limite: u32) -> Resultado<Vec<Execucao>> {
        let mut consulta = self
            .con
            .prepare(
                "SELECT inicio, titulo, artista, album, app, duracao_s, ouvido_s, bpm, energia
                 FROM execucoes ORDER BY inicio DESC, id DESC LIMIT ?1",
            )
            .map_err(erro)?;
        let linhas = consulta
            .query_map(params![limite], |l| {
                Ok(Execucao {
                    inicio: l.get(0)?,
                    titulo: l.get(1)?,
                    artista: l.get(2)?,
                    album: l.get(3)?,
                    app: l.get(4)?,
                    duracao_s: l.get(5)?,
                    ouvido_s: l.get(6)?,
                    bpm: l.get::<_, f64>(7)? as f32,
                    energia: l.get::<_, f64>(8)? as f32,
                })
            })
            .map_err(erro)?;
        linhas.collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    /// Os números de um período.
    pub fn resumo(&self, c: Consulta) -> Resultado<Resumo> {
        let desde = c.desde.unwrap_or(i64::MIN);
        let mut r = Resumo::default();

        (r.segundos, r.execucoes, r.musicas, r.artistas) = self
            .con
            .query_row(
                "SELECT COALESCE(SUM(ouvido_s), 0), COUNT(*),
                        COUNT(DISTINCT lower(titulo) || char(31) || lower(artista)),
                        COUNT(DISTINCT CASE WHEN artista <> '' THEN lower(artista) END)
                 FROM execucoes WHERE inicio >= ?1",
                params![desde],
                |l| Ok((l.get(0)?, l.get(1)?, l.get(2)?, l.get(3)?)),
            )
            .map_err(erro)?;

        // Médias pesadas pelo tempo ouvido: uma música inteira pesa mais que um trecho.
        let media = |coluna: &str| -> Resultado<f32> {
            let sql = format!(
                "SELECT COALESCE(SUM({coluna} * ouvido_s) / NULLIF(SUM(ouvido_s), 0), 0)
                 FROM execucoes WHERE inicio >= ?1 AND {coluna} > 0"
            );
            self.con.query_row(&sql, params![desde], |l| l.get::<_, f64>(0)).map(|v| v as f32).map_err(erro)
        };
        r.bpm_medio = media("bpm")?;
        r.energia_media = media("energia")?;

        if let Some(inicio) = c.desde {
            let tamanho = (c.agora - inicio).max(0);
            r.segundos_antes = Some(
                self.con
                    .query_row(
                        "SELECT COALESCE(SUM(ouvido_s), 0) FROM execucoes WHERE inicio >= ?1 AND inicio < ?2",
                        params![inicio - tamanho, inicio],
                        |l| l.get(0),
                    )
                    .map_err(erro)?,
            );
        }

        let mut por_hora = self
            .con
            .prepare(
                "SELECT ((inicio + ?2) % 86400 + 86400) % 86400 / 3600, SUM(ouvido_s)
                 FROM execucoes WHERE inicio >= ?1 GROUP BY 1",
            )
            .map_err(erro)?;
        let linhas = por_hora
            .query_map(params![desde, c.fuso_s], |l| Ok((l.get::<_, i64>(0)?, l.get::<_, f64>(1)?)))
            .map_err(erro)?;
        for linha in linhas {
            let (hora, segundos) = linha.map_err(erro)?;
            if let Some(v) = r.por_hora.get_mut(hora.clamp(0, 23) as usize) {
                *v = segundos;
            }
        }

        let mut por_dia = self
            .con
            .prepare(
                "SELECT (inicio + ?2) / 86400, SUM(ouvido_s)
                 FROM execucoes WHERE inicio >= ?1 GROUP BY 1 ORDER BY 1",
            )
            .map_err(erro)?;
        r.por_dia = por_dia
            .query_map(params![desde, c.fuso_s], |l| Ok((l.get(0)?, l.get(1)?)))
            .map_err(erro)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(erro)?;

        r.top_musicas = self.top(
            "SELECT titulo, artista, SUM(ouvido_s), COUNT(*) FROM execucoes
             WHERE inicio >= ?1 GROUP BY lower(titulo), lower(artista) ORDER BY 3 DESC, 1 LIMIT ?2",
            desde,
            c.tops,
        )?;
        r.top_artistas = self.top(
            "SELECT artista, '', SUM(ouvido_s), COUNT(*) FROM execucoes
             WHERE inicio >= ?1 AND artista <> '' GROUP BY lower(artista) ORDER BY 3 DESC, 1 LIMIT ?2",
            desde,
            c.tops,
        )?;

        r.sequencia = self.sequencia(c.agora, c.fuso_s)?;
        r.recentes = self.recentes(c.recentes)?;
        Ok(r)
    }

    fn top(&self, sql: &str, desde: i64, limite: u32) -> Resultado<Vec<Top>> {
        let mut consulta = self.con.prepare(sql).map_err(erro)?;
        let linhas = consulta
            .query_map(params![desde, limite], |l| {
                Ok(Top { nome: l.get(0)?, detalhe: l.get(1)?, segundos: l.get(2)?, vezes: l.get(3)? })
            })
            .map_err(erro)?;
        linhas.collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    fn sequencia(&self, agora: i64, fuso_s: i64) -> Resultado<u32> {
        let mut consulta = self
            .con
            .prepare("SELECT DISTINCT (inicio + ?1) / 86400 FROM execucoes ORDER BY 1 DESC LIMIT 3660")
            .map_err(erro)?;
        let dias: Vec<i64> = consulta
            .query_map(params![fuso_s], |l| l.get(0))
            .map_err(erro)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(erro)?;
        let hoje = dia_local(agora, fuso_s);
        let mut esperado = match dias.first() {
            Some(d) if *d == hoje || *d == hoje - 1 => *d,
            _ => return Ok(0),
        };
        let mut seguidos = 0;
        for d in dias {
            if d != esperado {
                break;
            }
            seguidos += 1;
            esperado -= 1;
        }
        Ok(seguidos)
    }

    // ------------------------------------------------------------------ letras

    pub fn letra(&self, chave: &str, agora: i64) -> Resultado<Guardada> {
        let linha = self
            .con
            .query_row(
                "SELECT sincronizada, simples, instrumental, buscada_em FROM letras WHERE chave = ?1",
                params![chave],
                |l| {
                    Ok((
                        LetraGuardada { sincronizada: l.get(0)?, simples: l.get(1)?, instrumental: l.get(2)? },
                        l.get::<_, i64>(3)?,
                    ))
                },
            )
            .optional()
            .map_err(erro)?;
        Ok(match linha {
            None => Guardada::Nada,
            Some((l, _)) if l.instrumental || l.sincronizada.is_some() || l.simples.is_some() => Guardada::Letra(l),
            Some((_, quando)) if agora - quando < VALIDADE_SEM_LETRA_S => Guardada::SemLetra,
            Some(_) => Guardada::Nada,
        })
    }

    /// Guarda o resultado de uma busca. `None` = a fonte respondeu que não tem a letra.
    pub fn guardar_letra(&self, chave: &str, letra: Option<&LetraGuardada>, agora: i64) -> Resultado<()> {
        let vazia = LetraGuardada::default();
        let l = letra.unwrap_or(&vazia);
        self.con
            .execute(
                "INSERT INTO letras (chave, sincronizada, simples, instrumental, buscada_em)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(chave) DO UPDATE SET sincronizada = ?2, simples = ?3, instrumental = ?4, buscada_em = ?5",
                params![chave, l.sincronizada, l.simples, l.instrumental, agora],
            )
            .map_err(erro)?;
        Ok(())
    }
}

fn finito(v: f64) -> f64 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

/// Começo do dia local de `agora`, em segundos desde 1970 (UTC).
pub fn inicio_do_dia(agora: i64, fuso_s: i64) -> i64 {
    agora - (agora + fuso_s).rem_euclid(SEGUNDOS_POR_DIA)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    /// 12/11/2026 15:00 em Brasília (18:00 UTC).
    const AGORA: i64 = 1_794_506_400;
    const FUSO: i64 = -3 * 3600;
    const DIA: i64 = 86_400;

    fn tocada(inicio: i64, titulo: &str, artista: &str, ouvido_s: f64) -> Execucao {
        Execucao {
            inicio,
            titulo: titulo.into(),
            artista: artista.into(),
            app: "Teste".into(),
            duracao_s: 200.0,
            ouvido_s,
            bpm: 120.0,
            energia: 0.5,
            ..Default::default()
        }
    }

    fn consulta(desde: Option<i64>) -> Consulta {
        Consulta { desde, agora: AGORA, fuso_s: FUSO, tops: 5, recentes: 10 }
    }

    #[test]
    fn soma_conta_e_ordena() {
        let b = Banco::em_memoria().expect("banco");
        b.registrar(&tocada(AGORA - 3600, "Aurora", "Banda Um", 200.0)).expect("a");
        b.registrar(&tocada(AGORA - 3000, "aurora", "banda um", 100.0)).expect("b");
        b.registrar(&tocada(AGORA - 2000, "Brisa", "Banda Dois", 250.0)).expect("c");
        b.registrar(&tocada(AGORA - 1000, "Som sem artista", "", 60.0)).expect("d");
        let r = b.resumo(consulta(None)).expect("resumo");
        assert_eq!(r.segundos, 610.0);
        assert_eq!(r.execucoes, 4);
        assert_eq!(r.musicas, 3, "maiúsculas não criam outra música");
        assert_eq!(r.artistas, 2, "sem artista não conta como artista");
        assert_eq!(r.top_musicas[0].nome.to_lowercase(), "aurora");
        assert_eq!(r.top_musicas[0].vezes, 2);
        assert_eq!(r.top_musicas[0].segundos, 300.0);
        assert_eq!(r.top_artistas.len(), 2);
        assert_eq!(r.top_artistas[0].segundos, 300.0);
        assert_eq!(r.recentes[0].titulo, "Som sem artista");
        assert_eq!(r.segundos_antes, None);
        assert!((r.bpm_medio - 120.0).abs() < 0.01);
    }

    #[test]
    fn periodo_e_comparacao_com_o_anterior() {
        let b = Banco::em_memoria().expect("banco");
        b.registrar(&tocada(AGORA - 2 * DIA, "Hoje não", "X", 500.0)).expect("a");
        b.registrar(&tocada(AGORA - 3600, "Hoje sim", "X", 120.0)).expect("b");
        let hoje = inicio_do_dia(AGORA, FUSO);
        assert_eq!(AGORA - hoje, 15 * 3600, "15h locais");
        let r = b.resumo(consulta(Some(hoje))).expect("resumo");
        assert_eq!(r.segundos, 120.0);
        assert_eq!(r.segundos_antes, Some(0.0));
        let semana = b.resumo(consulta(Some(AGORA - 7 * DIA))).expect("resumo");
        assert_eq!(semana.segundos, 620.0);
        assert_eq!(semana.por_dia.len(), 2);
        assert_eq!(semana.por_dia[1], (dia_local(AGORA, FUSO), 120.0));
    }

    #[test]
    fn hora_do_dia_usa_a_hora_local() {
        let b = Banco::em_memoria().expect("banco");
        // 14:00 locais.
        b.registrar(&tocada(AGORA - 3600, "A", "X", 90.0)).expect("a");
        // 23:30 locais da véspera (já é o dia seguinte em UTC).
        b.registrar(&tocada(AGORA - 15 * 3600 - 1800, "B", "X", 30.0)).expect("b");
        let r = b.resumo(consulta(None)).expect("resumo");
        assert_eq!(r.por_hora[14], 90.0);
        assert_eq!(r.por_hora[23], 30.0);
        assert_eq!(r.por_hora.iter().sum::<f64>(), 120.0);
        assert_eq!(r.por_dia.len(), 2, "a de 23:30 conta no dia local anterior");
    }

    #[test]
    fn sequencia_de_dias() {
        let b = Banco::em_memoria().expect("banco");
        assert_eq!(b.resumo(consulta(None)).expect("vazio").sequencia, 0);
        for dias_atras in [1, 2, 3, 5] {
            b.registrar(&tocada(AGORA - dias_atras * DIA, "A", "X", 60.0)).expect("a");
        }
        // Hoje ainda não tocou nada: a sequência até ontem continua valendo.
        assert_eq!(b.resumo(consulta(None)).expect("r").sequencia, 3);
        b.registrar(&tocada(AGORA - 60, "A", "X", 60.0)).expect("hoje");
        assert_eq!(b.resumo(consulta(None)).expect("r").sequencia, 4);
        // Parou há dois dias: acabou.
        let depois = Consulta { agora: AGORA + 2 * DIA, ..consulta(None) };
        assert_eq!(b.resumo(depois).expect("r").sequencia, 0);
    }

    #[test]
    fn atualizar_muda_a_mesma_linha() {
        let b = Banco::em_memoria().expect("banco");
        let mut e = tocada(AGORA, "A", "X", 30.0);
        let id = b.registrar(&e).expect("registrar");
        e.ouvido_s = 95.0;
        e.duracao_s = 180.0;
        e.bpm = f32::NAN;
        b.atualizar(id, &e).expect("atualizar");
        let lidas = b.recentes(10).expect("recentes");
        assert_eq!(lidas.len(), 1);
        assert_eq!(lidas[0].ouvido_s, 95.0);
        assert_eq!(lidas[0].duracao_s, 180.0);
        assert_eq!(lidas[0].bpm, 0.0, "valor inválido vira zero");
    }

    #[test]
    fn letra_guardada_e_validade_do_nao_tem() {
        let b = Banco::em_memoria().expect("banco");
        assert_eq!(b.letra("x|y|200", AGORA).expect("ler"), Guardada::Nada);
        b.guardar_letra("x|y|200", None, AGORA).expect("guardar");
        assert_eq!(b.letra("x|y|200", AGORA + 3600).expect("ler"), Guardada::SemLetra);
        assert_eq!(b.letra("x|y|200", AGORA + 25 * 3600).expect("ler"), Guardada::Nada, "pergunta de novo");
        let letra = LetraGuardada { sincronizada: Some("[00:01.00]linha de teste".into()), ..Default::default() };
        b.guardar_letra("x|y|200", Some(&letra), AGORA).expect("guardar");
        assert_eq!(b.letra("x|y|200", AGORA + 400 * DIA).expect("ler"), Guardada::Letra(letra), "letra não vence");
        let instrumental = LetraGuardada { instrumental: true, ..Default::default() };
        b.guardar_letra("i", Some(&instrumental), AGORA).expect("guardar");
        assert_eq!(b.letra("i", AGORA + 400 * DIA).expect("ler"), Guardada::Letra(instrumental));
    }

    #[test]
    fn arquivo_estragado_recomeca_vazio() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-teste-banco-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        let arquivo = pasta.join("historico.db");
        std::fs::write(&arquivo, b"isto nao e um banco de dados, so texto qualquer para estragar").expect("escrever");
        let b = Banco::abrir(&arquivo).expect("abre mesmo assim");
        assert!(b.recentes(5).expect("recentes").is_empty());
        assert!(arquivo.with_extension("db.corrompido").exists());
        drop(b);
        // Reabrir mantém o que foi gravado.
        let b = Banco::abrir(&arquivo).expect("abrir");
        b.registrar(&tocada(AGORA, "A", "X", 60.0)).expect("registrar");
        drop(b);
        assert_eq!(Banco::abrir(&arquivo).expect("reabrir").recentes(5).expect("recentes").len(), 1);
        let _ = std::fs::remove_dir_all(pasta);
    }
}
