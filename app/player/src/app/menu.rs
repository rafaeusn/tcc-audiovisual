//! O menu do botão direito e do ⋯: abrir, o teclado dele e executar o que a pessoa escolheu.

use super::*;
use crate::menu::{Acao, Alvo, Contexto, Entrada, entradas};
use crate::navegacao::{Campo, Espaco};
use crate::paginas::{Destino, rota_do_album, rota_do_artista, rota_do_item};
use crate::pares::QuemToca;
use provedores::{Capacidade, Faixa, Item};

/// O menu aberto.
pub(crate) struct MenuAberto {
    pub alvo: Alvo,
    pub entradas: Vec<Entrada>,
    pub ponto: Point<Pixels>,
    /// A linha escolhida pelas setas.
    pub escolhida: Option<usize>,
}

/// O que fazer com as faixas de um álbum, uma playlist ou um artista quando elas chegarem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AoChegar {
    Tocar,
    Aleatorio,
    PorNaFila,
}

impl Player {
    pub(crate) fn abrir_menu(&mut self, alvo: Alvo, ponto: Point<Pixels>) {
        let entradas = entradas(&alvo, &self.contexto_do_menu(&alvo));
        if entradas.is_empty() {
            return;
        }
        self.dica = None;
        self.anim.definir(("menu", 0), 0.0);
        self.menu = Some(MenuAberto { alvo, entradas, ponto, escolhida: None });
    }

    fn contexto_do_menu(&self, alvo: &Alvo) -> Contexto {
        let remoto = &self.local.remoto;
        // Os serviços em que dá para buscar, menos o do item.
        let buscam = |menos: &str| -> Vec<(String, String)> {
            self.espacos()
                .into_iter()
                .filter_map(|(e, nome)| match e {
                    Espaco::Servico(s) if s != menos && self.sabe(&s, Capacidade::Busca) => Some((s, nome)),
                    _ => None,
                })
                .collect()
        };
        let mut c = Contexto {
            na_fila: self.local.fila.itens().len(),
            modelo: self.ajustes.modelo,
            cores_reativas: self.ajustes.cores_reativas,
            modo_seguro: self.ajustes.modo_seguro,
            mudo: self.mudo,
            ilha: self.ajustes.ilha,
            estante: self.estante(),
            estreita: self.barra_apertada(),
            pagina_larga: (self.estante() && self.arranjo.modo != Modo::Empilhado).then_some(self.ajustes.pagina_larga),
            ..Default::default()
        };
        let servico = match alvo {
            Alvo::Faixa { faixa, .. } => faixa.servico.as_str(),
            Alvo::Item(item) => item.servico(),
            _ => "",
        };
        let posicao = match alvo {
            Alvo::Faixa { fila, .. } | Alvo::Local { fila, .. } => *fila,
            _ => None,
        };
        c.e_a_de_agora = posicao.is_some() && posicao == self.local.fila.atual();
        c.curtida = self.faixa_que_toca().map(|f| self.item_salvo(&Item::Faixa(f)));
        c.outros = buscam(servico);
        c.tem_servicos = !c.outros.is_empty();
        if !servico.is_empty() {
            c.toca = remoto.quem_toca(servico) != QuemToca::Ninguem;
            c.sabe_album = self.sabe(servico, Capacidade::Album);
            c.sabe_artista = self.sabe(servico, Capacidade::Artista);
        }
        match alvo {
            Alvo::Faixa { faixa, tambem, .. } => {
                c.salvo = self.item_salvo(&Item::Faixa((**faixa).clone()));
                c.nomes_dos_outros = tambem.iter().map(|f| self.nome_do_servico(&f.servico)).collect();
                c.tem_par = remoto.par(&crate::remoto::musica_de(faixa, "")).is_some();
            }
            Alvo::Item(item) => {
                c.salvo = self.item_salvo(item);
                c.fixado = self.local.salvo(item).is_some_and(|s| s.fixado);
            }
            _ => {}
        }
        c
    }

    /// O alvo de uma música da fila (ou da que toca).
    pub(crate) fn alvo_da_musica(m: &biblioteca::Musica, fila: Option<usize>) -> Alvo {
        match crate::paginas::faixa_da_musica(m) {
            Some(f) => Alvo::Faixa { faixa: Box::new(f), tambem: Vec::new(), fileira: Arc::default(), fila },
            None => Alvo::Local { musica: Box::new(m.clone()), fila },
        }
    }

    /// Tecla com o menu aberto: as setas escolhem, Enter executa, Esc fecha.
    pub(super) fn tecla_no_menu(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.as_mut() else { return };
        let ultimo = menu.entradas.len().saturating_sub(1);
        match ev.keystroke.key.as_str() {
            "down" => menu.escolhida = Some(menu.escolhida.map_or(0, |i| if i == ultimo { 0 } else { i + 1 })),
            "up" => menu.escolhida = Some(menu.escolhida.map_or(ultimo, |i| if i == 0 { ultimo } else { i - 1 })),
            "enter" | "space" => {
                if let Some(acao) = menu.escolhida.and_then(|i| menu.entradas.get(i)).map(|e| e.acao.clone()) {
                    self.executar_do_menu(acao, window, cx);
                }
            }
            _ => self.menu = None,
        }
        cx.notify();
    }

    /// Vai para onde o clique no nome da música (ou do artista) leva.
    pub(crate) fn ir_ao_destino_do_clique(&mut self, destino: Destino) {
        match destino {
            Destino::Rota(rota) => self.abrir_rota(*rota),
            Destino::Buscar(nome) => self.buscar_por(&nome),
            Destino::AlbumLocal(nome) => {
                self.abrir_origem(biblioteca::Origem::Todas);
                self.local.filtrar_album(Some(nome));
            }
            Destino::ArtistaLocal(nome) => {
                self.abrir_origem(biblioteca::Origem::Todas);
                self.local.filtrar_artista(Some(nome));
            }
        }
        self.tirar_o_painel_da_frente();
    }

    /// Toca um item pelo botão do cartão: a faixa na hora (com as outras da fileira); um álbum, uma playlist ou um artista
    /// quando as faixas dele chegarem, sem abrir a página.
    pub(crate) fn tocar_item(&mut self, item: &Item, fileira: &[Faixa]) {
        if item.faixa().is_some() {
            self.abrir_ou_tocar(item, fileira);
        } else {
            self.com_as_faixas(item, AoChegar::Tocar);
        }
    }

    /// Pede as faixas de um álbum, uma playlist ou um artista, para fazer algo com elas quando chegarem.
    fn com_as_faixas(&mut self, item: &Item, fazer: AoChegar) {
        let Some(pedido) = rota_do_item(item).and_then(|r| crate::paginas::pedido_da_rota(&r)) else { return };
        if self.avisar_se_nao_toca(item.servico()) {
            return;
        }
        self.lembrar_a_busca();
        self.local.recente(item);
        // Uma falha de antes não responde por este clique.
        match self.local.remoto.pagina(&pedido).as_deref() {
            Some(crate::remoto::PaginaRemota::Erro(_)) => self.local.remoto.de_novo(&pedido),
            _ => self.local.remoto.abrir_pagina(pedido.clone()),
        }
        self.tocar_ao_chegar = Some((pedido, Instant::now(), fazer));
    }

    /// As faixas chegaram: faz o que a pessoa pediu.
    pub(super) fn fazer_com_as_faixas(&mut self, faixas: &[Faixa], fazer: AoChegar) {
        match fazer {
            AoChegar::Tocar => self.tocar_faixas(faixas, 0),
            AoChegar::Aleatorio => {
                if !self.local.fila.aleatorio {
                    self.local.alternar_aleatorio();
                    self.ajustes.aleatorio = true;
                    self.salvar();
                }
                let i = crate::juntar::sorteio(faixas.len());
                self.tocar_faixas(faixas, i);
            }
            AoChegar::PorNaFila => {
                let musicas = self.musicas_de(faixas);
                let n = musicas.len();
                self.local.adicionar_a_fila(musicas);
                self.avisar(if n == 1 { "1 música na fila".to_string() } else { format!("{n} músicas na fila") });
            }
        }
    }

    fn musicas_de(&self, faixas: &[Faixa]) -> Vec<biblioteca::Musica> {
        faixas.iter().map(|f| crate::remoto::musica_de(f, &self.nome_do_servico(&f.servico))).collect()
    }

    /// Procura um texto só num serviço: o filtro muda e a busca vem pronta.
    fn procurar_no(&mut self, servico: &str, texto: &str) {
        self.buscar_por(texto);
        self.trocar_espaco(Espaco::Servico(servico.to_string()));
        self.focar(Campo::Busca);
    }

    /// Executa uma linha do menu (que fecha).
    pub(crate) fn executar_do_menu(&mut self, acao: Acao, window: &mut Window, cx: &mut Context<Self>) {
        let Some(menu) = self.menu.take() else { return };
        match acao {
            Acao::Comando(comando) => return self.executar(comando, window, cx),
            Acao::SaidaDeSom => return cx.open_url("ms-settings:sound"),
            _ => {}
        }
        match menu.alvo {
            Alvo::Faixa { faixa, tambem, fileira, fila } => self.na_faixa(acao, *faixa, &tambem, &fileira, fila),
            Alvo::Item(item) => self.no_item(&acao, &item),
            Alvo::Local { musica, fila } => self.na_musica(&acao, *musica, fila),
            Alvo::Barra | Alvo::Visual => {}
        }
        cx.notify();
    }

    fn na_faixa(&mut self, acao: Acao, faixa: Faixa, tambem: &[Faixa], fileira: &Arc<Vec<Faixa>>, fila: Option<usize>) {
        let item = Item::Faixa(faixa.clone());
        let musica = || crate::remoto::musica_de(&faixa, &self.nome_do_servico(&faixa.servico));
        match acao {
            Acao::Tocar => match fila {
                Some(i) => self.local.ir_para_na_fila(i),
                None => self.abrir_ou_tocar(&item, fileira),
            },
            Acao::TocarASeguir | Acao::PorNaFila => {
                let m = musica();
                if self.avisar_se_nao_toca(&faixa.servico) {
                    return;
                }
                if acao == Acao::TocarASeguir {
                    self.local.tocar_a_seguir(m);
                    self.avisar(format!("\"{}\" toca a seguir", faixa.titulo));
                } else {
                    self.local.adicionar_a_fila(vec![m]);
                    self.avisar(format!("\"{}\" foi para o fim da fila", faixa.titulo));
                }
            }
            Acao::PorNaPlaylist => {
                let m = musica();
                self.abrir_janela_playlist(vec![m], false);
            }
            Acao::Salvar => self.salvar_com_aviso(&item),
            Acao::VerFaixa => self.abrir_rota(Rota::Faixa(item)),
            Acao::IrAoAlbum => {
                if let Some(rota) = rota_do_album(&faixa, self.sabe(&faixa.servico, Capacidade::Album)) {
                    self.abrir_rota(rota);
                }
            }
            Acao::IrAoArtista(i) => {
                if let Some(rota) = rota_do_artista(&faixa, i, self.sabe(&faixa.servico, Capacidade::Artista)) {
                    self.abrir_rota(rota);
                }
            }
            Acao::TocarPeloOutro(i) => {
                if let Some(f) = tambem.get(i) {
                    self.tocar_faixas(std::slice::from_ref(f), 0);
                }
            }
            Acao::ProcurarNo(servico) => self.procurar_no(&servico, &crate::pares::consulta(&faixa)),
            Acao::Separar => {
                self.separadas.separar(&faixa, tambem);
                self.avisar("Separadas: elas aparecem em linhas diferentes");
            }
            Acao::OutraGravacao => {
                let m = musica();
                self.local.outro_par(&m);
                self.avisar("Procurando outra gravação");
            }
            _ => self.andar_na_fila(&acao, fila),
        }
    }

    fn no_item(&mut self, acao: &Acao, item: &Item) {
        match acao {
            Acao::Abrir => self.abrir_ou_tocar(item, &[]),
            Acao::Tocar => self.com_as_faixas(item, AoChegar::Tocar),
            Acao::Aleatorio => self.com_as_faixas(item, AoChegar::Aleatorio),
            Acao::PorNaFila => self.com_as_faixas(item, AoChegar::PorNaFila),
            Acao::Salvar => self.salvar_com_aviso(item),
            Acao::Fixar(fixar) => {
                self.fixar_item(item, *fixar);
                self.avisar(if *fixar { "Fixado na lateral" } else { "Tirado dos fixados" });
            }
            Acao::ProcurarNo(servico) => {
                let ficha = crate::paginas::cartao(item);
                let texto = match item {
                    Item::Album(a) => format!("{} {}", a.titulo, a.artistas.first().cloned().unwrap_or_default()),
                    _ => ficha.titulo,
                };
                self.procurar_no(servico, texto.trim());
            }
            _ => {}
        }
    }

    fn na_musica(&mut self, acao: &Acao, m: biblioteca::Musica, fila: Option<usize>) {
        match acao {
            Acao::Tocar => match fila {
                Some(i) => self.local.ir_para_na_fila(i),
                None => {
                    // Com as outras da lista em que ela está (a da Biblioteca ou a que a busca achou).
                    let caminho = m.caminho.clone();
                    if self.local.lista.musicas.iter().any(|x| x.caminho == caminho)
                        && self.nav.atual() != Some(&Rota::Busca)
                    {
                        self.local.tocar(usize::MAX, &caminho);
                    } else {
                        self.tocar_achada(&caminho);
                    }
                }
            },
            Acao::TocarASeguir => {
                self.avisar(format!("\"{}\" toca a seguir", m.titulo));
                self.local.tocar_a_seguir(m);
            }
            Acao::PorNaFila => {
                self.avisar(format!("\"{}\" foi para o fim da fila", m.titulo));
                self.local.adicionar_a_fila(vec![m]);
            }
            Acao::PorNaPlaylist => self.abrir_janela_playlist(vec![m], false),
            Acao::IrAoAlbum => self.ir_ao_destino_do_clique(Destino::AlbumLocal(m.album)),
            Acao::IrAoArtista(_) => self.ir_ao_destino_do_clique(Destino::ArtistaLocal(m.artista)),
            Acao::ProcurarNosServicos => {
                let texto = format!("{} {}", m.titulo, m.artista);
                self.buscar_por(texto.trim());
                self.trocar_espaco(Espaco::Tudo);
            }
            _ => self.andar_na_fila(acao, fila),
        }
    }

    fn andar_na_fila(&mut self, acao: &Acao, fila: Option<usize>) {
        let Some(i) = fila else { return };
        match acao {
            Acao::Subir if i > 0 => {
                self.local.fila.mover(i, i - 1);
            }
            Acao::Descer => {
                self.local.fila.mover(i, i + 1);
            }
            Acao::TirarDaFila => {
                if let Some(musica) = self.local.fila.tirar(i) {
                    let texto = format!("\"{}\" saiu da fila", musica.titulo);
                    self.avisar_com_desfazer(texto, Desfazer::TirarDaFila { indice: i, musica: Box::new(musica) });
                }
            }
            _ => {}
        }
    }

    /// Curte ou salva (ou tira), dizendo o que aconteceu.
    fn salvar_com_aviso(&mut self, item: &Item) {
        let salvar = !self.item_salvo(item);
        self.alternar_salvo(item, salvos::PULSO_DO_COMANDO);
        self.avisar(match (salvar, item.faixa().is_some()) {
            (true, true) => "Faixa curtida",
            (false, true) => "Faixa tirada das curtidas",
            (true, false) => "Salvo na biblioteca",
            (false, false) => "Tirado da biblioteca",
        });
    }

    /// Mostra uma aba do painel, abrindo-o. Já à vista, volta para Agora (ou fecha o painel, onde ele fecha). Devolve
    /// `false` quando não há painel nesta janela.
    pub(crate) fn alternar_no_painel(&mut self, aba: AbaDoPainel) -> bool {
        let a_vista = self.painel_a_vista();
        if a_vista && self.aba_do_painel == aba {
            self.aba_do_painel = AbaDoPainel::Agora;
            self.mudar_painel(false);
            return true;
        }
        if !a_vista && !self.mudar_painel(true) {
            return false;
        }
        self.mostrar_aba(aba);
        self.edicao = false;
        true
    }

    /// Troca a aba do painel. A fila abre na música de agora.
    pub(crate) fn mostrar_aba(&mut self, aba: AbaDoPainel) {
        self.aba_do_painel = aba;
        if let (AbaDoPainel::Fila, Some(i)) = (aba, self.local.fila.atual()) {
            self.rolagem_da_fila.scroll_to_item_strict(i, gpui::ScrollStrategy::Top);
        }
    }

    /// O painel está na tela?
    pub(crate) fn painel_a_vista(&self) -> bool {
        let a = &self.arranjo;
        a.painel.is_some() || a.gaveta.is_some()
    }

    /// Abre ou fecha o painel, onde ele abre e fecha.
    fn mudar_painel(&mut self, aberto: bool) -> bool {
        if self.estante() {
            self.gaveta_aberta = aberto;
            return true;
        }
        match self.arranjo.modo {
            Modo::Largo => {
                self.ajustes.painel_aberto = aberto;
                self.salvar();
                true
            }
            Modo::Gaveta => {
                self.gaveta_aberta = aberto;
                true
            }
            Modo::Empilhado => self.arranjo.painel.is_some(),
        }
    }

    /// A letra está na aba dela do painel?
    pub(crate) fn letra_na_aba(&self) -> bool {
        self.painel_a_vista() && self.aba_do_painel == AbaDoPainel::Letra
    }
}

/// O que o painel mostra.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum AbaDoPainel {
    /// Os módulos.
    #[default]
    Agora,
    Fila,
    Letra,
}

impl AbaDoPainel {
    pub const TODAS: [Self; 3] = [Self::Agora, Self::Fila, Self::Letra];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Agora => "Agora",
            Self::Fila => "Fila",
            Self::Letra => "Letra",
        }
    }
}
