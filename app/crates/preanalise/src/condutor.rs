//! O condutor: o "modo arquivo" do diretor (porte de `updateTimeline` em `preview/director.js`).
//!
//! Com a música pré-analisada, o visual não precisa adivinhar: a cada quadro o condutor lê a linha do tempo no
//! instante exato que está soando. A energia vem do degrau do trecho (sem o atraso do ao vivo), o pump dispara em
//! cada batida (mais forte no tempo forte do compasso), os ataques entre as batidas só contam quando a música não
//! tem batida clara, o drop cai no ataque grave exato e a subida começa antes dele. A fase da batida vem das
//! batidas reais.

use crate::Analise;
use analise::Rotulo;

/// O que o condutor entrega a cada quadro (os mesmos campos que o diretor ao vivo dá ao visual).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Conducao {
    pub energia: f32,
    pub volume: f32,
    pub atividade: f32,
    pub pulso: f32,
    pub drop: f32,
    pub preparo: f32,
    pub conf_batida: f32,
    pub bpm: f32,
    pub fase_batida: f32,
    pub rotulo: Rotulo,
    /// A soma dos ataques disparados neste quadro (o visual usa para empurrar a forma).
    pub impulso: f32,
    /// Em que tempo do compasso estamos (0 = o tempo forte), quando a música tem batida.
    pub tempo_do_compasso: Option<u8>,
}

#[derive(Default)]
pub struct Condutor {
    anterior: Option<f64>,
    b: usize,
    o: usize,
    d: usize,
    ultimo_forte: Option<usize>,
    energia: f32,
    volume: f32,
    atividade: f32,
    pulso: f32,
    drop: f32,
    rotulo: Rotulo,
    iniciado: bool,
}

/// O primeiro índice cujo instante passa de `t` (busca binária).
fn primeiro_depois(tempos: impl Fn(usize) -> f64, n: usize, t: f64) -> usize {
    let (mut lo, mut hi) = (0, n);
    while lo < hi {
        let m = (lo + hi) / 2;
        if tempos(m) <= t {
            lo = m + 1;
        } else {
            hi = m;
        }
    }
    lo
}

impl Condutor {
    /// Esquece a posição (música nova, pausa longa): no próximo quadro reposiciona sem disparar eventos velhos.
    pub fn reiniciar(&mut self) {
        self.anterior = None;
    }

    /// Um quadro. `t` é o instante da música que está soando agora; `porta` é 1 com som e 0 em silêncio.
    pub fn conduzir(&mut self, a: &Analise, dt: f32, t: f64, porta: f32) -> Conducao {
        let ema = |v: f32, x: f32, tau: f32| v + (x - v) * (1.0 - (-dt / tau).exp());
        if !self.iniciado {
            // Começa no ponto certo, sem a subida lenta do zero.
            self.energia = a.curva_em(&a.energia, t);
            self.iniciado = true;
        }
        // A curva já vem suavizada e alinhada aos trechos: só um filtro curto para não pular de quadro em quadro.
        self.energia = ema(self.energia, a.curva_em(&a.energia, t), 0.12);
        self.volume = ema(self.volume, a.curva_em(&a.volume, t) * porta, 0.08);
        self.atividade = ema(self.atividade, a.curva_em(&a.atividade, t), 0.2);
        let preparo = a.curva_em(&a.preparo, t);
        let conf_batida = a.curva_em(&a.conf, t);
        let mut impulso = 0.0f32;

        let (batidas, ataques, drops) = (&a.batidas, &a.ataques, &a.drops);
        let anterior = self.anterior.replace(t);
        let salto = anterior.is_none_or(|p| t < p - 0.05 || t - p > 1.0);
        if salto {
            // Começo ou salto (o usuário foi para outro ponto): reposiciona sem disparar eventos atrasados.
            self.b = primeiro_depois(|i| batidas[i].t, batidas.len(), t);
            self.o = primeiro_depois(|i| ataques[i].t, ataques.len(), t);
            self.d = primeiro_depois(|i| drops[i].t, drops.len(), t);
            self.ultimo_forte = (self.b.saturating_sub(4)..self.b).rev().find(|&i| batidas[i].forte);
        } else {
            let energia = self.energia;
            let mut disparar = |amp: f32, pulso: &mut f32| {
                let amp = amp * porta;
                if amp > 0.0 {
                    *pulso = pulso.max(amp);
                    impulso += amp;
                }
            };
            while self.b < batidas.len() && batidas[self.b].t <= t {
                let b = batidas[self.b];
                if b.forte {
                    self.ultimo_forte = Some(self.b);
                }
                // O tempo forte do compasso pesa mais; o pump cresce com a energia do trecho.
                disparar(b.forca * if b.forte { 1.0 } else { 0.75 } * (0.25 + 0.75 * energia), &mut self.pulso);
                self.b += 1;
            }
            while self.o < ataques.len() && ataques[self.o].t <= t {
                let o = ataques[self.o];
                let proxima = batidas.get(self.b.min(batidas.len().saturating_sub(1)));
                let anterior_b = batidas.get(self.b.saturating_sub(1));
                let distancia = proxima
                    .map_or(f64::INFINITY, |b| (o.t - b.t).abs())
                    .min(anterior_b.map_or(f64::INFINITY, |b| (o.t - b.t).abs()));
                let perto_da_batida = !batidas.is_empty() && distancia < 0.07;
                // Sem batida clara (fala, piano, orquestra), os ataques conduzem o pump; com batida, só os acentos
                // grandes entre as batidas (prato, impacto).
                if !(perto_da_batida && conf_batida > 0.3) {
                    let peso = if conf_batida < 0.4 {
                        0.9
                    } else if o.forca > 0.8 {
                        0.3
                    } else {
                        0.0
                    };
                    if peso > 0.0 {
                        disparar(o.forca * (0.15 + 0.85 * energia) * peso, &mut self.pulso);
                    }
                }
                self.o += 1;
            }
            while self.d < drops.len() && drops[self.d].t <= t {
                let d = drops[self.d];
                self.drop = self.drop.max(d.forca);
                disparar(d.forca, &mut self.pulso);
                self.d += 1;
            }
        }
        self.pulso *= (-dt / 0.15).exp();
        self.drop *= (-dt / 0.9).exp();

        // Fase da batida pelas batidas reais.
        let i = self.b;
        let fase_batida = if i > 0 && i < batidas.len() {
            let (b0, b1) = (batidas[i - 1].t, batidas[i].t);
            ((t - b0) / (b1 - b0).max(0.05)).clamp(0.0, 1.0) as f32
        } else {
            (t * f64::from(if a.bpm > 0.0 { a.bpm } else { 120.0 }) / 60.0).rem_euclid(1.0) as f32
        };

        let x = self.energia;
        self.rotulo = match self.rotulo {
            Rotulo::Calma if x > 0.42 => Rotulo::Ativa,
            Rotulo::Ativa if x > 0.74 => Rotulo::Intensa,
            Rotulo::Ativa if x < 0.34 => Rotulo::Calma,
            Rotulo::Intensa if x < 0.66 => Rotulo::Ativa,
            outro => outro,
        };
        let tempo_do_compasso = match self.ultimo_forte {
            Some(f) if i > f && a.clareza_da_batida > 0.3 => Some(((i - 1 - f) % 4) as u8),
            _ => None,
        };
        Conducao {
            tempo_do_compasso,
            energia: self.energia,
            volume: self.volume,
            atividade: self.atividade,
            pulso: self.pulso,
            drop: self.drop,
            preparo,
            conf_batida,
            bpm: a.bpm,
            fase_batida,
            rotulo: self.rotulo,
            impulso,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::{Batida, Drop};

    fn analise() -> Analise {
        Analise {
            fps: 10.0,
            bpm: 120.0,
            energia: vec![0.9; 200],
            volume: vec![0.8; 200],
            atividade: vec![0.5; 200],
            preparo: vec![0.0; 200],
            conf: vec![1.0; 200],
            batidas: (0..40).map(|i| Batida { t: f64::from(i) * 0.5, forca: 1.0, forte: i % 4 == 0 }).collect(),
            drops: vec![Drop { t: 8.0, forca: 1.0 }],
            ..Default::default()
        }
    }

    #[test]
    fn pump_na_batida_fase_e_drop_no_instante() {
        let a = analise();
        let mut c = Condutor::default();
        let dt = 1.0 / 60.0;
        let mut t = 0.9;
        c.conduzir(&a, dt, t, 1.0);
        let mut pulsos = Vec::new();
        let mut drop_em = None;
        while t < 9.0 {
            t += f64::from(dt);
            let q = c.conduzir(&a, dt, t, 1.0);
            if q.impulso > 0.0 {
                pulsos.push(t);
            }
            if q.drop > 0.9 && drop_em.is_none() {
                drop_em = Some(t);
            }
        }
        // Uma batida a cada 0,5 s, e cada pump cai no quadro logo depois da batida.
        assert!(pulsos.len() >= 16, "{} pulsos", pulsos.len());
        assert!(pulsos.iter().all(|p| (p * 2.0).fract() < 0.05 || (p * 2.0).fract() > 0.99), "{pulsos:?}");
        assert!(drop_em.is_some_and(|d| (d - 8.0).abs() < 0.03), "drop em {drop_em:?}");
        // A fase volta a zero em cada batida.
        let q = c.conduzir(&a, dt, 9.26, 1.0);
        assert!((q.fase_batida - 0.52).abs() < 0.05, "fase {}", q.fase_batida);
        assert_eq!(q.rotulo, Rotulo::Intensa);
    }

    #[test]
    fn ir_para_outro_ponto_nao_dispara_o_que_ficou_para_tras() {
        let a = analise();
        let mut c = Condutor::default();
        c.conduzir(&a, 1.0 / 60.0, 1.0, 1.0);
        let q = c.conduzir(&a, 1.0 / 60.0, 15.0, 1.0);
        assert_eq!(q.impulso, 0.0, "pular de 1 s para 15 s não dispara as batidas do meio nem o drop");
        assert_eq!(q.drop, 0.0);
        // Em silêncio (porta fechada), nada dispara.
        let q = c.conduzir(&a, 0.6, 15.6, 0.0);
        assert_eq!(q.impulso, 0.0);
    }
}
