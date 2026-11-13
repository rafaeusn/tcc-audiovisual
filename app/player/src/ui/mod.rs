//! As telas do Player. Cada arquivo desenha uma região; este guarda os blocos que todas usam.
//!
//! Escala de texto fixa (tamanho/altura de linha): 12/16, 14/20, 16/24, 20/28, 28/32 e 44/44. Os títulos usam a
//! Instrument Sans estreita; os números grandes, a Doto (matriz de pontos); tempos e nomes de comando, a mono.
//! Raios: 28 (regiões), 16 (módulos), 10 (controles), pílula e círculo.
//!
//! Movimento (ver o artefato "Micro-interações do Player"): botão afunda ao pressionar (100 ms para descer,
//! 160 ms para voltar), marcador de aba desliza em 200 ms, dígitos do BPM rolam em 240 ms. O que muda o tempo
//! todo (número da energia) não anima.

mod biblioteca;
mod busca_letra;
mod campo;
mod casca;
mod dock;
mod heroi;
mod loja;
mod modulo;
mod musicas;
mod painel;
mod paleta;

use crate::app::Player;
use crate::arranjo::Ret;
use crate::tema::Cores;
use gpui::{
    App, ClickEvent, Context, Div, Hsla, MouseButton, MouseDownEvent, Stateful, Svg, Window, canvas, div, prelude::*,
    px, svg,
};
use nucleo::Comando;
use std::time::Duration;

pub const FONTE: &str = "Instrument Sans";
pub const FONTE_TITULO: &str = "Instrument Sans Titulo";
pub const FONTE_NUMERO: &str = "Doto";
pub const FONTE_MONO: &str = "Fragment Mono";

pub const RAIO_MODULO: f32 = 16.0;
pub const RAIO_CONTROLE: f32 = 10.0;

/// Uma região posicionada pelo arranjo.
pub fn caixa(r: Ret) -> Div {
    div().absolute().left(px(r.x)).top(px(r.y)).w(px(r.w)).h(px(r.h))
}

/// Texto num dos tamanhos da escala, com a altura de linha que acompanha.
pub fn texto(tamanho: f32) -> Div {
    let linha = match tamanho as u32 {
        0..=12 => 16.0,
        13..=14 => 20.0,
        15..=16 => 24.0,
        17..=20 => 28.0,
        21..=28 => 32.0,
        _ => (tamanho * 1.2).ceil(),
    };
    div().text_size(px(tamanho)).line_height(px(linha))
}

/// Título: fonte estreita e pesada. A linha tem folga para as pernas das letras ("g", "p") não serem cortadas.
pub fn titulo(tamanho: f32) -> Div {
    div().font_family(FONTE_TITULO).text_size(px(tamanho)).line_height(px((tamanho * 1.2).ceil()))
}

/// Número grande em matriz de pontos (BPM, relógio).
pub fn numero(tamanho: f32) -> Div {
    div().font_family(FONTE_NUMERO).text_size(px(tamanho)).line_height(px(tamanho))
}

/// Texto pequeno de largura fixa (tempos, nomes de comando).
pub fn mono() -> Div {
    div().font_family(FONTE_MONO).text_size(px(12.0)).line_height(px(16.0))
}

/// Ícone embutido (`icones/<nome>.svg`), pintado com `cor`.
pub fn icone(caminho: &'static str, tamanho: f32, cor: Hsla) -> Svg {
    svg().path(caminho).size(px(tamanho)).flex_none().text_color(cor)
}

/// Tecla de atalho, como aparece na tela.
pub fn tecla(c: &Cores, nome: &'static str) -> Div {
    texto(12.0)
        .h(px(22.0))
        .min_w(px(22.0))
        .px(px(7.0))
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .rounded(px(999.0))
        .border_1()
        .border_color(c.linha_2)
        .text_color(c.texto_2)
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .child(nome)
}

/// Curva de saída forte (começa rápido, chega devagar), de 0 a 1.
pub fn saida(x: f32) -> f32 {
    let x = if x.is_finite() { x.clamp(0.0, 1.0) } else { 1.0 };
    1.0 - (1.0 - x).powi(3)
}

pub fn com_alfa(cor: Hsla, a: f32) -> Hsla {
    Hsla { a, ..cor }
}

/// "1:32" (ou "1:02:03" acima de uma hora).
pub fn tempo(segundos: f64) -> String {
    let s = if segundos.is_finite() { segundos.max(0.0) as u64 } else { 0 };
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

/// "agora", "há 4 min", "há 2 h".
pub fn ha_quanto(d: Duration) -> String {
    let min = d.as_secs() / 60;
    match min {
        0 => "agora".to_string(),
        1..=59 => format!("há {min} min"),
        _ => format!("há {} h", min / 60),
    }
}

/// Número com vírgula decimal ("0,93").
pub fn decimal(v: f32) -> String {
    format!("{:.2}", v.clamp(0.0, 1.0)).replace('.', ",")
}

/// Aparência de um botão de ícone.
pub(crate) struct Botao {
    pub id: &'static str,
    pub icone: &'static str,
    pub tamanho: f32,
    pub tamanho_icone: f32,
    pub raio: f32,
    pub cor: Hsla,
    /// Fundo fixo (ligado/selecionado), se houver.
    pub fundo: Option<Hsla>,
    /// Fundo ao passar o mouse.
    pub realce: Hsla,
    /// Quanto encolhe ao ser pressionado (0,94 a 0,97).
    pub afunda: f32,
}

/// Largura aproximada da dica, em pixels (texto de 12 px em negrito, mais as teclas do atalho). Serve só para
/// ela não sair da janela; o centro exato quem dá é o layout.
fn largura_estimada(rotulo: &str, teclas: &[&str]) -> f32 {
    let texto = 7.2 * rotulo.chars().count() as f32;
    let atalho: f32 = teclas.iter().map(|t| (14.0 + 7.2 * t.chars().count() as f32).max(22.0) + 3.0).sum();
    22.0 + texto + if teclas.is_empty() { 0.0 } else { 8.0 + atalho }
}

/// Tamanho de um botão enquanto afunda, arredondado para um número par de pixels: assim o desenho encolhe
/// igual dos dois lados e fica parado no lugar, em vez de tremer meio pixel a cada quadro da animação.
fn tamanho_par(tamanho: f32) -> f32 {
    (tamanho / 2.0).round() * 2.0
}

impl Player {
    /// Nome curto de um comando para a dica do botão. Muda com o estado: "Tocar" vira "Pausar".
    fn rotulo_curto(&self, comando: Comando) -> &'static str {
        use nucleo::Pagina;
        match comando {
            Comando::TocarPausar if self.midia.tocando => "Pausar",
            Comando::TocarPausar => "Tocar",
            Comando::Proxima => "Próxima",
            Comando::Anterior => "Anterior",
            Comando::Silenciar if self.mudo => "Ativar o som",
            Comando::Silenciar => "Silenciar",
            Comando::TelaCheia => "Tela cheia",
            Comando::AlternarMini if self.mini => "Voltar ao tamanho normal",
            Comando::AlternarMini => "Miniplayer",
            Comando::AlternarPainel => "Painel",
            Comando::AbrirAjustes => "Ajustes",
            Comando::AbrirPagina(Pagina::Inicio) => "Início",
            Comando::AbrirPagina(Pagina::Biblioteca) => "Biblioteca",
            Comando::AbrirPagina(Pagina::Loja) => "Loja",
            Comando::AbrirPaleta => "Buscar",
            Comando::ProximoVisual => "Trocar o visual",
            Comando::EditarLayout => "Personalizar o layout",
            outro => outro.nome(),
        }
    }

    /// Guarda onde o elemento `id` foi desenhado (é um filho invisível, do tamanho dele). A dica usa isso para
    /// aparecer colada no botão certo.
    pub(crate) fn marca(&self, id: &'static str) -> impl IntoElement + use<> {
        let areas = self.areas.clone();
        canvas(
            move |area, _, _| {
                areas.borrow_mut().insert(id, area);
            },
            |_, (), _, _| {},
        )
        .absolute()
        .left(px(0.0))
        .top(px(0.0))
        .size_full()
    }

    /// O que acontece quando o mouse entra ou sai do botão `id`: a dica dele passa a valer, ou deixa de valer.
    /// Com `comando`, o texto é o nome curto do comando (e acompanha o estado); sem, é `rotulo`.
    pub(crate) fn ao_pairar(
        &self,
        id: &'static str,
        comando: Option<Comando>,
        rotulo: &'static str,
        cx: &Context<Self>,
    ) -> impl Fn(&bool, &mut Window, &mut App) + 'static {
        cx.listener(move |this, dentro: &bool, _, cx| {
            if *dentro {
                this.dica = Some(crate::app::DicaAtiva { id, comando, rotulo, desde: std::time::Instant::now() });
            } else if this.dica.as_ref().is_some_and(|d| d.id == id) {
                this.dica = None;
            }
            cx.notify();
        })
    }

    /// A dica do botão em que o mouse parou: o nome curto do que ele faz e o atalho, se houver.
    ///
    /// Fica acima do botão, centralizada nele, para não cobrir os vizinhos da mesma linha. Só desce para baixo
    /// quando não há espaço em cima (os botões da janela, no topo), e vai para o lado nos botões da barra
    /// lateral, que ficam empilhados. Perto das bordas ela encosta na margem em vez de sair da janela.
    /// Não aparece na janela mini, com a barra de comandos aberta, nem se a pessoa desligou as dicas.
    pub(crate) fn ver_dica(&self) -> Option<Div> {
        /// Quanto o mouse precisa ficar parado no botão.
        const ESPERA: f32 = 0.45;
        const ALTURA: f32 = 30.0;
        const VAO: f32 = 8.0;
        const MARGEM: f32 = 8.0;
        let d = self.dica.as_ref()?;
        if !self.ajustes.dicas || self.mini || self.paleta.is_some() || self.pressionado.is_some() {
            return None;
        }
        let parado = d.desde.elapsed().as_secs_f32();
        if parado < ESPERA {
            return None;
        }
        let area = *self.areas.borrow().get(d.id)?;
        let c = self.cores();
        let janela = self.arranjo.janela;
        let rotulo = d.comando.map_or(d.rotulo, |k| self.rotulo_curto(k));
        let teclas = d.comando.map_or(&[][..], Comando::teclas);
        let (x0, y0) = (f32::from(area.origin.x), f32::from(area.origin.y));
        let (x1, y1) = (x0 + f32::from(area.size.width), y0 + f32::from(area.size.height));

        let mut pilula = texto(12.0)
            .h(px(ALTURA))
            .px(px(10.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .rounded(px(RAIO_CONTROLE))
            .bg(c.bloco)
            .border_1()
            .border_color(c.linha_2)
            .shadow_md()
            .text_color(c.texto)
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .whitespace_nowrap()
            .child(rotulo);
        if !teclas.is_empty() {
            let mut grupo = div().flex().flex_none().gap(px(3.0));
            for t in teclas {
                grupo = grupo.child(tecla(&c, t));
            }
            pilula = pilula.child(grupo);
        }

        let aparece = ((parado - ESPERA) / 0.12).clamp(0.0, 1.0);
        let faixa = div().absolute().flex().opacity(aparece);
        // Barra lateral: os botões ficam um em cima do outro, então a dica vai para o lado.
        let na_lateral = self.arranjo.trilho.is_some_and(|t| x1 <= t.x + t.w + 1.0);
        if na_lateral {
            let y = ((y0 + y1) / 2.0 - ALTURA / 2.0).clamp(MARGEM, (janela.h - ALTURA - MARGEM).max(MARGEM));
            return Some(faixa.left(px(x1 + VAO)).top(px(y)).child(pilula));
        }
        let em_cima = y0 - VAO - ALTURA >= MARGEM;
        let y = if em_cima { y0 - VAO - ALTURA } else { (y1 + VAO).min(janela.h - ALTURA - MARGEM) };
        // A pílula fica centralizada no meio do botão: a faixa tem largura zero nesse ponto e o conteúdo dela
        // transborda igual para os dois lados. Só perto de uma borda o centro se afasta o bastante para a pílula
        // (pela largura estimada do texto) caber na janela.
        let estimada = largura_estimada(rotulo, teclas);
        let folga = (janela.w / 2.0 - MARGEM).max(0.0).min(estimada / 2.0);
        let centro = ((x0 + x1) / 2.0).clamp(MARGEM + folga, (janela.w - MARGEM - folga).max(MARGEM + folga));
        Some(faixa.left(px(centro)).top(px(y)).w(px(0.0)).justify_center().child(pilula))
    }

    /// O que um botão faz ao ser clicado: executar um comando.
    pub(crate) fn ao_clicar(
        &self,
        comando: Comando,
        cx: &mut Context<Self>,
    ) -> impl Fn(&ClickEvent, &mut Window, &mut gpui::App) + 'static {
        cx.listener(move |this, _: &ClickEvent, window, cx| this.executar(comando, window, cx))
    }

    /// Marca o botão `id` como pressionado (o estado some quando o mouse é solto em qualquer lugar).
    pub(crate) fn ao_apertar(
        &self,
        id: &'static str,
        cx: &mut Context<Self>,
    ) -> impl Fn(&MouseDownEvent, &mut Window, &mut gpui::App) + 'static {
        cx.listener(move |this, _: &MouseDownEvent, _, cx| {
            this.pressionado = Some(id);
            cx.notify();
        })
    }

    /// Escala atual do botão `id`: desce até `fundo` enquanto está pressionado e volta ao soltar.
    /// Chamar uma vez por quadro por botão.
    pub(crate) fn pressao(&self, id: &'static str, fundo: f32) -> f32 {
        let apertado = self.pressionado == Some(id);
        self.anim.seguir((id, u32::MAX), if apertado { fundo } else { 1.0 }, if apertado { 0.1 } else { 0.16 })
    }

    /// Botão de ícone que afunda ao ser pressionado. A área de clique não muda de tamanho; só o desenho encolhe.
    /// Quem chama liga o clique (`on_click`).
    pub(crate) fn botao(&self, b: Botao, cx: &mut Context<Self>) -> Stateful<Div> {
        let e = self.pressao(b.id, b.afunda);
        div()
            .id(b.id)
            .group(b.id)
            .size(px(b.tamanho))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .child(
                div()
                    .size(px(tamanho_par(b.tamanho * e)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(b.raio))
                    .when_some(b.fundo, |d, fundo| d.bg(fundo))
                    .group_hover(b.id, move |s| s.bg(b.realce))
                    .child(icone(b.icone, tamanho_par(b.tamanho_icone * e), b.cor)),
            )
            .child(self.marca(b.id))
            .on_mouse_down(MouseButton::Left, self.ao_apertar(b.id, cx))
    }

    /// Botão só com ícone (40x40, alvo confortável), ligado a um comando.
    pub(crate) fn botao_icone(
        &self,
        id: &'static str,
        caminho: &'static str,
        ativo: bool,
        comando: Comando,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        self.botao(
            Botao {
                id,
                icone: caminho,
                tamanho: 40.0,
                tamanho_icone: 20.0,
                raio: RAIO_CONTROLE,
                cor: if ativo { c.texto } else { c.texto_2 },
                fundo: ativo.then_some(c.sel),
                realce: c.hover,
                afunda: 0.94,
            },
            cx,
        )
        .on_click(self.ao_clicar(comando, cx))
        .on_hover(self.ao_pairar(id, Some(comando), "", cx))
    }

    /// Seletor em pílula com o marcador que desliza até a opção escolhida. Devolve a moldura (já com o marcador);
    /// quem chama põe dentro uma [`opcao`] por item, na ordem.
    pub(crate) fn seletor(&self, chave: &'static str, largura: f32, n: usize, ativo: usize) -> Div {
        let c = self.cores();
        let cada = (largura - 8.0).max(0.0) / n.max(1) as f32;
        let i = self.anim.seguir((chave, 0), ativo as f32, 0.2);
        div()
            .relative()
            .w(px(largura))
            .h(px(40.0))
            .flex_none()
            .flex()
            .p(px(3.0))
            .rounded(px(999.0))
            .border_1()
            .border_color(c.linha_2)
            .child(
                div()
                    .absolute()
                    .left(px(3.0 + i * cada))
                    .top(px(3.0))
                    .w(px(cada))
                    .h(px(32.0))
                    .rounded(px(999.0))
                    .bg(c.inv),
            )
    }

    /// Uma opção de um [`seletor`](Self::seletor). O fundo da opção escolhida é o marcador do seletor.
    pub(crate) fn opcao(&self, id: (&'static str, usize), rotulo: &'static str, ativa: bool) -> Stateful<Div> {
        let c = self.cores();
        texto(14.0)
            .id(id)
            .relative()
            .flex_1()
            .min_w(px(0.0))
            .h(px(32.0))
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(999.0))
            .cursor_pointer()
            .font_weight(gpui::FontWeight::SEMIBOLD)
            .text_color(if ativa { c.inv_texto } else { c.texto_2 })
            .when(!ativa, |d| d.hover(|s| s.text_color(c.texto)))
            .child(rotulo)
    }

    /// Aviso curto (troca de visual, modo seguro), centralizado acima do dock.
    pub(crate) fn ver_aviso(&self) -> Option<Div> {
        let (texto_aviso, _) = self.aviso.as_ref()?;
        let c = self.cores();
        let a = &self.arranjo;
        let base = if a.so_visual { 32.0 } else { a.janela.h - a.dock.y + 16.0 };
        Some(
            div().absolute().left(px(0.0)).bottom(px(base)).w_full().flex().justify_center().child(
                texto(14.0)
                    .px(px(16.0))
                    .py(px(10.0))
                    .rounded(px(999.0))
                    .bg(c.inv)
                    .text_color(c.inv_texto)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child(texto_aviso.clone()),
            ),
        )
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn tempo_formatado() {
        assert_eq!(tempo(0.0), "0:00");
        assert_eq!(tempo(92.7), "1:32");
        assert_eq!(tempo(3723.0), "1:02:03");
        assert_eq!(tempo(f64::NAN), "0:00");
        assert_eq!(tempo(-5.0), "0:00");
    }

    #[test]
    fn ha_quanto_tempo() {
        assert_eq!(ha_quanto(Duration::from_secs(20)), "agora");
        assert_eq!(ha_quanto(Duration::from_secs(4 * 60 + 10)), "há 4 min");
        assert_eq!(ha_quanto(Duration::from_secs(2 * 3600 + 5)), "há 2 h");
    }

    #[test]
    fn curva_de_saida() {
        assert_eq!(saida(0.0), 0.0);
        assert_eq!(saida(1.0), 1.0);
        assert_eq!(saida(7.0), 1.0);
        assert_eq!(saida(f32::NAN), 1.0);
        assert!(saida(0.5) > 0.8, "sai rápido");
    }

    #[test]
    fn botao_afunda_em_pixels_pares() {
        assert_eq!(tamanho_par(40.0), 40.0);
        assert_eq!(tamanho_par(37.6), 38.0);
        assert_eq!(tamanho_par(18.8), 18.0);
        assert_eq!(tamanho_par(39.2), 40.0);
    }

    #[test]
    fn dica_tem_largura_para_o_texto_e_o_atalho() {
        let simples = largura_estimada("Loja", &[]);
        assert!((40.0..70.0).contains(&simples), "{simples}");
        assert!(largura_estimada("Miniplayer", &["N"]) > largura_estimada("Miniplayer", &[]) + 25.0);
        assert!(largura_estimada("Anterior", &["Ctrl", "←"]) > largura_estimada("Anterior", &["N"]));
    }

    #[test]
    fn decimal_com_virgula() {
        assert_eq!(decimal(0.934), "0,93");
        assert_eq!(decimal(7.0), "1,00");
    }
}
