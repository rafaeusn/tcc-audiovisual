//! O menu do botão direito e do ⋯: o que aparece para cada coisa e onde ele abre. Só lógica, testada; a caixa é
//! `ui::menu` e as ações são executadas em `app::menu`.

use crate::paginas::cartao;
use biblioteca::{Musica, TipoSalvo};
use nucleo::{Comando, Modelo};
use provedores::{Faixa, Item};
use std::sync::Arc;

/// Em que o menu foi aberto.
#[derive(Clone, Debug, PartialEq)]
pub enum Alvo {
    /// Uma faixa de serviço: as outras versões dela (os outros selos), a fileira em que está e a posição na fila.
    Faixa {
        faixa: Box<Faixa>,
        tambem: Vec<Faixa>,
        fileira: Arc<Vec<Faixa>>,
        fila: Option<usize>,
    },
    /// Um álbum, uma playlist ou um artista.
    Item(Box<Item>),
    /// Uma música do computador.
    Local {
        musica: Box<Musica>,
        fila: Option<usize>,
    },
    /// O ⋯ da barra de baixo.
    Barra,
    Visual,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Acao {
    Tocar,
    Aleatorio,
    TocarASeguir,
    PorNaFila,
    PorNaPlaylist,
    /// Curtir ou salvar (ou tirar).
    Salvar,
    Fixar(bool),
    Abrir,
    VerFaixa,
    IrAoAlbum,
    /// O artista desta posição.
    IrAoArtista(usize),
    /// Toca a versão do outro serviço (a posição em `tambem`).
    TocarPeloOutro(usize),
    /// Busca o nome no outro serviço.
    ProcurarNo(String),
    ProcurarNosServicos,
    /// As versões juntadas não são a mesma música.
    Separar,
    /// O Match toca por uma gravação errada: procurar outra.
    OutraGravacao,
    Subir,
    Descer,
    TirarDaFila,
    Comando(Comando),
    /// A saída de som, nas configurações do Windows.
    SaidaDeSom,
}

/// Uma linha do menu.
#[derive(Clone, Debug, PartialEq)]
pub struct Entrada {
    pub rotulo: String,
    pub icone: &'static str,
    pub acao: Acao,
    /// Muda de grupo: um traço separa.
    pub grupo: u8,
    /// O que está valendo agora (o visual escolhido, a Ilha ligada).
    pub marcada: bool,
}

/// O que o menu precisa saber do app.
#[derive(Clone, Debug, Default)]
pub struct Contexto {
    /// Alguém toca as faixas do serviço.
    pub toca: bool,
    pub sabe_album: bool,
    pub sabe_artista: bool,
    pub salvo: bool,
    pub fixado: bool,
    /// Os outros serviços em que dá para buscar (serviço e nome).
    pub outros: Vec<(String, String)>,
    /// Há algum serviço aberto para buscar.
    pub tem_servicos: bool,
    /// A faixa toca por um par (o Match).
    pub tem_par: bool,
    /// Quantas músicas há na fila.
    pub na_fila: usize,
    /// É a música que está tocando: já toca, e não muda de lugar na fila.
    pub e_a_de_agora: bool,
    /// A faixa de serviço que toca está curtida (vazio: o que toca não dá para curtir).
    pub curtida: Option<bool>,
    /// O nome de cada serviço de `tambem`, na mesma ordem.
    pub nomes_dos_outros: Vec<String>,
    pub modelo: Modelo,
    pub cores_reativas: bool,
    pub modo_seguro: bool,
    pub mudo: bool,
    pub ilha: bool,
    /// O layout Estante (o visual expande) e a janela estreita (a barra tem menos botões).
    pub estante: bool,
    pub estreita: bool,
    /// A página da Estante está larga (vazio: ela já ocupa a largura toda).
    pub pagina_larga: Option<bool>,
}

fn entrada(grupo: u8, icone: &'static str, rotulo: impl Into<String>, acao: Acao) -> Entrada {
    Entrada { rotulo: rotulo.into(), icone, acao, grupo, marcada: false }
}

/// As ações de quem está na fila: andar e sair.
fn da_fila(lista: &mut Vec<Entrada>, i: usize, cx: &Contexto) {
    if cx.e_a_de_agora {
        return;
    }
    if i > 0 {
        lista.push(entrada(1, "icones/chev-u.svg", "Subir na fila", Acao::Subir));
    }
    if i + 1 < cx.na_fila {
        lista.push(entrada(1, "icones/chev-d.svg", "Descer na fila", Acao::Descer));
    }
    lista.push(entrada(1, "icones/x.svg", "Tirar da fila", Acao::TirarDaFila));
}

/// O que o menu mostra: só o que vale para o alvo.
pub fn entradas(alvo: &Alvo, cx: &Contexto) -> Vec<Entrada> {
    let mut v = Vec::new();
    match alvo {
        Alvo::Faixa { faixa, tambem, fila, .. } => {
            if cx.toca {
                if !cx.e_a_de_agora {
                    v.push(entrada(0, "icones/play.svg", "Tocar", Acao::Tocar));
                }
                if fila.is_none() {
                    v.push(entrada(0, "icones/play-next.svg", "Tocar em seguida", Acao::TocarASeguir));
                    v.push(entrada(0, "icones/queue-add.svg", "Pôr no fim da fila", Acao::PorNaFila));
                }
            }
            if let Some(i) = fila {
                da_fila(&mut v, *i, cx);
            }
            v.push(entrada(2, "icones/playlist.svg", "Pôr numa playlist", Acao::PorNaPlaylist));
            let (icone, rotulo) =
                if cx.salvo { ("icones/heart-f.svg", "Tirar das curtidas") } else { ("icones/heart.svg", "Curtir") };
            v.push(entrada(2, icone, rotulo, Acao::Salvar));
            v.push(entrada(3, "icones/info.svg", "Ver a faixa", Acao::VerFaixa));
            if cx.sabe_album && faixa.album_id.as_deref().is_some_and(|id| !id.is_empty()) {
                v.push(entrada(3, "icones/album.svg", "Ir ao álbum", Acao::IrAoAlbum));
            }
            if cx.sabe_artista {
                let com_id: Vec<usize> =
                    (0..faixa.artistas.len()).filter(|i| faixa.id_do_artista(*i).is_some()).collect();
                let um_so = com_id.len() == 1;
                for i in com_id.into_iter().take(3) {
                    let rotulo =
                        if um_so { "Ir ao artista".to_string() } else { format!("Ir a {}", faixa.artistas[i]) };
                    v.push(entrada(3, "icones/person.svg", rotulo, Acao::IrAoArtista(i)));
                }
            }
            for (i, nome) in cx.nomes_dos_outros.iter().enumerate().take(tambem.len()) {
                v.push(entrada(4, "icones/swap.svg", format!("Tocar pelo {nome}"), Acao::TocarPeloOutro(i)));
            }
            for (servico, nome) in cx.outros.iter().filter(|(s, _)| !tambem.iter().any(|f| &f.servico == s)) {
                v.push(entrada(
                    4,
                    "icones/search.svg",
                    format!("Procurar no {nome}"),
                    Acao::ProcurarNo(servico.clone()),
                ));
            }
            if !tambem.is_empty() {
                v.push(entrada(4, "icones/struct.svg", "Não são a mesma música", Acao::Separar));
            }
            if cx.tem_par {
                v.push(entrada(4, "icones/swap.svg", "Procurar outra gravação", Acao::OutraGravacao));
            }
        }
        Alvo::Item(item) => {
            let tipo = cartao(item).tipo;
            v.push(entrada(0, "icones/expand.svg", "Abrir", Acao::Abrir));
            if cx.toca {
                v.push(entrada(0, "icones/play.svg", "Tocar", Acao::Tocar));
                v.push(entrada(0, "icones/shuffle.svg", "Tocar em ordem aleatória", Acao::Aleatorio));
                v.push(entrada(0, "icones/queue-add.svg", "Pôr no fim da fila", Acao::PorNaFila));
            }
            let (icone, rotulo) = match (cx.salvo, tipo) {
                (true, _) => ("icones/heart-f.svg", "Tirar da biblioteca"),
                (false, TipoSalvo::Artista) => ("icones/heart.svg", "Seguir na biblioteca"),
                (false, _) => ("icones/heart.svg", "Salvar na biblioteca"),
            };
            v.push(entrada(1, icone, rotulo, Acao::Salvar));
            let rotulo = if cx.fixado { "Soltar da lateral" } else { "Fixar na lateral" };
            v.push(entrada(1, "icones/pin.svg", rotulo, Acao::Fixar(!cx.fixado)));
            // Uma playlist de um serviço não existe no outro.
            if tipo != TipoSalvo::Playlist {
                for (servico, nome) in &cx.outros {
                    v.push(entrada(
                        2,
                        "icones/search.svg",
                        format!("Procurar no {nome}"),
                        Acao::ProcurarNo(servico.clone()),
                    ));
                }
            }
        }
        Alvo::Local { musica, fila } => {
            if !cx.e_a_de_agora {
                v.push(entrada(0, "icones/play.svg", "Tocar", Acao::Tocar));
            }
            if fila.is_none() {
                v.push(entrada(0, "icones/play-next.svg", "Tocar em seguida", Acao::TocarASeguir));
                v.push(entrada(0, "icones/queue-add.svg", "Pôr no fim da fila", Acao::PorNaFila));
            }
            if let Some(i) = fila {
                da_fila(&mut v, *i, cx);
            }
            v.push(entrada(2, "icones/playlist.svg", "Pôr numa playlist", Acao::PorNaPlaylist));
            if !musica.album.is_empty() {
                v.push(entrada(3, "icones/album.svg", "Ir ao álbum", Acao::IrAoAlbum));
            }
            if !musica.artista.is_empty() {
                v.push(entrada(3, "icones/person.svg", "Ir ao artista", Acao::IrAoArtista(0)));
            }
            if cx.tem_servicos {
                v.push(entrada(4, "icones/search.svg", "Procurar nos serviços", Acao::ProcurarNosServicos));
            }
        }
        Alvo::Barra => {
            let comando = |grupo, icone, rotulo: &str, c| entrada(grupo, icone, rotulo, Acao::Comando(c));
            if cx.estreita {
                v.push(comando(0, "icones/queue.svg", "Fila", Comando::AbrirFila));
                v.push(comando(0, "icones/lyrics.svg", "Letra", Comando::AbrirLetra));
                let mut mudo = comando(0, "icones/mute.svg", "Sem som", Comando::Silenciar);
                mudo.marcada = cx.mudo;
                v.push(mudo);
                if let Some(curtida) = cx.curtida {
                    let (icone, rotulo) = if curtida {
                        ("icones/heart-f.svg", "Tirar das curtidas")
                    } else {
                        ("icones/heart.svg", "Curtir")
                    };
                    v.push(comando(0, icone, rotulo, Comando::CurtirFaixa));
                }
            }
            v.push(comando(1, "icones/full.svg", "Tela cheia", Comando::TelaCheia));
            v.push(comando(1, "icones/lyrics.svg", "Letra em tela cheia", Comando::LetraTelaCheia));
            v.push(comando(1, "icones/pip.svg", "Janela mini", Comando::AlternarMini));
            let mut ilha = comando(1, "icones/ilha.svg", "Ilha no topo da tela", Comando::AlternarIlha);
            ilha.marcada = cx.ilha;
            v.push(ilha);
            v.push(entrada(2, "icones/vol.svg", "Saída de som do Windows", Acao::SaidaDeSom));
        }
        Alvo::Visual => {
            for m in [Modelo::Spectral, Modelo::Fluxo, Modelo::Barras] {
                let mut e = entrada(0, "icones/eye.svg", m.nome(), Acao::Comando(Comando::TrocarVisual(m)));
                e.marcada = m == cx.modelo;
                v.push(e);
            }
            let mut cores =
                entrada(1, "icones/drop.svg", "Cores reativas", Acao::Comando(Comando::AlternarCoresReativas));
            cores.marcada = cx.cores_reativas;
            let mut seguro = entrada(1, "icones/shield.svg", "Modo seguro", Acao::Comando(Comando::AlternarModoSeguro));
            seguro.marcada = cx.modo_seguro;
            v.extend([cores, seguro]);
            if cx.estante {
                v.push(entrada(2, "icones/expand.svg", "Só o visual", Acao::Comando(Comando::ExpandirVisual)));
            }
            if let Some(larga) = cx.pagina_larga {
                let mut e = entrada(2, "icones/larga.svg", "Página larga", Acao::Comando(Comando::AlternarPaginaLarga));
                e.marcada = larga;
                v.push(e);
            }
            v.push(entrada(2, "icones/full.svg", "Tela cheia", Acao::Comando(Comando::TelaCheia)));
            v.push(entrada(2, "icones/pip.svg", "Janela mini", Acao::Comando(Comando::AlternarMini)));
        }
    }
    v
}

/// A distância do menu até a borda da janela.
pub const MARGEM: f32 = 8.0;

/// Onde o menu fica: no ponto do clique, virando para o outro lado quando não cabe, sempre dentro da janela.
pub fn lugar(ponto: (f32, f32), menu: (f32, f32), janela: (f32, f32)) -> (f32, f32) {
    let um = |p: f32, tamanho: f32, total: f32| {
        let maximo = (total - tamanho - MARGEM).max(MARGEM);
        if p + tamanho + MARGEM <= total {
            p.max(MARGEM)
        } else if p - tamanho >= MARGEM {
            // Para o outro lado do ponto.
            p - tamanho
        } else {
            maximo
        }
    };
    (um(ponto.0, menu.0, janela.0), um(ponto.1, menu.1, janela.1))
}

#[cfg(test)]
mod testes {
    use super::*;
    use provedores::{Album, Artista, Playlist};

    fn faixa() -> Faixa {
        Faixa {
            servico: "yt".into(),
            id: "1".into(),
            titulo: "Corrente".into(),
            artistas: vec!["Lume".into(), "Telha".into()],
            artistas_ids: vec!["l".into(), "t".into()],
            album_id: Some("al".into()),
            ..Default::default()
        }
    }

    fn alvo(faixa: Faixa, tambem: Vec<Faixa>, fila: Option<usize>) -> Alvo {
        Alvo::Faixa { faixa: Box::new(faixa), tambem, fileira: Arc::default(), fila }
    }

    fn tudo() -> Contexto {
        Contexto {
            toca: true,
            sabe_album: true,
            sabe_artista: true,
            outros: vec![("sp".into(), "Spotify".into())],
            tem_servicos: true,
            na_fila: 3,
            ..Default::default()
        }
    }

    fn acoes(alvo: &Alvo, cx: &Contexto) -> Vec<Acao> {
        entradas(alvo, cx).into_iter().map(|e| e.acao).collect()
    }

    #[test]
    fn a_de_agora_nao_toca_de_novo_nem_mexe_na_fila() {
        let cx = Contexto { e_a_de_agora: true, ..tudo() };
        let lista = acoes(&alvo(faixa(), vec![], Some(1)), &cx);
        for a in [Acao::Tocar, Acao::Subir, Acao::Descer, Acao::TirarDaFila] {
            assert!(!lista.contains(&a), "{a:?}");
        }
        assert!(lista.contains(&Acao::PorNaPlaylist) && lista.contains(&Acao::IrAoAlbum));
    }

    #[test]
    fn a_barra_estreita_traz_o_que_nao_cabe_nela() {
        let cx = Contexto { estreita: true, curtida: Some(false), ..tudo() };
        let lista = acoes(&Alvo::Barra, &cx);
        for c in [Comando::AbrirFila, Comando::AbrirLetra, Comando::CurtirFaixa] {
            assert!(lista.contains(&Acao::Comando(c)), "{c:?}");
        }
        let larga = acoes(&Alvo::Barra, &Contexto { estreita: false, ..cx });
        assert!(!larga.contains(&Acao::Comando(Comando::CurtirFaixa)));
    }

    #[test]
    fn o_menu_da_faixa_de_servico() {
        use Acao as A;
        assert_eq!(
            acoes(&alvo(faixa(), vec![], None), &tudo()),
            [
                A::Tocar,
                A::TocarASeguir,
                A::PorNaFila,
                A::PorNaPlaylist,
                A::Salvar,
                A::VerFaixa,
                A::IrAoAlbum,
                A::IrAoArtista(0),
                A::IrAoArtista(1),
                A::ProcurarNo("sp".into()),
            ]
        );
        // Sem ids, sem quem toque e sem outro serviço: só o que vale.
        let pouca = Faixa { album_id: None, artistas_ids: vec![], ..faixa() };
        assert_eq!(acoes(&alvo(pouca, vec![], None), &Contexto::default()), [A::PorNaPlaylist, A::Salvar, A::VerFaixa]);
        // O provedor não abre álbum nem artista.
        let sem = Contexto { sabe_album: false, sabe_artista: false, ..tudo() };
        assert!(!acoes(&alvo(faixa(), vec![], None), &sem).contains(&A::IrAoAlbum));
        // Um artista só: sem o nome no rótulo.
        let um = Faixa { artistas: vec!["Lume".into()], artistas_ids: vec!["l".into()], ..faixa() };
        let e = entradas(&alvo(um, vec![], None), &tudo());
        assert!(e.iter().any(|e| e.rotulo == "Ir ao artista"));
        let curtida = Contexto { salvo: true, ..tudo() };
        assert!(entradas(&alvo(faixa(), vec![], None), &curtida).iter().any(|e| e.rotulo == "Tirar das curtidas"));
    }

    #[test]
    fn a_faixa_em_dois_servicos_e_a_do_match() {
        use Acao as A;
        let cx = Contexto { nomes_dos_outros: vec!["Spotify".into()], ..tudo() };
        let outra = Faixa { servico: "sp".into(), id: "9".into(), ..faixa() };
        let a = acoes(&alvo(faixa(), vec![outra], None), &cx);
        assert!(a.contains(&A::TocarPeloOutro(0)) && a.contains(&A::Separar));
        assert!(!a.contains(&A::ProcurarNo("sp".into())), "já está nele");
        let par = Contexto { tem_par: true, ..tudo() };
        assert!(acoes(&alvo(faixa(), vec![], None), &par).contains(&A::OutraGravacao));
    }

    #[test]
    fn na_fila_o_menu_anda_e_tira() {
        use Acao as A;
        let na = |i| acoes(&alvo(faixa(), vec![], Some(i)), &tudo());
        assert!(na(0).contains(&A::Descer) && !na(0).contains(&A::Subir), "a primeira não sobe");
        assert!(na(2).contains(&A::Subir) && !na(2).contains(&A::Descer), "a última não desce");
        assert!(na(1).contains(&A::TirarDaFila) && !na(1).contains(&A::TocarASeguir));
        let local = Alvo::Local { musica: Box::default(), fila: Some(1) };
        assert!(acoes(&local, &tudo()).contains(&A::TirarDaFila));
    }

    #[test]
    fn o_menu_de_album_playlist_e_artista() {
        use Acao as A;
        let al = Alvo::Item(Box::new(Item::Album(Album::default())));
        assert_eq!(
            acoes(&al, &tudo()),
            [A::Abrir, A::Tocar, A::Aleatorio, A::PorNaFila, A::Salvar, A::Fixar(true), A::ProcurarNo("sp".into())]
        );
        let fixado = Contexto { fixado: true, salvo: true, ..tudo() };
        assert!(acoes(&al, &fixado).contains(&A::Fixar(false)));
        let pl = Alvo::Item(Box::new(Item::Playlist(Playlist::default())));
        assert!(!acoes(&pl, &tudo()).contains(&A::ProcurarNo("sp".into())), "a playlist é do serviço");
        let ar = Alvo::Item(Box::new(Item::Artista(Artista::default())));
        assert!(entradas(&ar, &tudo()).iter().any(|e| e.rotulo == "Seguir na biblioteca"));
        assert_eq!(acoes(&al, &Contexto::default()), [A::Abrir, A::Salvar, A::Fixar(true)], "ninguém toca");
    }

    #[test]
    fn o_menu_da_musica_do_computador() {
        use Acao as A;
        let m = Musica { album: "Disco".into(), artista: "Lume".into(), ..Default::default() };
        let a = acoes(&Alvo::Local { musica: Box::new(m), fila: None }, &tudo());
        assert!(a.contains(&A::IrAoAlbum) && a.contains(&A::IrAoArtista(0)) && a.contains(&A::ProcurarNosServicos));
        assert!(!a.contains(&A::Salvar), "curtir é dos serviços");
        let sem = acoes(&Alvo::Local { musica: Box::default(), fila: None }, &Contexto::default());
        assert_eq!(sem, [A::Tocar, A::TocarASeguir, A::PorNaFila, A::PorNaPlaylist], "sem etiquetas e sem serviços");
    }

    #[test]
    fn os_menus_da_barra_e_do_visual() {
        let barra = |estreita| entradas(&Alvo::Barra, &Contexto { estreita, ilha: true, ..Default::default() });
        assert!(barra(true).iter().any(|e| e.acao == Acao::Comando(Comando::AbrirFila)));
        assert!(!barra(false).iter().any(|e| e.acao == Acao::Comando(Comando::AbrirFila)), "larga, é um botão");
        assert!(barra(false).iter().any(|e| e.marcada && e.rotulo.starts_with("Ilha")));
        let visual = entradas(&Alvo::Visual, &Contexto { modelo: Modelo::Fluxo, ..Default::default() });
        assert_eq!(visual.iter().filter(|e| e.marcada).map(|e| e.rotulo.as_str()).collect::<Vec<_>>(), ["Fluxo"]);
    }

    #[test]
    fn os_grupos_nao_voltam() {
        let cx = Contexto { nomes_dos_outros: vec!["Spotify".into()], tem_par: true, ..tudo() };
        let outra = Faixa { servico: "sp".into(), id: "9".into(), ..faixa() };
        for a in [
            alvo(faixa(), vec![outra], Some(1)),
            Alvo::Barra,
            Alvo::Visual,
            Alvo::Item(Box::new(Item::Album(Album::default()))),
        ] {
            let grupos: Vec<u8> = entradas(&a, &cx).iter().map(|e| e.grupo).collect();
            assert!(grupos.windows(2).all(|g| g[0] <= g[1]), "{grupos:?}");
        }
    }

    #[test]
    fn o_menu_cabe_na_janela() {
        let janela = (958.0, 560.0);
        assert_eq!(lugar((100.0, 100.0), (260.0, 300.0), janela), (100.0, 100.0), "no ponto do clique");
        assert_eq!(lugar((900.0, 500.0), (260.0, 300.0), janela), (640.0, 200.0), "vira para o outro lado");
        assert_eq!(lugar((2.0, 2.0), (260.0, 300.0), janela), (8.0, 8.0));
        // Não cabe nem para um lado nem para o outro: encosta na borda.
        assert_eq!(lugar((300.0, 300.0), (260.0, 500.0), janela), (300.0, 52.0));
        // Maior que a janela: começa na margem (e rola).
        assert_eq!(lugar((50.0, 50.0), (260.0, 900.0), (300.0, 400.0)).1, 8.0);
    }
}
