//! Configurações › Serviços: por onde as músicas de cada serviço tocam, e o player próprio dos que têm um (o Soloist).

use super::painel::{bloco_ajuste, linha_ajuste};
use super::texto;
use crate::app::Player;
use crate::contas::{BotaoDoTocador, frase_da_conta, por_onde_toca, tela_do_tocador};
use gpui::{ClickEvent, Context, Div, SharedString, div, prelude::*, px, relative};
use provedores::EstadoDoTocador;

/// Onde a pessoa cria a chave pessoal dela.
const PAGINA_DA_CHAVE: &str = "https://developer.spotify.com/dashboard/soloist";

impl Player {
    pub(super) fn bloco_servicos(&self, largura: f32, cx: &mut Context<Self>) -> Vec<Div> {
        let c = self.cores();
        let provedores = &self.local.remoto.provedores;
        let mut blocos = Vec::new();
        for (n, p) in provedores.iter().enumerate() {
            let quem = self.local.remoto.quem_toca(&p.servico);
            let toca = por_onde_toca(&quem, &p.situacao, |s| self.nome_do_servico(s));
            let frase = match &p.conta {
                Some(conta) => format!("{}. {toca}", frase_da_conta(conta)),
                None => toca,
            };
            let nome: &'static str = match p.servico.as_str() {
                "spotify" => "Spotify",
                "youtube-music" => "YouTube Music",
                _ => "Serviço instalado",
            };
            let frase = if nome == "Serviço instalado" { format!("{}: {frase}", p.nome) } else { frase };
            let mut bloco = bloco_ajuste(&c, n + 1 == provedores.len()).child(linha_ajuste(
                &c,
                "icones/globe.svg",
                nome,
                frase.into(),
            ));
            if let Some(lido) = self.local.remoto.tocador(&p.servico) {
                bloco = bloco.child(self.ver_tocador(n, &p.servico, &lido.estado, largura, cx));
            }
            blocos.push(bloco);
        }
        if blocos.is_empty() {
            blocos.push(
                bloco_ajuste(&c, true).child(
                    texto(14.0)
                        .text_color(c.texto_2)
                        .child("Nenhum serviço instalado. Eles ficam na aba Provedores da Loja."),
                ),
            );
        }
        blocos
    }

    /// O player próprio do serviço: a situação, o que falta e os botões.
    fn ver_tocador(
        &self,
        n: usize,
        servico: &str,
        estado: &EstadoDoTocador,
        largura: f32,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.cores();
        let tela = tela_do_tocador(estado);
        let mut botoes = div().flex().flex_wrap().gap(px(8.0));
        for (i, b) in tela.botoes.into_iter().enumerate() {
            let servico = servico.to_string();
            botoes =
                botoes.child(self.pilula_da_loja(("tocador-botao", n * 10 + i), b.rotulo(), b.principal()).on_click(
                    cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.apertar_do_tocador(&servico, b, cx);
                        cx.notify();
                    }),
                ));
        }
        let recuo = if largura < 340.0 { 0.0 } else { 44.0 };
        let mut corpo = div()
            .pl(px(recuo))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(texto(14.0).text_color(c.texto_2).child(SharedString::from(tela.frase)));
        if let Some(p) = tela.progresso {
            corpo = corpo.child(
                div()
                    .h(px(4.0))
                    .w_full()
                    .rounded(px(999.0))
                    .bg(c.sel)
                    .child(div().h_full().w(relative(p)).rounded(px(999.0)).bg(c.texto)),
            );
        }
        corpo.child(botoes)
    }

    fn apertar_do_tocador(&mut self, servico: &str, botao: BotaoDoTocador, cx: &mut Context<Self>) {
        match botao {
            BotaoDoTocador::PaginaDaChave => cx.open_url(PAGINA_DA_CHAVE),
            BotaoDoTocador::ColarChave => {
                // A chave sai da área de transferência direto para o provedor: não aparece na tela nem no registro.
                match cx.read_from_clipboard().and_then(|c| c.text()).filter(|t| !t.trim().is_empty()) {
                    Some(chave) => self.local.remoto.ligar_tocador(servico, Some(chave)),
                    None => self.avisar("Copie a chave na página do Spotify e tente de novo"),
                }
            }
            BotaoDoTocador::Ligar => self.local.remoto.ligar_tocador(servico, None),
            BotaoDoTocador::UsarOutro => self.local.remoto.trocar_tocador(servico),
            BotaoDoTocador::Desligar => self.local.remoto.desligar_tocador(servico),
        }
    }
}
