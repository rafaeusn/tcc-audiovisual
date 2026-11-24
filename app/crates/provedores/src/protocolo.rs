//! O protocolo entre o Player e um provedor (versão 1): uma mensagem JSON por linha, pedidos na entrada do
//! provedor e respostas na saída, cada resposta com o `id` do pedido. A descrição completa, para quem escreve um
//! provedor em outra linguagem, está em `docs/f1-provedores.md`.

use crate::modelo::{Capacidade, Item, Prateleira};
use serde::{Deserialize, Deserializer, Serialize};

/// A versão do protocolo que este Player fala.
pub const VERSAO_DO_PROTOCOLO: u32 = 1;
/// Quantos resultados uma busca pede quando ninguém diz.
pub const LIMITE_PADRAO: u32 = 20;

fn limite_padrao() -> u32 {
    LIMITE_PADRAO
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pedido {
    pub id: u64,
    #[serde(flatten)]
    pub corpo: CorpoDoPedido,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "pedido", rename_all = "kebab-case")]
pub enum CorpoDoPedido {
    /// O primeiro pedido: o provedor se apresenta.
    Ola {
        protocolo: u32,
    },
    Buscar {
        texto: String,
        #[serde(default = "limite_padrao")]
        limite: u32,
    },
    /// As prateleiras da página inicial do serviço.
    Inicio,
    // Os campos não se chamam `id`: esse nome é o do pedido, e o JSON ficaria com dois.
    /// As faixas de um álbum.
    Album {
        album: String,
    },
    /// Os álbuns e as faixas mais tocadas de um artista.
    Artista {
        artista: String,
    },
    /// As faixas de uma playlist.
    Playlist {
        playlist: String,
    },
    /// A biblioteca da conta (capacidade `Biblioteca`): playlists, álbuns e artistas salvos nela, na ordem do serviço.
    Biblioteca,
    /// Como tocar uma faixa.
    Resolver {
        faixa: String,
    },
    /// Começa o login (capacidade `Conta`): o provedor responde `ok` na hora (ou `erro`) e o login continua nele; o
    /// Player pergunta `conta` até sair de `entrando`.
    Entrar,
    /// Sai da conta e esquece a credencial guardada.
    Sair,
    /// Como está a conta agora.
    Conta,
    /// Como está o tocador do próprio serviço (capacidade `TocaSozinho`).
    Tocador,
    /// Liga o tocador: o provedor responde `ok` na hora (ou `erro`) e continua sozinho (baixar, abrir, parear); o Player
    /// pergunta `tocador`. `chave`: a que a pessoa colou (sem ela, vale a guardada).
    LigarTocador {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        chave: Option<Segredo>,
    },
    DesligarTocador,
    /// Manda no que toca nele.
    Controlar(Acao),
    /// O Player vai fechar: o provedor responde `ok` e sai.
    Tchau,
}

/// Um texto que não pode aparecer em registro nem em mensagem (o `Debug` o esconde).
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Segredo(pub String);

impl std::fmt::Debug for Segredo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Segredo(…)")
    }
}

/// O que fazer com o que toca no tocador do serviço.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "acao", rename_all = "kebab-case")]
pub enum Acao {
    Tocar { faixa: String },
    Pausar,
    Continuar,
    Ir { segundos: f64 },
    Parar,
}

/// Como está o tocador do serviço.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EstadoDoTocador {
    pub situacao: SituacaoDoTocador,
    /// O nome do aparelho, para a pessoa escolher no app do serviço ao parear.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nome: Option<String>,
    /// De 0 a 1, baixando.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progresso: Option<f32>,
    /// Por que falhou (para mostrar à pessoa).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erro: Option<String>,
    /// Já há uma chave guardada.
    pub tem_chave: bool,
    /// O programa que faz o som, para o visual ouvir só ele.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tocando: Option<Tocando>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SituacaoDoTocador {
    #[default]
    Desligado,
    Baixando,
    Abrindo,
    /// Esperando a pessoa escolher o aparelho no app do serviço.
    Pareando,
    Pronto,
    Falhou,
}

/// A faixa pedida por último ao tocador.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tocando {
    pub faixa: String,
    /// O número deste `tocar` (sobe a cada um): a mesma faixa pedida de novo não se confunde com a de antes.
    pub vez: u64,
    pub situacao: SituacaoDaFaixa,
    pub posicao_s: f64,
    pub duracao_s: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub erro: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SituacaoDaFaixa {
    #[default]
    Carregando,
    Tocando,
    Pausada,
    Acabou,
    /// O serviço passou a tocar em outro aparelho.
    EmOutroLugar,
    Falhou,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Resposta {
    pub id: u64,
    #[serde(flatten)]
    pub corpo: CorpoDaResposta,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "resposta", rename_all = "kebab-case")]
pub enum CorpoDaResposta {
    Apresentacao(Apresentacao),
    Itens {
        itens: Vec<Item>,
    },
    Prateleiras {
        prateleiras: Vec<Prateleira>,
    },
    Fonte(Fonte),
    /// Algo não deu. `causa` (opcional) diz de que tipo foi o erro, para o Player decidir se tenta de novo ou troca
    /// de jeito de tocar; provedor antigo não manda, e o Player trata como antes.
    Erro {
        mensagem: String,
        #[serde(default, deserialize_with = "causa_conhecida", skip_serializing_if = "Option::is_none")]
        causa: Option<Causa>,
    },
    Ok,
    Conta(Conta),
    Tocador(Box<EstadoDoTocador>),
}

impl CorpoDaResposta {
    /// Um erro sem causa (o que todo provedor antigo manda).
    pub fn erro(mensagem: impl Into<String>) -> Self {
        Self::Erro { mensagem: mensagem.into(), causa: None }
    }

    /// Um erro com causa: o Player escolhe o que fazer com ele.
    pub fn erro_com(causa: Causa, mensagem: impl Into<String>) -> Self {
        Self::Erro { mensagem: mensagem.into(), causa: Some(causa) }
    }
}

/// Por que um pedido falhou, quando o provedor sabe classificar (F6).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Causa {
    /// A rede, um prazo: tentar de novo costuma resolver.
    Rede,
    /// O serviço pediu para esperar.
    Limite,
    /// A faixa não toca: saiu do catálogo ou não está no país desta conta. Só esta faixa muda de jeito.
    Indisponivel,
    /// Sem conta, ou a conta não serve para tocar.
    SemConta,
    /// A conta não é Premium (o áudio do serviço exige).
    SemPremium,
    /// O serviço recusou a sessão: entrar de novo.
    SessaoRecusada,
    /// O serviço recusou a chave que decifra o áudio desta faixa.
    ChaveDeAudio,
    /// Uma resposta que o provedor não entendeu (o serviço mudou).
    Quebrado,
}

/// Lê a causa sem derrubar a resposta: um provedor mais novo pode mandar uma que este Player não conhece, e ela vale
/// como "sem causa" (o erro continua chegando com a mensagem).
fn causa_conhecida<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Causa>, D::Error> {
    let valor = Option::<String>::deserialize(d)?;
    Ok(valor.and_then(|v| serde_json::from_value(serde_json::Value::String(v)).ok()))
}

/// A conta do serviço, para os provedores que entram numa (capacidade `Conta`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Conta {
    pub situacao: SituacaoDaConta,
    /// O nome da conta, quando dentro.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nome: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub premium: Option<bool>,
    /// Por que a última tentativa de entrar falhou (para mostrar à pessoa).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub erro: Option<String>,
    /// Em `entrando`: o navegador está aberto, esperando a pessoa (senão, a credencial guardada está renovando).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub navegador: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SituacaoDaConta {
    #[default]
    Fora,
    /// O navegador está aberto esperando a aprovação, ou a credencial guardada está sendo renovada.
    Entrando,
    Dentro,
}

/// Quem é o provedor e o que ele sabe fazer.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Apresentacao {
    pub nome: String,
    pub servico: String,
    #[serde(default)]
    pub versao: String,
    pub protocolo: u32,
    /// As capacidades que este Player não conhece (de um provedor mais novo) são ignoradas.
    #[serde(default, deserialize_with = "so_as_conhecidas")]
    pub capacidades: Vec<Capacidade>,
    /// A cara do serviço (cor e logo), para o espaço dele no Player. Provedor antigo não manda.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub identidade: Option<Identidade>,
    /// A conta ao abrir (só provedores com a capacidade `Conta`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conta: Option<Conta>,
}

/// O tamanho máximo da logo (SVG em texto).
pub const MAXIMO_DA_LOGO: usize = 32 * 1024;

/// A identidade visual do serviço: a cor da marca (`#RRGGBB`) e a logo em SVG, num quadrado.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Identidade {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
}

impl Identidade {
    /// A cor como `0xRRGGBB`, só se vier no formato `#RRGGBB`.
    pub fn cor_rgb(&self) -> Option<u32> {
        let hex = self.cor.as_deref()?.strip_prefix('#')?;
        if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(hex, 16).ok()
    }

    /// A logo, se for um SVG de até [`MAXIMO_DA_LOGO`] bytes.
    pub fn logo_valida(&self) -> Option<&str> {
        let logo = self.logo.as_deref()?;
        if logo.len() > MAXIMO_DA_LOGO {
            return None;
        }
        let mut corpo = logo.trim_start();
        if corpo.starts_with("<?xml") {
            corpo = corpo.split_once("?>")?.1.trim_start();
        }
        corpo.starts_with("<svg").then_some(logo)
    }
}

impl Apresentacao {
    pub fn sabe(&self, c: Capacidade) -> bool {
        self.capacidades.contains(&c)
    }
}

fn so_as_conhecidas<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Capacidade>, D::Error> {
    let valores = Vec::<serde_json::Value>::deserialize(d)?;
    Ok(valores.into_iter().filter_map(|v| serde_json::from_value(v).ok()).collect())
}

/// Como tocar uma faixa.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "fonte", rename_all = "kebab-case")]
pub enum Fonte {
    /// O motor do Player baixa (aos pedaços), decodifica, mistura e pré-analisa.
    Endereco {
        url: String,
        /// Cabeçalhos que o serviço exige (agente, origem, cookies da sessão do usuário).
        #[serde(default)]
        cabecalhos: Vec<(String, String)>,
        /// A extensão do formato (`m4a`, `mp3`, `webm`...), para o decodificador não ter de adivinhar.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        formato: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duracao_s: Option<f64>,
    },
    /// O provedor toca sozinho (uma WebView2 com o player do serviço, por exemplo); o Player captura o som do
    /// processo `pid` para o visual.
    TocaSozinho { pid: u32 },
}

/// Uma linha de resposta lida: a resposta inteira ou, se só o `id` deu para ler, o erro para entregar a quem
/// pediu. Sem nem o `id`, `None`.
pub fn ler_resposta(linha: &str) -> Option<Resposta> {
    let valor: serde_json::Value = serde_json::from_str(linha).ok()?;
    let id = valor.get("id")?.as_u64()?;
    let corpo = serde_json::from_value::<CorpoDaResposta>(valor)
        .unwrap_or_else(|e| CorpoDaResposta::erro(format!("resposta ilegível do provedor: {e}")));
    Some(Resposta { id, corpo })
}

/// Um pedido lido (do lado do provedor): o pedido, ou o `id` com o motivo de não o entender.
pub fn ler_pedido(linha: &str) -> Option<Result<Pedido, (u64, String)>> {
    let valor: serde_json::Value = serde_json::from_str(linha).ok()?;
    let id = valor.get("id")?.as_u64()?;
    let nome = valor.get("pedido").and_then(|p| p.as_str()).unwrap_or("?").to_string();
    Some(match serde_json::from_value::<CorpoDoPedido>(valor) {
        Ok(corpo) => Ok(Pedido { id, corpo }),
        Err(_) => Err((id, format!("pedido desconhecido ou incompleto: {nome}"))),
    })
}

/// Uma mensagem numa linha só (o JSON escapa as quebras de linha dos textos).
pub fn em_linha<T: Serialize>(mensagem: &T) -> String {
    serde_json::to_string(mensagem).unwrap_or_default()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use crate::modelo::Faixa;

    #[test]
    fn conta_no_fio() {
        let p: Pedido = serde_json::from_str(r#"{"id":3,"pedido":"entrar"}"#).expect("entrar");
        assert_eq!(p.corpo, CorpoDoPedido::Entrar);
        let sair: Pedido = serde_json::from_str(r#"{"id":6,"pedido":"sair"}"#).expect("sair");
        assert_eq!(sair.corpo, CorpoDoPedido::Sair);
        assert_eq!(em_linha(&Pedido { id: 7, corpo: CorpoDoPedido::Conta }), r#"{"id":7,"pedido":"conta"}"#);
        let r = Resposta {
            id: 4,
            corpo: CorpoDaResposta::Conta(Conta {
                situacao: SituacaoDaConta::Dentro,
                nome: Some("rafael".into()),
                premium: Some(true),
                ..Conta::default()
            }),
        };
        assert_eq!(
            serde_json::to_string(&r).expect("json"),
            r#"{"id":4,"resposta":"conta","situacao":"dentro","nome":"rafael","premium":true}"#
        );
        let fora: Resposta =
            serde_json::from_str(r#"{"id":5,"resposta":"conta","situacao":"fora","erro":"x"}"#).expect("fora");
        assert!(matches!(
            fora.corpo,
            CorpoDaResposta::Conta(Conta { situacao: SituacaoDaConta::Fora, erro: Some(_), .. })
        ));
    }

    #[test]
    fn o_tocador_no_fio_e_a_chave_fora_dos_textos() {
        let p: Pedido =
            serde_json::from_str(r#"{"id":1,"pedido":"controlar","acao":"ir","segundos":12.5}"#).expect("ir");
        assert_eq!(p.corpo, CorpoDoPedido::Controlar(Acao::Ir { segundos: 12.5 }));
        assert_eq!(
            em_linha(&Pedido { id: 2, corpo: CorpoDoPedido::Controlar(Acao::Tocar { faixa: "f".into() }) }),
            r#"{"id":2,"pedido":"controlar","acao":"tocar","faixa":"f"}"#
        );
        let ligar = CorpoDoPedido::LigarTocador { chave: Some(Segredo("abc123".into())) };
        assert_eq!(
            em_linha(&Pedido { id: 3, corpo: ligar.clone() }),
            r#"{"id":3,"pedido":"ligar-tocador","chave":"abc123"}"#
        );
        assert!(!format!("{ligar:?}").contains("abc123"), "{ligar:?}");
        let sem: Pedido = serde_json::from_str(r#"{"id":4,"pedido":"ligar-tocador"}"#).expect("sem chave");
        assert_eq!(sem.corpo, CorpoDoPedido::LigarTocador { chave: None });
        let estado = EstadoDoTocador {
            situacao: SituacaoDoTocador::Pronto,
            tocando: Some(Tocando {
                faixa: "f".into(),
                vez: 2,
                situacao: SituacaoDaFaixa::EmOutroLugar,
                ..Default::default()
            }),
            ..Default::default()
        };
        let r = Resposta { id: 5, corpo: CorpoDaResposta::Tocador(Box::new(estado)) };
        let linha = em_linha(&r);
        assert!(linha.contains(r#""situacao":"em-outro-lugar""#), "{linha}");
        assert_eq!(ler_resposta(&linha), Some(r));
        let antigo: EstadoDoTocador = serde_json::from_str(r#"{"situacao":"pareando","nome":"X"}"#).expect("pouco");
        assert_eq!((antigo.situacao, antigo.tem_chave), (SituacaoDoTocador::Pareando, false));
    }

    #[test]
    fn apresentacao_sem_conta_le_igual() {
        let a: Apresentacao = serde_json::from_str(r#"{"nome":"X","servico":"x","protocolo":1}"#).expect("antiga");
        assert_eq!(a.conta, None);
        let com: Apresentacao =
            serde_json::from_str(r#"{"nome":"S","servico":"spotify","protocolo":1,"conta":{"situacao":"entrando"}}"#)
                .expect("nova");
        assert_eq!(com.conta.map(|c| (c.situacao, c.navegador)), Some((SituacaoDaConta::Entrando, false)));
        let navegador: Conta =
            serde_json::from_str(r#"{"situacao":"entrando","navegador":true}"#).expect("com o navegador");
        assert!(navegador.navegador);
    }

    #[test]
    fn pedidos_no_formato_do_documento() {
        let ola = em_linha(&Pedido { id: 1, corpo: CorpoDoPedido::Ola { protocolo: 1 } });
        assert_eq!(ola, r#"{"id":1,"pedido":"ola","protocolo":1}"#);
        let inicio = em_linha(&Pedido { id: 2, corpo: CorpoDoPedido::Inicio });
        assert_eq!(inicio, r#"{"id":2,"pedido":"inicio"}"#);
        let biblioteca = em_linha(&Pedido { id: 4, corpo: CorpoDoPedido::Biblioteca });
        assert_eq!(biblioteca, r#"{"id":4,"pedido":"biblioteca"}"#);
        let lido = ler_pedido(r#"{"id":3,"pedido":"buscar","texto":"a\nb"}"#).expect("linha").expect("pedido");
        assert_eq!(lido.corpo, CorpoDoPedido::Buscar { texto: "a\nb".into(), limite: LIMITE_PADRAO });
        assert!(!em_linha(&lido).contains('\n'), "a quebra de linha vai escapada");
        assert_eq!(
            ler_pedido(r#"{"id":4,"pedido":"voar"}"#),
            Some(Err((4, "pedido desconhecido ou incompleto: voar".into())))
        );
        // Nenhum pedido repete a chave "id" (a do pedido): o provedor perderia a quem responder.
        for corpo in [
            CorpoDoPedido::Album { album: "a".into() },
            CorpoDoPedido::Artista { artista: "a".into() },
            CorpoDoPedido::Playlist { playlist: "a".into() },
            CorpoDoPedido::Resolver { faixa: "a".into() },
        ] {
            let linha = em_linha(&Pedido { id: 8, corpo: corpo.clone() });
            assert_eq!(linha.matches("\"id\"").count(), 1, "{linha}");
            assert_eq!(ler_pedido(&linha), Some(Ok(Pedido { id: 8, corpo })));
        }
        assert_eq!(ler_pedido("lixo"), None);
        assert_eq!(ler_pedido(r#"{"pedido":"inicio"}"#), None, "sem id não há a quem responder");
    }

    #[test]
    fn respostas_vao_e_voltam() {
        let faixa =
            Item::Faixa(Faixa { servico: "s".into(), id: "1".into(), titulo: "T".into(), ..Default::default() });
        for corpo in [
            CorpoDaResposta::Apresentacao(Apresentacao {
                nome: "Exemplo".into(),
                servico: "exemplo".into(),
                versao: "0.1.0".into(),
                protocolo: 1,
                capacidades: vec![Capacidade::Busca, Capacidade::Audio],
                identidade: None,
                conta: None,
            }),
            CorpoDaResposta::Itens { itens: vec![faixa.clone()] },
            CorpoDaResposta::Prateleiras { prateleiras: vec![Prateleira { titulo: "P".into(), itens: vec![faixa] }] },
            CorpoDaResposta::Fonte(Fonte::Endereco {
                url: "https://x/y".into(),
                cabecalhos: vec![("User-Agent".into(), "z".into())],
                formato: Some("m4a".into()),
                duracao_s: Some(12.5),
            }),
            CorpoDaResposta::Fonte(Fonte::TocaSozinho { pid: 42 }),
            CorpoDaResposta::erro("não deu"),
            CorpoDaResposta::Ok,
        ] {
            let r = Resposta { id: 9, corpo };
            let linha = em_linha(&r);
            assert_eq!(ler_resposta(&linha), Some(r), "{linha}");
        }
    }

    #[test]
    fn resposta_estragada_vira_erro_para_quem_pediu() {
        let r = ler_resposta(r#"{"id":5,"resposta":"itens","itens":"não é lista"}"#).expect("tem id");
        assert_eq!(r.id, 5);
        assert!(matches!(r.corpo, CorpoDaResposta::Erro { .. }));
        assert_eq!(ler_resposta(r#"{"resposta":"ok"}"#), None);
    }

    #[test]
    fn o_erro_com_causa_chega_e_o_sem_causa_tambem() {
        let r = ler_resposta(r#"{"id":1,"resposta":"erro","mensagem":"sem chave","causa":"chave-de-audio"}"#)
            .expect("lida");
        assert_eq!(r.corpo, CorpoDaResposta::Erro { mensagem: "sem chave".into(), causa: Some(Causa::ChaveDeAudio) });
        let antigo = ler_resposta(r#"{"id":2,"resposta":"erro","mensagem":"quebrou"}"#).expect("antiga");
        assert_eq!(antigo.corpo, CorpoDaResposta::Erro { mensagem: "quebrou".into(), causa: None });
        assert_eq!(em_linha(&CorpoDaResposta::erro("x")), r#"{"resposta":"erro","mensagem":"x"}"#);
        let com = CorpoDaResposta::erro_com(Causa::SemPremium, "precisa de Premium");
        assert_eq!(em_linha(&com), r#"{"resposta":"erro","mensagem":"precisa de Premium","causa":"sem-premium"}"#);
    }

    #[test]
    fn causa_que_este_player_nao_conhece_vira_erro_comum() {
        // Um provedor mais novo manda uma causa nova: a mensagem continua chegando, e o Player não quebra.
        let r = ler_resposta(r#"{"id":3,"resposta":"erro","mensagem":"o serviço mudou","causa":"teletransporte"}"#)
            .expect("lida");
        assert_eq!(r.corpo, CorpoDaResposta::Erro { mensagem: "o serviço mudou".into(), causa: None });
    }

    #[test]
    fn capacidade_nova_e_ignorada() {
        let r = ler_resposta(
            r#"{"id":1,"resposta":"apresentacao","nome":"N","servico":"s","protocolo":1,"capacidades":["busca","teletransporte"]}"#,
        )
        .expect("lida");
        let CorpoDaResposta::Apresentacao(a) = r.corpo else { unreachable!("{:?}", r.corpo) };
        assert_eq!(a.capacidades, vec![Capacidade::Busca]);
        assert!(a.sabe(Capacidade::Busca) && !a.sabe(Capacidade::Audio));
    }

    #[test]
    fn apresentacao_antiga_sem_identidade_e_nova_com() {
        let antiga = r#"{"nome":"X","servico":"x","protocolo":1}"#;
        let a: Apresentacao = serde_json::from_str(antiga).expect("lê");
        assert_eq!(a.identidade, None);
        let nova = r##"{"nome":"X","servico":"x","protocolo":1,"identidade":{"cor":"#1ed760","logo":"<svg viewBox='0 0 1 1'/>"}}"##;
        let a: Apresentacao = serde_json::from_str(nova).expect("lê");
        let i = a.identidade.expect("tem");
        assert_eq!(i.cor_rgb(), Some(0x1ED760));
        assert!(i.logo_valida().is_some());
    }

    #[test]
    fn cor_e_logo_invalidas_sao_ignoradas() {
        let i = |cor: &str, logo: &str| Identidade { cor: Some(cor.into()), logo: Some(logo.into()) };
        assert_eq!(i("verde", "").cor_rgb(), None);
        assert_eq!(i("#12345", "").cor_rgb(), None);
        assert_eq!(i("#GG0000", "").cor_rgb(), None);
        assert!(i("#000000", "<png/>").logo_valida().is_none());
        assert!(i("#000000", "<?xml version='1.0'?>\n<svg/>").logo_valida().is_some());
        let grande = format!("<svg>{}</svg>", "a".repeat(MAXIMO_DA_LOGO));
        assert!(i("#000000", &grande).logo_valida().is_none());
    }
}
