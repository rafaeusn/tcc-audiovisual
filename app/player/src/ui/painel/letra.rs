//! O módulo Letra: a letra rolando numa mola, a linha cantada enchendo como num karaokê.

use super::*;

impl Player {
    /// Letra: a linha de agora, a anterior e as duas próximas. Rola sozinha com a música, numa mola; a linha de
    /// agora enche como num karaokê (palavra por palavra quando a letra traz os tempos delas) e as outras apagam
    /// com a distância.
    pub(crate) fn corpo_letra(&self, cx: &mut Context<Self>) -> Div {
        const VISIVEIS: f32 = 4.0;
        let c = self.cores();
        // Altura de cada linha da letra: cabem duas linhas de texto e, com a tradução ou as letras latinas, a de
        // baixo.
        let linha_h: f32 = if self.letra_tem_extras() { 68.0 } else { 48.0 };
        let aviso = |frase: &'static str| div().child(texto(14.0).text_color(c.texto_2).child(frase));
        if self.midia.faixa.as_ref().is_none_or(|f| f.titulo.is_empty()) {
            return aviso("Toque uma música em qualquer app para ver a letra.");
        }
        let desta_faixa = self.chave_letra.as_deref() == Some(self.letra.chave.as_str());
        let letra = match (&self.letra.situacao, desta_faixa) {
            (Situacao::Achou(l), true) => l.clone(),
            (Situacao::SemLetra, true) => {
                return aviso("Não achei a letra desta música.").flex().flex_col().items_start().gap(px(10.0)).child(
                    texto(14.0)
                        .id("letra-buscar")
                        .h(px(36.0))
                        .px(px(14.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(999.0))
                        .border_1()
                        .border_color(c.linha_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/search.svg", 16.0, c.texto))
                        .child("Buscar manualmente")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.abrir_busca_de_letra();
                            cx.notify();
                        })),
                );
            }
            (Situacao::SemRede, true) => return aviso("Sem conexão para buscar a letra. Tento de novo em instantes."),
            _ => return aviso("Procurando a letra..."),
        };
        if letra.linhas.is_empty() {
            return aviso("Faixa instrumental.");
        }

        let n = letra.linhas.len();
        let tempo = self.tempo_da_letra();
        // Com tempos, a linha é a que já começou. Só com texto, a letra rola na proporção da música.
        let atual = match (letra.sincronizada, tempo) {
            (true, Some((t, _))) => letras::linha_em(&letra.linhas, t),
            (false, Some((t, d))) => Some((((t / d).clamp(0.0, 1.0) * n as f64) as usize).min(n - 1)),
            (_, None) => None,
        };
        let destaque = letra.sincronizada && tempo.is_some();
        // A linha de agora fica na segunda posição. Na introdução, a primeira linha já espera ali, sem destaque.
        // Sem a posição da música, a letra fica parada no começo.
        let alvo = match (atual, tempo) {
            (Some(i), _) => i as f32,
            (None, Some(_)) => 0.0,
            (None, None) => 1.0,
        };
        // A letra rola numa mola (passa de leve e assenta). Um pulo grande (a pessoa adiantou a música) vai direto,
        // sem rolar a letra inteira.
        if self.anim.mola_atual(("letra", 0)).is_none_or(|v| (v - alvo).abs() > 6.0) {
            self.anim.definir_mola(("letra", 0), alvo);
        }
        let pos = self.anim.mola_amortecida(("letra", 0), alvo, 0.5, 0.86);
        let t = tempo.map_or(0.0, |(t, _)| t);

        let mut janela = div().relative().w_full().h(px(linha_h * VISIVEIS)).overflow_hidden();
        let primeira = (pos.floor() as i64 - 1).max(0) as usize;
        for k in primeira..(primeira + 7).min(n) {
            let Some(linha) = letra.linhas.get(k) else { break };
            // Quanto mais longe da linha de agora, mais apagada e um pouco menor (muda aos poucos, enquanto rola).
            let longe = (k as f32 - pos).abs();
            let alfa = (1.0 - 0.45 * longe.min(1.0) - 0.25 * (longe - 1.0).clamp(0.0, 1.0)).max(0.0);
            let tamanho = 17.0 - 1.5 * longe.min(1.0);
            let base = div()
                .w_full()
                .text_size(px(tamanho))
                .line_height(px((tamanho * 1.3).ceil()))
                .line_clamp(2)
                .font_weight(FontWeight::SEMIBOLD);
            let conteudo = if destaque {
                let (frase, cantado) = crate::ui::linha_da_letra(&letra.linhas, k, atual, t);
                base.child(crate::ui::letra_enchendo(
                    frase,
                    cantado,
                    com_alfa(c.texto, alfa),
                    com_alfa(c.texto, alfa.min(0.42)),
                    FontWeight::SEMIBOLD,
                ))
            } else {
                let frase: SharedString =
                    if linha.texto.is_empty() { "· · ·".into() } else { linha.texto.clone().into() };
                base.text_color(c.texto_2).child(frase)
            };
            let extra = self.extra_da_linha(k, &linha.texto).map(|e| {
                div()
                    .w_full()
                    .text_size(px(13.0))
                    .line_height(px(17.0))
                    .line_clamp(1)
                    .text_color(com_alfa(c.texto_2, alfa))
                    .child(e)
            });
            janela = janela.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px((k as f32 - pos + 1.0) * linha_h))
                    .w_full()
                    .h(px(linha_h))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(2.0))
                    .child(conteudo)
                    .children(extra),
            );
        }

        let de_onde = letra.fonte.as_deref().unwrap_or("LRCLIB");
        let palavra = letra.linhas.iter().any(|l| !l.palavras.is_empty());
        let fonte = if !letra.sincronizada {
            format!("Letra: {de_onde}, sem os tempos")
        } else if tempo.is_none() {
            format!("Letra: {de_onde}. Este app não informa a posição da música")
        } else if palavra {
            format!("Letra: {de_onde}, palavra por palavra")
        } else {
            format!("Letra: {de_onde}")
        };
        let mut rodape = div()
            .min_h(px(28.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(texto(12.0).flex_1().min_w(px(0.0)).text_color(c.texto_3).child(fonte))
            .child(
                div()
                    .id("letra-cheia")
                    .size(px(28.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone("icones/expand.svg", 16.0, c.texto_2))
                    .child(self.marca("letra-cheia"))
                    .on_hover(self.ao_pairar("letra-cheia", None, "Letra em tela cheia (Y)", cx))
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.executar(Comando::LetraTelaCheia, window, cx)
                    })),
            )
            // A letra é de outra música? Dá para procurar a certa.
            .child(
                div()
                    .id("letra-trocar")
                    .size(px(28.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone("icones/search.svg", 16.0, c.texto_2))
                    .child(self.marca("letra-trocar"))
                    .on_hover(self.ao_pairar("letra-trocar", None, "Buscar outra letra", cx))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.abrir_busca_de_letra();
                        cx.notify();
                    })),
            );
        if destaque {
            // Acerto fino: alguns apps informam a posição com atraso.
            let botao = |id: &'static str, caminho: &'static str, passo: f32| {
                div()
                    .id(id)
                    .size(px(28.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone(caminho, 16.0, c.texto_2))
                    .child(self.marca(id))
                    .on_hover(self.ao_pairar(
                        id,
                        None,
                        if passo < 0.0 { "Atrasar a letra" } else { "Adiantar a letra" },
                        cx,
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.deslocar_letra(passo);
                        cx.notify();
                    }))
            };
            let valor = if self.atraso_letra.abs() < 0.05 {
                "0,0 s".to_string()
            } else {
                format!("{:+.1} s", self.atraso_letra).replace('.', ",")
            };
            rodape = rodape
                .child(botao("letra-atrasar", "icones/minus.svg", -PASSO_DA_LETRA))
                .child(crate::ui::mono().w(px(48.0)).flex_none().text_center().text_color(c.texto_2).child(valor))
                .child(botao("letra-adiantar", "icones/plus.svg", PASSO_DA_LETRA));
        }
        div().flex().flex_col().gap(px(8.0)).child(janela).child(rodape)
    }
}
