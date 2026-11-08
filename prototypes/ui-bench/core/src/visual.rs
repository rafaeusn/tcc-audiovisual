//! O visual (anel espectral) desenhado com wgpu numa textura.
//!
//! O protótipo Slint entrega o `Device`/`Queue` do próprio Slint e mostra a textura direto na UI (zero cópia).
//! O protótipo GPUI não aceita texturas wgpu externas: ele cria um device próprio e lê os pixels de volta para a
//! CPU a cada quadro ([`Visual::read_rgba`]). Essa diferença faz parte da comparação.

use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct Params {
    pub resolution: [f32; 2],
    pub time: f32,
    pub bass: f32,
    pub mid: f32,
    pub high: f32,
    pub level: f32,
    pub intensity: f32,
    pub pulse: f32,
    pub phase: f32,
    pub _pad: [f32; 2],
}

pub const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

pub struct Visual {
    pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind: wgpu::BindGroup,
    texture: Option<wgpu::Texture>,
    readback: Option<(wgpu::Buffer, u32, u32, u32)>,
    format: wgpu::TextureFormat,
    pub size: (u32, u32),
}

impl Visual {
    pub fn new(device: &wgpu::Device) -> Self {
        Self::with_format(device, FORMAT)
    }

    /// O GPUI espera BGRA: desenhar direto em BGRA evita trocar os canais na CPU.
    pub fn with_format(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("visual"),
            source: wgpu::ShaderSource::Wgsl(include_str!("visual.wgsl").into()),
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("params"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("params"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("visual"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("visual"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                // Alfa pré-multiplicado: os cantos arredondados (modo camada) misturam com o fundo.
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { pipeline, uniforms, bind, texture: None, readback: None, format, size: (0, 0) }
    }

    /// Garante uma textura do tamanho pedido. Devolve true se criou uma nova (a UI precisa receber a nova imagem).
    pub fn ensure_size(&mut self, device: &wgpu::Device, w: u32, h: u32) -> bool {
        let (w, h) = (w.clamp(16, 4096), h.clamp(16, 4096));
        if self.texture.is_some() && self.size == (w, h) {
            return false;
        }
        self.texture = Some(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("visual"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }));
        self.size = (w, h);
        self.readback = None;
        true
    }

    pub fn texture(&self) -> Option<&wgpu::Texture> {
        self.texture.as_ref()
    }

    /// Modo camada: limpa `target` com a cor de fundo e desenha o visual só dentro de `rect` (pixels físicos),
    /// com cantos arredondados de raio `radius`. Sem textura intermediária: é o que vai para a tela.
    #[allow(clippy::too_many_arguments)]
    pub fn render_into(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target: &wgpu::TextureView,
        target_size: (u32, u32),
        clear: wgpu::Color,
        rect: (f32, f32, f32, f32),
        radius: f32,
        params: &Params,
    ) {
        let (x, y, w, h) = rect;
        let mut p = *params;
        p.resolution = [w.max(1.0), h.max(1.0)];
        p._pad[0] = radius;
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&p));
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("camada") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("camada"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(clear), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            // Recorte dentro do alvo (o painel pode estar parcialmente fora da janela durante uma animação).
            let (tw, th) = (target_size.0 as f32, target_size.1 as f32);
            let x0 = x.clamp(0.0, tw);
            let y0 = y.clamp(0.0, th);
            let x1 = (x + w).clamp(0.0, tw);
            let y1 = (y + h).clamp(0.0, th);
            if x1 - x0 >= 1.0 && y1 - y0 >= 1.0 {
                pass.set_viewport(x, y, w.max(1.0), h.max(1.0), 0.0, 1.0);
                pass.set_scissor_rect(x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32);
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, &self.bind, &[]);
                pass.draw(0..3, 0..1);
            }
        }
        queue.submit([enc.finish()]);
    }

    pub fn render(&self, device: &wgpu::Device, queue: &wgpu::Queue, params: &Params) {
        let Some(tex) = &self.texture else { return };
        let mut p = *params;
        p.resolution = [self.size.0 as f32, self.size.1 as f32];
        queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&p));
        let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("visual") });
        {
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("visual"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit([enc.finish()]);
    }

    /// Desenha e lê os pixels de volta para a CPU (RGBA, sem padding). Usado onde a UI não aceita textura wgpu.
    pub fn render_and_read(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, params: &Params, out: &mut Vec<u8>) {
        self.render(device, queue, params);
        let Some(tex) = &self.texture else { return };
        let (w, h) = self.size;
        let unpadded = w * 4;
        let padded = unpadded.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        if self.readback.as_ref().map(|r| (r.1, r.2)) != Some((w, h)) {
            let buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("readback"),
                size: (padded * h) as u64,
                usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                mapped_at_creation: false,
            });
            self.readback = Some((buf, w, h, padded));
        }
        let (buf, _, _, padded) = self.readback.as_ref().unwrap();
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("readback") });
        enc.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo { texture: tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            wgpu::TexelCopyBufferInfo {
                buffer: buf,
                layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(*padded), rows_per_image: Some(h) },
            },
            wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        );
        queue.submit([enc.finish()]);
        let slice = buf.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
        {
            let Ok(data) = slice.get_mapped_range() else { return };
            out.clear();
            out.reserve((unpadded * h) as usize);
            for row in 0..h as usize {
                let s = row * *padded as usize;
                out.extend_from_slice(&data[s..s + unpadded as usize]);
            }
        }
        buf.unmap();
    }
}

/// Device próprio, sem janela (para frameworks que não compartilham o deles).
pub fn headless_device() -> Result<(wgpu::Device, wgpu::Queue, String), String> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .map_err(|e| e.to_string())?;
    let info = adapter.get_info();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|e| e.to_string())?;
    Ok((device, queue, format!("{} ({:?})", info.name, info.backend)))
}
