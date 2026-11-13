//! A Biblioteca: o que foi ouvido com o Player aberto, em números.
//!
//! Tudo vem do histórico local (thread própria, ver `diario.rs`). A página pede o resumo do período ao abrir e de
//! novo a cada poucos segundos; aqui só se desenha o último resumo que ficou pronto.

use super::{RAIO_MODULO, decimal, icone, mono, numero, texto, titulo};
use crate::app::Player;
use crate::diario::{Periodo, Relatorio};
use crate::tema::Cores;
use gpui::{ClickEvent, Context, Div, FontWeight, SharedString, div, prelude::*, px};
use historico::{Execucao, Top, data_civil, dia_local};
use nucleo::{Comando, Pagina};

/// "45 s", "12 min", "3 h 05 min".
pub fn duracao_longa(segundos: f64) -> String {
    let s = if segundos.is_finite() { segundos.max(0.0).round() as u64 } else { 0 };
    match s {
        0..=59 => format!("{s} s"),
        60..=3599 => format!("{} min", s / 60),
        _ if (s / 60) % 60 == 0 => format!("{} h", s / 3600),
        _ => format!("{} h {:02} min", s / 3600, (s / 60) % 60),
    }
}

/// Quando uma música tocou: a hora, se foi hoje; "ontem"; ou o dia e o mês.
pub fn quando(inicio: i64, agora: i64, fuso_s: i64) -> String {
    let (dia, hoje) = (dia_local(inicio, fuso_s), dia_local(agora, fuso_s));
    if dia == hoje {
        let minutos = (inicio + fuso_s).rem_euclid(86_400) / 60;
        format!("{:02}:{:02}", minutos / 60, minutos % 60)
    } else if dia == hoje - 1 {
        "ontem".to_string()
    } else {
        let (_, mes, dia_do_mes) = data_civil(dia);
        format!("{dia_do_mes:02}/{mes:02}")
    }
}

/// "12% a mais que no período anterior", ou vazio quando não há com o que comparar.
pub fn comparacao(agora_s: f64, antes_s: Option<f64>) -> String {
    match antes_s {
        Some(antes) if antes >= 60.0 => {
            let variacao = ((agora_s - antes) / antes * 100.0).round() as i64;
            match variacao {
                0 => "igual ao período anterior".to_string(),
                v if v > 0 => format!("{v}% a mais que no período anterior"),
                v => format!("{}% a menos que no período anterior", -v),
            }
        }
        _ => String::new(),
    }
}

fn ladrilho(c: &Cores, rotulo: &'static str, valor: Div, detalhe: SharedString) -> Div {
    div()
        .flex_1()
        .min_w(px(150.0))
        .p(px(14.0))
        .flex()
        .flex_col()
        .gap(px(6.0))
        .rounded(px(RAIO_MODULO))
        .border_1()
        .border_color(c.linha)
        .child(texto(12.0).text_color(c.texto_3).font_weight(FontWeight::SEMIBOLD).child(rotulo))
        .child(valor)
        .child(texto(12.0).min_h(px(16.0)).text_color(c.texto_2).child(detalhe))
}

fn titulo_de_bloco(c: &Cores, nome: &'static str) -> Div {
    texto(12.0).pb(px(8.0)).text_color(c.texto_3).font_weight(FontWeight::SEMIBOLD).child(nome)
}

/// As 24 horas do dia em barras: a mais alta é a hora em que mais se ouviu.
fn horas(c: &Cores, por_hora: &[f64; 24]) -> Div {
    const ALTURA: f32 = 64.0;
    let maior = por_hora.iter().copied().fold(0.0, f64::max);
    let mut barras = div().h(px(ALTURA)).flex().items_end().gap(px(3.0));
    for segundos in por_hora {
        let parte = if maior > 0.0 { (segundos / maior) as f32 } else { 0.0 };
        let cor = if parte >= 0.999 && maior > 0.0 { c.texto } else { c.texto_3 };
        barras = barras.child(
            div().flex_1().min_w(px(0.0)).h(px((ALTURA * parte).max(3.0))).rounded(px(2.0)).bg(if *segundos > 0.0 {
                cor
            } else {
                c.linha_2
            }),
        );
    }
    let mut marcas = div().flex().justify_between();
    for marca in ["0h", "6h", "12h", "18h", "23h"] {
        marcas = marcas.child(mono().text_color(c.texto_3).child(marca));
    }
    div().flex().flex_col().gap(px(6.0)).child(barras).child(marcas)
}

fn lista_top(c: &Cores, nome: &'static str, itens: &[Top], vazio: &'static str) -> Div {
    let mut coluna = div().flex_1().min_w(px(240.0)).flex().flex_col().child(titulo_de_bloco(c, nome));
    if itens.is_empty() {
        return coluna.child(texto(14.0).text_color(c.texto_2).child(vazio));
    }
    for (i, item) in itens.iter().enumerate() {
        let vezes = if item.vezes == 1 { "1 vez".to_string() } else { format!("{} vezes", item.vezes) };
        let detalhe = if item.detalhe.is_empty() { vezes } else { format!("{} · {vezes}", item.detalhe) };
        coluna = coluna.child(
            div()
                .min_h(px(52.0))
                .py(px(6.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .when(i + 1 < itens.len(), |d| d.border_b_1().border_color(c.linha))
                .child(mono().w(px(16.0)).flex_none().text_color(c.texto_3).child((i + 1).to_string()))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .child(
                            texto(14.0)
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child(SharedString::from(item.nome.clone())),
                        )
                        .child(texto(12.0).text_color(c.texto_2).truncate().child(detalhe)),
                )
                .child(texto(12.0).flex_none().text_color(c.texto_2).child(duracao_longa(item.segundos))),
        );
    }
    coluna
}

fn lista_recentes(c: &Cores, itens: &[Execucao], agora: i64, fuso_s: i64) -> Div {
    let mut coluna = div().flex().flex_col().child(titulo_de_bloco(c, "Últimas músicas"));
    for (i, e) in itens.iter().enumerate() {
        let origem = if e.artista.is_empty() { e.app.clone() } else { format!("{} · {}", e.artista, e.app) };
        coluna = coluna.child(
            div()
                .min_h(px(48.0))
                .py(px(6.0))
                .flex()
                .items_center()
                .gap(px(12.0))
                .when(i + 1 < itens.len(), |d| d.border_b_1().border_color(c.linha))
                .child(
                    div()
                        .flex_1()
                        .min_w(px(0.0))
                        .flex()
                        .flex_col()
                        .child(
                            texto(14.0)
                                .font_weight(FontWeight::SEMIBOLD)
                                .truncate()
                                .child(SharedString::from(e.titulo.clone())),
                        )
                        .child(texto(12.0).text_color(c.texto_2).truncate().child(origem)),
                )
                .child(mono().flex_none().text_color(c.texto_2).child(quando(e.inicio, agora, fuso_s))),
        );
    }
    coluna
}

impl Player {
    pub(crate) fn ver_biblioteca(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let largo = a.heroi.w >= 640.0;
        let lado = if largo { 32.0 } else { 16.0 };
        let largura = a.heroi.w - 2.0 * lado;

        if self.aba_biblioteca == crate::app::AbaBiblioteca::Musicas {
            return self.ver_musicas(cx);
        }
        let abas = self.abas_da_biblioteca(largura, cx);
        let escolhido = Periodo::TODOS.iter().position(|p| *p == self.periodo).unwrap_or(0);
        let mut periodos = self.seletor("seletor-periodo", largura.min(400.0), Periodo::TODOS.len(), escolhido);
        for (i, p) in Periodo::TODOS.into_iter().enumerate() {
            periodos = periodos.child(self.opcao(("periodo", i), p.nome(), p == self.periodo).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.escolher_periodo(p);
                    cx.notify();
                },
            )));
        }

        let cabeca = div()
            .flex()
            .flex_none()
            .flex_col()
            .gap(px(16.0))
            .px(px(lado))
            .pt(px(lado.min(24.0)))
            .child(
                div()
                    .flex()
                    .items_start()
                    .justify_between()
                    .gap(px(12.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .gap(px(4.0))
                            .child(titulo(if largo { 40.0 } else { 28.0 }).child("Biblioteca"))
                            .child(
                                texto(14.0)
                                    .text_color(c.texto_2)
                                    .child("O que você ouviu com o Player aberto. Fica só neste computador."),
                            ),
                    )
                    .child(self.botao_icone(
                        "biblioteca-fechar",
                        "icones/x.svg",
                        false,
                        Comando::AbrirPagina(Pagina::Inicio),
                        cx,
                    )),
            )
            .child(abas)
            .child(periodos);

        let aviso = |caminho: &'static str, frase: &'static str| {
            div()
                .py(px(40.0))
                .flex()
                .flex_col()
                .items_center()
                .gap(px(12.0))
                .child(icone(caminho, 28.0, c.texto_3))
                .child(texto(14.0).max_w(px(420.0)).text_center().text_color(c.texto_2).child(frase))
        };
        let conteudo = match self.relatorio.as_deref().filter(|r| r.periodo == self.periodo) {
            _ if !self.historico_ativo() => aviso(
                "icones/info.svg",
                "O histórico está desligado: o Player não conseguiu usar a pasta de dados dele neste computador.",
            ),
            None => aviso("icones/history.svg", "Somando o que você ouviu..."),
            Some(r) if r.resumo.recentes.is_empty() => aviso(
                "icones/history.svg",
                "Ainda não há nada aqui. Toque uma música em qualquer app com o Player aberto: ela entra no histórico depois de meio minuto.",
            ),
            Some(r) => ver_resumo(&c, r),
        };

        let corpo = div()
            .id("biblioteca-corpo")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px(px(lado))
            .pt(px(20.0))
            .pb(px(lado))
            .child(conteudo);
        div().size_full().flex().flex_col().child(cabeca).child(corpo)
    }
}

/// Os números do período: ladrilhos, horas do dia, mais ouvidos e as últimas músicas.
fn ver_resumo(c: &Cores, r: &Relatorio) -> Div {
    {
        let s = &r.resumo;
        let grande = |valor: String| titulo(28.0).child(valor);

        let ritmo = if s.bpm_medio >= 1.0 {
            div()
                .flex()
                .items_end()
                .gap(px(6.0))
                .child(numero(28.0).child(format!("{:.0}", s.bpm_medio)))
                .child(texto(12.0).pb(px(2.0)).text_color(c.texto_2).font_weight(FontWeight::SEMIBOLD).child("BPM"))
        } else {
            grande("—".into())
        };
        let energia: SharedString = if s.energia_media > 0.0 {
            format!("energia média {}", decimal(s.energia_media)).into()
        } else {
            "".into()
        };
        let execucoes =
            if s.execucoes == 1 { "tocada 1 vez".to_string() } else { format!("tocadas {} vezes", s.execucoes) };
        let dias = if s.sequencia == 1 { "1 dia".to_string() } else { format!("{} dias", s.sequencia) };

        let ladrilhos = div()
            .flex()
            .flex_wrap()
            .gap(px(12.0))
            .child(ladrilho(
                c,
                "Tempo ouvido",
                grande(duracao_longa(s.segundos)),
                comparacao(s.segundos, s.segundos_antes).into(),
            ))
            .child(ladrilho(c, "Músicas", grande(s.musicas.to_string()), execucoes.into()))
            .child(ladrilho(c, "Artistas", grande(s.artistas.to_string()), "".into()))
            .child(ladrilho(c, "Ritmo médio", ritmo, energia))
            .child(ladrilho(c, "Sequência", grande(dias), "seguidos com música".into()));

        let mut pagina = div().flex().flex_col().gap(px(28.0)).child(ladrilhos);
        if s.execucoes == 0 {
            pagina = pagina.child(texto(14.0).text_color(c.texto_2).child("Nada tocou neste período."));
        } else {
            pagina = pagina
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .child(titulo_de_bloco(c, "Em que hora do dia"))
                        .child(horas(c, &s.por_hora)),
                )
                .child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap_x(px(32.0))
                        .gap_y(px(28.0))
                        .child(lista_top(c, "Músicas mais ouvidas", &s.top_musicas, "Nenhuma música ainda."))
                        .child(lista_top(
                            c,
                            "Artistas mais ouvidos",
                            &s.top_artistas,
                            "Os apps de origem não informaram o artista.",
                        )),
                );
        }
        pagina.child(lista_recentes(c, &s.recentes, r.agora, r.fuso_s)).child(
            texto(12.0).text_color(c.texto_3).child("Pastas de música, fila e busca entram aqui numa próxima versão."),
        )
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn duracoes_por_extenso() {
        assert_eq!(duracao_longa(0.0), "0 s");
        assert_eq!(duracao_longa(59.4), "59 s");
        assert_eq!(duracao_longa(60.0), "1 min");
        assert_eq!(duracao_longa(3599.0), "59 min");
        assert_eq!(duracao_longa(3600.0), "1 h");
        assert_eq!(duracao_longa(3.0 * 3600.0 + 5.0 * 60.0 + 40.0), "3 h 05 min");
        assert_eq!(duracao_longa(f64::NAN), "0 s");
    }

    #[test]
    fn quando_tocou() {
        // 12/11/2026 15:00 em Brasília.
        let (agora, fuso) = (1_794_506_400, -3 * 3600);
        assert_eq!(quando(agora - 90 * 60, agora, fuso), "13:30");
        assert_eq!(quando(agora - 20 * 3600, agora, fuso), "ontem");
        assert_eq!(quando(agora - 3 * 86_400, agora, fuso), "09/11");
        // 00:10 de hoje (03:10 UTC): continua sendo hoje.
        assert_eq!(quando(agora - 14 * 3600 - 50 * 60, agora, fuso), "00:10");
    }

    #[test]
    fn comparacao_com_o_periodo_anterior() {
        assert_eq!(comparacao(3600.0, None), "");
        assert_eq!(comparacao(3600.0, Some(0.0)), "", "sem base, sem porcentagem");
        assert_eq!(comparacao(3600.0, Some(1800.0)), "100% a mais que no período anterior");
        assert_eq!(comparacao(900.0, Some(1800.0)), "50% a menos que no período anterior");
        assert_eq!(comparacao(1800.0, Some(1801.0)), "igual ao período anterior");
    }
}
