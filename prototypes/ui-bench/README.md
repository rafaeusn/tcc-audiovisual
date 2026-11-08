# Protótipo comparativo de UI: Slint × GPUI

Dois apps com **o mesmo layout em árvore, o mesmo shader e as mesmas medições**, para escolher o framework de UI do
app em Rust. Resultados e análise: [`docs/prototipo-ui-resultados.md`](../../docs/prototipo-ui-resultados.md).

```
ui-bench/
├── core/          partes comuns (independentes de framework)
│   ├── layout.rs  layout em árvore estilo Hyprland (breakpoints, flutuantes, soltar na borda/no meio) + testes
│   ├── visual.rs  anel espectral em wgpu (visual.wgsl)
│   ├── audio.rs   som do sistema (loopback WASAPI) → grave/médio/agudo/volume, ou sinal sintético
│   └── metrics.rs primeiro quadro, fps, p50/p95/p99, RAM, CPU → JSON
├── slint-proto/   Slint 1.18: UI inteira em skins/*.slint, carregada e recarregada em tempo de execução
├── gpui-proto/    GPUI 0.2.2 (Zed): visual sem cópia (camada DirectComposition) + skins KDL recarregáveis
│   ├── src/underlay.rs  camada do visual por baixo da UI (wgpu DX12, zero cópia)
│   ├── src/skin.rs      interpretador de skins .kdl → elementos GPUI
│   └── skins/*.kdl      skins (default, neon)
├── themes/        tokens de tema (mesmos arquivos nos dois)
├── layouts/       cenários de medição (ex.: visual-cheio.toml)
└── layout.toml    layout padrão (o Slint salva as edições em layout.user.toml)
```

## Requisitos (Windows)

- Rust (rustup) e Visual Studio Build Tools com C++ e Windows SDK.
- Neste PC: Rust em `D:\Rust` (`CARGO_HOME`/`RUSTUP_HOME`) e builds em `D:\Rust\target` (configurado em
  `D:\Rust\cargo\config.toml`).

## Rodar

```bash
cargo run --release -p slint-proto
cargo run --release -p gpui-proto
```

Teclas: **E** modo edição (arrastar cabeçalhos e divisórias, mover/redimensionar o relógio), **T** tema,
**S** skin, **F** tela cheia. Com o app aberto, edite `slint-proto/skins/default/app.slint` ou
`gpui-proto/skins/default.kdl` (skin), `themes/*.toml` (tema) ou `layout.user.toml` (layout, só Slint): muda na hora.

Sem `--synthetic`, o visual reage ao som que estiver tocando no PC (Modo Escuta).

## Medir

```bash
cargo build --release -p slint-proto -p gpui-proto
D:/Rust/target/release/slint-proto.exe --bench 20 --synthetic --out bench-results/slint.json
D:/Rust/target/release/gpui-proto.exe --bench 20 --synthetic --out bench-results/gpui.json
```

`--ui-hz N` limita as atualizações dos números da UI a N por segundo (o visual continua a todo quadro).
No Slint, `--static-ui` e `--no-visual` servem para isolar gargalos. No GPUI: `--readback` (visual pelo caminho com
cópia), `--skin neon`, `--layout layouts/visual-cheio.toml`, `--fullscreen`.

O GPUI precisa do `fxc.exe` do Windows SDK; o caminho está em `.cargo/config.toml` (`GPUI_FXC_PATH`), ajuste se o
SDK instalado for outro.

## Testes

```bash
cargo test -p proto-core
```
