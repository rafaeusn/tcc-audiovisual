//! A aba Fila da Biblioteca: o que toca agora e o que vem depois, nas músicas do computador.
//!
//! Clicar numa música pula para ela. Com o mouse em cima aparecem subir, descer e tirar. No alto ficam o
//! aleatório, o repetir e "Limpar a seguir".

use super::{RAIO_CONTROLE, icone, mono, tempo, texto};
use crate::app::Player;
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, SharedString, div, prelude::*, px, uniform_list};
use nucleo::{Comando, Repetir};
use std::ops::Range;

const LINHA: f32 = 52.0;

impl Player {
    fn linha_da_fila(&self, i: usize, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let itens = self.local.fila.itens();
        let Some(m) = itens.get(i) else { return div().h(px(LINHA)).into_any_element() };
        let atual = self.local.fila.atual();
        let esta = atual == Some(i);
        let ja_tocou = atual.is_some_and(|a| i < a);
        let marca: AnyElement = if esta {
            let caminho = if self.local.situacao.tocando { "icones/wave.svg" } else { "icones/pause.svg" };
            icone(caminho, 16.0, c.texto).into_any_element()
        } else {
            mono().text_color(c.texto_3).child((i + 1).to_string()).into_any_element()
        };
        let artista = if m.artista.is_empty() { "Artista desconhecido".to_string() } else { m.artista.clone() };
        let ultimo = itens.len().saturating_sub(1);
        let acoes = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(2.0))
            .when(i > 0, |d| {
                d.child(self.acao_da_linha(("fila-subir", i), "icones/chev-u.svg", "fila-linha", false).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.local.fila.mover(i, i - 1);
                        cx.notify();
                    }),
                ))
            })
            .when(i < ultimo, |d| {
                d.child(self.acao_da_linha(("fila-descer", i), "icones/chev-d.svg", "fila-linha", false).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.local.fila.mover(i, i + 1);
                        cx.notify();
                    }),
                ))
            })
            .when(!esta, |d| {
                d.child(self.acao_da_linha(("fila-tirar", i), "icones/x.svg", "fila-linha", false).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        if let Some(musica) = this.local.fila.tirar(i) {
                            let texto = format!("\"{}\" saiu da fila", musica.titulo);
                            this.avisar_com_desfazer(
                                texto,
                                crate::app::Desfazer::TirarDaFila { indice: i, musica: Box::new(musica) },
                            );
                        }
                        cx.notify();
                    }),
                ))
            });
        div()
            .id(("fila", i))
            .group("fila-linha")
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
            .when(ja_tocou, |d| d.opacity(0.55))
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
                    .child(texto(12.0).text_color(c.texto_2).truncate().child(artista)),
            )
            .child(acoes)
            .when(m.duracao_s > 0.0, |d| {
                d.child(mono().w(px(44.0)).flex_none().text_right().text_color(c.texto_2).child(tempo(m.duracao_s)))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.local.ir_para_na_fila(i);
                cx.notify();
            }))
            .into_any_element()
    }

    pub(crate) fn ver_fila(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let largo = a.heroi.w >= 640.0;
        let lado = if largo { 32.0 } else { 16.0 };
        let largura = a.heroi.w - 2.0 * lado;
        let fila = &self.local.fila;

        let pilula = |id: &'static str, caminho: &'static str, rotulo: &'static str, ligada: bool| {
            texto(14.0)
                .id(id)
                .h(px(40.0))
                .px(px(16.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .border_1()
                .border_color(if ligada { c.texto } else { c.linha_2 })
                .when(ligada, |d| d.bg(c.sel))
                .hover(|s| s.bg(c.hover))
                .child(icone(caminho, 18.0, c.texto))
                .child(rotulo)
        };
        let (rotulo_repetir, icone_repetir) = match fila.repetir {
            Repetir::Nada => ("Repetir: não", "icones/repeat.svg"),
            Repetir::Fila => ("Repetir: a fila", "icones/repeat.svg"),
            Repetir::Faixa => ("Repetir: a música", "icones/repeat-1.svg"),
        };
        let restantes = fila.atual().map_or(0, |a| fila.itens().len().saturating_sub(a + 1));
        let resumo: SharedString = match (fila.vazia(), restantes) {
            (true, _) => "Nada na fila.".into(),
            (false, 0) => "Esta é a última música da fila.".into(),
            (false, 1) => "Mais 1 música depois desta.".into(),
            (false, n) => format!("Mais {n} músicas depois desta.").into(),
        };

        let controles = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(8.0))
            .child(
                pilula("fila-aleatorio", "icones/shuffle.svg", "Aleatório", fila.aleatorio)
                    .on_click(self.ao_clicar(Comando::AlternarAleatorio, cx)),
            )
            .child(
                pilula("fila-repetir", icone_repetir, rotulo_repetir, fila.repetir != Repetir::Nada)
                    .on_click(self.ao_clicar(Comando::AlternarRepetir, cx)),
            )
            .when(restantes > 0, |d| {
                d.child(pilula("fila-limpar", "icones/x.svg", "Limpar a seguir", false).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        let sairam = this.local.fila.limpar_a_seguir();
                        this.avisar_com_desfazer(
                            "Fila limpa: só a música de agora ficou",
                            crate::app::Desfazer::LimparASeguir(sairam),
                        );
                        cx.notify();
                    },
                )))
            })
            .child(div().flex_1())
            .child(texto(12.0).text_color(c.texto_3).child(resumo));

        let cabeca = div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(14.0))
            .px(px(lado))
            .pt(px(lado.min(24.0)))
            .child(self.cabeca_da_biblioteca(largo, "O que toca agora e o que vem depois.", cx))
            .child(self.abas_da_biblioteca(largura, cx))
            .when(!fila.vazia(), |d| d.child(controles));

        let quantas = fila.itens().len();
        let corpo: AnyElement = if quantas == 0 {
            div()
                .flex_1()
                .py(px(40.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(12.0))
                .child(icone("icones/queue.svg", 28.0, c.texto_3))
                .child(texto(14.0).max_w(px(440.0)).text_center().text_color(c.texto_2).child(
                    "A fila está vazia. Toque uma música em Músicas, ou passe o mouse numa música e use \"tocar a seguir\" ou \"pôr no fim da fila\".",
                ))
                .into_any_element()
        } else {
            uniform_list(
                "fila-lista",
                quantas,
                cx.processor(|this, faixa: Range<usize>, _, cx| {
                    faixa.map(|i| this.linha_da_fila(i, cx)).collect::<Vec<_>>()
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
