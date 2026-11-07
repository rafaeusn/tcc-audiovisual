// Port do "Spectral Audio Reactive.toe" (Pablo Torri, @pbltrr).
// Rede original: noise1 + noise2 -> remap1 -> geo1 (instancing 256x256) -> render1 -> bloom1 -> out1.
const SIM_RES = 256;
const SIGMA0 = 0.232;
// Faixa do nível médio (média de 5 s) entre trecho calmo e trecho energético.
const LEVEL_QUIET = 0.15;
const LEVEL_LOUD = 0.6;

const NOISE_LIB = /* glsl */ `
uint pcg(uint v) {
  uint s = v * 747796405u + 2891336453u;
  uint w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
  return (w >> 22u) ^ w;
}
vec2 grad(ivec2 p, uint seed) {
  uint h = pcg(uint(p.x) ^ pcg(uint(p.y) ^ pcg(seed)));
  float a = float(h) * (6.2831853 / 4294967295.0);
  return vec2(cos(a), sin(a));
}
float perlin(vec2 p, uint seed) {
  ivec2 i = ivec2(floor(p));
  vec2 f = fract(p);
  vec2 u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
  float a = dot(grad(i, seed), f);
  float b = dot(grad(i + ivec2(1, 0), seed), f - vec2(1, 0));
  float c = dot(grad(i + ivec2(0, 1), seed), f - vec2(0, 1));
  float d = dot(grad(i + ivec2(1, 1), seed), f - vec2(1, 1));
  return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}
// Equivalente ao Noise TOP: 4 harmônicos, "spread" = lacunaridade, "gain" = rugosidade.
// Normalizado para desvio-padrão ~1.
float fbm(vec2 p, uint seed, float spread, float gain) {
  float sum = 0.0, amp = 1.0, norm = 0.0;
  for (int k = 0; k < 4; k++) {
    sum += amp * perlin(p, seed + uint(k) * 7919u);
    norm += amp * amp;
    p = p * spread + vec2(17.31, 9.73);
    amp *= gain;
  }
  return sum / (sqrt(norm) * uSigma0);
}
vec3 sgnpow(vec3 x, float e) { return sign(x) * pow(abs(x), vec3(e)); }
`;

// Constantes calibradas contra as estatísticas exportadas do TouchDesigner via MCP:
// noise1: mediana 0.679, quartis +-0.08, p5/p95 0.26/1.10 ; noise2: mediana = offset, quartis +-0.003.
const simFrag = /* glsl */ `#version 300 es
precision highp float;
precision highp int;
out vec4 fragColor;
uniform float uSigma0, uOffset, uDrift, uWarp, uAmp1;
uniform int uDebug;
${NOISE_LIB}
const float K1 = 0.168;
const float K2 = 0.0056;
// O "period" do Noise TOP gera features ~4.4x menores que uv/period (medido pela autocorrelação).
const float FREQ = 4.4;
void main() {
  vec2 uv = gl_FragCoord.xy / ${SIM_RES}.0;

  // noise2 (period 0.38, spread 3.0, gain 0.31, exponent 1.59, amp 0.13) + offset = smsd
  vec2 p2 = uv * FREQ / 0.38;
  vec3 n2 = vec3(fbm(p2, 11u, 3.0, 0.31), fbm(p2, 4011u, 3.0, 0.31), fbm(p2, 9011u, 3.0, 0.31));
  n2 = sgnpow(n2, 1.59) * (K2 * uWarp) + uOffset;

  // remap1: horz = blue, vert = red, extend = hold
  vec2 s = clamp(vec2(n2.b, n2.r), 0.0, 1.0);
  if (uDebug == 1) s = uv;

  // noise1 (period 0.4, spread 2.6, gain 0.64, exponent 1.89, amp 1.09, offset 0.679, tx = t/23)
  vec2 p1 = (s * FREQ + vec2(uDrift, 0.0)) / 0.4;
  vec3 n1 = vec3(fbm(p1, 101u, 2.6, 0.64), fbm(p1, 5101u, 2.6, 0.64), fbm(p1, 12101u, 2.6, 0.64));
  vec3 v = 0.679 + K1 * uAmp1 * sgnpow(n1, 1.89);

  fragColor = uDebug == 2 ? vec4(n2, 1.0) : vec4(v, 1.0);
}`;

// geo1: instancetx = r, instancety = g, cor = rgb. A geometria é um line SOP de 0.0003 (um ponto).
const pointsVert = /* glsl */ `#version 300 es
precision highp float;
uniform highp sampler2D uPos;
uniform mat4 uMVP;
uniform float uSize, uDepth;
// Camada de energia (não existe no .toe): espalha a nuvem conforme o ritmo.
uniform vec2 uAnchor;   // centro da tela no plano z = 0
uniform vec2 uShift;    // deslocamento global (mola dos kicks + passeio)
uniform vec2 uVel;      // velocidade do deslocamento, gera rastro
uniform float uScale, uBurst, uJitter, uTime, uFlow, uSpin;
out vec3 vC;
float h1(float n) { return fract(sin(n * 12.9898) * 43758.5453); }
void main() {
  int id = gl_VertexID;
  vec4 d = texelFetch(uPos, ivec2(id % ${SIM_RES}, id / ${SIM_RES}), 0);
  float fid = float(id);
  float r1 = h1(fid), r2 = h1(fid + 71.3), r3 = h1(fid + 193.7);

  vec2 local = (d.rg - 0.679) * uScale;
  float cs = cos(uSpin), sn = sin(uSpin);
  local = mat2(cs, sn, -sn, cs) * local;
  // Ondas de fluxo: curvam as arestas retas que aparecem quando a nuvem é ampliada.
  local += uFlow * vec2(sin(local.y * 3.5 + uTime * 1.3), sin(local.x * 3.5 - uTime * 1.1));
  local *= 1.0 + uBurst * (0.3 + 1.7 * r1 * r1);
  vec2 jitter = vec2(sin(uTime * 7.0 + r2 * 40.0), cos(uTime * 6.3 + r3 * 40.0)) * uJitter * r3;
  vec2 trail = -uVel * (r2 * r2 * 0.25);

  vec3 w = vec3(uAnchor + local + uShift + jitter + trail, (d.b - 0.679) * uDepth * uScale);
  gl_Position = uMVP * vec4(w, 1.0);
  gl_PointSize = uSize;
  vC = d.rgb;
}`;

const pointsFrag = /* glsl */ `#version 300 es
precision highp float;
in vec3 vC;
out vec4 fragColor;
uniform vec3 uColA, uColB;
uniform float uBright, uAlpha;
void main() {
  float lum = clamp((vC.r + vC.g + vC.b) / 3.0, 0.0, 1.5);
  vec3 tint = mix(uColA, uColB, smoothstep(0.55, 0.9, vC.b));
  fragColor = vec4(tint * lum * uBright * uAlpha, 1.0);
}`;

// bloom1: preblacklevel 0.01, pregamma 2.63, prebrightness 1.02, fill 0.735, intensity 1.08
const PRE = /* glsl */ `
uniform float uBlack, uGamma, uPreBright;
// Gamma aplicado na luminância para manter a saturação das cores (o TD original é branco).
vec3 pre(vec3 c) {
  c = max(c - uBlack, 0.0) / (1.0 - uBlack);
  float l = max(max(c.r, c.g), c.b);
  if (l <= 0.0) return vec3(0.0);
  return c * (pow(l, 1.0 / uGamma) / l) * uPreBright;
}`;

const prefilterFrag = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 fragColor;
uniform sampler2D uSrc;
uniform vec2 uTexel;
${PRE}
void main() {
  vec2 t = uTexel * 0.5;
  vec3 c = pre(texture(uSrc, vUv + vec2(-t.x, -t.y)).rgb) + pre(texture(uSrc, vUv + vec2(t.x, -t.y)).rgb)
         + pre(texture(uSrc, vUv + vec2(-t.x, t.y)).rgb) + pre(texture(uSrc, vUv + vec2(t.x, t.y)).rgb);
  fragColor = vec4(c * 0.25, 1.0);
}`;

const downFrag = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 fragColor;
uniform sampler2D uSrc;
uniform vec2 uTexel;
void main() {
  vec2 t = uTexel;
  vec3 c = texture(uSrc, vUv).rgb * 4.0
         + texture(uSrc, vUv - t).rgb + texture(uSrc, vUv + t).rgb
         + texture(uSrc, vUv + vec2(t.x, -t.y)).rgb + texture(uSrc, vUv + vec2(-t.x, t.y)).rgb;
  fragColor = vec4(c / 8.0, 1.0);
}`;

const upFrag = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 fragColor;
uniform sampler2D uSrc;
uniform vec2 uTexel;
uniform float uWeight;
void main() {
  vec2 t = uTexel;
  vec3 c = texture(uSrc, vUv + vec2(-2.0 * t.x, 0.0)).rgb + texture(uSrc, vUv + vec2(2.0 * t.x, 0.0)).rgb
         + texture(uSrc, vUv + vec2(0.0, -2.0 * t.y)).rgb + texture(uSrc, vUv + vec2(0.0, 2.0 * t.y)).rgb
         + (texture(uSrc, vUv + vec2(-t.x, -t.y)).rgb + texture(uSrc, vUv + vec2(t.x, -t.y)).rgb
          + texture(uSrc, vUv + vec2(-t.x, t.y)).rgb + texture(uSrc, vUv + vec2(t.x, t.y)).rgb) * 2.0;
  fragColor = vec4(c / 12.0 * uWeight, 1.0);
}`;

const compositeFrag = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 fragColor;
uniform sampler2D uPoints, uBloom, uPrev;
uniform float uBloomInt, uFeedback;
${PRE}
void main() {
  vec3 base = pre(texture(uPoints, vUv).rgb);
  vec3 c = base + texture(uBloom, vUv).rgb * uBloomInt;
  vec3 prev = texture(uPrev, vUv).rgb * uFeedback;
  fragColor = vec4(max(c, prev), 1.0);
}`;

// ---------- matrizes ----------
function perspective(fovY, aspect, near, far) {
  const f = 1 / Math.tan(fovY / 2);
  const nf = 1 / (near - far);
  return [f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, (far + near) * nf, -1, 0, 0, 2 * far * near * nf, 0];
}
function lookAt(eye, target, up) {
  const sub = (a, b) => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
  const norm = (v) => { const l = Math.hypot(...v); return v.map((x) => x / l); };
  const cross = (a, b) => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
  const dot = (a, b) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
  const z = norm(sub(eye, target));
  const x = norm(cross(up, z));
  const y = cross(z, x);
  return [x[0], y[0], z[0], 0, x[1], y[1], z[1], 0, x[2], y[2], z[2], 0, -dot(x, eye), -dot(y, eye), -dot(z, eye), 1];
}
function mul(a, b) {
  const o = new Array(16);
  for (let c = 0; c < 4; c++)
    for (let r = 0; r < 4; r++)
      o[c * 4 + r] = a[r] * b[c * 4] + a[4 + r] * b[c * 4 + 1] + a[8 + r] * b[c * 4 + 2] + a[12 + r] * b[c * 4 + 3];
  return o;
}

// cam1: t = (0.71147, 0.8994, 3.55031), sem rotação, FOV 45° (render quadrado 1280x1280).
const CAM = [0.71147, 0.8994, 3.55031];
const ORBIT_CENTER = [0.62, 0.72, 0];

export function createSpectralTD(gl, { makeProgram, makeTarget, freeTarget, hasFloat }) {
  const sim = makeProgram(simFrag);
  const points = makeProgram(pointsFrag, pointsVert);
  const prefilter = makeProgram(prefilterFrag);
  const down = makeProgram(downFrag);
  const up = makeProgram(upFrag);
  const composite = makeProgram(compositeFrag);

  const simTarget = makeTarget(SIM_RES, SIM_RES, gl.NEAREST);
  let pointsTarget = null;
  let mips = [];

  const audio = { fast: 0, slow: 0.25 };
  const motion = { energy: 0, lastBeat: 0, angle: 0, wander: 0, off: [0, 0], vel: [0, 0], section: 0, floor: 1, ceil: 0, loud: 0, spin: 0, spinVel: 0, spinDir: 1 };

  // Converte o áudio em espaço: seções altas abrem a nuvem, kicks a lançam pela tela
  // (mola amortecida, alternando lados), batidas explodem as partículas e agudos dão tremor.
  function updateMotion(dt, bands, P, aspect) {
    // Seção: nível de ~1.5 s comparado ao piso e teto da própria música (adapta a cada faixa).
    motion.section += (audio.fast - motion.section) * (1 - Math.exp(-dt / 1.5));
    motion.floor = Math.min(motion.section, motion.floor + dt * 0.004);
    motion.ceil = Math.max(motion.section, motion.ceil - dt * 0.004);
    const rel = (motion.section - motion.floor) / Math.max(0.08, motion.ceil - motion.floor);
    const abs = (motion.section - LEVEL_QUIET) / (LEVEL_LOUD - LEVEL_QUIET);
    const loud = Math.min(1, Math.max(0, 0.4 * abs + 0.6 * rel));

    const punch = 0.6 * bands.flux + 0.4 * bands.beat;
    const rate = punch > motion.energy ? 14 : 3;
    motion.energy += (punch - motion.energy) * (1 - Math.exp(-dt * rate));

    if (bands.beat > motion.lastBeat + 0.3 && P.motion > 0) {
      motion.angle += Math.PI * (0.6 + Math.random() * 0.8);
      const kick = (1.2 + 2.4 * loud) * P.motion * (0.5 + bands.flux);
      motion.vel[0] += Math.cos(motion.angle) * kick * Math.min(aspect, 2) * 0.6;
      motion.vel[1] += Math.sin(motion.angle) * kick * 0.6;
      motion.spinDir = -motion.spinDir;
      motion.spinVel += motion.spinDir * (0.8 + 0.8 * loud) * P.motion;
    }
    motion.lastBeat = bands.beat;

    const k = 28, c = 4.5;
    for (let i = 0; i < 2; i++) {
      motion.vel[i] += (-k * motion.off[i] - c * motion.vel[i]) * dt;
      motion.off[i] += motion.vel[i] * dt;
    }
    motion.spinVel += (-10 * motion.spin - 3 * motion.spinVel) * dt;
    motion.spin += motion.spinVel * dt;

    motion.wander += dt * (0.3 + loud * 0.7) * P.speed;
    const A = 0.55 * loud * P.motion;
    const halfH = CAM[2] * Math.tan((22.5 * Math.PI) / 180) * 0.75;
    const halfW = halfH * aspect;
    const clamp = (v, m) => Math.max(-m, Math.min(m, v));
    motion.loud = loud;
    return {
      shift: [
        clamp(motion.off[0] + Math.sin(motion.wander * 0.9) * A * aspect * 0.7, halfW),
        clamp(motion.off[1] + Math.sin(motion.wander * 1.3 + 1.1) * A * 0.55, halfH),
      ],
      vel: motion.vel,
      scale: P.scale * (1 + P.spread * (0.7 * loud + 0.5 * motion.energy)),
      burst: P.spread * bands.beat * (0.2 + 0.4 * bands.flux) * 0.6,
      jitter: P.spread * bands.high * 0.03 * (0.5 + loud),
      warp: P.warp * (1 + P.spread * bands.mid * 0.4),
      flow: P.spread * (0.03 + 0.09 * loud + 0.05 * motion.energy),
      spin: motion.spin + motion.wander * 0.15 * P.motion,
      glow: 1 + motion.energy * 0.3 * P.spread,
    };
  }

  function ensureTargets(w, h) {
    if (pointsTarget && pointsTarget.w === w && pointsTarget.h === h) return;
    if (pointsTarget) { freeTarget(pointsTarget); mips.forEach(freeTarget); }
    pointsTarget = makeTarget(w, h);
    mips = [];
    let mw = w, mh = h;
    for (let i = 0; i < 6; i++) {
      mw = Math.max(1, mw >> 1);
      mh = Math.max(1, mh >> 1);
      mips.push(makeTarget(mw, mh));
    }
  }

  function bindTex(unit, tex) {
    gl.activeTexture(gl.TEXTURE0 + unit);
    gl.bindTexture(gl.TEXTURE_2D, tex);
  }
  function pass(target, prog) {
    gl.bindFramebuffer(gl.FRAMEBUFFER, target.fb);
    gl.viewport(0, 0, target.w, target.h);
    gl.useProgram(prog.p);
    return prog.u;
  }

  function render({ write, read, dt, time, bands, P, colA, colB, drop = 0 }) {
    if (!hasFloat) return false;
    ensureTargets(write.w, write.h);

    // smsd ("Slow Spectral Density"): média de ~10 s da energia espectral, normalizada.
    audio.fast += (bands.level - audio.fast) * (1 - Math.exp(-dt / 0.25));
    audio.slow += (audio.fast - audio.slow) * (1 - Math.exp(-dt / 5));
    const offset = Math.min(0.7, Math.max(0.3,
      0.49 + (audio.slow - 0.2) * 0.15 * P.intensity + (audio.fast - audio.slow) * 0.08 * P.react + bands.beat * 0.004 * P.react));
    const m = updateMotion(dt, bands, P, write.w / write.h);

    gl.disable(gl.BLEND);

    let u = pass(simTarget, sim);
    gl.uniform1f(u.uSigma0, SIGMA0);
    gl.uniform1f(u.uOffset, offset);
    gl.uniform1f(u.uDrift, time / 23);
    gl.uniform1f(u.uWarp, m.warp);
    gl.uniform1f(u.uAmp1, 1);
    gl.uniform1i(u.uDebug, 0);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    u = pass(pointsTarget, points);
    gl.uniform2f(u.uAnchor, CAM[0], CAM[1]);
    gl.uniform2fv(u.uShift, m.shift);
    gl.uniform2fv(u.uVel, m.vel);
    gl.uniform1f(u.uScale, m.scale);
    gl.uniform1f(u.uBurst, m.burst + drop * 0.6 * P.spread);
    gl.uniform1f(u.uJitter, m.jitter);
    gl.uniform1f(u.uFlow, m.flow);
    gl.uniform1f(u.uSpin, m.spin);
    gl.uniform1f(u.uTime, time);
    gl.clearColor(0, 0, 0, 1);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.enable(gl.BLEND);
    if (P.accum > 0) {
      gl.blendEquation(gl.FUNC_ADD);
      gl.blendFunc(gl.ONE, gl.ONE);
    } else {
      gl.blendEquation(gl.MAX);
    }
    const yaw = (P.orbit * Math.PI) / 180;
    const rel = [CAM[0] - ORBIT_CENTER[0], CAM[1] - ORBIT_CENTER[1], CAM[2] - ORBIT_CENTER[2]];
    const eye = [
      ORBIT_CENTER[0] + rel[0] * Math.cos(yaw) + rel[2] * Math.sin(yaw),
      CAM[1],
      ORBIT_CENTER[2] - rel[0] * Math.sin(yaw) + rel[2] * Math.cos(yaw),
    ];
    const fwd = [-Math.sin(yaw), 0, -Math.cos(yaw)];
    const view = lookAt(eye, [eye[0] + fwd[0], eye[1] + fwd[1], eye[2] + fwd[2]], [0, 1, 0]);
    const fov = 45 * (1 - (P.punch ?? 0) * bands.beat * 0.1 - drop * 0.15);
    const proj = perspective((fov * Math.PI) / 180, write.w / write.h, 0.1, 100);
    gl.uniformMatrix4fv(u.uMVP, false, mul(proj, view));
    bindTex(0, simTarget.tex);
    gl.uniform1i(u.uPos, 0);
    gl.uniform1f(u.uSize, Math.max(1, P.pointSize * (write.h / 1280)));
    gl.uniform1f(u.uDepth, P.depth);
    gl.uniform3fv(u.uColA, colA);
    gl.uniform3fv(u.uColB, colB);
    gl.uniform1f(u.uBright, P.bright * m.glow * (1 + bands.beat * 0.3 * P.react + drop * 0.8));
    gl.uniform1f(u.uAlpha, P.accum > 0 ? P.accum : 1);
    gl.drawArrays(gl.POINTS, 0, SIM_RES * SIM_RES);
    gl.disable(gl.BLEND);
    gl.blendEquation(gl.FUNC_ADD);

    const setPre = (uu) => {
      gl.uniform1f(uu.uBlack, 0.01);
      gl.uniform1f(uu.uGamma, 2.63);
      gl.uniform1f(uu.uPreBright, 1.02);
    };

    u = pass(mips[0], prefilter);
    bindTex(0, pointsTarget.tex);
    gl.uniform1i(u.uSrc, 0);
    gl.uniform2f(u.uTexel, 1 / pointsTarget.w, 1 / pointsTarget.h);
    setPre(u);
    gl.drawArrays(gl.TRIANGLES, 0, 3);

    for (let i = 1; i < mips.length; i++) {
      u = pass(mips[i], down);
      bindTex(0, mips[i - 1].tex);
      gl.uniform1i(u.uSrc, 0);
      gl.uniform2f(u.uTexel, 1 / mips[i - 1].w, 1 / mips[i - 1].h);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }

    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE);
    for (let i = mips.length - 1; i > 0; i--) {
      u = pass(mips[i - 1], up);
      bindTex(0, mips[i].tex);
      gl.uniform1i(u.uSrc, 0);
      gl.uniform2f(u.uTexel, 1 / mips[i].w, 1 / mips[i].h);
      gl.uniform1f(u.uWeight, 0.735);
      gl.drawArrays(gl.TRIANGLES, 0, 3);
    }
    gl.disable(gl.BLEND);

    u = pass(write, composite);
    bindTex(0, pointsTarget.tex);
    bindTex(1, mips[0].tex);
    bindTex(2, read.tex);
    gl.uniform1i(u.uPoints, 0);
    gl.uniform1i(u.uBloom, 1);
    gl.uniform1i(u.uPrev, 2);
    gl.uniform1f(u.uBloomInt, P.bloom * 0.45);
    gl.uniform1f(u.uFeedback, P.feedback);
    setPre(u);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    return true;
  }

  // Lê a textura de simulação para comparar com as estatísticas do TouchDesigner.
  function stats(debug = 0, offset = 0.514) {
    let u = pass(simTarget, sim);
    gl.uniform1f(u.uSigma0, SIGMA0);
    gl.uniform1f(u.uOffset, offset);
    gl.uniform1f(u.uDrift, 0);
    gl.uniform1f(u.uWarp, 1);
    gl.uniform1f(u.uAmp1, 1);
    gl.uniform1i(u.uDebug, debug);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    const px = new Float32Array(SIM_RES * SIM_RES * 4);
    gl.readPixels(0, 0, SIM_RES, SIM_RES, gl.RGBA, gl.FLOAT, px);
    const out = {};
    ['r', 'g', 'b'].forEach((c, ci) => {
      const v = [];
      for (let i = ci; i < px.length; i += 4) v.push(px[i]);
      v.sort((a, b) => a - b);
      const q = (p) => +v[Math.min(v.length - 1, Math.floor((p / 100) * v.length))].toFixed(3);
      out[c] = [0, 1, 5, 25, 50, 75, 95, 99, 100].map(q);
    });
    return out;
  }

  return { render, stats, motion };
}
