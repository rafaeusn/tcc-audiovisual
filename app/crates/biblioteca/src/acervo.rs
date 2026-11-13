//! O acervo: um arquivo SQLite com as pastas escolhidas e as músicas achadas nelas.
//!
//! Cada pasta é uma categoria (como uma playlist). A mesma música pode estar em duas pastas (uma dentro da
//! outra, ou uma cópia do arquivo): ela aparece em cada pasta, mas só uma vez em "Todas".

use crate::{Musica, normalizar};
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
        // A primeira versão guardava cada arquivo numa pasta só (tabela `musicas`). Ela sai; as pastas ficam e
        // são varridas de novo (ver `pastas_vazias`).
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
                 PRIMARY KEY (caminho, pasta)
             );
             CREATE INDEX IF NOT EXISTS faixas_pasta ON faixas(pasta);
             CREATE INDEX IF NOT EXISTS faixas_igual ON faixas(igual);",
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
                         (caminho, pasta, titulo, artista, album, numero, duracao_s, busca, igual)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )
                .map_err(erro)?;
            for m in musicas {
                let busca = normalizar(&format!("{} {} {}", m.titulo, m.artista, m.album));
                let duracao = if m.duracao_s.is_finite() { m.duracao_s.max(0.0) } else { 0.0 };
                let caminho = texto(&m.caminho);
                let igual = chave_de_igual(m, &caminho);
                inserir
                    .execute(params![caminho, p, m.titulo, m.artista, m.album, m.numero, duracao, busca, igual])
                    .map_err(erro)?;
            }
        }
        tx.commit().map_err(erro)
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

    /// As músicas que têm todas as palavras de `consulta` no título, no artista ou no álbum (vazia = todas), em
    /// ordem de artista, álbum, número da faixa e título. Com `pasta`, só as daquela pasta; sem, as de todas,
    /// cada música uma vez só.
    pub fn buscar(&self, consulta: &str, pasta: Option<&Path>, limite: u32) -> Resultado<Vec<Musica>> {
        let palavras: Vec<String> =
            normalizar(consulta).split(' ').filter(|p| !p.is_empty()).take(8).map(padrao).collect();
        let pasta = pasta.map(texto);
        let mut valores: Vec<&dyn rusqlite::ToSql> = vec![&limite];
        let mut condicoes = String::new();
        if let Some(p) = &pasta {
            valores.push(p);
            condicoes.push_str(" AND pasta = ?2");
        }
        for p in &palavras {
            valores.push(p);
            condicoes.push_str(&format!(" AND busca LIKE ?{} ESCAPE '\\'", valores.len()));
        }
        // Em "Todas", uma linha por música: `MIN(caminho)` escolhe qual arquivo representa as cópias, e o SQLite
        // traz as outras colunas dessa mesma linha.
        let (coluna, agrupar) =
            if pasta.is_some() { ("caminho", "") } else { ("MIN(caminho) AS caminho", " GROUP BY igual") };
        let sql = format!(
            "SELECT {coluna}, titulo, artista, album, numero, duracao_s FROM faixas WHERE 1 = 1{condicoes}{agrupar}
             ORDER BY lower(artista), lower(album), numero, lower(titulo), caminho LIMIT ?1"
        );
        let mut consulta = self.con.prepare(&sql).map_err(erro)?;
        let linhas = consulta
            .query_map(valores.as_slice(), |l| {
                Ok(Musica {
                    caminho: PathBuf::from(l.get::<_, String>(0)?),
                    titulo: l.get(1)?,
                    artista: l.get(2)?,
                    album: l.get(3)?,
                    numero: l.get(4)?,
                    duracao_s: l.get(5)?,
                })
            })
            .map_err(erro)?;
        linhas.collect::<Result<Vec<_>, _>>().map_err(erro)
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

    #[test]
    fn lista_em_ordem_de_artista_album_e_faixa() {
        let a = acervo();
        let todas = a.buscar("", None, 100).expect("buscar");
        assert_eq!(todas.iter().map(|m| m.titulo.as_str()).collect::<Vec<_>>(), ["Canção 100%", "Primeira", "Segunda"]);
        assert_eq!(a.total().expect("total"), 3);
        assert_eq!(a.buscar("", None, 2).expect("buscar").len(), 2, "respeita o limite");
    }

    #[test]
    fn busca_sem_acento_por_qualquer_campo_e_todas_as_palavras() {
        let a = acervo();
        let titulos =
            |c: &str| a.buscar(c, None, 100).expect("buscar").into_iter().map(|m| m.titulo).collect::<Vec<_>>();
        assert_eq!(titulos("cancao"), ["Canção 100%"]);
        assert_eq!(titulos("BANDA disco"), ["Primeira", "Segunda"]);
        assert_eq!(titulos("banda segunda"), ["Segunda"]);
        assert!(titulos("banda ana").is_empty(), "as duas palavras precisam aparecer");
        // Os curingas do banco valem como texto comum.
        assert!(titulos("%").len() == 3, "só pontuação vira busca vazia");
        assert!(titulos("zzz").is_empty());
        assert_eq!(padrao("a_b%"), "%a\\_b\\%%");
    }

    #[test]
    fn varrer_de_novo_troca_o_conteudo_da_pasta() {
        let mut a = acervo();
        let pasta = Path::new("C:/Musicas");
        a.guardar_varredura(pasta, &[musica("C:/Musicas/nova.mp3", "Nova", "Ana", "Solo", 2)]).expect("guardar");
        assert_eq!(a.total().expect("total"), 1, "o que sumiu do disco sai do acervo");
        // Outra pasta não é afetada.
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
        // Uma cópia de "Primeira" noutra pasta, e uma música que só existe lá.
        a.guardar_varredura(
            outra,
            &[
                musica("D:/Copias/b1.flac", "Primeira", "Banda B", "Coletânea", 1),
                musica("D:/Copias/z.mp3", "Zeta", "Zé", "", 0),
            ],
        )
        .expect("guardar");
        // Uma pasta dentro da outra: o mesmo arquivo nas duas.
        let dentro = Path::new("C:/Musicas/Disco");
        a.adicionar_pasta(dentro).expect("pasta");
        a.guardar_varredura(dentro, &[musica("C:/Musicas/b2.mp3", "Segunda", "Banda B", "Disco", 2)]).expect("guardar");

        let titulos = |p: Option<&str>| {
            a.buscar("", p.map(Path::new), 100).expect("buscar").into_iter().map(|m| m.titulo).collect::<Vec<_>>()
        };
        assert_eq!(titulos(None), ["Canção 100%", "Primeira", "Segunda", "Zeta"]);
        assert_eq!(a.total().expect("total"), 4);
        assert_eq!(titulos(Some("D:/Copias")), ["Primeira", "Zeta"]);
        assert_eq!(titulos(Some("C:/Musicas/Disco")), ["Segunda"]);
        assert!(a.buscar("zeta", Some(Path::new("C:/Musicas")), 100).expect("buscar").is_empty());
        let contagens = a.contagens().expect("contagens");
        assert_eq!(contagens.iter().map(|(_, n)| *n).collect::<Vec<_>>(), [3, 1, 2]);
        // Tirar a pasta de dentro não leva o arquivo embora da pasta de fora.
        a.remover_pasta(dentro).expect("remover");
        assert_eq!(titulos(Some("C:/Musicas")).len(), 3);
        // Pasta nova, ainda sem varredura.
        a.adicionar_pasta(Path::new("E:/Nova")).expect("pasta");
        assert_eq!(a.pastas_vazias().expect("vazias"), [PathBuf::from("E:/Nova")]);
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
