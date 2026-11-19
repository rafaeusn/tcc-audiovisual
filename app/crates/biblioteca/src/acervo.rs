//! O acervo: um arquivo SQLite com as pastas escolhidas, as músicas achadas nelas e as playlists.
//!
//! Cada pasta é uma categoria (como uma playlist). A mesma música pode estar em duas pastas (uma dentro da
//! outra, ou uma cópia do arquivo): ela aparece em cada pasta, mas só uma vez em "Todas". As playlists guardam
//! o caminho dos arquivos, na ordem escolhida.

use crate::{Filtro, Grupo, Musica, Origem, Playlist, normalizar};
use nucleo::{Erro, Resultado};
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

pub struct Acervo {
    con: Connection,
}

fn erro(e: rusqlite::Error) -> Erro {
    Erro::Dados(e.to_string())
}

fn texto(caminho: &Path) -> String {
    caminho.to_string_lossy().into_owned()
}

/// Duas faixas com esta chave igual são a mesma música: mesmo título, mesmo artista e a mesma duração (em
/// segundos inteiros). Sem duração ou sem nada nas etiquetas, cada arquivo vale por si.
fn chave_de_igual(m: &Musica, caminho: &str) -> String {
    let segundos = if m.duracao_s.is_finite() { m.duracao_s.round() as i64 } else { 0 };
    let nome = normalizar(&format!("{} {}", m.titulo, m.artista));
    if segundos <= 0 || nome.is_empty() { format!("arquivo:{caminho}") } else { format!("{nome}|{segundos}") }
}

/// Uma palavra da busca pronta para o LIKE: `%`, `_` e a barra valem como texto comum.
fn padrao(palavra: &str) -> String {
    let mut s = String::with_capacity(palavra.len() + 2);
    s.push('%');
    for c in palavra.chars() {
        if matches!(c, '\\' | '%' | '_') {
            s.push('\\');
        }
        s.push(c);
    }
    s.push('%');
    s
}

/// Colunas lidas para montar uma [`Musica`], na ordem de [`musica_da_linha`].
const COLUNAS: &str = "caminho, titulo, artista, album, numero, duracao_s, ganho_db, pico";

fn musica_da_linha(l: &rusqlite::Row) -> rusqlite::Result<Musica> {
    Ok(Musica {
        caminho: PathBuf::from(l.get::<_, String>(0)?),
        titulo: l.get(1)?,
        artista: l.get(2)?,
        album: l.get(3)?,
        numero: l.get(4)?,
        duracao_s: l.get(5)?,
        ganho_db: l.get(6)?,
        pico: l.get(7)?,
    })
}

/// A consulta SQL de um filtro: a tabela de onde as músicas saem e as condições, com os valores na ordem dos
/// `?` (começando em `?2`; o `?1` é o limite).
struct Consulta {
    de: String,
    condicoes: String,
    valores: Vec<rusqlite::types::Value>,
    ordem: &'static str,
}

impl Consulta {
    fn de(filtro: &Filtro) -> Self {
        use rusqlite::types::Value;
        let mut valores: Vec<Value> = Vec::new();
        let mut condicoes = String::new();
        let proximo = |valores: &mut Vec<Value>, v: Value| {
            valores.push(v);
            format!("?{}", valores.len() + 1)
        };
        let ordem_normal = "ORDER BY lower(artista), lower(album), numero, lower(titulo), caminho";
        let (de, ordem) = match &filtro.origem {
            // Em "Todas", uma linha por música: `MIN(caminho)` escolhe qual arquivo representa as cópias, e o
            // SQLite traz as outras colunas dessa mesma linha.
            Origem::Todas => (
                "(SELECT MIN(caminho) AS caminho, titulo, artista, album, numero, duracao_s, ganho_db, pico, busca
                  FROM faixas GROUP BY igual)"
                    .to_string(),
                ordem_normal,
            ),
            Origem::Pasta(p) => {
                let n = proximo(&mut valores, Value::Text(texto(p)));
                (format!("(SELECT * FROM faixas WHERE pasta = {n})"), ordem_normal)
            }
            Origem::Playlist(id) => {
                let n = proximo(&mut valores, Value::Integer(*id));
                (
                    format!(
                        "(SELECT f.caminho, f.titulo, f.artista, f.album, f.numero, f.duracao_s, f.ganho_db, f.pico,
                                 f.busca, i.posicao
                          FROM playlist_itens i
                          JOIN (SELECT * FROM faixas GROUP BY caminho) f ON f.caminho = i.caminho
                          WHERE i.playlist = {n})"
                    ),
                    "ORDER BY posicao",
                )
            }
        };
        for palavra in normalizar(&filtro.consulta).split(' ').filter(|p| !p.is_empty()).take(8) {
            let n = proximo(&mut valores, Value::Text(padrao(palavra)));
            condicoes.push_str(&format!(" AND busca LIKE {n} ESCAPE '\\'"));
        }
        if let Some(album) = &filtro.album {
            let n = proximo(&mut valores, Value::Text(album.clone()));
            condicoes.push_str(&format!(" AND lower(album) = lower({n})"));
        }
        if let Some(artista) = &filtro.artista {
            let n = proximo(&mut valores, Value::Text(artista.clone()));
            condicoes.push_str(&format!(" AND lower(artista) = lower({n})"));
        }
        Self { de, condicoes, valores, ordem }
    }

    fn parametros(&self, limite: u32) -> Vec<rusqlite::types::Value> {
        let mut v = vec![rusqlite::types::Value::Integer(i64::from(limite))];
        v.extend(self.valores.iter().cloned());
        v
    }
}

impl Acervo {
    /// Abre (ou cria) o acervo. Um arquivo estragado é guardado como `.corrompido` e o acervo recomeça vazio:
    /// as músicas continuam no disco, basta escolher as pastas de novo.
    pub fn abrir(caminho: &Path) -> Resultado<Self> {
        match Self::preparar(Connection::open(caminho).map_err(erro)?) {
            Ok(a) => Ok(a),
            Err(e) => {
                log::warn!("acervo ilegível ({e}); guardando como .corrompido e começando de novo");
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
        // Versões anteriores: a tabela `musicas` (um arquivo numa pasta só) e `faixas` sem o ReplayGain. Elas
        // saem; as pastas ficam e são varridas de novo (ver `pastas_vazias`). As playlists não são tocadas.
        let tem_ganho = con
            .prepare("SELECT 1 FROM pragma_table_info('faixas') WHERE name = 'ganho_db'")
            .and_then(|mut c| c.exists([]))
            .map_err(erro)?;
        if !tem_ganho {
            con.execute_batch("DROP TABLE IF EXISTS faixas;").map_err(erro)?;
        }
        con.execute_batch(
            "CREATE TABLE IF NOT EXISTS pastas (caminho TEXT PRIMARY KEY);
             DROP TABLE IF EXISTS musicas;
             CREATE TABLE IF NOT EXISTS faixas (
                 caminho   TEXT NOT NULL,
                 pasta     TEXT NOT NULL,
                 titulo    TEXT NOT NULL,
                 artista   TEXT NOT NULL,
                 album     TEXT NOT NULL,
                 numero    INTEGER NOT NULL,
                 duracao_s REAL NOT NULL,
                 busca     TEXT NOT NULL,
                 igual     TEXT NOT NULL,
                 ganho_db  REAL,
                 pico      REAL,
                 PRIMARY KEY (caminho, pasta)
             );
             CREATE INDEX IF NOT EXISTS faixas_pasta ON faixas(pasta);
             CREATE INDEX IF NOT EXISTS faixas_igual ON faixas(igual);
             CREATE TABLE IF NOT EXISTS playlists (
                 id    INTEGER PRIMARY KEY,
                 nome  TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS playlist_itens (
                 playlist INTEGER NOT NULL,
                 posicao  INTEGER NOT NULL,
                 caminho  TEXT NOT NULL,
                 PRIMARY KEY (playlist, posicao)
             );",
        )
        .map_err(erro)?;
        con.query_row("SELECT COUNT(*) FROM faixas", [], |l| l.get::<_, i64>(0)).map_err(erro)?;
        Ok(Self { con })
    }

    pub fn pastas(&self) -> Resultado<Vec<PathBuf>> {
        let mut consulta = self.con.prepare("SELECT caminho FROM pastas ORDER BY caminho").map_err(erro)?;
        let linhas = consulta.query_map([], |l| l.get::<_, String>(0)).map_err(erro)?;
        linhas.map(|l| l.map(PathBuf::from)).collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    pub fn adicionar_pasta(&self, pasta: &Path) -> Resultado<()> {
        self.con.execute("INSERT OR IGNORE INTO pastas (caminho) VALUES (?1)", params![texto(pasta)]).map_err(erro)?;
        Ok(())
    }

    /// Tira a pasta e as músicas que vieram dela (os arquivos não são tocados).
    pub fn remover_pasta(&self, pasta: &Path) -> Resultado<()> {
        let p = texto(pasta);
        self.con.execute("DELETE FROM faixas WHERE pasta = ?1", params![p]).map_err(erro)?;
        self.con.execute("DELETE FROM pastas WHERE caminho = ?1", params![p]).map_err(erro)?;
        Ok(())
    }

    /// Guarda o resultado da varredura de `pasta`: as músicas achadas entram (ou são atualizadas) e as que não
    /// estão mais lá saem. Tudo de uma vez, para a lista nunca aparecer pela metade.
    pub fn guardar_varredura(&mut self, pasta: &Path, musicas: &[Musica]) -> Resultado<()> {
        let p = texto(pasta);
        let tx = self.con.transaction().map_err(erro)?;
        tx.execute("DELETE FROM faixas WHERE pasta = ?1", params![p]).map_err(erro)?;
        {
            let mut inserir = tx
                .prepare(
                    "INSERT OR REPLACE INTO faixas
                         (caminho, pasta, titulo, artista, album, numero, duracao_s, busca, igual, ganho_db, pico)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                )
                .map_err(erro)?;
            for m in musicas {
                let busca = normalizar(&format!("{} {} {}", m.titulo, m.artista, m.album));
                let duracao = if m.duracao_s.is_finite() { m.duracao_s.max(0.0) } else { 0.0 };
                let caminho = texto(&m.caminho);
                let igual = chave_de_igual(m, &caminho);
                inserir
                    .execute(params![
                        caminho, p, m.titulo, m.artista, m.album, m.numero, duracao, busca, igual, m.ganho_db, m.pico
                    ])
                    .map_err(erro)?;
            }
        }
        tx.commit().map_err(erro)
    }

    /// Todos os arquivos do acervo, cada um uma vez (para a pré-análise percorrer).
    pub fn caminhos(&self) -> Resultado<Vec<PathBuf>> {
        let mut c = self.con.prepare("SELECT DISTINCT caminho FROM faixas ORDER BY caminho").map_err(erro)?;
        let linhas = c.query_map([], |l| l.get::<_, String>(0)).map_err(erro)?;
        linhas.map(|l| l.map(PathBuf::from)).collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    /// Quantas músicas há em "Todas" (sem contar duas vezes a mesma música).
    pub fn total(&self) -> Resultado<u32> {
        self.con.query_row("SELECT COUNT(DISTINCT igual) FROM faixas", [], |l| l.get(0)).map_err(erro)
    }

    /// Quantas músicas cada pasta tem, na ordem das pastas.
    pub fn contagens(&self) -> Resultado<Vec<(PathBuf, u32)>> {
        let mut consulta = self
            .con
            .prepare(
                "SELECT p.caminho, COUNT(f.caminho) FROM pastas p LEFT JOIN faixas f ON f.pasta = p.caminho
                 GROUP BY p.caminho ORDER BY p.caminho",
            )
            .map_err(erro)?;
        let linhas =
            consulta.query_map([], |l| Ok((PathBuf::from(l.get::<_, String>(0)?), l.get(1)?))).map_err(erro)?;
        linhas.collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    /// As pastas sem nenhuma música guardada (novas, ou de um acervo da versão anterior): precisam ser varridas.
    pub fn pastas_vazias(&self) -> Resultado<Vec<PathBuf>> {
        Ok(self.contagens()?.into_iter().filter(|(_, n)| *n == 0).map(|(p, _)| p).collect())
    }

    /// As músicas do filtro, até `limite`. Em pasta e em "Todas", na ordem de artista, álbum, número e título;
    /// em playlist, na ordem dela.
    pub fn buscar(&self, filtro: &Filtro, limite: u32) -> Resultado<Vec<Musica>> {
        let c = Consulta::de(filtro);
        let sql = format!("SELECT {COLUNAS} FROM {} WHERE 1 = 1{} {} LIMIT ?1", c.de, c.condicoes, c.ordem);
        let mut consulta = self.con.prepare(&sql).map_err(erro)?;
        let linhas =
            consulta.query_map(rusqlite::params_from_iter(c.parametros(limite)), musica_da_linha).map_err(erro)?;
        linhas.collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    /// Os álbuns das músicas do filtro (o filtro de álbum é ignorado), em ordem de artista e nome.
    pub fn albuns(&self, filtro: &Filtro) -> Resultado<Vec<Grupo>> {
        let f = Filtro { album: None, ..filtro.clone() };
        let c = Consulta::de(&f);
        let sql = format!(
            "SELECT album, MIN(artista), COUNT(*), MIN(caminho) FROM {} WHERE album <> ''{}
             GROUP BY lower(album) ORDER BY lower(MIN(artista)), lower(album) LIMIT ?1",
            c.de, c.condicoes
        );
        self.grupos(&sql, &c)
    }

    /// Os artistas das músicas do filtro (o filtro de artista é ignorado), em ordem alfabética.
    pub fn artistas(&self, filtro: &Filtro) -> Resultado<Vec<Grupo>> {
        let f = Filtro { artista: None, ..filtro.clone() };
        let c = Consulta::de(&f);
        let sql = format!(
            "SELECT artista, '', COUNT(*), MIN(caminho) FROM {} WHERE artista <> ''{}
             GROUP BY lower(artista) ORDER BY lower(artista) LIMIT ?1",
            c.de, c.condicoes
        );
        self.grupos(&sql, &c)
    }

    fn grupos(&self, sql: &str, c: &Consulta) -> Resultado<Vec<Grupo>> {
        let mut consulta = self.con.prepare(sql).map_err(erro)?;
        let linhas = consulta
            .query_map(rusqlite::params_from_iter(c.parametros(5000)), |l| {
                Ok(Grupo {
                    nome: l.get(0)?,
                    detalhe: l.get(1)?,
                    quantas: l.get(2)?,
                    exemplo: PathBuf::from(l.get::<_, String>(3)?),
                })
            })
            .map_err(erro)?;
        linhas.collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    // ------------------------------------------------------------------ playlists

    /// As playlists, em ordem de criação, com quantas músicas cada uma tem.
    pub fn playlists(&self) -> Resultado<Vec<Playlist>> {
        let mut consulta = self
            .con
            .prepare(
                // Só conta o que ainda está no acervo (a pasta da música pode ter sido retirada).
                "SELECT p.id, p.nome, COUNT(f.caminho) FROM playlists p
                 LEFT JOIN playlist_itens i ON i.playlist = p.id
                 LEFT JOIN (SELECT DISTINCT caminho FROM faixas) f ON f.caminho = i.caminho
                 GROUP BY p.id ORDER BY p.id",
            )
            .map_err(erro)?;
        let linhas = consulta
            .query_map([], |l| Ok(Playlist { id: l.get(0)?, nome: l.get(1)?, quantas: l.get(2)? }))
            .map_err(erro)?;
        linhas.collect::<Result<Vec<_>, _>>().map_err(erro)
    }

    /// Cria uma playlist vazia e devolve o número dela.
    pub fn criar_playlist(&self, nome: &str) -> Resultado<i64> {
        let nome: String = nome.chars().filter(|c| !c.is_control()).take(80).collect();
        let nome = if nome.trim().is_empty() { "Nova playlist".to_string() } else { nome.trim().to_string() };
        self.con.execute("INSERT INTO playlists (nome) VALUES (?1)", params![nome]).map_err(erro)?;
        Ok(self.con.last_insert_rowid())
    }

    /// Apaga a playlist e devolve o nome e as músicas dela, na ordem (para desfazer).
    pub fn apagar_playlist(&self, id: i64) -> Resultado<(String, Vec<PathBuf>)> {
        let nome: String =
            self.con.query_row("SELECT nome FROM playlists WHERE id = ?1", params![id], |l| l.get(0)).map_err(erro)?;
        let caminhos = {
            let mut c = self
                .con
                .prepare("SELECT caminho FROM playlist_itens WHERE playlist = ?1 ORDER BY posicao")
                .map_err(erro)?;
            let linhas = c.query_map(params![id], |l| l.get::<_, String>(0)).map_err(erro)?;
            linhas.map(|l| l.map(PathBuf::from)).collect::<Result<Vec<_>, _>>().map_err(erro)?
        };
        self.con.execute("DELETE FROM playlist_itens WHERE playlist = ?1", params![id]).map_err(erro)?;
        self.con.execute("DELETE FROM playlists WHERE id = ?1", params![id]).map_err(erro)?;
        Ok((nome, caminhos))
    }

    /// Põe as músicas no fim da playlist, na ordem dada. As que já estão nela não entram de novo. Devolve
    /// quantas entraram.
    pub fn adicionar_a_playlist(&mut self, id: i64, caminhos: &[PathBuf]) -> Resultado<usize> {
        let tx = self.con.transaction().map_err(erro)?;
        let mut posicao: i64 = tx
            .query_row("SELECT COALESCE(MAX(posicao), -1) FROM playlist_itens WHERE playlist = ?1", params![id], |l| {
                l.get(0)
            })
            .map_err(erro)?;
        let mut entraram = 0;
        for c in caminhos {
            let c = texto(c);
            let ja: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM playlist_itens WHERE playlist = ?1 AND caminho = ?2)",
                    params![id, c],
                    |l| l.get(0),
                )
                .map_err(erro)?;
            if !ja {
                posicao += 1;
                tx.execute(
                    "INSERT INTO playlist_itens (playlist, posicao, caminho) VALUES (?1, ?2, ?3)",
                    params![id, posicao, c],
                )
                .map_err(erro)?;
                entraram += 1;
            }
        }
        tx.commit().map_err(erro)?;
        Ok(entraram)
    }

    /// Tira a música da playlist e devolve em que posição ela estava (para desfazer).
    pub fn tirar_da_playlist(&self, id: i64, caminho: &Path) -> Resultado<Option<i64>> {
        let c = texto(caminho);
        let posicao: Option<i64> = self
            .con
            .query_row(
                "SELECT MIN(posicao) FROM playlist_itens WHERE playlist = ?1 AND caminho = ?2",
                params![id, c],
                |l| l.get(0),
            )
            .map_err(erro)?;
        self.con
            .execute("DELETE FROM playlist_itens WHERE playlist = ?1 AND caminho = ?2", params![id, c])
            .map_err(erro)?;
        Ok(posicao)
    }

    /// Põe a música na posição `posicao` da playlist, empurrando as seguintes (desfazer um "tirar"). Se ela já
    /// está na playlist, nada muda.
    pub fn inserir_na_playlist(&mut self, id: i64, posicao: i64, caminho: &Path) -> Resultado<()> {
        let c = texto(caminho);
        let tx = self.con.transaction().map_err(erro)?;
        let ja: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM playlist_itens WHERE playlist = ?1 AND caminho = ?2)",
                params![id, c],
                |l| l.get(0),
            )
            .map_err(erro)?;
        if !ja {
            // Em dois passos (negativo e de volta) para nenhuma posição bater com outra no meio da troca.
            tx.execute(
                "UPDATE playlist_itens SET posicao = -(posicao + 1) WHERE playlist = ?1 AND posicao >= ?2",
                params![id, posicao],
            )
            .map_err(erro)?;
            tx.execute("UPDATE playlist_itens SET posicao = -posicao WHERE playlist = ?1 AND posicao < 0", params![id])
                .map_err(erro)?;
            tx.execute(
                "INSERT INTO playlist_itens (playlist, posicao, caminho) VALUES (?1, ?2, ?3)",
                params![id, posicao.max(0), c],
            )
            .map_err(erro)?;
        }
        tx.commit().map_err(erro)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn musica(caminho: &str, titulo: &str, artista: &str, album: &str, numero: u32) -> Musica {
        Musica {
            caminho: PathBuf::from(caminho),
            titulo: titulo.into(),
            artista: artista.into(),
            album: album.into(),
            numero,
            duracao_s: 200.0,
            ..Default::default()
        }
    }

    fn acervo() -> Acervo {
        let mut a = Acervo::em_memoria().expect("acervo");
        let pasta = Path::new("C:/Musicas");
        a.adicionar_pasta(pasta).expect("pasta");
        a.guardar_varredura(
            pasta,
            &[
                musica("C:/Musicas/b2.mp3", "Segunda", "Banda B", "Disco", 2),
                musica("C:/Musicas/b1.mp3", "Primeira", "Banda B", "Disco", 1),
                musica("C:/Musicas/a.mp3", "Canção 100%", "Ana", "Solo", 1),
            ],
        )
        .expect("guardar");
        a
    }

    fn todas(consulta: &str) -> Filtro {
        Filtro { consulta: consulta.into(), ..Default::default() }
    }

    fn titulos(a: &Acervo, f: &Filtro) -> Vec<String> {
        a.buscar(f, 100).expect("buscar").into_iter().map(|m| m.titulo).collect()
    }

    #[test]
    fn lista_em_ordem_de_artista_album_e_faixa() {
        let a = acervo();
        assert_eq!(titulos(&a, &todas("")), ["Canção 100%", "Primeira", "Segunda"]);
        assert_eq!(a.total().expect("total"), 3);
        assert_eq!(a.buscar(&todas(""), 2).expect("buscar").len(), 2, "respeita o limite");
    }

    #[test]
    fn busca_sem_acento_por_qualquer_campo_e_todas_as_palavras() {
        let a = acervo();
        assert_eq!(titulos(&a, &todas("cancao")), ["Canção 100%"]);
        assert_eq!(titulos(&a, &todas("BANDA disco")), ["Primeira", "Segunda"]);
        assert_eq!(titulos(&a, &todas("banda segunda")), ["Segunda"]);
        assert!(titulos(&a, &todas("banda ana")).is_empty(), "as duas palavras precisam aparecer");
        // Os curingas do banco valem como texto comum.
        assert_eq!(titulos(&a, &todas("%")).len(), 3, "só pontuação vira busca vazia");
        assert!(titulos(&a, &todas("zzz")).is_empty());
        assert_eq!(padrao("a_b%"), "%a\\_b\\%%");
    }

    #[test]
    fn varrer_de_novo_troca_o_conteudo_da_pasta() {
        let mut a = acervo();
        let pasta = Path::new("C:/Musicas");
        a.guardar_varredura(pasta, &[musica("C:/Musicas/nova.mp3", "Nova", "Ana", "Solo", 2)]).expect("guardar");
        assert_eq!(a.total().expect("total"), 1, "o que sumiu do disco sai do acervo");
        let outra = Path::new("D:/Outras");
        a.adicionar_pasta(outra).expect("pasta");
        a.guardar_varredura(outra, &[musica("D:/Outras/x.mp3", "X", "", "", 0)]).expect("guardar");
        assert_eq!(a.total().expect("total"), 2);
        assert_eq!(a.pastas().expect("pastas").len(), 2);
        a.remover_pasta(pasta).expect("remover");
        assert_eq!(a.total().expect("total"), 1);
        assert_eq!(a.pastas().expect("pastas"), [PathBuf::from("D:/Outras")]);
    }

    #[test]
    fn pastas_sao_categorias_e_todas_nao_repete() {
        let mut a = acervo();
        let outra = Path::new("D:/Copias");
        a.adicionar_pasta(outra).expect("pasta");
        a.guardar_varredura(
            outra,
            &[
                musica("D:/Copias/b1.flac", "Primeira", "Banda B", "Coletânea", 1),
                musica("D:/Copias/z.mp3", "Zeta", "Zé", "", 0),
            ],
        )
        .expect("guardar");
        let dentro = Path::new("C:/Musicas/Disco");
        a.adicionar_pasta(dentro).expect("pasta");
        a.guardar_varredura(dentro, &[musica("C:/Musicas/b2.mp3", "Segunda", "Banda B", "Disco", 2)]).expect("guardar");

        let pasta = |p: &str| Filtro { origem: Origem::Pasta(p.into()), ..Default::default() };
        assert_eq!(titulos(&a, &todas("")), ["Canção 100%", "Primeira", "Segunda", "Zeta"]);
        assert_eq!(a.total().expect("total"), 4);
        assert_eq!(titulos(&a, &pasta("D:/Copias")), ["Primeira", "Zeta"]);
        assert_eq!(titulos(&a, &pasta("C:/Musicas/Disco")), ["Segunda"]);
        assert!(titulos(&a, &Filtro { consulta: "zeta".into(), ..pasta("C:/Musicas") }).is_empty());
        let contagens = a.contagens().expect("contagens");
        assert_eq!(contagens.iter().map(|(_, n)| *n).collect::<Vec<_>>(), [3, 1, 2]);
        a.remover_pasta(dentro).expect("remover");
        assert_eq!(titulos(&a, &pasta("C:/Musicas")).len(), 3);
        a.adicionar_pasta(Path::new("E:/Nova")).expect("pasta");
        assert_eq!(a.pastas_vazias().expect("vazias"), [PathBuf::from("E:/Nova")]);
    }

    #[test]
    fn albuns_e_artistas_filtram_a_lista() {
        let a = acervo();
        let albuns = a.albuns(&todas("")).expect("albuns");
        assert_eq!(
            albuns.iter().map(|g| (g.nome.as_str(), g.detalhe.as_str(), g.quantas)).collect::<Vec<_>>(),
            [("Solo", "Ana", 1), ("Disco", "Banda B", 2)]
        );
        let artistas = a.artistas(&todas("")).expect("artistas");
        assert_eq!(artistas.iter().map(|g| g.nome.as_str()).collect::<Vec<_>>(), ["Ana", "Banda B"]);
        let do_disco = Filtro { album: Some("disco".into()), ..Default::default() };
        assert_eq!(titulos(&a, &do_disco), ["Primeira", "Segunda"]);
        let da_ana = Filtro { artista: Some("ANA".into()), ..Default::default() };
        assert_eq!(titulos(&a, &da_ana), ["Canção 100%"]);
        // Com um artista escolhido, os álbuns são só os dele.
        assert_eq!(a.albuns(&da_ana).expect("albuns").len(), 1);
    }

    #[test]
    fn playlists_guardam_a_ordem_e_nao_repetem() {
        let mut a = acervo();
        let id = a.criar_playlist("  Para correr ").expect("criar");
        let p = |s: &str| PathBuf::from(s);
        let entraram = a
            .adicionar_a_playlist(id, &[p("C:/Musicas/b2.mp3"), p("C:/Musicas/a.mp3"), p("C:/Musicas/b2.mp3")])
            .expect("adicionar");
        assert_eq!(entraram, 2);
        let lista = Filtro { origem: Origem::Playlist(id), ..Default::default() };
        assert_eq!(titulos(&a, &lista), ["Segunda", "Canção 100%"], "na ordem em que entraram");
        assert_eq!(a.adicionar_a_playlist(id, &[p("C:/Musicas/b1.mp3")]).expect("adicionar"), 1);
        assert_eq!(titulos(&a, &lista), ["Segunda", "Canção 100%", "Primeira"]);
        // Música de fora do acervo (pasta retirada) fica guardada, mas não conta nem aparece.
        a.adicionar_a_playlist(id, &[p("Z:/sumiu.mp3")]).expect("adicionar");
        assert_eq!(a.playlists().expect("playlists")[0].quantas, 3);
        a.tirar_da_playlist(id, Path::new("Z:/sumiu.mp3")).expect("tirar");
        let estava = a.tirar_da_playlist(id, Path::new("C:/Musicas/a.mp3")).expect("tirar");
        assert_eq!(estava, Some(1));
        // Desfazer: volta para o mesmo lugar.
        a.inserir_na_playlist(id, 1, Path::new("C:/Musicas/a.mp3")).expect("inserir");
        assert_eq!(titulos(&a, &lista), ["Segunda", "Canção 100%", "Primeira"]);
        a.tirar_da_playlist(id, Path::new("C:/Musicas/a.mp3")).expect("tirar");
        let playlists = a.playlists().expect("playlists");
        assert_eq!(playlists, [Playlist { id, nome: "Para correr".into(), quantas: 2 }]);
        assert_eq!(titulos(&a, &Filtro { consulta: "prim".into(), ..lista.clone() }), ["Primeira"]);
        let (nome, caminhos) = a.apagar_playlist(id).expect("apagar");
        assert_eq!(nome, "Para correr");
        assert_eq!(caminhos, [p("C:/Musicas/b2.mp3"), p("C:/Musicas/b1.mp3")]);
        assert!(a.playlists().expect("playlists").is_empty());
        a.criar_playlist("   ").expect("criar");
        assert_eq!(a.playlists().expect("playlists")[0].nome, "Nova playlist");
    }

    #[test]
    fn ganho_igualado_respeita_o_pico() {
        let mut m = musica("a.mp3", "A", "", "", 0);
        assert_eq!(m.ganho_igualado(0.0), 1.0, "sem ReplayGain fica como está");
        m.ganho_db = Some(-6.0);
        assert!((m.ganho_igualado(0.0) - 0.501).abs() < 0.01);
        m.ganho_db = Some(6.0);
        m.pico = Some(0.8);
        assert!((m.ganho_igualado(0.0) - 1.25).abs() < 0.01, "o pico limita o ganho");
    }

    #[test]
    fn sem_duracao_ou_sem_nome_cada_arquivo_vale_por_si() {
        let mut m = musica("C:/a.mp3", "Faixa", "", "", 0);
        m.duracao_s = 0.0;
        assert_eq!(chave_de_igual(&m, "C:/a.mp3"), "arquivo:C:/a.mp3");
        m.duracao_s = 199.6;
        assert_eq!(chave_de_igual(&m, "C:/a.mp3"), "faixa|200");
    }

    #[test]
    fn acervo_antigo_sem_replaygain_e_refeito_e_playlists_ficam() {
        let pasta = crate::leitura::testes::pasta_de_teste("migrar");
        let arquivo = pasta.join("biblioteca.db");
        {
            let con = Connection::open(&arquivo).expect("abrir");
            con.execute_batch(
                "CREATE TABLE pastas (caminho TEXT PRIMARY KEY);
                 INSERT INTO pastas VALUES ('C:/Musicas');
                 CREATE TABLE faixas (caminho TEXT, pasta TEXT, titulo TEXT, artista TEXT, album TEXT, numero INT,
                                      duracao_s REAL, busca TEXT, igual TEXT, PRIMARY KEY (caminho, pasta));
                 INSERT INTO faixas VALUES ('C:/Musicas/a.mp3', 'C:/Musicas', 'A', '', '', 0, 1, 'a', 'a');",
            )
            .expect("antigo");
        }
        let a = Acervo::abrir(&arquivo).expect("abre");
        assert_eq!(a.pastas_vazias().expect("vazias"), [PathBuf::from("C:/Musicas")], "a pasta vai ser relida");
        drop(a);
        let _ = std::fs::remove_dir_all(pasta);
    }

    #[test]
    fn arquivo_estragado_recomeca_vazio() {
        let pasta = crate::leitura::testes::pasta_de_teste("acervo");
        let arquivo = pasta.join("biblioteca.db");
        std::fs::write(&arquivo, b"isto nao e um banco de dados, so um texto qualquer para estragar").expect("gravar");
        let a = Acervo::abrir(&arquivo).expect("abre mesmo assim");
        assert_eq!(a.total().expect("total"), 0);
        assert!(arquivo.with_extension("db.corrompido").exists());
        drop(a);
        let _ = std::fs::remove_dir_all(pasta);
    }
}
