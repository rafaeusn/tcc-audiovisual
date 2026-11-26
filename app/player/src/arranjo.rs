//! Arranjo responsivo: do tamanho da janela para o retângulo de cada região.
//!
//! É conta pura, então dá para testar todos os tamanhos sem abrir janela. As medidas são as do mockup v4
//! (`docs/design-ui-v1.md`), com o topo, o dock e a navegação maiores da remodelagem de 25/11/2026.
//!
//! | Largura da janela | Modo |
//! | --- | --- |
//! | acima de 1060 | `Largo`: barra lateral, visual, painel ao lado, dock |
//! | 761 a 1060 | `Gaveta`: o painel sai do lado e abre por cima, como gaveta |
//! | até 760 | `Empilhado`: tudo em coluna, com a navegação embaixo |
//!
//! Na Estante o visual fica ao lado da página: `Largo` (com a lateral) vai de 1200 para cima, e o painel toma o lugar da
//! página até 1679 (ver [`estante`]).

use crate::ui::medidas::{CARTAZ, PAGINA, SECAO};
use nucleo::ajustes::DIVISAO;

pub const MARGEM: f32 = 12.0;
/// Largura do trilho de navegação (ícones com o nome embaixo).
pub const TRILHO: f32 = 92.0;
pub const TOPO: f32 = 64.0;
pub const DOCK: f32 = 96.0;
/// O dock numa janela baixa (menos de 640 px).
const DOCK_BAIXO: f32 = 88.0;
const DOCK_EMPILHADO: f32 = 116.0;
/// O dock do empilhado numa janela baixa: uma linha só, sem a linha do tempo.
pub const DOCK_EMPILHADO_BAIXO: f32 = 72.0;

/// O dock do empilhado: em janela baixa, uma linha só (a página precisa da altura).
fn dock_empilhado(altura: f32) -> f32 {
    if altura < 640.0 { DOCK_EMPILHADO_BAIXO } else { DOCK_EMPILHADO }
}
const NAV: f32 = 72.0;
const PAINEL: f32 = 380.0;
const PAINEL_ESTREITO: f32 = 340.0;
/// Largura da lateral da Estante (destinos e biblioteca).
pub const LATERAL: f32 = 264.0;
/// A lateral da Estante aparece a partir desta largura de janela; abaixo, o trilho com nomes.
const COM_LATERAL: f32 = 1200.0;
/// O painel da Estante cabe ao lado da página a partir desta largura; abaixo, toma o lugar dela.
pub const PAINEL_AO_LADO: f32 = 1680.0;
/// Na Estante, o visual e a página nunca ficam mais estreitos que isto.
const VISUAL_MINIMO: f32 = 220.0;
const PAGINA_MINIMA: f32 = 320.0;
/// A tira do visual no alto da página larga, e numa janela baixa ou estreita.
const TIRA: f32 = 96.0;
const TIRA_BAIXA: f32 = 72.0;
/// Raio dos cantos do visual, do painel e do dock.
pub const RAIO: f32 = 28.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Ret {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Ret {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    #[cfg(test)]
    pub fn direita(&self) -> f32 {
        self.x + self.w
    }

    pub fn base(&self) -> f32 {
        self.y + self.h
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modo {
    Largo,
    Gaveta,
    Empilhado,
}

/// O que a interface quer mostrar (além do tamanho da janela).
#[derive(Clone, Copy, Debug, Default)]
pub struct Pedido {
    pub painel_aberto: bool,
    pub gaveta_aberta: bool,
    pub tela_cheia: bool,
    /// Painel à esquerda do visual.
    pub painel_a_esquerda: bool,
    /// Skin imersiva: o visual ocupa a janela inteira, por baixo das outras regiões.
    pub imersivo: bool,
    /// Janela mini: só o visual, com o título pequeno.
    pub mini: bool,
    /// Uma página com lista (Biblioteca, Loja) está aberta. No empilhado ela fica com o meio inteiro: no cartão de
    /// cima, dividindo a altura com o painel, a lista nem aparecia.
    pub pagina_cheia: bool,
    /// O layout Estante (o visual no centro e a página ao lado).
    pub estante: bool,
    /// Na Estante, quanto o visual já cresceu até a janela inteira (0 ao lado da página, 1 na janela).
    pub expansao: f32,
    /// Na Estante, a parte da largura que fica com o visual (`None` = o padrão da largura da janela).
    pub divisao: Option<f32>,
    /// A divisória está sendo arrastada.
    pub arrastando: bool,
    /// Página larga: onde a página fica e quanto o visual já virou tira (0 a 1).
    pub larga: bool,
    pub larga_em: f32,
    /// No empilhado da Estante, quanto o visual está alto (0 = a tira, 1 = o alto do Início).
    pub alta: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arranjo {
    pub modo: Modo,
    pub janela: Ret,
    /// Só o visual na tela: as outras regiões somem.
    pub so_visual: bool,
    pub trilho: Option<Ret>,
    pub topo: Ret,
    pub heroi: Ret,
    /// Onde o visual é desenhado (igual ao herói, ou a janela inteira na tela cheia e na skin imersiva).
    pub visual: Ret,
    /// Raio dos cantos do visual (0 quando ele ocupa a janela inteira).
    pub raio_visual: f32,
    /// Painel no fluxo (ao lado do visual no modo largo, abaixo dele no empilhado).
    pub painel: Option<Ret>,
    /// Painel aberto por cima, como gaveta.
    pub gaveta: Option<Ret>,
    pub dock: Ret,
    pub nav: Option<Ret>,
    /// Estante: a lateral com os destinos e a biblioteca.
    pub lateral: Option<Ret>,
    /// Estante: o miolo (o visual, a divisória e a página).
    pub miolo: Ret,
    /// Estante: o painel está no lugar da página.
    pub painel_na_pagina: bool,
    /// Estante: o vão entre o visual e a página, que se arrasta.
    pub divisoria: Option<Ret>,
    /// O visual é desenhado neste tamanho, e o meio dele aparece no retângulo (`None` = no tamanho do retângulo).
    pub interno: Option<(f32, f32)>,
    /// Tamanho-base do título da faixa sobre o visual (cartaz, página ou seção). O herói ainda encolhe nomes compridos.
    pub titulo: f32,
    /// Janela baixa: números e espaços menores.
    pub compacto: bool,
}

pub fn calcular(largura: f32, altura: f32, p: Pedido) -> Arranjo {
    let limpo = |v: f32, min: f32| if v.is_finite() { v.max(min) } else { min };
    // A janela mini é menor que qualquer outra: o arranjo dela acompanha o tamanho de verdade.
    let (menor_w, menor_h) = if p.mini { (160.0, 100.0) } else { (320.0, 400.0) };
    let (w, h) = (limpo(largura, menor_w), limpo(altura, menor_h));
    let janela = Ret::new(0.0, 0.0, w, h);
    let largo = if p.estante { w >= COM_LATERAL } else { w > 1060.0 };
    let modo = if largo {
        Modo::Largo
    } else if w > 760.0 {
        Modo::Gaveta
    } else {
        Modo::Empilhado
    };
    let compacto = h <= 780.0;

    if p.tela_cheia || p.mini {
        let titulo = if p.mini {
            SECAO
        } else if w < 700.0 {
            PAGINA
        } else {
            CARTAZ
        };
        let vazio = Ret::default();
        return Arranjo {
            modo,
            janela,
            so_visual: true,
            trilho: None,
            topo: vazio,
            heroi: janela,
            visual: janela,
            raio_visual: 0.0,
            painel: None,
            gaveta: None,
            dock: vazio,
            nav: None,
            lateral: None,
            miolo: Ret::default(),
            painel_na_pagina: false,
            divisoria: None,
            interno: None,
            titulo,
            compacto,
        };
    }

    if p.estante {
        return estante(janela, modo, compacto, p);
    }

    if modo == Modo::Empilhado {
        let lw = w - 2.0 * MARGEM;
        let topo = Ret::new(MARGEM, 0.0, lw, TOPO);
        let nav = Ret::new(0.0, h - NAV, w, NAV);
        let dock = Ret::new(MARGEM, nav.y - 8.0 - dock_empilhado(h), lw, dock_empilhado(h));
        let livre = (dock.y - MARGEM) - (TOPO + MARGEM);
        // O visual fica com 46% do espaço (no máximo um quadrado e meio); o painel rola no que sobra.
        let mut hh = (livre * 0.46).min(lw * 0.9).max(160.0);
        let mut resto = livre - hh - MARGEM;
        if p.pagina_cheia || resto < 120.0 {
            // Uma página com lista aberta, ou janela baixa demais para os dois: fica só o meio, sem o painel.
            hh = livre.max(120.0);
            resto = 0.0;
        }
        let heroi = Ret::new(MARGEM, TOPO + MARGEM, lw, hh);
        let painel = (resto > 0.0).then(|| Ret::new(MARGEM, heroi.base() + MARGEM, lw, resto));
        let (visual, raio_visual) = if p.imersivo { (janela, 0.0) } else { (heroi, RAIO) };
        return Arranjo {
            modo,
            janela,
            so_visual: false,
            trilho: None,
            topo,
            heroi,
            visual,
            raio_visual,
            painel,
            gaveta: None,
            dock,
            nav: Some(nav),
            lateral: None,
            miolo: Ret::default(),
            painel_na_pagina: false,
            divisoria: None,
            interno: None,
            titulo: SECAO,
            compacto,
        };
    }

    let x0 = TRILHO + MARGEM;
    let conteudo_w = w - x0 - MARGEM;
    let trilho = Ret::new(0.0, 0.0, TRILHO, h);
    let topo = Ret::new(TRILHO, 0.0, w - TRILHO - MARGEM, TOPO);
    let dh = dock_de(h);
    let dock = Ret::new(x0, h - MARGEM - dh, conteudo_w, dh);
    let y0 = TOPO + MARGEM;
    let meio_h = (dock.y - MARGEM - y0).max(120.0);

    let largura_painel = if w > 1240.0 { PAINEL } else { PAINEL_ESTREITO };
    let meio = |x: f32, largura: f32| Ret::new(x, y0, largura, meio_h);
    let (painel, gaveta, heroi) = match modo {
        Modo::Largo if p.painel_aberto => {
            let hw = conteudo_w - largura_painel - MARGEM;
            if p.painel_a_esquerda {
                (Some(meio(x0, largura_painel)), None, meio(x0 + largura_painel + MARGEM, hw))
            } else {
                (Some(meio(w - MARGEM - largura_painel, largura_painel)), None, meio(x0, hw))
            }
        }
        Modo::Gaveta if p.gaveta_aberta => {
            let gw = PAINEL.min(conteudo_w - 48.0);
            let gx = if p.painel_a_esquerda { x0 } else { w - MARGEM - gw };
            (None, Some(meio(gx, gw)), meio(x0, conteudo_w))
        }
        _ => (None, None, meio(x0, conteudo_w)),
    };
    let titulo = if heroi.w >= 640.0 && !compacto && heroi.h >= 420.0 { CARTAZ } else { PAGINA };
    let (visual, raio_visual) = if p.imersivo { (janela, 0.0) } else { (heroi, RAIO) };
    Arranjo {
        modo,
        janela,
        so_visual: false,
        trilho: Some(trilho),
        topo,
        heroi,
        visual,
        raio_visual,
        painel,
        gaveta,
        dock,
        nav: None,
        lateral: None,
        miolo: Ret::default(),
        painel_na_pagina: false,
        divisoria: None,
        interno: None,
        titulo,
        compacto,
    }
}

fn dock_de(altura: f32) -> f32 {
    if altura < 640.0 { DOCK_BAIXO } else { DOCK }
}

/// O retângulo do visual crescendo até a janela (`e` de 0 a 1), com o raio sumindo.
fn expandir(visual: Ret, raio: f32, janela: Ret, e: f32) -> (Ret, f32) {
    let e = if e.is_finite() { e.clamp(0.0, 1.0) } else { 0.0 };
    if e >= 0.999 {
        return (janela, 0.0);
    }
    (entre(&visual, &janela, e), raio * (1.0 - e))
}

/// A divisória da Estante: a largura do visual para uma parte `divisao` do `miolo` (sem o vão), entre os limites.
fn largura_do_visual(miolo: f32, divisao: f32) -> f32 {
    let util = miolo - MARGEM;
    let minimo = (util * DIVISAO.0).max(VISUAL_MINIMO);
    let maximo = (util * DIVISAO.1).min(util - PAGINA_MINIMA).max(minimo);
    let d = if divisao.is_finite() { divisao } else { 0.5 };
    (util * d).clamp(minimo, maximo).round()
}

/// A parte do visual com a divisória em `x` (o ponteiro, no meio do vão).
pub fn divisao_no_ponto(miolo: &Ret, x: f32) -> f32 {
    let util = (miolo.w - MARGEM).max(1.0);
    ((x - MARGEM / 2.0 - miolo.x) / util).clamp(DIVISAO.0, DIVISAO.1)
}

/// No Início do empilhado, o visual recolhe (vira tira) ao rolar a página e volta no alto dela. Só recolhe se a página
/// ainda rola depois de crescer o que o visual deixou (`ganho`); senão ela voltaria ao alto e ficaria indo e vindo.
pub fn recolhe(antes: bool, rolado: f32, sobra: f32, ganho: f32) -> bool {
    if antes { rolado > 0.5 } else { rolado > 24.0 && sobra > ganho + 48.0 }
}

/// Quanto a página do empilhado ganha quando o visual recolhe.
pub fn ganho_ao_recolher(miolo: &Ret) -> f32 {
    altura_alta(miolo).map_or(0.0, |h| h - TIRA_BAIXA)
}

/// O visual alto do Início no empilhado: 40% do miolo, quando sobra página.
fn altura_alta(miolo: &Ret) -> Option<f32> {
    let h = (miolo.h * 0.4).round();
    (h >= 120.0 && miolo.h - h - VAO_ESTREITO >= 240.0).then_some(h)
}

/// O vão entre as regiões do empilhado.
const VAO_ESTREITO: f32 = 8.0;

/// O layout Estante: a navegação (lateral, trilho ou barra embaixo), o visual, a divisória e a página. Com a página larga,
/// o visual é uma tira no alto dela. O painel fica ao lado em janela grande e no lugar da página nas outras. No
/// empilhado, o visual fica acima da página: alto no Início, uma tira nas outras.
fn estante(janela: Ret, modo: Modo, compacto: bool, p: Pedido) -> Arranjo {
    let (w, h) = (janela.w, janela.h);
    let y0 = TOPO + MARGEM;
    let vazio = Ret::default();
    let base = Arranjo {
        modo,
        janela,
        so_visual: false,
        trilho: None,
        topo: Ret::new(MARGEM, 0.0, w - 2.0 * MARGEM, TOPO),
        heroi: vazio,
        visual: vazio,
        raio_visual: RAIO,
        painel: None,
        gaveta: None,
        dock: vazio,
        nav: None,
        lateral: None,
        miolo: vazio,
        painel_na_pagina: false,
        divisoria: None,
        interno: None,
        titulo: PAGINA,
        compacto,
    };
    let mut a = if modo == Modo::Empilhado { estante_estreita(base, p) } else { estante_lado_a_lado(base, y0, p) };
    let e = if p.expansao.is_finite() { p.expansao.clamp(0.0, 1.0) } else { 0.0 };
    if e > 0.0 {
        a.interno = Some((w, h));
    }
    (a.visual, a.raio_visual) = expandir(a.visual, a.raio_visual, janela, e);
    a
}

fn estante_lado_a_lado(base: Arranjo, y0: f32, p: Pedido) -> Arranjo {
    let (w, h) = (base.janela.w, base.janela.h);
    // A lateral vai do alto ao pé da janela; o topo fica sobre o miolo.
    let lateral = (base.modo == Modo::Largo).then(|| Ret::new(MARGEM, MARGEM, LATERAL, (h - 2.0 * MARGEM).max(120.0)));
    let x0 = lateral.map_or(TRILHO + MARGEM, |l| l.x + l.w + MARGEM);
    let trilho = lateral.is_none().then(|| Ret::new(0.0, 0.0, TRILHO, h));
    let topo = Ret::new(x0, 0.0, w - MARGEM - x0, TOPO);
    let dh = dock_de(h);
    let dock = Ret::new(x0, h - MARGEM - dh, w - MARGEM - x0, dh);
    let meio_h = (dock.y - MARGEM - y0).max(120.0);

    let ao_lado = p.gaveta_aberta && w >= PAINEL_AO_LADO;
    let largura = w - MARGEM - x0 - if ao_lado { PAINEL + MARGEM } else { 0.0 };
    let miolo = Ret::new(x0, y0, largura, meio_h);
    let padrao = if w >= PAINEL_AO_LADO {
        0.52
    } else if w >= COM_LATERAL {
        0.46
    } else {
        0.48
    };
    let vw = largura_do_visual(miolo.w, p.divisao.unwrap_or(padrao));
    let lado = Ret::new(miolo.x, y0, vw, meio_h);
    let tira = Ret::new(miolo.x, y0, miolo.w, if h < 640.0 { TIRA_BAIXA } else { TIRA });
    let em = if p.larga_em.is_finite() { p.larga_em.clamp(0.0, 1.0) } else { f32::from(u8::from(p.larga)) };
    let visual = entre(&lado, &tira, em);
    let heroi = if p.larga {
        let y = tira.base() + MARGEM;
        Ret::new(miolo.x, y, miolo.w, (miolo.base() - y).max(120.0))
    } else {
        Ret::new(lado.x + vw + MARGEM, y0, miolo.w - vw - MARGEM, meio_h)
    };
    let painel =
        p.gaveta_aberta.then(|| if ao_lado { Ret::new(miolo.x + miolo.w + MARGEM, y0, PAINEL, meio_h) } else { heroi });
    // A tira mostra o meio da cena do miolo inteiro; arrastando, a cena fica na largura maior. Nada é recriado no caminho.
    let interno = if em > 0.0 {
        Some((miolo.w, miolo.h))
    } else {
        p.arrastando.then(|| (largura_do_visual(miolo.w, DIVISAO.1), miolo.h))
    };
    Arranjo {
        trilho,
        topo,
        heroi,
        visual,
        painel,
        dock,
        lateral,
        miolo,
        painel_na_pagina: p.gaveta_aberta && !ao_lado,
        divisoria: (!p.larga).then(|| Ret::new(lado.x + vw, y0, MARGEM, meio_h)),
        interno,
        titulo: if lado.w >= 360.0 && meio_h >= 300.0 { PAGINA } else { SECAO },
        ..base
    }
}

fn estante_estreita(base: Arranjo, p: Pedido) -> Arranjo {
    let (w, h) = (base.janela.w, base.janela.h);
    let y0 = TOPO + VAO_ESTREITO;
    let lw = w - 2.0 * MARGEM;
    let nav = Ret::new(0.0, h - NAV, w, NAV);
    let dock = Ret::new(MARGEM, nav.y - VAO_ESTREITO - dock_empilhado(h), lw, dock_empilhado(h));
    let miolo = Ret::new(MARGEM, y0, lw, (dock.y - VAO_ESTREITO - y0).max(120.0));
    let alta = altura_alta(&miolo);
    let em = if p.alta.is_finite() && !p.gaveta_aberta { p.alta.clamp(0.0, 1.0) } else { 0.0 };
    // Janela baixa: sem lugar para a página e a tira, a tira sai (a página, opaca, a cobriria).
    let altura = if miolo.h - TIRA_BAIXA - VAO_ESTREITO < 160.0 {
        0.0
    } else {
        alta.map_or(TIRA_BAIXA, |alta| TIRA_BAIXA + (alta - TIRA_BAIXA) * em)
    };
    let visual = if altura > 0.0 { Ret::new(miolo.x, y0, lw, altura) } else { Ret::default() };
    let yh = if altura > 0.0 { visual.base() + VAO_ESTREITO } else { y0 };
    let heroi = Ret::new(MARGEM, yh, lw, (miolo.base() - yh).max(120.0));
    Arranjo {
        heroi,
        visual,
        dock,
        nav: Some(nav),
        miolo,
        painel: p.gaveta_aberta.then_some(heroi),
        painel_na_pagina: p.gaveta_aberta,
        interno: (altura > 0.0).then(|| (lw, alta.unwrap_or(lw * 0.5))),
        titulo: SECAO,
        raio_visual: 16.0,
        ..base
    }
}

/// O retângulo a caminho de `a` para `b` (0 a 1).
fn entre(a: &Ret, b: &Ret, e: f32) -> Ret {
    let um = |a: f32, b: f32| a + (b - a) * e;
    Ret::new(um(a.x, b.x), um(a.y, b.y), um(a.w, b.w), um(a.h, b.h))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    const ABERTO: Pedido = Pedido {
        painel_aberto: true,
        gaveta_aberta: false,
        tela_cheia: false,
        painel_a_esquerda: false,
        imersivo: false,
        mini: false,
        pagina_cheia: false,
        estante: false,
        expansao: 0.0,
        divisao: None,
        arrastando: false,
        larga: false,
        larga_em: 0.0,
        alta: 0.0,
    };

    fn dentro(r: &Ret, j: &Ret) -> bool {
        r.x >= -0.01 && r.y >= -0.01 && r.direita() <= j.w + 0.01 && r.base() <= j.h + 0.01 && r.w > 0.0 && r.h > 0.0
    }

    fn cruza(a: &Ret, b: &Ret) -> bool {
        a.x < b.direita() - 0.01 && b.x < a.direita() - 0.01 && a.y < b.base() - 0.01 && b.y < a.base() - 0.01
    }

    const ESTANTE: Pedido = Pedido { estante: true, ..ABERTO };

    /// As regiões da Estante ficam na janela e não se cruzam.
    fn conferir(w: f32, h: f32, p: Pedido) -> Arranjo {
        let a = calcular(w, h, p);
        let mut regioes = vec![a.topo, a.dock];
        // O painel no lugar da página toma o retângulo dela.
        regioes.push(if a.painel_na_pagina { a.painel.expect("o painel") } else { a.heroi });
        regioes.extend(a.painel.filter(|_| !a.painel_na_pagina));
        regioes.extend(a.trilho);
        regioes.extend(a.lateral);
        regioes.extend(a.nav);
        regioes.extend((a.visual.w > 0.0).then_some(a.visual));
        for r in &regioes {
            assert!(dentro(r, &a.janela), "{w}x{h}: {r:?} fora da janela");
        }
        for (i, r1) in regioes.iter().enumerate() {
            for r2 in &regioes[i + 1..] {
                assert!(!cruza(r1, r2), "{w}x{h} {p:?}: {r1:?} cruza {r2:?}");
            }
        }
        a
    }

    const TAMANHOS: [(f32, f32); 13] = [
        (2560.0, 1440.0),
        (1920.0, 1080.0),
        (1680.0, 1050.0),
        (1440.0, 900.0),
        (1200.0, 720.0),
        (1199.0, 700.0),
        (958.0, 560.0),
        (761.0, 560.0),
        (760.0, 900.0),
        (410.0, 844.0),
        (400.0, 420.0),
        (400.0, 400.0),
        (320.0, 400.0),
    ];

    #[test]
    fn estante_nada_se_cruza_em_nenhum_tamanho() {
        for (w, h) in TAMANHOS {
            for gaveta_aberta in [false, true] {
                for larga in [false, true] {
                    for divisao in [None, Some(0.0), Some(0.28), Some(0.68), Some(2.0), Some(f32::NAN)] {
                        for alta in [0.0, 1.0] {
                            let larga_em = f32::from(u8::from(larga));
                            let p = Pedido { gaveta_aberta, larga, larga_em, divisao, alta, ..ESTANTE };
                            let a = conferir(w, h, p);
                            if a.modo != Modo::Empilhado {
                                assert!(a.visual.w >= VISUAL_MINIMO && a.heroi.w >= PAGINA_MINIMA, "{w}x{h}: {a:?}");
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn estante_lado_a_lado_com_lateral_ou_trilho() {
        let a = conferir(1440.0, 900.0, ESTANTE);
        assert_eq!(a.modo, Modo::Largo);
        assert_eq!(a.lateral.map(|l| (l.y, l.h)), Some((MARGEM, 900.0 - 2.0 * MARGEM)), "do alto ao pé");
        assert!(a.trilho.is_none() && a.painel.is_none(), "o painel vem fechado");
        // O visual e a página na mesma linha, com 46% para ele, e a divisória no vão.
        assert_eq!((a.visual.y, a.visual.h), (a.heroi.y, a.heroi.h));
        assert!((a.visual.w / (a.miolo.w - MARGEM) - 0.46).abs() < 0.01);
        assert_eq!(a.divisoria.map(|d| (d.x, d.w)), Some((a.visual.direita(), MARGEM)));
        assert_eq!(a.heroi.x, a.visual.direita() + MARGEM);
        // O dock e o topo ficam sob e sobre os dois, não sob a lateral.
        assert_eq!((a.dock.x, a.topo.x), (a.visual.x, a.visual.x));
        assert_eq!(a.dock.direita(), a.heroi.direita());
        assert_eq!(a.interno, None);
        let media = conferir(958.0, 560.0, ESTANTE);
        assert!(media.trilho.is_some() && media.lateral.is_none());
        assert!(media.visual.w >= 390.0 && media.heroi.w >= 420.0, "{:?} {:?}", media.visual, media.heroi);
        let grande = conferir(1920.0, 1080.0, ESTANTE);
        assert!(grande.visual.w > grande.heroi.w, "52 por 48");
    }

    #[test]
    fn estante_painel_ao_lado_ou_no_lugar_da_pagina() {
        let aberto = Pedido { gaveta_aberta: true, ..ESTANTE };
        let grande = conferir(1680.0, 1050.0, aberto);
        assert!(!grande.painel_na_pagina);
        assert_eq!(grande.painel.map(|p| p.w), Some(PAINEL));
        assert!(grande.miolo.w < calcular(1680.0, 1050.0, ESTANTE).miolo.w);
        for (w, h) in [(1679.0, 900.0), (958.0, 560.0), (410.0, 844.0)] {
            let a = conferir(w, h, aberto);
            assert!(a.painel_na_pagina, "{w}x{h}");
            assert_eq!(a.painel, Some(a.heroi));
            assert!(a.gaveta.is_none());
        }
        assert!(conferir(958.0, 560.0, aberto).painel.is_some_and(|p| p.h >= 360.0), "cabe uma lista na janela baixa");
    }

    #[test]
    fn a_divisoria_segue_o_ponteiro_nos_limites() {
        let a = calcular(1440.0, 900.0, ESTANTE);
        for x in [640.0, 700.0, 900.0] {
            let d = divisao_no_ponto(&a.miolo, x);
            let b = calcular(1440.0, 900.0, Pedido { divisao: Some(d), arrastando: true, ..ESTANTE });
            assert!((b.visual.direita() + MARGEM / 2.0 - x).abs() <= 1.0, "{x}: {:?}", b.visual);
            // Arrastando, a cena tem um tamanho só, que cobre qualquer largura.
            let cena = b.interno.expect("a cena");
            assert!(cena.0 >= b.visual.w && cena.1 == b.visual.h);
        }
        assert_eq!(divisao_no_ponto(&a.miolo, -50.0), DIVISAO.0);
        assert_eq!(divisao_no_ponto(&a.miolo, 5000.0), DIVISAO.1);
    }

    #[test]
    fn pagina_larga_deixa_uma_tira_do_visual() {
        let p = Pedido { larga: true, larga_em: 1.0, ..ESTANTE };
        let a = conferir(1440.0, 900.0, p);
        assert_eq!((a.visual.h, a.visual.w), (TIRA, a.miolo.w));
        assert_eq!(a.heroi.w, a.miolo.w);
        assert!(a.divisoria.is_none());
        assert_eq!(a.interno, Some((a.miolo.w, a.miolo.h)));
        assert_eq!(conferir(958.0, 560.0, p).visual.h, TIRA_BAIXA, "janela baixa");
        // No caminho: a página já está no lugar e o visual vai encolhendo, desenhado num tamanho só.
        let meio = calcular(1440.0, 900.0, Pedido { larga: true, larga_em: 0.5, ..ESTANTE });
        assert_eq!((meio.heroi, meio.interno), (a.heroi, a.interno));
        assert!(meio.visual.h > a.visual.h && meio.visual.w < a.visual.w);
    }

    #[test]
    fn estante_empilhada_visual_no_alto() {
        let inicio = conferir(410.0, 844.0, Pedido { alta: 1.0, ..ESTANTE });
        let outra = conferir(410.0, 844.0, ESTANTE);
        assert_eq!(outra.visual.h, TIRA_BAIXA);
        assert!(inicio.visual.h > 180.0 && inicio.visual.base() < inicio.heroi.y, "{:?}", inicio.visual);
        assert_eq!(inicio.interno, outra.interno, "recolher não recria nada");
        assert!(outra.heroi.h > 350.0);
        // Janela baixa: a tira sai, para a página (opaca) não cobrir o visual.
        let baixa = conferir(600.0, 420.0, Pedido { alta: 1.0, ..ESTANTE });
        assert!(baixa.visual.w < 2.0);
        // Com o painel, o visual é a tira.
        assert_eq!(conferir(410.0, 844.0, Pedido { alta: 1.0, gaveta_aberta: true, ..ESTANTE }).visual.h, TIRA_BAIXA);
    }

    /// Cada tamanho da interface em cada modo: o arranjo é feito na janela dividida pela escala, e nada se cruza nem sai.
    #[test]
    fn cada_tamanho_da_interface_em_cada_modo() {
        for tamanho in nucleo::ajustes::TAMANHOS {
            for (w, h) in [(1920.0, 1080.0), (1440.0, 900.0), (958.0, 560.0), (410.0, 844.0), (360.0, 400.0)] {
                let e = crate::ui::medidas::escala_que_cabe(tamanho, w, h);
                // (No Palco o visual fica por baixo da página: só a Estante passa pela conferência das regiões.)
                assert!(calcular(w / e, h / e, ABERTO).janela.w * e <= w + 0.5);
                for p in [ESTANTE, Pedido { gaveta_aberta: true, ..ESTANTE }] {
                    let a = conferir(w / e, h / e, p);
                    assert!(
                        a.janela.w * e <= w + 0.5 && a.janela.h * e <= h + 0.5,
                        "{tamanho}% {w}x{h}: {:?}",
                        a.janela
                    );
                }
            }
        }
        // Em 125% a janela de 958 de largura já é "média" (trilho), e a de 1440 continua com a lateral.
        let e = crate::ui::medidas::escala_que_cabe(125, 958.0, 560.0);
        assert_eq!(calcular(958.0 / e, 560.0 / e, ESTANTE).modo, Modo::Gaveta);
        assert!(calcular(1440.0 / 1.25, 900.0 / 1.25, ESTANTE).lateral.is_none(), "1152 lógicos: o trilho");
    }

    #[test]
    fn janela_baixa_e_estreita_fica_com_a_barra_de_uma_linha() {
        for p in [ABERTO, ESTANTE] {
            let a = calcular(400.0, 400.0, p);
            assert_eq!(a.dock.h, DOCK_EMPILHADO_BAIXO);
            assert!(a.heroi.h >= 160.0, "sobra página: {:?}", a.heroi);
            assert_eq!(calcular(400.0, 844.0, p).dock.h, DOCK_EMPILHADO);
        }
    }

    #[test]
    fn recolher_nao_fica_indo_e_vindo() {
        let ganho = ganho_ao_recolher(&calcular(410.0, 844.0, ESTANTE).miolo);
        assert!(ganho > 100.0);
        assert!(!recolhe(false, 10.0, 900.0, ganho), "rolou pouco");
        assert!(recolhe(false, 40.0, 900.0, ganho));
        assert!(!recolhe(false, 40.0, ganho, ganho), "a página caberia inteira: voltaria ao alto");
        assert!(recolhe(true, 30.0, 60.0, ganho), "continua recolhido até o alto");
        assert!(!recolhe(true, 0.0, 900.0, ganho));
    }

    #[test]
    fn expandir_leva_o_visual_a_janela_inteira() {
        let base = calcular(1440.0, 900.0, ESTANTE);
        let meio = calcular(1440.0, 900.0, Pedido { expansao: 0.5, ..ESTANTE });
        let fim = calcular(1440.0, 900.0, Pedido { expansao: 1.0, ..ESTANTE });
        assert!(meio.visual.w > base.visual.w && meio.visual.w < 1440.0);
        assert!(meio.raio_visual < base.raio_visual);
        assert_eq!(meio.interno, Some((1440.0, 900.0)));
        assert_eq!(fim.visual, fim.janela);
        assert_eq!(fim.raio_visual, 0.0);
        assert_eq!(meio.heroi, base.heroi, "as outras regiões ficam onde estão");
        // O empilhado e a página larga também expandem.
        for p in [ESTANTE, Pedido { larga: true, larga_em: 1.0, ..ESTANTE }] {
            let e = calcular(600.0, 900.0, Pedido { expansao: 1.0, ..p });
            assert_eq!(e.visual, e.janela);
        }
    }

    #[test]
    fn medidas_do_mockup_em_1440x900() {
        let a = calcular(1440.0, 900.0, ABERTO);
        assert_eq!(a.modo, Modo::Largo);
        assert_eq!(a.heroi, Ret::new(104.0, 76.0, 932.0, 704.0));
        assert_eq!(a.painel, Some(Ret::new(1048.0, 76.0, 380.0, 704.0)));
        assert_eq!(a.dock, Ret::new(104.0, 792.0, 1324.0, 96.0));
        assert_eq!(a.titulo, CARTAZ);
    }

    #[test]
    fn modos_por_largura() {
        assert_eq!(calcular(1061.0, 800.0, ABERTO).modo, Modo::Largo);
        assert_eq!(calcular(1060.0, 800.0, ABERTO).modo, Modo::Gaveta);
        assert_eq!(calcular(761.0, 800.0, ABERTO).modo, Modo::Gaveta);
        assert_eq!(calcular(760.0, 800.0, ABERTO).modo, Modo::Empilhado);
        assert_eq!(calcular(1200.0, 800.0, ABERTO).painel.map(|p| p.w), Some(340.0));
    }

    #[test]
    fn gaveta_abre_por_cima_sem_encolher_o_visual() {
        let fechada = calcular(1024.0, 700.0, ABERTO);
        let aberta = calcular(1024.0, 700.0, Pedido { gaveta_aberta: true, ..ABERTO });
        assert!(fechada.painel.is_none() && fechada.gaveta.is_none());
        assert_eq!(fechada.heroi, aberta.heroi);
        assert!(aberta.gaveta.is_some_and(|g| g.direita() <= 1024.0 - MARGEM + 0.01 && g.x > aberta.heroi.x));
    }

    #[test]
    fn nada_sai_da_janela_nem_se_sobrepoe() {
        for (w, h) in [
            (1920.0, 1080.0),
            (1440.0, 900.0),
            (1280.0, 720.0),
            (1100.0, 600.0),
            (1024.0, 700.0),
            (800.0, 560.0),
            (768.0, 1024.0),
            (390.0, 844.0),
            (360.0, 560.0),
            (320.0, 400.0),
        ] {
            for pedido in [
                ABERTO,
                Pedido::default(),
                Pedido { gaveta_aberta: true, ..ABERTO },
                Pedido { pagina_cheia: true, ..ABERTO },
            ] {
                let a = calcular(w, h, pedido);
                let mut regioes = vec![a.topo, a.heroi, a.dock];
                regioes.extend(a.trilho);
                regioes.extend(a.painel);
                regioes.extend(a.nav);
                for r in &regioes {
                    assert!(dentro(r, &a.janela), "{w}x{h}: {r:?} fora da janela");
                }
                for (i, r1) in regioes.iter().enumerate() {
                    for r2 in &regioes[i + 1..] {
                        assert!(!cruza(r1, r2), "{w}x{h}: {r1:?} cruza {r2:?}");
                    }
                }
                if let Some(g) = a.gaveta {
                    assert!(dentro(&g, &a.janela), "{w}x{h}: gaveta fora da janela");
                }
                assert!(a.heroi.w >= 120.0 && a.heroi.h >= 120.0, "{w}x{h}: visual pequeno demais");
            }
        }
    }

    #[test]
    fn empilhado_tem_navegacao_embaixo_e_titulo_menor() {
        let a = calcular(390.0, 844.0, ABERTO);
        assert_eq!(a.modo, Modo::Empilhado);
        assert!(a.trilho.is_none());
        assert_eq!(a.titulo, SECAO);
        assert!(a.nav.is_some_and(|n| (n.base() - 844.0).abs() < 0.01));
        assert!(a.painel.is_some_and(|p| p.y > a.heroi.base() && p.base() < a.dock.y));
    }

    #[test]
    fn pagina_com_lista_fica_com_o_meio_inteiro_no_empilhado() {
        for (w, h) in [(380.0, 720.0), (390.0, 844.0), (760.0, 1000.0)] {
            let normal = calcular(w, h, ABERTO);
            let cheia = calcular(w, h, Pedido { pagina_cheia: true, ..ABERTO });
            assert_eq!(cheia.modo, Modo::Empilhado);
            assert!(normal.painel.is_some() && cheia.painel.is_none(), "{w}x{h}: o painel sai");
            assert!(cheia.heroi.h > normal.heroi.h + 100.0, "{w}x{h}: a página cresce");
            assert!((cheia.heroi.base() - (cheia.dock.y - MARGEM)).abs() < 0.01, "{w}x{h}: vai até o dock");
        }
        // Fora do empilhado nada muda (a página fica no lugar do visual, ao lado do painel).
        assert_eq!(calcular(1440.0, 900.0, Pedido { pagina_cheia: true, ..ABERTO }), calcular(1440.0, 900.0, ABERTO));
    }

    #[test]
    fn tela_cheia_fica_so_o_visual() {
        let a = calcular(1920.0, 1080.0, Pedido { tela_cheia: true, ..ABERTO });
        assert!(a.so_visual);
        assert_eq!(a.heroi, a.janela);
    }

    #[test]
    fn tamanhos_invalidos_nao_quebram() {
        for (w, h) in [(0.0, 0.0), (f32::NAN, 500.0), (-10.0, f32::INFINITY), (1e9, 1e9)] {
            let a = calcular(w, h, ABERTO);
            assert!(a.heroi.w.is_finite() && a.heroi.h.is_finite() && a.heroi.w > 0.0 && a.heroi.h > 0.0);
        }
    }

    #[test]
    fn painel_a_esquerda_espelha_o_meio() {
        let direita = calcular(1440.0, 900.0, ABERTO);
        let esquerda = calcular(1440.0, 900.0, Pedido { painel_a_esquerda: true, ..ABERTO });
        assert_eq!(esquerda.painel, Some(Ret::new(104.0, 76.0, 380.0, 704.0)));
        assert_eq!(esquerda.heroi, Ret::new(496.0, 76.0, 932.0, 704.0));
        assert_eq!(esquerda.heroi.w, direita.heroi.w);
        let gaveta = calcular(1024.0, 700.0, Pedido { painel_a_esquerda: true, gaveta_aberta: true, ..ABERTO });
        assert!(gaveta.gaveta.is_some_and(|g| g.x == 104.0));
    }

    #[test]
    fn imersivo_poe_o_visual_na_janela_inteira() {
        for (w, h) in [(1440.0, 900.0), (1024.0, 700.0), (390.0, 844.0)] {
            let normal = calcular(w, h, ABERTO);
            let imersivo = calcular(w, h, Pedido { imersivo: true, ..ABERTO });
            assert_eq!(normal.visual, normal.heroi);
            assert_eq!(normal.raio_visual, RAIO);
            assert_eq!(imersivo.visual, imersivo.janela);
            assert_eq!(imersivo.raio_visual, 0.0);
            // As regiões continuam no mesmo lugar: só o visual cresce por baixo delas.
            assert_eq!(imersivo.heroi, normal.heroi);
            assert_eq!(imersivo.dock, normal.dock);
        }
    }

    #[test]
    fn janela_mini_e_so_o_visual_do_tamanho_da_janela() {
        let mini = calcular(384.0, 231.0, Pedido { mini: true, ..ABERTO });
        assert!(mini.so_visual);
        assert_eq!(mini.heroi, Ret::new(0.0, 0.0, 384.0, 231.0));
        assert_eq!(mini.visual, mini.heroi);
        assert_eq!(mini.titulo, SECAO);
        assert!(mini.painel.is_none() && mini.trilho.is_none());
        // Fora do mini, uma janela menor que o mínimo ainda é tratada como a mínima.
        assert_eq!(calcular(384.0, 231.0, ABERTO).janela.h, 400.0);
    }

    #[test]
    fn titulo_encolhe_em_janela_baixa() {
        assert_eq!(calcular(1280.0, 720.0, ABERTO).titulo, PAGINA);
        // Janela baixa: o dock perde 8 px.
        assert_eq!((calcular(958.0, 560.0, ABERTO).dock.h, calcular(958.0, 700.0, ABERTO).dock.h), (DOCK_BAIXO, DOCK));
        assert!(calcular(1280.0, 720.0, ABERTO).compacto);
    }
}
