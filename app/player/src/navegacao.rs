//! A navegação das páginas: os espaços (Tudo, o computador e um por serviço) e, em cada um, uma pilha de páginas
//! com voltar e avançar, como num navegador. Só lógica, testada; a tela lê [`Navegacao::atual`].

use provedores::Item;
use std::collections::HashMap;

/// Quantas páginas cada espaço lembra para trás.
const LIMITE: usize = 50;

/// Onde a pessoa está: todos os serviços juntos, as músicas do computador ou um serviço.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Espaco {
    Tudo,
    Computador,
    Servico(String),
}

impl Espaco {
    /// Como o espaço fica nos ajustes.
    pub fn chave(&self) -> String {
        match self {
            Self::Tudo => "tudo".into(),
            Self::Computador => "computador".into(),
            Self::Servico(s) => format!("servico:{s}"),
        }
    }

    /// O espaço de uma chave dos ajustes (uma chave estragada abre em Tudo).
    pub fn da_chave(s: &str) -> Self {
        match s {
            "computador" => Self::Computador,
            _ => match s.strip_prefix("servico:").filter(|x| !x.is_empty()) {
                Some(servico) => Self::Servico(servico.to_string()),
                None => Self::Tudo,
            },
        }
    }

    /// A página com que o espaço abre pela primeira vez.
    fn primeira(&self) -> Rota {
        match self {
            Self::Computador => Rota::Biblioteca,
            _ => Rota::ParaVoce,
        }
    }
}

/// Uma página. As de detalhe levam o item de onde vieram (com o serviço dele).
#[derive(Clone, Debug, PartialEq)]
pub enum Rota {
    /// As prateleiras do serviço ("Para você"); em Tudo, as de todos.
    ParaVoce,
    Busca,
    Biblioteca,
    Album(Item),
    Playlist(Item),
    Artista(Item),
    Faixa(Item),
    /// O que toca agora e o que vem depois (de todos os serviços).
    Fila,
    /// O que foi ouvido, em números.
    Historico,
    Loja,
    Configuracoes,
}

/// A página aberta mostra o campo de busca? (A Busca, e a Biblioteca de Tudo e do computador.) Só com ele na tela
/// o teclado vai para o campo.
pub fn tem_campo_de_busca(rota: Option<&Rota>, espaco: &Espaco) -> bool {
    match rota {
        Some(Rota::Busca) => true,
        Some(Rota::Biblioteca) => !matches!(espaco, Espaco::Servico(_)),
        _ => false,
    }
}

/// As abas do topo da folha em cada espaço. O computador não tem: a Biblioteca dele já tem a busca.
pub fn abas_do_espaco(espaco: &Espaco) -> Vec<(Rota, &'static str)> {
    match espaco {
        Espaco::Computador => Vec::new(),
        _ => vec![(Rota::ParaVoce, "Para você"), (Rota::Busca, "Busca"), (Rota::Biblioteca, "Biblioteca")],
    }
}

/// O histórico de um espaço. `None` é a folha fechada (só o visual, no Palco).
#[derive(Clone, Debug, Default)]
struct Pilha {
    voltar: Vec<Option<Rota>>,
    atual: Option<Rota>,
    avancar: Vec<Option<Rota>>,
}

impl Pilha {
    fn para(&mut self, nova: Option<Rota>) {
        if nova == self.atual {
            return;
        }
        self.voltar.push(std::mem::replace(&mut self.atual, nova));
        if self.voltar.len() > LIMITE {
            self.voltar.remove(0);
        }
        self.avancar.clear();
    }
}

pub struct Navegacao {
    espaco: Espaco,
    pilhas: HashMap<Espaco, Pilha>,
    /// Os espaços em que a Estante abriu a primeira página sozinha (a folha estava fechada no Palco).
    preenchidas: std::collections::HashSet<Espaco>,
}

impl Navegacao {
    /// Começa no espaço dado, sem página aberta.
    pub fn new(espaco: Espaco) -> Self {
        let mut pilhas = HashMap::new();
        pilhas.insert(espaco.clone(), Pilha::default());
        Self { espaco, pilhas, preenchidas: std::collections::HashSet::new() }
    }

    pub fn espaco(&self) -> &Espaco {
        &self.espaco
    }

    pub fn atual(&self) -> Option<&Rota> {
        self.pilhas.get(&self.espaco).and_then(|p| p.atual.as_ref())
    }

    fn pilha(&mut self) -> &mut Pilha {
        self.pilhas.entry(self.espaco.clone()).or_default()
    }

    /// Abre uma página: a de agora vai para trás e o "avançar" se perde. A mesma página não empilha.
    pub fn ir(&mut self, rota: Rota) {
        self.pilha().para(Some(rota));
    }

    /// Sem página aberta (a folha fechada do Palco), abre a primeira do espaço, sem empilhar nada: a Estante sempre
    /// mostra uma página.
    pub fn garantir_pagina(&mut self) {
        let primeira = self.espaco.primeira();
        let p = self.pilha();
        if p.atual.is_none() {
            p.atual = Some(primeira);
            self.preenchidas.insert(self.espaco.clone());
        }
    }

    /// De volta ao Palco: os espaços em que a Estante abriu a primeira página sozinha e em que nada mais foi aberto
    /// voltam com a folha fechada, como estavam.
    pub fn soltar_preenchidas(&mut self) {
        for e in std::mem::take(&mut self.preenchidas) {
            let primeira = e.primeira();
            if let Some(p) = self.pilhas.get_mut(&e)
                && p.voltar.is_empty()
                && p.avancar.is_empty()
                && p.atual.as_ref() == Some(&primeira)
            {
                p.atual = None;
            }
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

    /// A primeira página do espaço aberto.
    pub fn primeira(&self) -> Rota {
        self.espaco.primeira()
    }

    /// Fecha a folha (voltar a reabre).
    pub fn fechar(&mut self) {
        self.pilha().para(None);
    }

    pub fn voltar(&mut self) -> bool {
        let p = self.pilha();
        let Some(anterior) = p.voltar.pop() else { return false };
        p.avancar.push(std::mem::replace(&mut p.atual, anterior));
        true
    }

    pub fn avancar(&mut self) -> bool {
        let p = self.pilha();
        let Some(seguinte) = p.avancar.pop() else { return false };
        p.voltar.push(std::mem::replace(&mut p.atual, seguinte));
        true
    }

    pub fn pode_voltar(&self) -> bool {
        self.pilhas.get(&self.espaco).is_some_and(|p| !p.voltar.is_empty())
    }

    pub fn pode_avancar(&self) -> bool {
        self.pilhas.get(&self.espaco).is_some_and(|p| !p.avancar.is_empty())
    }

    /// Vai para outro espaço. Cada um guarda a pilha dele; um espaço novo abre na primeira página dele.
    pub fn trocar_espaco(&mut self, espaco: Espaco) {
        let primeira = espaco.primeira();
        let pilha = self.pilhas.entry(espaco.clone()).or_default();
        // Escolher um espaço é abrir uma página dele: um espaço que estava com a folha fechada abre na primeira.
        if pilha.atual.is_none() {
            pilha.atual = Some(primeira);
        }
        self.espaco = espaco;
    }

    /// O serviço saiu (desligado ou removido): a pilha dele some; se era o aberto, vai para Tudo.
    pub fn esquecer_servico(&mut self, servico: &str) {
        let e = Espaco::Servico(servico.to_string());
        self.pilhas.remove(&e);
        if self.espaco == e {
            self.espaco = Espaco::Tudo;
            self.pilhas.entry(Espaco::Tudo).or_default();
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn abas_de_cada_espaco() {
        let nomes = |e: Espaco| abas_do_espaco(&e).into_iter().map(|(_, n)| n).collect::<Vec<_>>();
        assert_eq!(nomes(Espaco::Servico("yt".into())), ["Para você", "Busca", "Biblioteca"]);
        assert_eq!(nomes(Espaco::Tudo), ["Para você", "Busca", "Biblioteca"]);
        assert!(nomes(Espaco::Computador).is_empty(), "o computador só tem a Biblioteca (com a busca dela)");
        assert_eq!(abas_do_espaco(&Espaco::Tudo)[1].0, Rota::Busca);
    }

    #[test]
    fn fila_abre_em_qualquer_espaco_e_voltar_volta() {
        let mut n = Navegacao::new(Espaco::Servico("yt".into()));
        n.trocar_espaco(Espaco::Servico("yt".into()));
        n.ir(Rota::Biblioteca);
        n.ir(Rota::Fila);
        assert_eq!(n.atual(), Some(&Rota::Fila), "a Fila, não a Biblioteca do serviço");
        assert_eq!(n.espaco(), &Espaco::Servico("yt".into()), "sem trocar de espaço");
        assert!(n.voltar());
        assert_eq!(n.atual(), Some(&Rota::Biblioteca));
    }

    #[test]
    fn o_campo_de_busca_so_existe_na_busca_e_na_biblioteca_do_computador() {
        let yt = Espaco::Servico("yt".into());
        assert!(tem_campo_de_busca(Some(&Rota::Busca), &yt));
        assert!(tem_campo_de_busca(Some(&Rota::Biblioteca), &Espaco::Tudo));
        assert!(tem_campo_de_busca(Some(&Rota::Biblioteca), &Espaco::Computador));
        assert!(!tem_campo_de_busca(Some(&Rota::Biblioteca), &yt), "a Biblioteca do serviço não tem campo");
        let album = Rota::Album(Item::Album(provedores::Album::default()));
        assert!(!tem_campo_de_busca(Some(&album), &Espaco::Tudo), "saiu da Busca para um álbum");
        assert!(!tem_campo_de_busca(Some(&Rota::Fila), &Espaco::Tudo));
        assert!(!tem_campo_de_busca(None, &Espaco::Tudo));
    }

    #[test]
    fn garantir_pagina_abre_a_primeira_sem_mexer_no_historico() {
        let mut n = Navegacao::new(Espaco::Servico("yt".into()));
        n.garantir_pagina();
        assert_eq!(n.atual(), Some(&Rota::ParaVoce));
        assert!(!n.pode_voltar(), "não vira uma página para trás");
        n.ir(Rota::Busca);
        n.garantir_pagina();
        assert_eq!(n.atual(), Some(&Rota::Busca), "com página aberta, nada muda");
        let mut c = Navegacao::new(Espaco::Computador);
        c.garantir_pagina();
        assert_eq!(c.atual(), Some(&Rota::Biblioteca));
    }

    #[test]
    fn soltar_as_preenchidas_devolve_a_folha_fechada() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.garantir_pagina();
        assert_eq!(n.atual(), Some(&Rota::ParaVoce));
        n.soltar_preenchidas();
        assert_eq!(n.atual(), None, "o Palco volta com a folha fechada, como estava");
        // Com navegação na Estante, a página que a pessoa abriu fica.
        n.garantir_pagina();
        n.ir(Rota::Busca);
        n.soltar_preenchidas();
        assert_eq!(n.atual(), Some(&Rota::Busca));
    }

    #[test]
    fn voltar_sem_vazias_pula_a_folha_fechada() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(Rota::Busca);
        n.fechar();
        n.ir(Rota::Loja);
        assert!(n.voltar_sem_vazias());
        assert_eq!(n.atual(), Some(&Rota::Busca), "a folha fechada do Palco fica de fora");
        assert!(!n.voltar_sem_vazias(), "antes só a folha fechada: fica onde está");
        assert_eq!(n.atual(), Some(&Rota::Busca));
        assert!(n.avancar_sem_vazias());
        assert_eq!(n.atual(), Some(&Rota::Loja));
    }

    #[test]
    fn ir_voltar_e_avancar() {
        let mut n = Navegacao::new(Espaco::Tudo);
        assert_eq!(n.atual(), None);
        assert!(!n.voltar() && !n.pode_voltar() && !n.pode_avancar());
        n.ir(Rota::Biblioteca);
        n.ir(Rota::Biblioteca);
        n.ir(Rota::Loja);
        assert!(n.voltar());
        assert_eq!(n.atual(), Some(&Rota::Biblioteca), "a repetida não empilhou");
        assert!(n.voltar());
        assert_eq!(n.atual(), None, "volta ao visual");
        assert!(!n.voltar());
        assert!(n.avancar() && n.avancar());
        assert_eq!(n.atual(), Some(&Rota::Loja));
        assert!(!n.avancar());
        n.voltar();
        n.ir(Rota::Configuracoes);
        assert!(!n.pode_avancar(), "ir apaga o avançar");
    }

    #[test]
    fn cada_espaco_guarda_a_sua_pilha() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(Rota::Loja);
        n.trocar_espaco(Espaco::Servico("youtube-music".into()));
        assert_eq!(n.atual(), Some(&Rota::ParaVoce));
        n.ir(Rota::Busca);
        n.trocar_espaco(Espaco::Computador);
        assert_eq!(n.atual(), Some(&Rota::Biblioteca));
        n.trocar_espaco(Espaco::Servico("youtube-music".into()));
        assert_eq!(n.atual(), Some(&Rota::Busca), "volta onde estava");
        n.esquecer_servico("youtube-music");
        assert_eq!(n.espaco(), &Espaco::Tudo);
        assert_eq!(n.atual(), Some(&Rota::Loja));
    }

    #[test]
    fn fechar_e_chaves() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.ir(Rota::Biblioteca);
        n.fechar();
        assert_eq!(n.atual(), None);
        assert!(n.voltar());
        assert_eq!(n.atual(), Some(&Rota::Biblioteca), "voltar reabre a folha");
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
        assert!(voltas <= 50, "a pilha tem limite");
    }

    #[test]
    fn trocar_para_um_espaco_fechado_abre_a_primeira_pagina() {
        let mut n = Navegacao::new(Espaco::Tudo);
        n.trocar_espaco(Espaco::Computador);
        n.fechar();
        n.trocar_espaco(Espaco::Tudo);
        assert_eq!(n.atual(), Some(&Rota::ParaVoce), "Tudo nunca aberto: a folha não some");
        n.trocar_espaco(Espaco::Computador);
        assert_eq!(n.atual(), Some(&Rota::Biblioteca), "fechado antes: reabre na primeira");
    }
}
