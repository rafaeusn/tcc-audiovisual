//! A ficha de um módulo da Loja: o que ele é, o que ele pede ao app e o botão de instalar.
//!
//! É aqui que a pessoa aceita (ou não) o que o módulo faz. Cada ação aparece com o motivo que o autor escreveu;
//! as críticas vêm destacadas. Instalar só fica disponível depois de marcar a caixa de que leu.

use super::{RAIO_CONTROLE, RAIO_MODULO, icone, mono, texto, titulo};
use crate::app::Player;
use gpui::{AnyElement, ClickEvent, Context, Div, FontWeight, SharedString, div, prelude::*, px};
use modulos::{Acao, Categoria, Manifesto};

fn icone_da_acao(a: Acao) -> &'static str {
    match a {
        Acao::TocandoLer => "icones/wave.svg",
        Acao::TocandoControlar => "icones/play.svg",
        Acao::Rede => "icones/globe.svg",
        Acao::DadosGuardar => "icones/lib.svg",
        Acao::HistoricoLer => "icones/history.svg",
        Acao::ContaEntrar => "icones/shield.svg",
        Acao::ProgramaLocal => "icones/panel.svg",
        Acao::PainelModulo => "icones/grid.svg",
        Acao::AparenciaMudar => "icones/tone.svg",
        Acao::Avisar => "icones/info.svg",
    }
}

pub fn icone_da_categoria(c: Categoria) -> &'static str {
    match c {
        Categoria::Letras => "icones/lyrics.svg",
        Categoria::Historico => "icones/history.svg",
        Categoria::Presenca => "icones/headphones.svg",
        Categoria::Aparencia => "icones/tone.svg",
        Categoria::Outro => "icones/puzzle.svg",
    }
}

impl Player {
    /// Para o Last.fm e o Discord já instalados: como o serviço está agora e o que dá para fazer (conectar a
    /// conta, por exemplo).
    fn situacao_do_servico(&self, m: &Manifesto, cx: &mut Context<Self>) -> Option<Div> {
        use crate::catalogo::{ID_DISCORD, ID_LASTFM};
        use crate::servicos::{SituacaoDiscord, SituacaoLastfm};
        self.instalados.achar(&m.id)?;
        let c = self.cores();
        // (texto, é um problema, rótulo do botão)
        let (frase, problema, botao): (SharedString, bool, Option<&'static str>) = if m.id == ID_LASTFM {
            match self.servicos.lastfm() {
                SituacaoLastfm::SemChave => (
                    "Este app foi compilado sem as chaves do Last.fm (LASTFM_API_KEY e LASTFM_API_SECRET).".into(),
                    true,
                    None,
                ),
                SituacaoLastfm::Desconectada => {
                    ("Nenhuma conta conectada ainda.".into(), false, Some("Conectar conta"))
                }
                SituacaoLastfm::Autorizando => {
                    ("Aprove o Player na página do Last.fm que abriu no navegador.".into(), false, None)
                }
                SituacaoLastfm::Conectada(usuario) => (
                    format!("Conectado como {usuario}. As músicas ouvidas já estão sendo enviadas.").into(),
                    false,
                    Some("Desconectar conta"),
                ),
                SituacaoLastfm::Falha(motivo) => {
                    (format!("Não deu certo: {motivo}.").into(), true, Some("Conectar conta"))
                }
            }
        } else if m.id == ID_DISCORD {
            match self.servicos.discord() {
                SituacaoDiscord::SemChave => (
                    "Este app foi compilado sem o identificador do aplicativo no Discord (DISCORD_APP_ID).".into(),
                    true,
                    None,
                ),
                SituacaoDiscord::Parado => ("Ligando ao Discord...".into(), false, None),
                SituacaoDiscord::Esperando => (
                    "O Discord não está aberto neste computador. O cartão aparece quando ele abrir.".into(),
                    false,
                    None,
                ),
                SituacaoDiscord::Ligado => {
                    ("Ligado ao Discord. O cartão aparece enquanto a música toca.".into(), false, None)
                }
            }
        } else {
            return None;
        };
        let conectado = matches!(self.servicos.lastfm(), SituacaoLastfm::Conectada(_));
        Some(
            div()
                .p(px(12.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .rounded(px(RAIO_CONTROLE + 2.0))
                .border_1()
                .border_color(if problema { c.alerta } else { c.linha_2 })
                .child(icone("icones/info.svg", 18.0, if problema { c.alerta } else { c.texto_2 }))
                .child(texto(14.0).flex_1().min_w(px(0.0)).child(frase))
                .when_some(botao, |d, rotulo| {
                    d.child(
                        texto(14.0)
                            .id("ficha-conta")
                            .h(px(36.0))
                            .px(px(14.0))
                            .flex()
                            .flex_none()
                            .items_center()
                            .rounded(px(999.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .when(conectado, |b| b.border_1().border_color(c.linha_2).hover(|s| s.bg(c.hover)))
                            .when(!conectado, |b| b.bg(c.inv).text_color(c.inv_texto).hover(|s| s.opacity(0.88)))
                            .child(rotulo)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                if conectado {
                                    this.servicos.desconectar_lastfm();
                                } else {
                                    this.servicos.conectar_lastfm();
                                }
                                cx.notify();
                            })),
                    )
                }),
        )
    }

    pub(crate) fn ver_ficha_do_modulo(&self, m: &Manifesto, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let a = &self.arranjo;
        let instalado = self.instalados.achar(&m.id).is_some();

        let cabeca = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(14.0))
            .p(px(20.0))
            .border_b_1()
            .border_color(c.linha_2)
            .child(
                div()
                    .size(px(48.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(RAIO_CONTROLE + 2.0))
                    .bg(c.sel)
                    .child(icone(icone_da_categoria(m.categoria), 24.0, c.texto)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(titulo(24.0).truncate().child(SharedString::from(m.nome.clone())))
                    .child(texto(12.0).text_color(c.texto_2).truncate().child(format!(
                        "{} · versão {} · por {}",
                        m.categoria.nome(),
                        m.versao,
                        m.autor
                    ))),
            )
            .child(
                div()
                    .id("ficha-fechar")
                    .size(px(36.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(RAIO_CONTROLE))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone("icones/x.svg", 18.0, c.texto_2))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.fechar_ficha();
                        cx.notify();
                    })),
            );

        let mut acoes = div().flex().flex_col();
        for (i, p) in m.acoes.iter().enumerate() {
            let critica = p.critica();
            let cor = if critica { c.alerta } else { c.texto };
            let mut textos = div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(texto(14.0).text_color(cor).font_weight(FontWeight::SEMIBOLD).child(p.acao.rotulo()))
                        .when(critica, |d| {
                            d.child(
                                texto(12.0)
                                    .h(px(20.0))
                                    .px(px(8.0))
                                    .flex()
                                    .flex_none()
                                    .items_center()
                                    .rounded(px(999.0))
                                    .border_1()
                                    .border_color(c.alerta)
                                    .text_color(c.alerta)
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child("Crítica"),
                            )
                        }),
                )
                .child(texto(14.0).text_color(c.texto_2).child(SharedString::from(p.motivo.clone())));
            if !p.enderecos.is_empty() {
                let onde = if p.enderecos.iter().any(|e| e == "*") {
                    "qualquer site".to_string()
                } else {
                    format!("só em {}", p.enderecos.join(", "))
                };
                textos = textos.child(mono().text_color(c.texto_3).child(onde));
            }
            acoes = acoes.child(
                div()
                    .py(px(12.0))
                    .flex()
                    .items_start()
                    .gap(px(12.0))
                    .when(i + 1 < m.acoes.len(), |d| d.border_b_1().border_color(c.linha))
                    .child(div().pt(px(2.0)).child(icone(icone_da_acao(p.acao), 18.0, cor)))
                    .child(textos),
            );
        }

        let corpo = div()
            .id("ficha-corpo")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px(px(20.0))
            .py(px(16.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(texto(14.0).text_color(c.texto_2).child(SharedString::from(m.descricao.clone())))
            .children(self.situacao_do_servico(m, cx))
            .child(
                texto(12.0)
                    .pt(px(4.0))
                    .text_color(c.texto_3)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("O que este módulo faz"),
            )
            .child(acoes)
            .child(texto(12.0).text_color(c.texto_3).child(
                "O módulo só pode fazer o que está nesta lista. Se uma versão nova pedir algo a mais, ela espera você aceitar.",
            ));

        let botao = |id: &'static str, rotulo: &'static str, cheio: bool, ativo: bool| {
            let b = texto(14.0)
                .id(id)
                .h(px(40.0))
                .px(px(18.0))
                .flex()
                .flex_none()
                .items_center()
                .rounded(px(999.0))
                .font_weight(FontWeight::SEMIBOLD)
                .child(rotulo);
            let b = if cheio {
                b.bg(c.inv).text_color(c.inv_texto)
            } else {
                b.border_1().border_color(c.linha_2).text_color(c.texto)
            };
            if ativo { b.cursor_pointer().hover(|s| s.opacity(0.88)) } else { b.opacity(0.4) }
        };
        let rodape: Div =
            div().flex().flex_none().items_center().gap(px(12.0)).p(px(20.0)).border_t_1().border_color(c.linha_2);
        let rodape = if !m.pronto {
            rodape.child(texto(14.0).flex_1().min_w(px(0.0)).text_color(c.texto_2).child(
                "Ainda não dá para instalar: o código deste módulo não ficou pronto. A lista acima é o que ele vai pedir.",
            ))
        } else if instalado {
            rodape
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .child(icone("icones/check.svg", 18.0, c.texto))
                        .child(texto(14.0).font_weight(FontWeight::SEMIBOLD).child("Instalado")),
                )
                .child(botao("ficha-desinstalar", "Desinstalar", false, true).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.desinstalar_modulo_aberto();
                        cx.notify();
                    },
                )))
        } else {
            let marcado = self.aceite_marcado;
            rodape
                .child(
                    div()
                        .id("ficha-aceite")
                        .flex_1()
                        .min_w(px(0.0))
                        .min_h(px(40.0))
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .cursor_pointer()
                        .child(
                            div()
                                .size(px(22.0))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(6.0))
                                .border_1()
                                .border_color(if marcado { c.inv } else { c.texto_2 })
                                .when(marcado, |d| d.bg(c.inv).child(icone("icones/check.svg", 16.0, c.inv_texto))),
                        )
                        .child(texto(14.0).child("Li o que este módulo faz"))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.aceite_marcado = !this.aceite_marcado;
                            cx.notify();
                        })),
                )
                .child(botao("ficha-instalar", "Instalar", true, marcado).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.instalar_modulo_aberto();
                        cx.notify();
                    },
                )))
        };

        let largura = 560.0f32.min(a.janela.w - 32.0).max(240.0);
        let altura = (a.janela.h - 64.0).clamp(240.0, 640.0);
        div()
            .id("ficha-fundo")
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
                this.fechar_ficha();
                cx.notify();
            }))
            .child(
                div()
                    .id("ficha")
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
                    .child(cabeca)
                    .child(corpo)
                    .child(rodape),
            )
            .into_any_element()
    }
}
