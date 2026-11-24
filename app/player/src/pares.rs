//! O Match: a faixa de um serviço que não entrega áudio (o Spotify) toca pela mesma música noutro serviço (o YouTube
//! Music). O casador da F1 escolhe o par, que fica guardado em `<dados>/pares.json` (só ids e títulos, nunca áudio).

use crate::remoto::{InfoDoProvedor, Situacao};
use provedores::{Capacidade, Faixa, Item, SituacaoDoTocador};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

/// Quantos pares ficam guardados.
const GUARDADOS: usize = 20_000;

/// O par de uma faixa noutro serviço.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Par {
    pub servico: String,
    pub id: String,
    /// Como a música se chama lá (para a pessoa conferir se é a mesma).
    pub titulo: String,
    pub artista: String,
    /// Os que a pessoa recusou ("Procurar outro par").
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub recusados: Vec<String>,
}

impl Par {
    /// A chave da gravação que toca (a pré-análise é dela, e não da faixa do serviço sem áudio).
    pub fn chave(&self) -> String {
        format!("{}:{}", self.servico, self.id)
    }
}

/// Quem toca as faixas de um serviço.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum QuemToca {
    /// O próprio provedor.
    Ele,
    /// Ainda não se sabe: quem poderia tocar está abrindo. A faixa espera.
    Abrindo,
    /// O tocador do próprio serviço (o Soloist): o som não passa pelo Player.
    Sozinho,
    /// Outro serviço, pelo Match.
    Outro(String),
    Ninguem,
}

/// `tocador`: como está o tocador do próprio serviço (desligado, se ele não tem um).
pub fn quem_toca(provedores: &[InfoDoProvedor], servico: &str, tocador: SituacaoDoTocador) -> QuemToca {
    let ele = provedores.iter().find(|p| p.servico == servico);
    if let Some(p) = ele
        && (p.situacao == Situacao::Abrindo || toca(p, &[Capacidade::Audio]))
    {
        return if p.situacao == Situacao::Abrindo { QuemToca::Abrindo } else { QuemToca::Ele };
    }
    // Abrindo também: a faixa espera os poucos segundos dele.
    if ele.is_some_and(|p| toca(p, &[Capacidade::TocaSozinho]))
        && matches!(tocador, SituacaoDoTocador::Pronto | SituacaoDoTocador::Abrindo)
    {
        return QuemToca::Sozinho;
    }
    let mut outros = provedores.iter().filter(|p| p.servico != servico);
    if let Some(p) = outros.clone().find(|p| toca(p, &[Capacidade::Busca, Capacidade::Audio])) {
        return QuemToca::Outro(p.servico.clone());
    }
    if outros.any(|p| p.situacao == Situacao::Abrindo) { QuemToca::Abrindo } else { QuemToca::Ninguem }
}

fn toca(p: &InfoDoProvedor, com: &[Capacidade]) -> bool {
    p.situacao == Situacao::Pronto && com.iter().all(|c| p.capacidades.contains(c))
}

/// O texto a buscar no outro serviço: o título sem o ruído ("Remastered", "Official Video") e o primeiro artista.
pub fn consulta(alvo: &Faixa) -> String {
    let artista = alvo.artistas.first().and_then(|a| a.split(", ").next()).unwrap_or_default();
    format!("{} {artista}", provedores::casador::sem_ruido(&alvo.titulo).trim()).trim().to_string()
}

/// O melhor par entre os resultados da busca, fora os recusados.
pub fn escolher(alvo: &Faixa, resultados: &[Item], recusados: &[String]) -> Option<Par> {
    let candidatas: Vec<Faixa> =
        resultados.iter().filter_map(Item::faixa).filter(|f| !recusados.contains(&f.id)).cloned().collect();
    let (i, _) = provedores::melhor_par(alvo, &candidatas)?;
    let f = &candidatas[i];
    Some(Par {
        servico: f.servico.clone(),
        id: f.id.clone(),
        titulo: f.titulo.clone(),
        artista: f.artistas_juntos(),
        recusados: recusados.to_vec(),
    })
}

/// Os pares guardados, pela chave (`servico:id`) da faixa de origem.
#[derive(Default)]
pub struct Pares {
    arquivo: Option<PathBuf>,
    pares: HashMap<String, Par>,
}

impl Pares {
    pub fn abrir(arquivo: Option<PathBuf>) -> Self {
        let texto = arquivo.as_ref().and_then(|a| std::fs::read_to_string(a).ok());
        let pares = texto.and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Self { arquivo, pares }
    }

    pub fn de(&self, chave: &str) -> Option<&Par> {
        self.pares.get(chave)
    }

    /// Guarda o par e devolve o que gravar no disco (quem chama grava depois de soltar a trava: a tela lê os pares a
    /// cada quadro).
    #[must_use]
    pub fn guardar(&mut self, chave: &str, par: Par) -> Option<AGravar> {
        if self.pares.len() >= GUARDADOS && !self.pares.contains_key(chave) {
            return None;
        }
        self.pares.insert(chave.to_string(), par);
        let arquivo = self.arquivo.clone()?;
        serde_json::to_string(&self.pares).ok().map(|texto| AGravar { arquivo, texto })
    }
}

/// Os pares, prontos para ir ao disco.
pub struct AGravar {
    arquivo: PathBuf,
    texto: String,
}

impl AGravar {
    pub fn gravar(self) {
        static UM_POR_VEZ: Mutex<()> = Mutex::new(());
        let _um = UM_POR_VEZ.lock().unwrap_or_else(PoisonError::into_inner);
        // Por um arquivo ao lado: fechar o Player no meio não estraga o que já estava guardado.
        let novo = self.arquivo.with_extension("json.novo");
        let gravou = std::fs::write(&novo, self.texto).and_then(|()| std::fs::rename(&novo, &self.arquivo));
        if let Err(e) = gravou {
            log::warn!("os pares do Match não foram guardados: {e}");
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn info(servico: &str, situacao: Situacao, capacidades: &[Capacidade]) -> InfoDoProvedor {
        InfoDoProvedor {
            nome: servico.into(),
            servico: servico.into(),
            versao: String::new(),
            capacidades: capacidades.to_vec(),
            situacao,
            embutido: false,
            cor: None,
            logo: None,
            conta: None,
        }
    }

    const TUDO: [Capacidade; 2] = [Capacidade::Busca, Capacidade::Audio];

    #[test]
    fn quem_toca_cada_servico() {
        let yt = info("yt", Situacao::Pronto, &TUDO);
        let spotify = info("spotify", Situacao::Pronto, &[Capacidade::Busca]);
        let com = |p: &[InfoDoProvedor], s: &str| quem_toca(p, s, SituacaoDoTocador::Desligado);
        let todos = [spotify.clone(), yt.clone()];
        assert_eq!(com(&todos, "yt"), QuemToca::Ele);
        assert_eq!(com(&todos, "spotify"), QuemToca::Outro("yt".into()));
        // Sem o YouTube Music pronto, ninguém toca o Spotify (e um que só tem áudio não serve: é preciso buscar).
        assert_eq!(com(std::slice::from_ref(&spotify), "spotify"), QuemToca::Ninguem);
        let desligado = info("yt", Situacao::Desligado, &TUDO);
        assert_eq!(com(&[spotify.clone(), desligado], "spotify"), QuemToca::Ninguem);
        let so_audio = info("yt", Situacao::Pronto, &[Capacidade::Audio]);
        assert_eq!(com(&[spotify, so_audio], "spotify"), QuemToca::Ninguem);
        // O próprio serviço desligado ou que não abriu: a música ainda dá para achar no outro.
        let caido = info("spotify", Situacao::Falhou("x".into()), &[]);
        assert_eq!(com(&[caido, yt.clone()], "spotify"), QuemToca::Outro("yt".into()));
        assert_eq!(com(std::slice::from_ref(&yt), "spotify"), QuemToca::Outro("yt".into()));
        // Abrindo (ele, ou quem tocaria no lugar dele): ainda não se sabe, e a faixa espera.
        let abrindo = info("yt", Situacao::Abrindo, &[]);
        assert_eq!(
            com(&[info("spotify", Situacao::Pronto, &[Capacidade::Busca]), abrindo], "spotify"),
            QuemToca::Abrindo
        );
        assert_eq!(com(&[info("spotify", Situacao::Abrindo, &[]), yt], "spotify"), QuemToca::Abrindo);
    }

    #[test]
    fn o_tocador_do_servico_vem_antes_do_match() {
        let spotify = info("spotify", Situacao::Pronto, &[Capacidade::Busca, Capacidade::TocaSozinho]);
        let yt = info("yt", Situacao::Pronto, &TUDO);
        let todos = [spotify, yt];
        let com = |t| quem_toca(&todos, "spotify", t);
        assert_eq!(com(SituacaoDoTocador::Pronto), QuemToca::Sozinho);
        assert_eq!(com(SituacaoDoTocador::Abrindo), QuemToca::Sozinho);
        for t in [
            SituacaoDoTocador::Desligado,
            SituacaoDoTocador::Baixando,
            SituacaoDoTocador::Pareando,
            SituacaoDoTocador::Falhou,
        ] {
            assert_eq!(com(t), QuemToca::Outro("yt".into()), "{t:?}");
        }
        assert_eq!(quem_toca(&todos, "yt", SituacaoDoTocador::Pronto), QuemToca::Ele);
    }

    fn faixa(id: &str, titulo: &str, artista: &str, duracao: f64, video: bool) -> Item {
        Item::Faixa(Faixa {
            servico: "yt".into(),
            id: id.into(),
            titulo: titulo.into(),
            artistas: vec![artista.into()],
            duracao_s: Some(duracao),
            video,
            ..Default::default()
        })
    }

    fn alvo() -> Faixa {
        Faixa {
            servico: "spotify".into(),
            id: "s1".into(),
            titulo: "Canção do Mar - Remastered 2011".into(),
            artistas: vec!["Ana, Bia".into()],
            duracao_s: Some(200.0),
            ..Default::default()
        }
    }

    #[test]
    fn a_consulta_e_o_titulo_limpo_e_o_primeiro_artista() {
        assert_eq!(consulta(&alvo()), "Canção do Mar Ana");
        assert_eq!(consulta(&Faixa { titulo: "Só".into(), ..Default::default() }), "Só");
        let remix = Faixa { titulo: "Nome (Remix)".into(), artistas: vec!["X".into()], ..Default::default() };
        assert_eq!(consulta(&remix), "Nome (Remix) X", "o que muda a gravação fica");
    }

    #[test]
    fn escolhe_a_mesma_gravacao() {
        let resultados = [
            faixa("clipe", "Ana - Canção do Mar (Official Video)", "Ana", 240.0, true),
            faixa("certa", "Canção do Mar", "Ana", 201.0, false),
            faixa("outra", "Outra Música", "Ana", 200.0, false),
            Item::Artista(Default::default()),
        ];
        let par = escolher(&alvo(), &resultados, &[]).expect("par");
        assert_eq!((par.servico.as_str(), par.id.as_str(), par.titulo.as_str()), ("yt", "certa", "Canção do Mar"));
        assert_eq!(par.chave(), "yt:certa");
        let outro = escolher(&alvo(), &resultados, &["certa".into()]).expect("o seguinte");
        assert_eq!(outro.id, "clipe");
        assert_eq!(outro.recusados, ["certa"], "os recusados ficam com o par");
        let nenhum = ["certa".to_string(), "clipe".to_string()];
        assert_eq!(escolher(&alvo(), &resultados, &nenhum).map(|p| p.id), None, "outra música não serve");
        assert_eq!(escolher(&alvo(), &[], &[]), None);
    }

    #[test]
    fn os_pares_ficam_guardados() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-pares-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).expect("pasta");
        let arquivo = pasta.join("pares.json");
        let _ = std::fs::remove_file(&arquivo);
        let par = Par { servico: "yt".into(), id: "v".into(), titulo: "T".into(), ..Default::default() };
        let mut p = Pares::abrir(Some(arquivo.clone()));
        assert_eq!(p.de("spotify:s1"), None);
        p.guardar("spotify:s1", par.clone()).expect("a gravar").gravar();
        assert_eq!(Pares::abrir(Some(arquivo.clone())).de("spotify:s1"), Some(&par), "na próxima abertura");
        std::fs::write(&arquivo, "estragado").expect("estragar");
        assert_eq!(Pares::abrir(Some(arquivo)).de("spotify:s1"), None, "estragado começa vazio");
        let mut sem = Pares::abrir(None);
        assert!(sem.guardar("a", par.clone()).is_none());
        assert_eq!(sem.de("a"), Some(&par), "sem pasta, vale nesta abertura");
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
