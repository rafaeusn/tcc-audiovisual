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
    /// Parar o mouse num botão de ícone mostra o nome e o atalho dele.
    pub dicas: bool,
    /// Fechar a janela só a esconde: o app continua na bandeja, ouvindo e contando o histórico.
    pub fechar_na_bandeja: bool,
    /// O que o Modo Escuta ouve: vazio = todo o som do PC; senão, o executável de um app (`spotify.exe`).
    pub escuta_app: String,
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
            dicas: true,
            fechar_na_bandeja: false,
            escuta_app: String::new(),
            modulos: vec![Modulo::Ritmo, Modulo::Energia, Modulo::Estrutura, Modulo::Sessao, Modulo::Visual],
            janela_largura: 1440.0,
            janela_altura: 900.0,
        }
    }
}

impl Ajustes {
    /// Corrige valores fora da faixa (arquivo editado à mão ou de uma versão antiga).
    pub fn validar(mut self) -> Self {
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
    }

    #[test]
    fn valores_fora_da_faixa_sao_corrigidos() {
        let a = Ajustes { paleta: 99, janela_largura: f32::NAN, janela_altura: 10.0, ..Default::default() }.validar();
        assert_eq!(a.paleta, 1);
        assert_eq!(a.janela_largura, 1440.0);
        assert_eq!(a.janela_altura, 560.0);
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
