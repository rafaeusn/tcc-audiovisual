//! O som que o próprio Player toca, como sai da mistura dele e antes do volume da música. O visual o ouve direto, sem dar a volta
//! pelo Windows: baixar a música (ou silenciar o PC) não muda o que ele vê, e o resto do PC não entra.

use crate::Anel;
use std::sync::atomic::{AtomicU32, Ordering};

/// Menos de meio segundo: o bastante para a análise.
const CAPACIDADE: usize = 16_384;

pub struct Derivacao {
    pub anel: Anel,
    taxa: AtomicU32,
}

impl Default for Derivacao {
    fn default() -> Self {
        Self { anel: Anel::new(CAPACIDADE), taxa: AtomicU32::new(48_000) }
    }
}

impl Derivacao {
    /// A taxa das amostras (a da placa de som).
    pub fn taxa(&self) -> u32 {
        self.taxa.load(Ordering::Acquire)
    }

    /// A saída de som abriu nesta taxa.
    pub fn abriu(&self, taxa: u32) {
        self.taxa.store(taxa, Ordering::Release);
    }
}
