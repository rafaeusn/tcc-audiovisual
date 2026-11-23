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
    Ola { protocolo: u32 },
    Buscar {
        texto: String,
        #[serde(default = "limite_padrao")]
        limite: u32,
    },
    /// As prateleiras da página inicial do serviço.
    Inicio,
    // Os campos não se chamam `id`: esse nome é o do pedido, e o JSON ficaria com dois.
    /// As faixas de um álbum.
    Album { album: String },
    /// Os álbuns e as faixas mais tocadas de um artista.
    Artista { artista: String },
    /// As faixas de uma playlist.
    Playlist { playlist: String },
    /// Como tocar uma faixa.
    Resolver { faixa: String },
    /// O Player vai fechar: o provedor responde `ok` e sai.
    Tchau,
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
    Itens { itens: Vec<Item> },
    Prateleiras { prateleiras: Vec<Prateleira> },
    Fonte(Fonte),
    Erro { mensagem: String },
    Ok,
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
        .unwrap_or_else(|e| CorpoDaResposta::Erro { mensagem: format!("resposta ilegível do provedor: {e}") });
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
    fn pedidos_no_formato_do_documento() {
        let ola = em_linha(&Pedido { id: 1, corpo: CorpoDoPedido::Ola { protocolo: 1 } });
        assert_eq!(ola, r#"{"id":1,"pedido":"ola","protocolo":1}"#);
        let inicio = em_linha(&Pedido { id: 2, corpo: CorpoDoPedido::Inicio });
        assert_eq!(inicio, r#"{"id":2,"pedido":"inicio"}"#);
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
            CorpoDaResposta::Erro { mensagem: "não deu".into() },
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
    fn capacidade_nova_e_ignorada() {
        let r = ler_resposta(
            r#"{"id":1,"resposta":"apresentacao","nome":"N","servico":"s","protocolo":1,"capacidades":["busca","teletransporte"]}"#,
        )
        .expect("lida");
        let CorpoDaResposta::Apresentacao(a) = r.corpo else { unreachable!("{:?}", r.corpo) };
        assert_eq!(a.capacidades, vec![Capacidade::Busca]);
        assert!(a.sabe(Capacidade::Busca) && !a.sabe(Capacidade::Audio));
    }
}
