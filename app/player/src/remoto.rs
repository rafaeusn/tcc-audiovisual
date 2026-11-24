//! O lado remoto do player: as músicas que vêm dos provedores (YouTube Music, Spotify...), par do [`crate::local`].
//!
//! Cada provedor instalado e ligado (`<pasta de dados>/provedores`) ganha uma thread própria, que abre o programa
//! dele e atende os pedidos em ordem; um provedor lento ou travado não segura os outros nem a interface. A
//! interface pede (buscar, resolver o endereço de uma faixa, ligar, desligar, instalar, remover) e lê o que já
//! chegou; nunca espera.
//!
//! A busca vai para todos os provedores ligados ao mesmo tempo, depois de uma pausa curta na digitação, e os
//! resultados chegam aos poucos. Cada resultado vira uma [`Musica`] com a parte remota preenchida (a chave
//! `servico:id` no lugar do caminho), e assim a fila, o dock, a Ilha, o histórico e a letra a tratam como qualquer
//! outra.
//!
//! Alguns provedores vêm com o app ([`EMBUTIDOS`]): o programa fica ao lado do `player.exe` e entra na lista sem
//! instalar nada (dá para desligar, não remover; um instalado do mesmo serviço vale mais). Cada provedor recebe a
//! sua pasta de dados em `TCCPLAYER_PROVEDOR_DADOS` (`<pasta de dados>/provedores-dados/<serviço>`).
//!
//! As páginas dos serviços (início, álbum, artista, playlist) são pedidas do mesmo jeito, à thread do provedor
//! dele, e ficam num cache curto ([`VALIDADE_DA_PAGINA`]): voltar a uma página não pergunta de novo. A identidade
//! do provedor (cor e logo) chega quando ele abre; a logo vai para a pasta de dados dele (`logo.svg`).
//!
//! Instalar, atualizar e remover mexem na pasta do provedor, e o Windows não deixa mexer na pasta de um programa
//! aberto: a thread dele é encerrada primeiro (o programa fecha junto), e a cópia ou a mudança de pasta acontecem
//! em segundo plano. No fim, a lista é lida de novo.

use crate::cadeia::Jeito;
use crate::externo::{Abertos, Lido, Tocadores};
use crate::pares::{Par, Pares, QuemToca, consulta, escolher, quem_toca, tem};
use biblioteca::{Musica, Remota};
use nucleo::CausaDoProvedor;
use provedores::{
    Acao, Capacidade, Conta, CorpoDoPedido, Faixa, Fonte, Identidade, Instalado, Item, Manifesto, Prateleira, Provedor,
    Segredo, SituacaoDaConta, SituacaoDoTocador,
};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

/// Depois da última tecla, quanto esperar para mandar a busca aos provedores.
const PAUSA_NA_DIGITACAO: Duration = Duration::from_millis(350);
/// Quantos resultados pedir a cada provedor.
const RESULTADOS: u32 = 20;
/// Prazo de uma busca e de uma resolução.
const PRAZO: Duration = Duration::from_secs(20);
/// Quanto tempo uma página de serviço vale no cache.
const VALIDADE_DA_PAGINA: Duration = Duration::from_secs(10 * 60);
/// Quantas vezes seguidas um provedor que fechou é reaberto.
const REABERTURAS: u32 = 3;
/// Os provedores que vêm com o app (serviço, nome, programa ao lado do `player.exe`).
pub const EMBUTIDOS: [(&str, &str, &str); 2] =
    [("youtube-music", "YouTube Music", "provedor-youtube-music.exe"), ("spotify", "Spotify", "provedor-spotify.exe")];

/// De quanto em quanto tempo perguntar a conta enquanto ela está `entrando` (a pessoa aprova no navegador).
const PERGUNTA_DA_CONTA: Duration = Duration::from_secs(2);
/// Quanto esperar a resposta (a conta vem da memória do provedor; um mudo não pode atrasar as outras tarefas).
const PRAZO_DA_CONTA: Duration = Duration::from_secs(3);
/// Quantas vezes perguntar a conta depois de um erro, até o provedor responder.
const DUVIDAS_DA_CONTA: u8 = 10;
/// Até quando perguntar (o provedor desiste do navegador em 5 min).
const ACOMPANHAR_A_CONTA: Duration = Duration::from_secs(6 * 60);

/// Os provedores embutidos cujo programa está em `pasta` (a do `player.exe`).
fn embutidos_em(pasta: &Path) -> Vec<Instalado> {
    EMBUTIDOS
        .iter()
        .filter(|(_, _, exe)| pasta.join(exe).is_file())
        .map(|(servico, nome, exe)| Instalado {
            manifesto: Manifesto {
                nome: (*nome).into(),
                servico: (*servico).into(),
                executavel: (*exe).into(),
                argumentos: Vec::new(),
                versao: String::new(),
            },
            pasta: pasta.to_path_buf(),
        })
        .collect()
}

/// Os instalados pela pessoa mais os embutidos que ela não trocou por um instalado do mesmo serviço; e quais
/// deles são embutidos.
fn juntar(mut instalados: Vec<Instalado>, embutidos: Vec<Instalado>) -> (Vec<Instalado>, HashSet<String>) {
    let mut deles = HashSet::new();
    for e in embutidos {
        if !instalados.iter().any(|i| i.manifesto.servico == e.manifesto.servico) {
            deles.insert(e.manifesto.servico.clone());
            instalados.push(e);
        }
    }
    (instalados, deles)
}

/// Como está um provedor.
#[derive(Clone, Debug, PartialEq)]
pub enum Situacao {
    Abrindo,
    Pronto,
    Desligado,
    Falhou(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct InfoDoProvedor {
    pub nome: String,
    pub servico: String,
    pub versao: String,
    /// O que ele sabe fazer (conhecido depois que ele abre).
    pub capacidades: Vec<Capacidade>,
    pub situacao: Situacao,
    /// Vem com o app (não dá para remover, só desligar).
    pub embutido: bool,
    /// A cor da marca (`0xRRGGBB`), quando o provedor manda.
    pub cor: Option<u32>,
    /// A logo, gravada na pasta de dados do provedor, quando ele manda.
    pub logo: Option<PathBuf>,
    /// A conta, para os provedores que entram numa (capacidade `Conta`); `None` para os outros.
    pub conta: Option<Conta>,
}

/// Uma página de um serviço.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PedidoDePagina {
    Inicio {
        servico: String,
    },
    Album {
        servico: String,
        id: String,
    },
    Artista {
        servico: String,
        id: String,
    },
    Playlist {
        servico: String,
        id: String,
    },
    /// O que a conta da pessoa tem salvo no serviço.
    Biblioteca {
        servico: String,
    },
}

impl PedidoDePagina {
    pub fn servico(&self) -> &str {
        match self {
            Self::Inicio { servico }
            | Self::Biblioteca { servico }
            | Self::Album { servico, .. }
            | Self::Artista { servico, .. }
            | Self::Playlist { servico, .. } => servico,
        }
    }

    /// A capacidade que o provedor precisa ter para responder.
    fn capacidade(&self) -> Capacidade {
        match self {
            Self::Inicio { .. } => Capacidade::Inicio,
            Self::Album { .. } => Capacidade::Album,
            Self::Artista { .. } => Capacidade::Artista,
            Self::Playlist { .. } => Capacidade::Playlists,
            Self::Biblioteca { .. } => Capacidade::Biblioteca,
        }
    }
}

/// O que se sabe de uma página pedida.
#[derive(Clone, Debug, PartialEq)]
pub enum PaginaRemota {
    Carregando,
    Prateleiras(Vec<Prateleira>),
    Itens(Vec<Item>),
    Erro(String),
}

/// A página guardada há mais tempo que [`VALIDADE_DA_PAGINA`] é pedida de novo.
fn vencida(desde: Instant, agora: Instant) -> bool {
    agora.saturating_duration_since(desde) > VALIDADE_DA_PAGINA
}

/// Grava a logo válida em `pasta/logo.svg` e devolve o caminho. Sem logo válida, apaga a antiga.
fn guardar_logo(pasta: &Path, identidade: Option<&Identidade>) -> Option<PathBuf> {
    let arquivo = pasta.join("logo.svg");
    match identidade.and_then(Identidade::logo_valida) {
        Some(logo) => {
            let gravou = std::fs::create_dir_all(pasta).and_then(|()| std::fs::write(&arquivo, logo));
            match gravou {
                Ok(()) => Some(arquivo),
                Err(e) => {
                    log::warn!("a logo do provedor não foi gravada em {}: {e}", arquivo.display());
                    None
                }
            }
        }
        None => {
            let _ = std::fs::remove_file(&arquivo);
            None
        }
    }
}

/// O resultado da busca de agora, juntando o que cada provedor já mandou.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Busca {
    pub texto: String,
    pub geracao: u64,
    /// Tudo o que cada provedor mandou (faixas, álbuns, artistas, playlists), na ordem dele: o melhor primeiro.
    pub itens: Vec<Item>,
    /// Os serviços que já responderam (com resultado ou erro).
    pub respondidos: Vec<String>,
    /// Quantos provedores ainda não responderam.
    pub faltam: usize,
    pub erros: Vec<String>,
}

/// O endereço de uma faixa e, pelo Match, de que gravação ele é.
struct FonteLida {
    endereco: Result<tocador::Endereco, String>,
    par: Option<Par>,
    /// Por que não deu, quando o provedor soube classificar (a cadeia de reserva decide a partir dela).
    causa: Option<CausaDoProvedor>,
}

#[derive(Default)]
struct Estado {
    provedores: Vec<InfoDoProvedor>,
    busca: Busca,
    /// O endereço de cada faixa resolvida (pela chave), ou por que não deu.
    fontes: HashMap<String, FonteLida>,
    /// Avisos das tarefas de segundo plano (instalou, removeu, não deu).
    avisos: Vec<String>,
    /// Um provedor foi removido (o serviço): o aviso dele ganha "Desfazer".
    removido: Option<String>,
    /// A pasta dos provedores mudou: a lista tem de ser lida de novo.
    recarregar: bool,
    /// As páginas pedidas, com a hora em que chegaram (ou foram pedidas).
    paginas: HashMap<PedidoDePagina, (Instant, Arc<PaginaRemota>)>,
}

enum Tarefa {
    Buscar {
        texto: String,
        geracao: u64,
    },
    Resolver {
        id: String,
        chave: String,
    },
    /// Acha, neste serviço, a mesma música da faixa `chave` e dá o endereço dela. `recusar`: a pessoa não quis a de agora.
    Casar {
        alvo: Box<Faixa>,
        chave: String,
        recusar: bool,
    },
    Pagina(PedidoDePagina),
    Entrar,
    Sair,
    /// Só para reabrir um provedor que fechou.
    Acordar,
}

/// A thread de um provedor: soltar `tarefas` a encerra (e o programa fecha); `fim` espera isso acontecer.
struct Trabalhador {
    tarefas: Sender<Tarefa>,
    fim: JoinHandle<()>,
}

fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// O motivo para mostrar: sem o "provedor:" que o tipo do erro põe na frente (o nome do provedor já vai junto).
pub(crate) fn motivo(e: &nucleo::Erro) -> String {
    e.mensagem()
}

/// Uma faixa de provedor como [`Musica`].
pub fn musica_de(f: &Faixa, fonte: &str) -> Musica {
    Musica {
        caminho: PathBuf::from(f.chave()),
        titulo: f.titulo.clone(),
        artista: f.artistas_juntos(),
        album: f.album.clone().unwrap_or_default(),
        duracao_s: f.duracao_s.filter(|d| d.is_finite() && *d > 0.0).unwrap_or(0.0),
        remota: Some(Remota {
            servico: f.servico.clone(),
            id: f.id.clone(),
            isrc: f.isrc_limpo(),
            capa: f.capa.clone(),
            fonte: fonte.to_string(),
        }),
        ..Default::default()
    }
}

/// A fonte que o provedor deu, no formato do tocador.
fn endereco_de(fonte: Fonte) -> Result<tocador::Endereco, String> {
    match fonte {
        Fonte::Endereco { url, cabecalhos, formato, .. } => Ok(tocador::Endereco { url, cabecalhos, formato }),
        // Quem toca sozinho é acompanhado pelo tocador do serviço (`externo.rs`), e não por um endereço.
        Fonte::TocaSozinho { .. } => Err("este provedor não deu um endereço para tocar".into()),
    }
}

pub struct Remoto {
    /// A pasta dos provedores (sem pasta de dados, nenhum).
    pasta: Option<PathBuf>,
    instalados: Vec<Instalado>,
    desligados: HashSet<String>,
    estado: Arc<Mutex<Estado>>,
    /// Os pares do Match.
    pares: Arc<Mutex<Pares>>,
    /// Os programas abertos, para os tocadores dos serviços.
    abertos: Abertos,
    tocadores: Tocadores,
    /// As faixas que o tocador do serviço não tocou nesta abertura: vão pelo Match.
    pelo_match: HashSet<String>,
    /// Os jeitos de tocar que caíram nesta abertura, por serviço, com o motivo da primeira queda.
    caidos: HashMap<String, (HashSet<Jeito>, CausaDoProvedor)>,
    versao: Arc<AtomicU64>,
    versao_vista: u64,
    /// A thread de cada provedor ligado, pelo serviço.
    trabalhadores: HashMap<String, Trabalhador>,
    /// A busca que espera a pausa na digitação.
    busca_pedida: Option<(String, Instant)>,
    geracao: u64,
    /// As faixas cujo endereço já foi pedido (para não pedir de novo a cada quadro).
    pedidas: HashSet<String>,
    /// O que a tela mostra (copiado quando muda).
    pub busca: Busca,
    pub provedores: Vec<InfoDoProvedor>,
    /// As páginas dos serviços (copiadas quando mudam; o `Arc` deixa a cópia barata).
    paginas: HashMap<PedidoDePagina, Arc<PaginaRemota>>,
    /// Um aviso para a pessoa: o app mostra e limpa.
    pub aviso: Option<String>,
    /// O provedor removido por último (o serviço), para o "Desfazer" do aviso.
    pub removido: Option<String>,
}

impl Remoto {
    /// Abre os provedores instalados em `pasta`, menos os `desligados` (cada um na sua thread; voltar não espera
    /// nenhum abrir).
    pub fn iniciar(pasta: Option<PathBuf>, desligados: &[String]) -> Self {
        let arquivo = pasta.as_deref().and_then(Path::parent).map(|d| d.join("pares.json"));
        let versao = Arc::new(AtomicU64::new(1));
        let abertos = Abertos::default();
        let estado: Arc<Mutex<Estado>> = Arc::default();
        let (mudou, avisos) = (versao.clone(), estado.clone());
        let tocadores = Tocadores::iniciar(abertos.clone(), move |aviso| {
            travar(&avisos).avisos.extend(aviso);
            mudou.fetch_add(1, Ordering::Release);
        });
        let mut r = Self {
            abertos,
            tocadores,
            pelo_match: HashSet::new(),
            caidos: HashMap::new(),
            pares: Arc::new(Mutex::new(Pares::abrir(arquivo))),
            pasta,
            instalados: Vec::new(),
            desligados: desligados.iter().cloned().collect(),
            estado,
            versao,
            versao_vista: 0,
            trabalhadores: HashMap::new(),
            busca_pedida: None,
            geracao: 0,
            pedidas: HashSet::new(),
            busca: Busca::default(),
            provedores: Vec::new(),
            paginas: HashMap::new(),
            aviso: None,
            removido: None,
        };
        r.recarregar();
        r
    }

    /// A pasta dos provedores.
    pub fn pasta(&self) -> Option<&Path> {
        self.pasta.as_deref()
    }

    /// Há algum provedor ligado.
    pub fn tem_provedores(&self) -> bool {
        !self.trabalhadores.is_empty()
    }

    /// Os serviços desligados, em ordem (para guardar nos ajustes).
    pub fn desligados(&self) -> Vec<String> {
        let mut v: Vec<String> = self.desligados.iter().cloned().collect();
        v.sort();
        v
    }

    /// Lê de novo a pasta dos provedores: abre os novos e os religados, fecha os removidos e os desligados.
    pub fn recarregar(&mut self) {
        let da_pessoa = self.pasta.as_deref().map(provedores::instalados).unwrap_or_default();
        let ao_lado = std::env::current_exe().ok().and_then(|e| e.parent().map(embutidos_em)).unwrap_or_default();
        let (instalados, embutidos) = juntar(da_pessoa, ao_lado);
        let fica = |s: &String| instalados.iter().any(|i| &i.manifesto.servico == s);
        // Soltar o trabalhador encerra a thread dele, e o programa fecha.
        let desligados = &self.desligados;
        self.trabalhadores.retain(|s, _| fica(s) && !desligados.contains(s));
        {
            let mut e = travar(&self.estado);
            // Os pedidos sem resposta podem ter ido a um provedor que saiu: quem espera pede de novo.
            self.pedidas.retain(|k| e.fontes.contains_key(k));
            let antigos = std::mem::take(&mut e.provedores);
            e.provedores = instalados
                .iter()
                .map(|i| {
                    let s = &i.manifesto.servico;
                    let antigo = antigos.iter().find(|a| &a.servico == s);
                    let situacao = if self.desligados.contains(s) {
                        Situacao::Desligado
                    } else if self.trabalhadores.contains_key(s) {
                        antigo.map_or(Situacao::Abrindo, |a| a.situacao.clone())
                    } else {
                        Situacao::Abrindo
                    };
                    InfoDoProvedor {
                        nome: i.manifesto.nome.clone(),
                        servico: s.clone(),
                        versao: i.manifesto.versao.clone(),
                        capacidades: antigo.map(|a| a.capacidades.clone()).unwrap_or_default(),
                        situacao,
                        embutido: embutidos.contains(s),
                        cor: antigo.and_then(|a| a.cor),
                        logo: antigo.and_then(|a| a.logo.clone()),
                        conta: antigo.and_then(|a| a.conta.clone()),
                    }
                })
                .collect();
        }
        for i in &instalados {
            let s = &i.manifesto.servico;
            if !self.desligados.contains(s) && !self.trabalhadores.contains_key(s) {
                self.ligar_trabalhador(i.clone());
            }
        }
        self.instalados = instalados;
        self.versao.fetch_add(1, Ordering::Release);
    }

    fn ligar_trabalhador(&mut self, inst: Instalado) {
        let (envio, tarefas) = channel();
        let (e, v, servico) = (self.estado.clone(), self.versao.clone(), inst.manifesto.servico.clone());
        let (pares, abertos) = (self.pares.clone(), self.abertos.clone());
        // A pasta de dados do provedor fica ao lado da dos provedores: `<dados>/provedores-dados/<serviço>`.
        let dados = self.pasta.as_deref().and_then(Path::parent).map(|d| d.join("provedores-dados").join(&servico));
        let criada = std::thread::Builder::new()
            .name(format!("provedor-{servico}-tarefas"))
            .spawn(move || trabalhar(&inst, dados.as_deref(), &tarefas, &e, &v, &pares, &abertos));
        match criada {
            Ok(fim) => {
                self.trabalhadores.insert(servico, Trabalhador { tarefas: envio, fim });
            }
            Err(err) => log::warn!("provedor {servico} não abriu: {err}"),
        }
    }

    /// Liga ou desliga um provedor (desligado, o programa dele fecha e a busca não pergunta a ele).
    pub fn ligar(&mut self, servico: &str, ligado: bool) {
        if ligado {
            self.desligados.remove(servico);
        } else {
            self.desligados.insert(servico.to_string());
        }
        self.recarregar();
    }

    /// Encerra a thread do provedor e devolve como esperar o programa fechar.
    fn fechar(&mut self, servico: &str) -> Option<JoinHandle<()>> {
        self.trabalhadores.remove(servico).map(|t| {
            drop(t.tarefas);
            t.fim
        })
    }

    /// Roda `tarefa` em segundo plano depois de `esperar` (o programa do provedor fechar); o aviso e a releitura
    /// chegam por [`Remoto::quadro`].
    fn em_segundo_plano(
        &self,
        esperar: Option<JoinHandle<()>>,
        tarefa: impl FnOnce() -> (String, Option<String>) + Send + 'static,
    ) {
        let (estado, versao) = (self.estado.clone(), self.versao.clone());
        let criada = std::thread::Builder::new().name("provedores-pasta".into()).spawn(move || {
            if let Some(f) = esperar {
                let _ = f.join();
            }
            let (aviso, removido) = tarefa();
            let mut e = travar(&estado);
            e.avisos.push(aviso);
            e.removido = removido;
            e.recarregar = true;
            drop(e);
            versao.fetch_add(1, Ordering::Release);
        });
        if let Err(e) = criada {
            log::warn!("não deu para mexer na pasta dos provedores: {e}");
        }
    }

    /// Instala (ou atualiza) o provedor que está na pasta `origem`.
    pub fn instalar(&mut self, origem: PathBuf) {
        let Some(pasta) = self.pasta.clone() else {
            self.aviso = Some("Sem a pasta de dados do Player, não há onde instalar provedores".into());
            return;
        };
        let servico = match provedores::registro::ler(&origem) {
            Ok(i) => i.manifesto.servico,
            Err(e) => {
                self.aviso = Some(format!("Essa pasta não tem um provedor ({e})"));
                return;
            }
        };
        // Atualizando: o programa antigo fecha antes da troca.
        let esperar = self.fechar(&servico);
        self.em_segundo_plano(esperar, move || match provedores::instalar(&origem, &pasta) {
            Ok(i) => {
                let versao =
                    if i.manifesto.versao.is_empty() { String::new() } else { format!(" {}", i.manifesto.versao) };
                (format!("Provedor {}{versao} instalado", i.manifesto.nome), None)
            }
            Err(e) => (format!("Não deu para instalar o provedor: {e}"), None),
        });
    }

    /// Remove um provedor (vai para os removidos; "Desfazer" o traz de volta).
    pub fn remover(&mut self, servico: &str) {
        if self.provedores.iter().any(|p| p.servico == servico && p.embutido) {
            self.aviso = Some("Este provedor vem com o app: dá para desligar, mas não remover".into());
            return;
        }
        let (Some(pasta), Some(inst)) =
            (self.pasta.clone(), self.instalados.iter().find(|i| i.manifesto.servico == servico).cloned())
        else {
            return;
        };
        let esperar = self.fechar(servico);
        let servico = servico.to_string();
        self.em_segundo_plano(esperar, move || match provedores::remover(&inst, &pasta) {
            Ok(()) => (format!("Provedor {} removido", inst.manifesto.nome), Some(servico)),
            Err(e) => (format!("Não deu para remover o provedor: {e}"), None),
        });
    }

    /// Traz de volta o provedor removido por último deste serviço.
    pub fn restaurar(&mut self, servico: String) {
        let Some(pasta) = self.pasta.clone() else { return };
        self.em_segundo_plano(None, move || match provedores::restaurar(&servico, &pasta) {
            Ok(i) => (format!("Provedor {} de volta", i.manifesto.nome), None),
            Err(e) => (format!("Não deu para trazer o provedor de volta: {e}"), None),
        });
    }

    /// Busca em todos os provedores, depois de uma pausa curta na digitação. Texto vazio limpa na hora.
    pub fn buscar(&mut self, texto: &str) {
        let texto = texto.trim();
        if texto.is_empty() {
            self.busca_pedida = None;
            self.geracao += 1;
            let mut e = travar(&self.estado);
            e.busca = Busca { geracao: self.geracao, ..Default::default() };
            self.versao.fetch_add(1, Ordering::Release);
            return;
        }
        if self.busca_pedida.as_ref().is_some_and(|(t, _)| t == texto) {
            return;
        }
        // Voltou ao texto que já foi buscado (digitou e apagou na pausa): a busca pendente não vale mais.
        if self.busca.texto == texto {
            self.busca_pedida = None;
            return;
        }
        self.busca_pedida = Some((texto.to_string(), Instant::now()));
    }

    /// Pede o endereço de uma faixa (uma vez só; a resposta chega depois, por [`Remoto::endereco`]).
    pub fn resolver(&mut self, m: &Musica) {
        let Some(r) = &m.remota else { return };
        let chave = m.caminho.to_string_lossy().into_owned();
        let quem = self.quem_toca_esta(m);
        // Quem toca está abrindo: ainda não há a quem pedir (quem espera a faixa pede de novo).
        if quem == QuemToca::Abrindo || !self.pedidas.insert(chave.clone()) {
            return;
        }
        let falhou = match quem {
            QuemToca::Abrindo => None,
            QuemToca::Sozinho => Some("o tocador do serviço é quem toca esta faixa".to_string()),
            QuemToca::Ele => match self.trabalhadores.get(&r.servico) {
                Some(t) => {
                    let _ = t.tarefas.send(Tarefa::Resolver { id: r.id.clone(), chave: chave.clone() });
                    None
                }
                None => Some(format!("o provedor {} não está instalado ou está desligado", r.fonte)),
            },
            QuemToca::Outro(servico) => {
                self.casar(&servico, m, &chave, false);
                None
            }
            QuemToca::Ninguem if self.trabalhadores.contains_key(&r.servico) => Some(PRECISA_DE_OUTRO.to_string()),
            QuemToca::Ninguem => Some(format!("o provedor {} não está instalado ou está desligado", r.fonte)),
        };
        if let Some(motivo) = falhou {
            travar(&self.estado).fontes.insert(chave, FonteLida { endereco: Err(motivo), par: None, causa: None });
            self.versao.fetch_add(1, Ordering::Release);
        }
    }

    /// Quem toca as faixas de um serviço agora.
    pub fn quem_toca(&self, servico: &str) -> QuemToca {
        let cairam = self.caidos.get(servico).map(|(j, _)| j.clone()).unwrap_or_default();
        quem_toca(&travar(&self.estado).provedores, servico, self.tocadores.situacao(servico), &cairam)
    }

    /// Um endereço de faixa falhou: a cadeia decide o que fazer, e devolve `true` se esta faixa deve ser tentada de
    /// novo (o jeito de tocar mudou). Só as causas que derrubam o jeito trocam alguma coisa; o resto segue o caminho de
    /// sempre (a faixa é pulada com o motivo).
    pub fn avaliar_falha(&mut self, m: &Musica, motivo: &str) -> bool {
        let Some(r) = m.remota.clone() else { return false };
        let Some(causa) = self.endereco_causa(m) else { return false };
        // O jeito que falhou é o que a faixa usaria agora.
        let (agora, _) = self.jeito_de(&r.servico);
        let Some(jeito) = agora else { return false };
        let decisao = crate::cadeia::decidir(jeito, Some(causa), self.proximo_depois_de(&r.servico, jeito));
        match decisao {
            crate::cadeia::Decisao::Trocar(_) => {
                log::warn!("o áudio de {} caiu ({motivo}); trocando de jeito de tocar", r.servico);
                self.caiu(&r.servico, jeito, causa);
                self.esquecer(m);
                true
            }
            crate::cadeia::Decisao::Desistir => {
                self.caiu(&r.servico, jeito, causa);
                // Sem próximo jeito, a faixa não toca: o aviso já saiu em `caiu`.
                false
            }
            crate::cadeia::Decisao::SoEstaFaixa | crate::cadeia::Decisao::TentarDeNovo => false,
        }
    }

    /// O jeito de tocar agora para as faixas de um serviço, e o próximo se ele cair. `None` quando nenhum está pronto
    /// (a cadeia responde "desistir", e o Player diz o motivo).
    pub fn jeito_de(&self, servico: &str) -> (Option<Jeito>, Option<Jeito>) {
        let quem = self.quem_toca(servico);
        let agora = match quem {
            QuemToca::Ele => Some(Jeito::Proprio),
            QuemToca::Sozinho => Some(Jeito::Tocador),
            QuemToca::Outro(_) => Some(Jeito::Match),
            QuemToca::Abrindo | QuemToca::Ninguem => None,
        };
        let proximo = agora.and_then(|j| self.proximo_depois_de(servico, j));
        (agora, proximo)
    }

    /// Há outro serviço pronto para tocar as faixas deste (busca e áudio).
    fn tem_match(&self, servico: &str) -> bool {
        self.provedores.iter().any(|p| {
            p.servico != servico && p.situacao == Situacao::Pronto && tem(p, &[Capacidade::Busca, Capacidade::Audio])
        })
    }

    /// Um jeito de tocar caiu nesta abertura: as faixas do serviço vão para o próximo jeito, e a pessoa é avisada uma
    /// vez do motivo. A decisão vem de `crate::cadeia::decidir`: o passageiro não derruba o jeito, a faixa indisponível
    /// só muda aquela faixa, e o resto derruba **este** jeito (`jeito`, e não o que a tela diria agora).
    pub fn caiu(&mut self, servico: &str, jeito: Jeito, causa: CausaDoProvedor) {
        // O próximo é o que sobra da cadeia depois deste, sem os que já caíram.
        let proximo = self.proximo_depois_de(servico, jeito);
        let decisao = crate::cadeia::decidir(jeito, Some(causa), proximo);
        let nome = self.nome_do_servico(servico);
        if let Some(texto) = crate::cadeia::aviso(Some(causa), decisao, &nome) {
            log::warn!("{servico}: {texto}");
            travar(&self.estado).avisos.push(texto);
        }
        if matches!(decisao, crate::cadeia::Decisao::Trocar(_) | crate::cadeia::Decisao::Desistir) {
            let entrada = self.caidos.entry(servico.to_string()).or_insert_with(|| (HashSet::new(), causa));
            entrada.0.insert(jeito);
            if entrada.0.len() == 1 {
                entrada.1 = causa;
            }
            self.esquecer_enderecos(servico);
        }
        self.versao.fetch_add(1, Ordering::Release);
    }

    /// O jeito que vem depois de `jeito` na cadeia, se ele estiver pronto e não tiver caído.
    fn proximo_depois_de(&self, servico: &str, jeito: Jeito) -> Option<Jeito> {
        let caidos = self.caidos.get(servico).map(|(j, _)| j.clone()).unwrap_or_default();
        let pronto_para_tocar_sozinho =
            self.provedores.iter().any(|p| p.servico == servico && p.capacidades.contains(&Capacidade::TocaSozinho))
                && self.tocadores.situacao(servico) == SituacaoDoTocador::Pronto
                && !caidos.contains(&Jeito::Tocador);
        match jeito {
            // Do áudio do próprio serviço: o tocador do serviço (se pronto), senão o Match.
            Jeito::Proprio => {
                if pronto_para_tocar_sozinho {
                    Some(Jeito::Tocador)
                } else {
                    self.tem_match(servico).then_some(Jeito::Match)
                }
            }
            Jeito::Tocador => self.tem_match(servico).then_some(Jeito::Match),
            Jeito::Match => None,
        }
    }

    /// Por que caiu um jeito de tocar deste serviço nesta abertura, se algum caiu (a tela mostra o motivo).
    pub fn caiu_para(&self, servico: &str) -> Option<CausaDoProvedor> {
        self.caidos.get(servico).map(|(_, causa)| *causa)
    }

    /// Os jeitos de tocar que caíram nesta abertura (para a tela e os testes).
    pub fn caidos(&self, servico: &str) -> HashSet<Jeito> {
        self.caidos.get(servico).map(|(j, _)| j.clone()).unwrap_or_default()
    }

    /// Esquece os endereços de um serviço (o servidor de áudio pode ter trocado de porta, ou caído).
    fn esquecer_enderecos(&mut self, servico: &str) {
        let prefixo = format!("{servico}:");
        let mut e = travar(&self.estado);
        e.fontes.retain(|chave, _| !chave.starts_with(&prefixo));
        drop(e);
        self.pedidas.retain(|c| !c.starts_with(&prefixo));
    }

    /// Quem toca esta faixa agora: a que o tocador do serviço não tocou vai pelo Match.
    pub fn quem_toca_esta(&self, m: &Musica) -> QuemToca {
        let Some(r) = &m.remota else { return QuemToca::Ninguem };
        if self.pelo_match.contains(m.caminho.to_string_lossy().as_ref()) {
            return quem_toca(
                &travar(&self.estado).provedores,
                &r.servico,
                SituacaoDoTocador::Desligado,
                &self.caidos(&r.servico),
            );
        }
        self.quem_toca(&r.servico)
    }

    /// O tocador do serviço não tocou esta faixa: daqui para a frente ela vai pelo Match.
    pub fn mandar_pelo_match(&mut self, m: &Musica) {
        self.esquecer(m);
        self.pelo_match.insert(m.caminho.to_string_lossy().into_owned());
    }

    /// Como está o tocador do serviço (`None`: ele não tem um).
    pub fn tocador(&self, servico: &str) -> Option<Lido> {
        let tem = |p: &InfoDoProvedor| p.servico == servico && p.capacidades.contains(&Capacidade::TocaSozinho);
        self.provedores.iter().any(tem).then(|| self.tocadores.estado(servico)).flatten()
    }

    /// O estado do tocador depois da ordem `numero`, se já chegou.
    pub fn tocador_depois(&self, servico: &str, numero: u64) -> Option<Lido> {
        self.tocadores.estado_depois(servico, numero)
    }

    /// Manda no que toca no tocador do serviço; devolve o número da ordem.
    pub fn controlar(&mut self, servico: &str, acao: Acao) -> u64 {
        self.tocadores.mandar(servico, CorpoDoPedido::Controlar(acao))
    }

    /// Liga o tocador do serviço (com a chave que a pessoa copiou, ou com a guardada).
    pub fn ligar_tocador(&mut self, servico: &str, chave: Option<String>) {
        // Uma chance nova para as faixas que ele não tocou, e para um provedor que fechou.
        self.pelo_match.clear();
        if let Some(t) = self.trabalhadores.get(servico) {
            let _ = t.tarefas.send(Tarefa::Acordar);
        }
        self.tocadores.mandar(servico, CorpoDoPedido::LigarTocador { chave: chave.map(Segredo) });
    }

    pub fn desligar_tocador(&mut self, servico: &str) {
        self.tocadores.mandar(servico, CorpoDoPedido::DesligarTocador);
    }

    fn casar(&self, servico: &str, m: &Musica, chave: &str, recusar: bool) {
        let Some(alvo) = crate::paginas::faixa_da_musica(m).map(Box::new) else { return };
        if let Some(t) = self.trabalhadores.get(servico) {
            let _ = t.tarefas.send(Tarefa::Casar { alvo, chave: chave.to_string(), recusar });
        }
    }

    /// O endereço de uma faixa, se já chegou (ou por que não deu).
    pub fn endereco(&self, m: &Musica) -> Option<Result<tocador::Endereco, String>> {
        travar(&self.estado).fontes.get(m.caminho.to_string_lossy().as_ref()).map(|f| f.endereco.clone())
    }

    /// A causa da falha do endereço desta faixa, quando o provedor soube classificar (a cadeia de reserva a usa).
    pub fn endereco_causa(&self, m: &Musica) -> Option<CausaDoProvedor> {
        travar(&self.estado).fontes.get(m.caminho.to_string_lossy().as_ref()).and_then(|f| f.causa)
    }

    /// De que áudio é a pré-análise desta faixa: a gravação (pelo Match, a do par) e o endereço. `tocando`: é a faixa que
    /// o tocador daqui está tocando, e vale o que ele abriu, mesmo que agora outro a tocasse.
    pub fn alvo_da_analise(&self, m: &Musica, tocando: bool) -> (Option<String>, Option<tocador::Endereco>) {
        let quem = self.quem_toca_esta(m);
        let chave = m.caminho.to_string_lossy();
        let lida = travar(&self.estado)
            .fontes
            .get(chave.as_ref())
            .map(|f| (f.par.as_ref().map(Par::chave), f.endereco.clone().ok()));
        match (lida, quem) {
            // O tocador do serviço faz o som: nenhuma gravação daqui é a dele.
            (_, QuemToca::Sozinho) if !tocando => (None, None),
            (Some(lida), _) => lida,
            (None, QuemToca::Outro(_)) => (self.par(m).map(|p| p.chave()), None),
            (None, _) => (None, None),
        }
    }

    /// O par de uma faixa que toca pelo Match (`None` se o próprio serviço a toca, ou se o par ainda não foi achado): o
    /// que toca agora ou, antes disso, o guardado.
    pub fn par(&self, m: &Musica) -> Option<Par> {
        m.remota.as_ref()?;
        let QuemToca::Outro(servico) = self.quem_toca_esta(m) else { return None };
        let chave = m.caminho.to_string_lossy();
        let toca = travar(&self.estado).fontes.get(chave.as_ref()).and_then(|f| f.par.clone());
        if toca.is_some() {
            return toca;
        }
        travar(&self.pares).de(m.caminho.to_string_lossy().as_ref()).filter(|p| p.servico == servico).cloned()
    }

    /// Recusa o par de agora e procura outro.
    pub fn outro_par(&mut self, m: &Musica) {
        if let QuemToca::Outro(servico) = self.quem_toca_esta(m) {
            self.esquecer(m);
            let chave = m.caminho.to_string_lossy().into_owned();
            self.pedidas.insert(chave.clone());
            self.casar(&servico, m, &chave, true);
        }
    }

    /// Esquece o endereço de uma faixa (ele venceu, ou não tocou): a próxima vez, ela é resolvida de novo.
    pub fn esquecer(&mut self, m: &Musica) {
        let chave = m.caminho.to_string_lossy().into_owned();
        self.pedidas.remove(&chave);
        travar(&self.estado).fontes.remove(&chave);
    }

    /// A cada quadro: manda a busca que esperava a pausa, copia o que chegou dos provedores e relê a pasta quando
    /// uma instalação ou remoção termina.
    pub fn quadro(&mut self) {
        if let Some((texto, desde)) = &self.busca_pedida
            && desde.elapsed() >= PAUSA_NA_DIGITACAO
        {
            let texto = texto.clone();
            self.busca_pedida = None;
            self.geracao += 1;
            let mut e = travar(&self.estado);
            e.busca = Busca {
                texto: texto.clone(),
                geracao: self.geracao,
                faltam: self.trabalhadores.len(),
                ..Default::default()
            };
            drop(e);
            for t in self.trabalhadores.values() {
                let _ = t.tarefas.send(Tarefa::Buscar { texto: texto.clone(), geracao: self.geracao });
            }
            self.versao.fetch_add(1, Ordering::Release);
        }
        let v = self.versao.load(Ordering::Acquire);
        if v != self.versao_vista {
            self.versao_vista = v;
            let mut e = travar(&self.estado);
            let recarregar = std::mem::take(&mut e.recarregar);
            if let Some(aviso) = e.avisos.pop() {
                e.avisos.clear();
                self.aviso = Some(aviso);
                self.removido = e.removido.take();
            }
            drop(e);
            if recarregar {
                self.recarregar();
            }
            let e = travar(&self.estado);
            self.busca = e.busca.clone();
            self.provedores = e.provedores.clone();
            self.paginas = e.paginas.iter().map(|(k, (_, p))| (k.clone(), Arc::clone(p))).collect();
        }
    }
}

impl Remoto {
    /// Pede uma página de um serviço. Uma guardada há menos de [`VALIDADE_DA_PAGINA`] (pronta, carregando ou com
    /// erro) não vai ao provedor de novo: chamar a cada quadro não custa nada, e o erro fica até
    /// [`Remoto::de_novo`]. A resposta chega depois, por [`Remoto::pagina`].
    pub fn abrir_pagina(&mut self, pedido: PedidoDePagina) {
        let agora = Instant::now();
        let mut e = travar(&self.estado);
        let guardada = e.paginas.get(&pedido).cloned();
        if guardada.as_ref().is_some_and(|(desde, _)| !vencida(*desde, agora)) {
            return;
        }
        // Uma página pronta que venceu fica na tela enquanto a nova não chega (não volta a "carregando").
        if let Some((_, pagina)) =
            guardada.filter(|(_, p)| matches!(**p, PaginaRemota::Prateleiras(_) | PaginaRemota::Itens(_)))
        {
            if let Some(t) = self.trabalhadores.get(pedido.servico()) {
                let _ = t.tarefas.send(Tarefa::Pagina(pedido.clone()));
            }
            e.paginas.insert(pedido, (agora, pagina));
            return;
        }
        // O cache não cresce sem fim: as vencidas saem quando uma página nova é pedida.
        e.paginas.retain(|_, (desde, _)| !vencida(*desde, agora));
        let pagina = match self.trabalhadores.get(pedido.servico()) {
            Some(t) if t.tarefas.send(Tarefa::Pagina(pedido.clone())).is_ok() => PaginaRemota::Carregando,
            _ => PaginaRemota::Erro(format!(
                "o provedor {} não está instalado ou está desligado",
                self.nome_do_servico(pedido.servico())
            )),
        };
        e.paginas.insert(pedido, (agora, Arc::new(pagina)));
        drop(e);
        self.versao.fetch_add(1, Ordering::Release);
    }

    /// Começa o login no serviço (o provedor abre o navegador); a conta muda em [`InfoDoProvedor::conta`].
    pub fn entrar(&self, servico: &str) {
        if let Some(t) = self.trabalhadores.get(servico) {
            let _ = t.tarefas.send(Tarefa::Entrar);
        }
    }

    /// Sai da conta do serviço.
    pub fn sair(&self, servico: &str) {
        if let Some(t) = self.trabalhadores.get(servico) {
            let _ = t.tarefas.send(Tarefa::Sair);
        }
    }

    /// Esquece a página guardada e pede de novo ("Tentar de novo").
    pub fn de_novo(&mut self, pedido: &PedidoDePagina) {
        travar(&self.estado).paginas.remove(pedido);
        self.abrir_pagina(pedido.clone());
    }

    /// O que se sabe da página (a cópia feita no último [`Remoto::quadro`]).
    pub fn pagina(&self, pedido: &PedidoDePagina) -> Option<Arc<PaginaRemota>> {
        self.paginas.get(pedido).cloned()
    }

    fn nome_do_servico(&self, servico: &str) -> String {
        self.provedores.iter().find(|i| i.servico == servico).map_or_else(|| servico.to_string(), |i| i.nome.clone())
    }
}

/// O que publicar e se continua perguntando, depois de uma resposta da conta. Passado o prazo (`venceu`), a tela sai
/// de `entrando` de qualquer jeito: nela não há o que clicar.
fn depois_de_perguntar(resposta: Result<Conta, String>, venceu: bool, nome: &str) -> (Option<Conta>, bool) {
    match resposta {
        Ok(c) if c.situacao != SituacaoDaConta::Entrando => (Some(c), false),
        _ if venceu => (Some(fora_com(format!("O login do {nome} demorou demais. Tente de novo."))), false),
        Ok(c) => (Some(c), true),
        Err(_) => (None, true),
    }
}

/// Uma faixa de um serviço sem áudio, e nenhum outro para tocá-la.
pub const PRECISA_DE_OUTRO: &str = "ligue o YouTube Music na Loja para tocar as músicas deste serviço";

/// O endereço da mesma música no serviço de `p` (o Match) e, se for o caso, um aviso para a pessoa.
fn casar(
    p: &Provedor,
    nome: &str,
    alvo: &Faixa,
    chave: &str,
    recusar: bool,
    pares: &Mutex<Pares>,
) -> (FonteLida, Option<String>) {
    let tocar = |par: Par| {
        let endereco = p.resolver(&par.id, PRAZO).map_err(|e| motivo(&e)).and_then(endereco_de);
        FonteLida { par: endereco.is_ok().then_some(par), endereco, causa: None }
    };
    let sem = |motivo: String| FonteLida { endereco: Err(motivo), par: None, causa: None };
    let guardado = travar(pares).de(chave).filter(|g| g.servico == p.apresentacao().servico).cloned();
    let mut recusados = guardado.as_ref().map(|g| g.recusados.clone()).unwrap_or_default();
    let mut fora = recusados.clone();
    let mut erro = None;
    if let Some(g) = &guardado {
        if recusar {
            recusados.push(g.id.clone());
        } else {
            match tocar(g.clone()) {
                lida @ FonteLida { endereco: Ok(_), .. } => return (lida, None),
                FonteLida { endereco: Err(e), .. } => erro = Some(e),
            }
        }
        fora.push(g.id.clone());
    }
    let itens = match p.buscar(&consulta(alvo), RESULTADOS, PRAZO) {
        Ok(itens) => itens,
        Err(e) => return (sem(erro.unwrap_or_else(|| motivo(&e))), None),
    };
    match (escolher(alvo, &itens, &fora), guardado) {
        (Some(par), _) => {
            let lida = tocar(Par { recusados, ..par });
            // A guardada que não tocou agora (a rede, um limite) volta na próxima vez: a de agora é só desta.
            if erro.is_none()
                && let Some(par) = &lida.par
            {
                let a_gravar = travar(pares).guardar(chave, par.clone());
                if let Some(a_gravar) = a_gravar {
                    a_gravar.gravar();
                }
            }
            (lida, None)
        }
        (None, Some(g)) if recusar => (tocar(g), Some(format!("Não achei outra gravação no {nome}: fica a de antes"))),
        (None, _) => (sem(erro.unwrap_or_else(|| format!("não achei esta música no {nome}"))), None),
    }
}

/// Fora da conta, com o motivo.
fn fora_com(erro: String) -> Conta {
    Conta { erro: Some(erro), ..Conta::default() }
}

/// A thread de um provedor: abre o programa e atende as tarefas em ordem, até as tarefas acabarem (o provedor foi
/// desligado ou removido; o programa fecha junto). Se o programa fecha sozinho, é reaberto (até [`REABERTURAS`]
/// vezes seguidas).
fn trabalhar(
    inst: &Instalado,
    dados: Option<&Path>,
    tarefas: &Receiver<Tarefa>,
    estado: &Mutex<Estado>,
    versao: &AtomicU64,
    pares: &Mutex<Pares>,
    abertos: &Abertos,
) {
    let servico = inst.manifesto.servico.clone();
    let ambiente: Vec<(String, std::ffi::OsString)> =
        dados.map(|d| ("TCCPLAYER_PROVEDOR_DADOS".to_string(), d.as_os_str().to_owned())).into_iter().collect();
    let abrir = || Provedor::iniciar_com(&inst.executavel(), &inst.manifesto.argumentos, &inst.pasta, &ambiente);
    let situar = |s: Situacao, aberto: Option<&Provedor>| {
        // A logo vai ao disco antes de travar o estado: a interface o lê a cada quadro e não pode esperar o disco.
        let logo = aberto.and_then(|p| dados.and_then(|d| guardar_logo(d, p.apresentacao().identidade.as_ref())));
        if let Some(info) = travar(estado).provedores.iter_mut().find(|i| i.servico == servico) {
            info.situacao = s;
            if let Some(p) = aberto {
                info.capacidades = p.apresentacao().capacidades.clone();
                info.cor = p.apresentacao().identidade.as_ref().and_then(Identidade::cor_rgb);
                info.logo = logo;
                info.conta = p
                    .apresentacao()
                    .sabe(Capacidade::Conta)
                    .then(|| p.apresentacao().conta.clone().unwrap_or_default());
                if !p.apresentacao().versao.is_empty() {
                    info.versao = p.apresentacao().versao.clone();
                }
            }
        }
        versao.fetch_add(1, Ordering::Release);
    };
    // Quem acompanha o tocador do serviço usa o mesmo programa, sem o segurar aberto.
    let abertura = |p: &Arc<Provedor>| {
        travar(abertos).insert(servico.clone(), Arc::downgrade(p));
    };
    let mut provedor = match abrir() {
        Ok(p) => {
            if p.servico() != servico {
                log::warn!("o provedor de {servico} se apresentou como {} (vale o do provedor.toml)", p.servico());
            }
            let p = Arc::new(p);
            abertura(&p);
            situar(Situacao::Pronto, Some(&p));
            Some(p)
        }
        Err(e) => {
            // (A conta, se o provedor tiver uma, chegou na apresentação.)
            log::warn!("o provedor {} não abriu: {e}", inst.manifesto.nome);
            situar(Situacao::Falhou(motivo(&e)), None);
            None
        }
    };
    let nome = inst.manifesto.nome.clone();
    let mut reaberturas = 0;
    // Enquanto a conta está entrando (o navegador aberto, ou a credencial guardada renovando), a thread pergunta a
    // conta a cada [`PERGUNTA_DA_CONTA`], entre uma tarefa e outra, até [`ACOMPANHAR_A_CONTA`].
    let ate_quando = |p: Option<&Provedor>| {
        let a = p?.apresentacao();
        let entrando = a.conta.as_ref().is_some_and(|c| c.situacao == SituacaoDaConta::Entrando);
        (a.sabe(Capacidade::Conta) && entrando).then(|| Instant::now() + ACOMPANHAR_A_CONTA)
    };
    let mut acompanhar_ate = ate_quando(provedor.as_deref());
    let mut perguntou = Instant::now();
    // Depois de um erro, a conta pode ter mudado no serviço: quantas vezes ainda perguntar (um provedor ocupado não
    // responde na hora).
    let mut duvidas = 0;
    let publicar_conta = |conta: Conta| {
        let mut e = travar(estado);
        if let Some(info) = e.provedores.iter_mut().find(|i| i.servico == servico) {
            let mudou = info.conta.as_ref().map(|c| c.situacao) != Some(conta.situacao);
            info.conta = Some(conta);
            if mudou {
                // O que o serviço mostra depende da conta: as páginas de antes (um erro, por exemplo) não valem mais.
                e.paginas.retain(|p, _| p.servico() != servico);
            }
        }
        drop(e);
        versao.fetch_add(1, Ordering::Release);
    };
    loop {
        let esperar = acompanhar_ate.is_some() || duvidas > 0;
        let tarefa = match esperar {
            true => match tarefas.recv_timeout(PERGUNTA_DA_CONTA) {
                Ok(t) => Some(t),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            },
            false => match tarefas.recv() {
                Ok(t) => Some(t),
                Err(_) => break,
            },
        };
        if matches!(tarefa, Some(Tarefa::Entrar | Tarefa::Acordar)) {
            // A pessoa pediu: vale tentar abrir de novo um provedor que desistiu de reabrir.
            reaberturas = 0;
        }
        if provedor.as_ref().is_some_and(|p| !p.vivo()) {
            // (Também sem tarefa nenhuma: um provedor que fecha no meio do login não pode deixar a tela esperando.)
            let no_login = acompanhar_ate.take().is_some();
            if reaberturas < REABERTURAS {
                reaberturas += 1;
                log::info!("o provedor {nome} fechou; abrindo de novo ({reaberturas}/{REABERTURAS})");
                provedor = abrir().ok().map(Arc::new);
                match &provedor {
                    Some(p) => {
                        abertura(p);
                        situar(Situacao::Pronto, Some(p));
                    }
                    None => situar(Situacao::Falhou("fechou".into()), None),
                }
                acompanhar_ate = ate_quando(provedor.as_deref());
                perguntou = Instant::now();
            }
            let sem_motivo =
                provedor.as_ref().is_none_or(|p| p.apresentacao().conta.as_ref().is_none_or(|c| c.erro.is_none()));
            if no_login && acompanhar_ate.is_none() && sem_motivo {
                publicar_conta(fora_com(format!("O {nome} fechou durante o login. Tente de novo.")));
            }
        }
        if let Some(ate) = acompanhar_ate
            && perguntou.elapsed() >= PERGUNTA_DA_CONTA
            && let Some(p) = provedor.as_ref().filter(|p| p.vivo())
        {
            perguntou = Instant::now();
            let resposta = p.conta(PRAZO_DA_CONTA).map_err(|e| motivo(&e));
            // (Se ele fechou ao responder, a volta seguinte do laço o reabre.)
            let (conta, continua) = depois_de_perguntar(resposta, Instant::now() > ate, &nome);
            if let Some(conta) = conta {
                publicar_conta(conta);
            }
            if !continua {
                acompanhar_ate = None;
            }
        }
        if duvidas > 0
            && acompanhar_ate.is_none()
            && perguntou.elapsed() >= PERGUNTA_DA_CONTA
            && let Some(p) = provedor.as_ref().filter(|p| p.vivo())
        {
            perguntou = Instant::now();
            duvidas -= 1;
            if let Ok(conta) = p.conta(PRAZO_DA_CONTA) {
                duvidas = 0;
                if conta.situacao == SituacaoDaConta::Entrando {
                    acompanhar_ate = Some(Instant::now() + ACOMPANHAR_A_CONTA);
                }
                publicar_conta(conta);
            }
        }
        let Some(tarefa) = tarefa else { continue };
        let vivo = provedor.as_ref().filter(|p| p.vivo());
        match tarefa {
            Tarefa::Buscar { texto, geracao } => {
                // Uma busca que já foi trocada por outra (a pessoa continuou digitando) nem vai ao provedor.
                if travar(estado).busca.geracao != geracao {
                    continue;
                }
                let resultado = match vivo {
                    Some(p) if p.apresentacao().sabe(Capacidade::Busca) => p.buscar(&texto, RESULTADOS, PRAZO),
                    Some(_) => Ok(Vec::new()),
                    None => Err(nucleo::Erro::Provedor("não está aberto".into())),
                };
                if resultado.is_err() && vivo.is_some_and(|p| p.apresentacao().sabe(Capacidade::Conta)) {
                    (duvidas, perguntou) = (DUVIDAS_DA_CONTA, Instant::now() - PERGUNTA_DA_CONTA);
                }
                let mut e = travar(estado);
                if e.busca.geracao == geracao {
                    e.busca.faltam = e.busca.faltam.saturating_sub(1);
                    e.busca.respondidos.push(inst.manifesto.servico.clone());
                    match resultado {
                        Ok(itens) => {
                            e.busca.itens.extend(itens);
                            reaberturas = 0;
                        }
                        Err(err) => e.busca.erros.push(format!("{nome}: {}", motivo(&err))),
                    }
                }
                drop(e);
                versao.fetch_add(1, Ordering::Release);
            }
            Tarefa::Pagina(pedido) => {
                let resultado = match vivo {
                    Some(p) if !p.apresentacao().sabe(pedido.capacidade()) => {
                        PaginaRemota::Erro(format!("o {nome} não tem esta página"))
                    }
                    Some(p) => {
                        let resposta = match &pedido {
                            PedidoDePagina::Inicio { .. } => p.inicio(PRAZO).map(PaginaRemota::Prateleiras),
                            PedidoDePagina::Biblioteca { .. } => {
                                p.itens(CorpoDoPedido::Biblioteca, PRAZO).map(PaginaRemota::Itens)
                            }
                            PedidoDePagina::Album { id, .. } => {
                                p.itens(CorpoDoPedido::Album { album: id.clone() }, PRAZO).map(PaginaRemota::Itens)
                            }
                            PedidoDePagina::Artista { id, .. } => {
                                p.itens(CorpoDoPedido::Artista { artista: id.clone() }, PRAZO).map(PaginaRemota::Itens)
                            }
                            PedidoDePagina::Playlist { id, .. } => p
                                .itens(CorpoDoPedido::Playlist { playlist: id.clone() }, PRAZO)
                                .map(PaginaRemota::Itens),
                        };
                        resposta.unwrap_or_else(|e| PaginaRemota::Erro(format!("{nome}: {}", motivo(&e))))
                    }
                    None => PaginaRemota::Erro(format!("o provedor {nome} não está aberto")),
                };
                let errou = matches!(resultado, PaginaRemota::Erro(_));
                let mut e = travar(estado);
                // Um erro ao renovar não apaga a página que já estava na tela.
                let tinha = e
                    .paginas
                    .get(&pedido)
                    .is_some_and(|(_, p)| matches!(**p, PaginaRemota::Prateleiras(_) | PaginaRemota::Itens(_)));
                if matches!(resultado, PaginaRemota::Erro(_)) && tinha {
                    log::info!("{nome}: a página não renovou; fica a de antes");
                } else {
                    e.paginas.insert(pedido, (Instant::now(), Arc::new(resultado)));
                }
                drop(e);
                versao.fetch_add(1, Ordering::Release);
                // O serviço pode ter derrubado o login no meio do pedido: a tela precisa mostrar o convite de novo.
                if errou && vivo.is_some_and(|p| p.apresentacao().sabe(Capacidade::Conta)) {
                    (duvidas, perguntou) = (DUVIDAS_DA_CONTA, Instant::now() - PERGUNTA_DA_CONTA);
                }
            }
            Tarefa::Entrar => {
                let conta = match vivo {
                    Some(p) if p.apresentacao().sabe(Capacidade::Conta) => match p.entrar(PRAZO) {
                        Ok(()) => {
                            acompanhar_ate = Some(Instant::now() + ACOMPANHAR_A_CONTA);
                            perguntou = Instant::now();
                            // O provedor diz se abriu o navegador ou se está renovando a credencial guardada.
                            let entrando = Conta { situacao: SituacaoDaConta::Entrando, ..Conta::default() };
                            p.conta(PRAZO_DA_CONTA).unwrap_or(entrando)
                        }
                        Err(e) => fora_com(motivo(&e)),
                    },
                    Some(_) => continue,
                    None => fora_com(format!("O provedor {nome} não abriu. Desligue e ligue ele na Loja.")),
                };
                if conta.situacao != SituacaoDaConta::Entrando {
                    acompanhar_ate = None;
                }
                publicar_conta(conta);
            }
            Tarefa::Sair => {
                if let Some(p) = vivo.filter(|p| p.apresentacao().sabe(Capacidade::Conta)) {
                    if let Err(e) = p.sair(PRAZO) {
                        log::warn!("{nome}: sair da conta falhou: {}", motivo(&e));
                    }
                    acompanhar_ate = None;
                    publicar_conta(Conta::default());
                }
            }
            Tarefa::Acordar => {}
            Tarefa::Casar { alvo, chave, recusar } => {
                let (lida, aviso) = match vivo {
                    Some(p) => casar(p, &nome, &alvo, &chave, recusar, pares),
                    None => {
                        let endereco = Err(format!("o provedor {nome} não está aberto"));
                        (FonteLida { endereco, par: None, causa: None }, None)
                    }
                };
                if let Err(e) = &lida.endereco {
                    log::warn!("{nome} não achou {chave}: {e}");
                }
                let mut e = travar(estado);
                e.fontes.insert(chave, lida);
                e.avisos.extend(aviso);
                drop(e);
                versao.fetch_add(1, Ordering::Release);
            }
            Tarefa::Resolver { id, chave } => {
                let (resultado, causa) = match vivo {
                    Some(p) => match p.resolver(&id, PRAZO) {
                        Ok(fonte) => (endereco_de(fonte), None),
                        Err(e) => (Err(motivo(&e)), e.causa_do_provedor()),
                    },
                    None => (Err(format!("o provedor {nome} não está aberto")), None),
                };
                if let Err(e) = &resultado {
                    log::warn!("{nome} não deu o endereço de {chave}: {e}");
                }
                travar(estado).fontes.insert(chave, FonteLida { endereco: resultado, par: None, causa });
                versao.fetch_add(1, Ordering::Release);
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn faixa_vira_musica_com_a_chave_no_caminho() {
        let f = Faixa {
            servico: "exemplo".into(),
            id: "42".into(),
            isrc: Some("br-x12-26-00001".into()),
            titulo: "Nome".into(),
            artistas: vec!["A".into(), "B".into()],
            duracao_s: Some(f64::NAN),
            capa: Some("https://capa".into()),
            ..Default::default()
        };
        let m = musica_de(&f, "Exemplo");
        assert_eq!(m.caminho, PathBuf::from("exemplo:42"));
        assert_eq!((m.titulo.as_str(), m.artista.as_str(), m.duracao_s), ("Nome", "A, B", 0.0));
        let r = m.remota.as_ref().map(|r| (r.servico.as_str(), r.id.as_str(), r.isrc.as_deref(), r.fonte.as_str()));
        assert_eq!(r, Some(("exemplo", "42", Some("BRX122600001"), "Exemplo")));
        assert!(m.e_remota());
    }

    #[test]
    fn voltar_ao_texto_ja_buscado_cancela_a_busca_pendente() {
        let mut r = Remoto::iniciar(None, &[]);
        r.busca.texto = "abc".into();
        r.buscar("abcd");
        assert!(r.busca_pedida.is_some());
        r.buscar("abc");
        assert!(r.busca_pedida.is_none(), "senão \"abcd\" sai depois da pausa e a tela fica esperando para sempre");
    }

    #[test]
    fn sem_provedores_nada_quebra() {
        let mut r = Remoto::iniciar(None, &[]);
        assert!(!r.tem_provedores());
        r.buscar("algo");
        std::thread::sleep(PAUSA_NA_DIGITACAO + Duration::from_millis(20));
        r.quadro();
        assert_eq!(r.busca.faltam, 0);
        assert!(r.busca.itens.is_empty());
        let m = musica_de(&Faixa { servico: "x".into(), id: "1".into(), ..Default::default() }, "X");
        r.resolver(&m);
        assert!(matches!(r.endereco(&m), Some(Err(_))), "sem o provedor, o erro chega na hora");
        r.buscar("");
        r.quadro();
        assert!(r.busca.texto.is_empty());
        r.instalar(PathBuf::from("C:/nao/existe"));
        assert!(r.aviso.take().is_some(), "sem pasta de dados, avisa");
        r.remover("x");
        r.ligar("x", false);
        assert_eq!(r.desligados(), vec!["x".to_string()]);
    }

    #[test]
    fn embutido_entra_sem_instalar_e_o_instalado_vale_mais() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-embutidos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        assert!(embutidos_em(&pasta).is_empty(), "sem o programa ao lado, nada");
        std::fs::write(pasta.join(EMBUTIDOS[0].2), b"programa").expect("exe");
        let embutidos = embutidos_em(&pasta);
        assert_eq!(embutidos.len(), 1);
        assert_eq!(embutidos[0].executavel(), pasta.join("provedor-youtube-music.exe"));
        let (todos, deles) = juntar(Vec::new(), embutidos.clone());
        assert_eq!(todos.len(), 1);
        assert!(deles.contains("youtube-music"));
        // Um instalado do mesmo serviço (uma versão nova, por exemplo) fica no lugar do embutido.
        let instalado = Instalado { pasta: pasta.join("outro"), ..embutidos[0].clone() };
        let (todos, deles) = juntar(vec![instalado.clone()], embutidos);
        assert_eq!(todos, vec![instalado]);
        assert!(deles.is_empty());
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn embutidos_com_o_spotify() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-embutidos-spotify-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        std::fs::write(pasta.join("provedor-youtube-music.exe"), b"programa").expect("exe");
        std::fs::write(pasta.join("provedor-spotify.exe"), b"programa").expect("exe");
        let mut servicos: Vec<String> = embutidos_em(&pasta).into_iter().map(|i| i.manifesto.servico).collect();
        servicos.sort();
        assert_eq!(servicos, ["spotify", "youtube-music"]);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    /// Um [`Remoto`] com o provedor-exemplo (`cargo test --workspace` o compila ao lado dos testes) instalado numa
    /// pasta só deste teste, com estes argumentos. `None` (com um aviso) se ele não foi compilado.
    fn com_o_exemplo(teste: &str, argumentos: &str) -> Option<(Remoto, PathBuf)> {
        let exe = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent()?.parent().map(|p| p.join("provedor-exemplo.exe")))
            .filter(|e| e.is_file());
        let Some(exe) = exe else {
            eprintln!("{teste}: sem o provedor-exemplo compilado (rode cargo test --workspace)");
            return None;
        };
        let raiz = std::env::temp_dir().join(format!("tccplayer-{teste}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let pasta = raiz.join("dados").join("provedores");
        let p = pasta.join("exemplo");
        std::fs::create_dir_all(&p).expect("pasta");
        // O manifesto só aceita o programa dentro da pasta do provedor.
        std::fs::copy(&exe, p.join("provedor-exemplo.exe")).expect("copiar o exemplo");
        let manifesto = format!(
            "nome = \"Exemplo\"\nservico = \"exemplo\"\nexecutavel = \"provedor-exemplo.exe\"\nargumentos = [{argumentos}]"
        );
        std::fs::write(p.join("provedor.toml"), manifesto).expect("manifesto");
        Some((Remoto::iniciar(Some(pasta), &[]), raiz))
    }

    fn conta_de(r: &Remoto) -> Conta {
        r.provedores.first().and_then(|i| i.conta.clone()).unwrap_or_default()
    }

    /// Pronto e com a conta que veio na apresentação.
    fn abriu_com_conta(r: &mut Remoto) -> bool {
        esperar(r, |r| r.provedores.first().is_some_and(|i| i.conta.is_some()))
    }

    #[test]
    fn entrar_ate_ficar_dentro() {
        let Some((mut r, raiz)) = com_o_exemplo("conta", "\"--conta\"") else { return };
        assert!(abriu_com_conta(&mut r), "a conta chega com a apresentação: {:?}", r.provedores);
        assert_eq!(conta_de(&r).situacao, SituacaoDaConta::Fora);
        // Uma página de antes de entrar (um erro, por exemplo) não fica para depois.
        let pedido = PedidoDePagina::Album { servico: "exemplo".into(), id: "x".into() };
        let velha = (Instant::now(), Arc::new(PaginaRemota::Erro("velho".into())));
        travar(&r.estado).paginas.insert(pedido.clone(), velha);
        r.entrar("exemplo");
        assert!(esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Dentro), "{:?}", r.provedores);
        assert_eq!(conta_de(&r).nome.as_deref(), Some("exemplo"));
        assert!(!travar(&r.estado).paginas.contains_key(&pedido), "a conta mudou: as páginas são pedidas de novo");
        r.sair("exemplo");
        assert!(esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Fora));
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn provedor_que_cai_no_login_volta_para_fora_com_o_motivo() {
        let Some((mut r, raiz)) = com_o_exemplo("conta-cai", "\"--conta\", \"--cai\"") else { return };
        assert!(abriu_com_conta(&mut r));
        r.entrar("exemplo");
        // Sem mais nenhuma tarefa: a tela não pode ficar esperando um provedor que fechou.
        assert!(
            esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Fora && conta_de(r).erro.is_some()),
            "{:?}",
            conta_de(&r)
        );
        r.entrar("exemplo");
        assert!(esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Dentro), "dá para tentar de novo");
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn provedor_reaberto_ainda_entrando_continua_acompanhado() {
        let Some((mut r, raiz)) = com_o_exemplo("conta-volta", "\"--conta\", \"--cai\", \"--volta\"") else { return };
        assert!(abriu_com_conta(&mut r));
        r.entrar("exemplo");
        assert!(esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Dentro), "{:?}", conta_de(&r));
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn a_biblioteca_da_conta_vem_como_pagina() {
        let Some((mut r, raiz)) = com_o_exemplo("biblioteca", "\"--conta\"") else { return };
        assert!(abriu_com_conta(&mut r));
        let pedido = PedidoDePagina::Biblioteca { servico: "exemplo".into() };
        r.abrir_pagina(pedido.clone());
        assert!(esperar(
            &mut r,
            |r| matches!(r.pagina(&pedido).as_deref(), Some(PaginaRemota::Itens(i)) if i.len() == 1)
        ));
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn erro_de_pagina_atualiza_a_conta() {
        let Some((mut r, raiz)) = com_o_exemplo("revogada", "\"--conta\", \"--revoga\"") else { return };
        assert!(abriu_com_conta(&mut r));
        r.entrar("exemplo");
        assert!(esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Dentro));
        r.abrir_pagina(PedidoDePagina::Biblioteca { servico: "exemplo".into() });
        assert!(esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Fora), "a tela mostra o convite de novo");
        assert_eq!(conta_de(&r).erro.as_deref(), Some("o login foi revogado"));
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn erro_de_busca_atualiza_a_conta() {
        let Some((mut r, raiz)) = com_o_exemplo("revogada-busca", "\"--conta\", \"--revoga\"") else { return };
        assert!(abriu_com_conta(&mut r));
        r.entrar("exemplo");
        assert!(esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Dentro));
        r.buscar("qualquer");
        assert!(esperar(&mut r, |r| conta_de(r).situacao == SituacaoDaConta::Fora), "o convite volta");
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    fn do_spotify(titulo: &str) -> Musica {
        let f = Faixa {
            servico: "spotify".into(),
            id: "s1".into(),
            titulo: titulo.into(),
            artistas: vec!["Artista Um".into()],
            duracao_s: Some(4.0),
            ..Default::default()
        };
        musica_de(&f, "Spotify")
    }

    #[test]
    fn faixa_sem_audio_toca_pelo_par_de_outro_servico() {
        let Some((mut r, raiz)) = com_o_exemplo("match", "\"--url\", \"http://127.0.0.1:9\"") else { return };
        assert!(esperar(&mut r, |r| r.provedores.first().is_some_and(|i| i.situacao == Situacao::Pronto)));
        let m = do_spotify("Primeira Faixa");
        r.resolver(&m);
        assert!(esperar(&mut r, |r| r.endereco(&m).is_some()));
        let endereco = r.endereco(&m).expect("resposta").expect("endereço");
        assert_eq!(endereco.url, "http://127.0.0.1:9/1", "o áudio da mesma música no outro serviço");
        assert_eq!(r.par(&m).map(|p| p.chave()).as_deref(), Some("exemplo:1"));
        assert!(raiz.join("dados").join("pares.json").is_file(), "o par fica guardado");
        // Recusado o único que serve: não inventa outra música, e a de antes continua.
        r.outro_par(&m);
        assert!(r.endereco(&m).is_none(), "procurando");
        assert!(esperar(&mut r, |r| r.aviso.is_some()));
        assert!(r.aviso.take().is_some_and(|a| a.contains("Exemplo") && a.contains("de antes")));
        assert!(matches!(r.endereco(&m), Some(Ok(_))));
        assert_eq!(r.par(&m).map(|p| p.chave()).as_deref(), Some("exemplo:1"));
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn o_tocador_do_servico_liga_toca_e_a_que_ele_nao_toca_vai_pelo_match() {
        let Some((mut r, raiz)) = com_o_exemplo("sozinho", "\"--sozinho\"") else { return };
        let situacao = |r: &Remoto| r.tocador("exemplo").map(|l| l.estado.situacao);
        assert!(esperar(&mut r, |r| situacao(r) == Some(SituacaoDoTocador::Desligado)), "{:?}", r.provedores);
        let m = musica_de(&Faixa { servico: "exemplo".into(), id: "1".into(), ..Default::default() }, "Exemplo");
        assert_eq!(r.quem_toca_esta(&m), QuemToca::Ninguem);
        r.ligar_tocador("exemplo", Some("errada".into()));
        assert!(esperar(&mut r, |r| situacao(r) == Some(SituacaoDoTocador::Falhou)));
        r.ligar_tocador("exemplo", Some("nenhuma".into()));
        assert!(esperar(&mut r, |r| r.aviso.is_some()), "a ordem recusada diz o motivo");
        assert_eq!(r.aviso.take().as_deref(), Some("cole a chave"));
        r.ligar_tocador("exemplo", Some("certa".into()));
        assert!(esperar(&mut r, |r| situacao(r) == Some(SituacaoDoTocador::Pareando)));
        assert!(esperar(&mut r, |r| situacao(r) == Some(SituacaoDoTocador::Pronto)), "a pessoa pareou");
        assert_eq!(r.quem_toca_esta(&m), QuemToca::Sozinho);
        let n = r.controlar("exemplo", Acao::Tocar { faixa: "1".into() });
        assert!(esperar(&mut r, |r| r.tocador_depois("exemplo", n).is_some()));
        let tocando = r.tocador_depois("exemplo", n).and_then(|l| l.estado.tocando).expect("a faixa");
        assert_eq!((tocando.faixa.as_str(), tocando.duracao_s), ("1", 4.0));
        r.resolver(&m);
        assert!(matches!(r.endereco(&m), Some(Err(_))), "o áudio dela não é pedido a ninguém");
        // Tocou antes pelo Match: o endereço é do par, e nunca vira a análise da faixa do Spotify.
        let par = Par { servico: "yt".into(), id: "v".into(), ..Default::default() };
        let endereco = Ok(tocador::Endereco::novo("http://127.0.0.1:9/v"));
        travar(&r.estado).fontes.insert("exemplo:1".into(), FonteLida { endereco, par: Some(par), causa: None });
        assert_eq!(r.alvo_da_analise(&m, false), (None, None), "quem soa é o tocador do serviço");
        let (par, endereco) = r.alvo_da_analise(&m, true);
        assert_eq!((par.as_deref(), endereco.is_some()), (Some("yt:v"), true), "a que o daqui ainda toca");
        r.mandar_pelo_match(&m);
        assert_eq!(r.quem_toca_esta(&m), QuemToca::Ninguem, "sem outro serviço, ninguém");
        assert_eq!(r.endereco(&m), None);
        r.desligar_tocador("exemplo");
        assert!(esperar(&mut r, |r| situacao(r) == Some(SituacaoDoTocador::Desligado)));
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn um_par_que_falha_agora_e_trocado_so_desta_vez() {
        let Some((mut r, raiz)) = com_o_exemplo("match-falha", "\"--url\", \"http://127.0.0.1:9\"") else { return };
        assert!(esperar(&mut r, |r| r.provedores.first().is_some_and(|i| i.situacao == Situacao::Pronto)));
        let m = do_spotify("Primeira Faixa");
        let velho = Par { servico: "exemplo".into(), id: "99".into(), ..Default::default() };
        assert!(travar(&r.pares).guardar("spotify:s1", velho).is_some());
        r.resolver(&m);
        assert!(esperar(&mut r, |r| r.endereco(&m).is_some()));
        assert!(matches!(r.endereco(&m), Some(Ok(e)) if e.url.ends_with("/1")), "toca a que dá");
        assert_eq!(
            travar(&r.pares).de("spotify:s1").map(|p| p.id.clone()).as_deref(),
            Some("99"),
            "a boa volta depois"
        );
        assert_eq!(r.par(&m).map(|p| p.id).as_deref(), Some("1"), "a tela diz a que toca");
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn faixa_pedida_com_o_provedor_abrindo_espera() {
        let Some((mut r, raiz)) = com_o_exemplo("abrindo", "\"--url\", \"http://127.0.0.1:9\"") else { return };
        let m = do_spotify("Primeira Faixa");
        r.resolver(&m);
        assert_eq!(r.endereco(&m), None, "ainda não se sabe quem toca");
        assert!(esperar(&mut r, |r| { r.provedores.first().is_some_and(|i| i.situacao == Situacao::Pronto) }));
        r.resolver(&m);
        assert!(esperar(&mut r, |r| matches!(r.endereco(&m), Some(Ok(_)))));
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn sem_a_musica_no_outro_servico_diz_isso() {
        let Some((mut r, raiz)) = com_o_exemplo("match-nada", "\"--url\", \"http://127.0.0.1:9\"") else { return };
        assert!(esperar(&mut r, |r| r.provedores.first().is_some_and(|i| i.situacao == Situacao::Pronto)));
        let m = do_spotify("Uma que não existe lá");
        r.resolver(&m);
        assert!(esperar(&mut r, |r| matches!(r.endereco(&m), Some(Err(_)))));
        assert_eq!(r.par(&m), None);
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn sem_ninguem_para_tocar_o_erro_vem_na_hora() {
        let mut r = Remoto::iniciar(None, &[]);
        let m = do_spotify("Primeira Faixa");
        r.resolver(&m);
        assert!(matches!(r.endereco(&m), Some(Err(_))));
    }

    #[test]
    fn a_pergunta_da_conta_tem_fim() {
        let entrando = Conta { situacao: SituacaoDaConta::Entrando, ..Conta::default() };
        let dentro = Conta { situacao: SituacaoDaConta::Dentro, ..Conta::default() };
        assert_eq!(depois_de_perguntar(Ok(entrando.clone()), false, "X"), (Some(entrando.clone()), true));
        assert_eq!(depois_de_perguntar(Ok(dentro.clone()), false, "X"), (Some(dentro), false));
        assert_eq!(depois_de_perguntar(Err("mudo".into()), false, "X"), (None, true), "um tropeço: pergunta de novo");
        for resposta in [Ok(entrando), Err("mudo".into())] {
            let (conta, continua) = depois_de_perguntar(resposta, true, "X");
            let conta = conta.expect("passou do prazo: a tela sai de entrando");
            assert_eq!(conta.situacao, SituacaoDaConta::Fora);
            assert!(conta.erro.is_some_and(|e| e.contains('X')));
            assert!(!continua);
        }
    }

    /// Espera até `pronto` (relendo o estado), por até 10 s.
    fn esperar(r: &mut Remoto, pronto: impl Fn(&Remoto) -> bool) -> bool {
        let ate = Instant::now() + Duration::from_secs(10);
        while Instant::now() < ate {
            r.quadro();
            if pronto(r) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        false
    }

    #[test]
    fn instala_desliga_liga_remove_e_desfaz() {
        // Um "provedor" que não abre (o programa é um arquivo qualquer): a lista, a pasta e o liga-desliga
        // funcionam do mesmo jeito; a situação dele fica "Falhou".
        let raiz = std::env::temp_dir().join(format!("tccplayer-remoto-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let baixado = raiz.join("baixado");
        std::fs::create_dir_all(&baixado).expect("pasta");
        std::fs::write(baixado.join("provedor.toml"), "nome = \"Teste\"\nservico = \"teste\"\nexecutavel = \"t.exe\"")
            .expect("manifesto");
        std::fs::write(baixado.join("t.exe"), b"nao e um programa").expect("exe");
        let pasta = raiz.join("dados").join("provedores");
        let mut r = Remoto::iniciar(Some(pasta.clone()), &[]);
        assert!(r.provedores.is_empty() || esperar(&mut r, |r| r.provedores.is_empty()));

        r.instalar(baixado.clone());
        assert!(esperar(&mut r, |r| r.provedores.len() == 1), "instalado aparece na lista");
        assert!(r.aviso.take().is_some_and(|a| a.contains("instalado")));
        assert!(esperar(&mut r, |r| matches!(r.provedores[0].situacao, Situacao::Falhou(_))), "não abre: falhou");

        r.ligar("teste", false);
        r.quadro();
        assert_eq!(r.provedores[0].situacao, Situacao::Desligado);
        assert!(!r.tem_provedores());
        r.ligar("teste", true);
        assert!(r.tem_provedores());

        r.remover("teste");
        assert!(esperar(&mut r, |r| r.provedores.is_empty()), "removido sai da lista");
        assert_eq!(r.removido.take().as_deref(), Some("teste"), "o aviso ganha Desfazer");
        r.restaurar("teste".into());
        assert!(esperar(&mut r, |r| r.provedores.len() == 1), "Desfazer traz de volta");

        r.instalar(raiz.join("sem-provedor"));
        assert!(r.aviso.take().is_some_and(|a| a.contains("não tem um provedor")));
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn pagina_de_servico_que_nao_existe_vira_erro_na_hora() {
        let mut r = Remoto::iniciar(None, &[]);
        let p = PedidoDePagina::Inicio { servico: "nenhum".into() };
        r.abrir_pagina(p.clone());
        r.quadro();
        assert!(matches!(r.pagina(&p).as_deref(), Some(PaginaRemota::Erro(m)) if m.contains("não está instalado")));
    }

    #[test]
    fn pagina_vence_em_dez_minutos() {
        let agora = Instant::now();
        assert!(!vencida(agora, agora + Duration::from_secs(9 * 60)));
        assert!(vencida(agora, agora + Duration::from_secs(10 * 60 + 1)));
    }

    #[test]
    fn provedor_que_nao_abre_responde_erro_na_pagina() {
        let raiz = std::env::temp_dir().join(format!("tccplayer-remoto-pagina-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&raiz);
        let pasta = raiz.join("dados").join("provedores");
        let p = pasta.join("teste");
        std::fs::create_dir_all(&p).expect("pasta");
        std::fs::write(p.join("provedor.toml"), "nome = \"Teste\"\nservico = \"teste\"\nexecutavel = \"t.exe\"")
            .expect("manifesto");
        std::fs::write(p.join("t.exe"), b"nao e um programa").expect("exe");
        let mut r = Remoto::iniciar(Some(pasta), &[]);
        assert!(esperar(&mut r, |r| r.provedores.first().is_some_and(|i| matches!(i.situacao, Situacao::Falhou(_)))));
        let pedido = PedidoDePagina::Album { servico: "teste".into(), id: "x".into() };
        r.abrir_pagina(pedido.clone());
        assert!(
            esperar(&mut r, |r| matches!(r.pagina(&pedido).as_deref(), Some(PaginaRemota::Erro(_)))),
            "não fica carregando"
        );
        drop(r);
        let _ = std::fs::remove_dir_all(&raiz);
    }

    #[test]
    fn logo_valida_e_gravada_e_invalida_nao() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-logo-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        let boa = provedores::Identidade { cor: None, logo: Some("<svg viewBox='0 0 1 1'/>".into()) };
        let caminho = guardar_logo(&pasta, Some(&boa)).expect("gravou");
        assert!(caminho.is_file());
        let ruim = provedores::Identidade { cor: None, logo: Some("<png/>".into()) };
        assert_eq!(guardar_logo(&pasta, Some(&ruim)), None);
        assert!(!caminho.exists(), "a logo antiga sai");
        assert_eq!(guardar_logo(&pasta, None), None);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn tentar_de_novo_pede_outra_vez() {
        let mut r = Remoto::iniciar(None, &[]);
        let p = PedidoDePagina::Playlist { servico: "nenhum".into(), id: "x".into() };
        r.abrir_pagina(p.clone());
        r.quadro();
        assert!(matches!(r.pagina(&p).as_deref(), Some(PaginaRemota::Erro(_))));
        // Como se o provedor tivesse respondido: a página chega no estado e a versão muda.
        travar(&r.estado).paginas.insert(p.clone(), (Instant::now(), Arc::new(PaginaRemota::Itens(Vec::new()))));
        r.versao.fetch_add(1, Ordering::Release);
        r.abrir_pagina(p.clone());
        r.quadro();
        assert_eq!(
            r.pagina(&p).as_deref(),
            Some(&PaginaRemota::Itens(Vec::new())),
            "guardada e válida: não pede de novo"
        );
        r.de_novo(&p);
        r.quadro();
        assert!(matches!(r.pagina(&p).as_deref(), Some(PaginaRemota::Erro(_))), "de novo esquece a guardada e pede");
        let a = PedidoDePagina::Artista { servico: "nenhum".into(), id: "y".into() };
        assert_eq!(a.servico(), "nenhum");
    }

    #[test]
    fn erro_fica_guardado_ate_tentar_de_novo() {
        let mut r = Remoto::iniciar(None, &[]);
        let p = PedidoDePagina::Inicio { servico: "nenhum".into() };
        r.abrir_pagina(p.clone());
        r.quadro();
        let versao = r.versao.load(Ordering::Acquire);
        // A tela pode chamar de novo sem querer (a cada quadro): o erro fica, sem pedir nem mexer na versão.
        r.abrir_pagina(p.clone());
        assert_eq!(r.versao.load(Ordering::Acquire), versao, "erro guardado não pede de novo");
        r.de_novo(&p);
        assert!(r.versao.load(Ordering::Acquire) > versao, "Tentar de novo pede");
    }

    #[test]
    fn paginas_vencidas_saem_do_cache() {
        let mut r = Remoto::iniciar(None, &[]);
        let velha = PedidoDePagina::Inicio { servico: "velho".into() };
        let antes = Instant::now().checked_sub(Duration::from_secs(11 * 60)).expect("relógio");
        travar(&r.estado).paginas.insert(velha.clone(), (antes, Arc::new(PaginaRemota::Itens(Vec::new()))));
        r.abrir_pagina(PedidoDePagina::Inicio { servico: "outro".into() });
        assert!(!travar(&r.estado).paginas.contains_key(&velha), "a vencida saiu");
    }

    #[test]
    fn pagina_vencida_fica_na_tela_enquanto_renova() {
        let mut r = Remoto::iniciar(None, &[]);
        let p = PedidoDePagina::Inicio { servico: "nenhum".into() };
        let antes = Instant::now().checked_sub(Duration::from_secs(11 * 60)).expect("relógio");
        travar(&r.estado).paginas.insert(p.clone(), (antes, Arc::new(PaginaRemota::Itens(Vec::new()))));
        r.versao.fetch_add(1, Ordering::Release);
        r.abrir_pagina(p.clone());
        r.quadro();
        assert_eq!(r.pagina(&p).as_deref(), Some(&PaginaRemota::Itens(Vec::new())), "não some para 'carregando'");
        let versao = r.versao.load(Ordering::Acquire);
        r.abrir_pagina(p.clone());
        assert_eq!(r.versao.load(Ordering::Acquire), versao, "renovada agora: não pede de novo a cada quadro");
    }
}
