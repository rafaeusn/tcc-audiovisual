//! As fontes de letra, perguntadas em cadeia, como o MixMusic faz (conferido no código dele em 23/11/2026):
//!
//! 1. **Better Lyrics** (`api.betterlyrics.org/getLyrics`): só letras palavra por palavra, em TTML; precisa do
//!    título, do artista, do álbum e da duração.
//! 2. **Unison** (`unison.betterlyrics.org/lyrics`): base aberta e comunitária, muita coisa palavra por palavra;
//!    pelo id do vídeo do YouTube (quando a faixa é do YouTube Music) ou pelo título e artista. TTML, LRC ou texto.
//! 3. **LRCLIB**: por linha (às vezes por palavra), pelo título, artista, álbum e duração.
//!
//! A cadeia para na primeira letra sincronizada. Com "preferir palavra por palavra", segue até achar uma palavra
//! por palavra (e fica com a melhor que achou). Uma fonte fora do ar não para a cadeia; só quando todas falham
//! por rede a resposta é "sem internet".

use crate::lrclib::{Cliente, MAXIMO_DA_RESPOSTA, Pedido};
use crate::ttml::{por_palavra, ttml_para_lrc};
use nucleo::{Erro, Resultado};
use serde::{Deserialize, Serialize};
use std::io::Read;

const BETTER_LYRICS: &str = "https://api.betterlyrics.org";
const UNISON: &str = "https://unison.betterlyrics.org";

/// Uma fonte de letra.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Fonte {
    BetterLyrics,
    Unison,
    Lrclib,
}

impl Fonte {
    /// Na ordem padrão.
    pub const TODAS: [Fonte; 3] = [Self::BetterLyrics, Self::Unison, Self::Lrclib];

    pub fn nome(self) -> &'static str {
        match self {
            Self::BetterLyrics => "Better Lyrics",
            Self::Unison => "Unison",
            Self::Lrclib => "LRCLIB",
        }
    }

    /// O nome guardado no banco (o mesmo dos ajustes).
    pub fn chave(self) -> &'static str {
        match self {
            Self::BetterLyrics => "better-lyrics",
            Self::Unison => "unison",
            Self::Lrclib => "lrclib",
        }
    }

    pub fn da_chave(chave: &str) -> Option<Self> {
        Self::TODAS.into_iter().find(|f| f.chave() == chave)
    }
}

/// Uma letra achada numa fonte.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Achada {
    /// LRC (com palavras, quando a fonte tem).
    pub sincronizada: Option<String>,
    pub simples: Option<String>,
    pub instrumental: bool,
    pub fonte: Option<Fonte>,
    /// O idioma, quando a fonte diz (código como "en", "pt", "ja").
    pub idioma: Option<String>,
}

impl Achada {
    /// 3 = palavra por palavra; 2 = sincronizada por linha (ou instrumental); 1 = só o texto; 0 = nada.
    pub fn nota(&self) -> u8 {
        match &self.sincronizada {
            Some(l) if por_palavra(l) => 3,
            Some(l) if !crate::ler_lrc(l).is_empty() => 2,
            _ if self.instrumental => 2,
            _ if self.simples.as_deref().is_some_and(|s| !s.trim().is_empty()) => 1,
            _ => 0,
        }
    }
}

/// O corpo e o código da resposta (os códigos de erro também trazem corpo na Unison).
fn ler(resposta: Result<ureq::Response, ureq::Error>, quem: &str) -> Resultado<(u16, String)> {
    let r = match resposta {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(e) => return Err(Erro::Rede(format!("{quem}: {e}"))),
    };
    let codigo = r.status();
    let mut corpo = String::new();
    r.into_reader()
        .take(MAXIMO_DA_RESPOSTA)
        .read_to_string(&mut corpo)
        .map_err(|e| Erro::Rede(format!("{quem}: {e}")))?;
    Ok((codigo, corpo))
}

/// Uma letra da Unison no formato que ela diz (ou no que parece, quando diz outro).
fn da_unison(texto: &str, formato: &str) -> Achada {
    let aparado = texto.trim();
    let formato = match formato {
        "ttml" | "lrc" | "plain" => formato,
        _ if aparado.starts_with('<') => "ttml",
        _ if aparado.starts_with('[') && !crate::ler_lrc(aparado).is_empty() => "lrc",
        _ => "plain",
    };
    let sincronizada = match formato {
        "ttml" => ttml_para_lrc(texto),
        "lrc" => Some(texto.to_string()).filter(|l| !crate::ler_lrc(l).is_empty()),
        _ => None,
    };
    let simples = (sincronizada.is_none() && formato == "plain").then(|| texto.to_string());
    Achada { sincronizada, simples, ..Default::default() }
}

impl Cliente {
    /// Better Lyrics: só letras palavra por palavra. `Ok(None)` quando não tem (ou falta o álbum ou a duração).
    pub fn better_lyrics(&self, p: &Pedido) -> Resultado<Option<Achada>> {
        let Some(d) = p.duracao_s.filter(|d| d.is_finite() && *d > 0.0) else { return Ok(None) };
        if p.titulo.is_empty() || p.artista.is_empty() || p.album.is_empty() {
            return Ok(None);
        }
        let (codigo, corpo) = ler(
            self.agente
                .get(&format!("{BETTER_LYRICS}/getLyrics"))
                .query("s", &p.titulo)
                .query("a", &p.artista)
                .query("al", &p.album)
                .query("d", &format!("{}", d.round() as i64))
                .call(),
            "Better Lyrics",
        )?;
        match codigo {
            200 => {}
            // 401: uma música que ela não guardou (precisaria de chave para buscar).
            401 | 404 | 422 => return Ok(None),
            outro => return Err(Erro::Rede(format!("a Better Lyrics respondeu {outro}"))),
        }
        #[derive(Deserialize)]
        struct Resposta {
            #[serde(default)]
            ttml: String,
        }
        let r: Resposta = serde_json::from_str(&corpo).map_err(|e| Erro::Rede(format!("Better Lyrics: {e}")))?;
        let lrc = ttml_para_lrc(&r.ttml).filter(|l| por_palavra(l));
        Ok(lrc.map(|l| Achada { sincronizada: Some(l), fonte: Some(Fonte::BetterLyrics), ..Default::default() }))
    }

    /// Unison: pelo id do vídeo (faixa do YouTube Music) ou pelo título e artista.
    pub fn unison(&self, p: &Pedido) -> Resultado<Option<Achada>> {
        let mut pedido = self.agente.get(&format!("{UNISON}/lyrics"));
        if let Some(v) = p.video_id.as_deref().filter(|v| !v.is_empty()) {
            pedido = pedido.query("v", v);
        } else if !p.titulo.is_empty() && !p.artista.is_empty() {
            pedido = pedido.query("song", &p.titulo).query("artist", &p.artista);
            if !p.album.is_empty() {
                pedido = pedido.query("album", &p.album);
            }
            if let Some(d) = p.duracao_s.filter(|d| d.is_finite() && *d > 0.0) {
                pedido = pedido.query("duration", &format!("{}", d.round() as i64));
            }
        } else {
            return Ok(None);
        }
        let (codigo, corpo) = ler(pedido.call(), "Unison")?;
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Dados {
            lyrics: String,
            format: String,
            language: String,
        }
        #[derive(Default, Deserialize)]
        #[serde(default)]
        struct Resposta {
            success: bool,
            code: String,
            data: Dados,
        }
        let r: Resposta = serde_json::from_str(&corpo).unwrap_or_default();
        if codigo == 404 && r.code == "NOT_FOUND" {
            return Ok(None);
        }
        if codigo != 200 {
            return Err(Erro::Rede(format!("a Unison respondeu {codigo}")));
        }
        if !r.success || r.data.lyrics.trim().is_empty() {
            return Ok(None);
        }
        let mut achada = da_unison(&r.data.lyrics, &r.data.format);
        if achada.nota() == 0 {
            return Ok(None);
        }
        achada.fonte = Some(Fonte::Unison);
        achada.idioma = Some(r.data.language).filter(|l| !l.is_empty());
        Ok(Some(achada))
    }

    /// LRCLIB, no formato das outras fontes.
    pub fn lrclib(&self, p: &Pedido) -> Resultado<Option<Achada>> {
        Ok(self.buscar(p)?.map(|r| Achada {
            sincronizada: r.synced_lyrics.filter(|s| !s.trim().is_empty()),
            simples: r.plain_lyrics.filter(|s| !s.trim().is_empty()),
            instrumental: r.instrumental,
            fonte: Some(Fonte::Lrclib),
            idioma: None,
        }))
    }

    /// Pergunta às fontes de `ordem`, uma por vez (ver o começo do módulo).
    pub fn em_cadeia(&self, p: &Pedido, ordem: &[Fonte], preferir_palavra: bool) -> Resultado<Option<Achada>> {
        em_cadeia(
            |f| match f {
                Fonte::BetterLyrics => self.better_lyrics(p),
                Fonte::Unison => self.unison(p),
                Fonte::Lrclib => self.lrclib(p),
            },
            ordem,
            preferir_palavra,
        )
    }
}

/// A cadeia em si, com a pergunta a cada fonte vinda de fora (para testar sem rede).
pub fn em_cadeia(
    mut perguntar: impl FnMut(Fonte) -> Resultado<Option<Achada>>,
    ordem: &[Fonte],
    preferir_palavra: bool,
) -> Resultado<Option<Achada>> {
    let mut melhor: Option<Achada> = None;
    let mut ultima_falha: Option<Erro> = None;
    let mut alguma_respondeu = false;
    for &f in ordem {
        match perguntar(f) {
            Ok(achada) => {
                alguma_respondeu = true;
                if let Some(a) = achada
                    && a.nota() > melhor.as_ref().map_or(0, Achada::nota)
                {
                    melhor = Some(a);
                }
            }
            Err(e) => {
                log::debug!("{} não respondeu: {e}", f.nome());
                ultima_falha = Some(e);
            }
        }
        let nota = melhor.as_ref().map_or(0, Achada::nota);
        if nota == 3 || (nota == 2 && !preferir_palavra) {
            break;
        }
    }
    match (melhor, ultima_falha) {
        (Some(a), _) => Ok(Some(a)),
        (None, Some(e)) if !alguma_respondeu => Err(e),
        _ => Ok(None),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn por_palavra_achada(f: Fonte) -> Achada {
        Achada {
            sincronizada: Some("[00:01.00]<00:01.00>uma <00:01.50>linha".into()),
            fonte: Some(f),
            ..Default::default()
        }
    }
    fn por_linha(f: Fonte) -> Achada {
        Achada { sincronizada: Some("[00:01.00]uma linha".into()), fonte: Some(f), ..Default::default() }
    }
    fn so_texto(f: Fonte) -> Achada {
        Achada { simples: Some("uma linha".into()), fonte: Some(f), ..Default::default() }
    }

    #[test]
    fn notas() {
        assert_eq!(por_palavra_achada(Fonte::Unison).nota(), 3);
        assert_eq!(por_linha(Fonte::Lrclib).nota(), 2);
        assert_eq!(so_texto(Fonte::Lrclib).nota(), 1);
        assert_eq!(Achada { instrumental: true, ..Default::default() }.nota(), 2);
        assert_eq!(Achada::default().nota(), 0);
    }

    #[test]
    fn para_na_primeira_sincronizada_ou_segue_atras_da_palavra_por_palavra() {
        let mut perguntadas = Vec::new();
        let r = em_cadeia(
            |f| {
                perguntadas.push(f);
                Ok(match f {
                    Fonte::BetterLyrics => None,
                    Fonte::Unison => Some(por_linha(f)),
                    Fonte::Lrclib => Some(por_palavra_achada(f)),
                })
            },
            &Fonte::TODAS,
            false,
        );
        assert_eq!(r.ok().flatten().and_then(|a| a.fonte), Some(Fonte::Unison));
        assert_eq!(perguntadas, vec![Fonte::BetterLyrics, Fonte::Unison], "parou na sincronizada");

        let r = em_cadeia(
            |f| Ok(if f == Fonte::Lrclib { Some(por_palavra_achada(f)) } else { Some(por_linha(f)) }),
            &Fonte::TODAS,
            true,
        );
        assert_eq!(r.ok().flatten().and_then(|a| a.fonte), Some(Fonte::Lrclib), "preferindo palavra por palavra");

        // Uma fonte que só tem texto não para a cadeia; a sincronizada de depois ganha.
        let r = em_cadeia(
            |f| Ok(if f == Fonte::BetterLyrics { Some(so_texto(f)) } else { Some(por_linha(f)) }),
            &Fonte::TODAS,
            false,
        );
        assert_eq!(r.ok().flatten().and_then(|a| a.fonte), Some(Fonte::Unison));
    }

    #[test]
    fn fonte_fora_do_ar_nao_para_a_cadeia() {
        let r = em_cadeia(
            |f| if f == Fonte::Lrclib { Ok(Some(por_linha(f))) } else { Err(Erro::Rede("fora do ar".into())) },
            &Fonte::TODAS,
            false,
        );
        assert_eq!(r.ok().flatten().and_then(|a| a.fonte), Some(Fonte::Lrclib));
        // Todas fora do ar: sem internet.
        assert!(em_cadeia(|_| Err(Erro::Rede("x".into())), &Fonte::TODAS, false).is_err());
        // Uma respondeu "não tem" e as outras falharam: não tem.
        let r = em_cadeia(
            |f| if f == Fonte::Unison { Ok(None) } else { Err(Erro::Rede("x".into())) },
            &Fonte::TODAS,
            false,
        );
        assert!(matches!(r, Ok(None)));
        // Nenhuma fonte ligada: nada.
        assert!(matches!(em_cadeia(|_| Ok(None), &[], false), Ok(None)));
    }

    #[test]
    fn formatos_da_unison() {
        let ttml = r#"<tt><body><div><p begin="1.0"><span begin="1.0" end="1.5">oi</span></p></div></body></tt>"#;
        assert_eq!(da_unison(ttml, "ttml").nota(), 3);
        assert_eq!(da_unison(ttml, "desconhecido").nota(), 3, "parece TTML");
        assert_eq!(da_unison("[00:01.00]linha", "lrc").nota(), 2);
        assert_eq!(da_unison("[00:01.00]linha", "").nota(), 2, "parece LRC");
        assert_eq!(da_unison("só texto", "plain").nota(), 1);
        assert_eq!(da_unison("[ar:etiqueta sem linhas]", "lrc").nota(), 0);
        assert_eq!(Fonte::da_chave("unison"), Some(Fonte::Unison));
        assert_eq!(Fonte::da_chave("outra"), None);
    }
}
