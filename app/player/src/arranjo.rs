//! Arranjo responsivo: do tamanho da janela para o retângulo de cada região.
//!
//! É conta pura, então dá para testar todos os tamanhos sem abrir janela. As medidas são as do mockup v4
//! (`docs/design-ui-v1.md`): margem de 12, barra lateral de 72, topo de 52, dock de 80, painel de 380.
//!
//! | Largura da janela | Modo |
//! | --- | --- |
//! | acima de 1060 | `Largo`: barra lateral, visual, painel ao lado, dock |
//! | 761 a 1060 | `Gaveta`: o painel sai do lado e abre por cima, como gaveta |
//! | até 760 | `Empilhado`: tudo em coluna, com a navegação embaixo |

pub const MARGEM: f32 = 12.0;
pub const TRILHO: f32 = 72.0;
pub const TOPO: f32 = 52.0;
pub const DOCK: f32 = 80.0;
const DOCK_EMPILHADO: f32 = 116.0;
const NAV: f32 = 64.0;
const PAINEL: f32 = 380.0;
const PAINEL_ESTREITO: f32 = 340.0;
/// Largura da lateral da Estante (espaços, páginas e salvos).
pub const LATERAL: f32 = 248.0;
/// Largura do trilho "Tocando" da Estante (o visual no alto, os módulos embaixo).
pub const TOCANDO: f32 = 340.0;
/// Altura da faixa do visual no empilhado da Estante.
pub const FAIXA_DO_VISUAL: f32 = 72.0;
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
    /// O layout Estante (lateral, folha e trilho Tocando).
    pub estante: bool,
    /// Na Estante, quanto o visual já cresceu até a janela inteira (0 no trilho, 1 na janela).
    pub expansao: f32,
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
    /// Estante: a lateral com os espaços, as páginas e os salvos.
    pub lateral: Option<Ret>,
    /// Estante: o trilho Tocando inteiro (o visual no alto dele; os módulos em `painel` ou `gaveta`).
    pub tocando: Option<Ret>,
    /// Tamanho-base do título da faixa sobre o visual (64, 40 ou 28). O herói ainda encolhe nomes compridos.
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
    let modo = if w > 1060.0 {
        Modo::Largo
    } else if w > 760.0 {
        Modo::Gaveta
    } else {
        Modo::Empilhado
    };
    let compacto = h <= 780.0;

    if p.tela_cheia || p.mini {
        let titulo = if p.mini {
            28.0
        } else if w < 700.0 {
            40.0
        } else {
            64.0
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
            tocando: None,
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
        let dock = Ret::new(MARGEM, nav.y - 8.0 - DOCK_EMPILHADO, lw, DOCK_EMPILHADO);
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
            tocando: None,
            titulo: 28.0,
            compacto,
        };
    }

    let x0 = TRILHO + MARGEM;
    let conteudo_w = w - x0 - MARGEM;
    let trilho = Ret::new(0.0, 0.0, TRILHO, h);
    let topo = Ret::new(TRILHO, 0.0, w - TRILHO - MARGEM, TOPO);
    let dock = Ret::new(x0, h - MARGEM - DOCK, conteudo_w, DOCK);
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
    let titulo = if heroi.w >= 640.0 && !compacto && heroi.h >= 420.0 { 64.0 } else { 40.0 };
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
        tocando: None,
        titulo,
        compacto,
    }
}

/// O retângulo do visual crescendo até a janela (`e` de 0 a 1), com o raio sumindo.
fn expandir(visual: Ret, raio: f32, janela: Ret, e: f32) -> (Ret, f32) {
    let e = if e.is_finite() { e.clamp(0.0, 1.0) } else { 0.0 };
    if e >= 0.999 {
        return (janela, 0.0);
    }
    let entre = |a: f32, b: f32| a + (b - a) * e;
    let r = Ret::new(
        entre(visual.x, janela.x),
        entre(visual.y, janela.y),
        entre(visual.w, janela.w),
        entre(visual.h, janela.h),
    );
    (r, raio * (1.0 - e))
}

/// A altura do visual no alto do trilho Tocando: um retângulo 4:3, sem passar de 45% da altura.
fn altura_do_visual(largura: f32, altura: f32) -> f32 {
    (largura * 0.75).min(altura * 0.45).max(120.0)
}

/// O layout Estante. Largo: lateral, folha (o herói) e o trilho Tocando, com o dock sob a folha e o trilho. Gaveta:
/// a barra de ícones e o Tocando como gaveta (o visual só com ela aberta). Empilhado: a faixa do visual acima do
/// dock.
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
        tocando: None,
        titulo: 40.0,
        compacto,
    };
    let mut a = match modo {
        Modo::Largo => {
            let lateral = Ret::new(MARGEM, y0, LATERAL, (h - y0 - MARGEM).max(120.0));
            let x1 = lateral.x + lateral.w + MARGEM;
            let dock = Ret::new(x1, h - MARGEM - DOCK, w - MARGEM - x1, DOCK);
            let meio_h = (dock.y - MARGEM - y0).max(120.0);
            let tocando = Ret::new(w - MARGEM - TOCANDO, y0, TOCANDO, meio_h);
            let heroi = Ret::new(x1, y0, (tocando.x - MARGEM - x1).max(120.0), meio_h);
            let visual = Ret::new(tocando.x, y0, TOCANDO, altura_do_visual(TOCANDO, meio_h));
            let resto = tocando.base() - visual.base() - MARGEM;
            let painel = (resto >= 80.0).then(|| Ret::new(tocando.x, visual.base() + MARGEM, TOCANDO, resto));
            Arranjo { lateral: Some(lateral), tocando: Some(tocando), heroi, visual, painel, dock, ..base }
        }
        Modo::Gaveta => {
            let x0 = TRILHO + MARGEM;
            let conteudo_w = w - x0 - MARGEM;
            let dock = Ret::new(x0, h - MARGEM - DOCK, conteudo_w, DOCK);
            let meio_h = (dock.y - MARGEM - y0).max(120.0);
            let heroi = Ret::new(x0, y0, conteudo_w, meio_h);
            let topo = Ret::new(TRILHO, 0.0, w - TRILHO - MARGEM, TOPO);
            let trilho = Some(Ret::new(0.0, 0.0, TRILHO, h));
            if p.gaveta_aberta {
                let gw = TOCANDO.min(conteudo_w - 48.0);
                let tocando = Ret::new(w - MARGEM - gw, y0, gw, meio_h);
                let visual = Ret::new(tocando.x, y0, gw, altura_do_visual(gw, meio_h));
                let resto = tocando.base() - visual.base() - MARGEM;
                let gaveta = (resto >= 80.0).then(|| Ret::new(tocando.x, visual.base() + MARGEM, gw, resto));
                // A gaveta empurra a página (não abre por cima): o visual é desenhado por baixo da interface.
                let heroi = Ret::new(x0, y0, (tocando.x - MARGEM - x0).max(120.0), meio_h);
                Arranjo { trilho, topo, heroi, dock, tocando: Some(tocando), visual, gaveta, ..base }
            } else {
                Arranjo { trilho, topo, heroi, dock, ..base }
            }
        }
        Modo::Empilhado => {
            let lw = w - 2.0 * MARGEM;
            let nav = Ret::new(0.0, h - NAV, w, NAV);
            let dock = Ret::new(MARGEM, nav.y - 8.0 - DOCK_EMPILHADO, lw, DOCK_EMPILHADO);
            let faixa = Ret::new(MARGEM, dock.y - 8.0 - FAIXA_DO_VISUAL, lw, FAIXA_DO_VISUAL);
            // Janela baixa: sem lugar para a página e a faixa, a faixa sai (a página, opaca, a cobriria).
            let cabe = faixa.y - 8.0 - y0 >= 160.0;
            let (visual, fim) = if cabe { (faixa, faixa.y - 8.0) } else { (vazio, dock.y - 8.0) };
            let heroi = Ret::new(MARGEM, y0, lw, (fim - y0).max(120.0));
            Arranjo { heroi, visual, dock, nav: Some(nav), titulo: 28.0, raio_visual: 16.0, ..base }
        }
    };
    let (visual, raio) = expandir(a.visual, a.raio_visual, janela, p.expansao);
    a.visual = visual;
    a.raio_visual = raio;
    a
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
    };

    fn dentro(r: &Ret, j: &Ret) -> bool {
        r.x >= -0.01 && r.y >= -0.01 && r.direita() <= j.w + 0.01 && r.base() <= j.h + 0.01 && r.w > 0.0 && r.h > 0.0
    }

    fn cruza(a: &Ret, b: &Ret) -> bool {
        a.x < b.direita() - 0.01 && b.x < a.direita() - 0.01 && a.y < b.base() - 0.01 && b.y < a.base() - 0.01
    }

    const ESTANTE: Pedido = Pedido { estante: true, ..ABERTO };

    #[test]
    fn estante_larga_tem_lateral_folha_e_trilho() {
        let a = calcular(1440.0, 900.0, ESTANTE);
        assert_eq!(a.modo, Modo::Largo);
        let lateral = a.lateral.expect("lateral");
        let tocando = a.tocando.expect("trilho Tocando");
        let painel = a.painel.expect("módulos embaixo do visual");
        assert!(a.trilho.is_none(), "a lateral toma o lugar da barra de ícones");
        assert_eq!(lateral.w, LATERAL);
        assert_eq!(tocando.w, TOCANDO);
        for r in [&lateral, &a.heroi, &tocando, &a.dock, &a.topo] {
            assert!(dentro(r, &a.janela), "{r:?}");
        }
        assert!(!cruza(&lateral, &a.heroi) && !cruza(&a.heroi, &tocando) && !cruza(&lateral, &a.dock));
        assert!(!cruza(&a.heroi, &a.dock) && !cruza(&tocando, &a.dock));
        // O visual no alto do trilho, os módulos embaixo dele, dentro do trilho.
        assert_eq!((a.visual.x, a.visual.y, a.visual.w), (tocando.x, tocando.y, tocando.w));
        assert!(a.visual.h >= 180.0 && a.visual.h < tocando.h / 2.0, "{:?}", a.visual);
        assert!(!cruza(&a.visual, &painel) && painel.base() <= tocando.base() + 0.01 && painel.y >= a.visual.base());
        // O dock fica sob a folha e o trilho, não sob a lateral.
        assert_eq!(a.dock.x, a.heroi.x);
        assert!((a.dock.x + a.dock.w - (tocando.x + tocando.w)).abs() < 0.01);
        assert!(!a.so_visual);
    }

    #[test]
    fn estante_gaveta_visual_so_com_a_gaveta() {
        let fechada = calcular(1000.0, 800.0, ESTANTE);
        assert_eq!(fechada.modo, Modo::Gaveta);
        assert!(fechada.trilho.is_some() && fechada.lateral.is_none(), "o trilho de ícones");
        assert!(fechada.visual.w < 2.0, "fechada, o visual não aparece: {:?}", fechada.visual);
        let aberta = calcular(1000.0, 800.0, Pedido { gaveta_aberta: true, ..ESTANTE });
        let tocando = aberta.tocando.expect("o Tocando na gaveta");
        let modulos = aberta.gaveta.expect("os módulos na gaveta");
        assert_eq!((aberta.visual.x, aberta.visual.y, aberta.visual.w), (tocando.x, tocando.y, tocando.w));
        assert!(aberta.visual.h >= 160.0);
        assert!(modulos.y >= aberta.visual.base() && modulos.base() <= tocando.base() + 0.01);
        // A gaveta empurra a página: por cima da folha o visual (desenhado por baixo da interface) não apareceria.
        assert!(!cruza(&aberta.heroi, &tocando), "{:?} {:?}", aberta.heroi, tocando);
        assert!(aberta.heroi.w < fechada.heroi.w);
    }

    #[test]
    fn estante_empilhada_visual_acima_do_dock() {
        let a = calcular(600.0, 900.0, ESTANTE);
        assert_eq!(a.modo, Modo::Empilhado);
        assert!(a.painel.is_none() && a.lateral.is_none());
        assert_eq!(a.visual.h, FAIXA_DO_VISUAL);
        assert!(a.visual.base() <= a.dock.y && !cruza(&a.visual, &a.heroi) && !cruza(&a.visual, &a.dock));
        assert!(a.heroi.h > 300.0, "a página fica com o resto: {:?}", a.heroi);
        assert!(a.nav.is_some());
        // Janela baixa: a faixa sai, para a página (opaca) não cobrir o visual.
        let baixa = calcular(600.0, 420.0, ESTANTE);
        assert!(baixa.visual.w < 2.0 || !cruza(&baixa.visual, &baixa.heroi), "{:?} {:?}", baixa.visual, baixa.heroi);
    }

    #[test]
    fn expandir_leva_o_visual_a_janela_inteira() {
        let base = calcular(1440.0, 900.0, ESTANTE);
        let meio = calcular(1440.0, 900.0, Pedido { expansao: 0.5, ..ESTANTE });
        let fim = calcular(1440.0, 900.0, Pedido { expansao: 1.0, ..ESTANTE });
        assert!(meio.visual.w > base.visual.w && meio.visual.w < 1440.0);
        assert!(meio.raio_visual < base.raio_visual);
        assert_eq!(fim.visual, fim.janela);
        assert_eq!(fim.raio_visual, 0.0);
        assert_eq!(meio.heroi, base.heroi, "as outras regiões ficam onde estão (só esmaecem)");
        // O empilhado também expande da faixa.
        let e = calcular(600.0, 900.0, Pedido { expansao: 1.0, ..ESTANTE });
        assert_eq!(e.visual, e.janela);
    }

    #[test]
    fn medidas_do_mockup_em_1440x900() {
        let a = calcular(1440.0, 900.0, ABERTO);
        assert_eq!(a.modo, Modo::Largo);
        assert_eq!(a.heroi, Ret::new(84.0, 64.0, 952.0, 732.0));
        assert_eq!(a.painel, Some(Ret::new(1048.0, 64.0, 380.0, 732.0)));
        assert_eq!(a.dock, Ret::new(84.0, 808.0, 1344.0, 80.0));
        assert_eq!(a.titulo, 64.0);
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
        assert_eq!(a.titulo, 28.0);
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
        assert_eq!(esquerda.painel, Some(Ret::new(84.0, 64.0, 380.0, 732.0)));
        assert_eq!(esquerda.heroi, Ret::new(476.0, 64.0, 952.0, 732.0));
        assert_eq!(esquerda.heroi.w, direita.heroi.w);
        let gaveta = calcular(1024.0, 700.0, Pedido { painel_a_esquerda: true, gaveta_aberta: true, ..ABERTO });
        assert!(gaveta.gaveta.is_some_and(|g| g.x == 84.0));
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
        assert_eq!(mini.titulo, 28.0);
        assert!(mini.painel.is_none() && mini.trilho.is_none());
        // Fora do mini, uma janela menor que o mínimo ainda é tratada como a mínima.
        assert_eq!(calcular(384.0, 231.0, ABERTO).janela.h, 400.0);
    }

    #[test]
    fn titulo_encolhe_em_janela_baixa() {
        assert_eq!(calcular(1280.0, 720.0, ABERTO).titulo, 40.0);
        assert!(calcular(1280.0, 720.0, ABERTO).compacto);
    }
}
