//! O módulo Letra: a letra rolando numa mola, a linha cantada enchendo como num karaokê.

use super::*;
use crate::ui::medidas;

/// Quanto da letra aparece e de que tamanho.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TamanhoDaLetra {
    /// A altura das linhas à vista.
    altura: f32,
    /// Quantas linhas ficam acima da cantada.
    acima: f32,
    texto: f32,
    /// A fonte, a tela cheia e o acerto, embaixo.
    rodape: bool,
}

impl TamanhoDaLetra {
    /// No módulo: a linha de agora, a anterior e as duas próximas.
    pub const MODULO: Self = Self { altura: 0.0, acima: 1.0, texto: 17.0, rodape: true };

    /// Enchendo a altura da aba do painel, com a linha cantada a um terço dela. Aba baixa: só a letra, sem o rodapé.
    pub fn da_aba(altura: f32) -> Self {
        let rodape = altura >= 240.0;
        // O rodapé pode ocupar duas linhas.
        let altura = if rodape { altura - 56.0 } else { altura.max(medidas::ALVO) };
        let linha = medidas::DESTAQUE * 2.7;
        Self { altura, acima: (altura / linha * 0.3).floor(), texto: medidas::DESTAQUE, rodape }
    }

    /// A altura de cada linha: cabem duas linhas de texto e, com a tradução ou as letras latinas, a de baixo.
    fn linha(self, com_extras: bool) -> f32 {
        ((self.texto * 1.3).ceil() * 2.0 + if com_extras { 22.0 } else { 2.0 }).ceil()
    }
}

impl Player {
    /// A aba Letra do painel: a letra enchendo a altura. Sem o módulo ligado, o caminho até a Loja.
    pub(super) fn aba_da_letra(&self, altura: f32, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        if !self.modulo_ativo(crate::catalogo::ID_LETRA) {
            return div()
                .flex()
                .flex_col()
                .items_start()
                .gap(px(10.0))
                .child(
                    medidas::apoio().text_color(c.texto_2).child("Ligue o módulo Letra na Loja para ver a letra aqui."),
                )
                .child(
                    medidas::apoio()
                        .id("letra-loja")
                        .h(px(medidas::FICHA))
                        .px(px(14.0))
                        .flex()
                        .items_center()
                        .rounded(px(medidas::PILULA))
                        .border_1()
                        .border_color(c.linha_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child("Ver na Loja")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.abrir_rota(crate::navegacao::Rota::Loja);
                            cx.notify();
                        })),
                );
        }
        self.corpo_letra(TamanhoDaLetra::da_aba(altura), cx)
    }

    /// Letra: a linha de agora, a anterior e as duas próximas. Rola sozinha com a música, numa mola; a linha de
    /// agora enche como num karaokê (palavra por palavra quando a letra traz os tempos delas) e as outras apagam
    /// com a distância.
    pub(crate) fn corpo_letra(&self, tamanho: TamanhoDaLetra, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let linha_h = tamanho.linha(self.letra_tem_extras());
        let altura = if tamanho.altura > 0.0 { tamanho.altura } else { linha_h * 4.0 };
        let visiveis = (altura / linha_h).ceil() as usize;
        let aviso = |frase: &'static str| div().child(medidas::apoio().text_color(c.texto_2).child(frase));
        if self.midia.faixa.as_ref().is_none_or(|f| f.titulo.is_empty()) {
            return aviso("Toque uma música em qualquer app para ver a letra.");
        }
        let desta_faixa = self.chave_letra.as_deref() == Some(self.letra.chave.as_str());
        let letra = match (&self.letra.situacao, desta_faixa) {
            (Situacao::Achou(l), true) => l.clone(),
            (Situacao::SemLetra, true) => {
                return aviso("Não achei a letra desta música.").flex().flex_col().items_start().gap(px(10.0)).child(
                    medidas::apoio()
                        .id("letra-buscar")
                        .h(px(medidas::FICHA))
                        .px(px(14.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(medidas::PILULA))
                        .border_1()
                        .border_color(c.linha_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/search.svg", medidas::ICONE_LINHA, c.texto))
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

        let mut janela = div().relative().w_full().h(px(altura)).overflow_hidden();
        let primeira = (pos.floor() as i64 - tamanho.acima as i64).max(0) as usize;
        for k in primeira..(primeira + visiveis + 3).min(n) {
            let Some(linha) = letra.linhas.get(k) else { break };
            // Quanto mais longe da linha de agora, mais apagada e um pouco menor (muda aos poucos, enquanto rola).
            let longe = (k as f32 - pos).abs();
            let alfa = (1.0 - 0.45 * longe.min(1.0) - 0.25 * (longe - 1.0).clamp(0.0, 1.0)).max(0.0);
            let corpo = tamanho.texto - 1.5 * longe.min(1.0);
            let base = div()
                .w_full()
                .text_size(px(corpo))
                .line_height(px((corpo * 1.3).ceil()))
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
                    .text_size(px(medidas::APOIO))
                    .line_height(px(17.0))
                    .line_clamp(1)
                    .text_color(com_alfa(c.texto_2, alfa))
                    .child(e)
            });
            janela = janela.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px((k as f32 - pos + tamanho.acima) * linha_h))
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
            .child(medidas::apoio().flex_1().min_w(px(0.0)).text_color(c.texto_3).child(fonte))
            .child(
                div()
                    .id("letra-cheia")
                    .size(px(28.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(medidas::PILULA))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone("icones/expand.svg", medidas::ICONE_LINHA, c.texto_2))
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
                    .rounded(px(medidas::PILULA))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone("icones/search.svg", medidas::ICONE_LINHA, c.texto_2))
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
                    .rounded(px(medidas::PILULA))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone(caminho, medidas::ICONE_LINHA, c.texto_2))
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
        div().flex().flex_col().gap(px(8.0)).child(janela).when(tamanho.rodape, |d| d.child(rodape))
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_letra_da_aba_cabe_nela() {
        for altura in [60.0, 134.0, 239.0, 240.0, 500.0, 900.0] {
            let t = TamanhoDaLetra::da_aba(altura);
            let total = t.altura + if t.rodape { 56.0 } else { 0.0 };
            assert!(total <= altura + 0.01, "{altura}: {total}");
            assert!(
                (t.acima + 1.0) * t.linha(false) <= t.altura.max(t.linha(false)) + 0.01,
                "{altura}: a cantada fica à vista"
            );
        }
        assert!(!TamanhoDaLetra::da_aba(134.0).rodape && TamanhoDaLetra::da_aba(400.0).rodape);
    }
}
