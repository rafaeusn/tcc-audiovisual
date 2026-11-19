# Player Audiovisual

Um player de música para Windows que **mostra a música**: um visual artístico em tempo real, conduzido por uma análise
do som (batidas, compasso, energia, drops), escrito em Rust.

- **Modo Escuta:** ouve o que o computador está tocando (Spotify, navegador, qualquer app) e move o visual ao vivo.
  Lê o "tocando agora" pelo controle de mídia do Windows.
- **Player local:** toca as músicas das suas pastas (MP3, FLAC, WAV, OGG, M4A), sem intervalo entre faixas, com
  crossfade, volume igualado e equalizador. Cada música é **pré-analisada**: o visual sabe onde estão as batidas, o
  tempo forte e os drops antes de eles acontecerem.
- **Em volta:** painel em módulos (ritmo, energia, letra...), histórico e estatísticas no seu computador, letras
  sincronizadas, temas, Loja de módulos, janela mini, bandeja, Last.fm e Discord.

Gratuito e de código aberto. Os serviços de streaming entram como **provedores opcionais da Loja**, que você instala e
conecta se quiser (plano em [`docs/plano-v2.md`](docs/plano-v2.md)).

## Compilar

Precisa do Rust (edição 2024, 1.88 ou mais nova) e do Windows SDK 10.0.22621 (o `fxc.exe` dos shaders; o caminho está
em `app/.cargo/config.toml`).

```powershell
cd app
cargo run -p player                       # abre o app
cargo build --release -p player           # versão otimizada
cargo test --workspace --exclude visual   # testes (o do visual usa a placa de vídeo)
cargo clippy --workspace --all-targets
```

Os detalhes de cada função estão em [`app/README.md`](app/README.md). Para contribuir, leia
[`CONTRIBUTING.md`](CONTRIBUTING.md); para agentes de IA, [`AGENTS.md`](AGENTS.md).

## Estrutura

| Pasta | Conteúdo |
| --- | --- |
| `app/` | O app em Rust (workspace Cargo: captura, análise, visual, tocador, interface em GPUI) |
| `preview/` | Protótipo em WebGL: o gabarito do visual e do diretor automático |
| `mockup/` | Mockup da interface |
| `docs/` | Planos, pesquisas e notas técnicas |
| `tools/` | Simulador do diretor e ferramentas de calibração |

## Privacidade

Nada sai do seu computador por conta do app: sem conta, sem servidor e sem telemetria. Histórico, estatísticas,
análises e ajustes ficam em `%APPDATA%\TccPlayer`. Cada serviço externo (letras, Last.fm, Discord, provedores) só é
usado quando você liga.

## Licença

[GPL-3.0-or-later](LICENSE). As licenças das bibliotecas e fontes usadas estão em
[`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md).
