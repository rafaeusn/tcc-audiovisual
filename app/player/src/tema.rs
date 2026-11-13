//! Temas: os tokens de cor do mockup v4 (`mockup/project/app.css`), e o tom do dock.
//!
//! Regra de desenho: contêiner ganha linha (`linha`), estado ganha preenchimento (`hover`, `sel`). Cor só com
//! significado: energia, capa, confirmação.

use gpui::{Hsla, Rgba, rgb, rgba};
use nucleo::Tema;

#[derive(Clone, Copy, Debug)]
pub struct Cores {
    /// Fundo da janela.
    pub parede: Hsla,
    /// Fundo do painel, da gaveta e da barra de comandos (igual à parede, ou vidro escuro na skin imersiva).
    pub painel: Hsla,
    pub bloco: Hsla,
    pub linha: Hsla,
    pub linha_2: Hsla,
    pub texto: Hsla,
    pub texto_2: Hsla,
    pub texto_3: Hsla,
    /// Fundo invertido (botão principal) e o texto sobre ele.
    pub inv: Hsla,
    pub inv_texto: Hsla,
    pub hover: Hsla,
    pub sel: Hsla,
    /// Folha das páginas por cima do visual.
    pub folha: Hsla,
    pub escurecer: Hsla,
    pub ponto: Hsla,
    pub calma: Hsla,
    pub ativa: Hsla,
    pub intensa: Hsla,
    pub alerta: Hsla,
    /// Base do dock quando ele não segue o visual.
    pub dock: [f32; 3],
}

fn h(hex: u32) -> Hsla {
    rgb(hex).into()
}

fn ha(hex: u32) -> Hsla {
    rgba(hex).into()
}

pub fn cores(tema: Tema) -> Cores {
    let noite = Cores {
        parede: h(0x000000),
        painel: h(0x000000),
        bloco: h(0x18181b),
        linha: h(0x1f1f22),
        linha_2: h(0x2a2a2e),
        texto: h(0xf2f2f3),
        texto_2: h(0xa7a7ad),
        texto_3: h(0x808087),
        inv: h(0xf4f4f5),
        inv_texto: h(0x0b0b0c),
        hover: h(0x18181b),
        sel: h(0x222226),
        folha: ha(0x000000db),
        escurecer: ha(0x00000099),
        ponto: ha(0xffffff1a),
        calma: h(0xe9eefb),
        ativa: h(0x3d7bff),
        intensa: h(0xff3348),
        alerta: h(0xff3348),
        dock: [0.067, 0.067, 0.075],
    };
    match tema {
        Tema::Noite => noite,
        Tema::Grafite => Cores {
            parede: h(0x0f0f10),
            painel: h(0x0f0f10),
            bloco: h(0x1f1f22),
            linha: h(0x26262a),
            linha_2: h(0x323237),
            texto_2: h(0xb1b1b7),
            texto_3: h(0x8e8e95),
            hover: h(0x1f1f22),
            sel: h(0x29292d),
            folha: ha(0x0f0f10e0),
            dock: [0.098, 0.098, 0.106],
            ..noite
        },
        Tema::Papel => Cores {
            parede: h(0xecece9),
            painel: h(0xecece9),
            bloco: h(0xe1e1dd),
            linha: h(0xd4d4cf),
            linha_2: h(0xc6c6c0),
            texto: h(0x111113),
            texto_2: h(0x46464c),
            texto_3: h(0x5c5c63),
            inv: h(0x111113),
            inv_texto: h(0xf7f7f5),
            hover: h(0xe1e1dd),
            sel: h(0xd6d6d1),
            folha: ha(0xecece9e6),
            escurecer: ha(0x14141447),
            ponto: ha(0x00000024),
            calma: h(0x6b7385),
            ativa: h(0x2457e6),
            intensa: h(0xd81b38),
            alerta: h(0xd81b38),
            dock: [0.89, 0.89, 0.875],
        },
        Tema::Mono => {
            Cores { calma: h(0xf2f2f3), ativa: h(0xf2f2f3), intensa: h(0xf2f2f3), alerta: h(0xf2f2f3), ..noite }
        }
    }
}

/// Cores com a skin aplicada. Na imersiva o visual (sempre escuro) fica por baixo de tudo, então a interface usa
/// vidro escuro e texto claro em qualquer tema; do tema ficam só as cores de energia.
pub fn cores_da_skin(tema: Tema, imersivo: bool) -> Cores {
    if !imersivo {
        return cores(tema);
    }
    let base = cores(if tema == Tema::Mono { Tema::Mono } else { Tema::Noite });
    Cores {
        parede: h(0x000000),
        painel: ha(0x0a0a0cb8),
        bloco: ha(0xffffff14),
        linha: ha(0xffffff1f),
        linha_2: ha(0xffffff2e),
        hover: ha(0xffffff14),
        sel: ha(0xffffff26),
        texto_2: h(0xc4c4ca),
        texto_3: h(0xa0a0a8),
        folha: ha(0x000000a6),
        ..base
    }
}

/// Cor da energia (calma, ativa, intensa) no tema.
pub fn cor_da_energia(c: &Cores, energia: f32) -> Hsla {
    if energia < 0.42 {
        c.calma
    } else if energia < 0.74 {
        c.ativa
    } else {
        c.intensa
    }
}

/// Cor de um nível de energia que pode estar no meio do caminho entre dois (0 = calma, 1 = agitada, 2 = intensa).
/// Serve para a cor da palavra da energia trocar aos poucos em vez de pular.
pub fn cor_do_nivel(c: &Cores, nivel: f32) -> Hsla {
    let n = if nivel.is_finite() { nivel.clamp(0.0, 2.0) } else { 0.0 };
    let (de, para, k) = if n <= 1.0 { (c.calma, c.ativa, n) } else { (c.ativa, c.intensa, n - 1.0) };
    let (a, b) = (tripla(de), tripla(para));
    cor_de([0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * k))
}

fn linear(v: f32) -> f32 {
    if v <= 0.04045 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
}

fn luminancia(p: [f32; 3]) -> f32 {
    0.2126 * linear(p[0]) + 0.7152 * linear(p[1]) + 0.0722 * linear(p[2])
}

/// Tom do dock a partir da cor do visual: 30% menos saturada e escurecida até a luminância 0,016, para o texto
/// do dock manter o contraste. No tema claro, só um toque da cor sobre a base. (`toneFor` no mockup.)
pub fn tom_do_dock(cor: [f32; 3], tema: Tema) -> [f32; 3] {
    let cinza = 0.2126 * cor[0] + 0.7152 * cor[1] + 0.0722 * cor[2];
    let c = cor.map(|v| v + (cinza - v) * 0.3);
    let mistura = |base: [f32; 3], k: f32| [0, 1, 2].map(|i| base[i] + (c[i] - base[i]) * k);
    if tema == Tema::Papel {
        return mistura([0.89, 0.89, 0.875], 0.12);
    }
    let base = if tema == Tema::Grafite { [0.078, 0.078, 0.086] } else { [0.0, 0.0, 0.0] };
    let (mut lo, mut hi) = (0.0f32, 1.0f32);
    for _ in 0..12 {
        let m = (lo + hi) / 2.0;
        if luminancia(mistura(base, m)) <= 0.016 {
            lo = m;
        } else {
            hi = m;
        }
    }
    let p = mistura(base, lo);
    if tema == Tema::Mono {
        let y = luminancia(p);
        let g = if y <= 0.003_130_8 { 12.92 * y } else { 1.055 * y.powf(1.0 / 2.4) - 0.055 };
        return [g, g, g];
    }
    p
}

pub fn cor_de(c: [f32; 3]) -> Hsla {
    Rgba { r: c[0].clamp(0.0, 1.0), g: c[1].clamp(0.0, 1.0), b: c[2].clamp(0.0, 1.0), a: 1.0 }.into()
}

pub fn tripla(c: Hsla) -> [f32; 3] {
    let c = c.to_rgb();
    [c.r, c.g, c.b]
}

/// Razão de contraste WCAG entre duas cores opacas.
#[cfg(test)]
fn contraste(a: [f32; 3], b: [f32; 3]) -> f32 {
    let (la, lb) = (luminancia(a), luminancia(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn texto_tem_contraste_minimo_em_todos_os_temas() {
        for tema in Tema::TODOS {
            let c = cores(tema);
            for (nome, cor) in [("texto", c.texto), ("texto_2", c.texto_2), ("texto_3", c.texto_3)] {
                let r = contraste(tripla(cor), tripla(c.parede));
                assert!(r >= 4.5, "{tema:?}: {nome} sobre a parede tem contraste {r:.2}");
            }
            let r = contraste(tripla(c.inv_texto), tripla(c.inv));
            assert!(r >= 4.5, "{tema:?}: botão principal com contraste {r:.2}");
        }
    }

    #[test]
    fn dock_mantem_o_contraste_com_qualquer_cor_do_visual() {
        for tema in Tema::TODOS {
            let c = cores(tema);
            for cor in [[1.0, 1.0, 1.0], [0.18, 0.42, 1.0], [1.0, 0.1, 0.22], [0.0, 1.0, 0.6], [1.0, 0.7, 0.0]] {
                let tom = tom_do_dock(cor, tema);
                for (nome, texto) in [("texto", c.texto), ("texto_2", c.texto_2)] {
                    let r = contraste(tripla(texto), tom);
                    assert!(r >= 4.5, "{tema:?} com {cor:?}: {nome} no dock tem contraste {r:.2}");
                }
            }
        }
    }

    #[test]
    fn tom_escuro_fica_na_luminancia_alvo() {
        let tom = tom_do_dock([1.0, 0.1, 0.22], Tema::Noite);
        assert!(luminancia(tom) <= 0.0161);
        assert!(tom[0] > tom[1] && tom[0] > tom[2], "continua avermelhado");
        let mono = tom_do_dock([1.0, 0.1, 0.22], Tema::Mono);
        assert!((mono[0] - mono[1]).abs() < 1e-6 && (mono[1] - mono[2]).abs() < 1e-6);
    }

    #[test]
    fn cor_do_nivel_passa_pelas_tres_cores() {
        let c = cores(Tema::Noite);
        assert_eq!(tripla(cor_do_nivel(&c, 0.0)), tripla(c.calma));
        assert_eq!(tripla(cor_do_nivel(&c, 2.0)), tripla(c.intensa));
        assert_eq!(tripla(cor_do_nivel(&c, f32::NAN)), tripla(c.calma));
        let meio = tripla(cor_do_nivel(&c, 1.5));
        let (a, b) = (tripla(c.ativa), tripla(c.intensa));
        assert!((meio[0] - (a[0] + b[0]) / 2.0).abs() < 1e-4);
    }

    #[test]
    fn imersivo_usa_vidro_e_texto_claro_em_qualquer_tema() {
        for tema in Tema::TODOS {
            let c = cores_da_skin(tema, true);
            assert!(c.painel.a < 1.0, "{tema:?}: o painel imersivo é translúcido");
            assert!(luminancia(tripla(c.texto)) > 0.7, "{tema:?}: texto claro sobre o visual escuro");
            assert_eq!(tripla(cores_da_skin(tema, false).parede), tripla(cores(tema).parede));
        }
    }

    #[test]
    fn energia_escolhe_a_cor_pelo_patamar() {
        let c = cores(Tema::Noite);
        assert_eq!(cor_da_energia(&c, 0.1), c.calma);
        assert_eq!(cor_da_energia(&c, 0.6), c.ativa);
        assert_eq!(cor_da_energia(&c, 0.95), c.intensa);
    }
}
