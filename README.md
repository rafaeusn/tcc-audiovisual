# Player Audiovisual (TCC)

Prévia e laboratório de um player musical com templates GLSL reativos à música (estilo Spotify + Last.fm + visualizador artístico).

## Prévia (já rodável)

```powershell
cd preview
python serve.py
```

Abra http://127.0.0.1:5500 — template **Spectral TD** + **Diretor automático** ligados por padrão.

## Documentação

- [`CONTEXTO-TCC.md`](CONTEXTO-TCC.md) — handoff completo do projeto (enviar ao próximo agente)
- [`docs/setup-touchdesigner-mcp.md`](docs/setup-touchdesigner-mcp.md) — TouchDesigner + MCP
- [`docs/port-spectral-td.md`](docs/port-spectral-td.md) — port do `.toe` → WebGL

## Estrutura

| Pasta | Conteúdo |
| --- | --- |
| `preview/` | Prévia WebGL (player, shaders, diretor) |
| `docs/` | Setup e notas técnicas |
| `touchdesigner-mcp/` | Componente MCP do TouchDesigner |
| `Spectral Audio Reactive.toe` | Shader original (laboratório) |
