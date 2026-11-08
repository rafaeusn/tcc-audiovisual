//! Partes comuns aos protótipos de UI (Slint e GPUI). Tudo aqui é independente de framework, para que a
//! comparação meça só a camada de interface.

pub mod audio;
pub mod layout;
pub mod metrics;
pub mod visual;

/// Layout padrão dos protótipos (o mesmo arquivo nos dois).
pub const DEFAULT_LAYOUT: &str = include_str!("../../layout.toml");
