//! O selo de um serviço: um disco escuro com a marca em linha na cor dele, embaixo à direita da capa. Diz de onde vem cada
//! coisa quando a tela mistura serviços. Nos cartões, com o mouse em cima, ele cresce e vira o botão de tocar.

use super::{com_alfa, icone, medidas};
use crate::app::Player;
use crate::navegacao::Espaco;
use crate::ui::medidas::px;
use gpui::{ClickEvent, Context, Div, Hsla, Stateful, div, img, prelude::*, rgba};
use nucleo::Tema;
use provedores::{Faixa, Item};
use std::path::PathBuf;
use std::sync::Arc;

/// O "serviço" das músicas do computador.
pub(crate) const COMPUTADOR: &str = "computador";

/// Como desenhar a marca de um serviço.
#[derive(Clone)]
pub(crate) struct Marca {
    /// O desenho em linha, quando é um serviço que conhecemos.
    icone: Option<&'static str>,
    /// A logo que o provedor manda (para os outros).
    logo: Option<PathBuf>,
    pub cor: Hsla,
}

/// O fundo do disco: quase preto, para a marca aparecer sobre qualquer capa.
fn fundo() -> Hsla {
    rgba(0x0b_0b_0c_e6).into()
}

/// Uma cor que aparece sobre o fundo do disco (uma marca escura clareia).
fn legivel(cor: [f32; 3]) -> Hsla {
    let preto = [0.043, 0.043, 0.047];
    let mut c = cor;
    for passo in 0..=10 {
        let k = passo as f32 / 10.0;
        c = cor.map(|v| v + (1.0 - v) * k);
        if crate::tema::contraste(c, preto) >= 3.0 {
            break;
        }
    }
    crate::tema::cor_de(c)
}

fn desenho(m: &Marca, lado: f32) -> Div {
    let dentro = (lado * 0.7).round();
    let caixa = div().size(px(lado)).flex_none().flex().items_center().justify_center();
    match (m.icone, &m.logo) {
        (Some(simbolo), _) => caixa.child(icone(simbolo, dentro, m.cor)),
        (None, Some(logo)) => caixa.child(img(logo.clone()).size(px(dentro)).rounded(px(medidas::PILULA))),
        (None, None) => caixa.child(div().size(px(dentro * 0.6)).rounded(px(medidas::PILULA)).bg(m.cor)),
    }
}

/// O disco de uma marca.
pub(crate) fn disco(m: &Marca, lado: f32) -> Div {
    desenho(m, lado).rounded(px(medidas::PILULA)).bg(fundo()).border_1().border_color(com_alfa(gpui::white(), 0.12))
}

/// Os discos de uma música: um, ou dois sobrepostos (o que toca na frente, à direita).
pub(crate) fn discos(marcas: &[Marca], lado: f32) -> Div {
    let passo = (lado * 0.6).round();
    let n = marcas.len().min(2);
    let mut caixa = div().relative().flex_none().h(px(lado)).w(px(lado + passo * n.saturating_sub(1) as f32));
    for (i, m) in marcas.iter().take(2).enumerate().rev() {
        caixa = caixa.child(div().absolute().top_0().right(px(passo * i as f32)).child(disco(m, lado)));
    }
    caixa
}

impl Player {
    /// A marca de um serviço (ou do computador).
    pub(crate) fn marca_do_servico(&self, servico: &str) -> Marca {
        // No tema Mono nada tem cor.
        let mono = self.ajustes.tema == Tema::Mono;
        let simbolo = match servico {
            COMPUTADOR => Some("icones/pasta.svg"),
            "spotify" => Some("icones/marca-spotify.svg"),
            "youtube-music" => Some("icones/marca-youtube-music.svg"),
            _ => None,
        };
        let logo = match simbolo {
            Some(_) => None,
            None => self.local.remoto.provedores.iter().find(|p| p.servico == servico).and_then(|p| p.logo.clone()),
        };
        let cor = if mono || servico == COMPUTADOR { gpui::white() } else { legivel(self.cor_do_servico(servico)) };
        Marca { icone: simbolo, logo, cor }
    }

    fn marcas(&self, servicos: &[&str]) -> Vec<Marca> {
        servicos.iter().map(|s| self.marca_do_servico(s)).collect()
    }

    /// A tela mistura serviços? (Só então os itens levam selo.)
    pub(super) fn mistura_servicos(&self) -> bool {
        self.nav.espaco() == &Espaco::Tudo
    }

    /// Uma capa pequena (de uma linha, da barra) com o selo no canto, um pouco para fora. Sem serviços, só a capa.
    pub(super) fn com_selo(&self, capa: impl IntoElement, servicos: &[&str], lado: f32) -> Div {
        let caixa = div().relative().flex_none().child(capa);
        if servicos.is_empty() {
            return caixa;
        }
        caixa.child(div().absolute().right(px(-4.0)).bottom(px(-4.0)).child(discos(&self.marcas(servicos), lado)))
    }

    /// A marca de um serviço e, com `com_nome`, o nome dele ao lado (numa página estreita, só a marca).
    pub(super) fn marca_com_nome(&self, servico: &str, com_nome: bool) -> Div {
        let c = self.cores();
        let nome = if servico == COMPUTADOR { "Neste PC".to_string() } else { self.nome_do_servico(servico) };
        div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .child(disco(&self.marca_do_servico(servico), medidas::MARCA_DA_BARRA))
            .when(com_nome, |d| d.child(medidas::apoio().text_color(c.texto_2).child(nome)))
    }

    /// O canto da capa de um cartão: o selo, que com o mouse em cima (`sob`) cresce e vira o botão de tocar na cor do
    /// serviço. Sem selo (a tela é de um serviço só), o botão aparece no lugar dele.
    pub(super) fn selo_do_cartao(
        &self,
        n: usize,
        item: &Item,
        servicos: &[&str],
        sob: bool,
        fileira: Arc<Vec<Faixa>>,
        cx: &mut Context<Self>,
    ) -> Div {
        // Abre em 140 ms e volta em 100.
        let t = self.anim.seguir(("selo", n as u32), f32::from(u8::from(sob)), if sob { 0.14 } else { 0.10 });
        let marca = self.marca_do_servico(servicos.first().copied().unwrap_or(item.servico()));
        let canto = div().absolute().right(px(8.0)).bottom(px(8.0)).flex().justify_end().items_end();
        if t < 0.02 {
            // Parado: só o selo (ou nada).
            return if servicos.is_empty() {
                canto
            } else {
                canto.child(discos(&self.marcas(servicos), medidas::MARCA))
            };
        }
        let lado = (medidas::MARCA + (medidas::BOTAO_DO_CARTAO - medidas::MARCA) * t).round();
        let cor = marca.cor;
        // O ícone no preto ou no branco, o que aparecer mais sobre a cor do serviço.
        let claro = crate::tema::contraste([1.0; 3], crate::tema::tripla(cor))
            > crate::tema::contraste([0.0; 3], crate::tema::tripla(cor));
        let frente = if claro { gpui::white() } else { gpui::black() };
        let item = item.clone();
        let botao: Stateful<Div> = div()
            .id(("cartao-tocar", n))
            .size(px(lado))
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .rounded(px(medidas::PILULA))
            .shadow_md()
            .bg(super::misturar(Hsla { a: 1.0, ..fundo() }, cor, t))
            .when(servicos.is_empty(), |d| d.opacity(t))
            .cursor_pointer()
            .hover(|s| s.opacity(0.92))
            .child(if t < 0.5 && !servicos.is_empty() {
                desenho(&marca, lado)
            } else {
                div().child(icone("icones/play.svg", (lado * 0.5).round(), com_alfa(frente, t)))
            })
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.tocar_item(&item, &fileira);
                cx.notify();
            }));
        canto.child(botao)
    }

    /// De que serviço é o que está tocando (o que toca na frente; atrás, o da faixa quando ela toca por outro). Vazio
    /// quando não dá para saber.
    pub(crate) fn servicos_do_tocando(&self) -> Vec<String> {
        if self.origem_local {
            return self.local.atual().map(|m| self.servicos_da_musica(m)).unwrap_or_default();
        }
        self.midia.faixa.as_ref().and_then(|f| self.servico_do_app(&f.app)).into_iter().collect()
    }

    /// De que serviço é uma música da fila.
    pub(crate) fn servicos_da_musica(&self, m: &biblioteca::Musica) -> Vec<String> {
        match &m.remota {
            None => vec![COMPUTADOR.to_string()],
            Some(r) => match self.local.remoto.par(m) {
                Some(par) => vec![par.servico, r.servico.clone()],
                None => vec![r.servico.clone()],
            },
        }
    }

    /// O serviço de um app pelo nome que aparece no histórico e no "tocando agora".
    pub(crate) fn servico_do_app(&self, app: &str) -> Option<String> {
        if app == "Neste computador" {
            return Some(COMPUTADOR.to_string());
        }
        let provedores: Vec<(String, String)> =
            self.local.remoto.provedores.iter().map(|p| (p.servico.clone(), p.nome.clone())).collect();
        crate::juntar::servico_do_app(app, &provedores)
    }

    /// As marcas prontas para outra janela desenhar (a Ilha).
    pub(crate) fn marcas_do_tocando(&self) -> Vec<Marca> {
        self.servicos_do_tocando().iter().map(|s| self.marca_do_servico(s)).collect()
    }
}
