//! A página sai da frente: com música tocando e ninguém mexendo por um tempo, a Estante fica só com o visual. Só a
//! lógica; quem mostra e esconde é o `app.rs`.

use std::time::{Duration, Instant};

/// O ponteiro precisa andar isto para contar: o Windows manda "mexeu" com o mouse parado (uma janela que abre embaixo
/// dele, por exemplo).
const ANDOU: f32 = 3.0;
/// A música parada por menos que isto não acorda (a troca de faixa).
const PAUSA: Duration = Duration::from_secs(2);

/// O que a tela mostra neste quadro.
#[derive(Clone, Copy, Debug)]
pub struct Entrada {
    /// O ajuste (0 = nunca).
    pub espera_s: u32,
    pub tocando: bool,
    pub na_frente: bool,
    /// O visual e a página estão à vista (e nenhum painel aberto: pode estar sendo lido).
    pub a_vista: bool,
    /// Um campo com o teclado, um menu, o modo edição, uma janela por cima, um arrasto.
    pub ocupado: bool,
}

#[derive(Debug)]
pub struct Descanso {
    /// Desde quando ninguém mexe.
    desde: Instant,
    /// Desde quando a música está parada.
    parada: Option<Instant>,
    ponteiro: Option<(f32, f32)>,
    dormindo: bool,
}

impl Descanso {
    pub fn new(agora: Instant) -> Self {
        Self { desde: agora, parada: None, ponteiro: None, dormindo: false }
    }

    pub fn dormindo(&self) -> bool {
        self.dormindo
    }

    /// Um clique, uma tecla ou a rodinha. Devolve se a página estava fora (e volta).
    pub fn atividade(&mut self, agora: Instant) -> bool {
        self.desde = agora;
        std::mem::take(&mut self.dormindo)
    }

    /// O ponteiro está em (x, y). Devolve se a página estava fora (e volta).
    pub fn mexeu(&mut self, x: f32, y: f32, agora: Instant) -> bool {
        let andou = self.ponteiro.is_none_or(|(px, py)| (x - px).hypot(y - py) > ANDOU);
        if !andou {
            return false;
        }
        // A primeira posição só diz onde ele está.
        let primeira = self.ponteiro.replace((x, y)).is_none();
        !primeira && self.atividade(agora)
    }

    /// Um quadro: dorme quando deu a hora, acorda quando a música para ou algo abre.
    pub fn passo(&mut self, e: &Entrada, agora: Instant) {
        self.parada = if e.tocando { None } else { self.parada.or(Some(agora)) };
        if self.dormindo {
            let parou = self.parada.is_some_and(|d| agora.duration_since(d) >= PAUSA);
            if parou || e.ocupado || e.espera_s == 0 {
                self.atividade(agora);
            }
            return;
        }
        if !(e.tocando && e.na_frente && e.a_vista) || e.ocupado {
            self.desde = agora;
        }
        self.dormindo = e.espera_s > 0 && agora.duration_since(self.desde) >= Duration::from_secs(e.espera_s.into());
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    const PRONTO: Entrada = Entrada { espera_s: 30, tocando: true, na_frente: true, a_vista: true, ocupado: false };

    fn s(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    #[test]
    fn dorme_depois_do_tempo_e_acorda_ao_mexer() {
        let t = Instant::now();
        let mut d = Descanso::new(t);
        d.mexeu(100.0, 100.0, t);
        d.passo(&PRONTO, t + s(29));
        assert!(!d.dormindo());
        d.passo(&PRONTO, t + s(30));
        assert!(d.dormindo());
        // O Windows avisa com o mouse parado: não conta.
        assert!(!d.mexeu(101.0, 101.0, t + s(31)));
        assert!(d.dormindo());
        assert!(d.mexeu(110.0, 100.0, t + s(32)));
        assert!(!d.dormindo());
        // E a conta recomeça daí.
        d.passo(&PRONTO, t + s(61));
        assert!(!d.dormindo());
        d.passo(&PRONTO, t + s(62));
        assert!(d.dormindo());
        assert!(d.atividade(t + s(63)), "um clique ou uma tecla");
        assert!(!d.atividade(t + s(64)), "já estava acordado");
    }

    #[test]
    fn o_que_nao_conta_como_parado() {
        for e in [
            Entrada { tocando: false, ..PRONTO },
            Entrada { na_frente: false, ..PRONTO },
            Entrada { a_vista: false, ..PRONTO },
            Entrada { ocupado: true, ..PRONTO },
            Entrada { espera_s: 0, ..PRONTO },
        ] {
            let t = Instant::now();
            let mut d = Descanso::new(t);
            d.passo(&e, t + s(300));
            assert!(!d.dormindo(), "{e:?}");
        }
        // Escreveu até agora: os 30 s contam do fim.
        let t = Instant::now();
        let mut d = Descanso::new(t);
        d.passo(&Entrada { ocupado: true, ..PRONTO }, t + s(40));
        d.passo(&PRONTO, t + s(60));
        assert!(!d.dormindo());
    }

    #[test]
    fn a_musica_que_para_traz_a_pagina_mas_a_troca_de_faixa_nao() {
        let t = Instant::now();
        let mut d = Descanso::new(t);
        d.passo(&PRONTO, t + s(30));
        let parada = Entrada { tocando: false, ..PRONTO };
        d.passo(&parada, t + s(40));
        d.passo(&parada, t + s(41));
        assert!(d.dormindo(), "trocando de faixa");
        d.passo(&PRONTO, t + s(42));
        d.passo(&parada, t + s(50));
        d.passo(&parada, t + s(52));
        assert!(!d.dormindo());
        // Outra janela na frente não tira o visual da tela.
        let mut d = Descanso::new(t);
        d.passo(&PRONTO, t + s(30));
        d.passo(&Entrada { na_frente: false, a_vista: false, ..PRONTO }, t + s(31));
        assert!(d.dormindo());
        d.passo(&Entrada { ocupado: true, ..PRONTO }, t + s(32));
        assert!(!d.dormindo(), "algo abriu por cima");
    }
}
