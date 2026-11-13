//! FFT radix-2 (tamanhos potência de dois), com a tabela de senos calculada uma vez.
//!
//! Os tamanhos usados (1024 e 2048, 60 vezes por segundo) são pequenos: uma FFT própria evita uma dependência
//! e custa microssegundos.

pub struct Fft {
    n: usize,
    cos: Vec<f32>,
    sin: Vec<f32>,
    rev: Vec<u32>,
}

impl Fft {
    /// `n` precisa ser potência de dois (≥ 2). Outros valores são arredondados para cima.
    pub fn new(n: usize) -> Self {
        let n = n.max(2).next_power_of_two();
        let bits = n.trailing_zeros();
        let (mut cos, mut sin) = (Vec::with_capacity(n / 2), Vec::with_capacity(n / 2));
        for k in 0..n / 2 {
            let a = -2.0 * std::f64::consts::PI * k as f64 / n as f64;
            cos.push(a.cos() as f32);
            sin.push(a.sin() as f32);
        }
        let rev = (0..n as u32).map(|i| i.reverse_bits() >> (32 - bits)).collect();
        Self { n, cos, sin, rev }
    }

    pub fn tamanho(&self) -> usize {
        self.n
    }

    /// Transforma no lugar. `re` e `im` precisam ter `n` elementos.
    pub fn transformar(&self, re: &mut [f32], im: &mut [f32]) {
        let n = self.n;
        debug_assert!(re.len() == n && im.len() == n);
        if re.len() != n || im.len() != n {
            return;
        }
        for i in 0..n {
            let j = self.rev[i] as usize;
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut tam = 2;
        while tam <= n {
            let meio = tam / 2;
            let passo = n / tam;
            let mut ini = 0;
            while ini < n {
                for k in 0..meio {
                    let (wr, wi) = (self.cos[k * passo], self.sin[k * passo]);
                    let (a, b) = (ini + k, ini + k + meio);
                    let tr = re[b] * wr - im[b] * wi;
                    let ti = re[b] * wi + im[b] * wr;
                    re[b] = re[a] - tr;
                    im[b] = im[a] - ti;
                    re[a] += tr;
                    im[a] += ti;
                }
                ini += tam;
            }
            tam *= 2;
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn bate_com_a_dft_direta() {
        let n = 64;
        let x: Vec<f32> = (0..n).map(|i| ((i * 7 % 13) as f32 - 6.0) / 6.0 + (i as f32 * 0.3).sin()).collect();
        let (mut re, mut im) = (x.clone(), vec![0.0; n]);
        Fft::new(n).transformar(&mut re, &mut im);
        for k in 0..n {
            let (mut sr, mut si) = (0.0f64, 0.0f64);
            for (i, v) in x.iter().enumerate() {
                let a = -2.0 * std::f64::consts::PI * (k * i) as f64 / n as f64;
                sr += *v as f64 * a.cos();
                si += *v as f64 * a.sin();
            }
            assert!((re[k] as f64 - sr).abs() < 1e-3, "re[{k}]");
            assert!((im[k] as f64 - si).abs() < 1e-3, "im[{k}]");
        }
    }

    #[test]
    fn seno_puro_cai_no_bin_certo() {
        let n = 1024;
        let bin = 37;
        let mut re: Vec<f32> =
            (0..n).map(|i| (2.0 * std::f32::consts::PI * bin as f32 * i as f32 / n as f32).sin()).collect();
        let mut im = vec![0.0; n];
        Fft::new(n).transformar(&mut re, &mut im);
        let mag = |k: usize| (re[k] * re[k] + im[k] * im[k]).sqrt();
        let pico = (1..n / 2).max_by(|a, b| mag(*a).total_cmp(&mag(*b))).unwrap_or(0);
        assert_eq!(pico, bin);
        assert!((mag(bin) - n as f32 / 2.0).abs() < 0.5);
    }
}
