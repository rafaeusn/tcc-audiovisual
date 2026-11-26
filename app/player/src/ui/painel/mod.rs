//! O painel: os módulos (Ritmo, Energia, Estrutura, Letra, Nesta sessão, Visual, Relógio) ou, quando pedidos, os
//! Ajustes.
//!
//! No modo largo ele fica ao lado do visual; no modo gaveta abre por cima; no empilhado fica abaixo do visual.
//! No modo edição (E) os módulos podem ser arrastados um sobre o outro para trocar de lugar, removidos e
//! adicionados. A ordem fica salva nos ajustes.

use super::{RAIO_CONTROLE, RAIO_MODULO, caixa, com_alfa, decimal, ha_quanto, icone, numero, tecla, tempo};
use crate::app::{AbaDoPainel, COLUNAS_ENERGIA, PASSO_DA_LETRA, Player};
use crate::arranjo::{RAIO, Ret};
use crate::letrista::Situacao;
use crate::tema::{self, Cores};
use crate::ui::medidas;
use crate::ui::medidas::px;
use analise::Rotulo;
use gpui::{
    AnyElement, ClickEvent, Context, CursorStyle, Div, FontWeight, Hsla, MouseButton, MouseDownEvent, SharedString,
    Stateful, canvas, deferred, div, img, linear_color_stop, linear_gradient, prelude::*,
};
use nucleo::ajustes::PALETAS;
use nucleo::{Comando, Modelo, Modulo, Skin, Tema};

mod ajustes;
mod corpos;
mod letra;
pub(crate) use letra::TamanhoDaLetra;

/// Quantas faixas da sessão aparecem no módulo (o histórico guarda mais).
const VISIVEIS_NA_SESSAO: usize = 6;
/// Pontos por coluna do medidor de energia.
const PONTOS: usize = 5;
/// Espaço entre os módulos e a borda interna de cada um (onde aparece o contorno do modo edição).
const VAO: f32 = 12.0;
const BORDA_DO_MODULO: f32 = 10.0;

fn rgb8(c: [u8; 3]) -> Hsla {
    tema::cor_de(c.map(|v| f32::from(v) / 255.0))
}

fn nivel(r: Rotulo) -> f32 {
    match r {
        Rotulo::Calma => 0.0,
        Rotulo::Ativa => 1.0,
        Rotulo::Intensa => 2.0,
    }
}

pub fn icone_do_modulo(m: Modulo) -> &'static str {
    match m {
        Modulo::Ritmo => "icones/metro.svg",
        Modulo::Energia => "icones/bolt.svg",
        Modulo::Estrutura => "icones/struct.svg",
        Modulo::Sessao => "icones/history.svg",
        Modulo::Visual => "icones/eye.svg",
        Modulo::Relogio => "icones/clock.svg",
        Modulo::Letra => "icones/lyrics.svg",
    }
}

/// Interruptor (ligado/desligado). A bolinha desliza em 200 ms.
pub(super) fn interruptor(c: &Cores, id: &'static str, ligado: bool, posicao: f32) -> Stateful<Div> {
    let trilho =
        div().id(id).w(px(44.0)).h(px(24.0)).flex_none().relative().rounded(px(medidas::PILULA)).cursor_pointer();
    let trilho = if ligado { trilho.bg(c.inv) } else { trilho.border_1().border_color(c.linha_2) };
    trilho.child(
        div()
            .absolute()
            .top(px(if ligado { 4.0 } else { 3.0 }))
            .left(px(3.0 + 21.0 * posicao))
            .size(px(16.0))
            .rounded(px(medidas::PILULA))
            .bg(if ligado { c.inv_texto } else { c.texto_2 }),
    )
}

/// Linha de um ajuste: ícone, título, explicação e o controle à direita.
pub(super) fn linha_ajuste(c: &Cores, caminho: &'static str, nome: &'static str, frase: SharedString) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(div().size(px(32.0)).flex_none().flex().items_center().justify_center().child(icone(
            caminho,
            medidas::ICONE,
            c.texto,
        )))
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(nome))
                .child(medidas::apoio().text_color(c.texto_2).child(frase)),
        )
}

pub(super) fn bloco_ajuste(c: &Cores, ultimo: bool) -> Div {
    div()
        .py(px(20.0))
        .flex()
        .flex_col()
        .flex_none()
        .gap(px(12.0))
        .when(!ultimo, |d| d.border_b_1().border_color(c.linha))
}

/// Os blocos de ajuste, prontos para a página Configurações escolher quais mostrar em cada seção.
pub(crate) struct BlocosDeAjuste {
    pub visual: Div,
    pub escuta: Div,
    pub reproducao: Div,
    pub cores: Div,
    pub seguro: Div,
    pub aparencia: Div,
    pub janela: Div,
    pub atalhos: Div,
    pub sobre: Div,
}

fn titulo_secao(c: &Cores, nome: &'static str) -> Div {
    medidas::apoio().text_color(c.texto_3).font_weight(FontWeight::SEMIBOLD).child(nome)
}

impl Player {
    pub(crate) fn ver_painel(&self, r: Ret, gaveta: bool, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let c = self.cores();
        let corpo = if self.configuracoes_no_painel() {
            self.ver_configuracoes_no_painel(r.w, cx).into_any_element()
        } else if self.edicao {
            self.ver_modulos(r.w, cx).into_any_element()
        } else {
            self.abas_do_painel(r, cx).into_any_element()
        };
        // O alto do painel ganha a cor do momento (a mesma do dock), que some em 220 px.
        let tom = tema::cor_de(self.tom);
        caixa(r)
            .id(if gaveta { "gaveta" } else { "painel" })
            .rounded(px(RAIO))
            .border_1()
            .border_color(if self.edicao { c.linha_2 } else { c.linha })
            .bg(c.painel)
            .overflow_hidden()
            .flex()
            .flex_col()
            // Cliques no painel não passam para o que está por baixo (o visual, na gaveta).
            .occlude()
            // O corte do painel é reto: sem os cantos próprios, o tom vazava para fora da borda arredondada.
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .w_full()
                    .h(px(220.0f32.min(r.h)))
                    .rounded_tl(px(RAIO - 1.0))
                    .rounded_tr(px(RAIO - 1.0))
                    .bg(linear_gradient(
                        180.0,
                        linear_color_stop(com_alfa(tom, if self.imersivo() { 0.5 } else { 1.0 }), 0.0),
                        linear_color_stop(com_alfa(tom, 0.0), 1.0),
                    )),
            )
            .child(corpo)
    }

    /// Escurece o resto da janela enquanto a gaveta está aberta; clicar fora fecha.
    pub(crate) fn ver_fundo_da_gaveta(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let c = self.cores();
        let a = &self.arranjo;
        // Começa abaixo do topo: os botões da janela continuam ao alcance.
        div()
            .id("gaveta-fundo")
            .absolute()
            .left(px(0.0))
            .top(px(a.topo.h))
            .w(px(a.janela.w))
            .h(px((a.janela.h - a.topo.h).max(0.0)))
            .bg(c.escurecer)
            .occlude()
            .on_click(self.ao_clicar(Comando::AlternarPainel, cx))
    }

    /// O painel em abas: os módulos, a fila e a letra.
    fn abas_do_painel(&self, r: Ret, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let lado = if self.arranjo.compacto || r.w < 300.0 { 16.0 } else { 24.0 };
        let mut abas =
            div().relative().flex().flex_none().gap(px(24.0)).px(px(lado)).border_b_1().border_color(c.linha);
        for (i, aba) in AbaDoPainel::TODAS.into_iter().enumerate() {
            abas = abas.child(self.aba_sublinhada(("painel-aba", i), aba.nome(), self.aba_do_painel == aba).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.mostrar_aba(aba);
                    cx.notify();
                }),
            ));
        }
        // Na Estante o painel se fecha nele mesmo: ele pode estar no lugar da página.
        if self.estante() {
            abas = abas.child(div().flex_1()).child(self.botao_icone(
                "painel-fechar",
                "icones/x.svg",
                false,
                Comando::AlternarPainel,
                cx,
            ));
        }
        let altura = (r.h - medidas::ALVO - 3.0).max(0.0);
        let corpo: AnyElement = match self.aba_do_painel {
            AbaDoPainel::Agora => self.ver_modulos(r.w, cx).into_any_element(),
            AbaDoPainel::Fila => self.aba_da_fila(cx).into_any_element(),
            AbaDoPainel::Letra => div()
                .relative()
                .px(px(lado))
                .py(px(12.0))
                .child(self.aba_da_letra(altura - 24.0, cx))
                .into_any_element(),
        };
        div()
            .relative()
            .size_full()
            .flex()
            .flex_col()
            .child(abas)
            .child(div().flex_1().min_h(px(0.0)).overflow_hidden().child(corpo))
    }

    // ---------------------------------------------------------------------------------------- módulos

    fn ver_modulos(&self, largura: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let lado = if self.arranjo.compacto || largura < 300.0 { 6.0 } else { 14.0 };
        let interno = (largura - 2.0 - 2.0 * lado).max(120.0);
        let metade = ((interno - VAO) / 2.0).floor();
        let mut grade = div()
            .id("modulos")
            .relative()
            .size_full()
            .overflow_y_scroll()
            .p(px(lado))
            .flex()
            .flex_wrap()
            .content_start()
            .items_start()
            .gap(px(VAO));

        for m in self.ajustes.modulos.iter().copied() {
            grade = grade.child(self.ver_modulo(m, if m.largo() { interno } else { metade }, cx));
        }
        if self.ajustes.modulos.is_empty() && !self.edicao {
            grade = grade.child(
                medidas::apoio()
                    .p(px(BORDA_DO_MODULO))
                    .text_color(c.texto_2)
                    .child("Nenhum módulo no painel. Aperte E para adicionar."),
            );
        }
        if self.edicao {
            // O que está fora do painel pode voltar com um clique.
            let fora: Vec<Modulo> = Modulo::TODOS.into_iter().filter(|m| !self.ajustes.modulos.contains(m)).collect();
            let mut linha =
                div().w(px(interno)).flex_none().p(px(BORDA_DO_MODULO)).flex().flex_wrap().items_center().gap(px(8.0));
            linha =
                linha.child(
                    medidas::apoio().w_full().text_color(c.texto_3).font_weight(FontWeight::SEMIBOLD).child(
                        if fora.is_empty() { "Todos os módulos estão no painel" } else { "Adicionar ao painel" },
                    ),
                );
            for m in fora {
                linha = linha.child(
                    medidas::corpo()
                        .id(("adicionar-modulo", m as usize))
                        .h(px(medidas::BOTAO))
                        .px(px(14.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(medidas::PILULA))
                        .border_1()
                        .border_color(c.linha_2)
                        .text_color(c.texto_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/plus.svg", medidas::ICONE_LINHA, c.texto_2))
                        .child(m.nome())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.adicionar_modulo(m);
                            cx.notify();
                        })),
                );
            }
            grade = grade.child(linha);
        }
        grade
    }

    /// Um módulo na grade: cabeçalho, corpo e, no modo edição, o contorno, a alça e o botão de tirar.
    fn ver_modulo(&self, m: Modulo, largura: f32, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let (dx, dy) = self.deslocamento_do_modulo(m);
        let arrastando = self.arrasto.as_ref().is_some_and(|a| a.modulo == m);
        let alvo = self.arrasto.as_ref().is_some_and(|a| a.alvo == Some(m));
        let (direita, corpo): (Option<AnyElement>, Div) = match m {
            Modulo::Ritmo => (None, self.corpo_ritmo()),
            Modulo::Energia => self.corpo_energia(),
            Modulo::Estrutura => self.corpo_estrutura(largura - 2.0 * BORDA_DO_MODULO - 2.0),
            Modulo::Sessao => self.corpo_sessao(),
            Modulo::Visual => self.corpo_visual(cx),
            Modulo::Relogio => (None, self.corpo_relogio()),
            Modulo::Letra => (None, self.corpo_letra(TamanhoDaLetra::MODULO, cx)),
        };

        // Num módulo de meia largura com algo à direita, o nome fica e o ícone sai.
        let aperta = largura < 200.0 && (direita.is_some() || self.edicao);
        let mut cabecalho = div()
            .h(px(24.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .when(!aperta, |d| d.child(icone(icone_do_modulo(m), medidas::ICONE_LINHA, c.texto_2)))
            .child(
                if aperta && self.edicao { medidas::apoio() } else { medidas::corpo() }
                    .min_w(px(0.0))
                    .text_color(c.texto_2)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(crate::ui::linha(m.nome())),
            )
            .child(div().flex_1());
        if self.edicao {
            cabecalho = cabecalho.child(icone("icones/grip.svg", medidas::ICONE_LINHA, c.texto_2)).child(
                div()
                    .id(("tirar-modulo", m as usize))
                    .size(px(24.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(medidas::PILULA))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.sel))
                    .child(icone("icones/x.svg", medidas::ICONE_MIUDO, c.texto_2))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.remover_modulo(m);
                        cx.notify();
                    })),
            );
        } else {
            cabecalho = cabecalho.children(direita);
        }

        // Guarda onde o módulo foi desenhado: é assim que o arrasto sabe em cima de quem o ponteiro está.
        let limites = self.limites.clone();
        let marcador = canvas(
            move |area, _, _| {
                limites.borrow_mut().insert(m, area);
            },
            |_, (), _, _| {},
        )
        .absolute()
        .left(px(0.0))
        .top(px(0.0))
        .size_full();

        let mut caixa_do_modulo = div()
            .relative()
            .left(px(dx))
            .top(px(dy))
            .w(px(largura))
            .flex_none()
            .p(px(BORDA_DO_MODULO))
            .rounded(px(RAIO_MODULO))
            .border_1()
            .border_color(gpui::transparent_black())
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(marcador)
            .child(cabecalho)
            .child(corpo.when(self.edicao, |d| d.opacity(0.62)));
        if self.edicao {
            caixa_do_modulo = caixa_do_modulo
                .border_dashed()
                .border_color(if alvo || arrastando { c.texto_2 } else { c.linha_2 })
                .cursor(if arrastando { CursorStyle::ClosedHand } else { CursorStyle::OpenHand })
                .when(alvo, |d| d.bg(c.sel))
                // Levantado: ganha fundo e sombra, e segue o ponteiro de onde foi pego.
                .when(arrastando, |d| d.bg(c.bloco).shadow_lg())
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                        this.pegar_modulo(m, ev.position);
                        cx.notify();
                    }),
                );
        }
        if arrastando {
            // Desenhado por último, por cima dos vizinhos.
            deferred(caixa_do_modulo).with_priority(1).into_any_element()
        } else {
            caixa_do_modulo.into_any_element()
        }
    }
}
