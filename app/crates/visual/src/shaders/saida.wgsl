// Último passe, comum aos três visuais: aberração cromática, compressão do brilho, vinheta e granulação.
// Desenha direto no destino (a camada por baixo da interface), dentro de um retângulo de cantos arredondados.
// Porte de `displayFrag` de preview/shaders.js.
//
// U.v[0] = (largura, altura, raio dos cantos em px, tempo)
// U.v[1] = (batida, aberração, granulação, vinheta)
// U.v[2] = (exposição, cinza 0..1, _, _)
// t0 = cena

fn hash_px(p0: vec2<f32>) -> f32 {
    var p = fract(p0 * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

@fragment
fn fs_saida(in: VsOut) -> @location(0) vec4<f32> {
    let res = U.v[0].xy;
    let raio = U.v[0].z;
    let d = in.uv - 0.5;
    let ab = U.v[1].y * (0.003 + U.v[1].x * 0.01);
    var c = vec3(tex(t0, in.uv + d * ab).r, tex(t0, in.uv).g, tex(t0, in.uv - d * ab).b);

    // Compressão pela luminância máxima: segura o brilho sem lavar a cor para branco.
    let m = max(max(c.r, c.g), c.b);
    if (m > 0.0) {
        c *= (1.0 - exp(-m * U.v[2].x)) / m;
    }
    c *= 1.0 - dot(d, d) * U.v[1].w * 2.0;
    c += (hash_px(in.pos.xy + fract(U.v[0].w) * 100.0) - 0.5) * U.v[1].z;
    c = clamp(c, vec3(0.0), vec3(1.0));
    // Tema monocromático: o visual perde a cor, não o brilho.
    c = mix(c, vec3(dot(c, vec3(0.2126, 0.7152, 0.0722))), U.v[2].y);

    // Máscara de retângulo arredondado (1 px de suavização), em alfa pré-multiplicado.
    var cobertura = 1.0;
    if (raio > 0.0) {
        let px = in.uv * res;
        let meio = res * 0.5;
        let q = abs(px - meio) - (meio - vec2(raio));
        let sdf = length(max(q, vec2(0.0))) + min(max(q.x, q.y), 0.0) - raio;
        cobertura = clamp(0.5 - sdf, 0.0, 1.0);
    }
    return vec4(c * cobertura, cobertura);
}
