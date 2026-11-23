//! A letra em tela cheia: o visual ocupa a janela inteira, escurecido, e a letra rola grande por cima, a linha de
//! agora enchendo como num karaokê. Abre pelo módulo Letra, pela barra de comandos ou pela tecla Y; Esc fecha.

use super::{icone, letra_enchendo, linha_da_letra, texto};
use crate::app::{PASSO_DA_LETRA, Player};
use crate::letrista::Situacao;
use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, Hsla, SharedString, WindowControlArea, div, hsla,
    linear_color_stop, linear_gradient, prelude::*, px,
};
use nucleo::Comando;

/// Sobre o visual o texto é sempre claro: o fundo dele é escuro em qualquer tema.
fn branco(a: f32) -> Hsla {
    hsla(0.0, 0.0, 1.0, a)
}

impl Player {
    pub(crate) fn ver_letra_cheia(&self, cx: &mut Context<Self>) -> Div {
        let a = self.arranjo.janela;
        let (w, h) = (a.w, a.h);
        let margem = if w >= 1100.0 {
            72.0
        } else if w >= 700.0 {
            40.0
        } else {
            20.0
        };
        let faixa = self.midia.faixa.clone().unwrap_or_default();

        let botao = |id: &'static str, caminho: &'static str, dica: &'static str| {
            div()
                .id(id)
                .size(px(40.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(999.0))
                .cursor_pointer()
                .bg(hsla(0.0, 0.0, 1.0, 0.08))
                .hover(|s| s.bg(hsla(0.0, 0.0, 1.0, 0.18)))
                .child(icone(caminho, 18.0, branco(0.9)))
                .child(self.marca(id))
                .occlude()
                .on_hover(self.ao_pairar(id, None, dica, cx))
        };

        let nome = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .child(
                texto(16.0)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(branco(0.95))
                    .truncate()
                    .child(SharedString::from(faixa.titulo.clone())),
            )
            .child(texto(14.0).text_color(branco(0.6)).truncate().child(SharedString::from(faixa.artista.clone())));
        // O topo arrasta a janela (sem a barra de título do Player); os botões ficam fora do arrasto.
        let topo = div()
            .flex_none()
            .relative()
            .h(px(72.0))
            .px(px(margem))
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(div().absolute().top_0().left_0().size_full().window_control_area(WindowControlArea::Drag))
            .child(nome)
            .child(botao("letra-cheia-buscar", "icones/search.svg", "Buscar outra letra").on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.abrir_busca_de_letra();
                    cx.notify();
                },
            )))
            .child(botao("letra-cheia-fechar", "icones/x.svg", "Fechar a letra (Esc)").on_click(
                cx.listener(|this, _: &ClickEvent, window, cx| this.executar(Comando::LetraTelaCheia, window, cx)),
            ));

        let meio_h = (h - 72.0 - 64.0).max(120.0);
        let (corpo, rodape) = self.corpo_da_letra_cheia(w - 2.0 * margem, meio_h, cx);

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            // O visual continua por baixo, escurecido para a letra ler bem; mais escuro em cima e embaixo, onde
            // ficam o nome da música e os botões.
            .bg(linear_gradient(
                180.0,
                linear_color_stop(hsla(0.0, 0.0, 0.0, 0.78), 0.0),
                linear_color_stop(hsla(0.0, 0.0, 0.0, 0.52), 0.5),
            ))
            .child(topo)
            .child(div().flex_1().min_h(px(0.0)).px(px(margem)).child(corpo))
            .child(
                div()
                    .flex_none()
                    .h(px(64.0))
                    .px(px(margem))
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .bg(hsla(0.0, 0.0, 0.0, 0.35))
                    .children(rodape),
            )
    }

    /// As linhas da letra (ou a frase no lugar dela) e o que vai no rodapé.
    fn corpo_da_letra_cheia(&self, largura: f32, altura: f32, cx: &mut Context<Self>) -> (AnyElement, Vec<AnyElement>) {
        let aviso = |frase: &'static str| {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .child(texto(22.0).text_color(branco(0.7)).child(frase))
                .into_any_element()
        };
        if self.midia.faixa.as_ref().is_none_or(|f| f.titulo.is_empty()) {
            return (aviso("Toque uma música em qualquer app para ver a letra."), Vec::new());
        }
        if !self.modulo_ativo(crate::catalogo::ID_LETRA) {
            return (aviso("Ligue o módulo Letra na Loja para ver a letra aqui."), Vec::new());
        }
        let desta_faixa = self.chave_letra.as_deref() == Some(self.letra.chave.as_str());
        let letra = match (&self.letra.situacao, desta_faixa) {
            (Situacao::Achou(l), true) => l.clone(),
            (Situacao::SemLetra, true) => return (aviso("Não achei a letra desta música."), Vec::new()),
            (Situacao::SemRede, true) => return (aviso("Sem conexão para buscar a letra."), Vec::new()),
            _ => return (aviso("Procurando a letra..."), Vec::new()),
        };
        if letra.linhas.is_empty() {
            return (aviso("Faixa instrumental."), Vec::new());
        }

        let linhas = &letra.linhas;
        let n = linhas.len();
        let tempo = self.tempo_da_letra();
        let atual = match (letra.sincronizada, tempo) {
            (true, Some((t, _))) => letras::linha_em(linhas, t),
            (false, Some((t, d))) => Some((((t / d).clamp(0.0, 1.0) * n as f64) as usize).min(n - 1)),
            (_, None) => None,
        };
        let destaque = letra.sincronizada && tempo.is_some();
        let alvo = atual.map_or(-0.8, |i| i as f32);
        let chave = ("letra-cheia", 0);
        if self.anim.mola_atual(chave).is_none_or(|v| (v - alvo).abs() > 6.0) {
            self.anim.definir_mola(chave, alvo);
        }
        let pos = self.anim.mola_amortecida(chave, alvo, 0.5, 0.86);
        let t = tempo.map_or(0.0, |(t, _)| t);

        // A letra é grande, do tamanho da janela, e todas as linhas têm quase o mesmo tamanho (a de agora acende,
        // as outras apagam): assim a quebra de linha não muda enquanto rola.
        let fonte_atual = (largura / 22.0).clamp(24.0, 52.0);
        let fonte_vizinha = fonte_atual * 0.86;
        let extras = self.letra_tem_extras();
        let banda = |fonte: f32| fonte * 1.18 * 2.0 + if extras { fonte * 0.62 * 1.3 + 6.0 } else { 0.0 } + 20.0;
        let (faixa_atual, faixa_vizinha) = (banda(fonte_atual), banda(fonte_vizinha));
        let meio = altura * 0.4;
        let mut caixa = div().relative().size_full().overflow_hidden();
        let primeira = (pos.floor() as i64 - 4).max(0) as usize;
        for k in primeira..(primeira + 11).min(n) {
            let d = k as f32 - pos;
            let longe = d.abs();
            let alfa = if longe <= 1.0 { 1.0 - 0.6 * longe } else { (0.4 - 0.12 * (longe - 1.0)).max(0.0) };
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
            let principal = div()
                .w_full()
                .text_size(px(fonte))
                .line_height(px(fonte * 1.18))
                .font_weight(FontWeight::BOLD)
                .line_clamp(2);
            let principal = if destaque {
                let (conteudo, cantado) = linha_da_letra(linhas, k, atual, t);
                principal.child(letra_enchendo(
                    conteudo,
                    cantado,
                    branco(alfa),
                    branco(alfa.min(0.34)),
                    FontWeight::BOLD,
                ))
            } else {
                let frase: SharedString =
                    if linhas[k].texto.is_empty() { "· · ·".into() } else { linhas[k].texto.clone().into() };
                principal.text_color(branco(alfa.min(0.8))).child(frase)
            };
            let extra = self.extra_da_linha(k, &linhas[k].texto).map(|e| {
                div()
                    .w_full()
                    .text_size(px(fonte * 0.62))
                    .line_height(px(fonte * 0.62 * 1.3))
                    .line_clamp(1)
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(branco(alfa * 0.62))
                    .child(e)
            });
            caixa = caixa.child(
                div()
                    .absolute()
                    .left_0()
                    .w_full()
                    .top(px(meio + y - faixa / 2.0))
                    .h(px(faixa))
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap(px(6.0))
                    .child(principal)
                    .children(extra),
            );
        }
        let corpo = div().size_full().max_w(px(1100.0)).child(caixa).into_any_element();

        let de_onde = letra.fonte.as_deref().unwrap_or("LRCLIB");
        let palavra = linhas.iter().any(|l| !l.palavras.is_empty());
        let fonte = if !letra.sincronizada {
            format!("Letra: {de_onde}, sem os tempos")
        } else if tempo.is_none() {
            format!("Letra: {de_onde}. Este app não informa a posição da música")
        } else if palavra {
            format!("Letra: {de_onde}, palavra por palavra")
        } else {
            format!("Letra: {de_onde}")
        };
        let mut rodape = vec![
            texto(13.0).flex_1().min_w(px(0.0)).truncate().text_color(branco(0.55)).child(fonte).into_any_element(),
        ];
        if destaque {
            let passo = |id: &'static str, caminho: &'static str, s: f32| {
                div()
                    .id(id)
                    .size(px(32.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .hover(|st| st.bg(hsla(0.0, 0.0, 1.0, 0.14)))
                    .child(icone(caminho, 16.0, branco(0.8)))
                    .child(self.marca(id))
                    .on_hover(self.ao_pairar(
                        id,
                        None,
                        if s < 0.0 { "Atrasar a letra" } else { "Adiantar a letra" },
                        cx,
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.deslocar_letra(s);
                        cx.notify();
                    }))
                    .into_any_element()
            };
            let valor = if self.atraso_letra.abs() < 0.05 {
                "0,0 s".to_string()
            } else {
                format!("{:+.1} s", self.atraso_letra).replace('.', ",")
            };
            rodape.push(passo("letra-cheia-atrasar", "icones/minus.svg", -PASSO_DA_LETRA));
            rodape.push(
                crate::ui::mono()
                    .w(px(52.0))
                    .flex_none()
                    .text_center()
                    .text_color(branco(0.75))
                    .child(valor)
                    .into_any_element(),
            );
            rodape.push(passo("letra-cheia-adiantar", "icones/plus.svg", PASSO_DA_LETRA));
        }
        (corpo, rodape)
    }
}
