// Calibração em lote: roda a pré-análise (e opcionalmente a simulação ao vivo) em todo o corpus
// (tools/corpus.json + corpus/extra.txt com faixas locais "caminho|gênero") e grava JSON para os gráficos.
// Uso: node tools/calibrate.mjs [--live] [--only id1,id2]
import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { decode, preanalyze, runPlayback } from './lib/sim.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const live = args.includes('--live');
const onlyIdx = args.indexOf('--only');
const only = onlyIdx >= 0 ? args[onlyIdx + 1].split(',') : null;
const out = join(root, 'corpus', '_out');
mkdirSync(out, { recursive: true });

const list = [];
const { tracks } = JSON.parse(readFileSync(join(root, 'tools', 'corpus.json'), 'utf8'));
for (const t of tracks) {
  const dir = join(root, 'corpus', t.id);
  if (!existsSync(dir)) continue;
  const f = readdirSync(dir).find((x) => /\.(mp3|ogg|flac|wav)$/i.test(x));
  if (f) list.push({ id: t.id, genre: t.genre, file: join(dir, f), expect: t.expect });
}
const extra = join(root, 'corpus', 'extra.txt');
if (existsSync(extra)) {
  for (const line of readFileSync(extra, 'utf8').split(/\r?\n/)) {
    const [file, genre] = line.split('|');
    if (file && existsSync(file)) list.push({ id: 'local-' + genre.replace(/[^a-z0-9]+/gi, '-').toLowerCase(), genre, file });
  }
}

const q = (arr, p) => { const v = Array.from(arr).sort((a, b) => a - b); return v[Math.min(v.length - 1, Math.floor(p * v.length))] ?? 0; };
const rows = [];
for (const tr of list) {
  if (only && !only.some((o) => tr.id.includes(o))) continue;
  const pcm = decode(tr.file);
  const tl = preanalyze(pcm);
  const audible = Array.from(tl.energy).filter((_, k) => tl.debug.lufs[k] > -60);
  const dump = {
    id: tr.id, genre: tr.genre, bpm: tl.bpm, beatConfidence: tl.beatConfidence, duration: tl.duration, fps: tl.fps,
    energy: Array.from(tl.energy), raw: Array.from(tl.raw), loud: Array.from(tl.loud), act: Array.from(tl.act),
    beatConf: Array.from(tl.beatConf), build: Array.from(tl.build), lufs: Array.from(tl.debug.lufs),
    D: Object.fromEntries(Object.entries(tl.debug.D).map(([k, v]) => [k, Array.from(v)])),
    R: Object.fromEntries(Object.entries(tl.debug.R).map(([k, v]) => [k, Array.from(v)])),
    beats: tl.beats, onsets: tl.onsets.map((o) => [o.t, o.strength]), sections: tl.sections, drops: tl.drops,
  };
  let liveInfo = '';
  if (live) {
    const r = runPlayback(pcm);
    dump.live = { t: r.rows.map((x) => x.t), E: r.rows.map((x) => x.E), pulses: r.pulses, drops: r.drops };
    liveInfo = ` | aoVivo E p50 ${q(dump.live.E, 0.5).toFixed(2)}`;
  }
  writeFileSync(join(out, tr.id + '.json'), JSON.stringify(dump));
  const row = {
    id: tr.id, genre: tr.genre, bpm: tl.bpm.toFixed(0), conf: tl.beatConfidence.toFixed(2),
    E10: q(audible, 0.1).toFixed(2), E50: q(audible, 0.5).toFixed(2), E90: q(audible, 0.9).toFixed(2),
    secs: tl.sections.length, drops: tl.drops.map((d) => `${d.t.toFixed(1)}(${d.mag.toFixed(2)})`).join(' '),
    expect: tr.expect ? `${tr.expect.energy.join('-')}${tr.expect.beat ? ' batida' : ''}` : '',
    ms: Math.round(tl.analysisMs),
  };
  rows.push(row);
  console.log(`${row.id.padEnd(28)} bpm ${row.bpm.padStart(3)} conf ${row.conf} | E p10/50/90 ${row.E10} ${row.E50} ${row.E90} | seções ${String(row.secs).padStart(2)} | drops ${row.drops || '-'}${liveInfo} | esperado ${row.expect}`);
}
writeFileSync(join(out, 'summary.json'), JSON.stringify(rows, null, 1));
