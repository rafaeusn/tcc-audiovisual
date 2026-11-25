//! Um provedor de exemplo: um catálogo fixo de três faixas, para testar o protocolo de ponta a ponta e servir de
//! modelo para os provedores de verdade (YouTube Music, Spotify...).
//!
//! O áudio vem do endereço dado em `--url <endereço>` (a faixa `1` toca `<endereço>/1`); sem ele, `resolver`
//! responde erro. Para os testes: buscar "devagar" demora 3 s para responder, e buscar "fechar" faz o provedor sair na hora
//! (como se tivesse travado).
//!
//! Com `--conta`, finge uma conta (capacidade `Conta`): `entrar` passa a `entrando`, e a pergunta `conta` seguinte
//! já responde `dentro` (o nome é "exemplo"); `sair` volta a `fora`. Com `--cai`, a primeira pergunta em `entrando`
//! fecha o programa (como o librespot faz com uma conta que não é Premium), uma vez só; com `--volta`, depois dessa
//! queda ele já abre `entrando`, como um provedor com a credencial guardada. Com a conta ele também tem uma biblioteca
//! (uma playlist); com `--revoga`, pedi-la (ou buscar) derruba a conta (`fora`, com o motivo), como um login revogado no serviço.
//!
//! Com `--falha-rede N`, os primeiros N `resolver` respondem erro com a causa `rede`.
//!
//! Com `--sozinho`, em vez de dar o áudio ele finge um tocador próprio (capacidade `TocaSozinho`): ligar com a chave
//! "errada" falha (e oferece outro programa, o "1.0"), com "nenhuma" é recusado; com outra, a primeira pergunta diz `pareando` e a seguinte `pronto`. A faixa anda com o relógio; a `3`
//! falha, e a `2` é tomada por outro aparelho depois de 1 s. Com `--atrasa`, pausar, continuar e pular só valem 350 ms
//! depois de respondidos, como no tocador de verdade. Com `--adianta`, ele aceita a próxima antes do fim (`preparar`) e passa para
//! ela sozinho.

use provedores::{
    Acao, Apresentacao, Capacidade, Causa, Conta, CorpoDaResposta, CorpoDoPedido, EstadoDoTocador, Faixa, Fonte, Item,
    Playlist, Prateleira, Servico, SituacaoDaConta, SituacaoDaFaixa, SituacaoDoTocador, Tocando, VERSAO_DO_PROTOCOLO,
    servir,
};

struct Exemplo {
    catalogo: Vec<Faixa>,
    url: Option<String>,
    /// A conta fingida (`--conta`).
    conta: Option<Conta>,
    /// O tocador fingido (`--sozinho`) e desde quando a faixa dele anda.
    tocador: EstadoDoTocador,
    andando: Option<std::time::Instant>,
    /// A próxima, se o Player já disse (`--adianta`).
    proxima: Option<String>,
    /// Com `--atrasa`, as ordens (fora `tocar`) que ainda não valeram, e quando valem.
    atrasadas: Vec<(std::time::Instant, Acao)>,
    /// Quantos `resolver` ainda falham por rede (`--falha-rede`).
    falhas_de_rede: u32,
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
            capacidades: [Capacidade::Busca, Capacidade::Inicio]
                .into_iter()
                .chain([if tem("--sozinho") { Capacidade::TocaSozinho } else { Capacidade::Audio }])
                .chain(
                    self.conta.is_some().then_some([Capacidade::Conta, Capacidade::Biblioteca]).into_iter().flatten(),
                )
                .collect(),
            // Sem identidade: o Player usa a inicial e uma cor tirada do nome (o caso dos provedores antigos).
            identidade: None,
            conta: self.conta.clone(),
        }
    }

    fn atender(&mut self, pedido: CorpoDoPedido) -> CorpoDaResposta {
        match pedido {
            CorpoDoPedido::Buscar { .. } if tem("--revoga") => {
                self.conta = Some(Conta { erro: Some("o login foi revogado".into()), ..Conta::default() });
                CorpoDaResposta::erro("o login foi revogado")
            }
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
            CorpoDoPedido::Resolver { .. } if self.falhas_de_rede > 0 => {
                self.falhas_de_rede -= 1;
                CorpoDaResposta::erro_com(Causa::Rede, "sem conexão")
            }
            CorpoDoPedido::Resolver { faixa: id } => {
                let Some(f) = self.catalogo.iter().find(|f| f.id == id) else {
                    return CorpoDaResposta::erro(format!("faixa desconhecida: {id}"));
                };
                match &self.url {
                    Some(base) => CorpoDaResposta::Fonte(Fonte::Endereco {
                        url: format!("{}/{id}", base.trim_end_matches('/')),
                        cabecalhos: vec![("User-Agent".into(), "provedor-exemplo".into())],
                        formato: Some("wav".into()),
                        duracao_s: f.duracao_s,
                    }),
                    None => CorpoDaResposta::erro("este exemplo não tem áudio"),
                }
            }
            CorpoDoPedido::Biblioteca if tem("--revoga") => {
                self.conta = Some(Conta { erro: Some("o login foi revogado".into()), ..Conta::default() });
                CorpoDaResposta::erro("o login foi revogado")
            }
            CorpoDoPedido::Biblioteca if self.conta.is_some() => CorpoDaResposta::Itens {
                itens: vec![Item::Playlist(Playlist {
                    servico: "exemplo".into(),
                    id: "p1".into(),
                    titulo: "Lista de exemplo".into(),
                    ..Default::default()
                })],
            },
            CorpoDoPedido::Entrar | CorpoDoPedido::Sair | CorpoDoPedido::Conta if self.conta.is_some() => {
                self.atender_conta(&pedido)
            }
            CorpoDoPedido::Tocador if tem("--sozinho") => {
                while self.atrasadas.first().is_some_and(|(q, _)| *q <= std::time::Instant::now()) {
                    let (_, acao) = self.atrasadas.remove(0);
                    self.controlar(acao);
                }
                self.andar();
                self.tocador.adianta = tem("--adianta");
                let agora = self.tocador.clone();
                if self.tocador.situacao == SituacaoDoTocador::Pareando {
                    self.tocador.situacao = SituacaoDoTocador::Pronto;
                }
                CorpoDaResposta::Tocador(Box::new(agora))
            }
            CorpoDoPedido::LigarTocador { chave: Some(c) } if tem("--sozinho") && c.0 == "nenhuma" => {
                CorpoDaResposta::erro("cole a chave")
            }
            CorpoDoPedido::LigarTocador { chave } if tem("--sozinho") => {
                let errada = chave.is_some_and(|c| c.0 == "errada");
                let situacao = if errada { SituacaoDoTocador::Falhou } else { SituacaoDoTocador::Pareando };
                let alternativa = errada.then(|| "1.0".to_string());
                self.tocador = EstadoDoTocador { situacao, tem_chave: true, alternativa, ..Default::default() };
                CorpoDaResposta::Ok
            }
            CorpoDoPedido::TrocarTocador if tem("--sozinho") => {
                let versao = self.tocador.alternativa.take();
                let situacao = SituacaoDoTocador::Pareando;
                self.tocador = EstadoDoTocador { situacao, tem_chave: true, versao, ..Default::default() };
                CorpoDaResposta::Ok
            }
            CorpoDoPedido::DesligarTocador if tem("--sozinho") => {
                self.tocador = EstadoDoTocador { tem_chave: true, ..Default::default() };
                CorpoDaResposta::Ok
            }
            CorpoDoPedido::Controlar(acao) if tem("--sozinho") => {
                match acao {
                    Acao::Pausar | Acao::Continuar | Acao::Ir { .. } if tem("--atrasa") => {
                        let quando = std::time::Instant::now() + std::time::Duration::from_millis(350);
                        self.atrasadas.push((quando, acao));
                    }
                    _ => self.controlar(acao),
                }
                CorpoDaResposta::Ok
            }
            outro => CorpoDaResposta::erro(format!("o exemplo não sabe responder {outro:?}")),
        }
    }
}

impl Exemplo {
    fn controlar(&mut self, acao: Acao) {
        self.andar();
        if let Acao::Preparar { faixa } = acao {
            self.proxima = Some(faixa);
            return;
        }
        if let Acao::Tocar { faixa } = &acao {
            self.proxima = None;
            let duracao_s = self.catalogo.iter().find(|f| &f.id == faixa).and_then(|f| f.duracao_s).unwrap_or(4.0);
            let vez = self.tocador.tocando.as_ref().map_or(1, |t| t.vez + 1);
            let (situacao, erro) = match faixa.as_str() {
                "3" => (SituacaoDaFaixa::Falhou, Some("esta não toca".to_string())),
                _ => (SituacaoDaFaixa::Tocando, None),
            };
            self.tocador.tocando =
                Some(Tocando { faixa: faixa.clone(), vez, situacao, posicao_s: 0.0, duracao_s, erro });
        } else if let Some(t) = self.tocador.tocando.as_mut() {
            match acao {
                Acao::Pausar => t.situacao = SituacaoDaFaixa::Pausada,
                Acao::Continuar => t.situacao = SituacaoDaFaixa::Tocando,
                Acao::Ir { segundos } => t.posicao_s = segundos.min(t.duracao_s),
                _ => self.tocador.tocando = None,
            }
        }
        self.andando = Some(std::time::Instant::now());
    }

    /// A faixa anda com o relógio.
    fn andar(&mut self) {
        let passou = self.andando.replace(std::time::Instant::now()).map_or(0.0, |d| d.elapsed().as_secs_f64());
        if let Some(t) = self.tocador.tocando.as_mut().filter(|t| t.situacao == SituacaoDaFaixa::Tocando) {
            t.posicao_s = (t.posicao_s + passou).min(t.duracao_s);
            if t.faixa == "2" && t.posicao_s >= 1.0 {
                t.situacao = SituacaoDaFaixa::EmOutroLugar;
            } else if t.posicao_s >= t.duracao_s {
                match self.proxima.take() {
                    Some(faixa) => *t = Tocando { faixa, posicao_s: 0.0, duracao_s: 4.0, ..t.clone() },
                    None => t.situacao = SituacaoDaFaixa::Acabou,
                }
            }
        }
    }

    fn atender_conta(&mut self, pedido: &CorpoDoPedido) -> CorpoDaResposta {
        let conta = self.conta.get_or_insert_with(Conta::default);
        match pedido {
            CorpoDoPedido::Entrar => {
                conta.situacao = SituacaoDaConta::Entrando;
                CorpoDaResposta::Ok
            }
            CorpoDoPedido::Sair => {
                *conta = Conta::default();
                CorpoDaResposta::Ok
            }
            _ => {
                if conta.situacao == SituacaoDaConta::Entrando && tem("--cai") && !ja_caiu().exists() {
                    let _ = std::fs::write(ja_caiu(), "");
                    std::process::exit(1);
                }
                if conta.situacao == SituacaoDaConta::Entrando {
                    *conta = Conta {
                        situacao: SituacaoDaConta::Dentro,
                        nome: Some("exemplo".into()),
                        premium: Some(true),
                        ..Conta::default()
                    };
                }
                CorpoDaResposta::Conta(conta.clone())
            }
        }
    }
}

fn tem(argumento: &str) -> bool {
    std::env::args().any(|a| a == argumento)
}

/// O arquivo que lembra a queda do `--cai`, ao lado do programa.
fn ja_caiu() -> std::path::PathBuf {
    std::env::current_exe().unwrap_or_default().with_file_name("caiu")
}

fn main() {
    let ao_abrir = || Conta {
        situacao: if tem("--volta") && ja_caiu().exists() { SituacaoDaConta::Entrando } else { SituacaoDaConta::Fora },
        ..Conta::default()
    };
    let mut exemplo = Exemplo {
        catalogo: vec![
            faixa("1", "Primeira Faixa", "Artista Um", 4.0),
            faixa("2", "Segunda Faixa", "Artista Dois", 5.0),
            faixa("3", "Canção Três", "Artista Um", 6.0),
        ],
        url: std::env::args().skip_while(|a| a != "--url").nth(1),
        conta: tem("--conta").then(ao_abrir),
        tocador: EstadoDoTocador::default(),
        andando: None,
        proxima: None,
        atrasadas: Vec::new(),
        falhas_de_rede: std::env::args()
            .skip_while(|a| a != "--falha-rede")
            .nth(1)
            .and_then(|n| n.parse().ok())
            .unwrap_or(0),
    };
    eprintln!("exemplo pronto");
    if let Err(e) = servir(std::io::stdin().lock(), std::io::stdout(), &mut exemplo) {
        eprintln!("exemplo: {e}");
    }
}
