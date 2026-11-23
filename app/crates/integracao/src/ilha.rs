//! A Ilha no Windows: a janela da pílula fica sempre por cima das outras, não rouba o foco de quem está sendo
//! usado e fica num dos monitores (o da janela principal, ou o que a pessoa escolheu; onde exatamente, a Ilha decide:
//! ver `player::ilha::encaixe`).
//!
//! Mover e redimensionar vão por mensagem para a própria janela (o GPUI perde o aviso de tamanho quando a troca
//! acontece dentro de um evento dele). Fora do Windows, tudo aqui não faz nada.

/// Um monitor: o retângulo em pixels da tela, a área de trabalho (sem a barra de tarefas: esquerda, topo, direita,
/// baixo), a escala do DPI dele (1,0 = 100%), o nome que o Windows dá a ele (`\\.\DISPLAY2`, guardado nos ajustes
/// para a Ilha voltar ao mesmo monitor) e se é o principal.
#[derive(Clone, Debug, PartialEq)]
pub struct Monitor {
    pub esquerda: i32,
    pub topo: i32,
    pub direita: i32,
    pub baixo: i32,
    pub area: (i32, i32, i32, i32),
    pub escala: f32,
    pub nome: String,
    pub principal: bool,
}

impl Monitor {
    /// O ponto (pixels da tela) está dentro deste monitor.
    pub fn contem(&self, x: i32, y: i32) -> bool {
        x >= self.esquerda && x < self.direita && y >= self.topo && y < self.baixo
    }
}

/// Os monitores ligados: o principal primeiro (o "Monitor 1", como o Windows costuma numerar), depois os outros da
/// esquerda para a direita (e de cima para baixo).
pub fn monitores() -> Vec<Monitor> {
    #[cfg(windows)]
    {
        let mut todos = crate::sistema_ilha::monitores();
        todos.sort_by_key(|m| (!m.principal, m.esquerda, m.topo));
        todos
    }
    #[cfg(not(windows))]
    {
        Vec::new()
    }
}

/// O monitor no ponto `x`, `y` (pixels da tela), ou o mais perto dele.
pub fn monitor_no_ponto(x: i32, y: i32) -> Option<Monitor> {
    #[cfg(windows)]
    {
        crate::sistema_ilha::monitor_no_ponto(x, y)
    }
    #[cfg(not(windows))]
    {
        let _ = (x, y);
        None
    }
}

/// Deixa a janela `hwnd` pronta para ser a Ilha: sem moldura, sempre por cima, sem roubar o foco ao ser clicada.
/// Devolve se deu certo.
pub fn preparar(hwnd: isize) -> bool {
    #[cfg(windows)]
    {
        crate::sistema_ilha::preparar(hwnd)
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}

/// O monitor da janela `hwnd`.
pub fn monitor(hwnd: isize) -> Option<Monitor> {
    #[cfg(windows)]
    {
        crate::sistema_ilha::monitor(hwnd)
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        None
    }
}

/// Move a Ilha para `x`, `y` (pixels do monitor), com o tamanho dado em pixels lógicos.
pub fn mover(hwnd: isize, x: i32, y: i32, largura: f32, altura: f32) {
    #[cfg(windows)]
    crate::sistema_ilha::mover(hwnd, x, y, largura, altura);
    #[cfg(not(windows))]
    let _ = (hwnd, x, y, largura, altura);
}

/// A janela `hwnd` está na frente e ocupa a tela inteira (maximizada ou em tela cheia).
pub fn na_frente_ocupando_a_tela(hwnd: isize) -> bool {
    #[cfg(windows)]
    {
        crate::sistema_ilha::na_frente_ocupando_a_tela(hwnd)
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}

/// O botão esquerdo do mouse está apertado agora (para o arrasto da Ilha).
pub fn botao_apertado() -> bool {
    #[cfg(windows)]
    {
        crate::sistema_ilha::botao_apertado()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Onde está o ponteiro, em pixels da tela.
pub fn cursor() -> Option<(i32, i32)> {
    #[cfg(windows)]
    {
        crate::sistema_ilha::cursor()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Mostra (sem ativar) ou esconde a Ilha.
pub fn mostrar(hwnd: isize, visivel: bool) {
    #[cfg(windows)]
    crate::sistema_ilha::mostrar(hwnd, visivel);
    #[cfg(not(windows))]
    let _ = (hwnd, visivel);
}

/// Outro app está em tela cheia (jogo, vídeo, apresentação): a Ilha não deve aparecer por cima.
pub fn tela_cheia_de_outro() -> bool {
    #[cfg(windows)]
    {
        crate::sistema_ilha::tela_cheia_de_outro()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// O Windows pede menos movimento ("Efeitos de animação" desligado nas configurações de acessibilidade).
pub fn movimento_reduzido() -> bool {
    #[cfg(windows)]
    {
        crate::sistema_ilha::movimento_reduzido()
    }
    #[cfg(not(windows))]
    {
        false
    }
}

/// Só o retângulo `esquerda..direita` × `topo..baixo` (pixels lógicos da janela) recebe cliques e aparece; o resto
/// da janela deixa os cliques passarem para o que estiver embaixo.
pub fn regiao(hwnd: isize, esquerda: f32, topo: f32, direita: f32, baixo: f32) {
    #[cfg(windows)]
    crate::sistema_ilha::regiao(hwnd, esquerda, topo, direita, baixo);
    #[cfg(not(windows))]
    let _ = (hwnd, esquerda, topo, direita, baixo);
}

/// Onde está o ponteiro, em pixels lógicos a partir do canto de cima à esquerda da janela `hwnd` (pode estar fora).
pub fn ponteiro_relativo(hwnd: isize) -> Option<(f32, f32)> {
    #[cfg(windows)]
    {
        crate::sistema_ilha::ponteiro_relativo(hwnd)
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        None
    }
}

/// O ponteiro do mouse está dentro da janela `hwnd`.
pub fn ponteiro_dentro(hwnd: isize) -> bool {
    #[cfg(windows)]
    {
        crate::sistema_ilha::ponteiro_dentro(hwnd)
    }
    #[cfg(not(windows))]
    {
        let _ = hwnd;
        false
    }
}
