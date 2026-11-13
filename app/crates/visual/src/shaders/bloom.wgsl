// Spectral, passos 3 a 5: bloom (dual Kawase em cadeia de mips) e composição com o rastro.
// Porte de bloom1 do .toe: preblacklevel 0.01, pregamma 2.63, prebrightness 1.02, fill 0.735.
//
// U.v[0] = (texel.x, texel.y, peso ou força do bloom, rastro)
// U.v[1] = (preto, gama, pré-brilho, _)

// Gama aplicada na luminância para manter a saturação das cores (o original é branco).
fn pre(c0: vec3<f32>) -> vec3<f32> {
    let preto = U.v[1].x;
    let c = max(c0 - preto, vec3(0.0)) / (1.0 - preto);
    let l = max(max(c.r, c.g), c.b);
    if (l <= 0.0) {
        return vec3(0.0);
    }
    return c * (pow(l, 1.0 / U.v[1].y) / l) * U.v[1].z;
}

@fragment
fn fs_prefiltro(in: VsOut) -> @location(0) vec4<f32> {
    let t = U.v[0].xy * 0.5;
    let c = pre(tex(t0, in.uv + vec2(-t.x, -t.y))) + pre(tex(t0, in.uv + vec2(t.x, -t.y)))
          + pre(tex(t0, in.uv + vec2(-t.x, t.y))) + pre(tex(t0, in.uv + vec2(t.x, t.y)));
    return vec4(c * 0.25, 1.0);
}

@fragment
fn fs_reduz(in: VsOut) -> @location(0) vec4<f32> {
    let t = U.v[0].xy;
    let c = tex(t0, in.uv) * 4.0
          + tex(t0, in.uv - t) + tex(t0, in.uv + t)
          + tex(t0, in.uv + vec2(t.x, -t.y)) + tex(t0, in.uv + vec2(-t.x, t.y));
    return vec4(c / 8.0, 1.0);
}

@fragment
fn fs_amplia(in: VsOut) -> @location(0) vec4<f32> {
    let t = U.v[0].xy;
    let c = tex(t0, in.uv + vec2(-2.0 * t.x, 0.0)) + tex(t0, in.uv + vec2(2.0 * t.x, 0.0))
          + tex(t0, in.uv + vec2(0.0, -2.0 * t.y)) + tex(t0, in.uv + vec2(0.0, 2.0 * t.y))
          + (tex(t0, in.uv + vec2(-t.x, -t.y)) + tex(t0, in.uv + vec2(t.x, -t.y))
           + tex(t0, in.uv + vec2(-t.x, t.y)) + tex(t0, in.uv + vec2(t.x, t.y))) * 2.0;
    return vec4(c / 12.0 * U.v[0].z, 1.0);
}

// t0 = partículas, t1 = bloom, t2 = quadro anterior (rastro)
@fragment
fn fs_composicao(in: VsOut) -> @location(0) vec4<f32> {
    let base = pre(tex(t0, in.uv));
    let c = base + tex(t1, in.uv) * U.v[0].z;
    let anterior = tex(t2, in.uv) * U.v[0].w;
    return vec4(max(c, anterior), 1.0);
}
