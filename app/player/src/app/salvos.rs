//! Curtir, salvar e tocar a partir das páginas dos serviços.

use super::*;
use crate::pares::QuemToca;
use crate::remoto::{PaginaRemota, PedidoDePagina};
use biblioteca::TipoSalvo;
use provedores::{Capacidade, Faixa, Item};
use std::sync::Arc;

/// O pulso do coração quando o curtir vem de um comando (não de um botão da tela).
pub(super) const PULSO_DO_COMANDO: u32 = u32::MAX;

/// Quanto esperar as faixas do que a pessoa mandou tocar pelo cartão.
const ESPERA_PARA_TOCAR: Duration = Duration::from_secs(25);

/// Um salvo pela chave (serviço, tipo, id).
pub(crate) type ChaveDoSalvo = (String, TipoSalvo, String);

/// A página de que saíram, as faixas e os álbuns.
#[derive(Clone)]
pub(crate) struct DetalheLido {
    pagina: Arc<PaginaRemota>,
    pub faixas: Arc<Vec<Faixa>>,
    pub albuns: Arc<Vec<Item>>,
    pub detalhe: Option<Arc<provedores::Detalhe>>,
}

/// A biblioteca de um filtro, já junta: refeita só quando os salvos ou a conta mudam (a tela a desenha a cada quadro).
pub(crate) struct BibliotecaLida {
    salvos: Arc<Vec<(biblioteca::Salvo, Option<Item>)>>,
    paginas: Vec<Arc<PaginaRemota>>,
    itens: Arc<Vec<crate::paginas::NaBiblioteca>>,
}

/// O que a Biblioteca aberta mostra: as posições, do tipo e com o texto do filtro, na ordem escolhida; e as faixas entre
/// elas (a fila de quem toca uma). Refeito só quando algo disso muda.
pub(crate) struct Arrumada {
    junta: Arc<Vec<crate::paginas::NaBiblioteca>>,
    pedido: (Option<TipoSalvo>, String, crate::paginas::Ordem),
    pub posicoes: Arc<Vec<usize>>,
    pub faixas: Arc<Vec<Faixa>>,
}

/// Os resultados da busca juntos. `de` diz de que resposta, de que filtro e com que preferência são.
pub(crate) struct BuscaLida {
    de: (u64, usize, usize, Vec<String>, crate::juntar::Regras),
    junta: Arc<crate::juntar::BuscaJunta>,
}

/// O Início junto, das páginas e dos recentes de que saiu.
pub(crate) struct InicioLido {
    paginas: Vec<(String, Arc<PaginaRemota>)>,
    recentes: Arc<Vec<Item>>,
    regras: crate::juntar::Regras,
    inicio: Arc<crate::juntar::Inicio>,
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
        let mut lidas = self.biblioteca_lida.borrow_mut();
        let chave = servico.map(str::to_string);
        if let Some(b) = lidas.get(&chave).filter(|b| {
            Arc::ptr_eq(&b.salvos, salvos)
                && b.paginas.len() == paginas.len()
                && b.paginas.iter().zip(&paginas).all(|(a, b)| Arc::ptr_eq(a, b))
        }) {
            return b.itens.clone();
        }
        let da_conta: Vec<&Item> = paginas
            .iter()
            .flat_map(|p| if let PaginaRemota::Itens(itens, _) = &**p { itens.as_slice() } else { &[] })
            .collect();
        let itens = Arc::new(crate::paginas::juntar_biblioteca(salvos, &da_conta, servico));
        lidas.insert(chave, BibliotecaLida { salvos: salvos.clone(), paginas, itens: itens.clone() });
        itens
    }

    /// Por que serviço tocar a música que está em mais de um: o preferido, depois os que tocam por eles mesmos, depois os
    /// outros.
    fn preferencia(&self) -> Vec<String> {
        let provedores = &self.local.remoto.provedores;
        let mut ordem: Vec<(u8, String)> = provedores
            .iter()
            .map(|p| {
                let peso = if p.servico == self.ajustes.servico_preferido {
                    0
                } else {
                    match self.local.remoto.quem_toca(&p.servico) {
                        QuemToca::Ele | QuemToca::Sozinho => 1,
                        _ => 2,
                    }
                };
                (peso, p.servico.clone())
            })
            .collect();
        ordem.sort_by_key(|(peso, _)| *peso);
        ordem.into_iter().map(|(_, s)| s).collect()
    }

    fn regras_para_juntar(&self) -> crate::juntar::Regras {
        crate::juntar::Regras { preferencia: self.preferencia(), separadas: self.separadas.pares() }
    }

    /// O que a busca achou nos serviços, junto.
    pub(crate) fn busca_junta(&self, servicos: &[String]) -> Arc<crate::juntar::BuscaJunta> {
        let busca = &self.local.remoto.busca;
        let regras = self.regras_para_juntar();
        let de = (busca.geracao, busca.respondidos.len(), busca.itens.len(), servicos.to_vec(), regras);
        let mut lida = self.busca_lida.borrow_mut();
        if let Some(l) = lida.as_ref().filter(|l| l.de == de) {
            return l.junta.clone();
        }
        let grupos: Vec<crate::paginas::GruposDaBusca> =
            servicos.iter().map(|s| crate::paginas::agrupar_busca(&busca.itens, Some(s))).collect();
        let grupos: Vec<&crate::paginas::GruposDaBusca> = grupos.iter().collect();
        let junta = Arc::new(crate::juntar::juntar_busca(&busca.texto, &grupos, &de.4));
        *lida = Some(BuscaLida { de, junta: junta.clone() });
        junta
    }

    /// O Início dos serviços dados (os que já responderam), junto com os recentes do filtro.
    pub(crate) fn inicio_junto(&self, paginas: Vec<(String, Arc<PaginaRemota>)>) -> Arc<crate::juntar::Inicio> {
        let recentes = self.local.lista.recentes.clone();
        let regras = self.regras_para_juntar();
        let mut lido = self.inicio_lido.borrow_mut();
        if let Some(l) = lido.as_ref().filter(|l| {
            Arc::ptr_eq(&l.recentes, &recentes)
                && l.regras == regras
                && l.paginas.len() == paginas.len()
                && l.paginas.iter().zip(&paginas).all(|(a, b)| a.0 == b.0 && Arc::ptr_eq(&a.1, &b.1))
        }) {
            return l.inicio.clone();
        }
        let prateleiras: Vec<(String, &[provedores::Prateleira])> = paginas
            .iter()
            .filter_map(|(s, p)| match &**p {
                PaginaRemota::Prateleiras(v) => Some((s.clone(), v.as_slice())),
                _ => None,
            })
            .collect();
        // Num serviço, só os recentes dele.
        let do_filtro: Vec<Item> = match self.servico_do_filtro() {
            Some(s) => recentes.iter().filter(|i| i.servico() == s).cloned().collect(),
            None => recentes.to_vec(),
        };
        let inicio = Arc::new(crate::juntar::juntar_inicio(&prateleiras, &do_filtro, &regras));
        *lido = Some(InicioLido { paginas, recentes, regras, inicio: inicio.clone() });
        inicio
    }

    /// As faixas do que a pessoa mandou tocar pelo cartão chegaram (ou não vieram)?
    pub(super) fn conferir_tocar_ao_chegar(&mut self) {
        let Some((pedido, desde, fazer)) = self.tocar_ao_chegar.clone() else { return };
        // (O pedido de agora pode ainda não ter aparecido.)
        let pagina = self.local.remoto.pagina(&pedido).unwrap_or_else(|| Arc::new(PaginaRemota::Carregando));
        match &*pagina {
            PaginaRemota::Carregando if desde.elapsed() < ESPERA_PARA_TOCAR => {}
            PaginaRemota::Itens(itens, _) => {
                self.tocar_ao_chegar = None;
                let (faixas, _) = crate::paginas::faixas_e_albuns(itens);
                if faixas.is_empty() {
                    self.avisar("Não veio nenhuma música para tocar");
                } else {
                    self.fazer_com_as_faixas(&faixas, fazer);
                }
            }
            PaginaRemota::Erro(motivo) => {
                self.tocar_ao_chegar = None;
                self.avisar(motivo.clone());
            }
            _ => {
                self.tocar_ao_chegar = None;
                self.avisar("O serviço demorou a responder");
            }
        }
    }

    /// A página aberta por um link chega sem a foto (ou a capa): quando o serviço manda o item completo, ele entra nos
    /// recentes no lugar do incompleto (uma vez por página).
    pub(super) fn completar_o_recente(&mut self) {
        let passo = self.nav.passo();
        if self.recente_completo == Some(passo) {
            return;
        }
        let Some(rota) = self.nav.atual() else { return };
        let Some(pedido) = crate::paginas::pedido_da_rota(rota) else { return };
        let Some(PaginaRemota::Itens(_, Some(detalhe))) = self.local.remoto.pagina(&pedido).as_deref().cloned() else {
            return;
        };
        self.recente_completo = Some(passo);
        if let (Some(completo), Some(aberto)) = (&detalhe.item, rota.item())
            && crate::paginas::chave_do_salvo(completo) == crate::paginas::chave_do_salvo(aberto)
            && crate::paginas::cartao(completo).capa.is_some()
            && crate::paginas::cartao(aberto).capa.is_none()
        {
            self.local.recente(completo);
        }
    }

    /// A música de serviço que começou a tocar entra nos recentes.
    pub(super) fn lembrar_o_que_toca(&mut self) {
        let tocando = self.local.atual().filter(|_| self.local.situacao.tocando);
        let Some(m) = tocando else { return };
        if self.recente_visto.as_ref() == Some(&m.caminho) {
            return;
        }
        self.recente_visto = Some(m.caminho.clone());
        if let Some(f) = crate::paginas::faixa_da_musica(m) {
            self.local.recente(&Item::Faixa(f));
        }
    }

    /// O que a Biblioteca mostra do tipo, com o texto do filtro dela e na ordem escolhida.
    pub(crate) fn arrumar(
        &self,
        junta: &Arc<Vec<crate::paginas::NaBiblioteca>>,
        tipo: Option<TipoSalvo>,
    ) -> (Arc<Vec<usize>>, Arc<Vec<Faixa>>) {
        let texto = self.buscas.biblioteca.trim();
        let ordem = self.ordem_da_biblioteca;
        let mut arrumada = self.biblioteca_arrumada.borrow_mut();
        if let Some(a) = arrumada.as_ref().filter(|a| {
            Arc::ptr_eq(&a.junta, junta) && a.pedido.0 == tipo && a.pedido.1 == texto && a.pedido.2 == ordem
        }) {
            return (a.posicoes.clone(), a.faixas.clone());
        }
        let posicoes = Arc::new(crate::paginas::arrumar_biblioteca(junta, tipo, texto, ordem));
        let faixas = Arc::new(posicoes.iter().filter_map(|i| junta[*i].item.faixa().cloned()).collect::<Vec<_>>());
        *arrumada = Some(Arrumada {
            junta: junta.clone(),
            pedido: (tipo, texto.to_string(), ordem),
            posicoes: posicoes.clone(),
            faixas: faixas.clone(),
        });
        (posicoes, faixas)
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
                let ((faixas, albuns), detalhe) = match &**pagina {
                    PaginaRemota::Itens(itens, detalhe) => (crate::paginas::faixas_e_albuns(itens), detalhe.clone()),
                    _ => ((Vec::new(), Vec::new()), None),
                };
                let d =
                    DetalheLido { pagina: pagina.clone(), faixas: Arc::new(faixas), albuns: Arc::new(albuns), detalhe };
                *lido = Some(d.clone());
                d
            }
        }
    }

    /// Os serviços que têm a biblioteca da conta para mostrar (entrou na conta), de um serviço ou de todos.
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

    /// O que se sabe das bibliotecas das contas, de um serviço ou de todos.
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
        self.salvos_pendentes.get(&chave).map(|(quer, _)| *quer).unwrap_or_else(|| self.local.salvo(item).is_some())
    }

    /// Curte ou tira dos salvos; o coração pulsa.
    pub(crate) fn alternar_salvo(&mut self, item: &Item, pulso: u32) {
        let salvar = !self.item_salvo(item);
        if salvar {
            self.local.salvar(item);
        } else {
            self.local.tirar_salvo(item);
        }
        self.salvos_pendentes.insert(crate::paginas::chave_do_salvo(item), (salvar, self.local.enviados()));
        self.anim.definir(("coracao", pulso), 1.35);
    }

    /// Curte (ou descurte) a faixa de serviço que toca no Player. Usa a versão mais completa que houver (a da página
    /// aberta ou a da busca, com os ids de álbum e artista); senão, a que a fila guarda.
    /// A faixa de serviço que está tocando no Player.
    pub(crate) fn faixa_que_toca(&self) -> Option<Faixa> {
        self.local.atual().filter(|_| self.origem_local).and_then(crate::paginas::faixa_da_musica)
    }

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
            .and_then(Rota::item)
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
        let Some(item) = self.nav.atual().and_then(Rota::item).cloned() else {
            self.avisar("Abra um álbum, uma playlist, um artista ou uma faixa para fixar");
            return;
        };
        let fixado = self.local.salvo(&item).is_some_and(|s| s.fixado);
        self.fixar_item(&item, !fixado);
        self.avisar(if fixado { "Tirado dos fixados" } else { "Fixado na lateral" });
    }

    /// Esquece os cliques que a biblioteca já confirmou.
    pub(super) fn conferir_salvos_pendentes(&mut self) {
        // Só quando a biblioteca atendeu o último clique: com cliques rápidos, uma lista do meio do caminho
        // mostraria o estado de antes, e o coração piscava.
        let atendidos = self.local.lista.atendidos;
        self.salvos_pendentes.retain(|_, (_, pedido)| atendidos < *pedido);
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
        let i = crate::juntar::sorteio(faixas.len());
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
        self.lembrar_a_busca();
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
