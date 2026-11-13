//! Abre o loopback da saída padrão e despeja as amostras (mono) no [`Anel`].

use crate::Anel;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use nucleo::{Erro, Resultado};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Uma captura em andamento. Soltar o valor encerra a captura.
///
/// O `cpal::Stream` não pode trocar de thread: quem cria a `Captura` é quem a guarda até o fim.
pub struct Captura {
    _stream: cpal::Stream,
    falhou: Arc<AtomicBool>,
    pub taxa: u32,
    pub dispositivo: String,
}

impl Captura {
    /// Abre a saída padrão em modo loopback. No WASAPI, abrir um stream de *entrada* num dispositivo de *saída*
    /// liga o loopback.
    pub fn abrir(anel: Arc<Anel>) -> Resultado<Self> {
        let erro = |e: &dyn std::fmt::Display| Erro::Audio(e.to_string());
        let host = cpal::default_host();
        let device =
            host.default_output_device().ok_or_else(|| Erro::Audio("nenhuma saída de áudio encontrada".into()))?;
        let dispositivo = nome(&device);
        let suportada = device.default_output_config().map_err(|e| erro(&e))?;
        let formato = suportada.sample_format();
        let config = suportada.config();
        let canais = (config.channels as usize).max(1);
        let taxa = config.sample_rate;
        if !(8000..=384_000).contains(&taxa) {
            return Err(Erro::Audio(format!("taxa de amostragem fora do esperado: {taxa} Hz")));
        }

        let falhou = Arc::new(AtomicBool::new(false));
        let sinal = falhou.clone();
        let ao_falhar = move |e: cpal::Error| {
            // No loopback o Windows marca uma "descontinuidade" sempre que o som começa ou recomeça. Isso é
            // normal e a captura segue funcionando; só os outros erros encerram a captura.
            if matches!(e.kind(), cpal::ErrorKind::Xrun | cpal::ErrorKind::RealtimeDenied) {
                return;
            }
            // Dispositivo removido ou trocado: quem cuida da captura vê o sinal e reabre.
            log::warn!("captura de áudio interrompida: {e}");
            sinal.store(true, Ordering::Release);
        };
        let inv = 1.0 / canais as f32;
        let stream = match formato {
            cpal::SampleFormat::F32 => device.build_input_stream::<f32, _, _>(
                config,
                move |dados: &[f32], _| anel.escrever(dados.chunks_exact(canais).map(|q| q.iter().sum::<f32>() * inv)),
                ao_falhar,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream::<i16, _, _>(
                config,
                move |dados: &[i16], _| {
                    anel.escrever(
                        dados.chunks_exact(canais).map(|q| q.iter().map(|s| *s as f32 / 32768.0).sum::<f32>() * inv),
                    )
                },
                ao_falhar,
                None,
            ),
            cpal::SampleFormat::I32 => device.build_input_stream::<i32, _, _>(
                config,
                move |dados: &[i32], _| {
                    anel.escrever(
                        dados
                            .chunks_exact(canais)
                            .map(|q| q.iter().map(|s| *s as f32 / 2_147_483_648.0).sum::<f32>() * inv),
                    )
                },
                ao_falhar,
                None,
            ),
            outro => return Err(Erro::Audio(format!("formato de amostra não suportado: {outro:?}"))),
        }
        .map_err(|e| erro(&e))?;
        stream.play().map_err(|e| erro(&e))?;
        log::info!("Modo Escuta: ouvindo \"{dispositivo}\" ({taxa} Hz, {canais} canais, {formato:?})");
        Ok(Self { _stream: stream, falhou, taxa, dispositivo })
    }

    /// Verdadeiro se o sistema encerrou a captura (dispositivo removido, por exemplo).
    pub fn falhou(&self) -> bool {
        self.falhou.load(Ordering::Acquire)
    }
}

fn nome(device: &cpal::Device) -> String {
    device.description().map(|d| d.to_string()).unwrap_or_else(|_| "saída padrão".into())
}

/// Nome da saída padrão de agora (para perceber quando o usuário troca de fone ou caixa).
pub fn nome_da_saida_padrao() -> Option<String> {
    cpal::default_host().default_output_device().map(|d| nome(&d))
}
