// Fluxo e Barras: visuais de um passe só, com rastro (o quadro anterior entra de volta, um pouco mais fraco).
// Porte de `fluxo` e `barras` de preview/shaders.js.
//
// U.v[0] = (largura, altura, tempo, modo)      modo: 1 = Fluxo, 2 = Barras
// U.v[1] = (grave, médio, agudo, volume)
// U.v[2] = (batida, intensidade, distorção, rastro)
// U.v[3] = (expansão, rotação, _, _)
// U.v[4] = cor A, U.v[5] = cor B
// t0 = quadro anterior, t1 = espectro (256x1)

fn espectro(x: f32) -> f32 {
    return textureSampleLevel(t1, amostrador, vec2(clamp(x, 0.0, 1.0), 0.5), 0.0).r;
}

fn hash(p0: vec2<f32>) -> f32 {
    var p = fract(p0 * vec2(123.34, 456.21));
    p += dot(p, p + 45.32);
    return fract(p.x * p.y);
}

fn ruido(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

fn fbm(p0: vec2<f32>) -> f32 {
    var p = p0;
    var v = 0.0;
    var a = 0.5;
    for (var i = 0; i < 5; i++) {
        v += a * ruido(p);
        p = vec2(1.6 * p.x - 1.2 * p.y, 1.2 * p.x + 1.6 * p.y);
        a *= 0.5;
    }
    return v;
}

fn paleta(t: f32) -> vec3<f32> {
    return mix(U.v[4].rgb, U.v[5].rgb, smoothstep(0.0, 1.0, clamp(t, 0.0, 1.0)));
}

fn girar(a: f32, v: vec2<f32>) -> vec2<f32> {
    let c = cos(a);
    let s = sin(a);
    return vec2(c * v.x + s * v.y, -s * v.x + c * v.y);
}

// Dobras de ruído (domain warping): o grave aumenta a distorção, o espectro acende as cristas.
fn fluxo(p0: vec2<f32>) -> vec3<f32> {
    let tempo = U.v[0].z;
    let grave = U.v[1].x;
    let agudo = U.v[1].z;
    let volume = U.v[1].w;
    let batida = U.v[2].x;
    let intensidade = U.v[2].y;
    let cor_a = U.v[4].rgb;
    let cor_b = U.v[5].rgb;

    let p = p0 * 1.6;
    let t = tempo * 0.15;
    let q = vec2(fbm(p + vec2(0.0, t)), fbm(p + vec2(5.2, 1.3) - t));
    let w = 2.0 + U.v[2].z * 2.5 * (0.4 + grave);
    let r = vec2(fbm(p + w * q + vec2(1.7, 9.2) + 0.6 * t), fbm(p + w * q + vec2(8.3, 2.8) - 0.4 * t));
    let f = fbm(p + w * r);
    let k = smoothstep(0.3, 0.85, f);
    let s = espectro(k);

    var col = mix(vec3(0.0), cor_a, smoothstep(0.0, 0.55, k));
    col = mix(col, cor_b, smoothstep(0.5, 1.0, k) * clamp(length(r) * 0.9, 0.0, 1.0));
    col *= k * (0.3 + volume * intensidade * 2.0 + batida * 0.4);
    col += cor_b * pow(s, 2.5) * k * intensidade * 0.8;
    let linhas = smoothstep(0.96, 1.0, fract(f * 7.0 - tempo * 0.3));
    col += mix(cor_a, vec3(1.0), 0.6) * linhas * k * (0.03 + agudo * 0.8);
    return col;
}

// 72 barras espelhadas a partir do centro: graves no meio, agudos nas pontas.
fn barras(uv: vec2<f32>) -> vec3<f32> {
    let x = abs(uv.x - 0.5) * 2.0;
    let n = 72.0;
    let id = floor(x * n);
    let fx = fract(x * n);
    let s = espectro((id + 0.5) / n);
    let h = s * 0.42 * U.v[2].y + 0.004;
    let y = abs(uv.y - 0.5);

    let corpo = step(y, h) * smoothstep(0.0, 0.18, fx) * (1.0 - smoothstep(0.82, 1.0, fx));
    let brilho = exp(-max(y - h, 0.0) * 25.0) * s * 0.5;
    var col = paleta(id / n) * (corpo * (0.35 + 0.9 * y / h) + brilho);
    col += paleta(0.5) * U.v[2].x * 0.08 * max(1.0 - y * 2.0, 0.0);
    return col;
}

@fragment
fn fs_cena(in: VsOut) -> @location(0) vec4<f32> {
    let res = U.v[0].xy;
    let tempo = U.v[0].z;
    let grave = U.v[1].x;
    let aspecto = res.x / res.y;
    // Mesmo sistema da prévia: origem no centro, y para cima, altura = 1.
    let p = vec2(in.pos.x - 0.5 * res.x, 0.5 * res.y - in.pos.y) / res.y;

    // Rastro: o quadro anterior gira, expande e ondula um pouco antes de entrar de volta.
    var c = in.uv - 0.5;
    c.x *= aspecto;
    c = girar(U.v[3].y * 0.01 * (1.0 + grave), c);
    c *= 1.0 - U.v[3].x * 0.006 * (1.0 + grave * 1.5);
    c += (vec2(ruido(c * 4.0 + tempo * 0.3), ruido(c * 4.0 - tempo * 0.3 + 7.0)) - 0.5) * U.v[2].z * 0.004;
    c.x /= aspecto;
    let anterior = tex(t0, c + 0.5) * U.v[2].w;

    var col = vec3(0.0);
    if (U.v[0].w < 1.5) {
        col = fluxo(p);
    } else {
        col = barras(in.uv);
    }
    return vec4(max(col, anterior), 1.0);
}
