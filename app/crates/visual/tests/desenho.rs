//! Desenha os três visuais sem janela, lê os pixels de volta e confere que saiu imagem.
//!
//! Também salva um PNG por visual em `target/visuais/`, para comparar a olho com a prévia.
//! Em máquina sem GPU (CI) o teste termina sem falhar.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use analise::{Motor as Analise, PASSO, Retrato};
use nucleo::Modelo;
use visual::{Destino, Motor, Quadro};

const W: u32 = 960;
const H: u32 = 540;
const FORMATO: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

fn dispositivo() -> Option<(wgpu::Device, wgpu::Queue)> {
    let instancia = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adaptador = pollster::block_on(instancia.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        ..Default::default()
    }))
    .ok()?;
    pollster::block_on(adaptador.request_device(&wgpu::DeviceDescriptor::default())).ok()
}

/// Música sintética: bumbo a 126 BPM, baixo, chimbal e um acorde, por alguns segundos.
fn retratos(segundos: f32, volume: f32) -> Vec<Retrato> {
    let taxa = 48000.0f32;
    let n = (taxa * segundos) as usize;
    let periodo = 60.0 / 126.0;
    let mut semente = 99u32;
    let sinal: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / taxa;
            let fase = t % periodo;
            let tau = std::f32::consts::TAU;
            let bumbo = (-fase * 16.0).exp() * (tau * (55.0 + 60.0 * (-fase * 30.0).exp()) * t).sin();
            let baixo = 0.35 * (tau * 82.0 * t).sin() * (1.0 - (-fase * 8.0).exp());
            semente = semente.wrapping_mul(1664525).wrapping_add(1013904223);
            let ruido = (semente >> 8) as f32 / 8388608.0 - 1.0;
            let chimbal = 0.25 * ruido * (-((t + periodo / 2.0) % (periodo / 2.0)) * 60.0).exp();
            let acorde = 0.12 * ((tau * 440.0 * t).sin() + (tau * 554.4 * t).sin() + (tau * 659.3 * t).sin());
            volume * (0.7 * bumbo + baixo + chimbal + acorde)
        })
        .collect();
    let mut analise = Analise::new(taxa);
    let por_passo = (taxa * PASSO) as usize;
    let mut saida = Vec::new();
    let mut fim = analise::FFT_PRINCIPAL;
    while fim <= sinal.len() {
        saida.push(analise.passo(PASSO, &sinal[fim - analise::FFT_PRINCIPAL..fim]));
        fim += por_passo;
    }
    saida
}

fn ler(device: &wgpu::Device, queue: &wgpu::Queue, textura: &wgpu::Texture) -> Vec<u8> {
    let por_linha = W * 4;
    let alinhado = por_linha.div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("leitura"),
        size: (alinhado * H) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("leitura") });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: textura,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(alinhado), rows_per_image: Some(H) },
        },
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    queue.submit([enc.finish()]);
    let fatia = buffer.slice(..);
    fatia.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::Wait { submission_index: None, timeout: None });
    let dados = fatia.get_mapped_range().expect("leitura mapeada");
    let mut saida = Vec::with_capacity((por_linha * H) as usize);
    for linha in 0..H as usize {
        let ini = linha * alinhado as usize;
        saida.extend_from_slice(&dados[ini..ini + por_linha as usize]);
    }
    saida
}

struct Resultado {
    pixels: Vec<u8>,
    /// Luminância média (0–255) dentro do retângulo do visual.
    media: f32,
    /// Fração de pixels acesos (qualquer canal acima de 24).
    acesos: f32,
}

fn desenhar(modelo: Modelo, retratos: &[Retrato], seguro: bool, nome: &str) -> Option<Resultado> {
    let (device, queue) = dispositivo()?;
    let mut motor = Motor::new(&device, FORMATO).expect("shaders válidos");
    let textura = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("destino"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMATO,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let vista = textura.create_view(&wgpu::TextureViewDescriptor::default());
    let margem = 20.0;
    for r in retratos {
        motor.desenhar(
            &device,
            &queue,
            &Destino {
                vista: &vista,
                tamanho: (W, H),
                fundo: wgpu::Color { r: 0.0, g: 0.0, b: 0.2, a: 1.0 },
                retangulo: (margem, margem, W as f32 - 2.0 * margem, H as f32 - 2.0 * margem),
                raio: 28.0,
            },
            &Quadro { retrato: r, modelo, cores: None, modo_seguro: seguro, cinza: false, dt: PASSO },
        );
    }
    let pixels = ler(&device, &queue, &textura);

    let pasta = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/visuais");
    std::fs::create_dir_all(&pasta).expect("pasta de saída");
    image::save_buffer(pasta.join(format!("{nome}.png")), &pixels, W, H, image::ColorType::Rgba8).expect("png");

    let (mut soma, mut acesos, mut total) = (0.0f64, 0u32, 0u32);
    for y in 40..H - 40 {
        for x in 40..W - 40 {
            let i = ((y * W + x) * 4) as usize;
            let (r, g, b) = (pixels[i] as f64, pixels[i + 1] as f64, pixels[i + 2] as f64);
            soma += 0.2126 * r + 0.7152 * g + 0.0722 * b;
            acesos += (r.max(g).max(b) > 24.0) as u32;
            total += 1;
        }
    }
    Some(Resultado { pixels, media: (soma / total as f64) as f32, acesos: acesos as f32 / total as f32 })
}

fn pixel(p: &[u8], x: u32, y: u32) -> [u8; 4] {
    let i = ((y * W + x) * 4) as usize;
    [p[i], p[i + 1], p[i + 2], p[i + 3]]
}

#[test]
fn os_tres_visuais_desenham_com_musica() {
    let musica = retratos(6.0, 0.6);
    for (modelo, nome) in [(Modelo::Spectral, "spectral"), (Modelo::Fluxo, "fluxo"), (Modelo::Barras, "barras")] {
        let Some(r) = desenhar(modelo, &musica, false, nome) else {
            eprintln!("sem GPU: teste pulado");
            return;
        };
        assert!(r.acesos > 0.01, "{nome}: só {:.2}% de pixels acesos", r.acesos * 100.0);
        assert!(r.media < 200.0, "{nome}: imagem estourada (média {})", r.media);
        // Fora do retângulo fica a cor de fundo; o canto arredondado também.
        assert_eq!(pixel(&r.pixels, 5, 5), [0, 0, 51, 255], "{nome}: fundo");
        assert_eq!(pixel(&r.pixels, 22, 22), [0, 0, 51, 255], "{nome}: canto arredondado");
    }
}

#[test]
fn silencio_deixa_a_tela_quase_escura_e_sem_erro() {
    let silencio = vec![Retrato::default(); 90];
    for (modelo, nome) in [
        (Modelo::Fluxo, "fluxo-silencio"),
        (Modelo::Barras, "barras-silencio"),
        (Modelo::Spectral, "spectral-silencio"),
    ] {
        let Some(r) = desenhar(modelo, &silencio, false, nome) else { return };
        assert!(r.media < 60.0, "{nome}: média {}", r.media);
    }
}

#[test]
fn modo_seguro_nao_e_mais_claro_que_o_normal() {
    let musica = retratos(6.0, 0.8);
    let Some(normal) = desenhar(Modelo::Spectral, &musica, false, "spectral-forte") else { return };
    let Some(seguro) = desenhar(Modelo::Spectral, &musica, true, "spectral-seguro") else { return };
    assert!(seguro.acesos > 0.005, "o modo seguro não pode apagar o visual");
    assert!(seguro.media <= normal.media * 1.1 + 2.0, "seguro {} > normal {}", seguro.media, normal.media);
}
