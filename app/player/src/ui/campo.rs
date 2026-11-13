//! Os campos de texto do app (a busca de músicas e a barra de comandos).
//!
//! As letras chegam pelo caminho de texto do sistema, não pelas teclas: assim funcionam os acentos com tecla
//! morta (´ + a = á), o teclado de outras línguas e o colar. As teclas de controle (Enter, Esc, setas, apagar)
//! continuam com quem cuida de cada campo. Só existe um campo ativo por vez, e o cursor fica sempre no fim.

use super::texto;
use crate::app::Player;
use crate::tema::Cores;
use gpui::{
    App, Bounds, Context, Div, ElementInputHandler, EntityInputHandler, FontWeight, Pixels, Point, SharedString,
    UTF16Selection, Window, canvas, div, prelude::*, px,
};
use nucleo::Pagina;
use std::ops::Range;

/// Quantas letras um campo aceita.
pub const MAXIMO_DE_LETRAS: usize = 64;

/// Posição em UTF-16 (como o sistema conta) para posição em bytes.
fn para_bytes(s: &str, utf16: usize) -> usize {
    let mut contadas = 0;
    for (i, c) in s.char_indices() {
        if contadas >= utf16 {
            return i;
        }
        contadas += c.len_utf16();
    }
    s.len()
}

fn para_utf16(s: &str, bytes: usize) -> usize {
    s[..bytes.min(s.len())].encode_utf16().count()
}

fn faixa_em_bytes(s: &str, utf16: &Range<usize>) -> Range<usize> {
    let inicio = para_bytes(s, utf16.start);
    inicio..para_bytes(s, utf16.end).max(inicio)
}

/// Troca `faixa` de `s` por `novo`, sem passar de [`MAXIMO_DE_LETRAS`] e sem caracteres de controle. Devolve
/// onde o texto novo ficou (em bytes).
pub fn trocar(s: &mut String, faixa: Range<usize>, novo: &str) -> Range<usize> {
    let faixa = faixa.start.min(s.len())..faixa.end.min(s.len());
    let sobra = MAXIMO_DE_LETRAS.saturating_sub(s.chars().count() - s[faixa.clone()].chars().count());
    let limpo: String = novo.chars().filter(|c| !c.is_control()).take(sobra).collect();
    let inicio = faixa.start;
    s.replace_range(faixa, &limpo);
    inicio..inicio + limpo.len()
}

impl Player {
    /// O texto do campo que está recebendo o teclado agora.
    fn campo_ativo(&mut self) -> Option<&mut String> {
        if let Some(p) = self.paleta.as_mut() {
            return Some(&mut p.consulta);
        }
        if self.buscando_musicas && self.pagina == Pagina::Biblioteca {
            return Some(&mut self.busca_musicas);
        }
        None
    }

    /// O campo ativo mudou de texto: cada campo reage do seu jeito.
    fn campo_mudou(&mut self) {
        if let Some(p) = self.paleta.as_mut() {
            p.selecionado = 0;
            if !p.letra {
                self.rolagem_paleta.scroll_to_item(0);
            }
        } else if self.buscando_musicas {
            self.local.buscar(&self.busca_musicas);
        }
    }

    /// Põe `texto` no fim do campo ativo (o colar usa isto).
    pub(crate) fn escrever_no_campo(&mut self, texto: &str) {
        self.marcado = None;
        let Some(s) = self.campo_ativo() else { return };
        let fim = s.len();
        // Colar texto com quebras de linha: vira uma linha só.
        let texto = texto.replace(['\r', '\n', '\t'], " ");
        trocar(s, fim..fim, &texto);
        self.campo_mudou();
    }

    /// Apaga a última letra (ou tudo, com Ctrl).
    pub(crate) fn apagar_no_campo(&mut self, tudo: bool) {
        self.marcado = None;
        let Some(s) = self.campo_ativo() else { return };
        if tudo {
            s.clear();
        } else {
            s.pop();
        }
        self.campo_mudou();
    }

    /// Cola o que está na área de transferência.
    pub(crate) fn colar_no_campo(&mut self, cx: &mut Context<Self>) {
        if let Some(texto) = cx.read_from_clipboard().and_then(|c| c.text()) {
            self.escrever_no_campo(&texto);
        }
    }

    /// Um elemento invisível que liga o caminho de texto do sistema ao campo. Vai dentro do campo (a janela de
    /// sugestões do teclado aparece ali).
    pub(crate) fn receber_texto(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let entidade = cx.entity();
        let foco = self.foco.clone();
        canvas(
            |_, _, _| (),
            move |area, (), window: &mut Window, cx: &mut App| {
                window.handle_input(&foco, ElementInputHandler::new(area, entidade.clone()), cx);
            },
        )
        .absolute()
        .left(px(0.0))
        .top(px(0.0))
        .size_full()
    }

    /// O texto do campo com o cursor colado nele, ou a frase de exemplo quando vazio. Texto mais largo que o
    /// campo some pela esquerda: o fim, onde se escreve, fica sempre à vista.
    pub(crate) fn conteudo_do_campo(c: &Cores, valor: &str, exemplo: &'static str, tamanho: f32, ativo: bool) -> Div {
        let cursor = div().w(px(2.0)).h(px(tamanho + 4.0)).flex_none().bg(c.texto);
        let corpo = div().flex_1().min_w(px(0.0)).h_full().flex().items_center().overflow_hidden();
        if valor.is_empty() {
            return corpo.when(ativo, |d| d.child(cursor)).child(
                texto(tamanho)
                    .pl(px(if ativo { 4.0 } else { 0.0 }))
                    .text_color(c.texto_3)
                    .font_weight(FontWeight::MEDIUM)
                    .truncate()
                    .child(exemplo),
            );
        }
        corpo
            .justify_end()
            .child(
                texto(tamanho)
                    .flex_none()
                    .font_weight(FontWeight::MEDIUM)
                    .whitespace_nowrap()
                    .child(SharedString::from(valor.to_string())),
            )
            .when(ativo, |d| d.child(cursor.ml(px(1.0))))
            // Ocupa o resto: com texto curto, ele fica à esquerda; com texto longo, some e o fim aparece.
            .child(div().flex_1())
    }
}

impl EntityInputHandler for Player {
    fn text_for_range(
        &mut self,
        faixa: Range<usize>,
        ajustada: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let s = self.campo_ativo()?;
        let bytes = faixa_em_bytes(s, &faixa);
        *ajustada = Some(para_utf16(s, bytes.start)..para_utf16(s, bytes.end));
        Some(s[bytes].to_string())
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        let s = self.campo_ativo()?;
        let fim = s.encode_utf16().count();
        Some(UTF16Selection { range: fim..fim, reversed: false })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        let marcado = self.marcado.clone()?;
        let s = if let Some(p) = self.paleta.as_ref() { &p.consulta } else { &self.busca_musicas };
        Some(para_utf16(s, marcado.start)..para_utf16(s, marcado.end))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marcado = None;
    }

    fn replace_text_in_range(
        &mut self,
        faixa: Option<Range<usize>>,
        novo: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let marcado = self.marcado.take();
        let Some(s) = self.campo_ativo() else { return };
        // Sem faixa: troca o que estava marcado (a tecla morta esperando a letra) ou escreve no fim.
        let alvo = match (faixa, marcado) {
            (Some(f), _) => faixa_em_bytes(s, &f),
            (None, Some(m)) => m,
            (None, None) => s.len()..s.len(),
        };
        trocar(s, alvo, novo);
        self.campo_mudou();
        cx.notify();
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        faixa: Option<Range<usize>>,
        novo: &str,
        _: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let marcado = self.marcado.take();
        let Some(s) = self.campo_ativo() else { return };
        let alvo = match (faixa, marcado) {
            (Some(f), _) => faixa_em_bytes(s, &f),
            (None, Some(m)) => m,
            (None, None) => s.len()..s.len(),
        };
        let ficou = trocar(s, alvo, novo);
        self.marcado = (!ficou.is_empty()).then_some(ficou);
        cx.notify();
    }

    fn bounds_for_range(
        &mut self,
        _: Range<usize>,
        area: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        Some(area)
    }

    fn character_index_for_point(&mut self, _: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        None
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn utf16_e_bytes_com_acento_e_emoji() {
        let s = "aé🎵b";
        assert_eq!(para_bytes(s, 0), 0);
        assert_eq!(para_bytes(s, 2), 3);
        assert_eq!(para_bytes(s, 4), 7, "o emoji conta dois em UTF-16");
        assert_eq!(para_bytes(s, 99), s.len());
        assert_eq!(para_utf16(s, 7), 4);
        assert_eq!(faixa_em_bytes(s, &(1..2)), 1..3);
    }

    #[test]
    fn tecla_morta_vira_a_letra_com_acento() {
        // ´ fica marcado esperando; a letra seguinte troca o marcado por "á".
        let mut s = String::from("caf");
        let marcado = trocar(&mut s, 3..3, "´");
        assert_eq!(s, "caf´");
        trocar(&mut s, marcado, "é");
        assert_eq!(s, "café");
    }

    #[test]
    fn nao_passa_do_limite_nem_aceita_controle() {
        let mut s = "a".repeat(MAXIMO_DE_LETRAS - 1);
        let fim = s.len();
        trocar(&mut s, fim..fim, "xyz");
        assert_eq!(s.chars().count(), MAXIMO_DE_LETRAS);
        assert!(s.ends_with('x'));
        let mut t = String::new();
        trocar(&mut t, 0..0, "a\u{8}b\r");
        assert_eq!(t, "ab");
        // Trocar dentro do limite ainda funciona com o campo cheio.
        let fim = s.len();
        trocar(&mut s, fim - 1..fim, "ç");
        assert!(s.ends_with('ç'));
    }
}
