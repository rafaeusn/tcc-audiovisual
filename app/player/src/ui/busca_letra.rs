//! Busca manual de letra: para quando a busca automática não acha a música, ou acha a errada.
//!
//! Usa a mesma moldura da barra de comandos. A pessoa digita o nome da música (e o artista, se quiser), aperta
//! Enter para procurar no LRCLIB e escolhe um dos resultados. A escolha vira a letra da faixa que está tocando
//! e fica guardada, como as letras achadas sozinhas.

use super::{RAIO_CONTROLE, RAIO_MODULO, icone, mono, tecla, tempo, texto};
use crate::app::Player;
use crate::letrista::Achados;
use gpui::{AnyElement, ClickEvent, Context, FontWeight, KeyDownEvent, SharedString, div, prelude::*, px};

impl Player {
    /// Os resultados valem para o que está digitado agora?
    fn resultados_da_consulta(&self) -> Option<&[letras::Registro]> {
        let consulta = self.paleta.as_ref()?.consulta.trim();
        match &self.procura.achados {
            Achados::Prontos(lista) if self.procura.consulta == consulta => Some(lista.as_slice()),
            _ => None,
        }
    }

    /// Tecla com a busca de letra aberta: digitar muda o texto, Enter procura (ou usa o resultado escolhido),
    /// as setas escolhem, Esc fecha.
    pub(crate) fn tecla_na_busca_de_letra(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let k = &ev.keystroke;
        let ctrl = k.modifiers.control || k.modifiers.platform;
        let quantos = self.resultados_da_consulta().map(<[_]>::len);
        let Some(paleta) = self.paleta.as_mut() else { return };
        match k.key.as_str() {
            "escape" => self.paleta = None,
            "enter" => match quantos {
                Some(n) if n > 0 => {
                    let escolhido = paleta.selecionado.min(n - 1);
                    self.fixar_letra(escolhido);
                }
                _ => self.procurar_letra(),
            },
            "down" | "up" => {
                let ultimo = quantos.unwrap_or(0).saturating_sub(1);
                paleta.selecionado = if k.key == "down" {
                    (paleta.selecionado + 1).min(ultimo)
                } else {
                    paleta.selecionado.saturating_sub(1)
                };
                self.rolagem_paleta.scroll_to_item(paleta.selecionado);
            }
            "backspace" => self.apagar_no_campo(ctrl),
            "v" if ctrl => self.colar_no_campo(cx),
            _ => {}
        }
        cx.notify();
    }

    pub(crate) fn ver_busca_de_letra(&self, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let a = &self.arranjo;
        let consulta: String = self.paleta.as_ref().map(|p| p.consulta.clone()).unwrap_or_default();
        let selecionado = self.paleta.as_ref().map_or(0, |p| p.selecionado);
        let resultados = self.resultados_da_consulta();

        let mut lista = div()
            .id("letra-lista")
            .track_scroll(&self.rolagem_paleta)
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .p(px(6.0))
            .flex()
            .flex_col();
        let aviso = |frase: &'static str| texto(14.0).px(px(14.0)).py(px(28.0)).text_color(c.texto_2).child(frase);
        match resultados {
            Some([]) => {
                lista = lista.child(aviso("Nada encontrado. Tente só o nome da música, ou outro jeito de escrever."))
            }
            Some(itens) => {
                for (i, r) in itens.iter().enumerate() {
                    let ativo = i == selecionado.min(itens.len() - 1);
                    let de_quem = match (r.artist_name.is_empty(), r.album_name.is_empty()) {
                        (false, false) => format!("{} · {}", r.artist_name, r.album_name),
                        (false, true) => r.artist_name.clone(),
                        _ => r.album_name.clone(),
                    };
                    let tipo = if r.sincronizada() {
                        "com os tempos"
                    } else if r.instrumental {
                        "instrumental"
                    } else {
                        "só o texto"
                    };
                    lista = lista.child(
                        div()
                            .id(("letra-item", i))
                            .min_h(px(52.0))
                            .py(px(6.0))
                            .flex()
                            .flex_none()
                            .items_center()
                            .gap(px(12.0))
                            .px(px(12.0))
                            .rounded(px(RAIO_CONTROLE))
                            .cursor_pointer()
                            .when(ativo, |d| d.bg(c.sel))
                            .when(!ativo, |d| d.hover(|s| s.bg(c.hover)))
                            .child(icone("icones/lyrics.svg", 18.0, if ativo { c.texto } else { c.texto_2 }))
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
                                            .child(SharedString::from(r.track_name.clone())),
                                    )
                                    .child(texto(12.0).text_color(c.texto_2).truncate().child(de_quem)),
                            )
                            .child(texto(12.0).flex_none().text_color(c.texto_2).child(tipo))
                            .child(mono().flex_none().text_color(c.texto_2).child(tempo(r.duration)))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.fixar_letra(i);
                                cx.notify();
                            })),
                    );
                }
            }
            None => {
                let frase = match &self.procura.achados {
                    Achados::Buscando => "Procurando...",
                    Achados::SemRede if self.procura.consulta == consulta.trim() => {
                        "Sem conexão para procurar. Aperte Enter para tentar de novo."
                    }
                    _ => "Escreva o nome da música (e o artista, se quiser) e aperte Enter.",
                };
                lista = lista.child(aviso(frase));
            }
        }

        let entrada = div()
            .h(px(60.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.0))
            .pl(px(20.0))
            .pr(px(16.0))
            .border_b_1()
            .border_color(c.linha_2)
            .child(icone("icones/lyrics.svg", 20.0, c.texto_3))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .relative()
                    .flex()
                    .child(self.receber_texto(cx))
                    .child(Self::conteudo_do_campo(&c, &consulta, "Buscar a letra: música e artista", 16.0, true)),
            )
            .child(tecla(&c, "Esc"));

        let tem_resultados = resultados.is_some_and(|r| !r.is_empty());
        let dica = |teclas: &[&'static str], frase: &'static str| {
            let mut grupo = div().flex().flex_none().items_center().gap(px(6.0));
            for t in teclas {
                grupo = grupo.child(tecla(&c, t));
            }
            grupo.child(texto(12.0).text_color(c.texto_2).child(frase))
        };
        let rodape = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(16.0))
            .px(px(20.0))
            .py(px(12.0))
            .border_t_1()
            .border_color(c.linha_2)
            .overflow_hidden()
            .when(tem_resultados, |d| d.child(dica(&["↑", "↓"], "escolher")))
            .child(dica(&["Enter"], if tem_resultados { "usar esta letra" } else { "procurar no LRCLIB" }));

        let largura = 640.0f32.min(a.janela.w - 32.0).max(240.0);
        let altura = 560.0f32.min(a.janela.h * 0.74).max(200.0);
        div()
            .id("letra-fundo")
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .size_full()
            .bg(c.escurecer)
            .occlude()
            .flex()
            .justify_center()
            .items_start()
            .pt(px((a.janela.h * 0.14).min(120.0)))
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.paleta = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("letra-busca")
                    .w(px(largura))
                    .max_h(px(altura))
                    .flex()
                    .flex_col()
                    .rounded(px(RAIO_MODULO))
                    .bg(c.bloco)
                    .border_1()
                    .border_color(c.linha_2)
                    .shadow_lg()
                    .overflow_hidden()
                    .occlude()
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(entrada)
                    .child(lista)
                    .child(rodape),
            )
            .into_any_element()
    }
}
