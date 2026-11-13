//! Quando uma música começou e quando ela "conta" (vira um scrobble).
//!
//! A regra é a do Last.fm: a faixa precisa ter mais de 30 segundos e ter sido ouvida até a metade ou por
//! 4 minutos, o que vier primeiro. Pausa e silêncio não contam. Cada execução conta uma vez só; repetir a mesma
//! música (a posição volta ao começo depois de ter passado da metade) é outra execução.

use crate::Faixa;

/// Tempo ouvido que sempre basta, em segundos.
const TETO_S: f64 = 240.0;
/// Faixas mais curtas que isto nunca contam.
const MINIMA_S: f64 = 30.0;

/// O que se sabe da reprodução num instante.
#[derive(Clone, Debug, Default)]
pub struct Amostra {
    /// A faixa que o app de origem informa (sem título = sem faixa).
    pub faixa: Option<Faixa>,
    pub tocando: bool,
    /// Há som de verdade saindo.
    pub com_som: bool,
    /// (posição, duração), quando o app de origem informa.
    pub posicao: Option<(f64, f64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Evento {
    /// A faixa começou a tocar: é a hora do "tocando agora".
    Comecou { faixa: Faixa, duracao_s: Option<f64> },
    /// A faixa foi ouvida o bastante: é a hora do scrobble. `inicio` é quando ela começou (segundos desde 1970).
    Contou { faixa: Faixa, inicio: i64, duracao_s: Option<f64> },
}

struct Execucao {
    faixa: Faixa,
    inicio: i64,
    ouvido_s: f64,
    duracao_s: Option<f64>,
    anunciada: bool,
    contada: bool,
    ultima_posicao: Option<f64>,
}

#[derive(Default)]
pub struct Marcador {
    atual: Option<Execucao>,
}

/// Quanto precisa ser ouvido para a faixa contar. `None`: a faixa é curta demais e nunca conta.
pub fn limite(duracao_s: Option<f64>) -> Option<f64> {
    match duracao_s {
        Some(d) if d <= MINIMA_S => None,
        Some(d) => Some((d / 2.0).min(TETO_S)),
        // Sem saber a duração, só os 4 minutos garantem a regra.
        None => Some(TETO_S),
    }
}

impl Marcador {
    /// Avança `dt` segundos. Devolve o que aconteceu neste passo (quase sempre nada).
    pub fn passo(&mut self, a: &Amostra, dt: f64, agora: i64) -> Vec<Evento> {
        let mut eventos = Vec::new();
        let faixa = a.faixa.as_ref().filter(|f| !f.titulo.is_empty());
        let mudou = match (&self.atual, faixa) {
            (Some(e), Some(f)) => e.faixa != *f,
            (None, None) => false,
            _ => true,
        };
        let repetiu = !mudou
            && self.atual.as_ref().zip(a.posicao).is_some_and(|(e, (posicao, duracao))| {
                posicao < 5.0
                    && e.ultima_posicao.is_some_and(|antes| antes - posicao > duracao * 0.5)
                    && e.ouvido_s >= duracao * 0.5
            });
        if mudou || repetiu {
            self.atual = faixa.map(|f| Execucao {
                faixa: f.clone(),
                inicio: agora,
                ouvido_s: 0.0,
                duracao_s: None,
                anunciada: false,
                contada: false,
                ultima_posicao: None,
            });
        }
        let Some(e) = self.atual.as_mut() else { return eventos };
        if let Some((posicao, duracao)) = a.posicao {
            e.duracao_s = Some(duracao).filter(|d| d.is_finite() && *d > 1.0);
            e.ultima_posicao = Some(posicao);
        }
        if a.tocando && a.com_som && dt > 0.0 {
            if !e.anunciada {
                e.anunciada = true;
                eventos.push(Evento::Comecou { faixa: e.faixa.clone(), duracao_s: e.duracao_s });
            }
            e.ouvido_s += dt;
            if !e.contada && limite(e.duracao_s).is_some_and(|l| e.ouvido_s >= l) {
                e.contada = true;
                eventos.push(Evento::Contou { faixa: e.faixa.clone(), inicio: e.inicio, duracao_s: e.duracao_s });
            }
        }
        eventos
    }

    /// Há uma faixa tocando agora (para o cartão do Discord): ela e (posição, duração), se informadas.
    pub fn atual(&self) -> Option<&Faixa> {
        self.atual.as_ref().map(|e| &e.faixa)
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn faixa(titulo: &str) -> Faixa {
        Faixa { titulo: titulo.into(), artista: "Banda".into(), album: String::new() }
    }

    /// Toca `segundos` da faixa, um passo por segundo, a partir da posição `de`.
    fn tocar(m: &mut Marcador, f: &Faixa, segundos: u32, de: f64, duracao: Option<f64>, tocando: bool) -> Vec<Evento> {
        let mut eventos = Vec::new();
        for i in 0..segundos {
            let a = Amostra {
                faixa: Some(f.clone()),
                tocando,
                com_som: true,
                posicao: duracao.map(|d| (de + f64::from(i), d)),
            };
            eventos.extend(m.passo(&a, 1.0, 1000 + i64::from(i)));
        }
        eventos
    }

    #[test]
    fn limite_e_a_metade_ou_quatro_minutos() {
        assert_eq!(limite(Some(200.0)), Some(100.0));
        assert_eq!(limite(Some(600.0)), Some(240.0));
        assert_eq!(limite(Some(30.0)), None, "até 30 s nunca conta");
        assert_eq!(limite(Some(31.0)), Some(15.5));
        assert_eq!(limite(None), Some(240.0));
    }

    #[test]
    fn anuncia_ao_comecar_e_conta_na_metade() {
        let (mut m, a) = (Marcador::default(), faixa("A"));
        let e = tocar(&mut m, &a, 99, 0.0, Some(200.0), true);
        assert_eq!(e, [Evento::Comecou { faixa: a.clone(), duracao_s: Some(200.0) }]);
        let e = tocar(&mut m, &a, 1, 99.0, Some(200.0), true);
        assert_eq!(e, [Evento::Contou { faixa: a.clone(), inicio: 1000, duracao_s: Some(200.0) }]);
        assert!(tocar(&mut m, &a, 100, 100.0, Some(200.0), true).is_empty(), "conta uma vez só");
    }

    #[test]
    fn pausa_nao_conta_e_trocar_de_faixa_recomeca() {
        let (mut m, a, b) = (Marcador::default(), faixa("A"), faixa("B"));
        assert!(tocar(&mut m, &a, 50, 0.0, Some(200.0), false).is_empty(), "pausada: nem anuncia");
        assert_eq!(tocar(&mut m, &a, 60, 0.0, Some(200.0), true).len(), 1);
        assert!(tocar(&mut m, &a, 500, 60.0, Some(200.0), false).is_empty());
        // Pulou para a B antes da metade da A: a A não conta.
        let e = tocar(&mut m, &b, 100, 0.0, Some(200.0), true);
        assert_eq!(e.len(), 2);
        assert!(matches!(&e[1], Evento::Contou { faixa, .. } if faixa.titulo == "B"));
    }

    #[test]
    fn repetir_a_musica_e_outra_execucao() {
        let (mut m, a) = (Marcador::default(), faixa("A"));
        assert_eq!(tocar(&mut m, &a, 100, 0.0, Some(100.0), true).len(), 2);
        assert_eq!(tocar(&mut m, &a, 60, 0.0, Some(100.0), true).len(), 2, "começou e contou de novo");
        // Voltar um pouco não é repetir.
        assert!(tocar(&mut m, &a, 30, 20.0, Some(100.0), true).is_empty());
    }

    #[test]
    fn sem_duracao_so_conta_com_quatro_minutos_e_curta_nunca() {
        let (mut m, a) = (Marcador::default(), faixa("A"));
        assert_eq!(tocar(&mut m, &a, 239, 0.0, None, true).len(), 1);
        assert_eq!(tocar(&mut m, &a, 1, 0.0, None, true).len(), 1);
        let (mut m, curta) = (Marcador::default(), faixa("Vinheta"));
        assert_eq!(tocar(&mut m, &curta, 25, 0.0, Some(25.0), true).len(), 1, "anuncia, mas não conta");
        // Sem título não há o que contar.
        let sem = Amostra { faixa: Some(Faixa::default()), tocando: true, com_som: true, posicao: None };
        assert!(Marcador::default().passo(&sem, 300.0, 0).is_empty());
    }
}
