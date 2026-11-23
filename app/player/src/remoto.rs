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
//! Instalar, atualizar e remover mexem na pasta do provedor, e o Windows não deixa mexer na pasta de um programa
//! aberto: a thread dele é encerrada primeiro (o programa fecha junto), e a cópia ou a mudança de pasta acontecem
//! em segundo plano. No fim, a lista é lida de novo.

use biblioteca::{Musica, Remota};
use provedores::{Capacidade, Faixa, Fonte, Instalado, Item, Manifesto, Provedor};
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
/// Quantas vezes seguidas um provedor que fechou é reaberto.
const REABERTURAS: u32 = 3;
/// Os provedores que vêm com o app (serviço, nome, programa ao lado do `player.exe`).
pub const EMBUTIDOS: [(&str, &str, &str); 1] = [("youtube-music", "YouTube Music", "provedor-youtube-music.exe")];

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
}

/// O resultado da busca de agora, juntando o que cada provedor já mandou.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Busca {
    pub texto: String,
    pub geracao: u64,
    pub faixas: Vec<Musica>,
    /// Quantos provedores ainda não responderam.
    pub faltam: usize,
    pub erros: Vec<String>,
}

#[derive(Default)]
struct Estado {
    provedores: Vec<InfoDoProvedor>,
    busca: Busca,
    /// O endereço de cada faixa resolvida (pela chave), ou por que não deu.
    fontes: HashMap<String, Result<tocador::Endereco, String>>,
    /// Avisos das tarefas de segundo plano (instalou, removeu, não deu).
    avisos: Vec<String>,
    /// Um provedor foi removido (o serviço): o aviso dele ganha "Desfazer".
    removido: Option<String>,
    /// A pasta dos provedores mudou: a lista tem de ser lida de novo.
    recarregar: bool,
}

enum Tarefa {
    Buscar { texto: String, geracao: u64 },
    Resolver { id: String, chave: String },
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
fn motivo(e: &nucleo::Erro) -> String {
    match e {
        nucleo::Erro::Provedor(m) => m.clone(),
        outro => outro.to_string(),
    }
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
        // Ainda não: entra com a captura por processo (F6, o player web do Spotify).
        Fonte::TocaSozinho { .. } => Err("este provedor toca sozinho, o que o Player ainda não sabe acompanhar".into()),
    }
}

pub struct Remoto {
    /// A pasta dos provedores (sem pasta de dados, nenhum).
    pasta: Option<PathBuf>,
    instalados: Vec<Instalado>,
    desligados: HashSet<String>,
    estado: Arc<Mutex<Estado>>,
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
    /// Um aviso para a pessoa: o app mostra e limpa.
    pub aviso: Option<String>,
    /// O provedor removido por último (o serviço), para o "Desfazer" do aviso.
    pub removido: Option<String>,
}

impl Remoto {
    /// Abre os provedores instalados em `pasta`, menos os `desligados` (cada um na sua thread; voltar não espera
    /// nenhum abrir).
    pub fn iniciar(pasta: Option<PathBuf>, desligados: &[String]) -> Self {
        let mut r = Self {
            pasta,
            instalados: Vec::new(),
            desligados: desligados.iter().cloned().collect(),
            estado: Arc::default(),
            versao: Arc::new(AtomicU64::new(1)),
            versao_vista: 0,
            trabalhadores: HashMap::new(),
            busca_pedida: None,
            geracao: 0,
            pedidas: HashSet::new(),
            busca: Busca::default(),
            provedores: Vec::new(),
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
        // A pasta de dados do provedor fica ao lado da dos provedores: `<dados>/provedores-dados/<serviço>`.
        let dados = self.pasta.as_deref().and_then(Path::parent).map(|d| d.join("provedores-dados").join(&servico));
        let criada = std::thread::Builder::new()
            .name(format!("provedor-{servico}-tarefas"))
            .spawn(move || trabalhar(&inst, dados.as_deref(), &tarefas, &e, &v));
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
        if self.busca_pedida.as_ref().is_some_and(|(t, _)| t == texto) || (self.busca.texto == texto) {
            return;
        }
        self.busca_pedida = Some((texto.to_string(), Instant::now()));
    }

    /// Pede o endereço de uma faixa (uma vez só; a resposta chega depois, por [`Remoto::endereco`]).
    pub fn resolver(&mut self, m: &Musica) {
        let Some(r) = &m.remota else { return };
        let chave = m.caminho.to_string_lossy().into_owned();
        if !self.pedidas.insert(chave.clone()) {
            return;
        }
        match self.trabalhadores.get(&r.servico) {
            Some(t) => {
                let _ = t.tarefas.send(Tarefa::Resolver { id: r.id.clone(), chave });
            }
            None => {
                let motivo = format!("o provedor {} não está instalado ou está desligado", r.fonte);
                travar(&self.estado).fontes.insert(chave, Err(motivo));
                self.versao.fetch_add(1, Ordering::Release);
            }
        }
    }

    /// O endereço de uma faixa, se já chegou (ou por que não deu).
    pub fn endereco(&self, m: &Musica) -> Option<Result<tocador::Endereco, String>> {
        travar(&self.estado).fontes.get(m.caminho.to_string_lossy().as_ref()).cloned()
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
        }
    }
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
) {
    let servico = inst.manifesto.servico.clone();
    let ambiente: Vec<(String, std::ffi::OsString)> =
        dados.map(|d| ("TCCPLAYER_PROVEDOR_DADOS".to_string(), d.as_os_str().to_owned())).into_iter().collect();
    let abrir = || Provedor::iniciar_com(&inst.executavel(), &inst.manifesto.argumentos, &inst.pasta, &ambiente);
    let situar = |s: Situacao, aberto: Option<&Provedor>| {
        if let Some(info) = travar(estado).provedores.iter_mut().find(|i| i.servico == servico) {
            info.situacao = s;
            if let Some(p) = aberto {
                info.capacidades = p.apresentacao().capacidades.clone();
                if !p.apresentacao().versao.is_empty() {
                    info.versao = p.apresentacao().versao.clone();
                }
            }
        }
        versao.fetch_add(1, Ordering::Release);
    };
    let mut provedor = match abrir() {
        Ok(p) => {
            if p.servico() != servico {
                log::warn!("o provedor de {servico} se apresentou como {} (vale o do provedor.toml)", p.servico());
            }
            situar(Situacao::Pronto, Some(&p));
            Some(p)
        }
        Err(e) => {
            log::warn!("o provedor {} não abriu: {e}", inst.manifesto.nome);
            situar(Situacao::Falhou(motivo(&e)), None);
            None
        }
    };
    let nome = inst.manifesto.nome.clone();
    let mut reaberturas = 0;
    for tarefa in tarefas {
        if provedor.as_ref().is_some_and(|p| !p.vivo()) && reaberturas < REABERTURAS {
            reaberturas += 1;
            log::info!("o provedor {nome} fechou; abrindo de novo ({reaberturas}/{REABERTURAS})");
            provedor = abrir().ok();
            match &provedor {
                Some(p) => situar(Situacao::Pronto, Some(p)),
                None => situar(Situacao::Falhou("fechou".into()), None),
            }
        }
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
                let mut e = travar(estado);
                if e.busca.geracao == geracao {
                    e.busca.faltam = e.busca.faltam.saturating_sub(1);
                    match resultado {
                        Ok(itens) => {
                            let faixas = itens.iter().filter_map(Item::faixa).map(|f| musica_de(f, &nome));
                            e.busca.faixas.extend(faixas);
                            reaberturas = 0;
                        }
                        Err(err) => e.busca.erros.push(format!("{nome}: {}", motivo(&err))),
                    }
                }
                drop(e);
                versao.fetch_add(1, Ordering::Release);
            }
            Tarefa::Resolver { id, chave } => {
                let resultado = match vivo {
                    Some(p) => p.resolver(&id, PRAZO).map_err(|e| motivo(&e)).and_then(endereco_de),
                    None => Err(format!("o provedor {nome} não está aberto")),
                };
                if let Err(e) = &resultado {
                    log::warn!("{nome} não deu o endereço de {chave}: {e}");
                }
                travar(estado).fontes.insert(chave, resultado);
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
    fn sem_provedores_nada_quebra() {
        let mut r = Remoto::iniciar(None, &[]);
        assert!(!r.tem_provedores());
        r.buscar("algo");
        std::thread::sleep(PAUSA_NA_DIGITACAO + Duration::from_millis(20));
        r.quadro();
        assert_eq!(r.busca.faltam, 0);
        assert!(r.busca.faixas.is_empty());
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
}
