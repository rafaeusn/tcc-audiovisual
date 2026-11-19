//! O cartão "ouvindo" no Discord.
//!
//! O Discord aberto no computador expõe um canal local; o cartão é entregue por ele. Não há login, senha nem
//! rede: a identidade é a de quem está no Discord. Se o Discord não estiver aberto, nada acontece e o app tenta
//! de novo depois.
//!
//! O Discord descarta em silêncio atualizações muito próximas, então quem usa [`Presenca`] deve chamar
//! [`Presenca::mostrar`] só quando o cartão mudou de verdade (o próprio tipo ignora repetições).

use crate::Faixa;
use crate::capas::{Capa, artista_no_lastfm, pagina_no_lastfm};
use discord_rich_presence::activity::{Activity, ActivityType, Assets, StatusDisplayType, Timestamps};
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
    /// A capa e a foto do artista, quando a busca achou (chega um pouco depois do resto).
    pub capa: Option<Capa>,
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
                || a.capa != b.capa
                || match (a.intervalo, b.intervalo) {
                    (Some((ia, fa)), Some((ib, fb))) => (ia - ib).abs() > 3000 || (fa - fb).abs() > 3000,
                    (None, None) => false,
                    _ => true,
                }
        }
        _ => true,
    }
}

/// O que o Discord respondeu a um pedido: `Ok` ou o motivo da recusa (vem no campo `data.message`).
pub fn ler_resposta(op: u32, v: &serde_json::Value) -> Result<(), String> {
    let evento = v.get("evt").and_then(|e| e.as_str()).unwrap_or_default();
    // Opcode 2 = o Discord fechou a ligação (por exemplo, identificador de aplicativo inválido).
    if op == 2 || evento == "ERROR" {
        let motivo = v
            .get("data")
            .and_then(|d| d.get("message"))
            .or_else(|| v.get("message"))
            .and_then(|m| m.as_str())
            .unwrap_or("sem motivo informado");
        return Err(motivo.chars().take(200).collect());
    }
    Ok(())
}

/// A ligação com o Discord deste computador.
pub struct Presenca {
    cliente: DiscordIpcClient,
    id_do_app: String,
    ligado: bool,
    mostrado: Option<Cartao>,
    /// A última recusa do Discord, para a pessoa ver na ficha do módulo.
    pub recusa: Option<String>,
}

impl Presenca {
    /// `id_do_app` é o identificador do aplicativo no Discord (o nome e o ícone do cartão vêm dele).
    pub fn nova(id_do_app: &str) -> Self {
        Self {
            cliente: DiscordIpcClient::new(id_do_app),
            id_do_app: id_do_app.to_string(),
            ligado: false,
            mostrado: None,
            recusa: None,
        }
    }

    /// O Discord está aberto e aceitou a ligação.
    pub fn ligado(&self) -> bool {
        self.ligado
    }

    /// Tenta ligar-se ao Discord. Falhar é normal (ele pode não estar aberto). A resposta do aperto de mão é
    /// conferida: um identificador de aplicativo inválido aparece em [`Presenca::recusa`].
    pub fn ligar(&mut self) -> bool {
        if self.ligado {
            return true;
        }
        self.mostrado = None;
        if self.cliente.connect_ipc().is_err() {
            return false;
        }
        let aperto = serde_json::json!({ "v": 1, "client_id": self.id_do_app });
        let resposta = self
            .cliente
            .send(aperto, 0)
            .map_err(|e| e.to_string())
            .and_then(|()| self.cliente.recv().map_err(|e| e.to_string()))
            .and_then(|(op, v)| ler_resposta(op, &v));
        match resposta {
            Ok(()) => {
                self.ligado = true;
                self.recusa = None;
            }
            Err(motivo) => {
                log::warn!("o Discord recusou a ligação: {motivo}");
                self.recusa = Some(motivo);
                let _ = self.cliente.close();
            }
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
                    atividade = atividade.state(artista.as_str());
                }
                if let Some((inicio, fim)) = c.intervalo {
                    atividade = atividade.timestamps(Timestamps::new().start(inicio).end(fim));
                }
                // O título e o artista viram links para as páginas deles no Last.fm.
                let (pagina, do_artista) = (pagina_no_lastfm(&c.faixa), artista_no_lastfm(&c.faixa));
                if let Some(p) = pagina.as_deref() {
                    atividade = atividade.details_url(p);
                }
                if let Some(p) = do_artista.as_deref() {
                    atividade = atividade.state_url(p);
                }
                // A capa grande (com o nome do álbum ao passar o mouse) e a foto do artista no canto.
                let album = c
                    .capa
                    .as_ref()
                    .map(|k| k.nome_do_album.as_str())
                    .filter(|n| !n.is_empty())
                    .or(Some(c.faixa.album.as_str()).filter(|n| !n.trim().is_empty()))
                    .map(linha);
                if let Some(capa) = c.capa.as_ref() {
                    let mut imagens = Assets::new().large_image(capa.album.as_str());
                    if let Some(nome) = album.as_deref() {
                        imagens = imagens.large_text(nome);
                    }
                    if let Some(p) = pagina.as_deref() {
                        imagens = imagens.large_url(p);
                    }
                    if !capa.artista.is_empty() {
                        imagens = imagens.small_image(capa.artista.as_str()).small_text(artista.as_str());
                    }
                    atividade = atividade.assets(imagens);
                }
                self.cliente.set_activity(atividade)
            }
        };
        // O Discord responde a cada pedido. A resposta é lida: diz se o cartão foi aceito, e não fica acumulada no
        // canal.
        let resposta = feito.map_err(|e| e.to_string()).and_then(|()| self.cliente.recv().map_err(|e| e.to_string()));
        match resposta {
            Ok((op, v)) => match ler_resposta(op, &v) {
                Ok(()) => {
                    self.mostrado = cartao.cloned();
                    self.recusa = None;
                }
                Err(motivo) => {
                    // Recusado, mas a ligação continua: não reenvia o mesmo cartão em seguida.
                    log::warn!("o Discord recusou o cartão: {motivo}");
                    self.recusa = Some(motivo);
                    self.mostrado = cartao.cloned();
                }
            },
            Err(e) => {
                // O Discord foi fechado: a próxima tentativa de ligar recomeça do zero.
                log::debug!("o Discord não respondeu ao cartão: {e}");
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

    #[test]
    fn respostas_do_discord() {
        let pronto = serde_json::json!({ "cmd": "DISPATCH", "evt": "READY", "data": {} });
        assert_eq!(ler_resposta(1, &pronto), Ok(()));
        let aceito = serde_json::json!({ "cmd": "SET_ACTIVITY", "evt": null, "data": { "type": 2 } });
        assert_eq!(ler_resposta(1, &aceito), Ok(()));
        let erro = serde_json::json!({ "cmd": "SET_ACTIVITY", "evt": "ERROR", "data": { "code": 4000, "message": "campo inválido" } });
        assert_eq!(ler_resposta(1, &erro), Err("campo inválido".to_string()));
        let fechou = serde_json::json!({ "code": 4000, "message": "Invalid Client ID" });
        assert_eq!(ler_resposta(2, &fechou), Err("Invalid Client ID".to_string()));
    }

    fn cartao(titulo: &str, intervalo: Option<(i64, i64)>) -> Cartao {
        cartao_com_capa(titulo, intervalo, None)
    }

    fn cartao_com_capa(titulo: &str, intervalo: Option<(i64, i64)>, capa: Option<Capa>) -> Cartao {
        Cartao {
            faixa: Faixa { titulo: titulo.into(), artista: "Banda".into(), album: String::new() },
            intervalo,
            capa,
        }
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
        // A capa chegou depois: vale reenviar.
        let capa = Capa { album: "https://x/a.jpg".into(), ..Default::default() };
        assert!(mudou(Some(&a), Some(&cartao_com_capa("A", Some((1000, 201_000)), Some(capa)))));
    }
}
