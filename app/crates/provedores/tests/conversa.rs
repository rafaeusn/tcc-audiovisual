//! A conversa de verdade com um provedor: abre o provedor de exemplo (um programa à parte) e testa o protocolo
//! inteiro, inclusive o prazo e o provedor que fecha no meio.

#![allow(clippy::expect_used)]

use provedores::{Capacidade, CorpoDoPedido, Fonte, Provedor};
use std::path::Path;
use std::time::Duration;

const PRAZO: Duration = Duration::from_secs(5);

fn abrir_com(argumentos: &[String]) -> Provedor {
    let exe = Path::new(env!("CARGO_BIN_EXE_provedor-exemplo"));
    Provedor::iniciar(exe, argumentos, exe.parent().expect("pasta")).expect("o exemplo abre")
}

fn abrir() -> Provedor {
    abrir_com(&[])
}

#[test]
fn apresenta_busca_mostra_o_inicio_e_resolve() {
    let p = abrir();
    let a = p.apresentacao();
    assert_eq!((a.servico.as_str(), a.nome.as_str(), a.protocolo), ("exemplo", "Exemplo", 1));
    assert!(a.sabe(Capacidade::Busca) && a.sabe(Capacidade::Audio));
    assert_eq!(p.servico(), "exemplo");

    let achadas = p.buscar("artista um", 10, PRAZO).expect("busca");
    let titulos: Vec<&str> = achadas.iter().filter_map(|i| i.faixa()).map(|f| f.titulo.as_str()).collect();
    assert_eq!(titulos, vec!["Primeira Faixa", "Canção Três"]);
    assert_eq!(p.buscar("faixa", 1, PRAZO).expect("busca").len(), 1, "respeita o limite");
    assert!(p.buscar("nada disso", 10, PRAZO).expect("busca").is_empty());

    let inicio = p.inicio(PRAZO).expect("início");
    assert_eq!(inicio.len(), 1);
    assert_eq!(inicio[0].itens.len(), 3);

    // Sem --url, o exemplo não tem áudio: o erro dele chega como erro.
    let sem_audio = p.resolver("1", PRAZO).expect_err("sem endereço");
    assert!(sem_audio.to_string().contains("não tem áudio"), "{sem_audio}");
    let pedido_que_ele_nao_sabe = p.itens(CorpoDoPedido::Album { album: "x".into() }, PRAZO).expect_err("não sabe");
    assert!(pedido_que_ele_nao_sabe.to_string().contains("não sabe responder"));
    assert!(p.vivo());
}

#[test]
fn varios_pedidos_ao_mesmo_tempo() {
    let p = abrir();
    std::thread::scope(|s| {
        let maos: Vec<_> = (0..8)
            .map(|i| {
                let p = &p;
                s.spawn(move || p.buscar(if i % 2 == 0 { "primeira" } else { "segunda" }, 5, PRAZO))
            })
            .collect();
        for (i, m) in maos.into_iter().enumerate() {
            let itens = m.join().expect("thread").expect("busca");
            let esperado = if i % 2 == 0 { "Primeira Faixa" } else { "Segunda Faixa" };
            assert_eq!(itens[0].faixa().map(|f| f.titulo.as_str()), Some(esperado), "cada um recebe a sua resposta");
        }
    });
}

#[test]
fn prazo_e_provedor_que_fecha() {
    let p = abrir();
    let atrasado = p.buscar("devagar", 5, Duration::from_millis(300)).expect_err("estoura o prazo");
    assert!(atrasado.to_string().contains("não respondeu"), "{atrasado}");
    // A resposta atrasada que chega depois é descartada, e o próximo pedido recebe a dele.
    std::thread::sleep(Duration::from_secs(3));
    assert_eq!(p.buscar("segunda", 5, PRAZO).expect("busca").len(), 1);

    let fechou = p.buscar("fechar", 5, PRAZO).expect_err("o provedor sai");
    assert!(fechou.to_string().contains("fechou"), "{fechou}");
    assert!(!p.vivo());
    assert!(p.buscar("primeira", 5, PRAZO).is_err(), "fechado, nem tenta");
}

#[test]
fn programa_que_nao_existe_da_erro() {
    let erro = Provedor::iniciar(Path::new("C:/nao/existe/provedor.exe"), &[], Path::new(".")).err();
    assert!(erro.is_some());
}

#[test]
fn endereco_vem_do_argumento() {
    let p = abrir_com(&["--url".into(), "http://127.0.0.1:9/".into()]);
    match p.resolver("2", PRAZO).expect("fonte") {
        Fonte::Endereco { url, formato, duracao_s, cabecalhos } => {
            assert_eq!(url, "http://127.0.0.1:9/2");
            assert_eq!(formato.as_deref(), Some("wav"));
            assert_eq!(duracao_s, Some(5.0));
            assert_eq!(cabecalhos[0].0, "User-Agent");
        }
        outra => unreachable!("{outra:?}"),
    }
}
