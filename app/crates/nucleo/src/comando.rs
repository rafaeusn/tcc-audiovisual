//! Tudo é comando: o botão, o atalho, a barra de comandos e (no futuro) a skin e os módulos chamam a mesma ação
//! pelo mesmo nome.

use crate::{Modelo, Tema};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pagina {
    Inicio,
    Biblioteca,
    Loja,
    /// Configurações: tudo o que muda o Player, em seções.
    Configuracoes,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Comando {
    TocarPausar,
    Proxima,
    Anterior,
    AumentarVolume,
    DiminuirVolume,
    Silenciar,
    TelaCheia,
    AlternarPainel,
    AbrirAjustes,
    AbrirPagina(Pagina),
    AbrirPaleta,
    Fechar,
    TrocarVisual(Modelo),
    ProximoVisual,
    AlternarCoresReativas,
    AlternarModoSeguro,
    ProximoTema,
    TrocarTema(Tema),
    AlternarImersivo,
    EditarLayout,
    TrocarLado,
    /// Janela pequena no canto da tela, por cima das outras.
    AlternarMini,
    /// Fechar a janela mantém o app na bandeja (ou não).
    AlternarBandeja,
    /// A Ilha: a pílula com o que toca, flutuando no topo da tela.
    AlternarIlha,
    /// A Ilha abre ao parar o mouse em cima (ou só com um clique).
    AlternarIlhaAoPairar,
    /// A Ilha avisa música nova, volume e drop.
    AlternarIlhaAvisos,
    /// O tamanho da Ilha (mínima, padrão, grande).
    TrocarTamanhoIlha(crate::TamanhoIlha),
    /// A Ilha volta ao centro do topo da tela, presa (depois de arrastada ou solta).
    CentralizarIlha,
    /// Trava a Ilha onde está (solta, ela volta para a última borda) ou destrava para arrastar livre.
    AlternarTravaIlha,
    /// Prende a Ilha numa borda da tela (sem precisar arrastar).
    PrenderIlha(crate::BordaIlha),
    /// Manda a Ilha para o próximo monitor (com mais de um).
    IlhaOutroMonitor,
    CopiarDiagnostico,
    /// Mostrar ou esconder as dicas (nome e atalho) dos botões de ícone.
    AlternarDicas,
    /// Ordem aleatória na fila de músicas locais.
    AlternarAleatorio,
    /// Repetir: nada, a fila inteira ou a música de agora (passa por um de cada vez).
    AlternarRepetir,
    /// A fila de músicas locais (na Biblioteca).
    AbrirFila,
    /// A letra em tela cheia, sobre o visual.
    LetraTelaCheia,
}

impl Comando {
    pub const TODOS: [Comando; 49] = [
        Comando::TocarPausar,
        Comando::Proxima,
        Comando::Anterior,
        Comando::AumentarVolume,
        Comando::DiminuirVolume,
        Comando::Silenciar,
        Comando::TelaCheia,
        Comando::AlternarPainel,
        Comando::AbrirAjustes,
        Comando::AbrirPagina(Pagina::Inicio),
        Comando::AbrirPagina(Pagina::Biblioteca),
        Comando::AbrirPagina(Pagina::Loja),
        Comando::AbrirPaleta,
        Comando::Fechar,
        Comando::TrocarVisual(Modelo::Spectral),
        Comando::TrocarVisual(Modelo::Fluxo),
        Comando::TrocarVisual(Modelo::Barras),
        Comando::ProximoVisual,
        Comando::AlternarCoresReativas,
        Comando::AlternarModoSeguro,
        Comando::ProximoTema,
        Comando::TrocarTema(Tema::Noite),
        Comando::TrocarTema(Tema::Grafite),
        Comando::TrocarTema(Tema::Papel),
        Comando::TrocarTema(Tema::Mono),
        Comando::AlternarImersivo,
        Comando::EditarLayout,
        Comando::TrocarLado,
        Comando::AlternarMini,
        Comando::AlternarBandeja,
        Comando::AlternarIlha,
        Comando::AlternarIlhaAoPairar,
        Comando::AlternarIlhaAvisos,
        Comando::TrocarTamanhoIlha(crate::TamanhoIlha::Minima),
        Comando::TrocarTamanhoIlha(crate::TamanhoIlha::Padrao),
        Comando::TrocarTamanhoIlha(crate::TamanhoIlha::Grande),
        Comando::CentralizarIlha,
        Comando::AlternarTravaIlha,
        Comando::PrenderIlha(crate::BordaIlha::Topo),
        Comando::PrenderIlha(crate::BordaIlha::Baixo),
        Comando::PrenderIlha(crate::BordaIlha::Esquerda),
        Comando::PrenderIlha(crate::BordaIlha::Direita),
        Comando::IlhaOutroMonitor,
        Comando::CopiarDiagnostico,
        Comando::AlternarDicas,
        Comando::AlternarAleatorio,
        Comando::AlternarRepetir,
        Comando::AbrirFila,
        Comando::LetraTelaCheia,
    ];

    /// Nome estável do comando (o mesmo que atalhos e skins vão usar).
    pub fn id(self) -> &'static str {
        match self {
            Self::TocarPausar => "player.tocar-pausar",
            Self::Proxima => "player.proxima",
            Self::Anterior => "player.anterior",
            Self::AumentarVolume => "player.volume-mais",
            Self::DiminuirVolume => "player.volume-menos",
            Self::Silenciar => "player.silenciar",
            Self::TelaCheia => "visual.tela-cheia",
            Self::AlternarPainel => "painel.alternar",
            Self::AbrirAjustes => "pagina.ajustes",
            Self::AbrirPagina(Pagina::Inicio) => "pagina.inicio",
            Self::AbrirPagina(Pagina::Biblioteca) => "pagina.biblioteca",
            Self::AbrirPagina(Pagina::Loja) => "pagina.loja",
            Self::AbrirPagina(Pagina::Configuracoes) => "pagina.configuracoes",
            Self::AbrirPaleta => "comandos.abrir",
            Self::Fechar => "pagina.fechar",
            Self::TrocarVisual(Modelo::Spectral) => "visual.spectral",
            Self::TrocarVisual(Modelo::Fluxo) => "visual.fluxo",
            Self::TrocarVisual(Modelo::Barras) => "visual.barras",
            Self::ProximoVisual => "visual.proximo",
            Self::AlternarCoresReativas => "visual.cores-reativas",
            Self::AlternarModoSeguro => "visual.modo-seguro",
            Self::ProximoTema => "tema.proximo",
            Self::TrocarTema(Tema::Noite) => "tema.noite",
            Self::TrocarTema(Tema::Grafite) => "tema.grafite",
            Self::TrocarTema(Tema::Papel) => "tema.papel",
            Self::TrocarTema(Tema::Mono) => "tema.mono",
            Self::AlternarImersivo => "skin.imersiva",
            Self::EditarLayout => "layout.editar",
            Self::TrocarLado => "layout.trocar-lado",
            Self::AlternarMini => "janela.mini",
            Self::AlternarBandeja => "janela.bandeja",
            Self::AlternarIlha => "janela.ilha",
            Self::AlternarIlhaAoPairar => "ilha.ao-pairar",
            Self::AlternarIlhaAvisos => "ilha.avisos",
            Self::TrocarTamanhoIlha(crate::TamanhoIlha::Minima) => "ilha.tamanho-minima",
            Self::TrocarTamanhoIlha(crate::TamanhoIlha::Padrao) => "ilha.tamanho-padrao",
            Self::TrocarTamanhoIlha(crate::TamanhoIlha::Grande) => "ilha.tamanho-grande",
            Self::CentralizarIlha => "ilha.centralizar",
            Self::AlternarTravaIlha => "ilha.travar",
            Self::PrenderIlha(crate::BordaIlha::Topo) => "ilha.prender-topo",
            Self::PrenderIlha(crate::BordaIlha::Baixo) => "ilha.prender-baixo",
            Self::PrenderIlha(crate::BordaIlha::Esquerda) => "ilha.prender-esquerda",
            Self::PrenderIlha(crate::BordaIlha::Direita) => "ilha.prender-direita",
            Self::IlhaOutroMonitor => "ilha.outro-monitor",
            Self::CopiarDiagnostico => "ajuda.diagnostico",
            Self::AlternarDicas => "ajuda.dicas",
            Self::AlternarAleatorio => "player.aleatorio",
            Self::AlternarRepetir => "player.repetir",
            Self::AbrirFila => "pagina.fila",
            Self::LetraTelaCheia => "letra.tela-cheia",
        }
    }

    /// Como o comando aparece para a pessoa (barra de comandos, lista de atalhos).
    pub fn nome(self) -> &'static str {
        match self {
            Self::TocarPausar => "Tocar ou pausar",
            Self::Proxima => "Próxima faixa",
            Self::Anterior => "Faixa anterior",
            Self::AumentarVolume => "Aumentar o volume",
            Self::DiminuirVolume => "Diminuir o volume",
            Self::Silenciar => "Silenciar ou voltar o som",
            Self::TelaCheia => "Tela cheia",
            Self::AlternarPainel => "Mostrar ou esconder o painel",
            Self::AbrirAjustes => "Abrir as Configurações",
            Self::AbrirPagina(Pagina::Inicio) => "Ir para o Início",
            Self::AbrirPagina(Pagina::Biblioteca) => "Ir para a Biblioteca",
            Self::AbrirPagina(Pagina::Loja) => "Ir para a Loja",
            Self::AbrirPagina(Pagina::Configuracoes) => "Ir para as Configurações",
            Self::AbrirPaleta => "Buscar ou executar um comando",
            Self::Fechar => "Fechar ou voltar",
            Self::TrocarVisual(Modelo::Spectral) => "Visual: Spectral",
            Self::TrocarVisual(Modelo::Fluxo) => "Visual: Fluxo",
            Self::TrocarVisual(Modelo::Barras) => "Visual: Barras",
            Self::ProximoVisual => "Próximo visual",
            Self::AlternarCoresReativas => "Ligar ou desligar as cores reativas",
            Self::AlternarModoSeguro => "Ligar ou desligar o modo seguro",
            Self::ProximoTema => "Próximo tema",
            Self::TrocarTema(Tema::Noite) => "Tema: Noite",
            Self::TrocarTema(Tema::Grafite) => "Tema: Grafite",
            Self::TrocarTema(Tema::Papel) => "Tema: Papel",
            Self::TrocarTema(Tema::Mono) => "Tema: Mono",
            Self::AlternarImersivo => "Ligar ou desligar o modo imersivo",
            Self::EditarLayout => "Personalizar o layout",
            Self::TrocarLado => "Trocar o painel de lado",
            Self::AlternarMini => "Janela mini, sempre por cima",
            Self::AlternarBandeja => "Continuar na bandeja ao fechar a janela",
            Self::AlternarIlha => "Mostrar ou esconder a Ilha no topo da tela",
            Self::AlternarIlhaAoPairar => "Abrir a Ilha ao parar o mouse em cima",
            Self::AlternarIlhaAvisos => "Avisos na Ilha (música nova, volume, drop)",
            Self::TrocarTamanhoIlha(crate::TamanhoIlha::Minima) => "Ilha mínima",
            Self::TrocarTamanhoIlha(crate::TamanhoIlha::Padrao) => "Ilha no tamanho padrão",
            Self::TrocarTamanhoIlha(crate::TamanhoIlha::Grande) => "Ilha grande",
            Self::CentralizarIlha => "Voltar a Ilha ao centro do topo",
            Self::AlternarTravaIlha => "Travar ou destravar a Ilha (destravada, vai para qualquer borda)",
            Self::PrenderIlha(crate::BordaIlha::Topo) => "Prender a Ilha no topo da tela",
            Self::PrenderIlha(crate::BordaIlha::Baixo) => "Prender a Ilha embaixo da tela",
            Self::PrenderIlha(crate::BordaIlha::Esquerda) => "Prender a Ilha à esquerda da tela",
            Self::PrenderIlha(crate::BordaIlha::Direita) => "Prender a Ilha à direita da tela",
            Self::IlhaOutroMonitor => "Mandar a Ilha para o outro monitor",
            Self::CopiarDiagnostico => "Copiar o relatório de diagnóstico",
            Self::AlternarDicas => "Mostrar ou esconder as dicas dos botões",
            Self::AlternarAleatorio => "Ordem aleatória (músicas do computador)",
            Self::AlternarRepetir => "Repetir: nada, a fila ou a música",
            Self::AbrirFila => "Ver a fila de músicas",
            Self::LetraTelaCheia => "Letra em tela cheia",
        }
    }

    /// Grupo do comando na barra de comandos.
    pub fn grupo(self) -> &'static str {
        match self {
            Self::TocarPausar
            | Self::Proxima
            | Self::Anterior
            | Self::AumentarVolume
            | Self::DiminuirVolume
            | Self::Silenciar
            | Self::AlternarAleatorio
            | Self::AlternarRepetir => "Player",
            Self::TrocarVisual(_)
            | Self::ProximoVisual
            | Self::AlternarCoresReativas
            | Self::AlternarModoSeguro
            | Self::TelaCheia
            | Self::LetraTelaCheia => "Visual",
            Self::ProximoTema | Self::TrocarTema(_) | Self::AlternarImersivo | Self::AlternarDicas => "Aparência",
            Self::EditarLayout
            | Self::TrocarLado
            | Self::AlternarPainel
            | Self::AlternarMini
            | Self::AlternarBandeja
            | Self::AlternarIlha
            | Self::AlternarIlhaAoPairar
            | Self::AlternarIlhaAvisos
            | Self::TrocarTamanhoIlha(_)
            | Self::CentralizarIlha
            | Self::AlternarTravaIlha
            | Self::PrenderIlha(_)
            | Self::IlhaOutroMonitor => "Layout",
            Self::AbrirAjustes
            | Self::AbrirPagina(_)
            | Self::AbrirPaleta
            | Self::Fechar
            | Self::CopiarDiagnostico
            | Self::AbrirFila => "Ir para",
        }
    }

    /// As teclas do atalho, como aparecem na tela (vazio = sem atalho).
    pub fn teclas(self) -> &'static [&'static str] {
        match self {
            Self::TocarPausar => &["Espaço"],
            Self::Proxima => &["Ctrl", "→"],
            Self::Anterior => &["Ctrl", "←"],
            Self::AumentarVolume => &["Ctrl", "↑"],
            Self::DiminuirVolume => &["Ctrl", "↓"],
            Self::Silenciar => &["M"],
            Self::TelaCheia => &["F"],
            Self::AlternarPainel => &["P"],
            Self::AbrirAjustes => &["Ctrl", ","],
            Self::AbrirPaleta => &["Ctrl", "K"],
            Self::Fechar => &["Esc"],
            Self::TrocarVisual(Modelo::Spectral) => &["1"],
            Self::TrocarVisual(Modelo::Fluxo) => &["2"],
            Self::TrocarVisual(Modelo::Barras) => &["3"],
            Self::ProximoVisual => &["V"],
            Self::ProximoTema => &["T"],
            Self::AlternarImersivo => &["I"],
            Self::EditarLayout => &["E"],
            Self::AlternarMini => &["N"],
            Self::AlternarIlha => &["D"],
            Self::AlternarAleatorio => &["S"],
            Self::AlternarRepetir => &["R"],
            Self::AbrirFila => &["Q"],
            Self::LetraTelaCheia => &["Y"],
            Self::AbrirPagina(Pagina::Inicio) => &["H"],
            Self::AbrirPagina(Pagina::Biblioteca) => &["B"],
            Self::AbrirPagina(Pagina::Loja) => &["L"],
            _ => &[],
        }
    }

    /// Atalho de teclado → comando. `ctrl` vale para Ctrl (Windows) e Cmd (macOS).
    pub fn do_atalho(tecla: &str, ctrl: bool) -> Option<Self> {
        Some(match (tecla, ctrl) {
            ("space", false) => Self::TocarPausar,
            ("right", true) => Self::Proxima,
            ("left", true) => Self::Anterior,
            ("up", true) => Self::AumentarVolume,
            ("down", true) => Self::DiminuirVolume,
            ("m", false) => Self::Silenciar,
            ("f", false) | ("f11", false) => Self::TelaCheia,
            ("p", false) => Self::AlternarPainel,
            (",", true) => Self::AbrirAjustes,
            ("k", true) => Self::AbrirPaleta,
            ("escape", false) => Self::Fechar,
            ("1", false) => Self::TrocarVisual(Modelo::Spectral),
            ("2", false) => Self::TrocarVisual(Modelo::Fluxo),
            ("3", false) => Self::TrocarVisual(Modelo::Barras),
            ("v", false) => Self::ProximoVisual,
            ("t", false) => Self::ProximoTema,
            ("i", false) => Self::AlternarImersivo,
            ("e", false) => Self::EditarLayout,
            ("n", false) => Self::AlternarMini,
            ("d", false) => Self::AlternarIlha,
            ("s", false) => Self::AlternarAleatorio,
            ("r", false) => Self::AlternarRepetir,
            ("q", false) => Self::AbrirFila,
            ("y", false) => Self::LetraTelaCheia,
            ("h", false) => Self::AbrirPagina(Pagina::Inicio),
            ("b", false) => Self::AbrirPagina(Pagina::Biblioteca),
            ("l", false) => Self::AbrirPagina(Pagina::Loja),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn atalhos_conhecidos() {
        assert_eq!(Comando::do_atalho("space", false), Some(Comando::TocarPausar));
        assert_eq!(Comando::do_atalho("2", false), Some(Comando::TrocarVisual(Modelo::Fluxo)));
        assert_eq!(Comando::do_atalho("2", true), None);
        assert_eq!(Comando::do_atalho("k", true), Some(Comando::AbrirPaleta));
        assert_eq!(Comando::do_atalho("q", false), Some(Comando::AbrirFila));
        assert_eq!(Comando::do_atalho("z", false), None);
    }

    #[test]
    fn ids_e_nomes_sao_unicos() {
        let mut ids: Vec<_> = Comando::TODOS.iter().map(|c| c.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), Comando::TODOS.len());
        let mut nomes: Vec<_> = Comando::TODOS.iter().map(|c| c.nome()).collect();
        nomes.sort_unstable();
        nomes.dedup();
        assert_eq!(nomes.len(), Comando::TODOS.len());
    }

    #[test]
    fn todo_atalho_mostrado_existe_de_verdade() {
        // Quem tem teclas na tela precisa ser alcançável por `do_atalho` (e vice-versa não é exigido).
        let alcancaveis: Vec<Comando> = [
            ("space", false),
            ("right", true),
            ("left", true),
            ("up", true),
            ("down", true),
            ("m", false),
            ("f", false),
            ("p", false),
            (",", true),
            ("k", true),
            ("escape", false),
            ("1", false),
            ("2", false),
            ("3", false),
            ("v", false),
            ("t", false),
            ("i", false),
            ("e", false),
            ("n", false),
            ("d", false),
            ("s", false),
            ("r", false),
            ("q", false),
            ("y", false),
            ("h", false),
            ("b", false),
            ("l", false),
        ]
        .into_iter()
        .filter_map(|(t, c)| Comando::do_atalho(t, c))
        .collect();
        for c in Comando::TODOS {
            assert_eq!(!c.teclas().is_empty(), alcancaveis.contains(&c), "{}", c.id());
        }
    }
}
