//! Onde a Ilha fica e que forma ela tem lá: presa a uma borda da tela (no topo, embaixo, à esquerda, à direita) ou
//! solta; o arrasto livre com ímã nas bordas; a janela que a contém; as caixas do desenho e a região que recebe
//! cliques. Só contas, em pixels lógicos, testadas sem janela.

use super::{ABERTA, Alerta, Forma, Medidas, medidas};
use nucleo::{BordaIlha as Borda, TamanhoIlha};

/// Um retângulo em pixels lógicos.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Caixa {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Caixa {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn direita(&self) -> f32 {
        self.x + self.w
    }

    pub fn base(&self) -> f32 {
        self.y + self.h
    }

    pub fn centro(&self) -> (f32, f32) {
        (self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    /// A mesma caixa vista de outra origem (da tela para a janela, por exemplo).
    pub fn menos(&self, origem: (f32, f32)) -> Self {
        Self { x: self.x - origem.0, y: self.y - origem.1, ..*self }
    }

    /// O ponto `p` está dentro, com uma `folga` em volta.
    pub fn contem(&self, p: (f32, f32), folga: f32) -> bool {
        p.0 >= self.x - folga && p.0 <= self.direita() + folga && p.1 >= self.y - folga && p.1 <= self.base() + folga
    }
}

/// Como a forma está: presa a uma borda (os cantos do lado dela ficam retos, com os ombros) ou solta (cantos todos
/// redondos, sem ombros).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Modo {
    Presa(Borda),
    Solta,
}

impl Modo {
    /// Presa a um lado, a Ilha fica em pé: uma barra lateral, com o conteúdo arrumado na vertical.
    pub fn de_pe(self) -> bool {
        matches!(self, Self::Presa(b) if b.de_pe())
    }

    pub fn borda(self) -> Option<Borda> {
        match self {
            Self::Presa(b) => Some(b),
            Self::Solta => None,
        }
    }
}

/// Onde a Ilha fica: presa a uma borda, numa posição ao longo dela (0 a 1), ou solta, com o centro numa fração da área
/// de trabalho (0 a 1 em cada eixo).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Lugar {
    Presa(Borda, f32),
    Solta(f32, f32),
}

impl Lugar {
    pub fn modo(self) -> Modo {
        match self {
            Self::Presa(b, _) => Modo::Presa(b),
            Self::Solta(..) => Modo::Solta,
        }
    }
}

/// Perto assim de uma borda (pixels lógicos), o arrasto gruda nela: o ímã.
pub const IMA: f32 = 56.0;
/// Presa, a Ilha só solta da borda quando o ponteiro se afasta isto dela (sem tremer na beira do ímã).
pub const SOLTAR: f32 = 104.0;
/// A barra lateral aberta (presa a um lado), na escala 1: largura, altura, raio e ombro.
const LATERAL: (f32, f32, f32, f32) = (340.0, 560.0, 36.0, 24.0);
/// Folga do quique em volta da maior forma, de cada lado (a janela não pode cortar a forma quando ela passa do tamanho).
const FOLGA_DO_QUIQUE: f32 = 16.0;
/// Lado da janela (quadrada) na escala 1: cabe a maior forma deitada (a aberta, com os ombros e a folga do quique) e a
/// maior em pé (a barra lateral aberta). Sendo a mesma em todas as bordas, a janela não muda de tamanho quando a
/// Ilha troca de borda (mudar de tamanho desenhava um quadro no tamanho velho, um tranco).
pub const LADO_DA_JANELA: f32 = ABERTA.0 + 2.0 * (ABERTA.3 + FOLGA_DO_QUIQUE);
/// Folga da região da janela em volta do alvo enquanto a forma anda (cabe o quique de qualquer tamanho), na escala 1.
pub const FOLGA_DA_REGIAO: f32 = 16.0;

fn finito(v: f32, outro: f32) -> f32 {
    if v.is_finite() { v } else { outro }
}

/// `v` entre `menor` e `maior`; se não couber (`maior` < `menor`), o meio dos dois.
fn entre(v: f32, menor: f32, maior: f32) -> f32 {
    if maior < menor { (menor + maior) / 2.0 } else { finito(v, menor).clamp(menor, maior) }
}

/// Onde a Ilha fica durante o arrasto. `ponteiro` decide o ímã: perto de uma borda, gruda nela; num canto, ganha a
/// borda mais perto. `centro` (o ponteiro menos o ponto em que ele pegou a Ilha) decide a posição. Os dois relativos à
/// área de trabalho, de tamanho `area`. Presa, só solta depois de o ponteiro se afastar [`SOLTAR`] da borda.
pub fn lugar_no_arrasto(ponteiro: (f32, f32), centro: (f32, f32), area: (f32, f32), anterior: Option<Lugar>) -> Lugar {
    let (aw, ah) = (finito(area.0, 1.0).max(1.0), finito(area.1, 1.0).max(1.0));
    let (x, y) = (finito(ponteiro.0, aw / 2.0).clamp(0.0, aw), finito(ponteiro.1, ah / 2.0).clamp(0.0, ah));
    let (cx, cy) = (finito(centro.0, x).clamp(0.0, aw), finito(centro.1, y).clamp(0.0, ah));
    let distancia = |b: Borda| match b {
        Borda::Topo => y,
        Borda::Baixo => ah - y,
        Borda::Esquerda => x,
        Borda::Direita => aw - x,
    };
    let ao_longo = |b: Borda| if b.de_pe() { cy / ah } else { cx / aw };
    if let Some(Lugar::Presa(b, _)) = anterior
        && distancia(b) < SOLTAR
    {
        return Lugar::Presa(b, ao_longo(b));
    }
    let perto =
        Borda::TODAS.into_iter().filter(|b| distancia(*b) < IMA).min_by(|a, b| distancia(*a).total_cmp(&distancia(*b)));
    match perto {
        Some(b) => Lugar::Presa(b, ao_longo(b)),
        None => Lugar::Solta(cx / aw, cy / ah),
    }
}

/// Onde a Ilha travada fica durante o arrasto: na mesma `borda`, na posição do `centro` ao longo dela (relativo à
/// área de trabalho, de tamanho `area`). Travada, arrastar continua livre, mas ela só desliza pela borda.
pub fn lugar_travado(centro: (f32, f32), area: (f32, f32), borda: Borda) -> Lugar {
    let (aw, ah) = (finito(area.0, 1.0).max(1.0), finito(area.1, 1.0).max(1.0));
    let p = if borda.de_pe() {
        finito(centro.1, ah / 2.0).clamp(0.0, ah) / ah
    } else {
        finito(centro.0, aw / 2.0).clamp(0.0, aw) / aw
    };
    Lugar::Presa(borda, p)
}

/// As medidas da forma no modo (largura e altura na tela, raio dos cantos, tamanho dos ombros). Deitada (no topo,
/// embaixo ou solta) são as de sempre; solta, sem ombros. Em pé (dos lados), o repouso é uma aba estreita e a aberta,
/// uma barra lateral; os avisos continuam deitados (um cartão que sai da borda).
pub fn medidas_no_modo(modo: Modo, forma: Forma, alerta: Option<Alerta>, escala: f32, minima: bool) -> Medidas {
    let m = |w: f32, h: f32, r: f32, e: f32| Medidas {
        largura: w * escala,
        altura: h * escala,
        raio: r * escala,
        ombro: e * escala,
    };
    match modo {
        Modo::Presa(b) if b.de_pe() => match (forma, alerta) {
            (Forma::Expandida, _) => m(LATERAL.0, LATERAL.1, LATERAL.2, LATERAL.3),
            (Forma::Alerta, Some(_)) => medidas(forma, alerta, escala, minima),
            _ if minima => m(32.0, 76.0, 14.0, 10.0),
            _ => m(44.0, 112.0, 17.0, 12.0),
        },
        Modo::Solta => Medidas { ombro: 0.0, ..medidas(forma, alerta, escala, minima) },
        Modo::Presa(_) => medidas(forma, alerta, escala, minima),
    }
}

/// Os raios dos quatro cantos (cima-esquerda, cima-direita, baixo-direita, baixo-esquerda) com os do lado da borda
/// `lado` valendo `perto` e os outros, `longe`. Presa, os de perto ficam retos (no lugar deles vão os ombros); solta,
/// todos redondos. Sem borda (sempre solta), os de cima contam como os de perto.
pub fn raios(lado: Option<Borda>, perto: f32, longe: f32) -> [f32; 4] {
    match lado {
        None | Some(Borda::Topo) => [perto, perto, longe, longe],
        Some(Borda::Baixo) => [longe, longe, perto, perto],
        Some(Borda::Esquerda) => [perto, longe, longe, perto],
        Some(Borda::Direita) => [longe, perto, perto, longe],
    }
}

/// A caixa da forma de tamanho `tam` (com ombros de `ombro`) no `lugar`, dentro da área de trabalho `area` (na tela).
/// A forma não sai da área: perto das pontas da borda (ou, solta, perto de qualquer borda), ela para na beira.
pub fn caixa_no_lugar(lugar: Lugar, tam: (f32, f32), ombro: f32, area: Caixa) -> Caixa {
    let (w, h) = (finito(tam.0, 0.0).max(0.0), finito(tam.1, 0.0).max(0.0));
    let e = finito(ombro, 0.0).max(0.0);
    match lugar {
        Lugar::Presa(b, p) => {
            let p = finito(p, 0.5).clamp(0.0, 1.0);
            if b.de_pe() {
                let cy = entre(area.y + p * area.h, area.y + h / 2.0 + e, area.base() - h / 2.0 - e);
                let x = if b == Borda::Esquerda { area.x } else { area.direita() - w };
                Caixa::new(x, cy - h / 2.0, w, h)
            } else {
                let cx = entre(area.x + p * area.w, area.x + w / 2.0 + e, area.direita() - w / 2.0 - e);
                let y = if b == Borda::Topo { area.y } else { area.base() - h };
                Caixa::new(cx - w / 2.0, y, w, h)
            }
        }
        Lugar::Solta(fx, fy) => {
            let cx = entre(area.x + finito(fx, 0.5) * area.w, area.x + w / 2.0, area.direita() - w / 2.0);
            let cy = entre(area.y + finito(fy, 0.5) * area.h, area.y + h / 2.0, area.base() - h / 2.0);
            Caixa::new(cx - w / 2.0, cy - h / 2.0, w, h)
        }
    }
}

/// O ponto que segura a forma no modo: presa, o meio do lado colado na borda; solta, o centro. Mudando de tamanho, a
/// forma cresce em volta da âncora (presa, para dentro da tela).
pub fn ancora(modo: Modo, c: Caixa) -> (f32, f32) {
    match modo {
        Modo::Presa(Borda::Topo) => (c.x + c.w / 2.0, c.y),
        Modo::Presa(Borda::Baixo) => (c.x + c.w / 2.0, c.base()),
        Modo::Presa(Borda::Esquerda) => (c.x, c.y + c.h / 2.0),
        Modo::Presa(Borda::Direita) => (c.direita(), c.y + c.h / 2.0),
        Modo::Solta => c.centro(),
    }
}

/// A caixa de tamanho `tam` segura pela âncora `a` no modo (o contrário de [`ancora`]).
pub fn caixa_da_ancora(modo: Modo, a: (f32, f32), tam: (f32, f32)) -> Caixa {
    let (w, h) = tam;
    match modo {
        Modo::Presa(Borda::Topo) => Caixa::new(a.0 - w / 2.0, a.1, w, h),
        Modo::Presa(Borda::Baixo) => Caixa::new(a.0 - w / 2.0, a.1 - h, w, h),
        Modo::Presa(Borda::Esquerda) => Caixa::new(a.0, a.1 - h / 2.0, w, h),
        Modo::Presa(Borda::Direita) => Caixa::new(a.0 - w, a.1 - h / 2.0, w, h),
        Modo::Solta => Caixa::new(a.0 - w / 2.0, a.1 - h / 2.0, w, h),
    }
}

/// A caixa com os ombros, que saem da forma ao longo da borda a que ela está presa.
pub fn caixa_com_ombros(c: Caixa, lado: Option<Borda>, ombro: f32) -> Caixa {
    let e = finito(ombro, 0.0).max(0.0);
    match lado {
        Some(b) if b.de_pe() => Caixa::new(c.x, c.y - e, c.w, c.h + 2.0 * e),
        Some(_) => Caixa::new(c.x - e, c.y, c.w + 2.0 * e, c.h),
        None => c,
    }
}

/// A maior escala da Ilha que cabe no monitor (área de trabalho e monitor em pixels lógicos): a janela quadrada não
/// passa do monitor, e nem a forma aberta deitada nem a barra lateral passam da área de trabalho. Numa tela pequena
/// (ou com o zoom do Windows alto), a Ilha encolhe para caber, em vez de sair da tela.
pub fn escala_que_cabe(area: (f32, f32), monitor: (f32, f32)) -> f32 {
    let lado = area.0.min(area.1).min(monitor.0).min(monitor.1);
    let e = if lado.is_finite() && lado > 0.0 { lado / LADO_DA_JANELA } else { 1.0 };
    e.clamp(0.5, TamanhoIlha::Grande.escala())
}

/// O tamanho da janela (quadrada) na escala máxima `escala` (ver [`escala_que_cabe`]).
pub fn tamanho_da_janela(escala: f32) -> (f32, f32) {
    let lado = LADO_DA_JANELA * finito(escala, 1.0).clamp(0.5, TamanhoIlha::Grande.escala());
    (lado, lado)
}

/// Onde pôr a janela de tamanho `janela` (o canto de cima à esquerda, na tela) para conter a forma `c`: centrada nela,
/// sem sair do monitor. Presa a uma borda, isso encosta a janela na borda; então ela fica parada enquanto a forma muda
/// de tamanho, e só anda quando a forma anda (arrastando, ou indo para outra borda).
pub fn janela_para(c: Caixa, janela: (f32, f32), monitor: Caixa) -> (f32, f32) {
    let (cx, cy) = c.centro();
    (
        entre(cx - janela.0 / 2.0, monitor.x, monitor.direita() - janela.0),
        entre(cy - janela.1 / 2.0, monitor.y, monitor.base() - janela.1),
    )
}

/// As caixas do corpo da Ilha neste quadro, na janela, em pixels inteiros. O GPUI arredonda a posição de cada elemento
/// em relação ao pai: com o corpo numa posição fracionária (a forma mudando de tamanho), o conteúdo, parado na tela,
/// pulava 1 px de um quadro para o outro (a "tremida"). Com tudo inteiro, o conteúdo fica parado.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caixas {
    /// O corpo (recebe cliques), na janela.
    pub corpo: Caixa,
    /// O recorte do conteúdo, dentro do corpo (relativo a ele). Fica longe dos cantos arredondados, para o conteúdo
    /// nunca aparecer fora da forma enquanto ela muda de tamanho.
    pub recorte: Caixa,
    /// Onde o conteúdo começa dentro do recorte (na janela, ele fica sempre no mesmo lugar).
    pub conteudo: (f32, f32),
    /// Onde o conteúdo começa na janela.
    pub conteudo_na_janela: (f32, f32),
}

/// As [`Caixas`] do corpo `corpo` (com os cantos de `raios`) e do conteúdo que começa em `conteudo`, tudo na janela.
pub fn caixas(corpo: Caixa, raios: [f32; 4], conteudo: (f32, f32)) -> Caixas {
    let (x0, y0) = (finito(corpo.x, 0.0).round(), finito(corpo.y, 0.0).round());
    let (x1, y1) = (finito(corpo.direita(), x0).round().max(x0), finito(corpo.base(), y0).round().max(y0));
    let (w, h) = (x1 - x0, y1 - y0);
    let maximo = (w / 2.0).min(h / 2.0).max(0.0);
    let r = raios.map(|v| finito(v, 0.0).clamp(0.0, maximo));
    // Um retângulo recuado 0,3 do raio nos dois lados de um canto redondo tem o canto dentro do arco (1 - 1/√2 ≈ 0,29).
    let recuo = |a: f32, b: f32| (a.max(b) * 0.3).ceil();
    let (esquerda, cima, direita, baixo) = (recuo(r[0], r[3]), recuo(r[0], r[1]), recuo(r[1], r[2]), recuo(r[3], r[2]));
    let conteudo_na_janela = (finito(conteudo.0, x0).round(), finito(conteudo.1, y0).round());
    Caixas {
        corpo: Caixa::new(x0, y0, w, h),
        recorte: Caixa::new(esquerda, cima, (w - esquerda - direita).max(0.0), (h - cima - baixo).max(0.0)),
        conteudo: (conteudo_na_janela.0 - x0 - esquerda, conteudo_na_janela.1 - y0 - cima),
        conteudo_na_janela,
    }
}

/// Uma região da janela: esquerda, topo, direita e baixo, em pixels lógicos.
pub type Regiao = (i32, i32, i32, i32);

/// A menor região que cobre as duas; uma região vazia não conta.
pub fn unir_regioes(a: Regiao, b: Regiao) -> Regiao {
    let vazia = |r: Regiao| r.2 <= r.0 || r.3 <= r.1;
    match (vazia(a), vazia(b)) {
        (_, true) => a,
        (true, false) => b,
        (false, false) => (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)),
    }
}

/// A região da janela (o que aparece e recebe cliques) neste quadro, para a forma com a caixa `caixa` (com os ombros)
/// indo para `caixa_do_alvo`. Parada, a região é a caixa da forma (com 1 px da borda suavizada). Andando, ela cobre a
/// de antes, a caixa de agora e a do alvo com a `folga` do quique, e só cresce: trocar a região a cada quadro (a forma
/// passa do alvo no quique) fazia a Ilha piscar.
pub fn regiao_do_quadro(assentou: bool, caixa: Caixa, caixa_do_alvo: Caixa, folga: f32, anterior: Regiao) -> Regiao {
    let com_folga = |c: Caixa, f: f32| {
        (
            (c.x - f).floor() as i32,
            (c.y - f).floor() as i32,
            (c.direita() + f).ceil() as i32,
            (c.base() + f).ceil() as i32,
        )
    };
    if assentou {
        return com_folga(caixa, 1.0);
    }
    unir_regioes(unir_regioes(com_folga(caixa_do_alvo, folga), com_folga(caixa, 1.0)), anterior)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::ilha::{MOLA_CRESCER, MOLA_ENCOLHER};

    const AREA: (f32, f32) = (1920.0, 1040.0);
    const MODOS: [Modo; 5] = [
        Modo::Presa(Borda::Topo),
        Modo::Presa(Borda::Baixo),
        Modo::Presa(Borda::Esquerda),
        Modo::Presa(Borda::Direita),
        Modo::Solta,
    ];

    fn presa(l: Lugar) -> Option<Borda> {
        l.modo().borda()
    }

    #[test]
    fn ima_gruda_na_borda_mais_perto() {
        let no = |x: f32, y: f32| lugar_no_arrasto((x, y), (x, y), AREA, None);
        assert_eq!(presa(no(10.0, 500.0)), Some(Borda::Esquerda));
        assert_eq!(presa(no(1915.0, 500.0)), Some(Borda::Direita));
        assert_eq!(presa(no(900.0, 5.0)), Some(Borda::Topo));
        assert_eq!(presa(no(900.0, 1035.0)), Some(Borda::Baixo));
        assert_eq!(presa(no(20.0, 30.0)), Some(Borda::Esquerda), "no canto ganha a borda mais perto");
        assert_eq!(presa(no(30.0, 20.0)), Some(Borda::Topo));
        assert_eq!(no(960.0, 520.0), Lugar::Solta(0.5, 0.5), "longe das bordas fica solta");
        // A posição ao longo da borda vem do centro da Ilha.
        let l = lugar_no_arrasto((5.0, 300.0), (30.0, 260.0), AREA, None);
        assert!(matches!(l, Lugar::Presa(Borda::Esquerda, p) if (p - 0.25).abs() < 1e-6), "{l:?}");
    }

    #[test]
    fn presa_so_solta_longe_da_borda() {
        let antes = Some(Lugar::Presa(Borda::Esquerda, 0.5));
        assert_eq!(presa(lugar_no_arrasto((80.0, 500.0), (80.0, 500.0), AREA, antes)), Some(Borda::Esquerda));
        assert_eq!(
            presa(lugar_no_arrasto((80.0, 500.0), (80.0, 500.0), AREA, None)),
            None,
            "sem estar presa, 80 px é longe"
        );
        assert_eq!(presa(lugar_no_arrasto((120.0, 500.0), (120.0, 500.0), AREA, antes)), None, "longe, solta");
    }

    #[test]
    fn travada_so_desliza_pela_borda() {
        // Longe de tudo, no meio da tela: travada, continua no topo, só que mais para a direita.
        assert_eq!(lugar_travado((1440.0, 520.0), AREA, Borda::Topo), Lugar::Presa(Borda::Topo, 0.75));
        // Perto de outra borda: não gruda nela.
        assert_eq!(lugar_travado((5.0, 260.0), AREA, Borda::Direita), Lugar::Presa(Borda::Direita, 0.25));
        // Fora da tela ou estragado: fica na ponta, ou no meio.
        assert_eq!(lugar_travado((-300.0, 0.0), AREA, Borda::Baixo), Lugar::Presa(Borda::Baixo, 0.0));
        assert_eq!(lugar_travado((f32::NAN, 0.0), AREA, Borda::Topo), Lugar::Presa(Borda::Topo, 0.5));
        assert_eq!(lugar_travado((10.0, 10.0), (0.0, f32::NAN), Borda::Esquerda), Lugar::Presa(Borda::Esquerda, 1.0));
    }

    #[test]
    fn numeros_estragados_nao_quebram() {
        for p in [(f32::NAN, 1.0), (f32::INFINITY, f32::NEG_INFINITY), (-500.0, 9e9)] {
            for area in [AREA, (0.0, 0.0), (f32::NAN, 10.0)] {
                let l = lugar_no_arrasto(p, p, area, None);
                if let Lugar::Solta(x, y) = l {
                    assert!(x.is_finite() && y.is_finite());
                }
                let c = caixa_no_lugar(l, (208.0, 36.0), 12.0, Caixa::new(0.0, 0.0, AREA.0, AREA.1));
                assert!(c.x.is_finite() && c.y.is_finite());
            }
        }
        let c = caixas(Caixa::new(f32::NAN, 0.0, 10.0, f32::INFINITY), [f32::NAN; 4], (f32::NAN, 1.0));
        assert!(c.corpo.x.is_finite() && c.recorte.w.is_finite() && c.conteudo.0.is_finite());
    }

    #[test]
    fn a_forma_fica_dentro_da_area_e_presa_encosta_na_borda() {
        let area = Caixa::new(0.0, 0.0, AREA.0, AREA.1);
        for borda in Borda::TODAS {
            for p in [0.0, 0.3, 0.5, 1.0] {
                for forma in [Forma::Repouso, Forma::Expandida] {
                    let modo = Modo::Presa(borda);
                    let m = medidas_no_modo(modo, forma, None, 1.22, false);
                    let c = caixa_no_lugar(Lugar::Presa(borda, p), (m.largura, m.altura), m.ombro, area);
                    let com = caixa_com_ombros(c, Some(borda), m.ombro);
                    assert!(com.x >= -0.01 && com.y >= -0.01, "{borda:?} {p}: {com:?}");
                    assert!(com.direita() <= area.w + 0.01 && com.base() <= area.h + 0.01, "{borda:?} {p}: {com:?}");
                    let encosta = match borda {
                        Borda::Topo => c.y,
                        Borda::Baixo => area.h - c.base(),
                        Borda::Esquerda => c.x,
                        Borda::Direita => area.w - c.direita(),
                    };
                    assert!(encosta.abs() < 0.01, "{borda:?}: encostada na borda");
                }
            }
        }
        for (fx, fy) in [(0.0, 0.0), (1.0, 1.0), (0.5, 0.5)] {
            let m = medidas_no_modo(Modo::Solta, Forma::Expandida, None, 1.22, false);
            let c = caixa_no_lugar(Lugar::Solta(fx, fy), (m.largura, m.altura), 0.0, area);
            assert!(c.x >= 0.0 && c.y >= 0.0 && c.direita() <= area.w + 0.01 && c.base() <= area.h + 0.01);
        }
    }

    #[test]
    fn ancora_vai_e_volta() {
        let c = Caixa::new(100.0, 40.0, 340.0, 560.0);
        for modo in MODOS {
            assert_eq!(caixa_da_ancora(modo, ancora(modo, c), (c.w, c.h)), c, "{modo:?}");
        }
    }

    #[test]
    fn presa_a_forma_cresce_para_dentro_da_tela() {
        // Do repouso à aberta, a âncora não anda: no topo, o lado de cima fica na borda; à direita, o lado direito.
        for borda in Borda::TODAS {
            let modo = Modo::Presa(borda);
            let area = Caixa::new(0.0, 0.0, AREA.0, AREA.1);
            let r = medidas_no_modo(modo, Forma::Repouso, None, 1.0, false);
            let c = caixa_no_lugar(Lugar::Presa(borda, 0.5), (r.largura, r.altura), r.ombro, area);
            let a = medidas_no_modo(modo, Forma::Expandida, None, 1.0, false);
            let aberta = caixa_da_ancora(modo, ancora(modo, c), (a.largura, a.altura));
            assert!(aberta.x >= -0.01 && aberta.y >= -0.01, "{borda:?}");
            assert!(aberta.direita() <= area.w + 0.01 && aberta.base() <= area.h + 0.01, "{borda:?}");
        }
    }

    #[test]
    fn a_janela_contem_a_forma_e_fica_parada_quando_ela_cresce() {
        let monitor = Caixa::new(0.0, 0.0, 1920.0, 1080.0);
        let area = Caixa::new(0.0, 0.0, AREA.0, AREA.1);
        let janela = tamanho_da_janela(escala_que_cabe(AREA, (1920.0, 1080.0)));
        for modo in MODOS {
            for p in [0.0, 0.5, 1.0] {
                let lugar = match modo {
                    Modo::Presa(b) => Lugar::Presa(b, p),
                    Modo::Solta => Lugar::Solta(p, p),
                };
                let mut origens = Vec::new();
                for forma in [Forma::Repouso, Forma::Expandida] {
                    let m = medidas_no_modo(modo, forma, None, 1.22, false);
                    let c = caixa_no_lugar(lugar, (m.largura, m.altura), m.ombro, area);
                    let com = caixa_com_ombros(c, modo.borda(), m.ombro);
                    let o = janela_para(c, janela, monitor);
                    assert!(o.0 >= -0.01 && o.1 >= -0.01, "{modo:?} {p}: a janela sai do monitor");
                    assert!(o.0 + janela.0 <= monitor.w + 0.01 && o.1 + janela.1 <= monitor.h + 0.01);
                    assert!(
                        com.x >= o.0 - 0.01 && com.direita() <= o.0 + janela.0 + 0.01,
                        "{modo:?} {p}: {com:?} {o:?}"
                    );
                    assert!(com.y >= o.1 - 0.01 && com.base() <= o.1 + janela.1 + 0.01, "{modo:?} {p}: {com:?} {o:?}");
                    origens.push(o);
                }
                if let Modo::Presa(_) = modo
                    && p == 0.5
                {
                    assert_eq!(origens[0], origens[1], "{modo:?}: presa, a janela não anda quando a forma cresce");
                }
            }
        }
    }

    #[test]
    fn numa_tela_pequena_a_ilha_encolhe_para_caber() {
        assert_eq!(escala_que_cabe((1920.0, 1040.0), (1920.0, 1080.0)), TamanhoIlha::Grande.escala());
        let e = escala_que_cabe((1366.0, 728.0), (1366.0, 768.0));
        assert!(e < 1.22 && e > 1.0, "{e}");
        let pequena = escala_que_cabe((800.0, 560.0), (800.0, 600.0));
        let (w, h) = tamanho_da_janela(pequena);
        assert!(w <= 600.0 && h <= 600.0);
        assert_eq!(escala_que_cabe((100.0, 100.0), (100.0, 100.0)), 0.5, "nunca some de tão pequena");
        assert_eq!(escala_que_cabe((f32::NAN, 0.0), (0.0, 0.0)), 1.0);
    }

    #[test]
    fn o_quique_cabe_na_janela_em_todos_os_modos() {
        // A forma cresce com quique do repouso (e do risco de quando aparece) até aberta: a maior medida, com os
        // ombros, tem que caber na janela centrada na forma, senão a janela corta a forma nesse instante.
        for escala in [0.86, 1.0, 1.22] {
            let (lado, _) = tamanho_da_janela(escala);
            for modo in MODOS {
                let aberta = medidas_no_modo(modo, Forma::Expandida, None, escala, false);
                let repouso = medidas_no_modo(modo, Forma::Repouso, None, escala, true);
                let risco = Medidas { largura: 90.0 * escala, altura: 2.0, raio: 1.0, ombro: 1.0 };
                for inicio in [repouso, risco] {
                    let a = crate::anim::Animacoes::default();
                    a.quadro(1.0 / 60.0, false);
                    a.definir_mola(("w", 0), inicio.largura);
                    a.definir_mola(("h", 0), inicio.altura);
                    let (mut w, mut h) = (0.0f32, 0.0f32);
                    for _ in 0..240 {
                        w = w.max(a.mola_amortecida(("w", 0), aberta.largura, MOLA_CRESCER.0, MOLA_CRESCER.1));
                        h = h.max(a.mola_amortecida(("h", 0), aberta.altura, MOLA_CRESCER.0, MOLA_CRESCER.1));
                    }
                    let c = caixa_com_ombros(Caixa::new(0.0, 0.0, w, h), modo.borda(), aberta.ombro);
                    assert!(c.w <= lado && c.h <= lado, "{modo:?} {escala}: {c:?} não cabe em {lado}");
                }
            }
        }
    }

    #[test]
    fn conteudo_fica_parado_enquanto_a_forma_muda() {
        // A forma cresce de quadro em quadro com medidas quebradas; o conteúdo (do tamanho do alvo, segurado pela
        // âncora) não pode andar, e todas as caixas ficam em pixels inteiros.
        for modo in MODOS {
            let alvo = medidas_no_modo(modo, Forma::Expandida, None, 1.22, false);
            let a = ancora(modo, Caixa::new(300.3, 120.7, alvo.largura, alvo.altura));
            let conteudo = caixa_da_ancora(modo, a, (alvo.largura, alvo.altura));
            let mut visto = None;
            for i in 0..240 {
                let k = i as f32 / 239.0;
                let tam = (40.3 + (alvo.largura + 14.7 - 40.3) * k, 30.6 + (alvo.altura + 6.1 - 30.6) * k);
                let corpo = caixa_da_ancora(modo, a, tam);
                let r = 14.2 + (alvo.raio - 14.2) * k;
                let c = caixas(
                    corpo,
                    raios(modo.borda(), if modo == Modo::Solta { r } else { 0.0 }, r),
                    (conteudo.x, conteudo.y),
                );
                for v in
                    [c.corpo.x, c.corpo.y, c.corpo.w, c.corpo.h, c.recorte.x, c.recorte.y, c.conteudo.0, c.conteudo.1]
                {
                    assert_eq!(v, v.round(), "{modo:?}: {v} não é inteiro");
                }
                let x = (c.corpo.x + c.recorte.x + c.conteudo.0, c.corpo.y + c.recorte.y + c.conteudo.1);
                assert_eq!(x, c.conteudo_na_janela);
                assert_eq!(x, *visto.get_or_insert(x), "{modo:?}: o conteúdo andou no quadro {i}");
            }
        }
    }

    #[test]
    fn recorte_fica_dentro_dos_cantos_arredondados() {
        // Os cantos do recorte do conteúdo ficam dentro do arco de cada canto redondo (com 1 px de folga pelo
        // arredondamento), então o conteúdo nunca aparece fora da forma.
        for modo in MODOS {
            for (w, h, r) in [(208.0, 36.0, 17.0), (560.0, 228.0, 36.0), (44.0, 112.0, 17.0), (300.3, 120.7, 30.1)] {
                let corpo = Caixa::new(50.0, 40.0, w, h);
                let rs = raios(modo.borda(), if modo == Modo::Solta { r } else { 0.0 }, r);
                let c = caixas(corpo, rs, (corpo.x, corpo.y));
                let maximo = (c.corpo.w / 2.0).min(c.corpo.h / 2.0);
                let recorte = Caixa::new(c.corpo.x + c.recorte.x, c.corpo.y + c.recorte.y, c.recorte.w, c.recorte.h);
                let cantos = [
                    ((recorte.x, recorte.y), (c.corpo.x, c.corpo.y), (1.0, 1.0)),
                    ((recorte.direita(), recorte.y), (c.corpo.direita(), c.corpo.y), (-1.0, 1.0)),
                    ((recorte.direita(), recorte.base()), (c.corpo.direita(), c.corpo.base()), (-1.0, -1.0)),
                    ((recorte.x, recorte.base()), (c.corpo.x, c.corpo.base()), (1.0, -1.0)),
                ];
                for (i, ((px, py), (qx, qy), (sx, sy))) in cantos.into_iter().enumerate() {
                    let rr = rs[i].min(maximo);
                    // O centro do arco do canto, para dentro da forma.
                    let (cx, cy) = (qx + sx * rr, qy + sy * rr);
                    let na_curva = (px - cx) * sx < 0.0 && (py - cy) * sy < 0.0;
                    if rr > 0.0 && na_curva {
                        let d = ((px - cx).powi(2) + (py - cy).powi(2)).sqrt();
                        assert!(d <= rr + 1.0, "{modo:?} {w}x{h} r{r}: canto {i} ({px},{py}) fora do arco");
                    }
                }
            }
        }
    }

    #[test]
    fn regiao_nao_muda_a_cada_quadro_no_quique() {
        // Do repouso até aberta (com quique) e de volta, em todos os modos: a região só troca no começo e quando assenta.
        for modo in MODOS {
            for (de, para, mola) in
                [(Forma::Repouso, Forma::Expandida, MOLA_CRESCER), (Forma::Expandida, Forma::Repouso, MOLA_ENCOLHER)]
            {
                let (inicio, fim) =
                    (medidas_no_modo(modo, de, None, 1.22, false), medidas_no_modo(modo, para, None, 1.22, false));
                let a = ancora(modo, Caixa::new(390.0, 0.0, inicio.largura, inicio.altura));
                let caixa =
                    |w: f32, h: f32| caixa_com_ombros(caixa_da_ancora(modo, a, (w, h)), modo.borda(), fim.ombro);
                let anim = crate::anim::Animacoes::default();
                anim.quadro(1.0 / 144.0, false);
                anim.definir_mola(("w", 0), inicio.largura);
                anim.definir_mola(("h", 0), inicio.altura);
                let mut regiao =
                    regiao_do_quadro(true, caixa(inicio.largura, inicio.altura), Caixa::default(), 0.0, (0, 0, 0, 0));
                let mut trocas = 0;
                for _ in 0..600 {
                    let w = anim.mola_amortecida(("w", 0), fim.largura, mola.0, mola.1);
                    let h = anim.mola_amortecida(("h", 0), fim.altura, mola.0, mola.1);
                    let assentou = anim.parada(("w", 0)) && anim.parada(("h", 0));
                    let agora = caixa(w, h);
                    let nova = regiao_do_quadro(
                        assentou,
                        agora,
                        caixa(fim.largura, fim.altura),
                        FOLGA_DA_REGIAO * 1.22,
                        regiao,
                    );
                    if nova != regiao {
                        trocas += 1;
                        regiao = nova;
                    }
                    // A região sempre cobre a forma de agora.
                    assert!(regiao.0 as f32 <= agora.x && regiao.2 as f32 >= agora.direita(), "{modo:?}");
                    assert!(regiao.1 as f32 <= agora.y && regiao.3 as f32 >= agora.base(), "{modo:?}");
                }
                assert!(trocas <= 3, "{modo:?} {de:?} -> {para:?}: a região trocou {trocas} vezes");
            }
        }
    }

    #[test]
    fn regioes_se_juntam_sem_contar_a_vazia() {
        assert_eq!(unir_regioes((10, 0, 50, 30), (0, 0, 0, 0)), (10, 0, 50, 30));
        assert_eq!(unir_regioes((0, 0, 0, 0), (10, 0, 50, 30)), (10, 0, 50, 30));
        assert_eq!(unir_regioes((10, 5, 50, 30), (20, 0, 80, 12)), (10, 0, 80, 30));
        assert_eq!(unir_regioes((5, 0, 5, 9), (20, 0, 80, 12)), (20, 0, 80, 12), "largura zero é vazia");
    }

    #[test]
    fn as_medidas_cabem_na_janela_em_todos_os_modos_e_tamanhos() {
        for (escala, minima) in [(0.86, true), (1.0, false), (1.22, false)] {
            let (lado, _) = tamanho_da_janela(escala);
            for modo in MODOS {
                for (forma, alerta) in [
                    (Forma::Repouso, None),
                    (Forma::Alerta, Some(Alerta::Faixa)),
                    (Forma::Alerta, Some(Alerta::Volume)),
                    (Forma::Alerta, Some(Alerta::Drop)),
                    (Forma::Expandida, None),
                ] {
                    let m = medidas_no_modo(modo, forma, alerta, escala, minima);
                    let c = caixa_com_ombros(Caixa::new(0.0, 0.0, m.largura, m.altura), modo.borda(), m.ombro);
                    assert!(c.w <= lado && c.h <= lado, "{modo:?} {forma:?}: {m:?}");
                    let r = m.raio * 2.0;
                    assert!(r <= m.altura.min(m.largura) + 0.01, "{modo:?} {forma:?}: canto maior que a forma");
                }
            }
        }
    }
}
