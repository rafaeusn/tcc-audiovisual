//! Análise de áudio ao vivo do Modo Escuta.
//!
//! Entra o som (amostras mono), sai um [`Retrato`]: os números que o visual e a interface leem a cada quadro.
//! Tudo aqui é puro (sem dispositivo de áudio, sem GPU), então roda igual em teste e no app.
//!
//! É um porte de `preview/features.js`, `rhythm.js` e da parte ao vivo de `director.js`.

mod analisador;
pub mod descritores;
pub mod diretor;
mod fft;
pub mod ritmo;

pub use descritores::{BINS, Bandas};
pub use diretor::{Dirigidos, Rotulo, cores};

use analisador::Analisador;
use descritores::Descritores;
use diretor::{Diretor, Pulso};
use ritmo::Ritmo;

/// Tamanho dos dois analisadores, os mesmos da prévia.
pub const FFT_PRINCIPAL: usize = 2048;
const FFT_ATAQUES: usize = 1024;
/// Passo fixo da análise: o diretor foi calibrado a 60 atualizações por segundo.
pub const PASSO: f32 = 1.0 / 60.0;

/// Tudo o que o resto do app precisa saber sobre o som neste instante. É pequeno e copiável de propósito:
/// a análise publica um retrato novo e a interface só lê, sem esperar ninguém.
#[derive(Clone, Copy, Debug)]
pub struct Retrato {
    pub bandas: Bandas,
    /// Espectro em 256 bandas logarítmicas (30 Hz–16 kHz), 0–255.
    pub espectro: [u8; BINS],
    /// Energia do trecho (0–1).
    pub energia: f32,
    pub loud: f32,
    pub atividade: f32,
    pub pulso: f32,
    /// Soma dos ataques desde o início. O visual usa a diferença entre dois quadros.
    pub impulso_total: f64,
    pub drop: f32,
    pub build: f32,
    /// 0 = silêncio, 1 = som.
    pub gate: f32,
    pub bpm: f32,
    pub fase_batida: f32,
    pub conf_batida: f32,
    pub rotulo: Rotulo,
}

impl Default for Retrato {
    fn default() -> Self {
        Self {
            bandas: Bandas::default(),
            espectro: [0; BINS],
            energia: 0.0,
            loud: 0.0,
            atividade: 0.0,
            pulso: 0.0,
            impulso_total: 0.0,
            drop: 0.0,
            build: 0.0,
            gate: 0.0,
            bpm: 0.0,
            fase_batida: 0.0,
            conf_batida: 0.0,
            rotulo: Rotulo::Calma,
        }
    }
}

impl Retrato {
    /// Andamento para mostrar ao usuário: a detecção pode travar na metade ou no dobro do tempo, então o valor
    /// é trazido para a faixa de 70 a 180 BPM (a mesma dobra que o diretor usa). Zero = sem pulso claro.
    pub fn bpm_exibido(&self) -> f32 {
        let mut b = self.bpm;
        if !b.is_finite() || b <= 0.0 || self.conf_batida < 0.15 {
            return 0.0;
        }
        while b < 70.0 {
            b *= 2.0;
        }
        while b > 180.0 {
            b /= 2.0;
        }
        b
    }
}

/// A cadeia inteira: espectro → descritores → ritmo → diretor.
pub struct Motor {
    principal: Analisador,
    ataques: Analisador,
    descritores: Descritores,
    ritmo: Ritmo,
    diretor: Diretor,
}

impl Motor {
    pub fn new(taxa_de_amostragem: f32) -> Self {
        Self {
            principal: Analisador::new(FFT_PRINCIPAL, 0.72, -85.0, -20.0),
            ataques: Analisador::new(FFT_ATAQUES, 0.0, -85.0, -20.0),
            descritores: Descritores::new(taxa_de_amostragem, FFT_PRINCIPAL, FFT_ATAQUES),
            ritmo: Ritmo::default(),
            diretor: Diretor::default(),
        }
    }

    /// Um passo da análise. `amostras` são as mais recentes (pelo menos [`FFT_PRINCIPAL`]); com menos que isso
    /// o passo é tratado como silêncio.
    pub fn passo(&mut self, dt: f32, amostras: &[f32]) -> Retrato {
        let dt = if dt.is_finite() { dt.clamp(1e-4, 0.05) } else { PASSO };
        let silencio = [0.0f32; FFT_PRINCIPAL];
        let amostras =
            if amostras.len() >= FFT_PRINCIPAL { &amostras[amostras.len() - FFT_PRINCIPAL..] } else { &silencio };

        self.descritores.sensibilidade = self.diretor.sens;
        let freq = self.principal.processar(amostras);
        let ataques = self.ataques.processar(amostras);
        self.descritores.atualizar(dt, freq, ataques, amostras);
        let b = self.descritores.bandas;
        let forca = self.ritmo.empurrar(dt, &b);
        let pulso = Pulso {
            forca,
            faixa: self.ritmo.last_band,
            groove: self.ritmo.groove,
            bpm: self.ritmo.bpm,
            densidade: self.ritmo.density,
        };
        self.diretor.atualizar(dt, &b, pulso);
        let d = &self.diretor;
        Retrato {
            bandas: b,
            espectro: self.descritores.espectro,
            energia: d.intensity,
            loud: d.loud,
            atividade: d.activity,
            pulso: d.pulse,
            impulso_total: d.impulso_total,
            drop: d.drop,
            build: d.build,
            gate: d.gate,
            bpm: d.bpm,
            fase_batida: d.beat_phase,
            conf_batida: d.beat_conf,
            rotulo: d.rotulo,
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Bumbo sintético (seno de 60 Hz com envelope) mais um chiado leve, a `bpm`.
    fn musica(taxa: f32, segundos: f32, bpm: f32, volume: f32) -> Vec<f32> {
        let n = (taxa * segundos) as usize;
        let periodo = 60.0 / bpm;
        let mut semente = 12345u32;
        (0..n)
            .map(|i| {
                let t = i as f32 / taxa;
                let fase = t % periodo;
                let bumbo = (-fase * 18.0).exp() * (std::f32::consts::TAU * 60.0 * t).sin();
                semente = semente.wrapping_mul(1664525).wrapping_add(1013904223);
                let chiado = (semente >> 8) as f32 / 8388608.0 - 1.0;
                volume * (0.8 * bumbo + 0.05 * chiado)
            })
            .collect()
    }

    fn rodar(sinal: &[f32], taxa: f32) -> Retrato {
        let mut m = Motor::new(taxa);
        let por_passo = (taxa * PASSO) as usize;
        let mut r = Retrato::default();
        let mut fim = FFT_PRINCIPAL;
        while fim <= sinal.len() {
            r = m.passo(PASSO, &sinal[fim - FFT_PRINCIPAL..fim]);
            fim += por_passo;
        }
        r
    }

    #[test]
    fn silencio_da_retrato_parado() {
        let r = rodar(&vec![0.0; 48000 * 5], 48000.0);
        assert!(r.gate < 0.01 && r.energia < 0.1 && r.pulso == 0.0);
        assert_eq!(r.rotulo, Rotulo::Calma);
    }

    #[test]
    fn bumbo_a_120_e_reconhecido() {
        let r = rodar(&musica(48000.0, 14.0, 120.0, 0.7), 48000.0);
        assert!(r.gate > 0.9, "gate = {}", r.gate);
        // A autocorrelação pode travar na metade do andamento; o valor para exibir é o dobrado.
        assert!((r.bpm_exibido() - 120.0).abs() < 5.0, "bpm = {}", r.bpm);
        assert!(r.impulso_total > 1.0, "impulso = {}", r.impulso_total);
        assert!(r.bandas.bass > r.bandas.high);
    }

    #[test]
    fn mais_volume_da_mais_energia() {
        let fraco = rodar(&musica(48000.0, 12.0, 120.0, 0.03), 48000.0);
        let forte = rodar(&musica(48000.0, 12.0, 120.0, 0.8), 48000.0);
        assert!(forte.energia > fraco.energia + 0.1, "fraco = {}, forte = {}", fraco.energia, forte.energia);
    }

    #[test]
    fn entrada_curta_ou_invalida_nao_quebra() {
        let mut m = Motor::new(44100.0);
        let r = m.passo(f32::NAN, &[0.5; 10]);
        assert!(r.energia.is_finite());
        let r = m.passo(PASSO, &vec![f32::INFINITY; FFT_PRINCIPAL]);
        assert!(r.energia.is_finite() && r.bandas.level.is_finite());
    }
}
