//! A aba Músicas da Biblioteca: as músicas das pastas escolhidas, com busca.
//!
//! - Categorias: "Todas" (cada música uma vez), uma por pasta e uma por playlist.
//! - Visões: as músicas, os álbuns ou os artistas da categoria. Escolher um álbum ou artista filtra a lista.
//! - Clicar numa música toca, e a lista inteira vira a fila. Passando o mouse aparecem: tocar a seguir, pôr no
//!   fim da fila, pôr numa playlist (e, dentro de uma playlist, tirar dela).
//!
//! - Com provedores instalados, a busca também vai a eles: o bloco "Nos serviços" mostra as faixas que chegam,
//!   acima das músicas do computador. Clicar toca, e os resultados viram a fila.
//!
//! A lista pode ter milhares de linhas e a interface é redesenhada a cada quadro, então só as linhas visíveis
//! são montadas (`uniform_list`).

use super::{RAIO_CONTROLE, com_alfa, icone, mono, tempo, texto, titulo};
use crate::app::{AbaBiblioteca, JanelaPlaylist, Player};
use crate::local::Visao;
use biblioteca::Origem;
use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, SharedString, Stateful, div, prelude::*, px, uniform_list,
};
use nucleo::{Comando, Pagina};
use std::ops::Range;

/// Altura de cada música (ou álbum, ou artista) na lista.
const LINHA: f32 = 52.0;
/// Quantas faixas dos serviços o bloco mostra.
const NOS_SERVICOS: usize = 5;

impl Player {
    /// O seletor Músicas | Fila | Histórico, no alto da Biblioteca.
    pub(crate) fn abas_da_biblioteca(&self, largura: f32, cx: &mut Context<Self>) -> Div {
        const ABAS: [(AbaBiblioteca, &str); 3] = [
            (AbaBiblioteca::Musicas, "Músicas"),
            (AbaBiblioteca::Fila, "Fila"),
            (AbaBiblioteca::Historico, "Histórico"),
        ];
        let escolhida = ABAS.iter().position(|(a, _)| *a == self.aba_biblioteca).unwrap_or(0);
        let mut abas = self.seletor("seletor-biblioteca", largura.min(360.0), ABAS.len(), escolhida);
        for (i, (aba, rotulo)) in ABAS.into_iter().enumerate() {
            abas = abas.child(self.opcao(("biblioteca-aba", i), rotulo, aba == self.aba_biblioteca).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.abrir_aba_da_biblioteca(aba);
                    cx.notify();
                }),
            ));
        }
        abas
    }

    /// Título "Biblioteca", a frase e o botão de fechar: o mesmo em todas as abas.
    pub(crate) fn cabeca_da_biblioteca(&self, largo: bool, frase: &'static str, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        div()
            .flex()
            .items_start()
            .justify_between()
            .gap(px(12.0))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(titulo(if largo { 40.0 } else { 28.0 }).child("Biblioteca"))
                    // Em janela estreita a descrição sai: a altura fica para a lista.
                    .when(largo, |d| d.child(texto(14.0).text_color(c.texto_2).child(frase))),
            )
            .child(self.botao_icone("musicas-fechar", "icones/x.svg", false, Comando::AbrirPagina(Pagina::Inicio), cx))
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
            .size(px(32.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(999.0))
            .cursor_pointer()
            .hover(|s| s.bg(c.sel))
            .when(!sempre, |d| d.invisible().group_hover(grupo, |s| s.visible()))
            .child(icone(caminho, 18.0, c.texto_2))
    }

    fn linha_de_musica(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let Some(m) = self.local.lista.musicas.get(i) else { return div().h(px(LINHA)).into_any_element() };
        let esta = self.local.atual().is_some_and(|a| a.caminho == m.caminho);
        let de_quem = match (m.artista.is_empty(), m.album.is_empty()) {
            (false, false) => format!("{} · {}", m.artista, m.album),
            (false, true) => m.artista.clone(),
            (true, false) => m.album.clone(),
            (true, true) => "Artista desconhecido".to_string(),
        };
        let marca: AnyElement = if esta {
            let caminho = if self.local.situacao.tocando { "icones/wave.svg" } else { "icones/pause.svg" };
            icone(caminho, 16.0, c.texto).into_any_element()
        } else {
            mono().text_color(c.texto_3).child((i + 1).to_string()).into_any_element()
        };
        let playlist = match self.local.lista.filtro.origem {
            Origem::Playlist(id) => Some(id),
            _ => None,
        };
        let (a_seguir, no_fim, numa_lista, tirar) = (m.clone(), m.clone(), m.caminho.clone(), m.caminho.clone());
        let esta_linha = m.caminho.clone();
        let acoes = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(2.0))
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
            .id(("musica", i))
            .group("musica-linha")
            .w_full()
            .h(px(LINHA))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(RAIO_CONTROLE))
            .cursor_pointer()
            .when(esta, |d| d.bg(c.sel))
            .when(!esta, |d| d.hover(|s| s.bg(c.hover)))
            .child(div().w(px(36.0)).flex_none().flex().justify_center().child(marca))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        texto(14.0)
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(SharedString::from(m.titulo.clone())),
                    )
                    .child(texto(12.0).text_color(c.texto_2).truncate().child(de_quem)),
            )
            .child(acoes)
            .when(m.duracao_s > 0.0, |d| {
                d.child(mono().w(px(44.0)).flex_none().text_right().text_color(c.texto_2).child(tempo(m.duracao_s)))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.tocar_musica(i, &esta_linha);
                cx.notify();
            }))
            .into_any_element()
    }

    /// O bloco "Nos serviços": as faixas que os provedores acharam para a busca (`None` sem provedor ou sem busca).
    fn nos_servicos(&self, cx: &mut Context<Self>) -> Option<Div> {
        let remoto = &self.local.remoto;
        if !remoto.tem_provedores() || self.busca_musicas.trim().is_empty() {
            return None;
        }
        let c = self.cores();
        let busca = &remoto.busca;
        let buscando = busca.faltam > 0 || busca.texto != self.busca_musicas.trim();
        let situacao: SharedString = if buscando {
            "Buscando...".into()
        } else if busca.faixas.is_empty() && !busca.erros.is_empty() {
            busca.erros.join(" · ").into()
        } else if busca.faixas.is_empty() {
            "Nada com esse nome.".into()
        } else {
            format!("{} {}", busca.faixas.len(), if busca.faixas.len() == 1 { "faixa" } else { "faixas" }).into()
        };
        let mut bloco = div().flex().flex_col().gap(px(2.0)).pb(px(12.0)).child(
            div()
                .h(px(32.0))
                .px(px(10.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .child(icone("icones/globe.svg", 16.0, c.texto_2))
                .child(texto(13.0).font_weight(FontWeight::SEMIBOLD).child("Nos serviços"))
                .child(texto(12.0).text_color(c.texto_3).truncate().child(situacao)),
        );
        for (i, m) in busca.faixas.iter().take(NOS_SERVICOS).enumerate() {
            let esta = self.local.atual().is_some_and(|a| a.caminho == m.caminho);
            let de_quem = match (&m.remota, m.artista.is_empty()) {
                (Some(r), false) => format!("{} · {}", m.artista, r.fonte),
                (Some(r), true) => r.fonte.clone(),
                (None, _) => m.artista.clone(),
            };
            let marca: AnyElement = if esta {
                let caminho = if self.local.situacao.tocando { "icones/wave.svg" } else { "icones/pause.svg" };
                icone(caminho, 16.0, c.texto).into_any_element()
            } else {
                icone("icones/play.svg", 14.0, c.texto_3).into_any_element()
            };
            let (a_seguir, no_fim) = (m.clone(), m.clone());
            let acoes = div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(2.0))
                .child(
                    self.acao_da_linha(("servico-a-seguir", i), "icones/play-next.svg", "servico-linha", false)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.local.tocar_a_seguir(a_seguir.clone());
                            this.avisar(format!("\"{}\" toca a seguir", a_seguir.titulo));
                            cx.notify();
                        })),
                )
                .child(
                    self.acao_da_linha(("servico-na-fila", i), "icones/queue-add.svg", "servico-linha", false)
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.local.adicionar_a_fila(vec![no_fim.clone()]);
                            this.avisar(format!("\"{}\" foi para o fim da fila", no_fim.titulo));
                            cx.notify();
                        })),
                );
            bloco = bloco.child(
                div()
                    .id(("servico", i))
                    .group("servico-linha")
                    .w_full()
                    .h(px(LINHA))
                    .px(px(10.0))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .rounded(px(RAIO_CONTROLE))
                    .cursor_pointer()
                    .when(esta, |d| d.bg(c.sel))
                    .when(!esta, |d| d.hover(|s| s.bg(c.hover)))
                    .child(div().w(px(36.0)).flex_none().flex().justify_center().child(marca))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(
                                texto(14.0)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .truncate()
                                    .child(SharedString::from(m.titulo.clone())),
                            )
                            .child(texto(12.0).text_color(c.texto_2).truncate().child(de_quem)),
                    )
                    .child(acoes)
                    .when(m.duracao_s > 0.0, |d| {
                        d.child(
                            mono().w(px(44.0)).flex_none().text_right().text_color(c.texto_2).child(tempo(m.duracao_s)),
                        )
                    })
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        // Os resultados dos serviços viram a fila, a partir da clicada.
                        let faixas = this.local.remoto.busca.faixas.clone();
                        this.local.tocar_lista(faixas, i);
                        cx.notify();
                    })),
            );
        }
        Some(bloco)
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
        let detalhe = if g.detalhe.is_empty() { quantas } else { format!("{} · {quantas}", g.detalhe) };
        let nome = g.nome.clone();
        div()
            .id(("grupo", i))
            .w_full()
            .h(px(LINHA))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(RAIO_CONTROLE))
            .cursor_pointer()
            .hover(|s| s.bg(c.hover))
            .child(
                div()
                    .size(px(36.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(if album { 6.0 } else { 999.0 }))
                    .bg(c.sel)
                    .child(icone(if album { "icones/album.svg" } else { "icones/person.svg" }, 18.0, c.texto_2)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        texto(14.0)
                            .font_weight(FontWeight::SEMIBOLD)
                            .truncate()
                            .child(SharedString::from(g.nome.clone())),
                    )
                    .child(texto(12.0).text_color(c.texto_2).truncate().child(detalhe)),
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
    pub(crate) fn abrir_janela_playlist(&mut self, musicas: Vec<std::path::PathBuf>, so_criar: bool) {
        self.buscando_musicas = false;
        self.marcado = None;
        self.janela_playlist = Some(JanelaPlaylist { musicas, nome: String::new(), so_criar });
    }

    pub(crate) fn ver_musicas(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let largo = a.heroi.w >= 640.0;
        let lado = if largo { 32.0 } else { 16.0 };
        let largura = a.heroi.w - 2.0 * lado;
        let lista = &self.local.lista;
        let filtro = &lista.filtro;

        // Em janela estreita, os botões de texto viram só o ícone (com a dica): sobra altura para a lista.
        let pilula = |id: &'static str, caminho: &'static str, rotulo: &'static str, cheia: bool| {
            let b = texto(14.0)
                .id(id)
                .h(px(40.0))
                .map(|d| if largo { d.px(px(16.0)) } else { d.w(px(40.0)).justify_center() })
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer();
            if cheia {
                b.bg(c.inv).text_color(c.inv_texto).hover(|s| s.opacity(0.9)).child(icone(caminho, 18.0, c.inv_texto))
            } else {
                b.border_1().border_color(c.linha_2).hover(|s| s.bg(c.hover)).child(icone(caminho, 18.0, c.texto))
            }
            .when(largo, |d| d.child(rotulo))
        };
        let dica = |b: Stateful<Div>, id: &'static str, rotulo: &'static str, cx: &Context<Self>| {
            b.when(!largo, |d| d.child(self.marca(id)).on_hover(self.ao_pairar(id, None, rotulo, cx)))
        };

        // A busca: clicar põe o teclado nela; Esc ou Enter devolvem os atalhos.
        let ativa = self.buscando_musicas;
        let busca = div()
            .id("musicas-busca")
            .flex_1()
            .min_w(px(160.0))
            .h(px(40.0))
            .relative()
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(14.0))
            .rounded(px(999.0))
            .border_1()
            .border_color(if ativa { c.texto } else { c.linha_2 })
            .cursor_text()
            .child(icone("icones/search.svg", 18.0, if ativa { c.texto } else { c.texto_2 }))
            .when(ativa, |d| d.child(self.receber_texto(cx)))
            .child(Self::conteudo_do_campo(&c, &self.busca_musicas, "Buscar música, artista ou álbum", 14.0, ativa))
            .when(!self.busca_musicas.is_empty(), |d| {
                d.child(
                    div()
                        .id("musicas-limpar")
                        .size(px(24.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(999.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/x.svg", 14.0, c.texto_2))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.busca_musicas.clear();
                            this.marcado = None;
                            this.local.buscar("");
                            this.local.remoto.buscar("");
                            cx.stop_propagation();
                            cx.notify();
                        })),
                )
            })
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.buscando_musicas = true;
                cx.notify();
            }));

        // As categorias: "Todas", uma por pasta e uma por playlist.
        let categoria = |id: (&'static str, usize),
                         simbolo: Option<&'static str>,
                         nome: SharedString,
                         quantas: u32,
                         escolhida: bool| {
            let cor = if escolhida { c.inv_texto } else { c.texto_2 };
            texto(13.0)
                .id(id)
                .h(px(32.0))
                .px(px(12.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.0))
                .rounded(px(999.0))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .when(escolhida, |d| d.bg(c.inv).text_color(c.inv_texto))
                .when(!escolhida, |d| d.bg(c.bloco).text_color(c.texto_2).hover(|s| s.bg(c.hover)))
                .when_some(simbolo, |d, s| d.child(icone(s, 14.0, cor)))
                .child(nome)
                .child(
                    mono()
                        .text_size(px(12.0))
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
                .rounded(px(999.0))
                .cursor_pointer()
                .hover(|s| s.bg(com_alfa(c.inv_texto, 0.15)))
                .child(icone("icones/x.svg", 12.0, c.inv_texto))
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
            let mut chip = categoria(("categoria-pasta", i), Some("icones/lib.svg"), nome.into(), *quantas, escolhida)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.apagar_playlist = None;
                    this.local.mostrar(Origem::Pasta(alvo.clone()));
                    cx.notify();
                }));
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
                        texto(12.0)
                            .id(("playlist-apagar-sim", i))
                            .h(px(24.0))
                            .px(px(8.0))
                            .flex()
                            .items_center()
                            .rounded(px(999.0))
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
                    chip.pr(px(4.0)).child(tirar(("playlist-apagar", i)).on_click(cx.listener(
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
            texto(13.0)
                .id("playlist-nova")
                .h(px(32.0))
                .px(px(12.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.0))
                .rounded(px(999.0))
                .border_1()
                .border_color(c.linha_2)
                .text_color(c.texto_2)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.bg(c.hover))
                .child(icone("icones/plus.svg", 14.0, c.texto_2))
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
            texto(13.0)
                .id(id)
                .h(px(32.0))
                .pl(px(10.0))
                .pr(px(4.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(6.0))
                .rounded(px(999.0))
                .bg(c.sel)
                .font_weight(FontWeight::SEMIBOLD)
                .max_w(px(260.0))
                .cursor_pointer()
                .child(icone(simbolo, 14.0, c.texto))
                .child(div().min_w(px(0.0)).truncate().child(nome))
                .child(
                    div()
                        .size(px(24.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(999.0))
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/x.svg", 12.0, c.texto_2)),
                )
        };
        let quantas = lista.musicas.len();
        let fila_toda = lista.musicas.as_ref().clone();
        let barra = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.0))
            .child(visoes)
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
            .child(div().flex_1())
            .when(lista.visao == Visao::Musicas && quantas > 0, |d| {
                d.child(
                    dica(
                        pilula("musicas-tocar-tudo", "icones/play.svg", "Tocar tudo", false),
                        "musicas-tocar-tudo",
                        "Tocar tudo",
                        cx,
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.buscando_musicas = false;
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

        let tem_algo = !lista.pastas.is_empty() || !lista.playlists.is_empty();
        let cabeca = div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(14.0))
            .px(px(lado))
            .pt(px(lado.min(24.0)))
            .child(self.cabeca_da_biblioteca(largo, "As músicas das suas pastas. Os arquivos ficam onde estão.", cx))
            .child(self.abas_da_biblioteca(largura, cx))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.0))
                    .child(busca)
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
            .when(tem_algo, |d| d.child(categorias).child(barra))
            .children(situacao.map(|frase| texto(12.0).text_color(c.texto_3).child(frase)));

        let aviso = |caminho: &'static str, frase: &'static str| {
            div()
                .flex_1()
                .py(px(40.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(12.0))
                .child(icone(caminho, 28.0, c.texto_3))
                .child(texto(14.0).max_w(px(420.0)).text_center().text_color(c.texto_2).child(frase))
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
                "Escolha uma pasta com as suas músicas (MP3, FLAC, WAV, OGG, M4A). O Player lê o que há nela e nas pastas de dentro.",
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
                    faixa.map(|i| this.linha_de_musica(i, cx)).collect::<Vec<_>>()
                }),
            )
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
