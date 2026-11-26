//! Navegar: os destinos, a busca do topo, o filtro de serviço e a pilha de voltar.

use super::*;
use crate::espacos;
use crate::navegacao::{AbaDaBiblioteca, Campo, Espaco, Rota, campo_na_tela, so_comandos, texto_para_buscar};
use crate::paginas::AbaDaBusca;
use crate::remoto::Situacao as SituacaoDoProvedor;

/// Quantas rolagens guardar antes de esquecer as dos passos que saíram da pilha.
const ROLAGENS_LEMBRADAS: usize = 150;
/// Com quantas letras a busca já procura comandos, e quantos aparecem junto com as músicas.
const LETRAS_PARA_COMANDOS: usize = 3;
const COMANDOS_NAS_PRINCIPAIS: usize = 3;
/// A altura de um comando na lista da busca.
pub(crate) const ALTURA_DO_COMANDO: f32 = 48.0;

impl Player {
    /// A página de hoje que a rota aberta representa (o que o resto da tela já sabe desenhar).
    pub(crate) fn pagina(&self) -> Pagina {
        match self.nav.atual() {
            None => Pagina::Inicio,
            Some(Rota::Loja) => Pagina::Loja,
            Some(Rota::Configuracoes) => Pagina::Configuracoes,
            Some(_) => Pagina::Biblioteca,
        }
    }

    /// O layout Estante está valendo (a janela mini é sempre só o visual).
    pub(crate) fn estante(&self) -> bool {
        self.ajustes.layout == nucleo::Layout::Estante && !self.mini
    }

    /// Troca o layout (gravado); a Estante abre numa página, o Palco volta como estava.
    pub(crate) fn trocar_layout(&mut self, layout: nucleo::Layout) {
        if self.ajustes.layout == layout {
            return;
        }
        self.ajustes.layout = layout;
        self.expandido = false;
        self.gaveta_aberta = false;
        if layout == nucleo::Layout::Palco {
            // A folha fechada do Palco volta fechada (a Estante abriu a primeira página só para ela).
            self.nav.soltar_preenchida();
        }
        self.salvar();
        self.avisar(format!("Layout {}", layout.nome()));
    }

    /// Fechar a página: no Palco, a folha some e fica o visual; na Estante sempre há página, então vai ao Início.
    pub(crate) fn fechar_folha(&mut self) {
        if self.estante() {
            self.abrir_rota(Rota::Inicio);
        } else {
            self.largar_campo();
            self.nav.fechar();
        }
    }

    /// Um destino da navegação (a lateral, o trilho ou a barra de baixo). O que já está aberto volta ao topo da página
    /// ou, no Palco, fecha a folha e mostra o visual.
    pub(crate) fn ir_ao_destino(&mut self, rota: Rota) {
        if rota == Rota::Busca {
            self.abrir_busca();
        } else if !self.nav.atual().is_some_and(|r| r.mesmo_destino(&rota)) {
            self.abrir_rota(rota);
        } else if self.estante() {
            self.rolagem_da_pagina().set_offset(Point::default());
        } else {
            self.fechar_folha();
        }
        self.tirar_o_painel_da_frente();
        self.edicao = false;
    }

    /// Abre a página Buscar com o teclado no campo do topo. O texto que estava lá fica selecionado: a próxima letra
    /// troca tudo.
    pub(crate) fn abrir_busca(&mut self) {
        let ja = self.nav.atual() == Some(&Rota::Busca);
        if !ja {
            self.abrir_rota(Rota::Busca);
            self.busca_de_passagem = true;
        }
        self.tirar_o_painel_da_frente();
        self.edicao = false;
        self.focar(Campo::Busca);
        self.campo_selecionado = !self.buscas.servicos.is_empty();
    }

    /// Põe o teclado num campo da página.
    pub(crate) fn focar(&mut self, campo: Campo) {
        self.campo = Some(campo);
        self.campo_selecionado = false;
        self.marcado = None;
    }

    /// Tira o teclado do campo (os atalhos voltam a valer).
    pub(crate) fn largar_campo(&mut self) {
        self.campo = None;
        self.campo_selecionado = false;
        self.marcado = None;
    }

    /// O campo em que o teclado está, se ele está na tela.
    pub(crate) fn campo_no_teclado(&self) -> Option<Campo> {
        self.campo.filter(|c| campo_na_tela(*c, self.nav.atual(), self.nav.espaco()))
    }

    /// Voltar ou avançar uma página (na Estante, pulando a folha fechada). Devolve se andou.
    pub(crate) fn andar(&mut self, para_tras: bool) -> bool {
        let estante = self.estante();
        let andou = self.pode_navegar()
            && match (para_tras, estante) {
                (true, true) => self.nav.voltar_sem_vazias(),
                (true, false) => self.nav.voltar(),
                (false, true) => self.nav.avancar_sem_vazias(),
                (false, false) => self.nav.avancar(),
            };
        if andou {
            self.depois_de_navegar();
            self.lembrar_filtro();
        }
        andou
    }

    /// O texto da busca de agora entra nas buscas recentes (quando a pessoa abre ou toca um resultado, ou dá Enter).
    pub(crate) fn lembrar_a_busca(&mut self) {
        if self.nav.atual() == Some(&Rota::Busca)
            && crate::navegacao::lembrar_busca(&mut self.ajustes.buscas_recentes, &self.buscas.servicos)
        {
            self.salvar();
        }
    }

    /// Escreve o texto na busca e procura (uma busca recente, por exemplo).
    pub(crate) fn buscar_por(&mut self, texto: &str) {
        self.buscas.servicos = texto.to_string();
        self.abrir_busca();
        self.campo_selecionado = false;
        self.procurar();
    }

    /// Manda o texto da busca aos serviços e ao computador (comandos não vão a lugar nenhum).
    pub(crate) fn procurar(&mut self) {
        self.comando_escolhido = 0;
        let texto = texto_para_buscar(&self.buscas.servicos).to_string();
        self.local.remoto.buscar(&texto);
        self.local.procurar(&texto);
    }

    /// Os comandos que a página Buscar mostra: todos os que combinam, na aba Comandos (ou com `>`); nas Principais, só
    /// alguns.
    pub(crate) fn comandos_da_busca(&self) -> Vec<Comando> {
        let texto = self.buscas.servicos.trim();
        match crate::paginas::aba_da_busca(texto, self.aba_da_busca) {
            AbaDaBusca::Comandos => crate::ui::comandos_para(so_comandos(texto).unwrap_or(texto)),
            AbaDaBusca::Principais if texto.chars().count() >= LETRAS_PARA_COMANDOS => {
                let mut achados = crate::ui::comandos_para(texto);
                achados.truncate(COMANDOS_NAS_PRINCIPAIS);
                achados
            }
            _ => Vec::new(),
        }
    }

    /// A aba Comandos está na tela: as setas escolhem e o Enter executa.
    pub(crate) fn escolhendo_comando(&self) -> bool {
        self.nav.atual() == Some(&Rota::Busca)
            && crate::paginas::aba_da_busca(&self.buscas.servicos, self.aba_da_busca) == AbaDaBusca::Comandos
            && !self.buscas.servicos.trim().is_empty()
    }

    /// Executa um comando escolhido na busca. Aberta pelo atalho, ela dá lugar à página de antes: a pessoa só queria o
    /// comando.
    pub(crate) fn executar_da_busca(&mut self, comando: Comando, window: &mut Window, cx: &mut Context<Self>) {
        self.largar_campo();
        self.buscas.servicos.clear();
        self.procurar();
        if std::mem::take(&mut self.busca_de_passagem) {
            self.andar(true);
        }
        self.executar(comando, window, cx);
    }

    /// Tecla com um campo da página ativo. As letras chegam pelo caminho de texto do sistema (ver `ui::campo`); aqui só
    /// as de controle: Esc e Enter devolvem os atalhos, as setas escolhem um comando.
    pub(super) fn tecla_no_campo(
        &mut self,
        campo: Campo,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let k = &ev.keystroke;
        let ctrl = k.modifiers.control || k.modifiers.platform;
        let comandos =
            if campo == Campo::Busca && self.escolhendo_comando() { self.comandos_da_busca() } else { Vec::new() };
        match k.key.as_str() {
            "enter" => match comandos.get(self.comando_escolhido).copied() {
                Some(comando) => self.executar_da_busca(comando, window, cx),
                None => {
                    if campo == Campo::Busca {
                        self.lembrar_a_busca();
                    }
                    self.largar_campo();
                }
            },
            "escape" => self.largar_campo(),
            "k" if ctrl => self.executar(Comando::AbrirPaleta, window, cx),
            "tab" if ctrl => self.executar(Comando::ProximoEspaco, window, cx),
            "a" if ctrl => self.campo_selecionado = !self.buscas.do_campo(campo).is_empty(),
            "down" => {
                self.comando_escolhido = (self.comando_escolhido + 1).min(comandos.len().saturating_sub(1));
                self.mostrar_o_comando_escolhido();
            }
            "up" => {
                self.comando_escolhido = self.comando_escolhido.saturating_sub(1);
                self.mostrar_o_comando_escolhido();
            }
            "left" | "right" | "home" | "end" => self.campo_selecionado = false,
            "backspace" | "delete" => self.apagar_no_campo(ctrl),
            "v" if ctrl => self.colar_no_campo(cx),
            _ => {}
        }
        cx.notify();
    }

    /// Rola a página até o comando escolhido pelas setas.
    fn mostrar_o_comando_escolhido(&self) {
        let Some(area) = self.areas.borrow().get("busca-comando-escolhido").copied() else { return };
        let folha = &self.arranjo.heroi;
        let (alto, pe) = (folha.y + 120.0, folha.y + folha.h - 24.0);
        // A marca é do quadro anterior: conta uma linha a mais para cada lado.
        let real = crate::ui::medidas::logico;
        let (topo, fim) = (real(area.origin.y) - ALTURA_DO_COMANDO, real(area.bottom()) + ALTURA_DO_COMANDO);
        let rolagem = self.rolagem_da_pagina();
        let agora = rolagem.offset();
        let quanto = if topo < alto {
            alto - topo
        } else if fim > pe {
            pe - fim
        } else {
            return;
        };
        let quanto = crate::ui::medidas::px(quanto);
        rolagem.set_offset(gpui::point(agora.x, (agora.y + quanto).min(gpui::px(0.0))));
    }

    /// A rolagem da página aberta (uma por passo da navegação: voltar devolve a página onde ela estava).
    pub(crate) fn rolagem_da_pagina(&self) -> gpui::ScrollHandle {
        let mut todas = self.rolagens_das_paginas.borrow_mut();
        if todas.len() > ROLAGENS_LEMBRADAS {
            todas.retain(|id, _| self.nav.tem_passo(*id));
        }
        todas.entry(self.nav.passo()).or_default().clone()
    }

    /// A rolagem da lista longa de músicas da página aberta.
    pub(crate) fn rolagem_da_lista(&self) -> gpui::UniformListScrollHandle {
        let mut todas = self.rolagens_das_listas.borrow_mut();
        if todas.len() > ROLAGENS_LEMBRADAS {
            todas.retain(|id, _| self.nav.tem_passo(*id));
        }
        todas.entry(self.nav.passo()).or_default().clone()
    }

    /// O painel que cobre a página (a gaveta do Palco, ou o que toma o lugar dela na Estante) fecha ao abrir uma página; o
    /// que fica ao lado continua.
    pub(crate) fn tirar_o_painel_da_frente(&mut self) {
        if !self.estante() || self.arranjo.painel_na_pagina {
            self.gaveta_aberta = false;
        }
    }

    pub(crate) fn abrir_rota(&mut self, rota: Rota) {
        if rota == Rota::Biblioteca(AbaDaBiblioteca::Historico) {
            self.pedir_relatorio();
        }
        if let Some(item) = rota.item() {
            self.lembrar_a_busca();
            // A página de uma faixa não é um lugar a que se volta: só álbum, playlist e artista.
            if item.faixa().is_none() {
                self.local.recente(item);
            }
        }
        // A página nova não tem o campo (a Busca o liga de novo, em `abrir_busca`).
        self.largar_campo();
        self.busca_de_passagem = false;
        self.expandido = false;
        self.tirar_o_painel_da_frente();
        if self.nav.atual() != Some(&rota) {
            self.pagina_nova();
        }
        self.nav.ir(rota);
    }

    /// Abre a página do item, ou toca a faixa (com as outras da `fileira` na fila).
    pub(crate) fn abrir_ou_tocar(&mut self, item: &provedores::Item, fileira: &[provedores::Faixa]) {
        match (crate::paginas::rota_do_item(item), item.faixa()) {
            (Some(rota), _) => self.abrir_rota(rota),
            (None, Some(f)) => {
                let i = fileira.iter().position(|x| x.id == f.id && x.servico == f.servico);
                match i {
                    Some(i) => self.tocar_faixas(fileira, i),
                    None => self.tocar_faixas(std::slice::from_ref(f), 0),
                }
            }
            (None, None) => {}
        }
    }

    /// Mostra uma pasta ou uma playlist do computador (na aba Pastas da Biblioteca).
    pub(crate) fn abrir_origem(&mut self, origem: biblioteca::Origem) {
        self.apagar_playlist = None;
        self.lembrar_a_busca();
        self.buscas.biblioteca.clear();
        self.local.buscar("");
        self.local.ver(crate::local::Visao::Musicas);
        self.local.mostrar(origem);
        // Num serviço não há pastas: a página abre em Tudo.
        if self.servico_do_filtro().is_some() {
            self.nav.ir(Rota::Biblioteca(AbaDaBiblioteca::Pastas));
            self.trocar_espaco(Espaco::Tudo);
        }
        self.abrir_rota(Rota::Biblioteca(AbaDaBiblioteca::Pastas));
    }

    /// Escolhe a aba da Biblioteca (lembrada para a próxima vez).
    pub(crate) fn abrir_aba_da_biblioteca(&mut self, aba: AbaDaBiblioteca) {
        self.largar_campo();
        self.aba_lembrada = aba;
        if aba == AbaDaBiblioteca::Historico {
            self.pedir_relatorio();
        }
        self.pagina_nova();
        self.nav.trocar_aba(aba);
    }

    /// Escolhe o filtro de serviço (lembrado nos ajustes). Numa página com filtro, ela continua aberta; nas outras, abre o
    /// Início nele.
    pub(crate) fn trocar_espaco(&mut self, espaco: Espaco) {
        if !self.pode_navegar() {
            return;
        }
        let fica = self.nav.atual().is_some_and(Rota::tem_filtro);
        if fica && &espaco == self.nav.espaco() {
            return;
        }
        // Filtrar no meio da digitação não tira o teclado do campo.
        let campo = self.campo_no_teclado();
        let de_passagem = self.busca_de_passagem;
        self.nav.trocar_filtro(espaco);
        self.depois_de_navegar();
        self.lembrar_filtro();
        if fica {
            self.campo = campo;
            self.busca_de_passagem = de_passagem;
        }
        if self.nav.aba() == AbaDaBiblioteca::Historico {
            self.pedir_relatorio();
        }
    }

    /// O filtro de agora vai para os ajustes.
    fn lembrar_filtro(&mut self) {
        let chave = self.nav.espaco().chave();
        if self.ajustes.espaco != chave {
            self.ajustes.espaco = chave;
            self.salvar();
        }
    }

    /// Navegar (voltar, avançar, trocar o filtro) só vale com a janela normal e nada aberto por cima: a janela
    /// mini não mostra páginas, e a barra de comandos e a janela de playlist ficam com o teclado.
    pub(crate) fn pode_navegar(&self) -> bool {
        !self.mini && self.paleta.is_none() && self.janela_playlist.is_none()
    }

    /// O que abrir uma página sempre desfaz (como `AbrirPagina`): o modo edição, a gaveta e o campo de texto.
    pub(crate) fn depois_de_navegar(&mut self) {
        self.edicao = false;
        self.tirar_o_painel_da_frente();
        // Navegar com o visual expandido: ele encolhe, para a página nova aparecer.
        self.expandido = false;
        self.largar_campo();
        self.busca_de_passagem = false;
        self.pagina_nova();
    }

    /// O que começa do zero em cada página: ela aparece aos poucos, "Ver mais" e a medida da lista longa (a da página
    /// anterior não vale).
    fn pagina_nova(&mut self) {
        self.anim.definir(("rota-entra", 0), 0.0);
        self.cartao_sob = None;
        self.ver_mais = false;
        self.aba_do_artista = Default::default();
        self.cartoes_a_mostrar = crate::ui::CARTOES_POR_VEZ;
        self.areas.borrow_mut().remove("detalhe-linhas");
    }

    /// Os filtros de serviço, com o nome: Tudo, os provedores ligados e abertos, e o computador.
    pub(crate) fn espacos(&self) -> Vec<(Espaco, String)> {
        let mut v = vec![(Espaco::Tudo, "Tudo".to_string())];
        v.extend(
            self.local
                .remoto
                .provedores
                .iter()
                .filter(|p| p.situacao == SituacaoDoProvedor::Pronto)
                .map(|p| (Espaco::Servico(p.servico.clone()), p.nome.clone())),
        );
        v.push((Espaco::Computador, "Neste PC".to_string()));
        v
    }

    /// Os serviços (e o nome) que a página mostra no filtro de agora: todos em Tudo, nenhum em Neste PC.
    pub(crate) fn servicos_no_filtro(&self) -> Vec<(String, String)> {
        let filtro = self.nav.espaco();
        self.espacos()
            .into_iter()
            .filter_map(|(e, nome)| match e {
                Espaco::Servico(s) if filtro == &Espaco::Tudo || filtro == &Espaco::Servico(s.clone()) => {
                    Some((s, nome))
                }
                _ => None,
            })
            .collect()
    }

    /// O serviço do filtro (`None` em Tudo e em Neste PC).
    pub(crate) fn servico_do_filtro(&self) -> Option<&str> {
        match self.nav.espaco() {
            Espaco::Servico(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// A cor de um serviço (a que o provedor manda, ou uma tirada do nome).
    pub(crate) fn cor_do_servico(&self, servico: &str) -> [f32; 3] {
        self.local
            .remoto
            .provedores
            .iter()
            .find(|p| p.servico == servico)
            .and_then(|p| p.cor)
            .map_or_else(|| espacos::cor_do_nome(servico), espacos::rgb)
    }

    /// O filtro é de um provedor que saiu (desligado ou removido): volta a Tudo. Só depois que a lista dos
    /// provedores chegou, para não perder o filtro lembrado enquanto eles abrem.
    pub(super) fn conferir_espaco(&mut self) {
        let Espaco::Servico(s) = self.nav.espaco().clone() else { return };
        let r = &self.local.remoto;
        let saiu = match r.provedores.iter().find(|p| p.servico == s) {
            None => r.provedores.iter().all(|p| p.situacao != SituacaoDoProvedor::Abrindo),
            Some(p) => matches!(p.situacao, SituacaoDoProvedor::Desligado | SituacaoDoProvedor::Falhou(_)),
        };
        if saiu {
            // Um provedor que não abriu (o Windows bloqueou o programa, por exemplo) só tira o filtro desta vez: o
            // lembrado continua, para a próxima abertura. Desligado ou removido, sai de vez.
            let so_agora =
                r.provedores.iter().any(|p| p.servico == s && matches!(p.situacao, SituacaoDoProvedor::Falhou(_)));
            self.nav.esquecer_servico(&s);
            if !so_agora {
                self.ajustes.espaco = self.nav.espaco().chave();
                self.salvar();
            }
        }
    }
}
