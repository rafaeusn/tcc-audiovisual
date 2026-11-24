//! Curtir, salvar e tocar a partir das páginas dos serviços.

use super::*;
use crate::pares::QuemToca;
use crate::remoto::{PaginaRemota, PedidoDePagina};
use biblioteca::TipoSalvo;
use provedores::{Capacidade, Faixa, Item};
use std::sync::Arc;

/// O pulso do coração quando o curtir vem de um comando (não de um botão da tela).
const PULSO_DO_COMANDO: u32 = u32::MAX;

/// Um salvo pela chave (serviço, tipo, id).
pub(crate) type ChaveDoSalvo = (String, TipoSalvo, String);

/// A página de que saíram, as faixas e os álbuns.
#[derive(Clone)]
pub(crate) struct DetalheLido {
    pagina: Arc<PaginaRemota>,
    pub faixas: Arc<Vec<Faixa>>,
    pub albuns: Arc<Vec<Item>>,
}

/// A biblioteca de um espaço, já junta: refeita só quando os salvos ou a conta mudam (a tela a desenha a cada quadro).
pub(crate) struct BibliotecaLida {
    servico: Option<String>,
    salvos: Arc<Vec<(biblioteca::Salvo, Option<Item>)>>,
    paginas: Vec<Arc<PaginaRemota>>,
    itens: Arc<Vec<crate::paginas::NaBiblioteca>>,
}

/// Um app que toca pela conta do Spotify (o oficial, o Spotifast).
fn e_do_spotify(app: &str) -> bool {
    let app = app.to_lowercase();
    app.contains("spotify") || app.contains("spotifast")
}

impl Player {
    /// O que foi salvo no Player e o que está na conta, de um serviço ou de todos.
    pub(crate) fn biblioteca_junta(&self, servico: Option<&str>) -> Arc<Vec<crate::paginas::NaBiblioteca>> {
        let salvos = &self.local.lista.salvos;
        let paginas = self.bibliotecas(servico);
        let mut lida = self.biblioteca_lida.borrow_mut();
        if let Some(b) = lida.as_ref().filter(|b| {
            b.servico.as_deref() == servico
                && Arc::ptr_eq(&b.salvos, salvos)
                && b.paginas.len() == paginas.len()
                && b.paginas.iter().zip(&paginas).all(|(a, b)| Arc::ptr_eq(a, b))
        }) {
            return b.itens.clone();
        }
        let da_conta: Vec<&Item> = paginas
            .iter()
            .flat_map(|p| if let PaginaRemota::Itens(itens) = &**p { itens.as_slice() } else { &[] })
            .collect();
        let itens = Arc::new(crate::paginas::juntar_biblioteca(salvos, &da_conta, servico));
        *lida = Some(BibliotecaLida {
            servico: servico.map(str::to_string),
            salvos: salvos.clone(),
            paginas,
            itens: itens.clone(),
        });
        itens
    }

    /// Ninguém toca as faixas deste serviço agora: avisa e devolve `true`. Pela primeira vez no Match, diz por onde elas
    /// tocam.
    pub(crate) fn avisar_se_nao_toca(&mut self, servico: &str) -> bool {
        match self.local.remoto.quem_toca(servico) {
            QuemToca::Ninguem => {
                let nome = self.nome_do_servico(servico);
                self.avisar(format!("Para tocar as músicas do {nome}, ligue o YouTube Music na Loja"));
                return true;
            }
            QuemToca::Outro(outro) if !self.avisou_do_match => {
                self.avisou_do_match = true;
                let (nome, outro) = (self.nome_do_servico(servico), self.nome_do_servico(&outro));
                self.avisar(format!(
                    "As músicas do {nome} tocam pelo {outro}: a mesma música, às vezes noutra gravação (Configurações › Serviços)"
                ));
            }
            QuemToca::Sozinho if !self.avisou_que_toma_a_vez => {
                // O serviço toca num lugar por vez por conta: quem tocava neste computador para.
                let nome = self.nome_do_servico(servico);
                let outro = self.outro.faixa.as_ref().filter(|f| self.outro.tocando && e_do_spotify(&f.app));
                if let Some(app) = outro.map(|f| f.app.clone()) {
                    self.avisou_que_toma_a_vez = true;
                    self.avisar(format!("O {nome} tocava no {app}: ao tocar aqui, ele para lá"));
                }
            }
            _ => {}
        }
        false
    }

    /// As faixas e os álbuns de uma página de itens (separados uma vez por página).
    pub(crate) fn detalhe(&self, pagina: &Arc<PaginaRemota>) -> DetalheLido {
        let mut lido = self.detalhe_lido.borrow_mut();
        match lido.as_ref().filter(|d| Arc::ptr_eq(&d.pagina, pagina)) {
            Some(d) => d.clone(),
            None => {
                let (faixas, albuns) = match &**pagina {
                    PaginaRemota::Itens(itens) => crate::paginas::faixas_e_albuns(itens),
                    _ => (Vec::new(), Vec::new()),
                };
                let d = DetalheLido { pagina: pagina.clone(), faixas: Arc::new(faixas), albuns: Arc::new(albuns) };
                *lido = Some(d.clone());
                d
            }
        }
    }

    /// Os serviços que têm a biblioteca da conta para mostrar (entrou na conta), de um espaço ou de todos.
    pub(crate) fn servicos_com_biblioteca(&self, servico: Option<&str>) -> Vec<String> {
        self.local
            .remoto
            .provedores
            .iter()
            .filter(|p| servico.is_none_or(|s| p.servico == s))
            .filter(|p| p.capacidades.contains(&Capacidade::Biblioteca))
            .filter(|p| p.conta.as_ref().is_none_or(|c| c.situacao == provedores::SituacaoDaConta::Dentro))
            .map(|p| p.servico.clone())
            .collect()
    }

    /// O que se sabe das bibliotecas das contas, de um espaço ou de todos.
    pub(crate) fn bibliotecas(&self, servico: Option<&str>) -> Vec<Arc<PaginaRemota>> {
        self.servicos_com_biblioteca(servico)
            .into_iter()
            .filter_map(|servico| self.local.remoto.pagina(&PedidoDePagina::Biblioteca { servico }))
            .collect()
    }

    /// Fixa ou desfixa; o que só está na conta é salvo no Player antes (a biblioteca atende em ordem).
    pub(crate) fn fixar_item(&mut self, item: &Item, fixar: bool) {
        if fixar && !self.item_salvo(item) {
            self.alternar_salvo(item, PULSO_DO_COMANDO);
        }
        self.local.fixar(item, fixar);
    }

    /// O item está salvo? Conta o clique de agora antes de a biblioteca confirmar (dois cliques rápidos ficam no
    /// estado do último).
    pub(crate) fn item_salvo(&self, item: &Item) -> bool {
        let chave = crate::paginas::chave_do_salvo(item);
        self.salvos_pendentes.get(&chave).copied().unwrap_or_else(|| self.local.salvo(item).is_some())
    }

    /// Curte ou tira dos salvos; o coração pulsa.
    pub(crate) fn alternar_salvo(&mut self, item: &Item, pulso: u32) {
        let salvar = !self.item_salvo(item);
        if salvar {
            self.local.salvar(item);
        } else {
            self.local.tirar_salvo(item);
        }
        self.salvos_pendentes.insert(crate::paginas::chave_do_salvo(item), salvar);
        self.anim.definir(("coracao", pulso), 1.35);
    }

    /// Curte (ou descurte) a faixa de serviço que toca no Player. Usa a versão mais completa que houver (a da página
    /// aberta ou a da busca, com os ids de álbum e artista); senão, a que a fila guarda.
    pub(crate) fn curtir_a_que_toca(&mut self) {
        let tocando = if self.origem_local { self.local.atual() } else { None };
        let Some(m) = tocando else {
            self.avisar("Nada tocando no Player para curtir");
            return;
        };
        let Some(faixa) = crate::paginas::faixa_da_musica(m) else {
            self.avisar("Curtir por aqui vale para as faixas dos serviços");
            return;
        };
        let chave = faixa.chave();
        let completa = self
            .nav
            .atual()
            .and_then(crate::paginas::item_da_rota)
            .into_iter()
            .chain(self.local.remoto.busca.itens.iter())
            .find(|i| matches!(i, Item::Faixa(f) if f.chave() == chave))
            .cloned();
        let item = completa.unwrap_or(Item::Faixa(faixa));
        let curtir = !self.item_salvo(&item);
        self.alternar_salvo(&item, PULSO_DO_COMANDO);
        self.avisar(if curtir { "Faixa curtida" } else { "Faixa tirada das curtidas" });
    }

    /// Fixa (ou desfixa) na lateral o item da página aberta; salva antes, se ele não estava salvo.
    pub(crate) fn fixar_a_pagina(&mut self) {
        let Some(item) = self.nav.atual().and_then(crate::paginas::item_da_rota).cloned() else {
            self.avisar("Abra um álbum, uma playlist, um artista ou uma faixa para fixar");
            return;
        };
        let fixado = self.local.salvo(&item).is_some_and(|s| s.fixado);
        self.fixar_item(&item, !fixado);
        self.avisar(if fixado { "Tirado dos fixados" } else { "Fixado na lateral" });
    }

    /// Esquece os cliques que a biblioteca já confirmou.
    pub(super) fn conferir_salvos_pendentes(&mut self) {
        let salvos = self.local.lista.salvos.clone();
        self.salvos_pendentes.retain(|(servico, tipo, id), quer| {
            let esta = salvos.iter().any(|(s, _)| &s.servico == servico && s.tipo == *tipo && &s.id == id);
            esta != *quer
        });
    }

    /// Toca em ordem aleatória: o aleatório liga antes (a fila embaralha todas as outras, nenhuma fica para trás).
    pub(crate) fn tocar_aleatorio(&mut self, faixas: &[Faixa], window: &mut Window, cx: &mut Context<Self>) {
        let Some(primeira) = faixas.first() else { return };
        // Antes de mexer no aleatório da fila que está tocando.
        let servico = primeira.servico.clone();
        if self.avisar_se_nao_toca(&servico) {
            return;
        }
        if !self.local.fila.aleatorio {
            self.executar(nucleo::Comando::AlternarAleatorio, window, cx);
        }
        let i = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos())
            as usize
            % faixas.len();
        self.tocar_faixas(faixas, i);
    }

    /// A rolagem de uma prateleira (guardada entre os quadros, para as setas).
    pub(crate) fn rolagem_da_prateleira(&self, k: usize) -> gpui::ScrollHandle {
        self.rolagens.borrow_mut().entry(k).or_default().clone()
    }

    /// Toca as faixas a partir da `i` (o resto vai para a fila).
    pub(crate) fn tocar_faixas(&mut self, faixas: &[Faixa], i: usize) {
        if faixas.is_empty() {
            return;
        }
        // Um serviço que ninguém toca: avisa, em vez de tentar a lista inteira e falhar em cada uma.
        let remoto = &self.local.remoto;
        let nao_toca = |servico: &str| remoto.quem_toca(servico) == QuemToca::Ninguem;
        let servico = faixas.get(i).or(faixas.last()).map(|f| f.servico.clone()).unwrap_or_default();
        let tocam = crate::paginas::so_as_que_tocam(faixas, i, nao_toca);
        self.avisar_se_nao_toca(&servico);
        let Some((faixas, i)) = tocam else { return };
        let musicas: Vec<biblioteca::Musica> =
            faixas.iter().map(|f| crate::remoto::musica_de(f, &self.nome_do_servico(&f.servico))).collect();
        if !musicas.is_empty() {
            self.local.tocar_lista(musicas, i.min(faixas.len() - 1));
        }
    }

    /// O nome do provedor de um serviço ("YouTube Music").
    pub(crate) fn nome_do_servico(&self, servico: &str) -> String {
        self.local
            .remoto
            .provedores
            .iter()
            .find(|p| p.servico == servico)
            .map_or_else(|| servico.to_string(), |p| p.nome.clone())
    }

    /// O provedor do serviço sabe fazer isso?
    pub(crate) fn sabe(&self, servico: &str, capacidade: Capacidade) -> bool {
        self.local.remoto.provedores.iter().any(|p| p.servico == servico && p.capacidades.contains(&capacidade))
    }
}
