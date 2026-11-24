//! Provedores de música (F1 do plano v2): cada serviço (YouTube Music, Spotify, SoundCloud...) é um programa à
//! parte que conversa com o Player pelo [`protocolo`]. Aqui ficam o formato comum das faixas ([`modelo`]), o
//! protocolo, o lado do Player ([`cliente`]), o lado do provedor ([`servidor`]), os provedores instalados
//! ([`registro`]) e o [`casador`], que acha a mesma gravação em outro serviço. Descrição para quem escreve um
//! provedor: `docs/f1-provedores.md`.

pub mod casador;
pub mod cliente;
pub mod modelo;
pub mod protocolo;
pub mod registro;
pub mod servidor;

pub use casador::{LIMIAR, melhor_par, nota};
pub use cliente::Provedor;
pub use modelo::{Album, Artista, Capacidade, Faixa, Item, Playlist, Prateleira};
pub use protocolo::{
    Acao, Apresentacao, Conta, CorpoDaResposta, CorpoDoPedido, EstadoDoTocador, Fonte, Identidade, MAXIMO_DA_LOGO,
    Segredo, SituacaoDaConta, SituacaoDaFaixa, SituacaoDoTocador, Tocando, VERSAO_DO_PROTOCOLO,
};
pub use registro::{Instalado, Manifesto, instalados, instalar, remover, restaurar};
pub use servidor::{EmSegundoPlano, Servico, servir};
