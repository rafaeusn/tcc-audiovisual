//! O que a API local do Soloist conta, e o que isso quer dizer para a faixa que o Player pediu. Sem rede nem relógio:
//! quem chama traz a hora.

use provedores::{SituacaoDaFaixa, Tocando};
use serde_json::{Value, json};

/// Quanto antes do fim a faixa é dada por acabada: o Spotify emendaria outra (a fila é do Player).
const ANTES_DO_FIM_MS: f64 = 250.0;
/// Quanto esperar a faixa pedida começar.
const PRAZO_PARA_COMECAR_MS: u64 = 15_000;
/// Quanto uma faixa diferente da pedida precisa ficar para ser a mesma com outro id (o Spotify troca ids por país).
const OUTRO_ID_MS: u64 = 1_500;
/// De quanto em quanto tempo perguntar o estado enquanto a faixa não começa.
const PERGUNTAR_MS: u64 = 700;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Status {
    #[default]
    Parado,
    Tocando,
    Pausado,
    Carregando,
}

fn status_de(texto: Option<&str>) -> Option<Status> {
    match texto? {
        "idle" => Some(Status::Parado),
        "playing" => Some(Status::Tocando),
        "paused" => Some(Status::Pausado),
        "buffering" => Some(Status::Carregando),
        _ => None,
    }
}

/// O que o Soloist contou por último.
#[derive(Default)]
pub struct Leitura {
    /// Há uma conta nele (pareado).
    pub dentro: bool,
    /// É ele o aparelho que toca na conta.
    ativo: bool,
    status: Option<Status>,
    uri: String,
    duracao_ms: f64,
    /// Posição, hora dela e velocidade.
    ancora: Option<(f64, u64, f64)>,
    /// O último erro que ele respondeu, até ser lido.
    pub erro: Option<String>,
}

impl Leitura {
    pub fn receber(&mut self, v: &Value) {
        match v["type"].as_str().unwrap_or_default() {
            "auth_state" => {
                self.dentro = v["logged_in"].as_bool().unwrap_or(false);
                self.ler_ativo(v);
            }
            "device_changed" => self.ler_ativo(v),
            "playback_state" => {
                self.dentro = true;
                self.ler_ativo(v);
                self.ler_status(v);
                self.ler_faixa(&v["item"]);
                self.ler_posicao(&v["position"]);
            }
            "track_changed" => self.ler_faixa(&v["item"]),
            "playback_changed" => self.ler_status(v),
            "position_sync" => self.ler_posicao(&v["position"]),
            "error" => self.erro = v["message"].as_str().map(str::to_string),
            _ => {}
        }
    }

    fn ler_ativo(&mut self, v: &Value) {
        if let Some(ativo) = v["is_active"].as_bool() {
            self.ativo = ativo;
        }
    }

    fn ler_status(&mut self, v: &Value) {
        if let Some(s) = status_de(v["status"].as_str()) {
            self.status = Some(s);
        }
    }

    fn ler_faixa(&mut self, item: &Value) {
        let uri = item["uri"].as_str().unwrap_or_default();
        if uri != self.uri {
            self.uri = uri.to_string();
            self.duracao_ms = 0.0;
            // A posição conhecida era da outra.
            self.ancora = None;
        }
        if let Some(d) = item["decorations"]["playback"]["duration_ms"].as_f64() {
            self.duracao_ms = d.max(0.0);
        }
    }

    fn ler_posicao(&mut self, p: &Value) {
        if let Some(ms) = p["position_ms"].as_f64() {
            let velocidade = p["speed"].as_f64().unwrap_or(0.0).clamp(0.0, 4.0);
            self.ancora = Some((ms.max(0.0), p["timestamp_ms"].as_u64().unwrap_or(0), velocidade));
        }
    }

    /// Sai som (ou vai sair) dele agora.
    pub fn soando(&self) -> bool {
        self.ativo && matches!(self.status, Some(Status::Tocando | Status::Carregando))
    }

    /// A posição agora, em milissegundos.
    fn posicao_ms(&self, agora_ms: u64) -> f64 {
        let Some((ms, quando, velocidade)) = self.ancora else { return 0.0 };
        let anda = if quando == 0 || self.status != Some(Status::Tocando) {
            0.0
        } else {
            agora_ms.saturating_sub(quando) as f64 * velocidade
        };
        let p = ms + anda;
        if self.duracao_ms > 0.0 { p.min(self.duracao_ms) } else { p }
    }

    /// Uma faixa nova foi pedida: o que se sabia da anterior não vale mais (a mesma pedida de novo parece no fim).
    fn esquecer_a_faixa(&mut self) {
        (self.status, self.ancora, self.duracao_ms) = (None, None, 0.0);
        self.uri.clear();
    }
}

/// A faixa que o Player pediu.
pub struct Pedida {
    faixa: String,
    vez: u64,
    uri: String,
    /// A que tocava antes (um evento atrasado dela não é a faixa nova).
    anterior: String,
    pedida_ms: u64,
    perguntada_ms: u64,
    /// Desde quando toca uma faixa diferente da pedida.
    diferente_ms: Option<u64>,
    comecou: bool,
    /// Ir para cá quando começar (voltar a tocar depois de outro aparelho).
    pub ir_para_s: Option<f64>,
    /// A pessoa pausou antes de a faixa começar: pausa quando começar.
    pausar_ao_comecar: bool,
    fim: Option<(SituacaoDaFaixa, Option<String>)>,
    posicao_s: f64,
    duracao_s: f64,
}

/// O endereço de uma faixa no Spotify. Só ids do formato dele: o texto vai para outro programa.
pub fn uri_da_faixa(id: &str) -> Option<String> {
    (id.len() == 22 && id.bytes().all(|b| b.is_ascii_alphanumeric())).then(|| format!("spotify:track:{id}"))
}

pub fn comando(nome: &str) -> Value {
    json!({ "type": "command", "command": nome })
}

impl Pedida {
    /// Pede a faixa: o que mandar ao Soloist e o que acompanhar.
    pub fn nova(faixa: &str, vez: u64, leitura: &mut Leitura, agora_ms: u64) -> (Self, Option<Value>) {
        let uri = uri_da_faixa(faixa);
        let anterior = std::mem::take(&mut leitura.uri);
        leitura.esquecer_a_faixa();
        leitura.erro = None;
        let fim = uri.is_none().then(|| (SituacaoDaFaixa::Falhou, Some("esta faixa não é do Spotify".to_string())));
        let pedida = Self {
            faixa: faixa.to_string(),
            vez,
            uri: uri.clone().unwrap_or_default(),
            anterior,
            pedida_ms: agora_ms,
            perguntada_ms: agora_ms,
            diferente_ms: None,
            comecou: false,
            ir_para_s: None,
            pausar_ao_comecar: false,
            fim,
            posicao_s: 0.0,
            duracao_s: 0.0,
        };
        let tocar = uri.map(|uri| {
            let mut c = comando("play");
            c["uri"] = json!(uri);
            c
        });
        (pedida, tocar)
    }

    /// O que a leitura diz desta faixa. Devolve o que mandar ao Soloist.
    pub fn avaliar(&mut self, l: &mut Leitura, agora_ms: u64) -> Vec<Value> {
        // O outro aparelho devolveu a vez, com a mesma faixa.
        if self.em_outro_lugar() && l.soando() && l.uri == self.uri {
            self.fim = None;
        }
        if self.fim.is_some() {
            return Vec::new();
        }
        if self.comecou { self.avaliar_tocando(l, agora_ms) } else { self.avaliar_comeco(l, agora_ms) }
    }

    fn avaliar_comeco(&mut self, l: &mut Leitura, agora_ms: u64) -> Vec<Value> {
        let tocando = matches!(l.status, Some(Status::Tocando | Status::Pausado));
        let e_ela = l.uri == self.uri;
        let outro_id = !e_ela && !l.uri.is_empty() && l.uri != self.anterior && tocando;
        let desde = outro_id.then(|| *self.diferente_ms.get_or_insert(agora_ms));
        if desde.is_none() {
            self.diferente_ms = None;
        }
        let trocado = desde.is_some_and(|d| agora_ms.saturating_sub(d) >= OUTRO_ID_MS);
        if l.ativo && tocando && (e_ela || trocado) {
            self.comecou = true;
            l.erro = None;
            self.uri = l.uri.clone();
            self.medir(l, agora_ms);
            let mut ordens = Vec::new();
            if let Some(s) = self.ir_para_s.take() {
                ordens.push(ir(s));
            }
            if std::mem::take(&mut self.pausar_ao_comecar) {
                ordens.push(comando("pause"));
            }
            return ordens;
        }
        if let Some(erro) = l.erro.take() {
            self.falhou(format!("o Soloist recusou: {erro}"));
        } else if agora_ms.saturating_sub(self.pedida_ms) >= PRAZO_PARA_COMECAR_MS {
            self.falhou("o Soloist não começou a tocar esta faixa".into());
        } else if agora_ms.saturating_sub(self.perguntada_ms) >= PERGUNTAR_MS {
            self.perguntada_ms = agora_ms;
            return vec![comando("get_state")];
        }
        Vec::new()
    }

    fn avaliar_tocando(&mut self, l: &Leitura, agora_ms: u64) -> Vec<Value> {
        // (Antes de mudar a posição guardada: se a faixa trocou, a leitura já é da outra.)
        if l.uri != self.uri && !l.uri.is_empty() {
            self.fim = Some((SituacaoDaFaixa::Acabou, None));
            return vec![comando("pause")];
        }
        if !l.ativo {
            self.fim = Some((SituacaoDaFaixa::EmOutroLugar, None));
            return Vec::new();
        }
        if l.status == Some(Status::Parado) {
            if self.perto_do_fim() {
                self.fim = Some((SituacaoDaFaixa::Acabou, None));
            } else {
                self.falhou("o Soloist parou no meio da faixa".into());
            }
            return Vec::new();
        }
        self.medir(l, agora_ms);
        // (Uma posição de antes do pedido é da faixa anterior, ou desta mesma na vez passada.)
        let desta_vez = l.ancora.is_some_and(|(_, quando, _)| quando >= self.pedida_ms);
        let acabando = desta_vez && l.duracao_ms > 0.0 && l.posicao_ms(agora_ms) >= l.duracao_ms - ANTES_DO_FIM_MS;
        if acabando && l.status == Some(Status::Tocando) {
            self.fim = Some((SituacaoDaFaixa::Acabou, None));
            return vec![comando("pause")];
        }
        Vec::new()
    }

    fn medir(&mut self, l: &Leitura, agora_ms: u64) {
        self.posicao_s = l.posicao_ms(agora_ms) / 1000.0;
        self.duracao_s = l.duracao_ms / 1000.0;
    }

    fn perto_do_fim(&self) -> bool {
        self.duracao_s <= 0.0 || self.posicao_s >= self.duracao_s - 5.0
    }

    pub fn falhou(&mut self, motivo: String) {
        if self.fim.is_none() {
            self.fim = Some((SituacaoDaFaixa::Falhou, Some(motivo)));
        }
    }

    pub fn acabou(&self) -> bool {
        self.fim.is_some()
    }

    /// Não tocou (e não: acabou, ou foi para outro aparelho).
    pub fn falhou_mesmo(&self) -> bool {
        matches!(self.fim, Some((SituacaoDaFaixa::Falhou, _)))
    }

    pub fn vez(&self) -> u64 {
        self.vez
    }

    pub fn em_outro_lugar(&self) -> bool {
        matches!(self.fim, Some((SituacaoDaFaixa::EmOutroLugar, _)))
    }

    pub fn faixa(&self) -> &str {
        &self.faixa
    }

    pub fn posicao_s(&self) -> f64 {
        self.posicao_s
    }

    /// A pessoa pausou: se a faixa ainda não começou, vale quando começar.
    pub fn pausar(&mut self) -> Option<Value> {
        if !self.comecou {
            self.pausar_ao_comecar = true;
            return None;
        }
        (!self.acabou()).then(|| comando("pause"))
    }

    pub fn continuar(&mut self) -> Option<Value> {
        if !self.comecou {
            self.pausar_ao_comecar = false;
            return None;
        }
        (!self.acabou()).then(|| comando("play"))
    }

    pub fn ir(&mut self, segundos: f64) -> Option<Value> {
        if !self.comecou {
            self.ir_para_s = Some(segundos);
            return None;
        }
        (!self.acabou()).then(|| ir(segundos))
    }

    pub fn publicar(&self, l: &Leitura) -> Tocando {
        let (situacao, erro) = match &self.fim {
            Some((s, e)) => (*s, e.clone()),
            None if !self.comecou => (SituacaoDaFaixa::Carregando, None),
            None if l.status == Some(Status::Pausado) => (SituacaoDaFaixa::Pausada, None),
            // Enchendo o buffer no meio da faixa: segue tocando, com a posição parada.
            None => (SituacaoDaFaixa::Tocando, None),
        };
        Tocando {
            faixa: self.faixa.clone(),
            vez: self.vez,
            situacao,
            posicao_s: self.posicao_s,
            duracao_s: self.duracao_s,
            erro,
        }
    }
}

fn ir(segundos: f64) -> Value {
    let mut c = comando("seek");
    let ms = if segundos.is_finite() { (segundos.max(0.0) * 1000.0).min(86_400_000.0) } else { 0.0 };
    c["position_ms"] = json!(ms as u64);
    c
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    const A: &str = "2JRo0gjbX4GrCqBYdRohoo";
    const B: &str = "6rqhFgbbKwnb9MLmUQDhG6";

    fn estado(id: &str, status: &str, posicao_ms: u64, quando: u64) -> Value {
        json!({
            "type": "playback_state", "status": status, "is_active": true,
            "item": { "uri": format!("spotify:track:{id}"), "entity_type": "track",
                      "decorations": { "playback": { "duration_ms": 200_000 } } },
            "position": { "position_ms": posicao_ms, "timestamp_ms": quando, "speed": 1.0 },
        })
    }

    fn comecada(agora: u64) -> (Leitura, Pedida) {
        let mut l = Leitura { dentro: true, ..Default::default() };
        let (mut p, tocar) = Pedida::nova(A, 1, &mut l, agora);
        let tocar = tocar.expect("comando");
        assert_eq!((tocar["command"].as_str(), tocar["type"].as_str()), (Some("play"), Some("command")));
        assert_eq!(tocar["uri"], json!(format!("spotify:track:{A}")));
        l.receber(&estado(A, "playing", 0, agora));
        assert!(p.avaliar(&mut l, agora).is_empty());
        (l, p)
    }

    #[test]
    fn a_faixa_comeca_anda_e_acaba_antes_de_o_spotify_emendar_outra() {
        let (mut l, mut p) = comecada(1_000);
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Tocando);
        assert!(p.avaliar(&mut l, 61_000).is_empty());
        let t = p.publicar(&l);
        assert_eq!((t.posicao_s.round(), t.duracao_s), (60.0, 200.0));
        l.receber(&json!({"type":"playback_changed","status":"paused"}));
        l.receber(&json!({"type":"position_sync","position":{"position_ms":60_000,"timestamp_ms":61_000,"speed":0.0}}));
        assert!(p.avaliar(&mut l, 90_000).is_empty());
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Pausada);
        assert_eq!(p.publicar(&l).posicao_s, 60.0, "pausada não anda");
        l.receber(&json!({"type":"playback_changed","status":"playing"}));
        l.receber(&json!({"type":"position_sync","position":{"position_ms":60_000,"timestamp_ms":90_000,"speed":1.0}}));
        assert!(p.avaliar(&mut l, 229_000).is_empty());
        assert_eq!(p.avaliar(&mut l, 229_800)[0]["command"], "pause", "para antes de emendar");
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Acabou);
        assert!(p.avaliar(&mut l, 230_000).is_empty(), "uma vez só");
    }

    #[test]
    fn se_o_spotify_emendou_outra_ela_e_calada() {
        let (mut l, mut p) = comecada(1_000);
        l.receber(&json!({"type":"track_changed","item":{"uri":format!("spotify:track:{B}")}}));
        assert_eq!(p.avaliar(&mut l, 2_000)[0]["command"], "pause");
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Acabou);
    }

    #[test]
    fn evento_atrasado_da_anterior_nao_e_a_faixa_nova() {
        let (mut l, _) = comecada(1_000);
        let (mut p, _) = Pedida::nova(B, 2, &mut l, 10_000);
        l.receber(&estado(A, "playing", 9_000, 10_000));
        for agora in [10_050, 12_000, 14_000] {
            p.avaliar(&mut l, agora);
            assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Carregando);
        }
        l.receber(&estado(B, "playing", 0, 14_000));
        p.avaliar(&mut l, 14_100);
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Tocando);
    }

    #[test]
    fn a_mesma_faixa_com_outro_id_vale_depois_de_um_tempo() {
        let mut l = Leitura { dentro: true, ..Default::default() };
        let (mut p, _) = Pedida::nova(A, 1, &mut l, 0);
        l.receber(&estado(B, "playing", 0, 100));
        p.avaliar(&mut l, 200);
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Carregando);
        p.avaliar(&mut l, 1_800);
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Tocando);
        assert!(p.avaliar(&mut l, 2_000).is_empty(), "e ela não é tomada por outra faixa");
    }

    #[test]
    fn repetir_a_mesma_faixa_espera_o_estado_novo() {
        let (mut l, mut p) = comecada(1_000);
        p.avaliar(&mut l, 200_900);
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Acabou);
        let (mut p, _) = Pedida::nova(A, 2, &mut l, 201_000);
        p.avaliar(&mut l, 201_010);
        assert_eq!((p.publicar(&l).situacao, p.publicar(&l).vez), (SituacaoDaFaixa::Carregando, 2));
        assert_eq!(p.avaliar(&mut l, 201_800)[0]["command"], "get_state", "pergunta enquanto não começa");
    }

    #[test]
    fn o_que_da_errado_tem_motivo() {
        let mut l = Leitura { dentro: true, ..Default::default() };
        let (p, tocar) = Pedida::nova("../x", 1, &mut l, 0);
        assert!(tocar.is_none() && p.acabou(), "um id estranho não vai ao Soloist");
        let (mut p, _) = Pedida::nova(A, 2, &mut l, 0);
        p.avaliar(&mut l, 15_000);
        assert!(p.publicar(&l).erro.expect("motivo").contains("não começou"));
        let (mut p, _) = Pedida::nova(A, 3, &mut l, 0);
        l.receber(&json!({"type":"error","message":"not available"}));
        p.avaliar(&mut l, 100);
        assert!(p.publicar(&l).erro.expect("motivo").contains("not available"));
        let (mut l, mut p) = comecada(1_000);
        p.avaliar(&mut l, 10_000);
        l.receber(&json!({"type":"playback_changed","status":"idle"}));
        p.avaliar(&mut l, 10_100);
        assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Falhou, "parou no meio");
    }

    #[test]
    fn outro_aparelho_toma_a_vez() {
        let (mut l, mut p) = comecada(1_000);
        p.avaliar(&mut l, 31_000);
        l.receber(&json!({"type":"device_changed","is_active":false,"device_name":"Celular"}));
        assert!(p.avaliar(&mut l, 32_000).is_empty(), "nada de mandar pausar o outro");
        assert!(p.em_outro_lugar());
        assert_eq!(p.posicao_s().round(), 30.0, "a posição fica para voltar");
    }

    #[test]
    fn a_vez_devolvida_por_outro_aparelho_volta_a_contar() {
        let (mut l, mut p) = comecada(1_000);
        p.avaliar(&mut l, 31_000);
        l.receber(&json!({"type":"device_changed","is_active":false}));
        p.avaliar(&mut l, 32_000);
        assert!(p.em_outro_lugar());
        l.receber(&estado(A, "playing", 40_000, 41_000));
        p.avaliar(&mut l, 41_000);
        let t = p.publicar(&l);
        assert_eq!((t.situacao, t.posicao_s.round()), (SituacaoDaFaixa::Tocando, 40.0));
    }

    #[test]
    fn a_posicao_da_faixa_anterior_nao_acaba_com_a_nova() {
        let (mut l, _) = comecada(1_000);
        let (mut p, _) = Pedida::nova(B, 2, &mut l, 200_900);
        // Atrasada, da anterior, no fim.
        l.receber(
            &json!({"type":"position_sync","position":{"position_ms":199_900,"timestamp_ms":200_850,"speed":1.0}}),
        );
        l.receber(&json!({"type":"track_changed","item":{"uri":format!("spotify:track:{B}"),
            "decorations":{"playback":{"duration_ms":100_000}}}}));
        l.receber(&json!({"type":"device_changed","is_active":true}));
        l.receber(&json!({"type":"playback_changed","status":"playing"}));
        for agora in [201_000, 201_100, 201_500] {
            p.avaliar(&mut l, agora);
            assert_eq!(p.publicar(&l).situacao, SituacaoDaFaixa::Tocando);
        }
        // E a mesma faixa pedida de novo (repetir).
        let (mut l, _) = comecada(1_000);
        let (mut p, _) = Pedida::nova(A, 2, &mut l, 200_900);
        l.receber(&estado(A, "playing", 199_900, 200_850));
        for agora in [201_000, 201_100] {
            p.avaliar(&mut l, agora);
            assert_ne!(p.publicar(&l).situacao, SituacaoDaFaixa::Acabou);
        }
    }

    #[test]
    fn pausar_e_pular_antes_de_comecar_valem_quando_comeca() {
        let mut l = Leitura { dentro: true, ..Default::default() };
        let (mut p, _) = Pedida::nova(A, 1, &mut l, 0);
        assert!(p.pausar().is_none() && p.ir(42.5).is_none());
        l.receber(&estado(A, "playing", 0, 100));
        let ordens = p.avaliar(&mut l, 100);
        assert_eq!((&ordens[0]["command"], &ordens[0]["position_ms"]), (&json!("seek"), &json!(42_500)));
        assert_eq!(ordens[1]["command"], "pause");
        assert_eq!(p.ir(f64::NAN).expect("seek")["position_ms"], 0);
    }

    #[test]
    fn lixo_nao_quebra() {
        let mut l = Leitura::default();
        for v in [json!(null), json!([]), json!({"type":"playback_state"}), json!({"type":"track_changed","item":3})] {
            l.receber(&v);
        }
        assert_eq!(l.posicao_ms(10), 0.0);
        l.receber(&json!({"type":"auth_state","logged_in":false}));
        assert!(!l.dentro);
    }
}
