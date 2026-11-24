//! Navegar: abrir páginas, trocar de espaço e a cor de cada espaço (com a transição).

use super::*;
use crate::espacos::{self, PaletaDoEspaco};
use crate::navegacao::{Espaco, Rota};
use crate::remoto::Situacao as SituacaoDoProvedor;

/// Quanto dura a troca de cor ao mudar de espaço.
const TROCA_DE_COR_S: f32 = 0.25;

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
            self.nav.soltar_preenchidas();
        }
        self.salvar();
        self.avisar(format!("Layout {}", layout.nome()));
    }

    /// Fechar a página: no Palco, a folha some e fica o visual; na Estante sempre há página, então volta (ou vai à
    /// primeira do espaço).
    /// (Ver [`Self::fechar_gaveta_fora_da_estante`].)
    pub(crate) fn fechar_folha(&mut self) {
        if self.estante() {
            // H e o × levam à primeira página do espaço (não andam para trás no histórico).
            self.abrir_rota(self.nav.primeira());
        } else {
            self.nav.fechar();
        }
    }

    /// No Palco a gaveta (o painel por cima) fecha ao abrir uma página; na Estante ela é o Tocando, ao lado da
    /// página, e fica.
    pub(crate) fn fechar_gaveta_fora_da_estante(&mut self) {
        if !self.estante() {
            self.gaveta_aberta = false;
        }
    }

    /// A aba da Biblioteca que a rota aberta mostra.
    pub(crate) fn aba_biblioteca(&self) -> AbaBiblioteca {
        match self.nav.atual() {
            Some(Rota::Fila) => AbaBiblioteca::Fila,
            Some(Rota::Historico) => AbaBiblioteca::Historico,
            _ => AbaBiblioteca::Musicas,
        }
    }

    /// O teclado está no campo de busca, e ele está na tela.
    pub(crate) fn campo_de_busca_ativo(&self) -> bool {
        self.buscando_musicas && crate::navegacao::tem_campo_de_busca(self.nav.atual(), self.nav.espaco())
    }

    /// Uma aba do topo da folha. A Busca já abre com o teclado no campo.
    pub(crate) fn abrir_aba_do_espaco(&mut self, rota: Rota) {
        let busca = rota == Rota::Busca;
        self.abrir_rota(rota);
        self.buscando_musicas = busca;
    }

    /// Da Biblioteca do computador para a Busca dos serviços, com o mesmo texto (em Tudo: todos os serviços).
    pub(crate) fn buscar_nos_servicos(&mut self) {
        if self.nav.espaco() == &Espaco::Computador {
            self.trocar_espaco(Espaco::Tudo);
        }
        self.abrir_aba_do_espaco(Rota::Busca);
    }

    pub(crate) fn abrir_rota(&mut self, rota: Rota) {
        if matches!(rota, Rota::Biblioteca | Rota::Historico) {
            self.pedir_relatorio();
        }
        // A página nova não tem o campo de busca (a Busca o liga de novo, em `abrir_aba_do_espaco`).
        self.buscando_musicas = false;
        self.expandido = false;
        if self.nav.atual() != Some(&rota) {
            self.anim.definir(("rota-entra", 0), 0.0);
            self.entrada_de = 1.0;
            self.pagina_nova();
            self.cartao_sob = None;
        }
        self.nav.ir(rota);
    }

    /// Vai para outro espaço (lembrado nos ajustes); a cor desliza da atual para a dele.
    pub(crate) fn trocar_espaco(&mut self, espaco: Espaco) {
        if &espaco == self.nav.espaco() || !self.pode_navegar() {
            return;
        }
        self.comecar_troca_de_cor();
        self.ajustes.espaco = espaco.chave();
        self.salvar();
        self.nav.trocar_espaco(espaco);
        self.depois_de_navegar();
    }

    /// Navegar (voltar, avançar, trocar de espaço) só vale com a janela normal e nada aberto por cima: a janela
    /// mini não mostra páginas, e a barra de comandos e a janela de playlist ficam com o teclado.
    pub(crate) fn pode_navegar(&self) -> bool {
        !self.mini && self.paleta.is_none() && self.janela_playlist.is_none()
    }

    /// O que abrir uma página sempre desfaz (como `AbrirPagina`): o modo edição, a gaveta e a busca de músicas.
    pub(crate) fn depois_de_navegar(&mut self) {
        self.edicao = false;
        self.fechar_gaveta_fora_da_estante();
        // Navegar com o visual expandido: ele encolhe, para a página nova aparecer.
        self.expandido = false;
        self.buscando_musicas = false;
        // A página que aparece entra deslizando (do lado de onde veio).
        self.anim.definir(("rota-entra", 0), 0.0);
        self.entrada_de = 1.0;
        self.pagina_nova();
        self.cartao_sob = None;
    }

    /// O que começa do zero em cada página: "Ver mais" e a medida da lista longa (a da página anterior não vale).
    fn pagina_nova(&mut self) {
        self.ver_mais = false;
        self.cartoes_a_mostrar = crate::ui::CARTOES_POR_VEZ;
        self.areas.borrow_mut().remove("detalhe-linhas");
    }

    fn comecar_troca_de_cor(&mut self) {
        self.paleta_de = self.paleta_agora;
        self.anim.definir(("espaco-cor", 0), 0.0);
    }

    /// Os espaços que dá para abrir, com o nome: Tudo, o computador e os provedores ligados e abertos.
    pub(crate) fn espacos(&self) -> Vec<(Espaco, String)> {
        let mut v = vec![(Espaco::Tudo, "Tudo".to_string()), (Espaco::Computador, "Neste computador".to_string())];
        v.extend(
            self.local
                .remoto
                .provedores
                .iter()
                .filter(|p| p.situacao == SituacaoDoProvedor::Pronto)
                .map(|p| (Espaco::Servico(p.servico.clone()), p.nome.clone())),
        );
        v
    }

    /// A cor de um espaço (Tudo não tem: usa o tema).
    pub(crate) fn cor_do_espaco(&self, espaco: &Espaco) -> Option<[f32; 3]> {
        match espaco {
            Espaco::Tudo => None,
            Espaco::Computador => Some(espacos::rgb(espacos::COR_DO_COMPUTADOR)),
            Espaco::Servico(s) => Some(
                self.local
                    .remoto
                    .provedores
                    .iter()
                    .find(|p| &p.servico == s)
                    .and_then(|p| p.cor)
                    .map_or_else(|| espacos::cor_do_nome(s), espacos::rgb),
            ),
        }
    }

    /// A paleta do espaço aberto neste quadro (as telas só leem; quem anda a transição é `atualizar_paleta`).
    pub(crate) fn paleta_do_espaco(&self) -> PaletaDoEspaco {
        self.paleta_agora
    }

    /// Anda a transição de cor do espaço. Uma vez por quadro: a animação conta o tempo a cada chamada.
    pub(super) fn atualizar_paleta(&mut self) {
        let base = self.cores();
        let alvo = match self.cor_do_espaco(self.nav.espaco()) {
            Some(cor) => espacos::paleta(cor, &base),
            None => espacos::neutra(&base),
        };
        let t = self.anim.seguir(("espaco-cor", 0), 1.0, TROCA_DE_COR_S);
        self.paleta_agora = espacos::entre(&self.paleta_de, &alvo, t);
    }

    /// O espaço aberto é de um provedor que saiu (desligado ou removido): volta a Tudo. Só depois que a lista dos
    /// provedores chegou, para não perder o espaço lembrado enquanto eles abrem.
    pub(super) fn conferir_espaco(&mut self) {
        let Espaco::Servico(s) = self.nav.espaco().clone() else { return };
        let r = &self.local.remoto;
        let saiu = match r.provedores.iter().find(|p| p.servico == s) {
            None => r.provedores.iter().all(|p| p.situacao != SituacaoDoProvedor::Abrindo),
            Some(p) => matches!(p.situacao, SituacaoDoProvedor::Desligado | SituacaoDoProvedor::Falhou(_)),
        };
        if saiu {
            // Um provedor que não abriu (o Windows bloqueou o programa, por exemplo) só tira o espaço desta vez: o
            // lembrado continua, para a próxima abertura. Desligado ou removido, sai de vez.
            let so_agora =
                r.provedores.iter().any(|p| p.servico == s && matches!(p.situacao, SituacaoDoProvedor::Falhou(_)));
            self.comecar_troca_de_cor();
            self.nav.esquecer_servico(&s);
            if !so_agora {
                self.ajustes.espaco = self.nav.espaco().chave();
                self.salvar();
            }
        }
    }
}
