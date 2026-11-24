//! A página da faixa: capa, título, artistas e álbum (links quando o provedor sabe abrir), tocar, curtir, a seguir e
//! fila, e as abas Letra (a da F4, quando a faixa toca), Mapa (a pré-análise: energia, trechos, drops, BPM e tom) e
//! Parecidas (só quando o provedor sabe fazer rádio).

use super::{com_alfa, icone, mono, tempo, texto, titulo};
use crate::app::Player;
use crate::arranjo::Modo;
use crate::paginas::{AbaDaFaixa, abas_da_faixa, rota_do_album, rota_do_artista};
use biblioteca::TipoSalvo;
use gpui::{
    AnyElement, Bounds, ClickEvent, Context, Div, FontWeight, Hsla, SharedString, canvas, div, fill, point, prelude::*,
    px, size,
};
use nucleo::Comando;
use provedores::{Capacidade, Faixa, Item};
use std::sync::Arc;

/// Altura do desenho do mapa.
const ALTURA_DO_MAPA: f32 = 120.0;
/// Os nomes dos trechos da pré-análise (nível 0, 1 e 2).
const TRECHOS: [&str; 3] = ["Calma", "Agitada", "Intensa"];

impl Player {
    /// A faixa da página é a que o Player está tocando agora (e é ela o "tocando agora", não outro app)?
    fn faixa_tocando(&self, f: &Faixa) -> bool {
        self.origem_local
            && self
                .local
                .atual()
                .is_some_and(|m| m.remota.as_ref().is_some_and(|r| r.id == f.id && r.servico == f.servico))
    }

    pub(crate) fn ver_faixa(&self, item: &Item, cx: &mut Context<Self>) -> Div {
        let Item::Faixa(f) = item else { return div() };
        let c = self.cores();
        let (p, dx) = self.entrada();
        let largura = self.arranjo.heroi.w;
        let lado = if largura >= 720.0 { 168.0 } else { 120.0 };
        let tamanho = if f.titulo.chars().count() > 28 { 28.0 } else { 38.0 };
        let tocando = self.faixa_tocando(f);

        // Artistas · álbum · duração; os nomes viram links quando o provedor deu o id e sabe abrir a página.
        let sabe_artista = self.sabe(&f.servico, Capacidade::Artista);
        let mut linha = div().flex().flex_wrap().items_center().text_color(c.texto_2);
        let link = |id: (&'static str, usize),
                    rotulo: String,
                    rota: Option<crate::navegacao::Rota>,
                    cx: &mut Context<Self>|
         -> AnyElement {
            match rota {
                Some(rota) => texto(14.0)
                    .id(id)
                    .text_color(c.texto)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|s| s.underline())
                    .child(rotulo)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.abrir_rota(rota.clone());
                        cx.notify();
                    }))
                    .into_any_element(),
                None => texto(14.0).child(rotulo).into_any_element(),
            }
        };
        for (k, nome) in f.artistas.iter().enumerate() {
            if k > 0 {
                linha = linha.child(texto(14.0).child(", "));
            }
            linha =
                linha.child(link(("faixa-pagina-artista", k), nome.clone(), rota_do_artista(f, k, sabe_artista), cx));
        }
        if let Some(album) = f.album.as_ref().filter(|a| !a.is_empty()) {
            let rota = rota_do_album(f, self.sabe(&f.servico, Capacidade::Album));
            linha =
                linha.child(texto(14.0).child(" · ")).child(link(("faixa-pagina-album", 0), album.clone(), rota, cx));
        }
        if let Some(d) = f.duracao_s {
            linha = linha.child(texto(14.0).child(format!(" · {}", tempo(d))));
        }

        let item_ = item.clone();
        let (esta, a_seguir, no_fim) = (f.clone(), f.clone(), f.clone());
        let (rotulo, simbolo) = match (tocando, self.local.situacao.tocando) {
            (true, true) => ("Pausar", "icones/pause.svg"),
            (true, false) => ("Continuar", "icones/play.svg"),
            _ => ("Tocar", "icones/play.svg"),
        };
        let acoes = div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(10.0))
            .child(self.pilula("faixa-pagina-tocar", simbolo, rotulo, true).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| {
                    if this.faixa_tocando(&esta) {
                        this.executar(Comando::TocarPausar, window, cx);
                    } else {
                        this.tocar_faixas(std::slice::from_ref(&esta), 0);
                    }
                    cx.notify();
                },
            )))
            .child(self.pilula("faixa-pagina-a-seguir", "icones/play-next.svg", "A seguir", false).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.notify();
                    if this.avisar_se_nao_toca(&a_seguir.servico) {
                        return;
                    }
                    let m = crate::remoto::musica_de(&a_seguir, &this.nome_do_servico(&a_seguir.servico));
                    this.local.tocar_a_seguir(m);
                    this.avisar(format!("\"{}\" toca a seguir", a_seguir.titulo));
                    cx.notify();
                }),
            ))
            .child(self.pilula("faixa-pagina-fila", "icones/queue-add.svg", "Na fila", false).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    cx.notify();
                    if this.avisar_se_nao_toca(&no_fim.servico) {
                        return;
                    }
                    let m = crate::remoto::musica_de(&no_fim, &this.nome_do_servico(&no_fim.servico));
                    this.local.adicionar_a_fila(vec![m]);
                    this.avisar(format!("\"{}\" foi para o fim da fila", no_fim.titulo));
                    cx.notify();
                },
            )))
            .child(self.coracao(("faixa-pagina-coracao", 41_000), &item_, 20.0, cx));

        // Pelo Match: a pessoa vê o que toca de verdade e pode trocar.
        let musica = crate::remoto::musica_de(f, &self.nome_do_servico(&f.servico));
        let pelo_par = self.local.remoto.par(&musica).map(|par| {
            let nome = self.nome_do_servico(&par.servico);
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap(px(10.0))
                .child(
                    texto(13.0)
                        .max_w_full()
                        .truncate()
                        .text_color(c.texto_2)
                        .child(format!("Toca pelo {nome}: {} · {}", par.titulo, par.artista)),
                )
                .child(
                    texto(13.0)
                        .id("faixa-outro-par")
                        .text_color(c.texto)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.underline())
                        .child("Procurar outra gravação")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.local.outro_par(&musica);
                            cx.notify();
                        })),
                )
        });

        let cabecalho = div()
            .flex()
            .items_end()
            .gap(px(20.0))
            .child(self.capa(f.capa.as_deref(), TipoSalvo::Faixa, lado, false).shadow_lg())
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(texto(12.0).text_color(c.texto_2).font_weight(FontWeight::SEMIBOLD).child(if f.video {
                        "CLIPE"
                    } else {
                        "FAIXA"
                    }))
                    .child(titulo(tamanho).line_clamp(2).child(SharedString::from(f.titulo.clone())))
                    .child(linha)
                    .children(pelo_par)
                    .child(div().mt(px(8.0)).child(acoes)),
            );

        // As abas, num seletor como o da Biblioteca.
        let abas = abas_da_faixa(self.sabe(&f.servico, Capacidade::Radio));
        let aba = if abas.contains(&self.aba_da_faixa) { self.aba_da_faixa } else { AbaDaFaixa::Letra };
        let escolhida = abas.iter().position(|a| *a == aba).unwrap_or(0);
        let mut seletor =
            self.seletor("seletor-faixa", (110.0 * abas.len() as f32).min(largura - 48.0), abas.len(), escolhida);
        for (i, a) in abas.into_iter().enumerate() {
            seletor = seletor.child(self.opcao(("faixa-aba", i), a.nome(), a == aba).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.aba_da_faixa = a;
                    cx.notify();
                },
            )));
        }
        let conteudo = match aba {
            AbaDaFaixa::Letra => self.letra_da_pagina(tocando, cx),
            AbaDaFaixa::Mapa => self.mapa_da_pagina(f, tocando),
            AbaDaFaixa::Parecidas => {
                self.aviso_da_pagina("As parecidas chegam quando o provedor mandar o rádio da faixa.")
            }
        };

        div()
            .relative()
            .left(px(dx))
            .opacity(p)
            .flex()
            .flex_col()
            .gap(px(24.0))
            .p(px(24.0))
            .child(cabecalho)
            .child(seletor)
            .child(conteudo)
    }

    fn aviso_da_pagina(&self, frase: &'static str) -> Div {
        div().child(texto(14.0).text_color(self.cores().texto_2).child(frase))
    }

    /// A letra é a da faixa que toca (a F4 busca só essa): de outra faixa, a frase diz quando ela aparece.
    fn letra_da_pagina(&self, tocando: bool, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        if !self.modulo_ativo(crate::catalogo::ID_LETRA) {
            return self.aviso_da_pagina("Ligue o módulo Letra na Loja para ver a letra aqui.");
        }
        if !tocando {
            return self.aviso_da_pagina("A letra aparece aqui quando esta faixa estiver tocando.");
        }
        // O módulo Letra já está à vista no painel: a mesma letra nos dois lugares andaria em dobro (a mola e as
        // dicas são uma só). Aqui fica o aviso e a tela cheia.
        let no_painel = match self.arranjo.modo {
            Modo::Largo => self.ajustes.painel_aberto,
            Modo::Gaveta => self.gaveta_aberta,
            _ => false,
        } && self.ajustes.modulos.contains(&nucleo::Modulo::Letra);
        let letra =
            if no_painel { self.aviso_da_pagina("A letra está no painel ao lado.") } else { self.corpo_letra(cx) };
        div().max_w(px(640.0)).flex().flex_col().items_start().gap(px(16.0)).child(letra).child(
            texto(13.0)
                .id("faixa-letra-cheia")
                .h(px(36.0))
                .px(px(14.0))
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .border_1()
                .border_color(c.linha_2)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.bg(c.hover))
                .child(icone("icones/expand.svg", 16.0, c.texto))
                .child("Letra em tela cheia")
                .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.executar(Comando::LetraTelaCheia, window, cx);
                    cx.notify();
                })),
        )
    }

    /// O mapa da pré-análise: a energia em barras, na cor do trecho (calma, agitada, intensa), os drops em cima e,
    /// tocando, a posição de agora. Embaixo, BPM, tom, drops e a legenda.
    fn mapa_da_pagina(&self, f: &Faixa, tocando: bool) -> Div {
        let c = self.cores();
        let chave = std::path::PathBuf::from(f.chave());
        let Some(analise) = self.local.preanalista().analise(&chave) else {
            return self.aviso_da_pagina(if tocando {
                "Analisando a faixa…"
            } else {
                "O mapa aparece quando a faixa é analisada: isso acontece quando ela toca ou é uma das próximas da fila."
            });
        };
        let acento = self.paleta_do_espaco().acento;
        let cores: [Hsla; 3] = [com_alfa(c.texto, 0.32), com_alfa(acento, 0.62), acento];
        let marca = c.texto;
        let posicao =
            tocando.then(|| (self.local.situacao.posicao_s / analise.duracao_s.max(1.0)).clamp(0.0, 1.0) as f32);
        let a = Arc::clone(&analise);
        let desenho = canvas(
            |_, _, _| (),
            move |area: Bounds<gpui::Pixels>, (), window, _| {
                let (largura, altura) = (f32::from(area.size.width), f32::from(area.size.height));
                let duracao = a.duracao_s.max(1.0);
                let n = ((largura / 5.0) as usize).clamp(8, 240);
                let celulas = a.mapa(n);
                let passo = largura / n as f32;
                let (x0, base) = (f32::from(area.origin.x), f32::from(area.origin.y) + altura);
                let nivel_em =
                    |t: f64| a.trechos.iter().rev().find(|tr| tr.inicio_s <= t).map_or(0, |tr| tr.nivel.min(2));
                for (i, e) in celulas.iter().enumerate() {
                    let t = (i as f64 + 0.5) / n as f64 * duracao;
                    let h = 3.0 + e.clamp(0.0, 1.0) * (altura - 14.0);
                    let barra = Bounds::new(
                        point(px(x0 + i as f32 * passo), px(base - h)),
                        size(px((passo - 1.5).max(1.0)), px(h)),
                    );
                    window.paint_quad(fill(barra, cores[nivel_em(t) as usize]).corner_radii(px(1.5)));
                }
                // Os drops: um tracinho no alto.
                for d in &a.drops {
                    let x = x0 + (d.t / duracao) as f32 * largura;
                    window.paint_quad(
                        fill(Bounds::new(point(px(x - 1.5), area.origin.y), size(px(3.0), px(8.0))), marca)
                            .corner_radii(px(1.5)),
                    );
                }
                if let Some(p) = posicao {
                    let x = x0 + p * largura;
                    window.paint_quad(fill(
                        Bounds::new(point(px(x - 1.0), area.origin.y), size(px(2.0), px(altura))),
                        marca,
                    ));
                }
            },
        )
        .w_full()
        .h(px(ALTURA_DO_MAPA));

        let dado = |rotulo: &'static str, valor: String| {
            div()
                .flex()
                .flex_col()
                .gap(px(2.0))
                .child(texto(12.0).text_color(c.texto_3).font_weight(FontWeight::SEMIBOLD).child(rotulo))
                .child(mono().text_color(c.texto).child(valor))
        };
        let tom = analise.tom.as_ref().map_or_else(|| "—".to_string(), |t| format!("{} · {}", t.nome, t.camelot));
        let bpm = if analise.bpm > 0.0 { format!("{:.0}", analise.bpm) } else { "—".to_string() };
        let mut legenda = div().flex().items_center().gap(px(14.0));
        for (nome, cor) in TRECHOS.iter().zip(cores) {
            legenda = legenda.child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(6.0))
                    .child(div().size(px(10.0)).rounded(px(3.0)).bg(cor))
                    .child(texto(12.0).text_color(c.texto_2).child(*nome)),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(18.0))
            .child(desenho)
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(28.0))
                    .child(dado("BPM", bpm))
                    .child(dado("TOM", tom))
                    .child(dado("DROPS", analise.drops.len().to_string()))
                    .child(dado("DURAÇÃO", tempo(analise.duracao_s))),
            )
            .child(legenda)
    }
}
