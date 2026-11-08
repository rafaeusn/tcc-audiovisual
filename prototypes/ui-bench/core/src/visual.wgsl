// Anel espectral (versão simplificada do template 4) para os protótipos de UI.
// Mesmo shader nos dois protótipos, para comparar só o framework.

struct Params {
    resolution: vec2<f32>,
    time: f32,
    bass: f32,
    mid: f32,
    high: f32,
    level: f32,
    intensity: f32,
    pulse: f32,
    phase: f32,
    // Raio dos cantos em pixels (modo camada); 0 = sem máscara.
    radius: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> P: Params;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Triângulo que cobre a tela toda, sem buffer de vértices.
@vertex
fn vs(@builtin(vertex_index) i: u32) -> VsOut {
    var p = array<vec2<f32>, 3>(vec2(-1.0, -3.0), vec2(-1.0, 1.0), vec2(3.0, 1.0));
    var o: VsOut;
    o.pos = vec4(p[i], 0.0, 1.0);
    o.uv = p[i] * 0.5 + 0.5;
    return o;
}

fn hash(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

// Paleta do diretor: branco (calmo) -> azul (agitado) -> vermelho (intenso).
fn palette(x: f32) -> vec3<f32> {
    let white = vec3(0.92, 0.94, 1.0);
    let blue = vec3(0.25, 0.45, 1.0);
    let red = vec3(1.0, 0.18, 0.22);
    if (x < 0.55) {
        return mix(white, blue, smoothstep(0.3, 0.55, x));
    }
    return mix(blue, red, smoothstep(0.75, 0.92, x));
}

@fragment
fn fs(in: VsOut) -> @location(0) vec4<f32> {
    let aspect = P.resolution.x / max(P.resolution.y, 1.0);
    var q = (in.uv - 0.5) * vec2(aspect, 1.0) * 2.0;
    let r = length(q);
    let a = atan2(q.y, q.x);

    // Raio do anel: maior no calmo, pulsa na batida, deformado pelo espectro.
    let base = 0.55 + 0.12 * (1.0 - P.intensity) + 0.07 * P.pulse;
    // Soma de senos com frequências inteiras do ângulo: contínua na volta inteira (sem emenda em ±π).
    let n = 0.5 * sin(3.0 * a + P.phase) + 0.3 * sin(5.0 * a - 1.3 * P.phase) + 0.2 * sin(8.0 * a + 0.7 * P.phase) * (0.4 + P.high);
    let wob = n * 0.5 * (0.08 + 0.35 * P.bass + 0.15 * P.mid);
    let ring = base + wob;

    let d = abs(r - ring);
    let width = 0.012 + 0.03 * P.level;
    let core = smoothstep(width, 0.0, d);
    let glow = exp(-d * (14.0 - 6.0 * P.intensity)) * (0.35 + 0.9 * P.level);

    // Raios finos para dentro, no agudo.
    let rays = pow(abs(sin(a * 24.0 + P.phase * 2.0)), 18.0) * smoothstep(ring, ring * 0.4, r) * P.high * 0.6;

    let col = palette(P.intensity + 0.35 * P.pulse);
    var c = col * (core * 1.4 + glow + rays);
    // Vinheta leve e fundo quase preto.
    c += vec3(0.015, 0.017, 0.03) * (1.0 - r * 0.4);
    c = c / (1.0 + c);
    // Máscara de retângulo arredondado (1 px de suavização), em alfa pré-multiplicado.
    var cover = 1.0;
    if (P.radius > 0.0) {
        let px = in.uv * P.resolution;
        let half = P.resolution * 0.5;
        let qd = abs(px - half) - (half - vec2(P.radius));
        let sdf = length(max(qd, vec2(0.0))) + min(max(qd.x, qd.y), 0.0) - P.radius;
        cover = clamp(0.5 - sdf, 0.0, 1.0);
    }
    return vec4(pow(c, vec3(1.0 / 2.2)) * cover, cover);
}
