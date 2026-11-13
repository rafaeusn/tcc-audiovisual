//! A camada do visual, por baixo da interface (Windows).
//!
//! O GPUI desenha a interface numa camada do DirectComposition marcada como "de cima" e deixa transparente onde
//! não há nada. O Windows aceita uma segunda camada na mesma janela: criamos essa camada, entregamos ao wgpu
//! (DX12) e desenhamos nela o fundo e o visual. Quem junta as duas é o compositor do sistema, na GPU. Nenhum
//! pixel passa pela CPU.
//!
//! Analogia: a interface é uma folha de acetato; o visual é a folha de papel embaixo.

#[cfg(windows)]
mod camada;

#[cfg(windows)]
pub use camada::Camada;
