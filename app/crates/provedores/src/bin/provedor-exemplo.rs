//! Um provedor de exemplo: um catálogo fixo de três faixas, para testar o protocolo de ponta a ponta e servir de
//! modelo para os provedores de verdade (YouTube Music, Spotify...).
//!
//! O áudio vem do endereço dado em `--url <endereço>` (a faixa `1` toca `<endereço>/1`); sem ele, `resolver`
//! responde erro. Para os testes: buscar "devagar" demora 3 s para responder, e buscar "fechar" faz o provedor sair na hora
//! (como se tivesse travado).

use provedores::{
    Apresentacao, Capacidade, CorpoDaResposta, CorpoDoPedido, Faixa, Fonte, Item, Prateleira, Servico,
    VERSAO_DO_PROTOCOLO, servir,
};

struct Exemplo {
    catalogo: Vec<Faixa>,
    url: Option<String>,
}

fn faixa(id: &str, titulo: &str, artista: &str, duracao_s: f64) -> Faixa {
    Faixa {
        servico: "exemplo".into(),
        id: id.into(),
        titulo: titulo.into(),
        artistas: vec![artista.into()],
        album: Some("Álbum de exemplo".into()),
        duracao_s: Some(duracao_s),
        ..Default::default()
    }
}

impl Servico for Exemplo {
    fn apresentacao(&self) -> Apresentacao {
        Apresentacao {
            nome: "Exemplo".into(),
            servico: "exemplo".into(),
            versao: env!("CARGO_PKG_VERSION").into(),
            protocolo: VERSAO_DO_PROTOCOLO,
            capacidades: vec![Capacidade::Busca, Capacidade::Inicio, Capacidade::Audio],
        }
    }

    fn atender(&mut self, pedido: CorpoDoPedido) -> CorpoDaResposta {
        match pedido {
            CorpoDoPedido::Buscar { texto, limite } => {
                match texto.as_str() {
                    "devagar" => std::thread::sleep(std::time::Duration::from_secs(3)),
                    "fechar" => std::process::exit(3),
                    _ => {}
                }
                let procura = nucleo::texto::normalizar(&texto);
                let itens = self
                    .catalogo
                    .iter()
                    .filter(|f| {
                        let onde = nucleo::texto::normalizar(&format!("{} {}", f.titulo, f.artistas_juntos()));
                        onde.contains(&procura)
                    })
                    .take(limite as usize)
                    .cloned()
                    .map(Item::Faixa)
                    .collect();
                CorpoDaResposta::Itens { itens }
            }
            CorpoDoPedido::Inicio => CorpoDaResposta::Prateleiras {
                prateleiras: vec![Prateleira {
                    titulo: "Para testar".into(),
                    itens: self.catalogo.iter().cloned().map(Item::Faixa).collect(),
                }],
            },
            CorpoDoPedido::Resolver { faixa: id } => {
                let Some(f) = self.catalogo.iter().find(|f| f.id == id) else {
                    return CorpoDaResposta::Erro { mensagem: format!("faixa desconhecida: {id}") };
                };
                match &self.url {
                    Some(base) => CorpoDaResposta::Fonte(Fonte::Endereco {
                        url: format!("{}/{id}", base.trim_end_matches('/')),
                        cabecalhos: vec![("User-Agent".into(), "provedor-exemplo".into())],
                        formato: Some("wav".into()),
                        duracao_s: f.duracao_s,
                    }),
                    None => CorpoDaResposta::Erro { mensagem: "este exemplo não tem áudio".into() },
                }
            }
            outro => CorpoDaResposta::Erro { mensagem: format!("o exemplo não sabe responder {outro:?}") },
        }
    }
}

fn main() {
    let mut exemplo = Exemplo {
        catalogo: vec![
            faixa("1", "Primeira Faixa", "Artista Um", 4.0),
            faixa("2", "Segunda Faixa", "Artista Dois", 5.0),
            faixa("3", "Canção Três", "Artista Um", 6.0),
        ],
        url: std::env::args().skip_while(|a| a != "--url").nth(1),
    };
    eprintln!("exemplo pronto");
    if let Err(e) = servir(std::io::stdin().lock(), std::io::stdout().lock(), &mut exemplo) {
        eprintln!("exemplo: {e}");
    }
}
