//! A janela de playlist: pôr músicas numa playlist que já existe, ou criar uma nova (com ou sem músicas).
//!
//! O nome usa o mesmo campo de texto do resto do app (`ui::campo`), então acentos e colar funcionam. Enter cria,
//! Esc fecha, clicar fora fecha.

use super::{RAIO_CONTROLE, RAIO_MODULO, icone, mono, texto, titulo};
use crate::app::Player;
use gpui::{AnyElement, ClickEvent, Context, FontWeight, KeyDownEvent, SharedString, div, prelude::*, px};

impl Player {
    fn fechar_janela_playlist(&mut self) {
        self.janela_playlist = None;
        self.marcado = None;
    }

    /// Cria a playlist com o nome escrito (e as músicas da janela).
    fn criar_playlist_da_janela(&mut self) {
        let Some(j) = self.janela_playlist.take() else { return };
        self.marcado = None;
        let nome = j.nome.trim().to_string();
        let quantas = j.musicas.len();
        self.local.criar_playlist(nome.clone(), j.musicas, j.so_criar);
        let nome = if nome.is_empty() { "Nova playlist".to_string() } else { nome };
        self.avisar(match quantas {
            0 => format!("Playlist \"{nome}\" criada"),
            1 => format!("Playlist \"{nome}\" criada com 1 música"),
            n => format!("Playlist \"{nome}\" criada com {n} músicas"),
        });
    }

    pub(crate) fn tecla_na_janela_playlist(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let k = &ev.keystroke;
        let ctrl = k.modifiers.control || k.modifiers.platform;
        // As letras chegam pelo caminho de texto do sistema (`ui::campo`); aqui só as teclas de controle.
        match k.key.as_str() {
            "escape" => self.fechar_janela_playlist(),
            "enter" => self.criar_playlist_da_janela(),
            "backspace" => self.apagar_no_campo(ctrl),
            "v" if ctrl => self.colar_no_campo(cx),
            _ => {}
        }
        cx.notify();
    }

    pub(crate) fn ver_janela_playlist(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let j = self.janela_playlist.as_ref()?;
        let c = self.cores();
        let a = &self.arranjo;
        let quantas = j.musicas.len();
        let titulo_janela = if j.so_criar { "Nova playlist" } else { "Pôr numa playlist" };
        let frase: SharedString = match quantas {
            0 => "Escreva o nome e aperte Enter.".into(),
            1 => "1 música vai entrar.".into(),
            n => format!("{n} músicas vão entrar.").into(),
        };

        let mut lista = div().flex().flex_col().gap(px(2.0));
        if !j.so_criar {
            for (i, p) in self.local.lista.playlists.iter().enumerate() {
                let (id, nome) = (p.id, p.nome.clone());
                lista = lista.child(
                    div()
                        .id(("janela-playlist", i))
                        .h(px(48.0))
                        .px(px(12.0))
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .rounded(px(RAIO_CONTROLE))
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/playlist.svg", 18.0, c.texto_2))
                        .child(
                            texto(14.0)
                                .flex_1()
                                .min_w(px(0.0))
                                .truncate()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(SharedString::from(p.nome.clone())),
                        )
                        .child(mono().text_color(c.texto_3).child(p.quantas.to_string()))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            if let Some(j) = this.janela_playlist.take() {
                                this.marcado = None;
                                this.local.adicionar_a_playlist(id, j.musicas);
                                this.avisar(format!("Na playlist \"{nome}\""));
                            }
                            cx.notify();
                        })),
                );
            }
        }

        let campo = div()
            .id("janela-playlist-nome")
            .flex_1()
            .min_w(px(0.0))
            .h(px(40.0))
            .relative()
            .flex()
            .items_center()
            .gap(px(10.0))
            .px(px(14.0))
            .rounded(px(999.0))
            .border_1()
            .border_color(c.texto)
            .cursor_text()
            .child(icone("icones/plus.svg", 16.0, c.texto))
            .child(self.receber_texto(cx))
            .child(Self::conteudo_do_campo(&c, &j.nome, "Nome da playlist nova", 14.0, true));
        let criar = texto(14.0)
            .id("janela-playlist-criar")
            .h(px(40.0))
            .px(px(16.0))
            .flex()
            .flex_none()
            .items_center()
            .rounded(px(999.0))
            .bg(c.inv)
            .text_color(c.inv_texto)
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(|s| s.opacity(0.9))
            .child("Criar")
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.criar_playlist_da_janela();
                cx.notify();
            }));

        let largura = 440.0f32.min(a.janela.w - 32.0).max(240.0);
        let altura = (a.janela.h - 64.0).clamp(220.0, 560.0);
        Some(
            div()
                .id("janela-playlist-fundo")
                .absolute()
                .left(px(0.0))
                .top(px(0.0))
                .size_full()
                .bg(c.escurecer)
                .occlude()
                .flex()
                .items_center()
                .justify_center()
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.fechar_janela_playlist();
                    cx.notify();
                }))
                .child(
                    div()
                        .id("janela-playlist")
                        .w(px(largura))
                        .max_h(px(altura))
                        .flex()
                        .flex_col()
                        .gap(px(14.0))
                        .p(px(20.0))
                        .rounded(px(RAIO_MODULO))
                        .bg(c.bloco)
                        .border_1()
                        .border_color(c.linha_2)
                        .shadow_lg()
                        .overflow_hidden()
                        .occlude()
                        .on_click(|_, _, cx| cx.stop_propagation())
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(px(4.0))
                                .child(titulo(24.0).child(titulo_janela))
                                .child(texto(13.0).text_color(c.texto_2).child(frase)),
                        )
                        .when(!j.so_criar && !self.local.lista.playlists.is_empty(), |d| {
                            d.child(
                                div()
                                    .id("janela-playlist-lista")
                                    .flex_1()
                                    .min_h(px(0.0))
                                    .overflow_y_scroll()
                                    .child(lista),
                            )
                        })
                        .child(div().flex().items_center().gap(px(8.0)).child(campo).child(criar)),
                )
                .into_any_element(),
        )
    }
}
