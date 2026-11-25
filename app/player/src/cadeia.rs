//! O que fazer quando uma faixa de serviço não tocou, pela causa que o provedor deu (a ideia veio da branch
//! `feat/spotify-audio-e-investigacao`). Só lógica, testada: o que é passageiro é tentado de novo, e o que é da faixa pula
//! só ela. Sem isto, a internet cair por um instante passava por todas as músicas da fila e a esvaziava.

use nucleo::CausaDoProvedor;
use std::time::Duration;

/// Quanto esperar antes de cada nova tentativa da mesma faixa.
const ESPERAS: [Duration; 3] = [Duration::from_secs(2), Duration::from_secs(5), Duration::from_secs(10)];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decisao {
    /// A mesma faixa de novo, depois disto.
    TentarDeNovo(Duration),
    /// Não é desta faixa (a rede, o serviço pedindo calma): a fila fica parada nela, sem perder nada.
    Esperar,
    /// É desta faixa (saiu do ar), ou não se sabe: segue para a próxima, com aviso.
    Pular,
}

/// `tentativas`: quantas vezes esta faixa já foi tentada de novo.
pub fn decidir(causa: Option<CausaDoProvedor>, tentativas: usize) -> Decisao {
    use CausaDoProvedor as C;
    match causa {
        Some(C::Rede | C::Limite) => match ESPERAS.get(tentativas) {
            Some(e) => Decisao::TentarDeNovo(if cfg!(test) { Duration::from_millis(60) } else { *e }),
            None => Decisao::Esperar,
        },
        _ => Decisao::Pular,
    }
}

/// O aviso da fila parada.
pub fn aviso_da_espera(causa: Option<CausaDoProvedor>, servico: &str) -> String {
    let por_que = match causa {
        Some(CausaDoProvedor::Limite) => format!("O {servico} pediu para esperar"),
        _ => format!("Sem conexão com o {servico}"),
    };
    format!("{por_que}: a fila ficou parada aqui. Aperte tocar para tentar de novo")
}

#[cfg(test)]
mod testes {
    use super::*;
    use CausaDoProvedor as C;

    #[test]
    fn o_passageiro_e_tentado_de_novo_e_depois_a_fila_espera() {
        for causa in [C::Rede, C::Limite] {
            for tentativa in 0..ESPERAS.len() {
                assert!(matches!(decidir(Some(causa), tentativa), Decisao::TentarDeNovo(_)), "{causa:?} {tentativa}");
            }
            assert_eq!(decidir(Some(causa), ESPERAS.len()), Decisao::Esperar, "a fila não é queimada");
        }
    }

    #[test]
    fn o_que_e_da_faixa_pula_so_ela() {
        for causa in [C::Indisponivel, C::Quebrado, C::SemConta, C::SemPremium, C::SessaoRecusada, C::Outra] {
            assert_eq!(decidir(Some(causa), 0), Decisao::Pular, "{causa:?}");
        }
        assert_eq!(decidir(None, 0), Decisao::Pular, "um provedor antigo não diz a causa");
    }

    #[test]
    fn o_aviso_diz_o_motivo() {
        assert!(aviso_da_espera(Some(C::Rede), "YouTube Music").contains("Sem conexão com o YouTube Music"));
        assert!(aviso_da_espera(Some(C::Limite), "YouTube Music").contains("pediu para esperar"));
    }
}
