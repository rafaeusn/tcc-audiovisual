// Pré-análise offline de uma faixa inteira (arquivo local). Módulo puro: roda num Web Worker e no Node.
//
// Como conhece a música toda, pode olhar para frente: as batidas saem de um rastreador de batidas por
// programação dinâmica (Ellis 2007, o mesmo princípio do librosa), as transições de energia caem
// exatamente nas fronteiras de seção, e os drops são conhecidos antes de acontecer (dá para preparar a
// subida). O diretor usa isso em vez da análise em tempo real sempre que a faixa é um arquivo.
//
// Saída (timeline):
//   fps (curvas, 10 Hz): energy, loud, act, bright, beatConf, build  (Float32Array, 0-1)
//   beats: { t, strength, downbeat } | onsets: { t, strength, low } | drops: [{ t, mag }]
//   sections: [{ t0, t1, energy }] | bpm, beatConfidence, duration

const CURVE_FPS = 10;

const clamp01 = (x) => Math.min(1, Math.max(0, x));
const smoothstep = (a, b, x) => {
  const t = clamp01((x - a) / (b - a));
  return t * t * (3 - 2 * t);
};

function percentile(arr, p) {
  const v = Array.from(arr).filter((x) => Number.isFinite(x)).sort((a, b) => a - b);
  if (!v.length) return 0;
  return v[Math.min(v.length - 1, Math.max(0, Math.floor(p * v.length)))];
}

// FFT complexa radix-2 in-place.
function makeFFT(N) {
  const levels = Math.round(Math.log2(N));
  const cos = new Float64Array(N / 2), sin = new Float64Array(N / 2);
  for (let i = 0; i < N / 2; i++) { cos[i] = Math.cos((2 * Math.PI * i) / N); sin[i] = Math.sin((2 * Math.PI * i) / N); }
  const rev = new Uint32Array(N);
  for (let i = 0; i < N; i++) {
    let r = 0;
    for (let b = 0; b < levels; b++) r |= ((i >> b) & 1) << (levels - 1 - b);
    rev[i] = r;
  }
  return (re, im) => {
    for (let i = 0; i < N; i++) {
      const j = rev[i];
      if (j > i) { let t = re[i]; re[i] = re[j]; re[j] = t; t = im[i]; im[i] = im[j]; im[j] = t; }
    }
    for (let size = 2; size <= N; size <<= 1) {
      const half = size >> 1, step = N / size;
      for (let i = 0; i < N; i += size) {
        for (let j = i, k = 0; j < i + half; j++, k += step) {
          const l = j + half;
          const tr = re[l] * cos[k] + im[l] * sin[k];
          const ti = -re[l] * sin[k] + im[l] * cos[k];
          re[l] = re[j] - tr; im[l] = im[j] - ti;
          re[j] += tr; im[j] += ti;
        }
      }
    }
  };
}

// Filtro biquad aplicado ao sinal inteiro.
function biquad(x, [b0, b1, b2, a1, a2]) {
  const y = new Float32Array(x.length);
  let x1 = 0, x2 = 0, y1 = 0, y2 = 0;
  for (let i = 0; i < x.length; i++) {
    const v = b0 * x[i] + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
    x2 = x1; x1 = x[i]; y2 = y1; y1 = v; y[i] = v;
  }
  return y;
}

// Ponderação K (ITU-R BS.1770): base da medida de loudness (LUFS).
function kWeight(x, sr) {
  let G = 3.99984385397, Q = 0.7071752369554193, fc = 1681.974450955533;
  let A = Math.pow(10, G / 40), w0 = (2 * Math.PI * fc) / sr, alpha = Math.sin(w0) / (2 * Q), c = Math.cos(w0);
  let a0 = (A + 1) - (A - 1) * c + 2 * Math.sqrt(A) * alpha;
  const shelf = [
    (A * ((A + 1) + (A - 1) * c + 2 * Math.sqrt(A) * alpha)) / a0,
    (-2 * A * ((A - 1) + (A + 1) * c)) / a0,
    (A * ((A + 1) + (A - 1) * c - 2 * Math.sqrt(A) * alpha)) / a0,
    (2 * ((A - 1) - (A + 1) * c)) / a0,
    ((A + 1) - (A - 1) * c - 2 * Math.sqrt(A) * alpha) / a0,
  ];
  Q = 0.5003270373253953; fc = 38.13547087613982;
  w0 = (2 * Math.PI * fc) / sr; alpha = Math.sin(w0) / (2 * Q); c = Math.cos(w0);
  a0 = 1 + alpha;
  const hp = [(1 + c) / 2 / a0, -(1 + c) / a0, (1 + c) / 2 / a0, (-2 * c) / a0, (1 - alpha) / a0];
  return biquad(biquad(x, shelf), hp);
}

// Suavização gaussiana não causal (olha para trás e para frente).
function gaussSmooth(x, sigma) {
  if (sigma <= 0) return Float32Array.from(x);
  const r = Math.ceil(sigma * 3);
  const w = [];
  for (let i = -r; i <= r; i++) w.push(Math.exp(-0.5 * (i / sigma) ** 2));
  const y = new Float32Array(x.length);
  for (let i = 0; i < x.length; i++) {
    let s = 0, ws = 0;
    for (let j = -r; j <= r; j++) {
      const k = i + j;
      if (k < 0 || k >= x.length) continue;
      s += x[k] * w[j + r]; ws += w[j + r];
    }
    y[i] = s / ws;
  }
  return y;
}

// ---------- 1. Espectrograma em bandas logarítmicas + fluxo espectral ----------
function spectralFrames(pcm, sr, onProgress) {
  const H = Math.round(sr / 100);
  const fps = sr / H;
  const N = sr > 50000 ? 4096 : 2048;
  const fft = makeFFT(N);
  const win = new Float64Array(N);
  for (let i = 0; i < N; i++) win[i] = 0.5 - 0.5 * Math.cos((2 * Math.PI * i) / N);
  const nFrames = Math.max(1, Math.ceil(pcm.length / H));

  // Banco de filtros triangulares em escala log, 30 Hz - 16 kHz.
  const B = 64, fmin = 30, fmax = Math.min(16000, sr * 0.45);
  const binHz = sr / N;
  const edges = [];
  for (let j = 0; j < B + 2; j++) edges.push(fmin * Math.pow(fmax / fmin, j / (B + 1)));
  const bands = [];
  for (let j = 0; j < B; j++) {
    const lo = edges[j], mid = edges[j + 1], hi = edges[j + 2];
    const w = [];
    for (let k = Math.max(1, Math.floor(lo / binHz)); k <= Math.ceil(hi / binHz) && k < N / 2; k++) {
      const f = k * binHz;
      const v = f < mid ? (f - lo) / (mid - lo) : (hi - f) / (hi - mid);
      if (v > 0) w.push([k, v]);
    }
    if (!w.length) w.push([Math.max(1, Math.round(mid / binHz)), 1]);
    bands.push({ w, hz: mid });
  }
  const lowB = bands.filter((b) => b.hz < 200).length;
  const midB = bands.filter((b) => b.hz < 2500).length;
  const bassB = bands.filter((b) => b.hz < 150).length;

  const odf = new Float32Array(nFrames), odfLow = new Float32Array(nFrames);
  const odfMid = new Float32Array(nFrames), odfHigh = new Float32Array(nFrames);
  const centroid = new Float32Array(nFrames), full = new Float32Array(nFrames);
  const bassDb = new Float32Array(nFrames), totDb = new Float32Array(nFrames);
  const flat = new Float32Array(nFrames), full20 = new Float32Array(nFrames);
  const MU = 2; // fluxo contra o quadro de 20 ms atrás (SuperFlux), com filtro de máximo em frequência
  const hist = [];
  const re = new Float64Array(N), im = new Float64Array(N);
  const mag = [new Float64Array(N / 2), new Float64Array(N / 2)];
  const norm = 4 / N; // amplitude ~ 1 para senoide de fundo de escala (janela de Hann)

  for (let n0 = 0; n0 < nFrames; n0 += 2) {
    // Duas janelas reais por FFT complexa (parte real e imaginária).
    for (let i = 0; i < N; i++) {
      const pa = n0 * H - N / 2 + i, pb = pa + H;
      re[i] = (pa >= 0 && pa < pcm.length ? pcm[pa] : 0) * win[i];
      im[i] = (pb >= 0 && pb < pcm.length && n0 + 1 < nFrames ? pcm[pb] : 0) * win[i];
    }
    fft(re, im);
    for (let k = 0; k < N / 2; k++) {
      const k2 = k === 0 ? 0 : N - k;
      const xr = (re[k] + re[k2]) / 2, xi = (im[k] - im[k2]) / 2;
      const yr = (im[k] + im[k2]) / 2, yi = -(re[k] - re[k2]) / 2;
      mag[0][k] = Math.hypot(xr, xi) * norm;
      mag[1][k] = Math.hypot(yr, yi) * norm;
    }
    for (let s = 0; s < 2 && n0 + s < nFrames; s++) {
      const n = n0 + s;
      const m = mag[s];
      const Y = new Float32Array(B);
      const E = new Float64Array(B);
      let etot = 0, ebass = 0, cpos = 0, maxDb = -200;
      for (let j = 0; j < B; j++) {
        let e = 0;
        for (const [k, v] of bands[j].w) e += v * m[k] * m[k];
        E[j] = e;
        etot += e;
        if (j < bassB) ebass += e;
        cpos += e * (j / (B - 1));
        Y[j] = Math.log1p(100 * Math.sqrt(e));
        const db = 10 * Math.log10(e + 1e-12);
        if (db > maxDb) maxDb = db;
      }
      let nf = 0, nf20 = 0, lg = 0;
      for (let j = 0; j < B; j++) {
        const db = 10 * Math.log10(E[j] + 1e-12);
        if (db > maxDb - 30) nf++;
        if (db > maxDb - 20) nf20++;
        lg += Math.log(E[j] + 1e-12);
      }
      full[n] = nf / B;
      full20[n] = nf20 / B;
      flat[n] = etot > 1e-10 ? Math.exp(lg / B) / (etot / B) : 0;
      centroid[n] = etot > 1e-10 ? cpos / etot : 0;
      bassDb[n] = 10 * Math.log10(ebass + 1e-12);
      totDb[n] = 10 * Math.log10(etot + 1e-12);
      const ref = hist.length >= MU ? hist[hist.length - MU] : null;
      if (ref) {
        let fa = 0, fl = 0, fm = 0, fh = 0;
        for (let j = 0; j < B; j++) {
          const r = Math.max(ref[j], j > 0 ? ref[j - 1] : 0, j < B - 1 ? ref[j + 1] : 0);
          const d = Y[j] - r;
          if (d > 0) {
            fa += d;
            if (j < lowB) fl += d; else if (j < midB) fm += d; else fh += d;
          }
        }
        odf[n] = fa / B; odfLow[n] = fl / Math.max(1, lowB);
        odfMid[n] = fm / Math.max(1, midB - lowB); odfHigh[n] = fh / Math.max(1, B - midB);
      }
      hist.push(Y);
      if (hist.length > MU + 1) hist.shift();
    }
    if (onProgress && (n0 & 1023) === 0) onProgress(0.7 * (n0 / nFrames));
  }
  return { fps, nFrames, odf, odfLow, odfMid, odfHigh, centroid, full, full20, flat, bassDb, totDb };
}

// ---------- 2. Ataques ----------
function detectOnsets(S) {
  const { fps, odf, odfLow } = S;
  const n = odf.length;
  const scale = percentile(odf, 0.98) + 1e-9;
  const x = Float32Array.from(odf, (v) => v / scale);
  const preMax = Math.round(0.03 * fps), postMax = Math.round(0.03 * fps);
  const preAvg = Math.round(0.1 * fps), postAvg = Math.round(0.07 * fps);
  const wait = Math.round(0.05 * fps);
  const onsets = [];
  let last = -1e9;
  for (let i = 1; i < n - 1; i++) {
    const v = x[i];
    if (v < 0.1) continue;
    let isMax = true;
    for (let j = Math.max(0, i - preMax); j <= Math.min(n - 1, i + postMax); j++) if (x[j] > v) { isMax = false; break; }
    if (!isMax) continue;
    let s = 0, c = 0;
    for (let j = Math.max(0, i - preAvg); j <= Math.min(n - 1, i + postAvg); j++) { s += x[j]; c++; }
    if (v < s / c + 0.07) continue;
    if (i - last < wait) continue;
    last = i;
    const low = odfLow[i] / (odf[i] * 1.5 + 1e-9);
    onsets.push({ t: i / fps, strength: clamp01(v), low: clamp01(low) });
  }
  return { onsets, odfNorm: x };
}

// ---------- 3. Andamento e batidas (programação dinâmica) ----------
function estimateTempo(odfNorm, fps) {
  const n = odfNorm.length;
  let mean = 0;
  for (let i = 0; i < n; i++) mean += odfNorm[i];
  mean /= n;
  const x = Float32Array.from(odfNorm, (v) => v - mean);
  const lagMin = Math.round((fps * 60) / 240), lagMax = Math.round((fps * 60) / 50);
  const ac = new Float64Array(lagMax + 2);
  for (let l = lagMin - 1; l <= lagMax + 1; l++) {
    let s = 0;
    for (let i = l; i < n; i++) s += x[i] * x[i - l];
    ac[l] = s / (n - l);
  }
  let best = -Infinity, bestLag = Math.round((fps * 60) / 120);
  for (let l = lagMin; l <= lagMax; l++) {
    const bpm = (fps * 60) / l;
    const prior = Math.exp(-0.5 * (Math.log2(bpm / 120) / 1.0) ** 2);
    const v = ac[l] * prior;
    if (v > best && ac[l] >= ac[l - 1] && ac[l] >= ac[l + 1]) { best = v; bestLag = l; }
  }
  const a = ac[bestLag - 1], b = ac[bestLag], c = ac[bestLag + 1];
  const d = a - 2 * b + c;
  const lag = bestLag + (d !== 0 ? Math.max(-0.5, Math.min(0.5, (0.5 * (a - c)) / d)) : 0);
  return (fps * 60) / lag;
}

function trackBeats(odfNorm, fps, bpm, tightness = 100) {
  const n = odfNorm.length;
  const period = Math.round((fps * 60) / bpm);
  let sd = 0, mu = 0;
  for (let i = 0; i < n; i++) mu += odfNorm[i];
  mu /= n;
  for (let i = 0; i < n; i++) sd += (odfNorm[i] - mu) ** 2;
  sd = Math.sqrt(sd / n) + 1e-9;
  // Pontuação local: ataques convoluídos com uma gaussiana estreita (largura ~ período/32).
  const local = new Float32Array(n);
  const K = [];
  for (let j = -period; j <= period; j++) K.push(Math.exp(-0.5 * ((j * 32) / period) ** 2));
  for (let i = 0; i < n; i++) {
    let s = 0;
    for (let j = -period; j <= period; j++) {
      const k = i + j;
      if (k >= 0 && k < n) s += (odfNorm[k] / sd) * K[j + period];
    }
    local[i] = s;
  }
  const wStart = -2 * period, wEnd = -Math.round(period / 2);
  const txwt = [];
  for (let o = wStart; o <= wEnd; o++) txwt.push(-tightness * Math.log(-o / period) ** 2);
  const cum = new Float32Array(n);
  const back = new Int32Array(n).fill(-1);
  const localMax = Math.max(...local);
  let first = true;
  for (let i = 0; i < n; i++) {
    let best = -Infinity, bestK = -1;
    for (let o = wStart, w = 0; o <= wEnd; o++, w++) {
      const k = i + o;
      if (k < 0) continue;
      const v = txwt[w] + cum[k];
      if (v > best) { best = v; bestK = k; }
    }
    if (bestK < 0) best = 0;
    cum[i] = local[i] + best;
    if (first && local[i] < 0.01 * localMax) back[i] = -1;
    else { back[i] = bestK; first = false; }
  }
  // Última batida: último máximo local acima de metade da mediana dos máximos.
  const maxes = [];
  for (let i = 1; i < n - 1; i++) if (cum[i] > cum[i - 1] && cum[i] >= cum[i + 1]) maxes.push(i);
  if (!maxes.length) return { beats: [], local, period };
  const med = percentile(maxes.map((i) => cum[i]), 0.5);
  let tail = maxes[0];
  for (const i of maxes) if (cum[i] >= 0.5 * med) tail = i;
  const beats = [tail];
  while (back[beats[beats.length - 1]] >= 0) beats.push(back[beats[beats.length - 1]]);
  beats.reverse();
  // Remove batidas fracas no começo e no fim (silêncio, fade).
  const sm = beats.map((b, i) => {
    let s = 0, c = 0;
    for (let j = i - 2; j <= i + 2; j++) if (j >= 0 && j < beats.length) { s += local[beats[j]]; c++; }
    return s / c;
  });
  const thr = 0.5 * Math.sqrt(sm.reduce((a, v) => a + v * v, 0) / sm.length);
  let a = 0, z = beats.length - 1;
  while (a < z && sm[a] <= thr) a++;
  while (z > a && sm[z] <= thr) z--;
  return { beats: beats.slice(a, z + 1), local, period };
}

function pulseClarity(odfNorm, fps, period) {
  const step = 0.5, half = Math.round(4 * fps);
  const n = odfNorm.length;
  const values = new Float32Array(Math.ceil(n / fps / step) + 1);
  const lags = [];
  for (const m of [1, 2]) for (let f = 0.97; f <= 1.031; f += 0.015) lags.push(Math.round(period * m * f));
  for (let v = 0; v < values.length; v++) {
    const c = Math.round(v * step * fps);
    const a = Math.max(0, c - half), z = Math.min(n, c + half);
    if (z - a < 2 * period * 2) { values[v] = 0; continue; }
    let mean = 0;
    for (let i = a; i < z; i++) mean += odfNorm[i];
    mean /= z - a;
    let r0 = 0;
    for (let i = a; i < z; i++) r0 += (odfNorm[i] - mean) ** 2;
    if (r0 < 1e-6) { values[v] = 0; continue; }
    let best = 0;
    for (const L of lags) {
      let r = 0;
      for (let i = a + L; i < z; i++) r += (odfNorm[i] - mean) * (odfNorm[i - L] - mean);
      r = (r / r0) * ((z - a) / (z - a - L));
      if (r > best) best = r;
    }
    values[v] = best;
  }
  return { step, values };
}

// ---------- análise completa ----------
export function analyzeTrack(pcm, sr, { onProgress } = {}) {
  const progress = onProgress || (() => {});
  const duration = pcm.length / sr;
  const S = spectralFrames(pcm, sr, progress);
  const { fps } = S;
  const { onsets, odfNorm } = detectOnsets(S);
  progress(0.75);

  const bpm = estimateTempo(odfNorm, fps);
  const { beats: beatIdx, local } = trackBeats(odfNorm, fps, bpm);
  progress(0.85);

  // Clareza do pulso por batida: pico do ataque na batida vs. média entre batidas.
  const beats = [];
  const halfWin = Math.round(0.03 * fps);
  for (let i = 0; i < beatIdx.length; i++) {
    const b = beatIdx[i];
    let pk = 0;
    for (let j = b - halfWin; j <= b + halfWin; j++) if (j >= 0 && j < odfNorm.length && odfNorm[j] > pk) pk = odfNorm[j];
    const nb = beatIdx[i + 1] ?? b + Math.round((fps * 60) / bpm);
    let s = 0, c = 0;
    for (let j = b + halfWin + 1; j < nb - halfWin; j++) if (j < odfNorm.length) { s += odfNorm[j]; c++; }
    beats.push({ t: b / fps, peak: pk, ratio: pk / ((c ? s / c : pk) + 0.02), low: S.odfLow[b] });
  }
  // Clareza do pulso: autocorrelação local do envelope de ataques no período da batida (janela de 8 s).
  // Fala, rubato e pads dão ~0-0.1; bateria regular dá 0.3-0.7. Isso decide se a batida "existe" ali.
  const clarity = pulseClarity(odfNorm, fps, (fps * 60) / bpm);
  const clarityAt = (t) => clarity.values[Math.min(clarity.values.length - 1, Math.max(0, Math.round(t / clarity.step)))];
  for (let i = 0; i < beats.length; i++) beats[i].conf = smoothstep(0.1, 0.45, clarityAt(beats[i].t));
  // Tempo forte do compasso (4/4): fase com mais grave + ataque nas batidas.
  let bestPhase = 0, bestScore = -1;
  for (let p = 0; p < 4; p++) {
    let s = 0;
    for (let i = p; i < beats.length; i += 4) s += beats[i].low * 1.5 + beats[i].peak;
    if (s > bestScore) { bestScore = s; bestPhase = p; }
  }
  const pkScale = percentile(beats.map((b) => b.peak), 0.9) + 1e-9;
  for (let i = 0; i < beats.length; i++) {
    beats[i].downbeat = (i - bestPhase) % 4 === 0;
    beats[i].strength = clamp01(beats[i].peak / pkScale) * beats[i].conf;
  }
  const beatConfidence = beats.length ? percentile(beats.map((b) => b.conf), 0.5) : 0;

  // ---------- 4. Descritores a 10 Hz ----------
  const nb = Math.max(1, Math.ceil(duration * CURVE_FPS));
  const kw = kWeight(pcm, sr);
  const lufs = new Float32Array(nb);
  const winS = Math.round(0.4 * sr);
  for (let k = 0; k < nb; k++) {
    const c = Math.round((k / CURVE_FPS) * sr);
    let s = 0, cnt = 0;
    for (let i = Math.max(0, c - winS / 2); i < Math.min(kw.length, c + winS / 2); i++) { s += kw[i] * kw[i]; cnt++; }
    lufs[k] = cnt ? -0.691 + 10 * Math.log10(s / cnt + 1e-12) : -70;
  }
  const frameAt = (k) => Math.min(S.nFrames - 1, Math.round((k / CURVE_FPS) * fps));
  const avgFrames = (arr, k, half = 5) => {
    const c = frameAt(k);
    let s = 0, cnt = 0;
    for (let i = Math.max(0, c - half); i <= Math.min(S.nFrames - 1, c + half); i++) { s += arr[i]; cnt++; }
    return s / cnt;
  };
  const dens = new Float32Array(nb);
  {
    const sig = 0.8;
    for (const o of onsets) {
      const k0 = Math.floor((o.t - 2.5 * sig) * CURVE_FPS), k1 = Math.ceil((o.t + 2.5 * sig) * CURVE_FPS);
      for (let k = Math.max(0, k0); k <= Math.min(nb - 1, k1); k++) {
        const d = (k / CURVE_FPS - o.t) / sig;
        dens[k] += (o.strength * Math.exp(-d * d)) / (sig * Math.sqrt(Math.PI));
      }
    }
  }
  const conf = new Float32Array(nb);
  {
    let j = 0;
    const per = 60 / bpm;
    for (let k = 0; k < nb; k++) {
      const t = k / CURVE_FPS;
      while (j < beats.length - 1 && beats[j + 1].t <= t) j++;
      if (!beats.length) break;
      const b0 = beats[j], b1 = beats[Math.min(beats.length - 1, j + 1)];
      const near = Math.min(Math.abs(t - b0.t), Math.abs(t - b1.t));
      conf[k] = near < 2 * per ? (b0.conf + b1.conf) / 2 : 0;
    }
  }
  const cont = new Float32Array(nb);
  {
    const half = Math.round(1.0 * fps);
    for (let k = 0; k < nb; k++) {
      const c = frameAt(k);
      let mx = -200;
      for (let i = Math.max(0, c - half); i <= Math.min(S.nFrames - 1, c + half); i++) if (S.totDb[i] > mx) mx = S.totDb[i];
      let on = 0, cnt = 0;
      for (let i = Math.max(0, c - half); i <= Math.min(S.nFrames - 1, c + half); i++) { if (S.totDb[i] > mx - 22) on++; cnt++; }
      cont[k] = cnt ? on / cnt : 0;
    }
  }
  // Descritores brutos por bloco de 100 ms.
  const R = { lufs, dens, clarity: new Float32Array(nb), odf: new Float32Array(nb), centroid: new Float32Array(nb),
    full20: new Float32Array(nb), flat: new Float32Array(nb), bassDb: new Float32Array(nb), totDb: new Float32Array(nb) };
  for (let k = 0; k < nb; k++) {
    R.clarity[k] = clarityAt(k / CURVE_FPS);
    R.odf[k] = avgFrames(S.odf, k, 5);
    R.centroid[k] = avgFrames(S.centroid, k);
    R.full20[k] = avgFrames(S.full20, k);
    R.flat[k] = avgFrames(S.flat, k);
    R.bassDb[k] = avgFrames(S.bassDb, k);
    R.totDb[k] = avgFrames(S.totDb, k);
  }
  // Normalizados (0-1). Escalas tiradas da distribuição do corpus (tools/calibrate.mjs): cada uma
  // separa o que deve separar (ex.: clareza do pulso < 0.1 em fala/piano/orquestra, 0.4-0.8 em música marcada).
  const bpmFold = bpm > 180 ? bpm / 2 : bpm < 70 ? bpm * 2 : bpm;
  const tempo01 = smoothstep(80, 160, bpmFold);
  const D = {};
  for (const key of ['loud', 'pulse', 'noise', 'full', 'flux', 'bass', 'bright']) D[key] = new Float32Array(nb);
  for (let k = 0; k < nb; k++) {
    const audible = lufs[k] > -60;
    D.loud[k] = clamp01((lufs[k] + 30) / 22);                       // -30 LUFS (mono) = 0, -8 = 1
    D.pulse[k] = smoothstep(0.08, 0.6, R.clarity[k]);               // batida regular
    D.noise[k] = audible ? smoothstep(0.03, 0.3, R.flat[k]) : 0;    // distorção/ruído (guitarras, saws)
    D.full[k] = audible ? smoothstep(0.3, 0.8, R.full20[k]) : 0;    // espectro cheio
    D.flux[k] = smoothstep(0.01, 0.09, R.odf[k]);                   // quanto o espectro muda
    D.bass[k] = smoothstep(-40, -5, R.bassDb[k]);                   // grave presente (para drops)
    D.bright[k] = smoothstep(0.2, 0.5, R.centroid[k]);
  }
  D.cont = gaussSmooth(cont, 1.5 * CURVE_FPS);
  // Dinâmica relativa: volume do trecho comparado à mediana da própria faixa (em dB). Faz o crescendo de
  // uma orquestra (gravada baixa) e o breakdown de um hardstyle (masterizado alto) aparecerem do mesmo jeito.
  const lufsS = gaussSmooth(lufs, 1.0 * CURVE_FPS);
  const Lmed = percentile(Array.from(lufsS).filter((_, k) => lufs[k] > -60), 0.5);
  const rel = Float32Array.from(lufsS, (v) => Math.max(-1, Math.min(1, (v - Lmed) / 10)));
  const W = ENERGY_WEIGHTS;
  const A = new Float32Array(nb);
  for (let k = 0; k < nb; k++) {
    if (lufs[k] <= -60) { A[k] = 0; continue; }
    // Pulso e andamento só "empurram" a energia quando há corpo sonoro: um acompanhamento regular de
    // piano baixinho continua calmo.
    const drive = smoothstep(0.1, 0.55, D.loud[k]);
    let a = W.loud * D.loud[k] + (W.pulse + W.tempo * tempo01) * D.pulse[k] * drive + W.noise * D.noise[k]
      + W.full * D.full[k] + W.flux * D.flux[k];
    // Som com pausas frequentes (fala, frases soltas) é menos "animado" que um fluxo contínuo.
    a *= 0.55 + 0.45 * smoothstep(0.55, 0.9, D.cont[k]);
    // Piso suave: som audível nunca fica "morto", mas a dinâmica das faixas calmas continua visível.
    A[k] = W.floor + (1 - W.floor) * clamp01(a + W.rel * rel[k]);
  }
  progress(0.9);

  // ---------- 5. Seções (novidade) ----------
  const feats = ['loud', 'bass', 'pulse', 'full', 'noise', 'flux', 'bright'];
  const fw = { loud: 2, bass: 1.5, pulse: 1.5, full: 1, noise: 1, flux: 1, bright: 0.5 };
  const Wn = 4 * CURVE_FPS;
  const novelty = new Float32Array(nb);
  for (let k = Wn; k < nb - Wn; k++) {
    let s = 0;
    for (const f of feats) {
      let a = 0, b = 0;
      for (let j = k - Wn; j < k; j++) a += D[f][j];
      for (let j = k; j < k + Wn; j++) b += D[f][j];
      s += fw[f] * ((b - a) / Wn) ** 2;
    }
    novelty[k] = Math.sqrt(s);
  }
  const nMean = novelty.reduce((a, v) => a + v, 0) / nb;
  const nSd = Math.sqrt(novelty.reduce((a, v) => a + (v - nMean) ** 2, 0) / nb);
  const minGap = 4 * CURVE_FPS;
  let bounds = [];
  for (let k = 1; k < nb - 1; k++) {
    if (novelty[k] < Math.max(0.12, nMean + 0.6 * nSd)) continue;
    let isMax = true;
    for (let j = Math.max(0, k - 3 * CURVE_FPS); j <= Math.min(nb - 1, k + 3 * CURVE_FPS); j++) if (novelty[j] > novelty[k]) { isMax = false; break; }
    if (isMax && (!bounds.length || k - bounds[bounds.length - 1] >= minGap)) bounds.push(k);
  }
  // Ajusta cada fronteira ao tempo forte (ou batida, ou ataque forte) mais próximo.
  const snap = (t) => {
    let best = t, bd = Infinity;
    if (beatConfidence > 0.35) {
      for (const b of beats) {
        const d = Math.abs(b.t - t) * (b.downbeat ? 0.6 : 1);
        if (d < bd && Math.abs(b.t - t) < 0.7) { bd = d; best = b.t; }
      }
      if (bd < Infinity) return best;
    }
    for (const o of onsets) {
      const d = Math.abs(o.t - t) / (0.3 + o.strength);
      if (Math.abs(o.t - t) < 0.4 && d < bd) { bd = d; best = o.t; }
    }
    return best;
  };
  const cuts = [0, ...bounds.map((k) => snap(k / CURVE_FPS)), duration];
  const sections = [];
  for (let i = 0; i < cuts.length - 1; i++) {
    const k0 = Math.floor(cuts[i] * CURVE_FPS), k1 = Math.max(k0 + 1, Math.floor(cuts[i + 1] * CURVE_FPS));
    const trim = Math.min(5, Math.floor((k1 - k0) / 5));
    let s = 0, c = 0;
    for (let k = k0 + trim; k < k1 - trim; k++) { s += A[k]; c++; }
    sections.push({ t0: cuts[i], t1: cuts[i + 1], energy: c ? s / c : A[k0] });
  }

  // ---------- 6. Curva de energia final ----------
  const Asm = gaussSmooth(A, 0.5 * CURVE_FPS);
  const step = new Float32Array(nb);
  for (const sec of sections) {
    for (let k = Math.floor(sec.t0 * CURVE_FPS); k < Math.min(nb, Math.ceil(sec.t1 * CURVE_FPS)); k++) step[k] = sec.energy;
  }
  // Subida: imediata na fronteira; descida: rampa de ~0.8 s.
  for (let i = 1; i < sections.length; i++) {
    const prev = sections[i - 1].energy, next = sections[i].energy;
    if (next >= prev) continue;
    const k0 = Math.round(sections[i].t0 * CURVE_FPS);
    for (let j = 0; j < 8 && k0 + j < nb; j++) step[k0 + j] = prev + (next - prev) * ((j + 1) / 8);
  }
  // Música com batida tem seções claras (degraus); fala e música livre seguem a curva contínua.
  const stepW = 0.25 + 0.5 * beatConfidence;
  let E = new Float32Array(nb);
  for (let k = 0; k < nb; k++) E[k] = stepW * step[k] + (1 - stepW) * Asm[k];
  // Contraste: faixas com pouca dinâmica entre seções ganham um pouco de expansão (sem exagerar ruído).
  const audibleE = Array.from(E).filter((_, k) => lufs[k] > -60);
  const med = percentile(audibleE, 0.5);
  const spread = percentile(audibleE, 0.9) - percentile(audibleE, 0.1);
  const g = Math.min(1.3, Math.max(1, 0.3 / Math.max(0.05, spread)));
  E = E.map((v, k) => (lufs[k] > -60 ? clamp01(med + g * (v - med)) : 0));
  for (const sec of sections) sec.energy = clamp01(med + g * (sec.energy - med));

  // ---------- 7. Drops e subidas ----------
  const drops = [];
  const build = new Float32Array(nb);
  for (let i = 1; i < sections.length; i++) {
    const prev = sections[i - 1], sec = sections[i];
    const jump = sec.energy - prev.energy;
    const kb = Math.round(sec.t0 * CURVE_FPS);
    const bassJump = (D.bass[Math.min(nb - 1, kb + 5)] ?? 0) - (D.bass[Math.max(0, kb - 10)] ?? 0);
    let pulseNew = 0, cnt = 0;
    for (let k = kb; k < Math.min(nb, kb + 4 * CURVE_FPS); k++) { pulseNew += D.pulse[k]; cnt++; }
    pulseNew /= Math.max(1, cnt);
    // Drop = a energia salta E a batida está lá (sem batida é só um crescendo, não um drop).
    let loudNew = 0;
    for (let k = kb; k < Math.min(nb, kb + 4 * CURVE_FPS); k++) loudNew += D.loud[k];
    loudNew /= Math.max(1, cnt);
    if (jump < 0.15 || sec.energy < 0.55 || pulseNew < 0.45 || loudNew < 0.5 || beatConfidence < 0.3) continue;
    let mag = clamp01((jump - 0.08) / 0.35) * smoothstep(0.45, 0.85, sec.energy) * (bassJump > 0.25 ? 1.2 : 1);
    mag = clamp01(mag);
    if (mag < 0.15) continue;
    // Instante exato: ataque grave mais forte perto da fronteira.
    let t = sec.t0, bs = -1;
    for (const o of onsets) {
      if (o.t < sec.t0 - 0.25 || o.t > sec.t0 + 0.35) continue;
      const s = o.strength * (0.5 + o.low);
      if (s > bs) { bs = s; t = o.t; }
    }
    drops.push({ t, mag });
    // Subida antes do drop: rampa quadrática desde o começo da seção anterior (máx. 16 s).
    const t0 = Math.max(prev.t0, t - 16);
    if (t - t0 >= 2) {
      for (let k = Math.ceil(t0 * CURVE_FPS); k < Math.floor(t * CURVE_FPS) && k < nb; k++) {
        const u = (k / CURVE_FPS - t0) / (t - t0);
        build[k] = Math.max(build[k], mag * u * u);
      }
    }
  }

  // Curvas auxiliares para a forma: volume rápido e atividade (velocidade da morfose).
  const loud = gaussSmooth(D.loud.map((v, k) => clamp01(0.5 * v + 0.5 * (0.5 + 0.5 * rel[k]))), 0.15 * CURVE_FPS);
  const act = gaussSmooth(D.flux.map((v, k) => clamp01(0.4 * v + 0.35 * D.pulse[k] + 0.25 * tempo01 * D.pulse[k])), 0.3 * CURVE_FPS);
  const bright = gaussSmooth(D.bright, 0.8 * CURVE_FPS);
  progress(1);

  return {
    version: 1,
    duration, fps: CURVE_FPS, bpm, beatConfidence,
    energy: E, loud, act, bright, beatConf: conf, build, raw: A,
    beats: beats.map((b) => ({ t: b.t, strength: b.strength, downbeat: b.downbeat })),
    onsets, drops, sections,
    debug: { lufs, novelty, D, R },
  };
}

// Pesos da energia absoluta (calibrados no corpus, ver tools/calibrate.mjs).
export const ENERGY_WEIGHTS = { loud: 0.3, pulse: 0.15, noise: 0.15, full: 0.1, flux: 0.1, tempo: 0.15, rel: 0.22, floor: 0.14 };
