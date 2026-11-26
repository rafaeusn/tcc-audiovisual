//! O filtro de serviço no alto do Início, do Buscar e da Biblioteca (Tudo, cada serviço com a marca dele, Neste PC); a
//! marca de cada um; e os pedidos de página que a rota aberta precisa.

use super::{icone, medidas};
use crate::app::Player;
use crate::navegacao::{Espaco, Rota};
use crate::remoto::PedidoDePagina;
use crate::ui::medidas::px;
use gpui::{ClickEvent, Context, Div, FontWeight, SharedString, div, img, prelude::*};

/// Altura da faixa do filtro (a ficha e o espaço em volta).
pub const ALTURA_DO_FILTRO: f32 = 64.0;
/// Com menos que isto de página, só a ficha escolhida mostra o nome.
const CABEM_OS_NOMES: f32 = 520.0;

impl Player {
    /// As fichas do filtro. A escolhida é invertida; trocar o filtro não troca de página.
    pub(crate) fn ver_filtro(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let nomes = self.arranjo.heroi.w >= CABEM_OS_NOMES;
        let mut fichas =
            div().id("filtro").flex().flex_1().min_w(px(0.0)).items_center().gap(px(8.0)).overflow_x_scroll();
        for (i, (espaco, nome)) in self.espacos().into_iter().enumerate() {
            let escolhida = &espaco == self.nav.espaco();
            let com_marca = espaco != Espaco::Tudo;
            let com_nome = nomes || escolhida || !com_marca;
            let marca = com_marca.then(|| self.marca_do_espaco(&espaco, &nome, medidas::ICONE_LINHA, escolhida));
            let ficha = medidas::apoio()
                .id(("filtro", i))
                .map(|d| self.focavel(d, medidas::PILULA))
                .h(px(medidas::FICHA))
                .min_w(px(medidas::FICHA))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .px(px(if com_nome { 16.0 } else { 10.0 }))
                .when(com_marca && com_nome, |d| d.pl(px(10.0)))
                .rounded(px(medidas::PILULA))
                .border_1()
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .map(|d| {
                    if escolhida {
                        d.bg(c.inv).border_color(c.inv).text_color(c.inv_texto)
                    } else {
                        d.border_color(c.linha_2).text_color(c.texto_2).hover(|s| s.bg(c.hover).text_color(c.texto))
                    }
                })
                .children(marca)
                .when(com_nome, |d| d.child(SharedString::from(nome)))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.trocar_espaco(espaco.clone());
                    cx.notify();
                }));
            fichas = fichas.child(ficha);
        }
        div()
            .flex_none()
            .h(px(ALTURA_DO_FILTRO))
            .px(px(self.lado_da_pagina()))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(fichas)
            .when(!self.estante(), |d| d.child(self.botao_fechar("folha-fechar", cx)))
            .children(self.botao_da_pagina_larga(cx))
    }

    /// A margem dos lados das páginas: menor em folha estreita.
    pub(super) fn lado_da_pagina(&self) -> f32 {
        if self.arranjo.heroi.w >= 640.0 { 24.0 } else { 16.0 }
    }

    /// A marca do filtro: a logo do serviço, a inicial na cor dele, ou o ícone (Tudo, computador). `invertida` é sobre a
    /// ficha escolhida.
    pub(super) fn marca_do_espaco(&self, espaco: &Espaco, nome: &str, tamanho: f32, invertida: bool) -> Div {
        let c = self.cores();
        let cor_do_icone = if invertida { c.inv_texto } else { c.texto_2 };
        let circulo =
            div().size(px(tamanho)).flex_none().flex().items_center().justify_center().rounded(px(medidas::PILULA));
        match espaco {
            Espaco::Tudo => circulo.child(icone("icones/grid.svg", tamanho, cor_do_icone)),
            Espaco::Computador => circulo.child(icone("icones/pasta.svg", tamanho, cor_do_icone)),
            Espaco::Servico(s) => {
                let logo = self.local.remoto.provedores.iter().find(|p| &p.servico == s).and_then(|p| p.logo.clone());
                match logo {
                    Some(arquivo) => circulo.child(img(arquivo).size(px(tamanho)).rounded(px(medidas::PILULA))),
                    None => {
                        let cor = crate::tema::cor_de(self.cor_do_servico(s));
                        let inicial: String =
                            nome.chars().next().map(|l| l.to_uppercase().collect()).unwrap_or_default();
                        // A inicial em preto ou branco, o que tiver mais contraste com a cor do serviço.
                        let fundo = crate::tema::tripla(cor);
                        let escuro = crate::tema::contraste([0.0; 3], fundo) >= crate::tema::contraste([1.0; 3], fundo);
                        let letra = if escuro { gpui::black() } else { gpui::white() };
                        circulo
                            .bg(cor)
                            .child(medidas::micro().font_weight(FontWeight::BOLD).text_color(letra).child(inicial))
                    }
                }
            }
        }
    }

    /// Os pedidos de página que a rota aberta precisa (repetir não custa: o remoto guarda as respostas).
    pub(crate) fn pedir_paginas_da_rota(&mut self) {
        self.pedir_pagina_aberta();
        // A biblioteca das contas: todas na lateral da Estante (sempre à vista), e as do filtro na página dela.
        let servicos = if self.estante() {
            self.servicos_com_biblioteca(None)
        } else if matches!(self.nav.atual(), Some(Rota::Biblioteca(_))) && self.nav.espaco() != &Espaco::Computador {
            self.servicos_com_biblioteca(self.servico_do_filtro())
        } else {
            return;
        };
        for servico in servicos {
            self.local.remoto.abrir_pagina(PedidoDePagina::Biblioteca { servico });
        }
    }

    fn pedir_pagina_aberta(&mut self) {
        if let Some(pedido) = self.nav.atual().and_then(crate::paginas::pedido_da_rota) {
            self.local.remoto.abrir_pagina(pedido);
        } else if self.nav.atual() == Some(&Rota::Inicio) {
            for (servico, _) in self.servicos_no_filtro() {
                self.local.remoto.abrir_pagina(PedidoDePagina::Inicio { servico });
            }
        }
    }
}
