//! Descritores do áudio a partir dos bytes de frequência (porte de `preview/features.js`).
//!
//! Módulo puro: recebe números, devolve números. Os mesmos valores e constantes da prévia, para que a calibração
//! do diretor continue valendo.

pub const BINS: usize = 256;
const F_MIN: f32 = 30.0;
const F_MAX: f32 = 16000.0;

#[derive(Clone, Copy, Debug)]
pub struct Bandas {
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
    pub level: f32,
    pub beat: f32,
    pub flux: f32,
    pub onset: f32,
    pub centroid: f32,
    pub flatness: f32,
    /// Volume RMS em dBFS (curto prazo, ~0,4 s). Não depende da sensibilidade.
    pub rms_db: f32,
    /// Pico/RMS relativo: baixo = som comprimido e denso.
    pub crest: f32,
    /// Fração do espectro com energia (0–1).
    pub fullness: f32,
    /// Fluxo espectral bruto por quadro: < 250 Hz, 250 Hz–2,5 kHz, 2,5–10 kHz.
    pub flux_low: f32,
    pub flux_mid: f32,
    pub flux_high: f32,
}

impl Default for Bandas {
    fn default() -> Self {
        Self {
            bass: 0.0,
            mid: 0.0,
            high: 0.0,
            level: 0.0,
            beat: 0.0,
            flux: 0.0,
            onset: 0.0,
            centroid: 0.4,
            flatness: 0.5,
            rms_db: -60.0,
            crest: 0.5,
            fullness: 0.0,
            flux_low: 0.0,
            flux_mid: 0.0,
            flux_high: 0.0,
        }
    }
}

pub struct Descritores {
    pub sensibilidade: f32,
    pub espectro: [u8; BINS],
    pub bandas: Bandas,
    t: f64,
    flux_avg: Option<f32>,
    low_flux_avg: Option<f32>,
    last_beat: f64,
    onset_prev: Vec<u8>,
    onset_bin_hz: f32,
    /// Por banda logarítmica: (primeiro bin, último bin exclusivo, frequência central).
    mapa: Vec<(usize, usize, f32)>,
}

impl Descritores {
    pub fn new(taxa: f32, fft_principal: usize, fft_ataques: usize) -> Self {
        let taxa = if taxa.is_finite() && taxa > 0.0 { taxa } else { 48000.0 };
        let bin_hz = taxa / fft_principal as f32;
        let max_bin = (fft_principal / 2).saturating_sub(1);
        let mapa = (0..BINS)
            .map(|i| {
                let f0 = F_MIN * (F_MAX / F_MIN).powf(i as f32 / BINS as f32);
                let f1 = F_MIN * (F_MAX / F_MIN).powf((i + 1) as f32 / BINS as f32);
                let b0 = max_bin.min((f0 / bin_hz).floor() as usize);
                let b1 = max_bin.min((b0 + 1).max((f1 / bin_hz).ceil() as usize));
                (b0, b1, (f0 * f1).sqrt())
            })
            .collect();
        Self {
            sensibilidade: 1.0,
            espectro: [0; BINS],
            bandas: Bandas::default(),
            t: 0.0,
            flux_avg: None,
            low_flux_avg: None,
            last_beat: -10.0,
            onset_prev: vec![0; fft_ataques / 2],
            onset_bin_hz: taxa / fft_ataques as f32,
            mapa,
        }
    }

    /// `freq` e `ataques` são bytes de frequência; `tempo` são as amostras (-1..1) do mesmo bloco.
    pub fn atualizar(&mut self, dt: f32, freq: &[u8], ataques: &[u8], tempo: &[f32]) {
        self.t += dt as f64;
        let b = &mut self.bandas;
        let pico_de =
            |b0: usize, b1: usize| freq.get(b0..b1.min(freq.len())).and_then(|s| s.iter().max()).copied().unwrap_or(0);

        // Volume em dBFS e crest factor.
        if !tempo.is_empty() {
            let (mut ss, mut pk) = (0.0f32, 0.0f32);
            for v in tempo {
                let v = if v.is_finite() { *v } else { 0.0 };
                ss += v * v;
                pk = pk.max(v.abs());
            }
            let rms = (ss / tempo.len() as f32).sqrt();
            let db = 20.0 * rms.max(1e-5).log10();
            let kr = 1.0 - (-dt / 0.4).exp();
            b.rms_db += (db - b.rms_db) * kr;
            if rms > 0.005 {
                b.crest += (((pk / rms - 1.0) / 5.0).min(1.0) - b.crest) * kr;
            }
        }

        // Densidade espectral e espectro em bandas logarítmicas (uma passada só).
        let (mut cheias, mut bass, mut mid, mut high, mut all) = (0u32, 0.0f32, 0.0f32, 0.0f32, 0.0f32);
        let (mut nb, mut nm, mut nh) = (0u32, 0u32, 0u32);
        for (i, (b0, b1, hz)) in self.mapa.iter().enumerate() {
            let m = pico_de(*b0, *b1);
            if m > 128 {
                cheias += 1;
            }
            let tilt = 0.85 + 0.55 * (i as f32 / BINS as f32);
            let v = ((m as f32 / 255.0).powf(1.5) * tilt * self.sensibilidade).min(1.0);
            self.espectro[i] = (v * 255.0) as u8;
            all += v;
            if *hz < 150.0 {
                bass += v;
                nb += 1;
            } else if *hz < 2000.0 {
                mid += v;
                nm += 1;
            } else if *hz < 12000.0 {
                high += v;
                nh += 1;
            }
        }
        b.fullness += (cheias as f32 / BINS as f32 - b.fullness) * (1.0 - (-dt / 0.4).exp());

        let seguir = |atual: &mut f32, alvo: f32| {
            let taxa = if alvo > *atual { 30.0 } else { 7.0 };
            *atual += (alvo - *atual) * (1.0 - (-dt * taxa).exp());
        };
        seguir(&mut b.bass, bass / nb.max(1) as f32);
        seguir(&mut b.mid, mid / nm.max(1) as f32);
        seguir(&mut b.high, high / nh.max(1) as f32);
        seguir(&mut b.level, all / BINS as f32);

        // Brilho (centróide) e tonalidade (flatness): só contam acima de um piso de energia.
        let (mut wsum, mut wpos, mut lsum) = (0.0f32, 0.0f32, 0.0f32);
        for (i, e) in self.espectro.iter().enumerate() {
            let v = *e as f32 / 255.0 + 1e-3;
            wsum += v;
            wpos += v * (i as f32 / (BINS - 1) as f32);
            lsum += v.ln();
        }
        if wsum / BINS as f32 > 0.02 {
            let kd = 1.0 - (-dt * 3.0).exp();
            let flat = (lsum / BINS as f32).exp() / (wsum / BINS as f32);
            b.centroid += (wpos / wsum - b.centroid) * kd;
            b.flatness += (flat.min(1.0) - b.flatness) * kd;
        }

        // Fluxo espectral (aumentos de energia entre blocos) em três faixas.
        let n = ataques.len().min(self.onset_prev.len());
        let (mut mudou, mut fl, mut fm, mut fh) = (0u32, 0.0f32, 0.0f32, 0.0f32);
        let (mut n_low, mut n_mid, mut n_high) = (0u32, 0u32, 0u32);
        // O índice é o bin: dele sai a frequência.
        #[allow(clippy::needless_range_loop)]
        for k in 1..n {
            let hz = k as f32 * self.onset_bin_hz;
            if hz >= 10000.0 {
                break;
            }
            let d = ataques[k] as i32 - self.onset_prev[k] as i32;
            if d != 0 {
                mudou += 1;
            }
            self.onset_prev[k] = ataques[k];
            let faixa = if hz < 250.0 {
                0
            } else if hz < 2500.0 {
                1
            } else {
                2
            };
            match faixa {
                0 => n_low += 1,
                1 => n_mid += 1,
                _ => n_high += 1,
            }
            if d > 0 {
                match faixa {
                    0 => fl += d as f32,
                    1 => fm += d as f32,
                    _ => fh += d as f32,
                }
            }
        }
        // Blocos repetidos (nenhuma mudança nos bytes) não contam: seguram o último valor.
        if mudou > 0 {
            b.flux_low = fl / (n_low.max(1) as f32 * 255.0);
            b.flux_mid = fm / (n_mid.max(1) as f32 * 255.0);
            b.flux_high = fh / (n_high.max(1) as f32 * 255.0);
        }
        let low = b.flux_low;
        let flux = 0.6 * low + 0.4 * (fl + fm + fh) / ((n_low + n_mid + n_high).max(1) as f32 * 255.0);
        let k = 1.0 - (-dt * 2.5).exp();
        let fa = self.flux_avg.unwrap_or(flux);
        let fa = fa + (flux - fa) * k;
        self.flux_avg = Some(fa);
        let la = self.low_flux_avg.unwrap_or(low);
        let la = la + (low - la) * k;
        self.low_flux_avg = Some(la);
        b.flux = (flux / (fa * 3.0 + 1e-4)).min(1.0);
        b.onset += (flux - b.onset) * (1.0 - (-dt / 0.15).exp());

        // Batida simples (grave): usada pelos visuais Fluxo e Barras.
        if low > la * 1.8 + 0.01 && self.t - self.last_beat > 0.28 {
            b.beat = 1.0;
            self.last_beat = self.t;
        }
        b.beat *= (-dt * 5.0).exp();
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn silencio_fica_em_zero() {
        let mut d = Descritores::new(48000.0, 2048, 1024);
        for _ in 0..120 {
            d.atualizar(1.0 / 60.0, &[0; 1024], &[0; 512], &[0.0; 2048]);
        }
        assert!(d.bandas.level < 1e-4 && d.bandas.bass < 1e-4);
        assert!(d.bandas.rms_db < -90.0);
        assert!(d.espectro.iter().all(|v| *v == 0));
    }

    #[test]
    fn grave_forte_sobe_o_grave() {
        let mut d = Descritores::new(48000.0, 2048, 1024);
        let mut freq = [0u8; 1024];
        // 48000/2048 = 23,4 Hz por bin: bins 2..5 ficam entre 47 e 117 Hz.
        freq[2..6].fill(230);
        for _ in 0..60 {
            d.atualizar(1.0 / 60.0, &freq, &[0; 512], &[0.0; 2048]);
        }
        assert!(d.bandas.bass > 0.3, "grave = {}", d.bandas.bass);
        assert!(d.bandas.high < 0.01);
    }

    #[test]
    fn ataque_no_grave_marca_batida() {
        let mut d = Descritores::new(48000.0, 2048, 1024);
        let quieto = [10u8; 512];
        for _ in 0..60 {
            d.atualizar(1.0 / 60.0, &[0; 1024], &quieto, &[0.0; 2048]);
        }
        let mut forte = quieto;
        forte[1..5].fill(220);
        d.atualizar(1.0 / 60.0, &[0; 1024], &forte, &[0.0; 2048]);
        assert!(d.bandas.beat > 0.8);
    }

    #[test]
    fn entradas_curtas_nao_quebram() {
        let mut d = Descritores::new(0.0, 2048, 1024);
        d.atualizar(1.0 / 60.0, &[], &[], &[]);
        assert!(d.bandas.level.is_finite());
    }
}
