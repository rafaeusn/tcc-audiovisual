import { vert, sceneFrag, displayFrag } from './shaders.js?v=3';
import { AudioEngine } from './audio.js?v=3';
import { createSpectralTD } from './spectralTD.js?v=6';
import { Director, DIRECTED_KEYS } from './director.js';
import { Rhythm } from './rhythm.js';

const $ = (id) => document.getElementById(id);
const showError = (msg) => {
  $('error').style.display = 'flex';
  $('error').textContent = msg;
};

// ---------- Parâmetros (schema gera a interface) ----------
const TEMPLATES = ['Spectral', 'Fluxo', 'Barras', 'Spectral TD (original)'];
const TD = 3;
const TEMPLATE_DEFAULTS = [
  { feedback: 0.86, zoom: 1.5, rot: 0.4, warp: 0.6 },
  { feedback: 0.35, zoom: 0.3, rot: 0, warp: 1.0 },
  { feedback: 0.8, zoom: -1.2, rot: 0, warp: 0.3 },
  { feedback: 0.3, warp: 1, intensity: 1, colA: '#ffffff', colB: '#dfe6ff', exposure: 1.3, aberr: 0.6, grain: 0.03, vignette: 0.3 },
];
const PALETTES = [
  ['#ffffff', '#dfe6ff'],
  ['#ff2d55', '#3df2ff'],
  ['#d7263d', '#f4e9d8'],
  ['#00e5ff', '#ff00e5'],
  ['#00ff9c', '#7a5cff'],
  ['#ffb300', '#ff3d00'],
  ['#9be7ff', '#ffffff'],
  ['#6a00ff', '#00ffd5'],
];

const schema = [
  { key: 'mode', label: 'Template', type: 'select', options: TEMPLATES, value: 0 },
  { key: 'colA', label: 'Cor primária', type: 'color', value: PALETTES[1][0] },
  { key: 'colB', label: 'Cor secundária', type: 'color', value: PALETTES[1][1] },
  { key: 'hueCycle', label: 'Ciclo de cor', min: 0, max: 1, value: 0 },
  { group: 'Reação' },
  { key: 'intensity', label: 'Intensidade', min: 0, max: 2.5, value: 1 },
  { key: 'sensitivity', label: 'Sensibilidade do áudio', min: 0.2, max: 3, value: 1.2 },
  { key: 'speed', label: 'Velocidade', min: 0, max: 3, value: 1 },
  { key: 'warp', label: 'Distorção', min: 0, max: 2, value: 0.6 },
  { group: 'Spectral TD', only: [TD] },
  { key: 'react', label: 'Reatividade rápida', min: 0, max: 2, value: 0.5, only: [TD] },
  { key: 'scale', label: 'Escala base', min: 0.5, max: 3, value: 1.6, only: [TD] },
  { key: 'spread', label: 'Expansão com a energia', min: 0, max: 2, value: 1, only: [TD] },
  { key: 'motion', label: 'Movimento pela tela', min: 0, max: 2, value: 1, only: [TD] },
  { key: 'punch', label: 'Impacto do kick (zoom)', min: 0, max: 1, value: 0.5, only: [TD] },
  { key: 'pointSize', label: 'Tamanho das partículas', min: 1, max: 4, value: 1.2, only: [TD] },
  { key: 'bright', label: 'Brilho das partículas', min: 0.2, max: 3, value: 1, only: [TD] },
  { key: 'accum', label: 'Acúmulo de luz', min: 0, max: 1, value: 0, only: [TD] },
  { key: 'bloom', label: 'Bloom', min: 0, max: 3, value: 1.08, only: [TD] },
  { key: 'depth', label: 'Profundidade 3D', min: 0, max: 3, value: 0, only: [TD] },
  { key: 'orbit', label: 'Órbita da câmera', min: -60, max: 60, value: 0, step: 1, only: [TD] },
  { group: 'Rastro (feedback)' },
  { key: 'feedback', label: 'Rastro', min: 0, max: 0.98, value: 0.86 },
  { key: 'zoom', label: 'Expansão', min: -3, max: 5, value: 1.5, only: [0, 1, 2] },
  { key: 'rot', label: 'Rotação', min: -3, max: 3, value: 0.4, only: [0, 1, 2] },
  { group: 'Sincronia' },
  { key: 'syncOffset', label: 'Ajuste de sincronia (ms)', min: -250, max: 250, value: 0, step: 5 },
  { group: 'Pós-processamento' },
  { key: 'aberr', label: 'Aberração cromática', min: 0, max: 3, value: 1 },
  { key: 'exposure', label: 'Exposição', min: 0.3, max: 4, value: 1.6 },
  { key: 'grain', label: 'Granulação', min: 0, max: 0.2, value: 0.04 },
  { key: 'vignette', label: 'Vinheta', min: 0, max: 1, value: 0.5 },
  { key: 'quality', label: 'Resolução', min: 0.4, max: 1, value: 0.85 },
];

const P = {};
const inputs = {};
const conditional = [];

function buildControls() {
  const root = $('controls');
  for (const item of schema) {
    if (item.group) {
      const h = document.createElement('h2');
      h.textContent = item.group;
      root.appendChild(h);
      if (item.only) conditional.push([h, item.only]);
      continue;
    }
    P[item.key] = item.value;
    const row = document.createElement('div');
    row.className = 'row';
    const label = document.createElement('label');
    label.textContent = item.label;
    row.appendChild(label);

    let input;
    if (item.type === 'select') {
      row.appendChild(document.createElement('span'));
      input = document.createElement('select');
      item.options.forEach((name, i) => input.add(new Option(`${i + 1}. ${name}`, i)));
      input.value = item.value;
      input.addEventListener('change', () => setTemplate(+input.value));
    } else if (item.type === 'color') {
      input = document.createElement('input');
      input.type = 'color';
      input.value = item.value;
      input.addEventListener('input', () => (P[item.key] = input.value));
    } else {
      const val = document.createElement('span');
      val.className = 'val';
      row.appendChild(val);
      input = document.createElement('input');
      input.type = 'range';
      input.min = item.min;
      input.max = item.max;
      input.step = item.step ?? 0.01;
      input.value = item.value;
      const sync = () => {
        P[item.key] = +input.value;
        val.textContent = (+input.value).toFixed(2);
      };
      input.addEventListener('input', sync);
      input._sync = sync;
      input._val = val;
      sync();
    }
    inputs[item.key] = input;
    row.appendChild(input);
    root.appendChild(row);
    if (item.only) conditional.push([row, item.only]);
  }
  updateVisibility();

  PALETTES.forEach(([a, b]) => {
    const btn = document.createElement('button');
    btn.style.background = `linear-gradient(90deg, ${a} 50%, ${b} 50%)`;
    btn.addEventListener('click', () => {
      setParam('colA', a);
      setParam('colB', b);
    });
    $('palettes').appendChild(btn);
  });
}

function setParam(key, value) {
  P[key] = value;
  const input = inputs[key];
  input.value = value;
  input._sync?.();
}

function updateVisibility() {
  for (const [el, only] of conditional) el.style.display = only.includes(P.mode) ? '' : 'none';
}

function setTemplate(i) {
  P.mode = i;
  inputs.mode.value = i;
  for (const [k, v] of Object.entries(TEMPLATE_DEFAULTS[i])) setParam(k, v);
  updateVisibility();
  needsClear = true;
}
let needsClear = false;

// ---------- Cor ----------
function hexToRgb(hex) {
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}
function shiftHue([r, g, b], deg) {
  // Rotação de matiz em torno do eixo de luminância.
  const a = (deg * Math.PI) / 180;
  const c = Math.cos(a), s = Math.sin(a);
  const m = [
    0.299 + 0.701 * c + 0.168 * s, 0.587 - 0.587 * c + 0.330 * s, 0.114 - 0.114 * c - 0.497 * s,
    0.299 - 0.299 * c - 0.328 * s, 0.587 + 0.413 * c + 0.035 * s, 0.114 - 0.114 * c + 0.292 * s,
    0.299 - 0.300 * c + 1.250 * s, 0.587 - 0.588 * c - 1.050 * s, 0.114 + 0.886 * c - 0.203 * s,
  ];
  return [
    Math.max(0, m[0] * r + m[1] * g + m[2] * b),
    Math.max(0, m[3] * r + m[4] * g + m[5] * b),
    Math.max(0, m[6] * r + m[7] * g + m[8] * b),
  ];
}

// ---------- WebGL ----------
const canvas = $('gl');
const gl = canvas.getContext('webgl2', { antialias: false, alpha: false });
if (!gl) {
  showError('WebGL2 não é suportado neste navegador.');
  throw new Error('no webgl2');
}
const hasFloat = !!gl.getExtension('EXT_color_buffer_float');

function compile(type, src) {
  const s = gl.createShader(type);
  gl.shaderSource(s, src);
  gl.compileShader(s);
  if (!gl.getShaderParameter(s, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(s));
  return s;
}
function makeProgram(fs, vs = vert) {
  const p = gl.createProgram();
  gl.attachShader(p, compile(gl.VERTEX_SHADER, vs));
  gl.attachShader(p, compile(gl.FRAGMENT_SHADER, fs));
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(p));
  const u = {};
  const n = gl.getProgramParameter(p, gl.ACTIVE_UNIFORMS);
  for (let i = 0; i < n; i++) {
    const name = gl.getActiveUniform(p, i).name;
    u[name] = gl.getUniformLocation(p, name);
  }
  return { p, u };
}

let scene, display;
try {
  scene = makeProgram(sceneFrag);
  display = makeProgram(displayFrag);
} catch (e) {
  showError('Erro ao compilar shader:\n' + e.message);
  throw e;
}

gl.bindVertexArray(gl.createVertexArray());

function makeTarget(w, h, filter = gl.LINEAR) {
  const tex = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, tex);
  gl.texImage2D(gl.TEXTURE_2D, 0, hasFloat ? gl.RGBA16F : gl.RGBA8, w, h, 0, gl.RGBA, hasFloat ? gl.HALF_FLOAT : gl.UNSIGNED_BYTE, null);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, filter);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  const fb = gl.createFramebuffer();
  gl.bindFramebuffer(gl.FRAMEBUFFER, fb);
  gl.framebufferTexture2D(gl.FRAMEBUFFER, gl.COLOR_ATTACHMENT0, gl.TEXTURE_2D, tex, 0);
  gl.clearColor(0, 0, 0, 1);
  gl.clear(gl.COLOR_BUFFER_BIT);
  return { tex, fb, w, h };
}
function freeTarget(t) {
  gl.deleteTexture(t.tex);
  gl.deleteFramebuffer(t.fb);
}

let spectralTD;
try {
  spectralTD = createSpectralTD(gl, { makeProgram, makeTarget, freeTarget, hasFloat });
} catch (e) {
  showError('Erro ao compilar o Spectral TD:\n' + e.message);
  throw e;
}
window.__spectralTD = spectralTD;

let read = null, write = null;
function resize() {
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  const w = Math.max(1, Math.floor(innerWidth * dpr * P.quality));
  const h = Math.max(1, Math.floor(innerHeight * dpr * P.quality));
  if (read && read.w === w && read.h === h) return;
  if (read) { freeTarget(read); freeTarget(write); }
  read = makeTarget(w, h);
  write = makeTarget(w, h);
  canvas.width = w;
  canvas.height = h;
}

const specTex = gl.createTexture();
gl.bindTexture(gl.TEXTURE_2D, specTex);
gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
gl.texImage2D(gl.TEXTURE_2D, 0, gl.R8, 256, 2, 0, gl.RED, gl.UNSIGNED_BYTE, new Uint8Array(512));
gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);

// ---------- Áudio e interface do player ----------
const audio = new AudioEngine();
window.__audio = audio;
const director = new Director();
const rhythm = new Rhythm();
function resetDirector() {
  director.reset();
  rhythm.reset();
  analysisId++; // descarta a pré-análise da fonte anterior
  analysis = { state: 'idle', progress: 0, timeline: null };
}
window.__director = director;
let autoMode = true;

function updateAutoLock() {
  for (const key of [...DIRECTED_KEYS, 'sensitivity', 'colA', 'colB', 'hueCycle']) {
    if (inputs[key]) inputs[key].disabled = autoMode;
  }
}
$('auto').addEventListener('change', (e) => {
  autoMode = e.target.checked;
  updateAutoLock();
  if (!autoMode) $('energy-info').textContent = 'Diretor desligado: ajuste os parâmetros manualmente.';
});

let uiTimer = 0;
function syncDirectorUI(dt) {
  uiTimer += dt;
  if (uiTimer < 0.12) return;
  uiTimer = 0;
  $('energy-fill').style.left = `${director.intensity * 100}%`;
  $('energy-label').textContent = director.label;
  if (!autoMode) return;
  for (const key of [...DIRECTED_KEYS, 'sensitivity', 'colA', 'colB']) {
    const input = inputs[key];
    if (!input || !(key in P)) continue;
    input.value = P[key];
    if (input._val) input._val.textContent = (+P[key]).toFixed(2);
  }
  const bpm = director.beatConf > 0.3 && director.bpm ? `${Math.round(director.bpm)} BPM` : 'sem pulso fixo';
  const mode = analysis.state === 'ready' ? 'sincronizado (pré-análise)'
    : analysis.state === 'analyzing' || analysis.state === 'decoding' ? `analisando ${Math.round(analysis.progress * 100)}%`
    : 'ao vivo';
  $('energy-info').textContent =
    `Energia ${Math.round(director.intensity * 100)}% · ${bpm} · ${mode}` +
    (director.build > 0.3 ? ' · SUBIDA' : '') + (director.drop > 0.3 ? ' · DROP' : '');
}
const fmt = (s) => (isFinite(s) ? `${Math.floor(s / 60)}:${String(Math.floor(s % 60)).padStart(2, '0')}` : '0:00');
const setTrackName = (t) => ($('track-name').textContent = t);

// ---------- Pré-análise (arquivo): a música inteira é analisada num Worker ----------
// Enquanto analisa, o diretor usa o modo ao vivo; quando termina, troca para a timeline (sincronia exata).
const analysisWorker = new Worker('./analysis.worker.js', { type: 'module' });
let analysisId = 0;
let analysis = { state: 'idle', progress: 0, timeline: null };
analysisWorker.onmessage = (e) => {
  const { id, progress, timeline, error } = e.data;
  if (id !== analysisId) return; // resposta de uma faixa antiga
  if (error) { analysis = { state: 'error', progress: 0, timeline: null }; console.warn('pré-análise falhou:', error); return; }
  if (timeline) {
    analysis = { state: 'ready', progress: 1, timeline };
    director.setTimeline(timeline);
    window.__timeline = timeline;
  } else analysis.progress = progress;
};

async function analyzeFile(file) {
  const id = ++analysisId;
  analysis = { state: 'decoding', progress: 0, timeline: null };
  director.setTimeline(null);
  try {
    const buf = await audio.ctx.decodeAudioData(await file.arrayBuffer());
    if (id !== analysisId) return;
    const n = buf.length, ch = buf.numberOfChannels;
    const pcm = new Float32Array(n);
    for (let c = 0; c < ch; c++) {
      const d = buf.getChannelData(c);
      for (let i = 0; i < n; i++) pcm[i] += d[i] / ch;
    }
    analysis.state = 'analyzing';
    analysisWorker.postMessage({ id, pcm, sampleRate: buf.sampleRate }, [pcm.buffer]);
  } catch (err) {
    if (id === analysisId) analysis = { state: 'error', progress: 0, timeline: null };
    console.warn('não foi possível decodificar para a pré-análise:', err);
  }
}

// Instante da música que está chegando aos ouvidos agora: posição do player menos a latência de saída
// do áudio, mais ~1 quadro (o quadro desenhado agora aparece no próximo vsync), mais o ajuste manual.
function mediaTime() {
  if (audio.source !== 'file' || !audio.ctx) return null;
  const outLat = (audio.ctx.outputLatency || 0) + (audio.ctx.baseLatency || 0);
  return audio.el.currentTime - outLat + 1 / 60 + (P.syncOffset || 0) / 1000;
}

function openFile(file) {
  if (!file || !file.type.startsWith('audio')) return;
  audio.playFile(file);
  resetDirector();
  analyzeFile(file);
  setTrackName(file.name.replace(/\.[^.]+$/, ''));
  $('btn-play').textContent = 'Pause';
}

$('btn-open').addEventListener('click', () => $('file').click());
$('file').addEventListener('change', (e) => openFile(e.target.files[0]));

$('btn-capture').addEventListener('click', async () => {
  try {
    const label = await audio.capture();
    resetDirector();
    setTrackName(`Modo Escuta: capturando "${label || 'áudio do sistema'}"`);
    $('btn-play').textContent = 'Play';
  } catch (e) {
    if (e.name !== 'NotAllowedError') alert(e.message);
  }
});
audio.onCaptureEnded = () => setTrackName('Captura encerrada.');

$('btn-demo').addEventListener('click', () => {
  audio.startDemo();
  resetDirector();
  setTrackName('Batida de teste (sintetizada, 110 BPM)');
});

function togglePlay() {
  if (audio.source === 'demo') {
    audio.stopAll();
    setTrackName('Batida de teste parada.');
    return;
  }
  if (!audio.el || !audio.el.src) return;
  audio.ensure();
  if (audio.el.paused) {
    audio.stopAll();
    audio.el.play();
    audio.source = 'file';
  } else {
    audio.el.pause();
  }
}
$('btn-play').addEventListener('click', togglePlay);

$('seek').addEventListener('input', (e) => {
  if (audio.el?.duration) audio.el.currentTime = (e.target.value / 1000) * audio.el.duration;
});
$('volume').addEventListener('input', (e) => audio.setVolume(+e.target.value));

let dragDepth = 0;
addEventListener('dragenter', (e) => { e.preventDefault(); dragDepth++; document.body.classList.add('dragging'); });
addEventListener('dragleave', () => { if (--dragDepth <= 0) { dragDepth = 0; document.body.classList.remove('dragging'); } });
addEventListener('dragover', (e) => e.preventDefault());
addEventListener('drop', (e) => {
  e.preventDefault();
  dragDepth = 0;
  document.body.classList.remove('dragging');
  openFile(e.dataTransfer.files[0]);
});

addEventListener('keydown', (e) => {
  if (e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;
  if (e.key === 'h' || e.key === 'H') document.body.classList.toggle('hide-ui');
  else if (e.key === 'f' || e.key === 'F') {
    if (document.fullscreenElement) document.exitFullscreen();
    else document.documentElement.requestFullscreen();
  } else if (e.code === 'Space') { e.preventDefault(); togglePlay(); }
  else if (['1', '2', '3', '4'].includes(e.key)) setTemplate(+e.key - 1);
});

// ---------- Loop de renderização ----------
let last = performance.now();
let t = 0;
let hue = 0;
let accent = '';
let tdTime = 0;

function renderScene(b, colA, colB) {
  gl.bindFramebuffer(gl.FRAMEBUFFER, write.fb);
  gl.viewport(0, 0, write.w, write.h);
  gl.useProgram(scene.p);
  const u = scene.u;
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, read.tex);
  gl.activeTexture(gl.TEXTURE1);
  gl.bindTexture(gl.TEXTURE_2D, specTex);
  gl.uniform1i(u.uPrev, 0);
  gl.uniform1i(u.uSpec, 1);
  gl.uniform2f(u.uRes, write.w, write.h);
  gl.uniform1f(u.uTime, t);
  gl.uniform1f(u.uBass, b.bass);
  gl.uniform1f(u.uMid, b.mid);
  gl.uniform1f(u.uHigh, b.high);
  gl.uniform1f(u.uLevel, b.level);
  gl.uniform1f(u.uBeat, b.beat);
  gl.uniform1f(u.uIntensity, P.intensity);
  gl.uniform1f(u.uWarp, P.warp);
  gl.uniform1f(u.uFeedback, P.feedback);
  gl.uniform1f(u.uZoom, P.zoom);
  gl.uniform1f(u.uRot, P.rot);
  gl.uniform3fv(u.uColA, colA);
  gl.uniform3fv(u.uColB, colB);
  gl.uniform1i(u.uMode, P.mode);
  gl.drawArrays(gl.TRIANGLES, 0, 3);
}

function frame(now) {
  const dt = Math.min(0.05, (now - last) / 1000);
  last = now;

  audio.sensitivity = P.sensitivity;
  audio.update(dt);
  const b = audio.bands;
  const playing = audio.source === 'file' ? !audio.el.paused : audio.source !== 'none';
  const strength = playing ? rhythm.push(dt, b) : 0;
  director.update(dt, b, playing, { strength, band: rhythm.lastBand, groove: rhythm.groove, bpm: rhythm.bpm, density: rhythm.density }, analysis.timeline ? mediaTime() : null);
  if (autoMode) director.apply(P, P.mode === TD);
  syncDirectorUI(dt);
  t += dt * P.speed * (0.6 + b.level * 1.2 + b.beat * 0.6);
  tdTime += dt * P.speed;
  hue = (hue + dt * P.hueCycle * 40) % 360;

  resize();
  if (needsClear) {
    needsClear = false;
    gl.clearColor(0, 0, 0, 1);
    for (const target of [read, write]) {
      gl.bindFramebuffer(gl.FRAMEBUFFER, target.fb);
      gl.clear(gl.COLOR_BUFFER_BIT);
    }
  }

  gl.activeTexture(gl.TEXTURE1);
  gl.bindTexture(gl.TEXTURE_2D, specTex);
  gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 256, 2, gl.RED, gl.UNSIGNED_BYTE, audio.spec);

  const colA = shiftHue(hexToRgb(P.colA), hue);
  const colB = shiftHue(hexToRgb(P.colB), hue);

  if (P.mode === TD) {
    spectralTD.render({ write, read, dt, time: tdTime, bands: b, P, colA, colB, drop: autoMode ? director.drop : 0, dir: autoMode ? director : null });
  } else {
    renderScene(b, colA, colB);
  }

  gl.bindFramebuffer(gl.FRAMEBUFFER, null);
  gl.viewport(0, 0, canvas.width, canvas.height);
  gl.useProgram(display.p);
  const d = display.u;
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, write.tex);
  gl.uniform1i(d.uScene, 0);
  gl.uniform1f(d.uTime, now / 1000);
  gl.uniform1f(d.uBeat, b.beat);
  gl.uniform1f(d.uAberr, P.aberr);
  gl.uniform1f(d.uGrain, P.grain);
  gl.uniform1f(d.uVignette, P.vignette);
  gl.uniform1f(d.uExposure, P.exposure);
  gl.drawArrays(gl.TRIANGLES, 0, 3);

  [read, write] = [write, read];

  $('m-bass').style.height = `${b.bass * 100}%`;
  $('m-mid').style.height = `${b.mid * 100}%`;
  $('m-high').style.height = `${b.high * 100}%`;
  $('m-beat').style.height = `${b.beat * 100}%`;
  if (audio.el && audio.source === 'file') {
    $('t-cur').textContent = fmt(audio.el.currentTime);
    $('t-dur').textContent = fmt(audio.el.duration);
    if (audio.el.duration) $('seek').value = (audio.el.currentTime / audio.el.duration) * 1000;
  }
  $('btn-play').textContent = audio.source === 'file' && !audio.el.paused ? 'Pause' : 'Play';
  if (P.colA !== accent) {
    accent = P.colA;
    const [r, g, bl] = hexToRgb(accent);
    document.documentElement.style.setProperty('--accent', accent);
    document.documentElement.style.setProperty('--accent-text', 0.299 * r + 0.587 * g + 0.114 * bl > 0.6 ? '#111' : '#fff');
  }

  requestAnimationFrame(frame);
}

buildControls();
setTemplate(TD);
updateAutoLock();
requestAnimationFrame(frame);
