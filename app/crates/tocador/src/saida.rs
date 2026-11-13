//! A saída de som: um fluxo aberto na placa padrão, alimentado por uma fila sem trava.
//!
//! A função que a placa chama para pedir som roda numa thread de tempo real: ela só tira amostras da fila e
//! preenche com silêncio o que faltar. Nada ali espera, aloca ou trava.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use nucleo::{Erro, Resultado};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// O que a thread do tocador e a função de som dividem.
#[derive(Default)]
pub struct Painel {
    pub pausado: AtomicBool,
    /// Pedido para jogar fora o que está na fila (troca de música, ida para outro ponto).
    pub descartar: AtomicBool,
    /// Quadros já entregues à placa desde a última vez que o contador foi zerado.
    pub tocados: AtomicU64,
    /// Toca em silêncio (para testar sem fazer barulho).
    pub mudo: AtomicBool,
    pub falhou: AtomicBool,
}

pub struct Saida {
    _fluxo: cpal::Stream,
    pub taxa: u32,
    pub fila: rtrb::Producer<f32>,
    pub painel: Arc<Painel>,
}

/// Preenche `destino` (com `canais` por quadro) a partir da fila estéreo.
fn preencher(destino: &mut [f32], canais: usize, fila: &mut rtrb::Consumer<f32>, painel: &Painel) {
    if painel.descartar.swap(false, Ordering::AcqRel) {
        while fila.pop().is_ok() {}
    }
    let mudo = painel.mudo.load(Ordering::Relaxed);
    let pausado = painel.pausado.load(Ordering::Acquire);
    let mut entregues = 0u64;
    for quadro in destino.chunks_mut(canais.max(1)) {
        let par = if pausado || fila.slots() < 2 { None } else { fila.pop().ok().zip(fila.pop().ok()) };
        let (esquerda, direita) = match par {
            Some(p) => {
                entregues += 1;
                if mudo { (0.0, 0.0) } else { p }
            }
            None => (0.0, 0.0),
        };
        match quadro {
            [unico] => *unico = (esquerda + direita) * 0.5,
            [e, d, resto @ ..] => {
                *e = esquerda;
                *d = direita;
                resto.fill(0.0);
            }
            [] => {}
        }
    }
    painel.tocados.fetch_add(entregues, Ordering::AcqRel);
}

impl Saida {
    /// Abre a saída padrão do sistema. A fila guarda meio segundo de som.
    pub fn abrir() -> Resultado<Self> {
        let erro = |e: &dyn std::fmt::Display| Erro::Audio(e.to_string());
        let dispositivo = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| Erro::Audio("nenhuma saída de áudio encontrada".into()))?;
        let suportada = dispositivo.default_output_config().map_err(|e| erro(&e))?;
        let formato = suportada.sample_format();
        let config = suportada.config();
        let canais = (config.channels as usize).max(1);
        let taxa = config.sample_rate;
        if !(8000..=384_000).contains(&taxa) {
            return Err(Erro::Audio(format!("taxa de amostragem fora do esperado: {taxa} Hz")));
        }
        let (fila, mut consumo) = rtrb::RingBuffer::<f32>::new(taxa as usize);
        let painel = Arc::new(Painel::default());
        let sinal = painel.clone();
        let ao_falhar = move |e: cpal::Error| {
            if !matches!(e.kind(), cpal::ErrorKind::Xrun | cpal::ErrorKind::RealtimeDenied) {
                log::warn!("a saída de som parou: {e}");
                sinal.falhou.store(true, Ordering::Release);
            }
        };
        let p = painel.clone();
        let fluxo = match formato {
            cpal::SampleFormat::F32 => dispositivo.build_output_stream::<f32, _, _>(
                config,
                move |destino, _| preencher(destino, canais, &mut consumo, &p),
                ao_falhar,
                None,
            ),
            cpal::SampleFormat::I16 => {
                let mut rascunho: Vec<f32> = Vec::new();
                dispositivo.build_output_stream::<i16, _, _>(
                    config,
                    move |destino, _| {
                        rascunho.resize(destino.len(), 0.0);
                        preencher(&mut rascunho, canais, &mut consumo, &p);
                        for (d, v) in destino.iter_mut().zip(&rascunho) {
                            *d = (v.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
                        }
                    },
                    ao_falhar,
                    None,
                )
            }
            outro => return Err(Erro::Audio(format!("formato de saída não suportado: {outro:?}"))),
        }
        .map_err(|e| erro(&e))?;
        fluxo.play().map_err(|e| erro(&e))?;
        Ok(Self { _fluxo: fluxo, taxa, fila, painel })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn fila_com(valores: &[f32]) -> (rtrb::Producer<f32>, rtrb::Consumer<f32>) {
        let (mut p, c) = rtrb::RingBuffer::<f32>::new(64);
        for v in valores {
            p.push(*v).expect("cabe");
        }
        (p, c)
    }

    #[test]
    fn estereo_falta_vira_silencio_e_conta_o_que_tocou() {
        let (_p, mut c) = fila_com(&[0.1, 0.2, 0.3, 0.4]);
        let painel = Painel::default();
        let mut destino = [9.0f32; 8];
        preencher(&mut destino, 2, &mut c, &painel);
        assert_eq!(destino, [0.1, 0.2, 0.3, 0.4, 0.0, 0.0, 0.0, 0.0]);
        assert_eq!(painel.tocados.load(Ordering::Acquire), 2);
    }

    #[test]
    fn pausado_nao_consome_e_descartar_esvazia() {
        let (_p, mut c) = fila_com(&[0.1, 0.2, 0.3, 0.4]);
        let painel = Painel::default();
        painel.pausado.store(true, Ordering::Release);
        let mut destino = [9.0f32; 4];
        preencher(&mut destino, 2, &mut c, &painel);
        assert_eq!(destino, [0.0; 4]);
        assert_eq!(c.slots(), 4, "a fila continua inteira");
        painel.pausado.store(false, Ordering::Release);
        painel.descartar.store(true, Ordering::Release);
        preencher(&mut destino, 2, &mut c, &painel);
        assert_eq!(destino, [0.0; 4], "o que estava na fila foi jogado fora");
        assert_eq!(painel.tocados.load(Ordering::Acquire), 0);
    }

    #[test]
    fn mono_mistura_e_muitos_canais_usam_os_dois_primeiros() {
        let (_p, mut c) = fila_com(&[0.2, 0.4]);
        let painel = Painel::default();
        let mut mono = [9.0f32; 1];
        preencher(&mut mono, 1, &mut c, &painel);
        assert!((mono[0] - 0.3).abs() < 1e-6);
        let (_p, mut c) = fila_com(&[0.2, 0.4]);
        let mut seis = [9.0f32; 6];
        preencher(&mut seis, 6, &mut c, &painel);
        assert_eq!(seis, [0.2, 0.4, 0.0, 0.0, 0.0, 0.0]);
        // No mudo a música anda, mas não sai som.
        let (_p, mut c) = fila_com(&[0.2, 0.4]);
        painel.mudo.store(true, Ordering::Release);
        let mut par = [9.0f32; 2];
        preencher(&mut par, 2, &mut c, &painel);
        assert_eq!(par, [0.0, 0.0]);
        assert_eq!(painel.tocados.load(Ordering::Acquire), 3);
    }
}
