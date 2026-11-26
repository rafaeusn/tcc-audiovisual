//! A Loja: módulos, provedores, temas, skins, layouts e pacotes.
//!
//! Os provedores (F1) são os programas dos serviços de música: a aba mostra os instalados, com a situação de cada
//! um, liga e desliga, instala de uma pasta (ou atualiza) e remove (com "Desfazer").
//!
//! Nesta versão tudo o que já vem com o app pode ser aplicado daqui (temas, skins, layouts, pacotes e os módulos
//! do painel). Os módulos que fazem mais que desenhar (usar a internet, entrar numa conta) têm uma ficha: ela
//! mostra o que o módulo pede, com o motivo de cada coisa, e é ali que se instala. Os que ainda não têm código
//! aparecem com a ficha completa, mas sem o botão de instalar: a Loja não finge que instala.

use super::painel::interruptor;
use super::{RAIO_CONTROLE, RAIO_MODULO, com_alfa, icone, medidas};
use crate::app::{AbaLoja, Player};
use crate::contas::{botao_da_conta, frase_da_conta};
use crate::remoto::Situacao;
use crate::tema::{self, Cores};
use crate::ui::medidas::px;
use gpui::{ClickEvent, Context, Div, FontWeight, Hsla, SharedString, Stateful, div, prelude::*};
use nucleo::{Comando, Modulo, Skin, Tema};
use provedores::Capacidade;

const ABAS: [(AbaLoja, &str); 6] = [
    (AbaLoja::Modulos, "Módulos"),
    (AbaLoja::Provedores, "Provedores"),
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

/// O nome de uma capacidade de provedor, para a lista.
fn nome_da_capacidade(c: Capacidade) -> &'static str {
    match c {
        Capacidade::Busca => "busca",
        Capacidade::Inicio => "início",
        Capacidade::Biblioteca => "biblioteca",
        Capacidade::Playlists => "playlists",
        Capacidade::EditarPlaylists => "editar playlists",
        Capacidade::Curtir => "curtir",
        Capacidade::Artista => "artistas",
        Capacidade::Album => "álbuns",
        Capacidade::Radio => "rádio",
        Capacidade::Letra => "letras",
        Capacidade::Audio => "áudio",
        Capacidade::TocaSozinho => "Player próprio",
        Capacidade::Conta => "conta",
        Capacidade::Podcast => "podcasts",
        Capacidade::Video => "vídeos",
        Capacidade::Canvas => "Canvas",
        Capacidade::Reconhecer => "reconhecer músicas",
    }
}

/// Miniatura da janela: o visual, o painel (de um lado ou de nenhum) e o dock, nas cores de um tema.
fn miniatura(c: &Cores, contorno: Hsla, imersiva: bool, painel: Option<bool>) -> Div {
    let bloco_painel = || {
        let b = div().w(px(44.0)).h_full().flex_none().rounded(px(medidas::raio_fino(10.0)));
        if imersiva { b.bg(com_alfa(c.texto, 0.16)) } else { b.border_1().border_color(c.linha_2) }
    };
    let visual = div()
        .flex_1()
        .h_full()
        .rounded(px(medidas::raio_fino(10.0)))
        .flex()
        .items_center()
        .justify_center()
        .when(!imersiva, |d| d.bg(gpui::black()).border_1().border_color(c.linha))
        .child(div().size(px(14.0)).rounded(px(medidas::PILULA)).bg(c.ativa));
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
        .child(div().h(px(12.0)).flex_none().rounded(px(medidas::raio_fino(12.0))).bg(tema::cor_de(c.dock)))
}

/// A miniatura da Estante: a lateral, o visual e a página.
fn miniatura_da_estante(c: &Cores, contorno: Hsla) -> Div {
    let regiao = || div().h_full().rounded(px(medidas::raio_fino(10.0)));
    div()
        .w_full()
        .h(px(104.0))
        .flex_none()
        .rounded(px(RAIO_MODULO))
        .border_1()
        .border_color(contorno)
        .bg(c.parede)
        .p(px(8.0))
        .flex()
        .gap(px(5.0))
        .child(regiao().w(px(30.0)).flex_none().border_1().border_color(c.linha_2))
        .child(
            div()
                .flex_1()
                .h_full()
                .flex()
                .flex_col()
                .gap(px(5.0))
                .child(
                    div()
                        .flex_1()
                        .min_h(px(0.0))
                        .flex()
                        .gap(px(5.0))
                        .child(
                            regiao()
                                .flex_1()
                                .bg(gpui::black())
                                .border_1()
                                .border_color(c.linha)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(div().size(px(18.0)).rounded(px(medidas::PILULA)).bg(c.ativa)),
                        )
                        .child(regiao().flex_1().border_1().border_color(c.linha_2)),
                )
                .child(div().h(px(12.0)).flex_none().rounded(px(medidas::raio_fino(12.0))).bg(tema::cor_de(c.dock))),
        )
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
        let selo = medidas::apoio()
            .h(px(medidas::SELO))
            .px(px(10.0))
            .flex()
            .flex_none()
            .items_center()
            .rounded(px(medidas::PILULA))
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
                            .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(super::linha(nome)))
                            .child(medidas::apoio().text_color(c.texto_2).child(descricao)),
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
                    .child(icone(caminho, medidas::ICONE, c.texto)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(medidas::corpo().font_weight(FontWeight::SEMIBOLD).child(nome))
                    .child(medidas::apoio().text_color(c.texto_2).child(descricao)),
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
                    .child(icone(super::modulo::icone_da_categoria(m.categoria), medidas::ICONE, c.texto)),
            )
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .flex()
                    .flex_col()
                    .child(
                        medidas::corpo()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(super::linha(gpui::SharedString::from(m.nome.clone()))),
                    )
                    .child(
                        medidas::apoio()
                            .text_color(c.texto_2)
                            .line_clamp(2)
                            .child(gpui::SharedString::from(m.descricao.clone())),
                    ),
            )
            .child(selo)
    }

    /// Botão em pílula da Loja (cheio, para a ação principal; com contorno, para as outras).
    pub(super) fn pilula_da_loja(&self, id: (&'static str, usize), rotulo: &'static str, cheio: bool) -> Stateful<Div> {
        self.pilula_da_loja_com(id, rotulo, cheio)
    }

    /// A mesma, com um nome que não é fixo (o de um serviço).
    pub(super) fn pilula_da_loja_com(
        &self,
        id: (&'static str, usize),
        rotulo: impl Into<SharedString>,
        cheio: bool,
    ) -> Stateful<Div> {
        let c = self.cores();
        let b = medidas::apoio()
            .id(id)
            .h(px(medidas::FICHA))
            .px(px(14.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(6.0))
            .rounded(px(medidas::PILULA))
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer();
        if cheio {
            b.bg(c.inv).text_color(c.inv_texto).hover(|s| s.opacity(0.9)).child(rotulo.into())
        } else {
            b.border_1().border_color(c.linha_2).text_color(c.texto_2).hover(|s| s.bg(c.hover)).child(rotulo.into())
        }
    }

    /// A aba Provedores: os instalados (situação, versão, o que sabem fazer), ligar e desligar, instalar e remover.
    fn lista_de_provedores(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let mut lista = div()
            .flex()
            .flex_col()
            .child(medidas::corpo().pb(px(4.0)).font_weight(FontWeight::SEMIBOLD).child("Provedores"))
            .child(medidas::apoio().text_color(c.texto_2).pb(px(12.0)).child(
                "Os serviços de música (YouTube Music, Spotify, SoundCloud...). Cada provedor é um programa à parte: instalado, ele entra na busca da Biblioteca e as faixas dele tocam aqui, com o visual e a análise.",
            ))
            .child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(px(8.0))
                    .pb(px(16.0))
                    .child(self.pilula_da_loja(("prov-instalar", 0), "Instalar de uma pasta", true).on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.escolher_provedor(cx);
                            cx.notify();
                        }),
                    ))
                    .child(self.pilula_da_loja(("prov-pasta", 0), "Abrir a pasta dos provedores", false).on_click(
                        cx.listener(|this, _: &ClickEvent, _, cx| {
                            this.abrir_pasta_dos_provedores();
                            cx.notify();
                        }),
                    )),
            );
        let provedores = &self.local.remoto.provedores;
        if provedores.is_empty() {
            return lista.child(
                div()
                    .py(px(28.0))
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(px(10.0))
                    .child(icone("icones/globe.svg", medidas::ICONE_GRANDE, c.texto_3))
                    .child(medidas::apoio().max_w(px(460.0)).text_center().text_color(c.texto_2).child(
                        "Nenhum provedor instalado ainda. O do YouTube Music é o próximo a chegar; um provedor é uma pasta com o provedor.toml e o programa dele.",
                    )),
            );
        }
        for (i, p) in provedores.iter().enumerate() {
            let ligado = p.situacao != Situacao::Desligado;
            let situacao = match &p.situacao {
                Situacao::Pronto => "Pronto".to_string(),
                Situacao::Abrindo => "Abrindo...".to_string(),
                Situacao::Desligado => "Desligado".to_string(),
                Situacao::Falhou(m) => format!("Não abriu: {m}"),
            };
            let mut detalhes = vec![situacao];
            if !p.versao.is_empty() {
                detalhes.push(format!("versão {}", p.versao));
            }
            if !p.capacidades.is_empty() {
                detalhes.push(p.capacidades.iter().map(|c| nome_da_capacidade(*c)).collect::<Vec<_>>().join(", "));
            }
            // A conta, para os provedores que entram numa: quem está conectado, ou o convite.
            let conta = p.conta.as_ref().filter(|_| p.situacao == Situacao::Pronto);
            if let Some(conta) = conta {
                detalhes.push(frase_da_conta(conta));
            }
            let botao_conta = conta.and_then(botao_da_conta);
            let convite = botao_conta == Some("Entrar");
            let s_conta = p.servico.clone();
            let cor_da_situacao = if matches!(p.situacao, Situacao::Falhou(_)) { c.texto } else { c.texto_2 };
            let posicao = self.anim.seguir(("prov-interruptor", i as u32), f32::from(u8::from(ligado)), 0.2);
            let (s_ligar, s_remover) = (p.servico.clone(), p.servico.clone());
            lista = lista.child(
                div()
                    .min_h(px(64.0))
                    .py(px(10.0))
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
                            .child(icone("icones/globe.svg", medidas::ICONE, if ligado { c.texto } else { c.texto_3 })),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w(px(0.0))
                            .flex()
                            .flex_col()
                            .child(
                                medidas::corpo()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .child(super::linha(format!("{} ({})", p.nome, p.servico))),
                            )
                            .child(
                                medidas::corpo()
                                    .text_color(cor_da_situacao)
                                    // Um erro de login precisa caber inteiro (diz qual programa fechar).
                                    .line_clamp(if p.conta.as_ref().is_some_and(|c| c.erro.is_some()) { 5 } else { 2 })
                                    .child(SharedString::from(detalhes.join(", "))),
                            ),
                    )
                    .when_some(botao_conta, |d, rotulo| {
                        d.child(self.pilula_da_loja(("prov-conta", i), rotulo, convite).on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| {
                                if convite {
                                    this.local.remoto.entrar(&s_conta);
                                } else {
                                    this.local.remoto.sair(&s_conta);
                                }
                                cx.notify();
                            },
                        )))
                    })
                    .when(p.embutido, |d| {
                        d.child(medidas::apoio().flex_none().text_color(c.texto_3).child("Vem com o app"))
                    })
                    .when(!p.embutido, |d| {
                        d.child(self.pilula_da_loja(("prov-remover", i), "Remover", false).on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| {
                                this.local.remoto.remover(&s_remover);
                                cx.notify();
                            },
                        )))
                    })
                    .child(
                        div()
                            .id(("prov-ligar", i))
                            .cursor_pointer()
                            .child(interruptor(&c, "prov-interruptor", ligado, posicao))
                            .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                                this.ligar_provedor(&s_ligar, !ligado);
                                cx.notify();
                            })),
                    ),
            );
        }
        lista
    }

    pub(crate) fn ver_loja(&self, cx: &mut Context<Self>) -> Div {
        let c = self.cores();
        let a = &self.arranjo;
        let lado = if a.heroi.w >= 640.0 { 32.0 } else { 16.0 };

        let escolhida = ABAS.iter().position(|(aba, _)| *aba == self.aba_loja).unwrap_or(0);
        let mut lista_de_abas = Vec::with_capacity(ABAS.len());
        for (i, (aba, rotulo)) in ABAS.into_iter().enumerate() {
            lista_de_abas.push(
                self.aba_sublinhada(("loja-aba", i), rotulo, aba == self.aba_loja)
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| {
                        this.aba_loja = aba;
                        cx.notify();
                    }))
                    .into_any_element(),
            );
        }
        let abas = self.abas_sublinhadas("loja-abas", 2_000_003, escolhida, lista_de_abas);

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
                            .child(medidas::pagina(a.heroi.w >= 640.0).child("Loja"))
                            // Em janela estreita a descrição sai: a altura fica para a lista.
                            .when(a.heroi.w >= 640.0, |d| {
                                d.child(medidas::apoio().text_color(c.texto_2).child(
                                    "Módulos, provedores, temas, skins e layouts. O que já vem com o app se aplica com um clique.",
                                ))
                            }),
                    )
                    .child(self.botao_fechar("loja-fechar", cx)),
            )
            .child(abas);

        let mut grade = div().flex().flex_wrap().gap(px(12.0));
        let mut lista = div().flex().flex_col();
        match self.aba_loja {
            AbaLoja::Provedores => lista = self.lista_de_provedores(cx),
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
                let palco = aj.layout == nucleo::Layout::Palco;
                for (i, (nome, descricao, aberto, esquerda)) in LAYOUTS.into_iter().enumerate() {
                    let em_uso = palco && aj.painel_aberto == aberto && (!aberto || aj.painel_a_esquerda == esquerda);
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
                            this.trocar_layout(nucleo::Layout::Palco);
                            this.aplicar_layout(aberto, lado);
                            cx.notify();
                        })),
                    );
                }
                // A Estante: a lateral, o visual e a página ao lado dele.
                grade = grade.child(
                    self.cartao(
                        ("loja-layout", LAYOUTS.len()),
                        miniatura_da_estante(&ct, c.linha_2),
                        "Estante",
                        "Biblioteca, visual no centro e página ao lado",
                        !palco,
                    )
                    .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                        this.trocar_layout(nucleo::Layout::Estante);
                        cx.notify();
                    })),
                );
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
                lista = lista
                    .child(medidas::corpo().pb(px(4.0)).font_weight(FontWeight::SEMIBOLD).child("Módulos do painel"));
                for m in Modulo::TODOS {
                    let no_painel = self.ajustes.modulos.contains(&m);
                    let botao = medidas::apoio()
                        .id(("loja-modulo", m as usize))
                        .h(px(medidas::FICHA))
                        .px(px(14.0))
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(6.0))
                        .rounded(px(medidas::PILULA))
                        .font_weight(FontWeight::SEMIBOLD)
                        .cursor_pointer();
                    let botao = if no_painel {
                        botao
                            .border_1()
                            .border_color(c.linha_2)
                            .text_color(c.texto_2)
                            .hover(|s| s.bg(c.hover))
                            .child(icone("icones/check.svg", medidas::ICONE_LINHA, c.texto_2))
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
                    medidas::corpo().pt(px(24.0)).pb(px(4.0)).font_weight(FontWeight::SEMIBOLD).child("Para instalar"),
                );
                for (i, m) in self.catalogo.iter().enumerate() {
                    let instalado = self.instalados.achar(&m.id).is_some();
                    let selo = medidas::apoio()
                        .h(px(medidas::SELO))
                        .px(px(10.0))
                        .flex()
                        .flex_none()
                        .items_center()
                        .gap(px(6.0))
                        .rounded(px(medidas::PILULA))
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
                    medidas::corpo()
                        .pt(px(24.0))
                        .pb(px(4.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child("Perfis de Escuta"),
                );
                for (nome, descricao) in EM_BREVE {
                    let selo = medidas::apoio()
                        .h(px(medidas::SELO))
                        .px(px(10.0))
                        .flex()
                        .flex_none()
                        .items_center()
                        .rounded(px(medidas::PILULA))
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
            .track_scroll(&self.rolagem_da_pagina())
            .px(px(lado))
            .pt(px(20.0))
            .pb(px(lado))
            .child(if matches!(self.aba_loja, AbaLoja::Modulos | AbaLoja::Provedores) { lista } else { grade });
        div().size_full().flex().flex_col().child(cabeca).child(corpo)
    }
}
