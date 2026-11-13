//! O Player dentro do Windows.
//!
//! - **Barra de tarefas**: os botões anterior, tocar/pausar e próxima que aparecem na miniatura da janela.
//! - **Bandeja**: um ícone ao lado do relógio, com o nome da música e um menu (mostrar, tocar, próxima, sair).
//!   Se a pessoa escolher, fechar a janela só a esconde: o app continua ouvindo e contando o histórico.
//! - **Janela mini**: a janela encolhe para um canto da tela e fica por cima das outras.
//!
//! Tudo acontece na thread da interface, dentro das mensagens que o Windows manda para a janela. Por isso os
//! botões e o menu funcionam mesmo com a janela escondida, quando a interface não está sendo desenhada: quem
//! recebe o pedido é a função dada em [`Integracao::instalar`], que só repassa o comando adiante.
//!
//! Nada aqui é essencial: se o Windows recusar qualquer parte, o app segue sem ela.

mod desenho;
#[cfg(windows)]
mod sistema;

/// Um pedido vindo da barra de tarefas ou do menu da bandeja.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Acao {
    Anterior,
    TocarPausar,
    Proxima,
}

/// Ligação com a janela. Soltar o valor tira o ícone da bandeja e desfaz a ligação.
pub struct Integracao {
    #[cfg(windows)]
    instalada: bool,
}

impl Integracao {
    /// Liga-se à janela `hwnd` (o identificador que o Windows dá à janela). Chamar na thread da interface.
    /// `ao_pedir` é chamada a cada clique num botão de mídia.
    pub fn instalar(hwnd: isize, ao_pedir: impl Fn(Acao) + 'static) -> Self {
        #[cfg(windows)]
        {
            Self { instalada: sistema::instalar(hwnd, Box::new(ao_pedir)) }
        }
        #[cfg(not(windows))]
        {
            let _ = (hwnd, ao_pedir);
            Self {}
        }
    }

    /// Mostra o botão de pausar (tocando) ou o de tocar na miniatura da barra de tarefas.
    pub fn tocando(&self, tocando: bool) {
        #[cfg(windows)]
        sistema::tocando(tocando);
        #[cfg(not(windows))]
        let _ = tocando;
    }

    /// O texto que aparece ao passar o mouse no ícone da bandeja (o nome da música).
    pub fn dica(&self, texto: &str) {
        #[cfg(windows)]
        sistema::dica(texto);
        #[cfg(not(windows))]
        let _ = texto;
    }

    /// Ligado: fechar a janela só a esconde, e o app continua na bandeja.
    pub fn fechar_esconde(&self, ligado: bool) {
        #[cfg(windows)]
        sistema::fechar_esconde(ligado);
        #[cfg(not(windows))]
        let _ = ligado;
    }

    /// Encolhe a janela para o canto da tela, por cima das outras (ou devolve o tamanho e o lugar de antes).
    /// Devolve se a troca aconteceu.
    pub fn mini(&self, ligado: bool) -> bool {
        #[cfg(windows)]
        {
            self.instalada && sistema::mini(ligado)
        }
        #[cfg(not(windows))]
        {
            let _ = ligado;
            false
        }
    }
}

impl Integracao {
    /// O ponteiro do mouse está em cima da janela agora. `None` quando não dá para saber.
    pub fn ponteiro_dentro(&self) -> Option<bool> {
        #[cfg(windows)]
        {
            self.instalada.then(sistema::ponteiro_dentro)
        }
        #[cfg(not(windows))]
        {
            None
        }
    }

    /// Sai da janela mini direto para a janela maximizada.
    pub fn mini_para_maximizada(&self) -> bool {
        #[cfg(windows)]
        {
            self.instalada && sistema::mini_para_maximizada()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }
}

impl Drop for Integracao {
    fn drop(&mut self) {
        #[cfg(windows)]
        sistema::remover();
    }
}

/// Texto da dica da bandeja: "Música · Artista", cortado no limite do Windows (127 caracteres).
pub fn texto_da_dica(titulo: &str, artista: &str) -> String {
    let texto = match (titulo.trim(), artista.trim()) {
        ("", _) => "Player".to_string(),
        (t, "") => t.to_string(),
        (t, a) => format!("{t} · {a}"),
    };
    let limpo: String = texto.chars().filter(|c| !c.is_control()).collect();
    if limpo.chars().count() > 120 { limpo.chars().take(119).chain(['…']).collect() } else { limpo }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn dica_da_bandeja() {
        assert_eq!(texto_da_dica("", ""), "Player");
        assert_eq!(texto_da_dica("Canção", ""), "Canção");
        assert_eq!(texto_da_dica(" Canção ", "Banda"), "Canção · Banda");
        let longa = texto_da_dica(&"x".repeat(300), "Banda");
        assert_eq!(longa.chars().count(), 120);
        assert!(longa.ends_with('…'));
        assert_eq!(texto_da_dica("a\nb", ""), "ab");
    }
}
