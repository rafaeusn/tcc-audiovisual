//! Diretor automático, modo ao vivo (porte de `preview/director.js`).
//!
//! Não assume gênero, BPM nem bumbo. Trabalha em três escalas de tempo:
//! 1. **Energia** (lenta, 1–2 s): quão animado está o trecho. Conduz cor, luz e distorção.
//! 2. **Pulso** (rápido, por ataque): um "pump" curto a cada ataque. Conduz o estalo da forma, o brilho e o bloom.
//! 3. **Eventos**: drop (salto de grave ou de volume) e subida (energia crescendo).
//!
//! As constantes são as calibradas no corpus (`docs/diretor-calibracao.md`). Mudar aqui exige recalibrar.

use crate::descritores::Bandas;
use crate::ritmo::Faixa;
use std::collections::VecDeque;

/// Escalas e pesos do modo ao vivo (`LIVE` no JS).
struct Vivo {
    loud_offset: f32,
    pulse_lo: f32,
    pulse_hi: f32,
    noise_lo: f32,
    noise_hi: f32,
    full_lo: f32,
    full_hi: f32,
    w_loud: f32,
    w_pulse: f32,
    w_tempo: f32,
    w_noise: f32,
    w_full: f32,
    w_rel: f32,
    floor: f32,
}

const VIVO: Vivo = Vivo {
    loud_offset: 28.0,
    pulse_lo: 0.1,
    pulse_hi: 0.5,
    noise_lo: 0.55,
    noise_hi: 0.95,
    full_lo: 0.35,
    full_hi: 0.75,
    w_loud: 0.32,
    w_pulse: 0.15,
    w_tempo: 0.15,
    w_noise: 0.15,
    w_full: 0.12,
    w_rel: 0.22,
    floor: 0.14,
};

/// Cor pela energia: calma = branco, agitada = azul, intensa = vermelho.
const PALETAS: [(f32, [f32; 3], [f32; 3]); 5] = [
    (0.0, [1.0, 1.0, 1.0], [0.87, 0.9, 1.0]),
    (0.3, [1.0, 1.0, 1.0], [0.87, 0.9, 1.0]),
    (0.55, [0.18, 0.42, 1.0], [0.56, 0.89, 1.0]),
    (0.75, [0.18, 0.42, 1.0], [0.56, 0.89, 1.0]),
    (0.92, [1.0, 0.1, 0.22], [1.0, 0.55, 0.22]),
];

pub fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Rotulo {
    #[default]
    Calma,
    Ativa,
    Intensa,
}

impl Rotulo {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Calma => "Calma",
            Self::Ativa => "Agitada",
            Self::Intensa => "Intensa",
        }
    }
}

/// O que o ritmo entrega ao diretor a cada passo.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pulso {
    pub forca: f32,
    pub faixa: Faixa,
    pub groove: f32,
    pub bpm: f32,
    pub densidade: f32,
}

/// Parâmetros que o diretor decide pela energia: `[calmo, intenso]` (`RANGES` no JS).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dirigidos {
    pub speed: f32,
    pub intensity: f32,
    pub aberr: f32,
    pub exposure: f32,
    pub warp: f32,
    // Só no Spectral:
    pub react: f32,
    pub spread: f32,
    pub motion: f32,
    pub scale: f32,
    pub bloom: f32,
    pub feedback: f32,
    pub punch: f32,
}

impl Dirigidos {
    pub fn para(energia: f32) -> Self {
        let e = energia.clamp(0.0, 1.0);
        Self {
            speed: lerp(0.85, 1.35, e),
            intensity: lerp(0.85, 1.35, e),
            aberr: lerp(0.15, 1.0, e),
            exposure: lerp(0.8, 2.3, e),
            warp: lerp(0.25, 1.2, e),
            react: lerp(0.65, 1.35, e),
            spread: lerp(0.1, 0.45, e),
            motion: lerp(0.25, 0.5, e),
            scale: lerp(1.35, 1.8, e),
            bloom: lerp(0.9, 1.6, e),
            feedback: lerp(0.1, 0.38, e),
            punch: lerp(0.0, 0.45, e),
        }
    }
}

/// As duas cores do visual para uma energia e um drop (0–1 cada).
pub fn cores(energia: f32, drop: f32) -> ([f32; 3], [f32; 3]) {
    // No drop a cor dá um salto em direção ao vermelho e volta.
    let x = (energia + 0.35 * drop).clamp(0.0, 1.0);
    let mut i = 0;
    while i < PALETAS.len() - 2 && x > PALETAS[i + 1].0 {
        i += 1;
    }
    let (p0, p1) = (PALETAS[i], PALETAS[i + 1]);
    let t = smoothstep(p0.0, p1.0, x);
    let mix = |a: [f32; 3], b: [f32; 3]| [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)];
    (mix(p0.1, p1.1), mix(p0.2, p1.2))
}

pub struct Diretor {
    play_time: f64,
    pub gate: f32,
    a: Option<f32>,
    af: Option<f32>,
    /// Energia final (0–1): cor, luz, distorção.
    pub intensity: f32,
    /// Volume rápido (0–1): amplitude da morfose.
    pub loud: f32,
    /// Atividade rápida (0–1): velocidade da morfose.
    pub activity: f32,
    /// Envelope do pump (0–1).
    pub pulse: f32,
    /// Soma de todos os ataques desde o início. Quem desenha usa a diferença entre dois quadros.
    pub impulso_total: f64,
    pub groove: f32,
    pub bpm: f32,
    pub drop: f32,
    pub build: f32,
    pub beat_phase: f32,
    pub beat_conf: f32,
    pub rotulo: Rotulo,
    /// Ganho automático de entrada (sensibilidade dos descritores).
    pub sens: f32,
    peak: f32,
    level: f32,
    bass_fast: f32,
    bass_slow: Option<f32>,
    bass_ref: Option<f32>,
    low_bass_time: f32,
    db_mu: Option<f32>,
    db_fast: Option<f32>,
    db_hist: VecDeque<(f64, f32)>,
    last_drop: f64,
    e_slow: f32,
}

impl Default for Diretor {
    fn default() -> Self {
        Self {
            play_time: 0.0,
            gate: 0.0,
            a: None,
            af: None,
            intensity: 0.25,
            loud: 0.0,
            activity: 0.0,
            pulse: 0.0,
            impulso_total: 0.0,
            groove: 0.0,
            bpm: 0.0,
            drop: 0.0,
            build: 0.0,
            beat_phase: 0.0,
            beat_conf: 0.0,
            rotulo: Rotulo::Calma,
            sens: 1.2,
            peak: 0.3,
            level: 0.0,
            bass_fast: 0.0,
            bass_slow: None,
            bass_ref: None,
            low_bass_time: 0.0,
            db_mu: None,
            db_fast: None,
            db_hist: VecDeque::new(),
            last_drop: -99.0,
            e_slow: 0.25,
        }
    }
}

impl Diretor {
    pub fn atualizar(&mut self, dt: f32, b: &Bandas, r: Pulso) {
        self.play_time += dt as f64;
        let now = self.play_time;
        let ema = |v: f32, x: f32, tau: f32| v + (x - v) * (1.0 - (-dt / tau).exp());

        // Porta de ruído: abaixo de ~-50 dBFS é silêncio.
        let audivel = b.rms_db > -50.0;
        self.gate = ema(self.gate, if audivel { 1.0 } else { 0.0 }, if audivel { 0.15 } else { 0.6 });

        self.ao_vivo(dt, b, r, now);

        // Ganho automático: mantém o pico recente do nível das bandas perto de 0,3.
        self.level = ema(self.level, b.level, 0.5);
        self.peak = self.level.max(self.peak - (self.peak - self.level) * dt / 40.0);
        if self.gate > 0.5 && now > 2.0 {
            let quero = self.sens * 0.3 / self.peak.max(0.08);
            let tau = lerp(3.0, 10.0, smoothstep(0.0, 20.0, now as f32));
            self.sens = ema(self.sens, quero, tau).clamp(0.45, 1.8);
        }

        let x = self.intensity;
        self.rotulo = match self.rotulo {
            Rotulo::Calma if x > 0.42 => Rotulo::Ativa,
            Rotulo::Ativa if x > 0.74 => Rotulo::Intensa,
            Rotulo::Ativa if x < 0.34 => Rotulo::Calma,
            Rotulo::Intensa if x < 0.66 => Rotulo::Ativa,
            outro => outro,
        };
    }

    fn ao_vivo(&mut self, dt: f32, b: &Bandas, r: Pulso, now: f64) {
        let ema = |v: f32, x: f32, tau: f32| v + (x - v) * (1.0 - (-dt / tau).exp());
        let l = &VIVO;
        let loud01 = ((b.rms_db + l.loud_offset) / 22.0).clamp(0.0, 1.0);
        self.loud = ema(self.loud, loud01 * self.gate, 0.3);
        self.groove = r.groove;
        self.bpm = r.bpm;
        let pulse = smoothstep(l.pulse_lo, l.pulse_hi, r.groove);
        let bpm_fold = if r.bpm > 180.0 {
            r.bpm / 2.0
        } else if r.bpm < 70.0 {
            r.bpm * 2.0
        } else {
            r.bpm
        };
        let tempo01 = if r.bpm > 0.0 { smoothstep(80.0, 160.0, bpm_fold) } else { 0.0 };
        let noise = smoothstep(l.noise_lo, l.noise_hi, b.flatness);
        let full = smoothstep(l.full_lo, l.full_hi, b.fullness);
        let drive = smoothstep(0.1, 0.55, loud01);

        // Dinâmica relativa: volume comparado à média longa da própria fonte.
        let db_mu = ema(self.db_mu.unwrap_or(b.rms_db), b.rms_db, 30.0);
        self.db_mu = Some(db_mu);
        let db_fast = ema(self.db_fast.unwrap_or(b.rms_db), b.rms_db, 1.0);
        self.db_fast = Some(db_fast);
        let rel = ((db_fast - db_mu) / 10.0).clamp(-1.0, 1.0) * smoothstep(2.0, 12.0, now as f32);
        let a =
            l.w_loud * loud01 + (l.w_pulse + l.w_tempo * tempo01) * pulse * drive + l.w_noise * noise + l.w_full * full;
        let a_raw = l.floor + (1.0 - l.floor) * (a + l.w_rel * rel).clamp(0.0, 1.0);
        let a_suave = ema(self.a.unwrap_or(a_raw), a_raw, 1.2);
        self.a = Some(a_suave);
        let af = ema(self.af.unwrap_or(a_raw), a_raw, 0.35);
        self.af = Some(af);
        let alvo = a_suave * self.gate;
        self.intensity = ema(self.intensity, alvo, if alvo > self.intensity { 0.8 } else { 1.6 });
        self.e_slow = ema(self.e_slow, self.intensity, 5.0);

        let dens01 = smoothstep(1.0, 3.2, r.densidade);
        self.activity = ema(self.activity, (0.45 * self.intensity + 0.3 * dens01 + 0.25 * pulse) * self.gate, 0.4);

        // Pulso: bumbo e caixa (graves e médios) dão o pump; agudos pesam pouco.
        if r.forca > 0.0 {
            let (peso, minimo) = match r.faixa {
                Faixa::Low => (1.0, 0.06),
                Faixa::Mid => (0.75, 0.06),
                Faixa::High => (0.2, 0.15),
            };
            let amp = r.forca * peso * (0.2 + 0.8 * self.intensity) * self.gate;
            if amp > minimo {
                self.pulse = self.pulse.max(amp);
                self.impulso_total += amp as f64;
            }
        }
        self.pulse *= (-dt / 0.15).exp();

        // Drop: depois de um trecho sem grave (2 s abaixo de 35% do nível típico) o grave volta acima de 75%,
        // ou o volume salta 6 dB sobre o mínimo dos últimos 3 s.
        let bass_n = (b.bass * 1.2 / self.sens).min(1.5);
        self.bass_fast = ema(self.bass_fast, bass_n, 0.2);
        let bass_slow = ema(self.bass_slow.unwrap_or(bass_n), bass_n, 1.5);
        self.bass_slow = Some(bass_slow);
        if self.gate > 0.5 {
            self.bass_ref = Some(ema(self.bass_ref.unwrap_or(bass_n), bass_n, 20.0));
        }
        let referencia = self.bass_ref.unwrap_or(0.3).max(0.15);
        self.low_bass_time = if bass_slow < 0.35 * referencia {
            self.low_bass_time + dt
        } else {
            (self.low_bass_time - dt * 0.5).max(0.0)
        };
        if self.db_hist.back().is_none_or(|u| now - u.0 >= 0.1) {
            self.db_hist.push_back((now, db_fast));
        }
        while self.db_hist.front().is_some_and(|p| now - p.0 > 3.0) {
            self.db_hist.pop_front();
        }
        let db_min = self.db_hist.iter().map(|p| p.1).fold(db_fast, f32::min);
        let salto = db_fast - db_min;
        let mag_grave = if self.low_bass_time > 2.0 && self.bass_fast > 0.75 * referencia {
            (0.4 + 0.3 * (self.low_bass_time - 2.0)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mag_volume = if db_min > -45.0 && salto > 6.0 { ((salto - 4.0) / 8.0).clamp(0.0, 1.0) } else { 0.0 };
        let mag = mag_grave.max(mag_volume) * (0.3 + 0.7 * smoothstep(0.35, 0.75, af));
        let na_janela = now - self.last_drop < 0.6;
        if mag > 0.2 && (na_janela || (now - self.last_drop > 8.0 && now > 6.0)) && self.gate > 0.5 {
            self.drop = self.drop.max(mag);
            if !na_janela {
                self.last_drop = now;
                self.low_bass_time = 0.0;
                log::debug!("drop {mag:.2} em {now:.1}s (grave {mag_grave:.2}, volume {mag_volume:.2})");
            }
        }
        self.drop *= (-dt / 0.9).exp();
        self.build = (smoothstep(0.04, 0.22, self.intensity - self.e_slow) * (1.0 - self.drop)).clamp(0.0, 1.0);

        // Fase aproximada pelo andamento estimado (ao vivo não se sabe onde cai a próxima batida).
        let bpm = if self.bpm > 0.0 { self.bpm as f64 } else { 120.0 };
        self.beat_phase = ((now * bpm / 60.0) % 1.0) as f32;
        self.beat_conf = pulse;
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn bandas(db: f32) -> Bandas {
        Bandas { rms_db: db, level: 0.3, bass: 0.3, flatness: 0.6, fullness: 0.5, ..Default::default() }
    }

    #[test]
    fn silencio_fecha_a_porta_e_baixa_a_energia() {
        let mut d = Diretor::default();
        for _ in 0..600 {
            d.atualizar(1.0 / 60.0, &bandas(-80.0), Pulso::default());
        }
        assert!(d.gate < 0.01);
        assert!(d.intensity < 0.05, "energia = {}", d.intensity);
        assert_eq!(d.rotulo, Rotulo::Calma);
    }

    #[test]
    fn som_alto_e_ritmado_e_mais_intenso_que_som_baixo() {
        let roda = |db: f32, groove: f32| {
            let mut d = Diretor::default();
            for _ in 0..1200 {
                d.atualizar(
                    1.0 / 60.0,
                    &bandas(db),
                    Pulso { groove, bpm: 128.0, densidade: 2.5, ..Default::default() },
                );
            }
            d.intensity
        };
        let (baixo, alto) = (roda(-34.0, 0.0), roda(-10.0, 0.8));
        assert!(alto > baixo + 0.25, "baixo = {baixo}, alto = {alto}");
        assert!((0.0..=1.0).contains(&alto));
    }

    #[test]
    fn ataque_gera_pulso_e_impulso() {
        let mut d = Diretor::default();
        for _ in 0..120 {
            d.atualizar(1.0 / 60.0, &bandas(-12.0), Pulso::default());
        }
        let antes = d.impulso_total;
        d.atualizar(1.0 / 60.0, &bandas(-12.0), Pulso { forca: 0.9, faixa: Faixa::Low, ..Default::default() });
        assert!(d.pulse > 0.1);
        assert!(d.impulso_total > antes);
    }

    #[test]
    fn cores_seguem_a_energia() {
        let (calma, _) = cores(0.1, 0.0);
        let (agitada, _) = cores(0.65, 0.0);
        let (intensa, _) = cores(1.0, 0.0);
        assert_eq!(calma, [1.0, 1.0, 1.0]);
        assert!(agitada[2] > agitada[0] * 3.0, "azul");
        assert!(intensa[0] > intensa[2] * 3.0, "vermelho");
        // O drop empurra a cor para o vermelho.
        assert!(cores(0.65, 1.0).0[0] > agitada[0]);
    }

    #[test]
    fn dirigidos_respeitam_as_faixas() {
        assert_eq!(Dirigidos::para(-3.0), Dirigidos::para(0.0));
        assert_eq!(Dirigidos::para(9.0), Dirigidos::para(1.0));
        assert!((Dirigidos::para(0.0).exposure - 0.8).abs() < 1e-6);
        assert!((Dirigidos::para(1.0).exposure - 2.3).abs() < 1e-6);
    }
}
