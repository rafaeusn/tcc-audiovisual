//! O dock: a faixa que está tocando, os controles, o mapa de energia e o volume. O fundo dele segue a cor do
//! visual.

use super::{Botao, RAIO_CONTROLE, caixa, com_alfa, icone, mono, numero, tempo, texto};
use crate::app::Player;
use crate::arranjo::{Modo, RAIO};
use crate::tema;
use gpui::{
    Bounds, Context, Div, FontWeight, Hsla, MouseButton, MouseDownEvent, PathBuilder, ScrollWheelEvent, SharedString,
    Stateful, canvas, div, fill, img, point, prelude::*, px, size,
};
use nucleo::{Comando, Tema};

/// Play e pause como dois polígonos de 4 pontos (numa grade de 24), para um virar o outro ponto a ponto.
const PLAY: [[(f32, f32); 4]; 2] =
    [[(8.0, 5.0), (14.0, 8.5), (14.0, 15.5), (8.0, 19.0)], [(14.0, 8.5), (20.0, 12.0), (20.0, 12.0), (14.0, 15.5)]];
const PAUSE: [[(f32, f32); 4]; 2] =
    [[(7.0, 5.0), (10.5, 5.0), (10.5, 19.0), (7.0, 19.0)], [(13.5, 5.0), (17.0, 5.0), (17.0, 19.0), (13.5, 19.0)]];

impl Player {
    pub(crate) fn ver_dock(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let a = &self.arranjo;
        let tom = tema::cor_de(self.tom);
        let base = caixa(a.dock)
            .rounded(px(RAIO))
            .bg(if self.imersivo() { com_alfa(tom, 0.72) } else { tom })
            // Cliques no dock não passam para o visual que, na skin imersiva, está por baixo.
            .occlude();
        if a.modo == Modo::Empilhado {
            // Duas linhas: faixa e controles em cima, linha do tempo embaixo.
            return base
                .flex()
                .flex_col()
                .justify_center()
                .gap(px(8.0))
                .px(px(14.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .child(self.dock_faixa(44.0).flex_1())
                        .child(self.dock_controles(cx))
                        .when(a.dock.w >= 440.0, |d| {
                            d.child(self.dock_botao(
                                "dock-mudo",
                                self.icone_do_volume(),
                                self.mudo,
                                Comando::Silenciar,
                                cx,
                            ))
                        }),
                )
                .child(self.dock_linha_do_tempo(cx));
        }

        let meio = ((a.dock.w - 36.0) * 0.4).clamp(260.0, 560.0);
        let painel_visivel = match a.modo {
            Modo::Largo => self.ajustes.painel_aberto,
            _ => self.gaveta_aberta,
        };
        // A barra de volume encolhe e some conforme o dock estreita; o botão de mudo fica.
        let pista = if a.dock.w >= 1040.0 {
            112.0
        } else if a.dock.w >= 860.0 {
            84.0
        } else {
            0.0
        };
        base.flex()
            .items_center()
            .gap(px(24.0))
            .px(px(18.0))
            .child(self.dock_faixa(48.0).flex_1().flex_basis(px(0.0)))
            .child(
                div()
                    .w(px(meio))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(self.dock_controles(cx))
                    .child(self.dock_linha_do_tempo(cx)),
            )
            .child(
                div()
                    .flex_1()
                    .flex_basis(px(0.0))
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .justify_end()
                    .gap(px(4.0))
                    .child(self.dock_botao(
                        "dock-painel",
                        "icones/panel.svg",
                        painel_visivel,
                        Comando::AlternarPainel,
                        cx,
                    ))
                    .child(self.dock_botao("dock-mini", "icones/pip.svg", false, Comando::AlternarMini, cx))
                    .child(self.dock_botao("dock-cheia", "icones/full.svg", false, Comando::TelaCheia, cx))
                    .child(self.dock_botao("dock-mudo", self.icone_do_volume(), false, Comando::Silenciar, cx))
                    .when(pista > 0.0, |d| d.child(self.dock_volume(pista, cx))),
            )
    }

    fn icone_do_volume(&self) -> &'static str {
        if self.mudo || self.volume <= 0.001 { "icones/mute.svg" } else { "icones/vol.svg" }
    }

    /// Botão de ícone do dock. O fundo do dock muda de cor com o visual, então o estado ligado e o realce usam
    /// um véu claro por cima dele em vez das cores fixas do tema.
    fn dock_botao(
        &self,
        id: &'static str,
        caminho: &'static str,
        ativo: bool,
        comando: Comando,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        self.botao(
            Botao {
                id,
                icone: caminho,
                tamanho: 40.0,
                tamanho_icone: 20.0,
                raio: RAIO_CONTROLE,
                cor: if ativo { c.texto } else { c.texto_2 },
                fundo: ativo.then_some(com_alfa(c.texto, 0.14)),
                realce: com_alfa(c.texto, 0.1),
                afunda: 0.94,
            },
            cx,
        )
        .on_click(self.ao_clicar(comando, cx))
        .on_hover(self.ao_pairar(id, Some(comando), "", cx))
    }

    /// Capa, título e artista (ou de onde vem o som).
    fn dock_faixa(&self, lado: f32) -> Div {
        let c = self.cores();
        let capa = match &self.capa {
            Some(imagem) => img(imagem.clone())
                .size(px(lado))
                .flex_none()
                .rounded(px(RAIO_CONTROLE))
                // No tema Mono nada tem cor, nem a capa.
                .grayscale(self.ajustes.tema == Tema::Mono)
                .into_any_element(),
            None => div()
                .size(px(lado))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(RAIO_CONTROLE))
                .bg(c.ponto)
                .child(icone("icones/headphones.svg", 20.0, c.texto_2))
                .into_any_element(),
        };
        let (nome, sub): (SharedString, SharedString) = match self.midia.faixa.as_ref().filter(|f| !f.titulo.is_empty())
        {
            Some(f) => (
                f.titulo.clone().into(),
                if f.artista.is_empty() { f.app.clone().into() } else { f.artista.clone().into() },
            ),
            None if self.retrato.gate > 0.5 => ("Som do sistema".into(), "Modo Escuta".into()),
            None => ("Nada tocando".into(), "Modo Escuta".into()),
        };
        div().min_w(px(0.0)).flex().items_center().gap(px(12.0)).child(capa).child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(texto(14.0).font_weight(FontWeight::SEMIBOLD).truncate().child(nome))
                .child(texto(14.0).text_color(c.texto_2).truncate().child(sub)),
        )
    }

    /// Anterior, tocar/pausar e próxima. Agem no app que está tocando; sem app, ficam apagados.
    fn dock_controles(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let ativo = self.midia.disponivel;
        let lateral =
            |id: &'static str, caminho: &'static str, comando: Comando, cx: &mut Context<Self>| -> Stateful<Div> {
                if !ativo {
                    return div()
                        .id(id)
                        .size(px(40.0))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(0.4)
                        .child(icone(caminho, 22.0, c.texto_2));
                }
                self.botao(
                    Botao {
                        id,
                        icone: caminho,
                        tamanho: 40.0,
                        tamanho_icone: 22.0,
                        raio: 999.0,
                        cor: c.texto_2,
                        fundo: None,
                        realce: com_alfa(c.texto, 0.1),
                        afunda: 0.94,
                    },
                    cx,
                )
                .on_click(self.ao_clicar(comando, cx))
                .on_hover(self.ao_pairar(id, Some(comando), "", cx))
            };

        // O triângulo do play se divide nas duas barras do pause (180 ms) em vez de trocar de ícone.
        let forma = self.anim.seguir(("tocar-forma", 0), f32::from(u8::from(self.midia.tocando)), 0.18);
        let e = if ativo { self.pressao("dock-tocar", 0.95) } else { 1.0 };
        let cor_do_icone = c.inv_texto;
        let desenho = canvas(
            |_, _, _| (),
            move |area: Bounds<gpui::Pixels>, (), window, _| {
                let escala = f32::from(area.size.width) / 24.0;
                let (x0, y0) = (f32::from(area.origin.x), f32::from(area.origin.y));
                for lado in 0..2 {
                    let mut caminho = PathBuilder::fill();
                    for (i, (de, para)) in PLAY[lado].iter().zip(PAUSE[lado].iter()).enumerate() {
                        let p = point(
                            px(x0 + (de.0 + (para.0 - de.0) * forma) * escala),
                            px(y0 + (de.1 + (para.1 - de.1) * forma) * escala),
                        );
                        if i == 0 { caminho.move_to(p) } else { caminho.line_to(p) }
                    }
                    caminho.close();
                    if let Ok(caminho) = caminho.build() {
                        window.paint_path(caminho, cor_do_icone);
                    }
                }
            },
        )
        .size(px(22.0 * e));
        let mut tocar = div().id("dock-tocar").size(px(44.0)).flex_none().flex().items_center().justify_center().child(
            div().size(px(44.0 * e)).flex().items_center().justify_center().rounded(px(999.0)).bg(c.inv).child(desenho),
        );
        tocar = if ativo {
            tocar
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, self.ao_apertar("dock-tocar", cx))
                .on_click(self.ao_clicar(Comando::TocarPausar, cx))
                .child(self.marca("dock-tocar"))
                .on_hover(self.ao_pairar("dock-tocar", Some(Comando::TocarPausar), "", cx))
        } else {
            tocar.opacity(0.4)
        };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .child(lateral("dock-anterior", "icones/prev.svg", Comando::Anterior, cx))
            .child(tocar)
            .child(lateral("dock-proxima", "icones/next.svg", Comando::Proxima, cx))
    }

    /// Tempo decorrido, mapa de energia e duração. O mapa mostra a energia de cada trecho já ouvido.
    fn dock_linha_do_tempo(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let posicao = self.midia.posicao.filter(|p| p.duracao_s > 1.0);
        let progresso = posicao.map(|p| (p.agora(self.midia.tocando) / p.duracao_s).clamp(0.0, 1.0) as f32);
        let celulas = self.mapa.clone();
        let (ouvido, adiante, vazio) = (c.texto, com_alfa(c.texto, 0.45), com_alfa(c.texto, 0.18));

        let mapa = canvas(
            |_, _, _| (),
            move |area: Bounds<gpui::Pixels>, (), window, _| {
                let n = celulas.len();
                let (largura, altura) = (f32::from(area.size.width), f32::from(area.size.height));
                if n == 0 || largura < 8.0 {
                    return;
                }
                let folga = if largura / n as f32 >= 3.0 { 1.0 } else { 0.0 };
                let passo = largura / n as f32;
                let (x0, meio) = (f32::from(area.origin.x), f32::from(area.origin.y) + altura / 2.0);
                for (i, e) in celulas.iter().enumerate() {
                    let (h, cor): (f32, Hsla) = if *e < 0.0 {
                        (2.0, vazio)
                    } else {
                        let passou = progresso.is_none_or(|p| (i as f32 + 0.5) / n as f32 <= p);
                        (3.0 + e.clamp(0.0, 1.0) * (altura - 3.0), if passou { ouvido } else { adiante })
                    };
                    let barra = Bounds::new(
                        point(px(x0 + i as f32 * passo), px(meio - h / 2.0)),
                        size(px((passo - folga).max(1.0)), px(h)),
                    );
                    window.paint_quad(fill(barra, cor).corner_radii(px(1.0)));
                }
            },
        )
        .flex_1()
        .min_w(px(0.0))
        .h(px(24.0));

        let relogio = |valor: Option<f64>| {
            mono().w(px(44.0)).flex_none().text_center().text_color(c.texto_2).children(valor.map(tempo))
        };
        div()
            .w_full()
            .h(px(32.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .when(posicao.is_some(), |d| {
                d.child(relogio(posicao.map(|p| p.agora(self.midia.tocando).min(p.duracao_s))))
            })
            .child(
                // Na música local, clicar no mapa vai para aquele ponto.
                div()
                    .id("dock-mapa")
                    .relative()
                    .flex_1()
                    .min_w(px(0.0))
                    .h(px(24.0))
                    .flex()
                    .when(self.local.ativo(), |d| d.cursor_pointer())
                    .child(mapa)
                    .child(self.marca("dock-mapa"))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, ev: &gpui::MouseDownEvent, _, cx| {
                            this.ir_no_mapa(ev.position.x);
                            cx.notify();
                        }),
                    ),
            )
            .when(posicao.is_some(), |d| d.child(relogio(posicao.map(|p| p.duracao_s))))
    }

    /// Barra de volume do sistema. Arrastar além das pontas estica a barra, que volta sozinha ao soltar; a roda
    /// do mouse anda de 2 em 2; o número só aparece enquanto se mexe.
    fn dock_volume(&self, largura: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let v = if self.mudo { 0.0 } else { self.volume.clamp(0.0, 1.0) };
        let esticado = if self.arrastando_volume {
            self.volume_esticado
        } else {
            self.anim.seguir(("volume-esticado", 0), 0.0, 0.35)
        };
        let (barra_x, barra_w) =
            if esticado < 0.0 { (esticado, largura - esticado) } else { (0.0, largura + esticado) };
        let preenchido = if esticado < 0.0 { v * largura - esticado } else { v * barra_w };

        let pista = self.pista_volume.clone();
        let marcador = canvas(move |area, _, _| pista.set(area), |_, (), _, _| {})
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .size_full();

        let mut controle = div()
            .id("volume")
            .group("volume")
            .relative()
            .w(px(largura))
            .h(px(32.0))
            .flex_none()
            .ml(px(4.0))
            .mr(px(8.0))
            .child(marcador)
            .child(
                div()
                    .absolute()
                    .left(px(barra_x))
                    .top(px(14.0))
                    .w(px(barra_w))
                    .h(px(4.0))
                    .rounded(px(999.0))
                    .bg(com_alfa(c.texto, 0.2))
                    .child(div().w(px(preenchido.clamp(0.0, barra_w))).h_full().rounded(px(999.0)).bg(c.texto)),
            );
        if !self.volume_disponivel {
            return controle.opacity(0.4);
        }
        let bolinha = div()
            .absolute()
            .left(px(v * largura - 7.0))
            .top(px(9.0))
            .size(px(14.0))
            .rounded(px(999.0))
            .bg(c.texto)
            .shadow_md();
        controle = if self.arrastando_volume {
            controle.child(bolinha).child(
                numero(16.0)
                    .absolute()
                    .left(px(v * largura - 20.0))
                    .top(px(-10.0))
                    .w(px(40.0))
                    .text_center()
                    .child(format!("{}", (v * 100.0).round() as u32)),
            )
        } else {
            controle.child(bolinha.invisible().group_hover("volume", |s| s.visible()))
        };
        controle
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseDownEvent, _, cx| {
                    this.pegar_volume(ev.position.x);
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, _, cx| {
                let giro = f32::from(ev.delta.pixel_delta(px(20.0)).y);
                if giro != 0.0 {
                    this.mudar_volume(this.volume + 0.02 * giro.signum());
                    cx.notify();
                }
            }))
    }
}
