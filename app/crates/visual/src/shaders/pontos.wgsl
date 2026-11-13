// Spectral, passo 2: 262.144 partículas. Cada instância lê a própria posição e cor na textura da simulação.
// No WebGL eram `gl.POINTS`; aqui cada partícula é um quadradinho de `tamanho` pixels (wgpu só tem ponto de 1 px).
//
// A camada de energia (não existe no .toe original) espalha a nuvem conforme o som: escala, explosão, tremor,
// ondas de fluxo e giro.

struct Pontos {
    mvp: mat4x4<f32>,
    // (ancora.x, ancora.y, desloc.x, desloc.y)
    a: vec4<f32>,
    // (escala, explosao, tremor, tempo)
    b: vec4<f32>,
    // (fluxo, giro, tamanho em px, profundidade)
    c: vec4<f32>,
    // (largura, altura, brilho, alfa)
    d: vec4<f32>,
    cor_a: vec4<f32>,
    cor_b: vec4<f32>,
};

@group(0) @binding(0) var<uniform> P: Pontos;
@group(0) @binding(1) var sim: texture_2d<f32>;

const LADO: u32 = 512u;
const CENTRO: f32 = 0.679;

struct Saida {
    @builtin(position) pos: vec4<f32>,
    @location(0) cor: vec3<f32>,
    // Posição dentro do quadradinho, em pixels a partir do centro (para suavizar a borda).
    @location(1) canto: vec2<f32>,
};

fn h1(n: f32) -> f32 {
    return fract(sin(n * 12.9898) * 43758.5453);
}

@vertex
fn vs(@builtin(vertex_index) vi: u32, @builtin(instance_index) id: u32) -> Saida {
    var cantos = array<vec2<f32>, 6>(
        vec2(-1.0, -1.0), vec2(1.0, -1.0), vec2(-1.0, 1.0),
        vec2(-1.0, 1.0), vec2(1.0, -1.0), vec2(1.0, 1.0),
    );
    let d = textureLoad(sim, vec2<i32>(i32(id % LADO), i32(id / LADO)), 0);
    let fid = f32(id);
    let r1 = h1(fid);
    let r2 = h1(fid + 71.3);
    let r3 = h1(fid + 193.7);
    let escala = P.b.x;
    let tempo = P.b.w;

    var local = (d.rg - CENTRO) * escala;
    let cs = cos(P.c.y);
    let sn = sin(P.c.y);
    local = vec2(cs * local.x - sn * local.y, sn * local.x + cs * local.y);
    // Ondas de fluxo: curvam as arestas retas que aparecem quando a nuvem é ampliada.
    local += P.c.x * vec2(sin(local.y * 3.5 + tempo * 1.3), sin(local.x * 3.5 - tempo * 1.1));
    local *= 1.0 + P.b.y * (0.3 + 1.7 * r1 * r1);
    let tremor = vec2(sin(tempo * 7.0 + r2 * 40.0), cos(tempo * 6.3 + r3 * 40.0)) * P.b.z * r3;

    let w = vec3(P.a.xy + local + P.a.zw + tremor, (d.b - CENTRO) * P.c.w * escala);
    var clip = P.mvp * vec4(w, 1.0);
    // Meio pixel a mais de cada lado: a borda da partícula some aos poucos em vez de serrilhar.
    let meio = P.c.z * 0.5 + 0.5;
    clip = vec4(clip.xy + cantos[vi] * (2.0 * meio) / P.d.xy * clip.w, clip.zw);

    var o: Saida;
    o.pos = clip;
    o.cor = d.rgb;
    o.canto = cantos[vi] * meio;
    return o;
}

@fragment
fn fs(in: Saida) -> @location(0) vec4<f32> {
    let lum = clamp((in.cor.r + in.cor.g + in.cor.b) / 3.0, 0.0, 1.5);
    let tinta = mix(P.cor_a.rgb, P.cor_b.rgb, smoothstep(0.55, 0.9, in.cor.b));
    let cobertura = clamp(P.c.z * 0.5 + 0.5 - length(in.canto), 0.0, 1.0);
    return vec4(tinta * lum * P.d.z * P.d.w * cobertura, 1.0);
}
