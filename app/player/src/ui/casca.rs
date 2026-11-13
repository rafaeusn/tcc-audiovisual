//! A casca: barra lateral, topo (busca, modo edição e os botões da janela) e a navegação de baixo das janelas
//! estreitas.

use super::{Botao, RAIO_MODULO, caixa, icone, tecla, texto};
use crate::app::Player;
use crate::arranjo::{Modo, Ret};
use crate::escuta::Situacao;
use gpui::{
    AnyElement, Context, Div, FontWeight, MouseButton, SharedString, Stateful, WindowControlArea, div, prelude::*, px,
};
use nucleo::{Comando, Pagina};

/// (página, rótulo, ícone normal, ícone da página ativa, id na barra lateral, id na navegação de baixo)
const PAGINAS: [(Pagina, &str, &str, &str, &str, &str); 3] = [
    (Pagina::Inicio, "Início", "icones/home.svg", "icones/home-f.svg", "nav-inicio", "bnav-inicio"),
    (Pagina::Biblioteca, "Biblioteca", "icones/lib.svg", "icones/lib-f.svg", "nav-biblioteca", "bnav-biblioteca"),
    (Pagina::Loja, "Loja", "icones/store.svg", "icones/store-f.svg", "nav-loja", "bnav-loja"),
];

/// O sistema descreve a saída com detalhes que não cabem no topo ("Alto-falantes (Realtek) [Speaker] via ...").
/// Fica só o começo, até o primeiro colchete.
fn nome_curto(dispositivo: &str) -> &str {
    let curto = dispositivo.split(" [").next().unwrap_or(dispositivo).trim();
    if curto.is_empty() { dispositivo } else { curto }
}

impl Player {
    fn pagina_ativa(&self, p: Pagina) -> bool {
        self.pagina == p && !(self.painel_ajustes && self.arranjo.modo == Modo::Empilhado)
    }

    fn item_do_trilho(
        &self,
        id: &'static str,
        caminho: &'static str,
        ativo: bool,
        comando: Comando,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        self.botao(
            Botao {
                id,
                icone: caminho,
                tamanho: 48.0,
                tamanho_icone: 24.0,
                raio: RAIO_MODULO,
                cor: if ativo { c.texto } else { c.texto_2 },
                fundo: ativo.then_some(c.sel),
                realce: c.hover,
                afunda: 0.94,
            },
            cx,
        )
        .on_click(self.ao_clicar(comando, cx))
        .on_hover(self.ao_pairar(id, Some(comando), "", cx))
    }

    pub(crate) fn ver_trilho(&self, r: Ret, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        // O logo pulsa uma vez por compasso (não em toda batida), e nunca no modo seguro.
        let logo = 24.0 * (1.0 + 0.08 * self.pulso_compasso);
        let mut coluna = caixa(r).flex().flex_col().items_center().gap(px(4.0)).pb(px(16.0));
        coluna = coluna.child(div().size(px(52.0)).flex_none().flex().items_center().justify_center().child(icone(
            "icones/logo.svg",
            logo,
            c.texto,
        )));
        coluna = coluna.child(div().h(px(12.0)));
        for (pagina, _, normal, cheio, id, _) in PAGINAS {
            let ativo = self.pagina_ativa(pagina);
            coluna = coluna.child(self.item_do_trilho(
                id,
                if ativo { cheio } else { normal },
                ativo,
                Comando::AbrirPagina(pagina),
                cx,
            ));
        }
        coluna
            .child(div().flex_1())
            .child(self.item_do_trilho("nav-editar", "icones/grid.svg", self.edicao, Comando::EditarLayout, cx))
            .child(self.item_do_trilho(
                "nav-ajustes",
                "icones/sliders.svg",
                self.painel_ajustes,
                Comando::AbrirAjustes,
                cx,
            ))
    }

    /// Topo: área de arrastar a janela, a situação do Modo Escuta, a busca e os botões de minimizar, maximizar e
    /// fechar. No modo edição a busca dá lugar aos controles da edição.
    pub(crate) fn ver_topo(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let curto = a.topo.w < 600.0;
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
        let estreito = a.modo == Modo::Empilhado;
        let controle = |nome: &'static str, area: WindowControlArea, caminho: &'static str, fechar: bool| {
            div()
                .id(nome)
                .w(px(46.0))
                .h(px(32.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(super::RAIO_CONTROLE))
                .hover(|s| if fechar { s.bg(c.alerta) } else { s.bg(c.hover) })
                .window_control_area(area)
                .child(icone(caminho, 16.0, c.texto_2))
        };

        let situacao = div()
            .h(px(32.0))
            .max_w_full()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .rounded(px(999.0))
            .border_1()
            .border_color(c.linha_2)
            .when(self.imersivo(), |d| d.bg(c.painel))
            .child(div().size(px(6.0)).flex_none().rounded(px(999.0)).bg(ponto))
            .child(
                texto(14.0)
                    .min_w(px(0.0))
                    .truncate()
                    .text_color(c.texto_2)
                    .font_weight(FontWeight::MEDIUM)
                    .child(frase),
            );

        // O meio do topo: controles da edição, ou a busca (que vira só um ícone quando não cabe).
        let cabe_busca = a.topo.w >= 720.0;
        let meio: Option<AnyElement> = if self.edicao {
            Some(self.ver_barra_de_edicao(cx).into_any_element())
        } else if cabe_busca {
            let largura = (a.topo.w * 0.44).min(440.0);
            Some(
                div()
                    .id("busca")
                    .w(px(largura))
                    .h(px(40.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .pl(px(14.0))
                    .pr(px(8.0))
                    .rounded(px(999.0))
                    .border_1()
                    .border_color(c.linha_2)
                    .cursor_pointer()
                    .when(self.imersivo(), |d| d.bg(c.painel))
                    .hover(|s| s.bg(c.hover))
                    .child(icone("icones/search.svg", 18.0, c.texto_2))
                    .child(
                        texto(14.0)
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .text_color(c.texto_2)
                            .child("Buscar ou executar um comando"),
                    )
                    .child(div().flex().flex_none().gap(px(4.0)).child(tecla(&c, "Ctrl")).child(tecla(&c, "K")))
                    .on_click(self.ao_clicar(Comando::AbrirPaleta, cx))
                    .into_any_element(),
            )
        } else {
            None
        };
        let busca_pequena = (!self.edicao && !cabe_busca)
            .then(|| self.botao_icone("busca-pequena", "icones/search.svg", false, Comando::AbrirPaleta, cx));

        caixa(a.topo)
            // Sem isto, o foco do teclado (na raiz) marca o clique como tratado e o Windows não recebe o clique
            // nos botões da janela nem o arrasto.
            .occlude()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(
                // Tudo o que não é botão arrasta a janela.
                div()
                    .flex_1()
                    .flex_basis(px(0.0))
                    .h_full()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .when(estreito, |d| d.child(icone("icones/logo.svg", 24.0, c.texto)).gap(px(12.0)))
                    .window_control_area(WindowControlArea::Drag)
                    .child(situacao),
            )
            .children(meio)
            .child(
                div()
                    .flex_1()
                    .flex_basis(px(0.0))
                    .h_full()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .child(div().flex_1().h_full().window_control_area(WindowControlArea::Drag))
                    .children(busca_pequena)
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
            texto(14.0)
                .id(id)
                .h(px(40.0))
                .px(px(16.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .rounded(px(999.0))
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
                    texto(14.0)
                        .h(px(40.0))
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(999.0))
                        .bg(c.sel)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(icone("icones/grid.svg", 18.0, c.texto))
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
                        .child(icone("icones/swap.svg", 18.0, c.texto_2))
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

    /// Navegação de baixo (janelas estreitas): os mesmos destinos da barra lateral, com rótulo.
    pub(crate) fn ver_nav(&self, r: Ret, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let mut linha = caixa(r).flex().items_center().gap(px(4.0)).px(px(12.0)).border_t_1().border_color(c.linha);
        if self.imersivo() {
            linha = linha.bg(c.painel);
        }
        let mut itens: Vec<(&'static str, &'static str, &'static str, bool, Comando)> = Vec::with_capacity(4);
        for (pagina, rotulo, normal, cheio, _, id) in PAGINAS {
            let ativo = self.pagina_ativa(pagina);
            itens.push((id, rotulo, if ativo { cheio } else { normal }, ativo, Comando::AbrirPagina(pagina)));
        }
        itens.push(("bnav-ajustes", "Ajustes", "icones/sliders.svg", self.painel_ajustes, Comando::AbrirAjustes));
        for (id, rotulo, caminho, ativo, comando) in itens {
            let e = self.pressao(id, 0.96);
            let cor = if ativo { c.texto } else { c.texto_2 };
            linha = linha.child(
                div()
                    .id(id)
                    .flex_1()
                    .h(px(52.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(2.0))
                    .rounded(px(RAIO_MODULO))
                    .cursor_pointer()
                    .when(ativo, |d| d.bg(c.sel))
                    .child(div().size(px(22.0)).flex().items_center().justify_center().child(icone(
                        caminho,
                        22.0 * e,
                        cor,
                    )))
                    .child(texto(12.0).text_color(cor).font_weight(FontWeight::SEMIBOLD).child(rotulo))
                    .on_mouse_down(MouseButton::Left, self.ao_apertar(id, cx))
                    .on_click(self.ao_clicar(comando, cx)),
            );
        }
        linha
    }
}

#[cfg(test)]
mod testes {
    use super::nome_curto;

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
