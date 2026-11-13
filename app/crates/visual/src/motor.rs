//! O motor dos visuais: recebe o [`Retrato`] do som e desenha um quadro no destino.
//!
//! Spectral: simulação (512x512) → 262.144 partículas → bloom em 6 níveis → composição com rastro.
//! Fluxo e Barras: um passe com rastro.
//! Os três terminam no mesmo passe de saída (aberração, compressão do brilho, vinheta, granulação), que desenha
//! direto no destino, dentro de um retângulo de cantos arredondados.

use crate::parametros::{Limitador, Luz, QuadroSpectral, Spectral, por_quadro};
use analise::{BINS, Dirigidos, Retrato};
use nucleo::{Erro, Modelo, Resultado};

const FORMATO_CENA: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;
const FORMATO_SIM: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;
const LADO_SIM: u32 = 512;
const PARTICULAS: u32 = LADO_SIM * LADO_SIM;
const NIVEIS_BLOOM: usize = 6;
/// Maior lado de alvo aceito (limite comum de textura das GPUs).
const LADO_MAXIMO: u32 = 8192;
/// Pré-processamento do bloom do .toe: (preto, gama, pré-brilho).
const PRE_BLOOM: [f32; 4] = [0.01, 2.63, 1.02, 0.0];
const PESO_BLOOM: f32 = 0.735;

/// Os 8 vetores que todo passe de tela cheia recebe (ver `shaders/tela.wgsl`).
type Bloco = [[f32; 4]; 8];

const TELA: &str = include_str!("shaders/tela.wgsl");

/// O que desenhar neste quadro.
pub struct Quadro<'a> {
    pub retrato: &'a Retrato,
    pub modelo: Modelo,
    /// `None` = cores reativas (o diretor escolhe). `Some` = as duas cores do usuário.
    pub cores: Option<([f32; 3], [f32; 3])>,
    pub modo_seguro: bool,
    /// Tema monocromático: o visual sai em tons de cinza.
    pub cinza: bool,
    /// Segundos desde o quadro anterior.
    pub dt: f32,
}

/// Onde desenhar: uma textura (a camada por baixo da interface) e o retângulo do visual dentro dela.
pub struct Destino<'a> {
    pub vista: &'a wgpu::TextureView,
    pub tamanho: (u32, u32),
    pub fundo: wgpu::Color,
    /// (x, y, largura, altura) em pixels físicos.
    pub retangulo: (f32, f32, f32, f32),
    pub raio: f32,
}

struct Alvo {
    _textura: wgpu::Texture,
    vista: wgpu::TextureView,
    w: u32,
    h: u32,
}

impl Alvo {
    fn new(device: &wgpu::Device, nome: &str, w: u32, h: u32, formato: wgpu::TextureFormat) -> Self {
        let textura = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(nome),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: formato,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let vista = textura.create_view(&wgpu::TextureViewDescriptor::default());
        Self { _textura: textura, vista, w, h }
    }

    fn texel(&self) -> [f32; 2] {
        [1.0 / self.w as f32, 1.0 / self.h as f32]
    }
}

/// Um passe de tela cheia: o bloco de parâmetros e as texturas que ele lê.
struct Passe {
    buffer: wgpu::Buffer,
    grupo: wgpu::BindGroup,
}

/// Tudo o que depende do tamanho do visual. É recriado quando o retângulo muda de tamanho.
struct Alvos {
    w: u32,
    h: u32,
    /// Dois alvos alternados: o quadro anterior alimenta o rastro do próximo.
    ping: [Alvo; 2],
    pontos: Alvo,
    niveis: Vec<Alvo>,
    prefiltro: Passe,
    reduz: Vec<Passe>,
    amplia: Vec<Passe>,
    /// `composicao[i]` e `cena[i]` leem `ping[i]` e escrevem em `ping[1 - i]`; `saida[i]` lê `ping[i]`.
    composicao: [Passe; 2],
    cena: [Passe; 2],
    saida: [Passe; 2],
}

pub struct Motor {
    layout_tela: wgpu::BindGroupLayout,
    amostrador: wgpu::Sampler,
    vazio: wgpu::TextureView,
    t_sim: wgpu::RenderPipeline,
    t_pontos: wgpu::RenderPipeline,
    t_prefiltro: wgpu::RenderPipeline,
    t_reduz: wgpu::RenderPipeline,
    t_amplia: wgpu::RenderPipeline,
    t_composicao: wgpu::RenderPipeline,
    t_cena: wgpu::RenderPipeline,
    t_saida: wgpu::RenderPipeline,
    sim: Alvo,
    p_sim: Passe,
    b_pontos: wgpu::Buffer,
    g_pontos: wgpu::BindGroup,
    espectro: wgpu::Texture,
    v_espectro: wgpu::TextureView,
    alvos: Option<Alvos>,
    lado: usize,
    spectral: Spectral,
    limitador: Limitador,
    impulso_visto: f64,
    tempo_cena: f32,
    relogio: f32,
}

fn modulo(device: &wgpu::Device, nome: &str, fonte: String) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(nome),
        source: wgpu::ShaderSource::Wgsl(fonte.into()),
    })
}

fn uniforme(visibilidade: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: visibilidade,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn textura(binding: u32, visibilidade: wgpu::ShaderStages, filtravel: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: visibilidade,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: filtravel },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

struct Tubo<'a> {
    nome: &'a str,
    layout: &'a wgpu::PipelineLayout,
    modulo: &'a wgpu::ShaderModule,
    fragmento: &'a str,
    formato: wgpu::TextureFormat,
    mistura: Option<wgpu::BlendState>,
}

fn tubo(device: &wgpu::Device, t: Tubo) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(t.nome),
        layout: Some(t.layout),
        vertex: wgpu::VertexState {
            module: t.modulo,
            entry_point: Some("vs"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: t.modulo,
            entry_point: Some(t.fragmento),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: t.formato,
                blend: t.mistura,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

fn buffer_uniforme(device: &wgpu::Device, nome: &str, bytes: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(nome),
        size: bytes,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

fn criar_passe(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    amostrador: &wgpu::Sampler,
    vazio: &wgpu::TextureView,
    nome: &str,
    texturas: [Option<&wgpu::TextureView>; 3],
) -> Passe {
    let buffer = buffer_uniforme(device, nome, std::mem::size_of::<Bloco>() as u64);
    // Passe que usa menos de 3 texturas recebe uma textura vazia de 1x1 no lugar.
    let t = texturas.map(|t| t.unwrap_or(vazio));
    let grupo = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(nome),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(amostrador) },
            wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(t[0]) },
            wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(t[1]) },
            wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(t[2]) },
        ],
    });
    Passe { buffer, grupo }
}

/// Mistura que soma (bloom) e mistura que fica com o mais claro (partículas).
const SOMA: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};
const MAIS_CLARO: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Max,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::One,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Max,
    },
};

impl Motor {
    /// Cria os shaders e as texturas fixas. `formato_destino` é o formato da textura que vai para a tela.
    /// Um erro aqui (shader recusado pelo driver, por exemplo) volta como [`Erro::Gpu`] em vez de derrubar o app.
    pub fn new(device: &wgpu::Device, formato_destino: wgpu::TextureFormat) -> Resultado<Self> {
        let escopo = device.push_error_scope(wgpu::ErrorFilter::Validation);
        let motor = Self::criar(device, formato_destino);
        match pollster::block_on(escopo.pop()) {
            Some(e) => Err(Erro::Gpu(format!("não foi possível preparar os visuais: {e}"))),
            None => Ok(motor),
        }
    }

    fn criar(device: &wgpu::Device, formato_destino: wgpu::TextureFormat) -> Self {
        let frag = wgpu::ShaderStages::FRAGMENT;
        let layout_tela = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("tela"),
            entries: &[
                uniforme(frag),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: frag,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                textura(2, frag, true),
                textura(3, frag, true),
                textura(4, frag, true),
            ],
        });
        let layout_pontos = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pontos"),
            entries: &[uniforme(wgpu::ShaderStages::VERTEX_FRAGMENT), textura(1, wgpu::ShaderStages::VERTEX, false)],
        });
        let pl_tela = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("tela"),
            bind_group_layouts: &[Some(&layout_tela)],
            immediate_size: 0,
        });
        let pl_pontos = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("pontos"),
            bind_group_layouts: &[Some(&layout_pontos)],
            immediate_size: 0,
        });

        let m_sim = modulo(device, "sim", format!("{TELA}\n{}", include_str!("shaders/sim.wgsl")));
        let m_bloom = modulo(device, "bloom", format!("{TELA}\n{}", include_str!("shaders/bloom.wgsl")));
        let m_cena = modulo(device, "cena", format!("{TELA}\n{}", include_str!("shaders/cena.wgsl")));
        let m_saida = modulo(device, "saida", format!("{TELA}\n{}", include_str!("shaders/saida.wgsl")));
        let m_pontos = modulo(device, "pontos", include_str!("shaders/pontos.wgsl").to_string());

        let tela = |nome, modulo, fragmento, formato, mistura| {
            tubo(device, Tubo { nome, layout: &pl_tela, modulo, fragmento, formato, mistura })
        };
        let t_sim = tela("sim", &m_sim, "fs_sim", FORMATO_SIM, None);
        let t_prefiltro = tela("prefiltro", &m_bloom, "fs_prefiltro", FORMATO_CENA, None);
        let t_reduz = tela("reduz", &m_bloom, "fs_reduz", FORMATO_CENA, None);
        let t_amplia = tela("amplia", &m_bloom, "fs_amplia", FORMATO_CENA, Some(SOMA));
        let t_composicao = tela("composicao", &m_bloom, "fs_composicao", FORMATO_CENA, None);
        let t_cena = tela("cena", &m_cena, "fs_cena", FORMATO_CENA, None);
        // Alfa pré-multiplicado: os cantos arredondados misturam com o fundo já pintado.
        let t_saida =
            tela("saida", &m_saida, "fs_saida", formato_destino, Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING));
        let t_pontos = tubo(
            device,
            Tubo {
                nome: "pontos",
                layout: &pl_pontos,
                modulo: &m_pontos,
                fragmento: "fs",
                formato: FORMATO_CENA,
                mistura: Some(MAIS_CLARO),
            },
        );

        let amostrador = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("linear"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let vazio = Alvo::new(device, "vazio", 1, 1, FORMATO_CENA).vista;
        let sim = Alvo::new(device, "sim", LADO_SIM, LADO_SIM, FORMATO_SIM);
        let espectro = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("espectro"),
            size: wgpu::Extent3d { width: BINS as u32, height: 1, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let v_espectro = espectro.create_view(&wgpu::TextureViewDescriptor::default());

        let b_pontos = buffer_uniforme(device, "pontos", std::mem::size_of::<[f32; 40]>() as u64);
        let g_pontos = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("pontos"),
            layout: &layout_pontos,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: b_pontos.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&sim.vista) },
            ],
        });

        let p_sim = criar_passe(device, &layout_tela, &amostrador, &vazio, "sim", [None, None, None]);
        Self {
            layout_tela,
            amostrador,
            vazio,
            t_sim,
            t_pontos,
            t_prefiltro,
            t_reduz,
            t_amplia,
            t_composicao,
            t_cena,
            t_saida,
            sim,
            p_sim,
            b_pontos,
            g_pontos,
            espectro,
            v_espectro,
            alvos: None,
            lado: 0,
            spectral: Spectral::default(),
            limitador: Limitador::default(),
            impulso_visto: 0.0,
            tempo_cena: 0.0,
            relogio: 0.0,
        }
    }

    fn passe(&self, device: &wgpu::Device, nome: &str, texturas: [Option<&wgpu::TextureView>; 3]) -> Passe {
        criar_passe(device, &self.layout_tela, &self.amostrador, &self.vazio, nome, texturas)
    }

    fn criar_alvos(&self, device: &wgpu::Device, w: u32, h: u32) -> Alvos {
        let cena = |nome| Alvo::new(device, nome, w, h, FORMATO_CENA);
        let ping = [cena("ping 0"), cena("ping 1")];
        let pontos = cena("pontos");
        let mut niveis = Vec::with_capacity(NIVEIS_BLOOM);
        let (mut mw, mut mh) = (w, h);
        for _ in 0..NIVEIS_BLOOM {
            mw = (mw / 2).max(1);
            mh = (mh / 2).max(1);
            niveis.push(Alvo::new(device, "bloom", mw, mh, FORMATO_CENA));
        }
        let prefiltro = self.passe(device, "prefiltro", [Some(&pontos.vista), None, None]);
        let reduz =
            (1..NIVEIS_BLOOM).map(|i| self.passe(device, "reduz", [Some(&niveis[i - 1].vista), None, None])).collect();
        let amplia =
            (1..NIVEIS_BLOOM).map(|i| self.passe(device, "amplia", [Some(&niveis[i].vista), None, None])).collect();
        let composicao = [0, 1].map(|i| {
            self.passe(device, "composicao", [Some(&pontos.vista), Some(&niveis[0].vista), Some(&ping[i].vista)])
        });
        let cena = [0, 1].map(|i| self.passe(device, "cena", [Some(&ping[i].vista), Some(&self.v_espectro), None]));
        let saida = [0, 1].map(|i| self.passe(device, "saida", [Some(&ping[i].vista), None, None]));
        Alvos { w, h, ping, pontos, niveis, prefiltro, reduz, amplia, composicao, cena, saida }
    }

    /// Desenha um quadro. Nunca falha: com um retângulo vazio só pinta o fundo.
    pub fn desenhar(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, destino: &Destino, quadro: &Quadro) {
        let dt = if quadro.dt.is_finite() { quadro.dt.clamp(0.0, 0.1) } else { 0.0 };
        let (x, y, lw, lh) = destino.retangulo;
        let visivel = [x, y, lw, lh].iter().all(|v| v.is_finite()) && lw >= 2.0 && lh >= 2.0;
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("visual") });
        let limpar = wgpu::LoadOp::Clear(destino.fundo);
        if !visivel {
            self.rodar(&mut enc, destino.vista, limpar, None);
            queue.submit([enc.finish()]);
            return;
        }
        let (w, h) = ((lw.round() as u32).clamp(2, LADO_MAXIMO), (lh.round() as u32).clamp(2, LADO_MAXIMO));
        if self.alvos.as_ref().is_none_or(|a| (a.w, a.h) != (w, h)) {
            self.alvos = Some(self.criar_alvos(device, w, h));
        }

        let r = quadro.retrato;
        let p = Dirigidos::para(r.energia);
        // Cores: reativas (o drop empurra para o vermelho, menos no modo seguro) ou as duas do usuário.
        let (cor_a, cor_b) =
            quadro.cores.unwrap_or_else(|| analise::cores(r.energia, if quadro.modo_seguro { 0.0 } else { r.drop }));
        let bruto = Luz {
            pulso: r.pulso,
            drop: r.drop,
            batida: r.bandas.beat,
            exposicao: p.exposure,
            bloom: p.bloom,
            cor_a,
            cor_b,
            movimento: 1.0,
        };
        let luz = self.limitador.aplicar(quadro.modo_seguro, dt, bruto);
        let impulso = (r.impulso_total - self.impulso_visto).clamp(0.0, 4.0) as f32;
        self.impulso_visto = r.impulso_total;
        self.relogio = (self.relogio + dt) % 3600.0;

        let escrever = |passe: &Passe, bloco: Bloco| queue.write_buffer(&passe.buffer, 0, bytemuck::cast_slice(&bloco));
        let zero = [0.0f32; 4];
        let Some(a) = self.alvos.as_ref() else { return };
        let (le, escreve) = (self.lado, 1 - self.lado);
        let (granulacao, vinheta);

        match quadro.modelo {
            Modelo::Spectral => {
                let q = self.spectral.quadro(dt, r, impulso, &p, &luz, w as f32, h as f32);
                escrever(
                    &self.p_sim,
                    [[q.offset, q.deriva, q.distorcao, 1.0], zero, zero, zero, zero, zero, zero, zero],
                );
                queue.write_buffer(
                    &self.b_pontos,
                    0,
                    bytemuck::cast_slice(&bloco_pontos(&q, &luz, w as f32, h as f32)),
                );
                let t = a.pontos.texel();
                escrever(&a.prefiltro, [[t[0], t[1], 0.0, 0.0], PRE_BLOOM, zero, zero, zero, zero, zero, zero]);
                for i in 1..NIVEIS_BLOOM {
                    let t = a.niveis[i - 1].texel();
                    escrever(&a.reduz[i - 1], [[t[0], t[1], 0.0, 0.0], PRE_BLOOM, zero, zero, zero, zero, zero, zero]);
                    let t = a.niveis[i].texel();
                    escrever(
                        &a.amplia[i - 1],
                        [[t[0], t[1], PESO_BLOOM, 0.0], PRE_BLOOM, zero, zero, zero, zero, zero, zero],
                    );
                }
                escrever(
                    &a.composicao[le],
                    [[0.0, 0.0, q.forca_bloom, q.rastro], PRE_BLOOM, zero, zero, zero, zero, zero, zero],
                );

                let preto = wgpu::LoadOp::Clear(wgpu::Color::BLACK);
                self.rodar(&mut enc, &self.sim.vista, preto, Some((&self.t_sim, &self.p_sim.grupo, 3, 1)));
                self.rodar(&mut enc, &a.pontos.vista, preto, Some((&self.t_pontos, &self.g_pontos, 6, PARTICULAS)));
                self.rodar(&mut enc, &a.niveis[0].vista, preto, Some((&self.t_prefiltro, &a.prefiltro.grupo, 3, 1)));
                for i in 1..NIVEIS_BLOOM {
                    self.rodar(&mut enc, &a.niveis[i].vista, preto, Some((&self.t_reduz, &a.reduz[i - 1].grupo, 3, 1)));
                }
                // Na volta, cada nível soma no nível maior (que já tem o próprio conteúdo).
                for i in (1..NIVEIS_BLOOM).rev() {
                    self.rodar(
                        &mut enc,
                        &a.niveis[i - 1].vista,
                        wgpu::LoadOp::Load,
                        Some((&self.t_amplia, &a.amplia[i - 1].grupo, 3, 1)),
                    );
                }
                self.rodar(
                    &mut enc,
                    &a.ping[escreve].vista,
                    preto,
                    Some((&self.t_composicao, &a.composicao[le].grupo, 3, 1)),
                );
                (granulacao, vinheta) = (0.03, 0.3);
            }
            Modelo::Fluxo | Modelo::Barras => {
                let b = &r.bandas;
                // (rastro, expansão, rotação) de cada visual, como na prévia.
                let (modo, rastro, expansao, rotacao) =
                    if quadro.modelo == Modelo::Fluxo { (1.0, 0.35f32, 0.3, 0.0) } else { (2.0, 0.8f32, -1.2, 0.0) };
                let k = por_quadro(dt);
                self.tempo_cena = (self.tempo_cena + dt * p.speed * (0.6 + b.level * 1.2 + luz.batida * 0.6)) % 3600.0;
                queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.espectro,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &r.espectro,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(BINS as u32),
                        rows_per_image: Some(1),
                    },
                    wgpu::Extent3d { width: BINS as u32, height: 1, depth_or_array_layers: 1 },
                );
                escrever(
                    &a.cena[le],
                    [
                        [w as f32, h as f32, self.tempo_cena, modo],
                        [b.bass, b.mid, b.high, b.level],
                        [luz.batida, p.intensity, p.warp, rastro.powf(k)],
                        [expansao * k, rotacao * k, 0.0, 0.0],
                        [luz.cor_a[0], luz.cor_a[1], luz.cor_a[2], 1.0],
                        [luz.cor_b[0], luz.cor_b[1], luz.cor_b[2], 1.0],
                        zero,
                        zero,
                    ],
                );
                self.rodar(
                    &mut enc,
                    &a.ping[escreve].vista,
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    Some((&self.t_cena, &a.cena[le].grupo, 3, 1)),
                );
                (granulacao, vinheta) = (0.04, 0.5);
            }
        }

        // No modo seguro a aberração e a granulação (que cintilam) caem pela metade.
        let calma = if quadro.modo_seguro { 0.5 } else { 1.0 };
        escrever(
            &a.saida[escreve],
            [
                [lw, lh, destino.raio.max(0.0), self.relogio],
                [luz.batida, p.aberr * calma, granulacao * calma, vinheta],
                [luz.exposicao, if quadro.cinza { 1.0 } else { 0.0 }, 0.0, 0.0],
                zero,
                zero,
                zero,
                zero,
                zero,
            ],
        );
        {
            let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("saida"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: destino.vista,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: limpar, store: wgpu::StoreOp::Store },
                })],
                ..Default::default()
            });
            // Recorte dentro do destino: o retângulo pode ficar parcialmente fora durante um redimensionamento.
            let (tw, th) = (destino.tamanho.0 as f32, destino.tamanho.1 as f32);
            let (x0, y0) = (x.clamp(0.0, tw), y.clamp(0.0, th));
            let (x1, y1) = ((x + lw).clamp(0.0, tw), (y + lh).clamp(0.0, th));
            if x1 - x0 >= 1.0 && y1 - y0 >= 1.0 && x >= -16384.0 && y >= -16384.0 {
                passe.set_viewport(x, y, lw, lh, 0.0, 1.0);
                passe.set_scissor_rect(x0 as u32, y0 as u32, (x1 - x0) as u32, (y1 - y0) as u32);
                passe.set_pipeline(&self.t_saida);
                passe.set_bind_group(0, &a.saida[escreve].grupo, &[]);
                passe.draw(0..3, 0..1);
            }
        }
        queue.submit([enc.finish()]);
        self.lado = escreve;
    }

    /// Um passe simples no alvo inteiro. `desenho` = (pipeline, grupo, vértices, instâncias); `None` só limpa.
    fn rodar(
        &self,
        enc: &mut wgpu::CommandEncoder,
        vista: &wgpu::TextureView,
        carga: wgpu::LoadOp<wgpu::Color>,
        desenho: Option<(&wgpu::RenderPipeline, &wgpu::BindGroup, u32, u32)>,
    ) {
        let mut passe = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: vista,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: carga, store: wgpu::StoreOp::Store },
            })],
            ..Default::default()
        });
        if let Some((pipeline, grupo, vertices, instancias)) = desenho {
            passe.set_pipeline(pipeline);
            passe.set_bind_group(0, grupo, &[]);
            passe.draw(0..vertices, 0..instancias);
        }
    }
}

/// O bloco de parâmetros de `shaders/pontos.wgsl`, na mesma ordem da struct de lá.
fn bloco_pontos(q: &QuadroSpectral, luz: &Luz, w: f32, h: f32) -> [f32; 40] {
    let mut b = [0.0f32; 40];
    b[..16].copy_from_slice(&q.mvp);
    b[16..20].copy_from_slice(&[q.ancora[0], q.ancora[1], q.desloc[0], q.desloc[1]]);
    b[20..24].copy_from_slice(&[q.escala, q.explosao, q.tremor, q.tempo]);
    b[24..28].copy_from_slice(&[q.fluxo, q.giro, q.tamanho_px, 0.0]);
    b[28..32].copy_from_slice(&[w, h, q.brilho, 1.0]);
    b[32..36].copy_from_slice(&[luz.cor_a[0], luz.cor_a[1], luz.cor_a[2], 1.0]);
    b[36..40].copy_from_slice(&[luz.cor_b[0], luz.cor_b[1], luz.cor_b[2], 1.0]);
    b
}
