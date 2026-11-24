//! As páginas dos serviços no Palco: "Para você" (prateleiras de cartões), álbum, playlist, artista e a
//! Biblioteca do serviço (o que foi salvo dele). Toda página abre na hora, com esqueleto, e se enche quando o
//! provedor responde.

use super::{com_alfa, icone, mono, tempo, texto, titulo};
use crate::app::Player;
use crate::navegacao::{Espaco, Rota};
use crate::paginas::{Cartao, cartao, janela_de_linhas, pedido_da_rota, rota_do_artista, rota_do_item};
use crate::remoto::{PaginaRemota, PedidoDePagina};
use biblioteca::TipoSalvo;
use gpui::{
    AnyElement, ClickEvent, Context, Div, FontWeight, ObjectFit, SharedString, Stateful, div, img, prelude::*, px,
};
use provedores::{Capacidade, Faixa, Item};
use std::sync::Arc;

/// Lado do cartão.
pub(super) const CARTAO: f32 = 156.0;
/// Altura da linha de faixa.
pub(super) const LINHA: f32 = 56.0;
/// Quantas faixas populares o artista mostra antes do "Ver mais".
const POPULARES: usize = 5;

/// O brilho lento do esqueleto (0..1), pelo relógio.
fn brilho() -> f32 {
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let t = (ms % 1600) as f32 / 1600.0;
    0.5 - 0.5 * (t * std::f32::consts::TAU).cos()
}

impl Player {
    /// Quanto a página de agora já entrou (0..1): desliza 24 px do lado de onde veio e aparece.
    pub(super) fn entrada(&self) -> (f32, f32) {
        let p = self.anim.seguir(("rota-entra", 0), 1.0, 0.24);
        // Em pixels inteiros: um pai em posição fracionária faz os filhos tremerem 1 px (AGENTS.md, seção 6).
        (p, (24.0 * (1.0 - p) * self.entrada_de).round())
    }

    /// A capa: a imagem baixada, ou a silhueta do tipo enquanto ela não vem (ou se não há).
    pub(super) fn capa(&self, url: Option<&str>, tipo: TipoSalvo, lado: f32, redondo: bool) -> Div {
        let c = self.cores();
        let raio = if redondo { lado / 2.0 } else { 8.0 };
        let caixa = div().size(px(lado)).flex_none().rounded(px(raio)).overflow_hidden().bg(c.bloco);
        let simbolo = match tipo {
            TipoSalvo::Faixa => "icones/wave.svg",
            TipoSalvo::Album => "icones/album.svg",
            TipoSalvo::Artista => "icones/person.svg",
            TipoSalvo::Playlist => "icones/playlist.svg",
        };
        let cor = c.texto_3;
        let silhueta = move || {
            div()
                .size(px(lado))
                .flex()
                .items_center()
                .justify_center()
                .child(icone(simbolo, lado * 0.32, cor))
                .into_any_element()
        };
        match url.and_then(|u| self.capas.arquivo(u)) {
            // Enquanto decodifica, e se o arquivo não for uma imagem que dê para ler, fica a silhueta.
            Some(arquivo) => caixa.child(
                img(arquivo).size(px(lado)).object_fit(ObjectFit::Cover).with_loading(silhueta).with_fallback(silhueta),
            ),
            None => caixa.child(silhueta()),
        }
    }

    /// Um cartão. A capa cresce um pouco sob o mouse; clicar abre a página do item, ou toca a faixa (com as outras
    /// faixas da fileira na fila).
    pub(super) fn ver_cartao(
        &self,
        n: usize,
        item: &Item,
        fileira: Arc<Vec<Faixa>>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        let Cartao { titulo: nome, sub, capa, redondo, tipo } = cartao(item);
        let sob = self.cartao_sob == Some(n);
        let escala = self.anim.seguir(("cartao", n as u32), if sob { 1.03 } else { 1.0 }, 0.16);
        let lado = CARTAO * escala;
        let imagem = div()
            .size(px(CARTAO))
            .flex()
            .items_center()
            .justify_center()
            .child(self.capa(capa.as_deref(), tipo, lado, redondo).when(!redondo, |d| d.shadow_md()));
        let item = item.clone();
        div()
            .id(("cartao", n))
            .w(px(CARTAO))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .cursor_pointer()
            .child(imagem)
            .child(
                texto(14.0)
                    .font_weight(FontWeight::SEMIBOLD)
                    .truncate()
                    .when(redondo, |d| d.text_center())
                    .child(SharedString::from(nome)),
            )
            .child(
                texto(12.0)
                    .text_color(c.texto_2)
                    .line_clamp(2)
                    .when(redondo, |d| d.text_center())
                    .child(SharedString::from(sub)),
            )
            .on_hover(cx.listener(move |this, dentro: &bool, _, cx| {
                if *dentro {
                    this.cartao_sob = Some(n);
                } else if this.cartao_sob == Some(n) {
                    this.cartao_sob = None;
                }
                cx.notify();
            }))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                match rota_do_item(&item) {
                    Some(rota) => this.abrir_rota(rota),
                    None => {
                        if let Item::Faixa(f) = &item {
                            let i = fileira.iter().position(|x| x.id == f.id).unwrap_or(0);
                            this.tocar_faixas(&fileira, i);
                        }
                    }
                }
                cx.notify();
            }))
    }

    /// Cartões cinza com brilho, no lugar dos que ainda não chegaram.
    pub(super) fn esqueleto_de_cartoes(&self, quantos: usize) -> Div {
        let c = self.cores();
        let a = 0.5 + 0.35 * brilho();
        let mut fila = div().flex().gap(px(16.0)).overflow_hidden();
        for _ in 0..quantos {
            fila = fila.child(
                div()
                    .w(px(CARTAO))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(div().size(px(CARTAO)).rounded(px(8.0)).bg(com_alfa(c.bloco, a)))
                    .child(div().h(px(12.0)).w(px(CARTAO * 0.7)).rounded(px(4.0)).bg(com_alfa(c.bloco, a)))
                    .child(div().h(px(10.0)).w(px(CARTAO * 0.45)).rounded(px(4.0)).bg(com_alfa(c.bloco, a))),
            );
        }
        fila
    }

    /// Uma prateleira: o título e a fileira de cartões, rolando na horizontal.
    pub(super) fn ver_prateleira(&self, k: usize, titulo_: &str, itens: &[Item], cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let fileira: Arc<Vec<Faixa>> = Arc::new(itens.iter().filter_map(Item::faixa).cloned().collect());
        let rolagem = self.rolagem_da_prateleira(k);
        let mut cartoes =
            div().id(("prateleira", k)).track_scroll(&rolagem).flex().gap(px(16.0)).pb(px(8.0)).overflow_x_scroll();
        // A rodinha desce a página; a prateleira rola de lado só com as setas, o Shift ou o touchpad.
        cartoes.style().restrict_scroll_to_axis = Some(true);
        for (i, item) in itens.iter().enumerate().take(30) {
            cartoes = cartoes.child(self.ver_cartao(k * 1000 + i, item, fileira.clone(), cx));
        }
        let (x, maximo) = (f32::from(rolagem.offset().x), f32::from(rolagem.max_offset().width));
        let passo = 3.0 * (CARTAO + 16.0);
        let seta = |lado: i32, simbolo: &'static str, visivel: bool, cx: &mut Context<Self>| {
            let rolagem = rolagem.clone();
            div()
                .id(("prateleira-seta", k * 2 + usize::from(lado > 0)))
                .absolute()
                .top(px(CARTAO / 2.0 - 18.0))
                .when(lado < 0, |d| d.left(px(-6.0)))
                .when(lado > 0, |d| d.right(px(-6.0)))
                .size(px(36.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(999.0))
                .bg(com_alfa(gpui::black(), 0.72))
                .border_1()
                .border_color(c.linha)
                .cursor_pointer()
                .when(!visivel, |d| d.invisible())
                .occlude()
                .child(icone(simbolo, 18.0, gpui::white()))
                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                    // O clique é da seta: não chega ao cartão de baixo.
                    cx.stop_propagation();
                    let agora = rolagem.offset();
                    let novo = (f32::from(agora.x) - lado as f32 * passo).clamp(-maximo, 0.0);
                    rolagem.set_offset(gpui::point(px(novo), agora.y));
                    cx.notify();
                }))
        };
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(texto(18.0).font_weight(FontWeight::BOLD).child(SharedString::from(titulo_.to_string())))
            .child(div().relative().child(cartoes).child(seta(-1, "icones/chev-l.svg", x < -1.0, cx)).child(seta(
                1,
                "icones/chev-r.svg",
                // Antes de a fileira ser medida (`maximo` 0), a seta aparece se há cartões além da tela.
                if maximo > 0.0 { -x < maximo - 1.0 } else { itens.len() > 3 },
                cx,
            )))
    }

    /// A frase de erro com "Tentar de novo".
    fn erro_da_pagina(&self, n: usize, motivo: &str, pedido: PedidoDePagina, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let paleta = self.paleta_do_espaco();
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(texto(14.0).text_color(c.texto_2).child(motivo.to_string()))
            .child(
                texto(13.0)
                    .id(("pagina-de-novo", n))
                    .h(px(32.0))
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .rounded(px(999.0))
                    .bg(paleta.acento)
                    .text_color(paleta.texto_no_acento)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .child("Tentar de novo")
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.local.remoto.de_novo(&pedido);
                        cx.notify();
                    })),
            )
    }

    /// O cartão de entrar na conta do serviço, enquanto fora dela (ou entrando). `compacto` (em Tudo) é uma linha.
    /// `None` quando o serviço não tem conta ou já está dentro.
    pub(super) fn cartao_da_conta(
        &self,
        servico: &str,
        compacto: bool,
        n: usize,
        cx: &mut Context<Self>,
    ) -> Option<Div> {
        use crate::contas::{Aviso, aviso_da_conta};
        let info = self.local.remoto.provedores.iter().find(|p| p.servico == servico)?;
        let aviso = aviso_da_conta(info.conta.as_ref());
        if aviso == Aviso::Nada {
            return None;
        }
        let c = self.cores();
        let paleta = self.paleta_do_espaco();
        let nome = info.nome.clone();
        let alvo = servico.to_string();
        let pilula = |id: &'static str, rotulo: String, cheio: bool| {
            let (fundo, frente) = if cheio { (paleta.acento, paleta.texto_no_acento) } else { (c.sel, c.texto) };
            texto(14.0)
                .id((id, n))
                .h(px(40.0))
                .px(px(18.0))
                .flex()
                .items_center()
                .rounded(px(999.0))
                .bg(fundo)
                .text_color(frente)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.opacity(0.88))
                .child(rotulo)
        };
        let acao = match &aviso {
            // Renovando a credencial guardada: não há navegador nem o que cancelar.
            Aviso::Entrando { navegador: false } => div().child(texto(14.0).text_color(c.texto_2).child("Conectando…")),
            Aviso::Entrando { navegador: true } => div()
                .flex()
                .items_center()
                .gap(px(14.0))
                .child(texto(14.0).text_color(c.texto_2).child("Aprove no navegador que abriu…"))
                .child(pilula("conta-cancelar", "Cancelar".into(), false).on_click(cx.listener(
                    move |this, _: &ClickEvent, _, cx| {
                        this.local.remoto.sair(&alvo);
                        cx.notify();
                    },
                ))),
            _ => div().child(pilula("conta-entrar", format!("Entrar no {nome}"), true).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.local.remoto.entrar(&alvo);
                    cx.notify();
                },
            ))),
        };
        let erro = match &aviso {
            Aviso::Entrar { erro: Some(e) } => Some(texto(13.0).text_color(c.texto_2).child(e.clone())),
            _ => None,
        };
        if compacto {
            return Some(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(14.0))
                            .child(texto(14.0).text_color(c.texto).child(format!("{nome}: entre na sua conta")))
                            .child(acao),
                    )
                    .children(erro),
            );
        }
        let logo = info.logo.clone().map(|arquivo| img(arquivo).size(px(56.0)).rounded(px(999.0)));
        Some(
            div()
                .max_w(px(520.0))
                .p(px(24.0))
                .rounded(px(14.0))
                .bg(com_alfa(c.bloco, 0.7))
                .flex()
                .flex_col()
                // O botão fica do tamanho do texto (sem esticar na largura do cartão).
                .items_start()
                .gap(px(14.0))
                .children(logo)
                .child(titulo(22.0).child(format!("Entre na sua conta do {nome}")))
                .child(
                    texto(14.0)
                        .text_color(c.texto_2)
                        .child("Abre o navegador; quem entra é você. A senha não passa pelo Player."),
                )
                .child(acao)
                .children(erro),
        )
    }

    /// "Para você": as prateleiras do serviço aberto (em Tudo, de todos, cada um com o selo).
    pub(crate) fn ver_para_voce(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let (p, dx) = self.entrada();
        let tudo = matches!(self.nav.espaco(), Espaco::Tudo);
        let servicos: Vec<(String, String)> = self
            .espacos()
            .into_iter()
            .filter_map(|(e, nome)| match e {
                Espaco::Servico(s) if tudo || self.nav.espaco() == &Espaco::Servico(s.clone()) => Some((s, nome)),
                _ => None,
            })
            .collect();
        let mut corpo = div()
            .relative()
            .left(px(dx))
            .opacity(p)
            .flex()
            .flex_col()
            .gap(px(28.0))
            .p(px(24.0))
            .child(titulo(32.0).child("Para você"));
        if servicos.is_empty() {
            let abrindo = self.local.remoto.provedores.iter().any(|p| p.situacao == crate::remoto::Situacao::Abrindo);
            let frase = if abrindo {
                "Abrindo os serviços…"
            } else {
                "Nenhum serviço aberto. Ligue um provedor na Loja, na aba Provedores."
            };
            return corpo.child(texto(14.0).text_color(c.texto_2).child(frase));
        }
        for (k, (servico, nome)) in servicos.into_iter().enumerate() {
            // Fora da conta, o cartão de entrar fica no lugar das prateleiras (o serviço ainda não tem o que mostrar).
            if let Some(cartao) = self.cartao_da_conta(&servico, tudo, k, cx) {
                corpo = corpo.child(cartao);
                continue;
            }
            let pedido = PedidoDePagina::Inicio { servico };
            let bloco = match self.local.remoto.pagina(&pedido).as_deref() {
                None | Some(PaginaRemota::Carregando) => div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(div().h(px(18.0)).w(px(160.0)).rounded(px(4.0)).bg(com_alfa(c.bloco, 0.6)))
                    .child(self.esqueleto_de_cartoes(6)),
                Some(PaginaRemota::Erro(motivo)) => self.erro_da_pagina(k, motivo, pedido.clone(), cx),
                Some(PaginaRemota::Prateleiras(prateleiras)) if prateleiras.is_empty() => div().child(
                    texto(14.0).text_color(c.texto_2).child(format!("O {nome} não mandou nada para o início agora.")),
                ),
                Some(PaginaRemota::Prateleiras(prateleiras)) => {
                    let mut lista = div().flex().flex_col().gap(px(28.0));
                    for (j, pr) in prateleiras.iter().enumerate() {
                        lista = lista.child(self.ver_prateleira(k * 20 + j, &pr.titulo, &pr.itens, cx));
                    }
                    lista
                }
                Some(PaginaRemota::Itens(_)) => div(),
            };
            corpo = corpo.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .when(tudo, |d| {
                        d.child(
                            texto(12.0)
                                .text_color(c.texto_3)
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(SharedString::from(nome.to_uppercase())),
                        )
                    })
                    .child(bloco),
            );
        }
        corpo
    }

    /// O coração de curtir (pulsa ao clicar).
    pub(super) fn coracao(
        &self,
        id: (&'static str, usize),
        item: &Item,
        tamanho: f32,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        let paleta = self.paleta_do_espaco();
        let salvo = self.item_salvo(item);
        let pulso = id.1 as u32;
        let escala = self.anim.seguir(("coracao", pulso), 1.0, 0.3);
        let item = item.clone();
        div()
            .id(id)
            .size(px(tamanho + 16.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(999.0))
            .cursor_pointer()
            .hover(|s| s.bg(c.hover))
            .child(icone(
                if salvo { "icones/heart-f.svg" } else { "icones/heart.svg" },
                tamanho * escala,
                if salvo { paleta.acento } else { c.texto_2 },
            ))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.alternar_salvo(&item, pulso);
                cx.notify();
            }))
    }

    /// Botão em pílula (Tocar no acento do espaço; os outros discretos).
    pub(super) fn pilula(
        &self,
        id: &'static str,
        simbolo: &'static str,
        rotulo: &'static str,
        principal: bool,
    ) -> Stateful<Div> {
        let c = self.cores();
        let paleta = self.paleta_do_espaco();
        let (fundo, frente) = if principal { (paleta.acento, paleta.texto_no_acento) } else { (c.sel, c.texto) };
        texto(14.0)
            .id(id)
            .h(px(40.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(px(999.0))
            .bg(fundo)
            .text_color(frente)
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(|s| s.opacity(0.88))
            .child(icone(simbolo, 18.0, frente))
            .child(rotulo)
    }

    /// Uma linha de faixa: número, título, artista (link quando dá), duração, coração e, no hover, a seguir e fila.
    pub(super) fn linha_de_faixa(
        &self,
        n: usize,
        i: usize,
        faixas: &Arc<Vec<Faixa>>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        let paleta = self.paleta_do_espaco();
        let f = &faixas[i];
        let esta = self
            .local
            .atual()
            .is_some_and(|m| m.remota.as_ref().is_some_and(|r| r.id == f.id && r.servico == f.servico));
        let sabe_artista = self.sabe(&f.servico, Capacidade::Artista);
        let mut artistas = div().flex().min_w(px(0.0)).overflow_hidden();
        for (k, nome) in f.artistas.iter().enumerate() {
            let separador = if k + 1 < f.artistas.len() { ", " } else { "" };
            let rotulo = SharedString::from(format!("{nome}{separador}"));
            artistas = artistas.child(match rota_do_artista(f, k, sabe_artista) {
                Some(rota) => texto(13.0)
                    .id(("faixa-artista", n * 10 + k))
                    .text_color(c.texto_2)
                    .flex_none()
                    .cursor_pointer()
                    .hover(|s| s.text_color(c.texto).underline())
                    .child(rotulo)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        cx.stop_propagation();
                        this.abrir_rota(rota.clone());
                        cx.notify();
                    }))
                    .into_any_element(),
                None => texto(13.0).text_color(c.texto_2).flex_none().child(rotulo).into_any_element(),
            });
        }
        let item = Item::Faixa(f.clone());
        let pagina = Rota::Faixa(item.clone());
        let (a_seguir, no_fim) = (f.clone(), f.clone());
        let lista = faixas.clone();
        let grupo = "faixa-linha";
        div()
            .id(("faixa", n))
            .group(grupo)
            .h(px(LINHA))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(8.0))
            .cursor_pointer()
            .hover(|s| s.bg(c.hover))
            .child(div().w(px(28.0)).flex_none().flex().justify_center().child(if esta {
                icone("icones/wave.svg", 16.0, paleta.acento).into_any_element()
            } else {
                mono().text_color(c.texto_3).child((i + 1).to_string()).into_any_element()
            }))
            .child(self.capa(f.capa.as_deref(), TipoSalvo::Faixa, 40.0, false))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        // O título abre a página da faixa (a linha, fora dele, toca).
                        div().flex().min_w(px(0.0)).child(
                            texto(14.0)
                                .id(("faixa-titulo", n))
                                .min_w(px(0.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .when(esta, |d| d.text_color(paleta.acento))
                                .hover(|s| s.underline())
                                .child(SharedString::from(f.titulo.clone()))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.abrir_rota(pagina.clone());
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(artistas),
            )
            .child(self.acao_da_linha(("faixa-a-seguir", n), "icones/play-next.svg", grupo, false).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    cx.notify();
                    if this.avisar_se_nao_toca(&a_seguir.servico) {
                        return;
                    }
                    let m = crate::remoto::musica_de(&a_seguir, &this.nome_do_servico(&a_seguir.servico));
                    this.local.tocar_a_seguir(m);
                    this.avisar(format!("\"{}\" toca a seguir", a_seguir.titulo));
                    cx.notify();
                }),
            ))
            .child(self.acao_da_linha(("faixa-na-fila", n), "icones/queue-add.svg", grupo, false).on_click(
                cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    cx.notify();
                    if this.avisar_se_nao_toca(&no_fim.servico) {
                        return;
                    }
                    let m = crate::remoto::musica_de(&no_fim, &this.nome_do_servico(&no_fim.servico));
                    this.local.adicionar_a_fila(vec![m]);
                    this.avisar(format!("\"{}\" foi para o fim da fila", no_fim.titulo));
                    cx.notify();
                }),
            ))
            .child(self.coracao(("faixa-coracao", 50_000 + n), &item, 16.0, cx))
            .child(
                mono()
                    .w(px(44.0))
                    .flex_none()
                    .text_color(c.texto_3)
                    .text_right()
                    .child(f.duracao_s.map(tempo).unwrap_or_default()),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.tocar_faixas(&lista, i);
                cx.notify();
            }))
    }

    /// Álbum, playlist ou artista.
    pub(crate) fn ver_detalhe(&self, rota: &Rota, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let (p, dx) = self.entrada();
        let (item, rotulo) = match rota {
            Rota::Album(i) => (i, "ÁLBUM"),
            Rota::Playlist(i) => (i, "PLAYLIST"),
            Rota::Artista(i) => (i, "ARTISTA"),
            _ => return div(),
        };
        let ficha = cartao(item);
        let largura = self.arranjo.heroi.w;
        let lado = if largura >= 720.0 { 168.0 } else { 120.0 };
        let tamanho = if ficha.titulo.chars().count() > 28 { 28.0 } else { 38.0 };
        let servico = crate::paginas::chave_do_salvo(item).0;
        let pagina = pedido_da_rota(rota).and_then(|p| self.local.remoto.pagina(&p));
        let (faixas, albuns) = match &pagina {
            Some(p) => {
                let d = self.detalhe(p);
                (d.faixas, d.albuns)
            }
            None => (Arc::new(Vec::new()), Arc::new(Vec::new())),
        };

        // O artista do álbum vira link quando o provedor deu o id.
        let subtitulo: AnyElement = match item {
            Item::Album(a) if a.id_do_artista(0).is_some() && self.sabe(&servico, Capacidade::Artista) => {
                let alvo = Rota::Artista(Item::Artista(provedores::Artista {
                    servico: servico.clone(),
                    id: a.id_do_artista(0).unwrap_or_default().to_string(),
                    nome: a.artistas.first().cloned().unwrap_or_default(),
                    foto: None,
                }));
                texto(14.0)
                    .id("detalhe-artista")
                    .text_color(c.texto)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .hover(|s| s.underline())
                    .child(SharedString::from(ficha.sub.clone()))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.abrir_rota(alvo.clone());
                        cx.notify();
                    }))
                    .into_any_element()
            }
            _ => texto(14.0).text_color(c.texto_2).child(SharedString::from(ficha.sub.clone())).into_any_element(),
        };
        let tocar_tudo = faixas.clone();
        let misturar = faixas.clone();
        let salvar = item.clone();
        let acoes = div()
            .flex()
            .items_center()
            .gap(px(10.0))
            .child(self.pilula("detalhe-tocar", "icones/play.svg", "Tocar", true).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.tocar_faixas(&tocar_tudo, 0);
                    cx.notify();
                },
            )))
            .child(self.pilula("detalhe-aleatorio", "icones/shuffle.svg", "Aleatório", false).on_click(cx.listener(
                move |this, _: &ClickEvent, window, cx| {
                    this.tocar_aleatorio(&misturar, window, cx);
                    cx.notify();
                },
            )))
            .child(self.coracao(("detalhe-coracao", 40_000), &salvar, 20.0, cx));

        let cabecalho = div()
            .flex()
            .items_end()
            .gap(px(20.0))
            .child(self.capa(ficha.capa.as_deref(), ficha.tipo, lado, ficha.redondo).shadow_lg())
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .gap(px(6.0))
                    .child(texto(12.0).text_color(c.texto_2).font_weight(FontWeight::SEMIBOLD).child(rotulo))
                    .child(titulo(tamanho).line_clamp(2).child(SharedString::from(ficha.titulo.clone())))
                    .when(!ficha.redondo, |d| d.child(subtitulo))
                    .child(div().mt(px(8.0)).child(acoes)),
            );

        let corpo =
            div().relative().left(px(dx)).opacity(p).flex().flex_col().gap(px(24.0)).p(px(24.0)).child(cabecalho);
        let pedido = pedido_da_rota(rota);
        let conteudo = match (pagina.as_deref(), pedido) {
            (None | Some(PaginaRemota::Carregando), _) => {
                let a = 0.5 + 0.35 * brilho();
                let mut linhas = div().flex().flex_col().gap(px(8.0));
                for _ in 0..6 {
                    linhas = linhas.child(div().h(px(LINHA - 8.0)).rounded(px(8.0)).bg(com_alfa(c.bloco, a)));
                }
                linhas
            }
            (Some(PaginaRemota::Erro(motivo)), Some(pedido)) => self.erro_da_pagina(900, motivo, pedido, cx),
            (Some(PaginaRemota::Itens(_)), _) if faixas.is_empty() && albuns.is_empty() => {
                let nome = self.nome_do_servico(&servico);
                div().child(
                    texto(14.0).text_color(c.texto_2).child(format!("O {nome} não mandou nada para esta página.")),
                )
            }
            _ => {
                let mut lista = div().flex().flex_col();
                let artista = matches!(rota, Rota::Artista(_));
                let mostrar = if artista && !self.ver_mais { faixas.len().min(POPULARES) } else { faixas.len() };
                if artista && !faixas.is_empty() {
                    lista = lista.child(texto(18.0).font_weight(FontWeight::BOLD).mb(px(8.0)).child("Populares"));
                }
                // Só as linhas à vista são montadas (as Curtidas podem ter milhares); o espaço das outras fica.
                let topo = self.areas.borrow().get("detalhe-linhas").map(|a| f32::from(a.origin.y));
                let folha = &self.arranjo.heroi;
                let janela = janela_de_linhas(topo, (folha.y, folha.y + folha.h), LINHA, mostrar);
                let mut linhas = div()
                    .relative()
                    .flex()
                    .flex_col()
                    .child(self.marca("detalhe-linhas"))
                    .child(div().flex_none().h(px(LINHA * janela.start as f32)));
                for i in janela.clone() {
                    linhas = linhas.child(self.linha_de_faixa(i, i, &faixas, cx));
                }
                linhas = linhas.child(div().flex_none().h(px(LINHA * (mostrar - janela.end) as f32)));
                lista = lista.child(linhas);
                if artista && faixas.len() > POPULARES {
                    let rotulo = if self.ver_mais { "Ver menos" } else { "Ver mais" };
                    lista = lista.child(
                        texto(13.0)
                            .id("detalhe-ver-mais")
                            .mt(px(6.0))
                            .px(px(12.0))
                            .text_color(c.texto_2)
                            .font_weight(FontWeight::SEMIBOLD)
                            .cursor_pointer()
                            .hover(|s| s.text_color(c.texto))
                            .child(rotulo)
                            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                                this.ver_mais = !this.ver_mais;
                                cx.notify();
                            })),
                    );
                }
                if !albuns.is_empty() {
                    let fileira = Arc::new(Vec::new());
                    let mut grade = div().flex().flex_wrap().gap(px(16.0));
                    for (i, a) in albuns.iter().take(self.cartoes_a_mostrar).enumerate() {
                        grade = grade.child(self.ver_cartao(7_000 + i, a, Arc::clone(&fileira), cx));
                    }
                    lista = lista.child(
                        div()
                            .mt(px(24.0))
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .child(texto(18.0).font_weight(FontWeight::BOLD).child("Álbuns"))
                            .child(grade)
                            .children(self.ver_mais_cartoes("albuns-ver-mais", albuns.len(), cx)),
                    );
                }
                lista
            }
        };
        corpo.child(conteudo)
    }

    /// "Ver mais (N)" no pé de uma grade longa, enquanto há cartões escondidos.
    fn ver_mais_cartoes(&self, id: &'static str, total: usize, cx: &mut Context<Self>) -> Option<Stateful<Div>> {
        let c = self.cores();
        let faltam = total.checked_sub(self.cartoes_a_mostrar).filter(|n| *n > 0)?;
        let botao = texto(13.0)
            .id(id)
            .mt(px(8.0))
            .text_color(c.texto_2)
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(|s| s.text_color(c.texto))
            .child(format!("Ver mais ({faltam})"));
        Some(botao.on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
            this.cartoes_a_mostrar += super::CARTOES_POR_VEZ;
            cx.notify();
        })))
    }

    /// A Biblioteca de um serviço: o que está na conta e o que foi salvo dele, com filtro por tipo e fixar.
    pub(crate) fn ver_biblioteca_do_servico(&self, servico: &str, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let (p, dx) = self.entrada();
        let nome = self.nome_do_servico(servico);
        let filtro = self.filtro_salvos;
        let paginas = self.bibliotecas(Some(servico));
        let junta = self.biblioteca_junta(Some(servico));
        let juntos: Vec<&crate::paginas::NaBiblioteca> =
            junta.iter().filter(|b| filtro.is_none_or(|t| t == crate::paginas::chave_do_salvo(&b.item).1)).collect();
        let mut chips = div().flex().gap(px(8.0));
        let opcoes: [(Option<TipoSalvo>, &'static str); 5] = [
            (None, "Tudo"),
            (Some(TipoSalvo::Faixa), "Faixas"),
            (Some(TipoSalvo::Album), "Álbuns"),
            (Some(TipoSalvo::Artista), "Artistas"),
            (Some(TipoSalvo::Playlist), "Playlists"),
        ];
        for (i, (tipo, rotulo)) in opcoes.into_iter().enumerate() {
            let ativo = self.filtro_salvos == tipo;
            chips = chips.child(
                texto(13.0)
                    .id(("salvos-filtro", i))
                    .h(px(32.0))
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .rounded(px(999.0))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .bg(if ativo { c.inv } else { c.sel })
                    .text_color(if ativo { c.inv_texto } else { c.texto })
                    .child(rotulo)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.filtro_salvos = tipo;
                        cx.notify();
                    })),
            );
        }
        let mut corpo = div()
            .relative()
            .left(px(dx))
            .opacity(p)
            .flex()
            .flex_col()
            .gap(px(20.0))
            .p(px(24.0))
            .child(titulo(32.0).child("Sua biblioteca"))
            // Fora da conta, o convite fica no alto; o que foi salvo no Player continua embaixo.
            .children(self.cartao_da_conta(servico, false, 200, cx))
            .child(chips);
        // A conta: carregando, ou o motivo de não ter vindo (o que foi salvo no Player continua embaixo).
        for pagina in &paginas {
            match &**pagina {
                PaginaRemota::Carregando => corpo = corpo.child(self.esqueleto_de_cartoes(6)),
                PaginaRemota::Erro(motivo) => {
                    let pedido = PedidoDePagina::Biblioteca { servico: servico.to_string() };
                    corpo = corpo.child(self.erro_da_pagina(910, motivo, pedido, cx));
                }
                _ => {}
            }
        }
        if juntos.is_empty() {
            if paginas.iter().any(|p| !matches!(**p, PaginaRemota::Itens(_))) {
                return corpo;
            }
            let frase = if filtro.is_some() {
                format!("Nada deste tipo na sua biblioteca do {nome}.")
            } else {
                format!("Curta faixas e salve álbuns, artistas e playlists do {nome}: eles aparecem aqui.")
            };
            return corpo.child(texto(14.0).text_color(c.texto_2).child(frase));
        }
        let fileira: Arc<Vec<Faixa>> = Arc::new(juntos.iter().filter_map(|b| b.item.faixa().cloned()).collect());
        let mut grade = div().flex().flex_wrap().gap(px(16.0));
        let mostrar = juntos.len().min(self.cartoes_a_mostrar);
        for (i, b) in juntos.iter().take(mostrar).enumerate() {
            let (item, fixado) = (&b.item, b.fixado);
            let alvo: Item = item.clone();
            let alfinete = div()
                .id(("salvo-fixar", i))
                .absolute()
                .top(px(6.0))
                .right(px(6.0))
                .size(px(32.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(999.0))
                .bg(com_alfa(gpui::black(), 0.6))
                .cursor_pointer()
                .when(!fixado, |d| d.invisible().group_hover("salvo", |s| s.visible()))
                .child(icone("icones/pin.svg", 16.0, gpui::white()))
                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                    cx.stop_propagation();
                    this.fixar_item(&alvo, !fixado);
                    cx.notify();
                }));
            grade = grade.child(
                div()
                    .relative()
                    .group("salvo")
                    .child(self.ver_cartao(9_000 + i, item, fileira.clone(), cx))
                    .child(alfinete),
            );
        }
        corpo = corpo.child(grade);
        corpo = corpo.children(self.ver_mais_cartoes("biblioteca-ver-mais", juntos.len(), cx));
        corpo
    }
}
