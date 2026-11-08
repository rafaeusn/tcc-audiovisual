//! Som do sistema (loopback WASAPI da saída padrão) reduzido a 4 números: grave, médio, agudo e volume.
//!
//! É o embrião do Modo Escuta: tudo o que toca no PC (Spotify, navegador, player) chega aqui. Para os protótipos
//! de UI basta uma análise barata: dois filtros passa-baixa de um polo separam as faixas, sem FFT. Sem dispositivo
//! de saída (ou com `--synthetic`), um sinal sintético determinístico mantém o visual andando, o que também deixa
//! as medições comparáveis entre os dois protótipos.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use crate::visual::Params;

#[derive(Default)]
struct Shared {
    bass: AtomicU32,
    mid: AtomicU32,
    high: AtomicU32,
    level: AtomicU32,
}

fn store(a: &AtomicU32, v: f32) {
    a.store(v.to_bits(), Ordering::Relaxed);
}
fn load(a: &AtomicU32) -> f32 {
    f32::from_bits(a.load(Ordering::Relaxed))
}

pub struct Listener {
    _stream: Option<cpal::Stream>,
    shared: Arc<Shared>,
    pub source: String,
}

impl Listener {
    pub fn start(synthetic: bool) -> Self {
        let shared = Arc::new(Shared::default());
        if synthetic {
            return Self { _stream: None, shared, source: "sintético".into() };
        }
        match open(shared.clone()) {
            Ok((stream, name)) => Self { _stream: Some(stream), shared, source: format!("loopback: {name}") },
            Err(e) => {
                eprintln!("áudio do sistema indisponível ({e}); usando sinal sintético");
                Self { _stream: None, shared, source: "sintético".into() }
            }
        }
    }

    pub fn is_synthetic(&self) -> bool {
        self._stream.is_none()
    }

    /// (grave, médio, agudo, volume) em RMS bruto.
    pub fn raw(&self) -> (f32, f32, f32, f32) {
        let s = &self.shared;
        (load(&s.bass), load(&s.mid), load(&s.high), load(&s.level))
    }
}

fn open(shared: Arc<Shared>) -> Result<(cpal::Stream, String), String> {
    let host = cpal::default_host();
    let device = host.default_output_device().ok_or("sem saída de áudio")?;
    let name = device.description().map(|d| d.to_string()).unwrap_or_else(|_| "saída padrão".into());
    let supported = device.default_output_config().map_err(|e| e.to_string())?;
    let config = supported.config();
    let channels = config.channels.max(1) as usize;
    let sr = config.sample_rate as f32;
    // Coeficientes de um polo: corte em 150 Hz e 2 kHz.
    let k1 = 1.0 - (-2.0 * std::f32::consts::PI * 150.0 / sr).exp();
    let k2 = 1.0 - (-2.0 * std::f32::consts::PI * 2000.0 / sr).exp();
    let mut st = [0.0f32; 2];
    let mut process = move |mono: &mut dyn Iterator<Item = f32>| {
        let (mut sb, mut sm, mut sh, mut sa, mut n) = (0.0f32, 0.0f32, 0.0f32, 0.0f32, 0u32);
        for x in mono {
            st[0] += (x - st[0]) * k1;
            st[1] += (x - st[1]) * k2;
            let (b, m, h) = (st[0], st[1] - st[0], x - st[1]);
            sb += b * b;
            sm += m * m;
            sh += h * h;
            sa += x * x;
            n += 1;
        }
        if n > 0 {
            let n = n as f32;
            store(&shared.bass, (sb / n).sqrt());
            store(&shared.mid, (sm / n).sqrt());
            store(&shared.high, (sh / n).sqrt());
            store(&shared.level, (sa / n).sqrt());
        }
    };
    let err = |e| eprintln!("erro no stream de áudio: {e}");
    // No WASAPI, abrir um stream de *entrada* num dispositivo de *saída* liga o modo loopback.
    let stream = match supported.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream::<f32, _, _>(
            config,
            move |data: &[f32], _| {
                let mut it = data.chunks(channels).map(|f| f.iter().sum::<f32>() / channels as f32);
                process(&mut it);
            },
            err,
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream::<i16, _, _>(
            config,
            move |data: &[i16], _| {
                let mut it = data
                    .chunks(channels)
                    .map(|f| f.iter().map(|&s| s as f32 / 32768.0).sum::<f32>() / channels as f32);
                process(&mut it);
            },
            err,
            None,
        ),
        other => return Err(format!("formato de amostra não suportado: {other:?}")),
    }
    .map_err(|e| e.to_string())?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, name))
}

/// Suavização, batida e intensidade a partir dos números crus. Roda na thread da UI, uma vez por quadro.
#[derive(Default)]
pub struct Analyzer {
    t: f32,
    bass: f32,
    mid: f32,
    high: f32,
    level: f32,
    slow: f32,
    peak: f32,
    bass_avg: f32,
    pulse: f32,
    last_beat: f32,
    phase: f32,
    intensity: f32,
}

impl Analyzer {
    pub fn update(&mut self, dt: f32, listener: &Listener) -> Params {
        self.t += dt;
        let (b, m, h, l) = if listener.is_synthetic() { synthetic(self.t) } else { listener.raw() };
        // Ganho automático: o pico recente vira a referência (funciona em qualquer volume do sistema).
        self.peak = (self.peak * (-dt / 8.0).exp()).max(l).max(1e-4);
        let g = 0.5 / self.peak;
        let ema = |cur: f32, x: f32, tau: f32| cur + (x - cur) * (1.0 - (-dt / tau).exp());
        let (b, m, h, l) = ((b * g * 1.6).min(1.5), (m * g * 2.2).min(1.5), (h * g * 4.0).min(1.5), (l * g).min(1.5));
        self.bass = ema(self.bass, b, if b > self.bass { 0.03 } else { 0.12 });
        self.mid = ema(self.mid, m, 0.08);
        self.high = ema(self.high, h, 0.06);
        self.level = ema(self.level, l, 0.1);
        self.slow = ema(self.slow, l, 2.5);
        // Batida: grave bem acima da própria média recente.
        self.bass_avg = ema(self.bass_avg, b, 0.4);
        if b > self.bass_avg * 1.35 + 0.05 && self.t - self.last_beat > 0.25 {
            self.pulse = 1.0;
            self.last_beat = self.t;
        }
        self.pulse *= (-dt / 0.15).exp();
        self.intensity = ema(self.intensity, (self.slow * 1.2).clamp(0.0, 1.0), 1.0);
        self.phase += dt * (0.25 + 0.9 * self.level) + 0.08 * self.pulse;
        Params {
            time: self.t,
            bass: self.bass.min(1.0),
            mid: self.mid.min(1.0),
            high: self.high.min(1.0),
            level: self.level.min(1.0),
            intensity: self.intensity,
            pulse: self.pulse,
            phase: self.phase,
            ..Default::default()
        }
    }
}

/// 120 BPM com bumbo, e uma "seção" que cresce e diminui a cada 16 s.
fn synthetic(t: f32) -> (f32, f32, f32, f32) {
    let beat = (t * 2.0).fract();
    let kick = (-beat * 9.0).exp();
    let section = 0.5 - 0.5 * (t * std::f32::consts::TAU / 16.0).cos();
    let b = 0.05 + 0.25 * kick * (0.3 + 0.7 * section);
    let m = 0.04 + 0.05 * section + 0.02 * (t * 3.1).sin().abs();
    let h = 0.01 + 0.03 * section * ((t * 8.0).fract() < 0.1) as u8 as f32;
    let l = 0.05 + 0.2 * section + 0.1 * kick;
    (b, m, h, l)
}
