// Simulador de uma faixa: roda a pré-análise (modo arquivo) e a simulação ao vivo (Modo Escuta)
// sobre um arquivo de áudio, com os mesmos módulos do navegador. Requer ffmpeg no PATH.
// Uso: node tools/simulate.mjs <audio>
import { decode, preanalyze, runPlayback } from './lib/sim.mjs';

const file = process.argv[2];
if (!file) { console.log('uso: node tools/simulate.mjs <audio>'); process.exit(1); }
const pcm = decode(file);
const tl = preanalyze(pcm);
const live = runPlayback(pcm);
const pre = runPlayback(pcm, undefined, { timeline: tl });
const q = (arr, p) => { const v = [...arr].sort((a, b) => a - b); return v[Math.min(v.length - 1, Math.floor(p * v.length))] ?? 0; };
const fmt = (v) => v.toFixed(2);
console.log(file);
console.log(`duração ${tl.duration.toFixed(0)} s | pré-análise em ${(tl.analysisMs / 1000).toFixed(1)} s | ${tl.bpm.toFixed(1)} BPM | clareza do pulso ${fmt(tl.beatConfidence)}`);
console.log(`batidas ${tl.beats.length} | ataques ${tl.onsets.length} | seções ${tl.sections.length}`);
console.log(`drops (pré)    : ${tl.drops.map((d) => `${d.t.toFixed(1)}s(${fmt(d.mag)})`).join(' ') || '-'}`);
console.log(`drops (ao vivo): ${live.drops.map((d) => `${d[0]}s(${fmt(d[1])})`).join(' ') || '-'}`);
for (const [name, r] of [['pré', pre], ['ao vivo', live]]) {
  const E = r.rows.map((x) => x.E);
  console.log(`energia ${name.padEnd(7)} p10/p50/p90: ${fmt(q(E, 0.1))} / ${fmt(q(E, 0.5))} / ${fmt(q(E, 0.9))} | pulsos ${r.pulses.length}`);
}
console.log('seções:', tl.sections.map((s) => `${s.t0.toFixed(0)}s:${fmt(s.energy)}`).join('  '));
