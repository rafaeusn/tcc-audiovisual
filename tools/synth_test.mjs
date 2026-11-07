// Testes sintéticos de sincronia: gera áudio com tempos conhecidos (bumbo, caixa, drops) e mede
// o acerto e a latência dos dois modos do diretor (pré-análise de arquivo e ao vivo).
// Uso: node tools/synth_test.mjs
import { preanalyze, runPlayback } from './lib/sim.mjs';

const SR = 48000;
function rng(seed) {
  return () => {
    seed |= 0; seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const add = (x, n0, i, v) => { if (n0 + i >= 0 && n0 + i < x.length) x[n0 + i] += v; };
function kick(x, t, amp = 0.9) {
  const n0 = Math.round(t * SR); let ph = 0;
  for (let i = 0; i < 0.35 * SR; i++) { const s = i / SR; ph += (2 * Math.PI * (50 + 110 * Math.exp(-s / 0.03))) / SR; add(x, n0, i, Math.sin(ph) * Math.exp(-s / 0.16) * amp); }
}
function snare(x, t, r, amp = 0.5) {
  const n0 = Math.round(t * SR);
  for (let i = 0; i < 0.25 * SR; i++) { const s = i / SR; add(x, n0, i, ((r() * 2 - 1) * Math.exp(-s / 0.09) * 0.7 + Math.sin(2 * Math.PI * 190 * s) * Math.exp(-s / 0.05) * 0.5) * amp); }
}
function hat(x, t, r, amp = 0.12) {
  const n0 = Math.round(t * SR); let prev = 0;
  for (let i = 0; i < 0.06 * SR; i++) { const s = i / SR; const w = r() * 2 - 1; add(x, n0, i, (w - prev) * Math.exp(-s / 0.02) * amp); prev = w; }
}
function pad(x, t0, t1, amp = 0.1) {
  for (let i = Math.round(t0 * SR); i < Math.min(x.length, t1 * SR); i++) {
    const s = i / SR; const lfo = 0.7 + 0.3 * Math.sin(2 * Math.PI * 0.2 * s);
    x[i] += amp * lfo * (Math.sin(2 * Math.PI * 220 * s) + Math.sin(2 * Math.PI * 277.2 * s) + Math.sin(2 * Math.PI * 329.6 * s)) / 3;
  }
}
function bass(x, t, dur, f, amp = 0.35) {
  const n0 = Math.round(t * SR);
  for (let i = 0; i < dur * SR; i++) { const s = i / SR; add(x, n0, i, Math.sin(2 * Math.PI * f * s) * Math.min(1, s / 0.01) * Math.exp(-s / (dur * 0.8)) * amp); }
}
function note(x, t, f, amp = 0.25) {
  const n0 = Math.round(t * SR);
  for (let i = 0; i < 1.5 * SR; i++) {
    const s = i / SR; let v = 0;
    for (let k = 1; k <= 5; k++) v += Math.sin(2 * Math.PI * k * f * s) / (k * k);
    add(x, n0, i, v * Math.exp(-s / 0.7) * Math.min(1, s / 0.004) * amp);
  }
}
function syllable(x, t, dur, r, amp = 0.3) {
  const n0 = Math.round(t * SR); const f0 = 110 + r() * 60; const f1 = 500 + r() * 400, f2 = 1200 + r() * 900;
  for (let i = 0; i < dur * SR; i++) {
    const s = i / SR; const env = Math.sin(Math.PI * Math.min(1, s / dur));
    const src = Math.sin(2 * Math.PI * f0 * s) + 0.5 * Math.sin(2 * Math.PI * 2 * f0 * s);
    add(x, n0, i, amp * env * (0.6 * src * Math.sin(2 * Math.PI * f1 * s) + 0.3 * src * Math.sin(2 * Math.PI * f2 * s) + 0.05 * (r() * 2 - 1)));
  }
}

// ---------- cenários ----------
function edm() {
  const r = rng(1), bpm = 128, p = 60 / bpm, dur = 96;
  const x = new Float32Array(dur * SR);
  const kicks = [], beats = [];
  pad(x, 0, 16); pad(x, 48, 64, 0.14);
  for (let t = 0; t < dur; t += p / 2) if (t < 48 || t >= 64) hat(x, t, r);
  for (const [a, b] of [[16, 48], [64, 96]]) {
    for (let i = 0, t = a; t < b - 1e-6; i++, t = a + i * p) {
      kick(x, t); kicks.push(t); beats.push(t);
      if (i % 2 === 1) snare(x, t, r);
      bass(x, t + p / 2, p / 2, 55);
    }
  }
  return { name: 'EDM 128 BPM (intro, drop 16 s, breakdown, drop 64 s)', x, bpm, kicks, beats, drops: [16, 64], sections: { calmo: [[0, 16], [48, 64]], intenso: [[16, 48], [64, 96]] } };
}
function hiphop() {
  const r = rng(2), bpm = 92, p = 60 / bpm, dur = 40;
  const x = new Float32Array(dur * SR);
  const kicks = [], beats = [];
  for (let bar = 0; bar * 4 * p < dur; bar++) {
    const t0 = bar * 4 * p;
    for (const k of [0, 1.5, 2.5]) { const t = t0 + k * p; if (t < dur) { kick(x, t, 0.8); kicks.push(t); } }
    for (const sn of [1, 3]) if (t0 + sn * p < dur) snare(x, t0 + sn * p, r, 0.6);
    for (let e = 0; e < 8; e++) { const t = t0 + Math.floor(e / 2) * p + (e % 2 ? 0.66 * p : 0); if (t < dur) hat(x, t, r, 0.1); }
    for (let b = 0; b < 4; b++) if (t0 + b * p < dur) beats.push(t0 + b * p);
  }
  pad(x, 0, dur, 0.06);
  return { name: 'Hip-hop 92 BPM com swing', x, bpm, kicks, beats, drops: [], sections: { intenso: [[0, dur]] } };
}
function piano() {
  const r = rng(3), dur = 40;
  const x = new Float32Array(dur * SR);
  let t = 0.3; const notes = [];
  while (t < dur - 1.5) { note(x, t, 196 * Math.pow(2, Math.floor(r() * 15) / 12), 0.12 + r() * 0.1); notes.push(t); t += 0.25 + r() * 1.1; }
  return { name: 'Piano rubato (sem batida)', x, bpm: 0, kicks: notes, beats: [], drops: [], sections: { calmo: [[0, dur]] } };
}
function speech() {
  const r = rng(4), dur = 40;
  const x = new Float32Array(dur * SR);
  let t = 0.3; const syl = [];
  while (t < dur - 1) {
    const words = 2 + Math.floor(r() * 5);
    for (let w = 0; w < words && t < dur - 1; w++) {
      const n = 1 + Math.floor(r() * 3);
      for (let k = 0; k < n; k++) { const d = 0.08 + r() * 0.15; syllable(x, t, d, r); syl.push(t); t += d + 0.02; }
      t += 0.08 + r() * 0.12;
    }
    t += 0.35 + r() * 0.7;
  }
  return { name: 'Fala sintética (sílabas e pausas)', x, bpm: 0, kicks: syl, beats: [], drops: [], sections: { calmo: [[0, dur]] } };
}

// ---------- métricas ----------
function match(est, ref, tolA, tolB = tolA) {
  // Para cada referência, o evento estimado mais próximo dentro de [-tolA, +tolB].
  const used = new Set(); const offs = [];
  for (const rt of ref) {
    let best = -1, bd = Infinity;
    for (let i = 0; i < est.length; i++) {
      if (used.has(i)) continue;
      const d = est[i] - rt;
      if (d < -tolA || d > tolB) continue;
      if (Math.abs(d) < bd) { bd = Math.abs(d); best = i; }
    }
    if (best >= 0) { used.add(best); offs.push(est[best] - rt); }
  }
  const tp = offs.length;
  const prec = est.length ? tp / est.length : 0, rec = ref.length ? tp / ref.length : 0;
  const f = prec + rec ? (2 * prec * rec) / (prec + rec) : 0;
  const mean = offs.length ? offs.reduce((a, v) => a + v, 0) / offs.length : 0;
  return { f, prec, rec, meanMs: mean * 1000 };
}
const inRanges = (t, ranges) => ranges.some(([a, b]) => t >= a && t < b);
const fmt = (v, d = 2) => (Number.isFinite(v) ? v.toFixed(d) : '-');

for (const sc of [edm(), hiphop(), piano(), speech()]) {
  const tl = preanalyze(sc.x, SR);
  const live = runPlayback(sc.x, SR);
  const pre = runPlayback(sc.x, SR, { timeline: tl });
  console.log(`\n=== ${sc.name}`);
  console.log(`pré-análise: ${fmt(tl.analysisMs / 1000, 1)} s | BPM ${fmt(tl.bpm, 1)} (real ${sc.bpm || '-'}) | clareza ${fmt(tl.beatConfidence)}`);
  if (sc.beats.length) {
    const bt = tl.beats.filter((b) => b.strength > 0.05).map((b) => b.t);
    const m = match(bt, sc.beats, 0.07);
    console.log(`  batidas (±70 ms): F ${fmt(m.f)} | precisão ${fmt(m.prec)} | cobertura ${fmt(m.rec)} | desvio médio ${fmt(m.meanMs, 1)} ms`);
  }
  const ons = match(tl.onsets.map((o) => o.t), sc.kicks, 0.05);
  console.log(`  ataques (±50 ms): cobertura ${fmt(ons.rec)} | desvio médio ${fmt(ons.meanMs, 1)} ms`);
  console.log(`  drops: esperado [${sc.drops.join(', ')}] | pré ${JSON.stringify(tl.drops.map((d) => +d.t.toFixed(2)))} | ao vivo ${JSON.stringify(live.drops)}`);
  for (const [name, ranges] of Object.entries(sc.sections)) {
    const eP = pre.rows.filter((r) => inRanges(r.t, ranges)).map((r) => r.E);
    const eL = live.rows.filter((r) => inRanges(r.t, ranges)).map((r) => r.E);
    const avg = (a) => a.reduce((s, v) => s + v, 0) / Math.max(1, a.length);
    console.log(`  energia média no trecho ${name}: pré ${fmt(avg(eP))} | ao vivo ${fmt(avg(eL))}`);
  }
  // Pulsos visuais vs. eventos reais (o que o olho vê contra o que o ouvido ouve).
  const ref = sc.kicks;
  for (const [label, res] of [['pré', pre], ['ao vivo', live]]) {
    const pts = res.pulses.filter(([, a]) => a > 0.08).map(([t]) => t);
    const m = match(pts, ref, 0.05, 0.15);
    console.log(`  pulsos ${label.padEnd(7)}: ${pts.length} | acertam ${fmt(m.rec)} dos ataques reais | precisão ${fmt(m.prec)} | atraso médio ${fmt(m.meanMs, 0)} ms`);
  }
}
