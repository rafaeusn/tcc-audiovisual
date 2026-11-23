//! Teste manual do tocador com endereços: toca o primeiro, deixa o segundo como seguinte e mostra a situação a
//! cada meio segundo. Sem som (modo mudo).
//!
//! `cargo run -p tocador --example tocar_endereco -- [--formato m4a] [--agente "..."] <url1> [url2]`
//! (sem `--formato`, vale `wav`).

use std::time::{Duration, Instant};
use tocador::{Comando, Endereco, Origem, Tocador};

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let formato = match args.iter().position(|a| a == "--formato") {
        Some(i) if i + 1 < args.len() => {
            let f = args.remove(i + 1);
            args.remove(i);
            f
        }
        _ => "wav".to_string(),
    };
    let agente = match args.iter().position(|a| a == "--agente") {
        Some(i) if i + 1 < args.len() => {
            let a = args.remove(i + 1);
            args.remove(i);
            Some(a)
        }
        _ => None,
    };
    let enderecos = args;
    let Some(primeiro) = enderecos.first() else {
        eprintln!("uso: tocar_endereco [--formato m4a] <endereço> [seguinte]");
        return;
    };
    let cabecalhos: Vec<(String, String)> = agente.iter().map(|a| ("User-Agent".to_string(), a.clone())).collect();
    let origem = |url: &str| {
        Origem::Endereco(Endereco {
            formato: Some(formato.clone()),
            cabecalhos: cabecalhos.clone(),
            ..Endereco::novo(url)
        })
    };
    let t = Tocador::iniciar(true);
    t.mandar(Comando::Tocar { origem: origem(primeiro), ganho: 1.0 });
    if let Some(segundo) = enderecos.get(1) {
        t.mandar(Comando::Seguinte(Some((origem(segundo), 1.0))));
    }
    let inicio = Instant::now();
    let mut antes = String::new();
    // TOCAR_IR=<segundos>: dois segundos depois de começar, pula para esse ponto (testa os pedaços do meio e do fim).
    let mut pular = std::env::var("TOCAR_IR").ok().and_then(|v| v.parse::<f64>().ok());
    let limite = std::env::var("TOCAR_SEGUNDOS").ok().and_then(|v| v.parse().ok()).unwrap_or(60);
    while inicio.elapsed() < Duration::from_secs(limite) {
        if let Some(ir) = pular.filter(|_| inicio.elapsed() > Duration::from_secs(2)) {
            t.mandar(Comando::Ir(ir));
            pular = None;
        }
        let s = t.situacao();
        let linha = format!(
            "toca={} pos={:.1} dur={:.1} origem={} seguinte={} avancou={} terminadas={} erro={:?}",
            s.tocando,
            s.posicao_s,
            s.duracao_s,
            s.origem.as_ref().map(ToString::to_string).unwrap_or_default(),
            s.seguinte.as_ref().map(ToString::to_string).unwrap_or_default(),
            s.avancou,
            s.terminadas,
            s.erro
        );
        if linha != antes {
            println!("{:5.1}s {linha}", inicio.elapsed().as_secs_f32());
            antes = linha;
        }
        if s.terminadas > 0 && !s.tocando {
            break;
        }
        std::thread::sleep(Duration::from_millis(500));
    }
}
