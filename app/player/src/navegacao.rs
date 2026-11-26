//! A navegação das páginas: três destinos (Início, Buscar, Biblioteca) que nunca mudam, o serviço como filtro deles e
//! uma pilha só de voltar e avançar, como num navegador. Só lógica, testada; a tela lê [`Navegacao::atual`].

use crate::ui::normalizar;
use biblioteca::TipoSalvo;
use provedores::Item;

/// Quantas páginas a pilha lembra para trás.
const LIMITE: usize = 50;

/// O filtro de serviço: todos juntos, as músicas do computador ou um serviço.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Espaco {
    Tudo,
    Computador,
    Servico(String),
}

impl Espaco {
    /// Como o filtro fica nos ajustes.
    pub fn chave(&self) -> String {
        match self {
            Self::Tudo => "tudo".into(),
            Self::Computador => "computador".into(),
            Self::Servico(s) => format!("servico:{s}"),
        }
    }

    /// O filtro de uma chave dos ajustes (uma chave estragada abre em Tudo).
    pub fn da_chave(s: &str) -> Self {
        match s {
            "computador" => Self::Computador,
            _ => match s.strip_prefix("servico:").filter(|x| !x.is_empty()) {
                Some(servico) => Self::Servico(servico.to_string()),
                None => Self::Tudo,
            },
        }
    }
}

/// As abas da Biblioteca.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AbaDaBiblioteca {
    #[default]
    Tudo,
    Playlists,
    Albuns,
    Artistas,
    Curtidas,
    /// As músicas das pastas do computador.
    Pastas,
    /// O que foi ouvido, em números.
    Historico,
}

impl AbaDaBiblioteca {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Tudo => "Tudo",
            Self::Playlists => "Playlists",
            Self::Albuns => "Álbuns",
            Self::Artistas => "Artistas",
            Self::Curtidas => "Curtidas",
            Self::Pastas => "Pastas",
            Self::Historico => "Histórico",
        }
    }

    /// O tipo de item que a aba mostra (as outras não são de um tipo só).
    pub fn tipo(self) -> Option<TipoSalvo> {
        match self {
            Self::Playlists => Some(TipoSalvo::Playlist),
            Self::Albuns => Some(TipoSalvo::Album),
            Self::Artistas => Some(TipoSalvo::Artista),
            Self::Curtidas => Some(TipoSalvo::Faixa),
            Self::Tudo | Self::Pastas | Self::Historico => None,
        }
    }
}

/// As abas que a Biblioteca tem em cada filtro.
pub fn abas_da_biblioteca(filtro: &Espaco) -> &'static [AbaDaBiblioteca] {
    use AbaDaBiblioteca as A;
    match filtro {
        Espaco::Tudo => &[A::Tudo, A::Playlists, A::Albuns, A::Artistas, A::Curtidas, A::Pastas, A::Historico],
        Espaco::Servico(_) => &[A::Tudo, A::Playlists, A::Albuns, A::Artistas, A::Curtidas, A::Historico],
        Espaco::Computador => &[A::Pastas, A::Historico],
    }
}

/// A aba que aparece de verdade: a escolhida, ou a primeira do filtro quando ela não vale nele.
pub fn aba_valida(filtro: &Espaco, aba: AbaDaBiblioteca) -> AbaDaBiblioteca {
    let abas = abas_da_biblioteca(filtro);
    if abas.contains(&aba) { aba } else { abas.first().copied().unwrap_or_default() }
}

/// Uma página. As de detalhe levam o item de onde vieram (com o serviço dele).
#[derive(Clone, Debug, PartialEq)]
pub enum Rota {
    Inicio,
    Busca,
    Biblioteca(AbaDaBiblioteca),
    Album(Item),
    Playlist(Item),
    Artista(Item),
    Faixa(Item),
    /// O que toca agora e o que vem depois (de todos os serviços).
    Fila,
    Loja,
    Configuracoes,
}

impl Rota {
    /// A página tem o filtro de serviço no alto?
    pub fn tem_filtro(&self) -> bool {
        matches!(self, Self::Inicio | Self::Busca | Self::Biblioteca(_))
    }

    /// As duas são o mesmo destino? (A Biblioteca é uma só, em qualquer aba.)
    pub fn mesmo_destino(&self, outra: &Self) -> bool {
        self == outra || matches!((self, outra), (Self::Biblioteca(_), Self::Biblioteca(_)))
    }

    /// O item da página de detalhe.
    pub fn item(&self) -> Option<&Item> {
        match self {
            Self::Album(i) | Self::Playlist(i) | Self::Artista(i) | Self::Faixa(i) => Some(i),
            _ => None,
        }
    }
}

/// Os campos de texto das páginas (um por vez recebe o teclado).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Campo {
    /// O do topo: a busca única.
    Busca,
    /// O filtro da Biblioteca.
    Biblioteca,
}

/// O campo está na tela com esta página aberta? Só com ele na tela o teclado vai para ele.
pub fn campo_na_tela(campo: Campo, rota: Option<&Rota>, filtro: &Espaco) -> bool {
    match (campo, rota) {
        (Campo::Busca, Some(Rota::Busca)) => true,
        (Campo::Biblioteca, Some(Rota::Biblioteca(aba))) => aba_valida(filtro, *aba) != AbaDaBiblioteca::Historico,
        _ => false,
    }
}

/// Os textos dos campos. O da busca e o da Biblioteca não se misturam: buscar no Spotify não esconde as músicas do
/// computador.
#[derive(Debug, Default)]
pub struct Buscas {
    pub servicos: String,
    pub biblioteca: String,
}

impl Buscas {
    pub fn do_campo(&self, campo: Campo) -> &str {
        match campo {
            Campo::Busca => &self.servicos,
            Campo::Biblioteca => &self.biblioteca,
        }
    }

    pub fn do_campo_mut(&mut self, campo: Campo) -> &mut String {
        match campo {
            Campo::Busca => &mut self.servicos,
            Campo::Biblioteca => &mut self.biblioteca,
        }
    }
}

/// O texto da busca pede só comandos (começa com `>`)? Devolve o que vem depois.
pub fn so_comandos(texto: &str) -> Option<&str> {
    texto.trim_start().strip_prefix('>').map(str::trim)
}

/// O que vai para os serviços e para o computador: nada quando o texto pede só comandos.
pub fn texto_para_buscar(texto: &str) -> &str {
    if so_comandos(texto).is_some() { "" } else { texto.trim() }
}

/// Põe a busca no começo das recentes, sem repetir (nem por acento ou maiúscula). Devolve se mudou.
pub fn lembrar_busca(recentes: &mut Vec<String>, texto: &str) -> bool {
    let texto = texto_para_buscar(texto);
    if texto.chars().count() < 2 || recentes.first().is_some_and(|r| r == texto) {
        return false;
    }
    let chave = normalizar(texto);
    recentes.retain(|r| normalizar(r) != chave);
    recentes.insert(0, texto.to_string());
    recentes.truncate(nucleo::ajustes::MAXIMO_DE_BUSCAS_RECENTES);
    true
}

/// Um passo da pilha: a página e o filtro com que ela estava. O `id` é a identidade da rolagem (a tela guarda uma por
/// passo).
#[derive(Clone, Debug)]
struct Passo {
    id: u64,
    /// `None` é a folha fechada (só o visual, no Palco).
    rota: Option<Rota>,
    filtro: Espaco,
}

pub struct Navegacao {
    voltar: Vec<Passo>,
    atual: Passo,
    avancar: Vec<Passo>,
    ultimo_id: u64,
    /// A Estante abriu a primeira página sozinha (a folha estava fechada no Palco).
    preenchida: bool,
}

impl Navegacao {
    /// Começa no filtro dado, sem página aberta.
    pub fn new(filtro: Espaco) -> Self {
        Self {
            voltar: Vec::new(),
            atual: Passo { id: 0, rota: None, filtro },
            avancar: Vec::new(),
            ultimo_id: 0,
            preenchida: false,
        }
    }

    /// O filtro de serviço de agora.
    pub fn espaco(&self) -> &Espaco {
        &self.atual.filtro
    }

    pub fn atual(&self) -> Option<&Rota> {
        self.atual.rota.as_ref()
    }

    /// A identidade do passo de agora (a rolagem dele).
    pub fn passo(&self) -> u64 {
        self.atual.id
    }

    /// O passo ainda está na pilha? (A tela esquece a rolagem dos que saíram.)
    pub fn tem_passo(&self, id: u64) -> bool {
        self.atual.id == id || self.voltar.iter().chain(&self.avancar).any(|p| p.id == id)
    }

    /// A aba que a Biblioteca aberta mostra (fora dela, a primeira).
    pub fn aba(&self) -> AbaDaBiblioteca {
        match self.atual() {
            Some(Rota::Biblioteca(aba)) => aba_valida(self.espaco(), *aba),
            _ => AbaDaBiblioteca::default(),
        }
    }

    fn novo_id(&mut self) -> u64 {
        self.ultimo_id += 1;
        self.ultimo_id
    }

    fn para(&mut self, rota: Option<Rota>) {
        if rota == self.atual.rota {
            return;
        }
        let novo = Passo { id: self.novo_id(), rota, filtro: self.atual.filtro.clone() };
        self.voltar.push(std::mem::replace(&mut self.atual, novo));
        if self.voltar.len() > LIMITE {
            self.voltar.remove(0);
        }
        self.avancar.clear();
    }

    /// Abre uma página, com o filtro de agora: a de agora vai para trás e o "avançar" se perde. A mesma não empilha.
    pub fn ir(&mut self, rota: Rota) {
        self.para(Some(rota));
    }

    /// Escolhe o filtro. Numa página com filtro, ela continua aberta, do começo, e não vira um passo; nas outras (e
    /// com a folha fechada), abre o Início nele.
    pub fn trocar_filtro(&mut self, filtro: Espaco) {
        if self.atual().is_some_and(Rota::tem_filtro) {
            if self.atual.filtro != filtro {
                self.atual.id = self.novo_id();
                self.atual.filtro = filtro;
            }
        } else {
            self.ir(Rota::Inicio);
            self.atual.filtro = filtro;
        }
    }

    /// Troca a aba da Biblioteca aberta (não vira um passo); fora dela, abre a Biblioteca na aba.
    pub fn trocar_aba(&mut self, aba: AbaDaBiblioteca) {
        match self.atual() {
            Some(Rota::Biblioteca(agora)) if *agora == aba => {}
            Some(Rota::Biblioteca(_)) => {
                self.atual.id = self.novo_id();
                self.atual.rota = Some(Rota::Biblioteca(aba));
            }
            _ => self.ir(Rota::Biblioteca(aba)),
        }
    }

    /// Sem página aberta (a folha fechada do Palco), abre o Início, sem empilhar nada: a Estante sempre mostra uma
    /// página.
    pub fn garantir_pagina(&mut self) {
        if self.atual.rota.is_none() {
            self.atual.rota = Some(Rota::Inicio);
            self.preenchida = true;
        }
    }

    /// De volta ao Palco: se a Estante abriu o Início sozinha e nada mais foi aberto, a folha volta fechada.
    pub fn soltar_preenchida(&mut self) {
        let so_ela = self.voltar.is_empty() && self.avancar.is_empty();
        if std::mem::take(&mut self.preenchida) && so_ela && self.atual() == Some(&Rota::Inicio) {
            self.atual.rota = None;
        }
    }

    /// Voltar pulando a folha fechada (que o Palco empilha): a Estante sempre tem página.
    pub fn voltar_sem_vazias(&mut self) -> bool {
        let mut passos = 0;
        while self.voltar() {
            passos += 1;
            if self.atual().is_some() {
                return true;
            }
        }
        // Só havia a folha fechada: fica onde estava.
        for _ in 0..passos {
            self.avancar();
        }
        false
    }

    /// Avançar pulando a folha fechada.
    pub fn avancar_sem_vazias(&mut self) -> bool {
        let mut passos = 0;
        while self.avancar() {
            passos += 1;
            if self.atual().is_some() {
                return true;
            }
        }
        for _ in 0..passos {
            self.voltar();
        }
        false
    }

    /// Fecha a folha (voltar a reabre).
    pub fn fechar(&mut self) {
        self.para(None);
    }

    pub fn voltar(&mut self) -> bool {
        let Some(anterior) = self.voltar.pop() else { return false };
        self.avancar.push(std::mem::replace(&mut self.atual, anterior));
        true
    }

    pub fn avancar(&mut self) -> bool {
        let Some(seguinte) = self.avancar.pop() else { return false };
        self.voltar.push(std::mem::replace(&mut self.atual, seguinte));
        true
    }

    /// Dá para voltar? Na Estante, a folha fechada não conta (`sem_vazias`).
    pub fn pode_voltar(&self, sem_vazias: bool) -> bool {
        self.voltar.iter().any(|p| !sem_vazias || p.rota.is_some())
    }

    pub fn pode_avancar(&self, sem_vazias: bool) -> bool {
        self.avancar.iter().any(|p| !sem_vazias || p.rota.is_some())
    }

    /// O serviço saiu (desligado ou removido): o filtro dele vira Tudo em toda a pilha e as páginas dele somem (a
    /// aberta vira o Início). Passos que ficaram iguais ao vizinho se juntam.
    pub fn esquecer_servico(&mut self, servico: &str) {
        let do_servico = |p: &Passo| {
            p.rota.as_ref().and_then(Rota::item).is_some_and(|i| crate::paginas::chave_do_salvo(i).0 == servico)
        };
        if do_servico(&self.atual) {
            self.atual.rota = Some(Rota::Inicio);
            self.atual.id = self.novo_id();
        }
        let agora = self.atual.rota.clone();
        for lado in [&mut self.voltar, &mut self.avancar] {
            // Do passo mais perto do de agora (o fim da lista) para o mais longe.
            let mut vizinho = agora.clone();
            let mut ficam: Vec<Passo> = Vec::with_capacity(lado.len());
            for p in lado.drain(..).rev().filter(|p| !do_servico(p)) {
                if p.rota != vizinho {
                    vizinho.clone_from(&p.rota);
                    ficam.push(p);
                }
            }
            ficam.reverse();
            *lado = ficam;
        }
        let filtro = Espaco::Servico(servico.to_string());
        for p in self.voltar.iter_mut().chain(&mut self.avancar).chain(std::iter::once(&mut self.atual)) {
            if p.filtro == filtro {
                p.filtro = Espaco::Tudo;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use AbaDaBiblioteca as A;

    fn yt() -> Espaco {
        Espaco::Servico("yt".into())
    }

    fn album(servico: &str, id: &str) -> Rota {
        Rota::Album(Item::Album(provedores::Album { servico: servico.into(), id: id.into(), ..Default::default() }))
    }

    #[test]
    fn a_busca_dos_servicos_nao_vaza_para_a_biblioteca() {
        let mut b = Buscas::default();
        b.do_campo_mut(Campo::Busca).push_str("lume");
        assert_eq!(b.do_campo(Campo::Busca), "lume");
        assert_eq!(b.do_campo(Campo::Biblioteca), "", "a Biblioteca não foi filtrada");
        b.do_campo_mut(Campo::Biblioteca).push_str("demos");
        assert_eq!((b.servicos.as_str(), b.biblioteca.as_str()), ("lume", "demos"));
    }

    #[test]
    fn abas_da_biblioteca_em_cada_filtro() {
        assert!(abas_da_biblioteca(&Espaco::Tudo).contains(&A::Pastas));
        assert!(!abas_da_biblioteca(&yt()).contains(&A::Pastas), "um serviço não tem pastas");
        assert_eq!(abas_da_biblioteca(&Espaco::Computador), &[A::Pastas, A::Historico]);
        assert_eq!(aba_valida(&Espaco::Tudo, A::Pastas), A::Pastas);
        assert_eq!(aba_valida(&yt(), A::Pastas), A::Tudo, "a aba que não vale cai na primeira");
        assert_eq!(aba_valida(&Espaco::Computador, A::Curtidas), A::Pastas);
        assert_eq!((A::Curtidas.tipo(), A::Tudo.tipo()), (Some(TipoSalvo::Faixa), None));
    }

    #[test]
    fn cada_campo_so_vale_na_pagina_dele() {
        assert!(campo_na_tela(Campo::Busca, Some(&Rota::Busca), &yt()));
        assert!(!campo_na_tela(Campo::Busca, Some(&album("yt", "a")), &yt()), "saiu da busca para um álbum");
        assert!(campo_na_tela(Campo::Biblioteca, Some(&Rota::Biblioteca(A::Pastas)), &Espaco::Tudo));
        assert!(!campo_na_tela(Campo::Biblioteca, Some(&Rota::Biblioteca(A::Historico)), &Espaco::Tudo));
        assert!(!campo_na_tela(Campo::Biblioteca, Some(&Rota::Busca), &Espaco::Tudo));
        assert!(!campo_na_tela(Campo::Busca, None, &Espaco::Tudo));
    }

    #[test]
    fn so_com_o_sinal_de_maior_pede_comandos() {
        assert_eq!(so_comandos("> tema papel "), Some("tema papel"));
        assert_eq!(so_comandos(">"), Some(""));
        assert_eq!(so_comandos("tema > papel"), None);
        assert_eq!(texto_para_buscar(" lume "), "lume");
        assert_eq!(texto_para_buscar(">lume"), "", "os serviços não recebem o comando");
    }

    #[test]
    fn as_buscas_recentes_nao_repetem() {
        let mut r = Vec::new();
        assert!(lembrar_busca(&mut r, "Lume") && lembrar_busca(&mut r, "ação"));
        assert!(lembrar_busca(&mut r, " lume "), "volta para o começo");
        assert_eq!(r, ["lume", "ação"]);
        assert!(!lembrar_busca(&mut r, "lume"), "a mesma de novo não muda nada");
        assert!(lembrar_busca(&mut r, "ACAO"));
        assert_eq!(r, ["ACAO", "lume"], "acento e maiúscula não contam");
        assert!(!lembrar_busca(&mut r, "x") && !lembrar_busca(&mut r, ">tema") && !lembrar_busca(&mut r, "  "));
        for i in 0..30 {
            lembrar_busca(&mut r, &format!("busca {i}"));
        }
        assert_eq!(r.len(), nucleo::ajustes::MAXIMO_DE_BUSCAS_RECENTES);
        assert_eq!(r[0], "busca 29");
    }

    #[test]
    fn ir_voltar_e_avancar() {
        let mut n = Navegacao::new(Espaco::Tudo);
        assert_eq!(n.atual(), None);
        assert!(!n.voltar() && !n.pode_voltar(false) && !n.pode_avancar(false));
        n.ir(Rota::Busca);
        n.ir(Rota::Busca);
        n.ir(Rota::Loja);
        assert!(n.voltar());
        assert_eq!(n.atual(), Some(&Rota::Busca), "a repetida não empilhou");
        assert!(n.voltar());
        assert_eq!(n.atual(), None, "volta ao visual");
        assert!(!n.voltar());
        assert!(n.avancar() && n.avancar());
        assert_eq!(n.atual(), Some(&Rota::Loja));
        assert!(!n.avancar());
        n.voltar();
        n.ir(Rota::Configuracoes);
        assert!(!n.pode_avancar(false), "ir apaga o avançar");
    }

    #[test]
    fn o_passo_guarda_a_rota_o_filtro_e_a_rolagem() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(Rota::Inicio);
        let inicio = n.passo();
        n.ir(Rota::Busca);
        assert_ne!(n.passo(), inicio, "cada passo tem a rolagem dele");
        n.trocar_filtro(yt());
        n.ir(album("yt", "a"));
        assert_eq!(n.espaco(), &yt(), "o filtro vai junto");
        assert!(n.voltar());
        assert_eq!((n.atual(), n.espaco()), (Some(&Rota::Busca), &yt()));
        assert!(n.voltar());
        assert_eq!((n.atual(), n.espaco(), n.passo()), (Some(&Rota::Inicio), &Espaco::Tudo, inicio));
        assert!(n.tem_passo(inicio) && !n.tem_passo(9999));
    }

    #[test]
    fn trocar_o_filtro_ou_a_aba_nao_vira_passo() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(Rota::Biblioteca(A::Tudo));
        let antes = n.passo();
        n.trocar_filtro(yt());
        n.trocar_aba(A::Albuns);
        assert_eq!(n.atual(), Some(&Rota::Biblioteca(A::Albuns)), "a mesma página");
        assert_ne!(n.passo(), antes, "do começo");
        let agora = n.passo();
        n.trocar_filtro(yt());
        n.trocar_aba(A::Albuns);
        assert_eq!(n.passo(), agora, "o mesmo filtro e a mesma aba não mexem em nada");
        assert!(n.voltar());
        assert_eq!(n.atual(), None);
        assert!(!n.pode_voltar(false));
        n.avancar();
        assert_eq!((n.espaco(), n.aba()), (&yt(), A::Albuns));
        n.trocar_filtro(Espaco::Computador);
        assert_eq!(n.aba(), A::Pastas, "Álbuns não vale no computador");
        n.trocar_filtro(yt());
        assert_eq!(n.aba(), A::Albuns, "e volta a valer no serviço");
    }

    #[test]
    fn o_filtro_numa_pagina_sem_ele_abre_o_inicio() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(album("yt", "a"));
        n.trocar_filtro(Espaco::Computador);
        assert_eq!((n.atual(), n.espaco()), (Some(&Rota::Inicio), &Espaco::Computador));
        assert!(n.voltar());
        assert_eq!((n.atual(), n.espaco()), (Some(&album("yt", "a")), &Espaco::Tudo));
        let mut fechada = Navegacao::new(Espaco::Tudo);
        fechada.trocar_filtro(yt());
        assert_eq!((fechada.atual(), fechada.espaco()), (Some(&Rota::Inicio), &yt()), "a folha fechada abre");
    }

    #[test]
    fn o_provedor_que_sai_no_meio_some_da_pilha() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(Rota::Inicio);
        n.ir(album("yt", "a"));
        n.ir(Rota::Inicio);
        n.trocar_filtro(yt());
        n.ir(album("yt", "b"));
        n.ir(album("sp", "c"));
        n.ir(album("yt", "d"));
        n.esquecer_servico("yt");
        assert_eq!((n.atual(), n.espaco()), (Some(&Rota::Inicio), &Espaco::Tudo), "a página aberta era dele");
        assert!(n.voltar());
        assert_eq!(n.atual(), Some(&album("sp", "c")));
        assert!(n.voltar());
        assert_eq!((n.atual(), n.espaco()), (Some(&Rota::Inicio), &Espaco::Tudo), "o filtro dele virou Tudo");
        assert!(n.voltar());
        assert_eq!(n.atual(), None, "os dois Inícios vizinhos viraram um");
        assert!(!n.voltar());
    }

    #[test]
    fn garantir_e_soltar_a_pagina() {
        let mut n = Navegacao::new(yt());
        n.garantir_pagina();
        assert_eq!(n.atual(), Some(&Rota::Inicio));
        assert!(!n.pode_voltar(false), "não vira uma página para trás");
        n.soltar_preenchida();
        assert_eq!(n.atual(), None, "o Palco volta com a folha fechada, como estava");
        n.garantir_pagina();
        n.ir(Rota::Busca);
        n.garantir_pagina();
        n.soltar_preenchida();
        assert_eq!(n.atual(), Some(&Rota::Busca), "a página que a pessoa abriu fica");
    }

    #[test]
    fn voltar_sem_vazias_pula_a_folha_fechada() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(Rota::Busca);
        n.fechar();
        n.ir(Rota::Loja);
        assert!(n.pode_voltar(true));
        assert!(n.voltar_sem_vazias());
        assert_eq!(n.atual(), Some(&Rota::Busca), "a folha fechada do Palco fica de fora");
        assert!(!n.pode_voltar(true) && n.pode_voltar(false));
        assert!(!n.voltar_sem_vazias(), "antes só a folha fechada: fica onde está");
        assert_eq!(n.atual(), Some(&Rota::Busca));
        assert!(n.avancar_sem_vazias());
        assert_eq!(n.atual(), Some(&Rota::Loja));
    }

    #[test]
    fn fechar_chaves_e_limite() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(Rota::Busca);
        n.fechar();
        assert_eq!(n.atual(), None);
        assert!(n.voltar());
        assert_eq!(n.atual(), Some(&Rota::Busca), "voltar reabre a folha");
        for e in [Espaco::Tudo, Espaco::Computador, Espaco::Servico("x".into())] {
            assert_eq!(Espaco::da_chave(&e.chave()), e);
        }
        assert_eq!(Espaco::da_chave("lixo"), Espaco::Tudo);
        for _ in 0..80 {
            n.ir(Rota::Loja);
            n.ir(Rota::Configuracoes);
        }
        let mut voltas = 0;
        while n.voltar() {
            voltas += 1;
        }
        assert!(voltas <= LIMITE, "a pilha tem limite");
    }

    #[test]
    fn a_biblioteca_e_um_destino_so() {
        assert!(Rota::Biblioteca(A::Tudo).mesmo_destino(&Rota::Biblioteca(A::Curtidas)));
        assert!(!Rota::Inicio.mesmo_destino(&Rota::Busca));
    }
}
