//! A caixa do menu do botão direito e do ⋯, por cima de tudo. Clicar fora fecha; as setas e o Enter também valem.

use super::{icone, medidas};
use crate::app::Player;
use crate::menu::{Alvo, MARGEM, lugar};
use crate::ui::medidas::px;
use gpui::{App, ClickEvent, Context, MouseButton, MouseDownEvent, Stateful, Window, div, prelude::*};

const LARGURA: f32 = 280.0;
const LINHA: f32 = 40.0;
/// O espaço de cada traço entre os grupos, e a folga em cima e embaixo.
const TRACO: f32 = 13.0;
const FOLGA: f32 = 6.0;

impl Player {
    /// Abre o menu de `alvo` onde o botão direito foi apertado.
    pub(super) fn ao_menu(
        &self,
        alvo: impl Fn() -> Alvo + 'static,
        cx: &Context<Self>,
    ) -> impl Fn(&MouseDownEvent, &mut Window, &mut App) + 'static {
        cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
            // O de dentro vale: o menu da linha, e não o do que está atrás dela.
            cx.stop_propagation();
            this.abrir_menu(alvo(), ev.position);
            cx.notify();
        })
    }

    /// O ⋯: o mesmo menu, com o botão esquerdo.
    pub(super) fn botao_mais(
        &self,
        id: (&'static str, usize),
        lado: f32,
        alvo: impl Fn() -> Alvo + 'static,
        cx: &Context<Self>,
    ) -> Stateful<gpui::Div> {
        let c = self.cores();
        div()
            .id(id)
            .map(|d| self.focavel(d, medidas::PILULA))
            .size(px(lado))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .rounded(px(medidas::PILULA))
            .cursor_pointer()
            .hover(|s| s.bg(super::com_alfa(c.texto, 0.1)))
            .child(icone("icones/mais.svg", medidas::ICONE_LINHA, c.texto_2))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.abrir_menu(alvo(), ev.position());
                cx.notify();
            }))
    }

    pub(crate) fn ver_menu(&self, cx: &mut Context<Self>) -> Option<Stateful<gpui::Div>> {
        let menu = self.menu.as_ref()?;
        let c = self.cores();
        let j = &self.arranjo.janela;
        let tracos = menu.entradas.windows(2).filter(|e| e[0].grupo != e[1].grupo).count();
        let altura = menu.entradas.len() as f32 * LINHA + tracos as f32 * TRACO + 2.0 * FOLGA + 2.0;
        let altura = altura.min(j.h - 2.0 * MARGEM);
        let ponto = (medidas::logico(menu.ponto.x), medidas::logico(menu.ponto.y));
        let (x, y) = lugar(ponto, (LARGURA, altura), (j.w, j.h));
        let p = self.anim.seguir(("menu", 0), 1.0, 0.1);
        let mut lista = div()
            .id("menu")
            .absolute()
            .left(px(x))
            .top(px(y))
            .w(px(LARGURA))
            .h(px(altura))
            .opacity(p)
            .py(px(FOLGA))
            .px(px(FOLGA))
            .rounded(px(medidas::RAIO_CARTAO))
            .bg(c.bloco)
            .border_1()
            .border_color(c.linha_2)
            .shadow_lg()
            .overflow_y_scroll()
            .occlude()
            .flex()
            .flex_col()
            // Clicar dentro (entre as linhas) não fecha.
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation());
        let mut grupo = menu.entradas.first().map_or(0, |e| e.grupo);
        for (i, e) in menu.entradas.iter().enumerate() {
            if e.grupo != grupo {
                grupo = e.grupo;
                lista = lista.child(div().flex_none().h(px(1.0)).my(px((TRACO - 1.0) / 2.0)).mx(px(8.0)).bg(c.linha_2));
            }
            let escolhida = menu.escolhida == Some(i);
            let acao = e.acao.clone();
            lista = lista.child(
                medidas::apoio()
                    .id(("menu-linha", i))
                    .h(px(LINHA))
                    .px(px(10.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(12.0))
                    .rounded(px(medidas::RAIO_CONTROLE))
                    .cursor_pointer()
                    .when(escolhida, |d| d.bg(c.sel))
                    .hover(|s| s.bg(c.sel))
                    .child(icone(e.icone, medidas::ICONE_LINHA, c.texto_2))
                    .child(div().flex_1().min_w(px(0.0)).text_color(c.texto).child(super::linha(e.rotulo.clone())))
                    .when(e.marcada, |d| d.child(icone("icones/check.svg", medidas::ICONE_LINHA, c.texto)))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.executar_do_menu(acao.clone(), window, cx);
                        cx.notify();
                    })),
            );
        }
        Some(
            div()
                .id("menu-fundo")
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .size_full()
                .occlude()
                .on_any_mouse_down(cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    this.menu = None;
                    cx.notify();
                }))
                .child(lista),
        )
    }
}
