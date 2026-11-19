//! Sonoridade integrada (LUFS) pela recomendação ITU-R BS.1770, a mesma base do ReplayGain 2 e da EBU R128.
//!
//! O som passa por um filtro que imita o ouvido (ponderação K: realça um pouco os agudos e corta os graves muito
//! baixos), é medido em blocos de 400 ms que andam de 100 em 100 ms, e os blocos quietos demais ficam de fora (o
//! silêncio de começo e fim não puxa a média para baixo).

/// Abaixo disto, um bloco é silêncio e não entra na conta.
const PORTA_ABSOLUTA: f64 = -70.0;
/// Blocos mais de 10 LU abaixo da média também saem (os trechos baixinhos de uma música alta).
const PORTA_RELATIVA: f64 = -10.0;
/// O ReplayGain 2 leva todas as músicas a este nível.
pub const REFERENCIA_LUFS: f32 = -18.0;

#[derive(Clone, Copy, Default)]
struct Biquad {
    b: [f64; 3],
    a: [f64; 2],
    x: [f64; 2],
    y: [f64; 2],
}

impl Biquad {
    fn passar(&mut self, x: f64) -> f64 {
        let y = self.b[0] * x + self.b[1] * self.x[0] + self.b[2] * self.x[1]
            - self.a[0] * self.y[0]
            - self.a[1] * self.y[1];
        self.x = [x, self.x[0]];
        self.y = [y, self.y[0]];
        y
    }
}

/// Os dois filtros da ponderação K para a taxa `taxa` (fórmulas da libebur128, que valem para qualquer taxa).
fn ponderacao_k(taxa: f64) -> [Biquad; 2] {
    use std::f64::consts::PI;
    // Prateleira alta de +4 dB por volta de 1,7 kHz.
    let (f0, g, q) = (1681.974450955533, 3.999843853973347, 0.7071752369554196);
    let k = (PI * f0 / taxa).tan();
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.4996667741545416);
    let a0 = 1.0 + k / q + k * k;
    let prateleira = Biquad {
        b: [(vh + vb * k / q + k * k) / a0, 2.0 * (k * k - vh) / a0, (vh - vb * k / q + k * k) / a0],
        a: [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0],
        ..Default::default()
    };
    // Passa-altas em 38 Hz.
    let (f0, q) = (38.13547087602444, 0.5003270373238773);
    let k = (PI * f0 / taxa).tan();
    let a0 = 1.0 + k / q + k * k;
    let passa_altas =
        Biquad { b: [1.0, -2.0, 1.0], a: [2.0 * (k * k - 1.0) / a0, (1.0 - k / q + k * k) / a0], ..Default::default() };
    [prateleira, passa_altas]
}

/// A ponderação K de um canal, amostra a amostra (para a sonoridade de curto prazo da pré-análise).
pub struct Filtro {
    f: [Biquad; 2],
}

impl Filtro {
    pub fn novo(taxa: u32) -> Self {
        Self { f: ponderacao_k(f64::from(taxa)) }
    }

    pub fn passar(&mut self, x: f64) -> f64 {
        let y = self.f[0].passar(x);
        self.f[1].passar(y)
    }
}

/// Mede a sonoridade aos poucos: o som entra em pedaços, a resposta sai no fim.
pub struct Medidor {
    filtros: [[Biquad; 2]; 2],
    /// Amostras por sub-bloco de 100 ms.
    por_sub: usize,
    soma: f64,
    contadas: usize,
    /// A energia média de cada sub-bloco de 100 ms.
    subs: Vec<f64>,
    pico: f32,
}

impl Medidor {
    pub fn novo(taxa: u32) -> Self {
        let k = ponderacao_k(f64::from(taxa));
        Self {
            filtros: [k, k],
            por_sub: (taxa as usize / 10).max(1),
            soma: 0.0,
            contadas: 0,
            subs: Vec::new(),
            pico: 0.0,
        }
    }

    /// Mais som estéreo intercalado (esquerda, direita).
    pub fn empurrar(&mut self, estereo: &[f32]) {
        for quadro in estereo.as_chunks::<2>().0 {
            let mut energia = 0.0;
            for (canal, v) in quadro.iter().enumerate() {
                self.pico = self.pico.max(v.abs());
                let mut s = f64::from(*v);
                for f in &mut self.filtros[canal] {
                    s = f.passar(s);
                }
                energia += s * s;
            }
            self.soma += energia;
            self.contadas += 1;
            if self.contadas == self.por_sub {
                self.subs.push(self.soma / self.por_sub as f64);
                self.soma = 0.0;
                self.contadas = 0;
            }
        }
    }

    /// O maior valor absoluto de amostra visto (1 = o máximo sem distorcer).
    pub fn pico(&self) -> f32 {
        self.pico
    }

    /// A sonoridade integrada em LUFS. `None` quando quase tudo foi silêncio.
    pub fn lufs(&self) -> Option<f32> {
        let sonoridade = |z: f64| -0.691 + 10.0 * z.log10();
        // Blocos de 400 ms (quatro sub-blocos) que andam de 100 em 100 ms.
        let blocos: Vec<f64> = self.subs.windows(4).map(|w| w.iter().sum::<f64>() / 4.0).collect();
        let acima: Vec<f64> = blocos.iter().copied().filter(|z| *z > 0.0 && sonoridade(*z) > PORTA_ABSOLUTA).collect();
        if acima.is_empty() {
            return None;
        }
        let media = acima.iter().sum::<f64>() / acima.len() as f64;
        let porta = sonoridade(media) + PORTA_RELATIVA;
        let ficam: Vec<f64> = acima.into_iter().filter(|z| sonoridade(*z) > porta).collect();
        let media = ficam.iter().sum::<f64>() / ficam.len().max(1) as f64;
        let l = sonoridade(media);
        l.is_finite().then_some(l as f32)
    }
}

/// O ganho de ReplayGain (dB) para levar uma música de `lufs` ao nível de referência.
pub fn ganho_db(lufs: f32) -> f32 {
    (REFERENCIA_LUFS - lufs).clamp(-24.0, 24.0)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn seno(taxa: u32, hz: f32, amplitude: f32, segundos: f32) -> Vec<f32> {
        (0..(taxa as f32 * segundos) as usize)
            .flat_map(|i| {
                let v = (i as f32 * hz * std::f32::consts::TAU / taxa as f32).sin() * amplitude;
                [v, v]
            })
            .collect()
    }

    fn medir(taxa: u32, som: &[f32]) -> Option<f32> {
        let mut m = Medidor::novo(taxa);
        // Em pedaços desiguais, como chega do decodificador.
        for pedaco in som.chunks(3001 * 2) {
            m.empurrar(pedaco);
        }
        m.lufs()
    }

    #[test]
    fn seno_de_1khz_da_o_valor_da_norma() {
        // Seno de 1 kHz com pico em -20 dBFS nos dois canais: perto de -20,0 LUFS (a ponderação K em 1 kHz dá
        // +0,7 dB, a soma dos dois canais +3 dB e o RMS do seno -3 dB).
        let a = 10f32.powf(-20.0 / 20.0);
        let l = medir(48_000, &seno(48_000, 1000.0, a, 5.0)).expect("mede");
        assert!((l - (-20.0)).abs() < 0.5, "deu {l}");
        // Mesma coisa em 44,1 kHz.
        let l44 = medir(44_100, &seno(44_100, 1000.0, a, 5.0)).expect("mede");
        assert!((l44 - l).abs() < 0.2, "44,1 kHz deu {l44}");
    }

    #[test]
    fn metade_da_amplitude_e_6_db_a_menos_e_silencio_nao_mede() {
        let alto = medir(44_100, &seno(44_100, 440.0, 0.5, 4.0)).expect("mede");
        let baixo = medir(44_100, &seno(44_100, 440.0, 0.25, 4.0)).expect("mede");
        assert!((alto - baixo - 6.02).abs() < 0.2, "{alto} e {baixo}");
        assert!(medir(44_100, &vec![0.0; 44_100 * 4]).is_none());
        // Silêncio antes e depois não muda a medida (a porta tira).
        let mut com_silencio = vec![0.0; 44_100 * 2 * 3];
        com_silencio.extend(seno(44_100, 440.0, 0.5, 4.0));
        com_silencio.extend(vec![0.0; 44_100 * 2 * 3]);
        let l = medir(44_100, &com_silencio).expect("mede");
        // Os blocos na fronteira entre silêncio e som baixam um pouco a média (a norma também).
        assert!((l - alto).abs() < 0.5, "{l} e {alto}");
    }

    #[test]
    fn ganho_leva_a_referencia() {
        assert_eq!(ganho_db(-8.0), -10.0);
        assert_eq!(ganho_db(-18.0), 0.0);
        assert_eq!(ganho_db(-80.0), 24.0, "limitado");
    }
}
