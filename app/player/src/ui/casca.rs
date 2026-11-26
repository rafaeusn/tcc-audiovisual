//! O topo da janela: voltar e avançar, a busca, a situação do Modo Escuta, o modo edição e os botões da janela.

use super::{caixa, icone, medidas};
use crate::app::Player;
use crate::arranjo::Modo;
use crate::escuta::Situacao;
use crate::navegacao::Campo;
use crate::ui::medidas::px;
use gpui::{AnyElement, Context, Div, FontWeight, MouseButton, SharedString, WindowControlArea, div, prelude::*};
use nucleo::Comando;

/// A largura dos botões da janela (minimizar, maximizar, fechar).
const CONTROLES: f32 = 3.0 * 46.0;
/// O que sempre sobra no topo para arrastar a janela.
const ARRASTE: f32 = 48.0;
/// A busca não fica mais larga que isto, e mais estreita que isto vira o campo da própria página.
const BUSCA: f32 = 560.0;
const BUSCA_MINIMA: f32 = 200.0;

/// A largura do campo do topo (`None` quando não cabe): o que sobra das setas, da situação e dos botões da janela.
fn largura_da_busca(topo: f32) -> Option<f32> {
    let setas = if topo >= DUAS_SETAS { 2.0 * medidas::ALVO } else { medidas::ALVO };
    let situacao = if topo >= SITUACAO_INTEIRA {
        260.0
    } else if topo >= SITUACAO_CURTA {
        180.0
    } else {
        0.0
    };
    let sobra = topo - setas - situacao - ARRASTE - CONTROLES - 24.0;
    (sobra >= BUSCA_MINIMA).then(|| sobra.min(BUSCA).floor())
}

/// A partir de que largura do topo cabem as duas setas, a situação só com o estado e a situação com o nome da saída.
const DUAS_SETAS: f32 = 480.0;
const SITUACAO_CURTA: f32 = 760.0;
const SITUACAO_INTEIRA: f32 = 1000.0;

/// O sistema descreve a saída com detalhes que não cabem no topo ("Alto-falantes (Realtek) [Speaker] via ...").
/// Fica só o começo, até o primeiro colchete.
fn nome_curto(dispositivo: &str) -> &str {
    let curto = dispositivo.split(" [").next().unwrap_or(dispositivo).trim();
    if curto.is_empty() { dispositivo } else { curto }
}

impl Player {
    /// O campo da busca está no topo? (Sem lugar para ele, a página Buscar o mostra.)
    pub(crate) fn campo_no_topo(&self) -> bool {
        !self.edicao && largura_da_busca(self.arranjo.topo.w).is_some()
    }

    /// Topo: voltar e avançar, a busca, a situação do Modo Escuta (na área de arrastar a janela) e os botões da janela.
    /// No modo edição a busca dá lugar aos controles da edição.
    pub(crate) fn ver_topo(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let largura = a.topo.w;
        let curto = largura < SITUACAO_INTEIRA;
        let (ponto, frase): (gpui::Hsla, SharedString) = match &self.situacao {
            Situacao::Iniciando => (c.texto_3, "Iniciando o Modo Escuta".into()),
            Situacao::Ouvindo { dispositivo } => {
                let som = self.retrato.gate > 0.5;
                // Em janela estreita o nome da saída não cabe: fica só o estado.
                let frase =
                    if curto { "Modo Escuta".into() } else { format!("Ouvindo {}", nome_curto(dispositivo)).into() };
                (if som { c.calma } else { c.texto_3 }, frase)
            }
            Situacao::EsperandoApp { app } if curto => (c.texto_3, format!("Esperando {}", nome_curto(app)).into()),
            Situacao::EsperandoApp { app } => (c.texto_3, format!("Esperando {app} tocar").into()),
            Situacao::SemAudio { .. } if curto => (c.alerta, "Sem saída de áudio".into()),
            Situacao::SemAudio { .. } => (c.alerta, "Sem saída de áudio. Tentando de novo...".into()),
        };
        let controle = |nome: &'static str, area: WindowControlArea, caminho: &'static str, fechar: bool| {
            div()
                .id(nome)
                .w(px(46.0))
                .h(px(medidas::FICHA))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(super::RAIO_CONTROLE))
                .hover(|s| if fechar { s.bg(c.alerta) } else { s.bg(c.hover) })
                .window_control_area(area)
                .child(icone(caminho, medidas::ICONE_LINHA, c.texto_2))
        };
        // Com a busca e sem lugar para a frase, fica o ponto (a cor dele diz se há som).
        let busca = largura_da_busca(largura);
        let com_frase = largura >= SITUACAO_CURTA || busca.is_none();
        let situacao = div()
            .h(px(medidas::FICHA))
            .max_w_full()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .when(com_frase, |d| {
                d.px(px(16.0))
                    .rounded(px(medidas::PILULA))
                    .border_1()
                    .border_color(c.linha_2)
                    .when(self.imersivo(), |d| d.bg(c.painel))
            })
            .child(div().size(px(8.0)).flex_none().rounded(px(medidas::PILULA)).bg(ponto))
            .when(com_frase, |d| {
                d.child(
                    medidas::apoio()
                        .min_w(px(0.0))
                        .text_color(c.texto_2)
                        .font_weight(FontWeight::MEDIUM)
                        .child(super::linha(frase)),
                )
            });

        // Uma página atrás ou à frente, como no navegador.
        let estante = self.estante();
        let seta = |id: &'static str, caminho: &'static str, pode: bool, comando: Comando, cx: &mut Context<Self>| {
            self.botao_icone(id, caminho, false, comando, cx).opacity(if pode { 1.0 } else { 0.4 })
        };
        let pode = (self.nav.pode_voltar(estante), self.nav.pode_avancar(estante));
        let setas = div()
            .flex()
            .flex_none()
            .child(seta("pagina-voltar", "icones/chev-l.svg", pode.0, Comando::Voltar, cx))
            .when(largura >= DUAS_SETAS, |d| {
                d.child(seta("pagina-avancar", "icones/chev-r.svg", pode.1, Comando::Avancar, cx))
            });
        // O meio: a busca ou os controles da edição.
        let meio: Option<AnyElement> = match busca {
            _ if self.edicao => Some(self.ver_barra_de_edicao(cx).into_any_element()),
            Some(w) => Some(
                div()
                    .w(px(w))
                    .flex()
                    .flex_none()
                    .child(self.campo_de_texto(Campo::Busca, "Buscar músicas ou comandos", cx))
                    .into_any_element(),
            ),
            // Sem lugar para ele, o campo fica na página Buscar (que está na barra de baixo).
            None => None,
        };
        caixa(a.topo)
            // Sem isto, o foco do teclado (na raiz) marca o clique como tratado e o Windows não recebe o clique
            // nos botões da janela nem o arrasto.
            .occlude()
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(setas)
            .children(meio)
            .child(
                // Tudo o que não é botão arrasta a janela.
                div()
                    .flex_1()
                    .flex_basis(px(0.0))
                    .h_full()
                    .min_w(px(0.0))
                    .pl(px(4.0))
                    .flex()
                    .items_center()
                    .window_control_area(WindowControlArea::Drag)
                    .child(situacao),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .child(controle("Minimizar", WindowControlArea::Min, "icones/min.svg", false))
                    .child(controle("Maximizar", WindowControlArea::Max, "icones/max.svg", false))
                    .child(controle("Fechar", WindowControlArea::Close, "icones/x.svg", true)),
            )
    }

    /// Controles do modo edição, no lugar da busca: de que lado fica o painel e o botão de concluir.
    fn ver_barra_de_edicao(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let pilula = |id: &'static str| {
            medidas::corpo()
                .id(id)
                .h(px(medidas::BOTAO))
                .px(px(16.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .rounded(px(medidas::PILULA))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .when(a.topo.w >= 900.0, |d| {
                d.child(
                    medidas::corpo()
                        .h(px(medidas::BOTAO))
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(medidas::PILULA))
                        .bg(c.sel)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(icone("icones/grid.svg", medidas::ICONE_LINHA, c.texto))
                        .child("Modo edição"),
                )
            })
            .when(a.modo != Modo::Empilhado, |d| {
                d.child(
                    pilula("edicao-lado")
                        .border_1()
                        .border_color(c.linha_2)
                        .text_color(c.texto_2)
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/swap.svg", medidas::ICONE_LINHA, c.texto_2))
                        .child("Trocar de lado")
                        .on_mouse_down(MouseButton::Left, self.ao_apertar("edicao-lado", cx))
                        .on_click(self.ao_clicar(Comando::TrocarLado, cx)),
                )
            })
            .child(
                pilula("edicao-concluir")
                    .bg(c.inv)
                    .text_color(c.inv_texto)
                    .hover(|s| s.opacity(0.9))
                    .child("Concluir")
                    .on_click(self.ao_clicar(Comando::EditarLayout, cx)),
            )
    }
}

#[cfg(test)]
mod testes {
    use super::{BUSCA, BUSCA_MINIMA, largura_da_busca, nome_curto};

    #[test]
    fn a_busca_cabe_no_topo_com_o_resto() {
        assert_eq!(largura_da_busca(1900.0), Some(BUSCA));
        for topo in [500.0, 676.0, 760.0, 846.0, 1000.0, 1160.0] {
            let busca = largura_da_busca(topo).unwrap_or_default();
            assert!((BUSCA_MINIMA..=BUSCA).contains(&busca), "{topo}: {busca}");
            assert!(busca + super::CONTROLES + super::ARRASTE < topo, "{topo}: sobra para arrastar");
        }
        assert_eq!(largura_da_busca(386.0), None, "estreito demais: o campo fica na página");
    }

    #[test]
    fn nome_da_saida_fica_curto() {
        assert_eq!(
            nome_curto("Alto-falantes (Realtek(R) Audio) [Speaker] via S/PDIF"),
            "Alto-falantes (Realtek(R) Audio)"
        );
        assert_eq!(nome_curto("Fones"), "Fones");
        assert_eq!(nome_curto(" [x]"), " [x]");
        assert_eq!(nome_curto(""), "");
    }
}
