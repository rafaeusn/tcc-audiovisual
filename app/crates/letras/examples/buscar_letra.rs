//! Teste manual das fontes de letra: pergunta a cada fonte e à cadeia e mostra de onde veio, como é sincronizada e
//! quantas linhas tem (nunca o texto da letra).
//!
//! `cargo run -p letras --example buscar_letra -- "<título>" "<artista>" "<álbum>" <segundos> [id do vídeo]`

use letras::{Achada, Cliente, Fonte, Pedido};

fn resumo(a: &Option<Achada>) -> String {
    match a {
        None => "não tem".into(),
        Some(a) => {
            let tipo = match a.nota() {
                3 => "palavra por palavra",
                2 if a.instrumental => "instrumental",
                2 => "por linha",
                1 => "só o texto",
                _ => "vazia",
            };
            let linhas = a.sincronizada.as_deref().map_or(0, |l| letras::ler_lrc(l).len());
            format!("{tipo}, {linhas} linhas sincronizadas, idioma {:?}", a.idioma)
        }
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let pedido = Pedido {
        titulo: a.first().cloned().unwrap_or_default(),
        artista: a.get(1).cloned().unwrap_or_default(),
        album: a.get(2).cloned().unwrap_or_default(),
        duracao_s: a.get(3).and_then(|d| d.parse().ok()),
        video_id: a.get(4).cloned(),
    };
    let cliente = match Cliente::novo() {
        Ok(c) => c,
        Err(e) => return eprintln!("sem cliente: {e}"),
    };
    for f in Fonte::TODAS {
        let r = match f {
            Fonte::BetterLyrics => cliente.better_lyrics(&pedido),
            Fonte::Unison => cliente.unison(&pedido),
            Fonte::Lrclib => cliente.lrclib(&pedido),
        };
        match r {
            Ok(achada) => println!("{}: {}", f.nome(), resumo(&achada)),
            Err(e) => println!("{}: falhou ({e})", f.nome()),
        }
    }
    match cliente.em_cadeia(&pedido, &Fonte::TODAS, false) {
        Ok(a) => {
            println!("cadeia: {} (de {:?})", resumo(&a), a.as_ref().and_then(|a| a.fonte).map(Fonte::nome));
            // A tradução para o português: só quantas linhas voltaram traduzidas (nunca o texto).
            let linhas: Vec<String> = a
                .as_ref()
                .and_then(|a| a.sincronizada.as_deref())
                .map(|l| letras::ler_lrc(l).into_iter().map(|l| l.texto).collect())
                .unwrap_or_default();
            if !linhas.is_empty() {
                match cliente.traduzir(&linhas, "pt", pedido.video_id.as_deref()) {
                    Ok(t) => println!("tradução: {} de {} linhas traduzidas", t.iter().flatten().count(), t.len()),
                    Err(e) => println!("tradução: falhou ({e})"),
                }
            }
        }
        Err(e) => println!("cadeia: falhou ({e})"),
    }
}
