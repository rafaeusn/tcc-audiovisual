//! As cores de um espaço: da cor da marca do serviço sai uma paleta discreta (a folha só tingida, o filete, o degradê
//! do topo e o acento), sempre com contraste para o texto. Só contas, testadas.

use crate::tema::{Cores, contraste, cor_de, luminancia, tripla};
use crate::ui::{com_alfa, misturar};
use gpui::Hsla;

/// A cor do espaço "Neste computador" (nossa, não de um serviço).
pub const COR_DO_COMPUTADOR: u32 = 0xF2B84B;
/// Quanto da cor do serviço tinge a folha e o filete, e o alfa do degradê do topo.
const TINTA_DA_FOLHA: f32 = 0.07;
const TINTA_DO_FILETE: f32 = 0.18;
const ALFA_DO_TOPO: f32 = 0.22;
/// O acento precisa deste contraste com a folha (texto grande e ícones).
const CONTRASTE_DO_ACENTO: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaletaDoEspaco {
    /// Botão principal, aba ativa, "tocando agora".
    pub acento: Hsla,
    /// O texto sobre o acento (preto ou branco).
    pub texto_no_acento: Hsla,
    pub folha: Hsla,
    pub filete: Hsla,
    /// O começo do degradê do topo das páginas (já com o alfa); vai até o transparente.
    pub topo: Hsla,
}

pub fn rgb(cor: u32) -> [f32; 3] {
    [(cor >> 16) & 0xFF, (cor >> 8) & 0xFF, cor & 0xFF].map(|c| c as f32 / 255.0)
}

/// Tinge `base` com `cor`, mantendo o alfa de `base` (a folha é translúcida sobre o visual).
fn tingir(base: Hsla, cor: [f32; 3], quanto: f32) -> Hsla {
    Hsla { a: base.a, ..misturar(Hsla { a: 1.0, ..base }, cor_de(cor), quanto) }
}

/// A paleta de um serviço de cor `cor` sobre o tema `base`.
pub fn paleta(cor: [f32; 3], base: &Cores) -> PaletaDoEspaco {
    let folha = tingir(base.folha, cor, TINTA_DA_FOLHA);
    let fundo = tripla(folha);
    // No escuro o acento clareia até ter contraste; no tema claro, escurece.
    let alvo = if luminancia(fundo) < 0.2 { [1.0; 3] } else { [0.0; 3] };
    let mut acento = cor;
    for passo in 0..=20 {
        let k = passo as f32 / 20.0;
        acento = [0, 1, 2].map(|i| cor[i] + (alvo[i] - cor[i]) * k);
        if contraste(acento, fundo) >= CONTRASTE_DO_ACENTO {
            break;
        }
    }
    let (preto, branco) = ([0.0; 3], [1.0; 3]);
    let texto = if contraste(preto, acento) >= contraste(branco, acento) { preto } else { branco };
    PaletaDoEspaco {
        acento: cor_de(acento),
        texto_no_acento: cor_de(texto),
        folha,
        filete: tingir(base.linha, cor, TINTA_DO_FILETE),
        topo: com_alfa(cor_de(cor), ALFA_DO_TOPO),
    }
}

/// A paleta de Tudo: as cores do tema, sem tinta.
pub fn neutra(base: &Cores) -> PaletaDoEspaco {
    PaletaDoEspaco {
        acento: base.inv,
        texto_no_acento: base.inv_texto,
        folha: base.folha,
        filete: base.linha,
        topo: com_alfa(base.folha, 0.0),
    }
}

/// Uma cor para o serviço que não mandou a dele: o matiz sai do nome (sempre o mesmo para o mesmo nome).
pub fn cor_do_nome(servico: &str) -> [f32; 3] {
    // FNV-1a: estável entre versões e aberturas (o `Hash` do Rust não promete isso).
    let mut h: u32 = 0x811c_9dc5;
    for b in servico.bytes() {
        h ^= u32::from(b);
        h = h.wrapping_mul(0x0100_0193);
    }
    tripla(Hsla { h: (h % 360) as f32 / 360.0, s: 0.65, l: 0.55, a: 1.0 })
}

/// A paleta no meio da transição de `a` para `b` (0 = `a`, 1 = `b`).
pub fn entre(a: &PaletaDoEspaco, b: &PaletaDoEspaco, t: f32) -> PaletaDoEspaco {
    if t <= 0.0 {
        return *a;
    }
    if t >= 1.0 {
        return *b;
    }
    PaletaDoEspaco {
        acento: misturar(a.acento, b.acento, t),
        texto_no_acento: if t < 0.5 { a.texto_no_acento } else { b.texto_no_acento },
        folha: misturar(a.folha, b.folha, t),
        filete: misturar(a.filete, b.filete, t),
        topo: misturar(a.topo, b.topo, t),
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::tema::{contraste, tripla};

    #[test]
    fn acento_tem_contraste_em_todos_os_temas() {
        for tema in nucleo::Tema::TODOS {
            let base = crate::tema::cores(tema);
            for cor in [
                rgb(0xFF0000),
                rgb(0x1ED760),
                rgb(0x0A0A0A),
                rgb(0xFFFFFF),
                rgb(COR_DO_COMPUTADOR),
                cor_do_nome("soundcloud"),
            ] {
                let p = paleta(cor, &base);
                let r = contraste(tripla(p.acento), tripla(p.folha));
                assert!(r >= 3.0, "{tema:?} {cor:?}: acento com contraste {r:.2}");
                let t = contraste(tripla(p.texto_no_acento), tripla(p.acento));
                assert!(t >= 4.5, "{tema:?} {cor:?}: texto no acento com {t:.2}");
                let texto = contraste(tripla(base.texto_2), tripla(p.folha));
                assert!(texto >= 4.5, "{tema:?} {cor:?}: texto secundário sobre a folha tingida com {texto:.2}");
            }
        }
    }

    #[test]
    fn cor_do_nome_e_estavel_e_muda_com_o_nome() {
        assert_eq!(cor_do_nome("soundcloud"), cor_do_nome("soundcloud"));
        assert_ne!(cor_do_nome("soundcloud"), cor_do_nome("deezer"));
    }

    #[test]
    fn transicao_vai_de_uma_a_outra() {
        let base = crate::tema::cores(nucleo::Tema::Noite);
        let (a, b) = (neutra(&base), paleta(rgb(0xFF0000), &base));
        assert_eq!(entre(&a, &b, 0.0), a);
        assert_eq!(entre(&a, &b, 1.0), b);
    }
}
