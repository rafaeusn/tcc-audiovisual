# Diretor audio-reativo: arquitetura, calibração e testes

Documento técnico do diretor automático (template Spectral TD, mas a lógica serve para qualquer
template). Gerado a partir da varredura de calibração de 07/11/2026.

## 1. Arquitetura

```
arquivo ──► decodeAudioData ──► Worker: preanalysis.js ──► timeline (batidas, seções, drops, curvas)
   │                                                            │
   └──► <audio> ──► AnalyserNode ──► features.js ──► rhythm.js ─┴─► director.js ──► spectralTD.js
                                    (tempo real)   (tempo real)     (modo arquivo ou ao vivo)
```

| Módulo | Papel |
| --- | --- |
| `preview/features.js` | Descritores em tempo real a partir dos bytes do AnalyserNode (bandas, dBFS, fluxo por faixa). Puro. |
| `preview/rhythm.js` | Ataques em 3 faixas, andamento e "groove" por autocorrelação, peso por alinhamento à grade. Tempo real. |
| `preview/preanalysis.js` | Análise da música inteira (Worker): ataques, andamento, batidas por programação dinâmica, seções, energia, drops e subidas. |
| `preview/director.js` | Converte tudo em parâmetros visuais. **Modo arquivo** (lê a timeline no instante exato) e **modo ao vivo** (Modo Escuta). |
| `preview/analysis.worker.js` | Roda a pré-análise fora da thread de renderização. |

**Sincronia no modo arquivo:** o instante usado é
`audio.currentTime − outputLatency − baseLatency + 1 quadro + ajuste manual` (slider "Ajuste de sincronia (ms)",
útil para fones Bluetooth). Os eventos (batida, ataque, drop) disparam no quadro em que o instante os ultrapassa.

## 2. Pré-análise (modo arquivo)

1. **Espectrograma**: FFT 2048 (4096 acima de 50 kHz), passo de 10 ms, 64 bandas logarítmicas (30 Hz-16 kHz).
2. **Fluxo espectral (SuperFlux simplificado)**: diferença log contra o quadro de 20 ms atrás com filtro de
   máximo em frequência (reduz falsos ataques por vibrato de voz). Faixas: grave < 200 Hz, médio < 2,5 kHz, agudo.
3. **Ataques**: picos locais (±30 ms) acima da média local (−100/+70 ms) + limiar, separação mínima de 50 ms.
4. **Andamento**: autocorrelação do envelope de ataques com prior log-normal centrado em 120 BPM.
5. **Batidas**: programação dinâmica (Ellis 2007, mesmo princípio do librosa), tightness 100.
6. **Clareza do pulso**: autocorrelação local (janela de 8 s) no período da batida. Fala, rubato e pads ≈ 0-0,1;
   música marcada 0,4-0,8. Batidas só geram pump onde há clareza.
7. **Tempo forte**: fase do compasso (4/4) com mais grave + ataque.
8. **Energia** (10 Hz), com descritores escalados pela distribuição do corpus:

| Descritor | Medida | Escala (0 → 1) |
| --- | --- | --- |
| volume | loudness K-weighted (LUFS, mono, janela 400 ms) | −30 → −8 LUFS |
| pulso | clareza do pulso | 0,08 → 0,6 |
| andamento | BPM (dobrado/dividido para 70-180) × pulso | 80 → 160 BPM |
| ruído | flatness espectral (guitarra distorcida, saws) | 0,03 → 0,3 |
| cheio | fração de bandas a menos de 20 dB do pico | 0,3 → 0,8 |
| fluxo | média do fluxo espectral | 0,01 → 0,09 |
| dinâmica relativa | LUFS do trecho − mediana da faixa | ±10 dB → ±1 |
| continuidade | fração sem pausas (fala tem pausas) | multiplica 0,55 → 1 |

`a = 0,30·volume + (0,15 + 0,15·andamento)·pulso·drive + 0,15·ruído + 0,10·cheio + 0,10·fluxo`
(`drive` = volume suficiente: piano baixinho com acompanhamento regular continua calmo),
`a ×= continuidade`, `A = 0,14 + 0,86·clamp(a + 0,22·dinâmica)`.

9. **Seções**: novidade (diferença de médias de ±4 s dos descritores), fronteiras ajustadas ao tempo forte.
   Energia final = degrau por seção (peso maior quanto mais clara a batida) + curva contínua suavizada sem atraso.
   Subidas de energia são imediatas na fronteira; descidas em ~0,8 s.
10. **Drops**: fronteira com salto de energia ≥ 0,15, nova seção ≥ 0,55, com batida e volume. Instante exato =
    ataque grave mais forte perto da fronteira. **Subida (build)**: rampa quadrática até o drop (máx. 16 s).

## 3. Modo ao vivo (Modo Escuta)

Mesma receita com descritores de tempo real (dBFS, groove, flatness e densidade do analyser).
Pesos calibrados para seguir a energia da pré-análise ("professor") no corpus. Drops exigem um breakdown
real (grave lento < 35% do típico por 2 s, depois retorno) ou salto de 6 dB do volume de ~1 s sobre o
mínimo dos últimos 3 s.

## 4. Corpus de calibração

Baixado por `node tools/fetch_corpus.mjs` (só Creative Commons / domínio público; ver `tools/corpus.json`)
+ faixas locais em `corpus/extra.txt` (`caminho|gênero`, não versionadas).

| Faixa | Gênero | Licença |
| --- | --- | --- |
| Um Apólogo (LibriVox) | voz (fala) | domínio público |
| Chopin, Waltz Op. 69 No. 1 (Musopen) | piano solo | domínio público |
| Beethoven, Sinfonia 5 III (Musopen) | orquestra | CC BY-SA 3.0 |
| Kevin MacLeod, Night on the Docks | jazz lento | CC BY 3.0 |
| Josh Woodward, Go | pop/folk | CC BY 3.0 |
| the Transisters, Under Control | indie rock | CC BY-NC-SA 2.0 |
| ProleteR, April Showers | hip-hop | CC BY-NC-ND 3.0 |
| Nine Inch Nails, Discipline | rock industrial | CC BY-NC-SA 3.0 |
| effort, Drop It Like It's Not | eletrônica | CC BY-NC-ND 2.5 |
| 2 hardstyle + Lloyd (locais) | hardstyle, R&B | uso local, não versionadas |

## 5. Resultados

Energia (pré-análise, trechos audíveis) e concordância do modo ao vivo:

| Faixa | BPM | Clareza | Energia p10 / p50 / p90 | Drops (pré) | Ao vivo p50 | Correlação ao vivo × pré |
| --- | --- | --- | --- | --- | --- | --- |
| voz | — | 0,00 | 0,12 / 0,24 / 0,38 | — | 0,22 | 0,00 (faixa estreita, erro 0,10) |
| piano | — | 0,00 | 0,13 / 0,18 / 0,34 | — | 0,20 | 0,24 (erro 0,08) |
| orquestra | — | 0,00 | 0,14 / 0,20 / 0,55 | — | 0,16 | 0,69 |
| jazz lento | — | 0,00 | 0,13 / 0,19 / 0,31 | — | 0,26 | 0,40 |
| pop/folk | 112 | 1,00 | 0,31 / 0,71 / 0,80 | 1 | 0,70 | 0,76 |
| indie rock | 157 | 1,00 | 0,87 / 0,90 / 0,95 | 3 | 0,68 | 0,41 |
| hip-hop | 94 | 1,00 | 0,36 / 0,63 / 0,66 | 2 fracos | 0,58 | 0,70 |
| rock industrial | 122 | 1,00 | 0,56 / 0,80 / 0,90 | 2 | 0,70 | 0,46 |
| eletrônica | 92 | 0,96 | 0,45 / 0,58 / 0,63 | 1 fraco | 0,57 | 0,50 |
| hardstyle (ATTSS) | 170 | 1,00 | 0,46 / 0,76 / 0,91 | 5 | 0,70 | 0,70 |
| hardstyle (Outside) | 150 | 1,00 | 0,46 / 0,83 / 0,91 | 4 | 0,82 | 0,58 |
| R&B (Lloyd) | 121 | 0,97 | 0,22 / 0,60 / 0,65 | 1 fraco | 0,50 | 0,84 |

Testes sintéticos com tempos conhecidos (`node tools/synth_test.mjs`):

| Cenário | BPM detectado | Batidas (F, ±70 ms) | Drops | Pulsos pré: acerto / atraso | Pulsos ao vivo: acerto / atraso |
| --- | --- | --- | --- | --- | --- |
| EDM 128 (intro, drop, breakdown, drop) | 127,9 | 0,99 | 16 s e 64 s (exatos) | 100% / 5 ms | 79% / 42 ms |
| Hip-hop 92 com swing | 92,1 | 1,00 | nenhum (correto) | 98% / 4 ms | 93% / 25 ms |
| Piano rubato | — (clareza 0,02) | — | nenhum | 93% / 11 ms | 98% / 32 ms |
| Fala sintética | — (clareza 0,00) | — | nenhum | 61% / 22 ms | 65% / 45 ms |

Pré-análise: ~2-3 s para uma faixa de 4 min (Node e navegador).

## 6. Como reproduzir

```bash
node tools/fetch_corpus.mjs                 # baixa o corpus CC
node tools/calibrate.mjs --live             # pré-análise + ao vivo em todas as faixas
python tools/plot_calibration.py corpus/_out corpus/_out/calibracao.png
node tools/synth_test.mjs                   # sincronia com tempos conhecidos
node tools/simulate.mjs caminho/musica.mp3  # uma faixa
```

## 7. Limitações conhecidas

- Andamento ambíguo em dobro/metade (ex.: eletrônica a 92 ou 184 BPM); a energia usa o andamento dobrado/dividido
  para 70-180, então o efeito é pequeno.
- Compasso assumido 4/4 para o tempo forte.
- O modo ao vivo não sabe o futuro: transições e drops chegam com atraso de ~1 s e os pulsos têm ~25-45 ms de atraso.
- A calibração usa julgamento de faixas esperadas por gênero, não anotação humana. Próximo passo para o TCC: um teste
  com usuários (ex.: notas de "sincronia" e "imersão" por trecho) para validar os pesos.
