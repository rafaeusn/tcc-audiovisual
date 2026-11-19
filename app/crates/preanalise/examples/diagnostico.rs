//! Diagnóstico do motor ao vivo sobre um arquivo, do mesmo jeito que `tools/lib/sim.mjs` faz com a prévia em JS
//! (mono a 48 kHz, 60 quadros por segundo, janela terminando na amostra do instante). Serve para comparar os
//! dois número a número.
//! Uso: cargo run --release -p preanalise --example diagnostico -- musica.mp3

use analise::{FFT_PRINCIPAL, Motor, PASSO};
use tocador::fonte::Fonte;

fn q(v: &[f32], p: f32) -> f32 {
    let mut v: Vec<f32> = v.iter().copied().filter(|x| x.is_finite()).collect();
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    v[((p * v.len() as f32) as usize).min(v.len() - 1)]
}

fn main() {
    let Some(caminho) = std::env::args().nth(1) else {
        println!("uso: diagnostico <arquivo>");
        return;
    };
    let taxa = 48_000u32;
    let Ok(mut fonte) = Fonte::abrir(std::path::Path::new(&caminho), taxa) else {
        println!("não abriu");
        return;
    };
    let mut estereo = Vec::new();
    while fonte.proximo(&mut estereo) {}
    let pcm: Vec<f32> = estereo.as_chunks::<2>().0.iter().map(|q| (q[0] + q[1]) * 0.5).collect();
    let mut motor = Motor::new(taxa as f32);
    let quadros = (pcm.len() as f64 / f64::from(taxa) / f64::from(PASSO)) as usize;
    let nomes = ["rms", "flat", "full", "E", "loud", "sens", "bass", "level", "ganho"];
    let mut col: Vec<Vec<f32>> = vec![Vec::new(); nomes.len()];
    let (mut bpms, mut drops, mut drop_antes) = (Vec::new(), Vec::new(), 0.0f32);
    let mut rotulos = [0u32; 3];
    let janela = vec![0.0f32; FFT_PRINCIPAL];
    for f in 0..quadros {
        let t = f as f64 * f64::from(PASSO);
        let fim = (t * f64::from(taxa)) as usize + 1;
        let mut j = janela.clone();
        for (i, v) in j.iter_mut().enumerate() {
            let p = fim as i64 - FFT_PRINCIPAL as i64 + i as i64;
            if p >= 0 && (p as usize) < pcm.len() {
                *v = pcm[p as usize];
            }
        }
        let r = motor.passo(PASSO, &j);
        if r.drop > 0.25 && drop_antes <= 0.25 {
            drops.push(t);
        }
        drop_antes = r.drop;
        if f % 15 == 0 {
            let b = r.bandas;
            for (c, v) in col
                .iter_mut()
                .zip([b.rms_db, b.flatness, b.fullness, r.energia, r.loud, r.sens, b.bass, b.level, r.ganho_db])
            {
                c.push(v);
            }
            if r.bpm > 0.0 {
                bpms.push(r.bpm);
            }
            rotulos[r.rotulo as usize] += 1;
        }
    }
    println!("{caminho}");
    for (n, v) in nomes.iter().zip(&col) {
        println!("{n:<6} p10/p50/p90: {:.3} / {:.3} / {:.3}", q(v, 0.1), q(v, 0.5), q(v, 0.9));
    }
    println!("bpm p50 {:.1}", q(&bpms, 0.5));
    let total: u32 = rotulos.iter().sum();
    println!(
        "rótulos: calma {:.0}% agitada {:.0}% intensa {:.0}%",
        100.0 * rotulos[0] as f32 / total.max(1) as f32,
        100.0 * rotulos[1] as f32 / total.max(1) as f32,
        100.0 * rotulos[2] as f32 / total.max(1) as f32
    );
    println!("drops {}", drops.iter().map(|d| format!("{d:.1}s")).collect::<Vec<_>>().join(" "));

    // A pré-análise (modo arquivo), para comparar com `node tools/simulate.mjs`.
    let inicio = std::time::Instant::now();
    match preanalise::analisar(std::path::Path::new(&caminho), &std::sync::atomic::AtomicBool::new(false)) {
        Ok(a) => {
            println!("--- pré-análise em {:.1} s", inicio.elapsed().as_secs_f32());
            println!(
                "{:.1} BPM | clareza {:.2} | batidas {} | ataques {} | seções {} | tom {}",
                a.bpm,
                a.clareza_da_batida,
                a.batidas.len(),
                a.ataques.len(),
                a.secoes.len(),
                a.tom.as_ref().map_or("?".to_string(), |t| format!("{} ({}, {:.2})", t.nome, t.camelot, t.confianca))
            );
            println!(
                "drops: {}",
                a.drops.iter().map(|d| format!("{:.1}s({:.2})", d.t, d.forca)).collect::<Vec<_>>().join(" ")
            );
            let audivel: Vec<f32> = a.energia.iter().copied().filter(|e| *e > 0.0).collect();
            println!(
                "energia p10/p50/p90: {:.2} / {:.2} / {:.2}",
                q(&audivel, 0.1),
                q(&audivel, 0.5),
                q(&audivel, 0.9)
            );
            println!(
                "seções: {}",
                a.secoes.iter().map(|s| format!("{:.0}s:{:.2}", s.inicio_s, s.energia)).collect::<Vec<_>>().join("  ")
            );
        }
        Err(e) => println!("pré-análise falhou: {e}"),
    }
}
