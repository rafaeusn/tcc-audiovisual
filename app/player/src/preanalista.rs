//! A thread da pré-análise: analisa as músicas do computador em segundo plano e guarda no cache.
//!
//! Ordem: primeiro a música de agora e as próximas da fila (o que a tela vai precisar já), depois o resto da
//! biblioteca, uma por vez e com folga entre elas, em prioridade baixa no Windows. Uma análise da biblioteca é
//! interrompida na hora se chegar um pedido da música de agora.
//!
//! A interface só lê o que está pronto ([`Preanalista::analise`], [`Preanalista::sonoridade`]); nunca espera.

use preanalise::Analise;
use preanalise::cache::Cache;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Folga entre duas análises da biblioteca (o computador continua livre para o resto).
const FOLGA_DA_BIBLIOTECA: Duration = Duration::from_millis(300);

enum Pedido {
    /// A música de agora primeiro, depois as próximas.
    Prioridade(Vec<PathBuf>),
    /// Todas as músicas da biblioteca (analisadas aos poucos, quando não há prioridade).
    Biblioteca(Vec<PathBuf>),
    /// Liga ou desliga a análise da biblioteca em segundo plano.
    AnalisarBiblioteca(bool),
}

/// Quanto da biblioteca já foi analisado.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Progresso {
    pub analisadas: u32,
    pub total: u32,
    /// A música sendo analisada agora (o nome do arquivo).
    pub analisando: Option<String>,
}

#[derive(Default)]
struct Compartilhado {
    prontas: Mutex<HashMap<PathBuf, Arc<Analise>>>,
    sonoridades: Mutex<HashMap<PathBuf, (f32, f32)>>,
    progresso: Mutex<Progresso>,
    versao: AtomicU64,
    /// Interrompe a análise da biblioteca em andamento (chegou prioridade, ou o app está fechando).
    interromper: AtomicBool,
    parar: AtomicBool,
}

fn trancar<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    match m.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    }
}

pub struct Preanalista {
    pedidos: Option<Sender<Pedido>>,
    c: Arc<Compartilhado>,
    prioridade_pedida: Vec<PathBuf>,
    biblioteca_pedida: usize,
}

impl Preanalista {
    /// `pasta` é a pasta de dados (sem ela, nada é analisado).
    pub fn iniciar(pasta: Option<PathBuf>, analisar_biblioteca: bool) -> Self {
        let c = Arc::new(Compartilhado::default());
        let mut pedidos = None;
        if let Some(pasta) = pasta {
            let (envio, fila) = channel();
            let c2 = c.clone();
            let criada = std::thread::Builder::new()
                .name("preanalise".into())
                .spawn(move || rodar(&pasta.join("analises.db"), &fila, &c2, analisar_biblioteca));
            match criada {
                Ok(_) => pedidos = Some(envio),
                Err(e) => log::warn!("pré-análise desligada: {e}"),
            }
        }
        Self { pedidos, c, prioridade_pedida: Vec::new(), biblioteca_pedida: 0 }
    }

    fn pedir(&self, p: Pedido) {
        if let Some(s) = &self.pedidos {
            let _ = s.send(p);
        }
    }

    /// As músicas que a tela precisa já: a de agora primeiro. Só manda se mudou.
    pub fn priorizar(&mut self, caminhos: Vec<PathBuf>) {
        if caminhos == self.prioridade_pedida {
            return;
        }
        self.prioridade_pedida.clone_from(&caminhos);
        if !caminhos.is_empty() {
            self.c.interromper.store(true, Ordering::Release);
        }
        self.pedir(Pedido::Prioridade(caminhos));
    }

    /// A biblioteca inteira, para analisar aos poucos. Só manda se o tamanho mudou.
    pub fn biblioteca(&mut self, caminhos: &[PathBuf]) {
        if caminhos.len() == self.biblioteca_pedida {
            return;
        }
        self.biblioteca_pedida = caminhos.len();
        self.pedir(Pedido::Biblioteca(caminhos.to_vec()));
    }

    pub fn analisar_biblioteca(&self, ligado: bool) {
        if !ligado {
            self.c.interromper.store(true, Ordering::Release);
        }
        self.pedir(Pedido::AnalisarBiblioteca(ligado));
    }

    /// A análise da música, se está pronta (só as da prioridade ficam à mão).
    pub fn analise(&self, caminho: &Path) -> Option<Arc<Analise>> {
        trancar(&self.c.prontas).get(caminho).cloned()
    }

    /// Sonoridade (LUFS) e pico da música, se ela já foi analisada alguma vez.
    pub fn sonoridade(&self, caminho: &Path) -> Option<(f32, f32)> {
        trancar(&self.c.sonoridades).get(caminho).copied()
    }

    pub fn progresso(&self) -> Progresso {
        trancar(&self.c.progresso).clone()
    }
}

impl Drop for Preanalista {
    fn drop(&mut self) {
        self.c.parar.store(true, Ordering::Release);
        self.c.interromper.store(true, Ordering::Release);
    }
}

/// Prioridade baixa para a thread: as análises nunca disputam o processador com o som e a interface.
fn prioridade_baixa() {
    #[cfg(windows)]
    {
        use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL};
        // SAFETY: muda a prioridade da própria thread; o identificador da thread atual não precisa ser fechado.
        let _ = unsafe { SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL) };
    }
}

fn nome(caminho: &Path) -> String {
    caminho.file_name().map_or_else(|| caminho.to_string_lossy().into_owned(), |n| n.to_string_lossy().into_owned())
}

fn rodar(arquivo: &Path, fila: &Receiver<Pedido>, c: &Compartilhado, mut analisar_biblioteca: bool) {
    prioridade_baixa();
    let cache = match Cache::abrir(arquivo) {
        Ok(k) => k,
        Err(e) => {
            log::warn!("pré-análise desligada: {e}");
            return;
        }
    };
    *trancar(&c.sonoridades) = cache.sonoridades();
    let mut prioridade: Vec<PathBuf> = Vec::new();
    let mut biblioteca: VecDeque<PathBuf> = VecDeque::new();
    let mut total_biblioteca = 0u32;
    // Arquivos que não deram para analisar nesta execução: não tenta de novo até reabrir o app.
    let mut falharam: HashSet<PathBuf> = HashSet::new();
    let publicar_progresso = |analisando: Option<String>, total: u32| {
        *trancar(&c.progresso) = Progresso { analisadas: cache.quantas().min(total), total, analisando };
    };

    while !c.parar.load(Ordering::Acquire) {
        // Espera pedido só quando não há nada a fazer.
        let ocioso = prioridade.iter().all(|p| trancar(&c.prontas).contains_key(p) || falharam.contains(p))
            && (!analisar_biblioteca || biblioteca.is_empty());
        let primeiro = if ocioso {
            match fila.recv_timeout(Duration::from_millis(500)) {
                Ok(p) => Some(p),
                Err(RecvTimeoutError::Timeout) => continue,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            None
        };
        for pedido in primeiro.into_iter().chain(fila.try_iter()) {
            match pedido {
                Pedido::Prioridade(lista) => {
                    // Só as da prioridade ficam à mão: as outras saem da memória (continuam no cache).
                    trancar(&c.prontas).retain(|k, _| lista.contains(k));
                    prioridade = lista;
                }
                Pedido::Biblioteca(lista) => {
                    total_biblioteca = lista.len() as u32;
                    biblioteca = lista.into();
                    publicar_progresso(None, total_biblioteca);
                }
                Pedido::AnalisarBiblioteca(ligado) => analisar_biblioteca = ligado,
            }
        }

        // 1. A música de agora e as próximas.
        let falta =
            prioridade.iter().find(|p| !trancar(&c.prontas).contains_key(*p) && !falharam.contains(*p)).cloned();
        if let Some(caminho) = falta {
            let analise = match cache.ler(&caminho) {
                Some(a) => Some(a),
                None => {
                    publicar_progresso(Some(nome(&caminho)), total_biblioteca);
                    match preanalise::analisar(&caminho, &c.parar) {
                        Ok(a) => {
                            if let Err(e) = cache.guardar(&caminho, &a) {
                                log::warn!("análise não guardada: {e}");
                            }
                            Some(a)
                        }
                        Err(e) => {
                            log::info!("não deu para analisar {}: {e}", caminho.display());
                            falharam.insert(caminho.clone());
                            None
                        }
                    }
                }
            };
            if let Some(a) = analise {
                if let Some(l) = a.lufs {
                    trancar(&c.sonoridades).insert(caminho.clone(), (l, a.pico));
                }
                trancar(&c.prontas).insert(caminho, Arc::new(a));
            }
            publicar_progresso(None, total_biblioteca);
            c.versao.fetch_add(1, Ordering::Release);
            continue;
        }

        // 2. O resto da biblioteca, uma por vez.
        if !analisar_biblioteca {
            continue;
        }
        let Some(caminho) = biblioteca.pop_front() else { continue };
        if falharam.contains(&caminho) || cache.tem(&caminho) {
            continue;
        }
        c.interromper.store(false, Ordering::Release);
        publicar_progresso(Some(nome(&caminho)), total_biblioteca);
        match preanalise::analisar(&caminho, &c.interromper) {
            Ok(a) => {
                if let Err(e) = cache.guardar(&caminho, &a) {
                    log::warn!("análise não guardada: {e}");
                }
                if let Some(l) = a.lufs {
                    trancar(&c.sonoridades).insert(caminho, (l, a.pico));
                }
                c.versao.fetch_add(1, Ordering::Release);
            }
            Err(e) if c.interromper.load(Ordering::Acquire) => {
                // Interrompida por uma prioridade: volta para a fila e é feita depois.
                log::debug!("análise de {} interrompida ({e})", nome(&caminho));
                biblioteca.push_front(caminho);
            }
            Err(e) => {
                log::info!("não deu para analisar {}: {e}", caminho.display());
                falharam.insert(caminho);
            }
        }
        publicar_progresso(None, total_biblioteca);
        std::thread::sleep(FOLGA_DA_BIBLIOTECA);
    }
}
