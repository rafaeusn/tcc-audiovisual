//! Volume da saída de som padrão do Windows, e o de um app no misturador dele.
//!
//! O da saída é o que vale para o que os outros apps tocam: o Modo Escuta analisa o som antes dele. O de um app
//! ([`definir_dos`]) vale para o Soloist, que o Player abre: a captura vem **depois** dele (medido em 25/11/2026: 0,25 deixa 0,25
//! e 0,00001 deixa 0,00001, sem perder detalhe em ponto flutuante; a mudança chega nela uns 50 ms depois), e quem ouve desfaz a
//! conta. (Um programa que captura a si mesmo vê o volume dele duas vezes: no que toca e no que captura.)
//!
//! As funções precisam ser chamadas de uma thread com COM iniciado (a da interface já tem). Cada chamada busca
//! a saída padrão de novo, então trocar de fone ou caixa não quebra nada.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Nivel {
    /// De 0 a 1.
    pub volume: f32,
    pub mudo: bool,
}

/// O som aberto por um processo, no misturador do Windows.
#[derive(Clone, Debug, PartialEq)]
pub struct SomDeApp {
    /// Muda a cada abertura, e de uma saída de som para outra.
    pub id: String,
    pub pid: u32,
    /// De 0 a 1.
    pub volume: f32,
}

#[cfg(windows)]
mod so {
    use super::Nivel;
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        AudioSessionStateExpired, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator,
        ISimpleAudioVolume, MMDeviceEnumerator, eMultimedia, eRender,
    };
    use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree};
    use windows::core::{Interface, Result};

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

    /// As sessões de som destes processos na saída padrão.
    pub fn sessoes(pids: &[u32]) -> Result<Vec<(String, u32, ISimpleAudioVolume)>> {
        crate::processo::ligar_o_com();
        // SAFETY: chamadas COM comuns; os objetos devolvidos são contados por referência e soltos no fim do escopo.
        unsafe {
            let enumerador: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let saida = enumerador.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let gerente: IAudioSessionManager2 = saida.Activate(CLSCTX_ALL, None)?;
            let lista = gerente.GetSessionEnumerator()?;
            let mut achadas = Vec::new();
            for i in 0..lista.GetCount()? {
                let Ok(sessao) = lista.GetSession(i) else { continue };
                if sessao.GetState().is_ok_and(|e| e == AudioSessionStateExpired) {
                    continue;
                }
                let Ok(controle) = sessao.cast::<IAudioSessionControl2>() else { continue };
                let Ok(pid) = controle.GetProcessId() else { continue };
                if pids.contains(&pid) {
                    let texto = controle.GetSessionInstanceIdentifier()?;
                    let id = texto.to_string().unwrap_or_default();
                    CoTaskMemFree(Some(texto.0.cast()));
                    achadas.push((id, pid, sessao.cast()?));
                }
            }
            Ok(achadas)
        }
    }
}

/// O som que estes processos têm aberto na saída padrão, com o volume dele no misturador do Windows.
pub fn ler_dos(pids: &[u32]) -> Vec<SomDeApp> {
    #[cfg(windows)]
    {
        let sessoes = so::sessoes(pids).unwrap_or_default();
        // SAFETY: as sessões são válidas, recém-obtidas.
        let volume = |s: &windows::Win32::Media::Audio::ISimpleAudioVolume| unsafe { s.GetMasterVolume() }.ok();
        let som = |(id, pid, s): (String, u32, _)| volume(&s).map(|volume| SomDeApp { id, pid, volume });
        sessoes.into_iter().filter_map(som).collect()
    }
    #[cfg(not(windows))]
    {
        let _ = pids;
        Vec::new()
    }
}

/// Define o volume (0 a 1) destes processos no misturador do Windows.
pub fn definir_dos(pids: &[u32], volume: f32) {
    if !volume.is_finite() {
        return;
    }
    #[cfg(windows)]
    for (_, _, s) in so::sessoes(pids).unwrap_or_default() {
        // SAFETY: a sessão é válida; o ponteiro nulo é o "contexto do evento", que é opcional.
        if let Err(e) = unsafe { s.SetMasterVolume(volume.clamp(0.0, 1.0), std::ptr::null()) } {
            log::debug!("não foi possível mudar o volume do app: {e}");
        }
    }
    #[cfg(not(windows))]
    let _ = pids;
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
