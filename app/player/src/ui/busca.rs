//! A página Busca: o campo (o mesmo texto da Biblioteca) e o que os serviços acharam, por tipo: o melhor resultado,
//! as faixas, os álbuns, os artistas e as playlists, com chips para filtrar. Em Tudo, cada serviço no seu bloco.

use super::paginas::{CARTAO, LINHA};
use super::{com_alfa, texto, titulo};
use crate::app::Player;
use crate::navegacao::{Espaco, Rota};
use crate::paginas::{FiltroDaBusca, GruposDaBusca, agrupar_busca, cartao, rota_do_item, servico_buscando};
use biblioteca::TipoSalvo;
use gpui::{ClickEvent, Context, Div, FontWeight, SharedString, Stateful, div, prelude::*, px};
use provedores::{Faixa, Item};
use std::sync::Arc;

/// Quantas faixas aparecem ao lado do melhor resultado.
const FAIXAS_NO_TOPO: usize = 4;
/// Com a folha pelo menos desta largura, o melhor resultado e as faixas ficam lado a lado (mais estreita, as
/// ações escondidas da linha comiam o título das faixas).
const LADO_A_LADO: f32 = 960.0;
/// A largura máxima do campo de busca.
const CAMPO: f32 = 560.0;

impl Player {
    pub(crate) fn ver_busca(&self, cx: &mut Context<Self>) -> Div {
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
        let consulta = self.busca_musicas.trim().to_string();
        let mut corpo = div()
            .relative()
            .left(px(dx))
            .opacity(p)
            .flex()
            .flex_col()
            .gap(px(20.0))
            .p(px(24.0))
            .child(titulo(32.0).child("Busca"))
            // Largura em pixels: em porcentagem, a grade dos filtros (que quebra linha) encolhia o campo.
            .child(
                div()
                    .w(px(CAMPO.min(self.arranjo.heroi.w - 48.0).max(160.0)))
                    .flex()
                    .child(self.campo_de_busca("Buscar faixas, álbuns, artistas e playlists", cx)),
            );
        if servicos.is_empty() {
            let abrindo = self.local.remoto.provedores.iter().any(|p| p.situacao == crate::remoto::Situacao::Abrindo);
            let frase = if abrindo {
                "Abrindo os serviços…"
            } else {
                "Nenhum serviço aberto. Ligue um provedor na Loja, na aba Provedores."
            };
            return corpo.child(texto(14.0).text_color(c.texto_2).child(frase));
        }
        // Fora da conta, o convite para entrar fica no alto (a busca do serviço vem com a conta).
        for (k, (s, _)) in servicos.iter().enumerate() {
            if let Some(cartao) = self.cartao_da_conta(s, tudo, 100 + k, cx) {
                corpo = corpo.child(cartao);
            }
        }
        if consulta.is_empty() {
            let onde = match servicos.as_slice() {
                [(_, nome)] if !tudo => format!("no {nome}"),
                _ => "nos serviços".to_string(),
            };
            return corpo.child(
                texto(14.0).text_color(c.texto_2).child(format!("Busque faixas, álbuns, artistas e playlists {onde}.")),
            );
        }

        // A busca de agora é de outro texto (a pausa na digitação ainda não passou): o que está na tela é velho e
        // aparece apagado até o novo chegar.
        let busca = &self.local.remoto.busca;
        let velha = busca.texto != consulta;
        let grupos: Vec<(String, String, GruposDaBusca)> = servicos
            .iter()
            .map(|(s, nome)| {
                let g = if busca.texto.is_empty() {
                    GruposDaBusca::default()
                } else {
                    agrupar_busca(&busca.itens, Some(s))
                };
                (s.clone(), nome.clone(), g)
            })
            .collect();

        // Os chips valem para a página inteira: Tudo e os tipos que algum serviço trouxe.
        let mut uniao = GruposDaBusca::default();
        for (_, _, g) in &grupos {
            uniao.faixas.extend(g.faixas.iter().cloned());
            uniao.albuns.extend(g.albuns.iter().cloned());
            uniao.artistas.extend(g.artistas.iter().cloned());
            uniao.playlists.extend(g.playlists.iter().cloned());
        }
        let filtros = uniao.filtros();
        let filtro = if filtros.contains(&self.filtro_busca) { self.filtro_busca } else { FiltroDaBusca::Tudo };
        if !filtros.is_empty() {
            let mut chips = div().flex().flex_wrap().gap(px(8.0));
            for f in filtros {
                let escolhido = f == filtro;
                chips = chips.child(
                    texto(13.0)
                        // Pelo filtro, não pela posição: os chips mudam enquanto os resultados chegam.
                        .id(("busca-chip", f as usize))
                        .h(px(32.0))
                        .px(px(14.0))
                        .flex()
                        .items_center()
                        .rounded(px(999.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        // Como os chips da Biblioteca do serviço.
                        .bg(if escolhido { c.inv } else { c.sel })
                        .text_color(if escolhido { c.inv_texto } else { c.texto })
                        .child(f.nome())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.filtro_busca = f;
                            cx.notify();
                        })),
                );
            }
            corpo = corpo.child(chips);
        }

        // Em Tudo, quantas músicas do computador batem (a Biblioteca mostra todas).
        let lista = &self.local.lista;
        let locais = if lista.filtro.consulta.trim() == consulta { lista.musicas.len() } else { 0 };
        if tudo && locais > 0 {
            let rotulo = if locais == 1 {
                "1 música neste computador".to_string()
            } else {
                format!("{locais} músicas neste computador")
            };
            corpo = corpo.child(
                div()
                    .id("busca-no-computador")
                    .h(px(40.0))
                    .px(px(12.0))
                    .flex()
                    .items_center()
                    .gap(px(10.0))
                    .rounded(px(8.0))
                    .bg(com_alfa(c.texto, 0.04))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(super::icone("icones/lib.svg", 16.0, c.texto_2))
                    .child(texto(13.0).flex_1().font_weight(FontWeight::SEMIBOLD).child(rotulo))
                    .child(super::icone("icones/chev-r.svg", 16.0, c.texto_2))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.abrir_rota(Rota::Biblioteca);
                        cx.notify();
                    })),
            );
        }

        for (k, (servico, nome, g)) in grupos.into_iter().enumerate() {
            let esperando = servico_buscando(&busca.texto, &busca.respondidos, &consulta, &servico);
            let vazio = g.melhor.is_none();
            let erro = busca.erros.iter().find(|e| e.starts_with(&format!("{nome}:"))).cloned();
            let bloco = if vazio && esperando {
                self.esqueleto_de_cartoes(5)
            } else if vazio {
                let frase = erro.unwrap_or_else(|| format!("Nada com esse nome no {nome}."));
                div().child(texto(14.0).text_color(c.texto_2).child(frase))
            } else {
                self.resultados(k, &g, filtro, cx).when(velha, |d| d.opacity(0.5))
            };
            let mostrar_nome = tudo || (esperando && !vazio);
            corpo = corpo.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .when(mostrar_nome, |d| {
                        d.child(
                            div()
                                .flex()
                                .items_center()
                                .gap(px(10.0))
                                .when(tudo, |d| {
                                    d.child(
                                        texto(12.0)
                                            .text_color(c.texto_3)
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child(SharedString::from(nome.to_uppercase())),
                                    )
                                })
                                .when(esperando && !vazio, |d| {
                                    d.child(texto(12.0).text_color(c.texto_3).child("Buscando…"))
                                }),
                        )
                    })
                    .child(bloco),
            );
        }
        corpo
    }

    /// Os resultados de um serviço no filtro escolhido.
    fn resultados(&self, k: usize, g: &GruposDaBusca, filtro: FiltroDaBusca, cx: &mut Context<Self>) -> Div {
        let faixas = Arc::new(g.faixas.clone());
        let grade = |itens: &[Item], cx: &mut Context<Self>| {
            let mut grade = div().flex().flex_wrap().gap(px(20.0));
            for (i, item) in itens.iter().enumerate() {
                grade = grade.child(self.ver_cartao(5_000_000 + k * 1000 + i, item, Arc::new(Vec::new()), cx));
            }
            grade
        };
        match filtro {
            FiltroDaBusca::Faixas => {
                let mut lista = div().flex().flex_col();
                for i in 0..faixas.len() {
                    lista = lista.child(self.linha_de_faixa(k * 100 + i, i, &faixas, cx));
                }
                lista
            }
            FiltroDaBusca::Albuns => grade(&g.albuns, cx),
            FiltroDaBusca::Artistas => grade(&g.artistas, cx),
            FiltroDaBusca::Playlists => grade(&g.playlists, cx),
            FiltroDaBusca::Tudo => {
                let lado_a_lado = self.arranjo.heroi.w >= LADO_A_LADO;
                let mut topo = div().flex().gap(px(24.0)).when(!lado_a_lado, |d| d.flex_col());
                if let Some(melhor) = &g.melhor {
                    topo = topo.child(
                        div()
                            .flex()
                            .flex_col()
                            .gap(px(12.0))
                            .when(lado_a_lado, |d| d.w(px(CARTAO * 2.0)).flex_none())
                            .when(!lado_a_lado, |d| d.max_w(px(CARTAO * 2.6)))
                            .child(texto(18.0).font_weight(FontWeight::BOLD).child("Melhor resultado"))
                            .child(self.melhor_resultado(k, melhor, &faixas, cx)),
                    );
                }
                if !faixas.is_empty() {
                    let mut lista = div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .gap(px(12.0))
                        .child(texto(18.0).font_weight(FontWeight::BOLD).child("Faixas"));
                    let mut linhas = div().flex().flex_col().min_h(px(LINHA));
                    for i in 0..faixas.len().min(FAIXAS_NO_TOPO) {
                        linhas = linhas.child(self.linha_de_faixa(k * 100 + i, i, &faixas, cx));
                    }
                    lista = lista.child(linhas);
                    topo = topo.child(lista);
                }
                let mut tudo = div().flex().flex_col().gap(px(28.0)).child(topo);
                for (j, (nome, itens)) in
                    [("Álbuns", &g.albuns), ("Artistas", &g.artistas), ("Playlists", &g.playlists)]
                        .into_iter()
                        .enumerate()
                {
                    if !itens.is_empty() {
                        tudo = tudo.child(self.ver_prateleira(1000 + k * 10 + j, nome, itens, cx));
                    }
                }
                tudo
            }
        }
    }

    /// O cartão grande do melhor resultado: clicar abre a página dele, ou toca a faixa (com as outras na fila).
    fn melhor_resultado(
        &self,
        k: usize,
        item: &Item,
        faixas: &Arc<Vec<Faixa>>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        let ficha = cartao(item);
        let tipo = match ficha.tipo {
            TipoSalvo::Faixa => "Faixa",
            TipoSalvo::Album => "Álbum",
            TipoSalvo::Artista => "Artista",
            TipoSalvo::Playlist => "Playlist",
        };
        let sub = if ficha.sub.is_empty() || ficha.tipo == TipoSalvo::Artista {
            tipo.to_string()
        } else {
            format!("{tipo} · {}", ficha.sub)
        };
        let (item, faixas) = (item.clone(), faixas.clone());
        div()
            .id(("busca-melhor", k))
            .flex()
            .flex_col()
            .gap(px(14.0))
            .p(px(20.0))
            .rounded(px(12.0))
            .bg(com_alfa(c.texto, 0.05))
            .cursor_pointer()
            .hover(|s| s.bg(com_alfa(c.texto, 0.09)))
            .child(
                self.capa(ficha.capa.as_deref(), ficha.tipo, 96.0, ficha.redondo)
                    .when(!ficha.redondo, |d| d.shadow_md()),
            )
            .child(match &item {
                // O título da faixa abre a página dela (o resto do cartão toca).
                Item::Faixa(_) => {
                    let pagina = crate::navegacao::Rota::Faixa(item.clone());
                    titulo(24.0)
                        .id(("busca-melhor-titulo", k))
                        .line_clamp(2)
                        .hover(|s| s.underline())
                        .child(SharedString::from(ficha.titulo))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            cx.stop_propagation();
                            this.abrir_rota(pagina.clone());
                            cx.notify();
                        }))
                        .into_any_element()
                }
                _ => titulo(24.0).line_clamp(2).child(SharedString::from(ficha.titulo)).into_any_element(),
            })
            .child(texto(13.0).text_color(c.texto_2).truncate().child(sub))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                match rota_do_item(&item) {
                    Some(rota) => this.abrir_rota(rota),
                    None => {
                        if let Item::Faixa(f) = &item {
                            let i = faixas.iter().position(|x| x.id == f.id).unwrap_or(0);
                            this.tocar_faixas(&faixas, i);
                        }
                    }
                }
                cx.notify();
            }))
    }
}
