// Web Worker da pré-análise: roda preanalysis.js fora da thread de renderização.
import { analyzeTrack } from './preanalysis.js';

self.onmessage = (e) => {
  const { id, pcm, sampleRate } = e.data;
  try {
    let last = 0;
    const tl = analyzeTrack(pcm, sampleRate, {
      onProgress: (p) => {
        if (p - last >= 0.05 || p === 1) { last = p; self.postMessage({ id, progress: p }); }
      },
    });
    delete tl.debug; // curvas de depuração só servem para a calibração offline
    self.postMessage({ id, timeline: tl });
  } catch (err) {
    self.postMessage({ id, error: String(err && err.message || err) });
  }
};
