// Baixa o corpus de calibração (tools/corpus.json) para corpus/<id>/. Só baixa o que falta.
// Uso: node tools/fetch_corpus.mjs
import { readFileSync, mkdirSync, existsSync, writeFileSync, statSync } from 'node:fs';
import { dirname, join, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const { tracks } = JSON.parse(readFileSync(join(root, 'tools', 'corpus.json'), 'utf8'));
for (const t of tracks) {
  const dir = join(root, 'corpus', t.id);
  const file = join(dir, decodeURIComponent(basename(new URL(t.url).pathname)));
  if (existsSync(file) && statSync(file).size > 10000) { console.log('ok      ', t.id); continue; }
  mkdirSync(dir, { recursive: true });
  const res = await fetch(t.url, { redirect: 'follow' });
  if (!res.ok) { console.log('ERRO    ', t.id, res.status); continue; }
  const buf = Buffer.from(await res.arrayBuffer());
  writeFileSync(file, buf);
  console.log('baixado ', t.id, (buf.length / 1e6).toFixed(1) + ' MB');
}
