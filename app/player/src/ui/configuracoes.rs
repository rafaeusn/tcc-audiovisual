//! A página Configurações: tudo o que muda o Player, em seções (como as Configurações do island e do Windows).
//!
//! Larga, as seções ficam numa coluna à esquerda e o conteúdo à direita. Média, as seções viram fichas no topo
//! (que rolam para o lado). Estreita (layout empilhado), a página inteira vai para a área do painel, rolando como os
//! módulos, e o visual continua em cima. O conteúdo reaproveita os blocos de ajuste (`painel::blocos_de_ajuste`);
//! a seção da Ilha é montada aqui.

use super::painel::{bloco_ajuste, interruptor, linha_ajuste};
use super::{icone, medidas};
use crate::app::Player;
use crate::arranjo::Modo;
use crate::ui::medidas::px;
use gpui::{ClickEvent, Context, Div, FontWeight, SharedString, Stateful, div, prelude::*};
use nucleo::{BordaIlha, Comando, Pagina, TamanhoIlha};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Secao {
    #[default]
    Geral,
    Aparencia,
    Visual,
    Som,
    Ilha,
    Letras,
    Servicos,
    Atalhos,
    Sobre,
}

/// Seção, nome, ícone e o que tem nela.
const SECOES: [(Secao, &str, &str, &str); 9] = [
    (Secao::Geral, "Geral", "icones/panel.svg", "Janela, bandeja e dicas dos botões."),
    (Secao::Aparencia, "Aparência", "icones/tone.svg", "Tema, skin, cores e o modo seguro."),
    (Secao::Visual, "Visual", "icones/eye.svg", "Qual visual se move com a música."),
    (Secao::Som, "Som", "icones/wave.svg", "O que o Modo Escuta ouve e como as músicas do computador tocam."),
    (Secao::Ilha, "Ilha", "icones/ilha.svg", "A pílula com o que toca, presa ao topo da tela."),
    (Secao::Letras, "Letras", "icones/lyrics.svg", "De onde vêm as letras, tradução e letras latinas."),
    (Secao::Servicos, "Serviços", "icones/globe.svg", "As contas e por onde as músicas de cada serviço tocam."),
    (Secao::Atalhos, "Atalhos", "icones/keyboard.svg", "As teclas de cada comando."),
    (Secao::Sobre, "Sobre", "icones/info.svg", "Versão, privacidade e diagnóstico."),
];

impl Player {
    /// Em janela estreita (layout empilhado), as Configurações ocupam a área do painel, e não uma folha por cima
    /// do visual (que ali seria baixa demais).
    pub(crate) fn configuracoes_no_painel(&self) -> bool {
        // Na Estante não há painel no empilhado: as Configurações ficam na folha, como as outras páginas.
        self.pagina() == Pagina::Configuracoes && self.arranjo.modo == Modo::Empilhado && !self.estante()
    }

    fn secao_escolhida(&self) -> (usize, &'static str, &'static str) {
        let i = SECOES.iter().position(|(s, ..)| *s == self.secao_config).unwrap_or(0);
        (i, SECOES[i].1, SECOES[i].3)
    }

    /// Os blocos da seção escolhida, para a coluna de conteúdo de `largura`.
    fn blocos_da_secao(&self, largura: f32, cx: &mut Context<Self>) -> Vec<Div> {
        let b = self.blocos_de_ajuste(largura, cx);
        match self.secao_config {
            Secao::Geral => vec![b.janela],
            Secao::Aparencia => vec![b.aparencia, b.cores, b.seguro],
            Secao::Visual => vec![b.visual],
            Secao::Som => vec![b.escuta, b.reproducao],
            Secao::Ilha => vec![self.bloco_ilha(largura, cx)],
            Secao::Letras => vec![self.bloco_letras(largura, cx)],
            Secao::Servicos => self.bloco_servicos(largura, cx),
            Secao::Atalhos => vec![b.atalhos],
            Secao::Sobre => vec![b.sobre],
        }
    }

    /// As seções como fichas numa fileira que rola para o lado (janela média ou estreita).
    fn fichas_das_secoes(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let mut fileira = div().id("cfg-fichas").w_full().overflow_x_scroll().flex().gap(px(8.0)).pb(px(4.0));
        for (i, (secao, nome, caminho, _)) in SECOES.into_iter().enumerate() {
            let ativa = secao == self.secao_config;
            fileira = fileira.child(
                medidas::apoio()
                    .id(("cfg-ficha", i))
                    .map(|d| self.focavel(d, medidas::PILULA))
                    .h(px(medidas::FICHA))
                    .px(px(14.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(8.0))
                    .rounded(px(medidas::PILULA))
                    .border_1()
                    .border_color(if ativa { c.texto } else { c.linha_2 })
                    .cursor_pointer()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(if ativa { c.texto } else { c.texto_2 })
                    .when(ativa, |d| d.bg(c.sel))
                    .hover(|s| s.bg(c.hover))
                    .child(icone(caminho, medidas::ICONE_LINHA, if ativa { c.texto } else { c.texto_2 }))
                    .child(nome)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.secao_config = secao;
                        cx.notify();
                    })),
            );
        }
        fileira
    }

    fn cabeca_das_configuracoes(&self, grande: bool, lado: f32, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        div()
            .flex()
            .flex_none()
            .items_start()
            .justify_between()
            .gap(px(12.0))
            .px(px(lado))
            .pt(px(lado.min(24.0)))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(4.0))
                    .child(medidas::pagina(grande).child("Configurações"))
                    .child(
                        medidas::apoio().text_color(c.texto_2).child("As mudanças valem na hora e ficam guardadas."),
                    ),
            )
            .child(self.botao_icone("cfg-fechar", "icones/x.svg", false, Comando::AbrirAjustes, cx))
    }

    fn titulo_da_secao(&self) -> Div {
        let c = self.cores();
        let (_, nome, frase) = self.secao_escolhida();
        div()
            .pb(px(8.0))
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(medidas::destaque().font_weight(FontWeight::SEMIBOLD).child(nome))
            .child(medidas::apoio().text_color(c.texto_2).child(frase))
    }

    /// A página como folha por cima do visual (janela larga ou média).
    pub(crate) fn ver_configuracoes(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let largo = a.heroi.w >= 760.0;
        let lado = if a.heroi.w >= 640.0 { 32.0 } else { 16.0 };
        let largura_nav = 220.0;
        // A borda da folha (1 px de cada lado) fica de fora da conta.
        let dentro = a.heroi.w - 4.0;
        let largura_conteudo =
            if largo { (dentro - largura_nav - 3.0 * lado).min(760.0) } else { (dentro - 2.0 * lado).max(200.0) };
        let blocos = self.blocos_da_secao(largura_conteudo, cx);
        let cabeca = self.cabeca_das_configuracoes(a.heroi.w >= 640.0, lado, cx);
        let conteudo = div()
            .id("cfg-conteudo")
            .flex_1()
            .min_w(px(0.0))
            .overflow_y_scroll()
            .track_scroll(&self.rolagem_da_pagina())
            .pb(px(24.0))
            .child(div().w(px(largura_conteudo)).flex().flex_col().child(self.titulo_da_secao()).children(blocos));

        if largo {
            let mut nav = div().w(px(largura_nav)).flex_none().flex().flex_col().gap(px(2.0));
            for (i, (secao, nome, caminho, _)) in SECOES.into_iter().enumerate() {
                let ativa = secao == self.secao_config;
                nav = nav.child(
                    div()
                        .id(("cfg-secao", i))
                        .map(|d| self.focavel(d, medidas::RAIO_CARTAO))
                        .h(px(48.0))
                        .px(px(12.0))
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .rounded(px(medidas::RAIO_CARTAO))
                        .cursor_pointer()
                        .when(ativa, |d| d.bg(c.sel))
                        .hover(|s| s.bg(c.hover))
                        .child(icone(caminho, medidas::ICONE_LINHA, if ativa { c.texto } else { c.texto_2 }))
                        .child(
                            medidas::corpo()
                                .font_weight(FontWeight::MEDIUM)
                                .text_color(if ativa { c.texto } else { c.texto_2 })
                                .child(nome),
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.secao_config = secao;
                            cx.notify();
                        })),
                );
            }
            div()
                .size_full()
                .flex()
                .flex_col()
                .gap(px(20.0))
                .child(cabeca)
                .child(div().flex_1().min_h(px(0.0)).px(px(lado)).flex().gap(px(lado)).child(nav).child(conteudo))
        } else {
            div()
                .size_full()
                .flex()
                .flex_col()
                .gap(px(16.0))
                .child(cabeca)
                .child(div().px(px(lado)).child(self.fichas_das_secoes(cx)))
                .child(div().flex_1().min_h(px(0.0)).px(px(lado)).flex().child(conteudo))
        }
    }

    /// A página na área do painel (janela estreita): rola inteira, com as seções em fichas no topo.
    pub(crate) fn ver_configuracoes_no_painel(&self, largura: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let lado = if largura < 300.0 { 10.0 } else { 16.0 };
        let largura_conteudo = (largura - 2.0 * lado).max(160.0);
        let blocos = self.blocos_da_secao(largura_conteudo, cx);
        div()
            .id("cfg-painel")
            .size_full()
            .overflow_y_scroll()
            .pb(px(24.0))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(self.cabeca_das_configuracoes(false, lado, cx))
            .child(div().px(px(lado)).child(self.fichas_das_secoes(cx)))
            .child(div().px(px(lado)).flex().flex_col().child(self.titulo_da_secao()).children(blocos))
    }

    /// A seção da Ilha: ligar, tamanho, como abre, os avisos e o lugar.
    /// A seção das letras: as fontes (ligar, desligar, ordem), a palavra por palavra, a tradução e as letras latinas.
    fn bloco_letras(&self, largura: f32, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let aj = &self.ajustes;
        let recuo = if largura < 340.0 { 0.0 } else { 44.0 };
        let largura_seletor = (largura - 2.0 - recuo).clamp(160.0, 420.0);
        let pos =
            |chave: &'static str, i: u32, ligado: bool| self.anim.seguir((chave, i), f32::from(u8::from(ligado)), 0.2);
        let mut bloco = bloco_ajuste(&c, true).child(linha_ajuste(
            &c,
            "icones/globe.svg",
            "Fontes",
            "Perguntadas nesta ordem; a primeira com a letra sincronizada vence. Better Lyrics e Unison têm muitas \
             palavra por palavra; o LRCLIB, por linha."
                .into(),
        ));
        let n = aj.letra_fontes.len();
        for (i, chave) in aj.letra_fontes.iter().enumerate() {
            let Some(fonte) = letras::Fonte::da_chave(chave) else { continue };
            let ligada = !aj.letra_fontes_desligadas.contains(chave);
            let descricao = match fonte {
                letras::Fonte::BetterLyrics => "Palavra por palavra (precisa do álbum e da duração).",
                letras::Fonte::Unison => "Base aberta e comunitária; acha as faixas do YouTube Music pelo vídeo.",
                letras::Fonte::Lrclib => "Base aberta de letras sincronizadas por linha.",
            };
            let seta = |id: &'static str, icone_: &'static str, ativa: bool| {
                div()
                    .id((id, i))
                    .size(px(36.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .rounded(px(medidas::PILULA))
                    .when(ativa, |d| d.cursor_pointer().hover(|s| s.bg(c.hover)))
                    .child(icone(icone_, medidas::ICONE_LINHA, if ativa { c.texto_2 } else { c.linha_2 }))
            };
            let (subir, descer, alternar) = (chave.clone(), chave.clone(), chave.clone());
            bloco = bloco.child(
                div()
                    .pl(px(recuo))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(
                                medidas::corpo()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(if ligada { c.texto } else { c.texto_3 })
                                    .child(format!("{}. {}", i + 1, fonte.nome())),
                            )
                            .child(medidas::apoio().text_color(c.texto_2).child(descricao)),
                    )
                    .child(seta("letra-fonte-subir", "icones/chev-u.svg", i > 0).on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.mover_fonte_de_letra(&subir, -1);
                            cx.notify();
                        },
                    )))
                    .child(seta("letra-fonte-descer", "icones/chev-d.svg", i + 1 < n).on_click(cx.listener(
                        move |this, _: &ClickEvent, _, cx| {
                            this.mover_fonte_de_letra(&descer, 1);
                            cx.notify();
                        },
                    )))
                    .child(
                        div()
                            .id(("letra-fonte-ligar", i))
                            .cursor_pointer()
                            .child(interruptor(
                                &c,
                                "letra-fonte-interruptor",
                                ligada,
                                pos("letra-fonte", i as u32, ligada),
                            ))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.alternar_fonte_de_letra(&alternar);
                                cx.notify();
                            })),
                    ),
            );
        }
        let frase = |sim: &'static str, nao: &'static str, ligado: bool| -> SharedString {
            (if ligado { sim } else { nao }).into()
        };
        let idiomas: [(&str, &str); 5] =
            [("pt", "Português"), ("en", "English"), ("es", "Español"), ("fr", "Français"), ("ja", "日本語")];
        let escolhido = idiomas.iter().position(|(cod, _)| *cod == aj.letra_idioma).unwrap_or(0);
        let mut seletor_idioma = self.seletor("seletor-idioma-letra", largura_seletor, idiomas.len(), escolhido);
        for (i, (cod, nome)) in idiomas.into_iter().enumerate() {
            seletor_idioma =
                seletor_idioma.child(self.opcao(("idioma-letra", i), nome, aj.letra_idioma == cod).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.ajustes.letra_idioma = cod.to_string();
                        this.salvar();
                        cx.notify();
                    }),
                ));
        }
        bloco
            .child(
                linha_ajuste(
                    &c,
                    "icones/lyrics.svg",
                    "Preferir palavra por palavra",
                    frase(
                        "Uma letra palavra por palavra de uma fonte de baixo ganha de uma por linha de uma de cima (pergunta a mais fontes).",
                        "Vale a primeira fonte com a letra sincronizada, por linha ou por palavra.",
                        aj.letra_palavra_por_palavra,
                    ),
                )
                .child(
                    interruptor(&c, "cfg-letra-palavra", aj.letra_palavra_por_palavra, pos("letra-palavra", 0, aj.letra_palavra_por_palavra))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.ajustes.letra_palavra_por_palavra = !this.ajustes.letra_palavra_por_palavra;
                            this.salvar();
                            this.configurar_letras();
                            cx.notify();
                        })),
                ),
            )
            .child(
                linha_ajuste(
                    &c,
                    "icones/globe.svg",
                    "Traduzir a letra",
                    frase(
                        "Cada linha ganha a tradução embaixo. A letra e o id do vídeo vão para a Unison, que traduz pelo Google; a tradução fica guardada.",
                        "A letra aparece como veio.",
                        aj.letra_traduzir,
                    ),
                )
                .child(
                    interruptor(&c, "cfg-letra-traduzir", aj.letra_traduzir, pos("letra-traduzir", 0, aj.letra_traduzir))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.ajustes.letra_traduzir = !this.ajustes.letra_traduzir;
                            this.salvar();
                            cx.notify();
                        })),
                ),
            )
            .child(div().pl(px(recuo)).child(seletor_idioma))
            .child(
                linha_ajuste(
                    &c,
                    "icones/lyrics.svg",
                    "Letras latinas",
                    frase(
                        "Coreano, japonês (kana), chinês (pinyin), russo, grego e outras escritas ganham a versão em letras latinas embaixo, feita no computador.",
                        "Cada letra aparece na escrita original.",
                        aj.letra_romanizar,
                    ),
                )
                .child(
                    interruptor(&c, "cfg-letra-romanizar", aj.letra_romanizar, pos("letra-romanizar", 0, aj.letra_romanizar))
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.ajustes.letra_romanizar = !this.ajustes.letra_romanizar;
                            this.salvar();
                            cx.notify();
                        })),
                ),
            )
    }

    fn bloco_ilha(&self, largura: f32, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let aj = &self.ajustes;
        let recuo = if largura < 340.0 { 0.0 } else { 44.0 };
        let largura_seletor = (largura - 2.0 - recuo).clamp(160.0, 420.0);
        let pos = |chave: &'static str, ligado: bool| self.anim.seguir((chave, 0), f32::from(u8::from(ligado)), 0.2);
        let frase = |sim: &'static str, nao: &'static str, ligado: bool| -> SharedString {
            (if ligado { sim } else { nao }).into()
        };
        let escolhido = TamanhoIlha::TODOS.iter().position(|t| *t == aj.ilha_tamanho).unwrap_or(1);
        let mut tamanhos = self.seletor("seletor-tamanho-ilha", largura_seletor, TamanhoIlha::TODOS.len(), escolhido);
        for (i, t) in TamanhoIlha::TODOS.into_iter().enumerate() {
            tamanhos = tamanhos.child(
                self.opcao(("tamanho-ilha", i), t.nome(), aj.ilha_tamanho == t)
                    .on_click(self.ao_clicar(Comando::TrocarTamanhoIlha(t), cx)),
            );
        }
        // Onde a Ilha fica: uma das quatro bordas (escolher aqui não precisa de arrastar).
        let borda_escolhida = BordaIlha::TODAS.iter().position(|b| *b == aj.ilha_borda).unwrap_or(0);
        let mut bordas = self.seletor("seletor-borda-ilha", largura_seletor, BordaIlha::TODAS.len(), borda_escolhida);
        for (i, b) in BordaIlha::TODAS.into_iter().enumerate() {
            let nome = match b {
                BordaIlha::Topo => "Topo",
                BordaIlha::Baixo => "Embaixo",
                BordaIlha::Esquerda => "Esquerda",
                BordaIlha::Direita => "Direita",
            };
            bordas = bordas.child(
                self.opcao(("borda-ilha", i), nome, aj.ilha_borda == b && aj.ilha_solta.is_none())
                    .on_click(self.ao_clicar(Comando::PrenderIlha(b), cx)),
            );
        }
        let frase_lugar: SharedString = if aj.ilha_solta.is_some() {
            "Solta no meio da tela. Escolha uma borda, arraste até uma ou trave: ela volta à última borda.".into()
        } else if aj.ilha_borda.de_pe() {
            "Dos lados, a Ilha vira uma barra lateral; aberta, as páginas ficam na vertical.".into()
        } else {
            "Em que borda da tela a Ilha fica.".into()
        };
        // Em que monitor: junto da janela do Player ou um escolhido (só aparece com mais de um ligado).
        const NOMES_DOS_MONITORES: [&str; 6] =
            ["Monitor 1", "Monitor 2", "Monitor 3", "Monitor 4", "Monitor 5", "Monitor 6"];
        let monitores = integracao::ilha::monitores();
        let monitores = &monitores[..monitores.len().min(NOMES_DOS_MONITORES.len())];
        let monitor_escolhido = aj.ilha_monitor.as_ref().and_then(|n| monitores.iter().position(|m| &m.nome == n));
        let mut seletor_monitor = self.seletor(
            "seletor-monitor-ilha",
            largura_seletor,
            monitores.len() + 1,
            monitor_escolhido.map_or(0, |i| i + 1),
        );
        seletor_monitor =
            seletor_monitor.child(self.opcao(("monitor-ilha", 0), "Do Player", monitor_escolhido.is_none()).on_click(
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    this.ajustes.ilha_monitor = None;
                    this.salvar();
                    cx.notify();
                }),
            ));
        for (i, m) in monitores.iter().enumerate() {
            let nome = m.nome.clone();
            seletor_monitor = seletor_monitor.child(
                self.opcao(("monitor-ilha", i + 1), NOMES_DOS_MONITORES[i], monitor_escolhido == Some(i)).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.ajustes.ilha = true;
                        this.ajustes.ilha_monitor = Some(nome.clone());
                        this.salvar();
                        cx.notify();
                    }),
                ),
            );
        }
        let frase_monitor: SharedString = match monitor_escolhido {
            Some(i) if monitores[i].principal => format!("Sempre no monitor {} (o principal).", i + 1).into(),
            Some(i) => format!("Sempre no monitor {}.", i + 1).into(),
            None => "Junto da janela do Player. Arrastar a Ilha até outro monitor também a deixa lá.".into(),
        };
        let frase_tamanho: SharedString = match aj.ilha_tamanho {
            TamanhoIlha::Minima => "Mínima: fechada, só o som em barras; aberta, um pouco menor.",
            TamanhoIlha::Padrao => "Padrão: a capa e o som fechada; páginas grandes aberta.",
            TamanhoIlha::Grande => "Grande: tudo maior, para telas grandes ou para ler de longe.",
        }
        .into();
        bloco_ajuste(&c, true)
            .child(
                linha_ajuste(
                    &c,
                    "icones/ilha.svg",
                    "Mostrar a Ilha",
                    frase(
                        "Enquanto a música toca, um entalhe preto no topo da tela mostra a capa e o som. Ela sai do caminho com jogo ou vídeo em tela cheia, com o Player maximizado na frente e depois de 30 s parada.",
                        "Desligada. Também dá para ligar com a tecla D ou pelo botão no dock, e fechar pelo × da Ilha aberta.",
                        aj.ilha,
                    ),
                )
                .child(
                    interruptor(&c, "cfg-ilha", aj.ilha, pos("interruptor-ilha", aj.ilha))
                        .on_click(self.ao_clicar(Comando::AlternarIlha, cx)),
                ),
            )
            .child(linha_ajuste(&c, "icones/expand.svg", "Tamanho", frase_tamanho))
            .child(div().pl(px(recuo)).child(tamanhos))
            .child(
                linha_ajuste(
                    &c,
                    "icones/swap.svg",
                    "Abrir ao parar o mouse em cima",
                    frase(
                        "Basta encostar o mouse nela (ou na borda de cima da tela, sobre ela) para abrir; ao tirar, ela fecha.",
                        "Ela só abre com um clique.",
                        aj.ilha_ao_pairar,
                    ),
                )
                .child(
                    interruptor(&c, "cfg-ilha-pairar", aj.ilha_ao_pairar, pos("interruptor-ilha-pairar", aj.ilha_ao_pairar))
                        .on_click(self.ao_clicar(Comando::AlternarIlhaAoPairar, cx)),
                ),
            )
            .child(
                linha_ajuste(
                    &c,
                    "icones/bolt.svg",
                    "Avisos",
                    frase(
                        "Ela alarga por um instante para avisar a música nova, o volume e, nas músicas pré-analisadas, a contagem do drop.",
                        "Sem avisos: ela só muda de forma quando você abre.",
                        aj.ilha_avisos,
                    ),
                )
                .child(
                    interruptor(&c, "cfg-ilha-avisos", aj.ilha_avisos, pos("interruptor-ilha-avisos", aj.ilha_avisos))
                        .on_click(self.ao_clicar(Comando::AlternarIlhaAvisos, cx)),
                ),
            )
            .child(linha_ajuste(&c, "icones/grip.svg", "Lugar", frase_lugar))
            .child(div().pl(px(recuo)).child(bordas))
            .child(
                linha_ajuste(
                    &c,
                    if aj.ilha_travada { "icones/lock.svg" } else { "icones/unlock.svg" },
                    "Travar a Ilha",
                    frase(
                        "Arrastar é sempre livre: travada, ela desliza só ao longo da borda em que está (e passa para outro monitor na mesma borda). Destrave para mandá-la a qualquer borda ou deixá-la solta.",
                        "Destravada: arraste a Ilha para qualquer canto, de qualquer monitor; perto de uma borda, ela gruda nela. Largada no meio, fica solta; travar a devolve à última borda.",
                        aj.ilha_travada,
                    ),
                )
                .child(
                    interruptor(&c, "cfg-ilha-trava", aj.ilha_travada, pos("interruptor-ilha-trava", aj.ilha_travada))
                        .on_click(self.ao_clicar(Comando::AlternarTravaIlha, cx)),
                ),
            )
            .when(monitores.len() > 1, |d| {
                d.child(linha_ajuste(&c, "icones/monitor.svg", "Monitor", frase_monitor))
                    .child(div().pl(px(recuo)).child(seletor_monitor))
            })
            .child(
                div().pl(px(recuo)).flex().child(
                    medidas::corpo()
                        .id("cfg-ilha-centro")
                        .h(px(medidas::BOTAO))
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(medidas::PILULA))
                        .border_1()
                        .border_color(c.linha_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/ilha.svg", medidas::ICONE_LINHA, c.texto))
                        .child("Voltar ao centro do topo")
                        .on_click(self.ao_clicar(Comando::CentralizarIlha, cx)),
                ),
            )
            .child(medidas::apoio().text_color(c.texto_3).child(
                "Aberta, a Ilha tem páginas: Tocando, Letra, Visual (o visual de verdade, reagindo ao som), Ritmo e Ajustes. Troque pelas abas, arrastando o conteúdo para o lado ou com a roda do mouse.",
            ))
    }
}
