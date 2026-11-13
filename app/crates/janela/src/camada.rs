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
        let erro = |o_que: &str, e: &dyn std::fmt::Display| Erro::Gpu(format!("{o_que}: {e}"));
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
        let (dcomp, alvo, camada, superficie) = unsafe {
            let dcomp: IDCompositionDevice = DCompositionCreateDevice2(None::<&windows::core::IUnknown>)
                .map_err(|e| erro("DirectComposition", &e))?;
            // `false` = abaixo da camada do GPUI (que usa `true`).
            let alvo = dcomp.CreateTargetForHwnd(HWND(hwnd as _), false).map_err(|e| erro("alvo da camada", &e))?;
            let camada = dcomp.CreateVisual().map_err(|e| erro("camada", &e))?;
            alvo.SetRoot(&camada).map_err(|e| erro("raiz da camada", &e))?;
            let superficie = instancia
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CompositionVisual(camada.as_raw()))
                .map_err(|e| erro("superfície", &e))?;
            (dcomp, alvo, camada, superficie)
        };

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
        if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
            config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
        }
        config.desired_maximum_frame_latency = 1;
        superficie.configure(&device, &config);
        // SAFETY: `dcomp` é o dispositivo criado acima; `Commit` só publica as mudanças pendentes.
        unsafe { dcomp.Commit() }.map_err(|e| erro("publicar a camada", &e))?;

        let motor = Motor::new(&device, config.format)?;
        let nome = format!("{} ({:?}, {:?})", info.name, info.backend, config.present_mode);
        log::info!("visual: {nome}, {w}x{h}");
        Ok(Self {
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

    /// Verdadeiro se o driver derrubou o dispositivo (atualização de driver, por exemplo). Quem usa a camada
    /// deve soltá-la e criar outra.
    pub fn perdida(&self) -> bool {
        self.perdida.load(Ordering::Acquire)
    }

    pub fn redimensionar(&mut self, largura: u32, altura: u32) {
        let (w, h) = (largura.clamp(1, LADO_MAXIMO), altura.clamp(1, LADO_MAXIMO));
        if (w, h) != (self.config.width, self.config.height) {
            self.config.width = w;
            self.config.height = h;
            self.superficie.configure(&self.device, &self.config);
            // SAFETY: mesma garantia do `Commit` em `new`.
            if let Err(e) = unsafe { self.dcomp.Commit() } {
                log::warn!("não foi possível publicar o novo tamanho da camada: {e}");
            }
        }
    }

    /// Pinta o fundo e o visual dentro de `retangulo` (pixels físicos) com cantos de raio `raio`.
    pub fn desenhar(&mut self, fundo: [f32; 3], retangulo: (f32, f32, f32, f32), raio: f32, quadro: &Quadro) {
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
            fundo: wgpu::Color { r: fundo[0] as f64, g: fundo[1] as f64, b: fundo[2] as f64, a: 1.0 },
            retangulo,
            raio,
        };
        self.motor.desenhar(&self.device, &self.queue, &destino, quadro);
        self.queue.present(textura);
    }
}
