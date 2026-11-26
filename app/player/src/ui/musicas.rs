//! A aba Pastas da Biblioteca: as músicas das pastas escolhidas, com filtro de texto.
//!
//! - Categorias: "Todas" (cada música uma vez), uma por pasta e uma por playlist.
//! - Visões: as músicas, os álbuns ou os artistas da categoria. Escolher um álbum ou artista filtra a lista.
//! - Clicar numa música toca, e a lista inteira vira a fila. Passando o mouse aparecem: tocar a seguir, pôr no
//!   fim da fila, pôr numa playlist (e, dentro de uma playlist, tirar dela).
//!
//! - Com provedores abertos, o texto do filtro também pode ir para a busca nos serviços.
//!
//! A lista pode ter milhares de linhas e a interface é redesenhada a cada quadro, então só as linhas visíveis
//! são montadas (`uniform_list`).

use super::{RAIO_CONTROLE, com_alfa, icone, medidas, mono, tecla, tempo};
use crate::app::{JanelaPlaylist, Player};
use crate::local::Visao;
use crate::navegacao::{Campo, Rota};
use crate::ui::medidas::px;
use biblioteca::Origem;
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, SharedString, Stateful, div, prelude::*, uniform_list};
use std::ops::Range;

/// De onde é uma linha de música do computador.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ListaDeMusicas {
    Biblioteca,
    /// O que a busca do topo achou.
    Busca,
}

/// Uma janela mais baixa que isto tem títulos de página menores.
const JANELA_BAIXA: f32 = 640.0;

/// Altura de cada música (ou álbum, ou artista) na lista.
const LINHA: f32 = medidas::LINHA;

impl Player {
    /// O título da página: grande, ou do tamanho de uma seção em folha estreita ou janela baixa.
    pub(super) fn titulo_da_pagina(&self, nome: &'static str) -> Div {
        let a = &self.arranjo;
        medidas::pagina(a.heroi.w >= 640.0 && a.janela.h >= JANELA_BAIXA).child(nome)
    }

    /// O alto da Biblioteca, igual em todas as abas: o título e as abas do filtro.
    pub(crate) fn cabeca_da_biblioteca(&self, cx: &mut Context<Self>) -> Div {
        let agora = self.nav.aba();
        let todas = crate::navegacao::abas_da_biblioteca(self.nav.espaco());
        let escolhida = todas.iter().position(|a| *a == agora).unwrap_or(0);
        let mut abas = Vec::new();
        for (i, &aba) in todas.iter().enumerate() {
            let botao = self.aba_sublinhada(("biblioteca-aba", i), aba.nome(), aba == agora).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.abrir_aba_da_biblioteca(aba);
                    cx.notify();
                },
            ));
            abas.push(botao.into_any_element());
        }
        div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(8.0))
            // Janela baixa: as abas bastam para dizer onde se está.
            .when(self.arranjo.janela.h >= JANELA_BAIXA, |d| d.child(self.titulo_da_pagina("Biblioteca")))
            .child(self.abas_sublinhadas("biblioteca-abas", 2_000_001, escolhida, abas))
    }

    /// Botão pequeno de ícone numa linha da lista (só aparece com o mouse na linha, ou sempre com `sempre`).
    pub(crate) fn acao_da_linha(
        &self,
        id: (&'static str, usize),
        caminho: &'static str,
        grupo: &'static str,
        sempre: bool,
    ) -> Stateful<Div> {
        let c = self.cores();
        div()
            .id(id)
            .size(px(36.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(medidas::PILULA))
            .cursor_pointer()
            .hover(|s| s.bg(c.sel))
            .when(!sempre, |d| d.invisible().group_hover(grupo, |s| s.visible()))
            .child(icone(caminho, medidas::ICONE_LINHA, c.texto_2))
    }

    /// Uma música do computador: a posição `i` da lista da Biblioteca ou do que a busca do topo achou.
    pub(super) fn linha_de_musica(&self, de: ListaDeMusicas, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let lista = match de {
            ListaDeMusicas::Biblioteca => &self.local.lista.musicas,
            ListaDeMusicas::Busca => &self.local.lista.achadas,
        };
        let Some(m) = lista.get(i) else { return div().h(px(LINHA)).into_any_element() };
        let esta = self.local.atual().is_some_and(|a| a.caminho == m.caminho);
        let de_quem = match (m.artista.is_empty(), m.album.is_empty()) {
            (false, false) => format!("{}, {}", m.artista, m.album),
            (false, true) => m.artista.clone(),
            (true, false) => m.album.clone(),
            (true, true) => "Artista desconhecido".to_string(),
        };
        let na_busca = de == ListaDeMusicas::Busca;
        let marca: AnyElement = if esta {
            let caminho = if self.local.situacao.tocando { "icones/wave.svg" } else { "icones/pause.svg" };
            icone(caminho, medidas::ICONE_LINHA, c.texto).into_any_element()
        } else {
            mono().text_color(c.texto_3).child((i + 1).to_string()).into_any_element()
        };
        let playlist = match self.local.lista.filtro.origem {
            Origem::Playlist(id) if de == ListaDeMusicas::Biblioteca => Some(id),
            _ => None,
        };
        let (a_seguir, no_fim, numa_lista, tirar) = (m.clone(), m.clone(), m.clone(), m.caminho.clone());
        let esta_linha = m.caminho.clone();
        let da_linha = m.clone();
        let do_menu = move || Player::alvo_da_musica(&da_linha, None);
        let fundo = if esta { c.sel } else { c.hover };
        let acoes = super::acoes_por_cima("musica-linha", fundo, if m.duracao_s > 0.0 { 110.0 } else { 54.0 })
            .child(self.acao_da_linha(("musica-a-seguir", i), "icones/play-next.svg", "musica-linha", false).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.local.tocar_a_seguir(a_seguir.clone());
                    this.avisar(format!("\"{}\" toca a seguir", a_seguir.titulo));
                    cx.notify();
                }),
            ))
            .child(self.acao_da_linha(("musica-na-fila", i), "icones/queue-add.svg", "musica-linha", false).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.local.adicionar_a_fila(vec![no_fim.clone()]);
                    this.avisar(format!("\"{}\" foi para o fim da fila", no_fim.titulo));
                    cx.notify();
                }),
            ))
            .child(self.acao_da_linha(("musica-playlist", i), "icones/playlist.svg", "musica-linha", false).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.abrir_janela_playlist(vec![numa_lista.clone()], false);
                    cx.notify();
                }),
            ))
            .when_some(playlist, |d, id| {
                d.child(self.acao_da_linha(("musica-tirar", i), "icones/x.svg", "musica-linha", false).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.local.tirar_da_playlist(id, tirar.clone());
                        this.avisar_com_desfazer(
                            "Música tirada da playlist (o arquivo continua no disco)",
                            crate::app::Desfazer::NaBiblioteca,
                        );
                        cx.notify();
                    }),
                ))
            });
        div()
            .id((if de == ListaDeMusicas::Busca { "achada" } else { "musica" }, i))
            .group("musica-linha")
            .relative()
            .w_full()
            .h(px(LINHA))
            .px(px(if na_busca { 12.0 } else { 10.0 }))
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(medidas::RAIO_CARTAO))
            .cursor_pointer()
            .when(esta, |d| d.bg(c.sel))
            .when(!esta, |d| d.hover(|s| s.bg(c.hover)))
            // Na busca, ela fica ao lado das dos serviços: com a capa e o selo do computador, sem o número.
            .when(na_busca, |d| {
                let servicos: &[&str] = if self.mistura_servicos() { &[super::selo::COMPUTADOR] } else { &[] };
                let capa = self.capa(None, biblioteca::TipoSalvo::Faixa, medidas::CAPA_DA_LINHA, false);
                d.child(self.com_selo(capa, servicos, medidas::MARCA_DA_LINHA))
            })
            .when(!na_busca, |d| d.child(div().w(px(36.0)).flex_none().flex().justify_center().child(marca)))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(super::linha(m.titulo.clone())))
                    .child(medidas::apoio().text_color(c.texto_2).child(super::linha(de_quem))),
            )
            // Uma faixa de serviço numa playlist do Player diz de onde é.
            .children(
                m.remota
                    .as_ref()
                    .map(|r| super::selo::disco(&self.marca_do_servico(&r.servico), medidas::MARCA_DA_LINHA)),
            )
            .when(m.duracao_s > 0.0, |d| {
                d.child(mono().w(px(44.0)).flex_none().text_right().text_color(c.texto_2).child(tempo(m.duracao_s)))
            })
            .child(acoes)
            .child(self.botao_mais(("musica-mais", i), 36.0, do_menu.clone(), cx).mr(px(-6.0)))
            .on_mouse_down(gpui::MouseButton::Right, self.ao_menu(do_menu, cx))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                match de {
                    ListaDeMusicas::Biblioteca => this.tocar_musica(i, &esta_linha),
                    ListaDeMusicas::Busca => this.tocar_achada(&esta_linha),
                }
                cx.notify();
            }))
            .into_any_element()
    }

    /// Com texto no filtro e provedores abertos, o atalho para buscar nos serviços (a página Buscar, com o mesmo
    /// texto).
    fn nos_servicos(&self, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let texto_ = self.buscas.biblioteca.trim();
        if !self.local.remoto.tem_provedores() || texto_.is_empty() {
            return None;
        }
        let c = self.cores();
        Some(
            div()
                .id("buscar-nos-servicos")
                .h(px(medidas::BOTAO))
                .px(px(10.0))
                .mb(px(8.0))
                .flex()
                .items_center()
                .gap(px(10.0))
                .rounded(px(RAIO_CONTROLE))
                .cursor_pointer()
                .hover(|s| s.bg(c.hover))
                .child(icone("icones/globe.svg", medidas::ICONE_LINHA, c.texto_2))
                .child(
                    medidas::apoio()
                        .flex_1()
                        .min_w(px(0.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(super::linha(format!("Buscar \"{texto_}\" nos serviços"))),
                )
                .child(icone("icones/chev-r.svg", medidas::ICONE_LINHA, c.texto_2))
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    let texto = this.buscas.biblioteca.clone();
                    this.buscar_por(&texto);
                    cx.notify();
                })),
        )
    }

    /// Um álbum ou artista na lista: clicar mostra as músicas dele.
    fn linha_de_grupo(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let Some(g) = self.local.lista.grupos.get(i) else { return div().h(px(LINHA)).into_any_element() };
        let album = self.local.lista.visao == Visao::Albuns;
        let quantas = match g.quantas {
            1 => "1 música".to_string(),
            n => format!("{n} músicas"),
        };
        let detalhe = if g.detalhe.is_empty() { quantas } else { format!("{}, {quantas}", g.detalhe) };
        let nome = g.nome.clone();
        div()
            .id(("grupo", i))
            .w_full()
            .h(px(LINHA))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(medidas::RAIO_CARTAO))
            .cursor_pointer()
            .hover(|s| s.bg(c.hover))
            .child(
                div()
                    .size(px(medidas::CAPA_DA_LINHA))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(if album { medidas::RAIO_CONTROLE } else { medidas::PILULA }))
                    .bg(c.sel)
                    .child(icone(
                        if album { "icones/album.svg" } else { "icones/person.svg" },
                        medidas::ICONE_LINHA,
                        c.texto_2,
                    )),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(super::linha(g.nome.clone())))
                    .child(medidas::apoio().text_color(c.texto_2).child(super::linha(detalhe))),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if album {
                    this.local.filtrar_album(Some(nome.clone()));
                } else {
                    this.local.filtrar_artista(Some(nome.clone()));
                }
                cx.notify();
            }))
            .into_any_element()
    }

    /// Abre a janela de playlist: pôr `musicas` numa playlist, ou só criar uma (`so_criar`).
    pub(crate) fn abrir_janela_playlist(&mut self, musicas: Vec<biblioteca::Musica>, so_criar: bool) {
        self.largar_campo();
        self.janela_playlist = Some(JanelaPlaylist { musicas, nome: String::new(), so_criar, renomear: None });
    }

    /// Abre a janela de playlist para trocar o nome de uma (o texto começa com o nome de agora).
    pub(crate) fn abrir_janela_renomear(&mut self, id: i64, nome: String) {
        self.largar_campo();
        self.janela_playlist = Some(JanelaPlaylist { musicas: Vec::new(), nome, so_criar: true, renomear: Some(id) });
    }

    /// Um campo de texto da página: o da busca (no topo) ou o filtro da Biblioteca. Clicar põe o teclado nele; Esc ou
    /// Enter devolvem os atalhos.
    pub(crate) fn campo_de_texto(&self, campo: Campo, exemplo: &'static str, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let ativo = self.campo_no_teclado() == Some(campo);
        // Fora da busca, o campo do topo só mantém o texto na página que saiu dela.
        let mostra =
            campo == Campo::Biblioteca || self.nav.atual().is_some_and(|r| r == &Rota::Busca || r.item().is_some());
        let valor = if mostra { self.buscas.do_campo(campo).to_string() } else { String::new() };
        let (id, id_limpar) = match campo {
            Campo::Busca => ("campo-busca", "campo-busca-limpar"),
            Campo::Biblioteca => ("campo-biblioteca", "campo-biblioteca-limpar"),
        };
        let cor = if ativo { c.texto } else { c.texto_2 };
        div()
            .id(id)
            .flex_1()
            .min_w(px(96.0))
            .h(px(48.0))
            .relative()
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(14.0))
            .rounded(px(medidas::PILULA))
            .border_1()
            .border_color(if ativo { c.texto } else { c.linha_2 })
            .when(campo == Campo::Busca && self.imersivo(), |d| d.bg(c.painel))
            .when(!ativo, |d| d.hover(|s| s.bg(c.hover)))
            .cursor_text()
            .child(icone("icones/search.svg", medidas::ICONE, cor))
            .when(ativo, |d| d.child(self.receber_texto(cx)))
            .child(Self::conteudo_do_campo(&c, &valor, exemplo, medidas::CORPO, ativo, ativo && self.campo_selecionado))
            .when(valor.is_empty() && !ativo && campo == Campo::Busca && self.arranjo.topo.w >= 760.0, |d| {
                d.child(div().flex().flex_none().gap(px(4.0)).child(tecla(&c, "Ctrl")).child(tecla(&c, "K")))
            })
            .when(!valor.is_empty(), |d| {
                d.child(
                    div()
                        .id(id_limpar)
                        .size(px(28.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(medidas::PILULA))
                        .cursor_pointer()
                        .hover(|s| s.bg(c.sel))
                        .child(icone("icones/x.svg", medidas::ICONE_MIUDO, c.texto_2))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            if campo == Campo::Busca {
                                this.abrir_busca();
                            }
                            this.focar(campo);
                            this.apagar_no_campo(true);
                            cx.stop_propagation();
                            cx.notify();
                        })),
                )
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                if campo == Campo::Busca && this.nav.atual() != Some(&Rota::Busca) {
                    this.abrir_busca();
                } else {
                    this.focar(campo);
                }
                cx.notify();
            }))
    }

    pub(crate) fn ver_musicas(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        // Janela baixa e larga: os controles cabem em duas linhas, e sobra altura para a lista.
        let compacta = a.heroi.w >= 640.0 && a.janela.h < JANELA_BAIXA;
        let largo = a.heroi.w >= 640.0 && !compacta;
        let lado = self.lado_da_pagina();
        let largura = a.heroi.w - 2.0 - 2.0 * lado;
        let lista = &self.local.lista;
        let filtro = &lista.filtro;

        // Em janela estreita, os botões de texto viram só o ícone (com a dica): sobra altura para a lista.
        let pilula = |id: &'static str, caminho: &'static str, rotulo: &'static str, cheia: bool| {
            let b = medidas::corpo()
                .id(id)
                .h(px(medidas::BOTAO))
                .map(|d| if largo { d.px(px(16.0)) } else { d.w(px(medidas::BOTAO)).justify_center() })
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .rounded(px(medidas::PILULA))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer();
            if cheia {
                b.bg(c.inv).text_color(c.inv_texto).hover(|s| s.opacity(0.9)).child(icone(
                    caminho,
                    medidas::ICONE_LINHA,
                    c.inv_texto,
                ))
            } else {
                b.border_1().border_color(c.linha_2).hover(|s| s.bg(c.hover)).child(icone(
                    caminho,
                    medidas::ICONE_LINHA,
                    c.texto,
                ))
            }
            .when(largo, |d| d.child(rotulo))
        };
        let dica = |b: Stateful<Div>, id: &'static str, rotulo: &'static str, cx: &Context<Self>| {
            b.when(!largo, |d| d.child(self.marca(id)).on_hover(self.ao_pairar(id, None, rotulo, cx)))
        };

        let busca = self.campo_de_texto(Campo::Biblioteca, "Filtrar as músicas", cx);

        // As categorias: "Todas", uma por pasta e uma por playlist.
        let categoria = |id: (&'static str, usize),
                         simbolo: Option<&'static str>,
                         nome: SharedString,
                         quantas: u32,
                         escolhida: bool| {
            let cor = if escolhida { c.inv_texto } else { c.texto_2 };
            medidas::apoio()
                .id(id)
                .h(px(medidas::FICHA))
                .px(px(12.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.0))
                .rounded(px(medidas::PILULA))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .when(escolhida, |d| d.bg(c.inv).text_color(c.inv_texto))
                .when(!escolhida, |d| d.bg(c.bloco).text_color(c.texto_2).hover(|s| s.bg(c.hover)))
                .when_some(simbolo, |d, s| d.child(icone(s, medidas::ICONE_MIUDO, cor)))
                .child(nome)
                .child(
                    mono()
                        .text_size(px(medidas::MICRO))
                        .text_color(if escolhida { c.inv_texto } else { c.texto_3 })
                        .child(quantas.to_string()),
                )
        };
        // O X da categoria escolhida: tira a pasta, ou apaga a playlist (com confirmação).
        let tirar = |id: (&'static str, usize)| {
            div()
                .id(id)
                .size(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(medidas::PILULA))
                .cursor_pointer()
                .hover(|s| s.bg(com_alfa(c.inv_texto, 0.15)))
                .child(icone("icones/x.svg", medidas::ICONE_MIUDO, c.inv_texto))
        };
        let mut categorias = div().flex().flex_wrap().items_center().gap(px(8.0)).child(
            categoria(("categoria", 0), None, "Todas".into(), lista.total, filtro.origem == Origem::Todas).on_click(
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.apagar_playlist = None;
                    this.local.mostrar(Origem::Todas);
                    cx.notify();
                }),
            ),
        );
        for (i, (pasta, quantas)) in lista.pastas.iter().enumerate() {
            let nome = pasta
                .file_name()
                .map_or_else(|| pasta.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned());
            let escolhida = filtro.origem == Origem::Pasta(pasta.clone());
            let (alvo, sai) = (pasta.clone(), pasta.clone());
            let mut chip =
                categoria(("categoria-pasta", i), Some("icones/pasta.svg"), nome.into(), *quantas, escolhida).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.apagar_playlist = None;
                        this.local.mostrar(Origem::Pasta(alvo.clone()));
                        cx.notify();
                    }),
                );
            // Só a categoria escolhida mostra o botão de tirar: menos chance de tirar a errada sem querer.
            if escolhida {
                chip = chip.pr(px(4.0)).child(tirar(("pasta-tirar", i)).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.local.remover_pasta(sai.clone());
                        this.avisar_com_desfazer(
                            "Pasta retirada da biblioteca (os arquivos continuam no disco)",
                            crate::app::Desfazer::TirarPasta(sai.clone()),
                        );
                        cx.notify();
                    },
                )));
            }
            categorias = categorias.child(chip);
        }
        for (i, p) in lista.playlists.iter().enumerate() {
            let escolhida = filtro.origem == Origem::Playlist(p.id);
            let id = p.id;
            let mut chip = categoria(
                ("categoria-playlist", i),
                Some("icones/playlist.svg"),
                p.nome.clone().into(),
                p.quantas,
                escolhida,
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.apagar_playlist = None;
                this.local.mostrar(Origem::Playlist(id));
                cx.notify();
            }));
            if escolhida {
                chip = if self.apagar_playlist == Some(id) {
                    chip.pr(px(4.0)).child(
                        medidas::apoio()
                            .id(("playlist-apagar-sim", i))
                            .h(px(medidas::SELO))
                            .px(px(8.0))
                            .flex()
                            .items_center()
                            .rounded(px(medidas::PILULA))
                            .bg(c.alerta)
                            .text_color(c.inv_texto)
                            .cursor_pointer()
                            .child("Apagar?")
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                this.apagar_playlist = None;
                                this.local.apagar_playlist(id);
                                this.avisar_com_desfazer(
                                    "Playlist apagada (as músicas continuam no disco)",
                                    crate::app::Desfazer::NaBiblioteca,
                                );
                                cx.notify();
                            })),
                    )
                } else {
                    let nome = p.nome.clone();
                    chip.pr(px(4.0))
                        .child(
                            medidas::apoio()
                                .id(("playlist-renomear", i))
                                .h(px(medidas::SELO))
                                .px(px(8.0))
                                .flex()
                                .items_center()
                                .rounded(px(medidas::PILULA))
                                .cursor_pointer()
                                .hover(|s| s.bg(com_alfa(c.inv_texto, 0.15)))
                                .child("Renomear")
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.abrir_janela_renomear(id, nome.clone());
                                    cx.notify();
                                })),
                        )
                        .child(tirar(("playlist-apagar", i)).on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| {
                                cx.stop_propagation();
                                this.apagar_playlist = Some(id);
                                cx.notify();
                            },
                        )))
                };
            }
            categorias = categorias.child(chip);
        }
        categorias = categorias.child(
            medidas::apoio()
                .id("playlist-nova")
                .h(px(medidas::FICHA))
                .px(px(12.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.0))
                .rounded(px(medidas::PILULA))
                .border_1()
                .border_color(c.linha_2)
                .text_color(c.texto_2)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.bg(c.hover))
                .child(icone("icones/plus.svg", medidas::ICONE_MIUDO, c.texto_2))
                .child("Playlist")
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.abrir_janela_playlist(Vec::new(), true);
                    cx.notify();
                })),
        );

        // Visão (músicas, álbuns, artistas) e os filtros ativos.
        const VISOES: [(Visao, &str); 3] =
            [(Visao::Musicas, "Músicas"), (Visao::Albuns, "Álbuns"), (Visao::Artistas, "Artistas")];
        let escolhida = VISOES.iter().position(|(v, _)| *v == lista.visao).unwrap_or(0);
        let mut visoes = self.seletor("seletor-visao", largura.min(300.0), VISOES.len(), escolhida);
        for (i, (visao, rotulo)) in VISOES.into_iter().enumerate() {
            visoes = visoes.child(self.opcao(("visao", i), rotulo, visao == lista.visao).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.local.ver(visao);
                    cx.notify();
                },
            )));
        }
        let filtro_ativo = |id: &'static str, simbolo: &'static str, nome: String| {
            medidas::apoio()
                .id(id)
                .h(px(medidas::FICHA))
                .pl(px(10.0))
                .pr(px(4.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.0))
                .rounded(px(medidas::PILULA))
                .bg(c.sel)
                .font_weight(FontWeight::SEMIBOLD)
                .max_w(px(260.0))
                .cursor_pointer()
                .child(icone(simbolo, medidas::ICONE_MIUDO, c.texto))
                .child(div().min_w(px(0.0)).child(super::linha(nome)))
                .child(
                    div()
                        .size(px(24.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(medidas::PILULA))
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/x.svg", medidas::ICONE_MIUDO, c.texto_2)),
                )
        };
        let quantas = lista.musicas.len();
        let fila_toda = lista.musicas.as_ref().clone();
        let tem_algo = !lista.pastas.is_empty() || !lista.playlists.is_empty();
        let (visoes_em_cima, visoes_embaixo) =
            if compacta && tem_algo { (Some(visoes), None) } else { (None, Some(visoes)) };
        let barra = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.0))
            .children(visoes_embaixo)
            .when_some(filtro.artista.clone(), |d, nome| {
                d.child(filtro_ativo("filtro-artista", "icones/person.svg", nome).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.local.filtrar_artista(None);
                        cx.notify();
                    },
                )))
            })
            .when_some(filtro.album.clone(), |d, nome| {
                d.child(filtro_ativo("filtro-album", "icones/album.svg", nome).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.local.filtrar_album(None);
                        cx.notify();
                    },
                )))
            })
            .when(!compacta, |d| d.child(div().flex_1()))
            .when(lista.visao == Visao::Musicas && quantas > 0, |d| {
                d.child(
                    dica(
                        pilula("musicas-tocar-tudo", "icones/play.svg", "Tocar tudo", false),
                        "musicas-tocar-tudo",
                        "Tocar tudo",
                        cx,
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.largar_campo();
                        this.local.tocar_lista(fila_toda.clone(), 0);
                        cx.notify();
                    })),
                )
            });

        let situacao: Option<SharedString> = match &lista.varrendo {
            Some(pasta) => Some(format!("Lendo as músicas de {pasta}...").into()),
            None if filtro.consulta.is_empty() => None,
            None => Some(match quantas {
                1 => "1 música encontrada".into(),
                n => format!("{n} músicas encontradas").into(),
            }),
        };

        let cabeca = div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(if compacta { 8.0 } else { 14.0 }))
            .px(px(lado))
            .child(self.cabeca_da_biblioteca(cx))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.0))
                    .child(busca)
                    .children(visoes_em_cima)
                    .when(!lista.pastas.is_empty(), |d| {
                        d.child(
                            dica(
                                pilula("musicas-atualizar", "icones/history.svg", "Atualizar", false),
                                "musicas-atualizar",
                                "Atualizar",
                                cx,
                            )
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.local.atualizar();
                                cx.notify();
                            })),
                        )
                    })
                    .child(
                        dica(
                            pilula("musicas-pasta", "icones/plus.svg", "Adicionar pasta", true),
                            "musicas-pasta",
                            "Adicionar pasta",
                            cx,
                        )
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.escolher_pasta(cx);
                        })),
                    ),
            )
            .when(tem_algo, |d| {
                if compacta {
                    let linha = div().flex().items_center().gap(px(8.0));
                    d.child(linha.child(categorias.flex_1().min_w(px(0.0))).child(barra.flex_none()))
                } else {
                    d.child(categorias).child(barra)
                }
            })
            .children(situacao.map(|frase| medidas::apoio().text_color(c.texto_3).child(frase)));

        let aviso = |caminho: &'static str, frase: &'static str| {
            div()
                .flex_1()
                .py(px(40.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(12.0))
                .child(icone(caminho, medidas::ICONE_GRANDE, c.texto_3))
                .child(medidas::apoio().max_w(px(420.0)).text_center().text_color(c.texto_2).child(frase))
        };
        let grupos = lista.grupos.len();
        let corpo: AnyElement = if !lista.disponivel {
            aviso(
                "icones/info.svg",
                "A biblioteca está desligada: o Player não conseguiu usar a pasta de dados dele neste computador.",
            )
            .into_any_element()
        } else if !tem_algo {
            aviso(
                "icones/lib.svg",
                "Escolha uma pasta com as suas músicas (MP3, FLAC, WAV, OGG, M4A). O Player lê o que há nela e nas pastas de dentro; os arquivos ficam onde estão.",
            )
            .into_any_element()
        } else if lista.visao != Visao::Musicas {
            if grupos == 0 {
                let frase = if lista.visao == Visao::Albuns {
                    "Nenhum álbum aqui: as músicas não têm o álbum nas etiquetas."
                } else {
                    "Nenhum artista aqui: as músicas não têm o artista nas etiquetas."
                };
                aviso("icones/album.svg", frase).into_any_element()
            } else {
                uniform_list(
                    "grupos-lista",
                    grupos,
                    cx.processor(|this, faixa: Range<usize>, _, cx| {
                        faixa.map(|i| this.linha_de_grupo(i, cx)).collect::<Vec<_>>()
                    }),
                )
                .flex_1()
                .min_h(px(0.0))
                .into_any_element()
            }
        } else if quantas == 0 && lista.varrendo.is_some() {
            aviso("icones/history.svg", "Lendo as músicas...").into_any_element()
        } else if quantas == 0 && !filtro.consulta.is_empty() {
            aviso("icones/search.svg", "Nenhuma música com esse nome.").into_any_element()
        } else if quantas == 0 && matches!(filtro.origem, Origem::Playlist(_)) {
            aviso(
                "icones/playlist.svg",
                "Playlist vazia. Passe o mouse em qualquer música e use o botão de playlist para pôr aqui.",
            )
            .into_any_element()
        } else if quantas == 0 {
            aviso("icones/lib.svg", "Não achei músicas aqui.").into_any_element()
        } else {
            uniform_list(
                "musicas-lista",
                quantas,
                cx.processor(|this, faixa: Range<usize>, _, cx| {
                    faixa.map(|i| this.linha_de_musica(ListaDeMusicas::Biblioteca, i, cx)).collect::<Vec<_>>()
                }),
            )
            .track_scroll(self.rolagem_da_lista())
            .flex_1()
            .min_h(px(0.0))
            .into_any_element()
        };

        let nos_servicos = self.nos_servicos(cx);
        div().size_full().flex().flex_col().child(cabeca).child(
            div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .flex_col()
                .px(px(lado - 10.0))
                .pt(px(12.0))
                .pb(px(12.0))
                .children(nos_servicos)
                .child(corpo),
        )
    }
}
