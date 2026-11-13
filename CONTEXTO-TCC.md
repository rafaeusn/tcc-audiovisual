# Contexto completo — Player Audiovisual (TCC)

Documento de handoff para continuar o desenvolvimento no **Claude Code Desktop** (ou outro agente).
Atualizado em 07/11/2026. Responda sempre em **pt-BR**.

**Repositório GitHub:** `https://github.com/rafaeusn/tcc-audiovisual`  
**Clone local:** `C:\Users\rafae\Desktop\tcc-audiovisual\`

### Como o próximo agente deve começar
0. **Ler primeiro o [`AGENTS.md`](AGENTS.md)**: regras, ambiente, arquitetura, estado atual e próximas fases do app
   em Rust (atualizado em 13/11/2026). Este arquivo guarda a história e as decisões de produto.
1. Ler este arquivo inteiro + `docs/plano-v1.md` + `docs/port-spectral-td.md`.
2. Rodar a prévia (`preview/serve.py`) e validar template 4 + diretor.
3. Seguir o **plano da v1** (`docs/plano-v1.md`): app em Rust + GPUI, Windows, fases 0–6.
4. Não reinventar o que já está em `spectralTD.js`, `director.js`, `audio.js` nem no protótipo `prototypes/ui-bench`:
   eles são a base do port e o gabarito dos testes de paridade.
5. Para ver ou mudar o design da UI: o mockup interativo (v4) está em `mockup/`, com prévia local e regras em
   `mockup/README.md`; as decisões estão em `docs/design-ui-v1.md` e `docs/repertorio-ui.md`. A versão publicada é um
   artefato do claude.ai, que só o Claude Code atualiza.

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
- **Agora:** desktop Windows, app nativo em **Rust + GPUI** (decidido em 07/11/2026; ver `docs/plano-v1.md`).
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

### Stack do app (Rust, ver `docs/plano-v1.md`)
- UI: GPUI 0.2.2 + visual wgpu numa camada DirectComposition (sem cópia) + skins KDL + temas TOML.
- Áudio: WASAPI loopback do sistema e por processo, `cpal` + `symphonia` no player local.
- Análise: port de `features/rhythm/preanalysis/director.js` para Rust (`realfft`, `rayon`).
- SMTC (Windows) para "tocando agora" e controles; SQLite local; módulos da loja em WASM (`wasmtime`).
- Sem servidor na v1 (Supabase e contas ficam para depois).

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

A lista antiga (React/Electron/Supabase) foi substituída pelo **plano da v1** em `docs/plano-v1.md` (08/11/2026):

0. Fundação: workspace `app/`, módulos do protótipo, CI.
1. Casca da UI: tela padrão (estilo Spotify com o visual no centro), skins KDL v2, comandos, modo edição.
2. Modo Escuta: captura por processo, SMTC, diretor ao vivo em Rust.
3. Spectral em wgpu: cores reativas/próprias, modo seguro para fotossensibilidade com medidor de flashes.
4. Player local: biblioteca SQLite, pré-análise em Rust com cache, sincronia.
5. Loja: catálogo assinado, temas/skins/layouts/pacotes, perfis de Escuta (Spotify, YT Music…), módulo Last.fm (WASM).
6. Acabamento e avaliação TCC: instalador, medições, SUS.

Depois da v1: Audius/Jamendo, modo para surdos, mais templates (pipeline TD → MCP), Studio.

### Estado do app em Rust (13/11/2026)

O app está em `app/` (ver `app/README.md`). Rodar com `cargo run -p player` dentro de `app/`. Já tem:

- Modo Escuta com o som do sistema ou de um app só (captura por processo, escolhida nos Ajustes), "tocando
  agora" via SMTC (filtrado pelo mesmo app), diretor ao vivo.
- Três visuais em wgpu na camada de baixo: Spectral TD (512x512 partículas), Fluxo e Barras; modo seguro.
- Interface GPUI responsiva (largo, gaveta, empilhado), 4 temas, skin padrão e imersiva, fontes do mockup.
- Painel em módulos (Ritmo, Energia, Estrutura, Letra, Nesta sessão, Visual, Relógio) com modo edição (E).
- Histórico local em SQLite (`crates/historico`, thread própria) e a página Biblioteca com as estatísticas.
- Módulo Letra (`crates/letras`): busca no LRCLIB, guarda no banco, acompanha a posição informada pelo SMTC.
- Player de arquivos locais (`crates/tocador`, `crates/biblioteca`, `player/src/local.rs`): aba Músicas da
  Biblioteca com pastas como categorias, "Todas" sem repetir, busca e fila; a faixa é publicada no controle de
  mídia do Windows.
- Integração com o Windows (`crates/integracao`): botões na miniatura da barra de tarefas, bandeja (com a opção
  de continuar rodando ao fechar), janela mini (N) e relatório de diagnóstico nos Ajustes.
- Barra de comandos (Ctrl K), Loja (temas, skins, layouts, pacotes; módulos baixáveis como "Em breve"),
  volume do sistema no dock, micro-interações do artefato "Micro-interações do Player".

Dos passos acima: o 0 está feito menos a CI; o 1 está feito menos as skins KDL carregadas de arquivo; o 2 está
feito; o 3 falta o medidor de flashes; 4, 5 (download e WASM) e 6 não começaram. O Spectral do
mockup (`mockup/project/Main.dc.html`) ainda é o antigo: falta portar o Spectral TD para lá.

A varredura de 12 repositórios de streaming (o que reaproveitar, o que não, e a ordem sugerida) está em
`docs/pesquisa-repos-streaming.md`. Dela já saíram o histórico, a letra, a integração com o Windows e o formato
dos módulos da Loja (`crates/modulos`: manifesto, ações com motivo, aceite, bloqueio). Os módulos Last.fm e
Discord têm código (`crates/servicos`), mas ainda não foram vistos funcionando de ponta a ponta: faltam as chaves
(`LASTFM_API_KEY`, `LASTFM_API_SECRET`, `DISCORD_APP_ID`, só por variável de ambiente). O ListenBrainz saiu do
plano a pedido do Rafael (12/11/2026).

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
- Prévia deve continuar rodável com `python serve.py`: ela é a referência (gabarito) dos testes de paridade do app Rust.

---

## 10. Comandos úteis

```powershell
# Prévia
cd C:\Users\rafae\Desktop\tcc-audiovisual\preview
python serve.py

# App em Rust
cd C:\Users\rafae\Desktop\tcc-audiovisual\app
cargo run -p player

# Verificar MCP TD
.\scripts\verificar-mcp.ps1
Test-NetConnection 127.0.0.1 -Port 9981
```
