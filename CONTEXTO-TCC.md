# Contexto completo — Player Audiovisual (TCC)

Documento de handoff para continuar o desenvolvimento no **Claude Code Desktop** (ou outro agente).
Atualizado em 07/11/2026. Responda sempre em **pt-BR**.

**Repositório GitHub:** `https://github.com/rafaeusn/tcc-audiovisual`  
**Clone local:** `C:\Users\rafae\Desktop\tcc-audiovisual\`

### Como o próximo agente deve começar
1. Ler este arquivo inteiro + `docs/port-spectral-td.md`.
2. Rodar a prévia (`preview/serve.py`) e validar template 4 + diretor.
3. Seguir a seção 7 (próximos passos) — prioridade: scaffold Vite/React/TS migrando `preview/`.
4. Não reinventar o que já está em `spectralTD.js`, `director.js`, `audio.js`.

---

## 1. O que é o projeto

App desktop (web-first) híbrido: **player nativo + modo Escuta**, com templates GLSL reativos à música, estilo Spotify + Last.fm + visualizador artístico.

**Problema que resolve:** a pessoa ouve música em um app e vê arte reativa em outro. Falta uma camada de experiência audiovisual pessoal, artística e memorável, sem exigir ser VJ e sem abandonar as músicas que já escuta.

**Posicionamento:** não reinventar o streaming. Ser a camada artística e estatística em cima da escuta — tocando o que for legal/nativo, escutando o resto no sistema, transformando cada faixa em cena visual pessoal.

**Assinatura (visão de produto):** templates/presets, matcher de biblioteca, pré-análise (BPM/seções/drop), stats estilo Last.fm + “templates mais usados”, modo Escuta, packs curados feitos no TouchDesigner. A assinatura vende experiência em cima da música, não o catálogo.

**Atualização 07/11/2026 — novo modelo de produto:** dois apps com o mesmo núcleo Rust. O **Player** é grátis e tem
módulos de serviço (Spotify estilo Spotifast, SoundCloud, Last.fm, YT Music). O **Studio** é pago (editor audiovisual
com linha do tempo e exportação de vídeo, templates de câmera/corpo) e usa só o Modo Escuta e arquivos importados.
O Player é **Modo Escuta primeiro**: não pede conexão com nenhum serviço, e os módulos de serviço são baixados à
parte numa loja dentro do app (estilo Marketplace do Spicetify). Desktop Windows primeiro. Framework de UI: protótipo comparativo (`prototypes/ui-bench`, resultados em
[`docs/prototipo-ui-resultados.md`](docs/prototipo-ui-resultados.md)) decidiu por **GPUI**:
visual sem cópia via camada DirectComposition + skins KDL recarregadas ao vivo (144 fps, ~14% de CPU). Detalhes, termos do Spotify (Developer Policy III.6/III.13/IV.2) e stack em
[`docs/pesquisa-referencias-app.md`](docs/pesquisa-referencias-app.md).

---

## 2. Decisões de arquitetura já tomadas

### Plataformas
- **Agora:** Web/desktop (React + Vite + TypeScript, empacotar depois com Electron/Tauri).
- **Mobile:** futuro (modo Player viável; modo Escuta limitado/impossível no iOS).

### Fontes de música (híbrido)
| Fonte | Papel |
| --- | --- |
| Arquivos locais | Principal (custo zero de storage) |
| Audius API | Streaming legal, catálogo independente |
| Jamendo API | Creative Commons, uso não comercial |
| Cofre privado R2 (opcional) | Upload do usuário, só dele |
| Modo Escuta (loopback) | Spotify/YT/etc. tocando fora; captura áudio do SO |
| Last.fm / export Spotify / Takeout | Só **metadados** (gosto, playlists, histórico) |

**Importante:** Spotify/YouTube **não** entregam áudio para tocar dentro do app (DRM / ToS). Desde fev/2026 o Spotify Dev Mode exige Premium do dono e máx. 5 usuários. Preferir export de dados do Spotify + Last.fm.

**Matcher:** Track canônico → local → cofre → Audius → Jamendo → senão “ouvir no modo Escuta”.

**Regra de banco:** servidor guarda **só metadados**, nunca áudio (exceto cofre opcional com cota).

### Stack da prévia atual (já implementada)
- Vanilla JS + WebGL2 (sem React ainda).
- `preview/serve.py` — HTTP local sem cache (`no-store`), porta **5500**.
- TouchDesigner 2025.33230 + MCP `touchdesigner-mcp-server@2.2.0` (porta **9981**).

### Stack planejada (ainda não)
- Electron (loopback `setDisplayMediaRequestHandler`) ou Tauri + cpal.
- React + Vite + TypeScript + Three.js.
- Supabase (auth, tracks, presets, listens).
- Análise: AnalyserNode + AudioWorklet; Meyda; essentia.js (BPM/seções).
- Helper SMTC (Windows) para “tocando agora”.

---

## 3. O que já existe no disco

Raiz: `C:\Users\rafae\Desktop\tcc-audiovisual\`

```
tcc-audiovisual/
├── CONTEXTO-TCC.md          ← este arquivo
├── Spectral Audio Reactive.toe   ← shader original (Pablo Torri @pbltrr)
├── preview/                 ← prévia WebGL rodável
│   ├── index.html
│   ├── main.js              ← UI, loop, templates, diretor
│   ├── audio.js             ← player, captura, FFT, beat via spectral flux
│   ├── shaders.js           ← templates 1–3 (Spectral anel, Fluxo, Barras)
│   ├── spectralTD.js        ← template 4: port do .toe
│   ├── director.js          ← diretor automático (energia, pulso, drop → params/cores)
│   ├── features.js          ← descritores de áudio (puro, testável)
│   ├── rhythm.js            ← ataques, BPM, groove (tempo real)
│   ├── preanalysis.js       ← análise da faixa inteira (batidas, seções, drops)
│   ├── analysis.worker.js   ← Worker da pré-análise
│   └── serve.py             ← python serve.py → http://127.0.0.1:5500
├── docs/
│   ├── setup-touchdesigner-mcp.md
│   └── port-spectral-td.md  ← equivalência TD→WebGL + calibração + diretor
├── scripts/
│   └── verificar-mcp.ps1
└── touchdesigner-mcp/       ← mcp_webserver_base.tox + modules/ (NÃO separar)
```

### Como rodar a prévia
```powershell
cd C:\Users\rafae\Desktop\tcc-audiovisual\preview
python serve.py
# abrir http://127.0.0.1:5500
```
- Abrir MP3 / arrastar arquivo / Modo Escuta / Batida de teste.
- Atalhos: `H` esconde UI, `F` fullscreen, `Espaço` play/pause, `1–4` templates.
- Template padrão ao abrir: **4. Spectral TD** com **Diretor automático** ligado.

### MCP TouchDesigner (já configurado no Cursor)
Em `~/.cursor/mcp.json` há o server `touchdesigner`:
```json
"touchdesigner": {
  "command": "npx",
  "args": ["-y", "touchdesigner-mcp-server@latest"],
  "env": {
    "TD_WEB_SERVER_HOST": "http://127.0.0.1",
    "TD_WEB_SERVER_PORT": "9981"
  }
}
```
Para usar: abrir o `.toe`, ter `mcp_webserver_base` ativo na porta 9981, reiniciar Cursor se preciso. Namespace MCP: `user-touchdesigner`.

---

## 4. Template 4 — Spectral TD (port do .toe)

### Rede original (TouchDesigner)
```
audiofilein1 → audioAnalysis (palette) → select1 [smsd]
noise2 (offset = smsd) ─┐
noise1 (tx = t/23) ─────┴→ remap1 → geo1 instancing 256×256 (65.536 pontos)
line1 (0.0003) como geo
render1 (cam1) → bloom1 → rgbkey1 → out1
```

### Equivalência
| TD | WebGL |
| --- | --- |
| noise1/noise2 Perlin 4D | fbm Perlin 4 oitavas + `sign(x)*|x|^exp` |
| remap (horz=blue, vert=red) | `noise1(clamp(vec2(n2.b, n2.r)))` |
| geo instancing | `gl.POINTS` + `texelFetch` por `gl_VertexID` |
| bloom | pré-filtro + dual Kawase mip chain |
| smsd | EMA lenta + rápida (não a média de 10 s exata) |

Calibração via MCP (medianas/percentis/autocorr) documentada em `docs/port-spectral-td.md`. Fator espacial do period TD: `FREQ = 4.4`.

### Camada de energia espacial (extensão, NÃO existe no .toe)
O original fica no centro. A prévia adiciona:
- **Kick** → impulso em mola amortecida (esquerda/direita/cima/baixo) + giro + burst.
- **Seção energética** → escala ~2× + ondas de fluxo nas bordas.
- **Médios** → mais dobras no noise2.
- **Agudos** → jitter fino.

### Detecção de kick
Método antigo (grave acima da média) falhava em hardstyle (grave sustentado). Agora: **spectral flux** em analyser sem smoothing, foco &lt; 250 Hz → ~3.0–3.5 ataques/s na faixa de teste.

---

## 5. Diretor automático (`features.js`, `rhythm.js`, `preanalysis.js`, `director.js`)

Agnóstico de fonte (música, voz, instrumento). Detalhes, corpus, números e testes em **`docs/diretor-calibracao.md`**.

- **Modo arquivo (padrão ao abrir música):** a faixa inteira é pré-analisada num Worker (~2-3 s): batidas por
  programação dinâmica, clareza do pulso, seções, energia, drops e subidas. O diretor lê tudo no instante exato
  de reprodução (com compensação de latência de saída + slider "Ajuste de sincronia"). Pulsos acertam 98-100% das
  batidas sintéticas com ~5 ms.
- **Modo ao vivo (Modo Escuta):** mesma receita em tempo real, calibrada para seguir a pré-análise.
- **Energia:** volume (LUFS), clareza do pulso, andamento, ruído espectral, densidade, fluxo e dinâmica relativa à
  própria faixa. Cor em patamares: branco (calma) → azul (agitada) → vermelho (intensa), com flash no drop.
- **Visual:** forma maior também no calmo, morfose que acelera logo após cada batida, passeio no ritmo de 4 compassos,
  contração/tremor na subida e explosão no drop.
- **Ferramentas:** `tools/fetch_corpus.mjs` (corpus CC), `tools/calibrate.mjs --live`, `tools/plot_calibration.py`,
  `tools/synth_test.mjs`, `tools/simulate.mjs`. Sempre validar no corpus inteiro antes de mudar pesos.

## 6. História do chat (resumo cronológico)

1. Ideia de TCC: Spotify + Last.fm + visuals artísticos; dúvida de APIs e volume de DB.
2. Plano companion → depois **híbrido** (player nativo + Escuta + import metadados).
3. Prévia WebGL com 3 templates genéricos.
4. Instalação TouchDesigner + MCP; análise do `.toe` via MCP.
5. Port fiel do Spectral TD (template 4).
6. Feedback: “preso no centro” → camada espacial + spectral flux.
7. Feedback: mais imersão automática → **Diretor automático** (cores + aquecimento + drop).
8. Pedido atual: commit no GitHub + este `.md` de contexto.

---

## 7. O que falta (próximos passos sugeridos)

Prioridade sugerida para o próximo agente:

1. **Scaffold do app real:** Vite + React + TS; migrar a engine da prévia (`spectralTD`, `director`, `audio`) para módulos tipados.
2. **Electron:** player local + loopback (`setDisplayMediaRequestHandler` / `audio: 'loopback'`).
3. **Biblioteca local:** scan de pastas + `music-metadata` + SQLite.
4. **Adaptadores Audius + Jamendo** + interface `SourceAdapter`.
5. **Importadores:** Last.fm (username), export Spotify JSON, Google Takeout; Matcher.
6. **Supabase:** auth, tracks, track_presets, listens, stats.
7. **Mais templates** no TD (GLSL TOP) → port (pipeline MCP).
8. **Pré-análise** (BPM, beat grid, seções) para presets por música.
9. **Avaliação TCC:** FPS, latência áudio→visual, taxa do matcher, SUS.

### Não fazer
- Baixar áudio do YouTube/Spotify (yt-dlp etc.) — viola ToS.
- Embutir runtime TouchDesigner no app (não existe; TD é só laboratório).
- Espalhar pasta `touchdesigner-mcp/modules/` (paths relativos do tox).

---

## 8. Referências do usuário

- `.toe`: `Spectral Audio Reactive.toe` (também em Desktop).
- Pinterest (estética imersiva / Alex Guevara GLSL): pins citados no chat (`596304806953362463`, `596304806953362498`).
- Faixa de teste usada (não versionar áudio): hardstyle *All The Things She Said*.

---

## 9. Convenções

- Idioma com o usuário: **sempre pt-BR**.
- Commits: conventional commits (`feat`, `fix`, `chore`, `docs`…).
- Não commitar MP3, secrets, `__pycache__`, zip do MCP.
- Prévia deve continuar rodável com `python serve.py` até o app React existir.

---

## 10. Comandos úteis

```powershell
# Prévia
cd C:\Users\rafae\Desktop\tcc-audiovisual\preview
python serve.py

# Verificar MCP TD
.\scripts\verificar-mcp.ps1
Test-NetConnection 127.0.0.1 -Port 9981
```
