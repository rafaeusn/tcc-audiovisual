//! Em Tudo, os serviços juntos: as prateleiras do Início por tipo, os resultados da busca intercalados e a mesma música de
//! dois serviços numa linha só, com os dois selos. Só lógica, testada.

use crate::paginas::{GruposDaBusca, cartao, chave_do_salvo};
use crate::ui::normalizar;
use provedores::{Faixa, Item, Prateleira, TipoDaPrateleira, casador};
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;

/// A partir desta nota, duas faixas de serviços diferentes são a mesma música (mais alta que a do Match: ali a nota menor
/// só escolhe o que tocar; aqui ela esconderia uma música).
const MESMA_MUSICA: f32 = 0.9;
/// Quantos atalhos o Início mostra.
pub const ATALHOS: usize = 8;
/// Com menos faixas que isto, "Tocadas há pouco" não aparece.
const MINIMO_DE_RECENTES: usize = 3;
/// Quantos itens uma fileira junta tem.
const POR_FILEIRA: usize = 30;

/// Um item de Tudo: o serviço dele e, quando a mesma música está em outros, quais.
#[derive(Clone, Debug, PartialEq)]
pub struct Junto {
    pub item: Item,
    /// A mesma música nos outros serviços.
    pub tambem: Vec<Faixa>,
}

impl Junto {
    pub fn so(item: Item) -> Self {
        Self { item, tambem: Vec::new() }
    }

    /// Os serviços do item, o que toca na frente.
    pub fn servicos(&self) -> Vec<&str> {
        std::iter::once(self.item.servico()).chain(self.tambem.iter().map(|f| f.servico.as_str())).collect()
    }
}

/// Uma fileira do Início.
#[derive(Clone, Debug, PartialEq)]
pub struct Fileira {
    pub titulo: String,
    /// De um serviço só (o nome dele aparece ao lado do título).
    pub de: Option<String>,
    pub itens: Vec<Junto>,
    /// As faixas entre os itens (a fila de quem toca uma).
    pub faixas: Arc<Vec<Faixa>>,
}

impl Fileira {
    fn nova(titulo: String, de: Option<String>, itens: Vec<Junto>) -> Self {
        let faixas = faixas_de(&itens);
        Self { titulo, de, itens, faixas }
    }
}

fn faixas_de(itens: &[Junto]) -> Arc<Vec<Faixa>> {
    Arc::new(itens.iter().filter_map(|j| j.item.faixa().cloned()).collect())
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Inicio {
    pub atalhos: Vec<Item>,
    pub fileiras: Vec<Fileira>,
}

/// O que a busca achou em todos os serviços.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuscaJunta {
    pub melhor: Option<Junto>,
    pub musicas: Vec<Junto>,
    /// As faixas de `musicas`, na mesma ordem.
    pub faixas: Arc<Vec<Faixa>>,
    pub albuns: Vec<Junto>,
    pub artistas: Vec<Junto>,
    pub playlists: Vec<Junto>,
}

/// Os títulos (já limpos) podem ser da mesma música? Uma peneira barata antes da nota: cada resultado é comparado com
/// todos os que já entraram.
fn titulos_parecem(a: &str, b: &str) -> bool {
    a.contains(b) || b.contains(a) || casador::parecido(a, b) >= 0.6
}

/// As duas faixas, de serviços diferentes, são a mesma música?
fn mesma_musica(a: &Faixa, titulo_a: &str, b: &Faixa, titulo_b: &str) -> bool {
    if a.servico == b.servico {
        return false;
    }
    if a.isrc_limpo().is_some() && a.isrc_limpo() == b.isrc_limpo() {
        return true;
    }
    if !titulos_parecem(titulo_a, titulo_b) {
        return false;
    }
    // Nos dois sentidos: o casador olha um lado como o original e o outro como candidato.
    casador::nota(a, b).max(casador::nota(b, a)) >= MESMA_MUSICA
}

/// Como juntar: por qual serviço tocar primeiro e o que a pessoa disse que não é a mesma música.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Regras {
    pub preferencia: Vec<String>,
    pub separadas: Arc<HashSet<String>>,
}

/// A chave de duas faixas separadas (a ordem não importa).
pub fn par_separado(a: &str, b: &str) -> String {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    format!("{a}|{b}")
}

/// Quantas separações ficam guardadas.
const SEPARADAS: usize = 5_000;

/// O que a pessoa separou, em `<dados>/separadas.json`.
#[derive(Default)]
pub struct Separadas {
    arquivo: Option<PathBuf>,
    pares: Arc<HashSet<String>>,
}

impl Separadas {
    pub fn abrir(arquivo: Option<PathBuf>) -> Self {
        let texto = arquivo.as_ref().and_then(|a| std::fs::read_to_string(a).ok());
        let pares = texto.and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Self { arquivo, pares: Arc::new(pares) }
    }

    pub fn pares(&self) -> Arc<HashSet<String>> {
        self.pares.clone()
    }

    /// Separa a faixa das outras. O arquivo é gravado em outra thread.
    pub fn separar(&mut self, faixa: &Faixa, outras: &[Faixa]) {
        let mut pares = (*self.pares).clone();
        for o in outras {
            if pares.len() < SEPARADAS {
                pares.insert(par_separado(&faixa.chave(), &o.chave()));
            }
        }
        self.pares = Arc::new(pares);
        let (Some(arquivo), Ok(texto)) = (self.arquivo.clone(), serde_json::to_string(&*self.pares)) else { return };
        std::thread::spawn(move || {
            let novo = arquivo.with_extension("json.novo");
            let gravou = std::fs::write(&novo, texto).and_then(|()| std::fs::rename(&novo, &arquivo));
            if let Err(e) = gravou {
                log::warn!("as músicas separadas não foram guardadas: {e}");
            }
        });
    }
}

/// A posição do serviço na ordem de preferência (o que não está nela vai para o fim).
fn posicao(preferencia: &[String], servico: &str) -> usize {
    preferencia.iter().position(|s| s == servico).unwrap_or(usize::MAX)
}

/// Um item de cada lista por vez, sem repetir. A mesma música de outro serviço se junta à que já entrou, no lugar dela, e
/// quem toca é o serviço preferido.
pub fn intercalar(listas: &[Vec<Item>], regras: &Regras, limite: usize) -> Vec<Junto> {
    let mut vistos = HashSet::new();
    let mut juntos: Vec<Junto> = Vec::new();
    // O título limpo de cada um que entrou (vazio no que não é faixa).
    let mut titulos: Vec<String> = Vec::new();
    let maior = listas.iter().map(Vec::len).max().unwrap_or(0);
    for i in 0..maior {
        for item in listas.iter().filter_map(|l| l.get(i)) {
            if !vistos.insert(chave(item)) {
                continue;
            }
            let titulo = item.faixa().map(|f| casador::limpar_titulo(&f.titulo)).unwrap_or_default();
            let igual = item.faixa().and_then(|f| {
                juntos.iter().zip(&titulos).position(|(j, t)| {
                    j.item.faixa().is_some_and(|g| {
                        mesma_musica(g, t, f, &titulo)
                            && !regras.separadas.contains(&par_separado(&g.chave(), &f.chave()))
                    }) && !j.tambem.iter().any(|x| x.servico == f.servico)
                })
            });
            match igual {
                Some(i) => {
                    let j = &mut juntos[i];
                    let na_frente =
                        posicao(&regras.preferencia, item.servico()) < posicao(&regras.preferencia, j.item.servico());
                    let outra = if na_frente { std::mem::replace(&mut j.item, item.clone()) } else { item.clone() };
                    if let Item::Faixa(f) = outra {
                        j.tambem.push(f);
                    }
                }
                None if juntos.len() < limite => {
                    juntos.push(Junto::so(item.clone()));
                    titulos.push(titulo);
                }
                None => {}
            }
        }
    }
    juntos
}

fn chave(item: &Item) -> (String, &'static str, String) {
    let (servico, tipo, id) = chave_do_salvo(item);
    (servico, tipo.chave(), id)
}

/// O título de uma fileira com mais de um serviço.
fn titulo_do_tipo(tipo: TipoDaPrateleira) -> &'static str {
    use TipoDaPrateleira as T;
    match tipo {
        T::Recentes => "Tocadas há pouco",
        T::ParaVoce => "Feito para você",
        T::Lancamentos => "Lançamentos",
        T::Artistas => "Artistas",
        T::Clipes => "Clipes",
        T::Paradas => "Em alta",
        T::UltimoLancamento => "Último lançamento",
        T::Parecidos => "Parecidos",
        T::MaisDoArtista => "Mais do artista",
        T::Atalhos | T::Outras => "",
    }
}

/// O Início: os atalhos (o que foi aberto há pouco, depois os dos serviços) e as fileiras. As do mesmo tipo se juntam; as
/// sem tipo se intercalam entre os serviços. `paginas` são só os serviços que responderam.
pub fn juntar_inicio(paginas: &[(String, &[Prateleira])], recentes: &[Item], regras: &Regras) -> Inicio {
    use TipoDaPrateleira as T;
    let do_tipo = |tipo: T| -> Vec<(&str, Vec<&Prateleira>)> {
        paginas
            .iter()
            .map(|(s, ps)| (s.as_str(), ps.iter().filter(|p| p.tipo == tipo && !p.itens.is_empty()).collect()))
            .collect()
    };
    let itens_de = |p: &Prateleira| p.itens.clone();

    // Os atalhos: o que a pessoa abriu há pouco, e depois um de cada serviço por vez.
    let abertos: Vec<Item> = recentes.iter().filter(|i| i.faixa().is_none()).cloned().collect();
    let dos_servicos: Vec<Vec<Item>> =
        do_tipo(T::Atalhos).into_iter().map(|(_, ps)| ps.into_iter().flat_map(itens_de).collect()).collect();
    let mut vistos = HashSet::new();
    let atalhos: Vec<Item> = abertos
        .into_iter()
        .chain(intercalar(&dos_servicos, regras, 2 * ATALHOS).into_iter().map(|j| j.item))
        .filter(|i| vistos.insert(chave(i)))
        .take(ATALHOS)
        .collect();

    let mut fileiras = Vec::new();
    for tipo in [T::Recentes, T::ParaVoce, T::Lancamentos, T::Artistas, T::Outras, T::Clipes, T::Paradas] {
        let por_servico = do_tipo(tipo);
        let quantas = por_servico.iter().map(|(_, ps)| ps.len()).max().unwrap_or(0);
        // As nossas "tocadas há pouco" valem como as de mais um serviço, na frente.
        let tocadas: Vec<Item> = match tipo {
            T::Recentes => recentes.iter().filter(|i| i.faixa().is_some()).cloned().collect(),
            _ => Vec::new(),
        };
        let tocadas = if tocadas.len() >= MINIMO_DE_RECENTES { tocadas } else { Vec::new() };
        for k in 0..quantas.max(usize::from(!tocadas.is_empty())) {
            let desta: Vec<(&str, &Prateleira)> =
                por_servico.iter().filter_map(|(s, ps)| ps.get(k).map(|p| (*s, *p))).collect();
            if tipo == T::Outras {
                for (s, p) in desta {
                    let itens = intercalar(&[itens_de(p)], regras, POR_FILEIRA);
                    fileiras.push(Fileira::nova(p.titulo.clone(), Some(s.to_string()), itens));
                }
                continue;
            }
            let mut listas: Vec<Vec<Item>> = Vec::new();
            if k == 0 && !tocadas.is_empty() {
                listas.push(tocadas.clone());
            }
            listas.extend(desta.iter().map(|(_, p)| itens_de(p)));
            // Com um serviço só (e nada nosso), a fileira é dele, com o título dele.
            let sozinho = if listas.len() == 1 { desta.first() } else { None };
            let (titulo, de) = match (sozinho, desta.first()) {
                (Some((s, p)), _) => (p.titulo.clone(), Some((*s).to_string())),
                // Da segunda em diante, o título nosso repetiria.
                (None, Some((_, p))) if k > 0 => (p.titulo.clone(), None),
                _ => (titulo_do_tipo(tipo).to_string(), None),
            };
            fileiras.push(Fileira::nova(titulo, de, intercalar(&listas, regras, POR_FILEIRA)));
        }
    }
    fileiras.retain(|f| !f.itens.is_empty());
    Inicio { atalhos, fileiras }
}

/// Os resultados da busca de vários serviços (na ordem em que aparecem), juntos por tipo.
pub fn juntar_busca(consulta: &str, grupos: &[&GruposDaBusca], regras: &Regras) -> BuscaJunta {
    let intercalados = |de: &dyn Fn(&GruposDaBusca) -> Vec<Item>| {
        let listas: Vec<Vec<Item>> = grupos.iter().map(|g| de(g)).collect();
        intercalar(&listas, regras, 100)
    };
    let musicas = intercalados(&|g| g.faixas.iter().cloned().map(Item::Faixa).collect());
    let melhores: Vec<&Item> = grupos.iter().filter_map(|g| g.melhor.as_ref()).collect();
    let melhor = melhor_de(consulta, &melhores).map(|item| {
        // A faixa que também está em outro serviço leva os dois selos, como na lista.
        let k = chave(item);
        musicas.iter().find(|j| chave(&j.item) == k).cloned().unwrap_or_else(|| Junto::so(item.clone()))
    });
    BuscaJunta {
        melhor,
        faixas: faixas_de(&musicas),
        musicas,
        albuns: intercalados(&|g| g.albuns.clone()),
        artistas: intercalados(&|g| g.artistas.clone()),
        playlists: intercalados(&|g| g.playlists.clone()),
    }
}

/// Entre o melhor de cada serviço, o que mais parece com o que foi escrito (empate: o primeiro; um artista ganha de uma
/// faixa com o mesmo nome).
fn melhor_de<'a>(consulta: &str, melhores: &[&'a Item]) -> Option<&'a Item> {
    let consulta = normalizar(consulta);
    let nota = |item: &Item| {
        let ficha = cartao(item);
        let nome = normalizar(&ficha.titulo);
        let com_artista = normalizar(&format!("{} {}", ficha.titulo, ficha.sub));
        let parecido = casador::parecido(&consulta, &nome).max(casador::parecido(&consulta, &com_artista));
        parecido + if ficha.redondo { 0.05 } else { 0.0 }
    };
    let mut escolhido: Option<(&Item, f32)> = None;
    for item in melhores {
        let n = nota(item);
        if escolhido.is_none_or(|(_, m)| n > m) {
            escolhido = Some((item, n));
        }
    }
    escolhido.map(|(i, _)| i)
}

/// Um número de 0 a `n` (sem ele), para começar uma lista em ordem aleatória.
pub fn sorteio(n: usize) -> usize {
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.subsec_nanos());
    nanos as usize % n.max(1)
}

/// A saudação do Início, pela hora.
pub fn saudacao(hora: u8) -> &'static str {
    match hora {
        5..=11 => "Bom dia",
        12..=17 => "Boa tarde",
        _ => "Boa noite",
    }
}

/// O serviço de um app pelo nome dele no histórico ou no "tocando agora" ("Spotify", "YouTube Music", "Spotify.exe").
pub fn servico_do_app(app: &str, provedores: &[(String, String)]) -> Option<String> {
    let app = normalizar(app);
    if app.is_empty() {
        return None;
    }
    let de = provedores.iter().find(|(_, nome)| normalizar(nome) == app);
    de.map(|(s, _)| s.clone()).or_else(|| {
        // O programa oficial e o Spotifast tocam pela conta do Spotify.
        let spotify = app.starts_with("spotify") || app.starts_with("spotifast");
        spotify.then(|| "spotify".to_string())
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use TipoDaPrateleira as T;
    use provedores::{Album, Artista, Playlist};

    fn f(servico: &str, id: &str, titulo: &str, artista: &str) -> Item {
        Item::Faixa(Faixa {
            servico: servico.into(),
            id: id.into(),
            titulo: titulo.into(),
            artistas: vec![artista.into()],
            duracao_s: Some(200.0),
            ..Default::default()
        })
    }

    fn al(servico: &str, id: &str) -> Item {
        Item::Album(Album { servico: servico.into(), id: id.into(), titulo: id.into(), ..Default::default() })
    }

    fn pr(titulo: &str, tipo: T, itens: Vec<Item>) -> Prateleira {
        Prateleira { titulo: titulo.into(), tipo, itens }
    }

    fn ids(juntos: &[Junto]) -> Vec<String> {
        juntos.iter().map(|j| format!("{}:{}", j.item.servico(), chave(&j.item).2)).collect()
    }

    fn com(preferencia: &[&str]) -> Regras {
        Regras { preferencia: preferencia.iter().map(|s| (*s).to_string()).collect(), ..Default::default() }
    }

    fn preferencia() -> Regras {
        com(&["yt", "sp"])
    }

    #[test]
    fn intercala_sem_repetir() {
        let listas =
            [vec![al("yt", "a"), al("yt", "b"), al("yt", "a")], vec![al("sp", "a"), al("sp", "c"), al("sp", "d")]];
        assert_eq!(ids(&intercalar(&listas, &preferencia(), 30)), ["yt:a", "sp:a", "yt:b", "sp:c", "sp:d"]);
        assert_eq!(intercalar(&listas, &preferencia(), 2).len(), 2);
        assert!(intercalar(&[], &preferencia(), 30).is_empty());
    }

    #[test]
    fn a_mesma_musica_leva_os_dois_selos_e_o_preferido_toca() {
        let listas = [vec![f("sp", "1", "Corrente", "Lume")], vec![f("yt", "9", "Corrente", "Lume")]];
        let j = intercalar(&listas, &preferencia(), 30);
        assert_eq!(j.len(), 1);
        assert_eq!(j[0].servicos(), ["yt", "sp"], "o preferido na frente, no lugar do outro");
        let j = intercalar(&listas, &com(&["sp"]), 30);
        assert_eq!(j[0].servicos(), ["sp", "yt"]);
        // Duas do mesmo serviço, iguais no nome: não se juntam.
        let iguais = [vec![f("sp", "1", "Corrente", "Lume"), f("sp", "2", "Corrente", "Lume")]];
        assert_eq!(intercalar(&iguais, &preferencia(), 30).len(), 2);
        // Três serviços, uma linha.
        let tres = [listas[0].clone(), listas[1].clone(), vec![f("sc", "5", "Corrente", "Lume")]];
        assert_eq!(intercalar(&tres, &preferencia(), 30)[0].servicos(), ["yt", "sp", "sc"]);
    }

    #[test]
    fn isrc_igual_junta_e_versao_diferente_nao() {
        let com_isrc = |servico: &str, titulo: &str, isrc: &str| {
            let Item::Faixa(mut x) = f(servico, "i", titulo, "Outra grafia") else { return f("", "", "", "") };
            x.isrc = Some(isrc.into());
            Item::Faixa(x)
        };
        let iguais = [vec![com_isrc("sp", "Nome", "BRX122600001")], vec![com_isrc("yt", "Nomë 2", "br-x12-26-00001")]];
        assert_eq!(intercalar(&iguais, &com(&[]), 30).len(), 1, "o ISRC decide");
        for outra in ["Corrente (Remix)", "Corrente (Ao Vivo)", "Corrente (Acústico)", "Corrente (Sped Up)"] {
            let listas = [vec![f("sp", "1", "Corrente", "Lume")], vec![f("yt", "2", outra, "Lume")]];
            assert_eq!(intercalar(&listas, &com(&[]), 30).len(), 2, "{outra} é outra gravação");
        }
        let outro_artista = [vec![f("sp", "1", "Corrente", "Lume")], vec![f("yt", "2", "Corrente", "Telha")]];
        assert_eq!(intercalar(&outro_artista, &com(&[]), 30).len(), 2, "nota baixa");
        let Item::Faixa(mut longa) = f("yt", "3", "Corrente", "Lume") else { return };
        longa.duracao_s = Some(420.0);
        let duracoes = [vec![f("sp", "1", "Corrente", "Lume")], vec![Item::Faixa(longa)]];
        assert_eq!(intercalar(&duracoes, &com(&[]), 30).len(), 2, "versão estendida");
    }

    #[test]
    fn o_que_a_pessoa_separou_fica_separado() {
        let listas = [vec![f("sp", "1", "Corrente", "Lume")], vec![f("yt", "9", "Corrente", "Lume")]];
        let mut s = Separadas::abrir(None);
        let (Item::Faixa(a), Item::Faixa(b)) = (&listas[0][0], &listas[1][0]) else { return };
        s.separar(a, std::slice::from_ref(b));
        let regras = Regras { separadas: s.pares(), ..preferencia() };
        assert_eq!(intercalar(&listas, &regras, 30).len(), 2);
        assert_eq!(par_separado("a", "b"), par_separado("b", "a"));
        // Guardado e lido de volta.
        let arquivo = std::env::temp_dir().join(format!("tccplayer-separadas-{}.json", std::process::id()));
        let _ = std::fs::remove_file(&arquivo);
        let mut s = Separadas::abrir(Some(arquivo.clone()));
        s.separar(b, std::slice::from_ref(a));
        for _ in 0..100 {
            if arquivo.is_file() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        assert_eq!(Separadas::abrir(Some(arquivo.clone())).pares(), s.pares());
        let _ = std::fs::remove_file(arquivo);
    }

    #[test]
    fn o_inicio_junta_por_tipo_e_intercala_as_sem_tipo() {
        let yt = [
            pr("Lançamentos", T::Lancamentos, vec![al("yt", "l1"), al("yt", "l2")]),
            pr("Clipes novos", T::Clipes, vec![f("yt", "c1", "Clipe", "X")]),
            pr("Mais tocadas", T::Paradas, vec![f("yt", "m1", "Uma", "X")]),
            pr("Em alta", T::Paradas, vec![f("yt", "m2", "Duas", "X")]),
            pr("Do YouTube", T::Outras, vec![al("yt", "o1")]),
        ];
        let sp = [
            pr("Para começar", T::Atalhos, vec![al("sp", "t1"), al("sp", "t2")]),
            pr("Novidades pra você", T::Lancamentos, vec![al("sp", "l3")]),
            pr("Uma", T::Outras, vec![al("sp", "o2")]),
            pr("Outra", T::Outras, vec![al("sp", "o3")]),
        ];
        let paginas = [("yt".to_string(), &yt[..]), ("sp".to_string(), &sp[..])];
        let i = juntar_inicio(&paginas, &[], &preferencia());
        let titulos: Vec<(&str, Option<&str>)> =
            i.fileiras.iter().map(|x| (x.titulo.as_str(), x.de.as_deref())).collect();
        assert_eq!(
            titulos,
            [
                ("Lançamentos", None),
                ("Do YouTube", Some("yt")),
                ("Uma", Some("sp")),
                ("Outra", Some("sp")),
                ("Clipes novos", Some("yt")),
                ("Mais tocadas", Some("yt")),
                ("Em alta", Some("yt")),
            ]
        );
        assert_eq!(ids(&i.fileiras[0].itens), ["yt:l1", "sp:l3", "yt:l2"]);
        assert_eq!(i.atalhos.len(), 2);
    }

    #[test]
    fn o_inicio_com_um_servico_so_ou_sem_resposta() {
        let sp = [pr("Feito para Ana", T::ParaVoce, vec![al("sp", "p1")])];
        let i = juntar_inicio(&[("sp".to_string(), &sp[..])], &[], &com(&[]));
        assert_eq!((i.fileiras.len(), i.fileiras[0].titulo.as_str()), (1, "Feito para Ana"), "o título dele");
        assert_eq!(i.fileiras[0].de.as_deref(), Some("sp"));
        // O outro respondeu vazio: igual.
        let i = juntar_inicio(&[("yt".to_string(), &[][..]), ("sp".to_string(), &sp[..])], &[], &com(&[]));
        assert_eq!(i.fileiras[0].de.as_deref(), Some("sp"));
        assert_eq!(juntar_inicio(&[], &[], &com(&[])), Inicio::default());
    }

    #[test]
    fn os_recentes_viram_atalhos_e_tocadas_ha_pouco() {
        let pl = Item::Playlist(Playlist { servico: "sp".into(), id: "pl".into(), ..Default::default() });
        let recentes =
            [f("yt", "1", "Um", "A"), al("sp", "t1"), f("sp", "2", "Dois", "B"), pl.clone(), f("yt", "3", "Três", "C")];
        let sp = [pr("Para começar", T::Atalhos, vec![al("sp", "t1"), al("sp", "t9")])];
        let i = juntar_inicio(&[("sp".to_string(), &sp[..])], &recentes, &com(&[]));
        assert_eq!(
            i.atalhos.iter().map(|x| chave(x).2).collect::<Vec<_>>(),
            ["t1", "pl", "t9"],
            "os abertos primeiro, sem repetir"
        );
        assert_eq!((i.fileiras[0].titulo.as_str(), i.fileiras[0].itens.len()), ("Tocadas há pouco", 3));
        // Poucas faixas: a fileira não aparece.
        assert!(juntar_inicio(&[], &recentes[..2], &com(&[])).fileiras.is_empty());
        let muitos: Vec<Item> = (0..30).map(|n| al("sp", &n.to_string())).collect();
        assert_eq!(juntar_inicio(&[], &muitos, &com(&[])).atalhos.len(), ATALHOS);
    }

    #[test]
    fn a_busca_junta_e_escolhe_o_melhor() {
        let artista = |s: &str, nome: &str| {
            Item::Artista(Artista { servico: s.into(), id: nome.into(), nome: nome.into(), foto: None })
        };
        let yt = GruposDaBusca {
            melhor: Some(f("yt", "1", "Corrente elétrica", "Lume")),
            faixas: vec![],
            artistas: vec![artista("yt", "Lume")],
            ..Default::default()
        };
        let sp = GruposDaBusca { melhor: Some(artista("sp", "Lume")), ..Default::default() };
        let b = juntar_busca("lume", &[&yt, &sp], &preferencia());
        assert_eq!(b.melhor.map(|j| j.item), Some(artista("sp", "Lume")), "o nome igual ganha");
        // Uma faixa nos dois: o melhor leva os dois selos.
        let Item::Faixa(a) = f("yt", "1", "Corrente", "Lume") else { return };
        let Item::Faixa(c) = f("sp", "2", "Corrente", "Lume") else { return };
        let um = GruposDaBusca { melhor: Some(Item::Faixa(a.clone())), faixas: vec![a], ..Default::default() };
        let dois = GruposDaBusca { melhor: Some(Item::Faixa(c.clone())), faixas: vec![c], ..Default::default() };
        let b = juntar_busca("corrente", &[&um, &dois], &preferencia());
        assert_eq!(b.musicas.len(), 1);
        assert_eq!(b.melhor.expect("melhor").servicos(), ["yt", "sp"]);
        assert_eq!(juntar_busca("x", &[], &com(&[])), BuscaJunta::default());
    }

    #[test]
    fn saudacao_e_servico_do_app() {
        assert_eq!(
            (saudacao(5), saudacao(12), saudacao(18), saudacao(3)),
            ("Bom dia", "Boa tarde", "Boa noite", "Boa noite")
        );
        let provedores = [("youtube-music".to_string(), "YouTube Music".to_string())];
        assert_eq!(servico_do_app("YouTube Music", &provedores).as_deref(), Some("youtube-music"));
        assert_eq!(servico_do_app("Spotify.exe", &provedores).as_deref(), Some("spotify"));
        assert_eq!(servico_do_app("Spotifast", &[]).as_deref(), Some("spotify"));
        assert_eq!(servico_do_app("Opera", &provedores), None);
        assert_eq!(servico_do_app("", &provedores), None);
    }
}
