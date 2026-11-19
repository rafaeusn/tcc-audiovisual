//! O traço da Ilha: o corpo com os cantos arredondados, os ombros que o ligam à borda da tela (presa) e o recorte do
//! visual.

use super::*;

/// Um retângulo com cantos arredondados (para o recorte do visual), em coordenadas da janela.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Recorte {
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) w: f32,
    pub(super) h: f32,
    pub(super) raio: f32,
}

impl Recorte {
    /// O mesmo retângulo, `d` pixels maior de cada lado (com o canto acompanhando).
    pub(super) fn inflado(self, d: f32) -> Self {
        Self { x: self.x - d, y: self.y - d, w: self.w + 2.0 * d, h: self.h + 2.0 * d, raio: self.raio + d }
    }
}

/// Fração das alças de uma curva cúbica que imita um quarto de círculo.
const K: f32 = 0.552_3;

/// O traço da Ilha com o corpo `corpo` (na janela) e os cantos de `raios` (cima-esquerda, cima-direita, baixo-direita,
/// baixo-esquerda). Presa (`lado`, com ombros), os cantos do lado da borda dão lugar aos ombros, curvas côncavas que
/// encostam na borda da tela; solta, os quatro cantos são redondos. Com `furo`, deixa um recorte para o visual.
pub(super) fn forma(
    origem: Point<Pixels>,
    corpo: Caixa,
    raios: [f32; 4],
    lado: Option<BordaIlha>,
    ombro: f32,
    furo: Option<Recorte>,
) -> Option<Path<Pixels>> {
    let (w, h) = (corpo.w.max(1.0), corpo.h.max(1.0));
    let maximo = (w / 2.0).min(h / 2.0);
    let r = raios.map(|v| if v.is_finite() { v.clamp(0.0, maximo) } else { 0.0 });
    let p = |x: f32, y: f32| point(origem.x + px(x), origem.y + px(y));
    let mut b = PathBuilder::fill();
    match lado.filter(|_| ombro > 0.5) {
        Some(lado) => {
            // Presa: o traço é o do topo (comprimento ao longo da borda, fundo para dentro da tela), girado ou
            // espelhado para o lado da borda.
            let (comprido, fundo) = if lado.de_pe() { (h, w) } else { (w, h) };
            let longe = match lado {
                BordaIlha::Topo => r[2].max(r[3]),
                BordaIlha::Baixo => r[0].max(r[1]),
                BordaIlha::Esquerda => r[1].max(r[2]),
                BordaIlha::Direita => r[0].max(r[3]),
            }
            .min((comprido / 2.0).min(fundo / 2.0));
            let e = ombro.clamp(0.0, (fundo - longe).max(0.0));
            let m = |u: f32, v: f32| {
                let (x, y) = match lado {
                    BordaIlha::Topo => (corpo.x + u, corpo.y + v),
                    BordaIlha::Baixo => (corpo.x + u, corpo.y + h - v),
                    BordaIlha::Esquerda => (corpo.x + v, corpo.y + u),
                    BordaIlha::Direita => (corpo.x + w - v, corpo.y + u),
                };
                p(x, y)
            };
            // Passa 2 px da borda, para encostar de verdade na borda da tela.
            let fora = -2.0;
            let (c, f) = (comprido, fundo);
            b.move_to(m(-e, fora));
            b.line_to(m(c + e, fora));
            b.line_to(m(c + e, 0.0));
            b.curve_to(m(c, e), m(c, 0.0));
            b.line_to(m(c, f - longe));
            b.cubic_bezier_to(m(c - longe, f), m(c, f - longe + K * longe), m(c - longe + K * longe, f));
            b.line_to(m(longe, f));
            b.cubic_bezier_to(m(0.0, f - longe), m(longe - K * longe, f), m(0.0, f - longe + K * longe));
            b.line_to(m(0.0, e));
            b.curve_to(m(-e, 0.0), m(0.0, 0.0));
            b.close();
        }
        None => {
            // Solta (ou largando a borda, já sem os ombros): os quatro cantos redondos, cada um com o seu raio.
            let (x, y, x1, y1) = (corpo.x, corpo.y, corpo.x + w, corpo.y + h);
            b.move_to(p(x + r[0], y));
            b.line_to(p(x1 - r[1], y));
            b.cubic_bezier_to(p(x1, y + r[1]), p(x1 - r[1] + K * r[1], y), p(x1, y + r[1] - K * r[1]));
            b.line_to(p(x1, y1 - r[2]));
            b.cubic_bezier_to(p(x1 - r[2], y1), p(x1, y1 - r[2] + K * r[2]), p(x1 - r[2] + K * r[2], y1));
            b.line_to(p(x + r[3], y1));
            b.cubic_bezier_to(p(x, y1 - r[3]), p(x + r[3] - K * r[3], y1), p(x, y1 - r[3] + K * r[3]));
            b.line_to(p(x, y + r[0]));
            b.cubic_bezier_to(p(x + r[0], y), p(x, y + r[0] - K * r[0]), p(x + r[0] - K * r[0], y));
            b.close();
        }
    }
    if let Some(f) = furo.filter(|f| f.w > 1.0 && f.h > 1.0) {
        // O recorte: a forma fica vazada ali (e o visual, embaixo, aparece).
        let r = f.raio.clamp(0.0, (f.w / 2.0).min(f.h / 2.0));
        let (x, y, x1, y1) = (f.x, f.y, f.x + f.w, f.y + f.h);
        b.move_to(p(x + r, y));
        b.cubic_bezier_to(p(x, y + r), p(x + r - K * r, y), p(x, y + r - K * r));
        b.line_to(p(x, y1 - r));
        b.cubic_bezier_to(p(x + r, y1), p(x, y1 - r + K * r), p(x + r - K * r, y1));
        b.line_to(p(x1 - r, y1));
        b.cubic_bezier_to(p(x1, y1 - r), p(x1 - r + K * r, y1), p(x1, y1 - r + K * r));
        b.line_to(p(x1, y + r));
        b.cubic_bezier_to(p(x1 - r, y), p(x1, y + r - K * r), p(x1 - r + K * r, y));
        b.close();
    }
    b.build().ok()
}
