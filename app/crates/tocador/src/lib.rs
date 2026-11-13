//! Toca arquivos de música.
//!
//! Uma thread própria decodifica o arquivo ([`fonte`]) e mantém cheia a fila da saída de som ([`saida`]). Quem
//! usa manda comandos ([`Comando`]) e lê a situação ([`Situacao`]); nunca espera o disco nem a placa de som.
//!
//! O volume não é daqui: é o do sistema, como no resto do app. O visual também não precisa de nada especial:
//! ele ouve o que sai do computador, e o que o tocador toca sai por lá.

pub mod fonte;
pub mod saida;

use fonte::Fonte;
use saida::Saida;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(Clone, Debug, PartialEq)]
pub enum Comando {
    /// Toca este arquivo do começo.
    Tocar(PathBuf),
    Pausar,
    Continuar,
    /// Para e esquece o arquivo.
    Parar,
    /// Vai para este ponto da música, em segundos.
    Ir(f64),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Situacao {
    /// O arquivo carregado (tocando ou pausado).
    pub arquivo: Option<PathBuf>,
    pub tocando: bool,
    pub posicao_s: f64,
    /// 0 quando o arquivo não informa.
    pub duracao_s: f64,
    /// Quantas músicas chegaram ao fim sozinhas. Quem cuida da fila toca a próxima quando este número muda.
    pub terminadas: u64,
    /// A última falha (arquivo ilegível, sem saída de som), em texto para a pessoa.
    pub erro: Option<String>,
}

/// Ligação com a thread do tocador. Soltar o valor encerra a thread e o som.
pub struct Tocador {
    comandos: Option<Sender<Comando>>,
    situacao: Arc<Mutex<Situacao>>,
}

fn com<T>(m: &Mutex<Situacao>, f: impl FnOnce(&mut Situacao) -> T) -> T {
    match m.lock() {
        Ok(mut g) => f(&mut g),
        Err(p) => f(&mut p.into_inner()),
    }
}

impl Tocador {
    /// `mudo`: toca sem som (a música anda, a posição conta, nada sai pela placa). Serve para testar.
    pub fn iniciar(mudo: bool) -> Self {
        let situacao = Arc::new(Mutex::new(Situacao::default()));
        let (comandos, fila) = channel::<Comando>();
        let s = situacao.clone();
        let criada = std::thread::Builder::new().name("tocador".into()).spawn(move || rodar(&fila, &s, mudo));
        match criada {
            Ok(_) => Self { comandos: Some(comandos), situacao },
            Err(e) => {
                log::error!("não foi possível iniciar o tocador: {e}");
                Self { comandos: None, situacao }
            }
        }
    }

    pub fn mandar(&self, comando: Comando) {
        if let Some(c) = &self.comandos {
            let _ = c.send(comando);
        }
    }

    pub fn situacao(&self) -> Situacao {
        com(&self.situacao, |s| s.clone())
    }
}

fn rodar(fila: &std::sync::mpsc::Receiver<Comando>, situacao: &Mutex<Situacao>, mudo: bool) {
    let mut saida: Option<Saida> = None;
    let mut fonte: Option<Fonte> = None;
    // De onde a contagem de quadros tocados parte (o começo, ou o ponto para onde a pessoa foi).
    let mut base_s = 0.0;
    let mut bloco: Vec<f32> = Vec::new();
    let mut acabou_de_ler = false;

    loop {
        let comando = match fila.recv_timeout(Duration::from_millis(8)) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        // A saída que falhou (fone desligado, por exemplo) é reaberta na próxima música.
        if saida.as_ref().is_some_and(|s| s.painel.falhou.load(Ordering::Acquire)) {
            saida = None;
            fonte = None;
            com(situacao, |s| {
                s.tocando = false;
                s.erro = Some("A saída de som foi desconectada.".into());
            });
        }
        match comando {
            Some(Comando::Tocar(caminho)) => {
                if saida.is_none() {
                    saida = Saida::abrir().map_err(|e| log::warn!("sem saída de som: {e}")).ok();
                }
                let aberta = saida.as_ref().map(|s| Fonte::abrir(&caminho, s.taxa));
                match (saida.as_ref(), aberta) {
                    (Some(s), Some(Ok(nova))) => {
                        s.painel.mudo.store(mudo, Ordering::Release);
                        s.painel.descartar.store(true, Ordering::Release);
                        s.painel.tocados.store(0, Ordering::Release);
                        s.painel.pausado.store(false, Ordering::Release);
                        base_s = 0.0;
                        bloco.clear();
                        acabou_de_ler = false;
                        let duracao_s = nova.duracao_s();
                        fonte = Some(nova);
                        com(situacao, |e| {
                            *e = Situacao {
                                arquivo: Some(caminho),
                                tocando: true,
                                duracao_s,
                                terminadas: e.terminadas,
                                ..Default::default()
                            };
                        });
                    }
                    (_, falha) => {
                        fonte = None;
                        let motivo = match falha {
                            Some(Err(e)) => format!("Não foi possível tocar este arquivo ({e})."),
                            _ => "Não há saída de som para tocar.".to_string(),
                        };
                        log::warn!("{motivo} {}", caminho.display());
                        com(situacao, |e| {
                            // A fila segue para a próxima: um arquivo ruim não trava a lista.
                            *e = Situacao { terminadas: e.terminadas + 1, erro: Some(motivo), ..Default::default() };
                        });
                    }
                }
            }
            Some(Comando::Pausar) | Some(Comando::Continuar) if fonte.is_some() => {
                let pausar = comando == Some(Comando::Pausar);
                if let Some(s) = &saida {
                    s.painel.pausado.store(pausar, Ordering::Release);
                }
                com(situacao, |e| e.tocando = !pausar);
            }
            Some(Comando::Parar) => {
                fonte = None;
                if let Some(s) = &saida {
                    s.painel.descartar.store(true, Ordering::Release);
                }
                com(situacao, |e| *e = Situacao { terminadas: e.terminadas, ..Default::default() });
            }
            Some(Comando::Ir(segundos)) => {
                if let (Some(f), Some(s)) = (fonte.as_mut(), saida.as_ref()) {
                    base_s = f.ir(segundos);
                    s.painel.descartar.store(true, Ordering::Release);
                    s.painel.tocados.store(0, Ordering::Release);
                    bloco.clear();
                    acabou_de_ler = false;
                }
            }
            _ => {}
        }

        let (Some(f), Some(s)) = (fonte.as_mut(), saida.as_mut()) else { continue };
        // Mantém a fila cheia. Enquanto um descarte está pendente, nada novo entra (seria jogado fora junto).
        while !s.painel.descartar.load(Ordering::Acquire) {
            if bloco.is_empty() && !acabou_de_ler && !f.proximo(&mut bloco) {
                acabou_de_ler = true;
            }
            if bloco.is_empty() || s.fila.slots() < bloco.len() {
                break;
            }
            for amostra in bloco.drain(..) {
                let _ = s.fila.push(amostra);
            }
        }
        let tocados = s.painel.tocados.load(Ordering::Acquire) as f64 / f64::from(s.taxa);
        let vazia = s.fila.slots() == s.fila.buffer().capacity();
        if acabou_de_ler && bloco.is_empty() && vazia {
            fonte = None;
            com(situacao, |e| {
                e.tocando = false;
                e.posicao_s = e.duracao_s.max(base_s + tocados);
                e.terminadas += 1;
            });
        } else {
            com(situacao, |e| e.posicao_s = base_s + tocados);
        }
    }
}
