// Cabeçalho comum dos passes de tela cheia: um triângulo que cobre o alvo, 8 vetores de parâmetros, um
// amostrador e até 3 texturas. Cada shader diz no topo o que guarda em cada vetor.

struct Params {
    v: array<vec4<f32>, 8>,
};

@group(0) @binding(0) var<uniform> U: Params;
@group(0) @binding(1) var amostrador: sampler;
@group(0) @binding(2) var t0: texture_2d<f32>;
@group(0) @binding(3) var t1: texture_2d<f32>;
@group(0) @binding(4) var t2: texture_2d<f32>;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    // Coordenada de textura: (0, 0) no canto de cima à esquerda, igual ao que foi desenhado.
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    var o: VsOut;
    o.pos = vec4(p[i], 0.0, 1.0);
    o.uv = vec2(p[i].x * 0.5 + 0.5, 0.5 - p[i].y * 0.5);
    return o;
}

fn tex(t: texture_2d<f32>, uv: vec2<f32>) -> vec3<f32> {
    return textureSampleLevel(t, amostrador, uv, 0.0).rgb;
}
