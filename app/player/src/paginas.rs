//! O que as páginas dos serviços mostram de cada item e para onde cada um leva. Só lógica, testada.

use crate::navegacao::Rota;
use crate::remoto::PedidoDePagina;
use biblioteca::TipoSalvo;
use provedores::{Faixa, Item};

/// O que um cartão mostra.
#[derive(Clone, Debug, PartialEq)]
pub struct Cartao {
    pub titulo: String,
    pub sub: String,
    pub capa: Option<String>,
    /// Artista: capa redonda.
    pub redondo: bool,
    pub tipo: TipoSalvo,
}

pub fn cartao(item: &Item) -> Cartao {
    match item {
        Item::Faixa(f) => Cartao {
            titulo: f.titulo.clone(),
            sub: f.artistas_juntos(),
            capa: f.capa.clone(),
            redondo: false,
            tipo: TipoSalvo::Faixa,
        },
        Item::Album(a) => Cartao {
            titulo: a.titulo.clone(),
            sub: [Some(a.artistas.join(", ")).filter(|s| !s.is_empty()), a.ano.map(|n| n.to_string())]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · "),
            capa: a.capa.clone(),
            redondo: false,
            tipo: TipoSalvo::Album,
        },
        Item::Artista(a) => Cartao {
            titulo: a.nome.clone(),
            sub: "Artista".into(),
            capa: a.foto.clone(),
            redondo: true,
            tipo: TipoSalvo::Artista,
        },
        Item::Playlist(p) => Cartao {
            titulo: p.titulo.clone(),
            sub: [p.dono.clone().filter(|s| !s.is_empty()), p.quantas.map(|n| format!("{n} faixas"))]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join(" · "),
            capa: p.capa.clone(),
            redondo: false,
            tipo: TipoSalvo::Playlist,
        },
    }
}

/// A página que o item abre (a faixa não abre: toca).
pub fn rota_do_item(item: &Item) -> Option<Rota> {
    match item {
        Item::Faixa(_) => None,
        Item::Album(_) => Some(Rota::Album(item.clone())),
        Item::Artista(_) => Some(Rota::Artista(item.clone())),
        Item::Playlist(_) => Some(Rota::Playlist(item.clone())),
    }
}

/// O que pedir ao provedor para a página da rota.
pub fn pedido_da_rota(rota: &Rota) -> Option<PedidoDePagina> {
    let (item, f): (&Item, fn(String, String) -> PedidoDePagina) = match rota {
        Rota::Album(i) => (i, |servico, id| PedidoDePagina::Album { servico, id }),
        Rota::Artista(i) => (i, |servico, id| PedidoDePagina::Artista { servico, id }),
        Rota::Playlist(i) => (i, |servico, id| PedidoDePagina::Playlist { servico, id }),
        _ => return None,
    };
    let (servico, _, id) = chave_do_salvo(item);
    Some(f(servico, id))
}

/// A faixa de serviço de uma música da fila (para curtir a que toca). Música do computador não tem.
pub fn faixa_da_musica(m: &biblioteca::Musica) -> Option<Faixa> {
    let r = m.remota.as_ref()?;
    Some(Faixa {
        servico: r.servico.clone(),
        id: r.id.clone(),
        isrc: r.isrc.clone(),
        titulo: m.titulo.clone(),
        artistas: if m.artista.is_empty() { Vec::new() } else { vec![m.artista.clone()] },
        album: Some(m.album.clone()).filter(|a| !a.is_empty()),
        duracao_s: Some(m.duracao_s).filter(|d| *d > 0.0),
        capa: r.capa.clone(),
        ..Default::default()
    })
}

/// O item da página aberta (o que dá para salvar e fixar): álbum, playlist, artista ou faixa.
pub fn item_da_rota(rota: &Rota) -> Option<&Item> {
    match rota {
        Rota::Album(i) | Rota::Playlist(i) | Rota::Artista(i) | Rota::Faixa(i) => Some(i),
        _ => None,
    }
}

/// Serviço, tipo e id do item, como ele fica nos salvos.
pub fn chave_do_salvo(item: &Item) -> (String, TipoSalvo, String) {
    match item {
        Item::Faixa(f) => (f.servico.clone(), TipoSalvo::Faixa, f.id.clone()),
        Item::Album(a) => (a.servico.clone(), TipoSalvo::Album, a.id.clone()),
        Item::Artista(a) => (a.servico.clone(), TipoSalvo::Artista, a.id.clone()),
        Item::Playlist(p) => (p.servico.clone(), TipoSalvo::Playlist, p.id.clone()),
    }
}

/// O título da página (o cabeçalho da folha na Estante): o nome da página, ou o do item nas de detalhe.
pub fn titulo_da_rota(rota: &Rota) -> String {
    match rota {
        Rota::ParaVoce => "Para você".into(),
        Rota::Busca => "Busca".into(),
        Rota::Biblioteca => "Biblioteca".into(),
        Rota::Album(i) | Rota::Playlist(i) | Rota::Artista(i) | Rota::Faixa(i) => cartao(i).titulo,
        Rota::Fila => "Fila".into(),
        Rota::Historico => "Histórico".into(),
        Rota::Loja => "Loja".into(),
        Rota::Configuracoes => "Configurações".into(),
    }
}

/// A página do artista `i` da faixa: só quando o provedor mandou o id e sabe abrir artistas.
pub fn rota_do_artista(faixa: &Faixa, i: usize, sabe_artista: bool) -> Option<Rota> {
    let id = faixa.id_do_artista(i).filter(|_| sabe_artista)?;
    let nome = faixa.artistas.get(i)?.clone();
    Some(Rota::Artista(Item::Artista(provedores::Artista {
        servico: faixa.servico.clone(),
        id: id.to_string(),
        nome,
        foto: None,
    })))
}

/// A página do álbum da faixa: só quando o provedor mandou o id e sabe abrir álbuns. Até a página chegar, o
/// cabeçalho mostra o nome do álbum e os artistas da faixa.
pub fn rota_do_album(faixa: &Faixa, sabe_album: bool) -> Option<Rota> {
    let id = faixa.album_id.as_deref().filter(|id| sabe_album && !id.is_empty())?;
    Some(Rota::Album(Item::Album(provedores::Album {
        servico: faixa.servico.clone(),
        id: id.to_string(),
        titulo: faixa.album.clone().unwrap_or_default(),
        artistas: faixa.artistas.clone(),
        artistas_ids: faixa.artistas_ids.clone(),
        ano: None,
        capa: faixa.capa.clone(),
    })))
}

/// As abas da página da faixa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbaDaFaixa {
    Letra,
    Mapa,
    Parecidas,
}

impl AbaDaFaixa {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Letra => "Letra",
            Self::Mapa => "Mapa",
            Self::Parecidas => "Parecidas",
        }
    }
}

/// Parecidas só aparece quando o provedor sabe fazer rádio.
pub fn abas_da_faixa(sabe_radio: bool) -> Vec<AbaDaFaixa> {
    let mut abas = vec![AbaDaFaixa::Letra, AbaDaFaixa::Mapa];
    if sabe_radio {
        abas.push(AbaDaFaixa::Parecidas);
    }
    abas
}

/// O item como ele vai para os salvos do acervo (em JSON, para a Biblioteca abrir sem rede).
pub fn salvo_de(item: &Item, quando: i64) -> biblioteca::Salvo {
    let (servico, tipo, id) = chave_do_salvo(item);
    biblioteca::Salvo {
        servico,
        tipo,
        id,
        dados: serde_json::to_string(item).unwrap_or_default(),
        quando,
        fixado: false,
    }
}

/// O item de um salvo (`None` se os dados estragaram).
pub fn item_do_salvo(s: &biblioteca::Salvo) -> Option<Item> {
    serde_json::from_str(&s.dados).ok()
}

/// Um número para cada página (a rolagem de cada uma fica guardada à parte).
pub fn chave_da_rota(rota: &Rota) -> usize {
    let texto = match rota {
        Rota::Album(i) | Rota::Playlist(i) | Rota::Artista(i) | Rota::Faixa(i) => {
            let (servico, tipo, id) = chave_do_salvo(i);
            format!("{servico}|{}|{id}", tipo.chave())
        }
        outra => format!("{outra:?}"),
    };
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in texto.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h as usize
}

/// A página do artista vem misturada: as faixas mais tocadas e os álbuns, cada um na ordem em que veio.
pub fn faixas_e_albuns(itens: &[Item]) -> (Vec<Faixa>, Vec<Item>) {
    let faixas = itens.iter().filter_map(Item::faixa).cloned().collect();
    let albuns = itens.iter().filter(|i| matches!(i, Item::Album(_))).cloned().collect();
    (faixas, albuns)
}

/// Um item da Biblioteca de um serviço: salvo no Player, na conta da pessoa, ou nos dois.
#[derive(Clone, Debug, PartialEq)]
pub struct NaBiblioteca {
    pub item: Item,
    /// Salvo no Player (o alfinete só fixa o que está salvo).
    pub salvo: bool,
    pub fixado: bool,
}

/// A Biblioteca de um serviço: os fixados, depois o que está na conta (na ordem dela), depois o resto do que foi salvo
/// no Player. De todos (sem `servico`), o que foi salvo vem antes: uma conta cheia não empurra o resto para fora da
/// vista. O mesmo item (serviço, tipo e id) aparece uma vez.
pub fn juntar_biblioteca(
    salvos: &[(biblioteca::Salvo, Option<Item>)],
    da_conta: &[&Item],
    servico: Option<&str>,
) -> Vec<NaBiblioteca> {
    let salvos: Vec<NaBiblioteca> = salvos
        .iter()
        .filter(|(s, _)| servico.is_none_or(|x| s.servico == x))
        .filter_map(|(s, i)| i.as_ref().map(|i| NaBiblioteca { item: i.clone(), salvo: true, fixado: s.fixado }))
        .collect();
    let guardados: std::collections::HashSet<_> = salvos.iter().map(|b| chave_do_salvo(&b.item)).collect();
    let conta: Vec<NaBiblioteca> = da_conta
        .iter()
        .map(|i| NaBiblioteca { item: (*i).clone(), salvo: guardados.contains(&chave_do_salvo(i)), fixado: false })
        .collect();
    let (fixados, resto): (Vec<_>, Vec<_>) = salvos.into_iter().partition(|b| b.fixado);
    let (segundo, terceiro) = if servico.is_some() { (conta, resto) } else { (resto, conta) };
    let mut vistos = std::collections::HashSet::new();
    let todos = fixados.into_iter().chain(segundo).chain(terceiro);
    todos.filter(|b| vistos.insert(chave_do_salvo(&b.item))).collect()
}

/// As faixas que dá para tocar (as dos serviços com áudio) e a posição da escolhida entre elas. `None` se a escolhida
/// não toca.
pub fn so_as_que_tocam(faixas: &[Faixa], i: usize, nao_toca: impl Fn(&str) -> bool) -> Option<(Vec<Faixa>, usize)> {
    let i = i.min(faixas.len().checked_sub(1)?);
    if nao_toca(&faixas[i].servico) {
        return None;
    }
    let antes = faixas[..i].iter().filter(|f| !nao_toca(&f.servico)).count();
    Some((faixas.iter().filter(|f| !nao_toca(&f.servico)).cloned().collect(), antes))
}

/// A folga, em linhas, de [`janela_de_linhas`] (a medida da lista é a do quadro anterior).
const FOLGA_DE_LINHAS: f32 = 12.0;
/// Quantas linhas montar enquanto a lista ainda não foi medida.
const LINHAS_SEM_MEDIDA: usize = 40;

/// As linhas de uma lista longa que vale a pena montar: as que aparecem em `visivel` (o alto e o pé da folha) e uma
/// folga acima e abaixo. `topo_da_lista` é onde a primeira linha está agora (negativo quando a folha rolou).
pub fn janela_de_linhas(
    topo_da_lista: Option<f32>,
    visivel: (f32, f32),
    linha: f32,
    total: usize,
) -> std::ops::Range<usize> {
    if linha <= 0.0 {
        return 0..total;
    }
    let Some(topo) = topo_da_lista.filter(|t| t.is_finite()) else { return 0..total.min(LINHAS_SEM_MEDIDA) };
    let de = ((visivel.0 - topo) / linha).floor() - FOLGA_DE_LINHAS;
    let ate = ((visivel.1 - topo) / linha).ceil() + FOLGA_DE_LINHAS;
    // (Um `as usize` de um negativo dá zero.)
    let (de, ate) = ((de as usize).min(total), (ate as usize).min(total));
    de..ate.max(de)
}

/// O chip escolhido na Busca.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FiltroDaBusca {
    #[default]
    Tudo,
    Faixas,
    Albuns,
    Artistas,
    Playlists,
}

impl FiltroDaBusca {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Tudo => "Tudo",
            Self::Faixas => "Faixas",
            Self::Albuns => "Álbuns",
            Self::Artistas => "Artistas",
            Self::Playlists => "Playlists",
        }
    }
}

/// Os resultados da busca de um serviço (ou de todos), por tipo.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GruposDaBusca {
    /// O primeiro que o provedor mandou (os provedores mandam o melhor primeiro).
    pub melhor: Option<Item>,
    pub faixas: Vec<Faixa>,
    pub albuns: Vec<Item>,
    pub artistas: Vec<Item>,
    pub playlists: Vec<Item>,
}

impl GruposDaBusca {
    /// Os chips que valem: Tudo e os tipos que têm resultado (nenhum, sem resultado).
    pub fn filtros(&self) -> Vec<FiltroDaBusca> {
        let tipos = [
            (FiltroDaBusca::Faixas, !self.faixas.is_empty()),
            (FiltroDaBusca::Albuns, !self.albuns.is_empty()),
            (FiltroDaBusca::Artistas, !self.artistas.is_empty()),
            (FiltroDaBusca::Playlists, !self.playlists.is_empty()),
        ];
        let com: Vec<FiltroDaBusca> = tipos.into_iter().filter(|(_, tem)| *tem).map(|(f, _)| f).collect();
        if com.is_empty() { com } else { std::iter::once(FiltroDaBusca::Tudo).chain(com).collect() }
    }
}

/// A busca de um serviço ainda não chegou? (A da página é de outro texto, ou ele ainda não respondeu.) Cada
/// serviço espera só a resposta dele: o que já respondeu não fica no esqueleto esperando os outros.
pub fn servico_buscando(texto_da_busca: &str, respondidos: &[String], consulta: &str, servico: &str) -> bool {
    texto_da_busca != consulta || !respondidos.iter().any(|s| s == servico)
}

/// Separa os itens da busca por tipo, só os do serviço (todos com `None`).
pub fn agrupar_busca(itens: &[Item], servico: Option<&str>) -> GruposDaBusca {
    let mut g = GruposDaBusca::default();
    for item in itens.iter().filter(|i| servico.is_none_or(|s| chave_do_salvo(i).0 == s)) {
        if g.melhor.is_none() {
            g.melhor = Some(item.clone());
        }
        match item {
            Item::Faixa(f) => g.faixas.push(f.clone()),
            Item::Album(_) => g.albuns.push(item.clone()),
            Item::Artista(_) => g.artistas.push(item.clone()),
            Item::Playlist(_) => g.playlists.push(item.clone()),
        }
    }
    g
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use provedores::{Album, Artista, Playlist};

    fn faixa() -> Faixa {
        Faixa {
            servico: "yt".into(),
            id: "f1".into(),
            titulo: "Faixa".into(),
            artistas: vec!["A".into(), "B".into()],
            artistas_ids: vec!["a1".into(), String::new()],
            ..Default::default()
        }
    }

    #[test]
    fn a_faixa_que_toca_vira_item_para_curtir() {
        let f = Faixa { capa: Some("http://c".into()), album: Some("Disco".into()), duracao_s: Some(200.0), ..faixa() };
        let m = crate::remoto::musica_de(&f, "YT");
        let volta = faixa_da_musica(&m).expect("faixa de serviço");
        assert_eq!(chave_do_salvo(&Item::Faixa(volta.clone())), chave_do_salvo(&Item::Faixa(f)));
        assert_eq!(
            (volta.titulo.as_str(), volta.album.as_deref(), volta.capa.as_deref(), volta.duracao_s),
            ("Faixa", Some("Disco"), Some("http://c"), Some(200.0))
        );
        assert_eq!(volta.artistas, ["A, B"]);
        assert!(faixa_da_musica(&biblioteca::Musica::default()).is_none(), "música do computador não");
    }

    #[test]
    fn biblioteca_junta_a_conta_e_os_salvos() {
        let lista = |servico: &str, id: &str| {
            Item::Playlist(Playlist { servico: servico.into(), id: id.into(), titulo: id.into(), ..Default::default() })
        };
        let salvo = |item: &Item, fixado: bool| {
            let mut s = salvo_de(item, 1);
            s.fixado = fixado;
            (s, Some(item.clone()))
        };
        let (fixada, so_salva, nas_duas, de_outro) =
            (lista("s", "fix"), lista("s", "salva"), lista("s", "duas"), lista("yt", "x"));
        let so_na_conta = lista("s", "conta");
        let salvos = [salvo(&fixada, true), salvo(&so_salva, false), salvo(&nas_duas, false), salvo(&de_outro, false)];
        let da_conta = [&so_na_conta, &nas_duas, &fixada];
        let juntos = juntar_biblioteca(&salvos, &da_conta, Some("s"));
        let ids: Vec<(String, bool, bool)> =
            juntos.iter().map(|b| (chave_do_salvo(&b.item).2, b.salvo, b.fixado)).collect();
        assert_eq!(
            ids,
            [
                ("fix".into(), true, true),
                ("conta".into(), false, false),
                ("duas".into(), true, false),
                ("salva".into(), true, false)
            ],
            "fixados, depois a conta na ordem dela, depois o resto; sem repetir"
        );
        assert_eq!(juntar_biblioteca(&salvos, &[], None).len(), 4, "em Tudo, de todos os serviços");
        let tudo: Vec<String> =
            juntar_biblioteca(&salvos, &da_conta, None).iter().map(|b| chave_do_salvo(&b.item).2).collect();
        assert_eq!(
            tudo,
            ["fix", "salva", "duas", "x", "conta"],
            "em Tudo, uma conta cheia não empurra o que foi salvo"
        );
        // O mesmo id em serviços ou tipos diferentes não é o mesmo item.
        let album = Item::Album(Album { servico: "s".into(), id: "fix".into(), ..Default::default() });
        assert_eq!(juntar_biblioteca(&salvos[..1], &[&album], Some("s")).len(), 2);
    }

    #[test]
    fn toca_so_o_que_tem_audio() {
        let de = |servico: &str, id: &str| Faixa { servico: servico.into(), id: id.into(), ..faixa() };
        let faixas = [de("sem", "1"), de("yt", "2"), de("sem", "3"), de("yt", "4")];
        let sem = |s: &str| s == "sem";
        let (tocam, i) = so_as_que_tocam(&faixas, 3, sem).expect("a escolhida toca");
        assert_eq!((tocam.len(), tocam[i].id.as_str()), (2, "4"));
        assert!(so_as_que_tocam(&faixas, 0, sem).is_none(), "a escolhida não toca");
        assert!(so_as_que_tocam(&[], 0, sem).is_none());
        let (todas, i) = so_as_que_tocam(&faixas, 2, |_| false).expect("todas");
        assert_eq!((todas.len(), i), (4, 2));
        assert_eq!(
            so_as_que_tocam(&faixas, 99, |_| false).map(|(f, i)| (f.len(), i)),
            Some((4, 3)),
            "fora da lista: a última"
        );
    }

    #[test]
    fn so_as_linhas_a_vista_sao_montadas() {
        // A folha vai de 100 a 900; a lista começa em 300, com linhas de 50.
        assert_eq!(janela_de_linhas(Some(300.0), (100.0, 900.0), 50.0, 2000), 0..24, "12 à vista e 12 de folga");
        // Rolada 10 000 px: a linha 196 está no alto da folha.
        assert_eq!(janela_de_linhas(Some(-9700.0), (100.0, 900.0), 50.0, 2000), 184..224);
        assert_eq!(janela_de_linhas(Some(-99_000.0), (100.0, 900.0), 50.0, 2000), 1970..2000, "no fim");
        assert_eq!(janela_de_linhas(Some(-999_999.0), (100.0, 900.0), 50.0, 2000), 2000..2000, "passou do fim");
        assert_eq!(janela_de_linhas(Some(5000.0), (100.0, 900.0), 50.0, 2000), 0..0, "ainda abaixo da tela");
        assert_eq!(janela_de_linhas(None, (100.0, 900.0), 50.0, 2000), 0..40, "sem medida ainda");
        assert_eq!(janela_de_linhas(None, (100.0, 900.0), 50.0, 7), 0..7);
        assert_eq!(janela_de_linhas(Some(300.0), (100.0, 900.0), 0.0, 7), 0..7, "sem altura, monta tudo");
        assert_eq!(janela_de_linhas(Some(f32::NAN), (100.0, 900.0), 50.0, 2000), 0..40);
    }

    #[test]
    fn o_item_da_pagina_aberta_para_fixar() {
        let al = Item::Album(Album { servico: "yt".into(), id: "al".into(), ..Default::default() });
        assert_eq!(item_da_rota(&Rota::Album(al.clone())), Some(&al));
        assert_eq!(item_da_rota(&Rota::Busca), None);
        assert_eq!(item_da_rota(&Rota::Fila), None);
        let f = Item::Faixa(faixa());
        assert_eq!(item_da_rota(&Rota::Faixa(f.clone())), Some(&f));
    }

    #[test]
    fn cartao_de_cada_tipo() {
        let c = cartao(&Item::Faixa(faixa()));
        assert_eq!((c.titulo.as_str(), c.sub.as_str(), c.redondo, c.tipo), ("Faixa", "A, B", false, TipoSalvo::Faixa));
        let al = Item::Album(Album {
            servico: "yt".into(),
            id: "al".into(),
            titulo: "Disco".into(),
            artistas: vec!["A".into()],
            ano: Some(2001),
            ..Default::default()
        });
        assert_eq!(cartao(&al).sub, "A · 2001");
        let ar = Item::Artista(Artista { servico: "yt".into(), id: "ar".into(), nome: "A".into(), foto: None });
        let c = cartao(&ar);
        assert!(c.redondo);
        assert_eq!(c.sub, "Artista");
        let pl = Item::Playlist(Playlist {
            servico: "yt".into(),
            id: "pl".into(),
            titulo: "Mix".into(),
            dono: Some("Eu".into()),
            quantas: Some(12),
            ..Default::default()
        });
        assert_eq!(cartao(&pl).sub, "Eu · 12 faixas");
    }

    #[test]
    fn rotas_pedidos_e_chaves() {
        assert_eq!(rota_do_item(&Item::Faixa(faixa())), None, "faixa toca, não abre página");
        let al = Item::Album(Album { servico: "yt".into(), id: "al".into(), ..Default::default() });
        let rota = rota_do_item(&al).expect("álbum abre");
        assert_eq!(pedido_da_rota(&rota), Some(PedidoDePagina::Album { servico: "yt".into(), id: "al".into() }));
        assert_eq!(pedido_da_rota(&Rota::Loja), None);
        assert_eq!(chave_do_salvo(&al), ("yt".to_string(), TipoSalvo::Album, "al".to_string()));
    }

    #[test]
    fn link_do_artista_so_com_id_e_capacidade() {
        let f = faixa();
        assert!(rota_do_artista(&f, 0, true).is_some());
        assert_eq!(rota_do_artista(&f, 0, false), None, "o provedor não abre artista");
        assert_eq!(rota_do_artista(&f, 1, true), None, "sem id");
        assert_eq!(rota_do_artista(&f, 9, true), None, "fora da lista");
    }

    #[test]
    fn faixas_e_albuns_do_artista() {
        let al = Item::Album(Album { servico: "yt".into(), id: "al".into(), ..Default::default() });
        let itens = vec![Item::Faixa(faixa()), al.clone(), Item::Faixa(Faixa { id: "f2".into(), ..faixa() })];
        let (faixas, albuns) = faixas_e_albuns(&itens);
        assert_eq!(faixas.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), ["f1", "f2"]);
        assert_eq!(albuns, vec![al]);
    }

    #[test]
    fn item_vai_para_o_salvo_e_volta() {
        let item = Item::Faixa(faixa());
        let s = salvo_de(&item, 42);
        assert_eq!(
            (s.servico.as_str(), s.tipo, s.id.as_str(), s.quando, s.fixado),
            ("yt", TipoSalvo::Faixa, "f1", 42, false)
        );
        assert_eq!(item_do_salvo(&s), Some(item));
        let estragado = biblioteca::Salvo { dados: "{nao e json".into(), ..s };
        assert_eq!(item_do_salvo(&estragado), None);
    }

    fn al(servico: &str, id: &str) -> Item {
        Item::Album(Album { servico: servico.into(), id: id.into(), ..Default::default() })
    }

    #[test]
    fn agrupar_busca_por_tipo_e_servico() {
        let ar = Item::Artista(Artista { servico: "yt".into(), id: "ar".into(), nome: "A".into(), foto: None });
        let outra = Item::Faixa(Faixa { servico: "sp".into(), id: "s1".into(), ..faixa() });
        let itens = vec![ar.clone(), Item::Faixa(faixa()), al("yt", "a1"), outra.clone(), al("sp", "a9")];
        let g = agrupar_busca(&itens, Some("yt"));
        assert_eq!(g.melhor, Some(ar.clone()), "o primeiro do serviço");
        assert_eq!(g.faixas.iter().map(|f| f.id.as_str()).collect::<Vec<_>>(), ["f1"]);
        assert_eq!(g.albuns, vec![al("yt", "a1")]);
        assert_eq!(g.artistas, vec![ar], "o melhor também fica no grupo dele");
        assert!(g.playlists.is_empty());
        let todos = agrupar_busca(&itens, None);
        assert_eq!(todos.faixas.len(), 2);
        assert_eq!(todos.albuns.len(), 2);
        assert!(agrupar_busca(&itens, Some("nenhum")).melhor.is_none());
    }

    #[test]
    fn filtros_so_dos_tipos_com_resultado() {
        let g = agrupar_busca(&[Item::Faixa(faixa()), al("yt", "a1")], None);
        assert_eq!(g.filtros(), vec![FiltroDaBusca::Tudo, FiltroDaBusca::Faixas, FiltroDaBusca::Albuns]);
        assert!(agrupar_busca(&[], None).filtros().is_empty(), "sem resultado, sem chips");
        assert_eq!(FiltroDaBusca::Albuns.nome(), "Álbuns");
    }

    #[test]
    fn cada_servico_espera_so_a_resposta_dele() {
        let respondidos = vec!["yt".to_string()];
        assert!(!servico_buscando("abc", &respondidos, "abc", "yt"), "o YouTube já respondeu");
        assert!(servico_buscando("abc", &respondidos, "abc", "sp"), "o outro ainda não");
        assert!(servico_buscando("ab", &respondidos, "abc", "yt"), "a resposta é de outro texto");
    }

    #[test]
    fn titulo_de_cada_rota() {
        assert_eq!(titulo_da_rota(&Rota::ParaVoce), "Para você");
        assert_eq!(titulo_da_rota(&Rota::Historico), "Histórico");
        assert_eq!(titulo_da_rota(&Rota::Configuracoes), "Configurações");
        let al =
            Item::Album(Album { servico: "yt".into(), id: "a".into(), titulo: "Disco".into(), ..Default::default() });
        assert_eq!(titulo_da_rota(&Rota::Album(al)), "Disco", "a de detalhe leva o nome do item");
        assert_eq!(titulo_da_rota(&Rota::Faixa(Item::Faixa(faixa()))), "Faixa");
    }

    #[test]
    fn parecidas_so_com_radio() {
        assert_eq!(abas_da_faixa(false), vec![AbaDaFaixa::Letra, AbaDaFaixa::Mapa]);
        assert_eq!(abas_da_faixa(true), vec![AbaDaFaixa::Letra, AbaDaFaixa::Mapa, AbaDaFaixa::Parecidas]);
        assert_eq!(AbaDaFaixa::Mapa.nome(), "Mapa");
    }

    #[test]
    #[allow(clippy::panic)]
    fn link_do_album_so_com_id_e_capacidade() {
        let com = Faixa { album: Some("Disco".into()), album_id: Some("al".into()), ..faixa() };
        let rota = rota_do_album(&com, true).expect("tem id e o provedor abre álbuns");
        let Rota::Album(Item::Album(a)) = rota else { panic!("rota de álbum") };
        assert_eq!((a.servico.as_str(), a.id.as_str(), a.titulo.as_str()), ("yt", "al", "Disco"));
        assert_eq!(a.artistas, com.artistas, "o álbum herda os artistas da faixa até a página chegar");
        assert_eq!(rota_do_album(&com, false), None, "o provedor não abre álbum");
        assert_eq!(rota_do_album(&Faixa { album_id: None, ..com.clone() }, true), None, "sem id");
    }

    #[test]
    fn cada_pagina_tem_a_sua_chave_de_rolagem() {
        let al =
            |id: &str| Rota::Album(Item::Album(Album { servico: "yt".into(), id: id.into(), ..Default::default() }));
        assert_eq!(chave_da_rota(&al("a")), chave_da_rota(&al("a")));
        assert_ne!(chave_da_rota(&al("a")), chave_da_rota(&al("b")));
        assert_ne!(chave_da_rota(&Rota::ParaVoce), chave_da_rota(&Rota::Biblioteca));
    }
}
