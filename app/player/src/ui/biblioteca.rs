//! A Biblioteca: uma só, com o filtro de serviço no alto. As abas Tudo, Playlists, Álbuns, Artistas e Curtidas mostram o
//! que está nas contas e o que foi salvo no Player (as outras duas são `ui::musicas` e `ui::historico`), em cartões ou em
//! linhas, com um filtro de texto e a ordem.

use super::estante::quantas_musicas;
use super::paginas::SeloDoCartao;
use super::{com_alfa, icone, medidas};
use crate::app::Player;
use crate::navegacao::{AbaDaBiblioteca, Campo, Espaco};
use crate::remoto::{PaginaRemota, PedidoDePagina};
use crate::ui::medidas::px;
use biblioteca::TipoSalvo;
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, SharedString, Stateful, div, prelude::*};
use provedores::Item;

impl Player {
    pub(crate) fn ver_biblioteca(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let aba = self.nav.aba();
        let servico = self.servico_do_filtro();
        let tudo = self.nav.espaco() == &Espaco::Tudo;
        let mut corpo = div()
            .opacity(self.entrada())
            .flex()
            .flex_col()
            .gap(px(20.0))
            .px(px(self.lado_da_pagina()))
            .pb(px(24.0))
            .child(self.cabeca_da_biblioteca(cx))
            .child(self.barra_da_biblioteca(aba, cx));
        // Fora da conta, o convite para entrar fica no alto; o que foi salvo no Player continua embaixo.
        for (k, (s, _)) in self.servicos_no_filtro().iter().enumerate() {
            if let Some(cartao) = self.cartao_da_conta(s, tudo, 200 + k, cx) {
                corpo = corpo.child(cartao);
            }
        }
        // As contas: carregando, ou o motivo de não terem vindo.
        let mut carregando = false;
        for (k, s) in self.servicos_com_biblioteca(servico).into_iter().enumerate() {
            let pedido = PedidoDePagina::Biblioteca { servico: s };
            match self.local.remoto.pagina(&pedido).as_deref() {
                None | Some(PaginaRemota::Carregando) => carregando = true,
                Some(PaginaRemota::Erro(motivo)) => {
                    corpo = corpo.child(self.erro_da_pagina(910 + k, motivo, pedido, cx));
                }
                _ => {}
            }
        }
        let junta = self.biblioteca_junta(servico);
        let (posicoes, faixas) = self.arrumar(&junta, aba.tipo());
        // As playlists feitas no Player valem em Tudo.
        let texto = super::normalizar(self.buscas.biblioteca.trim());
        let com_playlists = tudo && matches!(aba, AbaDaBiblioteca::Tudo | AbaDaBiblioteca::Playlists);
        let playlists: Vec<&biblioteca::Playlist> = self
            .local
            .lista
            .playlists
            .iter()
            .filter(|p| com_playlists && texto.split_whitespace().all(|t| super::normalizar(&p.nome).contains(t)))
            .collect();

        if posicoes.is_empty() && playlists.is_empty() {
            if carregando {
                return corpo.child(self.esqueleto_de_cartoes(6));
            }
            return corpo.child(self.biblioteca_vazia(aba, texto.is_empty(), cx));
        }
        if aba == AbaDaBiblioteca::Curtidas {
            return corpo.child(self.linhas_de_faixas(
                &faixas,
                faixas.len(),
                super::paginas::LinhaDeFaixa { numero: true, selo: tudo, ..Default::default() },
                cx,
            ));
        }
        let mostrar = posicoes.len().min(self.cartoes_a_mostrar);
        let itens = || posicoes.iter().take(mostrar).enumerate().map(|(n, i)| (n, &junta[*i]));
        if self.biblioteca_em_lista {
            let mut linhas = div().flex().flex_col().mx(px(-8.0));
            for (i, p) in playlists.iter().enumerate() {
                linhas = linhas.child(self.linha_de_playlist(("biblioteca-playlist", i), p, cx));
            }
            for (n, b) in itens() {
                linhas = linhas.child(self.linha_de_item_do_servico(
                    ("biblioteca-item", n),
                    &b.item,
                    b.fixado,
                    tudo,
                    Some(&faixas),
                    cx,
                ));
            }
            corpo = corpo.child(linhas);
        } else {
            let mut grade = div().flex().flex_wrap().gap(px(16.0));
            for (i, p) in playlists.iter().enumerate() {
                grade = grade.child(self.cartao_de_playlist(i, p, cx));
            }
            for (n, b) in itens() {
                grade = grade.child(self.cartao_com_alfinete(n, &b.item, b.fixado, &faixas, cx));
            }
            corpo = corpo.child(grade);
        }
        corpo = corpo.when(carregando, |d| {
            d.child(medidas::apoio().text_color(c.texto_2).child("Carregando o resto da sua biblioteca…"))
        });
        corpo.children(self.ver_mais_cartoes("biblioteca-ver-mais", posicoes.len(), cx))
    }

    /// O filtro de texto, a ordem e a troca entre cartões e linhas.
    fn barra_da_biblioteca(&self, aba: AbaDaBiblioteca, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let ordem = self.ordem_da_biblioteca;
        let em_lista = self.biblioteca_em_lista;
        let simbolo = if em_lista { "icones/grade.svg" } else { "icones/lista.svg" };
        let rotulo = if em_lista { "Ver em cartões" } else { "Ver em linhas" };
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.0))
            .child(self.campo_de_texto(Campo::Biblioteca, "Filtrar a biblioteca", cx).max_w(px(400.0)).min_w(px(160.0)))
            .child(
                medidas::apoio()
                    .id("biblioteca-ordem")
                    .h(px(medidas::BOTAO))
                    .px(px(16.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.0))
                    .rounded(px(medidas::PILULA))
                    .border_1()
                    .border_color(c.linha_2)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone("icones/swap.svg", medidas::ICONE_LINHA, c.texto_2))
                    .child(ordem.nome())
                    .child(self.marca("biblioteca-ordem"))
                    .on_hover(self.ao_pairar("biblioteca-ordem", None, "Trocar a ordem", cx))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.ordem_da_biblioteca = this.ordem_da_biblioteca.proxima();
                        cx.notify();
                    })),
            )
            // As Curtidas são músicas: sempre em linhas.
            .when(aba != AbaDaBiblioteca::Curtidas, |d| {
                d.child(
                    div()
                        .id("biblioteca-forma")
                        .size(px(medidas::BOTAO))
                        .flex()
                        .flex_none()
                        .items_center()
                        .justify_center()
                        .rounded(px(medidas::PILULA))
                        .border_1()
                        .border_color(c.linha_2)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone(simbolo, medidas::ICONE_LINHA, c.texto))
                        .child(self.marca("biblioteca-forma"))
                        .on_hover(self.ao_pairar("biblioteca-forma", None, rotulo, cx))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.biblioteca_em_lista = !this.biblioteca_em_lista;
                            cx.notify();
                        })),
                )
            })
    }

    /// Nada para mostrar: o motivo e o que fazer.
    fn biblioteca_vazia(&self, aba: AbaDaBiblioteca, sem_texto: bool, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let onde = self.servico_do_filtro().map(|s| self.nome_do_servico(s));
        let frase = if !sem_texto {
            format!("Nada com \"{}\" aqui.", self.buscas.biblioteca.trim())
        } else {
            let o_que = match aba {
                AbaDaBiblioteca::Playlists => "Suas playlists",
                AbaDaBiblioteca::Albuns => "Os álbuns que você salvar",
                AbaDaBiblioteca::Artistas => "Os artistas que você salvar",
                AbaDaBiblioteca::Curtidas => "As músicas que você curtir",
                _ => "O que você curtir e salvar",
            };
            match onde {
                Some(nome) => format!("{o_que} no {nome} aparecem aqui."),
                None => format!("{o_que} aparecem aqui."),
            }
        };
        div()
            .flex()
            .flex_col()
            .items_start()
            .gap(px(16.0))
            .child(medidas::apoio().text_color(c.texto_2).child(SharedString::from(frase)))
            .when(sem_texto, |d| {
                d.child(self.pilula("biblioteca-buscar", "icones/search.svg", "Buscar músicas", true).on_click(
                    cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.abrir_busca();
                        cx.notify();
                    }),
                ))
            })
    }

    /// Um cartão dos salvos: o fixado mostra o alfinete no canto (fixar e soltar ficam no menu).
    fn cartao_com_alfinete(
        &self,
        n: usize,
        item: &Item,
        fixado: bool,
        fileira: &std::sync::Arc<Vec<provedores::Faixa>>,
        cx: &mut Context<Self>,
    ) -> Div {
        let selo = SeloDoCartao { mostrar: self.mistura_servicos(), tambem: &[] };
        div().relative().child(self.ver_cartao(9_000 + n, item, selo, fileira.clone(), cx)).when(fixado, |d| {
            d.child(
                div()
                    .absolute()
                    .top(px(8.0))
                    .left(px(8.0))
                    .size(px(medidas::MARCA))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(medidas::PILULA))
                    .bg(com_alfa(gpui::black(), 0.6))
                    .child(icone("icones/pin.svg", medidas::ICONE_MIUDO, gpui::white())),
            )
        })
    }

    /// Um cartão simples (uma playlist ou uma pasta do computador): a silhueta, o nome e a linha de baixo.
    pub(super) fn cartao_local(
        &self,
        id: (&'static str, usize),
        capa: AnyElement,
        nome: String,
        sub: String,
    ) -> Stateful<Div> {
        let c = self.cores();
        div()
            .id(id)
            .w(px(self.lado_do_cartao()))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .cursor_pointer()
            .child(capa)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(super::linha(nome)))
                    .child(medidas::apoio().text_color(c.texto_2).child(super::linha(sub))),
            )
    }

    /// O cartão de uma playlist feita no Player.
    pub(super) fn cartao_de_playlist(
        &self,
        i: usize,
        p: &biblioteca::Playlist,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let capa = self.capa(None, TipoSalvo::Playlist, self.lado_do_cartao(), false).into_any_element();
        let alvo = p.id;
        self.cartao_local(("cartao-playlist", i), capa, p.nome.clone(), quantas_musicas(p.quantas)).on_click(
            cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.abrir_origem(biblioteca::Origem::Playlist(alvo));
                cx.notify();
            }),
        )
    }
}
