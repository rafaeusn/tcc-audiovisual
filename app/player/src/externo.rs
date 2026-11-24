//! Os tocadores dos próprios serviços (o Soloist do Spotify): uma thread pergunta o estado aos provedores que tocam sozinhos
//! e manda os comandos. A interface e o [`crate::local`] só leem o último estado.

use provedores::{
    Acao, Capacidade, CorpoDoPedido, EstadoDoTocador, Provedor, SituacaoDaFaixa, SituacaoDoTocador, Tocando,
};
use std::collections::HashMap;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

/// De quanto em quanto tempo perguntar: com uma faixa andando, com algo mudando (baixar, abrir, parear, pausada) e parado.
const TOCANDO: Duration = Duration::from_millis(200);
const MUDANDO: Duration = Duration::from_millis(500);
const PARADO: Duration = Duration::from_secs(3);
const PRAZO_DO_ESTADO: Duration = Duration::from_secs(3);
/// Ligar fecha o que estava aberto antes de responder.
const PRAZO_DA_ORDEM: Duration = Duration::from_secs(8);

/// Os provedores abertos, pelo serviço.
pub type Abertos = Arc<Mutex<HashMap<String, Weak<Provedor>>>>;

fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// O último estado de um tocador.
#[derive(Clone, Debug)]
pub struct Lido {
    pub estado: EstadoDoTocador,
    pub em: Instant,
    /// A última ordem já atendida quando este estado foi lido (ver [`Tocadores::mandar`]).
    feita: u64,
}

impl Lido {
    /// A posição agora: entre uma leitura e outra, a faixa que toca continua andando.
    pub fn posicao_s(&self, t: &Tocando, agora: Instant) -> f64 {
        let anda = if t.situacao == SituacaoDaFaixa::Tocando {
            agora.saturating_duration_since(self.em).as_secs_f64().min(1.0)
        } else {
            0.0
        };
        let p = t.posicao_s + anda;
        if t.duracao_s > 0.0 { p.min(t.duracao_s) } else { p }
    }
}

struct Ordem {
    servico: String,
    pedido: CorpoDoPedido,
    numero: u64,
}

pub struct Tocadores {
    estados: Arc<Mutex<HashMap<String, Lido>>>,
    ordens: Sender<Ordem>,
    numero: u64,
}

impl Tocadores {
    /// `mudou` é chamado quando a situação de um tocador muda (não a cada posição), ou com o motivo de uma ordem que não
    /// deu certo.
    pub fn iniciar(abertos: Abertos, mudou: impl Fn(Option<String>) + Send + 'static) -> Self {
        let (ordens, chegam) = channel();
        let estados = Arc::default();
        let laco = Laco { abertos, estados: Arc::clone(&estados), mudou: Box::new(mudou) };
        if let Err(e) = std::thread::Builder::new().name("tocadores".into()).spawn(move || laco.rodar(&chegam)) {
            log::warn!("os tocadores dos serviços não abriram: {e}");
        }
        Self { estados, ordens, numero: 0 }
    }

    /// Manda um pedido ao provedor do serviço e devolve o número da ordem (o estado lido depois dela é o que vale).
    pub fn mandar(&mut self, servico: &str, pedido: CorpoDoPedido) -> u64 {
        self.numero += 1;
        let _ = self.ordens.send(Ordem { servico: servico.to_string(), pedido, numero: self.numero });
        self.numero
    }

    pub fn estado(&self, servico: &str) -> Option<Lido> {
        travar(&self.estados).get(servico).cloned()
    }

    /// O estado lido depois da ordem `numero`, se já foi.
    pub fn estado_depois(&self, servico: &str, numero: u64) -> Option<Lido> {
        self.estado(servico).filter(|l| l.feita >= numero)
    }

    pub fn situacao(&self, servico: &str) -> SituacaoDoTocador {
        travar(&self.estados).get(servico).map(|l| l.estado.situacao).unwrap_or_default()
    }
}

struct Laco {
    abertos: Abertos,
    estados: Arc<Mutex<HashMap<String, Lido>>>,
    mudou: Box<dyn Fn(Option<String>) + Send>,
}

impl Laco {
    fn rodar(self, ordens: &Receiver<Ordem>) {
        loop {
            match ordens.recv_timeout(self.espera()) {
                Ok(ordem) => self.atender(ordem),
                Err(RecvTimeoutError::Timeout) => self.perguntar_a_todos(),
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    fn espera(&self) -> Duration {
        let lidos = travar(&self.estados);
        // Um provedor que abriu agora ainda não foi perguntado.
        let novo = travar(&self.abertos)
            .iter()
            .any(|(s, p)| !lidos.contains_key(s) && p.upgrade().is_some_and(|p| p.vivo() && toca_sozinho(&p)));
        if novo {
            return Duration::ZERO;
        }
        lidos.values().map(|l| ritmo(&l.estado)).min().unwrap_or(MUDANDO)
    }

    fn provedor(&self, servico: &str) -> Option<Arc<Provedor>> {
        travar(&self.abertos).get(servico).and_then(Weak::upgrade).filter(|p| p.vivo() && toca_sozinho(p))
    }

    fn atender(&self, ordem: Ordem) {
        let faixa = match &ordem.pedido {
            CorpoDoPedido::Controlar(Acao::Tocar { faixa }) => Some(faixa.clone()),
            _ => None,
        };
        let feito = match self.provedor(&ordem.servico) {
            Some(p) => p.ordenar(ordem.pedido, PRAZO_DA_ORDEM).map_err(|e| crate::remoto::motivo(&e)),
            None => Err("o provedor não está aberto".to_string()),
        };
        let mut estado = self.perguntar(&ordem.servico);
        if let Err(motivo) = feito {
            log::warn!("{}: {motivo}", ordem.servico);
            match faixa {
                Some(faixa) => {
                    let (situacao, erro) = (SituacaoDaFaixa::Falhou, Some(motivo));
                    estado.tocando = Some(Tocando { faixa, situacao, erro, ..Default::default() });
                }
                None => (self.mudou)(Some(motivo)),
            }
        }
        self.publicar(&ordem.servico, estado, Some(ordem.numero));
    }

    fn perguntar_a_todos(&self) {
        let servicos: Vec<String> = {
            let mut s: Vec<String> = travar(&self.estados).keys().cloned().collect();
            s.extend(travar(&self.abertos).keys().cloned());
            s.sort();
            s.dedup();
            s
        };
        for s in servicos {
            if self.provedor(&s).is_some() || travar(&self.estados).contains_key(&s) {
                let estado = self.perguntar(&s);
                self.publicar(&s, estado, None);
            }
        }
    }

    /// O estado agora; um provedor fechado tem o tocador desligado, e um ocupado fica como estava.
    fn perguntar(&self, servico: &str) -> EstadoDoTocador {
        let Some(p) = self.provedor(servico) else { return EstadoDoTocador::default() };
        match p.tocador(PRAZO_DO_ESTADO) {
            Ok(e) => e,
            Err(e) => {
                log::warn!("{servico}: {}", crate::remoto::motivo(&e));
                if !p.vivo() {
                    return EstadoDoTocador::default();
                }
                travar(&self.estados).get(servico).map(|l| l.estado.clone()).unwrap_or_default()
            }
        }
    }

    fn publicar(&self, servico: &str, estado: EstadoDoTocador, feita: Option<u64>) {
        let mut lidos = travar(&self.estados);
        let antes = lidos.get(servico);
        let mudou = antes.map(|l| l.estado.situacao) != Some(estado.situacao);
        let feita = feita.or(antes.map(|l| l.feita)).unwrap_or(0);
        lidos.insert(servico.to_string(), Lido { estado, em: Instant::now(), feita });
        drop(lidos);
        if mudou {
            (self.mudou)(None);
        }
    }
}

fn toca_sozinho(p: &Provedor) -> bool {
    p.apresentacao().sabe(Capacidade::TocaSozinho)
}

fn ritmo(e: &EstadoDoTocador) -> Duration {
    match (e.situacao, e.tocando.as_ref().map(|t| t.situacao)) {
        (SituacaoDoTocador::Pronto, Some(SituacaoDaFaixa::Carregando | SituacaoDaFaixa::Tocando)) => TOCANDO,
        (SituacaoDoTocador::Pronto, Some(SituacaoDaFaixa::Pausada | SituacaoDaFaixa::EmOutroLugar)) => MUDANDO,
        (SituacaoDoTocador::Baixando | SituacaoDoTocador::Abrindo | SituacaoDoTocador::Pareando, _) => MUDANDO,
        _ => PARADO,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_posicao_anda_entre_as_leituras_mas_nao_passa_do_fim() {
        let em = Instant::now();
        let lido = Lido { estado: EstadoDoTocador::default(), em, feita: 0 };
        let t = Tocando { situacao: SituacaoDaFaixa::Tocando, posicao_s: 10.0, duracao_s: 10.3, ..Default::default() };
        let depois = em + Duration::from_millis(200);
        assert!((lido.posicao_s(&t, depois) - 10.2).abs() < 1e-6);
        assert_eq!(lido.posicao_s(&t, em + Duration::from_secs(1)), 10.3);
        let pausada = Tocando { situacao: SituacaoDaFaixa::Pausada, ..t };
        assert_eq!(lido.posicao_s(&pausada, depois), 10.0);
        let sem_fim = Tocando { duracao_s: 0.0, situacao: SituacaoDaFaixa::Tocando, ..pausada };
        assert_eq!(lido.posicao_s(&sem_fim, em + Duration::from_secs(9)), 11.0, "sem leitura nova, para de andar");
    }
}
