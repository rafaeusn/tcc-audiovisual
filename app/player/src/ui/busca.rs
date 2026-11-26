//! A página Buscar: o que o texto do campo do topo achou nos serviços do filtro e no computador, em abas (principais,
//! músicas, álbuns, artistas, playlists) e os comandos do Player. Com o campo vazio, as buscas recentes e o que foi
//! aberto há pouco.

use super::paginas::{LINHA, LinhaDeFaixa, SeloDoCartao};
use super::{com_alfa, icone, medidas};
use crate::app::Player;
use crate::juntar::{BuscaJunta, Junto};
use crate::navegacao::{Campo, Espaco, so_comandos, texto_para_buscar};
use crate::paginas::{AbaDaBusca, aba_da_busca, cartao, servico_buscando};
use crate::ui::medidas::px;
use crate::ui::musicas::ListaDeMusicas;
use biblioteca::TipoSalvo;
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, Stateful, div, prelude::*};
use provedores::{Faixa, Item};
use std::sync::Arc;

/// Quantas músicas aparecem ao lado do melhor resultado.
const MUSICAS_NO_TOPO: usize = 5;
/// Com a folha pelo menos desta largura, o melhor resultado (36% da largura) e as músicas ficam lado a lado.
const LADO_A_LADO: f32 = 820.0;
/// Quantas músicas do computador aparecem nas Principais, e quantos itens em "Abertos há pouco".
const DO_COMPUTADOR: usize = 2;
const ABERTOS: usize = 12;

impl Player {
    pub(crate) fn ver_busca(&self, cx: &mut Context<Self>) -> Div {
        let texto = self.buscas.servicos.trim();
        let mut corpo =
            div().opacity(self.entrada()).flex().flex_col().gap(px(24.0)).px(px(self.lado_da_pagina())).pb(px(24.0));
        // Janela estreita demais para o campo do topo: ele fica aqui.
        if !self.campo_no_topo() {
            corpo = corpo.child(div().flex().child(self.campo_de_texto(Campo::Busca, "Buscar", cx)));
        }
        if texto.is_empty() {
            return corpo.child(self.busca_vazia(cx));
        }
        let aba = aba_da_busca(texto, self.aba_da_busca);
        // Com `>`, é uma lista de comandos: as abas não valem.
        if so_comandos(texto).is_none() {
            corpo = corpo.child(self.abas_da_busca(aba, cx));
        }
        if aba == AbaDaBusca::Comandos {
            return corpo.child(self.comandos_achados(true, cx));
        }
        corpo = corpo.child(self.achados(aba, cx));
        if aba == AbaDaBusca::Principais && !self.comandos_da_busca().is_empty() {
            corpo = corpo.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(medidas::secao().child("Comandos"))
                    .child(self.comandos_achados(false, cx)),
            );
        }
        corpo
    }

    fn abas_da_busca(&self, agora: AbaDaBusca, cx: &mut Context<Self>) -> Stateful<Div> {
        let mut abas = Vec::new();
        for aba in AbaDaBusca::TODAS {
            let botao = self.aba_sublinhada(("busca-aba", aba as usize), aba.nome(), aba == agora).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.aba_da_busca = aba;
                    this.comando_escolhido = 0;
                    cx.notify();
                }),
            );
            abas.push(botao.into_any_element());
        }
        self.abas_sublinhadas("busca-abas", 2_000_002, agora as usize, abas)
    }

    /// Os comandos que combinam. Na aba deles, as setas escolhem e o Enter executa.
    fn comandos_achados(&self, escolhendo: bool, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let comandos = self.comandos_da_busca();
        if comandos.is_empty() {
            return div().child(medidas::apoio().text_color(c.texto_2).child("Nenhum comando com esse nome."));
        }
        let mut lista = div().flex().flex_col();
        for (i, comando) in comandos.into_iter().enumerate() {
            let escolhido = escolhendo && i == self.comando_escolhido;
            lista = lista.child(
                self.linha_de_comando(i, comando, escolhido, self.arranjo.heroi.w)
                    .when(escolhido, |d| d.child(self.marca("busca-comando-escolhido")))
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.executar_da_busca(comando, window, cx);
                        cx.notify();
                    })),
            );
        }
        lista
    }

    /// Sem texto: as buscas recentes e o que foi aberto há pouco.
    fn busca_vazia(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let mut corpo = div().flex().flex_col().gap(px(32.0)).child(self.titulo_da_pagina("Buscar"));
        let recentes = &self.ajustes.buscas_recentes;
        if !recentes.is_empty() {
            let mut fichas = div().flex().flex_wrap().gap(px(8.0));
            for (i, texto) in recentes.iter().enumerate() {
                let alvo = texto.clone();
                fichas = fichas.child(
                    medidas::apoio()
                        .id(("busca-recente", i))
                        .h(px(medidas::BOTAO))
                        .max_w(px(280.0))
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(medidas::PILULA))
                        .border_1()
                        .border_color(c.linha_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/clock.svg", medidas::ICONE_LINHA, c.texto_2))
                        .child(div().min_w(px(0.0)).child(super::linha(texto.clone())))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.buscar_por(&alvo);
                            cx.notify();
                        })),
                );
            }
            let limpar = medidas::apoio()
                .id("busca-limpar-recentes")
                .h(px(medidas::FICHA))
                .px(px(12.0))
                .flex()
                .items_center()
                .rounded(px(medidas::PILULA))
                .text_color(c.texto_2)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.bg(c.hover).text_color(c.texto))
                .child("Limpar")
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.ajustes.buscas_recentes.clear();
                    this.salvar();
                    cx.notify();
                }));
            corpo = corpo.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(medidas::secao().child("Buscas recentes"))
                            .child(limpar),
                    )
                    .child(fichas),
            );
        }
        // Do filtro: num serviço, os dele; no computador, nenhum (o que se abre lá são as pastas).
        let servico = self.servico_do_filtro();
        let abertos: Vec<Junto> = self
            .local
            .lista
            .recentes
            .iter()
            .filter(|_| self.nav.espaco() != &Espaco::Computador)
            .filter(|i| servico.is_none_or(|s| i.servico() == s))
            .take(ABERTOS)
            .cloned()
            .map(Junto::so)
            .collect();
        if !abertos.is_empty() {
            let faixas = Arc::new(abertos.iter().filter_map(|j| j.item.faixa().cloned()).collect());
            corpo = corpo.child(self.ver_prateleira(990, "Abertos há pouco", None, &abertos, &faixas, cx));
        }
        corpo.child(medidas::apoio().text_color(c.texto_2).child(
            "Busque músicas, álbuns, artistas e playlists de todos os serviços e deste PC. Comece com > para executar um comando.",
        ))
    }

    /// O que os serviços do filtro e o computador acharam, junto, na aba escolhida.
    fn achados(&self, aba: AbaDaBusca, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let filtro = self.nav.espaco();
        let tudo = self.mistura_servicos();
        let servicos = self.servicos_no_filtro();
        let consulta = texto_para_buscar(&self.buscas.servicos).to_string();
        let mut corpo = div().flex().flex_col().gap(px(32.0));
        // Fora da conta, o convite para entrar (a busca do serviço vem com a conta).
        for (k, (s, _)) in servicos.iter().enumerate() {
            if let Some(cartao) = self.cartao_da_conta(s, tudo, 100 + k, cx) {
                corpo = corpo.child(cartao);
            }
        }
        // A busca de agora é de outro texto (a pausa na digitação ainda não passou): o que está na tela é velho e
        // aparece apagado até o novo chegar.
        let busca = &self.local.remoto.busca;
        let velha = !servicos.is_empty() && busca.texto != consulta;
        let esperando: Vec<&str> = servicos
            .iter()
            .filter(|(s, _)| servico_buscando(&busca.texto, &busca.respondidos, &consulta, s))
            .map(|(_, nome)| nome.as_str())
            .collect();
        let ids: Vec<String> = servicos.iter().map(|(s, _)| s.clone()).collect();
        let b = if busca.texto.is_empty() { Arc::default() } else { self.busca_junta(&ids) };
        let do_computador =
            if matches!(filtro, Espaco::Servico(_)) { 0 } else { self.achadas_no_computador(&consulta) };
        let playlists = if matches!(filtro, Espaco::Servico(_)) { Vec::new() } else { self.playlists_com(&consulta) };
        let tem = match aba {
            AbaDaBusca::Principais | AbaDaBusca::Comandos => {
                b.melhor.is_some() || do_computador > 0 || !playlists.is_empty()
            }
            AbaDaBusca::Musicas => !b.musicas.is_empty() || do_computador > 0,
            AbaDaBusca::Albuns => !b.albuns.is_empty(),
            AbaDaBusca::Artistas => !b.artistas.is_empty(),
            AbaDaBusca::Playlists => !b.playlists.is_empty() || !playlists.is_empty(),
        };
        if tem {
            corpo =
                corpo.child(self.resultados(&b, aba, do_computador, &playlists, cx).when(velha, |d| d.opacity(0.5)));
        }
        if !esperando.is_empty() {
            let frase = format!("Buscando no {}…", esperando.join(" e no "));
            corpo = corpo
                .child(medidas::apoio().text_color(c.texto_2).child(frase))
                .when(!tem, |d| d.child(self.esqueleto_de_cartoes(5)));
        } else if !tem {
            let abrindo = self.local.remoto.provedores.iter().any(|p| p.situacao == crate::remoto::Situacao::Abrindo);
            let frase = match (filtro, aba) {
                (Espaco::Computador, AbaDaBusca::Albuns | AbaDaBusca::Artistas) => {
                    "Os álbuns e os artistas do computador estão na Biblioteca, na aba Pastas.".to_string()
                }
                (Espaco::Computador, _) => "Nada com esse nome neste PC.".to_string(),
                _ if servicos.is_empty() && abrindo => "Abrindo os serviços…".to_string(),
                _ if servicos.is_empty() => {
                    "Nenhum serviço aberto. Ligue um provedor na Loja, na aba Provedores.".to_string()
                }
                _ => format!("Nada com \"{consulta}\" aqui."),
            };
            corpo = corpo.child(medidas::apoio().text_color(c.texto_2).child(frase));
        }
        // O serviço que não respondeu direito: o motivo, embaixo do que os outros acharam.
        for erro in &busca.erros {
            if !velha && servicos.iter().any(|(_, nome)| erro.starts_with(&format!("{nome}:"))) {
                corpo = corpo.child(medidas::apoio().text_color(c.texto_2).child(erro.clone()));
            }
        }
        corpo
    }

    /// Quantas músicas do computador batem (a lista pode ser do texto de antes: a resposta leva um instante).
    fn achadas_no_computador(&self, consulta: &str) -> usize {
        let lista = &self.local.lista;
        if lista.procura == consulta { lista.achadas.len() } else { 0 }
    }

    /// As playlists feitas no Player com o texto no nome.
    fn playlists_com(&self, consulta: &str) -> Vec<&biblioteca::Playlist> {
        let texto = super::normalizar(consulta);
        let todas = self.local.lista.playlists.iter();
        todas.filter(|p| texto.split_whitespace().all(|t| super::normalizar(&p.nome).contains(t))).collect()
    }

    /// As músicas do computador e as dos serviços numa lista só, até `limite`. As do computador vêm primeiro: são
    /// poucas, e são da pessoa.
    fn musicas_achadas(&self, b: &BuscaJunta, do_computador: usize, limite: usize, cx: &mut Context<Self>) -> Div {
        let tudo = self.mistura_servicos();
        let mut linhas = div().flex().flex_col().mx(px(-12.0)).min_h(px(LINHA));
        for i in 0..do_computador {
            linhas = linhas.child(self.linha_de_musica(ListaDeMusicas::Busca, i, cx));
        }
        for i in 0..b.musicas.len().min(limite.saturating_sub(do_computador)) {
            let como = LinhaDeFaixa { selo: tudo, tambem: &b.musicas[i].tambem, ..Default::default() };
            linhas = linhas.child(self.linha_de_faixa(i, i, &b.faixas, como, cx));
        }
        linhas
    }

    fn resultados(
        &self,
        b: &BuscaJunta,
        aba: AbaDaBusca,
        do_computador: usize,
        playlists: &[&biblioteca::Playlist],
        cx: &mut Context<Self>,
    ) -> Div {
        let tudo = self.mistura_servicos();
        let sem_faixas: Arc<Vec<Faixa>> = Arc::default();
        let grade = |itens: &[Junto], cx: &mut Context<Self>| {
            let mut grade = div().flex().flex_wrap().gap(px(16.0));
            for (i, j) in itens.iter().enumerate() {
                let selo = SeloDoCartao { mostrar: tudo, tambem: &j.tambem };
                grade = grade.child(self.ver_cartao(5_000_000 + i, &j.item, selo, sem_faixas.clone(), cx));
            }
            grade
        };
        let do_player = |cx: &mut Context<Self>| {
            let mut linhas = div().flex().flex_col().mx(px(-8.0));
            for (i, p) in playlists.iter().enumerate() {
                linhas = linhas.child(self.linha_de_playlist(("busca-playlist", i), p, cx));
            }
            linhas
        };
        match aba {
            AbaDaBusca::Musicas => self.musicas_achadas(b, do_computador, usize::MAX, cx),
            AbaDaBusca::Albuns => grade(&b.albuns, cx),
            AbaDaBusca::Artistas => grade(&b.artistas, cx),
            AbaDaBusca::Playlists => {
                let corpo = div().flex().flex_col().gap(px(24.0));
                corpo.when(!playlists.is_empty(), |d| d.child(do_player(cx))).child(grade(&b.playlists, cx))
            }
            AbaDaBusca::Principais | AbaDaBusca::Comandos => {
                let lado_a_lado = self.arranjo.heroi.w >= LADO_A_LADO;
                let mut topo = div().flex().gap(px(32.0)).when(!lado_a_lado, |d| d.flex_col());
                if let Some(melhor) = &b.melhor {
                    topo = topo.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .when(lado_a_lado, |d| {
                                d.w(px((self.arranjo.heroi.w * 0.36).clamp(240.0, 460.0))).flex_none()
                            })
                            .child(medidas::secao().child("Melhor resultado"))
                            .child(self.melhor_resultado(melhor, &b.faixas, !lado_a_lado, cx)),
                    );
                }
                let locais = do_computador.min(DO_COMPUTADOR);
                if !b.musicas.is_empty() || locais > 0 {
                    // Com mais do que cabe, o título leva à aba Músicas.
                    let mais = b.musicas.len() + do_computador > MUSICAS_NO_TOPO;
                    topo = topo.child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(self.titulo_com_mais("Músicas", AbaDaBusca::Musicas, mais, cx))
                            .child(self.musicas_achadas(b, locais, MUSICAS_NO_TOPO, cx)),
                    );
                }
                let mut tudo_ = div().flex().flex_col().gap(px(28.0)).child(topo);
                if !playlists.is_empty() {
                    tudo_ = tudo_.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(medidas::secao().child("Suas playlists"))
                            .child(do_player(cx)),
                    );
                }
                for (j, (nome, itens)) in
                    [("Artistas", &b.artistas), ("Álbuns", &b.albuns), ("Playlists", &b.playlists)]
                        .into_iter()
                        .enumerate()
                {
                    if !itens.is_empty() {
                        tudo_ = tudo_.child(self.ver_prateleira(1000 + j, nome, None, itens, &sem_faixas, cx));
                    }
                }
                tudo_
            }
        }
    }

    /// O título de uma parte das Principais, com "Ver tudo" quando a aba tem mais.
    fn titulo_com_mais(&self, nome: &'static str, aba: AbaDaBusca, mais: bool, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        div().flex().items_center().justify_between().child(medidas::secao().child(nome)).when(mais, |d| {
            d.child(
                medidas::apoio()
                    .id("busca-ver-tudo")
                    .h(px(34.0))
                    .flex()
                    .items_center()
                    .text_color(c.texto_2)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|s| s.text_color(c.texto))
                    .child("Ver tudo")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.aba_da_busca = aba;
                        this.rolagem_da_pagina().set_offset(gpui::Point::default());
                        cx.notify();
                    })),
            )
        })
    }

    /// O cartão grande do melhor resultado: clicar abre a página dele, ou toca a faixa (com as outras na fila).
    fn melhor_resultado(
        &self,
        junto: &Junto,
        faixas: &Arc<Vec<Faixa>>,
        em_linha: bool,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let item = &junto.item;
        let servicos = if self.mistura_servicos() { junto.servicos() } else { Vec::new() };
        let c = self.cores();
        let ficha = cartao(item);
        let tipo = match ficha.tipo {
            TipoSalvo::Faixa => "Música",
            TipoSalvo::Album => "Álbum",
            TipoSalvo::Artista => "Artista",
            TipoSalvo::Playlist => "Playlist",
        };
        let sub = if ficha.sub.is_empty() || ficha.tipo == TipoSalvo::Artista {
            tipo.to_string()
        } else {
            format!("{tipo}, {}", ficha.sub)
        };
        let (item, faixas) = (item.clone(), faixas.clone());
        let titulo_: AnyElement = match &item {
            // O título da faixa abre a página dela (o resto do cartão toca).
            Item::Faixa(_) => {
                let pagina = crate::navegacao::Rota::Faixa(item.clone());
                medidas::secao()
                    .id("busca-melhor-titulo")
                    .hover(|s| s.underline())
                    .child(super::linhas(ficha.titulo.clone(), 2))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.abrir_rota(pagina.clone());
                        cx.notify();
                    }))
                    .into_any_element()
            }
            _ => medidas::secao().child(super::linhas(ficha.titulo.clone(), 2)).into_any_element(),
        };
        let do_menu = super::paginas::menu_do_cartao(&item, &junto.tambem, &faixas);
        let (alvo, fila) = (item.clone(), faixas.clone());
        let tocar = div()
            .id("busca-melhor-tocar")
            .size(px(52.0))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .rounded(px(medidas::PILULA))
            .bg(c.inv)
            .cursor_pointer()
            .hover(|s| s.opacity(0.9))
            .child(icone("icones/play.svg", medidas::ICONE, c.inv_texto))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.tocar_item(&alvo, &fila);
                cx.notify();
            }));
        // Ao lado das músicas, o cartão é uma coluna; sozinho na largura, uma linha.
        let capa = self.com_selo(
            self.capa_do_cartao(&ficha, 112.0).when(!ficha.redondo, |d| d.shadow_md()),
            &servicos,
            medidas::MARCA,
        );
        let textos = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(titulo_)
            .child(medidas::apoio().text_color(c.texto_2).child(super::linha(sub)));
        div()
            .id("busca-melhor")
            .flex()
            .gap(px(20.0))
            .p(px(20.0))
            .rounded(px(medidas::RAIO_CARTAO))
            .bg(com_alfa(c.texto, 0.05))
            .cursor_pointer()
            .hover(|s| s.bg(com_alfa(c.texto, 0.09)))
            .map(|d| {
                if em_linha {
                    d.items_center().child(capa).child(textos).child(tocar)
                } else {
                    let pe = div().flex().items_end().gap(px(16.0)).child(textos).child(tocar);
                    d.flex_col().child(div().flex().child(capa)).child(pe)
                }
            })
            .on_mouse_down(gpui::MouseButton::Right, self.ao_menu(do_menu, cx))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.abrir_ou_tocar(&item, &faixas);
                cx.notify();
            }))
    }
}
