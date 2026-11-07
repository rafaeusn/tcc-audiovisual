// Biblioteca do simulador: decodifica áudio (ffmpeg), emula o AnalyserNode do navegador e roda
// os mesmos módulos do app (features, rhythm, director, preanalysis) sem navegador.
import { spawnSync } from 'node:child_process';
import { Features } from '../../preview/features.js';
import { Rhythm } from '../../preview/rhythm.js';
import { Director } from '../../preview/director.js';
import { analyzeTrack } from '../../preview/preanalysis.js';

export const SR = 48000;
export const FPS = 60;

export function decode(file, sr = SR) {
  const dec = spawnSync('ffmpeg', ['-v', 'error', '-i', file, '-ac', '1', '-ar', String(sr), '-f', 'f32le', '-'], { maxBuffer: 2 ** 31 });
  if (dec.status !== 0) throw new Error(String(dec.stderr));
  return new Float32Array(dec.stdout.buffer, dec.stdout.byteOffset, Math.floor(dec.stdout.byteLength / 4));
}

function makeFft(N) {
  const levels = Math.log2(N);
  const cos = new Float32Array(N / 2), sin = new Float32Array(N / 2);
  for (let i = 0; i < N / 2; i++) { cos[i] = Math.cos((2 * Math.PI * i) / N); sin[i] = Math.sin((2 * Math.PI * i) / N); }
  const rev = new Uint32Array(N);
  for (let i = 0; i < N; i++) { let r = 0; for (let b = 0; b < levels; b++) r |= ((i >> b) & 1) << (levels - 1 - b); rev[i] = r; }
  const re = new Float64Array(N), im = new Float64Array(N);
  return (x) => {
    for (let i = 0; i < N; i++) { re[rev[i]] = x[i]; im[rev[i]] = 0; }
    for (let size = 2; size <= N; size *= 2) {
      const half = size / 2, step = N / size;
      for (let i = 0; i < N; i += size) {
        for (let j = i, k = 0; j < i + half; j++, k += step) {
          const l = j + half;
          const tr = re[l] * cos[k] + im[l] * sin[k];
          const ti = -re[l] * sin[k] + im[l] * cos[k];
          re[l] = re[j] - tr; im[l] = im[j] - ti; re[j] += tr; im[j] += ti;
        }
      }
    }
    const mag = new Float64Array(N / 2);
    for (let i = 0; i < N / 2; i++) mag[i] = Math.hypot(re[i], im[i]) / N;
    return mag;
  };
}

// AnalyserNode: janela de Blackman, suavização temporal, dB -> byte (-85..-20 dB).
function makeAnalyser(pcm, N, smoothing) {
  const fft = makeFft(N);
  const win = new Float64Array(N);
  const a = 0.16;
  for (let i = 0; i < N; i++) win[i] = (1 - a) / 2 - 0.5 * Math.cos((2 * Math.PI * i) / N) + (a / 2) * Math.cos((4 * Math.PI * i) / N);
  const smooth = new Float64Array(N / 2);
  const out = new Uint8Array(N / 2);
  const buf = new Float64Array(N);
  return (endSample) => {
    for (let i = 0; i < N; i++) { const p = endSample - N + i; buf[i] = (p >= 0 && p < pcm.length ? pcm[p] : 0) * win[i]; }
    const mag = fft(buf);
    for (let i = 0; i < N / 2; i++) {
      smooth[i] = smoothing * smooth[i] + (1 - smoothing) * mag[i];
      const db = 20 * Math.log10(Math.max(smooth[i], 1e-12));
      out[i] = Math.max(0, Math.min(255, Math.floor((255 * (db + 85)) / 65)));
    }
    return out;
  };
}

// Simula a reprodução quadro a quadro. Com `timeline`, o diretor usa a pré-análise (modo arquivo);
// sem, usa só a análise em tempo real (modo Escuta). Retorna telemetria a cada 0.25 s e os pulsos.
export function runPlayback(pcm, sr = SR, { timeline = null, every = 15, onFrame = null } = {}) {
  const main = makeAnalyser(pcm, 2048, 0.72);
  const onset = makeAnalyser(pcm, 1024, 0);
  const features = new Features();
  features.configure(sr, 2048, 1024);
  const rhythm = new Rhythm();
  const director = new Director();
  if (timeline) director.setTimeline(timeline);
  const dt = 1 / FPS;
  const frames = Math.floor((pcm.length / sr) * FPS);
  const rows = [], pulses = [], rhythmEvents = [];
  const tb = new Uint8Array(2048);
  let lastImpulse = 0;
  for (let f = 0; f < frames; f++) {
    const t = f * dt;
    const end = Math.floor(t * sr) + 1;
    const freq = main(end);
    const onsetF = onset(end);
    for (let i = 0; i < 2048; i++) { const p = end - 2048 + i; const v = p >= 0 ? pcm[p] : 0; tb[i] = Math.max(0, Math.min(255, Math.round(128 + 128 * v))); }
    features.sensitivity = director.sens;
    features.update(dt, freq, onsetF, tb);
    const strength = rhythm.push(dt, features.bands);
    if (strength > 0) rhythmEvents.push([t, strength]);
    director.update(dt, features.bands, true, { strength, band: rhythm.lastBand, groove: rhythm.groove, bpm: rhythm.bpm, density: rhythm.density }, t);
    if (onFrame) onFrame(t, dt, director, features.bands);
    if (director.impulse > 0) { pulses.push([t, director.impulse]); director.impulse = 0; }
    if (f % every === 0) {
      const b = features.bands;
      rows.push({ t, rmsDb: b.rmsDb, bass: b.bass, groove: rhythm.groove, bpm: rhythm.bpm, ...director.telemetry() });
    }
  }
  return { rows, pulses, rhythmEvents, drops: director.dropLog ?? [] };
}

export function preanalyze(pcm, sr = SR) {
  const t0 = performance.now();
  const tl = analyzeTrack(pcm, sr);
  tl.analysisMs = performance.now() - t0;
  return tl;
}
