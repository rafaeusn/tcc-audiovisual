//! O herói: a moldura do visual, com a faixa que está tocando por cima. As páginas Biblioteca e Loja abrem
//! como uma folha sobre ele, e o visual continua rodando por baixo.

use super::{Botao, caixa, icone, saida, texto, titulo};
use crate::app::Player;
use crate::arranjo::RAIO;
use gpui::{Context, Div, FontWeight, Hsla, SharedString, WindowControlArea, div, prelude::*, px, rgba};
use nucleo::{Comando, Pagina};

/// Sobre o visual o texto é sempre claro: o fundo do visual é preto em qualquer tema.
fn claro(alfa: u32) -> Hsla {
    rgba(0xffffff00 | alfa).into()
}

/// Tamanho do título para um nome com `letras` caracteres: nomes compridos encolhem em degraus, para caberem em
/// duas linhas sem tomar o visual.
pub fn tamanho_do_titulo(base: f32, letras: usize) -> f32 {
    let fator = match letras {
        0..=14 => 1.0,
        15..=24 => 0.84,
        25..=40 => 0.68,
        _ => 0.56,
    };
    (base * fator).round().max(22.0)
}

impl Player {
    pub(crate) fn ver_heroi(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let margem = if a.titulo >= 64.0 {
            32.0
        } else if a.titulo >= 40.0 {
            24.0
        } else {
            16.0
        };
        let mut heroi = caixa(a.heroi).overflow_hidden();
        // Na skin imersiva o visual é o fundo da janela inteira: o herói não tem moldura.
        if !a.so_visual && !self.imersivo() {
            heroi = heroi.rounded(px(RAIO)).border_1().border_color(if self.edicao { c.linha_2 } else { c.linha });
        }

        // Sem a camada do visual: explica o que houve em vez de deixar um buraco.
        if let Some(erro) = &self.erro_visual {
            return heroi
                .bg(gpui::black())
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .p(px(24.0))
                .child(texto(16.0).text_color(claro(0xff)).font_weight(FontWeight::SEMIBOLD).child(erro.clone()))
                .child(
                    texto(14.0)
                        .text_color(claro(0xc2))
                        .child("O som continua sendo analisado. Atualize o driver de vídeo e abra o app de novo."),
                );
        }

        // Em cima: qual visual está rodando (clique troca) e o atalho para os ajustes.
        let chip = div()
            .id("visual-chip")
            .h(px(32.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .px(px(12.0))
            .rounded(px(999.0))
            .bg(rgba(0x0a0a0c99))
            .border_1()
            .border_color(claro(0x1f))
            .cursor_pointer()
            .hover(|s| s.bg(rgba(0x1c1c20cc)))
            .child(icone("icones/wave.svg", 16.0, claro(0xc2)))
            .child(
                texto(14.0).text_color(claro(0xf2)).font_weight(FontWeight::SEMIBOLD).child(self.ajustes.modelo.nome()),
            )
            // Na janela mini o visual inteiro arrasta a janela: o botão precisa ficar "por cima" disso.
            .when(self.mini, |d| d.occlude())
            .child(self.marca("visual-chip"))
            .on_hover(self.ao_pairar("visual-chip", Some(Comando::ProximoVisual), "", cx))
            .on_click(self.ao_clicar(Comando::ProximoVisual, cx));
        // Janela mini: os controles aparecem com o ponteiro em cima da janela e somem assim que ele sai.
        let mostra = if self.mini {
            self.anim.seguir(("mini-controles", 0), f32::from(u8::from(self.ponteiro_na_janela())), 0.12)
        } else {
            1.0
        };
        let redondo =
            |id: &'static str, caminho: &'static str, tamanho: f32, comando: Comando, cx: &mut Context<Self>| {
                self.botao(
                    Botao {
                        id,
                        icone: caminho,
                        tamanho,
                        tamanho_icone: tamanho * 0.5,
                        raio: 999.0,
                        cor: claro(0xf2),
                        fundo: Some(rgba(0x16161acc).into()),
                        realce: rgba(0x3a3a40e6).into(),
                        afunda: 0.94,
                    },
                    cx,
                )
                .occlude()
                .on_click(self.ao_clicar(comando, cx))
            };
        let mut topo =
            div().absolute().left(px(margem)).right(px(margem)).top(px(margem)).flex().items_center().justify_between();
        if self.edicao {
            // No modo edição o herói só mostra o que ele é.
            topo = topo.child(
                texto(14.0)
                    .h(px(32.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .rounded(px(999.0))
                    .bg(rgba(0x0a0a0c99))
                    .border_1()
                    .border_color(claro(0x1f))
                    .text_color(claro(0xf2))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Visual"),
            );
        } else {
            topo = topo.child(chip).when(!a.so_visual, |d| {
                d.child(
                    self.botao(
                        Botao {
                            id: "visual-ajustes",
                            icone: "icones/sliders.svg",
                            tamanho: 40.0,
                            tamanho_icone: 20.0,
                            raio: 999.0,
                            cor: claro(0xf2),
                            fundo: Some(rgba(0x2a2a2e99).into()),
                            realce: rgba(0x3a3a40cc).into(),
                            afunda: 0.94,
                        },
                        cx,
                    )
                    .on_click(self.ao_clicar(Comando::AbrirAjustes, cx))
                    .on_hover(self.ao_pairar(
                        "visual-ajustes",
                        Some(Comando::AbrirAjustes),
                        "",
                        cx,
                    )),
                )
            });
            if self.mini {
                // Na janela mini: voltar ao tamanho de antes.
                topo = topo.opacity(mostra).child(redondo(
                    "mini-voltar",
                    "icones/expand.svg",
                    34.0,
                    Comando::AlternarMini,
                    cx,
                ));
            }
        }
        // Janela mini: anterior, tocar/pausar e próxima. Com largura, ficam no canto de baixo à direita e o título
        // usa o resto da linha; em janela estreita sobem para baixo do topo, centralizados, e o título fica com a
        // largura toda. O lugar deles é reservado mesmo escondidos, para o título não pular ao passar o mouse.
        const BOTAO_LADO: f32 = 34.0;
        const BOTAO_MEIO: f32 = 42.0;
        const VAO_DOS_BOTOES: f32 = 6.0;
        /// Largura mínima que precisa sobrar para o título ao lado dos controles.
        const TITULO_MINIMO: f32 = 150.0;
        let largura_dos_controles = 2.0 * BOTAO_LADO + BOTAO_MEIO + 2.0 * VAO_DOS_BOTOES;
        let ao_lado = self.mini && a.heroi.w >= 2.0 * margem + largura_dos_controles + 12.0 + TITULO_MINIMO;
        let controles = (self.mini && mostra > 0.02).then(|| {
            let tocar = if self.midia.tocando { "icones/pause.svg" } else { "icones/play.svg" };
            let grupo = div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(VAO_DOS_BOTOES))
                .child(redondo("mini-anterior", "icones/prev.svg", BOTAO_LADO, Comando::Anterior, cx))
                .child(redondo("mini-tocar", tocar, BOTAO_MEIO, Comando::TocarPausar, cx))
                .child(redondo("mini-proxima", "icones/next.svg", BOTAO_LADO, Comando::Proxima, cx));
            if ao_lado {
                div().absolute().right(px(margem)).bottom(px(margem)).opacity(mostra).child(grupo)
            } else {
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(margem + 34.0 + 6.0))
                    .w_full()
                    .flex()
                    .justify_center()
                    .opacity(mostra)
                    .child(grupo)
            }
        });

        // Embaixo: de onde vem o som, o título e o artista. Sem faixa, o convite para tocar algo.
        let (origem, nome, sub): (SharedString, SharedString, SharedString) = match self.midia.faixa.as_ref() {
            Some(f) if !f.titulo.is_empty() => {
                (f.app.clone().into(), f.titulo.clone().into(), f.artista.clone().into())
            }
            _ if self.retrato.gate > 0.5 => {
                ("Modo Escuta".into(), "Som do sistema".into(), "O visual segue tudo o que o PC está tocando.".into())
            }
            _ => ("Modo Escuta".into(), "Toque uma música em qualquer app".into(), "O visual começa sozinho.".into()),
        };
        let ao_vivo = self.midia.tocando || self.retrato.gate > 0.5;
        let tamanho = tamanho_do_titulo(a.titulo, nome.chars().count());
        let tamanho_sub = if tamanho >= 40.0 { 20.0 } else { 14.0 };
        // Troca de faixa: o título novo sobe 8 px e aparece em 260 ms; o artista chega 40 ms depois.
        let desde = self.troca_de_faixa.elapsed().as_secs_f32();
        let (entra_titulo, entra_sub) = (saida(desde / 0.26), saida((desde - 0.04) / 0.26));
        let apagado = if self.edicao { 0.45 } else { 1.0 };
        // Janela mini baixa: o artista e a origem saem, para o título não disputar espaço com os controles.
        let (mini_baixa, mini_minima) = (self.mini && a.heroi.h < 190.0, self.mini && a.heroi.h < 170.0);
        let sub: SharedString = if mini_baixa { "".into() } else { sub };
        let rodape = div()
            .absolute()
            .left(px(margem))
            .right(px(if ao_lado { margem + largura_dos_controles + 12.0 } else { margem }))
            .bottom(px(margem))
            .flex()
            .flex_col()
            .gap(px(if tamanho >= 48.0 { 8.0 } else { 4.0 }))
            .opacity(apagado)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .when(mini_minima, |d| d.hidden())
                    .child(icone("icones/headphones.svg", 16.0, claro(0xcc)))
                    .child(
                        texto(14.0)
                            .min_w(px(0.0))
                            .truncate()
                            .text_color(claro(0xcc))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(origem),
                    )
                    .when(ao_vivo, |d| d.child(div().size(px(6.0)).rounded(px(999.0)).bg(claro(0xf4)))),
            )
            .child(
                titulo(tamanho)
                    .relative()
                    .top(px(8.0 * (1.0 - entra_titulo)))
                    .opacity(entra_titulo)
                    // Folga embaixo: sem ela o corte em duas linhas comia a perna do "g" e do "p".
                    .pb(px((tamanho * 0.08).ceil()))
                    .max_w(px((tamanho * 11.0).min(a.heroi.w - 2.0 * margem).max(120.0)))
                    .text_color(claro(0xff))
                    .line_clamp(2)
                    .child(nome),
            )
            .when(!sub.is_empty(), |d| {
                d.child(
                    texto(tamanho_sub)
                        .relative()
                        .top(px(8.0 * (1.0 - entra_sub)))
                        .opacity(entra_sub)
                        .text_color(claro(0xc2))
                        .font_weight(FontWeight::MEDIUM)
                        .truncate()
                        .child(sub),
                )
            });

        if self.pagina != Pagina::Inicio && !a.so_visual {
            // Com uma página aberta, o título e os botões do visual saem: só o visual fica por trás da folha.
            heroi.child(self.ver_folha(cx))
        } else {
            // Fechada: a próxima abertura anima do começo.
            self.anim.definir(("folha", 0), 0.0);
            // Na janela mini não há topo: o visual inteiro arrasta a janela (os botões ficam por cima).
            heroi
                .when(self.mini, |h| {
                    h.child(div().absolute().size_full().occlude().window_control_area(WindowControlArea::Drag))
                })
                .child(rodape)
                // Com os controles escondidos, eles também saem da frente do arrasto.
                .when(!self.mini || mostra > 0.02, |h| h.child(topo))
                .children(controles)
        }
    }

    /// Folha de página por cima do visual. Ela nasce do lado da barra de ícones (de onde foi aberta) em 240 ms;
    /// fechar é imediato.
    fn ver_folha(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let p = self.anim.seguir(("folha", 0), 1.0, 0.24);
        let folha = div()
            .absolute()
            .left(px(-24.0 * (1.0 - p)))
            .top(px(12.0 * (1.0 - p)))
            .w_full()
            .h(px((self.arranjo.heroi.h - 24.0 * (1.0 - p)).max(0.0)))
            .opacity(p)
            .rounded(px(RAIO))
            .bg(c.folha)
            .occlude()
            .flex()
            .flex_col();
        if self.pagina == Pagina::Loja {
            return folha.child(self.ver_loja(cx));
        }
        if self.pagina == Pagina::Biblioteca {
            return folha.child(self.ver_biblioteca(cx));
        }
        // Página sem conteúdo próprio: diz o que vem, sem inventar dados.
        folha
            .items_center()
            .justify_center()
            .gap(px(12.0))
            .p(px(24.0))
            .child(icone("icones/lib.svg", 32.0, c.texto_2))
            .child(titulo(28.0).child("Biblioteca"))
            .child(texto(14.0).max_w(px(440.0)).text_color(c.texto_2).text_center().child(
                "Aqui vão entrar as suas pastas de música, com fila e busca. Por enquanto o Player funciona no Modo Escuta: toque em qualquer app e o visual acompanha.",
            ))
            .child(
                texto(14.0)
                    .id("folha-voltar")
                    .mt(px(8.0))
                    .h(px(40.0))
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .rounded(px(999.0))
                    .bg(c.inv)
                    .text_color(c.inv_texto)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|s| s.opacity(0.9))
                    .child("Voltar ao visual")
                    .on_click(self.ao_clicar(Comando::AbrirPagina(Pagina::Inicio), cx)),
            )
    }
}

#[cfg(test)]
mod testes {
    use super::tamanho_do_titulo;

    #[test]
    fn titulo_encolhe_com_o_tamanho_do_nome() {
        assert_eq!(tamanho_do_titulo(64.0, 7), 64.0);
        assert_eq!(tamanho_do_titulo(64.0, 20), 54.0);
        assert_eq!(tamanho_do_titulo(64.0, 33), 44.0);
        assert_eq!(tamanho_do_titulo(64.0, 80), 36.0);
        // Nunca fica ilegível, nem em janela pequena com nome enorme.
        assert_eq!(tamanho_do_titulo(28.0, 80), 22.0);
        assert!(tamanho_do_titulo(40.0, 30) < tamanho_do_titulo(40.0, 10));
    }
}
