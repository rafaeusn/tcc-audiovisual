//! A Ilha: a lógica (sem desenho) do entalhe preso ao topo da tela.
//!
//! Quatro formas: oculta, repouso (capa e espectro), aviso (alarga e desce um pouco para avisar algo) e aberta (os
//! controles, e a letra ao lado se a pessoa quiser). Quem decide é [`Controle::passo`], a cada quadro, a partir do
//! que está acontecendo ([`Entrada`]). O desenho fica em `ui/ilha.rs`; a parte do Windows, em `integracao::ilha`.
//! Os tamanhos e tempos seguem o Nusa e o island (exuupery): abre quase na hora, fecha logo ao sair.

use std::time::{Duration, Instant};

pub mod encaixe;
pub use encaixe::*;

/// O ponteiro em cima abre a Ilha na hora: o tempo que se vê é só o da animação.
pub const ESPERA_PARA_ABRIR: Duration = Duration::ZERO;
/// Ponteiro fora por este tempo fecha a Ilha aberta: quase na hora, só para não piscar quando o mouse roça a borda.
pub const ESPERA_PARA_FECHAR: Duration = Duration::from_millis(80);
/// Parada há mais que isso, a Ilha some (pausar um instante não a esconde).
pub const PARADA_SOME_S: f32 = 30.0;
/// Sem faixa por este tempo, a Ilha some. Trocar de origem (música local ↔ outro app) apaga a faixa por um
/// instante, e a Ilha não deve piscar nem fechar por isso.
pub const SEM_FAIXA_SOME: Duration = Duration::from_secs(2);
/// A contagem do drop aparece quando falta até isso para ele.
pub const AVISO_DO_DROP_S: f32 = 3.5;

/// A maior forma, no tamanho padrão: a aberta.
pub(crate) const ABERTA: (f32, f32, f32, f32) = (560.0, 228.0, 36.0, 24.0);
/// As molas da forma (resposta em segundos, amortecimento): crescer tem impulso, com um leve quique, como o Nusa;
/// encolher assenta sem passar. Curtas: só o tempo de a mudança ser vista como movimento, não como salto.
pub const MOLA_CRESCER: (f32, f32) = (0.34, 0.74);
pub const MOLA_ENCOLHER: (f32, f32) = (0.24, 1.0);

/// As páginas da Ilha aberta: arrastar para o lado (ou rolar) passa de uma para outra.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pagina {
    #[default]
    Tocando,
    Letra,
    Visual,
    Ritmo,
    Ajustes,
}

impl Pagina {
    pub const TODAS: [Pagina; 5] = [Self::Tocando, Self::Letra, Self::Visual, Self::Ritmo, Self::Ajustes];

    pub fn indice(self) -> usize {
        Self::TODAS.iter().position(|p| *p == self).unwrap_or(0)
    }

    /// A página `passos` para o lado (negativo = para trás), sem dar a volta.
    pub fn andar(self, passos: i32) -> Self {
        let i = (self.indice() as i32 + passos).clamp(0, Self::TODAS.len() as i32 - 1);
        Self::TODAS[i as usize]
    }

    pub fn nome(self) -> &'static str {
        match self {
            Self::Tocando => "Tocando",
            Self::Letra => "Letra",
            Self::Visual => "Visual",
            Self::Ritmo => "Ritmo",
            Self::Ajustes => "Ajustes",
        }
    }

    pub fn icone(self) -> &'static str {
        match self {
            Self::Tocando => "icones/headphones.svg",
            Self::Letra => "icones/lyrics.svg",
            Self::Visual => "icones/eye.svg",
            Self::Ritmo => "icones/metro.svg",
            Self::Ajustes => "icones/sliders.svg",
        }
    }
}

/// Quanto arrastar para o lado troca de página: um terço da largura, ou menos com um gesto rápido.
pub fn pagina_depois_do_arrasto(atual: Pagina, deslocamento: f32, velocidade: f32, largura: f32) -> Pagina {
    // A mesma projeção da Apple: para onde o gesto iria com o impulso.
    let projetado = deslocamento + velocidade * 0.12;
    if projetado <= -largura / 3.0 {
        atual.andar(1)
    } else if projetado >= largura / 3.0 {
        atual.andar(-1)
    } else {
        atual
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Forma {
    Oculta,
    Repouso,
    Alerta,
    Expandida,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Alerta {
    /// Começou outra música.
    Faixa,
    /// O volume mudou.
    Volume,
    /// O drop está chegando (pela pré-análise).
    Drop,
}

impl Alerta {
    /// Quem ganha quando dois querem aparecer: o drop não espera.
    pub fn prioridade(self) -> u8 {
        match self {
            Self::Drop => 3,
            Self::Faixa => 2,
            Self::Volume => 1,
        }
    }

    /// Quanto o aviso fica à mostra. O do drop é renovado a cada quadro enquanto a contagem vale.
    pub fn duracao(self) -> Duration {
        match self {
            Self::Faixa => Duration::from_millis(1800),
            Self::Volume => Duration::from_millis(1000),
            Self::Drop => Duration::from_millis(400),
        }
    }
}

/// As medidas da forma: largura e altura do corpo, raio dos cantos de baixo e tamanho dos ombros (as curvas que
/// ligam o corpo à borda da tela), em pixels lógicos.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Medidas {
    pub largura: f32,
    pub altura: f32,
    pub raio: f32,
    pub ombro: f32,
}

/// O que está acontecendo neste quadro.
#[derive(Clone, Copy, Debug, Default)]
pub struct Entrada {
    /// A Ilha está ligada nas Configurações.
    pub ligada: bool,
    /// Parar o mouse em cima abre (senão, só o clique).
    pub abre_ao_pairar: bool,
    /// Mostrar avisos (música nova, volume, drop).
    pub avisos: bool,
    pub tem_faixa: bool,
    pub tocando: bool,
    pub ponteiro_dentro: bool,
    /// Outro app está em tela cheia (jogo, vídeo, apresentação).
    pub tela_cheia_de_outro: bool,
    /// A pessoa clicou na Ilha neste quadro.
    pub clicou: bool,
    /// Há quanto tempo a música está parada.
    pub parada_ha_s: f32,
    /// Quanto falta para o próximo drop, quando a análise sabe.
    pub drop_em_s: Option<f32>,
    /// A pessoa está levando a Ilha para outro lugar: ela fica fechada (pequena para levar, sem avisos por cima).
    pub arrastando: bool,
}

/// Os temporizadores e a fila de avisos da Ilha.
#[derive(Default)]
pub struct Controle {
    expandida: bool,
    dentro_desde: Option<Instant>,
    fora_desde: Option<Instant>,
    /// O aviso à mostra e até quando.
    alerta: Option<(Alerta, Instant)>,
    /// Avisos esperando a vez, cada um com a hora em que deixa de valer.
    esperando: Vec<(Alerta, Instant)>,
    /// Desde quando não há faixa (some só depois de [`SEM_FAIXA_SOME`]).
    sem_faixa_desde: Option<Instant>,
}

impl Controle {
    /// Pede um aviso. Igual ao que está à mostra só renova o tempo; um mais importante toma o lugar.
    pub fn alertar(&mut self, alerta: Alerta, agora: Instant) {
        if let Some((atual, ate)) = &mut self.alerta {
            if *atual == alerta {
                *ate = agora + alerta.duracao();
                return;
            }
            if alerta.prioridade() > atual.prioridade() {
                self.alerta = Some((alerta, agora + alerta.duracao()));
                return;
            }
        }
        self.esperando.retain(|(a, _)| *a != alerta);
        self.esperando.push((alerta, agora + alerta.duracao()));
    }

    /// Fecha na hora (o botão de fechar a Ilha aberta, por exemplo).
    pub fn recolher(&mut self) {
        self.expandida = false;
        self.dentro_desde = None;
        self.fora_desde = None;
    }

    /// Decide a forma deste quadro e o aviso à mostra (só na forma [`Forma::Alerta`]).
    pub fn passo(&mut self, e: &Entrada, agora: Instant) -> (Forma, Option<Alerta>) {
        self.sem_faixa_desde = if e.tem_faixa { None } else { Some(self.sem_faixa_desde.unwrap_or(agora)) };
        let sem_faixa = self.sem_faixa_desde.is_some_and(|d| agora.duration_since(d) >= SEM_FAIXA_SOME);
        let oculta = !e.ligada || sem_faixa || e.tela_cheia_de_outro || (!e.tocando && e.parada_ha_s > PARADA_SOME_S);
        if oculta {
            let desde = self.sem_faixa_desde;
            *self = Self::default();
            self.sem_faixa_desde = desde;
            return (Forma::Oculta, None);
        }

        if !e.avisos {
            self.alerta = None;
            self.esperando.clear();
        } else if e.drop_em_s.is_some_and(|s| s > 0.0 && s <= AVISO_DO_DROP_S) {
            self.alertar(Alerta::Drop, agora);
        }

        // Levando a Ilha: fechada, pequena para levar; os avisos esperam (e caducam se for demorado).
        if e.arrastando {
            self.expandida = false;
            self.dentro_desde = None;
            self.fora_desde = None;
            return (Forma::Repouso, None);
        }

        // Abrir e fechar pelo ponteiro.
        if e.clicou {
            // Clicar é estar em cima: a contagem para fechar recomeça.
            self.expandida = true;
            self.fora_desde = None;
        }
        if e.ponteiro_dentro {
            self.fora_desde = None;
            let desde = *self.dentro_desde.get_or_insert(agora);
            if e.abre_ao_pairar && agora.duration_since(desde) >= ESPERA_PARA_ABRIR {
                self.expandida = true;
            }
        } else {
            self.dentro_desde = None;
            if self.expandida {
                let desde = *self.fora_desde.get_or_insert(agora);
                if agora.duration_since(desde) >= ESPERA_PARA_FECHAR {
                    self.expandida = false;
                    self.fora_desde = None;
                }
            }
        }

        // Avisos: os vencidos saem; aberta, a Ilha não mostra nenhum (eles caducam esperando).
        self.esperando.retain(|(_, ate)| *ate > agora);
        if self.alerta.is_some_and(|(_, ate)| ate <= agora) {
            self.alerta = None;
        }
        if self.expandida {
            self.alerta = None;
            return (Forma::Expandida, None);
        }
        if self.alerta.is_none()
            && let Some(i) = (0..self.esperando.len()).max_by_key(|&i| self.esperando[i].0.prioridade())
        {
            let (a, _) = self.esperando.remove(i);
            self.alerta = Some((a, agora + a.duracao()));
        }
        match self.alerta {
            Some((a, _)) => (Forma::Alerta, Some(a)),
            None => (Forma::Repouso, None),
        }
    }
}

/// As medidas de cada forma na `escala` do tamanho escolhido. Na mínima, o repouso é só o espectro (sem a capa).
pub fn medidas(forma: Forma, alerta: Option<Alerta>, escala: f32, minima: bool) -> Medidas {
    let m = |largura: f32, altura: f32, raio: f32, ombro: f32| Medidas {
        largura: largura * escala,
        altura: altura * escala,
        raio: raio * escala,
        ombro: ombro * escala,
    };
    match (forma, alerta) {
        (Forma::Expandida, _) => m(ABERTA.0, ABERTA.1, ABERTA.2, ABERTA.3),
        (Forma::Alerta, Some(Alerta::Faixa)) => m(380.0, 64.0, 26.0, 15.0),
        (Forma::Alerta, Some(Alerta::Volume)) => m(310.0, 46.0, 21.0, 13.0),
        (Forma::Alerta, Some(Alerta::Drop)) => m(270.0, 46.0, 21.0, 13.0),
        _ if minima => m(132.0, 30.0, 14.0, 10.0),
        _ => m(208.0, 36.0, 17.0, 12.0),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn tocando() -> Entrada {
        Entrada {
            ligada: true,
            abre_ao_pairar: true,
            avisos: true,
            tem_faixa: true,
            tocando: true,
            ..Default::default()
        }
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn oculta_sem_faixa_ou_em_tela_cheia() {
        let mut c = Controle::default();
        let t = Instant::now();
        assert_eq!(c.passo(&Entrada { tem_faixa: false, ..tocando() }, t).0, Forma::Repouso);
        assert_eq!(c.passo(&Entrada { tem_faixa: false, ..tocando() }, t + ms(2100)).0, Forma::Oculta);
        assert_eq!(c.passo(&Entrada { tela_cheia_de_outro: true, ..tocando() }, t).0, Forma::Oculta);
        assert_eq!(c.passo(&Entrada { ligada: false, ..tocando() }, t).0, Forma::Oculta);
        assert_eq!(c.passo(&tocando(), t).0, Forma::Repouso);
    }

    #[test]
    fn faixa_sumindo_por_um_instante_nao_fecha() {
        let mut c = Controle::default();
        let t = Instant::now();
        assert_eq!(c.passo(&Entrada { clicou: true, ..tocando() }, t).0, Forma::Expandida);
        let sem = Entrada { tem_faixa: false, ponteiro_dentro: true, ..tocando() };
        assert_eq!(c.passo(&sem, t + ms(50)).0, Forma::Expandida, "a troca de origem não fecha a Ilha");
        assert_eq!(c.passo(&Entrada { ponteiro_dentro: true, ..tocando() }, t + ms(100)).0, Forma::Expandida);
        assert_eq!(c.passo(&sem, t + ms(200)).0, Forma::Expandida);
        assert_eq!(c.passo(&sem, t + ms(2300)).0, Forma::Oculta, "sem faixa de verdade, some");
    }

    #[test]
    fn some_depois_de_30s_parada() {
        let mut c = Controle::default();
        let t = Instant::now();
        let parada = |s| Entrada { tocando: false, parada_ha_s: s, ..tocando() };
        assert_eq!(c.passo(&parada(5.0), t).0, Forma::Repouso, "pausar um instante não esconde");
        assert_eq!(c.passo(&parada(31.0), t).0, Forma::Oculta);
        assert_eq!(c.passo(&tocando(), t).0, Forma::Repouso, "volta ao tocar");
    }

    #[test]
    fn abre_na_hora_e_fecha_logo_ao_sair() {
        let mut c = Controle::default();
        let t = Instant::now();
        let dentro = Entrada { ponteiro_dentro: true, ..tocando() };
        assert_eq!(c.passo(&dentro, t).0, Forma::Expandida, "abre no primeiro quadro com o ponteiro em cima");
        assert_eq!(c.passo(&tocando(), t + ms(40)).0, Forma::Expandida, "um roçar para fora não fecha");
        assert_eq!(c.passo(&dentro, t + ms(70)).0, Forma::Expandida, "voltar zera a espera");
        assert_eq!(c.passo(&tocando(), t + ms(100)).0, Forma::Expandida);
        assert_eq!(c.passo(&tocando(), t + ms(185)).0, Forma::Repouso, "fora por 80 ms, fecha");
    }

    #[test]
    fn sem_abrir_ao_pairar_so_o_clique_abre() {
        let mut c = Controle::default();
        let t = Instant::now();
        let dentro = Entrada { ponteiro_dentro: true, abre_ao_pairar: false, ..tocando() };
        assert_eq!(c.passo(&dentro, t).0, Forma::Repouso);
        assert_eq!(c.passo(&dentro, t + ms(800)).0, Forma::Repouso);
        assert_eq!(c.passo(&Entrada { clicou: true, ..dentro }, t + ms(900)).0, Forma::Expandida);
    }

    #[test]
    fn clicar_de_novo_recomeca_a_espera_para_fechar() {
        let mut c = Controle::default();
        let t = Instant::now();
        assert_eq!(c.passo(&Entrada { clicou: true, ..tocando() }, t).0, Forma::Expandida);
        assert_eq!(c.passo(&Entrada { clicou: true, ..tocando() }, t + ms(60)).0, Forma::Expandida);
        assert_eq!(c.passo(&tocando(), t + ms(120)).0, Forma::Expandida, "a espera recomeçou no segundo clique");
        assert_eq!(c.passo(&tocando(), t + ms(150)).0, Forma::Repouso);
    }

    #[test]
    fn levando_a_ilha_ela_fica_fechada() {
        let mut c = Controle::default();
        let t = Instant::now();
        assert_eq!(c.passo(&Entrada { clicou: true, ponteiro_dentro: true, ..tocando() }, t).0, Forma::Expandida);
        c.alertar(Alerta::Faixa, t);
        let levando = Entrada { arrastando: true, ponteiro_dentro: true, ..tocando() };
        assert_eq!(c.passo(&levando, t + ms(10)), (Forma::Repouso, None), "fecha e não mostra aviso");
        assert_eq!(c.passo(&tocando(), t + ms(30)), (Forma::Alerta, Some(Alerta::Faixa)), "o aviso esperou");
    }

    #[test]
    fn recolher_fecha_na_hora() {
        let mut c = Controle::default();
        let t = Instant::now();
        let dentro = Entrada { ponteiro_dentro: true, clicou: true, ..tocando() };
        assert_eq!(c.passo(&dentro, t).0, Forma::Expandida);
        c.recolher();
        assert_eq!(c.passo(&Entrada { ponteiro_dentro: false, ..tocando() }, t + ms(10)).0, Forma::Repouso);
    }

    #[test]
    fn alerta_dura_e_volta_ao_repouso() {
        let mut c = Controle::default();
        let t = Instant::now();
        c.alertar(Alerta::Faixa, t);
        assert_eq!(c.passo(&tocando(), t), (Forma::Alerta, Some(Alerta::Faixa)));
        assert_eq!(c.passo(&tocando(), t + ms(1500)), (Forma::Alerta, Some(Alerta::Faixa)));
        assert_eq!(c.passo(&tocando(), t + ms(1900)), (Forma::Repouso, None), "a música nova aparece por pouco tempo");
    }

    #[test]
    fn sem_avisos_nada_alarga() {
        let mut c = Controle::default();
        let t = Instant::now();
        c.alertar(Alerta::Faixa, t);
        let quieta = Entrada { avisos: false, drop_em_s: Some(2.0), ..tocando() };
        assert_eq!(c.passo(&quieta, t), (Forma::Repouso, None));
    }

    #[test]
    fn drop_ganha_da_faixa() {
        let mut c = Controle::default();
        let t = Instant::now();
        c.alertar(Alerta::Faixa, t);
        assert_eq!(c.passo(&tocando(), t).1, Some(Alerta::Faixa));
        let chegando = Entrada { drop_em_s: Some(3.0), ..tocando() };
        assert_eq!(c.passo(&chegando, t + ms(100)).1, Some(Alerta::Drop));
        assert_eq!(c.passo(&chegando, t + ms(1500)).1, Some(Alerta::Drop), "renova enquanto a contagem vale");
        assert_eq!(c.passo(&tocando(), t + ms(3200)).0, Forma::Repouso);
        assert_eq!(c.passo(&Entrada { drop_em_s: Some(8.0), ..tocando() }, t + ms(4000)).0, Forma::Repouso);
    }

    #[test]
    fn expandida_segura_os_alertas() {
        let mut c = Controle::default();
        let t = Instant::now();
        let dentro = Entrada { ponteiro_dentro: true, clicou: true, ..tocando() };
        assert_eq!(c.passo(&dentro, t).0, Forma::Expandida);
        c.alertar(Alerta::Volume, t + ms(10));
        assert_eq!(c.passo(&dentro, t + ms(20)), (Forma::Expandida, None));
        assert_eq!(c.passo(&tocando(), t + ms(100)).0, Forma::Expandida);
        assert_eq!(c.passo(&tocando(), t + ms(1700)), (Forma::Repouso, None), "fechada depois, o aviso caducou");
    }

    #[test]
    fn paginas_andam_sem_dar_a_volta() {
        assert_eq!(Pagina::Tocando.andar(1), Pagina::Letra);
        assert_eq!(Pagina::Tocando.andar(-1), Pagina::Tocando);
        assert_eq!(Pagina::Ajustes.andar(1), Pagina::Ajustes);
        assert_eq!(Pagina::Visual.andar(-2), Pagina::Tocando);
    }

    #[test]
    fn arrasto_troca_de_pagina_pela_distancia_ou_pelo_impulso() {
        let w = 540.0;
        assert_eq!(pagina_depois_do_arrasto(Pagina::Letra, -40.0, 0.0, w), Pagina::Letra, "pouco não troca");
        assert_eq!(pagina_depois_do_arrasto(Pagina::Letra, -200.0, 0.0, w), Pagina::Visual);
        assert_eq!(pagina_depois_do_arrasto(Pagina::Letra, 200.0, 0.0, w), Pagina::Tocando);
        assert_eq!(pagina_depois_do_arrasto(Pagina::Letra, -60.0, -1400.0, w), Pagina::Visual, "gesto rápido");
    }
}
