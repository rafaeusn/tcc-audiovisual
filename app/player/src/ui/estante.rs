//! O layout Estante: a lateral (os espaços, as páginas do espaço e o que foi salvo dele), a barra de ícones da
//! gaveta, o quadro do visual no alto do trilho Tocando (ou a faixa do visual no empilhado) e a navegação de baixo
//! do empilhado. A página fica na folha (o herói) e os módulos no painel, como no Palco.

use super::{caixa, com_alfa, icone, texto};
use crate::app::Player;
use crate::arranjo::{Modo, RAIO, Ret};
use crate::navegacao::{Espaco, Rota, abas_do_espaco};
use crate::paginas::{cartao, rota_do_item};
use biblioteca::TipoSalvo;
use gpui::{ClickEvent, Context, Div, FontWeight, SharedString, Stateful, div, prelude::*, px, rgba};
use nucleo::{Comando, Pagina};
use provedores::{Faixa, Item};
use std::sync::Arc;

/// Quantos salvos a lateral lista (o resto fica na Biblioteca do espaço).
const SALVOS_NA_LATERAL: usize = 60;

/// O ícone de cada página do espaço (o normal e o da página aberta).
fn icone_da_rota(rota: &Rota) -> (&'static str, &'static str) {
    match rota {
        Rota::ParaVoce => ("icones/home.svg", "icones/home-f.svg"),
        Rota::Busca => ("icones/search.svg", "icones/search.svg"),
        _ => ("icones/lib.svg", "icones/lib-f.svg"),
    }
}

/// As páginas da lateral: as abas do espaço, ou só a Biblioteca no computador.
fn paginas_do_espaco(espaco: &Espaco) -> Vec<(Rota, &'static str)> {
    let abas = abas_do_espaco(espaco);
    if abas.is_empty() { vec![(Rota::Biblioteca, "Biblioteca")] } else { abas }
}

impl Player {
    /// A página do espaço está aberta? (A Biblioteca também na Fila e no Histórico, que moram nas abas dela.)
    fn pagina_do_espaco_aberta(&self, rota: &Rota) -> bool {
        match (self.nav.atual(), rota) {
            (Some(Rota::Fila | Rota::Historico), Rota::Biblioteca) => true,
            (Some(atual), _) => atual == rota,
            (None, _) => false,
        }
    }

    /// As regiões da Estante. Enquanto o visual cresce até a janela, elas esmaecem juntas (o visual é desenhado por
    /// baixo de tudo, na camada).
    pub(crate) fn ver_estante(&self, cx: &mut Context<Self>) -> Div {
        let a = self.arranjo;
        let mut regioes =
            div().absolute().left(px(0.0)).top(px(0.0)).size_full().opacity((1.0 - self.expansao).clamp(0.0, 1.0));
        regioes = regioes.child(self.ver_heroi(cx));
        if let Some(r) = a.lateral {
            regioes = regioes.child(self.ver_lateral(r, cx));
        }
        regioes = regioes.child(self.ver_topo(cx)).child(self.ver_dock(cx));
        if let Some(r) = a.trilho {
            regioes = regioes.child(self.ver_trilho_da_estante(r, cx));
        }
        if let Some(r) = a.painel {
            regioes = regioes.child(self.ver_painel(r, false, cx));
        }
        // Na gaveta, o Tocando fica ao lado da página (ela encolhe): P ou o botão do dock fecham.
        if let Some(r) = a.gaveta {
            regioes = regioes.child(self.ver_painel(r, true, cx));
        }
        if a.visual.w >= 2.0 && a.visual.h >= 2.0 {
            regioes = regioes.child(self.ver_quadro_do_visual(cx));
        }
        if let Some(r) = a.nav {
            regioes = regioes.child(self.ver_nav_da_estante(r, cx));
        }
        regioes
    }

    /// A lateral da Estante larga.
    pub(crate) fn ver_lateral(&self, r: Ret, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let paleta = self.paleta_do_espaco();
        let espaco = self.nav.espaco().clone();

        // Os espaços: a marca de cada um num círculo; o aberto ganha o anel na cor dele.
        let mut espacos = div().flex().flex_wrap().gap(px(8.0));
        let mut nome_do_aberto = String::new();
        for (i, (e, nome)) in self.espacos().into_iter().enumerate() {
            let ativo = e == espaco;
            if ativo {
                nome_do_aberto.clone_from(&nome);
            }
            let marca = self.marca_do_espaco(&e, &nome, 24.0);
            espacos = espacos.child(
                div()
                    .id(("lateral-espaco", i))
                    .size(px(36.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .border_2()
                    .border_color(if ativo { paleta.acento } else { com_alfa(c.texto, 0.0) })
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(marca)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.trocar_espaco(e.clone());
                        cx.notify();
                    })),
            );
        }

        // As páginas do espaço.
        let mut paginas = div().flex().flex_col().gap(px(2.0));
        for (i, (rota, nome)) in paginas_do_espaco(&espaco).into_iter().enumerate() {
            let aberta = self.pagina_do_espaco_aberta(&rota);
            let (normal, cheio) = icone_da_rota(&rota);
            paginas = paginas.child(
                texto(14.0)
                    .id(("lateral-pagina", i))
                    .h(px(40.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .rounded(px(10.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .text_color(if aberta { c.texto } else { c.texto_2 })
                    .when(aberta, |d| d.bg(c.sel))
                    .when(!aberta, |d| d.hover(|s| s.bg(c.hover).text_color(c.texto)))
                    .child(icone(if aberta { cheio } else { normal }, 20.0, if aberta { c.texto } else { c.texto_2 }))
                    .child(nome)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.abrir_aba_do_espaco(rota.clone());
                        cx.notify();
                    })),
            );
        }

        let rodape = div()
            .flex()
            .items_center()
            .gap(px(4.0))
            .px(px(12.0))
            .py(px(10.0))
            .border_t_1()
            .border_color(paleta.filete)
            .child(self.botao_icone(
                "lateral-loja",
                "icones/store.svg",
                self.pagina() == Pagina::Loja,
                Comando::AbrirPagina(Pagina::Loja),
                cx,
            ))
            .child(self.botao_icone(
                "lateral-ajustes",
                "icones/sliders.svg",
                self.pagina() == Pagina::Configuracoes,
                Comando::AbrirAjustes,
                cx,
            ))
            .child(self.botao_icone("lateral-editar", "icones/grid.svg", self.edicao, Comando::EditarLayout, cx))
            .child(div().flex_1())
            .child(self.botao_icone("lateral-layout", "icones/layout.svg", false, Comando::TrocarLayout, cx));

        caixa(r)
            .id("lateral")
            .rounded(px(RAIO))
            .border_1()
            .border_color(paleta.filete)
            .bg(paleta.folha)
            .overflow_hidden()
            .occlude()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .p(px(16.0))
                    .child(espacos)
                    .child(
                        texto(18.0).font_weight(FontWeight::BOLD).truncate().child(SharedString::from(nome_do_aberto)),
                    )
                    .child(paginas),
            )
            .child(
                texto(12.0)
                    .px(px(16.0))
                    .pb(px(6.0))
                    .text_color(c.texto_3)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("SUA BIBLIOTECA"),
            )
            .child(self.salvos_da_lateral(&espaco, cx))
            .child(rodape)
    }

    /// O que foi salvo no espaço (os fixados primeiro, como a Biblioteca guarda); no computador, as playlists só
    /// de músicas locais.
    fn salvos_da_lateral(&self, espaco: &Espaco, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let mut lista = div().id("lateral-salvos").flex_1().min_h(px(0.0)).overflow_y_scroll().px(px(8.0)).pb(px(8.0));
        let linha = |id: (&'static str, usize), capa: Div, titulo: String, sub: String, fixado: bool| {
            div()
                .id(id)
                .h(px(52.0))
                .px(px(8.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .rounded(px(10.0))
                .cursor_pointer()
                .hover(|s| s.bg(c.hover))
                .child(capa)
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .child(texto(13.0).font_weight(FontWeight::SEMIBOLD).truncate().child(titulo))
                        .child(texto(12.0).text_color(c.texto_2).truncate().child(sub)),
                )
                .when(fixado, |d| d.child(icone("icones/pin.svg", 14.0, c.texto_3)))
        };
        if let Espaco::Computador = espaco {
            let playlists: Vec<_> = self.local.lista.playlists.iter().filter(|p| p.remotas == 0).collect();
            if playlists.is_empty() {
                return lista.child(
                    texto(12.0).px(px(8.0)).text_color(c.texto_3).child("Suas playlists do computador aparecem aqui."),
                );
            }
            for (i, p) in playlists.into_iter().enumerate() {
                let id = p.id;
                let capa = self.capa(None, TipoSalvo::Playlist, 40.0, false);
                let sub = if p.quantas == 1 { "1 música".to_string() } else { format!("{} músicas", p.quantas) };
                lista = lista.child(linha(("lateral-playlist", i), capa, p.nome.clone(), sub, false).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.local.mostrar(biblioteca::Origem::Playlist(id));
                        this.abrir_rota(Rota::Biblioteca);
                        cx.notify();
                    }),
                ));
            }
            return lista;
        }
        let servico = match espaco {
            Espaco::Servico(s) => Some(s.as_str()),
            _ => None,
        };
        // O que está na conta da pessoa aparece junto com o que ela salvou no Player.
        let junta = self.biblioteca_junta(servico);
        let itens: Vec<(bool, &Item)> = junta.iter().take(SALVOS_NA_LATERAL).map(|b| (b.fixado, &b.item)).collect();
        if itens.is_empty() {
            let paginas = self.bibliotecas(servico);
            let carregando = paginas.iter().any(|p| matches!(**p, crate::remoto::PaginaRemota::Carregando));
            let frase = if carregando {
                "Carregando a sua biblioteca…"
            } else if paginas.iter().any(|p| matches!(**p, crate::remoto::PaginaRemota::Erro(_))) {
                "A biblioteca da conta não veio. Abra a Biblioteca para tentar de novo."
            } else {
                "Curta faixas e salve álbuns, artistas e playlists: eles aparecem aqui."
            };
            return lista.child(texto(12.0).px(px(8.0)).text_color(c.texto_3).child(frase));
        }
        let faixas: Arc<Vec<Faixa>> = Arc::new(itens.iter().filter_map(|(_, i)| i.faixa().cloned()).collect());
        for (i, (fixado, item)) in itens.into_iter().enumerate() {
            let ficha = cartao(item);
            let tipo = match ficha.tipo {
                TipoSalvo::Faixa => "Faixa",
                TipoSalvo::Album => "Álbum",
                TipoSalvo::Artista => "Artista",
                TipoSalvo::Playlist => "Playlist",
            };
            // Em Tudo, o nome do serviço diz de onde é.
            let de_onde = if servico.is_none() {
                format!(" · {}", self.nome_do_servico(&crate::paginas::chave_do_salvo(item).0))
            } else {
                String::new()
            };
            let capa = self.capa(ficha.capa.as_deref(), ficha.tipo, 40.0, ficha.redondo);
            let alvo = item.clone();
            let fileira = faixas.clone();
            lista = lista.child(
                linha(("lateral-salvo", i), capa, ficha.titulo, format!("{tipo}{de_onde}"), fixado).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        match rota_do_item(&alvo) {
                            Some(rota) => this.abrir_rota(rota),
                            None => {
                                if let Item::Faixa(f) = &alvo {
                                    let i = fileira
                                        .iter()
                                        .position(|x| x.id == f.id && x.servico == f.servico)
                                        .unwrap_or(0);
                                    this.tocar_faixas(&fileira, i);
                                }
                            }
                        }
                        cx.notify();
                    }),
                ),
            );
        }
        lista
    }

    /// A barra de ícones da Estante na gaveta: os espaços, as páginas do espaço, a Loja e as Configurações.
    pub(crate) fn ver_trilho_da_estante(&self, r: Ret, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let paleta = self.paleta_do_espaco();
        let espaco = self.nav.espaco().clone();
        let mut coluna = caixa(r).flex().flex_col().items_center().gap(px(6.0)).pt(px(14.0)).pb(px(16.0));
        for (i, (e, nome)) in self.espacos().into_iter().enumerate() {
            let ativo = e == espaco;
            let marca = self.marca_do_espaco(&e, &nome, 24.0);
            coluna = coluna.child(
                div()
                    .id(("trilho-espaco", i))
                    .size(px(40.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .border_2()
                    .border_color(if ativo { paleta.acento } else { com_alfa(c.texto, 0.0) })
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(marca)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.trocar_espaco(e.clone());
                        cx.notify();
                    })),
            );
        }
        coluna = coluna.child(div().w(px(32.0)).h(px(1.0)).my(px(6.0)).bg(c.linha));
        for (i, (rota, _)) in paginas_do_espaco(&espaco).into_iter().enumerate() {
            let aberta = self.pagina_do_espaco_aberta(&rota);
            let (normal, cheio) = icone_da_rota(&rota);
            coluna = coluna.child(
                div()
                    .id(("trilho-pagina", i))
                    .size(px(48.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .rounded(px(14.0))
                    .cursor_pointer()
                    .when(aberta, |d| d.bg(c.sel))
                    .when(!aberta, |d| d.hover(|s| s.bg(c.hover)))
                    .child(icone(if aberta { cheio } else { normal }, 24.0, if aberta { c.texto } else { c.texto_2 }))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.abrir_aba_do_espaco(rota.clone());
                        cx.notify();
                    })),
            );
        }
        coluna
            .child(div().flex_1())
            .child(self.botao_icone(
                "trilho-loja",
                "icones/store.svg",
                self.pagina() == Pagina::Loja,
                Comando::AbrirPagina(Pagina::Loja),
                cx,
            ))
            .child(self.botao_icone(
                "trilho-ajustes",
                "icones/sliders.svg",
                self.pagina() == Pagina::Configuracoes,
                Comando::AbrirAjustes,
                cx,
            ))
    }

    /// O quadro do visual (no alto do trilho Tocando, ou a faixa do empilhado): por cima do visual, o nome dele, o
    /// título da faixa e expandir. Nada opaco: o visual é desenhado por baixo, na camada.
    pub(crate) fn ver_quadro_do_visual(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let a = &self.arranjo;
        let v = a.visual;
        let claro = |alfa: u32| -> gpui::Hsla { rgba(0xffffff00 | alfa).into() };
        let (titulo, artista): (SharedString, SharedString) = match self.midia.faixa.as_ref() {
            Some(f) if !f.titulo.is_empty() => (f.titulo.clone().into(), f.artista.clone().into()),
            _ => ("Modo Escuta".into(), "Toque uma música em qualquer app".into()),
        };
        let faixa = a.modo == Modo::Empilhado;
        // Sem a camada do visual: o aviso fica no quadro (a página continua usável).
        if let Some(erro) = &self.erro_visual {
            return div()
                .id("quadro-do-visual")
                .absolute()
                .left(px(v.x))
                .top(px(v.y))
                .w(px(v.w))
                .h(px(v.h))
                .rounded(px(a.raio_visual))
                .border_1()
                .border_color(c.linha)
                .bg(gpui::black())
                .p(px(14.0))
                .flex()
                .items_center()
                .justify_center()
                .child(texto(13.0).text_color(claro(0xc2)).text_center().child(erro.clone()));
        }
        let expandir = div()
            .id("visual-expandir")
            .size(px(32.0))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .rounded(px(999.0))
            .bg(rgba(0x16161acc))
            .cursor_pointer()
            .hover(|s| s.bg(rgba(0x3a3a40e6)))
            .child(icone("icones/expand.svg", 16.0, claro(0xf2)))
            .child(self.marca("visual-expandir"))
            .on_hover(self.ao_pairar("visual-expandir", Some(Comando::ExpandirVisual), "", cx));
        let textos = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(
                texto(if faixa { 14.0 } else { 16.0 })
                    .font_weight(FontWeight::BOLD)
                    .text_color(claro(0xff))
                    .truncate()
                    .child(titulo),
            )
            .child(texto(12.0).text_color(claro(0xc2)).truncate().child(artista));
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
            .cursor_pointer()
            .occlude()
            .on_click(self.ao_clicar(Comando::ExpandirVisual, cx));
        if faixa {
            // A faixa do empilhado: o título à esquerda sobre um degradê escuro, expandir à direita.
            return quadro.child(
                div()
                    .size_full()
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .px(px(14.0))
                    .bg(gpui::linear_gradient(
                        90.0,
                        gpui::linear_color_stop(rgba(0x000000b0), 0.0),
                        gpui::linear_color_stop(rgba(0x00000000), 0.75),
                    ))
                    .child(textos)
                    .child(expandir),
            );
        }
        quadro
            .child(
                div()
                    .absolute()
                    .top(px(12.0))
                    .left(px(12.0))
                    .right(px(12.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        texto(12.0)
                            .h(px(26.0))
                            .px(px(10.0))
                            .flex()
                            .items_center()
                            .rounded(px(999.0))
                            .bg(rgba(0x0a0a0c99))
                            .text_color(claro(0xf2))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(self.ajustes.modelo.nome()),
                    )
                    .child(expandir),
            )
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .right(px(0.0))
                    .bottom(px(0.0))
                    .pt(px(28.0))
                    .px(px(14.0))
                    .pb(px(12.0))
                    .flex()
                    .bg(gpui::linear_gradient(
                        180.0,
                        gpui::linear_color_stop(rgba(0x00000000), 0.0),
                        gpui::linear_color_stop(rgba(0x000000b0), 1.0),
                    ))
                    .child(textos),
            )
    }

    /// A navegação de baixo da Estante no empilhado: as páginas do espaço e as Configurações.
    pub(crate) fn ver_nav_da_estante(&self, r: Ret, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let mut linha = caixa(r).flex().items_center().gap(px(4.0)).px(px(12.0)).border_t_1().border_color(c.linha);
        let espaco = self.nav.espaco().clone();
        for (i, (rota, nome)) in paginas_do_espaco(&espaco).into_iter().enumerate() {
            let aberta = self.pagina_do_espaco_aberta(&rota);
            let (normal, cheio) = icone_da_rota(&rota);
            let cor = if aberta { c.texto } else { c.texto_2 };
            linha = linha.child(
                div()
                    .id(("estante-nav", i))
                    .flex_1()
                    .h(px(52.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(2.0))
                    .rounded(px(14.0))
                    .cursor_pointer()
                    .when(aberta, |d| d.bg(c.sel))
                    .child(icone(if aberta { cheio } else { normal }, 22.0, cor))
                    .child(texto(12.0).text_color(cor).font_weight(FontWeight::SEMIBOLD).child(nome))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.abrir_aba_do_espaco(rota.clone());
                        cx.notify();
                    })),
            );
        }
        let ajustes = self.pagina() == Pagina::Configuracoes;
        let cor = if ajustes { c.texto } else { c.texto_2 };
        linha.child(
            div()
                .id("estante-nav-ajustes")
                .flex_1()
                .h(px(52.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(2.0))
                .rounded(px(14.0))
                .cursor_pointer()
                .when(ajustes, |d| d.bg(c.sel))
                .child(icone("icones/sliders.svg", 22.0, cor))
                .child(texto(12.0).text_color(cor).font_weight(FontWeight::SEMIBOLD).child("Mais"))
                .on_click(self.ao_clicar(Comando::AbrirAjustes, cx)),
        )
    }
}
