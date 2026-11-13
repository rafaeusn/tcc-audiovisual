//! Volume da saída de som padrão do Windows.
//!
//! É o volume geral do sistema, não o do app que está tocando. Motivo: o Modo Escuta analisa o som antes desse
//! volume, então baixar o som por aqui não deixa o visual mais fraco. Mexer no volume do app de origem mudaria
//! o que a análise ouve.
//!
//! As funções precisam ser chamadas de uma thread com COM iniciado (a da interface já tem). Cada chamada busca
//! a saída padrão de novo, então trocar de fone ou caixa não quebra nada.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nivel {
    /// De 0 a 1.
    pub volume: f32,
    pub mudo: bool,
}

#[cfg(windows)]
mod so {
    use super::Nivel;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{IMMDeviceEnumerator, MMDeviceEnumerator, eMultimedia, eRender};
    use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance};
    use windows::core::Result;

    fn controle() -> Result<IAudioEndpointVolume> {
        // SAFETY: chamadas COM comuns; os objetos devolvidos são contados por referência e soltos no fim do escopo.
        unsafe {
            let enumerador: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let saida = enumerador.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            saida.Activate(CLSCTX_ALL, None)
        }
    }

    pub fn ler() -> Result<Nivel> {
        let c = controle()?;
        // SAFETY: `c` é um controle de volume válido, recém-obtido.
        unsafe { Ok(Nivel { volume: c.GetMasterVolumeLevelScalar()?.clamp(0.0, 1.0), mudo: c.GetMute()?.as_bool() }) }
    }

    pub fn definir(volume: f32) -> Result<()> {
        let c = controle()?;
        // SAFETY: idem; o ponteiro nulo é o "contexto do evento", que é opcional.
        unsafe { c.SetMasterVolumeLevelScalar(volume, std::ptr::null()) }
    }

    pub fn silenciar(mudo: bool) -> Result<()> {
        let c = controle()?;
        // SAFETY: idem.
        unsafe { c.SetMute(mudo, std::ptr::null()) }
    }
}

/// Volume atual da saída padrão. `None` se não há saída de som ou o sistema não respondeu.
pub fn ler() -> Option<Nivel> {
    #[cfg(windows)]
    {
        so::ler().ok()
    }
    #[cfg(not(windows))]
    {
        None
    }
}

/// Define o volume (0 a 1). Valores inválidos são ignorados. Devolve se deu certo.
pub fn definir(volume: f32) -> bool {
    if !volume.is_finite() {
        return false;
    }
    let volume = volume.clamp(0.0, 1.0);
    #[cfg(windows)]
    {
        match so::definir(volume) {
            Ok(()) => true,
            Err(e) => {
                log::debug!("não foi possível mudar o volume: {e}");
                false
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = volume;
        false
    }
}

/// Liga ou desliga o mudo da saída padrão. Devolve se deu certo.
pub fn silenciar(mudo: bool) -> bool {
    #[cfg(windows)]
    {
        match so::silenciar(mudo) {
            Ok(()) => true,
            Err(e) => {
                log::debug!("não foi possível mudar o mudo: {e}");
                false
            }
        }
    }
    #[cfg(not(windows))]
    {
        let _ = mudo;
        false
    }
}
