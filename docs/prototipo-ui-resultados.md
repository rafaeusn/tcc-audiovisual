# Protótipo comparativo de UI em Rust: Slint × GPUI

Experimento de 07/11/2026 para escolher o framework de interface do app (Windows primeiro, customização estilo
Hyprland/end4). Código e instruções: [`prototypes/ui-bench/`](../prototypes/ui-bench/README.md).

## 1. Método

Os dois protótipos compartilham tudo o que não é interface (`prototypes/ui-bench/core`), para medir só a UI:

| Parte comum | O que faz |
| --- | --- |
| Layout em árvore (`layout.rs`) | Estilo "dwindle" do Hyprland: divisões com proporção, breakpoints por largura de janela, widgets flutuantes, soltar painel na borda (nova divisão) ou no meio (troca). 5 testes automáticos |
| Visual (`visual.wgsl`) | Anel espectral simplificado (paleta branco → azul → vermelho), wgpu 30 |
| Áudio (`audio.rs`) | Loopback WASAPI da saída padrão (embrião do Modo Escuta) ou sinal sintético determinístico de 120 BPM |
| Medições (`metrics.rs`) | Tempo do processo até o 1º quadro, fps, tempos de quadro p50/p95/p99/máx, RAM (working set e privada), CPU |

| | Slint 1.18 | GPUI 0.2.2 (Zed) |
| --- | --- | --- |
| Onde a UI é escrita | `skins/*/app.slint`, **compilada em tempo de execução** (slint-interpreter) | Código Rust compilado |
| Renderer | femtovg sobre wgpu | DirectX 11 próprio do GPUI |
| Visual do shader na UI | Textura wgpu importada direto (**zero cópia**) | Device wgpu separado → pixels voltam para a CPU → sobem como imagem a cada quadro |
| Animação dos painéis | Declarativa (`animate x, y… { easing: cubic-bezier(…) }`) | Interpolação escrita à mão em Rust |

**Cenário:** janela 1280×800, sinal sintético, 2 s de aquecimento + 20 s medidos, layout padrão (visual, player,
fila, espectro, relógio flutuante). Modo "cheio": os números da UI (barras do espectro, progresso) mudam a todo quadro.
Modo "30 Hz": esses números mudam 30 vezes por segundo (o visual continua a todo quadro). Duas rodadas alternadas de
cada. **Máquina:** RTX 2060, monitor de 144 Hz, Windows 11.

## 2. Resultados

| Medida (média de 2 rodadas) | Slint, cheio | Slint, 30 Hz | GPUI, cheio | GPUI, 30 Hz |
| --- | --- | --- | --- | --- |
| fps (monitor = 144) | 104 | 138 | **144** | **144** |
| Quadro p50 / p95 / p99 (ms) | 9,3 / 11,3 / 13,4 | 6,5 / 11,1 / 13,4 | **6,9 / 7,1 / 7,2** | **6,9 / 7,1 / 7,3** |
| CPU (100% = 1 núcleo) | 100% | 93% | **32%** | **33%** |
| RAM working set | **172 MB** | **172 MB** | 209 MB | 209 MB |
| RAM privada | **208 MB** | **208 MB** | 454 MB | 454 MB |
| Até o 1º quadro | 1,12 s | 1,12 s | **0,84 s** | **0,85 s** |

| Outras medidas | Slint | GPUI |
| --- | --- | --- |
| Executável (release) | 26,1 MB (inclui o compilador de skins) | **13,2 MB** |
| Dependências (crates) | **353** | 589 |
| Linhas do protótipo (UI) | 520 de Rust + 470 de `.slint` | 680 de Rust |

Diagnóstico do Slint (rodadas curtas de 6 s):

| Variante | fps | CPU |
| --- | --- | --- |
| UI cheia | 109 | 102% |
| UI parada (`--static-ui`) | 144 | 60% |
| UI parada e shader parado (`--no-visual`) | 144 | 66% |
| UI a 60 Hz / 30 Hz | 140 / 144 | 85% / 81% |

**Leitura:**
- **O shader não é o gargalo** em nenhum dos dois.
- **No Slint, o próprio desenho da janela já custa ~60% de um núcleo a 144 fps.** Atualizar propriedades pela skin
  interpretada a todo quadro satura o núcleo.
- **O GPUI fica travado nos 144 fps com um terço da CPU**, mesmo pagando a cópia GPU → CPU → GPU do visual a cada
  quadro. Os quadros também são mais regulares: p99 de 7,2 ms contra 13,4 ms.
- **A RAM maior do GPUI vem de ter dois devices de GPU** (o DirectX dele e o wgpu do visual). Com o visual integrado
  sem cópia, esse número cai.

## 3. Customização (o outro critério)

| Requisito | Slint | GPUI |
| --- | --- | --- |
| Skin feita pelo usuário, carregada em tempo de execução | **Sim, pronto** (testado: troca de skin com `S`, e editar o `.slint` com o app aberto atualiza em < 1 s; uma skin com erro mostra a mensagem e a anterior continua) | Não existe. Precisaríamos construir uma camada própria (formato de skin em dados → elementos GPUI) |
| Temas por tokens recarregados ao vivo | Sim | Sim |
| Layout em árvore arrastável + flutuantes + breakpoints | Sim (motor nosso, testado com mouse) | Sim (o mesmo motor, testado com mouse) |
| Animações com curva | Declarativas, prontas | Feitas à mão (simples, já funcionam) |
| Visual de shader sem cópia | **Sim** (`unstable-wgpu-30`) | Não, por enquanto. Em 4K a cópia vira gargalo, e é preciso resolver isso |
| Maturidade | 1.x, API estável, documentação boa, licença royalty-free para desktop | Pré-1.0, documentação escassa. O ecossistema se dividiu: o `gpui-component`/`gpui-kit` (que tem dock pronto e extensões em JS) usa um fork, o `gpui-pre`, sem plataforma Windows publicada, e não compilou aqui |
| Instalação no Windows | Simples | Precisou apontar o `fxc.exe` do Windows SDK (`GPUI_FXC_PATH`) |

Prints em [`docs/img/prototipo-ui/`](img/prototipo-ui/): modo edição, skin Neon, recarga ao vivo e o resultado de
arrastar a fila para a borda do visual nos dois frameworks.

## 4. Conclusão e recomendação

- **Desempenho:** o GPUI vence com folga. São 144 fps estáveis, 1/3 da CPU, quadros mais regulares, abertura mais
  rápida e executável menor. Como desempenho é a prioridade número 1 do projeto, esse é o fator decisivo.
- **Customização pronta:** o Slint vence. Skins em tempo de execução vêm de fábrica, e é exatamente o modelo do end4.
- **A diferença de customização se resolve com código nosso** (uma camada de skin em dados interpretada para
  elementos GPUI: o GPUI reconstrói a árvore de elementos a cada quadro a partir de dados, então isso encaixa
  naturalmente). **A diferença de desempenho do Slint não depende de nós:** ela está no renderer dele.

**Recomendação: GPUI como base**, condicionada a dois testes curtos antes de fechar:
1. **Visual sem cópia no GPUI (Windows):** compartilhar a textura do wgpu com o DirectX do GPUI (handle
   compartilhado DX12 ↔ DX11) ou desenhar o visual numa camada própria por baixo da UI. Se isso falhar, o visual em
   tela cheia/4K fica limitado e o Slint volta a ser a escolha.
2. **Camada de skin em dados:** um formato (TOML/JSON ou uma DSL pequena) que descreve painéis, estilos e
   componentes, carregado e recarregado em tempo de execução, provando que dá para ter o "estilo end4" em cima do
   GPUI.

Se os dois passarem, temos o desempenho do GPUI com a customização que queremos. Se o primeiro falhar, o plano B é o
Slint com skins compiladas no app final e o interpretador só para criar e pré-visualizar skins, aceitando o custo de
CPU do renderer.

## 5. Os dois testes decisivos (resultado: passaram)

### 5.1 Visual sem cópia no GPUI (`gpui-proto/src/underlay.rs`)

**Como funciona.** No Windows, o GPUI desenha a interface numa camada do DirectComposition marcada como "de cima"
(`CreateTargetForHwnd(hwnd, topmost = true)`) e limpa com transparente onde não desenha nada. O Windows aceita uma
segunda camada na mesma janela (`topmost = false`). Criamos essa camada, entregamos ao wgpu (backend DX12, superfície
`CompositionVisual`) e desenhamos nela o fundo da janela e o visual, já com os cantos arredondados do painel. O
compositor do sistema junta as duas na GPU, e nenhum pixel passa pela CPU. Não foi preciso modificar o GPUI.

Analogia: o GPUI é uma folha de acetato com a interface desenhada; o visual é a folha de papel embaixo.

| Média de 3 rodadas de 20 s | Com cópia (readback) | **Sem cópia (camada)** |
| --- | --- | --- |
| Janela 1280×800: fps / p99 | 144 / 7,17 ms | 144 / 7,16 ms |
| Janela 1280×800: CPU | 29% (26–35) | **15%** (2–23) |
| Tela cheia 1920×1080, visual grande: fps / p99 | 144 / 7,41 ms | 144 / 7,16 ms |
| Tela cheia 1920×1080: CPU | 58% (52–67) | **10%** (3–24) |
| RAM working set | 208–218 MB | **177 MB** (empata com o Slint) |
| Até o 1º quadro | 0,83–0,86 s | **0,75–0,79 s** |

Com cópia, a CPU dobra da janela para a tela cheia, porque ela cresce com a área do visual. Sem cópia, ela não depende
do tamanho, o que deixa 4K e vários monitores viáveis. Funcionam por cima do visual: relógio flutuante, cabeçalhos,
modo edição e arrastar painéis (o visual acompanha a animação do painel).

**Pontos a acompanhar.**
- As duas camadas são apresentadas separadamente. Durante uma animação, a borda do painel e o visual podem ficar
  até um quadro (7 ms) fora de sincronia; não foi perceptível nos testes.
- Redimensionar a janela recria a superfície. Falta medir se pisca.
- A técnica é específica do Windows (DirectComposition). No macOS/Linux, o equivalente usa CoreAnimation/Wayland
  subsurfaces, para quando o app sair do Windows.

### 5.2 Skin em dados para o GPUI (`gpui-proto/src/skin.rs`, `gpui-proto/skins/*.kdl`)

**Como funciona.** Um arquivo [KDL](https://kdl.dev) (formato em árvore, o mesmo que o Zellij usa para layouts)
descreve a moldura dos painéis, o cabeçalho e o conteúdo de cada painel. O conteúdo é feito de elementos (`column`,
`row`, `box`, `text`, `progress`, `bar`, `each`, `spacer`) com estilos, tokens de tema e valores ao vivo
(`"{bass}"`, `"{now.title}"`, `each "queue" { … "{item.title}" }`). O app interpreta o arquivo e monta os elementos do
GPUI a cada quadro. Trecho da skin padrão:

```kdl
panel "player" title="TOCANDO AGORA" {
    column w="fill" h="fill" padding-top=34 padding-x=16 gap=10 {
        row gap=14 align="center" {
            box w=52 h=52 radius=10 bg="gradient(accent, accent-2)"
            column {
                text "{now.title}" size=16 weight="bold" color="text"
                text "{now.source}" size=12 color="muted"
            }
        }
        progress "{progress}" h=4 radius=2 track="surface-2" fill="accent"
    }
}
```

**Testado:**
- Troca de skin com `S`. A skin `neon` põe o visual como papel de parede da janela inteira e os painéis de vidro
  translúcido, com outra tipografia e outro arranjo interno.
- Editar o `.kdl` com o app aberto atualiza a interface em menos de 1 s.
- Uma skin com erro mostra a mensagem (com linha e coluna) e a anterior continua ativa.

| Média de 3 rodadas de 20 s (sem cópia) | fps | p99 | CPU | RAM | 1º quadro |
| --- | --- | --- | --- | --- | --- |
| Skin `default` (KDL) | 144 | 7,15 ms | 14% | 176 MB | 0,72 s |
| Skin `neon` (KDL, visual em tela cheia) | 144 | 7,16 ms | 14% | 177 MB | 0,72 s |

Interpretar a skin a cada quadro não teve custo mensurável.

### 5.3 Comparação final

| | Slint (skin interpretada) | **GPUI + camada + skin KDL** |
| --- | --- | --- |
| fps (monitor 144 Hz) | 104 (138 limitando a UI a 30 Hz) | **144** |
| p99 | 13,4 ms | **7,2 ms** |
| CPU | ~100% | **~14%** |
| RAM | 172 MB | 177 MB |
| 1º quadro | 1,12 s | **0,72 s** |
| Skins em tempo de execução + recarga ao vivo | Sim (pronto) | Sim (camada própria, ~400 linhas) |
| Visual sem cópia | Sim | Sim (Windows; outros sistemas a fazer) |

**Decisão: GPUI.** O desempenho é 7× melhor em CPU com quadros duas vezes mais regulares, e a customização ao vivo
ficou equivalente. O preço é manter código nosso: a camada do visual e o interpretador de skin, que viram parte
central do produto (e um capítulo do TCC). Outros riscos: o GPUI é pré-1.0, a API muda e a documentação é escassa.
O plano para conviver com isso é fixar a versão e isolar o GPUI atrás dos nossos módulos (layout, skin, visual).

Prints: [`gpui-camada-sem-copia.png`](img/prototipo-ui/gpui-camada-sem-copia.png),
[`gpui-skin-kdl-neon.png`](img/prototipo-ui/gpui-skin-kdl-neon.png),
[`gpui-skin-kdl-recarga.png`](img/prototipo-ui/gpui-skin-kdl-recarga.png),
[`gpui-skin-kdl-erro.png`](img/prototipo-ui/gpui-skin-kdl-erro.png).
