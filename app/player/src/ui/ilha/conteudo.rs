//! O conteúdo da Ilha: o repouso, os avisos, as abas e as páginas (Tocando, Letra, Visual, Ritmo, Ajustes).

use super::*;

impl VistaIlha {
    pub(super) fn botao(&self, id: &'static str, caminho: &'static str, lado: f32, ativo: bool) -> Stateful<Div> {
        div()
            .id(id)
            .size(px(lado))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .rounded(px(999.0))
            .cursor_pointer()
            .when(ativo, |d| d.bg(branco(0.16)))
            .hover(|s| s.bg(branco(0.14)))
            .active(|s| s.opacity(0.6))
            .child(icone(caminho, lado * 0.52, branco(if ativo { 1.0 } else { 0.88 })))
    }

    pub(super) fn capa(dados: &Dados, lado: f32, raio: f32) -> AnyElement {
        match &dados.capa {
            Some(imagem) => img(imagem.clone()).size(px(lado)).flex_none().rounded(px(raio)).into_any_element(),
            None => div()
                .size(px(lado))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(raio))
                .bg(linear_gradient(
                    135.0,
                    linear_color_stop(com_alfa(dados.tom, 0.55), 0.0),
                    linear_color_stop(branco(0.08), 1.0),
                ))
                .child(icone("icones/headphones.svg", lado * 0.5, branco(0.9)))
                .into_any_element(),
        }
    }

    /// As barras do espectro, movidas pelo som na cor do visual (baixas e apagadas na pausa).
    pub(super) fn barras(&self, dados: &Dados, largura: f32, altura: f32) -> impl IntoElement {
        let mut linha = div().flex().flex_none().items_center().gap(px(largura * 0.65)).h(px(altura));
        for (i, valor) in dados.barras.iter().enumerate() {
            let alvo = if dados.tocando { (0.2 + valor * 0.8) * altura } else { largura };
            let h = self.anim.seguir(("ilha-barra", i as u32), alvo, 0.08).max(largura);
            linha = linha.child(div().w(px(largura)).h(px(h)).rounded(px(largura)).bg(if dados.tocando {
                dados.tom
            } else {
                branco(0.3)
            }));
        }
        linha
    }
    /// O repouso: a capa e o som em barras (na mínima, só as barras). Em pé (a aba na lateral), um sobre o outro.
    pub(super) fn repouso(&self, dados: &Dados, s: f32, minima: bool, de_pe: bool) -> AnyElement {
        let barras = self.barras(dados, 3.5 * s, 16.0 * s);
        if minima {
            return div().size_full().flex().items_center().justify_center().child(barras).into_any_element();
        }
        if de_pe {
            return div()
                .size_full()
                .py(px(8.0 * s))
                .flex()
                .flex_col()
                .items_center()
                .justify_between()
                .child(Self::capa(dados, 28.0 * s, 8.0 * s))
                .child(div().pb(px(4.0 * s)).child(barras))
                .into_any_element();
        }
        div()
            .size_full()
            .px(px(8.0 * s))
            .flex()
            .items_center()
            .justify_between()
            .child(Self::capa(dados, 24.0 * s, 7.0 * s))
            .child(div().pr(px(4.0 * s)).child(barras))
            .into_any_element()
    }

    pub(super) fn aviso(&self, dados: &Dados, alerta: Alerta, s: f32) -> AnyElement {
        let base = div().size_full().flex().items_center();
        match alerta {
            Alerta::Faixa => base
                .px(px(12.0 * s))
                .gap(px(12.0 * s))
                .child(Self::capa(dados, 44.0 * s, 12.0 * s))
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .flex_col()
                        .child(texto(pequeno(s)).text_color(cinza()).child("Tocando agora"))
                        .child(texto(15.0 * s).font_weight(FontWeight::SEMIBOLD).truncate().child(dados.titulo.clone()))
                        .child(texto(13.0 * s).text_color(cinza()).truncate().child(dados.artista.clone())),
                )
                .child(div().pr(px(6.0 * s)).child(self.barras(dados, 3.5 * s, 22.0 * s)))
                .into_any_element(),
            Alerta::Volume => base
                .px(px(18.0 * s))
                .gap(px(12.0 * s))
                .child(icone("icones/vol.svg", 18.0 * s, branco(1.0)))
                .child(div().flex_1().h(px(5.0 * s)).rounded(px(3.0 * s)).bg(branco(0.18)).child(
                    div().h_full().rounded(px(3.0 * s)).bg(branco(1.0)).w(gpui::relative(dados.volume.clamp(0.0, 1.0))),
                ))
                .child(
                    texto(13.0 * s)
                        .w(px(30.0 * s))
                        .text_color(cinza())
                        .child(format!("{:.0}", dados.volume.clamp(0.0, 1.0) * 100.0)),
                )
                .into_any_element(),
            Alerta::Drop => {
                let falta = dados.drop_em_s.unwrap_or(0.0);
                let rotulo = if falta < 0.5 { "Drop!".to_string() } else { format!("Drop em {}", falta.ceil() as u32) };
                base.px(px(18.0 * s))
                    .gap(px(10.0 * s))
                    .child(icone("icones/bolt.svg", 18.0 * s, dados.tom))
                    .child(div().flex_1().child(texto(15.0 * s).font_weight(FontWeight::SEMIBOLD).child(rotulo)))
                    .child(self.barras(dados, 3.5 * s, 18.0 * s))
                    .into_any_element()
            }
        }
    }

    /// As abas no alto da Ilha aberta, de onde vem o som, a trava e o × que fecha a Ilha. Em pé (a barra lateral,
    /// estreita), as abas são só os ícones.
    pub(super) fn abas(&self, dados: &Dados, s: f32, de_pe: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let mut abas = div().flex().items_center().gap(px(4.0 * s));
        for (i, pagina) in Pagina::TODAS.into_iter().enumerate() {
            let ativa = pagina == self.pagina;
            let com_nome = ativa && !de_pe;
            abas = abas.child(
                div()
                    .id(("ilha-aba", i))
                    .h(px(30.0 * s))
                    .px(px(if com_nome { 12.0 } else { 8.0 } * s))
                    .flex()
                    .items_center()
                    .gap(px(6.0 * s))
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .when(ativa, |d| d.bg(branco(0.14)))
                    .hover(|st| st.bg(branco(0.1)))
                    .child(icone(pagina.icone(), 16.0 * s, branco(if ativa { 1.0 } else { 0.62 })))
                    .when(com_nome, |d| d.child(texto(13.0 * s).font_weight(FontWeight::SEMIBOLD).child(pagina.nome())))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        if !this.acabou_de_arrastar() {
                            this.pagina = pagina;
                            cx.notify();
                        }
                    })),
            );
        }
        let trava = if dados.travada { "icones/lock.svg" } else { "icones/unlock.svg" };
        div()
            .h(px(32.0 * s))
            .flex()
            .items_center()
            .gap(px(8.0 * s))
            .child(abas)
            .child(div().flex_1())
            .when(!de_pe, |d| {
                d.child(
                    texto(pequeno(s)).max_w(px(120.0 * s)).truncate().text_color(cinza()).child(dados.origem.clone()),
                )
            })
            // A trava: destravada (o cadeado aberto, aceso), a Ilha se arrasta para qualquer borda.
            .child(
                self.botao("ilha-trava", trava, 28.0 * s, !dados.travada)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.comandar(Comando::AlternarTravaIlha, cx))),
            )
            .child(self.botao("ilha-fechar", "icones/x.svg", 28.0 * s, false).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    if !this.acabou_de_arrastar() {
                        this.controle.recolher();
                        this.comandar(Comando::AlternarIlha, cx);
                    }
                },
            )))
    }
    /// A página Tocando: a capa, o título, o progresso e os controles. Deitada, a capa ao lado; em pé (a barra
    /// lateral), a capa grande em cima e o resto embaixo.
    pub(super) fn pagina_tocando(&self, dados: &Dados, s: f32, de_pe: bool, cx: &mut Context<Self>) -> AnyElement {
        let (pos, dur) = dados.progresso.unwrap_or((0.0, 0.0));
        let fracao = if dur > 0.0 { (pos / dur).clamp(0.0, 1.0) as f32 } else { 0.0 };
        let controles = div()
            .flex()
            .items_center()
            .gap(px(14.0 * s))
            .child(
                self.botao("ilha-anterior", "icones/prev.svg", 38.0 * s, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.comandar(Comando::Anterior, cx))),
            )
            .child(
                div()
                    .id("ilha-tocar")
                    .size(px(48.0 * s))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .bg(branco(1.0))
                    .cursor_pointer()
                    .hover(|st| st.opacity(0.92))
                    .active(|st| st.opacity(0.7))
                    .child(icone(if dados.tocando { "icones/pause.svg" } else { "icones/play.svg" }, 20.0 * s, preto()))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.comandar(Comando::TocarPausar, cx))),
            )
            .child(
                self.botao("ilha-proxima", "icones/next.svg", 38.0 * s, false)
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.comandar(Comando::Proxima, cx))),
            );
        let titulo = div()
            .flex()
            .flex_col()
            .gap(px(2.0 * s))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0 * s))
                    .child(
                        texto(20.0 * s)
                            .flex_1()
                            .min_w_0()
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .child(dados.titulo.clone()),
                    )
                    .child(self.barras(dados, 3.5 * s, 20.0 * s)),
            )
            .child(texto(14.0 * s).text_color(branco(0.7)).truncate().child(dados.artista.clone()));
        let progresso = div()
            .flex()
            .flex_col()
            .gap(px(6.0 * s))
            .child(
                div()
                    .h(px(5.0 * s))
                    .rounded(px(3.0 * s))
                    .bg(branco(0.18))
                    .child(div().h_full().rounded(px(3.0 * s)).bg(branco(0.95)).w(gpui::relative(fracao))),
            )
            .child(
                div()
                    .flex()
                    .justify_between()
                    .child(texto(pequeno(s)).text_color(branco(0.6)).child(tempo(pos)))
                    .child(texto(pequeno(s)).text_color(branco(0.6)).child(if dur > 0.0 {
                        format!("-{}", tempo((dur - pos).max(0.0)))
                    } else {
                        String::new()
                    })),
            );
        let rodape = div().flex().items_center().justify_between().child(controles).child(
            self.botao("ilha-abrir", "icones/expand.svg", 34.0 * s, false).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    if !this.acabou_de_arrastar() {
                        let _ = this.principal.update(cx, |p, _, _| p.trazer_janela());
                    }
                },
            )),
        );
        let cartao = div().size_full().p(px(14.0 * s)).rounded(px(24.0 * s)).bg(linear_gradient(
            if de_pe { 180.0 } else { 100.0 },
            linear_color_stop(com_alfa(dados.tom, 0.30), 0.0),
            linear_color_stop(com_alfa(dados.tom, 0.04), 1.0),
        ));
        if de_pe {
            return cartao
                .flex()
                .flex_col()
                .gap(px(16.0 * s))
                .child(div().flex().justify_center().pt(px(6.0 * s)).child(Self::capa(dados, 168.0 * s, 22.0 * s)))
                .child(titulo)
                .child(progresso)
                .child(rodape)
                .into_any_element();
        }
        cartao
            .flex()
            .items_center()
            .gap(px(18.0 * s))
            .child(Self::capa(dados, 124.0 * s, 18.0 * s))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .flex()
                    .flex_col()
                    .justify_between()
                    .child(titulo)
                    .child(progresso)
                    .child(rodape),
            )
            .into_any_element()
    }

    /// A letra sincronizada: a linha de agora grande, enchendo como num karaokê, entre a anterior e as próximas. A
    /// troca de linha rola com uma mola, e cada linha cresce e acende ao chegar no meio.
    pub(super) fn pagina_letra(&self, dados: &Dados, s: f32, altura: f32) -> AnyElement {
        let (letra, t) = match &dados.letra {
            Ok((letra, t)) => (letra.clone(), *t),
            Err(frase) => {
                return div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .gap(px(8.0 * s))
                    .child(icone("icones/lyrics.svg", 26.0 * s, cinza()))
                    .child(texto(14.0 * s).text_color(cinza()).child(*frase))
                    .into_any_element();
            }
        };
        let linhas = &letra.linhas;
        let atual = letras::linha_em(linhas, t);
        // Antes da primeira linha, ela espera logo abaixo do meio.
        let alvo = atual.map_or(-0.8, |i| i as f32);
        let chave = ("ilha-letra", 0);
        // Um pulo grande (outra música, a pessoa adiantou a faixa) vai direto, sem rolar a letra inteira.
        if self.anim.mola_atual(chave).is_none_or(|v| (v - alvo).abs() > 6.0) {
            self.anim.definir_mola(chave, alvo);
        }
        let pos = self.anim.mola_amortecida(chave, alvo, 0.5, 0.84);
        // A linha de agora ocupa uma faixa maior que as outras; tamanho, faixa e brilho mudam aos poucos com a
        // distância até o meio, então a linha que chega cresce e acende enquanto rola.
        let (faixa_atual, faixa_vizinha) = (56.0 * s, 36.0 * s);
        let (fonte_atual, fonte_vizinha) = (20.0 * s, 14.5 * s);
        let meio = altura * 0.42;
        let mut caixa = div().relative().size_full().overflow_hidden();
        let primeira = (pos.floor() as i64 - 2).max(0) as usize;
        for k in primeira..(primeira + 6).min(linhas.len()) {
            let d = k as f32 - pos;
            let longe = d.abs();
            let alfa = if longe <= 1.0 { 1.0 - 0.5 * longe } else { 0.5 * (1.0 - (longe - 1.0) / 1.2) };
            if alfa <= 0.01 {
                continue;
            }
            let perto = longe.min(1.0);
            let faixa = faixa_atual + (faixa_vizinha - faixa_atual) * perto;
            let fonte = fonte_atual + (fonte_vizinha - fonte_atual) * perto;
            let y = if longe <= 1.0 {
                d * (faixa_atual + faixa_vizinha) / 2.0
            } else {
                d.signum() * ((faixa_atual + faixa_vizinha) / 2.0 + (longe - 1.0) * faixa_vizinha)
            };
            let (conteudo, cantado) = linha_da_letra(linhas, k, atual, t);
            caixa = caixa.child(
                div()
                    .absolute()
                    .left_0()
                    .w_full()
                    .top(px(meio + y - faixa / 2.0))
                    .h(px(faixa))
                    .flex()
                    .items_center()
                    .child(
                        div()
                            .w_full()
                            .text_size(px(fonte))
                            .line_height(px(fonte * 1.22))
                            .font_weight(FontWeight::BOLD)
                            .line_clamp(2)
                            .child(letra_enchendo(
                                conteudo,
                                cantado,
                                branco(alfa),
                                branco(alfa.min(0.4)),
                                FontWeight::BOLD,
                            )),
                    ),
            );
        }
        div().size_full().px(px(8.0 * s)).child(caixa).into_any_element()
    }

    pub(super) fn pagina_visual(&self, dados: &Dados, s: f32, cx: &mut Context<Self>) -> AnyElement {
        let mut escolhas = div().flex().gap(px(6.0 * s));
        for (i, modelo) in Modelo::TODOS.into_iter().enumerate() {
            let ativo = modelo == dados.modelo;
            escolhas =
                escolhas.child(
                    texto(12.0 * s)
                        .id(("ilha-visual", i))
                        .h(px(28.0 * s))
                        .px(px(12.0 * s))
                        .flex()
                        .items_center()
                        .rounded(px(999.0))
                        .cursor_pointer()
                        .font_weight(FontWeight::SEMIBOLD)
                        .bg(if ativo { branco(1.0) } else { hsla(0.0, 0.0, 0.0, 0.55) })
                        .text_color(if ativo { preto() } else { branco(0.9) })
                        .hover(|st| st.opacity(0.9))
                        .child(modelo.nome())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.comandar(Comando::TrocarVisual(modelo), cx)
                        })),
                );
        }
        #[cfg(windows)]
        let preparando = self.camada.is_none() && !self.camada_falhou;
        #[cfg(not(windows))]
        let preparando = false;
        let aviso = if self.camada_falhou {
            Some("O visual não abriu nesta placa de vídeo.")
        } else if preparando {
            Some("Abrindo o visual…")
        } else {
            None
        };
        let aviso = aviso.map(|frase| texto(12.0 * s).text_color(cinza()).child(frase));
        div()
            .size_full()
            .p(px(10.0 * s))
            .flex()
            .flex_col()
            .justify_end()
            .children(aviso)
            .child(escolhas)
            .into_any_element()
    }

    /// A página Ritmo: o BPM, a energia e a estrutura da faixa. Deitada, lado a lado; em pé, um embaixo do outro.
    pub(super) fn pagina_ritmo(&self, dados: &Dados, s: f32, de_pe: bool) -> AnyElement {
        let bpm = dados.bpm.map_or_else(|| "--".to_string(), |b| b.to_string());
        let mut medidor = div().flex().gap(px(3.0 * s)).items_end().h(px(28.0 * s));
        for i in 0..14 {
            let limiar = i as f32 / 14.0;
            let aceso = dados.energia > limiar;
            medidor = medidor.child(
                div().w(px(6.0 * s)).h(px((8.0 + 20.0 * limiar) * s)).rounded(px(2.0 * s)).bg(if aceso {
                    dados.cor_do_rotulo
                } else {
                    branco(0.12)
                }),
            );
        }
        let (pos, dur) = dados.progresso.unwrap_or((0.0, 0.0));
        let fracao = if dur > 0.0 { (pos / dur).clamp(0.0, 1.0) as f32 } else { 0.0 };
        let mut mapa = div().h(px(34.0 * s)).flex().items_end().gap(px(2.0 * s)).relative();
        let n = dados.mapa.len().max(1);
        for (i, e) in dados.mapa.iter().enumerate() {
            let tocado = (i as f32 + 0.5) / n as f32 <= fracao;
            let altura = if *e < 0.0 { 4.0 } else { 6.0 + 28.0 * e.clamp(0.0, 1.0) };
            mapa = mapa.child(div().flex_1().h(px(altura * s)).rounded(px(2.0 * s)).bg(if *e < 0.0 {
                branco(0.08)
            } else if tocado {
                dados.tom
            } else {
                branco(0.28)
            }));
        }
        let rodape = [
            dados.drop_em_s.filter(|d| *d > 0.0 && *d < 120.0).map(|d| format!("Drop em {}", tempo(f64::from(d)))),
            dados.tom_musical.as_ref().map(ToString::to_string),
        ];
        div()
            .size_full()
            .px(px(8.0 * s))
            .flex()
            .when(de_pe, |d| d.flex_col().py(px(8.0 * s)))
            .gap(px(24.0 * s))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(8.0 * s))
                    .child(
                        div()
                            .flex()
                            .items_end()
                            .gap(px(6.0 * s))
                            .child(numero(52.0 * s).child(bpm))
                            .child(texto(12.0 * s).pb(px(6.0 * s)).text_color(cinza()).child("BPM")),
                    )
                    .child(
                        texto(13.0 * s)
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(dados.cor_do_rotulo)
                            .child(dados.rotulo.nome()),
                    )
                    .child(medidor),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(10.0 * s))
                    .child(texto(12.0 * s).text_color(cinza()).child("Estrutura da faixa"))
                    .child(mapa)
                    .children(rodape.into_iter().flatten().map(|t| texto(13.0 * s).text_color(branco(0.85)).child(t))),
            )
            .into_any_element()
    }
    /// A página Ajustes: blocos que ligam e desligam (brancos quando ligados, como no painel). Deitada, três por
    /// linha; em pé, dois.
    pub(super) fn pagina_ajustes(&self, dados: &Dados, s: f32, de_pe: bool, cx: &mut Context<Self>) -> AnyElement {
        let bloco = |id: &'static str,
                     caminho: &'static str,
                     nome: &'static str,
                     estado: SharedString,
                     ligado: bool| {
            // Ligado, o bloco fica branco com o texto preto, como os botões ligados do painel.
            let (fundo, frente, apagado) = if ligado {
                (branco(1.0), preto(), hsla(0.0, 0.0, 0.0, 0.58))
            } else {
                (branco(0.07), branco(0.95), branco(0.6))
            };
            div()
                .id(id)
                .flex_1()
                .min_w_0()
                .h(px(68.0 * s))
                .p(px(12.0 * s))
                .flex()
                .flex_col()
                .justify_between()
                .rounded(px(18.0 * s))
                .cursor_pointer()
                .bg(fundo)
                .hover(move |st| if ligado { st.opacity(0.9) } else { st.bg(branco(0.12)) })
                .active(|st| st.opacity(0.75))
                .child(icone(caminho, 18.0 * s, frente))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(
                            texto(13.0 * s).font_weight(FontWeight::SEMIBOLD).text_color(frente).truncate().child(nome),
                        )
                        .child(texto(pequeno(s)).text_color(apagado).truncate().child(estado)),
                )
        };
        let liga = |b: bool| -> SharedString { (if b { "Ligado" } else { "Desligado" }).into() };
        let proximo_tamanho = dados.tamanho.proximo();
        let (trava, estado_da_trava): (&'static str, SharedString) = if dados.travada {
            ("icones/lock.svg", "Travada".into())
        } else {
            ("icones/unlock.svg", "Destravada: arraste".into())
        };
        let blocos = [
            bloco(
                "ilha-aj-cores",
                "icones/drop.svg",
                "Cores reativas",
                liga(dados.cores_reativas),
                dados.cores_reativas,
            )
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.comandar(Comando::AlternarCoresReativas, cx))),
            bloco("ilha-aj-seguro", "icones/shield.svg", "Modo seguro", liga(dados.modo_seguro), dados.modo_seguro)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.comandar(Comando::AlternarModoSeguro, cx))),
            bloco("ilha-aj-tema", "icones/tone.svg", "Tema", dados.tema.into(), false)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.comandar(Comando::ProximoTema, cx))),
            bloco("ilha-aj-tamanho", "icones/ilha.svg", "Tamanho da Ilha", dados.tamanho.nome().into(), false)
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    this.comandar(Comando::TrocarTamanhoIlha(proximo_tamanho), cx);
                })),
            bloco("ilha-aj-trava", trava, "Trava", estado_da_trava, dados.travada)
                .on_click(cx.listener(|this, _: &ClickEvent, _, cx| this.comandar(Comando::AlternarTravaIlha, cx))),
            bloco("ilha-aj-config", "icones/sliders.svg", "Configurações", "Abrir no Player".into(), false).on_click(
                cx.listener(|this, _: &ClickEvent, _, cx| {
                    if this.acabou_de_arrastar() {
                        return;
                    }
                    let _ = this.principal.update(cx, |p, window, cx| {
                        p.executar(Comando::AbrirPagina(PaginaDoApp::Configuracoes), window, cx);
                        p.trazer_janela();
                        cx.notify();
                    });
                }),
            ),
        ];
        let por_linha = if de_pe { 2 } else { 3 };
        let mut coluna = div().size_full().flex().flex_col().gap(px(10.0 * s));
        let mut linha = div().flex().gap(px(10.0 * s));
        for (i, b) in blocos.into_iter().enumerate() {
            linha = linha.child(b);
            if (i + 1) % por_linha == 0 {
                coluna = coluna.child(linha);
                linha = div().flex().gap(px(10.0 * s));
            }
        }
        coluna.into_any_element()
    }
}
