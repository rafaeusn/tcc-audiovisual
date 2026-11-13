// Spectral, passo 1: a "simulação". Uma textura 512x512 em que cada pixel é a posição (r, g) e a cor de uma
// partícula. Porte de noise1 + noise2 + remap1 do "Spectral Audio Reactive.toe" (Pablo Torri, @pbltrr).
//
// U.v[0] = (offset, drift, warp, amp1)
//
// As constantes foram calibradas contra as estatísticas exportadas do TouchDesigner (docs/port-spectral-td.md):
// noise1: mediana 0,679, quartis ±0,08; noise2: mediana = offset, quartis ±0,003. Não mudar sem recalibrar.

const SIGMA0: f32 = 0.232;
const K1: f32 = 0.168;
const K2: f32 = 0.0056;
// O "period" do Noise TOP gera detalhes ~4,4x menores que uv/period (medido pela autocorrelação).
const FREQ: f32 = 4.4;
// O .toe usa 256x256. A superfície é uma função contínua de uv, então amostrar 512x512 dá a mesma forma com
// 4 vezes mais partículas (imagem mais nítida, sem recalibrar nada).
const LADO: f32 = 512.0;

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

fn grad(p: vec2<i32>, seed: u32) -> vec2<f32> {
    let h = pcg(bitcast<u32>(p.x) ^ pcg(bitcast<u32>(p.y) ^ pcg(seed)));
    let a = f32(h) * (6.2831853 / 4294967295.0);
    return vec2(cos(a), sin(a));
}

fn perlin(p: vec2<f32>, seed: u32) -> f32 {
    let i = vec2<i32>(floor(p));
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = dot(grad(i, seed), f);
    let b = dot(grad(i + vec2<i32>(1, 0), seed), f - vec2(1.0, 0.0));
    let c = dot(grad(i + vec2<i32>(0, 1), seed), f - vec2(0.0, 1.0));
    let d = dot(grad(i + vec2<i32>(1, 1), seed), f - vec2(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

// Equivalente ao Noise TOP: 4 harmônicos, "spread" = lacunaridade, "gain" = rugosidade. Desvio-padrão ~1.
fn fbm(p0: vec2<f32>, seed: u32, spread: f32, gain: f32) -> f32 {
    var p = p0;
    var sum = 0.0;
    var amp = 1.0;
    var norm = 0.0;
    for (var k = 0u; k < 4u; k++) {
        sum += amp * perlin(p, seed + k * 7919u);
        norm += amp * amp;
        p = p * spread + vec2(17.31, 9.73);
        amp *= gain;
    }
    return sum / (sqrt(norm) * SIGMA0);
}

fn sgnpow(x: vec3<f32>, e: f32) -> vec3<f32> {
    return sign(x) * pow(abs(x), vec3(e));
}

@fragment
fn fs_sim(in: VsOut) -> @location(0) vec4<f32> {
    let offset = U.v[0].x;
    let drift = U.v[0].y;
    let warp = U.v[0].z;
    let amp1 = U.v[0].w;
    let uv = in.pos.xy / LADO;

    // noise2 (period 0.38, spread 3.0, gain 0.31, exponent 1.59, amp 0.13) + offset
    let p2 = uv * FREQ / 0.38;
    var n2 = vec3(fbm(p2, 11u, 3.0, 0.31), fbm(p2, 4011u, 3.0, 0.31), fbm(p2, 9011u, 3.0, 0.31));
    n2 = sgnpow(n2, 1.59) * (K2 * warp) + offset;

    // remap1: horizontal = azul, vertical = vermelho, bordas seguradas
    let s = clamp(vec2(n2.b, n2.r), vec2(0.0), vec2(1.0));

    // noise1 (period 0.4, spread 2.6, gain 0.64, exponent 1.89, amp 1.09, offset 0.679, tx = t/23)
    let p1 = (s * FREQ + vec2(drift, 0.0)) / 0.4;
    let n1 = vec3(fbm(p1, 101u, 2.6, 0.64), fbm(p1, 5101u, 2.6, 0.64), fbm(p1, 12101u, 2.6, 0.64));
    let v = 0.679 + K1 * amp1 * sgnpow(n1, 1.89);
    return vec4(v, 1.0);
}
