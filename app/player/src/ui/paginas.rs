//! As páginas dos serviços: o Início (prateleiras de cartões), álbum, playlist e artista, e as peças delas. Toda
//! página abre na hora, com esqueleto, e se enche quando o provedor responde.

use super::{com_alfa, icone, medidas, mono, tempo};
use crate::app::Player;
use crate::juntar::Junto;
use crate::menu::Alvo;
use crate::navegacao::{Espaco, Rota};
use crate::paginas::{Cartao, cartao, janela_de_linhas, rota_do_artista};
use crate::remoto::{PaginaRemota, PedidoDePagina};
use crate::ui::medidas::px;
use biblioteca::TipoSalvo;
use gpui::{ClickEvent, Context, Div, FontWeight, ObjectFit, SharedString, Stateful, div, img, prelude::*};
use provedores::{Capacidade, Faixa, Item};
use std::sync::Arc;

/// Como uma linha de faixa aparece: com o número da posição, com o selo, e as versões dela noutros serviços.
#[derive(Clone, Copy, Default)]
pub(super) struct LinhaDeFaixa<'a> {
    pub numero: bool,
    pub selo: bool,
    pub tambem: &'a [Faixa],
    /// Sem a capa (no álbum ela seria a mesma em toda linha).
    pub sem_capa: bool,
    /// Quantas vezes foi ouvida (quando o serviço manda e há lugar).
    pub ouvidas: bool,
    /// O álbum de cada faixa, numa coluna (quando há lugar).
    pub album: bool,
}

/// A coluna das vezes ouvidas e a do álbum aparecem a partir destas larguras de página; abaixo de `ESTREITA`, o número
/// (das linhas com capa) e o coração vão embora (ficam no menu) e o nome fica com o lugar.
const COM_OUVIDAS: f32 = 640.0;
const COM_ALBUM: f32 = 760.0;
const ESTREITA: f32 = 480.0;

/// O selo de um cartão: se aparece, e as versões da faixa noutros serviços.
#[derive(Clone, Copy, Default)]
pub(super) struct SeloDoCartao<'a> {
    pub mostrar: bool,
    pub tambem: &'a [Faixa],
}

impl<'a> SeloDoCartao<'a> {
    fn servicos(self, item: &'a Item) -> Vec<&'a str> {
        if !self.mostrar {
            return Vec::new();
        }
        std::iter::once(item.servico()).chain(self.tambem.iter().map(|f| f.servico.as_str())).collect()
    }
}

/// O menu de um cartão (montado só quando a pessoa o abre).
pub(super) fn menu_do_cartao(item: &Item, tambem: &[Faixa], fileira: &Arc<Vec<Faixa>>) -> impl Fn() -> Alvo + use<> {
    let (item, tambem, fileira) = (item.clone(), tambem.to_vec(), fileira.clone());
    move || match &item {
        Item::Faixa(f) => {
            Alvo::Faixa { faixa: Box::new(f.clone()), tambem: tambem.clone(), fileira: fileira.clone(), fila: None }
        }
        outro => Alvo::Item(Box::new(outro.clone())),
    }
}

/// Entre um atalho e outro, e a capa dele.
const VAO_DOS_ATALHOS: f32 = 12.0;
const CAPA_DO_ATALHO: f32 = 56.0;

/// Entre um cartão e outro.
const VAO: f32 = 16.0;

/// Altura da linha de faixa.
pub(super) const LINHA: f32 = medidas::LINHA;

/// O lado do cartão numa página de `largura` útil: quantos couberem com o lado mínimo, preenchendo a largura. Página
/// estreita tem cartões menores, e sempre pelo menos dois por linha.
pub(super) fn lado_do_cartao_em(largura: f32) -> f32 {
    let minimo = if largura < 520.0 {
        150.0
    } else if largura < 760.0 {
        160.0
    } else {
        180.0
    };
    let n = ((largura + VAO) / (minimo + VAO)).floor().max(2.0);
    ((largura - VAO * (n - 1.0)) / n).floor().max(96.0)
}

/// O brilho lento do esqueleto (0..1), pelo relógio.
/// O brilho do esqueleto de agora (para as outras telas pulsarem juntas).
pub(super) fn brilho_do_esqueleto() -> f32 {
    brilho()
}

fn brilho() -> f32 {
    let ms = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis());
    let t = (ms % 1600) as f32 / 1600.0;
    0.5 - 0.5 * (t * std::f32::consts::TAU).cos()
}

impl Player {
    /// O lado do cartão nesta folha.
    pub(super) fn lado_do_cartao(&self) -> f32 {
        // A folha e o herói têm borda (1 px de cada lado, cada um).
        lado_do_cartao_em(self.arranjo.heroi.w - 4.0 - 2.0 * self.lado_da_pagina())
    }

    /// Quanto a página de agora já apareceu (0 a 1). Ela só esmaece: nada desliza ao trocar de página.
    pub(super) fn entrada(&self) -> f32 {
        self.anim.seguir(("rota-entra", 0), 1.0, 0.12)
    }

    /// A capa: a imagem baixada, ou a silhueta do tipo enquanto ela não vem (ou se não há).
    pub(super) fn capa(&self, url: Option<&str>, tipo: TipoSalvo, lado: f32, redondo: bool) -> Div {
        let c = self.cores();
        let raio = if redondo { lado / 2.0 } else { medidas::raio_da_capa(lado) };
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
        match url.and_then(|u| self.capas.arquivo_para(u, lado)) {
            // Enquanto decodifica, e se o arquivo não for uma imagem que dê para ler, fica a silhueta.
            // O raio é da imagem: o GPUI só recorta os filhos em retângulo.
            Some(arquivo) => caixa.child(
                img(arquivo)
                    .size(px(lado))
                    .rounded(px(raio))
                    .object_fit(ObjectFit::Cover)
                    .with_loading(silhueta)
                    .with_fallback(silhueta),
            ),
            None => caixa.child(silhueta()),
        }
    }

    /// A capa de um cartão (as Músicas Curtidas, que não têm imagem, ganham um coração).
    pub(super) fn capa_do_cartao(&self, ficha: &Cartao, lado: f32) -> Div {
        if !ficha.curtidas {
            return self.capa(ficha.capa.as_deref(), ficha.tipo, lado, ficha.redondo);
        }
        let c = self.cores();
        div()
            .size(px(lado))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(medidas::raio_da_capa(lado)))
            .bg(c.sel)
            .child(icone("icones/heart-f.svg", (lado * 0.36).round(), c.texto))
    }

    /// Um cartão. A capa cresce um pouco sob o mouse; clicar abre a página do item, ou toca a faixa (com as outras
    /// faixas da fileira na fila). O botão direito e o ⋯ abrem o menu.
    pub(super) fn ver_cartao(
        &self,
        n: usize,
        item: &Item,
        selo: SeloDoCartao,
        fileira: Arc<Vec<Faixa>>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        let servicos = selo.servicos(item);
        let ficha = cartao(item);
        let sob = self.cartao_sob == Some(n);
        let escala = self.anim.seguir(("cartao", n as u32), if sob { 1.03 } else { 1.0 }, 0.16);
        let lado = self.lado_do_cartao() * escala;
        let redondo = ficha.redondo;
        let imagem = div()
            .relative()
            .size(px(self.lado_do_cartao()))
            .flex()
            .items_center()
            .justify_center()
            .child(self.capa_do_cartao(&ficha, lado).when(!redondo, |d| d.shadow_md()))
            .child(self.selo_do_cartao(n, item, &servicos, sob, fileira.clone(), cx))
            .when(sob, |d| {
                d.child(
                    div()
                        .absolute()
                        .top(px(8.0))
                        .right(px(8.0))
                        .rounded(px(medidas::PILULA))
                        .bg(com_alfa(gpui::black(), 0.6))
                        .child(self.botao_mais(
                            ("cartao-mais", n),
                            36.0,
                            menu_do_cartao(item, selo.tambem, &fileira),
                            cx,
                        )),
                )
            });
        let do_menu = menu_do_cartao(item, selo.tambem, &fileira);
        let Cartao { titulo: nome, sub, .. } = ficha;
        let item = item.clone();
        div()
            .id(("cartao", n))
            .map(|d| self.focavel(d, medidas::RAIO_CARTAO))
            .w(px(self.lado_do_cartao()))
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(12.0))
            .cursor_pointer()
            .child(imagem)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .child(
                        medidas::corpo()
                            .font_weight(FontWeight::SEMIBOLD)
                            .when(redondo, |d| d.text_center())
                            .child(super::linha(nome)),
                    )
                    .child(
                        medidas::apoio()
                            .text_color(c.texto_2)
                            .when(redondo, |d| d.text_center())
                            .child(super::linhas(sub, 2)),
                    ),
            )
            .on_hover(cx.listener(move |this, dentro: &bool, _, cx| {
                if *dentro {
                    this.cartao_sob = Some(n);
                } else if this.cartao_sob == Some(n) {
                    this.cartao_sob = None;
                }
                cx.notify();
            }))
            .on_mouse_down(gpui::MouseButton::Right, self.ao_menu(do_menu, cx))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.abrir_ou_tocar(&item, &fileira);
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
                    .w(px(self.lado_do_cartao()))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .size(px(self.lado_do_cartao()))
                            .rounded(px(medidas::raio_da_capa(self.lado_do_cartao())))
                            .bg(com_alfa(c.bloco, a)),
                    )
                    .child(
                        div()
                            .h(px(16.0))
                            .w(px(self.lado_do_cartao() * 0.7))
                            .rounded(px(medidas::raio_fino(8.0)))
                            .bg(com_alfa(c.bloco, a)),
                    )
                    .child(
                        div()
                            .h(px(12.0))
                            .w(px(self.lado_do_cartao() * 0.45))
                            .rounded(px(medidas::raio_fino(8.0)))
                            .bg(com_alfa(c.bloco, a)),
                    ),
            );
        }
        fila
    }

    /// Uma prateleira: o título (e, quando é de um serviço só entre vários, o nome dele), as setas e a fileira de cartões,
    /// que rola de lado.
    pub(super) fn ver_prateleira(
        &self,
        k: usize,
        titulo_: &str,
        de: Option<&str>,
        itens: &[Junto],
        fileira: &Arc<Vec<Faixa>>,
        cx: &mut Context<Self>,
    ) -> Div {
        let c = self.cores();
        let mistura = self.mistura_servicos();
        let rolagem = self.rolagem_da_prateleira(k);
        let mut cartoes =
            div().id(("prateleira", k)).track_scroll(&rolagem).flex().gap(px(VAO)).pb(px(8.0)).overflow_x_scroll();
        // A rodinha desce a página; a prateleira rola de lado só com as setas, o Shift ou o touchpad.
        cartoes.style().restrict_scroll_to_axis = Some(true);
        for (i, junto) in itens.iter().enumerate().take(30) {
            let selo = SeloDoCartao { mostrar: mistura, tambem: &junto.tambem };
            cartoes = cartoes.child(self.ver_cartao(k * 1000 + i, &junto.item, selo, fileira.clone(), cx));
        }
        let (x, maximo) = (f32::from(rolagem.offset().x), f32::from(rolagem.max_offset().width));
        // Uma página de cartões por vez.
        let cada = self.lado_do_cartao() + VAO;
        let visiveis = ((self.arranjo.heroi.w - 2.0 * self.lado_da_pagina() + VAO) / cada).floor().max(1.0);
        let passo = visiveis * cada;
        let seta = |lado: i32, simbolo: &'static str, pode: bool, cx: &mut Context<Self>| {
            let rolagem = rolagem.clone();
            div()
                .id(("prateleira-seta", k * 2 + usize::from(lado > 0)))
                .map(|d| self.focavel(d, medidas::PILULA))
                .size(px(medidas::FICHA))
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .rounded(px(medidas::PILULA))
                .when(pode, |d| d.cursor_pointer().hover(|s| s.bg(c.hover)))
                .when(!pode, |d| d.opacity(0.35))
                .child(icone(simbolo, medidas::ICONE_LINHA, c.texto))
                .on_click(cx.listener(move |_, _: &ClickEvent, _, cx| {
                    let agora = rolagem.offset();
                    // A rolagem é em pixels de verdade; o passo, na medida das telas.
                    let novo = (f32::from(agora.x) - lado as f32 * passo * medidas::escala()).clamp(-maximo, 0.0);
                    rolagem.set_offset(gpui::point(gpui::px(novo), agora.y));
                    cx.notify();
                }))
        };
        // Antes de a fileira ser medida (`maximo` 0), as setas aparecem se há cartões além da tela.
        let rola = maximo > 1.0 || (maximo == 0.0 && itens.len() > visiveis as usize);
        let cabeca = div()
            .flex()
            .items_center()
            .gap(px(16.0))
            .min_h(px(medidas::FICHA))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .items_center()
                    .gap(px(16.0))
                    .child(medidas::secao().min_w(px(0.0)).child(super::linha(titulo_.to_string())))
                    .children(de.filter(|_| mistura).map(|s| self.marca_com_nome(s, self.arranjo.heroi.w >= 520.0))),
            )
            .when(rola, |d| {
                d.child(div().flex().flex_none().child(seta(-1, "icones/chev-l.svg", x < -1.0, cx)).child(seta(
                    1,
                    "icones/chev-r.svg",
                    maximo == 0.0 || -x < maximo - 1.0,
                    cx,
                )))
            });
        div().flex().flex_col().gap(px(12.0)).child(cabeca).child(cartoes)
    }

    /// A frase de erro com "Tentar de novo".
    pub(super) fn erro_da_pagina(&self, n: usize, motivo: &str, pedido: PedidoDePagina, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        div()
            .flex()
            .items_center()
            .gap(px(12.0))
            .child(medidas::apoio().text_color(c.texto_2).child(motivo.to_string()))
            .child(
                medidas::apoio()
                    .id(("pagina-de-novo", n))
                    .h(px(medidas::FICHA))
                    .px(px(16.0))
                    .flex()
                    .items_center()
                    .rounded(px(medidas::PILULA))
                    .bg(c.inv)
                    .text_color(c.inv_texto)
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
        let nome = info.nome.clone();
        let alvo = servico.to_string();
        let pilula = |id: &'static str, rotulo: String, cheio: bool| {
            let (fundo, frente) = if cheio { (c.inv, c.inv_texto) } else { (c.sel, c.texto) };
            medidas::corpo()
                .id((id, n))
                .h(px(medidas::BOTAO))
                .px(px(20.0))
                .flex()
                .items_center()
                .rounded(px(medidas::PILULA))
                .bg(fundo)
                .text_color(frente)
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .hover(|s| s.opacity(0.88))
                .child(rotulo)
        };
        let acao = match &aviso {
            // Renovando a credencial guardada: não há navegador nem o que cancelar.
            Aviso::Entrando { navegador: false } => {
                div().child(medidas::apoio().text_color(c.texto_2).child("Conectando…"))
            }
            Aviso::Entrando { navegador: true } => div()
                .flex()
                .items_center()
                .gap(px(14.0))
                .child(medidas::apoio().text_color(c.texto_2).child("Aprove no navegador que abriu…"))
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
            Aviso::Entrar { erro: Some(e) } => Some(medidas::apoio().text_color(c.texto_2).child(e.clone())),
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
                            .child(medidas::corpo().text_color(c.texto).child(format!("{nome}: entre na sua conta")))
                            .child(acao),
                    )
                    .children(erro),
            );
        }
        let logo = info.logo.clone().map(|arquivo| img(arquivo).size(px(56.0)).rounded(px(medidas::PILULA)));
        Some(
            div()
                .max_w(px(520.0))
                .p(px(24.0))
                .rounded(px(medidas::RAIO_CARTAO))
                .bg(com_alfa(c.bloco, 0.7))
                .flex()
                .flex_col()
                // O botão fica do tamanho do texto (sem esticar na largura do cartão).
                .items_start()
                .gap(px(14.0))
                .children(logo)
                .child(medidas::secao().child(format!("Entre na sua conta do {nome}")))
                .child(
                    medidas::apoio()
                        .text_color(c.texto_2)
                        .child("Abre o navegador; quem entra é você. A senha não passa pelo Player."),
                )
                .child(acao)
                .children(erro),
        )
    }

    /// O Início: a saudação, os atalhos e as prateleiras dos serviços do filtro (em Tudo, juntas), e o que há no
    /// computador.
    pub(crate) fn ver_inicio(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let tudo = self.mistura_servicos();
        let mut corpo = div()
            .opacity(self.entrada())
            .flex()
            .flex_col()
            .gap(px(28.0))
            .px(px(self.lado_da_pagina()))
            .pb(px(24.0))
            .child(self.titulo_da_pagina(crate::juntar::saudacao(crate::app::hora_local().0)));
        if self.nav.espaco() == &Espaco::Computador {
            return corpo.child(self.inicio_do_computador(cx));
        }
        let servicos = self.servicos_no_filtro();
        if servicos.is_empty() {
            let abrindo = self.local.remoto.provedores.iter().any(|p| p.situacao == crate::remoto::Situacao::Abrindo);
            let frase = if abrindo {
                "Abrindo os serviços…"
            } else {
                "Nenhum serviço aberto. Ligue um provedor na Loja, na aba Provedores."
            };
            corpo = corpo.child(medidas::apoio().text_color(c.texto_2).child(frase));
        }
        // Fora da conta, o cartão de entrar (o serviço ainda não tem o que mostrar); os outros entram conforme respondem.
        let mut prontas = Vec::new();
        let mut esperando = Vec::new();
        let mut falhas = Vec::new();
        for (k, (servico, nome)) in servicos.into_iter().enumerate() {
            if let Some(cartao) = self.cartao_da_conta(&servico, tudo, k, cx) {
                corpo = corpo.child(cartao);
                continue;
            }
            let pedido = PedidoDePagina::Inicio { servico: servico.clone() };
            match self.local.remoto.pagina(&pedido) {
                Some(p) if matches!(*p, PaginaRemota::Prateleiras(_)) => prontas.push((servico, p)),
                Some(p) => match &*p {
                    PaginaRemota::Erro(motivo) => falhas.push((k, motivo.clone(), pedido)),
                    _ => esperando.push(nome),
                },
                None => esperando.push(nome),
            }
        }
        let nada = prontas.iter().all(|(_, p)| matches!(&**p, PaginaRemota::Prateleiras(v) if v.is_empty()));
        let inicio = self.inicio_junto(prontas.clone());
        if !inicio.atalhos.is_empty() {
            corpo = corpo.child(self.atalhos_do_inicio(&inicio.atalhos, cx));
        }
        for (k, f) in inicio.fileiras.iter().enumerate() {
            corpo = corpo.child(self.ver_prateleira(k, &f.titulo, f.de.as_deref(), &f.itens, &f.faixas, cx));
        }
        if !esperando.is_empty() {
            let frase = format!("Carregando {}…", esperando.join(" e "));
            corpo = corpo.child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .child(medidas::apoio().text_color(c.texto_2).child(frase))
                    .child(self.esqueleto_de_cartoes(6)),
            );
        } else if nada && !prontas.is_empty() && inicio.fileiras.is_empty() {
            corpo = corpo.child(medidas::apoio().text_color(c.texto_2).child("Nada para o início agora."));
        }
        for (k, motivo, pedido) in falhas {
            corpo = corpo.child(self.erro_da_pagina(k, &motivo, pedido, cx));
        }
        if tudo && self.tem_musicas_no_computador() {
            corpo = corpo.child(self.inicio_do_computador(cx));
        }
        corpo
    }

    /// Os atalhos do Início: a capa, o nome (até duas linhas) e o selo.
    fn atalhos_do_inicio(&self, atalhos: &[Item], cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let util = self.arranjo.heroi.w - 4.0 - 2.0 * self.lado_da_pagina();
        // Só fileiras cheias: duas (três em página estreita, e um por linha onde o nome não caberia).
        let (colunas, cabem) = if util >= 980.0 {
            (4.0, 8)
        } else if util >= 660.0 {
            (3.0, 6)
        } else if util >= 340.0 {
            (2.0, 6)
        } else {
            (1.0, 4)
        };
        let n = colunas as usize;
        let mostrar = if atalhos.len() > n { atalhos.len().min(cabem) / n * n } else { atalhos.len() };
        let largura = ((util - VAO_DOS_ATALHOS * (colunas - 1.0)) / colunas).floor();
        let mut grade = div().flex().flex_wrap().gap(px(VAO_DOS_ATALHOS));
        for (i, item) in atalhos.iter().take(mostrar).enumerate() {
            let ficha = cartao(item);
            let servicos = if self.mistura_servicos() { vec![item.servico()] } else { Vec::new() };
            let capa = self.com_selo(self.capa_do_cartao(&ficha, CAPA_DO_ATALHO), &servicos, medidas::MARCA_DA_LINHA);
            let alvo = item.clone();
            grade = grade.child(
                div()
                    .id(("atalho", i))
                    .map(|d| self.focavel(d, medidas::RAIO_CARTAO))
                    .w(px(largura))
                    .h(px(72.0))
                    .px(px(8.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(16.0))
                    .rounded(px(medidas::RAIO_CARTAO))
                    .bg(com_alfa(c.texto, 0.06))
                    .cursor_pointer()
                    .hover(|s| s.bg(com_alfa(c.texto, 0.11)))
                    .child(capa)
                    .child(
                        medidas::apoio()
                            .flex_1()
                            .min_w(px(0.0))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(super::linhas(ficha.titulo, 2)),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.abrir_ou_tocar(&alvo, &[]);
                        cx.notify();
                    })),
            );
        }
        grade
    }

    /// Há pastas ou playlists no computador?
    fn tem_musicas_no_computador(&self) -> bool {
        !self.local.lista.pastas.is_empty() || !self.local.lista.playlists.is_empty()
    }

    /// O Início do computador: as playlists feitas no Player e as pastas.
    fn inicio_do_computador(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        if !self.tem_musicas_no_computador() {
            return div()
                .flex()
                .flex_col()
                .items_start()
                .gap(px(16.0))
                .child(medidas::apoio().text_color(c.texto_2).child(
                    "Escolha uma pasta com as suas músicas (MP3, FLAC, WAV, OGG, M4A). Os arquivos ficam onde estão.",
                ))
                .child(self.pilula("inicio-pasta", "icones/plus.svg", "Adicionar pasta", true).on_click(cx.listener(
                    |this, _: &ClickEvent, _, cx| {
                        this.escolher_pasta(cx);
                    },
                )));
        }
        let secao = |nome: &'static str, cartoes: Div| {
            div().flex().flex_col().gap(px(12.0)).child(medidas::secao().child(nome)).child(cartoes)
        };
        let mut corpo = div().flex().flex_col().gap(px(28.0));
        let playlists = &self.local.lista.playlists;
        if !playlists.is_empty() {
            let mut grade = div().flex().flex_wrap().gap(px(16.0));
            for (i, p) in playlists.iter().enumerate() {
                grade = grade.child(self.cartao_de_playlist(i, p, cx));
            }
            corpo = corpo.child(secao("Suas playlists", grade));
        }
        let pastas = &self.local.lista.pastas;
        if !pastas.is_empty() {
            let mut grade = div().flex().flex_wrap().gap(px(16.0));
            for (i, (pasta, quantas)) in pastas.iter().enumerate() {
                let lado = self.lado_do_cartao();
                let capa = div()
                    .size(px(lado))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(medidas::raio_da_capa(lado)))
                    .bg(c.bloco)
                    .child(icone("icones/pasta.svg", lado * 0.32, c.texto_3))
                    .into_any_element();
                let nome = pasta
                    .file_name()
                    .map_or_else(|| pasta.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned());
                let alvo = pasta.clone();
                grade = grade.child(
                    self.cartao_local(("cartao-pasta", i), capa, nome, super::estante::quantas_musicas(*quantas))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.abrir_origem(biblioteca::Origem::Pasta(alvo.clone()));
                            cx.notify();
                        })),
                );
            }
            corpo = corpo.child(secao("Suas pastas", grade));
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
        let salvo = self.item_salvo(item);
        let pulso = id.1 as u32;
        let escala = self.anim.seguir(("coracao", pulso), 1.0, 0.3);
        let item = item.clone();
        div()
            .id(id)
            .map(|d| self.focavel(d, medidas::PILULA))
            .size(px(tamanho + 16.0))
            .flex_none()
            .flex()
            .items_center()
            .justify_center()
            .rounded(px(medidas::PILULA))
            .cursor_pointer()
            .hover(|s| s.bg(c.hover))
            .child(icone(
                if salvo { "icones/heart-f.svg" } else { "icones/heart.svg" },
                tamanho * escala,
                if salvo { c.texto } else { c.texto_2 },
            ))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                cx.stop_propagation();
                this.alternar_salvo(&item, pulso);
                cx.notify();
            }))
    }

    /// Botão em pílula: o principal invertido, os outros discretos.
    pub(super) fn pilula(
        &self,
        id: &'static str,
        simbolo: &'static str,
        rotulo: &'static str,
        principal: bool,
    ) -> Stateful<Div> {
        let c = self.cores();
        let (fundo, frente) = if principal { (c.inv, c.inv_texto) } else { (c.sel, c.texto) };
        medidas::corpo()
            .id(id)
            .map(|d| self.focavel(d, medidas::PILULA))
            .h(px(medidas::BOTAO))
            .px(px(20.0))
            .flex()
            .items_center()
            .gap(px(8.0))
            .rounded(px(medidas::PILULA))
            .bg(fundo)
            .text_color(frente)
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .hover(|s| s.opacity(0.88))
            .child(icone(simbolo, medidas::ICONE_LINHA, frente))
            .child(rotulo)
    }

    /// Uma linha de faixa: número, título, artista (link quando dá), duração, coração e, no hover, a seguir e fila.
    pub(super) fn linha_de_faixa(
        &self,
        n: usize,
        i: usize,
        faixas: &Arc<Vec<Faixa>>,
        como: LinhaDeFaixa,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let c = self.cores();
        let f = &faixas[i];
        let servicos: Vec<&str> = if como.selo {
            std::iter::once(f.servico.as_str()).chain(como.tambem.iter().map(|x| x.servico.as_str())).collect()
        } else {
            Vec::new()
        };
        let (da_linha, tambem) = (faixas.clone(), como.tambem.to_vec());
        let do_menu = move || Alvo::Faixa {
            faixa: Box::new(da_linha[i].clone()),
            tambem: tambem.clone(),
            fileira: da_linha.clone(),
            fila: None,
        };
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
                Some(rota) => medidas::apoio()
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
                None => medidas::apoio().text_color(c.texto_2).flex_none().child(rotulo).into_any_element(),
            });
        }
        let item = Item::Faixa(f.clone());
        let pagina = Rota::Faixa(item.clone());
        let (a_seguir, no_fim) = (f.clone(), f.clone());
        let lista = faixas.clone();
        let grupo = "faixa-linha";
        // Por cima da linha, só com o mouse nela: escondidas, as ações não tiram largura do nome.
        let fundo = if esta { c.sel } else { c.hover };
        // Em folha estreita o nome vale mais que a duração, o número e o coração (eles ficam no menu).
        let largura = self.arranjo.heroi.w;
        let estreita = largura < ESTREITA;
        let com_duracao = !estreita;
        let numero = como.numero && !(estreita && !como.sem_capa);
        let ouvidas = f.ouvidas.filter(|_| como.ouvidas && largura >= COM_OUVIDAS);
        let coluna_do_album = (como.album && largura >= COM_ALBUM).then(|| f.album.clone().unwrap_or_default());
        let relogio = if com_duracao { 44.0 + 12.0 } else { 0.0 };
        let acoes = super::acoes_por_cima(grupo, fundo, 4.0 + 36.0 + 12.0 + relogio + 36.0 + 8.0)
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
            ));
        div()
            .id(("faixa", n))
            .map(|d| self.focavel(d, medidas::RAIO_CARTAO))
            .group(grupo)
            .relative()
            .h(px(LINHA))
            .px(px(12.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .rounded(px(medidas::RAIO_CARTAO))
            .cursor_pointer()
            .when(esta, |d| d.bg(c.sel))
            .when(!esta, |d| d.hover(|s| s.bg(c.hover)))
            .when(numero, |d| {
                d.child(div().w(px(28.0)).flex_none().flex().justify_center().child(if esta {
                    icone("icones/wave.svg", medidas::ICONE_LINHA, c.texto).into_any_element()
                } else {
                    mono().text_color(c.texto_3).child((i + 1).to_string()).into_any_element()
                }))
            })
            .when(!como.sem_capa, |d| {
                d.child(self.com_selo(
                    self.capa(f.capa.as_deref(), TipoSalvo::Faixa, medidas::CAPA_DA_LINHA, false),
                    &servicos,
                    medidas::MARCA_DA_LINHA,
                ))
            })
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        // O título abre a página da faixa (a linha, fora dele, toca).
                        div().flex().min_w(px(0.0)).child(
                            medidas::corpo()
                                .id(("faixa-titulo", n))
                                .min_w(px(0.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .hover(|s| s.underline())
                                .child(super::linha(f.titulo.clone()))
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    cx.stop_propagation();
                                    this.abrir_rota(pagina.clone());
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(artistas),
            )
            // Sem a coluna do número, as ondas da que toca ficam à direita.
            .when(esta && !numero, |d| d.child(icone("icones/wave.svg", medidas::ICONE_LINHA, c.texto)))
            .children(coluna_do_album.map(|nome| {
                div().flex_1().min_w(px(0.0)).child(medidas::apoio().text_color(c.texto_2).child(super::linha(nome)))
            }))
            .children(ouvidas.map(|n| {
                mono().w(px(112.0)).flex_none().text_color(c.texto_2).text_right().child(crate::detalhes::milhar(n))
            }))
            .child(acoes)
            .when(!estreita, |d| d.child(self.coracao(("faixa-coracao", 50_000 + n), &item, medidas::ICONE_LINHA, cx)))
            .when(com_duracao, |d| {
                d.child(
                    mono()
                        .w(px(44.0))
                        .flex_none()
                        .text_color(c.texto_3)
                        .text_right()
                        .child(f.duracao_s.map(tempo).unwrap_or_default()),
                )
            })
            .child(self.botao_mais(("faixa-mais", n), 36.0, do_menu.clone(), cx).mr(px(-8.0)))
            .on_mouse_down(gpui::MouseButton::Right, self.ao_menu(do_menu, cx))
            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                this.tocar_faixas(&lista, i);
                cx.notify();
            }))
    }

    /// As primeiras `mostrar` faixas, em linhas. Só as que estão à vista são montadas (as Curtidas podem ter milhares); o
    /// espaço das outras fica.
    pub(super) fn linhas_de_faixas(
        &self,
        faixas: &Arc<Vec<Faixa>>,
        mostrar: usize,
        como: LinhaDeFaixa,
        cx: &mut Context<Self>,
    ) -> Div {
        let topo = self.areas.borrow().get("detalhe-linhas").map(|a| medidas::logico(a.origin.y));
        let folha = &self.arranjo.heroi;
        let janela = janela_de_linhas(topo, (folha.y, folha.y + folha.h), LINHA, mostrar);
        let mut linhas = div()
            .relative()
            .flex()
            .flex_col()
            .child(self.marca("detalhe-linhas"))
            .child(div().flex_none().h(px(LINHA * janela.start as f32)));
        for i in janela.clone() {
            linhas = linhas.child(self.linha_de_faixa(i, i, faixas, como, cx));
        }
        linhas.child(div().flex_none().h(px(LINHA * (mostrar - janela.end) as f32)))
    }

    /// "Ver mais (N)" no pé de uma grade longa, enquanto há cartões escondidos.
    pub(super) fn ver_mais_cartoes(
        &self,
        id: &'static str,
        total: usize,
        cx: &mut Context<Self>,
    ) -> Option<Stateful<Div>> {
        let c = self.cores();
        let faltam = total.checked_sub(self.cartoes_a_mostrar).filter(|n| *n > 0)?;
        let botao = medidas::apoio()
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
}

#[cfg(test)]
mod testes {
    use super::lado_do_cartao_em;

    #[test]
    fn os_cartoes_preenchem_a_largura() {
        for largura in [280.0f32, 344.0, 520.0, 596.0, 760.0, 812.0, 1100.0, 1500.0] {
            let lado = lado_do_cartao_em(largura);
            let n = ((largura + 16.0) / (lado + 16.0)).floor();
            assert!(n >= 2.0, "{largura}: pelo menos dois por linha");
            let sobra = largura - (n * lado + (n - 1.0) * 16.0);
            assert!((0.0..n + 1.0).contains(&sobra), "{largura}: {n} de {lado}, sobram {sobra}");
            assert!(lado <= 260.0, "{largura}: {lado}");
        }
    }
}
