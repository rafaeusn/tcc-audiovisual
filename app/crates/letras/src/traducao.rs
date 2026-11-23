//! Tradução e letras latinas.
//!
//! - **Tradução** pela Unison (`unison.betterlyrics.org/translate`), que traduz pelo Google, como o MixMusic faz:
//!   até 200 linhas por vez, cada uma com até 500 caracteres (as maiores ficam sem tradução). Só vai para a rede
//!   quando a pessoa liga "Traduzir a letra".
//! - **Letras latinas** feitas no computador (`any_ascii`): coreano, kana do japonês, chinês (pinyin sem os tons),
//!   cirílico, grego, árabe e outras escritas. Nada sai do computador. O kanji vira a leitura chinesa (o japonês
//!   precisaria de um dicionário de leituras, que fica para depois).

use crate::lrclib::{Cliente, MAXIMO_DA_RESPOSTA};
use nucleo::{Erro, Resultado};
use serde::{Deserialize, Serialize};
use std::io::Read;

const UNISON: &str = "https://unison.betterlyrics.org";
const POR_VEZ: usize = 200;
const MAXIMO_POR_LINHA: usize = 500;

/// A tradução de cada linha (`None` para a linha que não precisava ou não pôde ser traduzida).
pub type Traducao = Vec<Option<String>>;

fn igual(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

impl Cliente {
    /// Traduz as linhas para o idioma `para` (código: `pt`, `en`...). `video_id` é o da faixa do YouTube Music,
    /// quando há (a Unison guarda a tradução com ele).
    pub fn traduzir(&self, linhas: &[String], para: &str, video_id: Option<&str>) -> Resultado<Traducao> {
        let mut saida = Vec::with_capacity(linhas.len());
        for parte in linhas.chunks(POR_VEZ) {
            saida.extend(self.traduzir_parte(parte, para, video_id)?);
        }
        Ok(saida)
    }

    #[allow(clippy::result_large_err)] // o erro do ureq é grande, mas só passa daqui para a função de cima
    fn enviar_traducao(&self, corpo: &str) -> Result<ureq::Response, ureq::Error> {
        self.agente
            .post(&format!("{UNISON}/translate"))
            .set("Content-Type", "application/json")
            .timeout(std::time::Duration::from_secs(20))
            .send_string(corpo)
    }

    fn traduzir_parte(&self, linhas: &[String], para: &str, video_id: Option<&str>) -> Resultado<Traducao> {
        #[derive(Serialize)]
        #[serde(rename_all = "camelCase")]
        struct Pedido<'a> {
            lines: Vec<&'a str>,
            to: &'a str,
            #[serde(skip_serializing_if = "Option::is_none")]
            video_id: Option<&'a str>,
        }
        #[derive(Default, Deserialize)]
        #[serde(default, rename_all = "camelCase")]
        struct Linha {
            translation: Option<String>,
            needs_translation: bool,
        }
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Resposta {
            lines: Vec<Linha>,
        }
        let pedido = Pedido {
            lines: linhas.iter().map(|l| if l.chars().count() <= MAXIMO_POR_LINHA { l.as_str() } else { "" }).collect(),
            to: para,
            video_id: video_id.filter(|v| !v.is_empty()),
        };
        let corpo = serde_json::to_string(&pedido).map_err(|e| Erro::Rede(e.to_string()))?;
        // Um 5xx de vez em quando é passageiro (o tradutor dela demorou): tenta mais uma vez.
        let mut resposta = self.enviar_traducao(&corpo);
        if let Err(ureq::Error::Status(500..=599, _)) = resposta {
            std::thread::sleep(std::time::Duration::from_secs(1));
            resposta = self.enviar_traducao(&corpo);
        }
        let r = match resposta {
            Ok(r) => r,
            // Nada para traduzir, ou um idioma que ela não reconheceu.
            Err(ureq::Error::Status(400, _)) => return Ok(vec![None; linhas.len()]),
            Err(ureq::Error::Status(codigo, _)) => return Err(Erro::Rede(format!("a Unison respondeu {codigo}"))),
            Err(e) => return Err(Erro::Rede(format!("Unison: {e}"))),
        };
        let mut texto = String::new();
        r.into_reader().take(MAXIMO_DA_RESPOSTA).read_to_string(&mut texto).map_err(|e| Erro::Rede(e.to_string()))?;
        let r: Resposta = serde_json::from_str(&texto).map_err(|e| Erro::Rede(format!("Unison: {e}")))?;
        if r.lines.len() != linhas.len() {
            return Err(Erro::Rede(format!("a Unison traduziu {} de {} linhas", r.lines.len(), linhas.len())));
        }
        Ok(r.lines
            .into_iter()
            .zip(linhas)
            .map(|(t, original)| {
                t.translation
                    .filter(|tr| t.needs_translation && !tr.trim().is_empty() && !igual(tr, original))
                    .map(|tr| tr.trim().to_string())
            })
            .collect())
    }
}

/// A linha em letras latinas, quando ela tem letras de outra escrita (e `None` quando já é latina).
pub fn romanizar(linha: &str) -> Option<String> {
    let outra_escrita = linha.chars().any(|c| c.is_alphabetic() && !c.is_ascii() && !latina(c));
    if !outra_escrita {
        return None;
    }
    // Cada caractere vira as letras dele. Os ideogramas (kanji, hanzi) ficam separados por espaço, como sílabas;
    // o kana e o hangul de uma palavra ficam juntos.
    let mut saida = String::with_capacity(linha.len() * 2);
    let mut antes_ideograma = false;
    for c in linha.chars() {
        if c.is_ascii() || latina(c) {
            saida.push(c);
            antes_ideograma = false;
            continue;
        }
        let mut buf = [0u8; 4];
        let t = any_ascii::any_ascii(c.encode_utf8(&mut buf));
        if t.is_empty() {
            continue;
        }
        let ideograma = matches!(c as u32, 0x4E00..=0x9FFF | 0x3400..=0x4DBF);
        if (ideograma || antes_ideograma) && !saida.is_empty() && !saida.ends_with(' ') {
            saida.push(' ');
        }
        saida.push_str(&t.to_lowercase());
        antes_ideograma = ideograma;
    }
    let arrumado = saida.split_whitespace().collect::<Vec<_>>().join(" ");
    (!arrumado.is_empty() && !igual(&arrumado, linha)).then_some(arrumado)
}

/// As letras com acento do alfabeto latino (português, francês, vietnamita...) não precisam de versão latina.
fn latina(c: char) -> bool {
    matches!(c as u32, 0x00C0..=0x024F | 0x1E00..=0x1EFF)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn latina_nao_muda_e_outras_escritas_viram_latinas() {
        assert_eq!(romanizar("Coração vagabundo"), None, "português já é latino");
        assert_eq!(romanizar("Ça va très bien"), None);
        assert_eq!(romanizar("123 !?"), None);
        assert_eq!(romanizar("Привет мир").as_deref(), Some("privet mir"));
        assert_eq!(romanizar("Καλημέρα").as_deref(), Some("kalimera"));
        let kana = romanizar("こんにちは").unwrap_or_default();
        assert!(kana.contains("ko") && kana.is_ascii(), "{kana}");
        let hangul = romanizar("사랑해").unwrap_or_default();
        assert!(hangul.starts_with("sa") && hangul.is_ascii(), "{hangul}");
        let hanzi = romanizar("你好").unwrap_or_default();
        assert!(hanzi.contains("ni") && hanzi.contains("hao"), "{hanzi}");
        assert_eq!(romanizar("love 사랑").map(|r| r.starts_with("love ")), Some(true), "o latino fica como está");
    }
}
