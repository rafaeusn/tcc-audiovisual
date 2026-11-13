//! O manifesto de um módulo: leitura e conferência.

use crate::{API, Acao};
use serde::Deserialize;

const MAXIMO_DE_ACOES: usize = 16;
const MAXIMO_DE_ENDERECOS: usize = 12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Categoria {
    Letras,
    Historico,
    Presenca,
    Aparencia,
    #[default]
    Outro,
}

impl Categoria {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Letras => "Letras",
            Self::Historico => "Histórico",
            Self::Presenca => "Presença",
            Self::Aparencia => "Aparência",
            Self::Outro => "Outros",
        }
    }
}

/// Uma ação que o módulo pede, com o motivo dado pelo autor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pedida {
    pub acao: Acao,
    pub motivo: String,
    /// Só para [`Acao::Rede`]: os endereços que o módulo pode acessar (`*` = qualquer um).
    pub enderecos: Vec<String>,
}

impl Pedida {
    pub fn critica(&self) -> bool {
        self.acao.critica(&self.enderecos)
    }
}

/// Um manifesto já conferido.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifesto {
    /// Identificador estável, em minúsculas e com pontos ("autor.modulo"). Nunca muda entre versões.
    pub id: String,
    pub nome: String,
    /// A versão como a pessoa vê ("1.2.0").
    pub versao: String,
    /// O número que se compara: maior é mais novo.
    pub codigo: u32,
    pub autor: String,
    pub categoria: Categoria,
    pub descricao: String,
    /// Falso: o módulo está no catálogo para ser conhecido, mas ainda não pode ser instalado.
    pub pronto: bool,
    pub acoes: Vec<Pedida>,
}

#[derive(Deserialize)]
#[serde(default)]
struct Bruto {
    id: String,
    nome: String,
    versao: String,
    codigo: u32,
    api: u32,
    autor: String,
    categoria: Categoria,
    descricao: String,
    pronto: bool,
    acoes: Vec<AcaoBruta>,
}

impl Default for Bruto {
    fn default() -> Self {
        Self {
            id: String::new(),
            nome: String::new(),
            versao: String::new(),
            codigo: 1,
            api: API,
            autor: String::new(),
            categoria: Categoria::default(),
            descricao: String::new(),
            pronto: true,
            acoes: Vec::new(),
        }
    }
}

#[derive(Default, Deserialize)]
#[serde(default)]
struct AcaoBruta {
    nome: String,
    motivo: String,
    enderecos: Vec<String>,
}

fn id_valido(id: &str) -> bool {
    let partes: Vec<&str> = id.split('.').collect();
    id.len() <= 64
        && partes.len() >= 2
        && partes.iter().all(|p| {
            p.starts_with(|c: char| c.is_ascii_lowercase())
                && p.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
}

/// Um endereço de rede aceitável: `*` ou um nome de servidor em minúsculas, sem protocolo nem caminho.
fn endereco_valido(e: &str) -> bool {
    e == "*"
        || (e.len() <= 100
            && e.contains('.')
            && !e.starts_with(['.', '-'])
            && !e.ends_with(['.', '-'])
            && e.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '-'))
}

fn tamanho(texto: &str) -> usize {
    texto.trim().chars().count()
}

impl Manifesto {
    /// Lê e confere um manifesto. Devolve todos os problemas de uma vez, em frases que o autor entende.
    pub fn ler(texto: &str) -> Result<Self, Vec<String>> {
        let bruto: Bruto = toml::from_str(texto).map_err(|e| vec![format!("o arquivo não é um TOML válido: {e}")])?;
        let mut problemas = Vec::new();
        let mut exigir = |ok: bool, frase: &str| {
            if !ok {
                problemas.push(frase.to_string());
            }
        };
        exigir(id_valido(&bruto.id), "id: use minúsculas com pontos, como \"autor.modulo\" (até 64 caracteres)");
        exigir((1..=40).contains(&tamanho(&bruto.nome)), "nome: de 1 a 40 caracteres");
        exigir((1..=20).contains(&tamanho(&bruto.versao)), "versao: de 1 a 20 caracteres");
        exigir(bruto.codigo >= 1, "codigo: 1 ou mais");
        exigir(bruto.api == API, "api: este app entende a versão 1 do formato");
        exigir((1..=60).contains(&tamanho(&bruto.autor)), "autor: de 1 a 60 caracteres");
        exigir(tamanho(&bruto.descricao) <= 500, "descricao: até 500 caracteres");
        exigir(!bruto.acoes.is_empty(), "acoes: o módulo precisa declarar pelo menos uma ação");
        exigir(bruto.acoes.len() <= MAXIMO_DE_ACOES, "acoes: no máximo 16");

        let mut acoes: Vec<Pedida> = Vec::with_capacity(bruto.acoes.len());
        for a in bruto.acoes.iter().take(MAXIMO_DE_ACOES) {
            let Some(acao) = Acao::da_chave(&a.nome) else {
                problemas.push(format!("ação desconhecida: \"{}\"", a.nome));
                continue;
            };
            if acoes.iter().any(|p| p.acao == acao) {
                problemas.push(format!("a ação \"{}\" aparece mais de uma vez", a.nome));
                continue;
            }
            if !(1..=80).contains(&tamanho(&a.motivo)) {
                problemas.push(format!("ação \"{}\": o motivo é obrigatório, com até 80 caracteres", a.nome));
            }
            match acao {
                Acao::Rede => {
                    if a.enderecos.is_empty() || a.enderecos.len() > MAXIMO_DE_ENDERECOS {
                        problemas.push("ação \"rede\": declare de 1 a 12 endereços".to_string());
                    }
                    for e in a.enderecos.iter().filter(|e| !endereco_valido(e)) {
                        problemas
                            .push(format!("endereço inválido: \"{e}\" (use só o nome do servidor, em minúsculas)"));
                    }
                }
                _ if !a.enderecos.is_empty() => {
                    problemas.push(format!("ação \"{}\": só a ação \"rede\" tem endereços", a.nome));
                }
                _ => {}
            }
            acoes.push(Pedida { acao, motivo: a.motivo.trim().to_string(), enderecos: a.enderecos.clone() });
        }
        if !problemas.is_empty() {
            return Err(problemas);
        }
        Ok(Self {
            id: bruto.id,
            nome: bruto.nome.trim().to_string(),
            versao: bruto.versao.trim().to_string(),
            codigo: bruto.codigo,
            autor: bruto.autor.trim().to_string(),
            categoria: bruto.categoria,
            descricao: bruto.descricao.trim().to_string(),
            pronto: bruto.pronto,
            acoes,
        })
    }

    /// O módulo pede alguma ação crítica?
    pub fn tem_critica(&self) -> bool {
        self.acoes.iter().any(Pedida::critica)
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    const VALIDO: &str = r#"
id = "exemplo.letras"
nome = "Letras de exemplo"
versao = "1.0.0"
codigo = 3
autor = "Fulana"
categoria = "letras"
campo_do_futuro = true

[[acoes]]
nome = "rede"
motivo = "Buscar a letra"
enderecos = ["exemplo.org"]
"#;

    #[test]
    fn le_um_manifesto_valido() {
        let m = Manifesto::ler(VALIDO).expect("válido");
        assert_eq!(m.id, "exemplo.letras");
        assert_eq!(m.codigo, 3);
        assert_eq!(m.categoria, Categoria::Letras);
        assert!(m.pronto, "pronto por padrão");
        assert_eq!(
            m.acoes,
            [Pedida { acao: Acao::Rede, motivo: "Buscar a letra".into(), enderecos: vec!["exemplo.org".into()] }]
        );
        assert!(!m.tem_critica());
    }

    #[test]
    fn junta_todos_os_problemas() {
        let ruim = r#"
id = "SemPonto"
nome = ""
versao = "1"
autor = "Fulana"
api = 9

[[acoes]]
nome = "rede"
motivo = "Tudo"

[[acoes]]
nome = "apagar.tudo"
motivo = "x"

[[acoes]]
nome = "avisar"
"#;
        let problemas = Manifesto::ler(ruim).expect_err("inválido");
        let texto = problemas.join("\n");
        for trecho in [
            "id:",
            "nome:",
            "api:",
            "declare de 1 a 12 endereços",
            "ação desconhecida: \"apagar.tudo\"",
            "\"avisar\": o motivo",
        ] {
            assert!(texto.contains(trecho), "faltou \"{trecho}\" em:\n{texto}");
        }
    }

    #[test]
    fn enderecos_e_repeticoes() {
        let com = |acoes: &str| format!("id = \"a.b\"\nnome = \"N\"\nversao = \"1\"\nautor = \"A\"\n{acoes}");
        let rede = |e: &str| com(&format!("[[acoes]]\nnome = \"rede\"\nmotivo = \"m\"\nenderecos = [{e}]\n"));
        assert!(Manifesto::ler(&rede("\"api.exemplo.org\"")).is_ok());
        assert!(Manifesto::ler(&rede("\"*\"")).expect("válido").tem_critica());
        for invalido in
            ["\"https://exemplo.org\"", "\"exemplo.org/caminho\"", "\"EXEMPLO.ORG\"", "\"localhost\"", "\".org\""]
        {
            assert!(Manifesto::ler(&rede(invalido)).is_err(), "{invalido}");
        }
        let repetida =
            com("[[acoes]]\nnome = \"avisar\"\nmotivo = \"m\"\n[[acoes]]\nnome = \"avisar\"\nmotivo = \"m\"\n");
        assert!(Manifesto::ler(&repetida).is_err());
        let endereco_fora = com("[[acoes]]\nnome = \"avisar\"\nmotivo = \"m\"\nenderecos = [\"exemplo.org\"]\n");
        assert!(Manifesto::ler(&endereco_fora).is_err());
        assert!(Manifesto::ler(&com("")).is_err(), "sem ações");
        assert!(Manifesto::ler("isto não é toml [[[").is_err());
    }

    #[test]
    fn identificadores() {
        for bom in ["a.b", "autor.modulo_2", "io.github.fulana.letras"] {
            assert!(id_valido(bom), "{bom}");
        }
        for ruim in ["", "semponto", "A.b", "a..b", "a.1b", "a.b-c", ".a.b", "a.b."] {
            assert!(!id_valido(ruim), "{ruim}");
        }
    }
}
