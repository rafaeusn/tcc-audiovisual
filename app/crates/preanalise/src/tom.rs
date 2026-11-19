//! O tom da música (tonalidade), pelo cromagrama e pelos perfis de Krumhansl-Kessler.
//!
//! O espectro é dobrado nas 12 notas (cromagrama), somado ao longo da música (com peso pelo volume de cada
//! trecho) e comparado, por correlação, com os 24 perfis de tom (12 maiores, 12 menores). A confiança é a
//! distância entre o melhor e o segundo melhor: perto de zero, o tom é ambíguo (percussão pura, atonal).

use crate::Tom;
use crate::pista::Fft;

/// Perfis de Krumhansl-Kessler: o quanto cada grau da escala "pertence" ao tom.
const MAIOR: [f32; 12] = [6.35, 2.23, 3.48, 2.33, 4.38, 4.09, 2.52, 5.19, 2.39, 3.66, 2.29, 2.88];
const MENOR: [f32; 12] = [6.33, 2.68, 3.52, 5.38, 2.60, 3.53, 2.54, 4.75, 3.98, 2.69, 3.34, 3.17];
const NOTAS: [&str; 12] = ["Dó", "Dó♯", "Ré", "Mi♭", "Mi", "Fá", "Fá♯", "Sol", "Lá♭", "Lá", "Si♭", "Si"];
/// Roda de Camelot (a notação dos DJs): número para cada tônica, A = menor, B = maior.
const CAMELOT_MAIOR: [u8; 12] = [8, 3, 10, 5, 12, 7, 2, 9, 4, 11, 6, 1];
const CAMELOT_MENOR: [u8; 12] = [5, 12, 7, 2, 9, 4, 11, 6, 1, 8, 3, 10];

fn correlacao(a: &[f32; 12], b: &[f32; 12]) -> f32 {
    let (ma, mb) = (a.iter().sum::<f32>() / 12.0, b.iter().sum::<f32>() / 12.0);
    let (mut num, mut da, mut db) = (0.0, 0.0, 0.0);
    for i in 0..12 {
        let (x, y) = (a[i] - ma, b[i] - mb);
        num += x * y;
        da += x * x;
        db += y * y;
    }
    if da <= 0.0 || db <= 0.0 { 0.0 } else { num / (da * db).sqrt() }
}

/// O tom a partir de um cromagrama somado (energia de cada nota, Dó = 0).
pub fn tom_do_croma(croma: &[f32; 12]) -> Option<Tom> {
    if croma.iter().sum::<f32>() <= 0.0 {
        return None;
    }
    let mut notas: Vec<(f32, usize, bool)> = Vec::with_capacity(24);
    for tonica in 0..12 {
        let girar = |p: &[f32; 12]| {
            let mut r = [0.0f32; 12];
            for (i, v) in r.iter_mut().enumerate() {
                *v = p[(i + 12 - tonica) % 12];
            }
            r
        };
        notas.push((correlacao(croma, &girar(&MAIOR)), tonica, false));
        notas.push((correlacao(croma, &girar(&MENOR)), tonica, true));
    }
    notas.sort_by(|a, b| b.0.total_cmp(&a.0));
    let (melhor, tonica, menor) = notas[0];
    let confianca = ((melhor - notas[1].0) * 4.0).clamp(0.0, 1.0);
    if melhor < 0.3 {
        return None;
    }
    Some(Tom {
        nome: format!("{} {}", NOTAS[tonica], if menor { "menor" } else { "maior" }),
        camelot: format!(
            "{}{}",
            if menor { CAMELOT_MENOR[tonica] } else { CAMELOT_MAIOR[tonica] },
            if menor { 'A' } else { 'B' }
        ),
        confianca,
    })
}

/// O cromagrama somado da música (mono), olhando um trecho a cada 200 ms.
pub fn croma(pcm: &[f32], taxa: u32) -> [f32; 12] {
    const N: usize = 8192;
    let fft = Fft::nova(N);
    let janela: Vec<f64> =
        (0..N).map(|i| 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / N as f64).cos()).collect();
    let bin_hz = taxa as f32 / N as f32;
    // Notas de Dó2 (65 Hz) a Dó7 (2 kHz): abaixo, o bumbo confunde; acima, os harmônicos dominam.
    let (k0, k1) = ((65.0 / bin_hz).ceil() as usize, ((2100.0 / bin_hz).floor() as usize).min(N / 2 - 1));
    let nota_do_bin: Vec<usize> = (0..N / 2)
        .map(|k| {
            let f = k as f32 * bin_hz;
            if f <= 0.0 {
                return 0;
            }
            let midi = 69.0 + 12.0 * (f / 440.0).log2();
            (midi.round() as i64).rem_euclid(12) as usize
        })
        .collect();
    let passo = (taxa as usize / 5).max(1);
    let mut soma = [0.0f32; 12];
    let (mut re, mut im) = (vec![0.0f64; N], vec![0.0f64; N]);
    let mut inicio = 0;
    while inicio + N <= pcm.len() {
        for i in 0..N {
            re[i] = f64::from(pcm[inicio + i]) * janela[i];
            im[i] = 0.0;
        }
        fft.rodar(&mut re, &mut im);
        let mut quadro = [0.0f32; 12];
        let mut energia = 0.0f32;
        for k in k0..=k1 {
            let m = re[k].hypot(im[k]) as f32;
            quadro[nota_do_bin[k]] += m;
            energia += m;
        }
        if energia > 1e-3 {
            // Cada trecho conta pelo formato do seu cromagrama (normalizado), com peso pela raiz do volume:
            // trechos fortes mandam mais, sem que um só trecho decida tudo.
            let peso = energia.sqrt();
            for (s, q) in soma.iter_mut().zip(quadro) {
                *s += q / energia * peso;
            }
        }
        inicio += passo;
    }
    soma
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn acorde(notas: &[f32], taxa: u32, segundos: f32) -> Vec<f32> {
        (0..(taxa as f32 * segundos) as usize)
            .map(|i| {
                let t = i as f32 / taxa as f32;
                notas.iter().map(|f| (std::f32::consts::TAU * f * t).sin()).sum::<f32>() * 0.2
            })
            .collect()
    }

    #[test]
    fn reconhece_la_menor_e_do_maior() {
        // Lá menor: Lá, Dó, Mi (e a tônica reforçada).
        let la_menor = acorde(&[220.0, 261.63, 329.63, 440.0], 44_100, 4.0);
        let tom = tom_do_croma(&croma(&la_menor, 44_100)).expect("tom");
        assert_eq!(tom.nome, "Lá menor");
        assert_eq!(tom.camelot, "8A");
        let do_maior = acorde(&[261.63, 329.63, 392.0, 523.25], 44_100, 4.0);
        let tom = tom_do_croma(&croma(&do_maior, 44_100)).expect("tom");
        assert_eq!(tom.nome, "Dó maior");
        assert_eq!(tom.camelot, "8B");
        assert!(tom_do_croma(&[0.0; 12]).is_none());
    }
}
