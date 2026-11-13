//! Equivalente ao `AnalyserNode` do Web Audio, que a prévia usa.
//!
//! O diretor foi calibrado em cima dos bytes que o `AnalyserNode` devolve. Para a calibração continuar valendo no
//! app, este módulo reproduz a mesma conta: janela de Blackman, FFT, módulo dividido pelo tamanho, suavização no
//! tempo, conversão para dB e escala de `min_db..max_db` para 0–255.

use crate::fft::Fft;

pub struct Analisador {
    fft: Fft,
    janela: Vec<f32>,
    re: Vec<f32>,
    im: Vec<f32>,
    suave: Vec<f32>,
    bytes: Vec<u8>,
    suavizacao: f32,
    min_db: f32,
    max_db: f32,
}

impl Analisador {
    /// `suavizacao` é o `smoothingTimeConstant` (0 = sem memória).
    pub fn new(tamanho: usize, suavizacao: f32, min_db: f32, max_db: f32) -> Self {
        let fft = Fft::new(tamanho);
        let n = fft.tamanho();
        let janela = (0..n)
            .map(|i| {
                let x = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
                (0.42 - 0.5 * x.cos() + 0.08 * (2.0 * x).cos()) as f32
            })
            .collect();
        Self {
            fft,
            janela,
            re: vec![0.0; n],
            im: vec![0.0; n],
            suave: vec![0.0; n / 2],
            bytes: vec![0; n / 2],
            suavizacao: suavizacao.clamp(0.0, 0.999),
            min_db,
            max_db: max_db.max(min_db + 1.0),
        }
    }

    /// Recebe as últimas `tamanho` amostras (mono, -1..1) e devolve os bytes de frequência (metade do tamanho).
    pub fn processar(&mut self, amostras: &[f32]) -> &[u8] {
        let n = self.fft.tamanho();
        if amostras.len() < n {
            return &self.bytes;
        }
        let trecho = &amostras[amostras.len() - n..];
        // O índice também é a posição na janela e no buffer de saída.
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            // Amostra inválida (NaN/infinito de um driver com defeito) vira silêncio.
            let v = if trecho[i].is_finite() { trecho[i] } else { 0.0 };
            self.re[i] = v * self.janela[i];
            self.im[i] = 0.0;
        }
        self.fft.transformar(&mut self.re, &mut self.im);
        let escala = 255.0 / (self.max_db - self.min_db);
        let tau = self.suavizacao;
        for k in 0..n / 2 {
            let mag = (self.re[k] * self.re[k] + self.im[k] * self.im[k]).sqrt() / n as f32;
            let s = tau * self.suave[k] + (1.0 - tau) * mag;
            self.suave[k] = if s.is_finite() { s } else { 0.0 };
            let db = if s > 0.0 { 20.0 * s.log10() } else { -1000.0 };
            self.bytes[k] = (escala * (db - self.min_db)).clamp(0.0, 255.0) as u8;
        }
        &self.bytes
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn silencio_da_zero() {
        let mut a = Analisador::new(1024, 0.0, -85.0, -20.0);
        assert!(a.processar(&vec![0.0; 1024]).iter().all(|b| *b == 0));
    }

    #[test]
    fn seno_forte_satura_o_bin() {
        let n = 2048;
        let mut a = Analisador::new(n, 0.0, -85.0, -20.0);
        let x: Vec<f32> =
            (0..n).map(|i| 0.8 * (2.0 * std::f32::consts::PI * 100.0 * i as f32 / n as f32).sin()).collect();
        let b = a.processar(&x);
        assert_eq!(b[100], 255);
        assert_eq!(b[600], 0);
    }

    #[test]
    fn entrada_curta_ou_invalida_nao_quebra() {
        let mut a = Analisador::new(1024, 0.5, -85.0, -20.0);
        assert_eq!(a.processar(&[0.0; 10]).len(), 512);
        let x = vec![f32::NAN; 1024];
        assert!(a.processar(&x).iter().all(|b| *b == 0));
    }
}
