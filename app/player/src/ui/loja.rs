//! A Loja: módulos, temas, skins, layouts e pacotes.
//!
//! Nesta versão tudo o que já vem com o app pode ser aplicado daqui (temas, skins, layouts, pacotes e os módulos
//! do painel). Os módulos que fazem mais que desenhar (usar a internet, entrar numa conta) têm uma ficha: ela
//! mostra o que o módulo pede, com o motivo de cada coisa, e é ali que se instala. Os que ainda não têm código
//! aparecem com a ficha completa, mas sem o botão de instalar: a Loja não finge que instala.

use super::{RAIO_CONTROLE, RAIO_MODULO, com_alfa, icone, texto, titulo};
use crate::app::{AbaLoja, Player};
use crate::tema::{self, Cores};
use gpui::{ClickEvent, Context, Div, FontWeight, Hsla, Stateful, div, prelude::*, px};
use nucleo::{Comando, Modulo, Pagina, Skin, Tema};

const ABAS: [(AbaLoja, &str); 5] = [
    (AbaLoja::Modulos, "Módulos"),
    (AbaLoja::Temas, "Temas"),
    (AbaLoja::Skins, "Skins"),
    (AbaLoja::Layouts, "Layouts"),
    (AbaLoja::Pacotes, "Pacotes"),
];

/// (nome, o que faz) dos itens que ainda não dá para instalar.
const EM_BREVE: [(&str, &str); 5] = [
    ("Perfil de Escuta: Spotify", "Ensina o app a reconhecer o Spotify e a ouvir só ele."),
    ("Perfil de Escuta: YouTube Music", "Reconhece o YouTube Music no navegador e ouve só essa aba."),
    ("Perfil de Escuta: Deezer", "Reconhece o Deezer e ouve só ele."),
    ("Perfil de Escuta: Apple Music", "Reconhece o Apple Music e ouve só ele."),
    ("Perfil de Escuta: Tidal", "Reconhece o Tidal e ouve só ele."),
];

/// (nome, descrição, painel aberto, painel à esquerda)
const LAYOUTS: [(&str, &str, bool, bool); 3] = [
    ("Padrão", "Visual e painel à direita", true, false),
    ("Espelhado", "Painel à esquerda do visual", true, true),
    ("Foco", "Só o visual e o dock", false, false),
];

struct Pacote {
    nome: &'static str,
    descricao: &'static str,
    tema: Tema,
    skin: Skin,
    painel_aberto: bool,
}

const PACOTES: [Pacote; 2] = [
    Pacote {
        nome: "Noite completo",
        descricao: "Noite, Padrão e painel aberto",
        tema: Tema::Noite,
        skin: Skin::Padrao,
        painel_aberto: true,
    },
    Pacote {
        nome: "Vidro escuro",
        descricao: "Grafite, Imersiva e Foco",
        tema: Tema::Grafite,
        skin: Skin::Imersiva,
        painel_aberto: false,
    },
];

/// Miniatura da janela: o visual, o painel (de um lado ou de nenhum) e o dock, nas cores de um tema.
fn miniatura(c: &Cores, contorno: Hsla, imersiva: bool, painel: Option<bool>) -> Div {
    let bloco_painel = || {
        let b = div().w(px(44.0)).h_full().flex_none().rounded(px(5.0));
        if imersiva { b.bg(com_alfa(c.texto, 0.16)) } else { b.border_1().border_color(c.linha_2) }
    };
    let visual = div()
        .flex_1()
        .h_full()
        .rounded(px(5.0))
        .flex()
        .items_center()
        .justify_center()
        .when(!imersiva, |d| d.bg(gpui::black()).border_1().border_color(c.linha))
        .child(div().size(px(14.0)).rounded(px(999.0)).bg(c.ativa));
    div()
        .w_full()
        .h(px(104.0))
        .flex_none()
        .rounded(px(RAIO_MODULO))
        .border_1()
        .border_color(contorno)
        .bg(if imersiva { gpui::black() } else { c.parede })
        .p(px(8.0))
        .flex()
        .flex_col()
        .gap(px(5.0))
        .overflow_hidden()
        .child(
            div()
                .flex_1()
                .min_h(px(0.0))
                .flex()
                .gap(px(5.0))
                .when(painel == Some(true), |d| d.child(bloco_painel()))
                .child(visual)
                .when(painel == Some(false), |d| d.child(bloco_painel())),
        )
        .child(div().h(px(12.0)).flex_none().rounded(px(6.0)).bg(tema::cor_de(c.dock)))
}

impl Player {
    /// Cartão de escolha: a miniatura, o nome, uma linha de descrição e o selo "Em uso" ou "Aplicar".
    fn cartao(
        &self,
        id: (&'static str, usize),
        previa: Div,
        nome: &'static str,
        descricao: &'static str,
        em_uso: bool,
    ) -> Stateful<Div> {
        let c = self.cores();
        let selo = texto(12.0)
            .h(px(24.0))
            .px(px(10.0))
            .flex()
            .flex_none()
            .items_center()
            .rounded(px(999.0))
            .font_weight(FontWeight::SEMIBOLD);
        let selo = if em_uso {
            selo.bg(c.inv).text_color(c.inv_texto).child("Em uso")
        } else {
            selo.border_1().border_color(c.linha_2).text_color(c.texto_2).child("Aplicar")
        };
        div()
            .id(id)
            .w(px(212.0))
            .flex_none()
            .p(px(8.0))
            .flex()
            .flex_col()
            .gap(px(10.0))
            .rounded(px(RAIO_MODULO + 6.0))
            .cursor_pointer()
            .when(em_uso, |d| d.bg(c.sel))
            .when(!em_uso, |d| d.hover(|s| s.bg(c.hover)))
            .child(previa)
            .child(
                div()
                    .px(px(4.0))
                    .pb(px(4.0))
                    .flex()
                    .items_start()
                    .gap(px(8.0))
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(texto(14.0).font_weight(FontWeight::SEMIBOLD).truncate().child(nome))
                            .child(texto(12.0).text_color(c.texto_2).child(descricao)),
                    )
                    .child(selo),
            )
    }

    /// Linha de item da aba Módulos: ícone, nome, descrição e o que dá para fazer com ele.
    fn linha_de_modulo(
        &self,
        caminho: &'static str,
        nome: &'static str,
        descricao: &'static str,
        acao: gpui::AnyElement,
    ) -> Div {
        let c = self.cores();
        div()
            .min_h(px(64.0))
            .py(px(10.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(14.0))
            .border_b_1()
            .border_color(c.linha)
            .child(
                div()
                    .size(px(44.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(RAIO_CONTROLE + 2.0))
                    .bg(c.bloco)
                    .child(icone(caminho, 22.0, c.texto)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(texto(14.0).font_weight(FontWeight::SEMIBOLD).child(nome))
                    .child(texto(14.0).text_color(c.texto_2).child(descricao)),
            )
            .child(acao)
    }

    /// Linha de um módulo do catálogo: ícone da categoria, nome, descrição (em até duas linhas) e o selo.
    fn linha_do_catalogo(&self, m: &modulos::Manifesto, selo: gpui::AnyElement) -> Div {
        let c = self.cores();
        div()
            .min_h(px(64.0))
            .py(px(10.0))
            .px(px(4.0))
            .flex()
            .items_center()
            .gap(px(14.0))
            .border_b_1()
            .border_color(c.linha)
            .child(
                div()
                    .size(px(44.0))
                    .flex_none()
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(RAIO_CONTROLE + 2.0))
                    .bg(c.bloco)
                    .child(icone(super::modulo::icone_da_categoria(m.categoria), 22.0, c.texto)),
            )
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
                            .child(gpui::SharedString::from(m.nome.clone())),
                    )
                    .child(
                        texto(14.0)
                            .text_color(c.texto_2)
                            .line_clamp(2)
                            .child(gpui::SharedString::from(m.descricao.clone())),
                    ),
            )
            .child(selo)
    }

    pub(crate) fn ver_loja(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let lado = if a.heroi.w >= 640.0 { 32.0 } else { 16.0 };
        let largura = a.heroi.w - 2.0 * lado;

        let escolhida = ABAS.iter().position(|(aba, _)| *aba == self.aba_loja).unwrap_or(0);
        let mut abas = self.seletor("seletor-loja", largura.min(520.0), ABAS.len(), escolhida);
        for (i, (aba, rotulo)) in ABAS.into_iter().enumerate() {
            abas = abas.child(self.opcao(("loja-aba", i), rotulo, aba == self.aba_loja).on_click(cx.listener(
                move |this, _: &ClickEvent, _, cx| {
                    this.aba_loja = aba;
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
                            .child(titulo(if a.heroi.w >= 640.0 { 40.0 } else { 28.0 }).child("Loja"))
                            .child(texto(14.0).text_color(c.texto_2).child(
                                "Módulos, temas, skins e layouts. O que já vem com o app se aplica com um clique.",
                            )),
                    )
                    .child(self.botao_icone(
                        "loja-fechar",
                        "icones/x.svg",
                        false,
                        Comando::AbrirPagina(Pagina::Inicio),
                        cx,
                    )),
            )
            .child(abas);

        let mut grade = div().flex().flex_wrap().gap(px(12.0));
        let mut lista = div().flex().flex_col();
        match self.aba_loja {
            AbaLoja::Temas => {
                for (i, t) in Tema::TODOS.into_iter().enumerate() {
                    let ct = tema::cores(t);
                    let descricao = match t {
                        Tema::Noite => "Preto, para o visual brilhar",
                        Tema::Grafite => "Cinza escuro, mais suave",
                        Tema::Papel => "Claro, para ambientes iluminados",
                        Tema::Mono => "Sem cor: interface, visual e capas",
                    };
                    grade = grade.child(
                        self.cartao(
                            ("loja-tema", i),
                            miniatura(&ct, c.linha_2, false, Some(false)),
                            t.nome(),
                            descricao,
                            self.ajustes.tema == t,
                        )
                        .on_click(self.ao_clicar(Comando::TrocarTema(t), cx)),
                    );
                }
            }
            AbaLoja::Skins => {
                let ct = tema::cores(self.ajustes.tema);
                for (i, (skin, descricao)) in [
                    (Skin::Padrao, "Moldura, visual e painel separados por filete"),
                    (Skin::Imersiva, "O visual vira o fundo e os painéis viram vidro"),
                ]
                .into_iter()
                .enumerate()
                {
                    let imersiva = skin == Skin::Imersiva;
                    let cores_da_previa = if imersiva { tema::cores_da_skin(self.ajustes.tema, true) } else { ct };
                    grade = grade.child(
                        self.cartao(
                            ("loja-skin", i),
                            miniatura(&cores_da_previa, c.linha_2, imersiva, Some(false)),
                            skin.nome(),
                            descricao,
                            self.ajustes.skin == skin,
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            this.trocar_skin(skin);
                            cx.notify();
                        })),
                    );
                }
            }
            AbaLoja::Layouts => {
                let ct = tema::cores(self.ajustes.tema);
                let aj = &self.ajustes;
                for (i, (nome, descricao, aberto, esquerda)) in LAYOUTS.into_iter().enumerate() {
                    let em_uso = aj.painel_aberto == aberto && (!aberto || aj.painel_a_esquerda == esquerda);
                    grade = grade.child(
                        self.cartao(
                            ("loja-layout", i),
                            miniatura(&ct, c.linha_2, false, aberto.then_some(esquerda)),
                            nome,
                            descricao,
                            em_uso,
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            // "Foco" só fecha o painel: o lado escolhido antes continua guardado.
                            let lado = if aberto { esquerda } else { this.ajustes.painel_a_esquerda };
                            this.aplicar_layout(aberto, lado);
                            cx.notify();
                        })),
                    );
                }
            }
            AbaLoja::Pacotes => {
                let aj = &self.ajustes;
                for (i, p) in PACOTES.iter().enumerate() {
                    let em_uso = aj.tema == p.tema && aj.skin == p.skin && aj.painel_aberto == p.painel_aberto;
                    let imersiva = p.skin == Skin::Imersiva;
                    let cores_da_previa = tema::cores_da_skin(p.tema, imersiva);
                    grade = grade.child(
                        self.cartao(
                            ("loja-pacote", i),
                            miniatura(&cores_da_previa, c.linha_2, imersiva, p.painel_aberto.then_some(false)),
                            p.nome,
                            p.descricao,
                            em_uso,
                        )
                        .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                            let p = &PACOTES[i];
                            this.trocar_tema(p.tema);
                            this.trocar_skin(p.skin);
                            let lado = this.ajustes.painel_a_esquerda;
                            this.aplicar_layout(p.painel_aberto, lado);
                            this.avisar(format!("Pacote {} aplicado", p.nome));
                            cx.notify();
                        })),
                    );
                }
            }
            AbaLoja::Modulos => {
                lista =
                    lista.child(texto(16.0).pb(px(4.0)).font_weight(FontWeight::SEMIBOLD).child("Módulos do painel"));
                for m in Modulo::TODOS {
                    let no_painel = self.ajustes.modulos.contains(&m);
                    let botao = texto(14.0)
                        .id(("loja-modulo", m as usize))
                        .h(px(36.0))
                        .px(px(14.0))
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(6.0))
                        .rounded(px(999.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer();
                    let botao = if no_painel {
                        botao
                            .border_1()
                            .border_color(c.linha_2)
                            .text_color(c.texto_2)
                            .hover(|s| s.bg(c.hover))
                            .child(icone("icones/check.svg", 16.0, c.texto_2))
                            .child("No painel")
                    } else {
                        botao.bg(c.inv).text_color(c.inv_texto).hover(|s| s.opacity(0.9)).child("Adicionar")
                    };
                    let descricao = match m {
                        Modulo::Ritmo => "BPM e o tempo do compasso",
                        Modulo::Energia => "Calma, agitada ou intensa, e os últimos segundos",
                        Modulo::Estrutura => "Os trechos da música, enquanto ela toca",
                        Modulo::Sessao => "O que você ouviu desde que abriu o app",
                        Modulo::Visual => "Cores reativas e modo seguro à mão",
                        Modulo::Relogio => "A hora, em matriz de pontos",
                        Modulo::Letra => "A letra acompanha a música. Usa a internet: instalar mostra o que ela pede",
                    };
                    lista = lista.child(
                        self.linha_de_modulo(
                            super::painel::icone_do_modulo(m),
                            m.nome(),
                            descricao,
                            botao
                                .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                    if this.ajustes.modulos.contains(&m) {
                                        this.remover_modulo(m);
                                    } else {
                                        this.adicionar_modulo(m);
                                    }
                                    cx.notify();
                                }))
                                .into_any_element(),
                        ),
                    );
                }
                lista = lista.child(
                    texto(16.0).pt(px(24.0)).pb(px(4.0)).font_weight(FontWeight::SEMIBOLD).child("Para instalar"),
                );
                for (i, m) in self.catalogo.iter().enumerate() {
                    let instalado = self.instalados.achar(&m.id).is_some();
                    let selo = texto(12.0)
                        .h(px(24.0))
                        .px(px(10.0))
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(6.0))
                        .rounded(px(999.0))
                        .font_weight(FontWeight::SEMIBOLD);
                    let selo = if instalado {
                        selo.bg(c.inv).text_color(c.inv_texto).child("Instalado")
                    } else if m.pronto {
                        selo.border_1().border_color(c.texto_2).text_color(c.texto).child("Ver e instalar")
                    } else {
                        selo.border_1().border_color(c.linha_2).text_color(c.texto_2).child("Em breve")
                    };
                    let id = m.id.clone();
                    lista = lista.child(
                        div()
                            .id(("loja-catalogo", i))
                            .cursor_pointer()
                            .rounded(px(RAIO_CONTROLE))
                            .hover(|s| s.bg(c.hover))
                            .child(self.linha_do_catalogo(m, selo.into_any_element()))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.abrir_ficha(&id);
                                cx.notify();
                            })),
                    );
                }
                lista = lista.child(
                    texto(16.0).pt(px(24.0)).pb(px(4.0)).font_weight(FontWeight::SEMIBOLD).child("Perfis de Escuta"),
                );
                for (nome, descricao) in EM_BREVE {
                    let selo = texto(12.0)
                        .h(px(24.0))
                        .px(px(10.0))
                        .flex()
                        .flex_none()
                        .items_center()
                        .rounded(px(999.0))
                        .border_1()
                        .border_color(c.linha_2)
                        .text_color(c.texto_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Em breve");
                    lista = lista.child(self.linha_de_modulo(
                        "icones/puzzle.svg",
                        nome,
                        descricao,
                        selo.into_any_element(),
                    ));
                }
            }
        }

        let corpo = div()
            .id("loja-corpo")
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .px(px(lado))
            .pt(px(20.0))
            .pb(px(lado))
            .child(if self.aba_loja == AbaLoja::Modulos { lista } else { grade });
        div().size_full().flex().flex_col().child(cabeca).child(corpo)
    }
}
