// Ritmo agnóstico de fonte: detecta ataques (onsets) em três faixas e mede quão periódico é o pulso.
// Funciona para bumbo eletrônico, caixa de bateria, violão dedilhado, consoantes de voz ou palmas:
// cada faixa de frequência é comparada com a própria média recente ("surpresa"), então não depende
// de volume nem de gênero. Sem BPM fixo: a periodicidade é medida por autocorrelação.

const GRID_HZ = 50;                 // resolução do envelope de ataques para a autocorrelação
const GRID_N = 8 * GRID_HZ;         // 8 s de histórico
const LAG_MIN = Math.round(0.25 * GRID_HZ);  // 240 BPM
const LAG_MAX = Math.round(1.1 * GRID_HZ);   // ~55 BPM
const WEIGHT = { low: 1.0, mid: 0.85, high: 0.6 };
const PEAK_RATIO = 2.4;             // pico precisa ser 2.4x a média recente da faixa
const REFRACTORY = 0.13;            // s entre ataques

const clamp01 = (x) => Math.min(1, Math.max(0, x));
const smoothstep = (a, b, x) => {
  const t = clamp01((x - a) / (b - a));
  return t * t * (3 - 2 * t);
};

export class Rhythm {
  constructor() {
    this.reset();
  }

  reset() {
    this.t = 0;
    this.mean = { low: 0.004, mid: 0.004, high: 0.004 };
    this.env = 0;
    this.e1 = 0;
    this.e2 = 0;
    this.lastEvent = -10;
    this.events = [];          // [tempo, força]
    this.grid = new Float32Array(GRID_N);
    this.gridPos = 0;
    this.gridT = 0;
    this.acT = 0;
    this.groove = 0;
    this.bpm = 0;
    this.density = 0;          // ataques/s ponderados pela força, janela de 3 s
    this.coherence = 0;        // quão bem os ataques se alinham a uma grade de batidas (0-1)
    this.onBeat = 0;           // 1 no quadro em que um ataque cai na batida
    this.lastBand = 'low';     // faixa dominante do último ataque (low/mid/high)
    this.b1 = 'low';
  }

  // Devolve a força (0-1) se um ataque acabou de ser confirmado, senão 0.
  push(dt, bands) {
    this.t += dt;
    const t = this.t;
    const k = 1 - Math.exp(-dt / 3);
    let env = 0, band = 'low';
    for (const b of ['low', 'mid', 'high']) {
      const f = bands['flux' + b[0].toUpperCase() + b.slice(1)];
      const m = this.mean[b];
      const surprise = (f / (m + 0.0015)) * WEIGHT[b];
      if (surprise > env) { env = surprise; band = b; }
      // Linha de base robusta: os próprios picos (bumbo a cada batida) entram limitados a 1.5x a média,
      // senão um bumbo regular vira "o normal" e deixa de ser detectado.
      this.mean[b] = m + (Math.min(f, 1.5 * m + 0.002) - m) * k;
    }
    // Quase silêncio: não há ataque.
    if (bands.level < 0.03) env = 0;
    this.env = env;

    // Envelope em grade fixa (máximo por célula) para a autocorrelação.
    this.gridT += dt;
    const cell = 1 / GRID_HZ;
    this.curCell = Math.max(this.curCell || 0, Math.min(8, env));
    while (this.gridT >= cell) {
      this.gridT -= cell;
      this.grid[this.gridPos] = this.curCell;
      this.gridPos = (this.gridPos + 1) % GRID_N;
      this.curCell = 0;
    }

    // Pico: o quadro anterior (e1) é máximo local, com um quadro de atraso (~16 ms).
    let strength = 0;
    if (this.e1 >= PEAK_RATIO && this.e1 >= this.e2 && this.e1 > env && t - this.lastEvent > REFRACTORY) {
      strength = clamp01((this.e1 - 1.8) / 3);
      this.lastEvent = t;
      strength *= this.alignWeight(t, strength);
      this.events.push([t, strength]);
      this.lastBand = this.b1;
    }
    this.e2 = this.e1;
    this.e1 = env;
    this.b1 = band;

    while (this.events.length && t - this.events[0][0] > 3) this.events.shift();
    let sum = 0;
    for (const e of this.events) sum += 0.4 + 0.6 * e[1];
    this.density = sum / Math.min(3, Math.max(1, t));

    this.acT += dt;
    if (this.acT >= 0.25) {
      this.acT = 0;
      this.autocorr();
    }
    return strength;
  }

  // Ataques que caem na grade de batidas (fase média dos ataques recentes, no período medido)
  // valem mais; subdivisões e ruído valem menos. Sem pulso regular, todos valem igual.
  alignWeight(t, strength) {
    this.onBeat = 0;
    const T = this.bpm > 0 ? 60 / this.bpm : 0;
    if (!T || this.groove < 0.2) { this.coherence *= 0.9; return 1; }
    let cx = 0, cy = 0, sum = 0;
    for (const [te, se] of this.events) {
      if (t - te > 6) continue;
      const a = 2 * Math.PI * ((te / T) % 1);
      cx += se * Math.cos(a); cy += se * Math.sin(a); sum += se;
    }
    if (sum < 0.5) return 1;
    const phi = Math.atan2(cy, cx) / (2 * Math.PI);
    const c = Math.hypot(cx, cy) / sum;
    this.coherence += (c - this.coherence) * 0.3;
    const dist = (u) => Math.abs(((u % 1) + 1.5) % 1 - 0.5);
    const ph = (t / T) % 1;
    const dBeat = dist(ph - phi);
    const dHalf = dist(ph - phi - 0.5);
    const align = Math.max(1 - smoothstep(0.08, 0.22, dBeat), 0.5 * (1 - smoothstep(0.06, 0.16, dHalf)));
    if (dBeat < 0.12) this.onBeat = 1;
    const trust = smoothstep(0.25, 0.6, this.coherence);
    return 0.35 + 0.65 * (1 - trust + trust * align);
  }

  autocorr() {
    const n = Math.min(GRID_N, Math.floor(this.t * GRID_HZ));
    if (n < LAG_MAX * 2) return;
    // Série ordenada no tempo (do mais antigo ao mais novo).
    const x = new Float32Array(n);
    let mean = 0;
    for (let i = 0; i < n; i++) {
      x[i] = this.grid[(this.gridPos - n + i + GRID_N * 2) % GRID_N];
      mean += x[i];
    }
    mean /= n;
    let r0 = 0;
    for (let i = 0; i < n; i++) { x[i] -= mean; r0 += x[i] * x[i]; }
    if (r0 < 1e-3) { this.groove += (0 - this.groove) * 0.3; return; }
    let best = 0, bestLag = 0;
    const r = new Float32Array(LAG_MAX + 2);
    for (let lag = LAG_MIN - 1; lag <= LAG_MAX + 1; lag++) {
      let s = 0;
      for (let i = lag; i < n; i++) s += x[i] * x[i - lag];
      r[lag] = s / r0 * (n / (n - lag));
    }
    for (let lag = LAG_MIN; lag <= LAG_MAX; lag++) {
      // Só máximos locais contam como candidatos a período.
      if (r[lag] > r[lag - 1] && r[lag] >= r[lag + 1] && r[lag] > best) { best = r[lag]; bestLag = lag; }
    }
    // Se o dobro do período também é forte, prefere o menor lag (pulso mais rápido). Já é o caso por varredura crescente.
    const g = clamp01(best);
    this.groove += (g - this.groove) * 0.35;
    if (bestLag) {
      const a = r[bestLag - 1], b = r[bestLag], c = r[bestLag + 1];
      const d = a - 2 * b + c;
      const off = d !== 0 ? (0.5 * (a - c)) / d : 0;
      const lag = bestLag + Math.max(-0.5, Math.min(0.5, off));
      const bpm = 60 * GRID_HZ / lag;
      this.bpm = this.bpm ? this.bpm + (bpm - this.bpm) * 0.3 : bpm;
    }
  }
}
