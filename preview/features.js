// Extração de descritores a partir dos dados de frequência (bytes de um AnalyserNode).
// Módulo puro: não toca em Web Audio, então roda igual no navegador (audio.js) e no simulador offline (tools/simulate.mjs).

export const BINS = 256;
const F_MIN = 30;
const F_MAX = 16000;

export class Features {
  constructor() {
    this.sensitivity = 1;
    this.spec = new Uint8Array(BINS);
    this.bands = {
      bass: 0, mid: 0, high: 0, level: 0, beat: 0, flux: 0, onset: 0,
      centroid: 0.4, flatness: 0.5,
      // Descritores independentes do ganho de entrada (sensibilidade):
      rmsDb: -60,   // volume RMS em dBFS (curto prazo, ~0.4 s)
      crest: 0.5,   // pico/RMS relativo: baixo = som comprimido e denso
      fullness: 0,  // fração do espectro com energia (0-1): denso = cheio
      // Fluxo espectral bruto por quadro (positivo, 0-1), segurado entre blocos repetidos:
      fluxLow: 0,   // < 250 Hz (bumbo, baixo)
      fluxMid: 0,   // 250 Hz - 2.5 kHz (voz, instrumentos, caixa)
      fluxHigh: 0,  // 2.5 - 10 kHz (hi-hats, consoantes, brilho)
    };
    this.t = 0;
    this.fluxAvg = null;
    this.lowFluxAvg = null;
    this.lastBeat = -10;
    this.onsetPrev = null;
  }

  // freqBins/onsetBins = frequencyBinCount dos analysers, fft = fftSize.
  configure(sampleRate, mainFft, onsetFft) {
    this.sampleRate = sampleRate;
    this.mainBinHz = sampleRate / mainFft;
    this.onsetBinHz = sampleRate / onsetFft;
    const maxBin = mainFft / 2 - 1;
    this.binMap = [];
    for (let i = 0; i < BINS; i++) {
      const f0 = F_MIN * Math.pow(F_MAX / F_MIN, i / BINS);
      const f1 = F_MIN * Math.pow(F_MAX / F_MIN, (i + 1) / BINS);
      const b0 = Math.min(maxBin, Math.floor(f0 / this.mainBinHz));
      const b1 = Math.min(maxBin, Math.max(b0 + 1, Math.ceil(f1 / this.mainBinHz)));
      this.binMap.push([b0, b1, Math.sqrt(f0 * f1)]);
    }
    this.onsetPrev = new Uint8Array(onsetFft / 2);
  }

  update(dt, freq, onsetFreq, time) {
    this.t += dt;
    const bands = this.bands;

    // Volume em dBFS, crest factor e densidade espectral: não dependem da sensibilidade.
    if (time) {
      let ss = 0, pk = 0;
      for (let i = 0; i < time.length; i++) {
        const v = (time[i] - 128) / 128;
        ss += v * v;
        if (Math.abs(v) > pk) pk = Math.abs(v);
      }
      const rms = Math.sqrt(ss / time.length);
      const db = 20 * Math.log10(Math.max(rms, 1e-5));
      const kr = 1 - Math.exp(-dt / 0.4);
      bands.rmsDb += (db - bands.rmsDb) * kr;
      if (rms > 0.005) bands.crest += (Math.min(1, (pk / rms - 1) / 5) - bands.crest) * kr;
    }
    {
      let full = 0;
      for (let i = 0; i < BINS; i++) {
        const [b0, b1] = this.binMap[i];
        let m = 0;
        for (let b = b0; b < b1; b++) if (freq[b] > m) m = freq[b];
        if (m > 128) full++;
      }
      bands.fullness += (full / BINS - bands.fullness) * (1 - Math.exp(-dt / 0.4));
    }

    // Espectro em bandas logarítmicas.
    let bass = 0, mid = 0, high = 0, all = 0;
    let nb = 0, nm = 0, nh = 0;
    for (let i = 0; i < BINS; i++) {
      const [b0, b1, hz] = this.binMap[i];
      let m = 0;
      for (let b = b0; b < b1; b++) if (freq[b] > m) m = freq[b];
      const tilt = 0.85 + 0.55 * (i / BINS);
      const v = Math.min(1, Math.pow(m / 255, 1.5) * tilt * this.sensitivity);
      this.spec[i] = v * 255;
      all += v;
      if (hz < 150) { bass += v; nb++; }
      else if (hz < 2000) { mid += v; nm++; }
      else if (hz < 12000) { high += v; nh++; }
    }

    const raw = { bass: bass / nb, mid: mid / nm, high: high / nh, level: all / BINS };
    for (const k of ['bass', 'mid', 'high', 'level']) {
      const rate = raw[k] > bands[k] ? 30 : 7;
      bands[k] += (raw[k] - bands[k]) * (1 - Math.exp(-dt * rate));
    }

    // Brilho (centróide) e tonalidade (flatness): só contam acima de um piso de energia.
    let wsum = 0, wpos = 0, lsum = 0;
    for (let i = 0; i < BINS; i++) {
      const v = this.spec[i] / 255 + 1e-3;
      wsum += v;
      wpos += v * (i / (BINS - 1));
      lsum += Math.log(v);
    }
    if (wsum / BINS > 0.02) {
      const kd = 1 - Math.exp(-dt * 3);
      const flat = Math.exp(lsum / BINS) / (wsum / BINS);
      bands.centroid += (wpos / wsum - bands.centroid) * kd;
      bands.flatness += (Math.min(1, flat) - bands.flatness) * kd;
    }

    // Fluxo espectral (aumentos de energia entre blocos) em três faixas.
    // Blocos repetidos (nenhuma mudança nos bytes) não contam: seguram o último valor.
    const n = onsetFreq.length;
    const prev = this.onsetPrev;
    let changed = 0, fl = 0, fm = 0, fh = 0, nLow = 0, nMid = 0, nHigh = 0;
    for (let b = 1; b < n; b++) {
      const hz = b * this.onsetBinHz;
      if (hz >= 10000) break;
      const d = onsetFreq[b] - prev[b];
      if (d !== 0) changed++;
      prev[b] = onsetFreq[b];
      const band = hz < 250 ? 0 : hz < 2500 ? 1 : 2;
      if (band === 0) nLow++; else if (band === 1) nMid++; else nHigh++;
      if (d > 0) {
        if (band === 0) fl += d; else if (band === 1) fm += d; else fh += d;
      }
    }
    if (changed > 0) {
      bands.fluxLow = fl / (Math.max(1, nLow) * 255);
      bands.fluxMid = fm / (Math.max(1, nMid) * 255);
      bands.fluxHigh = fh / (Math.max(1, nHigh) * 255);
    }
    const low = bands.fluxLow;
    const flux = 0.6 * low + 0.4 * (fl + fm + fh) / (Math.max(1, nLow + nMid + nHigh) * 255);
    const k = 1 - Math.exp(-dt * 2.5);
    this.fluxAvg = (this.fluxAvg ?? flux) + (flux - (this.fluxAvg ?? flux)) * k;
    this.lowFluxAvg = (this.lowFluxAvg ?? low) + (low - (this.lowFluxAvg ?? low)) * k;
    bands.flux = Math.min(1, flux / (this.fluxAvg * 3 + 1e-4));
    bands.onset += (flux - bands.onset) * (1 - Math.exp(-dt / 0.15));

    // Batida simples (grave): usada só pelos templates 1-3.
    if (low > this.lowFluxAvg * 1.8 + 0.01 && this.t - this.lastBeat > 0.28) {
      bands.beat = 1;
      this.lastBeat = this.t;
    }
    bands.beat *= Math.exp(-dt * 5);
  }
}
