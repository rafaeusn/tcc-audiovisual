use nucleo::{Erro, Resultado};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use visual::{Destino, Motor, Quadro};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::core::Interface;

/// Maior lado de superfície aceito (protege contra tamanhos absurdos vindos da janela).
const LADO_MAXIMO: u32 = 16384;

pub struct Camada {
    dcomp: IDCompositionDevice,
    _alvo: IDCompositionTarget,
    _camada: IDCompositionVisual,
    device: wgpu::Device,
    queue: wgpu::Queue,
    superficie: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    motor: Motor,
    perdida: Arc<AtomicBool>,
    /// Nome da placa de vídeo em uso (para o registro e a tela Sobre).
    pub adaptador: String,
}

impl Camada {
    /// Cria a camada na janela `hwnd`, com `largura` x `altura` em pixels físicos.
    pub fn new(hwnd: isize, largura: u32, altura: u32) -> Resultado<Self> {
        Self::criar(hwnd, largura, altura, false)
    }

    /// Camada com fundo transparente: fora do retângulo do visual aparece o que está atrás da janela (a Ilha usa
    /// assim, com o visual num recorte da forma).
    pub fn new_transparente(hwnd: isize, largura: u32, altura: u32) -> Resultado<Self> {
        Self::criar(hwnd, largura, altura, true)
    }

    fn criar(hwnd: isize, largura: u32, altura: u32, transparente: bool) -> Resultado<Self> {
        let (dcomp, alvo, camada, instancia, superficie) = partes_dcomp(hwnd)?;
        let partes = partes_gpu(&instancia, superficie, largura, altura, transparente)?;
        montar(dcomp, alvo, camada, partes)
    }

    /// Prepara uma camada transparente sem travar quem chama: a parte demorada (abrir a placa de vídeo e compilar os
    /// shaders do visual) roda em outra thread. Pergunte a [`CamadaPendente::pronta`] a cada quadro.
    pub fn preparar_transparente(hwnd: isize, largura: u32, altura: u32) -> Resultado<CamadaPendente> {
        let (dcomp, alvo, camada, instancia, superficie) = partes_dcomp(hwnd)?;
        let (envia, recebe) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("camada-ilha".into())
            .spawn(move || {
                let _ = envia.send(partes_gpu(&instancia, superficie, largura, altura, true));
            })
            .map_err(|e| Erro::Gpu(format!("thread da camada: {e}")))?;
        Ok(CamadaPendente { com: Some((dcomp, alvo, camada)), recebe })
    }

    /// Verdadeiro se o driver derrubou o dispositivo (atualização de driver, por exemplo). Quem usa a camada
    /// deve soltá-la e criar outra.
    pub fn perdida(&self) -> bool {
        self.perdida.load(Ordering::Acquire)
    }

    /// Muda o tamanho da superfície. Verdadeiro se mudou: o conteúdo novo é indefinido até o próximo desenho.
    pub fn redimensionar(&mut self, largura: u32, altura: u32) -> bool {
        let (w, h) = (largura.clamp(1, LADO_MAXIMO), altura.clamp(1, LADO_MAXIMO));
        if (w, h) == (self.config.width, self.config.height) {
            return false;
        }
        self.config.width = w;
        self.config.height = h;
        self.superficie.configure(&self.device, &self.config);
        // SAFETY: mesma garantia do `Commit` em `montar`.
        if let Err(e) = unsafe { self.dcomp.Commit() } {
            log::warn!("não foi possível publicar o novo tamanho da camada: {e}");
        }
        true
    }

    /// Pinta o fundo e o visual dentro de `retangulo` (pixels físicos) com cantos de raio `raio`.
    pub fn desenhar(&mut self, fundo: [f32; 3], retangulo: (f32, f32, f32, f32), raio: f32, quadro: &Quadro) {
        self.desenhar_esticado(fundo, retangulo, None, raio, quadro);
    }

    /// Como [`desenhar`](Self::desenhar), mas desenha o visual no tamanho `interno` e o estica no retângulo (para um
    /// retângulo que anima de tamanho sem recriar nada a cada quadro).
    pub fn desenhar_esticado(
        &mut self,
        fundo: [f32; 3],
        retangulo: (f32, f32, f32, f32),
        interno: Option<(f32, f32)>,
        raio: f32,
        quadro: &Quadro,
    ) {
        let fundo = wgpu::Color { r: fundo[0] as f64, g: fundo[1] as f64, b: fundo[2] as f64, a: 1.0 };
        self.desenhar_com(fundo, retangulo, interno, raio, quadro);
    }

    /// Pinta só o visual dentro de `retangulo`, com o resto transparente (camada criada por `new_transparente`).
    pub fn desenhar_recorte(&mut self, retangulo: (f32, f32, f32, f32), raio: f32, quadro: &Quadro) {
        self.desenhar_com(wgpu::Color::TRANSPARENT, retangulo, None, raio, quadro);
    }

    fn desenhar_com(
        &mut self,
        fundo: wgpu::Color,
        retangulo: (f32, f32, f32, f32),
        interno: Option<(f32, f32)>,
        raio: f32,
        quadro: &Quadro,
    ) {
        if self.perdida() {
            return;
        }
        let textura = match self.superficie.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(t) | wgpu::CurrentSurfaceTexture::Suboptimal(t) => t,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.superficie.configure(&self.device, &self.config);
                return;
            }
            // Janela coberta ou minimizada, ou o sistema demorou: pula o quadro.
            _ => return,
        };
        let vista = textura.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let destino = Destino {
            vista: &vista,
            tamanho: (self.config.width, self.config.height),
            fundo,
            retangulo,
            raio,
            interno,
        };
        self.motor.desenhar(&self.device, &self.queue, &destino, quadro);
        self.queue.present(textura);
    }
}

/// Uma camada sendo preparada em outra thread (ver [`Camada::preparar_transparente`]).
pub struct CamadaPendente {
    com: Option<(IDCompositionDevice, IDCompositionTarget, IDCompositionVisual)>,
    recebe: std::sync::mpsc::Receiver<Resultado<Partes>>,
}

impl CamadaPendente {
    /// A camada, quando a outra thread termina (ou o erro dela). `None` enquanto ainda prepara.
    pub fn pronta(&mut self) -> Option<Resultado<Camada>> {
        let resultado = match self.recebe.try_recv() {
            Ok(r) => r,
            Err(std::sync::mpsc::TryRecvError::Empty) => return None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(Erro::Gpu("a preparação da camada parou".into())),
        };
        let (dcomp, alvo, camada) = self.com.take()?;
        Some(resultado.and_then(|partes| montar(dcomp, alvo, camada, partes)))
    }
}

/// O que a thread de preparo devolve: tudo do wgpu, pronto para ligar à janela.
struct Partes {
    superficie: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    motor: Motor,
    perdida: Arc<AtomicBool>,
    nome: String,
}

fn erro(o_que: &str, e: &dyn std::fmt::Display) -> Erro {
    Erro::Gpu(format!("{o_que}: {e}"))
}

/// A parte do DirectComposition (na thread da janela): a camada embaixo do GPUI e a superfície ligada a ela.
fn partes_dcomp(
    hwnd: isize,
) -> Resultado<(IDCompositionDevice, IDCompositionTarget, IDCompositionVisual, wgpu::Instance, wgpu::Surface<'static>)>
{
    if hwnd == 0 {
        return Err(Erro::Gpu("janela inválida".into()));
    }
    let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
    desc.backends = wgpu::Backends::DX12;
    let instancia = wgpu::Instance::new(desc);
    // SAFETY: chamadas COM do DirectComposition com um HWND válido, que o chamador recebeu do sistema de
    // janelas e que vive mais que esta camada (a camada é solta junto com a janela). Os objetos COM são
    // contados por referência e guardados na struct, então o ponteiro entregue ao wgpu continua válido
    // enquanto a superfície existir.
    unsafe {
        let dcomp: IDCompositionDevice =
            DCompositionCreateDevice2(None::<&windows::core::IUnknown>).map_err(|e| erro("DirectComposition", &e))?;
        // `false` = abaixo da camada do GPUI (que usa `true`).
        let alvo = dcomp.CreateTargetForHwnd(HWND(hwnd as _), false).map_err(|e| erro("alvo da camada", &e))?;
        let camada = dcomp.CreateVisual().map_err(|e| erro("camada", &e))?;
        alvo.SetRoot(&camada).map_err(|e| erro("raiz da camada", &e))?;
        let superficie = instancia
            .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CompositionVisual(camada.as_raw()))
            .map_err(|e| erro("superfície", &e))?;
        Ok((dcomp, alvo, camada, instancia, superficie))
    }
}

/// A parte demorada (pode rodar em outra thread): placa de vídeo, dispositivo, configuração e os shaders do visual.
fn partes_gpu(
    instancia: &wgpu::Instance,
    superficie: wgpu::Surface<'static>,
    largura: u32,
    altura: u32,
    transparente: bool,
) -> Resultado<Partes> {
    let adaptador = pollster::block_on(instancia.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&superficie),
        ..Default::default()
    }))
    .map_err(|e| erro("placa de vídeo", &e))?;
    let info = adaptador.get_info();
    let (device, queue) = pollster::block_on(adaptador.request_device(&wgpu::DeviceDescriptor::default()))
        .map_err(|e| erro("dispositivo", &e))?;

    // Erros de validação fora de um escopo viram registro, não queda do app.
    device.on_uncaptured_error(Arc::new(|e: wgpu::Error| log::error!("wgpu: {e}")));
    let perdida = Arc::new(AtomicBool::new(false));
    let sinal = perdida.clone();
    device.set_device_lost_callback(move |motivo, texto| {
        log::error!("placa de vídeo perdida ({motivo:?}): {texto}");
        sinal.store(true, Ordering::Release);
    });

    let caps = superficie.get_capabilities(&adaptador);
    let (w, h) = (largura.clamp(1, LADO_MAXIMO), altura.clamp(1, LADO_MAXIMO));
    let mut config = superficie
        .get_default_config(&adaptador, w, h)
        .ok_or_else(|| Erro::Gpu("superfície sem configuração".into()))?;
    // Formato sem sRGB: os shaders já entregam a cor pronta para a tela, como na prévia.
    if caps.formats.contains(&wgpu::TextureFormat::Bgra8Unorm) {
        config.format = wgpu::TextureFormat::Bgra8Unorm;
    }
    // O ritmo é o do GPUI: a camada não pode segurar a thread esperando o vsync dela.
    config.present_mode = [wgpu::PresentMode::Mailbox, wgpu::PresentMode::Immediate]
        .into_iter()
        .find(|m| caps.present_modes.contains(m))
        .unwrap_or(wgpu::PresentMode::Fifo);
    let alfa = if transparente { wgpu::CompositeAlphaMode::PreMultiplied } else { wgpu::CompositeAlphaMode::Opaque };
    if caps.alpha_modes.contains(&alfa) {
        config.alpha_mode = alfa;
    } else if transparente {
        return Err(Erro::Gpu("a placa não compõe superfície transparente".into()));
    }
    config.desired_maximum_frame_latency = 1;
    let motor = Motor::new(&device, config.format)?;
    let nome = format!("{} ({:?}, {:?})", info.name, info.backend, config.present_mode);
    Ok(Partes { superficie, device, queue, config, motor, perdida, nome })
}

/// Liga a superfície à camada (na thread da janela) e publica.
fn montar(
    dcomp: IDCompositionDevice,
    alvo: IDCompositionTarget,
    camada: IDCompositionVisual,
    partes: Partes,
) -> Resultado<Camada> {
    let Partes { superficie, device, queue, config, motor, perdida, nome } = partes;
    superficie.configure(&device, &config);
    // SAFETY: `dcomp` é o dispositivo criado em `partes_dcomp`; `Commit` só publica as mudanças pendentes.
    unsafe { dcomp.Commit() }.map_err(|e| erro("publicar a camada", &e))?;
    log::info!("visual: {nome}, {}x{}", config.width, config.height);
    Ok(Camada {
        dcomp,
        _alvo: alvo,
        _camada: camada,
        device,
        queue,
        superficie,
        config,
        motor,
        perdida,
        adaptador: nome,
    })
}
