//! Texto que termina em "…" quando não cabe.
//!
//! O do GPUI 0.2.2 não serve: o `truncate` mede uma vez e guarda (medido antes de a largura ser conhecida, corta sem
//! as reticências), e o `line_clamp` com `text_ellipsis` guarda o corte da primeira medida e o aplica à segunda,
//! fatiando o texto no meio de uma letra com acento: o app fecha. Este mede do zero a cada vez e só corta na largura
//! final, ao pintar.

use gpui::{
    App, AvailableSpace, Bounds, Element, ElementId, GlobalElementId, InspectorElementId, IntoElement, LayoutId,
    Pixels, SharedString, Style, TextAlign, TextStyle, Window, WrappedLine, point, px, size,
};

pub struct Linhas {
    texto: SharedString,
    quantas: usize,
}

/// Uma linha só.
pub fn linha(texto: impl Into<SharedString>) -> Linhas {
    linhas(texto, 1)
}

/// Até `quantas` linhas.
pub fn linhas(texto: impl Into<SharedString>, quantas: usize) -> Linhas {
    // Uma quebra de linha no texto viraria uma linha a mais.
    let texto: SharedString = texto.into();
    let texto = if texto.contains(['\n', '\r']) { texto.replace(['\n', '\r'], " ").into() } else { texto };
    Linhas { texto, quantas: quantas.max(1) }
}

/// Em quantas linhas o texto fica numa largura.
fn quantas_linhas(texto: &SharedString, estilo: &TextStyle, largura: Pixels, window: &mut Window) -> usize {
    let tamanho = estilo.font_size.to_pixels(window.rem_size());
    let linhas =
        window.text_system().shape_text(texto.clone(), tamanho, &[estilo.to_run(texto.len())], Some(largura), None);
    linhas.map_or(1, |l| l.iter().map(|x| x.wrap_boundaries().len() + 1).sum())
}

/// O maior começo do texto, com "…", que cabe em `quantas` linhas (`cabe` responde se um candidato cabe). Corta no fim de
/// uma letra, sem deixar espaço antes das reticências.
fn cortar(texto: &str, mut cabe: impl FnMut(&str) -> bool) -> String {
    let fins: Vec<usize> = texto.char_indices().map(|(i, c)| i + c.len_utf8()).collect();
    let com = |n: usize| format!("{}…", texto[..if n == 0 { 0 } else { fins[n - 1] }].trim_end());
    let (mut baixo, mut alto) = (0, fins.len());
    while baixo < alto {
        let meio = (baixo + alto).div_ceil(2);
        if cabe(&com(meio)) {
            baixo = meio;
        } else {
            alto = meio - 1;
        }
    }
    com(baixo)
}

/// O texto cortado para caber em `largura` (sem largura, inteiro), já em linhas.
fn moldar(
    texto: &SharedString,
    estilo: &TextStyle,
    quantas: usize,
    largura: Option<Pixels>,
    window: &mut Window,
    cx: &mut App,
) -> Vec<WrappedLine> {
    let tamanho = estilo.font_size.to_pixels(window.rem_size());
    let cortado: SharedString = match largura {
        None => texto.clone(),
        Some(l) if quantas == 1 => {
            let mut partes = vec![estilo.to_run(texto.len())];
            cx.text_system().line_wrapper(estilo.font(), tamanho).truncate_line(texto.clone(), l, "…", &mut partes)
        }
        Some(l) if quantas_linhas(texto, estilo, l, window) <= quantas => texto.clone(),
        Some(l) => cortar(texto, |t| quantas_linhas(&t.to_string().into(), estilo, l, window) <= quantas).into(),
    };
    // Os trechos de fonte são refeitos para o texto que ficou: o corte nunca sobra de outra medida.
    let partes = [estilo.to_run(cortado.len())];
    // Uma linha só não quebra: um pixel de diferença mandaria a última palavra para a linha de baixo, que não aparece.
    let quebra = if quantas > 1 { largura } else { None };
    window
        .text_system()
        .shape_text(cortado, tamanho, &partes, quebra, Some(quantas))
        .map(|l| l.into_iter().collect())
        .unwrap_or_default()
}

impl IntoElement for Linhas {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Linhas {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        _: &mut App,
    ) -> (LayoutId, ()) {
        let (texto, quantas, estilo) = (self.texto.clone(), self.quantas, window.text_style());
        // Encolhe com o pai: o texto nunca empurra o que está ao lado.
        let mut caixa = Style::default();
        caixa.min_size.width = px(0.0).into();
        let id = window.request_measured_layout(caixa, move |conhecida, disponivel, window, cx| {
            let largura = conhecida.width.or(match disponivel.width {
                AvailableSpace::Definite(x) => Some(x),
                _ => None,
            });
            let altura = estilo.line_height_in_pixels(window.rem_size());
            let mut medida = size(px(0.0), px(0.0));
            for l in moldar(&texto, &estilo, quantas, largura, window, cx) {
                let s = l.size(altura);
                medida.width = medida.width.max(s.width).ceil();
                medida.height += s.height;
            }
            medida
        });
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        (): &mut (),
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        area: Bounds<Pixels>,
        (): &mut (),
        (): &mut (),
        window: &mut Window,
        cx: &mut App,
    ) {
        let estilo = window.text_style();
        let altura = estilo.line_height_in_pixels(window.rem_size());
        // Um pixel de folga: a largura da caixa é a do texto arredondada, e não pode cortar o que coube.
        let linhas = moldar(&self.texto, &estilo, self.quantas, Some(area.size.width + px(1.0)), window, cx);
        let mut y = area.origin.y;
        for l in &linhas {
            let sobra = (area.size.width - l.size(altura).width).max(px(0.0));
            let x = match estilo.text_align {
                TextAlign::Left => px(0.0),
                TextAlign::Center => sobra / 2.0,
                TextAlign::Right => sobra,
            };
            let _ = l.paint(point(area.origin.x + x, y), altura, TextAlign::Left, None, window, cx);
            y += l.size(altura).height;
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::cortar;
    use std::path::{Path, PathBuf};

    #[test]
    fn corta_no_fim_de_uma_letra_e_no_maximo_que_cabe() {
        // "Cabe" até 8 letras, contando as reticências.
        let cabe = |t: &str| t.chars().count() <= 8;
        assert_eq!(cortar("coração bobo", cabe), "coração…");
        assert_eq!(cortar("ação é bom", cabe), "ação é…", "sem espaço antes das reticências");
        assert_eq!(cortar("é🎵ç", |t| t.chars().count() <= 2), "é…");
        assert_eq!(cortar("abc", |_| false), "…");
        assert_eq!(cortar("", |_| true), "…");
        // Um título de 200 letras sem espaço corta no meio da palavra, sem travar.
        let enorme = "a".repeat(200);
        assert_eq!(cortar(&enorme, |t| t.chars().count() <= 30), format!("{}…", "a".repeat(29)));
    }

    fn arquivos(pasta: &Path, saida: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(pasta).expect("pasta das telas").flatten() {
            let p = e.path();
            if p.is_dir() {
                arquivos(&p, saida);
            } else if p.extension().is_some_and(|x| x == "rs") {
                saida.push(p);
            }
        }
    }

    /// O corte com "…" do GPUI fecha o app em texto com acento (ver o alto deste arquivo).
    #[test]
    fn ninguem_usa_o_corte_do_gpui() {
        let raiz = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut todos = Vec::new();
        arquivos(&raiz, &mut todos);
        let este = raiz.join("ui").join("linha.rs");
        for arquivo in todos.into_iter().filter(|a| *a != este) {
            let texto = std::fs::read_to_string(&arquivo).expect("ler");
            for proibido in [".text_ellipsis()", ".text_overflow("] {
                assert!(!texto.contains(proibido), "{}: use `linha` ou `linhas`", arquivo.display());
            }
        }
    }
}
