//! A mesa: o volume só da música que toca por fora do Player (o Soloist), no misturador do Windows, sem mexer no resto do PC.
//!
//! O visual ouve o Soloist **depois** desse volume, então a mesa também diz quanto multiplicar para desfazê-lo. A mudança demora uns 50 ms para chegar na captura: até lá vale o maior volume dos últimos
//! instantes, para o visual no máximo piscar para baixo, e nunca estourar.

use audio::volume::SomDeApp;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// Por quanto tempo o volume de antes ainda pode estar na captura (medido: até 57 ms).
const ATE_CHEGAR: Duration = Duration::from_millis(80);
/// O menor volume do programa (-100 dB, não se ouve): do zero, o visual não teria o que desfazer.
const PISO: f32 = 1e-5;
/// De quanto em quanto tempo olhar o misturador: com algo mudando e parado.
const MUDANDO: Duration = Duration::from_millis(10);
const PARADO: Duration = Duration::from_millis(400);
/// De quanto em quanto tempo procurar de novo os processos do programa.
const PROCURA: Duration = Duration::from_secs(2);
/// Menos que isto é arredondamento do Windows, e não alguém mexendo no misturador.
const IGUAL: f32 = 0.002;

fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

enum Pedido {
    Volume(f32),
    /// O programa que faz o som da música de agora (`None`: é o próprio Player).
    Programa(Option<String>),
}

pub struct Mesa {
    pedidos: Sender<Pedido>,
    ganho: Arc<AtomicU32>,
    de_fora: Arc<Mutex<Option<f32>>>,
    programa: Option<String>,
}

impl Mesa {
    pub fn iniciar(barra: f32) -> Self {
        let (pedidos, chegam) = channel();
        let ganho = Arc::new(AtomicU32::new(1f32.to_bits()));
        let de_fora = Arc::default();
        let (g, f) = (Arc::clone(&ganho), Arc::clone(&de_fora));
        if let Err(e) = std::thread::Builder::new().name("mesa".into()).spawn(move || rodar(&chegam, barra, &g, &f)) {
            log::warn!("o volume da música não abriu: {e}");
        }
        Self { pedidos, ganho, de_fora, programa: None }
    }

    /// Quanto multiplicar o que se captura do programa, para o visual não sentir o volume (os bits de um `f32`).
    pub fn ganho(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.ganho)
    }

    /// `barra`: onde está a barra do volume (0 a 1).
    pub fn volume(&self, barra: f32) {
        let _ = self.pedidos.send(Pedido::Volume(barra));
    }

    pub fn programa(&mut self, programa: Option<String>) {
        if programa != self.programa {
            self.programa.clone_from(&programa);
            let _ = self.pedidos.send(Pedido::Programa(programa));
        }
    }

    /// Onde a barra deve ir, se alguém mexeu no misturador do Windows desde a última pergunta.
    pub fn mudou_de_fora(&self) -> Option<f32> {
        travar(&self.de_fora).take().map(f32::sqrt)
    }
}

/// O ganho para uma barra nesta posição: o quadrado, porque o ouvido não é linear (na metade da barra, a metade do volume).
pub fn ganho_da_barra(barra: f32) -> f32 {
    if barra.is_finite() { barra.clamp(0.0, 1.0).powi(2) } else { 1.0 }
}

/// A conta da mesa, sem o Windows.
struct Conta {
    /// O volume do programa no misturador.
    alvo: f32,
    /// Os volumes dos últimos instantes (o primeiro é o que valia antes deles).
    marcos: VecDeque<(Instant, f32)>,
    /// O que já foi posto em cada som aberto.
    postos: HashMap<String, f32>,
}

#[derive(Debug, Default, PartialEq)]
struct Passo {
    /// Os processos que precisam do volume de agora.
    definir: Vec<u32>,
    /// Alguém mexeu no misturador.
    de_fora: Option<f32>,
}

impl Conta {
    fn nova(barra: f32) -> Self {
        Self { alvo: do_programa(barra), marcos: VecDeque::new(), postos: HashMap::new() }
    }

    fn marcar(&mut self, agora: Instant, volume: f32) {
        if self.marcos.back().is_none_or(|(_, v)| *v != volume) {
            self.marcos.push_back((agora, volume));
        }
        // O último de antes da janela é o que valia até ela começar.
        while self.marcos.get(1).is_some_and(|(q, _)| agora.duration_since(*q) >= ATE_CHEGAR) {
            self.marcos.pop_front();
        }
    }

    /// O que está no misturador agora: o que falta pôr, e se alguém mexeu lá.
    fn passo(&mut self, agora: Instant, sons: &[SomDeApp]) -> Passo {
        let mut passo = Passo::default();
        for som in sons {
            match self.postos.get(&som.id) {
                Some(posto) if (posto - som.volume).abs() > IGUAL => {
                    passo.de_fora = Some(som.volume);
                    self.alvo = som.volume.clamp(PISO, 1.0);
                }
                // (Um som que acabou de abrir, ou de ir para outra saída, vem com o volume que o Windows guardou.)
                None => self.marcar(agora, som.volume),
                Some(_) => {}
            }
        }
        self.postos.retain(|id, _| sons.iter().any(|s| s.id == *id));
        for som in sons {
            if (self.alvo - som.volume).abs() > 1e-6 && !passo.definir.contains(&som.pid) {
                passo.definir.push(som.pid);
            }
            self.postos.insert(som.id.clone(), self.alvo);
        }
        self.marcar(agora, self.alvo);
        passo
    }

    /// Quanto multiplicar a captura agora.
    fn ganho(&mut self, agora: Instant) -> f32 {
        self.marcar(agora, self.alvo);
        1.0 / self.marcos.iter().map(|(_, v)| *v).fold(PISO, f32::max)
    }

    fn mudando(&self) -> bool {
        self.marcos.len() > 1
    }
}

fn do_programa(barra: f32) -> f32 {
    ganho_da_barra(barra).max(PISO)
}

fn rodar(pedidos: &Receiver<Pedido>, barra: f32, ganho: &AtomicU32, de_fora: &Mutex<Option<f32>>) {
    let mut conta = Conta::nova(barra);
    let mut programa: Option<String> = None;
    let (mut pids, mut procurar_em) = (Vec::new(), Instant::now());
    loop {
        let espera = match &programa {
            None => Duration::from_secs(60),
            Some(_) if conta.mudando() => MUDANDO,
            Some(_) => PARADO,
        };
        let mut chegou = match pedidos.recv_timeout(espera) {
            Ok(p) => Some(p),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };
        // Arrastando a barra chegam muitos: só o último importa.
        while let Some(p) = chegou.take() {
            match p {
                Pedido::Volume(barra) => conta.alvo = do_programa(barra),
                Pedido::Programa(p) => {
                    programa = p;
                    conta.postos.clear();
                    procurar_em = Instant::now();
                }
            }
            chegou = pedidos.try_recv().ok();
        }
        let agora = Instant::now();
        let Some(exe) = &programa else { continue };
        if agora >= procurar_em {
            procurar_em = agora + PROCURA;
            pids = audio::Panorama::ler().do_player(exe);
        }
        let passo = conta.passo(agora, &audio::volume::ler_dos(&pids));
        if let Some(v) = passo.de_fora {
            *travar(de_fora) = Some(v);
        }
        // Subindo, o ganho desce antes de o som subir.
        ganho.store(conta.ganho(agora).to_bits(), Ordering::Release);
        if !passo.definir.is_empty() {
            audio::volume::definir_dos(&passo.definir, conta.alvo);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn ms(m: u64) -> Duration {
        Duration::from_millis(m)
    }

    fn som(pid: u32, volume: f32) -> SomDeApp {
        SomDeApp { id: format!("som-{pid}"), pid, volume }
    }

    #[test]
    fn baixar_so_e_desfeito_quando_o_som_ja_baixou() {
        let (mut c, t) = (Conta::nova(1.0), Instant::now());
        assert_eq!(c.passo(t, &[som(7, 1.0)]), Passo::default());
        assert_eq!(c.ganho(t), 1.0);
        c.alvo = do_programa(0.1);
        let p = c.passo(t + ms(500), &[som(7, 1.0)]);
        assert_eq!(p, Passo { definir: vec![7], de_fora: None });
        assert_eq!(c.ganho(t + ms(500)), 1.0, "o som de antes ainda está chegando");
        assert_eq!(c.ganho(t + ms(560)), 1.0);
        assert!(c.mudando());
        assert!((c.ganho(t + ms(581)) - 100.0).abs() < 0.01, "0,01 volta a ser 1");
        assert!(!c.mudando());
        // Subir: o ganho cai na hora, antes do som.
        c.alvo = 1.0;
        assert_eq!(c.ganho(t + ms(900)), 1.0);
    }

    #[test]
    fn arrastando_a_barra_nunca_passa_da_conta() {
        let (mut c, t) = (Conta::nova(1.0), Instant::now());
        c.passo(t, &[som(7, 1.0)]);
        // Uma barra que desce em meio segundo; o som que se captura é o de 50 ms antes.
        let mut pior = 0.0f32;
        for i in 0..100u64 {
            let em = t + ms(1000 + i * 10);
            c.alvo = do_programa(1.0 - i as f32 * 0.0099);
            c.passo(em, &[som(7, c.alvo)]);
            let chegando = do_programa(1.0 - i.saturating_sub(5) as f32 * 0.0099);
            pior = pior.max(chegando * c.ganho(em));
        }
        assert!(pior <= 1.0 + 1e-3, "{pior}");
        let em = t + ms(3000);
        let vale = do_programa(1.0 - 99.0 * 0.0099);
        assert!((vale * c.ganho(em) - 1.0).abs() < 1e-3, "no fim, o visual vê o som inteiro");
    }

    #[test]
    fn no_zero_o_visual_ainda_tem_o_que_desfazer() {
        let (mut c, t) = (Conta::nova(0.0), Instant::now());
        assert_eq!(c.alvo, PISO);
        c.passo(t, &[som(7, PISO)]);
        assert!((PISO * c.ganho(t + ms(100)) - 1.0).abs() < 1e-3);
        assert_eq!((do_programa(f32::NAN), ganho_da_barra(0.5), ganho_da_barra(7.0)), (1.0, 0.25, 1.0));
    }

    #[test]
    fn quem_mexe_no_misturador_manda_mas_um_programa_que_abre_nao() {
        let (mut c, t) = (Conta::nova(0.5), Instant::now());
        // Abriu com o volume que o Windows lembrava: vale o do Player.
        assert_eq!(c.passo(t, &[som(7, 0.9)]), Passo { definir: vec![7], de_fora: None });
        assert!((c.ganho(t) - 1.0 / 0.9).abs() < 1e-3, "o que está saindo agora é o de antes");
        assert_eq!(c.passo(t + ms(400), &[som(7, 0.25)]), Passo::default());
        // A pessoa mexeu lá.
        let p = c.passo(t + ms(800), &[som(7, 0.04)]);
        assert_eq!(p.de_fora, Some(0.04));
        assert_eq!(c.alvo, 0.04);
        assert!(p.definir.is_empty());
        // Foi para outra saída de som, que guardava o volume dela: não é ninguém mexendo.
        let outra = SomDeApp { id: "fone".into(), pid: 7, volume: 1.0 };
        assert_eq!(c.passo(t + ms(1000), &[outra]), Passo { definir: vec![7], de_fora: None });
        assert_eq!(c.alvo, 0.04);
        // O programa fechou e abriu de novo.
        c.passo(t + ms(1200), &[]);
        assert_eq!(c.passo(t + ms(1600), &[som(9, 1.0)]), Passo { definir: vec![9], de_fora: None });
    }
}
