//! O lado do provedor: lê os pedidos, responde e cuida do `ola` e do `tchau`. Um provedor em Rust só implementa
//! [`Servico`] e chama [`servir`] com a entrada e a saída padrão.

use crate::protocolo::{Apresentacao, CorpoDaResposta, CorpoDoPedido, Resposta, em_linha, ler_pedido};
use std::io::{BufRead, Write};

pub trait Servico {
    fn apresentacao(&self) -> Apresentacao;
    /// Responde um pedido (nunca `ola` nem `tchau`, que [`servir`] já atende).
    fn atender(&mut self, pedido: CorpoDoPedido) -> CorpoDaResposta;
}

/// Atende os pedidos de `entrada` até o `tchau` ou o fim da entrada (o Player fechou), escrevendo em `saida`.
pub fn servir(entrada: impl BufRead, mut saida: impl Write, servico: &mut impl Servico) -> std::io::Result<()> {
    for linha in entrada.lines() {
        let linha = linha?;
        if linha.trim().is_empty() {
            continue;
        }
        let (id, corpo, sair) = match ler_pedido(&linha) {
            None => continue,
            Some(Err((id, motivo))) => (id, CorpoDaResposta::Erro { mensagem: motivo }, false),
            Some(Ok(p)) => match p.corpo {
                CorpoDoPedido::Ola { .. } => (p.id, CorpoDaResposta::Apresentacao(servico.apresentacao()), false),
                CorpoDoPedido::Tchau => (p.id, CorpoDaResposta::Ok, true),
                outro => (p.id, servico.atender(outro), false),
            },
        };
        writeln!(saida, "{}", em_linha(&Resposta { id, corpo }))?;
        saida.flush()?;
        if sair {
            break;
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use crate::protocolo::ler_resposta;

    struct Eco;

    impl Servico for Eco {
        fn apresentacao(&self) -> Apresentacao {
            Apresentacao { nome: "Eco".into(), servico: "eco".into(), protocolo: 1, ..Default::default() }
        }

        fn atender(&mut self, pedido: CorpoDoPedido) -> CorpoDaResposta {
            match pedido {
                CorpoDoPedido::Buscar { texto, .. } => CorpoDaResposta::Erro { mensagem: texto },
                _ => CorpoDaResposta::Ok,
            }
        }
    }

    #[test]
    fn atende_ate_o_tchau() {
        let entrada = concat!(
            r#"{"id":1,"pedido":"ola","protocolo":1}"#,
            "\n\nlixo\n",
            r#"{"id":2,"pedido":"buscar","texto":"oi"}"#,
            "\n",
            r#"{"id":3,"pedido":"voar"}"#,
            "\n",
            r#"{"id":4,"pedido":"tchau"}"#,
            "\n",
            r#"{"id":5,"pedido":"inicio"}"#,
            "\n"
        );
        let mut saida = Vec::new();
        servir(entrada.as_bytes(), &mut saida, &mut Eco).expect("serve");
        let respostas: Vec<Resposta> =
            String::from_utf8(saida).expect("utf8").lines().filter_map(ler_resposta).collect();
        assert_eq!(respostas.iter().map(|r| r.id).collect::<Vec<_>>(), vec![1, 2, 3, 4], "depois do tchau, nada");
        assert!(matches!(&respostas[0].corpo, CorpoDaResposta::Apresentacao(a) if a.servico == "eco"));
        assert_eq!(respostas[1].corpo, CorpoDaResposta::Erro { mensagem: "oi".into() });
        assert!(matches!(&respostas[2].corpo, CorpoDaResposta::Erro { mensagem } if mensagem.contains("voar")));
        assert_eq!(respostas[3].corpo, CorpoDaResposta::Ok);
    }
}
