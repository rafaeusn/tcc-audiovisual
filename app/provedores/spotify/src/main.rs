//! Provedor do Spotify (F6): conversa com o Player pelo protocolo de provedores e com o Spotify pela sessão do
//! librespot, como o Spotifast (MIT) faz.
//!
//! Parte 1: o login. A pessoa aprova no navegador (o Client ID público do librespot, sem app de desenvolvedor); o
//! refresh token fica cifrado na pasta de dados do provedor, e nas próximas aberturas a sessão entra sozinha.
//!
//! Parte 2: os dados. A busca, o início, as páginas e a biblioteca vêm do Pathfinder (`pathfinder.rs`), com a sessão; a
//! conversão é pura (`conversao.rs`) e o `dados.rs` junta as páginas.
//!
//! Parte 4: tocar, pelo Soloist (`soloist/`), o player oficial do Spotify, se a pessoa o ligar. Sem ele, o Player toca a
//! mesma música por outro serviço (o Match).
//!
//! `provedor-spotify --teste-login` entra (abrindo o navegador, se preciso) e mostra a conta, sem o Player.

mod audio;
mod codigos;
mod conta;
mod conversao;
mod dados;
mod pathfinder;
mod porta;
mod soloist;
mod spotify;

use conta::Login;
use provedores::{
    Apresentacao, Capacidade, Causa, CorpoDaResposta, CorpoDoPedido, EmSegundoPlano, Fonte, Identidade, Servico,
    SituacaoDaConta, VERSAO_DO_PROTOCOLO, servir,
};
use std::path::PathBuf;
use std::sync::Arc;
/// A cor da marca.
const COR: &str = "#1DB954";

struct Spotify {
    login: Arc<Login>,
    dados: Arc<dados::Dados>,
    soloist: soloist::Soloist,
    /// O áudio do Spotify (librespot) para o motor do Player tocar (parte 3). Sem conta, ele não liga.
    audio: Arc<audio::Audio>,
}

/// A pasta que o Player dá a este provedor (sem o Player, uma só dele na pasta temporária do sistema, nunca a pasta
/// de onde foi aberto): a credencial e a pasta temporária do librespot.
fn pasta_de_dados() -> PathBuf {
    let pasta = std::env::var_os("TCCPLAYER_PROVEDOR_DADOS")
        .map_or_else(|| std::env::temp_dir().join("player-provedor-spotify"), PathBuf::from);
    let _ = std::fs::create_dir_all(&pasta);
    pasta
}

impl Servico for Spotify {
    fn apresentacao(&self) -> Apresentacao {
        Apresentacao {
            nome: "Spotify".into(),
            servico: "spotify".into(),
            versao: env!("CARGO_PKG_VERSION").into(),
            protocolo: VERSAO_DO_PROTOCOLO,
            capacidades: vec![
                Capacidade::Busca,
                Capacidade::Inicio,
                Capacidade::Album,
                Capacidade::Artista,
                Capacidade::Playlists,
                Capacidade::Conta,
                Capacidade::Biblioteca,
                // O áudio do Spotify no motor do Player, pelo librespot (só decifrado, só na memória).
                Capacidade::Audio,
                Capacidade::TocaSozinho,
            ],
            identidade: Some(Identidade { cor: Some(COR.into()), logo: Some(include_str!("logo.svg").into()) }),
            conta: Some(self.login.estado()),
        }
    }

    fn atender(&mut self, pedido: CorpoDoPedido) -> CorpoDaResposta {
        match pedido {
            CorpoDoPedido::Conta => CorpoDaResposta::Conta(self.login.estado()),
            CorpoDoPedido::Entrar => match self.login.entrar() {
                Ok(()) => CorpoDaResposta::Ok,
                Err(mensagem) => CorpoDaResposta::erro_com(Causa::SessaoRecusada, mensagem),
            },
            CorpoDoPedido::Sair => {
                self.login.sair();
                // Sem conta, o áudio some (a porta fecha e o cache é esquecido).
                self.audio.desligar();
                CorpoDaResposta::Ok
            }
            CorpoDoPedido::Tocador => CorpoDaResposta::Tocador(Box::new(self.soloist.estado())),
            CorpoDoPedido::LigarTocador { chave } => ok(self.soloist.ligar(chave.as_ref().map(|c| c.0.as_str()))),
            CorpoDoPedido::DesligarTocador => {
                self.soloist.desligar();
                CorpoDaResposta::Ok
            }
            CorpoDoPedido::Controlar(acao) => ok(self.soloist.controlar(acao)),
            // O áudio do Spotify no motor do Player, pelo librespot (só os bytes decifrados, na memória).
            CorpoDoPedido::Resolver { faixa } => self.resolver(&faixa),
            CorpoDoPedido::Buscar { texto, limite } => buscar(&self.login, &self.dados, &texto, limite),
            outro => pagina(&self.login, &self.dados, outro),
        }
    }

    fn em_segundo_plano(&mut self, pedido: CorpoDoPedido) -> Result<EmSegundoPlano, CorpoDoPedido> {
        let (login, dados) = (self.login.clone(), self.dados.clone());
        match pedido {
            CorpoDoPedido::Buscar { texto, limite } => Ok(Box::new(move || buscar(&login, &dados, &texto, limite))),
            CorpoDoPedido::Inicio
            | CorpoDoPedido::Album { .. }
            | CorpoDoPedido::Artista { .. }
            | CorpoDoPedido::Playlist { .. }
            | CorpoDoPedido::Biblioteca => Ok(Box::new(move || pagina(&login, &dados, pedido))),
            outro => Err(outro),
        }
    }

    fn encerrar(&mut self) {
        self.audio.desligar();
        self.soloist.fechar();
    }
}

impl Spotify {
    /// O endereço do áudio de uma faixa para o motor do Player. Só com a conta dentro: o librespot precisa da sessão, e
    /// a causa do erro (`provedores::Causa`) diz ao Player se vale trocar de jeito de tocar.
    fn resolver(&self, faixa: &str) -> CorpoDaResposta {
        if self.login.estado().situacao != SituacaoDaConta::Dentro {
            return CorpoDaResposta::erro_com(Causa::SemConta, "Entre na sua conta do Spotify.");
        }
        if !self.audio.ligado() {
            // A conta entrou, mas o servidor não subiu (uma porta, um temporário): tentar de novo resolve.
            return CorpoDaResposta::erro_com(Causa::Rede, "O áudio do Spotify ainda não abriu. Tente de novo.");
        }
        let valido = !faixa.is_empty() && faixa.len() <= 64 && faixa.bytes().all(|b| b.is_ascii_alphanumeric());
        if !valido {
            return CorpoDaResposta::erro_com(Causa::Indisponivel, "este id não é de uma faixa do Spotify");
        }
        match self.audio.endereco(faixa) {
            Ok((url, formato)) => CorpoDaResposta::Fonte(Fonte::Endereco {
                url,
                // O servidor é nosso: não há cabeçalho a mandar.
                cabecalhos: Vec::new(),
                formato,
                duracao_s: None,
            }),
            // A causa vem do librespot (sem conta, sem Premium, chave recusada, fora do catálogo): é ela que faz o
            // Player trocar de jeito de tocar em vez de tentar esta faixa de novo à toa.
            Err(falha) => CorpoDaResposta::erro_com(falha.causa, falha.mensagem),
        }
    }
}

fn ok(feito: Result<(), String>) -> CorpoDaResposta {
    feito.map_or_else(erro, |()| CorpoDaResposta::Ok)
}

fn erro(mensagem: String) -> CorpoDaResposta {
    CorpoDaResposta::erro(mensagem)
}

fn itens(resultado: Result<Vec<provedores::Item>, String>) -> CorpoDaResposta {
    resultado.map_or_else(erro, |itens| CorpoDaResposta::Itens { itens })
}

fn buscar(login: &Login, dados: &dados::Dados, texto: &str, limite: u32) -> CorpoDaResposta {
    // Fora da conta, o convite para entrar já está na tela: um erro na busca só atrapalharia.
    if login.estado().situacao != SituacaoDaConta::Dentro {
        return CorpoDaResposta::Itens { itens: Vec::new() };
    }
    itens(dados.buscar(texto, limite))
}

fn pagina(login: &Login, dados: &dados::Dados, pedido: CorpoDoPedido) -> CorpoDaResposta {
    match login.estado().situacao {
        SituacaoDaConta::Dentro => {}
        SituacaoDaConta::Entrando => return erro("O Spotify ainda está entrando na conta.".into()),
        SituacaoDaConta::Fora => return erro("Entre na sua conta do Spotify.".into()),
    }
    match pedido {
        CorpoDoPedido::Inicio => {
            dados.inicio().map_or_else(erro, |prateleiras| CorpoDaResposta::Prateleiras { prateleiras })
        }
        CorpoDoPedido::Album { album } => itens(dados.album(&album)),
        CorpoDoPedido::Artista { artista } => itens(dados.artista(&artista)),
        CorpoDoPedido::Playlist { playlist } => itens(dados.playlist(&playlist)),
        CorpoDoPedido::Biblioteca => itens(dados.biblioteca()),
        outro => erro(format!("o provedor do Spotify não sabe responder {outro:?}")),
    }
}

/// O modo de teste: entra (pelo navegador, se não houver login guardado) e mostra a conta.
fn testar_login(login: &Login, autorizador: &spotify::Real) {
    use conta::Autorizador;
    println!("porta do login: {}", autorizador.porta_ocupada().unwrap_or_else(|| "livre".into()));
    let espera_ate = std::time::Instant::now() + spotify::PRAZO_DO_NAVEGADOR + std::time::Duration::from_secs(40);
    let esperar = || {
        while login.estado().situacao == SituacaoDaConta::Entrando && std::time::Instant::now() < espera_ate {
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    };
    esperar();
    if login.estado().situacao == SituacaoDaConta::Fora {
        println!("entrar: {:?}", login.entrar());
        esperar();
    }
    println!("conta: {:?}", login.estado());
}

/// Liga o registro do `librespot` (ele escreve o motivo de uma recusa no `RUST_LOG`). Sem isto, um "chave-de-audio" não
/// diz **por que** o Spotify recusou, e o conserto vira adivinhação.
///
/// Só quando a pessoa pede (`RUST_LOG` definido): o provedor não enche o registro do Player sozinho.
fn ligar_registro_do_librespot() {
    if std::env::var_os("RUST_LOG").is_none() {
        return;
    }
    // O `env_logger` não vem com o Player; um inicializador simples basta (escreve no stderr, que o Player anota).
    struct Registro;
    impl log::Log for Registro {
        fn enabled(&self, _: &log::Metadata) -> bool {
            true
        }
        fn log(&self, r: &log::Record) {
            if self.enabled(r.metadata()) {
                eprintln!("[{}] {}: {}", r.level(), r.target(), r.args());
            }
        }
        fn flush(&self) {}
    }
    static REGISTRO: Registro = Registro;
    let _ = log::set_logger(&REGISTRO).map(|()| log::set_max_level(log::LevelFilter::Debug));
}

fn main() {
    ligar_registro_do_librespot();
    let pasta = pasta_de_dados();
    let real = match spotify::Real::novo(&pasta) {
        Ok(r) => std::sync::Arc::new(r),
        Err(e) => {
            eprintln!("o provedor do Spotify não abriu: {e}");
            std::process::exit(1);
        }
    };
    let login = Arc::new(Login::novo(real.clone(), pasta.join("conta.dpapi")));
    if std::env::args().any(|a| a == "--teste-login") {
        testar_login(&login, &real);
        login.encerrar();
        return;
    }
    let codigos = codigos::Codigos::novo(pasta.join("codigos.json"));
    let dados = match pathfinder::Pathfinder::novo(pathfinder::Enderecos::default(), codigos) {
        Ok(p) => dados::Dados::novo(Box::new(spotify::PelasConsultas::novo(real.clone(), p)), login.clone()),
        Err(e) => {
            eprintln!("o provedor do Spotify não abriu: {e}");
            std::process::exit(1);
        }
    };
    // O áudio (librespot): o servidor local sobe quando a sessão entra, e desce quando a conta sai.
    let audio = Arc::new(audio::Audio::novo(real.runtime(), pasta.clone()));
    real.ao_entrar({
        let audio = Arc::clone(&audio);
        let real = Arc::clone(&real);
        move |_| {
            if let Err(e) = audio.ligar(real.clone()) {
                eprintln!("spotify: o áudio do Spotify não abriu: {e}");
            }
        }
    });
    let soloist = soloist::Soloist::novo(Arc::new(soloist::DoSpotify::novo(pasta.join("soloist"))), pasta);
    let mut spotify = Spotify { login, dados: Arc::new(dados), soloist, audio };
    // A saída sem `lock()`: cada resposta trava só enquanto é escrita. O `librespot-oauth` escreve o endereço do
    // login na saída padrão, de outra thread, antes de escutar na porta; com a saída travada o tempo todo, ele
    // esperava para sempre e o navegador voltava para uma porta fechada.
    if let Err(e) = servir(std::io::stdin().lock(), std::io::stdout(), &mut spotify) {
        eprintln!("spotify: {e}");
    }
    spotify.login.encerrar();
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic)]
mod testes {
    use super::*;
    use conta::falso::{Falso, cofre};

    /// O provedor com um Spotify de mentira, que responde uma faixa a qualquer playlist.
    fn spotify_de_teste(teste: &str) -> Spotify {
        let login = Arc::new(Login::novo(Arc::new(Falso::default()), cofre(teste)));
        let fonte = dados::falsas::Falsas {
            responder: Box::new(|_, _| Err(pathfinder::Falha::Rede("sem conexão".into()))),
            pedidos: Arc::default(),
        };
        let pasta = std::env::temp_dir().join(format!("tccplayer-spotify-{teste}-{}", std::process::id()));
        let soloist = soloist::Soloist::novo(Arc::new(soloist::DoSpotify::novo(pasta.join("soloist"))), pasta.clone());
        let audio = Arc::new(audio::Audio::novo(
            Arc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().expect("rt")),
            pasta,
        ));
        Spotify { dados: Arc::new(dados::Dados::novo(Box::new(fonte), login.clone())), login, soloist, audio }
    }

    #[test]
    fn fora_da_conta_a_busca_vem_vazia_e_a_pagina_diz_para_entrar() {
        let mut s = spotify_de_teste("fora");
        let buscar = CorpoDoPedido::Buscar { texto: "x".into(), limite: 20 };
        assert_eq!(
            s.atender(buscar.clone()),
            CorpoDaResposta::Itens { itens: Vec::new() },
            "o convite já está na tela"
        );
        let CorpoDaResposta::Erro { mensagem, .. } = s.atender(CorpoDoPedido::Inicio) else { panic!("devia ser erro") };
        assert!(mensagem.contains("Entre na sua conta"), "{mensagem}");
        s.login.entrar().expect("entrar");
        assert!(conta::falso::esperar(|| s.login.estado().situacao == SituacaoDaConta::Dentro));
        assert!(
            matches!(s.atender(buscar), CorpoDaResposta::Erro { mensagem, .. } if mensagem.contains("sem conexão"))
        );
    }

    #[test]
    fn sem_conta_o_resolver_diz_para_entrar_e_o_tocador_segue_desligado() {
        let mut s = spotify_de_teste("tocar");
        let resolver = s.atender(CorpoDoPedido::Resolver { faixa: "f".into() });
        assert!(
            matches!(resolver, CorpoDaResposta::Erro { causa: Some(Causa::SemConta), .. }),
            "sem conta não há endereço, e o Player sabe por quê"
        );
        let CorpoDaResposta::Tocador(e) = s.atender(CorpoDoPedido::Tocador) else { panic!("o estado") };
        assert_eq!(*e, provedores::EstadoDoTocador::default());
        let tocar = CorpoDoPedido::Controlar(provedores::Acao::Tocar { faixa: "f".into() });
        assert!(matches!(s.atender(tocar), CorpoDaResposta::Erro { mensagem, .. } if mensagem.contains("desligado")));
        let ligar = CorpoDoPedido::LigarTocador { chave: None };
        assert!(matches!(s.atender(ligar), CorpoDaResposta::Erro { mensagem, .. } if mensagem.contains("chave")));
    }

    #[test]
    fn dentro_da_conta_com_audio_ligado_a_faixa_sem_sessao_diz_o_porque() {
        let mut s = spotify_de_teste("audio");
        s.login.entrar().expect("entrar");
        assert!(conta::falso::esperar(|| s.login.estado().situacao == SituacaoDaConta::Dentro));
        // O áudio sobe com a sessão (o falso de teste não avisa): liga aqui, como o `Real` faria.
        s.audio.ligar(Arc::new(audio::FonteNenhuma)).expect("liga");
        // Sem uma sessão de verdade (a do teste é falsa), a faixa não abre — e o erro sai com a causa, em vez de um
        // endereço local que só daria 404 no primeiro pedaço. Era esse o defeito que o teste de ponta a ponta achou.
        let resposta = s.atender(CorpoDoPedido::Resolver { faixa: "4uLU6hMCjMI75M1A2tKUQC".into() });
        assert!(matches!(resposta, CorpoDaResposta::Erro { causa: Some(Causa::SemConta), .. }), "{resposta:?}");
    }

    #[test]
    fn o_audio_desligado_nao_da_endereco() {
        let mut s = spotify_de_teste("audio-desligado");
        s.login.entrar().expect("entrar");
        assert!(conta::falso::esperar(|| s.login.estado().situacao == SituacaoDaConta::Dentro));
        let resposta = s.atender(CorpoDoPedido::Resolver { faixa: "4uLU6hMCjMI75M1A2tKUQC".into() });
        assert!(
            matches!(resposta, CorpoDaResposta::Erro { causa: Some(Causa::Rede), .. }),
            "sem o servidor no ar, o Player deve tentar de novo: {resposta:?}"
        );
    }

    #[test]
    fn as_paginas_rodam_em_segundo_plano_e_o_tocador_nao() {
        let mut s = spotify_de_teste("plano");
        assert!(s.em_segundo_plano(CorpoDoPedido::Inicio).is_ok());
        assert!(s.em_segundo_plano(CorpoDoPedido::Buscar { texto: "x".into(), limite: 5 }).is_ok());
        assert!(s.em_segundo_plano(CorpoDoPedido::Tocador).is_err());
        assert!(s.em_segundo_plano(CorpoDoPedido::Sair).is_err());
    }

    #[test]
    fn servir_responde_a_conta() {
        let mut s = spotify_de_teste("servir");
        let entrada = "{\"id\":1,\"pedido\":\"ola\",\"protocolo\":1}\n{\"id\":2,\"pedido\":\"conta\"}\n";
        let mut saida = Vec::new();
        servir(entrada.as_bytes(), &mut saida, &mut s).expect("servir");
        let texto = String::from_utf8(saida).expect("utf-8");
        let linhas: Vec<&str> = texto.lines().collect();
        assert!(linhas[0].contains(r#""busca","inicio","album","artista","playlists","conta""#), "{}", linhas[0]);
        assert!(linhas[0].contains(r##""cor":"#1DB954""##), "{}", linhas[0]);
        assert_eq!(linhas[1], r#"{"id":2,"resposta":"conta","situacao":"fora"}"#);
    }
}
