//! Ritmo agnóstico de fonte (porte de `preview/rhythm.js`).
//!
//! Detecta ataques em três faixas e mede quão periódico é o pulso. Cada faixa é comparada com a própria média
//! recente ("surpresa"), então não depende de volume nem de gênero. Sem BPM fixo: a periodicidade vem da
//! autocorrelação.

use crate::descritores::Bandas;
use std::collections::VecDeque;

const GRID_HZ: f32 = 50.0;
const GRID_N: usize = 8 * 50;
const LAG_MIN: usize = 13; // 0,25 s = 240 BPM (arredondado como no JS)
const LAG_MAX: usize = 55; // 1,1 s ≈ 55 BPM
const PEAK_RATIO: f32 = 2.4;
const REFRACTORY: f64 = 0.13;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Faixa {
    #[default]
    Low,
    Mid,
    High,
}

const FAIXAS: [(Faixa, f32); 3] = [(Faixa::Low, 1.0), (Faixa::Mid, 0.85), (Faixa::High, 0.6)];

fn smoothstep(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

pub struct Ritmo {
    t: f64,
    mean: [f32; 3],
    e1: f32,
    e2: f32,
    last_event: f64,
    events: VecDeque<(f64, f32)>,
    grid: [f32; GRID_N],
    grid_pos: usize,
    grid_t: f32,
    cur_cell: f32,
    ac_t: f32,
    b1: Faixa,
    pub groove: f32,
    pub bpm: f32,
    /// Ataques por segundo ponderados pela força (janela de 3 s).
    pub density: f32,
    pub coherence: f32,
    /// Faixa dominante do último ataque.
    pub last_band: Faixa,
}

impl Default for Ritmo {
    fn default() -> Self {
        Self {
            t: 0.0,
            mean: [0.004; 3],
            e1: 0.0,
            e2: 0.0,
            last_event: -10.0,
            events: VecDeque::new(),
            grid: [0.0; GRID_N],
            grid_pos: 0,
            grid_t: 0.0,
            cur_cell: 0.0,
            ac_t: 0.0,
            b1: Faixa::Low,
            groove: 0.0,
            bpm: 0.0,
            density: 0.0,
            coherence: 0.0,
            last_band: Faixa::Low,
        }
    }
}

impl Ritmo {
    /// Devolve a força (0–1) se um ataque acabou de ser confirmado, senão 0.
    pub fn empurrar(&mut self, dt: f32, b: &Bandas) -> f32 {
        self.t += dt as f64;
        let t = self.t;
        let k = 1.0 - (-dt / 3.0).exp();
        let (mut env, mut band) = (0.0f32, Faixa::Low);
        let fluxos = [b.flux_low, b.flux_mid, b.flux_high];
        for (i, (faixa, peso)) in FAIXAS.iter().enumerate() {
            let f = fluxos[i];
            let m = self.mean[i];
            let surpresa = (f / (m + 0.0015)) * peso;
            if surpresa > env {
                env = surpresa;
                band = *faixa;
            }
            // Linha de base robusta: os próprios picos entram limitados a 1,5x a média, senão um bumbo regular
            // vira "o normal" e deixa de ser detectado.
            self.mean[i] = m + (f.min(1.5 * m + 0.002) - m) * k;
        }
        if b.level < 0.03 {
            env = 0.0;
        }

        // Envelope em grade fixa (máximo por célula) para a autocorrelação.
        self.grid_t += dt;
        let cell = 1.0 / GRID_HZ;
        self.cur_cell = self.cur_cell.max(env.min(8.0));
        while self.grid_t >= cell {
            self.grid_t -= cell;
            self.grid[self.grid_pos] = self.cur_cell;
            self.grid_pos = (self.grid_pos + 1) % GRID_N;
            self.cur_cell = 0.0;
        }

        // Pico: o quadro anterior (e1) é máximo local, com um quadro de atraso (~16 ms).
        let mut forca = 0.0;
        if self.e1 >= PEAK_RATIO && self.e1 >= self.e2 && self.e1 > env && t - self.last_event > REFRACTORY {
            forca = ((self.e1 - 1.8) / 3.0).clamp(0.0, 1.0);
            self.last_event = t;
            forca *= self.peso_alinhamento(t);
            self.events.push_back((t, forca));
            self.last_band = self.b1;
        }
        self.e2 = self.e1;
        self.e1 = env;
        self.b1 = band;

        while self.events.front().is_some_and(|e| t - e.0 > 3.0) {
            self.events.pop_front();
        }
        let soma: f32 = self.events.iter().map(|e| 0.4 + 0.6 * e.1).sum();
        self.density = soma / (t as f32).clamp(1.0, 3.0);

        self.ac_t += dt;
        if self.ac_t >= 0.25 {
            self.ac_t = 0.0;
            self.autocorrelacao();
        }
        forca
    }

    /// Ataques que caem na grade de batidas valem mais; subdivisões e ruído valem menos.
    fn peso_alinhamento(&mut self, t: f64) -> f32 {
        let periodo = if self.bpm > 0.0 { 60.0 / self.bpm as f64 } else { 0.0 };
        if periodo <= 0.0 || self.groove < 0.2 {
            self.coherence *= 0.9;
            return 1.0;
        }
        let (mut cx, mut cy, mut soma) = (0.0f64, 0.0f64, 0.0f64);
        for (te, se) in &self.events {
            if t - te > 6.0 {
                continue;
            }
            let a = std::f64::consts::TAU * ((te / periodo) % 1.0);
            cx += *se as f64 * a.cos();
            cy += *se as f64 * a.sin();
            soma += *se as f64;
        }
        if soma < 0.5 {
            return 1.0;
        }
        let phi = cy.atan2(cx) / std::f64::consts::TAU;
        let c = (cx.hypot(cy) / soma) as f32;
        self.coherence += (c - self.coherence) * 0.3;
        let dist = |u: f64| ((((u % 1.0) + 1.5) % 1.0) - 0.5).abs() as f32;
        let ph = (t / periodo) % 1.0;
        let d_beat = dist(ph - phi);
        let d_half = dist(ph - phi - 0.5);
        let align = (1.0 - smoothstep(0.08, 0.22, d_beat)).max(0.5 * (1.0 - smoothstep(0.06, 0.16, d_half)));
        let trust = smoothstep(0.25, 0.6, self.coherence);
        0.35 + 0.65 * (1.0 - trust + trust * align)
    }

    fn autocorrelacao(&mut self) {
        let n = GRID_N.min((self.t * GRID_HZ as f64) as usize);
        if n < LAG_MAX * 2 {
            return;
        }
        // Série ordenada no tempo (do mais antigo ao mais novo), sem a média.
        let mut x = vec![0.0f32; n];
        let mut media = 0.0;
        for (i, v) in x.iter_mut().enumerate() {
            *v = self.grid[(self.grid_pos + GRID_N * 2 - n + i) % GRID_N];
            media += *v;
        }
        media /= n as f32;
        let mut r0 = 0.0;
        for v in x.iter_mut() {
            *v -= media;
            r0 += *v * *v;
        }
        if r0 < 1e-3 {
            self.groove += (0.0 - self.groove) * 0.3;
            return;
        }
        let mut r = [0.0f32; LAG_MAX + 2];
        for (lag, rv) in r.iter_mut().enumerate().skip(LAG_MIN - 1) {
            let s: f32 = (lag..n).map(|i| x[i] * x[i - lag]).sum();
            *rv = s / r0 * (n as f32 / (n - lag) as f32);
        }
        // Só máximos locais contam como candidatos a período.
        let (mut melhor, mut melhor_lag) = (0.0f32, 0usize);
        for lag in LAG_MIN..=LAG_MAX {
            if r[lag] > r[lag - 1] && r[lag] >= r[lag + 1] && r[lag] > melhor {
                melhor = r[lag];
                melhor_lag = lag;
            }
        }
        self.groove += (melhor.clamp(0.0, 1.0) - self.groove) * 0.35;
        if melhor_lag > 0 {
            let (a, b, c) = (r[melhor_lag - 1], r[melhor_lag], r[melhor_lag + 1]);
            let d = a - 2.0 * b + c;
            let off = if d != 0.0 { (0.5 * (a - c)) / d } else { 0.0 };
            let lag = melhor_lag as f32 + off.clamp(-0.5, 0.5);
            let bpm = 60.0 * GRID_HZ / lag;
            self.bpm = if self.bpm > 0.0 { self.bpm + (bpm - self.bpm) * 0.3 } else { bpm };
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Bumbo sintético: um pico de fluxo grave a cada `periodo` segundos.
    fn rodar(bpm: f32, segundos: f32) -> (Ritmo, u32) {
        let mut r = Ritmo::default();
        let dt = 1.0 / 60.0;
        let periodo = 60.0 / bpm;
        let (mut t, mut prox, mut ataques) = (0.0f32, 0.5f32, 0u32);
        while t < segundos {
            let mut b = Bandas { level: 0.3, flux_low: 0.002, flux_mid: 0.002, flux_high: 0.002, ..Default::default() };
            if t >= prox {
                b.flux_low = 0.12;
                prox += periodo;
            }
            if r.empurrar(dt, &b) > 0.0 {
                ataques += 1;
            }
            t += dt;
        }
        (r, ataques)
    }

    #[test]
    fn acha_o_andamento_de_um_bumbo_regular() {
        let (r, ataques) = rodar(120.0, 12.0);
        assert!((r.bpm - 120.0).abs() < 4.0, "bpm = {}", r.bpm);
        assert!(r.groove > 0.5, "groove = {}", r.groove);
        assert!(ataques >= 18, "ataques = {ataques}");
    }

    #[test]
    fn silencio_nao_gera_ataques() {
        let mut r = Ritmo::default();
        for _ in 0..600 {
            assert_eq!(r.empurrar(1.0 / 60.0, &Bandas::default()), 0.0);
        }
        assert_eq!(r.bpm, 0.0);
    }
}
