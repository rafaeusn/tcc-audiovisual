//! O layout Estante: a lateral (os destinos e a biblioteca), o visual no centro com a música por cima, a divisória e a
//! página (o herói) ao lado dele. O painel, o trilho com nomes e a barra de baixo são os mesmos do Palco.

use super::{Botao, caixa, icone, medidas, texto, titulo};
use crate::app::Player;
use crate::arranjo::{Modo, RAIO, Ret};
use crate::menu::Alvo;
use crate::paginas::cartao;
use crate::ui::medidas::px;
use biblioteca::TipoSalvo;
use gpui::{
    AnyElement, ClickEvent, Context, CursorStyle, Div, FontWeight, MouseButton, MouseDownEvent, SharedString, Stateful,
    div, prelude::*, rgba, uniform_list,
};
use nucleo::Comando;
use provedores::{Faixa, Item};
use std::ops::Range;
use std::sync::Arc;

/// "1 música", "38 músicas".
pub(super) fn quantas_musicas(n: u32) -> String {
    if n == 1 { "1 música".into() } else { format!("{n} músicas") }
}

impl Player {
    /// As regiões da Estante. Enquanto o visual cresce até a janela, a página sai para a direita e o resto esmaece (o
    /// visual é desenhado por baixo de tudo, na camada).
    pub(crate) fn ver_estante(&self, cx: &mut Context<Self>) -> Div {
        let a = self.arranjo;
        let expansao = self.expansao.clamp(0.0, 1.0);
        let mut regioes =
            div().absolute().left(px(0.0)).top(px(0.0)).size_full().overflow_hidden().opacity(1.0 - expansao);
        // Na ordem do Tab: a navegação, o topo, a página (e o painel), a barra e o visual.
        if let Some(r) = a.lateral {
            regioes = regioes.child(self.ver_lateral(r, cx));
        }
        if let Some(r) = a.trilho {
            regioes = regioes.child(self.ver_trilho(r, cx));
        }
        regioes = regioes.child(self.ver_topo(cx));
        let mut pagina =
            div().absolute().left(px((a.janela.w - a.heroi.x) * expansao)).top(px(0.0)).w(px(a.janela.w)).h_full();
        if !a.painel_na_pagina {
            pagina = pagina.child(self.ver_heroi(cx));
        }
        if let Some(r) = a.painel {
            pagina = pagina.child(self.ver_painel(r, false, cx));
        }
        if let Some(r) = a.divisoria {
            pagina = pagina.child(self.ver_divisoria(r, cx));
        }
        regioes = regioes.child(pagina).child(self.ver_dock(cx));
        if let Some(r) = a.nav {
            regioes = regioes.child(self.ver_nav(r, cx));
        }
        if a.visual.w >= 2.0 && a.visual.h >= 2.0 {
            // O quadro acompanha o visual, que passa por cima da lateral: o que está escrito nele sai logo.
            let quadro = self.ver_quadro_do_visual(cx).opacity((1.0 - 4.0 * expansao).max(0.0));
            regioes = regioes.child(quadro);
        }
        regioes
    }

    /// O vão entre o visual e a página: arrastar muda a largura dos dois, e dois cliques voltam ao padrão.
    fn ver_divisoria(&self, r: Ret, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let pega = if self.arrastando_divisao { PEGA_ABERTA } else { PEGA };
        let pega = self.anim.seguir(("divisoria", 0), pega, 0.16);
        let cor = if self.arrastando_divisao { c.texto } else { c.linha_2 };
        div()
            .id("divisoria")
            .group("divisoria")
            .absolute()
            .left(px(r.x))
            .top(px(r.y))
            .w(px(r.w))
            .h(px(r.h))
            .occlude()
            .flex()
            .items_center()
            .justify_center()
            .cursor(CursorStyle::ResizeLeftRight)
            .child(
                div()
                    .w(px(4.0))
                    .h(px(pega))
                    .rounded(px(medidas::PILULA))
                    .bg(cor)
                    .group_hover("divisoria", |s| s.bg(c.texto)),
            )
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                    if ev.click_count >= 2 {
                        this.arrastando_divisao = false;
                        this.ajustes.divisao = None;
                        this.salvar();
                    } else {
                        this.arrastando_divisao = true;
                    }
                    cx.notify();
                }),
            )
    }

    /// O botão da página larga, no canto da página (`None` onde ela já ocupa a largura toda).
    pub(super) fn botao_da_pagina_larga(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        (self.estante() && self.arranjo.modo != Modo::Empilhado).then(|| {
            let caminho = if self.ajustes.pagina_larga { "icones/lado.svg" } else { "icones/larga.svg" };
            self.botao_icone("pagina-larga", caminho, false, Comando::AlternarPaginaLarga, cx)
        })
    }

    /// A lateral da Estante larga: os destinos, a biblioteca (de todos os serviços, seja qual for o filtro da página) e,
    /// embaixo, a Loja e as Configurações.
    pub(crate) fn ver_lateral(&self, r: Ret, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        caixa(r)
            .id("lateral")
            .rounded(px(RAIO))
            .border_1()
            .border_color(c.linha)
            .bg(c.folha)
            .overflow_hidden()
            .occlude()
            .flex()
            .flex_col()
            .child(self.destinos_em_linhas(cx).p(px(12.0)))
            .child(div().mx(px(16.0)).h(px(1.0)).flex_none().bg(c.linha))
            .child(
                medidas::corpo()
                    .px(px(28.0))
                    .pt(px(16.0))
                    .pb(px(8.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Sua biblioteca"),
            )
            .child(self.salvos_da_lateral(cx))
            .child(self.destinos_do_player_em_linhas(cx).p(px(12.0)).border_t_1().border_color(c.linha))
    }

    /// Uma linha de item: a capa, o nome, o que é e o alfinete.
    pub(super) fn linha_de_item(
        &self,
        id: (&'static str, usize),
        capa: Div,
        titulo: String,
        sub: String,
        fixado: bool,
    ) -> Stateful<Div> {
        let c = self.cores();
        div()
            .id(id)
            .map(|d| self.focavel(d, medidas::RAIO_CARTAO))
            .w_full()
            .h(px(medidas::LINHA))
            .px(px(8.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.0))
            .rounded(px(medidas::RAIO_CARTAO))
            .cursor_pointer()
            .hover(|s| s.bg(c.hover))
            .child(capa)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(super::linha(titulo)))
                    .child(medidas::apoio().text_color(c.texto_2).child(super::linha(sub))),
            )
            .when(fixado, |d| d.child(icone("icones/pin.svg", medidas::ICONE_MIUDO, c.texto_3)))
    }

    /// A linha de um item dos serviços: o que é e, com `com_selo`, de qual serviço. Clicar abre a página dele, ou toca a faixa (com as
    /// outras da `fileira` na fila; sem ela, as de toda a biblioteca).
    pub(super) fn linha_de_item_do_servico(
        &self,
        id: (&'static str, usize),
        item: &Item,
        fixado: bool,
        com_selo: bool,
        fileira: Option<&Arc<Vec<Faixa>>>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let ficha = cartao(item);
        let tipo = match ficha.tipo {
            TipoSalvo::Faixa => "Música",
            TipoSalvo::Album => "Álbum",
            TipoSalvo::Artista => "Artista",
            TipoSalvo::Playlist => "Playlist",
        };
        let mut sub = tipo.to_string();
        if !ficha.redondo && !ficha.sub.is_empty() {
            sub = format!("{sub}, {}", ficha.sub);
        }
        let servicos = if com_selo { vec![item.servico()] } else { Vec::new() };
        let capa =
            self.com_selo(self.capa_do_cartao(&ficha, medidas::CAPA_DA_LINHA), &servicos, medidas::MARCA_DA_LINHA);
        let (alvo, fileira) = (item.clone(), fileira.cloned());
        let (do_menu, da_fileira) = (item.clone(), fileira.clone().unwrap_or_default());
        let menu = move || match &do_menu {
            Item::Faixa(f) => {
                Alvo::Faixa { faixa: Box::new(f.clone()), tambem: Vec::new(), fileira: da_fileira.clone(), fila: None }
            }
            outro => Alvo::Item(Box::new(outro.clone())),
        };
        let linha = self.linha_de_item(id, capa, ficha.titulo, sub, fixado);
        linha.on_mouse_down(MouseButton::Right, self.ao_menu(menu, cx)).on_click(cx.listener(
            move |this, _: &ClickEvent, _, cx| {
                let fileira = fileira.clone().unwrap_or_else(|| {
                    Arc::new(this.biblioteca_junta(None).iter().filter_map(|b| b.item.faixa().cloned()).collect())
                });
                this.abrir_ou_tocar(&alvo, &fileira);
                cx.notify();
            },
        ))
    }

    /// A linha de uma playlist do computador: clicar mostra as músicas dela.
    pub(super) fn linha_de_playlist(
        &self,
        id: (&'static str, usize),
        p: &biblioteca::Playlist,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let capa = self.capa(None, TipoSalvo::Playlist, medidas::CAPA_DA_LINHA, false);
        let alvo = p.id;
        let sub = format!("Playlist, {}", quantas_musicas(p.quantas));
        self.linha_de_item(id, capa, p.nome.clone(), sub, false).on_click(cx.listener(
            move |this, _: &ClickEvent, _, cx| {
                this.abrir_origem(biblioteca::Origem::Playlist(alvo));
                cx.notify();
            },
        ))
    }

    /// O que está na biblioteca: os fixados, as playlists feitas no Player, o que foi salvo e o que está nas contas. Só as
    /// linhas à vista são montadas.
    fn salvos_da_lateral(&self, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let total = self.biblioteca_junta(None).len() + self.local.lista.playlists.len();
        if total == 0 {
            let paginas = self.bibliotecas(None);
            let carregando = paginas.iter().any(|p| matches!(**p, crate::remoto::PaginaRemota::Carregando));
            let frase = if carregando {
                "Carregando a sua biblioteca…"
            } else if paginas.iter().any(|p| matches!(**p, crate::remoto::PaginaRemota::Erro(_))) {
                "A biblioteca da conta não veio. Abra a Biblioteca para tentar de novo."
            } else {
                "O que você curtir e salvar aparece aqui."
            };
            return div()
                .flex_1()
                .px(px(28.0))
                .flex()
                .flex_col()
                .items_start()
                .gap(px(16.0))
                .child(medidas::apoio().text_color(c.texto_2).child(frase))
                .when(!carregando, |d| d.child(self.buscar_musicas("lateral-buscar", cx)))
                .into_any_element();
        }
        uniform_list(
            "lateral-salvos",
            total,
            cx.processor(|this, faixa: Range<usize>, _, cx| {
                faixa.map(|i| this.linha_da_lateral(i, cx)).collect::<Vec<_>>()
            }),
        )
        .flex_1()
        .min_h(px(0.0))
        .px(px(12.0))
        .pb(px(8.0))
        .into_any_element()
    }

    /// A linha `i` da lateral: os fixados, as playlists do Player e o resto.
    fn linha_da_lateral(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let junta = self.biblioteca_junta(None);
        let playlists = &self.local.lista.playlists;
        let fixados = junta.iter().take_while(|b| b.fixado).count();
        let item = if i < fixados {
            junta.get(i)
        } else if let Some(p) = playlists.get(i - fixados) {
            return self.linha_de_playlist(("lateral-playlist", i), p, cx).into_any_element();
        } else {
            junta.get(i - playlists.len())
        };
        match item {
            Some(b) => self
                .linha_de_item_do_servico(("lateral-salvo", i), &b.item, b.fixado, true, None, cx)
                .into_any_element(),
            None => div().h(px(medidas::LINHA)).into_any_element(),
        }
    }

    /// O quadro do visual: por cima dele, a música e os botões. Nada opaco: o visual é desenhado por baixo, na camada.
    /// Numa tira (a página larga, o empilhado) tudo cabe numa linha.
    pub(crate) fn ver_quadro_do_visual(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let a = &self.arranjo;
        let v = a.visual;
        let claro = |alfa: u32| -> gpui::Hsla { rgba(0xffffff00 | alfa).into() };
        let (nome, artista): (SharedString, SharedString) = match self.midia.faixa.as_ref() {
            Some(f) if !f.titulo.is_empty() => (f.titulo.clone().into(), f.artista.clone().into()),
            _ if self.retrato.gate > 0.5 => ("Som do sistema".into(), "O visual segue o que o PC está tocando.".into()),
            _ => ("Modo Escuta".into(), "Toque uma música em qualquer app".into()),
        };
        let quadro = div()
            .id("quadro-do-visual")
            .absolute()
            .left(px(v.x))
            .top(px(v.y))
            .w(px(v.w))
            .h(px(v.h))
            .rounded(px(a.raio_visual))
            .border_1()
            .border_color(c.linha)
            .overflow_hidden()
            .occlude();
        // Sem a camada do visual: o aviso fica no quadro (a página continua usável).
        if let Some(erro) = &self.erro_visual {
            return quadro
                .bg(gpui::black())
                .p(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .child(medidas::apoio().text_color(claro(0xc2)).text_center().child(erro.clone()));
        }
        let botao = |id: &'static str, caminho: &'static str, comando: Comando, cx: &mut Context<Self>| {
            self.botao(
                Botao {
                    id,
                    icone: caminho,
                    tamanho: medidas::FICHA,
                    tamanho_icone: medidas::ICONE_LINHA,
                    raio: medidas::PILULA,
                    cor: claro(0xf2),
                    fundo: Some(rgba(0x16161acc).into()),
                    realce: rgba(0x3a3a40e6).into(),
                    afunda: 0.94,
                },
                cx,
            )
            .on_click(self.ao_clicar(comando, cx))
            .on_hover(self.ao_pairar(id, Some(comando), "", cx))
        };
        let tira = v.h < ALTURA_DA_TIRA;
        // Lado a lado outra vez: só onde a página larga virou o visual em tira.
        let lado_a_lado = (tira && a.modo != Modo::Empilhado)
            .then(|| botao("visual-lado", "icones/lado.svg", Comando::AlternarPaginaLarga, cx));
        let expandir = botao("visual-expandir", "icones/expand.svg", Comando::ExpandirVisual, cx);
        let quadro = quadro.on_mouse_down(MouseButton::Right, self.ao_menu(|| Alvo::Visual, cx)).on_click(cx.listener(
            |this, ev: &ClickEvent, window, cx| {
                if ev.click_count() == 2 {
                    this.executar(Comando::ExpandirVisual, window, cx);
                }
            },
        ));
        if tira {
            return quadro.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .px(px(if a.modo == Modo::Empilhado { 16.0 } else { 24.0 }))
                    .bg(gpui::linear_gradient(
                        90.0,
                        gpui::linear_color_stop(rgba(0x000000b0), 0.0),
                        gpui::linear_color_stop(rgba(0x00000000), 0.75),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(
                                medidas::corpo()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(claro(0xff))
                                    .child(super::linha(nome)),
                            )
                            .child(medidas::apoio().text_color(claro(0xc2)).child(super::linha(artista))),
                    )
                    .children(lado_a_lado)
                    .child(expandir),
            );
        }
        // O título e o artista embaixo; o BPM quando sobra lugar.
        let margem = if v.w >= 360.0 { 24.0 } else { 16.0 };
        let tamanho = super::heroi::tamanho_do_titulo(a.titulo, nome.chars().count());
        let bpm = (v.w >= COM_BPM && self.retrato.bpm > 0.0).then(|| {
            div()
                .flex()
                .flex_none()
                .items_end()
                .gap(px(8.0))
                .child(super::numero(medidas::PAGINA).text_color(claro(0xff)).child(format!("{:.0}", self.retrato.bpm)))
                .child(medidas::apoio().text_color(claro(0xc2)).child("BPM"))
        });
        quadro
            .child(
                div()
                    .absolute()
                    .top(px(16.0))
                    .left(px(16.0))
                    .right(px(16.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        medidas::apoio()
                            .id("visual-nome")
                            .map(|d| self.focavel(d, medidas::PILULA))
                            .h(px(medidas::FICHA))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .rounded(px(medidas::PILULA))
                            .bg(rgba(0x0a0a0c99))
                            .text_color(claro(0xf2))
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .hover(|s| s.bg(rgba(0x1c1c20cc)))
                            .child(self.ajustes.modelo.nome())
                            .child(self.marca("visual-nome"))
                            .on_hover(self.ao_pairar("visual-nome", Some(Comando::ProximoVisual), "", cx))
                            .on_click(self.ao_clicar(Comando::ProximoVisual, cx)),
                    )
                    .child(expandir),
            )
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .pt(px(48.0))
                    .px(px(margem))
                    .pb(px(margem - 4.0))
                    .flex()
                    .items_end()
                    .gap(px(16.0))
                    .bg(gpui::linear_gradient(
                        180.0,
                        gpui::linear_color_stop(rgba(0x00000000), 0.0),
                        gpui::linear_color_stop(rgba(0x000000b0), 1.0),
                    ))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(
                                titulo(tamanho)
                                    // Folga embaixo: sem ela o corte em duas linhas comia a perna do "g" e do "p".
                                    .pb(px((tamanho * 0.08).ceil()))
                                    .text_color(claro(0xff))
                                    .child(super::linhas(nome, 2)),
                            )
                            .child(
                                texto(if v.w >= 360.0 { medidas::DESTAQUE } else { medidas::CORPO })
                                    .text_color(claro(0xc2))
                                    .font_weight(FontWeight::MEDIUM)
                                    .child(super::linha(artista)),
                            ),
                    )
                    .children(bpm),
            )
    }
}

/// O que segura a divisória: parada e arrastando.
const PEGA: f32 = 48.0;
const PEGA_ABERTA: f32 = 96.0;
/// O visual mais baixo que isto é uma tira: tudo numa linha.
const ALTURA_DA_TIRA: f32 = 110.0;
/// Com esta largura, o BPM cabe ao lado do título.
const COM_BPM: f32 = 480.0;
