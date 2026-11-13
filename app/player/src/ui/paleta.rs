//! A barra de comandos (Ctrl+K): buscar e executar qualquer comando pelo nome.
//!
//! Ela aparece e some sem animação de propósito: é aberta pelo teclado muitas vezes, e qualquer animação viraria
//! espera. As letras chegam pelo campo de texto do app (`ui::campo`); aqui ficam as teclas que escolhem e executam.

use super::{RAIO_CONTROLE, RAIO_MODULO, icone, mono, tecla, texto};
use crate::app::Player;
use gpui::{AnyElement, ClickEvent, Context, FontWeight, KeyDownEvent, Window, div, prelude::*, px};
use nucleo::Comando;

/// Minúsculas e sem acento, para "visual" achar "Visual" e "proximo" achar "Próximo".
pub fn normalizar(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            outro => outro,
        })
        .collect()
}

/// O comando combina com a busca quando todas as palavras digitadas aparecem no nome, no grupo ou no id dele.
pub fn combina(consulta: &str, comando: Comando) -> bool {
    let alvo = normalizar(&format!("{} {} {}", comando.nome(), comando.grupo(), comando.id()));
    normalizar(consulta).split_whitespace().all(|palavra| alvo.contains(palavra))
}

fn ordem_do_grupo(grupo: &str) -> u8 {
    match grupo {
        "Player" => 0,
        "Visual" => 1,
        "Aparência" => 2,
        "Layout" => 3,
        _ => 4,
    }
}

/// Comandos que a barra mostra para uma busca, juntos por grupo.
pub fn comandos_para(consulta: &str) -> Vec<Comando> {
    let mut achados: Vec<Comando> = Comando::TODOS
        .into_iter()
        // Abrir a própria barra e fechar não fazem sentido dentro dela.
        .filter(|c| !matches!(c, Comando::AbrirPaleta | Comando::Fechar))
        .filter(|c| combina(consulta, *c))
        .collect();
    achados.sort_by_key(|c| ordem_do_grupo(c.grupo()));
    achados
}

fn icone_do_grupo(grupo: &str) -> &'static str {
    match grupo {
        "Player" => "icones/play.svg",
        "Visual" => "icones/eye.svg",
        "Aparência" => "icones/tone.svg",
        "Layout" => "icones/layout.svg",
        _ => "icones/home.svg",
    }
}

impl Player {
    fn itens_da_paleta(&self) -> Vec<Comando> {
        self.paleta.as_ref().map(|p| comandos_para(&p.consulta)).unwrap_or_default()
    }

    /// Posição do item `i` entre os filhos da lista (que também tem os títulos dos grupos), para a rolagem.
    fn filho_do_item(itens: &[Comando], i: usize) -> usize {
        let mut grupos = 0;
        let mut ultimo = "";
        for c in itens.iter().take(i + 1) {
            if c.grupo() != ultimo {
                ultimo = c.grupo();
                grupos += 1;
            }
        }
        i + grupos
    }

    /// Tecla com a barra de comandos aberta: digitar filtra, as setas escolhem, Enter executa, Esc fecha.
    pub(crate) fn tecla_na_paleta(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.paleta.as_ref().is_some_and(|p| p.letra) {
            self.tecla_na_busca_de_letra(ev, cx);
            return;
        }
        let k = &ev.keystroke;
        let ctrl = k.modifiers.control || k.modifiers.platform;
        let itens = self.itens_da_paleta();
        let Some(paleta) = self.paleta.as_mut() else { return };
        match k.key.as_str() {
            "escape" => self.paleta = None,
            "k" if ctrl => self.paleta = None,
            "enter" => {
                let escolhido = itens.get(paleta.selecionado).copied();
                self.paleta = None;
                if let Some(comando) = escolhido {
                    self.executar(comando, window, cx);
                }
            }
            "down" | "up" => {
                let ultimo = itens.len().saturating_sub(1);
                paleta.selecionado = if k.key == "down" {
                    (paleta.selecionado + 1).min(ultimo)
                } else {
                    paleta.selecionado.saturating_sub(1)
                };
                self.rolagem_paleta.scroll_to_item(Self::filho_do_item(&itens, paleta.selecionado));
            }
            "backspace" => self.apagar_no_campo(ctrl),
            "v" if ctrl => self.colar_no_campo(cx),
            _ => {}
        }
        cx.notify();
    }

    pub(crate) fn ver_paleta(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.paleta.as_ref().is_some_and(|p| p.letra) {
            return self.ver_busca_de_letra(cx);
        }
        let c = self.cores();
        let a = &self.arranjo;
        let consulta: String = self.paleta.as_ref().map(|p| p.consulta.clone()).unwrap_or_default();
        let selecionado = self.paleta.as_ref().map_or(0, |p| p.selecionado);
        let itens = self.itens_da_paleta();

        let mut lista = div()
            .id("paleta-lista")
            .track_scroll(&self.rolagem_paleta)
            .flex_1()
            .min_h(px(0.0))
            .overflow_y_scroll()
            .p(px(6.0))
            .flex()
            .flex_col();
        let mut grupo_atual = "";
        for (i, comando) in itens.iter().copied().enumerate() {
            if comando.grupo() != grupo_atual {
                grupo_atual = comando.grupo();
                lista = lista.child(
                    texto(12.0)
                        .flex_none()
                        .px(px(12.0))
                        .pt(px(12.0))
                        .pb(px(6.0))
                        .text_color(c.texto_2)
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(grupo_atual),
                );
            }
            let ativo = i == selecionado;
            let mut teclas = div().flex().flex_none().gap(px(4.0));
            for t in comando.teclas() {
                teclas = teclas.child(tecla(&c, t));
            }
            lista = lista.child(
                div()
                    .id(("paleta-item", i))
                    .min_h(px(44.0))
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(12.0))
                    .px(px(12.0))
                    .rounded(px(RAIO_CONTROLE))
                    .cursor_pointer()
                    .when(ativo, |d| d.bg(c.sel))
                    .when(!ativo, |d| d.hover(|s| s.bg(c.hover)))
                    .child(icone(icone_do_grupo(comando.grupo()), 18.0, if ativo { c.texto } else { c.texto_2 }))
                    .child(
                        texto(14.0)
                            .flex_1()
                            .min_w(px(0.0))
                            .truncate()
                            .text_color(if ativo { c.texto } else { c.texto_2 })
                            .font_weight(FontWeight::MEDIUM)
                            .child(comando.nome()),
                    )
                    // O nome estável do comando aparece no item escolhido: é o mesmo que atalhos e skins usam.
                    .when(ativo && a.janela.w >= 560.0, |d| {
                        d.child(mono().flex_none().text_color(c.texto_2).child(comando.id()))
                    })
                    .child(teclas)
                    .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                        this.paleta = None;
                        this.executar(comando, window, cx);
                    })),
            );
        }
        if itens.is_empty() {
            lista = lista.child(
                texto(14.0).px(px(14.0)).py(px(28.0)).text_color(c.texto_2).child("Nenhum comando com esse nome."),
            );
        }

        let entrada = div()
            .h(px(60.0))
            .flex()
            .flex_none()
            .items_center()
            .gap(px(12.0))
            .pl(px(20.0))
            .pr(px(16.0))
            .border_b_1()
            .border_color(c.linha_2)
            .child(icone("icones/search.svg", 20.0, c.texto_3))
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .h_full()
                    .relative()
                    .flex()
                    .child(self.receber_texto(cx))
                    .child(Self::conteudo_do_campo(&c, &consulta, "Buscar ou executar um comando", 16.0, true)),
            )
            .child(tecla(&c, "Esc"));

        let rodape = div()
            .flex()
            .flex_none()
            .items_center()
            .gap(px(16.0))
            .px(px(20.0))
            .py(px(12.0))
            .border_t_1()
            .border_color(c.linha_2)
            .overflow_hidden()
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(6.0))
                    .child(tecla(&c, "↑"))
                    .child(tecla(&c, "↓"))
                    .child(texto(12.0).text_color(c.texto_2).child("escolher")),
            )
            .child(
                div()
                    .flex()
                    .flex_none()
                    .items_center()
                    .gap(px(6.0))
                    .child(tecla(&c, "Enter"))
                    .child(texto(12.0).text_color(c.texto_2).child("executar")),
            );

        let largura = 640.0f32.min(a.janela.w - 32.0).max(240.0);
        let altura = 560.0f32.min(a.janela.h * 0.74).max(200.0);
        div()
            .id("paleta-fundo")
            .absolute()
            .left(px(0.0))
            .top(px(0.0))
            .size_full()
            .bg(c.escurecer)
            .occlude()
            .flex()
            .justify_center()
            .items_start()
            .pt(px((a.janela.h * 0.14).min(120.0)))
            // Clicar fora fecha.
            .on_click(cx.listener(|this, _: &ClickEvent, _, cx| {
                this.paleta = None;
                cx.notify();
            }))
            .child(
                div()
                    .id("paleta")
                    .w(px(largura))
                    .max_h(px(altura))
                    .flex()
                    .flex_col()
                    .rounded(px(RAIO_MODULO))
                    .bg(c.bloco)
                    .border_1()
                    .border_color(c.linha_2)
                    .shadow_lg()
                    .overflow_hidden()
                    .occlude()
                    // Clicar dentro não fecha.
                    .on_click(|_, _, cx| cx.stop_propagation())
                    .child(entrada)
                    .child(lista)
                    .child(rodape),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use nucleo::{Modelo, Tema};

    #[test]
    fn busca_ignora_acento_e_maiusculas() {
        assert_eq!(normalizar("Próxima FAIXA, ação"), "proxima faixa, acao");
        assert!(combina("proximo visual", Comando::ProximoVisual));
        assert!(combina("PAPEL", Comando::TrocarTema(Tema::Papel)));
        assert!(!combina("papel", Comando::TrocarTema(Tema::Noite)));
    }

    #[test]
    fn todas_as_palavras_precisam_aparecer() {
        assert!(combina("visual fluxo", Comando::TrocarVisual(Modelo::Fluxo)));
        assert!(!combina("visual fluxo", Comando::TrocarVisual(Modelo::Barras)));
        // Dá para achar pelo nome estável do comando também.
        assert!(combina("player.proxima", Comando::Proxima));
    }

    #[test]
    fn busca_vazia_mostra_tudo_menos_abrir_e_fechar() {
        let todos = comandos_para("");
        assert_eq!(todos.len(), Comando::TODOS.len() - 2);
        assert!(!todos.contains(&Comando::AbrirPaleta) && !todos.contains(&Comando::Fechar));
        assert!(comandos_para("zzzz não existe").is_empty());
        // Cada grupo aparece uma vez só, com os comandos dele juntos.
        let mut grupos: Vec<&str> = todos.iter().map(|c| c.grupo()).collect();
        grupos.dedup();
        assert_eq!(grupos, ["Player", "Visual", "Aparência", "Layout", "Ir para"]);
    }

    #[test]
    fn posicao_do_item_conta_os_titulos_de_grupo() {
        let itens = comandos_para("");
        // O primeiro item vem depois do título do primeiro grupo.
        assert_eq!(Player::filho_do_item(&itens, 0), 1);
        let primeiro_de_outro_grupo = itens.iter().position(|c| c.grupo() != itens[0].grupo()).unwrap_or(0);
        assert_eq!(Player::filho_do_item(&itens, primeiro_de_outro_grupo), primeiro_de_outro_grupo + 2);
    }
}
