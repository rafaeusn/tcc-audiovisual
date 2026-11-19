//! A análise da música inteira: porte de `preview/preanalysis.js` (a pré-análise da prévia, calibrada no corpus
//! em `docs/diretor-calibracao.md`). As constantes são as mesmas; mudar aqui exige recalibrar.
//!
//! 1. Espectrograma em 64 bandas logarítmicas a cada 10 ms e fluxo espectral (SuperFlux simplificado).
//! 2. Ataques: picos do fluxo acima da média local.
//! 3. Andamento por autocorrelação (com preferência por volta de 120 BPM) e batidas por programação dinâmica
//!    (Ellis 2007, o princípio do librosa). Clareza do pulso a cada trecho. Tempo forte do compasso (4/4).
//! 4. Descritores a 10 Hz (sonoridade K-ponderada, pulso, ruído, espectro cheio, fluxo, grave, brilho) e a
//!    energia, com os pesos calibrados.
//! 5. Trechos pela novidade (o quanto os descritores mudam), com a fronteira presa ao tempo forte.
//! 6. Energia final: degrau por trecho mais a curva contínua, com contraste para faixas pouco dinâmicas.
//! 7. Drops (salto de energia com batida e volume, no ataque grave mais forte) e a subida antes de cada um.

use crate::{Ataque, Batida, Drop, Secao};
use analise::diretor::smoothstep;

/// Taxa das curvas (energia, volume, atividade, subida, clareza).
pub const FPS_CURVA: f32 = 10.0;

/// Pesos da energia absoluta (`ENERGY_WEIGHTS` na prévia).
struct Pesos {
    loud: f32,
    pulse: f32,
    noise: f32,
    full: f32,
    flux: f32,
    tempo: f32,
    rel: f32,
    floor: f32,
}
const PESOS: Pesos =
    Pesos { loud: 0.3, pulse: 0.15, noise: 0.15, full: 0.1, flux: 0.1, tempo: 0.15, rel: 0.22, floor: 0.14 };

fn clamp01(x: f32) -> f32 {
    x.clamp(0.0, 1.0)
}

/// Percentil como na prévia: ordena e pega o índice `floor(p·n)`.
fn percentil(v: impl IntoIterator<Item = f32>, p: f32) -> f32 {
    let mut v: Vec<f32> = v.into_iter().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    v[((p * v.len() as f32).floor() as usize).min(v.len() - 1)]
}

/// Suavização gaussiana que olha para trás e para frente (`sigma` em amostras da curva).
fn suavizar(x: &[f32], sigma: f32) -> Vec<f32> {
    if sigma <= 0.0 {
        return x.to_vec();
    }
    let r = (sigma * 3.0).ceil() as i64;
    let w: Vec<f32> = (-r..=r).map(|i| (-0.5 * (i as f32 / sigma).powi(2)).exp()).collect();
    let n = x.len() as i64;
    (0..n)
        .map(|i| {
            let (mut s, mut ws) = (0.0f32, 0.0f32);
            for j in -r..=r {
                let k = i + j;
                if k >= 0 && k < n {
                    s += x[k as usize] * w[(j + r) as usize];
                    ws += w[(j + r) as usize];
                }
            }
            s / ws
        })
        .collect()
}

/// FFT complexa radix-2 no lugar (a mesma da prévia).
pub(crate) struct Fft {
    n: usize,
    cos: Vec<f64>,
    sin: Vec<f64>,
    rev: Vec<usize>,
}

impl Fft {
    pub(crate) fn nova(n: usize) -> Self {
        let niveis = n.trailing_zeros();
        let cos = (0..n / 2).map(|i| (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos()).collect();
        let sin = (0..n / 2).map(|i| (2.0 * std::f64::consts::PI * i as f64 / n as f64).sin()).collect();
        let rev = (0..n)
            .map(|i| {
                let mut r = 0;
                for b in 0..niveis {
                    r |= ((i >> b) & 1) << (niveis - 1 - b);
                }
                r
            })
            .collect();
        Self { n, cos, sin, rev }
    }

    pub(crate) fn rodar(&self, re: &mut [f64], im: &mut [f64]) {
        let n = self.n;
        for i in 0..n {
            let j = self.rev[i];
            if j > i {
                re.swap(i, j);
                im.swap(i, j);
            }
        }
        let mut tamanho = 2;
        while tamanho <= n {
            let (metade, passo) = (tamanho / 2, n / tamanho);
            let mut i = 0;
            while i < n {
                let mut k = 0;
                for j in i..i + metade {
                    let l = j + metade;
                    let tr = re[l] * self.cos[k] + im[l] * self.sin[k];
                    let ti = -re[l] * self.sin[k] + im[l] * self.cos[k];
                    re[l] = re[j] - tr;
                    im[l] = im[j] - ti;
                    re[j] += tr;
                    im[j] += ti;
                    k += passo;
                }
                i += tamanho;
            }
            tamanho <<= 1;
        }
    }
}

/// O que sai do espectrograma, quadro a quadro (100 por segundo).
struct Quadros {
    fps: f32,
    n: usize,
    odf: Vec<f32>,
    odf_grave: Vec<f32>,
    centroide: Vec<f32>,
    cheio20: Vec<f32>,
    plano: Vec<f32>,
    grave_db: Vec<f32>,
    total_db: Vec<f32>,
}

/// 1. Espectrograma em bandas logarítmicas e fluxo espectral.
fn quadros(pcm: &[f32], taxa: u32, parar: &dyn Fn() -> bool) -> Option<Quadros> {
    let taxa_f = taxa as f32;
    let h = (taxa_f / 100.0).round() as usize;
    let fps = taxa_f / h as f32;
    let n_fft = if taxa > 50_000 { 4096 } else { 2048 };
    let fft = Fft::nova(n_fft);
    let janela: Vec<f64> =
        (0..n_fft).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n_fft as f64).cos()).collect();
    let n = pcm.len().div_ceil(h).max(1);

    // Banco de filtros triangulares em escala log, 30 Hz – 16 kHz.
    const B: usize = 64;
    let (fmin, fmax) = (30.0f32, (taxa_f * 0.45).min(16_000.0));
    let bin_hz = taxa_f / n_fft as f32;
    let bordas: Vec<f32> = (0..B + 2).map(|j| fmin * (fmax / fmin).powf(j as f32 / (B + 1) as f32)).collect();
    let mut bandas: Vec<(Vec<(usize, f32)>, f32)> = Vec::with_capacity(B);
    for j in 0..B {
        let (lo, meio, hi) = (bordas[j], bordas[j + 1], bordas[j + 2]);
        let mut w = Vec::new();
        let k0 = ((lo / bin_hz).floor() as usize).max(1);
        let k1 = (hi / bin_hz).ceil() as usize;
        for k in k0..=k1 {
            if k >= n_fft / 2 {
                break;
            }
            let f = k as f32 * bin_hz;
            let v = if f < meio { (f - lo) / (meio - lo) } else { (hi - f) / (hi - meio) };
            if v > 0.0 {
                w.push((k, v));
            }
        }
        if w.is_empty() {
            w.push((((meio / bin_hz).round() as usize).max(1), 1.0));
        }
        bandas.push((w, meio));
    }
    let n_low = bandas.iter().filter(|b| b.1 < 200.0).count();
    let n_bass = bandas.iter().filter(|b| b.1 < 150.0).count();

    let mut q = Quadros {
        fps,
        n,
        odf: vec![0.0; n],
        odf_grave: vec![0.0; n],
        centroide: vec![0.0; n],
        cheio20: vec![0.0; n],
        plano: vec![0.0; n],
        grave_db: vec![0.0; n],
        total_db: vec![0.0; n],
    };
    // Fluxo contra o quadro de 20 ms atrás (SuperFlux), com filtro de máximo em frequência.
    const MU: usize = 2;
    let mut historico: Vec<[f32; B]> = Vec::with_capacity(MU + 2);
    let (mut re, mut im) = (vec![0.0f64; n_fft], vec![0.0f64; n_fft]);
    let mut mag = [vec![0.0f64; n_fft / 2], vec![0.0f64; n_fft / 2]];
    // Amplitude ~1 para uma senoide de fundo de escala (janela de Hann).
    let norma = 4.0 / n_fft as f64;
    let amostra = |p: i64| if p >= 0 && (p as usize) < pcm.len() { f64::from(pcm[p as usize]) } else { 0.0 };

    let mut n0 = 0;
    while n0 < n {
        if (n0 & 1023) == 0 && parar() {
            return None;
        }
        // Duas janelas reais numa FFT complexa (parte real e imaginária).
        for i in 0..n_fft {
            let pa = (n0 * h) as i64 - (n_fft / 2) as i64 + i as i64;
            let pb = pa + h as i64;
            re[i] = amostra(pa) * janela[i];
            im[i] = if n0 + 1 < n { amostra(pb) * janela[i] } else { 0.0 };
        }
        fft.rodar(&mut re, &mut im);
        for k in 0..n_fft / 2 {
            let k2 = if k == 0 { 0 } else { n_fft - k };
            let (xr, xi) = ((re[k] + re[k2]) / 2.0, (im[k] - im[k2]) / 2.0);
            let (yr, yi) = ((im[k] + im[k2]) / 2.0, -(re[k] - re[k2]) / 2.0);
            mag[0][k] = xr.hypot(xi) * norma;
            mag[1][k] = yr.hypot(yi) * norma;
        }
        for (s, m) in mag.iter().enumerate() {
            let idx = n0 + s;
            if idx >= n {
                break;
            }
            let mut y = [0.0f32; B];
            let mut e = [0.0f64; B];
            let (mut etot, mut ebass, mut cpos, mut max_db) = (0.0f64, 0.0f64, 0.0f64, -200.0f64);
            for j in 0..B {
                let ej: f64 = bandas[j].0.iter().map(|(k, v)| f64::from(*v) * m[*k] * m[*k]).sum();
                e[j] = ej;
                etot += ej;
                if j < n_bass {
                    ebass += ej;
                }
                cpos += ej * (j as f64 / (B - 1) as f64);
                y[j] = (1.0 + 100.0 * ej.sqrt()).ln() as f32;
                max_db = max_db.max(10.0 * (ej + 1e-12).log10());
            }
            let (mut nf20, mut lg) = (0usize, 0.0f64);
            for ej in &e {
                let db = 10.0 * (ej + 1e-12).log10();
                if db > max_db - 20.0 {
                    nf20 += 1;
                }
                lg += (ej + 1e-12).ln();
            }
            q.cheio20[idx] = nf20 as f32 / B as f32;
            q.plano[idx] = if etot > 1e-10 { ((lg / B as f64).exp() / (etot / B as f64)) as f32 } else { 0.0 };
            q.centroide[idx] = if etot > 1e-10 { (cpos / etot) as f32 } else { 0.0 };
            q.grave_db[idx] = (10.0 * (ebass + 1e-12).log10()) as f32;
            q.total_db[idx] = (10.0 * (etot + 1e-12).log10()) as f32;
            if historico.len() >= MU {
                let r = &historico[historico.len() - MU];
                let (mut fa, mut fl) = (0.0f32, 0.0f32);
                for j in 0..B {
                    let antes =
                        r[j].max(if j > 0 { r[j - 1] } else { 0.0 }).max(if j < B - 1 { r[j + 1] } else { 0.0 });
                    let d = y[j] - antes;
                    if d > 0.0 {
                        fa += d;
                        if j < n_low {
                            fl += d;
                        }
                    }
                }
                q.odf[idx] = fa / B as f32;
                q.odf_grave[idx] = fl / n_low.max(1) as f32;
            }
            historico.push(y);
            if historico.len() > MU + 1 {
                historico.remove(0);
            }
        }
        n0 += 2;
    }
    Some(q)
}

/// 2. Ataques: picos locais do fluxo acima da média local. Devolve os ataques e o fluxo normalizado.
fn ataques(q: &Quadros) -> (Vec<Ataque>, Vec<f32>) {
    let n = q.odf.len();
    let escala = percentil(q.odf.iter().copied(), 0.98) + 1e-9;
    let x: Vec<f32> = q.odf.iter().map(|v| v / escala).collect();
    let fps = q.fps;
    let (pre_max, pos_max) = ((0.03 * fps).round() as usize, (0.03 * fps).round() as usize);
    let (pre_med, pos_med) = ((0.1 * fps).round() as usize, (0.07 * fps).round() as usize);
    let espera = (0.05 * fps).round() as i64;
    let mut saida = Vec::new();
    let mut ultimo = i64::MIN / 2;
    for i in 1..n.saturating_sub(1) {
        let v = x[i];
        if v < 0.1 {
            continue;
        }
        if (i.saturating_sub(pre_max)..=(i + pos_max).min(n - 1)).any(|j| x[j] > v) {
            continue;
        }
        let faixa = i.saturating_sub(pre_med)..=(i + pos_med).min(n - 1);
        let quantos = faixa.clone().count() as f32;
        let media = faixa.map(|j| x[j]).sum::<f32>() / quantos;
        if v < media + 0.07 || (i as i64) - ultimo < espera {
            continue;
        }
        ultimo = i as i64;
        let grave = q.odf_grave[i] / (q.odf[i] * 1.5 + 1e-9);
        saida.push(Ataque { t: f64::from(i as f32 / fps), forca: clamp01(v), grave: clamp01(grave) });
    }
    (saida, x)
}

/// 3a. Andamento: autocorrelação do fluxo, com preferência log-normal por volta de 120 BPM.
fn andamento(odf: &[f32], fps: f32) -> f32 {
    let n = odf.len();
    if n < 10 {
        return 120.0;
    }
    let media = odf.iter().sum::<f32>() / n as f32;
    let x: Vec<f32> = odf.iter().map(|v| v - media).collect();
    let lag_min = (fps * 60.0 / 240.0).round() as usize;
    let lag_max = ((fps * 60.0 / 50.0).round() as usize).min(n.saturating_sub(2));
    if lag_max <= lag_min + 2 {
        return 120.0;
    }
    let mut ac = vec![0.0f64; lag_max + 2];
    for l in lag_min - 1..=lag_max + 1 {
        if l >= n {
            continue;
        }
        let s: f64 = (l..n).map(|i| f64::from(x[i]) * f64::from(x[i - l])).sum();
        ac[l] = s / (n - l) as f64;
    }
    let (mut melhor, mut lag_melhor) = (f64::NEG_INFINITY, (fps * 60.0 / 120.0).round() as usize);
    for l in lag_min..=lag_max {
        let bpm = f64::from(fps) * 60.0 / l as f64;
        let preferencia = (-0.5 * (bpm / 120.0).log2().powi(2)).exp();
        let v = ac[l] * preferencia;
        if v > melhor && ac[l] >= ac[l - 1] && ac[l] >= ac[l + 1] {
            melhor = v;
            lag_melhor = l;
        }
    }
    let (a, b, c) = (ac[lag_melhor - 1], ac[lag_melhor], ac[lag_melhor + 1]);
    let d = a - 2.0 * b + c;
    let lag = lag_melhor as f64 + if d != 0.0 { (0.5 * (a - c) / d).clamp(-0.5, 0.5) } else { 0.0 };
    (f64::from(fps) * 60.0 / lag) as f32
}

/// 3b. Batidas por programação dinâmica. Devolve os índices dos quadros das batidas.
fn batidas_dp(odf: &[f32], fps: f32, bpm: f32) -> Vec<usize> {
    const RIGIDEZ: f32 = 100.0;
    let n = odf.len();
    let periodo = (fps * 60.0 / bpm).round().max(2.0) as i64;
    let mu = odf.iter().sum::<f32>() / n as f32;
    let sd = (odf.iter().map(|v| (v - mu).powi(2)).sum::<f32>() / n as f32).sqrt() + 1e-9;
    // Pontuação local: o fluxo convoluído com uma gaussiana estreita (~ período/32).
    let nucleo: Vec<f32> =
        (-periodo..=periodo).map(|j| (-0.5 * ((j * 32) as f32 / periodo as f32).powi(2)).exp()).collect();
    let local: Vec<f32> = (0..n as i64)
        .map(|i| {
            let mut s = 0.0;
            for j in -periodo..=periodo {
                let k = i + j;
                if k >= 0 && k < n as i64 {
                    s += odf[k as usize] / sd * nucleo[(j + periodo) as usize];
                }
            }
            s
        })
        .collect();
    let (w_ini, w_fim) = (-2 * periodo, -(periodo as f32 / 2.0).round() as i64);
    let pesos: Vec<f32> = (w_ini..=w_fim).map(|o| -RIGIDEZ * ((-o) as f32 / periodo as f32).ln().powi(2)).collect();
    let mut acumulado = vec![0.0f32; n];
    let mut volta = vec![-1i64; n];
    let max_local = local.iter().copied().fold(f32::MIN, f32::max);
    let mut primeiro = true;
    for i in 0..n {
        let (mut melhor, mut k_melhor) = (f32::NEG_INFINITY, -1i64);
        for (w, o) in (w_ini..=w_fim).enumerate() {
            let k = i as i64 + o;
            if k < 0 {
                continue;
            }
            let v = pesos[w] + acumulado[k as usize];
            if v > melhor {
                melhor = v;
                k_melhor = k;
            }
        }
        if k_melhor < 0 {
            melhor = 0.0;
        }
        acumulado[i] = local[i] + melhor;
        if primeiro && local[i] < 0.01 * max_local {
            volta[i] = -1;
        } else {
            volta[i] = k_melhor;
            primeiro = false;
        }
    }
    // Última batida: o último máximo local acima de metade da mediana dos máximos.
    let maximos: Vec<usize> = (1..n.saturating_sub(1))
        .filter(|&i| acumulado[i] > acumulado[i - 1] && acumulado[i] >= acumulado[i + 1])
        .collect();
    let Some(&primeiro_max) = maximos.first() else { return Vec::new() };
    let mediana = percentil(maximos.iter().map(|&i| acumulado[i]), 0.5);
    let mut fim = primeiro_max;
    for &i in &maximos {
        if acumulado[i] >= 0.5 * mediana {
            fim = i;
        }
    }
    let mut batidas = vec![fim];
    while let Some(&ultima) = batidas.last() {
        let anterior = volta[ultima];
        if anterior < 0 || batidas.len() > n {
            break;
        }
        batidas.push(anterior as usize);
    }
    batidas.reverse();
    // Tira as batidas fracas do começo e do fim (silêncio, fade).
    let medias: Vec<f32> = (0..batidas.len())
        .map(|i| {
            let faixa = i.saturating_sub(2)..=(i + 2).min(batidas.len() - 1);
            let c = faixa.clone().count() as f32;
            faixa.map(|j| local[batidas[j]]).sum::<f32>() / c
        })
        .collect();
    let limiar = 0.5 * (medias.iter().map(|v| v * v).sum::<f32>() / medias.len() as f32).sqrt();
    let (mut a, mut z) = (0usize, batidas.len() - 1);
    while a < z && medias[a] <= limiar {
        a += 1;
    }
    while z > a && medias[z] <= limiar {
        z -= 1;
    }
    batidas[a..=z].to_vec()
}

/// 3c. Clareza do pulso a cada 0,5 s: autocorrelação local (janela de 8 s) no período da batida. Fala e pads
/// dão ~0–0,1; bateria regular, 0,3–0,7.
fn clareza(odf: &[f32], fps: f32, periodo: f32) -> Vec<f32> {
    const PASSO_S: f32 = 0.5;
    let metade = (4.0 * fps).round() as usize;
    let n = odf.len();
    let total = (n as f32 / fps / PASSO_S).ceil() as usize + 1;
    let mut lags = Vec::new();
    for m in [1.0f32, 2.0] {
        let mut f = 0.97f32;
        while f <= 1.031 {
            lags.push((periodo * m * f).round() as usize);
            f += 0.015;
        }
    }
    (0..total)
        .map(|v| {
            let c = (v as f32 * PASSO_S * fps).round() as usize;
            let (a, z) = (c.saturating_sub(metade), (c + metade).min(n));
            if (z.saturating_sub(a) as f32) < 2.0 * periodo * 2.0 {
                return 0.0;
            }
            let media = odf[a..z].iter().sum::<f32>() / (z - a) as f32;
            let r0: f32 = odf[a..z].iter().map(|x| (x - media).powi(2)).sum();
            if r0 < 1e-6 {
                return 0.0;
            }
            let mut melhor = 0.0f32;
            for &l in &lags {
                if a + l >= z {
                    continue;
                }
                let r: f32 = (a + l..z).map(|i| (odf[i] - media) * (odf[i - l] - media)).sum();
                let r = r / r0 * ((z - a) as f32 / (z - a - l) as f32);
                melhor = melhor.max(r);
            }
            melhor
        })
        .collect()
}

/// O resultado da análise da música inteira.
pub struct Linha {
    pub duracao_s: f64,
    pub bpm: f32,
    pub clareza_da_batida: f32,
    pub energia: Vec<f32>,
    pub volume: Vec<f32>,
    pub atividade: Vec<f32>,
    pub preparo: Vec<f32>,
    pub conf: Vec<f32>,
    pub batidas: Vec<Batida>,
    pub ataques: Vec<Ataque>,
    pub drops: Vec<Drop>,
    pub secoes: Vec<Secao>,
}

/// A análise completa. `pcm` em mono (a soma -3 dB dos canais, como o ffmpeg faz, que é o nível em que a
/// prévia foi calibrada). `parar` interrompe no meio.
pub fn analisar_pcm(pcm: &[f32], taxa: u32, parar: &dyn Fn() -> bool) -> Option<Linha> {
    let duracao = pcm.len() as f64 / f64::from(taxa);
    let q = quadros(pcm, taxa, parar)?;
    let fps = q.fps;
    let (ataques, odf) = ataques(&q);
    if parar() {
        return None;
    }
    let bpm = andamento(&odf, fps);
    let idx_batidas = batidas_dp(&odf, fps, bpm);
    if parar() {
        return None;
    }

    // Por batida: pico do fluxo na batida e o grave.
    let meia = (0.03 * fps).round() as i64;
    let mut picos = Vec::with_capacity(idx_batidas.len());
    for &b in &idx_batidas {
        let mut pk = 0.0f32;
        for j in b as i64 - meia..=b as i64 + meia {
            if j >= 0 && (j as usize) < odf.len() {
                pk = pk.max(odf[j as usize]);
            }
        }
        picos.push((pk, q.odf_grave[b]));
    }
    let periodo = fps * 60.0 / bpm;
    let clar = clareza(&odf, fps, periodo);
    let clareza_em = |t: f64| clar[((t / 0.5).round().max(0.0) as usize).min(clar.len().saturating_sub(1))];
    let tempos: Vec<f64> = idx_batidas.iter().map(|&b| f64::from(b as f32 / fps)).collect();
    let confs: Vec<f32> = tempos.iter().map(|&t| smoothstep(0.1, 0.45, clareza_em(t))).collect();
    // Tempo forte (4/4): a fase com mais grave e ataque.
    let (mut fase_forte, mut melhor) = (0usize, -1.0f32);
    for p in 0..4 {
        let s: f32 = (p..picos.len()).step_by(4).map(|i| picos[i].1 * 1.5 + picos[i].0).sum();
        if s > melhor {
            melhor = s;
            fase_forte = p;
        }
    }
    let escala_pico = percentil(picos.iter().map(|p| p.0), 0.9) + 1e-9;
    let batidas: Vec<Batida> = (0..tempos.len())
        .map(|i| Batida {
            t: tempos[i],
            forca: clamp01(picos[i].0 / escala_pico) * confs[i],
            forte: i >= fase_forte && (i - fase_forte) % 4 == 0,
        })
        .collect();
    let clareza_da_batida = if confs.is_empty() { 0.0 } else { percentil(confs.iter().copied(), 0.5) };

    // ---------- 4. Descritores a 10 Hz ----------
    let nb = ((duracao * f64::from(FPS_CURVA)).ceil() as usize).max(1);
    let lufs = sonoridade_curta(pcm, taxa, nb);
    let quadro_em = |k: usize| ((k as f32 / FPS_CURVA * fps).round() as usize).min(q.n - 1);
    let media_quadros = |arr: &[f32], k: usize, meia: usize| {
        let c = quadro_em(k);
        let faixa = c.saturating_sub(meia)..=(c + meia).min(q.n - 1);
        let quantos = faixa.clone().count() as f32;
        faixa.map(|i| arr[i]).sum::<f32>() / quantos
    };
    let mut conf = vec![0.0f32; nb];
    if !batidas.is_empty() {
        let per = 60.0 / f64::from(bpm);
        let mut j = 0;
        for (k, c) in conf.iter_mut().enumerate() {
            let t = k as f64 / f64::from(FPS_CURVA);
            while j < batidas.len() - 1 && batidas[j + 1].t <= t {
                j += 1;
            }
            let (b0, b1) = (j, (j + 1).min(batidas.len() - 1));
            let perto = (t - batidas[b0].t).abs().min((t - batidas[b1].t).abs());
            *c = if perto < 2.0 * per { (confs[b0] + confs[b1]) / 2.0 } else { 0.0 };
        }
    }
    let mut continuidade = vec![0.0f32; nb];
    {
        let meia = fps.round() as usize;
        for (k, c) in continuidade.iter_mut().enumerate() {
            let centro = quadro_em(k);
            let faixa = centro.saturating_sub(meia)..=(centro + meia).min(q.n - 1);
            let mx = faixa.clone().map(|i| q.total_db[i]).fold(-200.0f32, f32::max);
            let (on, cnt) =
                faixa.fold((0u32, 0u32), |(on, cnt), i| (on + u32::from(q.total_db[i] > mx - 22.0), cnt + 1));
            *c = if cnt > 0 { on as f32 / cnt as f32 } else { 0.0 };
        }
    }
    let dobra = if bpm > 180.0 {
        bpm / 2.0
    } else if bpm < 70.0 {
        bpm * 2.0
    } else {
        bpm
    };
    let tempo01 = smoothstep(80.0, 160.0, dobra);
    let mut d_loud = vec![0.0f32; nb];
    let mut d_pulse = vec![0.0f32; nb];
    let mut d_noise = vec![0.0f32; nb];
    let mut d_full = vec![0.0f32; nb];
    let mut d_flux = vec![0.0f32; nb];
    let mut d_bass = vec![0.0f32; nb];
    let mut d_bright = vec![0.0f32; nb];
    for k in 0..nb {
        let audivel = lufs[k] > -60.0;
        d_loud[k] = clamp01((lufs[k] + 30.0) / 22.0);
        d_pulse[k] = smoothstep(0.08, 0.6, clareza_em(k as f64 / f64::from(FPS_CURVA)));
        d_noise[k] = if audivel { smoothstep(0.03, 0.3, media_quadros(&q.plano, k, 5)) } else { 0.0 };
        d_full[k] = if audivel { smoothstep(0.3, 0.8, media_quadros(&q.cheio20, k, 5)) } else { 0.0 };
        d_flux[k] = smoothstep(0.01, 0.09, media_quadros(&q.odf, k, 5));
        d_bass[k] = smoothstep(-40.0, -5.0, media_quadros(&q.grave_db, k, 5));
        d_bright[k] = smoothstep(0.2, 0.5, media_quadros(&q.centroide, k, 5));
    }
    let d_cont = suavizar(&continuidade, 1.5 * FPS_CURVA);
    // Dinâmica relativa: o volume do trecho comparado à mediana da própria faixa.
    let lufs_s = suavizar(&lufs, FPS_CURVA);
    let mediana_lufs = percentil((0..nb).filter(|&k| lufs[k] > -60.0).map(|k| lufs_s[k]), 0.5);
    let rel: Vec<f32> = lufs_s.iter().map(|v| ((v - mediana_lufs) / 10.0).clamp(-1.0, 1.0)).collect();
    let w = &PESOS;
    let bruta: Vec<f32> = (0..nb)
        .map(|k| {
            if lufs[k] <= -60.0 {
                return 0.0;
            }
            // Pulso e andamento só empurram a energia quando há corpo sonoro.
            let drive = smoothstep(0.1, 0.55, d_loud[k]);
            let mut a = w.loud * d_loud[k]
                + (w.pulse + w.tempo * tempo01) * d_pulse[k] * drive
                + w.noise * d_noise[k]
                + w.full * d_full[k]
                + w.flux * d_flux[k];
            // Som com pausas frequentes (fala) é menos animado que um fluxo contínuo.
            a *= 0.55 + 0.45 * smoothstep(0.55, 0.9, d_cont[k]);
            w.floor + (1.0 - w.floor) * clamp01(a + w.rel * rel[k])
        })
        .collect();
    if parar() {
        return None;
    }

    // ---------- 5. Trechos (novidade) ----------
    let descritores: [(&[f32], f32); 7] = [
        (&d_loud, 2.0),
        (&d_bass, 1.5),
        (&d_pulse, 1.5),
        (&d_full, 1.0),
        (&d_noise, 1.0),
        (&d_flux, 1.0),
        (&d_bright, 0.5),
    ];
    let wn = (4.0 * FPS_CURVA) as usize;
    let mut novidade = vec![0.0f32; nb];
    if nb > 2 * wn {
        for (k, nv) in novidade.iter_mut().enumerate().take(nb - wn).skip(wn) {
            let s: f32 = descritores
                .iter()
                .map(|(d, peso)| {
                    let a: f32 = d[k - wn..k].iter().sum();
                    let b: f32 = d[k..k + wn].iter().sum();
                    peso * ((b - a) / wn as f32).powi(2)
                })
                .sum();
            *nv = s.sqrt();
        }
    }
    let n_media = novidade.iter().sum::<f32>() / nb as f32;
    let n_sd = (novidade.iter().map(|v| (v - n_media).powi(2)).sum::<f32>() / nb as f32).sqrt();
    let vao = wn;
    let mut fronteiras: Vec<usize> = Vec::new();
    let raio = (3.0 * FPS_CURVA) as usize;
    for k in 1..nb.saturating_sub(1) {
        if novidade[k] < (n_media + 0.6 * n_sd).max(0.12) {
            continue;
        }
        let e_max = (k.saturating_sub(raio)..=(k + raio).min(nb - 1)).all(|j| novidade[j] <= novidade[k]);
        if e_max && fronteiras.last().is_none_or(|&u| k - u >= vao) {
            fronteiras.push(k);
        }
    }
    // Cada fronteira vai para o tempo forte (ou batida, ou ataque forte) mais perto.
    let prender = |t: f64| -> f64 {
        let (mut melhor_t, mut melhor_d) = (t, f64::INFINITY);
        if clareza_da_batida > 0.35 {
            for b in &batidas {
                let d = (b.t - t).abs() * if b.forte { 0.6 } else { 1.0 };
                if d < melhor_d && (b.t - t).abs() < 0.7 {
                    melhor_d = d;
                    melhor_t = b.t;
                }
            }
            if melhor_d < f64::INFINITY {
                return melhor_t;
            }
        }
        for o in &ataques {
            let d = (o.t - t).abs() / (0.3 + f64::from(o.forca));
            if (o.t - t).abs() < 0.4 && d < melhor_d {
                melhor_d = d;
                melhor_t = o.t;
            }
        }
        melhor_t
    };
    let mut cortes = vec![0.0];
    cortes.extend(fronteiras.iter().map(|&k| prender(k as f64 / f64::from(FPS_CURVA))));
    cortes.push(duracao);
    let mut secoes: Vec<Secao> = Vec::new();
    for i in 0..cortes.len() - 1 {
        let k0 = (cortes[i] * f64::from(FPS_CURVA)).floor() as usize;
        let k1 = ((cortes[i + 1] * f64::from(FPS_CURVA)).floor() as usize).max(k0 + 1);
        let corte = 5.min((k1 - k0) / 5);
        let faixa = (k0 + corte)..(k1.saturating_sub(corte)).min(nb);
        let c = faixa.len();
        let energia = if c > 0 { faixa.map(|k| bruta[k]).sum::<f32>() / c as f32 } else { bruta[k0.min(nb - 1)] };
        secoes.push(Secao { inicio_s: cortes[i], fim_s: cortes[i + 1], energia });
    }

    // ---------- 6. Curva de energia final ----------
    let bruta_suave = suavizar(&bruta, 0.5 * FPS_CURVA);
    let mut degrau = vec![0.0f32; nb];
    for s in &secoes {
        let a = (s.inicio_s * f64::from(FPS_CURVA)).floor() as usize;
        let z = ((s.fim_s * f64::from(FPS_CURVA)).ceil() as usize).min(nb);
        for d in degrau.iter_mut().take(z).skip(a) {
            *d = s.energia;
        }
    }
    // Subida: imediata na fronteira; descida: rampa de ~0,8 s.
    for i in 1..secoes.len() {
        let (antes, depois) = (secoes[i - 1].energia, secoes[i].energia);
        if depois >= antes {
            continue;
        }
        let k0 = (secoes[i].inicio_s * f64::from(FPS_CURVA)).round() as usize;
        for j in 0..8 {
            if k0 + j < nb {
                degrau[k0 + j] = antes + (depois - antes) * ((j + 1) as f32 / 8.0);
            }
        }
    }
    // Música com batida tem trechos claros (degraus); fala e música livre seguem a curva contínua.
    let peso_degrau = 0.25 + 0.5 * clareza_da_batida;
    let mut energia: Vec<f32> =
        (0..nb).map(|k| peso_degrau * degrau[k] + (1.0 - peso_degrau) * bruta_suave[k]).collect();
    // Contraste: faixas com pouca dinâmica entre trechos ganham um pouco de expansão.
    let audiveis: Vec<f32> = (0..nb).filter(|&k| lufs[k] > -60.0).map(|k| energia[k]).collect();
    let mediana = percentil(audiveis.iter().copied(), 0.5);
    let espalhamento = percentil(audiveis.iter().copied(), 0.9) - percentil(audiveis.iter().copied(), 0.1);
    let g = (0.3 / espalhamento.max(0.05)).clamp(1.0, 1.3);
    for (k, e) in energia.iter_mut().enumerate() {
        *e = if lufs[k] > -60.0 { clamp01(mediana + g * (*e - mediana)) } else { 0.0 };
    }
    for s in &mut secoes {
        s.energia = clamp01(mediana + g * (s.energia - mediana));
    }

    // ---------- 7. Drops e subidas ----------
    let mut drops = Vec::new();
    let mut preparo = vec![0.0f32; nb];
    for i in 1..secoes.len() {
        let (antes, s) = (&secoes[i - 1], &secoes[i]);
        let salto = s.energia - antes.energia;
        let kb = (s.inicio_s * f64::from(FPS_CURVA)).round() as usize;
        let grave_em = |k: usize| d_bass.get(k.min(nb - 1)).copied().unwrap_or(0.0);
        let salto_grave = grave_em(kb + 5) - grave_em(kb.saturating_sub(10));
        let janela = kb..(kb + (4.0 * FPS_CURVA) as usize).min(nb);
        let cnt = janela.len().max(1) as f32;
        let pulso_novo = janela.clone().map(|k| d_pulse[k]).sum::<f32>() / cnt;
        let volume_novo = janela.map(|k| d_loud[k]).sum::<f32>() / cnt;
        // Drop = a energia salta E a batida está lá (sem batida é só um crescendo).
        if salto < 0.15 || s.energia < 0.55 || pulso_novo < 0.45 || volume_novo < 0.5 || clareza_da_batida < 0.3 {
            continue;
        }
        let mut forca = clamp01((salto - 0.08) / 0.35)
            * smoothstep(0.45, 0.85, s.energia)
            * if salto_grave > 0.25 { 1.2 } else { 1.0 };
        forca = clamp01(forca);
        if forca < 0.15 {
            continue;
        }
        // Instante exato: o ataque grave mais forte perto da fronteira.
        let (mut t, mut bs) = (s.inicio_s, -1.0f32);
        for o in &ataques {
            if o.t < s.inicio_s - 0.25 || o.t > s.inicio_s + 0.35 {
                continue;
            }
            let v = o.forca * (0.5 + o.grave);
            if v > bs {
                bs = v;
                t = o.t;
            }
        }
        drops.push(Drop { t, forca });
        // Subida antes do drop: rampa quadrática desde o começo do trecho anterior (no máximo 16 s).
        let t0 = antes.inicio_s.max(t - 16.0);
        if t - t0 >= 2.0 {
            let k_ini = (t0 * f64::from(FPS_CURVA)).ceil() as usize;
            let k_fim = ((t * f64::from(FPS_CURVA)).floor() as usize).min(nb);
            for (k, p) in preparo.iter_mut().enumerate().take(k_fim).skip(k_ini) {
                let u = ((k as f64 / f64::from(FPS_CURVA) - t0) / (t - t0)) as f32;
                *p = p.max(forca * u * u);
            }
        }
    }

    // Curvas para a forma: volume rápido e atividade (velocidade da morfose).
    let volume = suavizar(
        &(0..nb).map(|k| clamp01(0.5 * d_loud[k] + 0.5 * (0.5 + 0.5 * rel[k]))).collect::<Vec<_>>(),
        0.15 * FPS_CURVA,
    );
    let atividade = suavizar(
        &(0..nb)
            .map(|k| clamp01(0.4 * d_flux[k] + 0.35 * d_pulse[k] + 0.25 * tempo01 * d_pulse[k]))
            .collect::<Vec<_>>(),
        0.3 * FPS_CURVA,
    );

    Some(Linha {
        duracao_s: duracao,
        bpm,
        clareza_da_batida,
        energia,
        volume,
        atividade,
        preparo,
        conf,
        batidas,
        ataques,
        drops,
        secoes,
    })
}

/// Sonoridade de curto prazo (K-ponderada, janela de 400 ms centrada) a cada 100 ms, em LUFS.
fn sonoridade_curta(pcm: &[f32], taxa: u32, nb: usize) -> Vec<f32> {
    let mut medidor = crate::sonoridade::Filtro::novo(taxa);
    // Soma acumulada da energia ponderada: cada janela sai em tempo constante.
    let mut acumulado = Vec::with_capacity(pcm.len() + 1);
    acumulado.push(0.0f64);
    let mut s = 0.0f64;
    for v in pcm {
        let y = medidor.passar(f64::from(*v));
        s += y * y;
        acumulado.push(s);
    }
    let meia = (0.4 * taxa as f32).round() as usize / 2;
    (0..nb)
        .map(|k| {
            let c = (k as f64 / f64::from(FPS_CURVA) * f64::from(taxa)).round() as usize;
            let (a, z) = (c.saturating_sub(meia), (c + meia).min(pcm.len()));
            if z <= a {
                return -70.0;
            }
            let media = (acumulado[z] - acumulado[a]) / (z - a) as f64;
            (-0.691 + 10.0 * (media + 1e-12).log10()) as f32
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn percentil_e_suavizacao_como_na_previa() {
        assert_eq!(percentil([3.0, 1.0, 2.0, 4.0], 0.5), 3.0);
        assert_eq!(percentil([], 0.5), 0.0);
        let s = suavizar(&[0.0, 0.0, 10.0, 0.0, 0.0], 1.0);
        assert!(s[2] < 10.0 && s[1] > 0.0 && (s[1] - s[3]).abs() < 1e-6);
    }

    #[test]
    fn andamento_de_um_pulso_regular() {
        // Fluxo com um pico a cada 0,4 s (150 BPM) a 100 quadros por segundo.
        let odf: Vec<f32> = (0..3000).map(|i| if i % 40 == 0 { 1.0 } else { 0.0 }).collect();
        let bpm = andamento(&odf, 100.0);
        assert!((bpm - 150.0).abs() < 2.0, "bpm {bpm}");
        let batidas = batidas_dp(&odf, 100.0, bpm);
        assert!(batidas.len() > 60, "{} batidas", batidas.len());
        assert!(batidas.windows(2).all(|w| (w[1] - w[0]).abs_diff(40) <= 1), "batidas no pulso");
    }
}
