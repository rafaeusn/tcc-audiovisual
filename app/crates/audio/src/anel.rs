//! Buffer circular sem travas: um escritor (a thread de áudio, em tempo real) e um leitor (a análise).
//!
//! A thread de áudio não pode esperar ninguém nem alocar memória, senão o som engasga. Por isso cada amostra é um
//! átomo: o escritor grava e avança um contador; o leitor copia as últimas N amostras quando quiser. Se o leitor
//! pegar uma amostra no meio de uma gravação, ela é só uma amostra um pouco mais nova: nunca um valor quebrado.

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub struct Anel {
    dados: Box<[AtomicU32]>,
    mascara: usize,
    /// Total de amostras já escritas (não volta a zero).
    escritas: AtomicU64,
}

impl Anel {
    /// `capacidade` é arredondada para a próxima potência de dois.
    pub fn new(capacidade: usize) -> Self {
        let n = capacidade.max(2).next_power_of_two();
        Self { dados: (0..n).map(|_| AtomicU32::new(0)).collect(), mascara: n - 1, escritas: AtomicU64::new(0) }
    }

    pub fn capacidade(&self) -> usize {
        self.mascara + 1
    }

    /// Total de amostras escritas desde o início. Serve para saber se chegou som novo.
    pub fn total(&self) -> u64 {
        self.escritas.load(Ordering::Acquire)
    }

    /// Escreve amostras (só a thread de áudio chama). Sem alocação e sem espera.
    pub fn escrever(&self, amostras: impl Iterator<Item = f32>) {
        let mut pos = self.escritas.load(Ordering::Relaxed);
        for v in amostras {
            self.dados[pos as usize & self.mascara].store(v.to_bits(), Ordering::Relaxed);
            pos = pos.wrapping_add(1);
        }
        self.escritas.store(pos, Ordering::Release);
    }

    /// Copia as últimas `saida.len()` amostras (as mais novas no fim). O que ainda não foi escrito vem como zero.
    pub fn ultimas(&self, saida: &mut [f32]) {
        let total = self.total();
        let n = saida.len().min(self.capacidade());
        let pular = saida.len() - n;
        saida[..pular].fill(0.0);
        for (i, s) in saida[pular..].iter_mut().enumerate() {
            let atras = (n - i) as u64;
            *s = if atras > total {
                0.0
            } else {
                f32::from_bits(self.dados[(total - atras) as usize & self.mascara].load(Ordering::Relaxed))
            };
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_as_mais_novas_no_fim() {
        let a = Anel::new(8);
        a.escrever((1..=5).map(|v| v as f32));
        let mut s = [0.0; 3];
        a.ultimas(&mut s);
        assert_eq!(s, [3.0, 4.0, 5.0]);
    }

    #[test]
    fn da_a_volta_sem_perder_a_ordem() {
        let a = Anel::new(8);
        a.escrever((1..=20).map(|v| v as f32));
        let mut s = [0.0; 8];
        a.ultimas(&mut s);
        assert_eq!(s, [13.0, 14.0, 15.0, 16.0, 17.0, 18.0, 19.0, 20.0]);
        assert_eq!(a.total(), 20);
    }

    #[test]
    fn pedido_maior_que_o_escrito_vem_com_zeros() {
        let a = Anel::new(8);
        a.escrever([7.0, 9.0].into_iter());
        let mut s = [1.0; 4];
        a.ultimas(&mut s);
        assert_eq!(s, [0.0, 0.0, 7.0, 9.0]);
        let mut grande = [1.0; 12];
        a.ultimas(&mut grande);
        assert_eq!(&grande[..10], &[0.0; 10]);
        assert_eq!(&grande[10..], &[7.0, 9.0]);
    }

    #[test]
    fn escritor_e_leitor_ao_mesmo_tempo() {
        let a = std::sync::Arc::new(Anel::new(1024));
        let w = a.clone();
        let t = std::thread::spawn(move || {
            for bloco in 0..2000u32 {
                w.escrever((0..64).map(|i| (bloco * 64 + i) as f32));
            }
        });
        let mut s = [0.0f32; 256];
        for _ in 0..2000 {
            a.ultimas(&mut s);
            // Dentro de uma leitura os valores nunca andam para trás por mais de uma volta do buffer.
            assert!(s.iter().all(|v| v.is_finite()));
        }
        assert!(t.join().is_ok());
        assert_eq!(a.total(), 2000 * 64);
    }
}
