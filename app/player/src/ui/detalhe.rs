//! As páginas de álbum, playlist e artista: o cabeçalho com a imagem atrás (a foto larga do artista, ou a capa desfocada),
//! os números que o serviço manda, as abas do artista e o artista no outro serviço. As contas ficam em `detalhes.rs`.

use super::paginas::{LINHA, LinhaDeFaixa, SeloDoCartao};
use super::{com_alfa, medidas};
use crate::app::Player;
use crate::arranjo::RAIO;
use crate::capas::{Fundo, TipoDeFundo};
use crate::detalhes::{self, AbaDoArtista};
use crate::juntar::Junto;
use crate::menu::Alvo;
use crate::navegacao::{Espaco, Rota};
use crate::paginas::{cartao, pedido_da_rota};
use crate::remoto::PaginaRemota;
use crate::tema;
use crate::ui::medidas::px;
use biblioteca::TipoSalvo;
use gpui::{
    AnyElement, Bounds, ClickEvent, Context, Div, FontWeight, ObjectFit, PathBuilder, Pixels, SharedString, canvas,
    div, img, linear_color_stop, linear_gradient, point, prelude::*,
};
use provedores::{Capacidade, Detalhe, Faixa, Item, Prateleira, TipoDaPrateleira, TipoDeAlbum};
use std::sync::Arc;

/// Embaixo do cabeçalho, a imagem se apaga na folha nesta altura (o texto fica acima dela).
const SUMICO: f32 = 48.0;
/// Quantas populares aparecem antes de "Ver mais".
const POPULARES: usize = 5;
/// A partir desta largura útil, as populares e o último lançamento ficam lado a lado.
const DUAS_COLUNAS: f32 = 820.0;

/// O tipo do lançamento, como se lê no cabeçalho.
fn nome_do_tipo(tipo: Option<TipoDeAlbum>) -> &'static str {
    match tipo {
        Some(TipoDeAlbum::Single) => "Single",
        Some(TipoDeAlbum::Ep) => "EP",
        Some(TipoDeAlbum::Coletanea) => "Coletânea",
        _ => "Álbum",
    }
}

/// O mesmo tipo de item e o mesmo id (o detalhe só completa o item da página quando é ele).
fn mesmo_item(a: &Item, b: &Item) -> bool {
    std::mem::discriminant(a) == std::mem::discriminant(b)
        && crate::paginas::chave_do_salvo(a) == crate::paginas::chave_do_salvo(b)
}

impl Player {
    /// A camada de trás do cabeçalho: a imagem (ou a cor) e o véu da folha por cima, calculado para o texto do tema ler
    /// bem nela; embaixo, a imagem se apaga na folha.
    pub(super) fn fundo_do_cabecalho(&self, fundo: Option<&Fundo>, tipo: TipoDeFundo, cor: Option<[u8; 3]>) -> Div {
        let c = self.cores();
        let escuro = tema::luminancia(tema::tripla(c.folha)) < 0.5;
        let base = cor.map(|cor| tema::cor_de(cor.map(|v| f32::from(v) / 255.0)));
        let alfa = match (fundo, cor) {
            (Some(f), _) => detalhes::veu(f.claro, f.escuro, escuro),
            (None, Some([r, g, b])) => {
                let l = (0.2126 * f32::from(r) + 0.7152 * f32::from(g) + 0.0722 * f32::from(b)) / 255.0;
                detalhes::veu(l, l, escuro)
            }
            (None, None) => 1.0,
        };
        let mut camada =
            div().absolute().top(px(0.0)).left(px(0.0)).size_full().overflow_hidden().bg(base.unwrap_or(c.folha));
        if let Some(f) = fundo {
            camada = camada.child(img(f.arquivo.clone()).absolute().size_full().object_fit(ObjectFit::Cover));
        }
        // Na foto larga o texto fica embaixo: em cima a foto aparece mais.
        let veu = match tipo {
            TipoDeFundo::Largo => div().bg(linear_gradient(
                180.0,
                linear_color_stop(com_alfa(c.folha, alfa * 0.4), 0.0),
                linear_color_stop(com_alfa(c.folha, alfa), 0.55),
            )),
            TipoDeFundo::Desfocado => div().bg(com_alfa(c.folha, alfa)),
        };
        camada.child(veu.absolute().size_full()).child(
            div().absolute().left(px(0.0)).bottom(px(0.0)).w_full().h(px(SUMICO)).bg(linear_gradient(
                180.0,
                linear_color_stop(com_alfa(c.folha, 0.0), 0.0),
                linear_color_stop(c.folha, 1.0),
            )),
        )
    }

    /// Os cantos da folha por cima do conteúdo: o GPUI só recorta em retângulo, e uma imagem no alto da página sairia
    /// pelos cantos redondos. Pinta o lado de fora de cada canto com a cor de trás e redesenha a borda.
    pub(super) fn cantos_da_folha(&self) -> Div {
        let c = self.cores();
        let fora = c.parede;
        // Quanto a curva de Bézier se afasta do canto para parecer um quarto de círculo.
        const K: f32 = 0.552_284_8;
        let desenho = canvas(
            |_, _, _| (),
            move |b: Bounds<Pixels>, (), window, _| {
                let (x0, y0) = (b.origin.x, b.origin.y);
                let (x1, y1) = (x0 + b.size.width, y0 + b.size.height);
                for ((x, y), dx, dy) in
                    [((x0, y0), 1.0, 1.0), ((x1, y0), -1.0, 1.0), ((x0, y1), 1.0, -1.0), ((x1, y1), -1.0, -1.0)]
                {
                    let (rx, ry) = (px(RAIO * dx), px(RAIO * dy));
                    let mut p = PathBuilder::fill();
                    p.move_to(point(x, y));
                    p.line_to(point(x + rx, y));
                    p.cubic_bezier_to(point(x, y + ry), point(x + rx * (1.0 - K), y), point(x, y + ry * (1.0 - K)));
                    p.close();
                    if let Ok(caminho) = p.build() {
                        window.paint_path(caminho, fora);
                    }
                }
            },
        );
        div()
            .absolute()
            .top(px(0.0))
            .left(px(0.0))
            .size_full()
            .child(desenho.absolute().size_full())
            .child(div().absolute().size_full().rounded(px(RAIO)).border_1().border_color(c.linha))
    }

    /// O fundo pronto de um endereço (pede e segue: a imagem chega num dos próximos quadros).
    fn fundo_de(&self, url: Option<&str>, tipo: TipoDeFundo) -> Option<Fundo> {
        url.filter(|u| !u.is_empty()).and_then(|u| self.capas.fundo(u, tipo))
    }

    /// Álbum, playlist ou artista.
    pub(crate) fn ver_detalhe(&self, rota: &Rota, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let item = match rota {
            Rota::Album(i) | Rota::Playlist(i) | Rota::Artista(i) => i,
            _ => return div(),
        };
        let pagina = pedido_da_rota(rota).and_then(|p| self.local.remoto.pagina(&p));
        let lido = pagina.as_ref().map(|p| self.detalhe(p));
        let (faixas, albuns) = lido
            .as_ref()
            .map_or_else(|| (Arc::new(Vec::new()), Arc::new(Vec::new())), |d| (d.faixas.clone(), d.albuns.clone()));
        let detalhe = lido.as_ref().and_then(|d| d.detalhe.clone());
        // O item da página, completo quando o serviço mandou (o artista aberto por um link ganha a foto).
        let cabeca = detalhe
            .as_ref()
            .and_then(|d| d.item.clone())
            .filter(|i| mesmo_item(i, item))
            .unwrap_or_else(|| item.clone());
        let artista = matches!(rota, Rota::Artista(_));
        let largo = detalhe.as_ref().and_then(|d| d.imagem.clone()).filter(|_| artista);

        let cabecalho = match largo {
            Some(imagem) => self.cabecalho_do_artista(&cabeca, &imagem, detalhe.as_deref(), &faixas, cx),
            None => self.cabecalho_da_lista(&cabeca, detalhe.as_deref(), &faixas, cx),
        };
        let corpo = div().opacity(self.entrada()).flex().flex_col().child(cabecalho);
        let conteudo = match (pagina.as_deref(), pedido_da_rota(rota)) {
            (None | Some(PaginaRemota::Carregando), _) => self.esqueleto_de_linhas(),
            (Some(PaginaRemota::Erro(motivo)), Some(pedido)) => self.erro_da_pagina(900, motivo, pedido, cx),
            (Some(PaginaRemota::Itens(..)), _) if faixas.is_empty() && albuns.is_empty() => {
                let nome = self.nome_do_servico(item.servico());
                div().child(
                    medidas::apoio().text_color(c.texto_2).child(format!("O {nome} não mandou nada para esta página.")),
                )
            }
            _ if artista => self.corpo_do_artista(&faixas, &albuns, detalhe.as_deref(), cx),
            _ => self.corpo_da_lista(rota, &faixas, detalhe.as_deref(), cx),
        };
        corpo.child(div().px(px(self.lado_da_pagina())).pb(px(32.0)).child(conteudo))
    }

    fn esqueleto_de_linhas(&self) -> Div {
        let c = self.cores();
        let a = 0.5 + 0.35 * super::paginas::brilho_do_esqueleto();
        let mut linhas = div().flex().flex_col().gap(px(8.0));
        for _ in 0..6 {
            linhas = linhas.child(div().h(px(LINHA - 8.0)).rounded(px(medidas::RAIO_CARTAO)).bg(com_alfa(c.bloco, a)));
        }
        linhas
    }

    /// Tocar, aleatório, curtir (ou seguir) e o ⋯. Os do cabeçalho do artista e da lista são os mesmos.
    fn acoes_do_detalhe(&self, item: &Item, faixas: &Arc<Vec<Faixa>>, cx: &mut Context<Self>) -> Div {
        let largura = self.arranjo.heroi.w;
        let (tocar_tudo, misturar, salvar, menu) = (faixas.clone(), faixas.clone(), item.clone(), item.clone());
        let seguir = matches!(item, Item::Artista(_)).then(|| {
            let rotulo = if self.item_salvo(item) { "Seguindo" } else { "Seguir" };
            let alvo = item.clone();
            self.pilula("detalhe-seguir", "icones/person.svg", rotulo, false).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.alternar_salvo(&alvo, 40_001);
                    cx.notify();
                },
            ))
        });
        div()
            .flex()
            .flex_wrap()
            .items_center()
            .gap(px(12.0))
            .child(self.pilula("detalhe-tocar", "icones/play.svg", "Tocar", true).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.tocar_faixas(&tocar_tudo, 0);
                    cx.notify();
                },
            )))
            // Na folha mais estreita o aleatório fica só no ⋯.
            .when(largura >= 420.0, |d| {
                d.child(self.pilula("detalhe-aleatorio", "icones/shuffle.svg", "Aleatório", false).on_click(
                    cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.tocar_aleatorio(&misturar, window, cx);
                        cx.notify();
                    }),
                ))
            })
            .map(|d| match seguir {
                Some(s) => d.child(s),
                None => d.child(self.coracao(("detalhe-coracao", 40_000), &salvar, medidas::ICONE, cx)),
            })
            .child(self.botao_mais(("detalhe-mais", 0), medidas::FICHA, move || Alvo::Item(Box::new(menu.clone())), cx))
    }

    /// "Ver no YouTube Music": o mesmo artista nos outros serviços (só quando o primeiro resultado da busca pelo nome é
    /// um artista com o mesmo nome).
    fn no_outro_servico(&self, artista: &provedores::Artista, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let c = self.cores();
        let mut links = Vec::new();
        for (espaco, nome) in self.espacos() {
            let Espaco::Servico(destino) = espaco else { continue };
            if destino == artista.servico || !self.sabe(&destino, Capacidade::Artista) {
                continue;
            }
            let Some(la) = self.local.remoto.artista_em(artista, &destino) else { continue };
            let rota = Rota::Artista(Item::Artista(la));
            links.push(
                medidas::apoio()
                    .id(SharedString::from(format!("artista-em-{destino}")))
                    .text_color(c.texto)
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .underline()
                    .child(format!("Ver no {nome}"))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.abrir_rota(rota.clone());
                        cx.notify();
                    }))
                    .into_any_element(),
            );
        }
        links
    }

    /// O cabeçalho do artista com a foto larga: o nome grande embaixo, os ouvintes, as ações.
    fn cabecalho_do_artista(
        &self,
        item: &Item,
        imagem: &str,
        detalhe: Option<&Detalhe>,
        faixas: &Arc<Vec<Faixa>>,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let largura = a.heroi.w;
        let baixa = a.janela.h < 640.0;
        let altura = match (largura < 520.0, baixa) {
            (true, true) => 232.0,
            (false, true) => 260.0,
            (true, false) => 300.0,
            (false, false) => 380.0,
        };
        let base = if largura >= 720.0 && !baixa { medidas::CARTAZ } else { medidas::PAGINA };
        let nome = cartao(item).titulo;
        let tamanho = super::heroi::tamanho_do_titulo(base, nome.chars().count());
        let fundo = self.fundo_de(Some(imagem), TipoDeFundo::Largo);
        let cor = detalhe.and_then(|d| detalhes::cor(d.cor.as_deref()?));
        let numeros = detalhe.and_then(|d| match (d.ouvintes, d.seguidores) {
            (Some(n), _) => Some(format!("{} de ouvintes por mês", detalhes::resumido(n))),
            (None, Some(n)) => Some(format!("{} de seguidores", detalhes::resumido(n))),
            _ => None,
        });
        let links = match item {
            Item::Artista(artista) => self.no_outro_servico(artista, cx),
            _ => Vec::new(),
        };
        let lado = self.lado_da_pagina();
        div()
            .relative()
            .min_h(px(altura))
            .flex()
            .flex_col()
            .justify_end()
            .child(self.fundo_do_cabecalho(fundo.as_ref(), TipoDeFundo::Largo, cor))
            .child(
                div()
                    .relative()
                    .px(px(lado))
                    .pt(px(64.0))
                    .pb(px(SUMICO))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(
                        super::titulo(tamanho)
                            .pb(px((tamanho * 0.08).ceil()))
                            .text_color(c.texto)
                            .child(super::linhas(nome, 2)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_x(px(16.0))
                            .gap_y(px(4.0))
                            .child(super::selo::disco(&self.marca_do_servico(item.servico()), medidas::MARCA_DA_BARRA))
                            .children(numeros.map(|n| medidas::corpo().text_color(c.texto).child(n)))
                            .children(links),
                    )
                    .child(div().mt(px(4.0)).child(self.acoes_do_detalhe(item, faixas, cx))),
            )
    }

    /// O cabeçalho do álbum e da playlist (e do artista sem foto larga): a capa desfocada atrás, a capa, o nome e os dados.
    fn cabecalho_da_lista(
        &self,
        item: &Item,
        detalhe: Option<&Detalhe>,
        faixas: &Arc<Vec<Faixa>>,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let largura = a.heroi.w;
        let baixa = a.janela.h < 640.0;
        let lado_da_capa = if largura < 480.0 {
            120.0
        } else if baixa {
            140.0
        } else if largura < 760.0 {
            176.0
        } else {
            232.0
        };
        let ficha = cartao(item);
        let servico = item.servico().to_string();
        let rotulo = match item {
            Item::Album(al) => nome_do_tipo(detalhe.and_then(|d| d.tipo).or(al.tipo)),
            Item::Playlist(_) => "Playlist",
            _ => "Artista",
        };
        // O artista do álbum vira link quando o provedor deu o id.
        let quem: Option<AnyElement> = match item {
            Item::Album(al) => {
                // Só os artistas: o ano vai nos dados, ao lado.
                let nome = SharedString::from(al.artistas.join(", "));
                match al.id_do_artista(0).filter(|_| self.sabe(&servico, Capacidade::Artista)) {
                    Some(id) => {
                        let alvo = Rota::Artista(Item::Artista(provedores::Artista {
                            servico: servico.clone(),
                            id: id.to_string(),
                            nome: al.artistas.first().cloned().unwrap_or_default(),
                            foto: None,
                        }));
                        Some(
                            medidas::corpo()
                                .id("detalhe-artista")
                                .text_color(c.texto)
                                .font_weight(FontWeight::SEMIBOLD)
                                .cursor_pointer()
                                .hover(|s| s.underline())
                                .child(nome)
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    this.abrir_rota(alvo.clone());
                                    cx.notify();
                                }))
                                .into_any_element(),
                        )
                    }
                    None => Some(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(nome).into_any_element()),
                }
            }
            Item::Playlist(p) => detalhe
                .and_then(|d| d.dono.clone())
                .or(p.dono.clone())
                .map(|d| medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(d).into_any_element()),
            _ => None,
        };
        let ano = match item {
            Item::Album(al) => al.ano,
            _ => None,
        };
        let dados = if matches!(item, Item::Artista(_)) {
            detalhe
                .and_then(|d| d.ouvintes.map(|n| format!("{} de ouvintes por mês", detalhes::resumido(n))))
                .into_iter()
                .collect()
        } else {
            detalhes::dados_da_lista(ano, detalhe, faixas)
        };
        let seguidores = match item {
            Item::Playlist(_) => {
                detalhe.and_then(|d| d.seguidores).map(|n| detalhes::quantas(n, "seguidor", "seguidores"))
            }
            _ => None,
        };
        let descricao = detalhe.and_then(|d| d.descricao.clone()).filter(|_| matches!(item, Item::Playlist(_)));
        let links = match item {
            Item::Artista(artista) => self.no_outro_servico(artista, cx),
            _ => Vec::new(),
        };
        let fundo = self.fundo_de(ficha.capa.as_deref(), TipoDeFundo::Desfocado);
        let cor = detalhe.and_then(|d| detalhes::cor(d.cor.as_deref()?));
        let titulo = medidas::pagina(largura >= 720.0 && ficha.titulo.chars().count() <= 28)
            .text_color(c.texto)
            .child(super::linhas(ficha.titulo.clone(), 2));
        let mut meta = div().flex().flex_wrap().items_center().gap_x(px(16.0)).gap_y(px(4.0)).text_color(c.texto);
        meta = meta.children(quem);
        for d in dados.into_iter().chain(seguidores) {
            meta = meta.child(medidas::corpo().child(d));
        }
        meta =
            meta.child(super::selo::disco(&self.marca_do_servico(&servico), medidas::MARCA_DA_BARRA)).children(links);
        let acoes = self.acoes_do_detalhe(item, faixas, cx);
        // Em folha estreita as ações não cabem ao lado da capa: ficam numa linha própria.
        let (ao_lado, embaixo) = if largura < 640.0 { (None, Some(acoes)) } else { (Some(acoes), None) };
        let textos = div()
            .flex_1()
            .min_w(px(0.0))
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(medidas::corpo().text_color(c.texto).child(rotulo))
            .child(titulo)
            .child(meta)
            .children(descricao.map(|d| medidas::apoio().text_color(c.texto).child(super::linhas(d, 2))))
            .children(ao_lado.map(|a| div().mt(px(8.0)).child(a)));
        let capa = self.capa_do_cartao(&ficha, lado_da_capa).shadow_lg();
        div().relative().child(self.fundo_do_cabecalho(fundo.as_ref(), TipoDeFundo::Desfocado, cor)).child(
            div()
                .relative()
                .px(px(self.lado_da_pagina()))
                .pt(px(if baixa { 24.0 } else { 40.0 }))
                .pb(px(SUMICO))
                .flex()
                .flex_col()
                .gap(px(20.0))
                .child(
                    div()
                        .flex()
                        .items_end()
                        .gap(px(if largura < 480.0 { 20.0 } else { 32.0 }))
                        .child(capa)
                        .child(textos),
                )
                .children(embaixo),
        )
    }

    /// As faixas do álbum (com o número, sem a capa repetida) ou da playlist (com a capa), o rodapé com a data e a gravadora,
    /// e as prateleiras que o serviço mandou.
    fn corpo_da_lista(
        &self,
        rota: &Rota,
        faixas: &Arc<Vec<Faixa>>,
        detalhe: Option<&Detalhe>,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.cores();
        let album = matches!(rota, Rota::Album(_));
        let como = LinhaDeFaixa { numero: album, sem_capa: album, ouvidas: album, album: !album, ..Default::default() };
        let mut lista = div().flex().flex_col().child(self.linhas_de_faixas(faixas, faixas.len(), como, cx));
        let rodape: Vec<String> = detalhe
            .map(|d| d.data.as_deref().map(detalhes::data_longa).into_iter().chain(d.gravadora.clone()).collect())
            .unwrap_or_default();
        if album && !rodape.is_empty() {
            let mut pe = div().mt(px(24.0)).flex().flex_col().gap(px(4.0));
            for linha in rodape {
                pe = pe.child(medidas::apoio().text_color(c.texto_2).child(linha));
            }
            lista = lista.child(pe);
        }
        lista.children(self.prateleiras_do_detalhe(detalhe.map_or(&[][..], |d| &d.secoes), 0, cx))
    }

    /// As prateleiras de um detalhe (de um serviço só: sem selos).
    fn prateleiras_do_detalhe(&self, secoes: &[Prateleira], de: usize, cx: &mut Context<Self>) -> Vec<Div> {
        let fileira = Arc::new(Vec::new());
        secoes
            .iter()
            .filter(|s| !s.itens.is_empty())
            .enumerate()
            .map(|(i, s)| {
                let juntos: Vec<Junto> = s.itens.iter().cloned().map(Junto::so).collect();
                div().mt(px(40.0)).child(self.ver_prateleira(
                    3_000_100 + de + i,
                    &s.titulo,
                    None,
                    &juntos,
                    &fileira,
                    cx,
                ))
            })
            .collect()
    }

    /// A página do artista abaixo do cabeçalho: as abas e o que cada uma mostra.
    fn corpo_do_artista(
        &self,
        faixas: &Arc<Vec<Faixa>>,
        albuns: &Arc<Vec<Item>>,
        detalhe: Option<&Detalhe>,
        cx: &mut Context<Self>,
    ) -> Div {
        let abas = detalhes::abas_do_artista(albuns, detalhe);
        let aba = if abas.contains(&self.aba_do_artista) { self.aba_do_artista } else { AbaDoArtista::VisaoGeral };
        let escolhida = abas.iter().position(|a| *a == aba).unwrap_or(0);
        let mut botoes = Vec::with_capacity(abas.len());
        for (i, a) in abas.iter().copied().enumerate() {
            botoes.push(
                self.aba_sublinhada(("artista-aba", i), a.nome(), a == aba)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.aba_do_artista = a;
                        this.cartoes_a_mostrar = super::CARTOES_POR_VEZ;
                        cx.notify();
                    }))
                    .into_any_element(),
            );
        }
        let corpo = match aba {
            AbaDoArtista::VisaoGeral => self.visao_geral(faixas, albuns, detalhe, cx),
            AbaDoArtista::Sobre => self.sobre_o_artista(detalhe),
            outra => self.grade_de_lancamentos(&detalhes::lancamentos(albuns, outra), cx),
        };
        div()
            .flex()
            .flex_col()
            .gap(px(28.0))
            .child(self.abas_sublinhadas("artista-abas", 2_000_004, escolhida, botoes))
            .child(corpo)
    }

    /// As populares (com quantas vezes cada uma foi ouvida) e o último lançamento ao lado; os álbuns, os singles e o que
    /// mais o serviço mandou, em prateleiras.
    fn visao_geral(
        &self,
        faixas: &Arc<Vec<Faixa>>,
        albuns: &Arc<Vec<Item>>,
        detalhe: Option<&Detalhe>,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.cores();
        let mostrar = if self.ver_mais { faixas.len() } else { faixas.len().min(POPULARES) };
        let como = LinhaDeFaixa { numero: true, ouvidas: true, ..Default::default() };
        let mut populares = div().flex_1().min_w(px(0.0)).flex().flex_col();
        if !faixas.is_empty() {
            populares = populares
                .child(medidas::secao().mb(px(12.0)).child("Populares"))
                .child(self.linhas_de_faixas(faixas, mostrar, como, cx));
        }
        if faixas.len() > POPULARES {
            let rotulo = if self.ver_mais { "Ver menos" } else { "Ver mais" };
            populares = populares.child(
                medidas::apoio()
                    .id("detalhe-ver-mais")
                    .mt(px(8.0))
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
        let secoes = detalhe.map_or(&[][..], |d| &d.secoes);
        let ultimo = secoes.iter().find(|s| s.tipo == TipoDaPrateleira::UltimoLancamento).and_then(|s| s.itens.first());
        let util = self.arranjo.heroi.w - 2.0 * self.lado_da_pagina();
        let lado_a_lado = util >= DUAS_COLUNAS;
        let ultimo = ultimo.map(|item| {
            let cartao = self.ver_cartao(6_999, item, SeloDoCartao::default(), Arc::new(Vec::new()), cx);
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .when(lado_a_lado, |d| d.w(px(self.lado_do_cartao().max(200.0))).flex_none())
                .child(medidas::secao().child("Último lançamento"))
                .child(cartao)
        });
        let topo = if lado_a_lado {
            div().flex().items_start().gap(px(40.0)).child(populares).children(ultimo)
        } else {
            div().flex().flex_col().gap(px(32.0)).child(populares).children(ultimo)
        };
        let fileira = Arc::new(Vec::new());
        let mut corpo = div().flex().flex_col().child(topo);
        for (k, aba) in [AbaDoArtista::Albuns, AbaDoArtista::Singles, AbaDoArtista::Coletaneas].into_iter().enumerate()
        {
            let juntos: Vec<Junto> = detalhes::lancamentos(albuns, aba).into_iter().map(Junto::so).collect();
            if !juntos.is_empty() {
                corpo = corpo.child(div().mt(px(40.0)).child(self.ver_prateleira(
                    3_000_000 + k,
                    aba.nome(),
                    None,
                    &juntos,
                    &fileira,
                    cx,
                )));
            }
        }
        let outras: Vec<Prateleira> =
            secoes.iter().filter(|s| s.tipo != TipoDaPrateleira::UltimoLancamento).cloned().collect();
        corpo.children(self.prateleiras_do_detalhe(&outras, 10, cx))
    }

    /// Uma aba de lançamentos: a grade de cartões.
    fn grade_de_lancamentos(&self, itens: &[Item], cx: &mut Context<Self>) -> Div {
        let fileira = Arc::new(Vec::new());
        let mut grade = div().flex().flex_wrap().gap(px(16.0));
        for (i, a) in itens.iter().take(self.cartoes_a_mostrar).enumerate() {
            grade = grade.child(self.ver_cartao(7_000 + i, a, SeloDoCartao::default(), Arc::clone(&fileira), cx));
        }
        div().flex().flex_col().gap(px(12.0)).child(grade).children(self.ver_mais_cartoes(
            "lancamentos-ver-mais",
            itens.len(),
            cx,
        ))
    }

    /// Os números do artista, a biografia e as fotos.
    fn sobre_o_artista(&self, detalhe: Option<&Detalhe>) -> Div {
        let c = self.cores();
        let Some(d) = detalhe else { return div() };
        let numero = |valor: u64, nome: &'static str| {
            div()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .child(medidas::secao().child(detalhes::milhar(valor)))
                .child(medidas::apoio().text_color(c.texto_2).child(nome))
        };
        let mut numeros = div().flex().flex_wrap().gap(px(40.0));
        if let Some(n) = d.ouvintes {
            numeros = numeros.child(numero(n, "ouvintes por mês"));
        }
        if let Some(n) = d.seguidores {
            numeros = numeros.child(numero(n, "seguidores"));
        }
        let mut corpo = div().flex().flex_col().gap(px(28.0)).child(numeros);
        if let Some(texto) = d.descricao.as_deref().filter(|t| !t.trim().is_empty()) {
            let mut bio = div().max_w(px(720.0)).flex().flex_col().gap(px(12.0));
            for paragrafo in texto.split('\n').map(str::trim).filter(|p| !p.is_empty()) {
                bio = bio.child(medidas::corpo().text_color(c.texto).child(paragrafo.to_string()));
            }
            corpo = corpo.child(bio);
        }
        if !d.fotos.is_empty() {
            let lado = self.lado_do_cartao();
            let mut fotos = div().flex().flex_wrap().gap(px(16.0));
            for url in &d.fotos {
                fotos = fotos.child(self.capa(Some(url), TipoSalvo::Artista, lado, false));
            }
            corpo = corpo.child(fotos);
        }
        corpo
    }
}
