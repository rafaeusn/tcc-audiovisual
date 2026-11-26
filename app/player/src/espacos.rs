//! A cor de um serviço, para a marca dele. Ela só aparece no selo e no filtro: as telas têm as cores do tema, e a cor
//! grande é da energia da música.

use crate::tema::tripla;
use gpui::Hsla;

pub fn rgb(cor: u32) -> [f32; 3] {
    [(cor >> 16) & 0xFF, (cor >> 8) & 0xFF, cor & 0xFF].map(|c| c as f32 / 255.0)
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

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn cor_do_nome_e_estavel_e_muda_com_o_nome() {
        assert_eq!(cor_do_nome("soundcloud"), cor_do_nome("soundcloud"));
        assert_ne!(cor_do_nome("soundcloud"), cor_do_nome("deezer"));
        assert_eq!(rgb(0xFF0000), [1.0, 0.0, 0.0]);
    }
}
