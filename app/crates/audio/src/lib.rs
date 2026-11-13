//! Modo Escuta: o som que o PC está tocando, capturado da saída padrão (loopback WASAPI).
//!
//! Por padrão, tudo o que toca (Spotify, navegador, qualquer player) chega aqui já misturado; também dá para
//! ouvir um app só (`CapturaDeApp`). Nada é gravado em disco nem sai
//! do computador: as amostras ficam num buffer circular de menos de meio segundo e são sobrescritas.

mod anel;
mod captura;
mod processo;
pub mod volume;

pub use anel::Anel;
pub use captura::{Captura, nome_da_saida_padrao};
pub use processo::{CapturaDeApp, Panorama, Processo, TAXA_DE_APP};
