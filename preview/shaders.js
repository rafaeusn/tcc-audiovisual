export const vert = /* glsl */ `#version 300 es
const vec2 P[3] = vec2[3](vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
out vec2 vUv;
void main() {
  vec2 p = P[gl_VertexID];
  vUv = p * 0.5 + 0.5;
  gl_Position = vec4(p, 0.0, 1.0);
}`;

export const sceneFrag = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 fragColor;

uniform sampler2D uPrev;
uniform sampler2D uSpec;   // linha 0: espectro log (256), linha 1: forma de onda
uniform vec2 uRes;
uniform float uTime, uBass, uMid, uHigh, uLevel, uBeat;
uniform float uIntensity, uWarp, uFeedback, uZoom, uRot;
uniform vec3 uColA, uColB;
uniform int uMode;

#define PI 3.14159265

float spec(float x) { return texture(uSpec, vec2(clamp(x, 0.0, 1.0), 0.25)).r; }
float wave(float x) { return texture(uSpec, vec2(clamp(x, 0.0, 1.0), 0.75)).r * 2.0 - 1.0; }

float hash(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }
float noise(vec2 p) {
  vec2 i = floor(p), f = fract(p);
  vec2 u = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1, 0)), u.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), u.x), u.y);
}
float fbm(vec2 p) {
  float v = 0.0, a = 0.5;
  mat2 m = mat2(1.6, 1.2, -1.2, 1.6);
  for (int i = 0; i < 5; i++) { v += a * noise(p); p = m * p; a *= 0.5; }
  return v;
}
vec3 pal(float t) { return mix(uColA, uColB, smoothstep(0.0, 1.0, clamp(t, 0.0, 1.0))); }
mat2 rot(float a) { float c = cos(a), s = sin(a); return mat2(c, -s, s, c); }

vec3 spectral(vec2 p) {
  float r = length(p);
  float a = atan(p.y, p.x);
  float fa = abs(a) / PI;
  float s = spec(fa * 0.9);

  float baseR = 0.16 + uBass * 0.07 * uIntensity + uBeat * 0.03;
  float ringR = baseR + s * 0.24 * uIntensity;

  vec3 col = pal(fa) * (0.0022 / (abs(r - ringR) + 0.0025)) * (0.35 + 1.2 * s);

  float inside = step(baseR, r) * (1.0 - smoothstep(ringR - 0.01, ringR, r));
  float spokes = pow(abs(sin(a * 60.0)), 6.0);
  col += pal(fa) * inside * spokes * s * 0.35;

  float r2 = baseR * 0.62 + wave(fa) * 0.06 * uIntensity;
  col += pal(1.0 - fa) * (0.0014 / (abs(r - r2) + 0.002)) * (0.3 + uMid);

  col += pal(0.5) * (0.02 / (r + 0.04)) * (0.2 + uBeat * 0.8) * uIntensity * 0.5;

  vec2 g = rot(uTime * 0.05) * p * 18.0;
  vec2 gi = floor(g);
  float spark = step(0.985 - uHigh * 0.04, hash(gi)) * (1.0 - smoothstep(0.0, 0.35, length(fract(g) - 0.5)));
  col += pal(hash(gi + 3.1)) * spark * uHigh * 1.2 * step(ringR, r);
  return col;
}

vec3 fluxo(vec2 p) {
  p *= 1.6;
  float t = uTime * 0.15;
  vec2 q = vec2(fbm(p + vec2(0.0, t)), fbm(p + vec2(5.2, 1.3) - t));
  float w = 2.0 + uWarp * 2.5 * (0.4 + uBass);
  vec2 r = vec2(fbm(p + w * q + vec2(1.7, 9.2) + 0.6 * t), fbm(p + w * q + vec2(8.3, 2.8) - 0.4 * t));
  float f = fbm(p + w * r);
  float k = smoothstep(0.3, 0.85, f);
  float s = spec(k);

  vec3 col = mix(vec3(0.0), uColA, smoothstep(0.0, 0.55, k));
  col = mix(col, uColB, smoothstep(0.5, 1.0, k) * clamp(length(r) * 0.9, 0.0, 1.0));
  col *= k * (0.3 + uLevel * uIntensity * 2.0 + uBeat * 0.4);
  col += uColB * pow(s, 2.5) * k * uIntensity * 0.8;
  float lines = smoothstep(0.96, 1.0, fract(f * 7.0 - uTime * 0.3));
  col += mix(uColA, vec3(1.0), 0.6) * lines * k * (0.03 + uHigh * 0.8);
  return col;
}

vec3 barras(vec2 uv) {
  float x = abs(uv.x - 0.5) * 2.0;
  float N = 72.0;
  float id = floor(x * N);
  float fx = fract(x * N);
  float s = spec((id + 0.5) / N);
  float h = s * 0.42 * uIntensity + 0.004;
  float y = abs(uv.y - 0.5);

  float body = step(y, h) * smoothstep(0.0, 0.18, fx) * (1.0 - smoothstep(0.82, 1.0, fx));
  float glow = exp(-max(y - h, 0.0) * 25.0) * s * 0.5;
  vec3 col = pal(id / N) * (body * (0.35 + 0.9 * y / h) + glow);
  col += pal(0.5) * uBeat * 0.08 * max(1.0 - y * 2.0, 0.0);
  return col;
}

void main() {
  vec2 p = (gl_FragCoord.xy - 0.5 * uRes) / uRes.y;
  float aspect = uRes.x / uRes.y;

  vec2 c = vUv - 0.5;
  c.x *= aspect;
  c = rot(uRot * 0.01 * (1.0 + uBass)) * c;
  c *= 1.0 - uZoom * 0.006 * (1.0 + uBass * 1.5);
  c += (vec2(noise(c * 4.0 + uTime * 0.3), noise(c * 4.0 - uTime * 0.3 + 7.0)) - 0.5) * uWarp * 0.004;
  c.x /= aspect;
  vec3 prev = texture(uPrev, c + 0.5).rgb * uFeedback;

  vec3 col;
  if (uMode == 0) col = spectral(p);
  else if (uMode == 1) col = fluxo(p);
  else col = barras(vUv);

  fragColor = vec4(max(col, prev), 1.0);
}`;

export const displayFrag = /* glsl */ `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 fragColor;

uniform sampler2D uScene;
uniform float uTime, uBeat, uAberr, uGrain, uVignette, uExposure;

float hash(vec2 p) { p = fract(p * vec2(123.34, 456.21)); p += dot(p, p + 45.32); return fract(p.x * p.y); }

void main() {
  vec2 d = vUv - 0.5;
  float ab = uAberr * (0.003 + uBeat * 0.01);
  vec3 c = vec3(
    texture(uScene, vUv + d * ab).r,
    texture(uScene, vUv).g,
    texture(uScene, vUv - d * ab).b
  );
  // Tonemap pela luminância máxima: comprime o brilho sem lavar a cor para branco.
  float m = max(max(c.r, c.g), c.b);
  if (m > 0.0) c *= (1.0 - exp(-m * uExposure)) / m;
  c *= 1.0 - dot(d, d) * uVignette * 2.0;
  c += (hash(gl_FragCoord.xy + fract(uTime) * 100.0) - 0.5) * uGrain;
  fragColor = vec4(c, 1.0);
}`;
