//! Ouvir um app só: a lista dos apps que tocam som e a captura do som de um processo (com os processos que ele
//! abre), sem misturar o resto do sistema.
//!
//! O Windows oferece isso desde a versão 10 de 2021 ("process loopback"). Em versões mais antigas a abertura
//! falha e quem chamou decide o que fazer.

use crate::Anel;
use nucleo::{Erro, Resultado};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;

/// A taxa em que o som de um app é entregue (o Windows converte para ela).
pub const TAXA_DE_APP: u32 = 48_000;

/// Um processo do sistema: número, número de quem o abriu e o nome do executável em minúsculas.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Processo {
    pub pid: u32,
    pub pai: u32,
    pub exe: String,
}

/// Sobe de `pid` até o primeiro processo do mesmo app (o que abriu os outros). Navegadores e o Spotify tocam o
/// som num processo filho; capturar a partir do primeiro pega todos.
pub fn raiz(pid: u32, processos: &[Processo]) -> u32 {
    let achar = |pid: u32| processos.iter().find(|p| p.pid == pid);
    let Some(mut atual) = achar(pid) else { return pid };
    // O limite evita uma volta sem fim se o Windows reaproveitou um número de processo.
    for _ in 0..32 {
        match achar(atual.pai) {
            Some(pai) if pai.exe == atual.exe && pai.pid != atual.pid => atual = pai,
            _ => break,
        }
    }
    atual.pid
}

/// O processo a capturar para ouvir o app `exe`: o primeiro processo de quem está tocando som; se ninguém do
/// app tem som aberto ainda, o primeiro processo dele. `None` quando o app não está aberto.
pub fn alvo(exe: &str, com_som: &[u32], processos: &[Processo]) -> Option<u32> {
    let exe = exe.to_lowercase();
    let do_app = |pid: &u32| processos.iter().any(|p| p.pid == *pid && p.exe == exe);
    com_som
        .iter()
        .find(|pid| do_app(pid))
        .copied()
        .or_else(|| processos.iter().find(|p| p.exe == exe).map(|p| p.pid))
        .map(|pid| raiz(pid, processos))
}

/// Os executáveis que têm som aberto, sem repetir e sem o Player (nem este, nem outra cópia dele aberta), em
/// ordem alfabética.
pub fn nomes_com_som(com_som: &[u32], processos: &[Processo], proprio: u32) -> Vec<String> {
    let exe_proprio = processos.iter().find(|p| p.pid == proprio).map(|p| p.exe.as_str());
    let mut nomes: Vec<String> = com_som
        .iter()
        .filter(|pid| **pid != 0 && raiz(**pid, processos) != raiz(proprio, processos))
        .filter_map(|pid| processos.iter().find(|p| p.pid == *pid).map(|p| p.exe.clone()))
        .filter(|exe| Some(exe.as_str()) != exe_proprio)
        .collect();
    nomes.sort();
    nomes.dedup();
    nomes
}

/// O que o sistema tem agora: os processos e quais deles têm som aberto na saída padrão.
#[derive(Clone, Debug, Default)]
pub struct Panorama {
    pub processos: Vec<Processo>,
    pub com_som: Vec<u32>,
}

impl Panorama {
    /// Precisa ser chamada de uma thread que usa o som do Windows (a do Modo Escuta).
    pub fn ler() -> Self {
        #[cfg(windows)]
        {
            Self { processos: so::processos(), com_som: so::com_som().unwrap_or_default() }
        }
        #[cfg(not(windows))]
        {
            Self::default()
        }
    }

    pub fn alvo(&self, exe: &str) -> Option<u32> {
        alvo(exe, &self.com_som, &self.processos)
    }

    pub fn apps(&self) -> Vec<String> {
        nomes_com_som(&self.com_som, &self.processos, std::process::id())
    }

    /// O primeiro processo do próprio Player (para ouvir só a música local).
    pub fn raiz_propria(&self) -> u32 {
        raiz(std::process::id(), &self.processos)
    }
}

/// A captura do som de um processo. Soltar o valor encerra a captura.
pub struct CapturaDeApp {
    parar: Arc<AtomicBool>,
    falhou: Arc<AtomicBool>,
    fio: Option<JoinHandle<()>>,
    pub pid: u32,
}

impl CapturaDeApp {
    /// Começa a ouvir o processo `pid` e os que ele abriu. As amostras (mono, a [`TAXA_DE_APP`]) vão para `anel`.
    pub fn abrir(anel: Arc<Anel>, pid: u32) -> Resultado<Self> {
        #[cfg(windows)]
        {
            let parar = Arc::new(AtomicBool::new(false));
            let falhou = Arc::new(AtomicBool::new(false));
            let (pronto, espera) = std::sync::mpsc::channel::<Result<(), String>>();
            let (p, f) = (parar.clone(), falhou.clone());
            let fio = std::thread::Builder::new()
                .name("escuta-app".into())
                .spawn(move || so::capturar(pid, &anel, &p, &f, &pronto))
                .map_err(|e| Erro::Audio(e.to_string()))?;
            match espera.recv_timeout(std::time::Duration::from_secs(3)) {
                Ok(Ok(())) => Ok(Self { parar, falhou, fio: Some(fio), pid }),
                Ok(Err(motivo)) => {
                    let _ = fio.join();
                    Err(Erro::Audio(motivo))
                }
                Err(_) => {
                    // A thread fica para trás e encerra sozinha quando o Windows responder.
                    parar.store(true, Ordering::Release);
                    Err(Erro::Audio("o Windows não respondeu ao pedido de ouvir o app".into()))
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (anel, pid);
            Err(Erro::Audio("ouvir um app só existe no Windows".into()))
        }
    }

    /// Verdadeiro se o sistema encerrou a captura (o app fechou, por exemplo).
    pub fn falhou(&self) -> bool {
        self.falhou.load(Ordering::Acquire)
    }
}

impl Drop for CapturaDeApp {
    fn drop(&mut self) {
        self.parar.store(true, Ordering::Release);
        if let Some(f) = self.fio.take() {
            let _ = f.join();
        }
    }
}

#[cfg(windows)]
mod so {
    use super::{Processo, TAXA_DE_APP};
    use crate::Anel;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::Sender;
    use std::time::Duration;
    use windows::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
    use windows::Win32::Media::Audio::{
        AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED, AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
        AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMFLAGS_LOOPBACK, AUDIOCLIENT_ACTIVATION_PARAMS,
        AUDIOCLIENT_ACTIVATION_PARAMS_0, AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
        AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS, ActivateAudioInterfaceAsync, AudioSessionStateExpired,
        IActivateAudioInterfaceAsyncOperation, IActivateAudioInterfaceCompletionHandler,
        IActivateAudioInterfaceCompletionHandler_Impl, IAudioCaptureClient, IAudioClient, IAudioSessionControl2,
        IAudioSessionManager2, IMMDeviceEnumerator, MMDeviceEnumerator,
        PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE, VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK, WAVEFORMATEX,
        eMultimedia, eRender,
    };
    use windows::Win32::System::Com::StructuredStorage::PROPVARIANT;
    use windows::Win32::System::Com::{
        BLOB, CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemAlloc,
    };
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{CreateEventW, WaitForSingleObject};
    use windows::Win32::System::Variant::VT_BLOB;
    use windows::core::{HRESULT, IUnknown, Interface, Ref, Result, implement};

    const CANAIS: u16 = 2;
    const BITS: u16 = 16;

    /// Liga o COM nesta thread (uma vez por thread; ligar de novo não faz mal).
    fn com() {
        thread_local!(static LIGADO: std::cell::Cell<bool> = const { std::cell::Cell::new(false) });
        LIGADO.with(|l| {
            if !l.replace(true) {
                // SAFETY: inicialização comum do COM; o resultado "já estava ligado" também serve.
                let _ = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
            }
        });
    }

    pub fn processos() -> Vec<Processo> {
        let mut lista = Vec::new();
        // SAFETY: a foto dos processos é um identificador do Windows, fechado no fim; a estrutura tem o tamanho
        // informado, como a função exige.
        unsafe {
            let Ok(foto) = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) else { return lista };
            let mut item = PROCESSENTRY32W { dwSize: size_of::<PROCESSENTRY32W>() as u32, ..Default::default() };
            let mut tem = Process32FirstW(foto, &mut item).is_ok();
            while tem {
                let fim = item.szExeFile.iter().position(|c| *c == 0).unwrap_or(item.szExeFile.len());
                lista.push(Processo {
                    pid: item.th32ProcessID,
                    pai: item.th32ParentProcessID,
                    exe: String::from_utf16_lossy(&item.szExeFile[..fim]).to_lowercase(),
                });
                tem = Process32NextW(foto, &mut item).is_ok();
            }
            let _ = CloseHandle(foto);
        }
        lista
    }

    /// Os processos com som aberto na saída padrão (tocando agora ou em pausa).
    pub fn com_som() -> Result<Vec<u32>> {
        com();
        // SAFETY: chamadas COM comuns; os objetos devolvidos são contados por referência e soltos no fim do escopo.
        unsafe {
            let enumerador: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
            let saida = enumerador.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
            let gerente: IAudioSessionManager2 = saida.Activate(CLSCTX_ALL, None)?;
            let sessoes = gerente.GetSessionEnumerator()?;
            let mut pids = Vec::new();
            for i in 0..sessoes.GetCount()? {
                let Ok(sessao) = sessoes.GetSession(i) else { continue };
                if sessao.GetState().is_ok_and(|e| e == AudioSessionStateExpired) {
                    continue;
                }
                if let Ok(pid) = sessao.cast::<IAudioSessionControl2>().and_then(|s| s.GetProcessId())
                    && pid != 0
                {
                    pids.push(pid);
                }
            }
            Ok(pids)
        }
    }

    #[implement(IActivateAudioInterfaceCompletionHandler)]
    struct AoAtivar(Sender<Result<IUnknown>>);

    impl IActivateAudioInterfaceCompletionHandler_Impl for AoAtivar_Impl {
        fn ActivateCompleted(&self, operacao: Ref<'_, IActivateAudioInterfaceAsyncOperation>) -> Result<()> {
            let resultado = operacao.ok().and_then(|o| {
                let mut codigo = HRESULT::default();
                let mut interface: Option<IUnknown> = None;
                // SAFETY: os dois ponteiros apontam para variáveis locais vivas durante a chamada.
                unsafe { o.GetActivateResult(&mut codigo, &mut interface)? };
                codigo.ok()?;
                interface.ok_or_else(|| windows::core::Error::from_hresult(HRESULT(-2147467259)))
            });
            let _ = self.0.send(resultado);
            Ok(())
        }
    }

    /// Uma captura aberta: o cliente de áudio e o leitor.
    struct Aberta {
        cliente: IAudioClient,
        leitor: IAudioCaptureClient,
    }

    /// Pede ao Windows o som do processo `pid` (com os filhos) e devolve a captura já iniciada.
    fn abrir(pid: u32, evento: HANDLE) -> Result<Aberta> {
        let parametros = AUDIOCLIENT_ACTIVATION_PARAMS {
            ActivationType: AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK,
            Anonymous: AUDIOCLIENT_ACTIVATION_PARAMS_0 {
                ProcessLoopbackParams: AUDIOCLIENT_PROCESS_LOOPBACK_PARAMS {
                    TargetProcessId: pid,
                    ProcessLoopbackMode: PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE,
                },
            },
        };
        let tamanho = size_of::<AUDIOCLIENT_ACTIVATION_PARAMS>();
        // O bloco de parâmetros vai em memória do alocador do COM: o Windows fica com ele e o libera quando
        // a captura acaba. Em memória do programa (pilha ou caixa do Rust), essa liberação corrompia a memória.
        // SAFETY: pede `tamanho` bytes ao COM e copia os parâmetros para lá; o ponteiro é conferido antes.
        let bloco_de_parametros = unsafe {
            let p = CoTaskMemAlloc(tamanho).cast::<AUDIOCLIENT_ACTIVATION_PARAMS>();
            if p.is_null() {
                return Err(windows::core::Error::from_hresult(HRESULT(0x8007_000E_u32 as i32)));
            }
            p.write(parametros);
            p
        };
        let mut variante = PROPVARIANT::default();
        // SAFETY: a variante começa zerada e recebe o bloco de bytes acima; a ativação termina antes de a
        // variante sair de escopo.
        let cliente: IAudioClient = unsafe {
            let dentro = &mut *variante.Anonymous.Anonymous;
            dentro.vt = VT_BLOB;
            dentro.Anonymous.blob = BLOB { cbSize: tamanho as u32, pBlobData: bloco_de_parametros.cast() };
            let (envio, espera) = std::sync::mpsc::channel();
            let ao_ativar: IActivateAudioInterfaceCompletionHandler = AoAtivar(envio).into();
            let _operacao = ActivateAudioInterfaceAsync(
                VIRTUAL_AUDIO_DEVICE_PROCESS_LOOPBACK,
                &IAudioClient::IID,
                Some(&raw const variante),
                &ao_ativar,
            )?;
            espera
                .recv_timeout(Duration::from_secs(2))
                // ERROR_TIMEOUT
                .map_err(|_| windows::core::Error::from_hresult(HRESULT(0x8007_05B4_u32 as i32)))??
                .cast()?
        };
        let bloco = CANAIS * BITS / 8;
        let formato = WAVEFORMATEX {
            wFormatTag: 1,
            nChannels: CANAIS,
            nSamplesPerSec: TAXA_DE_APP,
            nAvgBytesPerSec: TAXA_DE_APP * u32::from(bloco),
            nBlockAlign: bloco,
            wBitsPerSample: BITS,
            cbSize: 0,
        };
        // SAFETY: `formato` vive durante a chamada; o evento é um identificador válido criado por quem chamou.
        unsafe {
            cliente.Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK | AUDCLNT_STREAMFLAGS_EVENTCALLBACK | AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM,
                0,
                0,
                &raw const formato,
                None,
            )?;
            cliente.SetEventHandle(evento)?;
            let leitor: IAudioCaptureClient = cliente.GetService()?;
            cliente.Start()?;
            Ok(Aberta { cliente, leitor })
        }
    }

    /// O corpo da thread de captura: abre, avisa por `pronto` se deu certo e copia o som para o anel até
    /// `parar`.
    pub fn capturar(
        pid: u32,
        anel: &Anel,
        parar: &AtomicBool,
        falhou: &AtomicBool,
        pronto: &Sender<std::result::Result<(), String>>,
    ) {
        com();
        // SAFETY: cria um evento sem nome, de reinício automático; é fechado no fim desta função.
        let evento = match unsafe { CreateEventW(None, false, false, None) } {
            Ok(e) => e,
            Err(e) => {
                let _ = pronto.send(Err(e.message()));
                return;
            }
        };
        let aberta = match abrir(pid, evento) {
            Ok(a) => a,
            Err(e) => {
                let _ = pronto.send(Err(e.message()));
                // SAFETY: o evento foi criado acima e ninguém mais o usa.
                let _ = unsafe { CloseHandle(evento) };
                return;
            }
        };
        let _ = pronto.send(Ok(()));
        let leitor = &aberta.leitor;
        let canais = usize::from(CANAIS);
        while !parar.load(Ordering::Acquire) {
            // SAFETY: espera pelo evento criado acima, com limite de tempo (o som pode ficar em silêncio).
            let _acordou = unsafe { WaitForSingleObject(evento, 20) } == WAIT_OBJECT_0;
            loop {
                // SAFETY: o leitor é válido; os ponteiros de saída apontam para variáveis locais. O bloco
                // devolvido vale até `ReleaseBuffer`, e só é lido dentro desse intervalo, no tamanho informado.
                let lido = unsafe {
                    match leitor.GetNextPacketSize() {
                        Ok(0) => break,
                        Ok(_) => {}
                        Err(e) => {
                            log::warn!("captura do app interrompida: {e}");
                            falhou.store(true, Ordering::Release);
                            break;
                        }
                    }
                    let (mut dados, mut quadros, mut marcas) = (std::ptr::null_mut::<u8>(), 0u32, 0u32);
                    leitor.GetBuffer(&mut dados, &mut quadros, &mut marcas, None, None).map(|()| {
                        let silencio = marcas & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 || dados.is_null();
                        if silencio {
                            anel.escrever(std::iter::repeat_n(0.0, quadros as usize));
                        } else {
                            let amostras = std::slice::from_raw_parts(dados.cast::<i16>(), quadros as usize * canais);
                            anel.escrever(
                                amostras
                                    .chunks_exact(canais)
                                    .map(|q| q.iter().map(|s| f32::from(*s) / 32768.0).sum::<f32>() / canais as f32),
                            );
                        }
                        let _ = leitor.ReleaseBuffer(quadros);
                    })
                };
                if let Err(e) = lido {
                    log::warn!("captura do app interrompida: {e}");
                    falhou.store(true, Ordering::Release);
                    break;
                }
            }
            if falhou.load(Ordering::Acquire) {
                break;
            }
        }
        // SAFETY: encerra o cliente e fecha o evento, ambos criados nesta função.
        unsafe {
            let _ = aberta.cliente.Stop();
            drop(aberta);
            let _ = CloseHandle(evento);
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn p(pid: u32, pai: u32, exe: &str) -> Processo {
        Processo { pid, pai, exe: exe.into() }
    }

    fn sistema() -> Vec<Processo> {
        vec![
            p(4, 0, "system"),
            p(100, 4, "explorer.exe"),
            p(200, 100, "chrome.exe"),
            p(210, 200, "chrome.exe"),
            p(220, 210, "chrome.exe"),
            p(300, 100, "spotify.exe"),
            p(310, 300, "spotify.exe"),
            p(400, 100, "player.exe"),
        ]
    }

    #[test]
    fn sobe_ate_o_primeiro_processo_do_app() {
        let s = sistema();
        assert_eq!(raiz(220, &s), 200);
        assert_eq!(raiz(200, &s), 200);
        assert_eq!(raiz(310, &s), 300);
        assert_eq!(raiz(999, &s), 999, "processo que já fechou fica como está");
        // Pai e filho apontando um para o outro não prendem a busca.
        let volta = vec![p(1, 2, "a.exe"), p(2, 1, "a.exe")];
        assert!([1, 2].contains(&raiz(1, &volta)));
    }

    #[test]
    fn alvo_prefere_quem_tem_som_e_cai_para_o_app_aberto() {
        let s = sistema();
        assert_eq!(alvo("Chrome.exe", &[220, 310], &s), Some(200));
        assert_eq!(alvo("spotify.exe", &[220], &s), Some(300), "aberto, mas ainda sem som");
        assert_eq!(alvo("vlc.exe", &[220], &s), None);
    }

    #[test]
    fn lista_de_apps_sem_repetir_e_sem_o_proprio_player() {
        let s = sistema();
        assert_eq!(nomes_com_som(&[220, 210, 310, 400, 0, 999], &s, 400), ["chrome.exe", "spotify.exe"]);
        // Outra cópia do Player aberta (outro processo, mesmo executável) também fica de fora.
        let mut com_copia = s.clone();
        com_copia.push(p(500, 100, "player.exe"));
        assert_eq!(nomes_com_som(&[220, 500], &com_copia, 400), ["chrome.exe"]);
        assert!(nomes_com_som(&[], &s, 400).is_empty());
    }
}
