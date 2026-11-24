//! O topo da folha: voltar, avançar, os espaços (Tudo, o computador e cada serviço, com a logo e a cor dele) e as
//! abas do espaço (Para você, Busca, Biblioteca); e os pedidos de página que a rota aberta precisa.

use super::{com_alfa, icone, texto};
use crate::app::Player;
use crate::navegacao::{Espaco, Rota, abas_do_espaco};
use crate::remoto::PedidoDePagina;
use gpui::{ClickEvent, Context, Div, FontWeight, SharedString, div, img, prelude::*, px};
use nucleo::Comando;

/// Altura do topo da folha.
pub const ALTURA_DO_TOPO: f32 = 52.0;

impl Player {
    /// Voltar, avançar e a fileira de espaços.
    pub(crate) fn ver_topo_da_folha(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let paleta = self.paleta_do_espaco();
        let seta = |id: &'static str, caminho: &'static str, pode: bool, comando: Comando, cx: &mut Context<Self>| {
            self.botao_icone(id, caminho, false, comando, cx).opacity(if pode { 1.0 } else { 0.4 })
        };
        let mut fileira = div().flex().items_center().gap(px(6.0)).min_w(px(0.0)).overflow_hidden();
        for (i, (espaco, nome)) in self.espacos().into_iter().enumerate() {
            let ativo = &espaco == self.nav.espaco();
            // O ativo vira pílula com o nome; o fundo acende aos poucos.
            let acesa = self.anim.seguir(("espaco-ativo", i as u32), if ativo { 1.0 } else { 0.0 }, 0.18);
            let marca = self.marca_do_espaco(&espaco, &nome, 20.0);
            let pilula = div()
                .id(("espaco", i))
                .h(px(32.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .px(px(6.0))
                .rounded(px(999.0))
                .cursor_pointer()
                .bg(com_alfa(paleta.acento, 0.16 * acesa))
                .hover(|s| s.bg(com_alfa(c.texto, 0.08)))
                .child(marca)
                .when(ativo, |d| {
                    d.pr(px(12.0)).child(
                        texto(13.0)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(c.texto)
                            .opacity(acesa)
                            .child(SharedString::from(nome.clone())),
                    )
                })
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.trocar_espaco(espaco.clone());
                    cx.notify();
                }));
            fileira = fileira.child(pilula);
        }
        // Na Estante larga (e na gaveta) a lateral já tem os espaços e as páginas: o topo da folha fica com o
        // título. No empilhado, os espaços ficam aqui e as páginas vão para a navegação de baixo.
        let estante = self.estante();
        let mostrar_espacos = !estante || self.arranjo.modo == crate::arranjo::Modo::Empilhado;
        let abas_do_topo = if estante { Vec::new() } else { abas_do_espaco(self.nav.espaco()) };
        let titulo_da_folha = (estante && !mostrar_espacos)
            .then(|| self.nav.atual().map(crate::paginas::titulo_da_rota).unwrap_or_default());
        // As abas do espaço (Para você, Busca, Biblioteca). A Biblioteca fica acesa também na Fila e no Histórico,
        // que moram nas abas dela.
        let mut abas = div().flex().flex_none().items_center().gap(px(2.0));
        for (i, (rota, nome)) in abas_do_topo.into_iter().enumerate() {
            let acesa = match (self.nav.atual(), &rota) {
                (Some(Rota::Fila | Rota::Historico), Rota::Biblioteca) => true,
                (Some(atual), _) => atual == &rota,
                (None, _) => false,
            };
            abas = abas.child(
                texto(13.0)
                    .id(("folha-aba", i))
                    .h(px(32.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .rounded(px(999.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .when(acesa, |d| d.bg(c.sel).text_color(c.texto))
                    .when(!acesa, |d| d.text_color(c.texto_2).hover(|s| s.bg(c.hover).text_color(c.texto)))
                    .child(nome)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.abrir_aba_do_espaco(rota.clone());
                        cx.notify();
                    })),
            );
        }
        div()
            .flex_none()
            .h(px(ALTURA_DO_TOPO))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(2.0))
            .border_b_1()
            .border_color(paleta.filete)
            .child(seta("folha-voltar-pagina", "icones/chev-l.svg", self.nav.pode_voltar(), Comando::Voltar, cx))
            .child(seta("folha-avancar-pagina", "icones/chev-r.svg", self.nav.pode_avancar(), Comando::Avancar, cx))
            .child(div().w(px(8.0)))
            .when(mostrar_espacos, |d| d.child(fileira.flex_1()))
            .when_some(titulo_da_folha, |d, t| {
                d.child(
                    texto(15.0)
                        .flex_1()
                        .min_w(px(0.0))
                        .truncate()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(c.texto)
                        .child(SharedString::from(t)),
                )
            })
            .child(abas)
    }

    /// O círculo do espaço: o ícone (Tudo, computador), a logo do serviço ou a inicial na cor dele.
    pub(super) fn marca_do_espaco(&self, espaco: &Espaco, nome: &str, tamanho: f32) -> Div {
        let c = self.cores();
        let circulo = div().size(px(tamanho)).flex_none().flex().items_center().justify_center().rounded(px(999.0));
        match espaco {
            Espaco::Tudo => circulo.child(icone("icones/grid.svg", tamanho * 0.8, c.texto_2)),
            Espaco::Computador => circulo.child(icone("icones/lib.svg", tamanho * 0.8, c.texto_2)),
            Espaco::Servico(s) => {
                let logo = self.local.remoto.provedores.iter().find(|p| &p.servico == s).and_then(|p| p.logo.clone());
                match logo {
                    Some(arquivo) => circulo.child(img(arquivo).size(px(tamanho)).rounded(px(999.0))),
                    None => {
                        let cor = self.cor_do_espaco(espaco).map_or(c.texto_2, crate::tema::cor_de);
                        let inicial: String =
                            nome.chars().next().map(|l| l.to_uppercase().collect()).unwrap_or_default();
                        // A inicial em preto ou branco, o que tiver mais contraste com a cor do serviço.
                        let fundo = crate::tema::tripla(cor);
                        let escuro = crate::tema::contraste([0.0; 3], fundo) >= crate::tema::contraste([1.0; 3], fundo);
                        let letra = if escuro { gpui::black() } else { gpui::white() };
                        circulo
                            .bg(cor)
                            .child(texto(11.0).font_weight(FontWeight::BOLD).text_color(letra).child(inicial))
                    }
                }
            }
        }
    }

    /// Os pedidos de página que a rota aberta precisa (repetir não custa: o remoto guarda as respostas).
    pub(crate) fn pedir_paginas_da_rota(&mut self) {
        self.pedir_pagina_aberta();
        // A biblioteca da conta: na página dela, e na lateral da Estante (sempre à vista).
        if self.estante() || self.nav.atual() == Some(&Rota::Biblioteca) {
            let servicos = match self.nav.espaco() {
                Espaco::Servico(s) => self.servicos_com_biblioteca(Some(s)),
                Espaco::Tudo => self.servicos_com_biblioteca(None),
                Espaco::Computador => Vec::new(),
            };
            for servico in servicos {
                self.local.remoto.abrir_pagina(PedidoDePagina::Biblioteca { servico });
            }
        }
    }

    fn pedir_pagina_aberta(&mut self) {
        if let Some(pedido) = self.nav.atual().and_then(crate::paginas::pedido_da_rota) {
            self.local.remoto.abrir_pagina(pedido);
            return;
        }
        if self.nav.atual() != Some(&Rota::ParaVoce) {
            return;
        }
        let servicos: Vec<String> = match self.nav.espaco() {
            Espaco::Servico(s) => vec![s.clone()],
            Espaco::Tudo => self
                .espacos()
                .into_iter()
                .filter_map(|(e, _)| if let Espaco::Servico(s) = e { Some(s) } else { None })
                .collect(),
            Espaco::Computador => Vec::new(),
        };
        for servico in servicos {
            self.local.remoto.abrir_pagina(PedidoDePagina::Inicio { servico });
        }
    }
}
