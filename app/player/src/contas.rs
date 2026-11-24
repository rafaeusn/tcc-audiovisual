//! O que a tela mostra da conta de um serviço (entrar, entrando ou nada) e do tocador próprio dele. Só lógica, testada.

use crate::pares::QuemToca;
use crate::remoto::Situacao;
use provedores::{Conta, EstadoDoTocador, SituacaoDoTocador};

/// Por onde as músicas de um serviço tocam, numa frase.
pub fn por_onde_toca(quem: &QuemToca, situacao: &Situacao, nome: impl Fn(&str) -> String) -> String {
    match (situacao, quem) {
        (Situacao::Abrindo, _) | (_, QuemToca::Abrindo) => "Abrindo…".into(),
        (_, QuemToca::Ele) => "As músicas tocam pelo próprio Player.".into(),
        (_, QuemToca::Sozinho) => "As músicas tocam pelo Soloist, o player oficial do Spotify.".into(),
        (_, QuemToca::Outro(s)) => {
            format!("As músicas tocam pelo {}: a mesma música, às vezes noutra gravação.", nome(s))
        }
        (Situacao::Desligado, QuemToca::Ninguem) => "Desligado na Loja.".into(),
        (_, QuemToca::Ninguem) => "As músicas não tocam agora: ligue o YouTube Music na Loja.".into(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BotaoDoTocador {
    PaginaDaChave,
    ColarChave,
    Ligar,
    Desligar,
}

impl BotaoDoTocador {
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::PaginaDaChave => "Criar a chave no Spotify",
            Self::ColarChave => "Colar a chave copiada",
            Self::Ligar => "Ligar o Soloist",
            Self::Desligar => "Desligar o Soloist",
        }
    }

    pub fn principal(self) -> bool {
        matches!(self, Self::ColarChave | Self::Ligar)
    }
}

/// O que a tela mostra do tocador próprio de um serviço.
#[derive(Clone, Debug, PartialEq)]
pub struct TelaDoTocador {
    pub frase: String,
    pub progresso: Option<f32>,
    pub botoes: Vec<BotaoDoTocador>,
}

pub fn tela_do_tocador(e: &EstadoDoTocador) -> TelaDoTocador {
    use BotaoDoTocador::{ColarChave, Desligar, Ligar, PaginaDaChave};
    let erro = e.erro.as_deref().map(|m| format!(" Da última vez: {m}.")).unwrap_or_default();
    let nome = e.nome.as_deref().unwrap_or("Player (Soloist)");
    let (frase, botoes) = match e.situacao {
        SituacaoDoTocador::Desligado if e.tem_chave => (
            format!("O Soloist está desligado. Ligado, as músicas tocam com o som do próprio Spotify.{erro}"),
            vec![Ligar, ColarChave],
        ),
        SituacaoDoTocador::Desligado => (
            format!(
                "Para tocar com o som do próprio Spotify, ligue o Soloist, o player oficial dele (precisa de Premium). \
                 Crie a sua chave na página do Spotify, copie e cole aqui: ela fica cifrada neste computador, e o \
                 programa é baixado do Spotify.{erro}"
            ),
            vec![ColarChave, PaginaDaChave],
        ),
        SituacaoDoTocador::Baixando => ("Baixando o Soloist do Spotify…".into(), vec![Desligar]),
        SituacaoDoTocador::Abrindo => ("Abrindo o Soloist…".into(), vec![Desligar]),
        SituacaoDoTocador::Pareando => (
            format!(
                "Falta parear, uma vez só: abra o app do Spotify neste computador ou no celular (na mesma rede e na mesma \
                 conta), toque uma música e escolha \"{nome}\" na lista de aparelhos."
            ),
            vec![Desligar],
        ),
        SituacaoDoTocador::Pronto => (
            "O Soloist está ligado: o som é o do próprio Spotify. Equalizador, crossfade e pré-análise valem só pelo \
             YouTube Music."
                .into(),
            vec![Desligar],
        ),
        SituacaoDoTocador::Falhou => (
            format!("O Soloist parou: {}.", e.erro.as_deref().unwrap_or("não disse por quê")),
            if e.tem_chave { vec![Ligar, ColarChave, Desligar] } else { vec![ColarChave, PaginaDaChave, Desligar] },
        ),
    };
    let progresso = e
        .progresso
        .filter(|_| e.situacao == SituacaoDoTocador::Baixando)
        .map(|p| if p.is_finite() { p.clamp(0.0, 1.0) } else { 0.0 });
    TelaDoTocador { frase, progresso, botoes }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Aviso {
    /// Fora da conta: o cartão com o botão de entrar (e o erro da última tentativa, se houve).
    Entrar { erro: Option<String> },
    /// Entrando: com o navegador aberto esperando a pessoa, ou renovando a credencial guardada (sem navegador).
    Entrando { navegador: bool },
    /// Dentro, ou um serviço sem conta: as páginas de sempre.
    Nada,
}

pub fn aviso_da_conta(conta: Option<&Conta>) -> Aviso {
    use provedores::SituacaoDaConta;
    match conta {
        None => Aviso::Nada,
        Some(c) => match c.situacao {
            SituacaoDaConta::Fora => Aviso::Entrar { erro: c.erro.clone() },
            SituacaoDaConta::Entrando => Aviso::Entrando { navegador: c.navegador },
            SituacaoDaConta::Dentro => Aviso::Nada,
        },
    }
}

/// A situação da conta numa frase (a ficha do provedor na Loja).
pub fn frase_da_conta(conta: &Conta) -> String {
    use provedores::SituacaoDaConta;
    match conta.situacao {
        SituacaoDaConta::Fora => match &conta.erro {
            Some(e) => format!("Não conectado: {e}"),
            None => "Não conectado".into(),
        },
        SituacaoDaConta::Entrando if conta.navegador => "Aprove no navegador que abriu…".into(),
        SituacaoDaConta::Entrando => "Conectando…".into(),
        SituacaoDaConta::Dentro => {
            let plano = match conta.premium {
                Some(true) => " · Premium",
                Some(false) => " · Gratuita",
                None => "",
            };
            format!("Conectado como {}{plano}", conta.nome.as_deref().unwrap_or_default())
        }
    }
}

/// O botão da conta na Loja (`None` enquanto não há o que fazer).
pub fn botao_da_conta(conta: &Conta) -> Option<&'static str> {
    use provedores::SituacaoDaConta;
    match conta.situacao {
        SituacaoDaConta::Fora => Some("Entrar"),
        SituacaoDaConta::Entrando => conta.navegador.then_some("Cancelar"),
        SituacaoDaConta::Dentro => Some("Sair"),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_tela_do_tocador_mostra_o_proximo_passo() {
        use BotaoDoTocador::{ColarChave, Desligar, Ligar, PaginaDaChave};
        let com = |situacao, tem_chave| EstadoDoTocador { situacao, tem_chave, ..Default::default() };
        let tela = |s, k| tela_do_tocador(&com(s, k));
        assert_eq!(tela(SituacaoDoTocador::Desligado, false).botoes, [ColarChave, PaginaDaChave]);
        assert_eq!(tela(SituacaoDoTocador::Desligado, true).botoes, [Ligar, ColarChave]);
        assert!(tela(SituacaoDoTocador::Falhou, true).botoes.contains(&ColarChave), "a chave errada se troca");
        for s in [
            SituacaoDoTocador::Baixando,
            SituacaoDoTocador::Abrindo,
            SituacaoDoTocador::Pareando,
            SituacaoDoTocador::Pronto,
            SituacaoDoTocador::Falhou,
        ] {
            assert!(tela(s, true).botoes.contains(&Desligar), "{s:?}: sempre dá para sair");
        }
        let pareando = EstadoDoTocador { nome: Some("Sala".into()), ..com(SituacaoDoTocador::Pareando, true) };
        assert!(tela_do_tocador(&pareando).frase.contains("Sala"));
        let recusada =
            EstadoDoTocador { erro: Some("cole a chave".into()), ..com(SituacaoDoTocador::Desligado, false) };
        assert!(tela_do_tocador(&recusada).frase.contains("cole a chave"));
        let baixando = EstadoDoTocador { progresso: Some(7.0), ..com(SituacaoDoTocador::Baixando, true) };
        assert_eq!(tela_do_tocador(&baixando).progresso, Some(1.0));
        let abrindo = EstadoDoTocador { progresso: Some(0.5), ..com(SituacaoDoTocador::Abrindo, true) };
        assert_eq!(tela_do_tocador(&abrindo).progresso, None);
    }

    #[test]
    fn por_onde_toca_diz_o_caminho() {
        let frase = |q| por_onde_toca(&q, &Situacao::Pronto, |s| s.to_uppercase());
        assert!(frase(QuemToca::Outro("yt".into())).contains("YT"));
        assert!(frase(QuemToca::Sozinho).contains("Soloist"));
        assert!(frase(QuemToca::Ninguem).contains("Loja"));
        assert_eq!(por_onde_toca(&QuemToca::Ninguem, &Situacao::Desligado, |s| s.into()), "Desligado na Loja.");
    }
    use provedores::SituacaoDaConta;

    #[test]
    fn o_que_mostrar_da_conta() {
        let conta = |situacao, erro: Option<&str>| Conta { situacao, erro: erro.map(Into::into), ..Conta::default() };
        assert_eq!(aviso_da_conta(None), Aviso::Nada, "serviço sem conta");
        assert_eq!(aviso_da_conta(Some(&conta(SituacaoDaConta::Fora, None))), Aviso::Entrar { erro: None });
        assert_eq!(
            aviso_da_conta(Some(&conta(SituacaoDaConta::Fora, Some("porta")))),
            Aviso::Entrar { erro: Some("porta".into()) }
        );
        let renovando = conta(SituacaoDaConta::Entrando, None);
        assert_eq!(aviso_da_conta(Some(&renovando)), Aviso::Entrando { navegador: false });
        let navegador = Conta { navegador: true, ..renovando };
        assert_eq!(aviso_da_conta(Some(&navegador)), Aviso::Entrando { navegador: true });
        assert_eq!(aviso_da_conta(Some(&conta(SituacaoDaConta::Dentro, None))), Aviso::Nada);
    }

    #[test]
    fn a_conta_na_loja() {
        let fora = Conta::default();
        assert_eq!((frase_da_conta(&fora).as_str(), botao_da_conta(&fora)), ("Não conectado", Some("Entrar")));
        let com_erro = Conta { erro: Some("a porta está ocupada".into()), ..Conta::default() };
        assert_eq!(frase_da_conta(&com_erro), "Não conectado: a porta está ocupada");
        let renovando = Conta { situacao: SituacaoDaConta::Entrando, ..Conta::default() };
        assert_eq!((frase_da_conta(&renovando).as_str(), botao_da_conta(&renovando)), ("Conectando…", None));
        let navegador = Conta { navegador: true, ..renovando };
        assert_eq!(frase_da_conta(&navegador), "Aprove no navegador que abriu…");
        assert_eq!(botao_da_conta(&navegador), Some("Cancelar"));
        let dentro = |premium| Conta {
            situacao: SituacaoDaConta::Dentro,
            nome: Some("Ana".into()),
            premium,
            ..Conta::default()
        };
        assert_eq!(frase_da_conta(&dentro(Some(true))), "Conectado como Ana · Premium");
        assert_eq!(frase_da_conta(&dentro(Some(false))), "Conectado como Ana · Gratuita");
        assert_eq!(frase_da_conta(&dentro(None)), "Conectado como Ana");
        assert_eq!(botao_da_conta(&dentro(None)), Some("Sair"));
    }
}
