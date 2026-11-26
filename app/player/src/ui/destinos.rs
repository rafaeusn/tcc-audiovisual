//! Os destinos (Início, Buscar, Biblioteca; e Loja e Configurações): a mesma lista na lateral da Estante, no trilho com
//! nomes e na barra de baixo das janelas estreitas, nos dois layouts. E as abas sublinhadas das páginas.

use super::{RAIO_MODULO, caixa, icone, medidas};
use crate::app::Player;
use crate::arranjo::Ret;
use crate::navegacao::Rota;
use crate::ui::medidas::px;
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, MouseButton, Stateful, div, prelude::*};

/// Um destino: a página, o nome, o nome curto (embaixo do ícone), o ícone e o ícone da página aberta.
struct Destino {
    id: &'static str,
    rota: Rota,
    nome: &'static str,
    curto: &'static str,
    icone: &'static str,
    cheio: &'static str,
}

fn destino(
    id: &'static str,
    rota: Rota,
    nome: &'static str,
    curto: &'static str,
    icone: &'static str,
    cheio: &'static str,
) -> Destino {
    Destino { id, rota, nome, curto, icone, cheio }
}

/// Como um destino aparece.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Forma {
    /// Ícone e nome na mesma linha (a lateral).
    Linha,
    /// Nome embaixo do ícone (o trilho e a barra de baixo).
    Pilha,
}

impl Player {
    /// Os três destinos da música.
    fn destinos(&self) -> [Destino; 3] {
        [
            destino("ir-inicio", Rota::Inicio, "Início", "Início", "icones/home.svg", "icones/home-f.svg"),
            destino("ir-buscar", Rota::Busca, "Buscar", "Buscar", "icones/search.svg", "icones/search.svg"),
            destino(
                "ir-biblioteca",
                Rota::Biblioteca(self.aba_lembrada),
                "Biblioteca",
                "Biblioteca",
                "icones/lib.svg",
                "icones/lib-f.svg",
            ),
        ]
    }

    /// Os do Player: a Loja e as Configurações.
    fn destinos_do_player(&self) -> [Destino; 2] {
        [
            destino("ir-loja", Rota::Loja, "Loja", "Loja", "icones/store.svg", "icones/store-f.svg"),
            destino(
                "ir-ajustes",
                Rota::Configuracoes,
                "Configurações",
                "Ajustes",
                "icones/sliders.svg",
                "icones/sliders.svg",
            ),
        ]
    }

    fn ver_destino(&self, d: &Destino, forma: Forma, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let aberto = self.nav.atual().is_some_and(|r| r.mesmo_destino(&d.rota));
        let cor = if aberto { c.texto } else { c.texto_2 };
        let caminho = if aberto { d.cheio } else { d.icone };
        let simbolo = medidas::ICONE * self.pressao(d.id, 0.94);
        let rota = d.rota.clone();
        let desenho =
            div().size(px(medidas::ICONE)).flex().items_center().justify_center().child(icone(caminho, simbolo, cor));
        let base = div()
            .id(d.id)
            .map(|e| self.focavel(e, RAIO_MODULO))
            .group(d.id)
            .flex()
            .items_center()
            .rounded(px(RAIO_MODULO))
            .cursor_pointer();
        let item = match forma {
            Forma::Linha => base
                .h(px(48.0))
                .px(px(16.0))
                .gap(px(16.0))
                .when(aberto, |x| x.bg(c.sel))
                .when(!aberto, |x| x.hover(|s| s.bg(c.hover)))
                .child(desenho)
                .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).text_color(cor).child(d.nome)),
            Forma::Pilha => base
                .h(px(60.0))
                .flex_col()
                .justify_center()
                .gap(px(4.0))
                // A pílula acende atrás do ícone; o nome fica embaixo, sem fundo.
                .child(
                    div()
                        .w(px(56.0))
                        .h(px(32.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(medidas::PILULA))
                        .when(aberto, |x| x.bg(c.sel))
                        .when(!aberto, |x| x.group_hover(d.id, |s| s.bg(c.hover)))
                        .child(desenho),
                )
                .child(medidas::micro().font_weight(FontWeight::SEMIBOLD).text_color(cor).child(d.curto)),
        };
        item.on_mouse_down(MouseButton::Left, self.ao_apertar(d.id, cx)).on_click(cx.listener(
            move |this, _: &ClickEvent, _, cx| {
                this.ir_ao_destino(rota.clone());
                cx.notify();
            },
        ))
    }

    /// Os destinos da música em linhas (o alto da lateral).
    pub(super) fn destinos_em_linhas(&self, cx: &mut Context<Self>) -> Div {
        let mut coluna = div().flex().flex_col().gap(px(4.0));
        for d in self.destinos() {
            coluna = coluna.child(self.ver_destino(&d, Forma::Linha, cx));
        }
        coluna
    }

    /// A Loja e as Configurações em linhas (o pé da lateral).
    pub(super) fn destinos_do_player_em_linhas(&self, cx: &mut Context<Self>) -> Div {
        let mut coluna = div().flex().flex_col().gap(px(4.0));
        for d in self.destinos_do_player() {
            coluna = coluna.child(self.ver_destino(&d, Forma::Linha, cx));
        }
        coluna
    }

    /// O trilho com nomes: o logo, os destinos e, embaixo, a Loja e as Configurações.
    pub(crate) fn ver_trilho(&self, r: Ret, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        // O logo pulsa uma vez por compasso (não em toda batida), e nunca no modo seguro.
        let logo = medidas::ICONE * (1.0 + 0.08 * self.pulso_compasso);
        let mut coluna = caixa(r).flex().flex_col().items_center().gap(px(4.0)).px(px(8.0)).pb(px(16.0)).child(
            div().w_full().h(px(crate::arranjo::TOPO)).flex_none().flex().items_center().justify_center().child(icone(
                "icones/logo.svg",
                logo,
                c.texto,
            )),
        );
        // Janela baixa: o espaço entre o logo e os destinos some.
        if r.h >= 640.0 {
            coluna = coluna.child(div().h(px(12.0)));
        }
        for d in self.destinos() {
            coluna = coluna.child(self.ver_destino(&d, Forma::Pilha, cx).w_full());
        }
        coluna = coluna.child(div().flex_1());
        for d in self.destinos_do_player() {
            coluna = coluna.child(self.ver_destino(&d, Forma::Pilha, cx).w_full());
        }
        coluna
    }

    /// A barra de baixo das janelas estreitas: os mesmos destinos.
    pub(crate) fn ver_nav(&self, r: Ret, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let mut linha = caixa(r).flex().items_center().gap(px(4.0)).px(px(8.0)).border_t_1().border_color(c.linha);
        if self.imersivo() {
            linha = linha.bg(c.painel);
        }
        for d in self.destinos().into_iter().chain(self.destinos_do_player()) {
            linha = linha.child(self.ver_destino(&d, Forma::Pilha, cx).flex_1().min_w(px(0.0)));
        }
        linha
    }

    /// Uma fileira de abas sublinhadas (o tipo do que a página mostra). Fichas são outra coisa: o filtro de serviço.
    /// Quem não cabe rola de lado; a escolhida fica à vista. `n` é a rolagem da fileira.
    pub(super) fn abas_sublinhadas(
        &self,
        id: &'static str,
        n: usize,
        escolhida: usize,
        abas: Vec<AnyElement>,
    ) -> Stateful<Div> {
        let c = self.cores();
        let rolagem = self.rolagem_da_prateleira(n);
        // Só quando a aba (ou a página) muda: rolar a fileira com a mão continua valendo.
        let agora = (self.nav.passo(), escolhida);
        if self.abas_vistas.borrow_mut().insert(id, agora) != Some(agora) {
            rolagem.scroll_to_item(escolhida);
        }
        div()
            .id(id)
            .track_scroll(&rolagem)
            .flex()
            .flex_none()
            .gap(px(24.0))
            .border_b_1()
            .border_color(c.linha)
            .overflow_x_scroll()
            .children(abas)
    }

    /// Uma aba de [`abas_sublinhadas`](Self::abas_sublinhadas).
    pub(super) fn aba_sublinhada(
        &self,
        id: (&'static str, usize),
        nome: &'static str,
        escolhida: bool,
    ) -> Stateful<Div> {
        let c = self.cores();
        medidas::corpo()
            .id(id)
            .map(|d| self.focavel(d, super::RAIO_CONTROLE))
            .h(px(medidas::ALVO))
            .flex()
            .flex_none()
            .items_center()
            .border_b_2()
            .border_color(if escolhida { c.texto } else { super::com_alfa(c.texto, 0.0) })
            .text_color(if escolhida { c.texto } else { c.texto_2 })
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(|s| s.text_color(c.texto))
            .child(nome)
    }
}
