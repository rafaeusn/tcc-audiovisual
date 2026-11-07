// Diretor automático: lê o ritmo grave e a energia da música e conduz os parâmetros visuais.
// Começa contido (aquecimento) e sobe conforme a música: calma = branco, agitada = azul, energética = vermelho.

const PALETTES = [
  { at: 0.0, a: [1, 1, 1], b: [0.87, 0.9, 1] },
  { at: 0.5, a: [0.18, 0.42, 1], b: [0.56, 0.89, 1] },
  { at: 0.85, a: [1, 0.1, 0.22], b: [1, 0.55, 0.22] },
];

// [calmo, energético]. TD = só no template Spectral TD.
const RANGES = {
  speed: [0.45, 1.9],
  intensity: [0.8, 1.4],
  aberr: [0.2, 1.3],
  react: [0.25, 1.5, 'TD'],
  spread: [0.35, 1.4, 'TD'],
  motion: [0.25, 1.5, 'TD'],
  scale: [1.05, 1.4, 'TD'],
  bloom: [0.9, 1.7, 'TD'],
  feedback: [0.15, 0.45, 'TD'],
  punch: [0, 1, 'TD'],
};

const WARMUP_SECONDS = 25;

const clamp01 = (x) => Math.min(1, Math.max(0, x));
const lerp = (a, b, t) => a + (b - a) * t;
const smoothstep = (a, b, x) => {
  const t = clamp01((x - a) / (b - a));
  return t * t * (3 - 2 * t);
};
const toHex = (c) => '#' + c.map((v) => Math.round(clamp01(v) * 255).toString(16).padStart(2, '0')).join('');

export const DIRECTED_KEYS = Object.keys(RANGES);

export class Director {
  constructor() {
    this.reset();
  }

  reset() {
    this.playTime = 0;
    this.intensity = 0;
    this.section = 0;
    this.floor = 1;
    this.ceil = 0;
    this.bass = 0;
    this.bassFloor = 1;
    this.bassCeil = 0;
    this.onsets = [];
    this.lastBeat = 0;
    this.kickRate = 0;
    this.drop = 0;
    this.loudHist = [];
    this.label = 'Calma';
  }

  update(dt, bands, playing) {
    if (!playing) return;
    this.playTime += dt;
    const now = this.playTime;
    const ema = (v, x, tau) => v + (x - v) * (1 - Math.exp(-dt / tau));

    // Seção e grave, cada um comparado ao piso/teto da própria música.
    this.section = ema(this.section, bands.level, 1.5);
    this.floor = Math.min(this.section, this.floor + dt * 0.004);
    this.ceil = Math.max(this.section, this.ceil - dt * 0.004);
    const loud = clamp01(0.4 * (this.section - 0.15) / 0.45
      + 0.6 * (this.section - this.floor) / Math.max(0.08, this.ceil - this.floor));

    this.bass = ema(this.bass, bands.bass, 1);
    this.bassFloor = Math.min(this.bass, this.bassFloor + dt * 0.004);
    this.bassCeil = Math.max(this.bass, this.bassCeil - dt * 0.004);
    const bassE = 0.5 * clamp01((this.bass - 0.2) / 0.5)
      + 0.5 * clamp01((this.bass - this.bassFloor) / Math.max(0.08, this.bassCeil - this.bassFloor));

    // Ritmo: kicks nos últimos 4 s (0.5/s = calmo, 3/s = pulsação máxima).
    if (bands.beat > this.lastBeat + 0.3) this.onsets.push(now);
    this.lastBeat = bands.beat;
    while (this.onsets.length && now - this.onsets[0] > 4) this.onsets.shift();
    this.kickRate = this.onsets.length / Math.min(4, Math.max(1, now));
    const rhythm = clamp01((this.kickRate - 0.5) / 2.5);

    const raw = 0.4 * rhythm + 0.35 * bassE + 0.25 * loud;
    const warm = smoothstep(0, WARMUP_SECONDS, now);
    const target = raw * (0.45 + 0.55 * warm);
    this.intensity = ema(this.intensity, target, target > this.intensity ? 3 : 5);

    // Drop: salto forte de energia em ~2.5 s.
    this.loudHist.push([now, loud]);
    while (this.loudHist.length && now - this.loudHist[0][0] > 2.5) this.loudHist.shift();
    if (loud - this.loudHist[0][1] > 0.35 && this.drop < 0.1 && now > 8) this.drop = 1;
    this.drop *= Math.exp(-dt * 1.2);

    const x = this.intensity;
    if (this.label === 'Calma' && x > 0.42) this.label = 'Agitada';
    else if (this.label === 'Agitada' && x > 0.72) this.label = 'Energética';
    else if (this.label === 'Agitada' && x < 0.36) this.label = 'Calma';
    else if (this.label === 'Energética' && x < 0.66) this.label = 'Agitada';
    this.warm = warm;
  }

  colors() {
    const x = this.intensity;
    let i = 0;
    while (i < PALETTES.length - 2 && x > PALETTES[i + 1].at) i++;
    const p0 = PALETTES[i], p1 = PALETTES[i + 1];
    const t = smoothstep(p0.at + (p1.at - p0.at) * 0.35, p1.at, x);
    return [
      toHex(p0.a.map((v, k) => lerp(v, p1.a[k], t))),
      toHex(p0.b.map((v, k) => lerp(v, p1.b[k], t))),
    ];
  }

  apply(P, isTD) {
    for (const [key, [calm, hot, only]] of Object.entries(RANGES)) {
      if (only === 'TD' && !isTD) continue;
      P[key] = lerp(calm, hot, this.intensity);
    }
    [P.colA, P.colB] = this.colors();
    P.hueCycle = 0;
  }
}
