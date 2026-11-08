//! Visual sem cópia por baixo da UI do GPUI (Windows).
//!
//! O GPUI desenha a interface numa camada do DirectComposition marcada como "de cima" (`topmost = true`) e limpa
//! com transparente onde não há nada. O Windows aceita uma segunda camada na mesma janela (`topmost = false`):
//! criamos essa camada, entregamos ao wgpu (backend DX12) e desenhamos nela o fundo e o visual. Quem junta as duas é
//! o compositor do sistema, na GPU. Nenhum pixel passa pela CPU.
//!
//! Analogia: o GPUI é uma folha de acetato com a interface desenhada; o visual é a folha de papel embaixo.

use proto_core::visual::{Params, Visual};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice2, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::core::Interface;

pub struct Underlay {
    dcomp: IDCompositionDevice,
    _target: IDCompositionTarget,
    _layer: IDCompositionVisual,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    visual: Visual,
    pub adapter: String,
}

impl Underlay {
    pub fn new(hwnd: isize, width: u32, height: u32) -> Result<Self, String> {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::DX12;
        let instance = wgpu::Instance::new(desc);
        let e = |m: &str, err: &dyn std::fmt::Display| format!("{m}: {err}");
        unsafe {
            let dcomp: IDCompositionDevice =
                DCompositionCreateDevice2(None::<&windows::core::IUnknown>).map_err(|x| e("DComp", &x))?;
            // `false` = abaixo da camada do GPUI (que usa `true`).
            let target = dcomp.CreateTargetForHwnd(HWND(hwnd as _), false).map_err(|x| e("alvo", &x))?;
            let layer = dcomp.CreateVisual().map_err(|x| e("visual", &x))?;
            target.SetRoot(&layer).map_err(|x| e("raiz", &x))?;
            let surface = instance
                .create_surface_unsafe(wgpu::SurfaceTargetUnsafe::CompositionVisual(layer.as_raw()))
                .map_err(|x| e("superfície", &x))?;
            let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                ..Default::default()
            }))
            .map_err(|x| e("adaptador", &x))?;
            let info = adapter.get_info();
            let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
                .map_err(|x| e("device", &x))?;
            let caps = surface.get_capabilities(&adapter);
            let mut config =
                surface.get_default_config(&adapter, width.max(1), height.max(1)).ok_or("superfície sem configuração")?;
            // Formato sem sRGB, igual ao resto (o shader já aplica a gama).
            if let Some(f) = caps.formats.iter().find(|f| **f == wgpu::TextureFormat::Bgra8Unorm) {
                config.format = *f;
            }
            // O ritmo é o do GPUI: a camada não pode bloquear a thread esperando o vsync dela.
            config.present_mode = [wgpu::PresentMode::Mailbox, wgpu::PresentMode::Immediate]
                .into_iter()
                .find(|m| caps.present_modes.contains(m))
                .unwrap_or(wgpu::PresentMode::Fifo);
            if caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
                config.alpha_mode = wgpu::CompositeAlphaMode::Opaque;
            }
            config.desired_maximum_frame_latency = 1;
            surface.configure(&device, &config);
            dcomp.Commit().map_err(|x| e("commit", &x))?;
            let visual = Visual::with_format(&device, config.format);
            let adapter_name = format!("{} ({:?}, {:?})", info.name, info.backend, config.present_mode);
            Ok(Self {
                dcomp,
                _target: target,
                _layer: layer,
                device,
                queue,
                surface,
                config,
                visual,
                adapter: adapter_name,
            })
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let (w, h) = (width.max(1), height.max(1));
        if (w, h) != (self.config.width, self.config.height) {
            self.config.width = w;
            self.config.height = h;
            self.surface.configure(&self.device, &self.config);
            unsafe {
                let _ = self.dcomp.Commit();
            }
        }
    }

    /// Fundo + visual dentro de `rect` (pixels físicos) com cantos de raio `radius`.
    pub fn render(&mut self, bg: [f32; 4], rect: (f32, f32, f32, f32), radius: f32, params: &Params) {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(f) | wgpu::CurrentSurfaceTexture::Suboptimal(f) => f,
            wgpu::CurrentSurfaceTexture::Outdated | wgpu::CurrentSurfaceTexture::Lost => {
                self.surface.configure(&self.device, &self.config);
                return;
            }
            _ => return,
        };
        let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let clear = wgpu::Color { r: bg[0] as f64, g: bg[1] as f64, b: bg[2] as f64, a: 1.0 };
        self.visual.render_into(
            &self.device,
            &self.queue,
            &view,
            (self.config.width, self.config.height),
            clear,
            rect,
            radius,
            params,
        );
        self.queue.present(frame);
    }
}
