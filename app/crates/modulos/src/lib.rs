//! O formato dos módulos da Loja.
//!
//! Um módulo se descreve num **manifesto** (`modulo.toml`): quem é, o que faz e, principalmente, a lista de
//! **ações** que ele precisa do app. As ações vêm de um catálogo fechado ([`Acao`]): ler o que está tocando, usar
//! a internet em endereços declarados, guardar dados, entrar numa conta. Cada ação pedida traz um **motivo**
//! escrito pelo autor, que a pessoa lê antes de instalar. As ações críticas aparecem destacadas.
//!
//! Instalar é aceitar o conjunto inteiro: o que foi aceito fica guardado ([`Aceite`]). Uma versão nova só se
//! aplica sozinha se não pedir nada além disso ([`Aceite::mudancas`]): ação nova, endereço novo, motivo
//! diferente ou outro autor esperam a pessoa aceitar de novo, vendo só o que mudou.
//!
//! A [lista de bloqueio](Bloqueios) desliga um módulo que se mostrou perigoso, pelo identificador ou pelo
//! resumo (hash) de uma versão específica.
//!
//! Esta crate só descreve e confere. Quem executa cada módulo é o app, e só dentro do que o manifesto declara.

mod instalados;
mod manifesto;

pub use instalados::{Instalado, Instalados};
pub use manifesto::{Categoria, Manifesto, Pedida};

use serde::{Deserialize, Serialize};

/// Versão do formato que este app entende.
pub const API: u32 = 1;

/// O que um módulo pode pedir ao app. O catálogo é fechado: nome desconhecido recusa o manifesto.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Acao {
    TocandoLer,
    TocandoControlar,
    Rede,
    DadosGuardar,
    HistoricoLer,
    ContaEntrar,
    ProgramaLocal,
    PainelModulo,
    AparenciaMudar,
    Avisar,
}

impl Acao {
    pub const TODAS: [Acao; 10] = [
        Acao::TocandoLer,
        Acao::TocandoControlar,
        Acao::Rede,
        Acao::DadosGuardar,
        Acao::HistoricoLer,
        Acao::ContaEntrar,
        Acao::ProgramaLocal,
        Acao::PainelModulo,
        Acao::AparenciaMudar,
        Acao::Avisar,
    ];

    /// O nome da ação no manifesto.
    pub fn chave(self) -> &'static str {
        match self {
            Self::TocandoLer => "tocando.ler",
            Self::TocandoControlar => "tocando.controlar",
            Self::Rede => "rede",
            Self::DadosGuardar => "dados.guardar",
            Self::HistoricoLer => "historico.ler",
            Self::ContaEntrar => "conta.entrar",
            Self::ProgramaLocal => "programa.local",
            Self::PainelModulo => "painel.modulo",
            Self::AparenciaMudar => "aparencia.mudar",
            Self::Avisar => "avisar",
        }
    }

    pub fn da_chave(chave: &str) -> Option<Self> {
        Self::TODAS.into_iter().find(|a| a.chave() == chave)
    }

    /// Como a ação aparece para a pessoa.
    pub fn rotulo(self) -> &'static str {
        match self {
            Self::TocandoLer => "Saber o que você ouve",
            Self::TocandoControlar => "Controlar a reprodução",
            Self::Rede => "Usar a internet",
            Self::DadosGuardar => "Guardar dados próprios",
            Self::HistoricoLer => "Ler o seu histórico",
            Self::ContaEntrar => "Entrar numa conta sua",
            Self::ProgramaLocal => "Falar com outro programa deste computador",
            Self::PainelModulo => "Mostrar um módulo no painel",
            Self::AparenciaMudar => "Mudar a aparência",
            Self::Avisar => "Mostrar avisos curtos",
        }
    }

    /// Ação que merece destaque antes de instalar: dá acesso a algo sensível ou sai do computador sem limite.
    /// A internet só é crítica quando o módulo pede qualquer endereço (`*`).
    pub fn critica(self, enderecos: &[String]) -> bool {
        match self {
            Self::HistoricoLer | Self::ContaEntrar | Self::ProgramaLocal => true,
            Self::Rede => enderecos.iter().any(|e| e == "*"),
            _ => false,
        }
    }
}

/// O que a pessoa aceitou ao instalar: é contra isto que uma versão nova é comparada.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Aceite {
    pub autor: String,
    pub acoes: Vec<AcaoAceita>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AcaoAceita {
    pub nome: String,
    pub motivo: String,
    pub enderecos: Vec<String>,
}

/// Uma diferença entre o que foi aceito e o que uma versão nova pede.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Mudanca {
    AcaoNova(Acao),
    EnderecoNovo(String),
    /// O autor reescreveu o motivo: quem julga se o sentido mudou é a pessoa, não o app.
    MotivoMudou(Acao),
    AutorMudou,
}

impl Aceite {
    pub fn de(m: &Manifesto) -> Self {
        Self {
            autor: m.autor.clone(),
            acoes: m
                .acoes
                .iter()
                .map(|p| AcaoAceita {
                    nome: p.acao.chave().to_string(),
                    motivo: p.motivo.clone(),
                    enderecos: p.enderecos.clone(),
                })
                .collect(),
        }
    }

    /// O que `novo` pede além do que já foi aceito. Vazio = a versão nova pode se aplicar sozinha.
    /// Deixar de pedir alguma coisa nunca é uma mudança que precise de aceite.
    pub fn mudancas(&self, novo: &Manifesto) -> Vec<Mudanca> {
        let mut mudancas = Vec::new();
        if self.autor != novo.autor {
            mudancas.push(Mudanca::AutorMudou);
        }
        for pedida in &novo.acoes {
            match self.acoes.iter().find(|a| a.nome == pedida.acao.chave()) {
                None => mudancas.push(Mudanca::AcaoNova(pedida.acao)),
                Some(aceita) => {
                    if aceita.motivo != pedida.motivo {
                        mudancas.push(Mudanca::MotivoMudou(pedida.acao));
                    }
                    for endereco in &pedida.enderecos {
                        if !aceita.enderecos.contains(endereco) {
                            mudancas.push(Mudanca::EnderecoNovo(endereco.clone()));
                        }
                    }
                }
            }
        }
        mudancas
    }
}

/// A lista de bloqueio: módulos que o app desliga e não deixa instalar.
#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
pub struct Bloqueios {
    bloqueado: Vec<Bloqueado>,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(default)]
struct Bloqueado {
    id: String,
    motivo: String,
    /// Vazio = todas as versões. Com itens, só as versões com estes resumos.
    sha256: Vec<String>,
}

impl Bloqueios {
    /// Lê a lista. Texto ilegível vira lista vazia (e fica registrado): uma lista estragada não pode desligar
    /// todos os módulos, nem derrubar o app.
    pub fn ler(texto: &str) -> Self {
        toml::from_str(texto).unwrap_or_else(|e| {
            log::warn!("lista de bloqueio ilegível: {e}");
            Self::default()
        })
    }

    /// O motivo do bloqueio do módulo `id` (com o resumo `sha256` da versão, se conhecido), ou `None`.
    pub fn motivo(&self, id: &str, sha256: Option<&str>) -> Option<&str> {
        self.bloqueado
            .iter()
            .find(|b| {
                b.id == id
                    && (b.sha256.is_empty()
                        || sha256.is_some_and(|h| b.sha256.iter().any(|x| x.eq_ignore_ascii_case(h))))
            })
            .map(|b| if b.motivo.is_empty() { "bloqueado" } else { b.motivo.as_str() })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    const BASE: &str = r#"
id = "exemplo.letras"
nome = "Letras de exemplo"
versao = "1.0.0"
autor = "Fulana"
descricao = "Um módulo de teste."

[[acoes]]
nome = "tocando.ler"
motivo = "Saber qual música buscar"

[[acoes]]
nome = "rede"
motivo = "Buscar a letra"
enderecos = ["exemplo.org"]
"#;

    fn manifesto(texto: &str) -> Manifesto {
        Manifesto::ler(texto).expect("manifesto válido")
    }

    #[test]
    fn catalogo_de_acoes_e_consistente() {
        for a in Acao::TODAS {
            assert_eq!(Acao::da_chave(a.chave()), Some(a));
            assert!(!a.rotulo().is_empty());
        }
        assert_eq!(Acao::da_chave("apagar.tudo"), None);
        assert!(Acao::ContaEntrar.critica(&[]));
        assert!(!Acao::Rede.critica(&["exemplo.org".into()]));
        assert!(Acao::Rede.critica(&["*".into()]), "qualquer endereço é crítico");
        assert!(!Acao::TocandoLer.critica(&[]));
    }

    #[test]
    fn versao_nova_sem_nada_novo_passa_sozinha() {
        let aceite = Aceite::de(&manifesto(BASE));
        assert!(aceite.mudancas(&manifesto(BASE)).is_empty());
        // Deixar de pedir uma ação também passa.
        let menor = BASE
            .replace("[[acoes]]\nnome = \"rede\"\nmotivo = \"Buscar a letra\"\nenderecos = [\"exemplo.org\"]\n", "");
        assert!(aceite.mudancas(&manifesto(&menor)).is_empty());
    }

    #[test]
    fn o_que_muda_espera_o_aceite() {
        let aceite = Aceite::de(&manifesto(BASE));
        let outro_endereco = BASE.replace("[\"exemplo.org\"]", "[\"exemplo.org\", \"outro.net\"]");
        assert_eq!(aceite.mudancas(&manifesto(&outro_endereco)), [Mudanca::EnderecoNovo("outro.net".into())]);

        let acao_nova = format!("{BASE}\n[[acoes]]\nnome = \"historico.ler\"\nmotivo = \"Montar estatísticas\"\n");
        assert_eq!(aceite.mudancas(&manifesto(&acao_nova)), [Mudanca::AcaoNova(Acao::HistoricoLer)]);

        let motivo = BASE.replace("Buscar a letra", "Buscar a letra e a capa");
        assert_eq!(aceite.mudancas(&manifesto(&motivo)), [Mudanca::MotivoMudou(Acao::Rede)]);

        let autor = BASE.replace("Fulana", "Outra Pessoa");
        assert_eq!(aceite.mudancas(&manifesto(&autor)), [Mudanca::AutorMudou]);
    }

    #[test]
    fn lista_de_bloqueio() {
        let lista = Bloqueios::ler(
            r#"
[[bloqueado]]
id = "ruim.total"
motivo = "envia dados sem avisar"

[[bloqueado]]
id = "ruim.uma_versao"
sha256 = ["ABCDEF"]
"#,
        );
        assert_eq!(lista.motivo("ruim.total", None), Some("envia dados sem avisar"));
        assert_eq!(lista.motivo("ruim.total", Some("qualquer")), Some("envia dados sem avisar"));
        assert_eq!(lista.motivo("ruim.uma_versao", Some("abcdef")), Some("bloqueado"));
        assert_eq!(lista.motivo("ruim.uma_versao", Some("outra")), None);
        assert_eq!(lista.motivo("ruim.uma_versao", None), None);
        assert_eq!(lista.motivo("bom", None), None);
        // Lista estragada não bloqueia ninguém.
        assert_eq!(Bloqueios::ler("[[[").motivo("ruim.total", None), None);
    }
}
