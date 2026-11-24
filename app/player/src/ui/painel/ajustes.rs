//! Os blocos de ajuste (Aparência, Visual, Som...), usados pela página Configurações.

use super::*;

impl Player {
    /// Os blocos de ajuste, um por assunto, para a página Configurações. `largura` é a da coluna de conteúdo.
    pub(crate) fn blocos_de_ajuste(&self, largura: f32, cx: &mut Context<Self>) -> BlocosDeAjuste {
        let c = self.cores();
        let aj = &self.ajustes;
        // Largura dos seletores: a coluna menos o recuo que alinha com o texto das linhas.
        let recuo = if largura < 340.0 { 0.0 } else { 44.0 };
        let largura_seletor = (largura - 2.0 - recuo).max(160.0);

        // Qual visual.
        let escolhido = Modelo::TODOS.iter().position(|m| *m == aj.modelo).unwrap_or(0);
        let mut visuais = self.seletor("seletor-visual", largura_seletor, Modelo::TODOS.len(), escolhido);
        for (i, m) in Modelo::TODOS.into_iter().enumerate() {
            visuais = visuais.child(
                self.opcao(("visual", i), m.nome(), aj.modelo == m)
                    .on_click(self.ao_clicar(Comando::TrocarVisual(m), cx)),
            );
        }
        let descricao: SharedString = match aj.modelo {
            Modelo::Spectral => "Nuvem de partículas conduzida pelo diretor automático.",
            Modelo::Fluxo => "Correntes de luz que seguem a energia da música.",
            Modelo::Barras => "O espectro do som em barras, com rastro.",
        }
        .into();
        let visual = bloco_ajuste(&c, false)
            .child(linha_ajuste(&c, "icones/eye.svg", "Visual", descricao))
            .child(div().pl(px(recuo)).child(visuais));

        // O que o Modo Escuta ouve: todo o PC ou um app só.
        let escolhido = aj.escuta_app.clone();
        let frase_escuta: SharedString = if escolhido.is_empty() {
            "Todo o som do PC, de qualquer app.".into()
        } else {
            format!(
                "Só o {}. Anúncios e sons de outros apps não mexem no visual nem no tocando agora.",
                sessao::nome_do_app(&escolhido)
            )
            .into()
        };
        // Os apps com som aberto agora, e o escolhido mesmo que esteja fechado.
        let mut apps = self.apps_com_som.clone();
        if !escolhido.is_empty() && !apps.contains(&escolhido) {
            apps.insert(0, escolhido.clone());
        }
        let opcao_de_fonte = |id: (&'static str, usize), nome: SharedString, ativa: bool, aberto: bool| {
            texto(14.0)
                .id(id)
                .h(px(36.0))
                .px(px(14.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .border_1()
                .border_color(if ativa { c.texto } else { c.linha_2 })
                .cursor_pointer()
                .font_weight(FontWeight::MEDIUM)
                .text_color(if ativa { c.texto } else { c.texto_2 })
                .when(ativa, |d| d.bg(c.sel))
                .hover(|s| s.bg(c.hover))
                .child(div().size(px(8.0)).rounded(px(999.0)).bg(if aberto { c.calma } else { c.texto_3 }))
                .child(nome)
        };
        let mut fontes = div().flex().flex_wrap().gap(px(8.0)).child(
            opcao_de_fonte(("fonte", 0), "Todo o PC".into(), escolhido.is_empty(), true).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.escolher_fonte("");
                    cx.notify();
                },
            )),
        );
        for (i, exe) in apps.into_iter().enumerate() {
            let aberto = self.apps_com_som.contains(&exe);
            let nome: SharedString = sessao::nome_do_app(&exe).into();
            let ativa = exe == escolhido;
            fontes = fontes.child(opcao_de_fonte(("fonte", i + 1), nome, ativa, aberto).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.escolher_fonte(&exe);
                    cx.notify();
                },
            )));
        }
        let escuta = bloco_ajuste(&c, false)
            .child(linha_ajuste(&c, "icones/wave.svg", "O que o Modo Escuta ouve", frase_escuta))
            .child(div().pl(px(recuo)).flex().flex_col().gap(px(8.0)).child(fontes).child(
                texto(12.0).text_color(c.texto_3).child(
                    "Aparecem aqui os apps que estão com som aberto agora. Comece a tocar no app para ele aparecer.",
                ),
            ));

        // Cores: seguem a música ou usam uma paleta.
        let frase_cores: SharedString = if aj.cores_reativas {
            "A cor acompanha a energia da música."
        } else {
            "O visual usa a paleta escolhida."
        }
        .into();
        let detalhe_cores = if aj.cores_reativas {
            let item = |cor: Hsla, nome: &'static str| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().size(px(10.0)).rounded(px(999.0)).bg(cor))
                    .child(texto(14.0).text_color(c.texto_2).child(nome))
            };
            div()
                .flex()
                .flex_wrap()
                .gap_x(px(16.0))
                .gap_y(px(8.0))
                .child(item(c.calma, "Calma"))
                .child(item(c.ativa, "Agitada"))
                .child(item(c.intensa, "Intensa"))
        } else {
            let mut amostras = div().flex().flex_wrap().gap(px(10.0));
            for (i, p) in PALETAS.iter().enumerate() {
                let escolhida = aj.paleta == i;
                amostras = amostras.child(
                    div()
                        .id(("paleta", i))
                        .h(px(40.0))
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(999.0))
                        .border_1()
                        .border_color(if escolhida { c.texto } else { c.linha_2 })
                        .cursor_pointer()
                        .when(escolhida, |d| d.bg(c.sel))
                        .hover(|s| s.bg(c.hover))
                        .child(
                            div()
                                .flex()
                                .child(div().size(px(16.0)).rounded(px(999.0)).bg(rgb8(p.a)))
                                .child(div().size(px(16.0)).ml(px(-6.0)).rounded(px(999.0)).bg(rgb8(p.b))),
                        )
                        .child(
                            texto(14.0)
                                .text_color(if escolhida { c.texto } else { c.texto_2 })
                                .font_weight(FontWeight::MEDIUM)
                                .child(p.nome),
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.escolher_paleta(i);
                            cx.notify();
                        })),
                );
            }
            amostras
        };
        let pos_cores = self.anim.seguir(("interruptor-cores", 0), f32::from(u8::from(aj.cores_reativas)), 0.2);
        let cores = bloco_ajuste(&c, false)
            .child(
                linha_ajuste(&c, "icones/drop.svg", "Cores reativas", frase_cores).child(
                    interruptor(&c, "aj-cores", aj.cores_reativas, pos_cores)
                        .on_click(self.ao_clicar(Comando::AlternarCoresReativas, cx)),
                ),
            )
            .child(div().pl(px(recuo)).child(detalhe_cores));

        // Modo seguro.
        let pos_seguro = self.anim.seguir(("interruptor-seguro", 0), f32::from(u8::from(aj.modo_seguro)), 0.2);
        let mut seguro = bloco_ajuste(&c, false).child(
            linha_ajuste(
                &c,
                "icones/shield.svg",
                "Modo seguro",
                "Para quem tem sensibilidade a luzes piscando.".into(),
            )
            .child(
                interruptor(&c, "aj-seguro", aj.modo_seguro, pos_seguro)
                    .on_click(self.ao_clicar(Comando::AlternarModoSeguro, cx)),
            ),
        );
        if aj.modo_seguro {
            let mut regras = div().pl(px(recuo)).flex().flex_col().gap(px(8.0));
            for regra in
                ["Sem clarão no drop", "O brilho e a cor mudam devagar", "Movimento mais calmo, sem pulso na interface"]
            {
                regras = regras.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(icone("icones/check.svg", 16.0, c.texto))
                        .child(texto(14.0).text_color(c.texto_2).child(regra)),
                );
            }
            seguro = seguro.child(regras);
        }

        // Aparência: tema e skin.
        let tema_atual = Tema::TODOS.iter().position(|t| *t == aj.tema).unwrap_or(0);
        let mut temas = self.seletor("seletor-tema", largura_seletor, Tema::TODOS.len(), tema_atual);
        for (i, t) in Tema::TODOS.into_iter().enumerate() {
            temas = temas.child(
                self.opcao(("tema", i), t.nome(), aj.tema == t).on_click(self.ao_clicar(Comando::TrocarTema(t), cx)),
            );
        }
        let imersiva = aj.skin == Skin::Imersiva;
        let mut skins = self.seletor("seletor-skin", largura_seletor, 2, usize::from(imersiva));
        for (i, s) in [Skin::Padrao, Skin::Imersiva].into_iter().enumerate() {
            skins = skins.child(self.opcao(("skin", i), s.nome(), aj.skin == s).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.trocar_skin(s);
                    cx.notify();
                },
            )));
        }
        let estante = aj.layout == nucleo::Layout::Estante;
        let mut layouts = self.seletor("seletor-layout", largura_seletor, 2, usize::from(estante));
        for (i, l) in [nucleo::Layout::Palco, nucleo::Layout::Estante].into_iter().enumerate() {
            layouts = layouts.child(self.opcao(("layout", i), l.nome(), aj.layout == l).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.trocar_layout(l);
                    cx.notify();
                },
            )));
        }
        let aparencia = bloco_ajuste(&c, false)
            .child(linha_ajuste(
                &c,
                "icones/layout.svg",
                "Layout",
                if estante {
                    "Estante: os espaços ao lado, a página no meio e o visual no trilho Tocando (Ctrl L)."
                } else {
                    "Palco: o visual no centro e as páginas por cima dele (Ctrl L)."
                }
                .into(),
            ))
            .child(div().pl(px(recuo)).child(layouts))
            .child(linha_ajuste(
                &c,
                "icones/tone.svg",
                "Tema",
                "As cores da interface. O visual só muda no Mono.".into(),
            ))
            .child(div().pl(px(recuo)).child(temas))
            .child(linha_ajuste(
                &c,
                "icones/immersive.svg",
                "Skin",
                if imersiva {
                    "Imersiva: o visual é o fundo e os painéis são de vidro."
                } else {
                    "Padrão: visual e painel lado a lado."
                }
                .into(),
            ))
            .child(div().pl(px(recuo)).child(skins))
            .child(
                div().pl(px(recuo)).child(
                    texto(14.0)
                        .id("aj-editar")
                        .h(px(40.0))
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .rounded(px(999.0))
                        .border_1()
                        .border_color(c.linha_2)
                        .text_color(c.texto)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/grid.svg", 18.0, c.texto))
                        .child("Personalizar o layout")
                        .child(tecla(&c, "E"))
                        .on_click(self.ao_clicar(Comando::EditarLayout, cx)),
                ),
            );

        // Janela: continuar na bandeja ao fechar e a janela mini.
        let pos_bandeja = self.anim.seguir(("interruptor-bandeja", 0), f32::from(u8::from(aj.fechar_na_bandeja)), 0.2);
        let pos_dicas = self.anim.seguir(("interruptor-dicas", 0), f32::from(u8::from(aj.dicas)), 0.2);
        let pilula = |id: &'static str, caminho: &'static str, rotulo: &'static str| {
            texto(14.0)
                .id(id)
                .h(px(40.0))
                .px(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .border_1()
                .border_color(c.linha_2)
                .text_color(c.texto)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.bg(c.hover))
                .child(icone(caminho, 18.0, c.texto))
                .child(rotulo)
        };
        let janela = bloco_ajuste(&c, false)
            .child(
                linha_ajuste(
                    &c,
                    "icones/panel.svg",
                    "Continuar na bandeja",
                    if aj.fechar_na_bandeja {
                        "Fechar a janela só a esconde. O Player segue ouvindo; para sair, use o ícone ao lado do relógio."
                    } else {
                        "Fechar a janela encerra o Player."
                    }
                    .into(),
                )
                .child(
                    interruptor(&c, "aj-bandeja", aj.fechar_na_bandeja, pos_bandeja)
                        .on_click(self.ao_clicar(Comando::AlternarBandeja, cx)),
                ),
            )
            .child(
                linha_ajuste(
                    &c,
                    "icones/info.svg",
                    "Dicas dos botões",
                    if aj.dicas {
                        "Parar o mouse num ícone mostra o nome e o atalho dele."
                    } else {
                        "Os ícones não mostram nome nem atalho."
                    }
                    .into(),
                )
                .child(
                    interruptor(&c, "aj-dicas", aj.dicas, pos_dicas)
                        .on_click(self.ao_clicar(Comando::AlternarDicas, cx)),
                ),
            )
            .child(
                div().pl(px(recuo)).flex().child(
                    pilula("aj-mini", "icones/expand.svg", "Janela mini")
                        .child(tecla(&c, "N"))
                        .on_click(self.ao_clicar(Comando::AlternarMini, cx)),
                ),
            );

        // Atalhos: os mesmos comandos da barra de comandos, direto da lista de comandos.
        let com_atalho: Vec<Comando> = Comando::TODOS.into_iter().filter(|k| !k.teclas().is_empty()).collect();
        let mut atalhos =
            bloco_ajuste(&c, false).gap(px(0.0)).child(div().pb(px(8.0)).child(titulo_secao(&c, "Atalhos")));
        for (i, comando) in com_atalho.iter().enumerate() {
            let mut grupo = div().flex().flex_none().gap(px(4.0));
            for t in comando.teclas() {
                grupo = grupo.child(tecla(&c, t));
            }
            atalhos = atalhos.child(
                div()
                    .min_h(px(44.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .when(i + 1 < com_atalho.len(), |d| d.border_b_1().border_color(c.linha))
                    .child(texto(14.0).min_w(px(0.0)).text_color(c.texto_2).child(comando.nome()))
                    .child(grupo),
            );
        }

        // Sobre.
        let placa: SharedString = if self.placa.is_empty() {
            "Placa de vídeo: indisponível".into()
        } else {
            format!("Placa de vídeo: {}", self.placa).into()
        };
        let sobre = bloco_ajuste(&c, true)
            .gap(px(8.0))
            .child(titulo_secao(&c, "Sobre"))
            .child(texto(14.0).child(format!("Player {} (nome provisório)", env!("CARGO_PKG_VERSION"))))
            .child(texto(14.0).text_color(c.texto_2).child(
                "No Modo Escuta o app analisa o som que o PC está tocando para mover o visual. Nenhum áudio é gravado nem enviado para fora. O histórico fica só neste computador. Com o módulo Letra no painel, o título e o artista da faixa vão ao LRCLIB para buscar a letra.",
            ))
            .child(texto(14.0).text_color(c.texto_2).child(placa))
            .child(
                div().pt(px(4.0)).flex().child(
                    pilula("aj-diagnostico", "icones/info.svg", "Copiar relatório de diagnóstico")
                        .on_click(self.ao_clicar(Comando::CopiarDiagnostico, cx)),
                ),
            );

        BlocosDeAjuste {
            visual,
            escuta,
            reproducao: self.bloco_reproducao(recuo, largura_seletor, cx),
            cores,
            seguro,
            aparencia,
            janela,
            atalhos,
            sobre,
        }
    }
}
