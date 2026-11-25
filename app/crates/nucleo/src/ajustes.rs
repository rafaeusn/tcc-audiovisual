//! Ajustes do usuário (`ajustes.toml`).
//!
//! - Leitura tolerante: campo que falta usa o padrão, valor fora da faixa é corrigido, arquivo corrompido é
//!   guardado como `.corrompido` e o app abre com os padrões.
//! - Gravação atômica: escreve num temporário e troca de nome, então uma queda de energia no meio da gravação
//!   nunca deixa o arquivo pela metade.

use crate::{Erro, Resultado};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

/// Os visuais da v1.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modelo {
    #[default]
    Spectral,
    Fluxo,
    Barras,
}

impl Modelo {
    pub const TODOS: [Modelo; 3] = [Modelo::Spectral, Modelo::Fluxo, Modelo::Barras];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Spectral => "Spectral",
            Self::Fluxo => "Fluxo",
            Self::Barras => "Barras",
        }
    }

    pub fn proximo(self) -> Self {
        match self {
            Self::Spectral => Self::Fluxo,
            Self::Fluxo => Self::Barras,
            Self::Barras => Self::Spectral,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tema {
    #[default]
    Noite,
    Grafite,
    Papel,
    Mono,
}

impl Tema {
    pub const TODOS: [Tema; 4] = [Tema::Noite, Tema::Grafite, Tema::Papel, Tema::Mono];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Noite => "Noite",
            Self::Grafite => "Grafite",
            Self::Papel => "Papel",
            Self::Mono => "Mono",
        }
    }

    pub fn proximo(self) -> Self {
        let i = Self::TODOS.iter().position(|t| *t == self).unwrap_or(0);
        Self::TODOS[(i + 1) % Self::TODOS.len()]
    }
}

/// Os módulos do painel. A ordem e quais aparecem são escolha da pessoa (modo edição).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Modulo {
    Ritmo,
    Energia,
    Estrutura,
    Sessao,
    Visual,
    Relogio,
    /// A letra da música, linha a linha. Só busca na internet enquanto está no painel.
    Letra,
}

impl Modulo {
    pub const TODOS: [Modulo; 7] = [
        Modulo::Ritmo,
        Modulo::Energia,
        Modulo::Estrutura,
        Modulo::Letra,
        Modulo::Sessao,
        Modulo::Visual,
        Modulo::Relogio,
    ];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Ritmo => "Ritmo",
            Self::Energia => "Energia",
            Self::Estrutura => "Estrutura da faixa",
            Self::Sessao => "Nesta sessão",
            Self::Visual => "Visual",
            Self::Relogio => "Relógio",
            Self::Letra => "Letra",
        }
    }

    /// Módulo que ocupa a largura inteira do painel (os outros dividem a linha em dois).
    pub fn largo(self) -> bool {
        matches!(self, Self::Estrutura | Self::Sessao | Self::Visual | Self::Letra)
    }
}

/// A skin muda a forma da interface. `Imersiva`: o visual ocupa a janela inteira e as outras regiões ficam
/// translúcidas por cima dele.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Skin {
    #[default]
    Padrao,
    Imersiva,
}

impl Skin {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Padrao => "Padrão",
            Self::Imersiva => "Imersiva",
        }
    }
}

/// O layout da janela. `Palco`: o visual no centro e as páginas numa folha por cima dele. `Estante`: a lateral com
/// os espaços e os salvos, a página no meio e o trilho "Tocando" com o visual ao vivo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Layout {
    #[default]
    Palco,
    Estante,
}

impl Layout {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Palco => "Palco",
            Self::Estante => "Estante",
        }
    }
}

/// A borda da tela onde a Ilha fica presa: no topo (o padrão), embaixo ou de um dos lados, como uma barra lateral.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BordaIlha {
    #[default]
    Topo,
    Baixo,
    Esquerda,
    Direita,
}

impl BordaIlha {
    pub const TODAS: [BordaIlha; 4] = [Self::Topo, Self::Baixo, Self::Esquerda, Self::Direita];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Topo => "No topo",
            Self::Baixo => "Embaixo",
            Self::Esquerda => "À esquerda",
            Self::Direita => "À direita",
        }
    }

    /// Dos lados, a Ilha fica em pé (uma barra lateral); no topo e embaixo, deitada.
    pub fn de_pe(self) -> bool {
        matches!(self, Self::Esquerda | Self::Direita)
    }
}

/// O tamanho da Ilha: mínima (só o essencial), padrão ou grande.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TamanhoIlha {
    Minima,
    #[default]
    Padrao,
    Grande,
}

impl TamanhoIlha {
    pub const TODOS: [TamanhoIlha; 3] = [Self::Minima, Self::Padrao, Self::Grande];

    pub fn nome(self) -> &'static str {
        match self {
            Self::Minima => "Mínima",
            Self::Padrao => "Padrão",
            Self::Grande => "Grande",
        }
    }

    /// Quanto tudo na Ilha cresce ou encolhe em relação ao padrão.
    pub fn escala(self) -> f32 {
        match self {
            Self::Minima => 0.86,
            Self::Padrao => 1.0,
            Self::Grande => 1.22,
        }
    }

    pub fn proximo(self) -> Self {
        match self {
            Self::Minima => Self::Padrao,
            Self::Padrao => Self::Grande,
            Self::Grande => Self::Minima,
        }
    }
}

/// Repetir, no player de músicas do computador.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Repetir {
    #[default]
    Nada,
    /// A fila inteira: depois da última, volta à primeira.
    Fila,
    /// A música de agora, de novo e de novo.
    Faixa,
}

impl Repetir {
    pub fn proximo(self) -> Self {
        match self {
            Self::Nada => Self::Fila,
            Self::Fila => Self::Faixa,
            Self::Faixa => Self::Nada,
        }
    }
}

/// Bandas do equalizador (o tocador usa as mesmas, em Hz).
pub const BANDAS_EQ: usize = 10;

/// Equalizadores prontos: nome e ganho de cada banda, em dB (31 Hz a 16 kHz).
pub const PRESETS_EQ: [(&str, [f32; BANDAS_EQ]); 7] = [
    ("Plano", [0.0; BANDAS_EQ]),
    ("Graves", [6.0, 5.0, 4.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0]),
    ("Agudos", [0.0, 0.0, 0.0, 0.0, 0.0, 1.0, 2.0, 4.0, 5.0, 6.0]),
    ("Voz", [-2.0, -2.0, -1.0, 1.0, 3.0, 4.0, 3.0, 1.0, 0.0, -1.0]),
    ("Rock", [4.0, 3.0, 2.0, 0.0, -1.0, -1.0, 1.0, 3.0, 4.0, 4.0]),
    ("Eletrônica", [5.0, 4.0, 1.0, 0.0, -2.0, 1.0, 0.0, 1.0, 4.0, 5.0]),
    ("Acústico", [3.0, 2.0, 1.0, 1.0, 2.0, 1.0, 2.0, 3.0, 2.0, 1.0]),
];

/// Uma paleta pronta para quando as cores reativas estão desligadas.
#[derive(Clone, Copy, Debug)]
pub struct Paleta {
    pub nome: &'static str,
    pub a: [u8; 3],
    pub b: [u8; 3],
}

pub const PALETAS: [Paleta; 6] = [
    Paleta { nome: "Gelo", a: [0xff, 0xff, 0xff], b: [0xdf, 0xe6, 0xff] },
    Paleta { nome: "Neon", a: [0xff, 0x2d, 0x55], b: [0x3d, 0xf2, 0xff] },
    Paleta { nome: "Ciano", a: [0x00, 0xe5, 0xff], b: [0xff, 0x00, 0xe5] },
    Paleta { nome: "Aurora", a: [0x00, 0xff, 0x9c], b: [0x7a, 0x5c, 0xff] },
    Paleta { nome: "Brasa", a: [0xff, 0xb3, 0x00], b: [0xff, 0x3d, 0x00] },
    Paleta { nome: "Violeta", a: [0x6a, 0x00, 0xff], b: [0x00, 0xff, 0xd5] },
];

/// As fontes de letra que o app conhece, na ordem padrão (as mesmas de `letras::Fonte`).
pub const FONTES_DE_LETRA: [&str; 3] = ["better-lyrics", "unison", "lrclib"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "kebab-case")]
pub struct Ajustes {
    pub modelo: Modelo,
    pub tema: Tema,
    /// Ligado: a cor segue a energia da música. Desligado: usa a paleta escolhida.
    pub cores_reativas: bool,
    /// Índice em [`PALETAS`].
    pub paleta: usize,
    /// Modo seguro para fotossensibilidade.
    pub modo_seguro: bool,
    pub painel_aberto: bool,
    /// Painel à esquerda do visual (o padrão é à direita).
    pub painel_a_esquerda: bool,
    pub skin: Skin,
    /// Palco ou Estante.
    pub layout: Layout,
    /// Parar o mouse num botão de ícone mostra o nome e o atalho dele.
    pub dicas: bool,
    /// Fechar a janela só a esconde: o app continua na bandeja, ouvindo e contando o histórico.
    pub fechar_na_bandeja: bool,
    /// A Ilha: a pílula com o que toca, presa ao topo da tela. Opcional, como a janela mini (desligada por padrão).
    pub ilha: bool,
    /// A Ilha abre ao parar o mouse em cima; desligado, só com um clique.
    pub ilha_ao_pairar: bool,
    /// A Ilha avisa música nova, mudança de volume e a contagem do drop.
    pub ilha_avisos: bool,
    pub ilha_tamanho: TamanhoIlha,
    /// Onde a Ilha fica ao longo da borda: o centro dela, de 0 a 1 do comprimento da borda (da esquerda para a
    /// direita no topo e embaixo, de cima para baixo dos lados).
    pub ilha_posicao: f32,
    /// A borda onde a Ilha fica presa (quando solta, a última em que esteve: travar a devolve para lá).
    pub ilha_borda: BordaIlha,
    /// Arrastar a Ilha é sempre livre. Travada, ela só desliza ao longo da borda em que está; destravada, vai para
    /// qualquer borda (ou fica solta no meio da tela).
    pub ilha_travada: bool,
    /// Solta (destravada e longe das bordas): o centro dela, em frações da área de trabalho (0 a 1).
    pub ilha_solta: Option<[f32; 2]>,
    /// Em que monitor a Ilha fica, pelo nome que o Windows dá a ele (`\\.\DISPLAY2`); vazio, no da janela principal.
    pub ilha_monitor: Option<String>,
    /// O que o Modo Escuta ouve: vazio = todo o som do PC; senão, o executável de um app (`spotify.exe`).
    pub escuta_app: String,
    /// Os provedores desligados (pelo serviço). Instalados, eles vêm ligados.
    pub provedores_desligados: Vec<String>,
    /// As fontes de letra na ordem em que são perguntadas (`better-lyrics`, `unison`, `lrclib`).
    pub letra_fontes: Vec<String>,
    /// As fontes de letra desligadas (nunca perguntadas).
    pub letra_fontes_desligadas: Vec<String>,
    /// Uma letra palavra por palavra de uma fonte de baixo ganha de uma por linha de uma de cima.
    pub letra_palavra_por_palavra: bool,
    /// Traduzir a letra (embaixo de cada linha) para `letra_idioma`.
    pub letra_traduzir: bool,
    /// O idioma da tradução (código: `pt`, `en`, `es`...).
    pub letra_idioma: String,
    /// Mostrar, embaixo de cada linha, a letra em letras latinas (coreano, japonês, chinês, russo, grego...).
    pub letra_romanizar: bool,
    /// O espaço aberto da última vez (`tudo`, `computador` ou `servico:<nome>`).
    pub espaco: String,
    /// Músicas do computador: segundos de crossfade entre uma e outra (0 = sem; a troca continua sem intervalo).
    pub crossfade_s: f32,
    /// O volume da música que o Player toca (0 a 1). O do PC não é daqui.
    pub volume_musica: f32,
    /// Igualar o volume entre as músicas pelo ReplayGain do arquivo.
    pub volume_igualado: bool,
    /// Equalizador ligado e o ganho de cada banda, em dB.
    pub eq_ligado: bool,
    pub eq: Vec<f32>,
    pub aleatorio: bool,
    pub repetir: Repetir,
    /// Analisar as músicas da biblioteca em segundo plano (a de agora e as próximas são analisadas sempre).
    pub analisar_biblioteca: bool,
    /// Módulos visíveis no painel, na ordem em que aparecem.
    pub modulos: Vec<Modulo>,
    pub janela_largura: f32,
    pub janela_altura: f32,
}

impl Default for Ajustes {
    fn default() -> Self {
        Self {
            modelo: Modelo::Spectral,
            tema: Tema::Noite,
            cores_reativas: true,
            paleta: 1,
            modo_seguro: false,
            painel_aberto: true,
            painel_a_esquerda: false,
            skin: Skin::Padrao,
            layout: Layout::Palco,
            dicas: true,
            fechar_na_bandeja: false,
            ilha: false,
            ilha_ao_pairar: true,
            ilha_avisos: true,
            ilha_tamanho: TamanhoIlha::Padrao,
            ilha_posicao: 0.5,
            ilha_borda: BordaIlha::Topo,
            ilha_travada: true,
            ilha_solta: None,
            ilha_monitor: None,
            escuta_app: String::new(),
            provedores_desligados: Vec::new(),
            letra_fontes: FONTES_DE_LETRA.iter().map(|s| s.to_string()).collect(),
            letra_fontes_desligadas: Vec::new(),
            letra_palavra_por_palavra: false,
            letra_traduzir: false,
            letra_idioma: "pt".into(),
            letra_romanizar: false,
            espaco: "tudo".into(),
            crossfade_s: 0.0,
            volume_musica: 1.0,
            volume_igualado: true,
            eq_ligado: false,
            eq: vec![0.0; BANDAS_EQ],
            aleatorio: false,
            repetir: Repetir::Nada,
            analisar_biblioteca: true,
            modulos: vec![Modulo::Ritmo, Modulo::Energia, Modulo::Estrutura, Modulo::Sessao, Modulo::Visual],
            janela_largura: 1440.0,
            janela_altura: 900.0,
        }
    }
}

impl Ajustes {
    /// Corrige valores fora da faixa (arquivo editado à mão ou de uma versão antiga).
    pub fn validar(mut self) -> Self {
        self.ilha_posicao = if self.ilha_posicao.is_finite() { self.ilha_posicao.clamp(0.0, 1.0) } else { 0.5 };
        // Solta só destravada, e dentro da tela; um número estragado volta a Ilha para a borda.
        self.ilha_solta = self
            .ilha_solta
            .filter(|p| !self.ilha_travada && p.iter().all(|v| v.is_finite()))
            .map(|p| p.map(|v| v.clamp(0.0, 1.0)));
        self.ilha_monitor = self.ilha_monitor.filter(|m| !m.trim().is_empty());
        self.provedores_desligados.retain(|s| !s.trim().is_empty());
        self.provedores_desligados.sort();
        self.provedores_desligados.dedup();
        // As fontes de letra: cada uma conhecida uma vez, as que faltam no fim; pelo menos uma ligada.
        let mut fontes: Vec<String> = Vec::new();
        for f in self.letra_fontes.iter().map(String::as_str).chain(FONTES_DE_LETRA) {
            if FONTES_DE_LETRA.contains(&f) && !fontes.iter().any(|j| j == f) {
                fontes.push(f.to_string());
            }
        }
        self.letra_fontes = fontes;
        self.letra_fontes_desligadas.retain(|f| FONTES_DE_LETRA.contains(&f.as_str()));
        self.letra_fontes_desligadas.sort();
        self.letra_fontes_desligadas.dedup();
        if self.letra_fontes_desligadas.len() >= FONTES_DE_LETRA.len() {
            self.letra_fontes_desligadas.retain(|f| f != "lrclib");
        }
        let idioma = self.letra_idioma.trim().to_ascii_lowercase();
        self.letra_idioma =
            if (2..=8).contains(&idioma.len()) && idioma.chars().all(|c| c.is_ascii_alphabetic() || c == '-') {
                idioma
            } else {
                "pt".into()
            };
        let espaco = self.espaco.trim();
        self.espaco = if (1..=80).contains(&espaco.len()) && !espaco.chars().any(char::is_control) {
            espaco.to_string()
        } else {
            "tudo".into()
        };
        if self.paleta >= PALETAS.len() {
            self.paleta = Self::default().paleta;
        }
        // Módulo repetido (arquivo editado à mão): fica só a primeira vez.
        let mut vistos = Vec::with_capacity(self.modulos.len());
        self.modulos.retain(|m| {
            !vistos.contains(m) && {
                vistos.push(*m);
                true
            }
        });
        // Nome de executável: minúsculas, sem pastas, de tamanho razoável.
        self.escuta_app = self.escuta_app.trim().to_lowercase();
        if self.escuta_app.len() > 120 || self.escuta_app.contains(['/', '\\']) {
            self.escuta_app.clear();
        }
        self.volume_musica = if self.volume_musica.is_finite() { self.volume_musica.clamp(0.0, 1.0) } else { 1.0 };
        self.crossfade_s = if self.crossfade_s.is_finite() { self.crossfade_s.clamp(0.0, 12.0) } else { 0.0 };
        self.eq.resize(BANDAS_EQ, 0.0);
        for g in &mut self.eq {
            *g = if g.is_finite() { g.clamp(-12.0, 12.0) } else { 0.0 };
        }
        let faixa = |v: f32, min: f32, max: f32, padrao: f32| if v.is_finite() { v.clamp(min, max) } else { padrao };
        self.janela_largura = faixa(self.janela_largura, 360.0, 7680.0, 1440.0);
        self.janela_altura = faixa(self.janela_altura, 560.0, 4320.0, 900.0);
        self
    }

    pub fn paleta_atual(&self) -> Paleta {
        PALETAS[self.paleta.min(PALETAS.len() - 1)]
    }

    /// Lê os ajustes. Nunca falha: qualquer problema volta aos padrões e é registrado.
    pub fn carregar(caminho: &Path) -> Self {
        let texto = match std::fs::read_to_string(caminho) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                log::warn!("não foi possível ler {}: {e}", caminho.display());
                return Self::default();
            }
        };
        match toml::from_str::<Ajustes>(&texto) {
            Ok(a) => a.validar(),
            Err(e) => {
                log::warn!("ajustes corrompidos ({e}); usando os padrões");
                let _ = std::fs::rename(caminho, caminho.with_extension("toml.corrompido"));
                Self::default()
            }
        }
    }

    /// Grava de forma atômica: temporário na mesma pasta, `sync`, troca de nome.
    pub fn salvar(&self, caminho: &Path) -> Resultado<()> {
        let texto = toml::to_string_pretty(self).map_err(|e| Erro::Ajustes(e.to_string()))?;
        let tmp = caminho.with_extension("toml.tmp");
        let gravar = || -> std::io::Result<()> {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(texto.as_bytes())?;
            f.sync_all()?;
            drop(f);
            std::fs::rename(&tmp, caminho)
        };
        gravar().map_err(|e| {
            let _ = std::fs::remove_file(&tmp);
            Erro::arquivo(caminho, e)
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn pasta_temp(nome: &str) -> std::path::PathBuf {
        let p = std::env::temp_dir().join(format!("tccplayer-teste-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("pasta temporária");
        p
    }

    #[test]
    fn layout_padrao_e_palco_e_volta_do_toml() {
        assert_eq!(Ajustes::default().layout, Layout::Palco);
        let lido: Ajustes = toml::from_str("layout = \"estante\"").expect("toml");
        assert_eq!(lido.layout, Layout::Estante);
        let escrito = toml::to_string(&Ajustes { layout: Layout::Estante, ..Default::default() }).expect("toml");
        assert!(escrito.contains("layout = \"estante\""), "{escrito}");
        assert_eq!(Layout::Estante.nome(), "Estante");
    }

    #[test]
    fn ida_e_volta() {
        let pasta = pasta_temp("ida");
        let arq = pasta.join("ajustes.toml");
        let a = Ajustes { modelo: Modelo::Barras, modo_seguro: true, paleta: 3, ..Default::default() };
        a.salvar(&arq).expect("salvar");
        assert_eq!(Ajustes::carregar(&arq), a);
        assert!(!arq.with_extension("toml.tmp").exists());
        let _ = std::fs::remove_dir_all(pasta);
    }

    #[test]
    fn campo_faltando_usa_padrao() {
        let a: Ajustes = toml::from_str("modelo = \"fluxo\"").expect("toml");
        assert_eq!(a.modelo, Modelo::Fluxo);
        assert!(a.cores_reativas);
        assert!(!a.ilha, "a Ilha é opcional: arquivo antigo a abre desligada");
        assert!(a.ilha_ao_pairar && a.ilha_avisos);
        assert_eq!(a.ilha_tamanho, TamanhoIlha::Padrao);
        assert_eq!(a.ilha_posicao, 0.5);
        assert_eq!(
            (a.ilha_borda, a.ilha_travada, a.ilha_solta),
            (BordaIlha::Topo, true, None),
            "presa no topo e travada"
        );
    }

    #[test]
    fn valores_fora_da_faixa_sao_corrigidos() {
        let a = Ajustes { paleta: 99, janela_largura: f32::NAN, janela_altura: 10.0, ..Default::default() }.validar();
        assert_eq!(a.paleta, 1);
        assert_eq!(a.janela_largura, 1440.0);
        assert_eq!(a.janela_altura, 560.0);
        let fora = Ajustes { ilha_posicao: 3.0, ..Default::default() }.validar();
        assert_eq!(fora.ilha_posicao, 1.0);
        let nan = Ajustes { ilha_posicao: f32::NAN, ..Default::default() }.validar();
        assert_eq!(nan.ilha_posicao, 0.5);
        // Solta: só destravada, com números válidos, dentro da tela.
        let solta = Ajustes { ilha_travada: false, ilha_solta: Some([1.4, -0.2]), ..Default::default() }.validar();
        assert_eq!(solta.ilha_solta, Some([1.0, 0.0]));
        let travada = Ajustes { ilha_travada: true, ilha_solta: Some([0.5, 0.5]), ..Default::default() }.validar();
        assert_eq!(travada.ilha_solta, None, "travada, a Ilha fica na borda");
        let estragada = Ajustes { ilha_travada: false, ilha_solta: Some([f32::NAN, 0.5]), ..Default::default() };
        assert_eq!(estragada.validar().ilha_solta, None);
        let fontes = Ajustes {
            letra_fontes: vec!["lrclib".into(), "outra".into(), "lrclib".into()],
            letra_fontes_desligadas: vec!["better-lyrics".into(), "unison".into(), "lrclib".into()],
            letra_idioma: "Português do Brasil!".into(),
            ..Default::default()
        }
        .validar();
        assert_eq!(fontes.letra_fontes, vec!["lrclib", "better-lyrics", "unison"], "as que faltam no fim, sem repetir");
        assert!(!fontes.letra_fontes_desligadas.contains(&"lrclib".into()), "pelo menos uma fica ligada");
        assert_eq!(fontes.letra_idioma, "pt");
        let desligados = Ajustes {
            provedores_desligados: vec!["b".into(), " ".into(), "a".into(), "b".into()],
            ..Default::default()
        };
        assert_eq!(desligados.validar().provedores_desligados, vec!["a", "b"], "sem vazio nem repetido");
        let sem_nome = Ajustes { ilha_monitor: Some("  ".into()), ..Default::default() }.validar();
        assert_eq!(sem_nome.ilha_monitor, None, "monitor sem nome volta a ser o da janela principal");
        assert_eq!(Ajustes::default().espaco, "tudo");
        let espaco = Ajustes { espaco: " servico:youtube-music ".into(), ..Default::default() }.validar();
        assert_eq!(espaco.espaco, "servico:youtube-music");
        let comprido = Ajustes { espaco: "x".repeat(500), ..Default::default() }.validar();
        assert_eq!(comprido.espaco, "tudo", "chave estragada volta a Tudo");
    }

    #[test]
    fn arquivo_corrompido_volta_ao_padrao() {
        let pasta = pasta_temp("corrompido");
        let arq = pasta.join("ajustes.toml");
        std::fs::write(&arq, "modelo = [[[").expect("escrever");
        assert_eq!(Ajustes::carregar(&arq), Ajustes::default());
        assert!(arq.with_extension("toml.corrompido").exists());
        let _ = std::fs::remove_dir_all(pasta);
    }

    #[test]
    fn modulos_repetidos_sao_removidos_e_a_ordem_fica() {
        let a = Ajustes {
            modulos: vec![Modulo::Visual, Modulo::Ritmo, Modulo::Visual, Modulo::Relogio],
            ..Default::default()
        }
        .validar();
        assert_eq!(a.modulos, vec![Modulo::Visual, Modulo::Ritmo, Modulo::Relogio]);
        let lido: Ajustes = toml::from_str(
            "modulos = [\"relogio\", \"energia\"]
skin = \"imersiva\"",
        )
        .expect("toml");
        assert_eq!(lido.modulos, vec![Modulo::Relogio, Modulo::Energia]);
        assert_eq!(lido.skin, Skin::Imersiva);
    }

    #[test]
    fn arquivo_ausente_usa_padrao() {
        assert_eq!(Ajustes::carregar(Path::new("Z:/nao/existe/ajustes.toml")), Ajustes::default());
    }
}
