//! Os módulos instalados (`modulos.toml`): quais são, se estão ligados e o que foi aceito de cada um.

use crate::{Aceite, Manifesto};
use nucleo::{Erro, Resultado};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Instalado {
    pub id: String,
    pub versao: String,
    pub codigo: u32,
    pub ligado: bool,
    pub aceite: Aceite,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Instalados {
    modulo: Vec<Instalado>,
}

impl Instalados {
    /// Lê o arquivo. Nunca falha: arquivo ausente ou ilegível vira "nada instalado" (o ilegível fica guardado
    /// como `.corrompido`). Instalar de novo é barato; abrir o app é obrigatório.
    pub fn carregar(caminho: &Path) -> Self {
        let texto = match std::fs::read_to_string(caminho) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Self::default(),
            Err(e) => {
                log::warn!("não foi possível ler {}: {e}", caminho.display());
                return Self::default();
            }
        };
        toml::from_str(&texto).unwrap_or_else(|e| {
            log::warn!("lista de módulos instalados ilegível ({e}); começando vazia");
            let _ = std::fs::rename(caminho, caminho.with_extension("toml.corrompido"));
            Self::default()
        })
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

    pub fn todos(&self) -> &[Instalado] {
        &self.modulo
    }

    pub fn achar(&self, id: &str) -> Option<&Instalado> {
        self.modulo.iter().find(|m| m.id == id)
    }

    /// O módulo está instalado e ligado.
    pub fn ativo(&self, id: &str) -> bool {
        self.achar(id).is_some_and(|m| m.ligado)
    }

    /// Instala (ou atualiza) um módulo, guardando o que a pessoa acabou de aceitar. Fica ligado.
    pub fn instalar(&mut self, m: &Manifesto) {
        let novo = Instalado {
            id: m.id.clone(),
            versao: m.versao.clone(),
            codigo: m.codigo,
            ligado: true,
            aceite: Aceite::de(m),
        };
        match self.modulo.iter_mut().find(|i| i.id == m.id) {
            Some(existente) => *existente = novo,
            None => self.modulo.push(novo),
        }
    }

    /// Tira o módulo. Devolve se ele estava instalado.
    pub fn remover(&mut self, id: &str) -> bool {
        let antes = self.modulo.len();
        self.modulo.retain(|m| m.id != id);
        self.modulo.len() != antes
    }

    pub fn ligar(&mut self, id: &str, ligado: bool) {
        if let Some(m) = self.modulo.iter_mut().find(|m| m.id == id) {
            m.ligado = ligado;
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn manifesto(codigo: u32) -> Manifesto {
        Manifesto::ler(&format!(
            "id = \"exemplo.um\"\nnome = \"Um\"\nversao = \"1.{codigo}\"\ncodigo = {codigo}\nautor = \"Fulana\"\n\
             [[acoes]]\nnome = \"rede\"\nmotivo = \"Buscar\"\nenderecos = [\"exemplo.org\"]\n"
        ))
        .expect("manifesto")
    }

    #[test]
    fn instalar_atualizar_desligar_remover() {
        let mut lista = Instalados::default();
        assert!(!lista.ativo("exemplo.um"));
        lista.instalar(&manifesto(1));
        assert!(lista.ativo("exemplo.um"));
        assert_eq!(lista.achar("exemplo.um").expect("instalado").aceite.acoes[0].enderecos, ["exemplo.org"]);
        lista.instalar(&manifesto(2));
        assert_eq!(lista.todos().len(), 1, "atualizar não duplica");
        assert_eq!(lista.todos()[0].codigo, 2);
        lista.ligar("exemplo.um", false);
        assert!(!lista.ativo("exemplo.um"));
        assert!(lista.remover("exemplo.um"));
        assert!(!lista.remover("exemplo.um"));
        assert!(lista.todos().is_empty());
    }

    #[test]
    fn ida_e_volta_e_arquivo_estragado() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-teste-modulos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        let arquivo = pasta.join("modulos.toml");
        assert_eq!(Instalados::carregar(&arquivo), Instalados::default());
        let mut lista = Instalados::default();
        lista.instalar(&manifesto(1));
        lista.salvar(&arquivo).expect("salvar");
        assert_eq!(Instalados::carregar(&arquivo), lista);
        std::fs::write(&arquivo, "modulo = [[[").expect("escrever");
        assert_eq!(Instalados::carregar(&arquivo), Instalados::default());
        assert!(arquivo.with_extension("toml.corrompido").exists());
        let _ = std::fs::remove_dir_all(pasta);
    }
}
