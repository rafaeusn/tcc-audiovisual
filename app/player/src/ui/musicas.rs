//! A aba Músicas da Biblioteca: as músicas das pastas escolhidas, com busca. Cada pasta é uma categoria (como
//! uma playlist) e "Todas" junta as pastas sem repetir música. Clicar numa música toca; as que vêm depois dela
//! na lista viram a fila.
//!
//! A lista pode ter milhares de linhas e a interface é redesenhada a cada quadro, então só as linhas visíveis
//! são montadas (`uniform_list`).

use super::{RAIO_CONTROLE, com_alfa, icone, mono, tempo, texto, titulo};
use crate::app::{AbaBiblioteca, Player};
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, SharedString, div, prelude::*, px, uniform_list};
use nucleo::{Comando, Pagina};
use std::ops::Range;

/// Altura de cada música na lista.
const LINHA: f32 = 52.0;

impl Player {
    /// O seletor Músicas | Histórico, no alto da Biblioteca.
    pub(crate) fn abas_da_biblioteca(&self, largura: f32, cx: &mut Context<Self>) -> Div {
        const ABAS: [(AbaBiblioteca, &str); 2] =
            [(AbaBiblioteca::Musicas, "Músicas"), (AbaBiblioteca::Historico, "Histórico")];
        let escolhida = ABAS.iter().position(|(a, _)| *a == self.aba_biblioteca).unwrap_or(0);
        let mut abas = self.seletor("seletor-biblioteca", largura.min(280.0), ABAS.len(), escolhida);
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
        div()
            .id(("musica", i))
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
            .when(m.duracao_s > 0.0, |d| d.child(mono().flex_none().text_color(c.texto_2).child(tempo(m.duracao_s))))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.tocar_musica(i);
                cx.notify();
            }))
            .into_any_element()
    }

    pub(crate) fn ver_musicas(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let largo = a.heroi.w >= 640.0;
        let lado = if largo { 32.0 } else { 16.0 };
        let largura = a.heroi.w - 2.0 * lado;
        let lista = &self.local.lista;

        let pilula = |id: &'static str, caminho: &'static str, rotulo: &'static str, cheia: bool| {
            let b = texto(14.0)
                .id(id)
                .h(px(40.0))
                .px(px(16.0))
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
            .child(rotulo)
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
                            cx.stop_propagation();
                            cx.notify();
                        })),
                )
            })
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.buscando_musicas = true;
                cx.notify();
            }));

        // As categorias: "Todas" (cada música uma vez) e uma por pasta.
        let categoria = |id: (&'static str, usize), nome: SharedString, quantas: u32, escolhida: bool| {
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
                .child(nome)
                .child(
                    mono()
                        .text_size(px(12.0))
                        .text_color(if escolhida { c.inv_texto } else { c.texto_3 })
                        .child(quantas.to_string()),
                )
        };
        let mut categorias = div().flex().flex_wrap().items_center().gap(px(8.0)).child(
            categoria(("categoria", 0), "Todas".into(), lista.total, lista.pasta.is_none()).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.local.mostrar(None);
                    cx.notify();
                },
            )),
        );
        for (i, (pasta, quantas)) in lista.pastas.iter().enumerate() {
            let nome = pasta
                .file_name()
                .map_or_else(|| pasta.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned());
            let escolhida = lista.pasta.as_ref() == Some(pasta);
            let (alvo, tirar) = (pasta.clone(), pasta.clone());
            let mut chip = categoria(("categoria", i + 1), nome.into(), *quantas, escolhida).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.local.mostrar(Some(alvo.clone()));
                    cx.notify();
                },
            ));
            // Só a pasta escolhida mostra o botão de tirar: menos chance de tirar a pasta errada sem querer.
            if escolhida {
                chip = chip.pr(px(4.0)).child(
                    div()
                        .id(("pasta-tirar", i))
                        .size(px(24.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(999.0))
                        .cursor_pointer()
                        .hover(|s| s.bg(com_alfa(c.inv_texto, 0.15)))
                        .child(icone("icones/x.svg", 12.0, c.inv_texto))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.local.remover_pasta(tirar.clone());
                            this.avisar("Pasta retirada da biblioteca (os arquivos continuam no disco)");
                            cx.notify();
                        })),
                );
            }
            categorias = categorias.child(chip);
        }
        let quantas_aqui = match &lista.pasta {
            Some(p) => lista.pastas.iter().find(|(q, _)| q == p).map_or(0, |(_, n)| *n),
            None => lista.total,
        };
        let situacao: Option<SharedString> = match &lista.varrendo {
            Some(pasta) => Some(format!("Lendo as músicas de {pasta}...").into()),
            None if lista.consulta.is_empty() => None,
            None => Some(format!("{} de {} músicas", lista.musicas.len(), quantas_aqui).into()),
        };

        let cabeca = div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(14.0))
            .px(px(lado))
            .pt(px(lado.min(24.0)))
            .child(
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
                            .child(
                                texto(14.0)
                                    .text_color(c.texto_2)
                                    .child("As músicas das suas pastas. Os arquivos ficam onde estão."),
                            ),
                    )
                    .child(self.botao_icone(
                        "musicas-fechar",
                        "icones/x.svg",
                        false,
                        Comando::AbrirPagina(Pagina::Inicio),
                        cx,
                    )),
            )
            .child(self.abas_da_biblioteca(largura, cx))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .items_center()
                    .gap(px(8.0))
                    .child(busca)
                    .when(!lista.pastas.is_empty(), |d| {
                        d.child(pilula("musicas-atualizar", "icones/history.svg", "Atualizar", false).on_click(
                            cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.local.atualizar();
                                cx.notify();
                            }),
                        ))
                    })
                    .child(pilula("musicas-pasta", "icones/plus.svg", "Adicionar pasta", true).on_click(cx.listener(
                        |this, _: &ClickEvent, _, cx| {
                            this.escolher_pasta(cx);
                        },
                    ))),
            )
            .when(!lista.pastas.is_empty(), |d| d.child(categorias))
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
        let quantas = lista.musicas.len();
        let corpo: AnyElement = if !lista.disponivel {
            aviso(
                "icones/info.svg",
                "A biblioteca está desligada: o Player não conseguiu usar a pasta de dados dele neste computador.",
            )
            .into_any_element()
        } else if lista.pastas.is_empty() {
            aviso(
                "icones/lib.svg",
                "Escolha uma pasta com as suas músicas (MP3, FLAC, WAV, OGG, M4A). O Player lê o que há nela e nas pastas de dentro.",
            )
            .into_any_element()
        } else if quantas == 0 && lista.varrendo.is_some() {
            aviso("icones/history.svg", "Lendo as músicas...").into_any_element()
        } else if quantas == 0 && !lista.consulta.is_empty() {
            aviso("icones/search.svg", "Nenhuma música com esse nome.").into_any_element()
        } else if quantas == 0 {
            aviso("icones/lib.svg", "Não achei músicas nessas pastas.").into_any_element()
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

        div().size_full().flex().flex_col().child(cabeca).child(
            div().flex_1().min_h(px(0.0)).flex().flex_col().px(px(lado - 10.0)).pt(px(12.0)).pb(px(12.0)).child(corpo),
        )
    }
}
