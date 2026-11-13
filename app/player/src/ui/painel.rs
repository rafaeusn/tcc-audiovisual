//! O painel: os módulos (Ritmo, Energia, Estrutura, Letra, Nesta sessão, Visual, Relógio) ou, quando pedidos, os
//! Ajustes.
//!
//! No modo largo ele fica ao lado do visual; no modo gaveta abre por cima; no empilhado fica abaixo do visual.
//! No modo edição (E) os módulos podem ser arrastados um sobre o outro para trocar de lugar, removidos e
//! adicionados. A ordem fica salva nos ajustes.

use super::{
    RAIO_CONTROLE, RAIO_MODULO, caixa, com_alfa, decimal, ha_quanto, icone, numero, tecla, tempo, texto, titulo,
};
use crate::app::{COLUNAS_ENERGIA, PASSO_DA_LETRA, Player};
use crate::arranjo::{RAIO, Ret};
use crate::letrista::Situacao;
use crate::tema::{self, Cores};
use analise::Rotulo;
use gpui::{
    AnyElement, ClickEvent, Context, CursorStyle, Div, FontWeight, Hsla, MouseButton, MouseDownEvent, SharedString,
    Stateful, canvas, deferred, div, img, linear_color_stop, linear_gradient, prelude::*, px,
};
use nucleo::ajustes::PALETAS;
use nucleo::{Comando, Modelo, Modulo, Skin, Tema};

/// Quantas faixas da sessão aparecem no módulo (o histórico guarda mais).
const VISIVEIS_NA_SESSAO: usize = 6;
/// Pontos por coluna do medidor de energia.
const PONTOS: usize = 5;
/// Espaço entre os módulos e a borda interna de cada um (onde aparece o contorno do modo edição).
const VAO: f32 = 12.0;
const BORDA_DO_MODULO: f32 = 10.0;

fn rgb8(c: [u8; 3]) -> Hsla {
    tema::cor_de(c.map(|v| f32::from(v) / 255.0))
}

fn nivel(r: Rotulo) -> f32 {
    match r {
        Rotulo::Calma => 0.0,
        Rotulo::Ativa => 1.0,
        Rotulo::Intensa => 2.0,
    }
}

pub fn icone_do_modulo(m: Modulo) -> &'static str {
    match m {
        Modulo::Ritmo => "icones/metro.svg",
        Modulo::Energia => "icones/bolt.svg",
        Modulo::Estrutura => "icones/struct.svg",
        Modulo::Sessao => "icones/history.svg",
        Modulo::Visual => "icones/eye.svg",
        Modulo::Relogio => "icones/clock.svg",
        Modulo::Letra => "icones/lyrics.svg",
    }
}

/// Interruptor (ligado/desligado). A bolinha desliza em 200 ms.
fn interruptor(c: &Cores, id: &'static str, ligado: bool, posicao: f32) -> Stateful<Div> {
    let trilho = div().id(id).w(px(44.0)).h(px(24.0)).flex_none().relative().rounded(px(999.0)).cursor_pointer();
    let trilho = if ligado { trilho.bg(c.inv) } else { trilho.border_1().border_color(c.linha_2) };
    trilho.child(
        div()
            .absolute()
            .top(px(if ligado { 4.0 } else { 3.0 }))
            .left(px(3.0 + 21.0 * posicao))
            .size(px(16.0))
            .rounded(px(999.0))
            .bg(if ligado { c.inv_texto } else { c.texto_2 }),
    )
}

/// Linha de um ajuste: ícone, título, explicação e o controle à direita.
fn linha_ajuste(c: &Cores, caminho: &'static str, nome: &'static str, frase: SharedString) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.0))
        .child(
            div()
                .size(px(32.0))
                .flex_none()
                .flex()
                .items_center()
                .justify_center()
                .child(icone(caminho, 20.0, c.texto)),
        )
        .child(
            div()
                .flex_1()
                .min_w(px(0.0))
                .flex()
                .flex_col()
                .child(texto(14.0).font_weight(FontWeight::SEMIBOLD).child(nome))
                .child(texto(14.0).text_color(c.texto_2).child(frase)),
        )
}

fn bloco_ajuste(c: &Cores, ultimo: bool) -> Div {
    div()
        .py(px(20.0))
        .flex()
        .flex_col()
        .flex_none()
        .gap(px(12.0))
        .when(!ultimo, |d| d.border_b_1().border_color(c.linha))
}

fn titulo_secao(c: &Cores, nome: &'static str) -> Div {
    texto(12.0).text_color(c.texto_3).font_weight(FontWeight::SEMIBOLD).child(nome)
}

impl Player {
    pub(crate) fn ver_painel(&self, r: Ret, gaveta: bool, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let c = self.cores();
        let corpo = if self.painel_ajustes { self.ver_ajustes(r.w, cx) } else { self.ver_modulos(r.w, cx) };
        // O alto do painel ganha a cor do momento (a mesma do dock), que some em 220 px.
        let tom = tema::cor_de(self.tom);
        caixa(r)
            .id(if gaveta { "gaveta" } else { "painel" })
            .rounded(px(RAIO))
            .border_1()
            .border_color(if self.edicao { c.linha_2 } else { c.linha })
            .bg(c.painel)
            .overflow_hidden()
            .flex()
            .flex_col()
            // Cliques no painel não passam para o que está por baixo (o visual, na gaveta).
            .occlude()
            // O corte do painel é reto: sem os cantos próprios, o tom vazava para fora da borda arredondada.
            .child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px(0.0))
                    .w_full()
                    .h(px(220.0f32.min(r.h)))
                    .rounded_tl(px(RAIO - 1.0))
                    .rounded_tr(px(RAIO - 1.0))
                    .bg(linear_gradient(
                        180.0,
                        linear_color_stop(com_alfa(tom, if self.imersivo() { 0.5 } else { 1.0 }), 0.0),
                        linear_color_stop(com_alfa(tom, 0.0), 1.0),
                    )),
            )
            .child(corpo)
    }

    /// Escurece o resto da janela enquanto a gaveta está aberta; clicar fora fecha.
    pub(crate) fn ver_fundo_da_gaveta(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let c = self.cores();
        let a = &self.arranjo;
        // Começa abaixo do topo: os botões da janela continuam ao alcance.
        div()
            .id("gaveta-fundo")
            .absolute()
            .left(px(0.0))
            .top(px(a.topo.h))
            .w(px(a.janela.w))
            .h(px((a.janela.h - a.topo.h).max(0.0)))
            .bg(c.escurecer)
            .occlude()
            .on_click(self.ao_clicar(Comando::AlternarPainel, cx))
    }

    // ---------------------------------------------------------------------------------------- módulos

    fn ver_modulos(&self, largura: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let lado = if self.arranjo.compacto || largura < 300.0 { 6.0 } else { 14.0 };
        let interno = (largura - 2.0 - 2.0 * lado).max(120.0);
        let metade = ((interno - VAO) / 2.0).floor();
        let mut grade = div()
            .id("modulos")
            .relative()
            .size_full()
            .overflow_y_scroll()
            .p(px(lado))
            .flex()
            .flex_wrap()
            .content_start()
            .items_start()
            .gap(px(VAO));

        for m in self.ajustes.modulos.iter().copied() {
            grade = grade.child(self.ver_modulo(m, if m.largo() { interno } else { metade }, cx));
        }
        if self.ajustes.modulos.is_empty() && !self.edicao {
            grade = grade.child(
                texto(14.0)
                    .p(px(BORDA_DO_MODULO))
                    .text_color(c.texto_2)
                    .child("Nenhum módulo no painel. Aperte E para adicionar."),
            );
        }
        if self.edicao {
            // O que está fora do painel pode voltar com um clique.
            let fora: Vec<Modulo> = Modulo::TODOS.into_iter().filter(|m| !self.ajustes.modulos.contains(m)).collect();
            let mut linha =
                div().w(px(interno)).flex_none().p(px(BORDA_DO_MODULO)).flex().flex_wrap().items_center().gap(px(8.0));
            linha =
                linha.child(
                    texto(12.0).w_full().text_color(c.texto_3).font_weight(FontWeight::SEMIBOLD).child(
                        if fora.is_empty() { "Todos os módulos estão no painel" } else { "Adicionar ao painel" },
                    ),
                );
            for m in fora {
                linha = linha.child(
                    texto(14.0)
                        .id(("adicionar-modulo", m as usize))
                        .h(px(40.0))
                        .px(px(14.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(999.0))
                        .border_1()
                        .border_color(c.linha_2)
                        .text_color(c.texto_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/plus.svg", 16.0, c.texto_2))
                        .child(m.nome())
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.adicionar_modulo(m);
                            cx.notify();
                        })),
                );
            }
            grade = grade.child(linha);
        }
        grade
    }

    /// Um módulo na grade: cabeçalho, corpo e, no modo edição, o contorno, a alça e o botão de tirar.
    fn ver_modulo(&self, m: Modulo, largura: f32, cx: &mut Context<Self>) -> AnyElement {
        let c = self.cores();
        let (dx, dy) = self.deslocamento_do_modulo(m);
        let arrastando = self.arrasto.as_ref().is_some_and(|a| a.modulo == m);
        let alvo = self.arrasto.as_ref().is_some_and(|a| a.alvo == Some(m));
        let (direita, corpo): (Option<AnyElement>, Div) = match m {
            Modulo::Ritmo => (None, self.corpo_ritmo()),
            Modulo::Energia => self.corpo_energia(),
            Modulo::Estrutura => self.corpo_estrutura(largura - 2.0 * BORDA_DO_MODULO - 2.0),
            Modulo::Sessao => self.corpo_sessao(),
            Modulo::Visual => self.corpo_visual(cx),
            Modulo::Relogio => (None, self.corpo_relogio()),
            Modulo::Letra => (None, self.corpo_letra(cx)),
        };

        let mut cabecalho = div()
            .h(px(24.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(8.0))
            .child(icone(icone_do_modulo(m), 18.0, c.texto_2))
            .child(
                texto(14.0)
                    .min_w(px(0.0))
                    .truncate()
                    .text_color(c.texto_2)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(m.nome()),
            )
            .child(div().flex_1());
        if self.edicao {
            cabecalho = cabecalho.child(icone("icones/grip.svg", 18.0, c.texto_2)).child(
                div()
                    .id(("tirar-modulo", m as usize))
                    .size(px(24.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.sel))
                    .child(icone("icones/x.svg", 14.0, c.texto_2))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.remover_modulo(m);
                        cx.notify();
                    })),
            );
        } else {
            cabecalho = cabecalho.children(direita);
        }

        // Guarda onde o módulo foi desenhado: é assim que o arrasto sabe em cima de quem o ponteiro está.
        let limites = self.limites.clone();
        let marcador = canvas(
            move |area, _, _| {
                limites.borrow_mut().insert(m, area);
            },
            |_, (), _, _| {},
        )
        .absolute()
        .left(px(0.0))
        .top(px(0.0))
        .size_full();

        let mut caixa_do_modulo = div()
            .relative()
            .left(px(dx))
            .top(px(dy))
            .w(px(largura))
            .flex_none()
            .p(px(BORDA_DO_MODULO))
            .rounded(px(RAIO_MODULO))
            .border_1()
            .border_color(gpui::transparent_black())
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(marcador)
            .child(cabecalho)
            .child(corpo.when(self.edicao, |d| d.opacity(0.62)));
        if self.edicao {
            caixa_do_modulo = caixa_do_modulo
                .border_dashed()
                .border_color(if alvo || arrastando { c.texto_2 } else { c.linha_2 })
                .cursor(if arrastando { CursorStyle::ClosedHand } else { CursorStyle::OpenHand })
                .when(alvo, |d| d.bg(c.sel))
                // Levantado: ganha fundo e sombra, e segue o ponteiro de onde foi pego.
                .when(arrastando, |d| d.bg(c.bloco).shadow_lg())
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                        this.pegar_modulo(m, ev.position);
                        cx.notify();
                    }),
                );
        }
        if arrastando {
            // Desenhado por último, por cima dos vizinhos.
            deferred(caixa_do_modulo).with_priority(1).into_any_element()
        } else {
            caixa_do_modulo.into_any_element()
        }
    }

    /// Dígitos do BPM em matriz de pontos. Cada dígito rola até o valor novo em 240 ms (o BPM muda pouco).
    fn digitos_do_bpm(&self, bpm: u32) -> Div {
        const ALTURA: f32 = 44.0;
        const LARGURA: f32 = 29.0;
        let digitos: Vec<u32> = bpm.to_string().chars().filter_map(|d| d.to_digit(10)).collect();
        let n = digitos.len();
        let mut linha = div().flex().h(px(ALTURA));
        for (i, d) in digitos.into_iter().enumerate() {
            // A chave conta da direita: a unidade continua sendo a unidade quando o número ganha uma casa.
            let v = self.anim.seguir(("bpm", (n - 1 - i) as u32), d as f32, 0.24).clamp(0.0, 9.0);
            let (base, resto) = (v.floor(), v.fract());
            let algarismo = |valor: f32, topo: f32| {
                numero(ALTURA)
                    .absolute()
                    .left(px(0.0))
                    .top(px(topo))
                    .w_full()
                    .text_center()
                    .child(format!("{}", valor as u32 % 10))
            };
            linha = linha.child(
                div()
                    .relative()
                    .w(px(LARGURA))
                    .h(px(ALTURA))
                    .overflow_hidden()
                    .child(algarismo(base, -resto * ALTURA))
                    .when(resto > 0.001, |s| s.child(algarismo(base + 1.0, (1.0 - resto) * ALTURA))),
            );
        }
        linha
    }

    fn corpo_ritmo(&self) -> Div {
        let c = self.cores();
        let r = &self.retrato;
        let bpm = r.bpm_exibido();
        let tem_pulso = r.gate > 0.5 && r.conf_batida > 0.15 && bpm >= 1.0;
        if !tem_pulso {
            let frase = if r.gate > 0.5 { "Sem pulso firme neste trecho." } else { "Sem som agora." };
            return div().child(texto(14.0).text_color(c.texto_2).child(frase));
        }
        let cor = tema::cor_da_energia(&c, r.energia);
        let mut batidas = div().h(px(14.0)).flex().items_center().gap(px(10.0));
        for i in 0..4 {
            let acesa = i == self.batida;
            let ponto =
                div().size(px(if acesa { 12.0 } else { 10.0 })).rounded(px(999.0)).bg(if acesa { cor } else { c.sel });
            // O primeiro tempo do compasso tem um anel em volta.
            batidas = batidas.child(
                div()
                    .size(px(14.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .when(i == 0, |d| d.border_1().border_color(c.texto_3))
                    .child(ponto.when(i == 0, |d| d.size(px(if acesa { 10.0 } else { 8.0 })))),
            );
        }
        div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(
                div().flex().items_end().gap(px(6.0)).child(self.digitos_do_bpm(bpm.round() as u32)).child(
                    texto(12.0).pb(px(6.0)).text_color(c.texto_2).font_weight(FontWeight::SEMIBOLD).child("BPM"),
                ),
            )
            .child(batidas)
    }

    fn corpo_energia(&self) -> (Option<AnyElement>, Div) {
        let c = self.cores();
        let r = &self.retrato;
        let com_som = r.gate > 0.5;
        let energia = if com_som { r.energia } else { 0.0 };
        let palavra = if com_som { r.rotulo.nome() } else { "Silêncio" };
        // O número não anima (muda o tempo todo); só a cor da palavra troca, em 300 ms.
        let cor_palavra = if com_som {
            tema::cor_do_nivel(&c, self.anim.seguir(("nivel", 0), nivel(r.rotulo), 0.3))
        } else {
            c.texto_2
        };

        let mut grade = div().flex().items_end().gap(px(3.0));
        for i in 0..COLUNAS_ENERGIA {
            let e = self.energia_recente.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            let acesos = (e * PONTOS as f32).round() as usize;
            let cor = tema::cor_da_energia(&c, e);
            let mut coluna = div().flex_1().min_w(px(0.0)).flex().flex_col().gap(px(3.0));
            for p in (0..PONTOS).rev() {
                coluna =
                    coluna.child(div().w_full().h(px(6.0)).rounded(px(2.0)).bg(if p < acesos { cor } else { c.ponto }));
            }
            grade = grade.child(coluna);
        }
        // Ponto que pulsa uma vez por compasso, ao lado do número.
        let ponto = 8.0 * (1.0 + 0.6 * self.pulso_compasso);
        let direita = com_som.then(|| {
            div()
                .flex()
                .flex_none()
                .items_center()
                .gap(px(8.0))
                .child(super::mono().text_color(c.texto_2).child(decimal(energia)))
                .child(
                    div()
                        .size(px(14.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(div().size(px(ponto)).rounded(px(999.0)).bg(cor_palavra)),
                )
                .into_any_element()
        });
        let corpo = div()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(titulo(24.0).text_color(cor_palavra).child(palavra))
            .child(grade);
        (direita, corpo)
    }

    /// Estrutura da faixa: o nível de energia de agora, desde quando, e a barra com os trechos já ouvidos.
    fn corpo_estrutura(&self, largura: f32) -> (Option<AnyElement>, Div) {
        let c = self.cores();
        let agora = self.tempo_da_faixa();
        let Some(atual) = self.secoes.last().filter(|_| self.retrato.gate > 0.5 || self.midia.tocando) else {
            return (
                None,
                div().child(
                    texto(14.0).text_color(c.texto_2).child("Os trechos da música aparecem aqui enquanto ela toca."),
                ),
            );
        };
        let duracao = self.midia.posicao.filter(|p| p.duracao_s > 1.0).map(|p| p.duracao_s);
        let total = duracao.unwrap_or(agora).max(agora).max(1.0);
        let inicio_da_barra = self.secoes.first().map_or(0.0, |s| s.inicio_s.min(agora));
        let escala = f64::from(largura) / (total - inicio_da_barra).max(1.0);

        let mut barra = div().w_full().h(px(8.0)).flex().gap(px(3.0));
        for (i, s) in self.secoes.iter().enumerate() {
            let fim = self.secoes.get(i + 1).map_or(agora, |p| p.inicio_s);
            let w = (((fim - s.inicio_s).max(0.0) * escala) as f32 - 3.0).max(4.0);
            let cor = tema::cor_do_nivel(&c, nivel(s.rotulo));
            let ultimo = i + 1 == self.secoes.len();
            barra = barra.child(
                div()
                    .w(px(w))
                    .h(px(8.0))
                    .flex_none()
                    .rounded(px(999.0))
                    .bg(com_alfa(cor, if ultimo { 1.0 } else { 0.5 })),
            );
        }
        // O que ainda não tocou (só dá para saber o tamanho quando o app informa a duração).
        if duracao.is_some_and(|d| d - agora > 1.0) {
            barra = barra.child(div().flex_1().min_w(px(4.0)).h(px(8.0)).rounded(px(999.0)).bg(c.ponto));
        }
        let corpo = div()
            .flex()
            .flex_col()
            .gap(px(10.0))
            .child(
                div()
                    .flex()
                    .items_end()
                    .justify_between()
                    .gap(px(12.0))
                    .child(texto(16.0).font_weight(FontWeight::SEMIBOLD).child(atual.rotulo.nome()))
                    .child(
                        texto(12.0).flex_none().text_color(c.texto_2).child(format!("desde {}", tempo(atual.inicio_s))),
                    ),
            )
            .child(barra.overflow_hidden());
        let direita = texto(12.0)
            .flex_none()
            .text_color(c.texto_2)
            .font_weight(FontWeight::MEDIUM)
            .child("Ao vivo")
            .into_any_element();
        (Some(direita), corpo)
    }

    /// Três barrinhas na faixa que está tocando. Seguem o som de verdade e descem no pause.
    fn equalizador(&self, tocando: bool) -> Div {
        let c = self.cores();
        let r = &self.retrato;
        let niveis = [r.energia, r.pulso.max(r.loud * 0.6), r.atividade];
        let mut barras = div().w(px(14.0)).h(px(14.0)).flex().flex_none().items_end().justify_end().gap(px(2.0));
        for (i, v) in niveis.into_iter().enumerate() {
            let alvo = if tocando { 3.0 + 11.0 * v.clamp(0.0, 1.0) } else { 3.0 };
            let h = self.anim.seguir(("equalizador", i as u32), alvo, if tocando { 0.08 } else { 0.22 });
            barras = barras.child(div().w(px(3.0)).h(px(h)).rounded(px(2.0)).bg(tema::cor_da_energia(&c, r.energia)));
        }
        barras
    }

    fn corpo_sessao(&self) -> (Option<AnyElement>, Div) {
        let c = self.cores();
        let total = self.historico.len();
        let cinza = self.ajustes.tema == Tema::Mono;
        let mut lista = div().flex().flex_col();
        for (i, o) in self.historico.iter().take(VISIVEIS_NA_SESSAO).enumerate() {
            let esta = i == 0 && self.midia.faixa.as_ref().is_some_and(|f| f.chave() == o.faixa.chave());
            let capa = match &o.capa {
                Some(imagem) => img(imagem.clone())
                    .size(px(44.0))
                    .flex_none()
                    .rounded(px(RAIO_CONTROLE))
                    .grayscale(cinza)
                    .into_any_element(),
                None => div()
                    .size(px(44.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(RAIO_CONTROLE))
                    .bg(c.bloco)
                    .child(icone("icones/wave.svg", 20.0, c.texto_3))
                    .into_any_element(),
            };
            let artista: SharedString =
                if o.faixa.artista.is_empty() { o.faixa.app.clone().into() } else { o.faixa.artista.clone().into() };
            let direita = if esta {
                self.equalizador(self.midia.tocando && self.retrato.gate > 0.5).into_any_element()
            } else {
                texto(12.0)
                    .flex_none()
                    .text_color(c.texto_2)
                    .font_weight(FontWeight::MEDIUM)
                    .child(ha_quanto(o.quando.elapsed()))
                    .into_any_element()
            };
            lista = lista.child(
                div()
                    .min_h(px(56.0))
                    .py(px(6.0))
                    .flex()
                    .items_center()
                    .gap(px(12.0))
                    .child(capa)
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
                                    .child(SharedString::from(o.faixa.titulo.clone())),
                            )
                            .child(texto(14.0).text_color(c.texto_2).truncate().child(artista)),
                    )
                    .child(direita),
            );
        }
        if total == 0 {
            lista = lista
                .child(texto(14.0).py(px(8.0)).text_color(c.texto_2).child("As músicas desta sessão aparecem aqui."));
        }
        let direita = (total > 0).then(|| {
            texto(12.0)
                .flex_none()
                .text_color(c.texto_2)
                .font_weight(FontWeight::MEDIUM)
                .child(total.to_string())
                .into_any_element()
        });
        (direita, lista)
    }

    fn corpo_visual(&self, cx: &mut Context<Self>) -> (Option<AnyElement>, Div) {
        let c = self.cores();
        let botao = |id: &'static str, caminho: &'static str, rotulo: &'static str, ligado: bool| {
            let cor = if ligado { c.inv_texto } else { c.texto_2 };
            div()
                .id(id)
                .flex_1()
                .min_w(px(0.0))
                .h(px(44.0))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .cursor_pointer()
                .when(ligado, |d| d.bg(c.inv))
                .when(!ligado, |d| d.border_1().border_color(c.linha_2).hover(|s| s.bg(c.hover)))
                .child(icone(caminho, 18.0, cor))
                .child(texto(14.0).text_color(cor).font_weight(FontWeight::SEMIBOLD).truncate().child(rotulo))
        };
        let corpo = div()
            .flex()
            .gap(px(8.0))
            .child(
                botao("rapido-cores", "icones/drop.svg", "Cores reativas", self.ajustes.cores_reativas)
                    .on_click(self.ao_clicar(Comando::AlternarCoresReativas, cx)),
            )
            .child(
                botao("rapido-seguro", "icones/shield.svg", "Modo seguro", self.ajustes.modo_seguro)
                    .on_click(self.ao_clicar(Comando::AlternarModoSeguro, cx)),
            );
        let direita = texto(12.0)
            .flex_none()
            .text_color(c.texto_2)
            .font_weight(FontWeight::MEDIUM)
            .child(self.ajustes.modelo.nome())
            .into_any_element();
        (Some(direita), corpo)
    }

    /// Letra: a linha de agora, a anterior e as duas próximas. Rola sozinha com a música.
    fn corpo_letra(&self, cx: &mut Context<Self>) -> Div {
        /// Altura de cada linha da letra (cabem duas linhas de texto).
        const LINHA: f32 = 48.0;
        const VISIVEIS: f32 = 4.0;
        let c = self.cores();
        let aviso = |frase: &'static str| div().child(texto(14.0).text_color(c.texto_2).child(frase));
        if self.midia.faixa.as_ref().is_none_or(|f| f.titulo.is_empty()) {
            return aviso("Toque uma música em qualquer app para ver a letra.");
        }
        let desta_faixa = self.chave_letra.as_deref() == Some(self.letra.chave.as_str());
        let letra = match (&self.letra.situacao, desta_faixa) {
            (Situacao::Achou(l), true) => l.clone(),
            (Situacao::SemLetra, true) => {
                return aviso("Não achei a letra desta música.").flex().flex_col().items_start().gap(px(10.0)).child(
                    texto(14.0)
                        .id("letra-buscar")
                        .h(px(36.0))
                        .px(px(14.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(999.0))
                        .border_1()
                        .border_color(c.linha_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/search.svg", 16.0, c.texto))
                        .child("Buscar manualmente")
                        .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.abrir_busca_de_letra();
                            cx.notify();
                        })),
                );
            }
            (Situacao::SemRede, true) => return aviso("Sem conexão para buscar a letra. Tento de novo em instantes."),
            _ => return aviso("Procurando a letra..."),
        };
        if letra.linhas.is_empty() {
            return aviso("Faixa instrumental.");
        }

        let n = letra.linhas.len();
        let tempo = self.tempo_da_letra();
        // Com tempos, a linha é a que já começou. Só com texto, a letra rola na proporção da música.
        let atual = match (letra.sincronizada, tempo) {
            (true, Some((t, _))) => letras::linha_em(&letra.linhas, t),
            (false, Some((t, d))) => Some((((t / d).clamp(0.0, 1.0) * n as f64) as usize).min(n - 1)),
            (_, None) => None,
        };
        let destaque = letra.sincronizada && tempo.is_some();
        // A linha de agora fica na segunda posição. Na introdução, a primeira linha já espera ali, sem destaque.
        // Sem a posição da música, a letra fica parada no começo.
        let alvo = match (atual, tempo) {
            (Some(i), _) => i as f32,
            (None, Some(_)) => 0.0,
            (None, None) => 1.0,
        };
        let pos = self.anim.seguir(("letra", 0), alvo, 0.36);

        let mut janela = div().relative().w_full().h(px(LINHA * VISIVEIS)).overflow_hidden();
        let primeira = (pos.floor() as i64 - 1).max(0);
        for k in primeira..(primeira + 7).min(n as i64) {
            let Some(linha) = letra.linhas.get(k as usize) else { break };
            let e_atual = destaque && atual == Some(k as usize);
            let cor = if e_atual {
                c.texto
            } else if destaque && (k as f32 - pos).abs() > 1.5 {
                c.texto_3
            } else {
                c.texto_2
            };
            let conteudo: SharedString =
                if linha.texto.is_empty() { "· · ·".into() } else { linha.texto.clone().into() };
            janela = janela.child(
                div()
                    .absolute()
                    .left(px(0.0))
                    .top(px((k as f32 - pos + 1.0) * LINHA))
                    .w_full()
                    .h(px(LINHA))
                    .flex()
                    .items_center()
                    .child(
                        texto(16.0)
                            .w_full()
                            .line_clamp(2)
                            .text_color(cor)
                            .font_weight(if e_atual { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                            .child(conteudo),
                    ),
            );
        }

        let fonte = if !letra.sincronizada {
            "Letra do LRCLIB, sem os tempos"
        } else if tempo.is_none() {
            "Letra do LRCLIB. Este app não informa a posição da música"
        } else {
            "Letra do LRCLIB"
        };
        let mut rodape = div()
            .min_h(px(28.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .child(texto(12.0).flex_1().min_w(px(0.0)).text_color(c.texto_3).child(fonte))
            // A letra é de outra música? Dá para procurar a certa.
            .child(
                div()
                    .id("letra-trocar")
                    .size(px(28.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone("icones/search.svg", 16.0, c.texto_2))
                    .child(self.marca("letra-trocar"))
                    .on_hover(self.ao_pairar("letra-trocar", None, "Buscar outra letra", cx))
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.abrir_busca_de_letra();
                        cx.notify();
                    })),
            );
        if destaque {
            // Acerto fino: alguns apps informam a posição com atraso.
            let botao = |id: &'static str, caminho: &'static str, passo: f32| {
                div()
                    .id(id)
                    .size(px(28.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(999.0))
                    .cursor_pointer()
                    .hover(|s| s.bg(c.hover))
                    .child(icone(caminho, 16.0, c.texto_2))
                    .child(self.marca(id))
                    .on_hover(self.ao_pairar(
                        id,
                        None,
                        if passo < 0.0 { "Atrasar a letra" } else { "Adiantar a letra" },
                        cx,
                    ))
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.deslocar_letra(passo);
                        cx.notify();
                    }))
            };
            let valor = if self.atraso_letra.abs() < 0.05 {
                "0,0 s".to_string()
            } else {
                format!("{:+.1} s", self.atraso_letra).replace('.', ",")
            };
            rodape = rodape
                .child(botao("letra-atrasar", "icones/minus.svg", -PASSO_DA_LETRA))
                .child(super::mono().w(px(48.0)).flex_none().text_center().text_color(c.texto_2).child(valor))
                .child(botao("letra-adiantar", "icones/plus.svg", PASSO_DA_LETRA));
        }
        div().flex().flex_col().gap(px(8.0)).child(janela).child(rodape)
    }

    fn corpo_relogio(&self) -> Div {
        let c = self.cores();
        let (h, m) = self.hora;
        div()
            .flex()
            .items_center()
            .gap(px(2.0))
            .child(numero(44.0).child(format!("{h:02}")))
            .child(numero(44.0).text_color(c.texto_3).child(":"))
            .child(numero(44.0).child(format!("{m:02}")))
    }

    // ---------------------------------------------------------------------------------------- ajustes

    fn ver_ajustes(&self, largura: f32, cx: &mut Context<Self>) -> Stateful<Div> {
        let c = self.cores();
        let aj = &self.ajustes;
        let lado = if self.arranjo.compacto { 16.0 } else { 24.0 };
        // Largura dos seletores: o painel menos as margens e o recuo que alinha com o texto das linhas.
        let recuo = if largura < 340.0 { 0.0 } else { 44.0 };
        let largura_seletor = (largura - 2.0 - 2.0 * lado - recuo).max(160.0);

        // Qual visual.
        let escolhido = Modelo::TODOS.iter().position(|m| *m == aj.modelo).unwrap_or(0);
        let mut visuais = self.seletor("seletor-visual", largura_seletor, Modelo::TODOS.len(), escolhido);
        for (i, m) in Modelo::TODOS.into_iter().enumerate() {
            visuais = visuais.child(
                self.opcao(("visual", i), m.nome(), aj.modelo == m)
                    .on_click(self.ao_clicar(Comando::TrocarVisual(m), cx)),
            );
        }
        let descricao: SharedString = match aj.modelo {
            Modelo::Spectral => "Nuvem de partículas conduzida pelo diretor automático.",
            Modelo::Fluxo => "Correntes de luz que seguem a energia da música.",
            Modelo::Barras => "O espectro do som em barras, com rastro.",
        }
        .into();
        let visual = bloco_ajuste(&c, false)
            .child(linha_ajuste(&c, "icones/eye.svg", "Visual", descricao))
            .child(div().pl(px(recuo)).child(visuais));

        // O que o Modo Escuta ouve: todo o PC ou um app só.
        let escolhido = aj.escuta_app.clone();
        let frase_escuta: SharedString = if escolhido.is_empty() {
            "Todo o som do PC, de qualquer app.".into()
        } else {
            format!(
                "Só o {}. Anúncios e sons de outros apps não mexem no visual nem no tocando agora.",
                sessao::nome_do_app(&escolhido)
            )
            .into()
        };
        // Os apps com som aberto agora, e o escolhido mesmo que esteja fechado.
        let mut apps = self.apps_com_som.clone();
        if !escolhido.is_empty() && !apps.contains(&escolhido) {
            apps.insert(0, escolhido.clone());
        }
        let opcao_de_fonte = |id: (&'static str, usize), nome: SharedString, ativa: bool, aberto: bool| {
            texto(14.0)
                .id(id)
                .h(px(36.0))
                .px(px(14.0))
                .flex()
                .items_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .border_1()
                .border_color(if ativa { c.texto } else { c.linha_2 })
                .cursor_pointer()
                .font_weight(FontWeight::MEDIUM)
                .text_color(if ativa { c.texto } else { c.texto_2 })
                .when(ativa, |d| d.bg(c.sel))
                .hover(|s| s.bg(c.hover))
                .child(div().size(px(8.0)).rounded(px(999.0)).bg(if aberto { c.calma } else { c.texto_3 }))
                .child(nome)
        };
        let mut fontes = div().flex().flex_wrap().gap(px(8.0)).child(
            opcao_de_fonte(("fonte", 0), "Todo o PC".into(), escolhido.is_empty(), true).on_click(cx.listener(
                |this, _: &ClickEvent, _, cx| {
                    this.escolher_fonte("");
                    cx.notify();
                },
            )),
        );
        for (i, exe) in apps.into_iter().enumerate() {
            let aberto = self.apps_com_som.contains(&exe);
            let nome: SharedString = sessao::nome_do_app(&exe).into();
            let ativa = exe == escolhido;
            fontes = fontes.child(opcao_de_fonte(("fonte", i + 1), nome, ativa, aberto).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.escolher_fonte(&exe);
                    cx.notify();
                },
            )));
        }
        let escuta = bloco_ajuste(&c, false)
            .child(linha_ajuste(&c, "icones/wave.svg", "O que o Modo Escuta ouve", frase_escuta))
            .child(div().pl(px(recuo)).flex().flex_col().gap(px(8.0)).child(fontes).child(
                texto(12.0).text_color(c.texto_3).child(
                    "Aparecem aqui os apps que estão com som aberto agora. Comece a tocar no app para ele aparecer.",
                ),
            ));

        // Cores: seguem a música ou usam uma paleta.
        let frase_cores: SharedString = if aj.cores_reativas {
            "A cor acompanha a energia da música."
        } else {
            "O visual usa a paleta escolhida."
        }
        .into();
        let detalhe_cores = if aj.cores_reativas {
            let item = |cor: Hsla, nome: &'static str| {
                div()
                    .flex()
                    .items_center()
                    .gap(px(8.0))
                    .child(div().size(px(10.0)).rounded(px(999.0)).bg(cor))
                    .child(texto(14.0).text_color(c.texto_2).child(nome))
            };
            div()
                .flex()
                .flex_wrap()
                .gap_x(px(16.0))
                .gap_y(px(8.0))
                .child(item(c.calma, "Calma"))
                .child(item(c.ativa, "Agitada"))
                .child(item(c.intensa, "Intensa"))
        } else {
            let mut amostras = div().flex().flex_wrap().gap(px(10.0));
            for (i, p) in PALETAS.iter().enumerate() {
                let escolhida = aj.paleta == i;
                amostras = amostras.child(
                    div()
                        .id(("paleta", i))
                        .h(px(40.0))
                        .px(px(10.0))
                        .flex()
                        .items_center()
                        .gap(px(8.0))
                        .rounded(px(999.0))
                        .border_1()
                        .border_color(if escolhida { c.texto } else { c.linha_2 })
                        .cursor_pointer()
                        .when(escolhida, |d| d.bg(c.sel))
                        .hover(|s| s.bg(c.hover))
                        .child(
                            div()
                                .flex()
                                .child(div().size(px(16.0)).rounded(px(999.0)).bg(rgb8(p.a)))
                                .child(div().size(px(16.0)).ml(px(-6.0)).rounded(px(999.0)).bg(rgb8(p.b))),
                        )
                        .child(
                            texto(14.0)
                                .text_color(if escolhida { c.texto } else { c.texto_2 })
                                .font_weight(FontWeight::MEDIUM)
                                .child(p.nome),
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.escolher_paleta(i);
                            cx.notify();
                        })),
                );
            }
            amostras
        };
        let pos_cores = self.anim.seguir(("interruptor-cores", 0), f32::from(u8::from(aj.cores_reativas)), 0.2);
        let cores = bloco_ajuste(&c, false)
            .child(
                linha_ajuste(&c, "icones/drop.svg", "Cores reativas", frase_cores).child(
                    interruptor(&c, "aj-cores", aj.cores_reativas, pos_cores)
                        .on_click(self.ao_clicar(Comando::AlternarCoresReativas, cx)),
                ),
            )
            .child(div().pl(px(recuo)).child(detalhe_cores));

        // Modo seguro.
        let pos_seguro = self.anim.seguir(("interruptor-seguro", 0), f32::from(u8::from(aj.modo_seguro)), 0.2);
        let mut seguro = bloco_ajuste(&c, false).child(
            linha_ajuste(
                &c,
                "icones/shield.svg",
                "Modo seguro",
                "Para quem tem sensibilidade a luzes piscando.".into(),
            )
            .child(
                interruptor(&c, "aj-seguro", aj.modo_seguro, pos_seguro)
                    .on_click(self.ao_clicar(Comando::AlternarModoSeguro, cx)),
            ),
        );
        if aj.modo_seguro {
            let mut regras = div().pl(px(recuo)).flex().flex_col().gap(px(8.0));
            for regra in
                ["Sem clarão no drop", "O brilho e a cor mudam devagar", "Movimento mais calmo, sem pulso na interface"]
            {
                regras = regras.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(10.0))
                        .child(icone("icones/check.svg", 16.0, c.texto))
                        .child(texto(14.0).text_color(c.texto_2).child(regra)),
                );
            }
            seguro = seguro.child(regras);
        }

        // Aparência: tema e skin.
        let tema_atual = Tema::TODOS.iter().position(|t| *t == aj.tema).unwrap_or(0);
        let mut temas = self.seletor("seletor-tema", largura_seletor, Tema::TODOS.len(), tema_atual);
        for (i, t) in Tema::TODOS.into_iter().enumerate() {
            temas = temas.child(
                self.opcao(("tema", i), t.nome(), aj.tema == t).on_click(self.ao_clicar(Comando::TrocarTema(t), cx)),
            );
        }
        let imersiva = aj.skin == Skin::Imersiva;
        let mut skins = self.seletor("seletor-skin", largura_seletor, 2, usize::from(imersiva));
        for (i, s) in [Skin::Padrao, Skin::Imersiva].into_iter().enumerate() {
            skins = skins.child(self.opcao(("skin", i), s.nome(), aj.skin == s).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.trocar_skin(s);
                    cx.notify();
                },
            )));
        }
        let aparencia = bloco_ajuste(&c, false)
            .child(linha_ajuste(
                &c,
                "icones/tone.svg",
                "Tema",
                "As cores da interface. O visual só muda no Mono.".into(),
            ))
            .child(div().pl(px(recuo)).child(temas))
            .child(linha_ajuste(
                &c,
                "icones/immersive.svg",
                "Skin",
                if imersiva {
                    "Imersiva: o visual é o fundo e os painéis são de vidro."
                } else {
                    "Padrão: visual e painel lado a lado."
                }
                .into(),
            ))
            .child(div().pl(px(recuo)).child(skins))
            .child(
                div().pl(px(recuo)).child(
                    texto(14.0)
                        .id("aj-editar")
                        .h(px(40.0))
                        .px(px(16.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .gap(px(8.0))
                        .rounded(px(999.0))
                        .border_1()
                        .border_color(c.linha_2)
                        .text_color(c.texto)
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer()
                        .hover(|s| s.bg(c.hover))
                        .child(icone("icones/grid.svg", 18.0, c.texto))
                        .child("Personalizar o layout")
                        .child(tecla(&c, "E"))
                        .on_click(self.ao_clicar(Comando::EditarLayout, cx)),
                ),
            );

        // Janela: continuar na bandeja ao fechar e a janela mini.
        let pos_bandeja = self.anim.seguir(("interruptor-bandeja", 0), f32::from(u8::from(aj.fechar_na_bandeja)), 0.2);
        let pos_dicas = self.anim.seguir(("interruptor-dicas", 0), f32::from(u8::from(aj.dicas)), 0.2);
        let pilula = |id: &'static str, caminho: &'static str, rotulo: &'static str| {
            texto(14.0)
                .id(id)
                .h(px(40.0))
                .px(px(16.0))
                .flex()
                .items_center()
                .justify_center()
                .gap(px(8.0))
                .rounded(px(999.0))
                .border_1()
                .border_color(c.linha_2)
                .text_color(c.texto)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.bg(c.hover))
                .child(icone(caminho, 18.0, c.texto))
                .child(rotulo)
        };
        let janela = bloco_ajuste(&c, false)
            .child(
                linha_ajuste(
                    &c,
                    "icones/panel.svg",
                    "Continuar na bandeja",
                    if aj.fechar_na_bandeja {
                        "Fechar a janela só a esconde. O Player segue ouvindo; para sair, use o ícone ao lado do relógio."
                    } else {
                        "Fechar a janela encerra o Player."
                    }
                    .into(),
                )
                .child(
                    interruptor(&c, "aj-bandeja", aj.fechar_na_bandeja, pos_bandeja)
                        .on_click(self.ao_clicar(Comando::AlternarBandeja, cx)),
                ),
            )
            .child(
                linha_ajuste(
                    &c,
                    "icones/info.svg",
                    "Dicas dos botões",
                    if aj.dicas {
                        "Parar o mouse num ícone mostra o nome e o atalho dele."
                    } else {
                        "Os ícones não mostram nome nem atalho."
                    }
                    .into(),
                )
                .child(
                    interruptor(&c, "aj-dicas", aj.dicas, pos_dicas)
                        .on_click(self.ao_clicar(Comando::AlternarDicas, cx)),
                ),
            )
            .child(
                div().pl(px(recuo)).child(
                    pilula("aj-mini", "icones/expand.svg", "Janela mini")
                        .child(tecla(&c, "N"))
                        .on_click(self.ao_clicar(Comando::AlternarMini, cx)),
                ),
            );

        // Atalhos: os mesmos comandos da barra de comandos, direto da lista de comandos.
        let com_atalho: Vec<Comando> = Comando::TODOS.into_iter().filter(|k| !k.teclas().is_empty()).collect();
        let mut atalhos =
            bloco_ajuste(&c, false).gap(px(0.0)).child(div().pb(px(8.0)).child(titulo_secao(&c, "Atalhos")));
        for (i, comando) in com_atalho.iter().enumerate() {
            let mut grupo = div().flex().flex_none().gap(px(4.0));
            for t in comando.teclas() {
                grupo = grupo.child(tecla(&c, t));
            }
            atalhos = atalhos.child(
                div()
                    .min_h(px(44.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(12.0))
                    .when(i + 1 < com_atalho.len(), |d| d.border_b_1().border_color(c.linha))
                    .child(texto(14.0).min_w(px(0.0)).text_color(c.texto_2).child(comando.nome()))
                    .child(grupo),
            );
        }

        // Sobre.
        let placa: SharedString = if self.placa.is_empty() {
            "Placa de vídeo: indisponível".into()
        } else {
            format!("Placa de vídeo: {}", self.placa).into()
        };
        let sobre = bloco_ajuste(&c, true)
            .gap(px(8.0))
            .child(titulo_secao(&c, "Sobre"))
            .child(texto(14.0).child(format!("Player {} (nome provisório)", env!("CARGO_PKG_VERSION"))))
            .child(texto(14.0).text_color(c.texto_2).child(
                "No Modo Escuta o app analisa o som que o PC está tocando para mover o visual. Nenhum áudio é gravado nem enviado para fora. O histórico fica só neste computador. Com o módulo Letra no painel, o título e o artista da faixa vão ao LRCLIB para buscar a letra.",
            ))
            .child(texto(14.0).text_color(c.texto_2).child(placa))
            .child(
                div().pt(px(4.0)).flex().child(
                    pilula("aj-diagnostico", "icones/info.svg", "Copiar relatório de diagnóstico")
                        .on_click(self.ao_clicar(Comando::CopiarDiagnostico, cx)),
                ),
            );

        let cabeca = div()
            .min_h(px(40.0))
            .flex()
            .flex_none()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .child(titulo(28.0).child("Ajustes"))
            .child(self.botao_icone("aj-fechar", "icones/x.svg", false, Comando::AbrirAjustes, cx));

        div()
            .id("ajustes")
            .relative()
            .size_full()
            .overflow_y_scroll()
            .px(px(lado))
            .pt(px(20.0))
            .pb(px(24.0))
            .flex()
            .flex_col()
            .child(cabeca)
            .child(visual)
            .child(escuta)
            .child(cores)
            .child(seguro)
            .child(aparencia)
            .child(janela)
            .child(atalhos)
            .child(sobre)
    }
}
