//! O corpo de cada módulo do painel: Ritmo, Energia, Estrutura da faixa, Nesta sessão, Visual e Relógio.

use super::*;

impl Player {
    /// Dígitos do BPM em matriz de pontos. Cada dígito rola até o valor novo em 240 ms (o BPM muda pouco).
    pub(super) fn digitos_do_bpm(&self, bpm: u32) -> Div {
        const ALTURA: f32 = 44.0;
        const LARGURA: f32 = 29.0;
        let digitos: Vec<u32> = bpm.to_string().chars().filter_map(|d| d.to_digit(10)).collect();
        let n = digitos.len();
        let mut linha = div().flex().h(px(ALTURA));
        for (i, d) in digitos.into_iter().enumerate() {
            // A chave conta da direita: a unidade continua sendo a unidade quando o número ganha uma casa.
            let v = self.anim.seguir(("bpm", (n - 1 - i) as u32), d as f32, 0.24).clamp(0.0, 9.0);
            let (base, resto) = (v.floor(), v.fract());
            let algarismo = |valor: f32, topo: f32| {
                numero(ALTURA)
                    .absolute()
                    .left(px(0.0))
                    .top(px(topo))
                    .w_full()
                    .text_center()
                    .child(format!("{}", valor as u32 % 10))
            };
            linha = linha.child(
                div()
                    .relative()
                    .w(px(LARGURA))
                    .h(px(ALTURA))
                    .overflow_hidden()
                    .child(algarismo(base, -resto * ALTURA))
                    .when(resto > 0.001, |s| s.child(algarismo(base + 1.0, (1.0 - resto) * ALTURA))),
            );
        }
        linha
    }

    pub(super) fn corpo_ritmo(&self) -> Div {
        let c = self.cores();
        let r = &self.retrato;
        let bpm = r.bpm_exibido();
        let tem_pulso = r.gate > 0.5 && r.conf_batida > 0.15 && bpm >= 1.0;
        if !tem_pulso {
            let frase = if r.gate > 0.5 { "Sem pulso firme neste trecho." } else { "Sem som agora." };
            return div().child(texto(14.0).text_color(c.texto_2).child(frase));
        }
        let cor = tema::cor_da_energia(&c, r.energia);
        let mut batidas = div().h(px(14.0)).flex().items_center().gap(px(10.0));
        for i in 0..4 {
            let acesa = i == self.batida;
            let ponto =
                div().size(px(if acesa { 12.0 } else { 10.0 })).rounded(px(999.0)).bg(if acesa { cor } else { c.sel });
            // O primeiro tempo do compasso tem um anel em volta.
            batidas = batidas.child(
                div()
                    .size(px(14.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .when(i == 0, |d| d.border_1().border_color(c.texto_3))
                    .child(ponto.when(i == 0, |d| d.size(px(if acesa { 10.0 } else { 8.0 })))),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div().flex().items_end().gap(px(6.0)).child(self.digitos_do_bpm(bpm.round() as u32)).child(
                    texto(12.0).pb(px(6.0)).text_color(c.texto_2).font_weight(FontWeight::SEMIBOLD).child("BPM"),
                ),
            )
            .child(batidas)
            .children(self.tom_da_musica().map(|tom| texto(12.0).text_color(c.texto_2).child(tom)))
    }

    /// O tom da música local pré-analisada ("Tom: Lá menor · 8A"), quando a análise tem confiança.
    pub(crate) fn tom_da_musica(&self) -> Option<SharedString> {
        if !self.origem_local {
            return None;
        }
        let a = self.local.analise_atual()?;
        let tom = a.tom.as_ref().filter(|t| t.confianca >= 0.15)?;
        Some(format!("Tom: {} · {}", tom.nome, tom.camelot).into())
    }

    pub(super) fn corpo_energia(&self) -> (Option<AnyElement>, Div) {
        let c = self.cores();
        let r = &self.retrato;
        let com_som = r.gate > 0.5;
        let energia = if com_som { r.energia } else { 0.0 };
        let palavra = if com_som { r.rotulo.nome() } else { "Silêncio" };
        // O número não anima (muda o tempo todo); só a cor da palavra troca, em 300 ms.
        let cor_palavra = if com_som {
            tema::cor_do_nivel(&c, self.anim.seguir(("nivel", 0), nivel(r.rotulo), 0.3))
        } else {
            c.texto_2
        };

        let mut grade = div().flex().items_end().gap(px(3.0));
        for i in 0..COLUNAS_ENERGIA {
            let e = self.energia_recente.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            let acesos = (e * PONTOS as f32).round() as usize;
            let cor = tema::cor_da_energia(&c, e);
            let mut coluna = div().flex_1().min_w(px(0.0)).flex().flex_col().gap(px(3.0));
            for p in (0..PONTOS).rev() {
                coluna =
                    coluna.child(div().w_full().h(px(6.0)).rounded(px(2.0)).bg(if p < acesos { cor } else { c.ponto }));
            }
            grade = grade.child(coluna);
        }
        // Ponto que pulsa uma vez por compasso, ao lado do número.
        let ponto = 8.0 * (1.0 + 0.6 * self.pulso_compasso);
        let direita = com_som.then(|| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .child(crate::ui::mono().text_color(c.texto_2).child(decimal(energia)))
                .child(
                    div()
                        .size(px(14.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().size(px(ponto)).rounded(px(999.0)).bg(cor_palavra)),
                )
                .into_any_element()
        });
        let corpo = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(titulo(24.0).text_color(cor_palavra).child(palavra))
            .child(grade);
        (direita, corpo)
    }

    /// Estrutura da faixa: o nível de energia de agora, desde quando, e a barra com os trechos já ouvidos.
    /// A estrutura da música local pré-analisada: a música inteira, com o que ainda vai vir.
    pub(super) fn corpo_estrutura_pre(&self, largura: f32, a: &preanalise::Analise) -> (Option<AnyElement>, Div) {
        let c = self.cores();
        let t = self.local.situacao.posicao_s;
        let total = a.duracao_s.max(1.0);
        let escala = f64::from(largura) / total;
        let mut barra = div().w_full().h(px(8.0)).flex().gap(px(3.0));
        for (i, s) in a.trechos.iter().enumerate() {
            let fim = a.trechos.get(i + 1).map_or(total, |p| p.inicio_s);
            let w = (((fim - s.inicio_s).max(0.0) * escala) as f32 - 3.0).max(4.0);
            let cor = tema::cor_do_nivel(&c, nivel(s.rotulo()));
            // O que já tocou fica mais apagado; o de agora, cheio; o que vem, só um traço.
            let alfa = if fim <= t {
                0.45
            } else if s.inicio_s <= t {
                1.0
            } else {
                0.22
            };
            barra = barra.child(div().w(px(w)).h(px(8.0)).flex_none().rounded(px(999.0)).bg(com_alfa(cor, alfa)));
        }
        // As marcas dos drops, por cima da barra.
        let mut marcas = div().absolute().left(px(0.0)).top(px(-3.0)).w_full().h(px(14.0));
        for d in a.drops.iter().map(|d| d.t) {
            let x = (d * escala) as f32;
            marcas = marcas.child(
                div()
                    .absolute()
                    .left(px((x - 1.0).max(0.0)))
                    .top(px(0.0))
                    .w(px(2.0))
                    .h(px(14.0))
                    .rounded(px(1.0))
                    .bg(if d <= t { com_alfa(c.texto, 0.4) } else { c.texto }),
            );
        }
        let atual = a.trecho_em(t).map_or(Rotulo::Calma, preanalise::Trecho::rotulo);
        let desde = a.trecho_em(t).map_or(0.0, |s| s.inicio_s);
        let proximo: SharedString = match (a.proximo_drop(t), a.proximo_trecho(t)) {
            (Some(d), _) if d - t <= 60.0 => format!("Drop em {}", tempo(d - t)).into(),
            (_, Some(s)) => format!("Depois: {} em {}", s.rotulo().nome(), tempo(s.inicio_s - t)).into(),
            (Some(d), None) => format!("Drop em {}", tempo(d - t)).into(),
            (None, None) => "Até o fim neste ritmo".into(),
        };
        let corpo = div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap(px(12.0))
                    .child(texto(16.0).font_weight(FontWeight::SEMIBOLD).child(atual.nome()))
                    .child(texto(12.0).flex_none().text_color(c.texto_2).child(format!("desde {}", tempo(desde)))),
            )
            .child(div().relative().child(barra.overflow_hidden()).child(marcas))
            .child(texto(12.0).text_color(c.texto_2).child(proximo));
        let direita = texto(12.0)
            .flex_none()
            .text_color(c.texto_2)
            .font_weight(FontWeight::MEDIUM)
            .child("Pré-analisada")
            .into_any_element();
        (Some(direita), corpo)
    }

    pub(super) fn corpo_estrutura(&self, largura: f32) -> (Option<AnyElement>, Div) {
        if self.origem_local
            && let Some(a) = self.local.analise_atual()
        {
            return self.corpo_estrutura_pre(largura, &a);
        }
        let c = self.cores();
        let agora = self.tempo_da_faixa();
        let Some(atual) = self.secoes.last().filter(|_| self.retrato.gate > 0.5 || self.midia.tocando) else {
            return (
                None,
                div().child(
                    texto(14.0).text_color(c.texto_2).child("Os trechos da música aparecem aqui enquanto ela toca."),
                ),
            );
        };
        let duracao = self.midia.posicao.filter(|p| p.duracao_s > 1.0).map(|p| p.duracao_s);
        let total = duracao.unwrap_or(agora).max(agora).max(1.0);
        let inicio_da_barra = self.secoes.first().map_or(0.0, |s| s.inicio_s.min(agora));
        let escala = f64::from(largura) / (total - inicio_da_barra).max(1.0);

        let mut barra = div().w_full().h(px(8.0)).flex().gap(px(3.0));
        for (i, s) in self.secoes.iter().enumerate() {
            let fim = self.secoes.get(i + 1).map_or(agora, |p| p.inicio_s);
            let w = (((fim - s.inicio_s).max(0.0) * escala) as f32 - 3.0).max(4.0);
            let cor = tema::cor_do_nivel(&c, nivel(s.rotulo));
            let ultimo = i + 1 == self.secoes.len();
            barra = barra.child(
                div()
                    .w(px(w))
                    .h(px(8.0))
                    .flex_none()
                    .rounded(px(999.0))
                    .bg(com_alfa(cor, if ultimo { 1.0 } else { 0.5 })),
            );
        }
        // O que ainda não tocou (só dá para saber o tamanho quando o app informa a duração).
        if duracao.is_some_and(|d| d - agora > 1.0) {
            barra = barra.child(div().flex_1().min_w(px(4.0)).h(px(8.0)).rounded(px(999.0)).bg(c.ponto));
        }
        let corpo = div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap(px(12.0))
                    .child(texto(16.0).font_weight(FontWeight::SEMIBOLD).child(atual.rotulo.nome()))
                    .child(
                        texto(12.0).flex_none().text_color(c.texto_2).child(format!("desde {}", tempo(atual.inicio_s))),
                    ),
            )
            .child(barra.overflow_hidden());
        let direita = texto(12.0)
            .flex_none()
            .text_color(c.texto_2)
            .font_weight(FontWeight::MEDIUM)
            .child("Ao vivo")
            .into_any_element();
        (Some(direita), corpo)
    }

    /// Três barrinhas na faixa que está tocando. Seguem o som de verdade e descem no pause.
    pub(super) fn equalizador(&self, tocando: bool) -> Div {
        let c = self.cores();
        let r = &self.retrato;
        let niveis = [r.energia, r.pulso.max(r.loud * 0.6), r.atividade];
        let mut barras = div().w(px(14.0)).h(px(14.0)).flex().flex_none().items_end().justify_end().gap(px(2.0));
        for (i, v) in niveis.into_iter().enumerate() {
            let alvo = if tocando { 3.0 + 11.0 * v.clamp(0.0, 1.0) } else { 3.0 };
            let h = self.anim.seguir(("equalizador", i as u32), alvo, if tocando { 0.08 } else { 0.22 });
            barras = barras.child(div().w(px(3.0)).h(px(h)).rounded(px(2.0)).bg(tema::cor_da_energia(&c, r.energia)));
        }
        barras
    }

    pub(super) fn corpo_sessao(&self) -> (Option<AnyElement>, Div) {
        let c = self.cores();
        let total = self.historico.len();
        let cinza = self.ajustes.tema == Tema::Mono;
        let mut lista = div().flex().flex_col();
        for (i, o) in self.historico.iter().take(VISIVEIS_NA_SESSAO).enumerate() {
            let esta = i == 0 && self.midia.faixa.as_ref().is_some_and(|f| f.chave() == o.faixa.chave());
            let capa = match &o.capa {
                Some(imagem) => img(imagem.clone())
                    .size(px(44.0))
                    .flex_none()
                    .rounded(px(RAIO_CONTROLE))
                    .grayscale(cinza)
                    .into_any_element(),
                None => div()
                    .size(px(44.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(RAIO_CONTROLE))
                    .bg(c.bloco)
                    .child(icone("icones/wave.svg", 20.0, c.texto_3))
                    .into_any_element(),
            };
            let artista: SharedString =
                if o.faixa.artista.is_empty() { o.faixa.app.clone().into() } else { o.faixa.artista.clone().into() };
            let direita = if esta {
                self.equalizador(self.midia.tocando && self.retrato.gate > 0.5).into_any_element()
            } else {
                texto(12.0)
                    .flex_none()
                    .text_color(c.texto_2)
                    .font_weight(FontWeight::MEDIUM)
                    .child(ha_quanto(o.quando.elapsed()))
                    .into_any_element()
            };
            lista = lista.child(
                div()
                    .min_h(px(56.0))
                    .py(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(capa)
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
                                    .child(SharedString::from(o.faixa.titulo.clone())),
                            )
                            .child(texto(14.0).text_color(c.texto_2).truncate().child(artista)),
                    )
                    .child(direita),
            );
        }
        if total == 0 {
            lista = lista
                .child(texto(14.0).py(px(8.0)).text_color(c.texto_2).child("As músicas desta sessão aparecem aqui."));
        }
        let direita = (total > 0).then(|| {
            texto(12.0)
                .flex_none()
                .text_color(c.texto_2)
                .font_weight(FontWeight::MEDIUM)
                .child(total.to_string())
                .into_any_element()
        });
        (direita, lista)
    }

    pub(super) fn corpo_visual(&self, cx: &mut Context<Self>) -> (Option<AnyElement>, Div) {
        let c = self.cores();
        let botao = |id: &'static str, caminho: &'static str, rotulo: &'static str, ligado: bool| {
            let cor = if ligado { c.inv_texto } else { c.texto_2 };
            div()
                .id(id)
                .flex_1()
                .min_w(px(0.0))
                .h(px(44.0))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .cursor_pointer()
                .when(ligado, |d| d.bg(c.inv))
                .when(!ligado, |d| d.border_1().border_color(c.linha_2).hover(|s| s.bg(c.hover)))
                .child(icone(caminho, 18.0, cor))
                .child(texto(14.0).text_color(cor).font_weight(FontWeight::SEMIBOLD).truncate().child(rotulo))
        };
        let corpo = div()
            .flex()
            .gap(px(8.0))
            .child(
                botao("rapido-cores", "icones/drop.svg", "Cores reativas", self.ajustes.cores_reativas)
                    .on_click(self.ao_clicar(Comando::AlternarCoresReativas, cx)),
            )
            .child(
                botao("rapido-seguro", "icones/shield.svg", "Modo seguro", self.ajustes.modo_seguro)
                    .on_click(self.ao_clicar(Comando::AlternarModoSeguro, cx)),
            );
        let direita = texto(12.0)
            .flex_none()
            .text_color(c.texto_2)
            .font_weight(FontWeight::MEDIUM)
            .child(self.ajustes.modelo.nome())
            .into_any_element();
        (Some(direita), corpo)
    }

    pub(super) fn corpo_relogio(&self) -> Div {
        let c = self.cores();
        let (h, m) = self.hora;
        div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(numero(44.0).child(format!("{h:02}")))
            .child(numero(44.0).text_color(c.texto_3).child(":"))
            .child(numero(44.0).child(format!("{m:02}")))
    }
}
