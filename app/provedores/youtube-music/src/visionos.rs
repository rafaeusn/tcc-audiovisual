//! O áudio pelo cliente do YouTube para visionOS (a "rota direta").
//!
//! É o caminho que o MixMusic e o limusic usam (conferido no código deles em 23/11/2026): o YouTube responde a
//! esse cliente, sem conta, com endereços de áudio que tocam como vêm, sem assinatura para decifrar, sem desafio de
//! JavaScript e sem PO token, e o googlevideo serve qualquer faixa de bytes deles. O AAC (itag 140) vem junto do
//! Opus; o motor do Player decodifica o AAC.
//!
//! A conversa: um ID de visitante (`youtubei/v1/visitor_id`), guardado por 6 horas, e o player do vídeo
//! (`youtubei/v1/player`) com esse ID. Se o YouTube pede para confirmar que não é um robô, o ID é trocado e o
//! pedido é refeito uma vez. Quando o YouTube muda o que pede a esse cliente, os valores abaixo mudam junto (como
//! nos projetos de referência e no yt-dlp).

use provedores::Causa;
use serde_json::{Value, json};
use std::time::{Duration, Instant};

const NOME: &str = "VISIONOS";
const VERSAO: &str = "1.02";
/// O número do cliente, para os cabeçalhos.
const NUMERO: &str = "101";
pub const AGENTE: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 15_7_3) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.0 Safari/605.1.15";
const APARELHO: (&str, &str, &str, &str) = ("Apple", "RealityDevice17,1", "visionOS", "26.5.23O471");
const ORIGEM: &str = "https://www.youtube.com";
/// Quanto tempo um ID de visitante vale.
const VALIDADE_DO_VISITANTE: Duration = Duration::from_secs(6 * 60 * 60);

/// O áudio escolhido de uma faixa.
#[derive(Clone, Debug, PartialEq)]
pub struct Audio {
    pub url: String,
    pub itag: u64,
    pub duracao_s: Option<f64>,
}

pub struct Direto {
    http: reqwest::Client,
    visitante: Option<(String, Instant)>,
}

/// Por que o áudio não veio, e de que tipo foi (o Player tenta de novo o que é passageiro).
#[derive(Clone, Debug, PartialEq)]
pub struct Falha {
    pub causa: Option<Causa>,
    pub motivo: String,
}

impl Falha {
    pub fn nova(causa: Causa, motivo: impl Into<String>) -> Self {
        Self { causa: Some(causa), motivo: motivo.into() }
    }
}

/// A resposta de erro do YouTube: `429` e `403` são calma, o servidor fora do ar é passageiro.
pub fn falha_do_status(status: u16) -> Falha {
    let causa = match status {
        403 | 429 => Causa::Limite,
        500.. => Causa::Rede,
        _ => Causa::Quebrado,
    };
    Falha::nova(causa, format!("o YouTube respondeu {status}"))
}

/// O motivo de o YouTube não tocar, em português.
fn motivo_do_youtube(status: &Value) -> Falha {
    let estado = status.get("status").and_then(Value::as_str).unwrap_or("?");
    let razao = status.get("reason").and_then(Value::as_str).unwrap_or("");
    match estado {
        "LOGIN_REQUIRED" if razao.contains("bot") => {
            Falha::nova(Causa::Limite, "o YouTube pediu para confirmar que não é um robô")
        }
        "LOGIN_REQUIRED" => Falha::nova(
            Causa::Indisponivel,
            format!("o YouTube só toca esta faixa para quem entrou na conta ({razao})"),
        ),
        "UNPLAYABLE" => Falha::nova(Causa::Indisponivel, format!("o YouTube não toca esta faixa aqui ({razao})")),
        "ERROR" => Falha::nova(Causa::Indisponivel, format!("a faixa não está disponível ({razao})")),
        outro => Falha { causa: None, motivo: format!("o YouTube respondeu {outro} ({razao})") },
    }
}

/// O áudio para tocar: o AAC (140; o 139 só se não houver o 140), com endereço direto.
pub fn escolher(resposta: &Value) -> Result<Audio, Falha> {
    let status = resposta.get("playabilityStatus").cloned().unwrap_or(Value::Null);
    if status.get("status").and_then(Value::as_str) != Some("OK") {
        return Err(motivo_do_youtube(&status));
    }
    let formatos = resposta
        .pointer("/streamingData/adaptiveFormats")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let com_url = |itag: u64| {
        formatos.iter().find(|f| {
            f.get("itag").and_then(Value::as_u64) == Some(itag)
                && f.get("url").and_then(Value::as_str).is_some_and(|u| !u.is_empty())
        })
    };
    let sem_aac = || Falha::nova(Causa::Indisponivel, "o YouTube não deu o áudio em AAC desta faixa");
    let f = com_url(140).or_else(|| com_url(139)).ok_or_else(sem_aac)?;
    let duracao_s = f
        .get("approxDurationMs")
        .and_then(Value::as_str)
        .and_then(|ms| ms.parse::<f64>().ok())
        .map(|ms| ms / 1000.0)
        .filter(|s| s.is_finite() && *s > 0.0);
    Ok(Audio {
        url: f.get("url").and_then(Value::as_str).unwrap_or_default().to_string(),
        itag: f.get("itag").and_then(Value::as_u64).unwrap_or(0),
        duracao_s,
    })
}

impl Direto {
    pub fn novo() -> Result<Self, String> {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self { http, visitante: None })
    }

    /// Um pedido à API do YouTube como o cliente visionOS, sem conta.
    async fn chamar(&self, ponto: &str, corpo: Value, visitante: Option<&str>) -> Result<Value, Falha> {
        let (marca, modelo, so, versao_do_so) = APARELHO;
        let mut cliente = json!({
            "clientName": NOME, "clientVersion": VERSAO, "deviceMake": marca, "deviceModel": modelo,
            "osName": so, "osVersion": versao_do_so, "userAgent": AGENTE, "hl": "en", "timeZone": "UTC",
            "utcOffsetMinutes": 0,
        });
        if let Some(v) = visitante {
            cliente["visitorData"] = json!(v);
        }
        let mut dados = json!({ "context": { "client": cliente } });
        if let (Some(d), Value::Object(extra)) = (dados.as_object_mut(), corpo) {
            d.extend(extra);
        }
        let mut pedido = self
            .http
            .post(format!("{ORIGEM}/youtubei/v1/{ponto}?prettyPrint=false"))
            .header("Content-Type", "application/json")
            .header("User-Agent", AGENTE)
            .header("Origin", ORIGEM)
            .header("X-YouTube-Client-Name", NUMERO)
            .header("X-YouTube-Client-Version", VERSAO)
            .body(dados.to_string());
        if let Some(v) = visitante {
            pedido = pedido.header("X-Goog-Visitor-Id", v);
        }
        let sem_resposta =
            |e: reqwest::Error| Falha::nova(Causa::Rede, format!("sem resposta do YouTube: {}", e.without_url()));
        let resposta = pedido.send().await.map_err(sem_resposta)?;
        if !resposta.status().is_success() {
            return Err(falha_do_status(resposta.status().as_u16()));
        }
        resposta.json::<Value>().await.map_err(|e| match e.is_decode() {
            true => Falha::nova(Causa::Quebrado, format!("resposta estranha do YouTube: {}", e.without_url())),
            false => sem_resposta(e),
        })
    }

    async fn visitante(&mut self) -> Result<String, Falha> {
        if let Some((v, desde)) = &self.visitante
            && desde.elapsed() < VALIDADE_DO_VISITANTE
        {
            return Ok(v.clone());
        }
        let r = self.chamar("visitor_id", json!({}), None).await?;
        let v = r
            .pointer("/responseContext/visitorData")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| Falha::nova(Causa::Quebrado, "o YouTube não deu um ID de visitante"))?
            .to_string();
        self.visitante = Some((v.clone(), Instant::now()));
        Ok(v)
    }

    /// O áudio de um vídeo. Um ID de visitante recusado é trocado e o pedido refeito uma vez.
    pub async fn audio(&mut self, video: &str) -> Result<Audio, Falha> {
        let corpo = || {
            json!({
                "videoId": video, "contentCheckOk": true, "racyCheckOk": true,
                "playbackContext": { "contentPlaybackContext": { "html5Preference": "HTML5_PREF_WANTS" } },
            })
        };
        let visitante = self.visitante().await?;
        let resposta = self.chamar("player", corpo(), Some(&visitante)).await?;
        match escolher(&resposta) {
            Err(falha) if falha.causa == Some(Causa::Limite) => {
                self.visitante = None;
                let novo = self.visitante().await?;
                escolher(&self.chamar("player", corpo(), Some(&novo)).await?)
            }
            outro => {
                // O YouTube às vezes devolve um ID novo junto: vale o dele.
                if let Some(v) = resposta.pointer("/responseContext/visitorData").and_then(Value::as_str)
                    && !v.is_empty()
                    && v != visitante
                {
                    self.visitante = Some((v.to_string(), Instant::now()));
                }
                outro
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn escolhe_o_aac_com_endereco() {
        let r = json!({
            "playabilityStatus": { "status": "OK" },
            "streamingData": { "adaptiveFormats": [
                { "itag": 251, "url": "https://opus", "approxDurationMs": "429533" },
                { "itag": 139, "url": "https://aac-baixo" },
                { "itag": 140, "url": "https://aac", "approxDurationMs": "429533" },
            ]}
        });
        assert_eq!(
            escolher(&r),
            Ok(Audio { url: "https://aac".into(), itag: 140, duracao_s: Some(429.533) }),
            "o 140 ganha do 139 e o Opus fica de fora"
        );
        let so_139 = json!({
            "playabilityStatus": { "status": "OK" },
            "streamingData": { "adaptiveFormats": [{ "itag": 140 }, { "itag": 139, "url": "https://b" }] }
        });
        assert_eq!(escolher(&so_139).map(|a| a.itag), Ok(139), "o 140 sem endereço não serve");
        let sem_aac = json!({ "playabilityStatus": { "status": "OK" }, "streamingData": { "adaptiveFormats": [] } });
        assert!(escolher(&sem_aac).is_err());
    }

    #[test]
    fn diz_por_que_o_youtube_nao_tocou() {
        let robo = json!({ "playabilityStatus": { "status": "LOGIN_REQUIRED", "reason": "Sign in to confirm you're not a bot" } });
        let calma = Falha::nova(Causa::Limite, "o YouTube pediu para confirmar que não é um robô");
        assert_eq!(escolher(&robo), Err(calma), "passa: não é desta faixa");
        let fora =
            json!({ "playabilityStatus": { "status": "UNPLAYABLE", "reason": "not available in your country" } });
        assert!(escolher(&fora).is_err_and(|f| {
            f.causa == Some(Causa::Indisponivel) && f.motivo.contains("não toca") && f.motivo.contains("country")
        }));
        assert!(escolher(&json!({})).is_err_and(|f| f.causa.is_none()), "resposta vazia não quebra");
        assert_eq!(falha_do_status(429).causa, Some(Causa::Limite));
        assert_eq!(falha_do_status(503).causa, Some(Causa::Rede));
        assert_eq!(falha_do_status(400).causa, Some(Causa::Quebrado));
    }
}
