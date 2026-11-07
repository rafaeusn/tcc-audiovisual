import { Features, BINS } from './features.js';

export class AudioEngine {
  constructor() {
    this.ctx = null;
    this.sensitivity = 1;
    this.spec = new Uint8Array(BINS * 2);
    this.features = new Features();
    this.bands = this.features.bands;
    this.source = 'none';
    this.captureStream = null;
    this.captureNode = null;
    this.demo = null;
  }

  ensure() {
    if (!this.ctx) {
      const ctx = (this.ctx = new AudioContext());
      this.analyser = ctx.createAnalyser();
      this.analyser.fftSize = 2048;
      this.analyser.smoothingTimeConstant = 0.72;
      this.analyser.minDecibels = -85;
      this.analyser.maxDecibels = -20;
      this.freq = new Uint8Array(this.analyser.frequencyBinCount);
      this.time = new Uint8Array(this.analyser.fftSize);

      // Analyser sem suavização temporal, só para detectar ataques (spectral flux).
      this.onsetAnalyser = ctx.createAnalyser();
      this.onsetAnalyser.fftSize = 1024;
      this.onsetAnalyser.smoothingTimeConstant = 0;
      this.onsetAnalyser.minDecibels = -85;
      this.onsetAnalyser.maxDecibels = -20;
      this.onsetFreq = new Uint8Array(this.onsetAnalyser.frequencyBinCount);
      this.onsetPrev = new Uint8Array(this.onsetAnalyser.frequencyBinCount);

      // Os analysers precisam estar ligados ao destino para serem processados, mas sem som (evita eco no modo Escuta).
      const silent = ctx.createGain();
      silent.gain.value = 0;
      this.analyser.connect(this.onsetAnalyser);
      this.onsetAnalyser.connect(silent);
      silent.connect(ctx.destination);

      this.out = ctx.createGain();
      this.out.gain.value = 0.8;
      this.out.connect(ctx.destination);

      this.el = new Audio();
      this.el.crossOrigin = 'anonymous';
      const elSrc = ctx.createMediaElementSource(this.el);
      elSrc.connect(this.analyser);
      elSrc.connect(this.out);

      this.features.configure(ctx.sampleRate, this.analyser.fftSize, this.onsetAnalyser.fftSize);
    }
    if (this.ctx.state === 'suspended') this.ctx.resume();
  }

  setVolume(v) {
    if (this.out) this.out.gain.value = v;
  }

  stopAll() {
    if (this.el) this.el.pause();
    if (this.captureStream) {
      this.captureStream.getTracks().forEach((t) => t.stop());
      this.captureNode?.disconnect();
      this.captureStream = null;
      this.captureNode = null;
    }
    if (this.demo) {
      clearInterval(this.demo.timer);
      this.demo.bus.disconnect();
      this.demo = null;
    }
    this.source = 'none';
  }

  playFile(file) {
    this.ensure();
    this.stopAll();
    if (this.el.src) URL.revokeObjectURL(this.el.src);
    this.el.src = URL.createObjectURL(file);
    this.el.play();
    this.source = 'file';
  }

  async capture() {
    this.ensure();
    const stream = await navigator.mediaDevices.getDisplayMedia({
      video: true,
      audio: { echoCancellation: false, noiseSuppression: false, autoGainControl: false },
      systemAudio: 'include',
      selfBrowserSurface: 'exclude',
    });
    const audioTracks = stream.getAudioTracks();
    if (!audioTracks.length) {
      stream.getTracks().forEach((t) => t.stop());
      throw new Error('Nenhum áudio foi compartilhado. Marque "Compartilhar áudio" na janela de seleção.');
    }
    this.stopAll();
    stream.getVideoTracks().forEach((t) => t.stop());
    this.captureStream = stream;
    this.captureNode = this.ctx.createMediaStreamSource(new MediaStream(audioTracks));
    this.captureNode.connect(this.analyser);
    this.source = 'capture';
    audioTracks[0].addEventListener('ended', () => {
      if (this.source === 'capture') this.stopAll();
      this.onCaptureEnded?.();
    });
    return audioTracks[0].label;
  }

  startDemo() {
    this.ensure();
    this.stopAll();
    const ctx = this.ctx;
    const bus = ctx.createGain();
    bus.gain.value = 0.55;
    bus.connect(this.analyser);
    bus.connect(this.out);

    const noise = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate);
    const data = noise.getChannelData(0);
    for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;

    const chords = [
      [220, 261.63, 329.63],
      [174.61, 220, 261.63],
      [196, 246.94, 293.66],
      [164.81, 207.65, 246.94],
    ];
    const spb = 60 / 110;
    let next = ctx.currentTime + 0.1;
    let step = 0;

    const env = (node, t, peak, attack, decay) => {
      node.gain.setValueAtTime(0.0001, t);
      node.gain.exponentialRampToValueAtTime(peak, t + attack);
      node.gain.exponentialRampToValueAtTime(0.0001, t + attack + decay);
    };
    const kick = (t) => {
      const o = ctx.createOscillator();
      const g = ctx.createGain();
      o.frequency.setValueAtTime(150, t);
      o.frequency.exponentialRampToValueAtTime(42, t + 0.12);
      env(g, t, 1, 0.003, 0.35);
      o.connect(g).connect(bus);
      o.start(t);
      o.stop(t + 0.4);
    };
    const noiseHit = (t, type, freq, peak, decay) => {
      const s = ctx.createBufferSource();
      s.buffer = noise;
      const f = ctx.createBiquadFilter();
      f.type = type;
      f.frequency.value = freq;
      const g = ctx.createGain();
      env(g, t, peak, 0.002, decay);
      s.connect(f).connect(g).connect(bus);
      s.start(t);
      s.stop(t + decay + 0.05);
    };
    const pad = (freqs, t, dur) => {
      const f = ctx.createBiquadFilter();
      f.type = 'lowpass';
      f.frequency.value = 1400;
      const g = ctx.createGain();
      g.gain.setValueAtTime(0.0001, t);
      g.gain.exponentialRampToValueAtTime(0.06, t + 0.25);
      g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
      f.connect(g).connect(bus);
      freqs.forEach((hz) => {
        const o = ctx.createOscillator();
        o.type = 'sawtooth';
        o.frequency.value = hz;
        o.connect(f);
        o.start(t);
        o.stop(t + dur + 0.05);
      });
    };
    const pluck = (hz, t) => {
      const o = ctx.createOscillator();
      o.type = 'triangle';
      o.frequency.value = hz;
      const g = ctx.createGain();
      env(g, t, 0.12, 0.004, 0.18);
      o.connect(g).connect(bus);
      o.start(t);
      o.stop(t + 0.25);
    };

    const schedule = (s, t) => {
      const chord = chords[Math.floor(s / 16) % chords.length];
      if (s % 4 === 0) kick(t);
      if (s % 8 === 4) noiseHit(t, 'bandpass', 1800, 0.6, 0.18);
      if (s % 2 === 1) noiseHit(t, 'highpass', 7000, 0.18, 0.05);
      if (s % 16 === 0) pad(chord, t, spb * 4);
      if (s % 2 === 0) pluck(chord[(s / 2) % 3] * 2, t);
    };

    const timer = setInterval(() => {
      while (next < ctx.currentTime + 0.2) {
        schedule(step, next);
        next += spb / 4;
        step++;
      }
    }, 40);
    this.demo = { timer, bus };
    this.source = 'demo';
  }

  update(dt) {
    if (!this.analyser) return;
    this.analyser.getByteFrequencyData(this.freq);
    this.analyser.getByteTimeDomainData(this.time);
    this.onsetAnalyser.getByteFrequencyData(this.onsetFreq);

    this.features.sensitivity = this.sensitivity;
    this.features.update(dt, this.freq, this.onsetFreq, this.time);

    this.spec.set(this.features.spec, 0);
    for (let i = 0; i < BINS; i++) {
      this.spec[BINS + i] = this.time[Math.floor((i / BINS) * this.time.length)];
    }
  }
}
