//! Todas as medidas das telas: os papéis de texto, os ícones, os raios e as alturas. Nenhuma tela escolhe número por
//! conta própria (o teste `sem_numero_solto` confere); a Ilha tem a escala dela.

use super::{texto, titulo};
use gpui::{Div, Pixels};
use std::sync::atomic::{AtomicU32, Ordering};

/// A escala da interface da janela que está desenhando (1 = 100%). Cada janela a define no começo do desenho: o GPUI
/// desenha uma janela inteira de cada vez.
static ESCALA: AtomicU32 = AtomicU32::new(0x3F80_0000);

pub fn definir_escala(escala: f32) {
    let e = if escala.is_finite() { escala.clamp(0.5, 2.0) } else { 1.0 };
    ESCALA.store(e.to_bits(), Ordering::Relaxed);
}

pub fn escala() -> f32 {
    f32::from_bits(ESCALA.load(Ordering::Relaxed))
}

/// Um tamanho das telas na escala da interface. Em 100% passa como está; nas outras, de 2 px para cima vira pixel
/// inteiro (meio pixel treme quando o pai anda, e borra o texto).
pub fn escalar(v: f32, escala: f32) -> f32 {
    if escala == 1.0 {
        return v;
    }
    let s = v * escala;
    if s.abs() >= 2.0 { s.round() } else { s }
}

/// O `px` das telas: o número na escala da interface.
pub fn px(v: f32) -> Pixels {
    gpui::px(escalar(v, escala()))
}

/// Um valor em pixels de verdade (do mouse, de uma área medida) na medida das telas e do arranjo.
pub fn logico(v: Pixels) -> f32 {
    f32::from(v) / escala()
}

/// A escala que vale nesta janela: a pedida, menos quando a janela não teria o tamanho mínimo do arranjo (320 × 400).
pub fn escala_que_cabe(tamanho: u16, largura: f32, altura: f32) -> f32 {
    let pedida = f32::from(tamanho) / 100.0;
    if !(largura.is_finite() && altura.is_finite()) {
        return pedida;
    }
    pedida.min(largura / 320.0).min(altura / 400.0).max(0.5)
}

/// Teclas, tempos e o nome embaixo do ícone da navegação. Nunca texto de ler.
pub const MICRO: f32 = 12.0;
/// Artista, dados, fichas, avisos.
pub const APOIO: f32 = 14.0;
/// Nomes, navegação, botões.
pub const CORPO: f32 = 16.0;
/// O artista sobre o visual, o título de uma seção das Configurações.
pub const DESTAQUE: f32 = 20.0;
/// Prateleiras, janelas.
pub const SECAO: f32 = 28.0;
/// O título da página.
pub const PAGINA: f32 = 44.0;
/// O título sobre o visual e o nome do artista.
pub const CARTAZ: f32 = 72.0;

pub fn micro() -> Div {
    texto(MICRO)
}

pub fn apoio() -> Div {
    texto(APOIO)
}

pub fn corpo() -> Div {
    texto(CORPO)
}

pub fn destaque() -> Div {
    texto(DESTAQUE)
}

pub fn secao() -> Div {
    titulo(SECAO)
}

/// O título da página; em janela estreita, do tamanho de uma seção.
pub fn pagina(largo: bool) -> Div {
    titulo(if largo { PAGINA } else { SECAO })
}

/// Dentro de ficha ou de botão pequeno.
pub const ICONE_MIUDO: f32 = 16.0;
/// Ao lado de um texto.
pub const ICONE_LINHA: f32 = 20.0;
/// Botões e navegação.
pub const ICONE: f32 = 24.0;
/// O tocar, e o desenho de uma página vazia.
pub const ICONE_GRANDE: f32 = 28.0;

/// Cartões, linhas de lista, menus e capas a partir de 64 px.
pub const RAIO_CARTAO: f32 = 16.0;
/// Controles e capas pequenas.
pub const RAIO_CONTROLE: f32 = 10.0;
pub const PILULA: f32 = 999.0;

/// O raio de uma capa quadrada: cresce com ela.
pub fn raio_da_capa(lado: f32) -> f32 {
    if lado < 64.0 {
        RAIO_CONTROLE
    } else if lado <= 200.0 {
        RAIO_CARTAO
    } else {
        crate::arranjo::RAIO
    }
}

/// O raio de uma barra de gráfico, de um medidor ou de uma miniatura: metade da altura, sem passar de 6.
pub fn raio_fino(altura: f32) -> f32 {
    (altura / 2.0).clamp(0.0, 6.0)
}

/// Um botão só com ícone (a área de clique).
pub const ALVO: f32 = 44.0;
/// Uma linha de música.
pub const LINHA: f32 = 64.0;
/// A capa numa linha.
pub const CAPA_DA_LINHA: f32 = 48.0;
/// Uma ficha (filtro, botão pequeno).
pub const FICHA: f32 = 40.0;
/// Um botão com nome.
pub const BOTAO: f32 = 44.0;
/// O selo de um serviço: no cartão, na linha e na barra de baixo.
pub const MARCA: f32 = 28.0;
pub const MARCA_DA_LINHA: f32 = 20.0;
pub const MARCA_DA_BARRA: f32 = 24.0;
/// O botão de tocar que o selo vira, no cartão.
pub const BOTAO_DO_CARTAO: f32 = 44.0;
/// Um selo de estado ("Em breve", "Vem com o app").
pub const SELO: f32 = 28.0;

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use std::path::{Path, PathBuf};

    fn arquivos(pasta: &Path, saida: &mut Vec<PathBuf>) {
        for e in std::fs::read_dir(pasta).expect("pasta das telas").flatten() {
            let p = e.path();
            if p.is_dir() {
                if p.file_name().is_some_and(|n| n != "ilha") {
                    arquivos(&p, saida);
                }
            } else if p.extension().is_some_and(|x| x == "rs") && p.file_name().is_some_and(|n| n != "medidas.rs") {
                saida.push(p);
            }
        }
    }

    /// O que vem logo depois de `abre` começa com um número? (`texto(14.0)` sim, `texto(tamanho)` não.)
    fn numero_depois(linha: &str, abre: &str) -> bool {
        linha.match_indices(abre).any(|(i, _)| {
            // `contexto(` não é `texto(`.
            let solto = !linha[..i].ends_with(|c: char| c.is_alphanumeric() || c == '_');
            solto && linha[i + abre.len()..].trim_start().starts_with(|c: char| c.is_ascii_digit())
        })
    }

    /// `icone("icones/x.svg", 18.0, ..)`: um número como segundo argumento, na mesma linha.
    fn icone_com_numero(linha: &str) -> bool {
        linha.match_indices("icone(").any(|(i, _)| {
            let mut fundo = 0;
            for (j, c) in linha[i + 6..].char_indices() {
                match c {
                    '(' | '{' => fundo += 1,
                    ')' | '}' => fundo -= 1,
                    ',' if fundo == 0 => {
                        return linha[i + 6 + j + 1..].trim_start().starts_with(|c: char| c.is_ascii_digit());
                    }
                    _ => {}
                }
                if fundo < 0 {
                    return false;
                }
            }
            false
        })
    }

    #[test]
    fn sem_numero_solto() {
        let mut todos = Vec::new();
        arquivos(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui"), &mut todos);
        assert!(todos.len() > 20, "achou as telas: {}", todos.len());
        let abre = ["texto(", "titulo(", "numero(", ".text_size(px(", "tamanho_icone: "];
        let raios = [".rounded(px(", ".rounded_t(px(", ".rounded_b(px(", ".rounded_l(px(", ".rounded_r(px("];
        let mut soltos = Vec::new();
        for a in todos {
            let codigo = std::fs::read_to_string(&a).expect("tela");
            let telas = codigo.split("#[cfg(test)]").next().unwrap_or_default();
            for (n, l) in telas.lines().enumerate().filter(|(_, l)| !l.trim_start().starts_with("//")) {
                let achou = abre.iter().any(|x| numero_depois(l, x))
                    || raios.iter().any(|x| {
                        l.contains(x) && {
                            let i = l.find(x).unwrap_or_default() + x.len();
                            l[i..].starts_with(|c: char| c.is_ascii_digit())
                        }
                    })
                    || icone_com_numero(l);
                if achou {
                    let nome = a.file_name().unwrap_or_default().to_string_lossy().into_owned();
                    soltos.push(format!("{nome}:{} {}", n + 1, l.trim()));
                }
            }
        }
        assert!(soltos.is_empty(), "{} medidas soltas:\n{}", soltos.len(), soltos.join("\n"));
    }

    #[test]
    fn a_escala_arredonda_e_respeita_a_janela() {
        assert_eq!(escalar(14.5, 1.0), 14.5, "em 100% nada muda");
        assert_eq!(escalar(14.0, 1.1), 15.0);
        assert_eq!(escalar(16.0, 1.25), 20.0);
        assert_eq!(escalar(1.0, 1.25), 1.25, "os pequenos não somem nem dobram");
        assert_eq!(escalar(-24.0, 0.9), -22.0);
        assert_eq!(escala_que_cabe(125, 1440.0, 900.0), 1.25);
        assert_eq!(escala_que_cabe(90, 360.0, 400.0), 0.9);
        assert_eq!(escala_que_cabe(125, 360.0, 400.0), 1.0, "a janela mínima não aguenta 125%");
        assert_eq!(escala_que_cabe(110, f32::NAN, 400.0), 1.1);
    }

    #[test]
    fn o_raio_da_capa_cresce_com_ela() {
        assert_eq!(raio_da_capa(48.0), RAIO_CONTROLE);
        assert_eq!(raio_da_capa(156.0), RAIO_CARTAO);
        assert_eq!(raio_da_capa(232.0), 28.0);
        assert_eq!((raio_fino(4.0), raio_fino(40.0), raio_fino(-1.0)), (2.0, 6.0, 0.0));
    }
}
