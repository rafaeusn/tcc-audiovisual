//! A capa da música (e a foto do artista) para o cartão do Discord.
//!
//! O Discord só mostra imagens que estão na internet: a capa embutida no arquivo, ou a que o Windows recebe do
//! Spotify, não serve. Por isso o título e o artista vão à busca pública do Deezer (sem conta nem chave), que
//! devolve o endereço da capa do álbum e da foto do artista. Quem abre a imagem é o Discord.
//!
//! Cada música é buscada uma vez por execução do app; o resultado (até a falta dele) fica guardado.

use crate::Faixa;
use std::collections::HashMap;
use std::time::Duration;

/// Os endereços das imagens de uma música, prontos para o cartão.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Capa {
    /// A capa do álbum (grande).
    pub album: String,
    /// O nome do álbum, para quando o mouse passa na capa.
    pub nome_do_album: String,
    /// A foto do artista (pequena, no canto da capa).
    pub artista: String,
}

/// Só letras e números, em minúsculas: "Cigarettes After Sex" e "cigarettes-after-sex" são iguais.
fn chave(s: &str) -> String {
    s.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

/// O primeiro artista de "A, B feat. C" (as buscas acham melhor só com ele).
pub fn artista_principal(artista: &str) -> &str {
    let corte = [",", " feat.", " ft.", " & ", " x ", " part. "]
        .iter()
        .filter_map(|s| artista.to_lowercase().find(s))
        .min()
        .unwrap_or(artista.len());
    artista.get(..corte).unwrap_or(artista).trim()
}

/// O título sem o que costuma vir entre parênteses ou depois de " - " ("(Ao Vivo)", "- Remastered 2011").
pub fn titulo_limpo(titulo: &str) -> &str {
    let corte = [" (", " [", " - "].iter().filter_map(|s| titulo.find(s)).min().unwrap_or(titulo.len());
    let limpo = titulo.get(..corte).unwrap_or(titulo).trim();
    if limpo.is_empty() { titulo.trim() } else { limpo }
}

/// Escolhe, na resposta da busca do Deezer, o resultado que é a música pedida. Prefere o que tem o mesmo artista
/// e o mesmo título; aceita o mesmo artista com título parecido. Resultado de outro artista não serve (seria a
/// capa errada no seu perfil).
pub fn escolher(resposta: &serde_json::Value, faixa: &Faixa) -> Option<Capa> {
    let lista = resposta.get("data")?.as_array()?;
    let artista = chave(artista_principal(&faixa.artista));
    let titulo = chave(titulo_limpo(&faixa.titulo));
    if artista.is_empty() || titulo.is_empty() {
        return None;
    }
    let do_artista = |r: &&serde_json::Value| {
        let nome = chave(r.pointer("/artist/name").and_then(|v| v.as_str()).unwrap_or_default());
        !nome.is_empty() && (nome.contains(&artista) || artista.contains(&nome))
    };
    let titulo_de = |r: &serde_json::Value| chave(r.get("title").and_then(|v| v.as_str()).unwrap_or_default());
    let achado = lista.iter().filter(do_artista).find(|r| titulo_de(r) == titulo).or_else(|| {
        lista.iter().filter(do_artista).find(|r| {
            let t = titulo_de(r);
            !t.is_empty() && (t.contains(&titulo) || titulo.contains(&t))
        })
    })?;
    let texto = |caminho: &str| achado.pointer(caminho).and_then(|v| v.as_str()).unwrap_or_default().to_string();
    let album =
        [texto("/album/cover_xl"), texto("/album/cover_big")].into_iter().find(|u| u.starts_with("https://"))?;
    let foto = texto("/artist/picture_medium");
    Some(Capa {
        album,
        nome_do_album: texto("/album/title"),
        artista: if foto.starts_with("https://") { foto } else { String::new() },
    })
}

/// Busca as capas e guarda o que achou (e o que não achou), uma vez por música.
pub struct Buscador {
    agente: ureq::Agent,
    guardadas: HashMap<(String, String), Option<Capa>>,
}

impl Default for Buscador {
    fn default() -> Self {
        Self::novo()
    }
}

impl Buscador {
    pub fn novo() -> Self {
        let agente = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout(Duration::from_secs(8))
            .user_agent(concat!("Player-Escuta/", env!("CARGO_PKG_VERSION")))
            .build();
        Self { agente, guardadas: HashMap::new() }
    }

    /// A capa já buscada desta música: `Some(Some(..))` achou, `Some(None)` buscou e não achou, `None` ainda não
    /// buscou.
    pub fn guardada(&self, faixa: &Faixa) -> Option<Option<Capa>> {
        self.guardadas.get(&(chave(&faixa.artista), chave(&faixa.titulo))).cloned()
    }

    /// Busca na internet (espera a resposta: chame fora da interface). Sem artista, nem tenta.
    pub fn buscar(&mut self, faixa: &Faixa) -> Option<Capa> {
        if let Some(ja) = self.guardada(faixa) {
            return ja;
        }
        let capa = if faixa.artista.trim().is_empty() { None } else { self.perguntar(faixa) };
        // Sem limite, uma sessão muito longa guardaria tudo: depois de 500 músicas, recomeça.
        if self.guardadas.len() > 500 {
            self.guardadas.clear();
        }
        self.guardadas.insert((chave(&faixa.artista), chave(&faixa.titulo)), capa.clone());
        capa
    }

    fn perguntar(&self, faixa: &Faixa) -> Option<Capa> {
        // A busca simples ("artista título") acha mais que a avançada (`artist:"" track:""`), que costuma voltar
        // vazia; a escolha do resultado confere o artista, então a capa errada não passa.
        let consulta =
            format!("{} {}", artista_principal(&faixa.artista), titulo_limpo(&faixa.titulo)).replace('"', "");
        let resposta = self
            .agente
            .get("https://api.deezer.com/search")
            .query("q", &consulta)
            .query("limit", "10")
            .call()
            .map_err(|e| log::debug!("capa não buscada: {e}"))
            .ok()?;
        let corpo = resposta.into_string().map_err(|e| log::debug!("capa ilegível: {e}")).ok()?;
        let json: serde_json::Value =
            serde_json::from_str(&corpo).map_err(|e| log::debug!("capa ilegível: {e}")).ok()?;
        let capa = escolher(&json, faixa);
        if capa.is_none() {
            log::debug!("nenhuma capa para \"{}\" de {}", faixa.titulo, faixa.artista);
        }
        capa
    }
}

/// A página da música no Last.fm (para o título do cartão virar link).
pub fn pagina_no_lastfm(faixa: &Faixa) -> Option<String> {
    if faixa.artista.trim().is_empty() || faixa.titulo.trim().is_empty() {
        return None;
    }
    Some(format!(
        "https://www.last.fm/music/{}/_/{}",
        codificar(artista_principal(&faixa.artista)),
        codificar(faixa.titulo.trim())
    ))
}

/// A página do artista no Last.fm.
pub fn artista_no_lastfm(faixa: &Faixa) -> Option<String> {
    let artista = artista_principal(&faixa.artista);
    (!artista.is_empty()).then(|| format!("https://www.last.fm/music/{}", codificar(artista)))
}

/// Codifica um pedaço de endereço do jeito do Last.fm: espaço vira `+`, o resto que não é letra ou número vira
/// `%XX`.
fn codificar(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    for b in texto.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => saida.push(b as char),
            b' ' => saida.push('+'),
            _ => saida.push_str(&format!("%{b:02X}")),
        }
    }
    saida
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn faixa(titulo: &str, artista: &str) -> Faixa {
        Faixa { titulo: titulo.into(), artista: artista.into(), album: String::new() }
    }

    fn resposta() -> serde_json::Value {
        serde_json::json!({ "data": [
            { "title": "Twizzler (Live)", "artist": { "name": "Outra Banda", "picture_medium": "https://x/outra.jpg" },
              "album": { "title": "Ao Vivo", "cover_xl": "https://x/errada.jpg" } },
            { "title": "Twizzler", "artist": { "name": "Cigarettes After Sex", "picture_medium": "https://x/cas.jpg" },
              "album": { "title": "Twizzler", "cover_xl": "https://x/twizzler-xl.jpg", "cover_big": "https://x/big.jpg" } }
        ]})
    }

    #[test]
    fn escolhe_o_mesmo_artista_e_titulo() {
        let c = escolher(&resposta(), &faixa("Twizzler", "Cigarettes After Sex")).expect("achou");
        assert_eq!(c.album, "https://x/twizzler-xl.jpg");
        assert_eq!(c.artista, "https://x/cas.jpg");
        assert_eq!(c.nome_do_album, "Twizzler");
    }

    #[test]
    fn outro_artista_nao_serve_e_titulo_parecido_serve() {
        assert!(escolher(&resposta(), &faixa("Twizzler", "Ninguém")).is_none());
        let c = escolher(&resposta(), &faixa("Twizzler - Remastered", "cigarettes after sex, Fulano")).expect("achou");
        assert_eq!(c.album, "https://x/twizzler-xl.jpg");
        assert!(escolher(&serde_json::json!({ "data": [] }), &faixa("A", "B")).is_none());
        assert!(escolher(&serde_json::json!({ "erro": 1 }), &faixa("A", "B")).is_none());
    }

    #[test]
    fn limpa_titulo_e_artista() {
        assert_eq!(titulo_limpo("Song (Ao Vivo)"), "Song");
        assert_eq!(titulo_limpo("Song - Remastered 2011"), "Song");
        assert_eq!(titulo_limpo("(Interlude)"), "(Interlude)");
        assert_eq!(artista_principal("Dj Slk, Mc Talibã"), "Dj Slk");
        assert_eq!(artista_principal("A feat. B"), "A");
        assert_eq!(artista_principal("Solo"), "Solo");
    }

    #[test]
    fn links_do_lastfm() {
        let f = faixa("Twizzler", "Cigarettes After Sex");
        assert_eq!(pagina_no_lastfm(&f).as_deref(), Some("https://www.last.fm/music/Cigarettes+After+Sex/_/Twizzler"));
        assert_eq!(artista_no_lastfm(&faixa("Ação", "Zé & Cia")).as_deref(), Some("https://www.last.fm/music/Z%C3%A9"));
        assert!(pagina_no_lastfm(&faixa("Só título", "")).is_none());
    }
}
