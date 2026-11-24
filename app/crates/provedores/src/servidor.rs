//! O lado do provedor: lê os pedidos, responde e cuida do `ola` e do `tchau`. Um provedor em Rust só implementa
//! [`Servico`] e chama [`servir`] com a entrada e a saída padrão.

use crate::protocolo::{Apresentacao, CorpoDaResposta, CorpoDoPedido, Resposta, em_linha, ler_pedido};
use std::io::{BufRead, Write};
use std::sync::{Mutex, PoisonError};

/// Um pedido lento, para responder de outra thread.
pub type EmSegundoPlano = Box<dyn FnOnce() -> CorpoDaResposta + Send>;

pub trait Servico {
    fn apresentacao(&self) -> Apresentacao;
    /// Responde um pedido (nunca `ola` nem `tchau`, que [`servir`] já atende).
    fn atender(&mut self, pedido: CorpoDoPedido) -> CorpoDaResposta;
    /// Os pedidos lentos, que não podem segurar os outros (pausar, pular): o que for devolvido roda noutra thread, e a
    /// resposta sai quando ficar pronta. `Err` devolve o pedido, que vai para [`Servico::atender`].
    fn em_segundo_plano(&mut self, pedido: CorpoDoPedido) -> Result<EmSegundoPlano, CorpoDoPedido> {
        Err(pedido)
    }
    /// O Player saiu (ou pediu `tchau`): solta o que não pode continuar, antes de esperar as tarefas lentas.
    fn encerrar(&mut self) {}
}

fn responder(saida: &Mutex<impl Write>, id: u64, corpo: CorpoDaResposta) -> std::io::Result<()> {
    let mut saida = saida.lock().unwrap_or_else(PoisonError::into_inner);
    writeln!(saida, "{}", em_linha(&Resposta { id, corpo }))?;
    saida.flush()
}

/// Atende os pedidos de `entrada` até o `tchau` ou o fim da entrada (o Player fechou), escrevendo em `saida`.
pub fn servir(entrada: impl BufRead, saida: impl Write + Send, servico: &mut impl Servico) -> std::io::Result<()> {
    let saida = Mutex::new(saida);
    std::thread::scope(|escopo| {
        let feito = (|| {
            for linha in entrada.lines() {
                let linha = linha?;
                if linha.trim().is_empty() {
                    continue;
                }
                let (id, corpo, sair) = match ler_pedido(&linha) {
                    None => continue,
                    Some(Err((id, motivo))) => (id, CorpoDaResposta::erro(motivo), false),
                    Some(Ok(p)) => match p.corpo {
                        CorpoDoPedido::Ola { .. } => {
                            (p.id, CorpoDaResposta::Apresentacao(servico.apresentacao()), false)
                        }
                        CorpoDoPedido::Tchau => (p.id, CorpoDaResposta::Ok, true),
                        outro => match servico.em_segundo_plano(outro) {
                            Err(outro) => (p.id, servico.atender(outro), false),
                            Ok(tarefa) => {
                                let (saida, id) = (&saida, p.id);
                                escopo.spawn(move || {
                                    // (Uma falha ao escrever aparece na próxima resposta do laço.)
                                    let _ = responder(saida, id, tarefa());
                                });
                                continue;
                            }
                        },
                    },
                };
                responder(&saida, id, corpo)?;
                if sair {
                    break;
                }
            }
            Ok(())
        })();
        servico.encerrar();
        feito
    })
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
                CorpoDoPedido::Buscar { texto, .. } => CorpoDaResposta::erro(texto),
                _ => CorpoDaResposta::Ok,
            }
        }
    }

    /// O `inicio` só termina depois que o `sair` foi atendido: em ordem, os dois esperariam para sempre.
    struct Lento {
        solta: Option<std::sync::mpsc::Receiver<()>>,
        soltar: std::sync::mpsc::Sender<()>,
    }

    impl Servico for Lento {
        fn apresentacao(&self) -> Apresentacao {
            Apresentacao::default()
        }

        fn atender(&mut self, _: CorpoDoPedido) -> CorpoDaResposta {
            let _ = self.soltar.send(());
            CorpoDaResposta::Ok
        }

        fn em_segundo_plano(&mut self, pedido: CorpoDoPedido) -> Result<EmSegundoPlano, CorpoDoPedido> {
            match (pedido, self.solta.take()) {
                (CorpoDoPedido::Inicio, Some(solta)) => Ok(Box::new(move || match solta.recv() {
                    Ok(()) => CorpoDaResposta::Prateleiras { prateleiras: Vec::new() },
                    Err(_) => CorpoDaResposta::erro("não soltou"),
                })),
                (outro, _) => Err(outro),
            }
        }
    }

    #[test]
    fn um_pedido_lento_nao_segura_os_outros() {
        let (soltar, solta) = std::sync::mpsc::channel();
        let entrada = concat!(r#"{"id":1,"pedido":"inicio"}"#, "\n", r#"{"id":2,"pedido":"sair"}"#, "\n");
        let mut saida = Vec::new();
        servir(entrada.as_bytes(), &mut saida, &mut Lento { solta: Some(solta), soltar }).expect("serve");
        let respostas: Vec<Resposta> =
            String::from_utf8(saida).expect("utf8").lines().filter_map(ler_resposta).collect();
        assert_eq!(respostas.iter().map(|r| r.id).collect::<Vec<_>>(), vec![2, 1]);
        assert!(matches!(respostas[1].corpo, CorpoDaResposta::Prateleiras { .. }));
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
        assert_eq!(respostas[1].corpo, CorpoDaResposta::erro("oi"));
        assert!(matches!(&respostas[2].corpo, CorpoDaResposta::Erro { mensagem, .. } if mensagem.contains("voar")));
        assert_eq!(respostas[3].corpo, CorpoDaResposta::Ok);
    }
}
