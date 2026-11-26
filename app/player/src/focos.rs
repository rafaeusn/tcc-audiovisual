//! Os focos dos elementos que o Tab alcança, e a conta de quanto rolar a página para o focado aparecer.
//!
//! O GPUI cria sozinho o foco de um elemento com `tab_index`, mas não deixa saber qual elemento está focado. Com um
//! foco nosso por elemento (pelo id dele), o elemento sabe que é ele e diz onde foi desenhado, e a página rola até ele.

use gpui::{App, ElementId, FocusHandle};
use std::collections::HashMap;

/// Quantos quadros um foco sem uso fica guardado (o elemento pode só ter saído da tela por um instante).
const QUADROS_GUARDADO: u64 = 120;
/// Focos novos sempre à mão: eles só podem ser criados com o `App`, antes de desenhar.
const RESERVA: usize = 64;

#[derive(Default)]
pub(crate) struct Focos {
    /// O foco de cada elemento: o id e quantas vezes o mesmo id já apareceu neste quadro (ids se repetem entre listas).
    usados: HashMap<(ElementId, u32), (FocusHandle, u64)>,
    vezes: HashMap<ElementId, u32>,
    livres: Vec<FocusHandle>,
    quadro: u64,
}

impl Focos {
    /// No começo de cada quadro da janela principal: esquece os focos velhos e repõe a reserva.
    pub(crate) fn preparar(&mut self, cx: &mut App) {
        self.quadro += 1;
        self.vezes.clear();
        let quadro = self.quadro;
        self.usados.retain(|_, (_, usado)| quadro.saturating_sub(*usado) <= QUADROS_GUARDADO);
        while self.livres.len() < RESERVA {
            self.livres.push(cx.focus_handle().tab_index(0).tab_stop(true));
        }
    }

    /// O foco do elemento `id` (o mesmo nos quadros seguintes). `None` se a reserva acabou neste quadro.
    pub(crate) fn pegar(&mut self, id: ElementId) -> Option<FocusHandle> {
        let vez = self.vezes.entry(id.clone()).or_default();
        let chave = (id, *vez);
        *vez += 1;
        let quadro = self.quadro;
        if let Some((foco, usado)) = self.usados.get_mut(&chave) {
            *usado = quadro;
            return Some(foco.clone());
        }
        let foco = self.livres.pop()?;
        self.usados.insert(chave, (foco.clone(), quadro));
        Some(foco)
    }
}

/// O novo deslocamento da rolagem (negativo para baixo, como no GPUI) para o elemento entre `topo` e `base` aparecer
/// na janela de rolagem entre `visto_topo` e `visto_base`, com `folga` de sobra. `None` se ele já aparece.
pub(crate) fn rolar_para_ver(
    (topo, base): (f32, f32),
    (visto_topo, visto_base): (f32, f32),
    atual: f32,
    maximo: f32,
    folga: f32,
) -> Option<f32> {
    let novo = if base + folga > visto_base && topo - folga > visto_topo {
        // Abaixo da vista: sobe o conteúdo até a base dele aparecer (sem passar do topo dele).
        atual - (base + folga - visto_base).min(topo - folga - visto_topo)
    } else if topo - folga < visto_topo {
        atual + (visto_topo - (topo - folga))
    } else {
        return None;
    };
    let novo = novo.clamp(-maximo.max(0.0), 0.0);
    ((novo - atual).abs() >= 0.5).then_some(novo)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn rola_so_o_preciso() {
        // Vista de 100 a 500; o elemento já aparece.
        assert_eq!(rolar_para_ver((200.0, 260.0), (100.0, 500.0), 0.0, 1000.0, 16.0), None);
        // Abaixo: a base dele (600) + folga vai até a base da vista.
        assert_eq!(rolar_para_ver((540.0, 600.0), (100.0, 500.0), 0.0, 1000.0, 16.0), Some(-116.0));
        // Acima: o topo dele (40) − folga vai até o topo da vista.
        assert_eq!(rolar_para_ver((40.0, 90.0), (100.0, 500.0), -300.0, 1000.0, 16.0), Some(-224.0));
    }

    #[test]
    fn nao_passa_das_pontas() {
        assert_eq!(rolar_para_ver((540.0, 600.0), (100.0, 500.0), 0.0, 50.0, 16.0), Some(-50.0));
        assert_eq!(rolar_para_ver((40.0, 90.0), (100.0, 500.0), -20.0, 1000.0, 16.0), Some(0.0));
        // No topo, um elemento acima não tem para onde ir.
        assert_eq!(rolar_para_ver((40.0, 90.0), (100.0, 500.0), 0.0, 1000.0, 16.0), None);
    }

    #[test]
    fn elemento_maior_que_a_vista_mostra_o_topo() {
        // 900 px de altura numa vista de 400: o topo dele fica à vista.
        assert_eq!(rolar_para_ver((600.0, 1500.0), (100.0, 500.0), 0.0, 2000.0, 16.0), Some(-484.0));
    }
}
