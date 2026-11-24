//! A cadeia de reserva do Spotify (F6): quando um jeito de tocar falha, qual vem depois — e por quê.
//!
//! Os jeitos, na ordem em que a pessoa os quer: **librespot** (o áudio do Spotify no motor do Player, com pré-análise e
//! troca sem intervalo), **Soloist** (o player oficial do Spotify, com o som saindo por ele) e **Match** (a mesma música
//! no YouTube Music). A causa do erro decide: o que é passageiro tenta de novo o mesmo jeito, o que é daquela faixa manda
//! só ela para o Match, e o que é da conta ou da sessão derruba o jeito nesta abertura.
//!
//! A regra é pura de propósito: é a parte que mais erra, e a que os testes cobrem sem rede, sem Spotify e sem som.

use nucleo::CausaDoProvedor;

/// Como uma faixa de um serviço sem áudio próprio pode tocar.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Jeito {
    /// O áudio do serviço, no motor do Player (pelo `librespot`, no caso do Spotify).
    Proprio,
    /// O tocador do próprio serviço (o Soloist do Spotify): o som sai por ele.
    Tocador,
    /// A mesma música noutro serviço, pelo casador da F1.
    Match,
}

impl Jeito {
    /// O nome que a pessoa lê.
    pub fn nome(self) -> &'static str {
        match self {
            Self::Proprio => "o áudio do próprio serviço",
            Self::Tocador => "o player do serviço",
            Self::Match => "outro serviço",
        }
    }
}

/// O que fazer depois de uma falha.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decisao {
    /// Tenta de novo o mesmo jeito (a rede piscou, o serviço pediu para esperar).
    TentarDeNovo,
    /// Só esta faixa vai para o Match; o jeito continua valendo para as outras.
    SoEstaFaixa,
    /// O jeito caiu nesta abertura: a próxima é `Jeito`, se ela existir.
    Trocar(Jeito),
    /// Não há mais para onde ir: a faixa é pulada com o motivo.
    Desistir,
}

/// A decisão para uma falha com `causa`, sabendo qual é o jeito de agora e qual seria o próximo.
///
/// `proximo` é `None` quando não há outro jeito pronto (sem o Soloist ligado e sem outro serviço, por exemplo).
pub fn decidir(agora: Jeito, causa: Option<CausaDoProvedor>, proximo: Option<Jeito>) -> Decisao {
    use CausaDoProvedor as C;
    // O jeito seguinte, que não pode ser o de agora (trocar para si mesmo seria um laço).
    let adiante = proximo.filter(|p| *p != agora);
    match causa {
        // Passageiro: o mesmo jeito, de novo, costuma resolver.
        Some(C::Rede | C::Limite) => Decisao::TentarDeNovo,
        // A faixa: só ela muda de jeito (o resto da fila continua por aqui).
        Some(C::Indisponivel) => match adiante {
            Some(_) => Decisao::SoEstaFaixa,
            None => Decisao::Desistir,
        },
        // A conta ou a sessão inteira não serve: o tocador depende dela, então o próximo útil é o Match.
        Some(C::SemPremium | C::SessaoRecusada | C::SemConta) => adiante.map_or(Decisao::Desistir, Decisao::Trocar),
        // O áudio em si falhou (a chave recusada, uma resposta que não deu para ler): cai este jeito.
        Some(C::ChaveDeAudio | C::Quebrado) => adiante.map_or(Decisao::Desistir, Decisao::Trocar),
        // Sem causa (provedor antigo) ou uma que este Player não conhece: cai o jeito, como antes da cadeia.
        Some(C::Outra) | None => adiante.map_or(Decisao::Desistir, Decisao::Trocar),
    }
}

/// O texto do aviso, uma vez por troca, com o motivo em palavras simples.
pub fn aviso(causa: Option<CausaDoProvedor>, decisao: Decisao, servico: &str) -> Option<String> {
    use CausaDoProvedor as C;
    let por_que = match causa {
        Some(C::Rede) => "a conexão falhou",
        Some(C::Limite) => "o serviço pediu para esperar",
        Some(C::Indisponivel) => "a faixa não está disponível",
        Some(C::SemConta) => "a conta não está conectada",
        Some(C::SemPremium) => "a conta não é Premium",
        Some(C::SessaoRecusada) => "o serviço recusou a sessão",
        Some(C::ChaveDeAudio) => "o serviço recusou o áudio",
        Some(C::Quebrado) => "o serviço respondeu de um jeito que não deu para entender",
        Some(C::Outra) | None => "deu erro",
    };
    match decisao {
        Decisao::TentarDeNovo => None,
        Decisao::SoEstaFaixa => Some(format!("\"{servico}\" não está disponível aqui: tocando outra gravação")),
        Decisao::Trocar(para) => Some(format!("{por_que}: tocando pelo {}", para.nome())),
        // Sem próximo jeito, quem avisa é a faixa que não tocou (ela diz o nome e o motivo exato); um aviso daqui
        // apagaria esse.
        Decisao::Desistir => None,
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use CausaDoProvedor as C;

    #[test]
    fn a_rede_e_o_limite_nao_trocam_de_jeito() {
        for causa in [Some(C::Rede), Some(C::Limite)] {
            for agora in [Jeito::Proprio, Jeito::Tocador, Jeito::Match] {
                assert_eq!(decidir(agora, causa, proximo_de(agora)), Decisao::TentarDeNovo, "{agora:?} {causa:?}");
            }
        }
    }

    #[test]
    fn a_faixa_indisponivel_muda_so_ela() {
        assert_eq!(decidir(Jeito::Proprio, Some(C::Indisponivel), Some(Jeito::Match)), Decisao::SoEstaFaixa);
        assert_eq!(decidir(Jeito::Proprio, Some(C::Indisponivel), None), Decisao::Desistir);
    }

    #[test]
    fn a_sessao_recusada_vai_direto_ao_match() {
        // Do librespot, o Soloist depende da mesma sessão: pular para o Match.
        assert_eq!(
            decidir(Jeito::Proprio, Some(C::SessaoRecusada), Some(Jeito::Tocador)),
            Decisao::Trocar(Jeito::Tocador)
        );
        assert_eq!(decidir(Jeito::Proprio, Some(C::SemPremium), Some(Jeito::Tocador)), Decisao::Trocar(Jeito::Tocador));
        // Sem o Soloist e sem outro serviço, o erro diz que não há para onde ir.
        assert_eq!(decidir(Jeito::Proprio, Some(C::SemPremium), None), Decisao::Desistir);
    }

    #[test]
    fn a_chave_de_audio_derruba_o_jeito_e_vai_ao_proximo() {
        assert_eq!(
            decidir(Jeito::Proprio, Some(C::ChaveDeAudio), Some(Jeito::Tocador)),
            Decisao::Trocar(Jeito::Tocador)
        );
        // No Soloist, o próximo é o Match.
        assert_eq!(decidir(Jeito::Tocador, Some(C::ChaveDeAudio), Some(Jeito::Match)), Decisao::Trocar(Jeito::Match));
        // No Match não há mais para onde ir.
        assert_eq!(decidir(Jeito::Match, Some(C::ChaveDeAudio), Some(Jeito::Match)), Decisao::Desistir);
    }

    #[test]
    fn sem_proximo_nao_ha_troca() {
        assert_eq!(decidir(Jeito::Proprio, Some(C::Quebrado), None), Decisao::Desistir);
        assert_eq!(decidir(Jeito::Proprio, Some(C::Quebrado), Some(Jeito::Proprio)), Decisao::Desistir);
    }

    #[test]
    fn sem_causa_o_jeito_cai_como_antes() {
        assert_eq!(decidir(Jeito::Proprio, None, Some(Jeito::Match)), Decisao::Trocar(Jeito::Match));
        assert_eq!(decidir(Jeito::Proprio, Some(C::Outra), Some(Jeito::Tocador)), Decisao::Trocar(Jeito::Tocador));
    }

    #[test]
    fn o_aviso_sai_uma_vez_e_com_o_motivo() {
        let a = aviso(Some(C::ChaveDeAudio), Decisao::Trocar(Jeito::Tocador), "Spotify");
        let a = a.expect("aviso");
        assert!(a.contains("recusou o áudio") && a.contains("player do serviço"), "{a}");
        assert_eq!(aviso(Some(C::Rede), Decisao::TentarDeNovo, "Spotify"), None);
        // Sem próximo jeito, quem avisa é a faixa que não tocou (o nome e o motivo exato): nenhum aviso daqui.
        assert_eq!(aviso(Some(C::Quebrado), Decisao::Desistir, "Spotify"), None);
    }

    /// O próximo jeito da cadeia, como o Player o vê: librespot → Soloist → Match.
    fn proximo_de(agora: Jeito) -> Option<Jeito> {
        match agora {
            Jeito::Proprio => Some(Jeito::Tocador),
            Jeito::Tocador => Some(Jeito::Match),
            Jeito::Match => None,
        }
    }

    #[test]
    fn a_cadeia_inteira_termina_no_match() {
        // Do librespot ao Soloist, do Soloist ao Match, e no Match acaba.
        let mut agora = Jeito::Proprio;
        let mut passos = 0;
        while let Decisao::Trocar(para) = decidir(agora, Some(C::Quebrado), proximo_de(agora)) {
            agora = para;
            passos += 1;
            assert!(passos < 5, "a cadeia não pode girar");
        }
        assert_eq!((agora, passos), (Jeito::Match, 2));
    }
}
