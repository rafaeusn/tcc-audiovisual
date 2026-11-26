//! A página Fila: o que toca agora e o que vem depois.
//!
//! Clicar numa música pula para ela. Com o mouse em cima aparecem subir, descer e tirar. No alto ficam o
//! aleatório, o repetir e "Limpar a seguir".

use super::{icone, medidas, mono, tempo};
use crate::app::Player;
use crate::ui::medidas::px;
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, div, prelude::*, uniform_list};
use nucleo::{Comando, Repetir};
use std::ops::Range;

const LINHA: f32 = medidas::LINHA;

impl Player {
    /// Uma linha da fila. `compacta` é a do painel: sem o número, a duração e as ações por cima (estão no menu).
    fn linha_da_fila(&self, i: usize, compacta: bool, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let itens = self.local.fila.itens();
        let Some(m) = itens.get(i) else { return div().h(px(LINHA)).into_any_element() };
        let atual = self.local.fila.atual();
        let esta = atual == Some(i);
        let ja_tocou = atual.is_some_and(|a| i < a);
        let marca: AnyElement = if esta {
            let caminho = if self.local.situacao.tocando { "icones/wave.svg" } else { "icones/pause.svg" };
            icone(caminho, medidas::ICONE_LINHA, c.texto).into_any_element()
        } else {
            mono().text_color(c.texto_3).child((i + 1).to_string()).into_any_element()
        };
        let da_linha = m.clone();
        let do_menu = move || Player::alvo_da_musica(&da_linha, Some(i));
        let de_onde = self.servicos_da_musica(m);
        let servicos: Vec<&str> = de_onde.iter().map(String::as_str).collect();
        let url = m.remota.as_ref().and_then(|r| r.capa.as_deref());
        let capa = self.capa(url, biblioteca::TipoSalvo::Faixa, medidas::CAPA_DA_LINHA, false);
        let artista = if m.artista.is_empty() { "Artista desconhecido".to_string() } else { m.artista.clone() };
        let ultimo = itens.len().saturating_sub(1);
        let fundo = if esta { c.sel } else { c.hover };
        let acoes = super::acoes_por_cima("fila-linha", fundo, if m.duracao_s > 0.0 { 110.0 } else { 54.0 })
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
            .relative()
            .w_full()
            .h(px(LINHA))
            .px(px(10.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(medidas::RAIO_CARTAO))
            .cursor_pointer()
            .when(esta, |d| d.bg(c.sel))
            .when(!esta, |d| d.hover(|s| s.bg(c.hover)))
            .when(ja_tocou, |d| d.opacity(0.55))
            .when(!compacta, |d| d.child(div().w(px(36.0)).flex_none().flex().justify_center().child(marca)))
            .child(self.com_selo(capa, &servicos, medidas::MARCA_DA_LINHA))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(super::linha(m.titulo.clone())))
                    .child(medidas::apoio().text_color(c.texto_2).child(super::linha(artista))),
            )
            .when(m.duracao_s > 0.0 && !compacta, |d| {
                d.child(mono().w(px(44.0)).flex_none().text_right().text_color(c.texto_2).child(tempo(m.duracao_s)))
            })
            .when(!compacta, |d| d.child(acoes))
            .child(self.botao_mais(("fila-mais", i), 36.0, do_menu.clone(), cx).mr(px(-6.0)))
            .on_mouse_down(gpui::MouseButton::Right, self.ao_menu(do_menu, cx))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.local.ir_para_na_fila(i);
                cx.notify();
            }))
            .into_any_element()
    }

    /// "Buscar músicas": a ação de toda tela vazia de músicas.
    pub(super) fn buscar_musicas(&self, id: &'static str, cx: &mut Context<Self>) -> gpui::Stateful<Div> {
        self.pilula(id, "icones/search.svg", "Buscar músicas", true).on_click(cx.listener(
            |this, _: &gpui::ClickEvent, _, cx| {
                this.abrir_busca();
                cx.notify();
            },
        ))
    }

    /// A aba Fila do painel: os modos, quantas faltam e a lista.
    pub(super) fn aba_da_fila(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let fila = &self.local.fila;
        let quantas = fila.itens().len();
        if quantas == 0 {
            return div()
                .p(px(24.0))
                .flex()
                .flex_col()
                .items_start()
                .gap(px(16.0))
                .child(
                    medidas::apoio().text_color(c.texto_2).child(
                        "A fila está vazia. Toque uma música, ou use o botão direito para pôr uma no fim da fila.",
                    ),
                )
                .child(self.buscar_musicas("fila-painel-buscar", cx));
        }
        let modo = |id: &'static str, caminho: &'static str, ligado: bool, comando: Comando, cx: &mut Context<Self>| {
            self.botao(
                super::Botao {
                    id,
                    icone: caminho,
                    tamanho: medidas::FICHA,
                    tamanho_icone: medidas::ICONE_LINHA,
                    raio: medidas::PILULA,
                    cor: if ligado { c.texto } else { c.texto_3 },
                    fundo: ligado.then_some(super::com_alfa(c.texto, 0.1)),
                    realce: super::com_alfa(c.texto, 0.1),
                    afunda: 0.94,
                },
                cx,
            )
            .on_click(self.ao_clicar(comando, cx))
            .on_hover(self.ao_pairar(id, Some(comando), "", cx))
        };
        let icone_repetir = if fila.repetir == Repetir::Faixa { "icones/repeat-1.svg" } else { "icones/repeat.svg" };
        let cabeca = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(4.0))
            .pl(px(16.0))
            .pr(px(10.0))
            .py(px(6.0))
            .child(medidas::apoio().flex_1().min_w(px(0.0)).text_color(c.texto_2).child(super::linha(
                match self.restam_na_fila() {
                    0 => "A última música".to_string(),
                    1 => "Mais 1 música".to_string(),
                    n => format!("Mais {n} músicas"),
                },
            )))
            .child(modo("painel-aleatorio", "icones/shuffle.svg", fila.aleatorio, Comando::AlternarAleatorio, cx))
            .child(modo("painel-repetir", icone_repetir, fila.repetir != Repetir::Nada, Comando::AlternarRepetir, cx))
            .when(self.restam_na_fila() > 0, |d| {
                d.child(
                    medidas::apoio()
                        .id("painel-limpar")
                        .h(px(medidas::FICHA))
                        .px(px(12.0))
                        .flex()
                        .flex_none()
                        .items_center()
                        .rounded(px(medidas::PILULA))
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child("Limpar")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.limpar_a_seguir();
                            cx.notify();
                        })),
                )
            });
        // A folga de baixo tira as linhas do canto redondo do painel (o corte é reto).
        div().size_full().pb(px(12.0)).flex().flex_col().child(cabeca).child(
            uniform_list(
                "painel-fila",
                quantas,
                cx.processor(|this, faixa: Range<usize>, _, cx| {
                    faixa.map(|i| this.linha_da_fila(i, true, cx)).collect::<Vec<_>>()
                }),
            )
            .track_scroll(self.rolagem_da_fila.clone())
            .flex_1()
            .min_h(px(0.0))
            .px(px(6.0)),
        )
    }

    fn restam_na_fila(&self) -> usize {
        let fila = &self.local.fila;
        fila.atual().map_or(0, |a| fila.itens().len().saturating_sub(a + 1))
    }

    fn resumo_da_fila(&self) -> String {
        match (self.local.fila.vazia(), self.restam_na_fila()) {
            (true, _) => "Nada na fila.".into(),
            (false, 0) => "Esta é a última música.".into(),
            (false, 1) => "Mais 1 música depois desta.".into(),
            (false, n) => format!("Mais {n} músicas depois desta."),
        }
    }

    fn limpar_a_seguir(&mut self) {
        let sairam = self.local.fila.limpar_a_seguir();
        self.avisar_com_desfazer("Fila limpa: só a música de agora ficou", crate::app::Desfazer::LimparASeguir(sairam));
    }

    pub(crate) fn ver_fila(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let lado = self.lado_da_pagina();
        let fila = &self.local.fila;

        let pilula = |id: &'static str, caminho: &'static str, rotulo: &'static str, ligada: bool| {
            medidas::corpo()
                .id(id)
                .h(px(medidas::BOTAO))
                .px(px(16.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .rounded(px(medidas::PILULA))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .border_1()
                .border_color(if ligada { c.texto } else { c.linha_2 })
                .when(ligada, |d| d.bg(c.sel))
                .hover(|s| s.bg(c.hover))
                .child(icone(caminho, medidas::ICONE_LINHA, c.texto))
                .child(rotulo)
        };
        let (rotulo_repetir, icone_repetir) = match fila.repetir {
            Repetir::Nada => ("Repetir: não", "icones/repeat.svg"),
            Repetir::Fila => ("Repetir: a fila", "icones/repeat.svg"),
            Repetir::Faixa => ("Repetir: a música", "icones/repeat-1.svg"),
        };
        let restantes = self.restam_na_fila();
        let resumo = self.resumo_da_fila();

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
                        this.limpar_a_seguir();
                        cx.notify();
                    },
                )))
            })
            .child(div().flex_1())
            .child(medidas::apoio().text_color(c.texto_3).child(resumo));

        let cabeca = div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(14.0))
            .px(px(lado))
            .pt(px(lado.min(24.0)))
            .child(self.titulo_da_pagina("Fila"))
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
                .child(icone("icones/queue.svg", medidas::ICONE_GRANDE, c.texto_3))
                .child(medidas::apoio().max_w(px(440.0)).text_center().text_color(c.texto_2).child(
                    "A fila está vazia. Toque uma música, ou passe o mouse nela e use \"tocar a seguir\" ou \"pôr no fim da fila\".",
                ))
                .child(self.buscar_musicas("fila-buscar", cx))
                .into_any_element()
        } else {
            uniform_list(
                "fila-lista",
                quantas,
                cx.processor(|this, faixa: Range<usize>, _, cx| {
                    faixa.map(|i| this.linha_da_fila(i, false, cx)).collect::<Vec<_>>()
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
