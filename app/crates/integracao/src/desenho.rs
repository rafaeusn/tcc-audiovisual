//! Os ícones pequenos (barra de tarefas e bandeja), desenhados na hora. Assim o app não depende de arquivos de
//! ícone e as formas ficam nítidas em qualquer escala de tela.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forma {
    Anterior,
    Tocar,
    Pausar,
    Proxima,
    /// O ícone do app na bandeja: um anel com um ponto no meio.
    App,
}

fn no_triangulo(p: (f32, f32), a: (f32, f32), b: (f32, f32), c: (f32, f32)) -> bool {
    let lado = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| (p.0 - r.0) * (q.1 - r.1) - (q.0 - r.0) * (p.1 - r.1);
    let (d1, d2, d3) = (lado(p, a, b), lado(p, b, c), lado(p, c, a));
    let negativo = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let positivo = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    !(negativo && positivo)
}

fn no_retangulo(p: (f32, f32), x0: f32, x1: f32) -> bool {
    (x0..=x1).contains(&p.0) && (0.22..=0.78).contains(&p.1)
}

/// O ponto (x, y), de 0 a 1, faz parte da forma?
fn dentro(forma: Forma, x: f32, y: f32) -> bool {
    let p = (x, y);
    match forma {
        Forma::Tocar => no_triangulo(p, (0.30, 0.20), (0.30, 0.80), (0.80, 0.50)),
        Forma::Pausar => no_retangulo(p, 0.27, 0.43) || no_retangulo(p, 0.57, 0.73),
        Forma::Proxima => no_triangulo(p, (0.22, 0.22), (0.22, 0.78), (0.64, 0.50)) || no_retangulo(p, 0.68, 0.79),
        Forma::Anterior => dentro(Forma::Proxima, 1.0 - x, y),
        Forma::App => {
            let d = ((x - 0.5).powi(2) + (y - 0.5).powi(2)).sqrt();
            d <= 0.15 || (0.30..=0.44).contains(&d)
        }
    }
}

/// Pixels BGRA (branco, com a transparência nas bordas) de um ícone quadrado de `lado` pixels.
pub fn desenhar(forma: Forma, lado: usize) -> Vec<u8> {
    const AMOSTRAS: usize = 4;
    let mut pixels = Vec::with_capacity(lado * lado * 4);
    for py in 0..lado {
        for px in 0..lado {
            let mut cobertos = 0;
            for sy in 0..AMOSTRAS {
                for sx in 0..AMOSTRAS {
                    let x = (px as f32 + (sx as f32 + 0.5) / AMOSTRAS as f32) / lado as f32;
                    let y = (py as f32 + (sy as f32 + 0.5) / AMOSTRAS as f32) / lado as f32;
                    cobertos += usize::from(dentro(forma, x, y));
                }
            }
            let alfa = (cobertos * 255 / (AMOSTRAS * AMOSTRAS)) as u8;
            pixels.extend_from_slice(&[255, 255, 255, alfa]);
        }
    }
    pixels
}

#[cfg(test)]
mod testes {
    use super::*;

    fn alfa(pixels: &[u8], lado: usize, x: usize, y: usize) -> u8 {
        pixels[(y * lado + x) * 4 + 3]
    }

    #[test]
    fn formas_ficam_no_meio_e_os_cantos_vazios() {
        for forma in [Forma::Anterior, Forma::Tocar, Forma::Pausar, Forma::Proxima, Forma::App] {
            let p = desenhar(forma, 32);
            assert_eq!(p.len(), 32 * 32 * 4);
            for (x, y) in [(0, 0), (31, 0), (0, 31), (31, 31)] {
                assert_eq!(alfa(&p, 32, x, y), 0, "{forma:?} canto {x},{y}");
            }
            let cheios = p.chunks(4).filter(|c| c[3] > 0).count();
            assert!((60..700).contains(&cheios), "{forma:?} tem {cheios} pixels");
        }
    }

    #[test]
    fn cada_forma_e_o_que_diz() {
        let tocar = desenhar(Forma::Tocar, 32);
        assert_eq!(alfa(&tocar, 32, 14, 16), 255, "o meio do triângulo");
        assert_eq!(alfa(&tocar, 32, 27, 6), 0);
        let pausar = desenhar(Forma::Pausar, 32);
        assert_eq!(alfa(&pausar, 32, 11, 16), 255);
        assert_eq!(alfa(&pausar, 32, 16, 16), 0, "o vão entre as barras");
        assert_eq!(alfa(&pausar, 32, 20, 16), 255);
        // Anterior é a próxima espelhada.
        let (anterior, proxima) = (desenhar(Forma::Anterior, 32), desenhar(Forma::Proxima, 32));
        for y in 0..32 {
            for x in 0..32 {
                assert_eq!(alfa(&anterior, 32, x, y), alfa(&proxima, 32, 31 - x, y));
            }
        }
        let app = desenhar(Forma::App, 32);
        assert_eq!(alfa(&app, 32, 16, 16), 255, "o ponto do meio");
        assert_eq!(alfa(&app, 32, 16, 9), 0, "entre o ponto e o anel");
        assert_eq!(alfa(&app, 32, 16, 4), 255, "o anel");
    }
}
