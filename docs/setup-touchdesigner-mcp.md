# Setup TouchDesigner + MCP (TCC)

## O que já está pronto
- Componentes MCP em `Desktop\tcc-audiovisual\touchdesigner-mcp\`
  - `mcp_webserver_base.tox` + pasta `modules/` (não mover nem separar)
- Pacote npm `touchdesigner-mcp-server@2.2.0` pré-baixado
- Servidor MCP adicionado em `~/.cursor/mcp.json` (porta `9981`)
- Arquivo de exemplo copiado: `Desktop\tcc-audiovisual\Spectral Audio Reactive.toe`

## Passo 1 — Instalar o TouchDesigner
1. Crie uma conta grátis em https://derivative.ca (Non-Commercial).
2. Baixe o **Web Installer** Windows 2025.33230 (~778 MB) em https://derivative.ca/download
3. Instale normalmente.
4. Na primeira abertura, faça login com a conta Derivative e ative a key Non-Commercial.

Limite Non-Commercial: resolução máxima 1280×1280. Para o TCC / laboratório de shaders, basta.

## Passo 2 — Abrir o projeto com o componente MCP
1. Abra o TouchDesigner.
2. File → Open → `Desktop\tcc-audiovisual\Spectral Audio Reactive.toe`
3. No Network Editor (área de nós), arraste o arquivo:
   `Desktop\tcc-audiovisual\touchdesigner-mcp\mcp_webserver_base.tox`
   para dentro de `/project1` (idealmente ficando em `/project1/mcp_webserver_base`).
4. **Não** mova a pasta `modules/` nem o `.tox` para fora dessa estrutura.
5. Abra o Textport (`Alt + T`) e confirme algo como "server started" / porta `9981`.
6. Salve o `.toe` (Ctrl+S) para manter o componente na próxima abertura.

## Passo 3 — Conectar o Cursor
1. Reinicie o Cursor (ou toggle o servidor `touchdesigner` em Settings → MCP).
2. Com o TouchDesigner **aberto** e o `.tox` carregado, o MCP deve listar tools.
3. Me avise que está pronto: eu inspeciono a rede do Spectral (operadores, parâmetros, GLSL) e documento o port para o app.

## Troubleshooting
| Sintoma | Ação |
| --- | --- |
| MCP tools não aparecem | TD aberto? tox carregado? porta 9981 livre? reiniciar Cursor |
| Erro de versão API | Use o `touchdesigner-mcp-td` da release v2.2.0 (já baixado) |
| Tox com erro vermelho | Confirme que `modules/` está ao lado do `.tox` |
| Porta em uso | Feche outra instância do TD ou altere a porta no tox e no `mcp.json` |

## Comando de verificação (PowerShell)
```powershell
Test-NetConnection 127.0.0.1 -Port 9981
```
Se `TcpTestSucceeded : True`, o webserver do TD está no ar.
