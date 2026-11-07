# Port do "Spectral Audio Reactive.toe" para WebGL

Original: Pablo Torri (@pbltrr), TouchDesigner 2023.12370. Analisado via `touchdesigner-mcp` (2.2.0).
Implementação: `preview/spectralTD.js` (template 4 da prévia).

## Rede original

```
audiofilein1 -> audioAnalysis (palette) -> select1 [smsd]
                                              |
noise2 (offset = smsd) --+                    v
noise1 (tx = t/23) ------+--> remap1 --> geo1 (instancing 256x256 = 65.536)
                                          line1 (0.0003) como geometria
render1 (cam1) -> bloom1 -> rgbkey1 -> out1 (1280x1280)
```

## Equivalência de operadores

| TouchDesigner | Parâmetros relevantes | WebGL (prévia) |
| --- | --- | --- |
| `noise1` Noise TOP | perlin4d, period 0.4, harmon 4, spread 2.6, gain 0.64, exp 1.89, amp 1.09, offset 0.679, `tx = absTime.seconds/23` | fbm Perlin 4 oitavas + `sign(x)*|x|^1.89`, `uDrift = t/23` |
| `noise2` Noise TOP | period 0.38, spread 3.0, gain 0.31, exp 1.59, amp 0.13, `offset = smsd` | fbm Perlin + `uOffset` (áudio) |
| `remap1` Remap TOP | horz = blue, vert = red, extend hold | `noise1(clamp(vec2(n2.b, n2.r)))` |
| `geo1` instancing | tx = r, ty = g, cor = rgb | `gl.POINTS` com `texelFetch` por `gl_VertexID` |
| `cam1` | t = (0.711, 0.899, 3.550), FOV 45° | `lookAt` + `perspective` (+ órbita opcional) |
| `bloom1` | blacklevel 0.01, gamma 2.63, brightness 1.02, fill 0.735, intensity 1.08 | pré-filtro + mip chain (dual Kawase) |
| `audioAnalysis.smsd` | soma do Audio Spectrum, gauss 0.5 s, média 10 s, faixa 100–1800 | EMA 0.25 s (rápida) + EMA 5 s (lenta) |

## Calibração (dados exportados pelo MCP)

| Métrica | TouchDesigner | Port |
| --- | --- | --- |
| `noise1` mediana / p5–p95 | 0.679 / 0.26–1.10 | 0.679 / 0.29–1.06 |
| `noise2` mediana / p5–p95 | 0.514 / 0.502–0.525 | 0.514 / 0.503–0.524 |
| autocorrelação `noise1` = 0.5 | ~5 px | ~5 px (fator `FREQ = 4.4`) |

Ferramenta: `window.__spectralTD.stats(modo, offset)` no console da prévia
(`1` = noise1, `2` = noise2, `0` = posições após o remap).

## Comportamento observado
- O `noise2` varia só ±0.02 em torno de `smsd`: o remap lê uma janela de ~6% do `noise1`
  e a amplia nas 65 mil partículas, o que produz o "tecido" dobrado com filamentos.
- O áudio original reage devagar (média de 10 s). A prévia adiciona "Reatividade rápida",
  que mistura a energia de curto prazo e as batidas para deixar o visual mais responsivo.

## Camada de energia (extensão, não existe no .toe)

O original fica preso ao centro e reage devagar. A prévia adiciona uma camada espacial:

| Sinal de áudio | Efeito visual | Controle |
| --- | --- | --- |
| Seção (nível de ~1.5 s vs. piso/teto da própria música) | nuvem abre até ~2x, ondas de fluxo mais fortes | Expansão com a energia |
| Kick (spectral flux dos graves < 250 Hz) | impulso numa mola amortecida, alternando lados; giro que balança | Movimento pela tela |
| Kick | explosão radial (partículas aleatórias vão mais longe) | Expansão com a energia |
| Médios | aumenta a amplitude do `noise2` (mais dobras) | Distorção |
| Agudos | tremor fino das partículas | Expansão com a energia |

Detecção de kick: o método antigo ("grave acima da média") falhava em hardstyle porque o grave é
sustentado (0–0.9 kicks/s detectados). Com spectral flux num analyser sem suavização, a faixa de
teste ("All The Things She Said", hardstyle) passou a 3.0–3.5 ataques/s (kick + reverse bass).

## Diretor automático (`preview/director.js`)

Conduz os parâmetros sozinho a partir da música, sem preset manual:

- **Energia (0–1)** = 40% ritmo (kicks/s nos últimos 4 s) + 35% grave + 25% volume da seção,
  os dois últimos relativos ao piso/teto da própria faixa.
- **Aquecimento:** nos primeiros 25 s a energia é limitada (começa em 45% do teto), para o visual
  começar contido e crescer com a música. Sobe com constante de 3 s e desce com 5 s.
- **Cores:** calma = branco, agitada = azul, energética = vermelho (transição contínua, com histerese no rótulo).
- **Parâmetros** interpolados entre calmo e energético: velocidade, intensidade, reatividade,
  expansão, movimento, escala, bloom, rastro, aberração e impacto do kick (zoom da câmera).
- **Drop:** salto de volume > 0.35 em 2.5 s gera estouro de partículas, flash e zoom.

Faixa de teste (hardstyle): calma aos 3 s (23%), agitada aos 10 s (49%), energética a partir de ~20 s (83–95%).

## Diferenças conhecidas
- O ruído não é bit-idêntico ao Perlin 4D do TD (implementação interna). A forma é a mesma
  em escala e distribuição, mas não pixel a pixel.
- Os pontos têm no mínimo 1 px; em resolução baixa ficam mais granulados que as linhas AA do TD.
