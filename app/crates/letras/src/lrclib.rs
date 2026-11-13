//! Busca no LRCLIB (<https://lrclib.net/docs>): aberto, sem conta e sem chave.
//!
//! Primeiro a pergunta exata (`/api/get`, com a duração); se não houver, a busca (`/api/search`) e a escolha pela
//! duração mais próxima. A duração é o que evita a letra de outra versão da mesma música, que sairia de sincronia.

use crate::texto::normalizar;
use nucleo::{Erro, Resultado};
use serde::Deserialize;
use std::io::Read;
use std::time::Duration;

const RAIZ: &str = "https://lrclib.net/api";
/// O LRCLIB pede que cada app se identifique. O nome é genérico: não diz quem é a pessoa nem o computador.
const AGENTE: &str = concat!("Player-Escuta/", env!("CARGO_PKG_VERSION"));
/// Resposta maior que isto não é uma letra: para de ler.
const MAXIMO_DA_RESPOSTA: u64 = 2 * 1024 * 1024;
/// Diferença de duração aceita entre a faixa e a letra, em segundos.
const FOLGA_EXATA_S: f64 = 2.0;
const FOLGA_MAXIMA_S: f64 = 5.0;

/// O que se sabe da faixa. Título e artista já passados por [`crate::limpar`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pedido {
    pub titulo: String,
    pub artista: String,
    pub album: String,
    pub duracao_s: Option<f64>,
}

/// Uma letra como o LRCLIB devolve.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Registro {
    pub track_name: String,
    pub artist_name: String,
    pub album_name: String,
    pub duration: f64,
    pub instrumental: bool,
    pub plain_lyrics: Option<String>,
    /// Texto no formato LRC.
    pub synced_lyrics: Option<String>,
}

impl Registro {
    /// A letra tem os tempos de cada linha.
    pub fn sincronizada(&self) -> bool {
        self.synced_lyrics.as_deref().is_some_and(|s| !s.trim().is_empty())
    }

    fn tem_algo(&self) -> bool {
        self.instrumental || self.sincronizada() || self.plain_lyrics.as_deref().is_some_and(|s| !s.trim().is_empty())
    }
}

/// Os dois nomes são o mesmo, ou um está dentro do outro ("Banda" e "Banda e Convidado").
fn combina(a: &str, b: &str) -> bool {
    let (a, b) = (normalizar(a), normalizar(b));
    !a.is_empty() && !b.is_empty() && (a == b || a.contains(&b) || b.contains(&a))
}

/// Escolhe, entre os resultados de uma busca, a letra da faixa pedida.
///
/// Só entra quem tem o mesmo título (e o mesmo artista, quando se sabe qual é). Com a duração conhecida, a letra
/// precisa estar a até 5 s dela; uma sincronizada a até 2 s ganha de qualquer outra. Sem a duração, vale a primeira
/// sincronizada (a busca já vem ordenada por relevância).
pub fn escolher(candidatos: Vec<Registro>, pedido: &Pedido) -> Option<Registro> {
    let duracao = pedido.duracao_s.filter(|d| d.is_finite() && *d > 0.0);
    let mut melhor: Option<(f64, Registro)> = None;
    for c in candidatos {
        if !c.tem_algo() || !combina(&c.track_name, &pedido.titulo) {
            continue;
        }
        if !pedido.artista.is_empty() && !combina(&c.artist_name, &pedido.artista) {
            continue;
        }
        let distancia = duracao.map_or(0.0, |d| (c.duration - d).abs());
        if distancia > FOLGA_MAXIMA_S {
            continue;
        }
        // Nota menor é melhor: sincronizada e perto primeiro; depois sincronizada; por fim só texto.
        let nota = distancia
            + if !c.sincronizada() {
                100.0
            } else if distancia <= FOLGA_EXATA_S {
                0.0
            } else {
                10.0
            };
        if melhor.as_ref().is_none_or(|(n, _)| nota < *n) {
            melhor = Some((nota, c));
        }
    }
    melhor.map(|(_, c)| c)
}

pub struct Cliente {
    agente: ureq::Agent,
}

impl Cliente {
    pub fn novo() -> Resultado<Self> {
        let tls = native_tls::TlsConnector::new().map_err(|e| Erro::Rede(e.to_string()))?;
        let agente = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(6))
            .timeout(Duration::from_secs(12))
            .user_agent(AGENTE)
            .tls_connector(std::sync::Arc::new(tls))
            .build();
        Ok(Self { agente })
    }

    /// `Ok(None)`: o LRCLIB respondeu e não tem a letra. `Err`: não deu para perguntar (sem internet, por exemplo).
    pub fn buscar(&self, pedido: &Pedido) -> Resultado<Option<Registro>> {
        if pedido.titulo.trim().is_empty() {
            return Ok(None);
        }
        let duracao = pedido.duracao_s.filter(|d| d.is_finite() && *d > 0.0);

        // A pergunta exata precisa do artista.
        if !pedido.artista.is_empty() {
            let mut exata = self
                .agente
                .get(&format!("{RAIZ}/get"))
                .query("track_name", &pedido.titulo)
                .query("artist_name", &pedido.artista);
            if !pedido.album.is_empty() {
                exata = exata.query("album_name", &pedido.album);
            }
            if let Some(d) = duracao {
                exata = exata.query("duration", &format!("{}", d.round() as i64));
            }
            if let Some(corpo) = ler(exata.call())?
                && let Ok(registro) = serde_json::from_str::<Registro>(&corpo)
                && registro.tem_algo()
            {
                return Ok(Some(registro));
            }
        }

        let mut busca = self.agente.get(&format!("{RAIZ}/search")).query("track_name", &pedido.titulo);
        if !pedido.artista.is_empty() {
            busca = busca.query("artist_name", &pedido.artista);
        }
        let Some(corpo) = ler(busca.call())? else { return Ok(None) };
        let candidatos: Vec<Registro> =
            serde_json::from_str(&corpo).map_err(|e| Erro::Rede(format!("resposta inesperada do LRCLIB: {e}")))?;
        Ok(escolher(candidatos, pedido))
    }
}

impl Cliente {
    /// Busca livre, pelo texto que a pessoa digitou (nome da música, artista, um trecho). Devolve até 20
    /// resultados que tenham alguma letra, na ordem de relevância do LRCLIB.
    pub fn procurar(&self, consulta: &str) -> Resultado<Vec<Registro>> {
        let consulta = consulta.trim();
        if consulta.is_empty() {
            return Ok(Vec::new());
        }
        let Some(corpo) = ler(self.agente.get(&format!("{RAIZ}/search")).query("q", consulta).call())? else {
            return Ok(Vec::new());
        };
        let achados: Vec<Registro> =
            serde_json::from_str(&corpo).map_err(|e| Erro::Rede(format!("resposta inesperada do LRCLIB: {e}")))?;
        Ok(achados.into_iter().filter(Registro::tem_algo).take(20).collect())
    }
}

/// O corpo da resposta. `Ok(None)` para "não encontrado"; qualquer outra falha é erro de rede.
fn ler(resposta: Result<ureq::Response, ureq::Error>) -> Resultado<Option<String>> {
    match resposta {
        Ok(r) => {
            let mut corpo = String::new();
            r.into_reader()
                .take(MAXIMO_DA_RESPOSTA)
                .read_to_string(&mut corpo)
                .map_err(|e| Erro::Rede(e.to_string()))?;
            Ok(Some(corpo))
        }
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(ureq::Error::Status(codigo, _)) => Err(Erro::Rede(format!("o LRCLIB respondeu {codigo}"))),
        Err(e) => Err(Erro::Rede(e.to_string())),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn registro(titulo: &str, artista: &str, duracao: f64, sincronizada: bool) -> Registro {
        Registro {
            track_name: titulo.into(),
            artist_name: artista.into(),
            album_name: String::new(),
            duration: duracao,
            instrumental: false,
            plain_lyrics: Some("linha de teste".into()),
            synced_lyrics: sincronizada.then(|| "[00:01.00]linha de teste".into()),
        }
    }

    fn pedido(duracao: Option<f64>) -> Pedido {
        Pedido {
            titulo: "Canção de Teste".into(),
            artista: "Banda Tal".into(),
            album: String::new(),
            duracao_s: duracao,
        }
    }

    #[test]
    fn le_a_resposta_do_lrclib() {
        let json = r#"{"id":1,"trackName":"Canção de Teste","artistName":"Banda Tal","albumName":"Disco",
                       "duration":201.0,"instrumental":false,"plainLyrics":"linha de teste",
                       "syncedLyrics":"[00:01.00]linha de teste","campoNovo":true}"#;
        let r: Registro = serde_json::from_str(json).expect("json");
        assert_eq!(r, Registro { album_name: "Disco".into(), ..registro("Canção de Teste", "Banda Tal", 201.0, true) });
        let vazio: Registro = serde_json::from_str(r#"{"trackName":"x","syncedLyrics":null}"#).expect("json");
        assert!(!vazio.tem_algo());
    }

    #[test]
    fn prefere_a_sincronizada_com_a_duracao_certa() {
        let candidatos = vec![
            registro("Canção de Teste", "Banda Tal", 200.0, false),
            registro("Canção de Teste", "Banda Tal", 204.0, true),
            registro("Canção de Teste", "Banda Tal", 201.0, true),
            registro("Canção de Teste (Ao Vivo)", "Banda Tal", 330.0, true),
        ];
        let escolhida = escolher(candidatos, &pedido(Some(200.0))).expect("acha");
        assert_eq!(escolhida.duration, 201.0);
    }

    #[test]
    fn duracao_distante_nao_serve() {
        let candidatos = vec![registro("Canção de Teste", "Banda Tal", 260.0, true)];
        assert_eq!(escolher(candidatos.clone(), &pedido(Some(200.0))), None);
        // Sem saber a duração, a primeira sincronizada vale.
        assert!(escolher(candidatos, &pedido(None)).is_some());
    }

    #[test]
    fn so_texto_vale_quando_nao_ha_sincronizada() {
        let candidatos = vec![
            registro("Canção de Teste", "Banda Tal", 203.0, false),
            registro("cancao de teste", "BANDA TAL", 200.0, false),
        ];
        assert_eq!(escolher(candidatos, &pedido(Some(200.0))).expect("acha").duration, 200.0);
    }

    #[test]
    fn outra_musica_ou_outro_artista_nao_entram() {
        let candidatos = vec![
            registro("Outra Coisa", "Banda Tal", 200.0, true),
            registro("Canção de Teste", "Gente Diferente", 200.0, true),
            Registro {
                track_name: "Canção de Teste".into(),
                artist_name: "Banda Tal".into(),
                duration: 200.0,
                ..Default::default()
            },
        ];
        assert_eq!(escolher(candidatos.clone(), &pedido(Some(200.0))), None);
        // Sem artista conhecido (título vindo do navegador), o título basta.
        let sem_artista = Pedido { artista: String::new(), ..pedido(Some(200.0)) };
        assert_eq!(escolher(candidatos, &sem_artista).expect("acha").artist_name, "Gente Diferente");
    }
}
