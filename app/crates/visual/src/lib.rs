//! Os visuais da v1: **Spectral** (porte do "Spectral Audio Reactive" do TouchDesigner), **Fluxo** e **Barras**.
//!
//! Este crate não conhece janela nem interface: recebe um `Device` do wgpu, o [`analise::Retrato`] do som e uma
//! textura de destino. Quem cria a janela e a camada é o crate `janela`.

mod motor;
mod parametros;

pub use motor::{Destino, Motor, Quadro};
pub use parametros::Limitador;
