//! O cartão "ouvindo" no Discord.
//!
//! O Discord aberto no computador expõe um canal local; o cartão é entregue por ele. Não há login, senha nem
//! rede: a identidade é a de quem está no Discord. Se o Discord não estiver aberto, nada acontece e o app tenta
//! de novo depois.
//!
//! O Discord descarta em silêncio atualizações muito próximas, então quem usa [`Presenca`] deve chamar
//! [`Presenca::mostrar`] só quando o cartão mudou de verdade (o próprio tipo ignora repetições).

use crate::Faixa;
use discord_rich_presence::activity::{Activity, ActivityType, StatusDisplayType, Timestamps};
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};

/// Limite de caracteres de cada linha do cartão (o Discord recusa mais que isso).
const MAXIMO_POR_LINHA: usize = 120;

/// O que o cartão mostra.
#[derive(Clone, Debug, PartialEq)]
pub struct Cartao {
    pub faixa: Faixa,
    /// (começo, fim) da música em milissegundos desde 1970, para a barra de progresso. Sem a duração, não há
    /// barra.
    pub intervalo: Option<(i64, i64)>,
}

/// Corta um texto no limite do Discord, que também exige pelo menos dois caracteres por linha.
pub fn linha(texto: &str) -> String {
    let limpo: String = texto.trim().chars().filter(|c| !c.is_control()).take(MAXIMO_POR_LINHA).collect();
    if limpo.chars().count() < 2 { format!("{limpo} ") } else { limpo }
}

/// O intervalo da barra de progresso a partir da posição e da duração (segundos) e da hora de agora
/// (milissegundos desde 1970).
pub fn intervalo(posicao_s: f64, duracao_s: f64, agora_ms: i64) -> Option<(i64, i64)> {
    if !posicao_s.is_finite() || !duracao_s.is_finite() || duracao_s <= 1.0 {
        return None;
    }
    let inicio = agora_ms - (posicao_s.clamp(0.0, duracao_s) * 1000.0) as i64;
    Some((inicio, inicio + (duracao_s * 1000.0) as i64))
}

/// O cartão mudou o bastante para valer um envio? A barra só é reenviada se a música pulou mais de 3 segundos
/// (a pessoa arrastou a posição); o tempo que passa sozinho o Discord já conta.
pub fn mudou(antes: Option<&Cartao>, agora: Option<&Cartao>) -> bool {
    match (antes, agora) {
        (None, None) => false,
        (Some(a), Some(b)) => {
            a.faixa != b.faixa
                || match (a.intervalo, b.intervalo) {
                    (Some((ia, fa)), Some((ib, fb))) => (ia - ib).abs() > 3000 || (fa - fb).abs() > 3000,
                    (None, None) => false,
                    _ => true,
                }
        }
        _ => true,
    }
}

/// A ligação com o Discord deste computador.
pub struct Presenca {
    cliente: DiscordIpcClient,
    ligado: bool,
    mostrado: Option<Cartao>,
}

impl Presenca {
    /// `id_do_app` é o identificador do aplicativo no Discord (o nome e o ícone do cartão vêm dele).
    pub fn nova(id_do_app: &str) -> Self {
        Self { cliente: DiscordIpcClient::new(id_do_app), ligado: false, mostrado: None }
    }

    /// O Discord está aberto e aceitou a ligação.
    pub fn ligado(&self) -> bool {
        self.ligado
    }

    /// Tenta ligar-se ao Discord. Falhar é normal (ele pode não estar aberto).
    pub fn ligar(&mut self) -> bool {
        if !self.ligado {
            self.ligado = self.cliente.connect().is_ok();
            self.mostrado = None;
        }
        self.ligado
    }

    /// Mostra o cartão (ou tira, com `None`). Só fala com o Discord se algo mudou. Devolve se a ligação segue de
    /// pé.
    pub fn mostrar(&mut self, cartao: Option<&Cartao>) -> bool {
        if !self.ligado || !mudou(self.mostrado.as_ref(), cartao) {
            return self.ligado;
        }
        let feito = match cartao {
            None => self.cliente.clear_activity(),
            Some(c) => {
                let (titulo, artista) = (linha(&c.faixa.titulo), linha(&c.faixa.artista));
                let mut atividade = Activity::new()
                    .activity_type(ActivityType::Listening)
                    .status_display_type(StatusDisplayType::Details)
                    .details(titulo);
                if !c.faixa.artista.trim().is_empty() {
                    atividade = atividade.state(artista);
                }
                if let Some((inicio, fim)) = c.intervalo {
                    atividade = atividade.timestamps(Timestamps::new().start(inicio).end(fim));
                }
                self.cliente.set_activity(atividade)
            }
        };
        match feito {
            Ok(()) => self.mostrado = cartao.cloned(),
            Err(e) => {
                // O Discord foi fechado: a próxima tentativa de ligar recomeça do zero.
                log::debug!("o Discord não aceitou o cartão: {e}");
                let _ = self.cliente.close();
                self.ligado = false;
                self.mostrado = None;
            }
        }
        self.ligado
    }

    /// Tira o cartão e fecha a ligação.
    pub fn desligar(&mut self) {
        if self.ligado {
            let _ = self.cliente.clear_activity();
            let _ = self.cliente.close();
        }
        self.ligado = false;
        self.mostrado = None;
    }
}

impl Drop for Presenca {
    fn drop(&mut self) {
        self.desligar();
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn cartao(titulo: &str, intervalo: Option<(i64, i64)>) -> Cartao {
        Cartao { faixa: Faixa { titulo: titulo.into(), artista: "Banda".into(), album: String::new() }, intervalo }
    }

    #[test]
    fn linhas_no_limite_do_discord() {
        assert_eq!(linha("  Música  "), "Música");
        assert_eq!(linha("A"), "A ", "o Discord exige dois caracteres");
        assert_eq!(linha(&"x".repeat(500)).chars().count(), 120);
        assert_eq!(linha("a\nb"), "ab");
    }

    #[test]
    fn barra_de_progresso() {
        assert_eq!(intervalo(30.0, 200.0, 1_000_000), Some((970_000, 1_170_000)));
        assert_eq!(intervalo(0.0, 0.5, 0), None, "sem duração não há barra");
        assert_eq!(intervalo(f64::NAN, 200.0, 0), None);
        assert_eq!(intervalo(500.0, 200.0, 1_000_000), Some((800_000, 1_000_000)), "posição além do fim é o fim");
    }

    #[test]
    fn so_envia_quando_muda_de_verdade() {
        let a = cartao("A", Some((1000, 201_000)));
        assert!(mudou(None, Some(&a)));
        assert!(mudou(Some(&a), None));
        assert!(!mudou(None, None));
        assert!(!mudou(Some(&a), Some(&a.clone())));
        // Um segundo de diferença é só o relógio; dez é a pessoa arrastando a música.
        assert!(!mudou(Some(&a), Some(&cartao("A", Some((2000, 202_000))))));
        assert!(mudou(Some(&a), Some(&cartao("A", Some((11_000, 211_000))))));
        assert!(mudou(Some(&a), Some(&cartao("B", Some((1000, 201_000))))));
        assert!(mudou(Some(&a), Some(&cartao("A", None))));
    }
}
