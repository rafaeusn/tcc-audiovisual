//! Ajustes > Reprodução: como as músicas do computador tocam.
//!
//! Crossfade, volume igualado (ReplayGain) e um equalizador de 10 bandas. Valem só para o player local: o som
//! dos outros apps não passa pelo tocador do Player.

use super::painel::{bloco_ajuste, interruptor, linha_ajuste};
use super::{medidas, mono};
use crate::app::Player;
use crate::ui::medidas::px;
use gpui::{
    Bounds, ClickEvent, Context, Div, FontWeight, MouseButton, MouseDownEvent, MouseMoveEvent, Pixels, Point,
    SharedString, canvas, div, prelude::*,
};
use nucleo::{BANDAS_EQ, PRESETS_EQ};

/// Os nomes das áreas das barras do equalizador (para saber onde o clique caiu).
const AREAS_EQ: [&str; BANDAS_EQ] = ["eq-0", "eq-1", "eq-2", "eq-3", "eq-4", "eq-5", "eq-6", "eq-7", "eq-8", "eq-9"];
const ROTULOS_EQ: [&str; BANDAS_EQ] = ["31", "62", "125", "250", "500", "1k", "2k", "4k", "8k", "16k"];
const TETO_DB: f32 = 12.0;
const ALTURA_BARRA: f32 = 120.0;
/// As opções de crossfade, em segundos.
const CROSSFADES: [f32; 6] = [0.0, 2.0, 4.0, 6.0, 8.0, 12.0];

/// O ganho (dB) para um ponto `y` dentro da barra: o meio é 0, o alto é +12, o baixo é −12. Meio em meio dB.
pub fn ganho_no_ponto(y: f32, topo: f32, altura: f32) -> f32 {
    if altura <= 0.0 {
        return 0.0;
    }
    let meio = topo + altura / 2.0;
    let g = ((meio - y) / (altura / 2.0) * TETO_DB).clamp(-TETO_DB, TETO_DB);
    (g * 2.0).round() / 2.0
}

impl Player {
    /// Põe a banda `i` no ganho do ponto onde o mouse está.
    fn mexer_no_eq(&mut self, i: usize, ponto: Point<Pixels>) {
        let Some(area) = self.areas.borrow().get(AREAS_EQ[i]).copied() else { return };
        let ganho = ganho_no_ponto(f32::from(ponto.y), f32::from(area.origin.y), f32::from(area.size.height));
        if self.ajustes.eq.get(i).is_some_and(|g| (*g - ganho).abs() < 0.01) && self.ajustes.eq_ligado {
            return;
        }
        self.mudar_reproducao(|aj| {
            aj.eq_ligado = true;
            if let Some(g) = aj.eq.get_mut(i) {
                *g = ganho;
            }
        });
    }

    pub(crate) fn bloco_reproducao(&self, recuo: f32, largura: f32, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let aj = &self.ajustes;

        // Crossfade.
        let escolhido = CROSSFADES.iter().position(|s| (*s - aj.crossfade_s).abs() < 0.01).unwrap_or(0);
        let mut crossfades = self.seletor("seletor-crossfade", largura, CROSSFADES.len(), escolhido);
        for (i, s) in CROSSFADES.into_iter().enumerate() {
            let rotulo: &'static str = match i {
                0 => "Não",
                1 => "2 s",
                2 => "4 s",
                3 => "6 s",
                4 => "8 s",
                _ => "12 s",
            };
            crossfades = crossfades.child(self.opcao(("crossfade", i), rotulo, i == escolhido).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.mudar_reproducao(|aj| aj.crossfade_s = s);
                    cx.notify();
                },
            )));
        }
        let frase_crossfade: SharedString = if aj.crossfade_s > 0.0 {
            format!("A próxima música entra nos últimos {} segundos da de agora.", aj.crossfade_s as u32).into()
        } else {
            "Uma música emenda na outra, sem silêncio no meio.".into()
        };

        // Volume igualado.
        let pos_igualado = self.anim.seguir(("interruptor-igualado", 0), f32::from(u8::from(aj.volume_igualado)), 0.2);
        let igualado = linha_ajuste(
            &c,
            "icones/vol.svg",
            "Volume igualado",
            "Usa o ReplayGain do arquivo para as músicas tocarem no mesmo volume. Vale a partir da próxima.".into(),
        )
        .child(interruptor(&c, "aj-igualado", aj.volume_igualado, pos_igualado).on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                this.mudar_reproducao(|aj| aj.volume_igualado = !aj.volume_igualado);
                cx.notify();
            },
        )));

        // Equalizador.
        let pos_eq = self.anim.seguir(("interruptor-eq", 0), f32::from(u8::from(aj.eq_ligado)), 0.2);
        let preset_atual = PRESETS_EQ.iter().position(|(_, g)| g.as_slice() == aj.eq.as_slice());
        let mut presets = div().flex().flex_wrap().gap(px(8.0));
        for (i, (nome, ganhos)) in PRESETS_EQ.iter().enumerate() {
            let escolhido = aj.eq_ligado && preset_atual == Some(i);
            presets = presets.child(
                medidas::apoio()
                    .id(("eq-preset", i))
                    .h(px(medidas::FICHA))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .rounded(px(medidas::PILULA))
                    .border_1()
                    .border_color(if escolhido { c.texto } else { c.linha_2 })
                    .when(escolhido, |d| d.bg(c.sel))
                    .text_color(if escolhido { c.texto } else { c.texto_2 })
                    .font_weight(FontWeight::MEDIUM)
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(*nome)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.mudar_reproducao(|aj| {
                            aj.eq_ligado = true;
                            aj.eq = ganhos.to_vec();
                        });
                        cx.notify();
                    })),
            );
        }
        let cor_barra = if aj.eq_ligado { c.texto } else { c.texto_3 };
        let mut barras = div().flex().gap(px(6.0)).h(px(ALTURA_BARRA + 40.0));
        for i in 0..BANDAS_EQ {
            let ganho = aj.eq.get(i).copied().unwrap_or(0.0);
            let metade = ALTURA_BARRA / 2.0;
            let altura = (ganho.abs() / TETO_DB * metade).max(2.0);
            let topo = if ganho >= 0.0 { metade - altura } else { metade };
            let areas = self.areas.clone();
            let id = AREAS_EQ[i];
            barras = barras.child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(4.0))
                    .child(mono().text_color(c.texto_3).child(if ganho > 0.0 {
                        format!("+{ganho:.0}")
                    } else {
                        format!("{ganho:.0}")
                    }))
                    .child(
                        div()
                            .id(("eq-barra", i))
                            .w_full()
                            .h(px(ALTURA_BARRA))
                            .relative()
                            .cursor_pointer()
                            .child(
                                canvas(
                                    move |area: Bounds<Pixels>, _, _| {
                                        areas.borrow_mut().insert(id, area);
                                    },
                                    |_, (), _, _| {},
                                )
                                .absolute()
                                .left(px(0.0))
                                .top(px(0.0))
                                .size_full(),
                            )
                            // O trilho e a marca do zero.
                            .child(
                                div()
                                    .absolute()
                                    .left_1_2()
                                    .ml(px(-1.0))
                                    .top(px(0.0))
                                    .w(px(2.0))
                                    .h_full()
                                    .rounded(px(medidas::raio_fino(2.0)))
                                    .bg(c.linha_2),
                            )
                            .child(
                                div()
                                    .absolute()
                                    .left(px(4.0))
                                    .right(px(4.0))
                                    .top(px(metade - 0.5))
                                    .h(px(1.0))
                                    .bg(c.linha_2),
                            )
                            // O ganho: do meio para cima (reforço) ou para baixo (corte).
                            .child(
                                div()
                                    .absolute()
                                    .left_1_2()
                                    .ml(px(-4.0))
                                    .top(px(topo))
                                    .w(px(8.0))
                                    .h(px(altura))
                                    .rounded(px(medidas::raio_fino(8.0)))
                                    .bg(cor_barra),
                            )
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                                    this.mexer_no_eq(i, ev.position);
                                    cx.notify();
                                }),
                            )
                            .on_mouse_move(cx.listener(move |this, ev: &MouseMoveEvent, _, cx| {
                                if ev.pressed_button == Some(MouseButton::Left) {
                                    this.mexer_no_eq(i, ev.position);
                                    cx.notify();
                                }
                            })),
                    )
                    .child(mono().text_color(c.texto_2).child(ROTULOS_EQ[i])),
            );
        }
        let equalizador = linha_ajuste(
            &c,
            "icones/eq.svg",
            "Equalizador",
            "Clique ou arraste nas barras. Vale só para as músicas do computador.".into(),
        )
        .child(interruptor(&c, "aj-eq", aj.eq_ligado, pos_eq).on_click(cx.listener(
            |this, _: &ClickEvent, _, cx| {
                this.mudar_reproducao(|aj| aj.eq_ligado = !aj.eq_ligado);
                cx.notify();
            },
        )));

        // Pré-análise da biblioteca, com o andamento.
        let progresso = self.local.preanalista().progresso();
        let frase_analise: SharedString = match (&progresso.analisando, progresso.total) {
            (Some(nome), _) => format!("Analisando {nome}... ({} de {})", progresso.analisadas, progresso.total).into(),
            (None, 0) => "Analisa cada música antes de tocar: o mapa inteiro, os trechos e o drop chegam antes.".into(),
            (None, total) if progresso.analisadas >= total => format!("As {total} músicas já foram analisadas.").into(),
            (None, total) => format!("{} de {total} músicas analisadas.", progresso.analisadas).into(),
        };
        let pos_analise =
            self.anim.seguir(("interruptor-analise", 0), f32::from(u8::from(aj.analisar_biblioteca)), 0.2);
        let analise = linha_ajuste(&c, "icones/struct.svg", "Analisar a biblioteca", frase_analise).child(
            interruptor(&c, "aj-analise", aj.analisar_biblioteca, pos_analise).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.mudar_reproducao(|aj| aj.analisar_biblioteca = !aj.analisar_biblioteca);
                    cx.notify();
                },
            )),
        );

        bloco_ajuste(&c, false)
            .child(
                medidas::apoio().text_color(c.texto_3).font_weight(FontWeight::SEMIBOLD).child("Músicas do computador"),
            )
            .child(analise)
            .child(linha_ajuste(&c, "icones/swap.svg", "Crossfade", frase_crossfade))
            .child(div().pl(px(recuo)).child(crossfades))
            .child(igualado)
            .child(equalizador)
            .child(div().pl(px(recuo)).flex().flex_col().gap(px(12.0)).child(presets).child(barras))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn ponto_na_barra_vira_ganho() {
        assert_eq!(ganho_no_ponto(60.0, 0.0, 120.0), 0.0);
        assert_eq!(ganho_no_ponto(0.0, 0.0, 120.0), 12.0);
        assert_eq!(ganho_no_ponto(120.0, 0.0, 120.0), -12.0);
        assert_eq!(ganho_no_ponto(-50.0, 0.0, 120.0), 12.0, "fora da barra fica no limite");
        assert_eq!(ganho_no_ponto(130.0, 100.0, 120.0), 6.0);
        assert_eq!(ganho_no_ponto(10.0, 0.0, 0.0), 0.0);
    }
}
