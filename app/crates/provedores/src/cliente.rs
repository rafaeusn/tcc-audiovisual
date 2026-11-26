//! O lado do Player: abre o programa de um provedor, conversa com ele e entrega cada resposta a quem pediu.
//!
//! Cada provedor tem uma thread que lê a saída dele e outra que passa a saída de erro para o registro. Os pedidos
//! podem vir de várias threads ao mesmo tempo (cada um espera a própria resposta, com prazo). Quem chama fica
//! esperando: na interface, os pedidos saem de uma thread própria, nunca da janela.

use crate::modelo::{Detalhe, Item, Prateleira};
use crate::protocolo::{
    Apresentacao, Causa, CorpoDaResposta, CorpoDoPedido, Fonte, Pedido, VERSAO_DO_PROTOCOLO, em_linha, ler_resposta,
};
use nucleo::{CausaDoProvedor, Erro, Resultado};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Quanto o provedor tem para se apresentar depois de aberto.
const PRAZO_DO_OLA: Duration = Duration::from_secs(10);
/// Quanto esperar o provedor sair sozinho depois do `tchau`, antes de encerrá-lo.
const PRAZO_DO_TCHAU: Duration = Duration::from_millis(400);

type Pendentes = Arc<Mutex<HashMap<u64, Sender<CorpoDaResposta>>>>;

fn erro(e: impl std::fmt::Display) -> Erro {
    Erro::Provedor(e.to_string())
}

/// O erro que o provedor mandou, com a causa quando ele soube classificar.
fn erro_do_provedor(mensagem: String, causa: Option<Causa>) -> Erro {
    match causa {
        Some(c) => Erro::ProvedorComCausa { mensagem, causa: do_protocolo(c) },
        None => Erro::Provedor(mensagem),
    }
}

/// A causa do protocolo na do núcleo, onde a interface e o tocador a entendem.
pub fn do_protocolo(c: Causa) -> CausaDoProvedor {
    match c {
        Causa::Rede => CausaDoProvedor::Rede,
        Causa::Limite => CausaDoProvedor::Limite,
        Causa::Indisponivel => CausaDoProvedor::Indisponivel,
        Causa::SemConta => CausaDoProvedor::SemConta,
        Causa::SemPremium => CausaDoProvedor::SemPremium,
        Causa::SessaoRecusada => CausaDoProvedor::SessaoRecusada,
        Causa::ChaveDeAudio => CausaDoProvedor::ChaveDeAudio,
        Causa::Quebrado => CausaDoProvedor::Quebrado,
    }
}

fn travar<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Um provedor aberto. Soltar o valor pede `tchau` e fecha o programa.
pub struct Provedor {
    apresentacao: Apresentacao,
    filho: Mutex<Child>,
    escrita: Mutex<Option<ChildStdin>>,
    pendentes: Pendentes,
    proximo: AtomicU64,
    vivo: Arc<AtomicBool>,
}

impl Provedor {
    /// Abre `executavel` (com `argumentos`, rodando em `pasta`) e espera ele se apresentar.
    pub fn iniciar(executavel: &Path, argumentos: &[String], pasta: &Path) -> Resultado<Self> {
        Self::iniciar_com(executavel, argumentos, pasta, &[])
    }

    /// Como [`Provedor::iniciar`], com variáveis de ambiente para o programa (a pasta de dados dele, por exemplo).
    pub fn iniciar_com(
        executavel: &Path,
        argumentos: &[String],
        pasta: &Path,
        ambiente: &[(String, std::ffi::OsString)],
    ) -> Resultado<Self> {
        let mut comando = Command::new(executavel);
        comando.args(argumentos).current_dir(pasta).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
        for (nome, valor) in ambiente {
            comando.env(nome, valor);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // CREATE_NO_WINDOW: o provedor não abre uma janela de console.
            comando.creation_flags(0x0800_0000);
        }
        let mut filho = comando.spawn().map_err(|e| Erro::arquivo(executavel, e))?;
        let nome = executavel.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let (Some(saida), Some(entrada), Some(erros)) = (filho.stdout.take(), filho.stdin.take(), filho.stderr.take())
        else {
            let _ = filho.kill();
            return Err(erro("o provedor abriu sem entrada ou saída"));
        };
        let pendentes: Pendentes = Arc::default();
        let vivo = Arc::new(AtomicBool::new(true));
        {
            let (pendentes, vivo) = (pendentes.clone(), vivo.clone());
            let leitor = std::thread::Builder::new()
                .name(format!("provedor-{nome}"))
                .spawn(move || ler_saida(BufReader::new(saida), &pendentes, &vivo));
            if let Err(e) = leitor {
                let _ = filho.kill();
                return Err(erro(e));
            }
        }
        let rotulo = nome.clone();
        let _ = std::thread::Builder::new().name(format!("provedor-{nome}-erros")).spawn(move || {
            for linha in BufReader::new(erros).lines().map_while(Result::ok) {
                log::info!("provedor {rotulo}: {linha}");
            }
        });
        let mut provedor = Self {
            apresentacao: Apresentacao::default(),
            filho: Mutex::new(filho),
            escrita: Mutex::new(Some(entrada)),
            pendentes,
            proximo: AtomicU64::new(1),
            vivo,
        };
        match provedor.pedir(CorpoDoPedido::Ola { protocolo: VERSAO_DO_PROTOCOLO }, PRAZO_DO_OLA)? {
            CorpoDaResposta::Apresentacao(a) if a.protocolo == VERSAO_DO_PROTOCOLO => {
                log::info!("provedor {} ({} {}) pronto", a.servico, a.nome, a.versao);
                provedor.apresentacao = a;
                Ok(provedor)
            }
            CorpoDaResposta::Apresentacao(a) => Err(erro(format!(
                "o provedor {} fala o protocolo {}, e este Player fala o {VERSAO_DO_PROTOCOLO}",
                a.nome, a.protocolo
            ))),
            outra => Err(erro(format!("o provedor não se apresentou (respondeu {outra:?})"))),
        }
    }

    pub fn apresentacao(&self) -> &Apresentacao {
        &self.apresentacao
    }

    pub fn servico(&self) -> &str {
        &self.apresentacao.servico
    }

    /// O programa ainda está rodando e conversando.
    pub fn vivo(&self) -> bool {
        self.vivo.load(Ordering::Acquire)
    }

    /// Manda um pedido e espera a resposta por até `prazo`. Uma resposta `erro` do provedor vira [`Erro::Provedor`].
    pub fn pedir(&self, corpo: CorpoDoPedido, prazo: Duration) -> Resultado<CorpoDaResposta> {
        if !self.vivo() {
            return Err(erro("o provedor fechou"));
        }
        let id = self.proximo.fetch_add(1, Ordering::Relaxed);
        let (avisar, resposta) = channel();
        travar(&self.pendentes).insert(id, avisar);
        let linha = em_linha(&Pedido { id, corpo });
        let escrito = match travar(&self.escrita).as_mut() {
            Some(e) => writeln!(e, "{linha}").and_then(|()| e.flush()).map_err(erro),
            None => Err(erro("o provedor fechou")),
        };
        if let Err(e) = escrito {
            travar(&self.pendentes).remove(&id);
            self.vivo.store(false, Ordering::Release);
            return Err(e);
        }
        match resposta.recv_timeout(prazo) {
            Ok(CorpoDaResposta::Erro { mensagem, causa }) => Err(erro_do_provedor(mensagem, causa)),
            Ok(corpo) => Ok(corpo),
            Err(RecvTimeoutError::Timeout) => {
                travar(&self.pendentes).remove(&id);
                let mensagem = format!("o provedor não respondeu em {:.0} s", prazo.as_secs_f32());
                Err(erro_do_provedor(mensagem, Some(Causa::Rede)))
            }
            // (O Player o reabre: tentar de novo resolve.)
            Err(RecvTimeoutError::Disconnected) => Err(erro_do_provedor("o provedor fechou".into(), Some(Causa::Rede))),
        }
    }

    pub fn buscar(&self, texto: &str, limite: u32, prazo: Duration) -> Resultado<Vec<Item>> {
        match self.pedir(CorpoDoPedido::Buscar { texto: texto.to_string(), limite }, prazo)? {
            CorpoDaResposta::Itens { itens, .. } => Ok(itens),
            outra => Err(inesperada(&outra)),
        }
    }

    pub fn inicio(&self, prazo: Duration) -> Resultado<Vec<Prateleira>> {
        match self.pedir(CorpoDoPedido::Inicio, prazo)? {
            CorpoDaResposta::Prateleiras { prateleiras } => Ok(prateleiras),
            outra => Err(inesperada(&outra)),
        }
    }

    /// Os itens de um álbum, artista ou playlist (`pedido` é `Album`, `Artista` ou `Playlist`).
    pub fn itens(&self, pedido: CorpoDoPedido, prazo: Duration) -> Resultado<Vec<Item>> {
        match self.pedir(pedido, prazo)? {
            CorpoDaResposta::Itens { itens, .. } => Ok(itens),
            outra => Err(inesperada(&outra)),
        }
    }

    /// Como [`itens`](Self::itens), com o que mais o serviço mandou da página.
    pub fn pagina(&self, pedido: CorpoDoPedido, prazo: Duration) -> Resultado<(Vec<Item>, Option<Box<Detalhe>>)> {
        match self.pedir(pedido, prazo)? {
            CorpoDaResposta::Itens { itens, detalhe } => Ok((itens, detalhe)),
            outra => Err(inesperada(&outra)),
        }
    }

    pub fn resolver(&self, id: &str, prazo: Duration) -> Resultado<Fonte> {
        match self.pedir(CorpoDoPedido::Resolver { faixa: id.to_string() }, prazo)? {
            CorpoDaResposta::Fonte(f) => Ok(f),
            outra => Err(inesperada(&outra)),
        }
    }
}

impl Provedor {
    /// Como está a conta (só para provedores com a capacidade `Conta`).
    pub fn conta(&self, prazo: Duration) -> Resultado<crate::protocolo::Conta> {
        match self.pedir(CorpoDoPedido::Conta, prazo)? {
            CorpoDaResposta::Conta(c) => Ok(c),
            outra => Err(inesperada(&outra)),
        }
    }

    /// Começa o login; o provedor continua sozinho (pergunte [`Self::conta`] depois).
    pub fn entrar(&self, prazo: Duration) -> Resultado<()> {
        match self.pedir(CorpoDoPedido::Entrar, prazo)? {
            CorpoDaResposta::Ok => Ok(()),
            outra => Err(inesperada(&outra)),
        }
    }

    pub fn sair(&self, prazo: Duration) -> Resultado<()> {
        match self.pedir(CorpoDoPedido::Sair, prazo)? {
            CorpoDaResposta::Ok => Ok(()),
            outra => Err(inesperada(&outra)),
        }
    }
}

impl Provedor {
    /// Como está o tocador do serviço (só para provedores com a capacidade `TocaSozinho`).
    pub fn tocador(&self, prazo: Duration) -> Resultado<crate::protocolo::EstadoDoTocador> {
        match self.pedir(CorpoDoPedido::Tocador, prazo)? {
            CorpoDaResposta::Tocador(e) => Ok(*e),
            outra => Err(inesperada(&outra)),
        }
    }

    /// Um pedido que só responde `ok` (ligar ou desligar o tocador, mandar nele).
    pub fn ordenar(&self, pedido: CorpoDoPedido, prazo: Duration) -> Resultado<()> {
        match self.pedir(pedido, prazo)? {
            CorpoDaResposta::Ok => Ok(()),
            outra => Err(inesperada(&outra)),
        }
    }
}

fn inesperada(r: &CorpoDaResposta) -> Erro {
    erro(format!("resposta inesperada do provedor: {r:?}"))
}

/// Lê a saída do provedor e entrega cada resposta a quem pediu. No fim (o provedor fechou), avisa quem ainda espera.
fn ler_saida(saida: impl BufRead, pendentes: &Pendentes, vivo: &AtomicBool) {
    for linha in saida.lines().map_while(Result::ok) {
        if linha.trim().is_empty() {
            continue;
        }
        match ler_resposta(&linha) {
            Some(r) => {
                if let Some(quem) = travar(pendentes).remove(&r.id) {
                    let _ = quem.send(r.corpo);
                }
            }
            None => log::warn!("provedor: linha ilegível na saída: {}", linha.chars().take(200).collect::<String>()),
        }
    }
    vivo.store(false, Ordering::Release);
    // Soltar os avisos acorda quem espera, com "o provedor fechou".
    travar(pendentes).clear();
}

impl Drop for Provedor {
    fn drop(&mut self) {
        if self.vivo() {
            let tchau = em_linha(&Pedido { id: 0, corpo: CorpoDoPedido::Tchau });
            if let Some(e) = travar(&self.escrita).as_mut() {
                let _ = writeln!(e, "{tchau}").and_then(|()| e.flush());
            }
        }
        // Fechar a entrada também avisa o provedor (fim da entrada = o Player saiu).
        travar(&self.escrita).take();
        let mut filho = travar(&self.filho);
        let ate = Instant::now() + PRAZO_DO_TCHAU;
        while Instant::now() < ate {
            if matches!(filho.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = filho.kill();
        let _ = filho.wait();
    }
}
