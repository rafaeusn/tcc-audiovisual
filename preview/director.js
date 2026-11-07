// Diretor automático agnóstico de fonte: música, voz, instrumento ou qualquer áudio.
// Não assume gênero, BPM nem kicks. Trabalha com três escalas de tempo:
//
//   1. Energia (lenta, ~1-2 s): "quão animado está este trecho". Mistura sinais ABSOLUTOS (volume em dBFS,
//      densidade espectral, regularidade do pulso), que dizem se a música é calma ou pesada em si,
//      com um termo RELATIVO (o trecho comparado à média recente da própria faixa), que exagera as seções:
//      breakdown fica bem mais fraco que a média, drop bem mais forte. Conduz cor, luz, distorção.
//   2. Pulso (rápido, por ataque): cada ataque detectado (bumbo, caixa, dedilhado, consoante) gera um "pump"
//      curto, proporcional à força do ataque e à energia. Conduz o estalo da forma, o brilho e o bloom.
//   3. Eventos (drop e build): salto de grave/volume sobre os últimos segundos (drop) e subida
//      contínua de energia (build).
//
// Dois modos:
//   - Arquivo (com pré-análise, setTimeline): energia, batidas, ataques, drops e subidas vêm da análise da
//     música inteira (preanalysis.js) e são lidos no instante exato de reprodução. Sincronia de quadro.
//   - Ao vivo (Modo Escuta, sem timeline): tudo é estimado em tempo real a partir do áudio que chega.

// Cor pela energia: calma = branco, agitada = azul, intensa = vermelho (transição contínua).
// Patamares nítidos: branco até ~0.3 (calma), azul entre ~0.55 e 0.75 (agitada), vermelho a partir de ~0.9.
const PALETTES = [
  { at: 0.0, a: [1, 1, 1], b: [0.87, 0.9, 1] },
  { at: 0.3, a: [1, 1, 1], b: [0.87, 0.9, 1] },
  { at: 0.55, a: [0.18, 0.42, 1], b: [0.56, 0.89, 1] },
  { at: 0.75, a: [0.18, 0.42, 1], b: [0.56, 0.89, 1] },
  { at: 0.92, a: [1, 0.1, 0.22], b: [1, 0.55, 0.22] },
];

// [calmo, intenso]. TD = só no template Spectral TD.
const RANGES = {
  speed: [0.85, 1.35],
  intensity: [0.85, 1.35],
  aberr: [0.15, 1.0],
  react: [0.65, 1.35, 'TD'],
  spread: [0.1, 0.45, 'TD'],
  motion: [0.25, 0.5, 'TD'],
  scale: [1.35, 1.8, 'TD'],  // maior também no calmo (antes ficava pequeno demais)
  bloom: [0.9, 1.6, 'TD'],
  feedback: [0.1, 0.38, 'TD'],
  punch: [0, 0.45, 'TD'],
  // Luz e textura: trecho calmo = luz fraca e pouca distorção; trecho animado = luz forte e mais dobras.
  exposure: [0.8, 2.3],
  warp: [0.25, 1.2],
};

// Modo ao vivo: escalas dos descritores em tempo real (diferentes das da pré-análise) e pesos.
// Calibrados para seguir a energia da pré-análise no corpus (tools/calibrate.mjs --live).
export const LIVE = {
  loudOffset: 28, pulseLo: 0.1, pulseHi: 0.5, noiseLo: 0.55, noiseHi: 0.95, fullLo: 0.35, fullHi: 0.75,
  wLoud: 0.32, wPulse: 0.15, wTempo: 0.15, wNoise: 0.15, wFull: 0.12, wRel: 0.22, floor: 0.14,
};

const clamp = (x, a, b) => Math.min(b, Math.max(a, x));
const clamp01 = (x) => clamp(x, 0, 1);
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

  // Pré-análise da faixa (ou null para voltar ao modo ao vivo).
  setTimeline(tl) {
    this.tl = tl || null;
    this.tlPrev = null;
    this.cur = { b: 0, o: 0, d: 0, lastDown: -1 };
  }

  reset() {
    this.tl = null;
    this.tlPrev = null;
    this.cur = { b: 0, o: 0, d: 0, lastDown: -1 };
    this.beatPhase = 0;     // 0-1 entre duas batidas
    this.barPhase = 0;      // 0-1 dentro do compasso (4 batidas)
    this.beatConf = 0;
    this.playTime = 0;
    this.gate = 0;          // 0 = silêncio, 1 = som
    // Energia
    this.A = 0;             // energia absoluta (suave)
    this.Af = 0;
    this.Amu = 0;           // média longa da energia absoluta
    this.intensity = 0.25;  // energia final (0-1): cor, luz, distorção
    this.loud = 0;          // volume rápido (0-1): amplitude da morfose
    this.activity = 0;      // atividade rápida (0-1): velocidade da morfose
    // Pulso
    this.pulse = 0;         // envelope do pump (0-1)
    this.impulse = 0;       // força acumulada de ataques ainda não consumidos pelo visual
    this.groove = 0;
    this.bpm = 0;
    this.kickRate = 0;      // ataques/s (informativo)
    // Eventos
    this.drop = 0;
    this.build = 0;
    this.bassFast = 0;
    this.bassMin = undefined;
    this.dbMin = undefined;
    this.dbHist = [];
    this.dbMu = undefined;
    this.dbFast = undefined;
    this.bassSlow = undefined;
    this.bassRef = undefined;
    this.lowBassTime = 0;
    this.dropLog = undefined;
    this.lastDrop = -99;
    this.Eslow = 0.25;
    // Outros
    this.bright = 0.4;
    this.tonal = 0.5;
    this.label = 'Calma';
    this.sens = 1.2;        // sensibilidade automática (ganho de entrada)
    this.peak = 0.3;
    this._level = 0;
  }

  // tMedia: instante da música que está sendo ouvido agora (segundos). Só é usado no modo arquivo.
  update(dt, bands, playing, rhythm, tMedia = null) {
    if (!playing) {
      this.gate += (0 - this.gate) * (1 - Math.exp(-dt * 2));
      this.pulse *= Math.exp(-dt * 6);
      return;
    }
    this.playTime += dt;
    const now = this.playTime;
    const ema = (v, x, tau) => v + (x - v) * (1 - Math.exp(-dt / tau));
    const r = rhythm || { strength: 0, groove: 0, bpm: 0, density: 0 };

    // Porta de ruído: abaixo de ~-50 dBFS é silêncio.
    const audible = bands.rmsDb > -50;
    this.gate = ema(this.gate, audible ? 1 : 0, audible ? 0.15 : 0.6);

    if (this.tl && tMedia != null) this.updateTimeline(dt, tMedia, ema);
    else this.updateLive(dt, bands, r, now, ema);

    // ---------- Ganho automático ----------
    // Mantém o pico recente do nível das bandas perto de 0.3 (visual consistente entre fontes).
    this._level = ema(this._level, bands.level, 0.5);
    this.peak = Math.max(this._level, this.peak - (this.peak - this._level) * dt / 40);
    if (this.gate > 0.5 && now > 2) {
      const want = this.sens * 0.3 / Math.max(0.08, this.peak);
      this.sens = clamp(ema(this.sens, want, lerp(3, 10, smoothstep(0, 20, now))), 0.45, 1.8);
    }

    this.bright = ema(this.bright, bands.centroid, 1.2);
    this.tonal = ema(this.tonal, 1 - bands.flatness, 1.5);

    const x = this.intensity;
    if (this.label === 'Calma' && x > 0.42) this.label = 'Ativa';
    else if (this.label === 'Ativa' && x > 0.74) this.label = 'Intensa';
    else if (this.label === 'Ativa' && x < 0.34) this.label = 'Calma';
    else if (this.label === 'Intensa' && x < 0.66) this.label = 'Ativa';
  }

  // ---------- Modo arquivo: lê a pré-análise no instante exato ----------
  updateTimeline(dt, t, ema) {
    const tl = this.tl;
    const kf = t * tl.fps, k0 = Math.floor(kf), fr = kf - k0;
    const at = (arr) => {
      if (k0 <= 0) return arr[0];
      if (k0 >= arr.length - 1) return arr[arr.length - 1];
      return arr[k0] + (arr[k0 + 1] - arr[k0]) * fr;
    };
    const E = at(tl.energy);
    // A curva já vem suavizada e alinhada às seções: só um filtro curto para não pular de quadro em quadro.
    this.intensity = ema(this.intensity, E, 0.12);
    this.loud = ema(this.loud, at(tl.loud) * this.gate, 0.08);
    this.activity = ema(this.activity, at(tl.act), 0.2);
    this.build = at(tl.build);
    this.beatConf = at(tl.beatConf);
    this.groove = this.beatConf;
    this.bpm = tl.bpm;
    this.kickRate = 0;

    const beats = tl.beats, onsets = tl.onsets, drops = tl.drops;
    const prev = this.tlPrev;
    this.tlPrev = t;
    const firstAfter = (arr, x) => {
      let lo = 0, hi = arr.length;
      while (lo < hi) { const m = (lo + hi) >> 1; if (arr[m].t <= x) lo = m + 1; else hi = m; }
      return lo;
    };
    if (prev == null || t < prev - 0.05 || t - prev > 1) {
      // Começo ou salto (seek): reposiciona sem disparar eventos atrasados.
      this.cur.b = firstAfter(beats, t);
      this.cur.o = firstAfter(onsets, t);
      this.cur.d = firstAfter(drops, t);
      this.cur.lastDown = -1;
      for (let i = this.cur.b - 1; i >= 0 && i >= this.cur.b - 4; i--) if (beats[i].downbeat) { this.cur.lastDown = i; break; }
    } else {
      const fire = (amp) => {
        amp *= this.gate;
        if (amp <= 0) return;
        this.pulse = Math.max(this.pulse, amp);
        this.impulse += amp;
      };
      while (this.cur.b < beats.length && beats[this.cur.b].t <= t) {
        const b = beats[this.cur.b];
        if (b.downbeat) this.cur.lastDown = this.cur.b;
        // Tempo forte do compasso pesa mais; o pump cresce com a energia do trecho.
        fire(b.strength * (b.downbeat ? 1 : 0.75) * (0.25 + 0.75 * this.intensity));
        this.cur.b++;
      }
      while (this.cur.o < onsets.length && onsets[this.cur.o].t <= t) {
        const o = onsets[this.cur.o];
        const nb = beats[Math.min(beats.length - 1, this.cur.b)], pb = beats[Math.max(0, this.cur.b - 1)];
        const dist = Math.min(Math.abs(o.t - (nb ? nb.t : 1e9)), Math.abs(o.t - (pb ? pb.t : 1e9)));
        const nearBeat = beats.length > 0 && dist < 0.07;
        // Sem batida clara (fala, piano, orquestra) os ataques conduzem o pump; com batida, só acentos fortes.
        if (!(nearBeat && this.beatConf > 0.3)) {
          // Com batida clara, só acentos grandes (prato, impacto) entre as batidas, para não poluir o pump.
          const w = this.beatConf < 0.4 ? 0.9 : o.strength > 0.8 ? 0.3 : 0;
          if (w > 0) fire(o.strength * (0.15 + 0.85 * this.intensity) * w);
        }
        this.cur.o++;
      }
      while (this.cur.d < drops.length && drops[this.cur.d].t <= t) {
        const d = drops[this.cur.d];
        this.drop = Math.max(this.drop, d.mag);
        fire(d.mag);
        (this.dropLog = this.dropLog || []).push([+d.t.toFixed(1), +d.mag.toFixed(2)]);
        this.cur.d++;
      }
    }
    this.pulse *= Math.exp(-dt / 0.15);
    this.drop *= Math.exp(-dt / 0.9);

    // Fase da batida e do compasso: o visual pode andar no tempo da música.
    const i = this.cur.b;
    if (i > 0 && i < beats.length) {
      const b0 = beats[i - 1].t, b1 = beats[i].t;
      this.beatPhase = clamp01((t - b0) / Math.max(0.05, b1 - b0));
      const sinceDown = this.cur.lastDown >= 0 ? (i - 1 - this.cur.lastDown) : 0;
      this.barPhase = ((sinceDown % 4) + this.beatPhase) / 4;
    } else {
      this.beatPhase = (t * (this.bpm || 120) / 60) % 1;
      this.barPhase = (t * (this.bpm || 120) / 240) % 1;
    }
  }

  // ---------- Modo ao vivo: estima tudo em tempo real ----------
  updateLive(dt, bands, r, now, ema) {
    // Mesma receita da pré-análise (preanalysis.js), com descritores que dão para medir em tempo real.
    // Os pesos foram ajustados para que a curva ao vivo acompanhe a da pré-análise no corpus
    // (tools/calibrate.mjs --live), que serve de referência.
    const L = LIVE;
    const loud01 = clamp01((bands.rmsDb + L.loudOffset) / 22);
    this.loud = ema(this.loud, loud01 * this.gate, 0.3);
    this.groove = r.groove;
    this.bpm = r.bpm;
    const pulse = smoothstep(L.pulseLo, L.pulseHi, r.groove);
    const bpmFold = r.bpm > 180 ? r.bpm / 2 : r.bpm < 70 ? r.bpm * 2 : r.bpm;
    const tempo01 = r.bpm ? smoothstep(80, 160, bpmFold) : 0;
    const noise = smoothstep(L.noiseLo, L.noiseHi, bands.flatness);
    const full = smoothstep(L.fullLo, L.fullHi, bands.fullness);
    const drive = smoothstep(0.1, 0.55, loud01);
    // Dinâmica relativa: volume comparado à média longa da própria fonte.
    if (!this.dbMu) this.dbMu = bands.rmsDb;
    this.dbMu = ema(this.dbMu, bands.rmsDb, 30);
    this.dbFast = ema(this.dbFast ?? bands.rmsDb, bands.rmsDb, 1.0);
    const rel = Math.max(-1, Math.min(1, (this.dbFast - this.dbMu) / 10)) * smoothstep(2, 12, now);
    const a = L.wLoud * loud01 + (L.wPulse + L.wTempo * tempo01) * pulse * drive + L.wNoise * noise + L.wFull * full;
    const Araw = L.floor + (1 - L.floor) * clamp01(a + L.wRel * rel);
    this.A = ema(this.A || Araw, Araw, 1.2);
    this.Af = ema(this.Af || Araw, Araw, 0.35);
    const target = this.A * this.gate;
    this.intensity = ema(this.intensity, target, target > this.intensity ? 0.8 : 1.6);
    this.Eslow = ema(this.Eslow, this.intensity, 5);

    const dens01 = smoothstep(1.0, 3.2, r.density);
    this.activity = ema(this.activity, (0.45 * this.intensity + 0.3 * dens01 + 0.25 * pulse) * this.gate, 0.4);

    // ---------- Pulso ----------
    // Bumbo e caixa (graves/médios) dão o pump; agudos (chimbal, sibilância) pesam pouco.
    this.kickRate = r.density;
    if (r.strength > 0) {
      const bandW = r.band === 'low' ? 1 : r.band === 'mid' ? 0.75 : 0.2;
      const amp = r.strength * bandW * (0.2 + 0.8 * this.intensity) * this.gate;
      if (amp > (r.band === 'high' ? 0.15 : 0.06)) {
        this.pulse = Math.max(this.pulse, amp);
        this.impulse += amp;
      }
    }
    this.pulse *= Math.exp(-dt / 0.15);

    // ---------- Drop ----------
    // Só depois de um trecho sem grave de verdade (breakdown): o grave lento fica abaixo de 35% do nível
    // típico da faixa por 2 s ou mais, e então o grave volta acima de 75% dele. Ou um salto de volume
    // de 6 dB sobre o mínimo dos últimos segundos. Força proporcional ao salto e à energia.
    const bassN = Math.min(1.5, bands.bass * 1.2 / this.sens);
    this.bassFast = ema(this.bassFast, bassN, 0.2);
    this.bassSlow = ema(this.bassSlow ?? bassN, bassN, 1.5);
    if (this.gate > 0.5) this.bassRef = ema(this.bassRef ?? bassN, bassN, 20);
    const ref = Math.max(0.15, this.bassRef ?? 0.3);
    this.lowBassTime = this.bassSlow < 0.35 * ref ? (this.lowBassTime || 0) + dt : Math.max(0, (this.lowBassTime || 0) - dt * 0.5);
    // Volume de ~1 s (não o instantâneo: o vaivém de bumbo e pausa não conta) contra o mínimo
    // dos últimos 3 s (janela deslizante a 10 Hz).
    this.dbHist = this.dbHist || [];
    if (!this.dbHist.length || now - this.dbHist[this.dbHist.length - 1][0] >= 0.1) this.dbHist.push([now, this.dbFast]);
    while (this.dbHist.length && now - this.dbHist[0][0] > 3) this.dbHist.shift();
    let dbMin = this.dbFast;
    for (const [, v] of this.dbHist) if (v < dbMin) dbMin = v;
    this.dbMin = dbMin;
    const dbJump = this.dbFast - dbMin;
    const magBass = this.lowBassTime > 2 && this.bassFast > 0.75 * ref ? clamp01(0.4 + 0.3 * (this.lowBassTime - 2)) : 0;
    const magDb = this.dbMin > -45 && dbJump > 6 ? clamp01((dbJump - 4) / 8) : 0;
    const mag = Math.max(magBass, magDb) * (0.3 + 0.7 * smoothstep(0.35, 0.75, this.Af));
    const liveWin = now - this.lastDrop < 0.6;
    if (mag > 0.2 && (liveWin || (now - this.lastDrop > 8 && now > 6)) && this.gate > 0.5) {
      this.drop = Math.max(this.drop, mag);
      if (!liveWin) {
        this.lastDrop = now;
        this.lowBassTime = 0;
        (this.dropLog = this.dropLog || []).push([+now.toFixed(1), +mag.toFixed(2), magBass > magDb ? 'grave' : 'volume', +this.bassSlow.toFixed(2), +ref.toFixed(2), +dbJump.toFixed(1)]);
      }
    }
    this.drop *= Math.exp(-dt / 0.9);
    this.build = clamp01(smoothstep(0.04, 0.22, this.intensity - this.Eslow) * (1 - this.drop));
    // Fase aproximada pelo andamento estimado (ao vivo não se sabe com exatidão onde cai a próxima batida).
    const bpm = this.bpm || 120;
    this.beatPhase = (now * bpm / 60) % 1;
    this.barPhase = (now * bpm / 240) % 1;
    this.beatConf = pulse;
  }

  telemetry() {
    return {
      E: this.intensity, A: this.A, loud: this.loud, act: this.activity,
      pulse: this.pulse, drop: this.drop, build: this.build, sens: this.sens, conf: this.beatConf,
      lbl: this.label === 'Calma' ? 0 : this.label === 'Ativa' ? 1 : 2,
    };
  }

  colors() {
    // No drop a cor dá um salto em direção ao vermelho (proporcional à força do drop) e volta.
    const x = clamp01(this.intensity + 0.35 * this.drop);
    let i = 0;
    while (i < PALETTES.length - 2 && x > PALETTES[i + 1].at) i++;
    const p0 = PALETTES[i], p1 = PALETTES[i + 1];
    const t = smoothstep(p0.at, p1.at, x);
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
    P.sensitivity = this.sens;
    [P.colA, P.colB] = this.colors();
    P.hueCycle = 0;
  }
}
