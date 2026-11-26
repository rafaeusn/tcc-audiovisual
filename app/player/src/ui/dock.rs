//! O dock: a faixa que está tocando, os controles, o mapa de energia e o volume. O fundo dele segue a cor do
//! visual.

use super::{Botao, caixa, com_alfa, icone, medidas, mono, numero, tempo};
use crate::app::{AbaDoPainel, Player};
use crate::arranjo::{Modo, RAIO};
use crate::menu::Alvo;
use crate::paginas::{Destino, destino_do_artista, destino_do_titulo};
use crate::tema;
use crate::ui::medidas::px;
use gpui::{
    App, Bounds, ClickEvent, Context, Div, FontWeight, Hsla, MouseButton, MouseDownEvent, PathBuilder,
    ScrollWheelEvent, SharedString, Stateful, canvas, div, fill, img, point, prelude::*, size,
};
use nucleo::{Comando, Tema};
use provedores::{Capacidade, Item};

/// Play e pause como dois polígonos de 4 pontos (numa grade de 24), para um virar o outro ponto a ponto.
const PLAY: [[(f32, f32); 4]; 2] =
    [[(8.0, 5.0), (14.0, 8.5), (14.0, 15.5), (8.0, 19.0)], [(14.0, 8.5), (20.0, 12.0), (20.0, 12.0), (14.0, 15.5)]];
const PAUSE: [[(f32, f32); 4]; 2] =
    [[(7.0, 5.0), (10.5, 5.0), (10.5, 19.0), (7.0, 19.0)], [(13.5, 5.0), (17.0, 5.0), (17.0, 19.0), (13.5, 19.0)]];

/// O botão de ícone do dock e o de tocar.
const DOCK_BOTAO: f32 = 40.0;
const TOCAR: f32 = 52.0;

impl Player {
    pub(crate) fn ver_dock(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let a = &self.arranjo;
        let tom = tema::cor_de(self.tom);
        let base = caixa(a.dock)
            .rounded(px(RAIO))
            .bg(if self.imersivo() { com_alfa(tom, 0.72) } else { tom })
            // Cliques no dock não passam para o visual que, na skin imersiva, está por baixo.
            .occlude();
        if a.modo == Modo::Empilhado && a.dock.h <= crate::arranjo::DOCK_EMPILHADO_BAIXO {
            // Janela baixa: uma linha só (a capa, o nome, os controles e o ⋯); o tempo fica para a janela maior.
            return base
                .flex()
                .items_center()
                .gap(px(8.0))
                .px(px(10.0))
                .child(self.dock_faixa(52.0, false, cx).flex_1())
                .child(self.dock_controles(false, cx))
                .child(self.botao_mais(("dock-mais", 0), DOCK_BOTAO, || Alvo::Barra, cx));
        }
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
                        .child(self.dock_faixa(52.0, a.dock.w >= 440.0, cx).flex_1())
                        .child(self.dock_controles(a.dock.w >= 440.0, cx))
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
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(self.dock_linha_do_tempo(cx).flex_1())
                        .child(self.botao_mais(("dock-mais", 0), 28.0, || Alvo::Barra, cx)),
                );
        }

        let painel_visivel = match a.modo {
            _ if self.estante() => a.painel.is_some(),
            Modo::Largo => self.ajustes.painel_aberto,
            _ => self.gaveta_aberta,
        };
        // Com largura, o play fica no meio da barra. Abaixo disso o nome da música vale mais que o meio exato: a coluna
        // dele fica com pelo menos 264, e a barra de volume sai (o botão de mudo fica).
        let larga = a.dock.w >= 1100.0;
        let pista = if larga { 88.0 } else { 0.0 };
        // Apertada, a letra e o mudo vão para o ⋯.
        let apertada = self.barra_apertada();
        let botoes = 3.0 + if apertada { 0.0 } else { 2.0 };
        let direita = DOCK_BOTAO * botoes + if pista > 0.0 { pista + 12.0 } else { 0.0 };
        let esquerda = if larga { direita } else { 264.0 };
        let meio = (a.dock.w - 36.0 - 48.0 - direita - esquerda).clamp(252.0, 560.0);
        let lados =
            |d: Div| if larga { d.flex_1().flex_basis(px(0.0)).min_w(px(0.0)) } else { d.flex_none().w(px(direita)) };
        base.flex()
            .items_center()
            .gap(px(24.0))
            .px(px(18.0))
            .child(self.dock_faixa(64.0, !apertada, cx).flex_1().flex_basis(px(0.0)))
            .child(
                div()
                    .w(px(meio))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .items_center()
                    .child(self.dock_controles(true, cx))
                    .child(self.dock_linha_do_tempo(cx)),
            )
            .child(
                lados(div())
                    .flex()
                    .items_center()
                    .justify_end()
                    .when(!apertada, |d| {
                        d.child(self.dock_botao(
                            "dock-letra",
                            "icones/lyrics.svg",
                            self.letra_na_aba() || self.letra_cheia,
                            Comando::AbrirLetra,
                            cx,
                        ))
                    })
                    .child(self.dock_botao(
                        "dock-fila",
                        "icones/queue.svg",
                        (self.painel_a_vista() && self.aba_do_painel == AbaDoPainel::Fila)
                            || self.nav.atual() == Some(&crate::navegacao::Rota::Fila),
                        Comando::AbrirFila,
                        cx,
                    ))
                    .child(self.dock_botao(
                        "dock-painel",
                        "icones/panel.svg",
                        painel_visivel,
                        Comando::AlternarPainel,
                        cx,
                    ))
                    .when(!apertada, |d| {
                        d.child(self.dock_botao("dock-mudo", self.icone_do_volume(), false, Comando::Silenciar, cx))
                    })
                    .when(pista > 0.0, |d| d.child(self.dock_volume(pista, cx)))
                    .child(self.botao_mais(("dock-mais", 0), DOCK_BOTAO, || Alvo::Barra, cx)),
            )
    }

    /// A barra não tem lugar para a letra, o coração e o mudo: eles vão para o ⋯.
    pub(crate) fn barra_apertada(&self) -> bool {
        self.arranjo.dock.w < 800.0
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
                tamanho: DOCK_BOTAO,
                tamanho_icone: medidas::ICONE,
                raio: medidas::PILULA,
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

    /// O clique que leva a um destino da música (o álbum, o artista).
    fn ao_destino(
        &self,
        destino: Destino,
        cx: &Context<Self>,
    ) -> impl Fn(&ClickEvent, &mut gpui::Window, &mut App) + 'static {
        cx.listener(move |this, _: &ClickEvent, _, cx| {
            this.ir_ao_destino_do_clique(destino.clone());
            cx.notify();
        })
    }

    /// Capa, título e artista (ou de onde vem o som). Com a música do Player, a capa e o título levam ao álbum e o
    /// artista à página dele; o som de outro app não leva a lugar nenhum.
    fn dock_faixa(&self, lado: f32, com_coracao: bool, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let capa = match &self.capa {
            Some(imagem) => img(imagem.clone())
                .size(px(lado))
                .flex_none()
                .rounded(px(medidas::raio_da_capa(lado)))
                // No tema Mono nada tem cor, nem a capa.
                .grayscale(self.ajustes.tema == Tema::Mono)
                .into_any_element(),
            None => div()
                .size(px(lado))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(medidas::raio_da_capa(lado)))
                .bg(c.ponto)
                .child(icone("icones/headphones.svg", medidas::ICONE, c.texto_2))
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
        let de_onde = self.servicos_do_tocando();
        let servicos: Vec<&str> = de_onde.iter().map(String::as_str).collect();
        // Dois selos: a faixa é de um serviço e toca por outro.
        let pelo_outro = servicos.len() > 1;
        let capa = self
            .com_selo(capa, &servicos, medidas::MARCA_DA_BARRA)
            .id("dock-capa")
            .child(self.marca("dock-capa"))
            .when(pelo_outro, |d| d.on_hover(self.ao_pairar("dock-capa", None, "Tocando pelo serviço da frente", cx)));
        let tocando = self.local.atual().filter(|_| self.origem_local);
        let sabe = |capacidade| {
            let servico = tocando.and_then(|m| m.remota.as_ref()).map(|r| r.servico.as_str());
            servico.is_some_and(|s| self.sabe(s, capacidade))
        };
        let ao_titulo = tocando.and_then(|m| destino_do_titulo(m, sabe(Capacidade::Album)));
        let ao_artista = tocando.and_then(|m| destino_do_artista(m, sabe(Capacidade::Artista)));
        let capa = match ao_titulo.clone() {
            Some(d) => capa.cursor_pointer().on_click(self.ao_destino(d, cx)),
            None => capa,
        };
        let titulo = medidas::corpo().id("dock-titulo").font_weight(FontWeight::SEMIBOLD).child(super::linha(nome));
        let titulo = match ao_titulo {
            Some(d) => titulo.cursor_pointer().hover(|s| s.underline()).on_click(self.ao_destino(d, cx)),
            None => titulo,
        };
        let artista = medidas::apoio().id("dock-artista").text_color(c.texto_2).child(super::linha(sub));
        let artista = match ao_artista {
            Some(d) => {
                artista.cursor_pointer().hover(|s| s.underline().text_color(c.texto)).on_click(self.ao_destino(d, cx))
            }
            None => artista,
        };
        let coracao = self
            .faixa_que_toca()
            .filter(|_| com_coracao)
            .map(|f| self.coracao(("dock-coracao", 60_000), &Item::Faixa(f), medidas::ICONE, cx));
        let do_menu = tocando.map(|m| Player::alvo_da_musica(m, self.local.fila.atual()));
        div()
            .min_w(px(0.0))
            .flex()
            .items_center()
            .gap(px(16.0))
            .child(capa)
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .gap(px(4.0))
                    .child(div().flex_1().min_w(px(0.0)).flex().flex_col().items_start().child(titulo).child(artista))
                    .children(coracao),
            )
            .when_some(do_menu, |d, alvo| d.on_mouse_down(MouseButton::Right, self.ao_menu(move || alvo.clone(), cx)))
    }

    /// Anterior, tocar/pausar e próxima. Agem no app que está tocando; sem app, ficam apagados.
    /// `modos`: com o aleatório e o repetir (em janela estreita eles dão lugar ao nome da música; ficam na Fila).
    fn dock_controles(&self, modos: bool, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let ativo = self.midia.disponivel;
        let lateral =
            |id: &'static str, caminho: &'static str, comando: Comando, cx: &mut Context<Self>| -> Stateful<Div> {
                if !ativo {
                    return div()
                        .id(id)
                        .size(px(medidas::ALVO))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .opacity(0.4)
                        .child(icone(caminho, medidas::ICONE, c.texto_2));
                }
                self.botao(
                    Botao {
                        id,
                        icone: caminho,
                        tamanho: medidas::ALVO,
                        tamanho_icone: medidas::ICONE,
                        raio: medidas::PILULA,
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
                            gpui::px(x0 + (de.0 + (para.0 - de.0) * forma) * escala),
                            gpui::px(y0 + (de.1 + (para.1 - de.1) * forma) * escala),
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
        .size(px(medidas::ICONE * e));
        let mut tocar =
            div().id("dock-tocar").size(px(TOCAR)).flex_none().flex().items_center().justify_center().child(
                div()
                    .size(px(TOCAR * e))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(medidas::PILULA))
                    .bg(c.inv)
                    .child(desenho),
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
        // Aleatório e repetir só existem para as músicas do computador: os outros apps têm os deles.
        let local = self.origem_local && modos;
        let modo = |id: &'static str, caminho: &'static str, comando: Comando, ligado: bool, cx: &mut Context<Self>| {
            self.botao(
                Botao {
                    id,
                    icone: caminho,
                    tamanho: DOCK_BOTAO,
                    tamanho_icone: medidas::ICONE_LINHA,
                    raio: medidas::PILULA,
                    cor: if ligado { c.texto } else { c.texto_3 },
                    fundo: ligado.then_some(com_alfa(c.texto, 0.1)),
                    realce: com_alfa(c.texto, 0.1),
                    afunda: 0.94,
                },
                cx,
            )
            .on_click(self.ao_clicar(comando, cx))
            .on_hover(self.ao_pairar(id, Some(comando), "", cx))
        };
        let (aleatorio, repetir) = (self.local.fila.aleatorio, self.local.fila.repetir);
        let icone_repetir = if repetir == nucleo::Repetir::Faixa { "icones/repeat-1.svg" } else { "icones/repeat.svg" };
        let botao_aleatorio =
            local.then(|| modo("dock-aleatorio", "icones/shuffle.svg", Comando::AlternarAleatorio, aleatorio, cx));
        let botao_repetir = local.then(|| {
            modo("dock-repetir", icone_repetir, Comando::AlternarRepetir, repetir != nucleo::Repetir::Nada, cx)
        });
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .children(botao_aleatorio)
            .child(lateral("dock-anterior", "icones/prev.svg", Comando::Anterior, cx))
            .child(tocar)
            .child(lateral("dock-proxima", "icones/next.svg", Comando::Proxima, cx))
            .children(botao_repetir)
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
                        point(gpui::px(x0 + i as f32 * passo), gpui::px(meio - h / 2.0)),
                        size(gpui::px((passo - folga).max(1.0)), gpui::px(h)),
                    );
                    window.paint_quad(fill(barra, cor).corner_radii(gpui::px(1.0)));
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
            .h(px(28.0))
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
                    .when(self.origem_local, |d| d.cursor_pointer())
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

    /// Barra de volume (da música do Player, ou do sistema: ver [`Player::volume_e_da_musica`]). Arrastar além das pontas estica a barra, que volta sozinha ao soltar; a roda
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
            .child(self.marca("volume"))
            .on_hover(self.ao_pairar(
                "volume",
                None,
                if self.volume_e_da_musica() { "Volume da música" } else { "Volume do PC" },
                cx,
            ))
            .child(marcador)
            .child(
                div()
                    .absolute()
                    .left(px(barra_x))
                    .top(px(14.0))
                    .w(px(barra_w))
                    .h(px(4.0))
                    .rounded(px(medidas::PILULA))
                    .bg(com_alfa(c.texto, 0.2))
                    .child(
                        div().w(px(preenchido.clamp(0.0, barra_w))).h_full().rounded(px(medidas::PILULA)).bg(c.texto),
                    ),
            );
        if !self.volume_disponivel {
            return controle.opacity(0.4);
        }
        let bolinha = div()
            .absolute()
            .left(px(v * largura - 7.0))
            .top(px(9.0))
            .size(px(14.0))
            .rounded(px(medidas::PILULA))
            .bg(c.texto)
            .shadow_md();
        controle = if self.arrastando_volume {
            controle.child(bolinha).child(
                numero(medidas::CORPO)
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
