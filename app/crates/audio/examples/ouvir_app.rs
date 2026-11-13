//! Teste manual: ouve só um app por 4 segundos e mostra o nível do som.
//! Uso: cargo run -p audio --example ouvir_app -- opera.exe

use audio::{Anel, CapturaDeApp, Panorama};
use std::sync::Arc;

struct Tela;
impl log::Log for Tela {
    fn enabled(&self, _: &log::Metadata) -> bool {
        true
    }
    fn log(&self, r: &log::Record) {
        eprintln!("{} {}", r.level(), r.args());
    }
    fn flush(&self) {}
}

fn main() {
    let _ = log::set_boxed_logger(Box::new(Tela)).map(|()| log::set_max_level(log::LevelFilter::Debug));
    // Vários apps em seguida (opera.exe soloist.exe opera.exe): abre e fecha um de cada vez, como a troca na tela.
    let apps: Vec<String> = std::env::args().skip(1).collect();
    for exe in if apps.is_empty() { vec!["opera.exe".to_string()] } else { apps } {
        let p = Panorama::ler();
        println!("apps com som: {:?}", p.apps());
        let Some(pid) = p.alvo(&exe) else {
            println!("{exe} não está aberto");
            continue;
        };
        println!("alvo {exe}: processo {pid}");
        let anel = Arc::new(Anel::new(16384));
        match CapturaDeApp::abrir(anel.clone(), pid) {
            Ok(c) => {
                let mut amostras = vec![0.0f32; 4096];
                for _ in 0..4 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    anel.ultimas(&mut amostras);
                    let pico = amostras.iter().fold(0.0f32, |m, v| m.max(v.abs()));
                    println!("total {} pico {pico:.3} falhou {}", anel.total(), c.falhou());
                }
            }
            Err(e) => println!("erro: {e}"),
        }
        // Força a memória a ser usada de novo: uma corrupção aparece aqui, e não minutos depois.
        let lixo: Vec<Vec<u8>> = (0..2000).map(|i| vec![i as u8; 64]).collect();
        drop(lixo);
        println!("fechou {exe} sem erro");
    }
}
