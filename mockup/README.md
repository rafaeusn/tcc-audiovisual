# Mockup da UI do Player (v4)

Cópia do código do mockup interativo publicado como artefato no claude.ai:
https://claude.ai/artifact/BL3KK2BQtkkwn1JfWAjXz5 (privado; só o dono abre, a menos que compartilhe pelo menu Share).

Esta pasta existe para que qualquer IDE ou agente consiga ler o estado atual, rodar a prévia e propor mudanças. O
artefato continua sendo a versão publicada; o caminho de volta está na seção 8.

- O que foi decidido e por quê: [docs/design-ui-v1.md](../docs/design-ui-v1.md) e
  [docs/repertorio-ui.md](../docs/repertorio-ui.md).
- Plano do app de verdade (Rust + GPUI): [docs/plano-v1.md](../docs/plano-v1.md). O mockup é só referência visual e
  de comportamento; nada daqui vira código do app diretamente.

## 1. Estrutura

| Caminho | O que é |
| --- | --- |
| `project/Main.dc.html` | O protótipo inteiro: template, lógica, simulação da música e o visual em WebGL |
| `project/app.css` | Todos os estilos e tokens (temas Noite, Grafite, Papel e Monocromático) |
| `project/*.dc.html` (os outros 14) | Pranchas: cada uma importa o `Main` com props fixas (estado, tamanho, tema) |
| `project/canvas.json` | Posição, tamanho e título das pranchas no canvas do artefato |
| `assets/covers/` | As 12 capas geradas, servidas como `/_blob/<id>` |
| `harness/` | Prévia local: um interpretador mínimo do formato `.dc.html` e um servidor |
| `tools/` | Auditoria automática, captura de tela e os scripts que geraram as capas |

## 2. Rodar a prévia

```bash
python mockup/harness/serve.py
```

Depois abra `http://127.0.0.1:5510/harness/preview.html?f=Main.dc.html`. Qualquer prancha funciona no `f`
(`f=Sistema.dc.html`, por exemplo). As props do `Main` entram como parâmetros da URL, em camelCase ou kebab-case:

| Prop | Valores |
| --- | --- |
| `tela` | `inicio`, `biblioteca`, `loja`, `ajustes`, `stats` |
| `cenario` | `escuta` (Spotify ao vivo), `arquivo` (arquivo local), `vazio` |
| `momento` | segundos da faixa, de 0 a 130 (44 cai no drop, 3 é calmo, 20 é agitado) |
| `tema` | `noite`, `grafite`, `papel`, `mono` |
| `skin` / `layout` | `blocos`, `imersiva` / `padrao`, `foco`, `mini` (este aparece como Enxuto) |
| `coresReativas`, `modoSeguro`, `edicao`, `telaCheia`, `paleta`, `mini`, `painel`, `lastfm` | `true` ou `false` |
| `lojaAba` / `ajustesAba` | `modulos`, `temas`, `skins`, `layouts`, `pacotes` / `visual`, `aparencia`, `atalhos`, `sobre` |
| `faixa` | id de uma faixa da biblioteca (`go`, `chopin`...), com `cenario=arquivo` |
| `estatico` | `true` para parar a animação depois de 1,5 s (bom para captura) |
| `raf` | `timeout` quando a aba fica em segundo plano: troca o `requestAnimationFrame` por timer |

Atalhos no protótipo: Espaço toca ou pausa, E edita o layout, F abre a tela cheia, M abre o mini player, Ctrl K abre
a paleta, T troca o tema, S troca a skin, Esc fecha.

## 3. O formato `.dc.html`

É o formato do tipo de artefato Design do claude.ai. Resumo do que o mockup usa:

- Tudo que aparece fica dentro de `<x-dc>`. O `<helmet>` carrega fontes e estilos do documento.
- O template é HTML comum com `{{ nome }}` para valores. Um atributo inteiro com `{{ x }}` recebe o valor como
  está (função, booleano); dentro de texto, vira string.
- `<sc-if value="{{ cond }}">` mostra o conteúdo se a condição for verdadeira. `<sc-for list="{{ lista }}" as="item">`
  repete o conteúdo; dentro dele valem `item.campo` e os valores de fora.
- Eventos são atributos com o handler: `onClick`, `onPointerDown`, `onPointerMove`, `onPointerLeave`, `onKeyDown`,
  `onDoubleClick`, `onMouseEnter`, `onChange`. `ref="{{ fn }}"` entrega o elemento para `fn`.
- A lógica fica em `<script type="text/x-dc" data-dc-script>`: uma `class Component extends DCLogic` com
  `this.state`, `setState`, `componentDidMount`, `componentDidUpdate`, `componentWillUnmount` e `renderVals()`, que
  devolve o objeto com tudo que o template usa.
- `data-props` no mesmo script declara as props editáveis no canvas (`editor`, `options`, `default`, `section`) e o
  tamanho de prévia (`$preview`).
- `<dc-import name="Main" prop-a="x">` usa outro arquivo como componente. Props em kebab-case chegam em camelCase.
- `./support.js` é o runtime do artefato. Ele não existe aqui: o `harness/preview.html` faz esse papel.
- `/_blob/<id>` são arquivos enviados ao artefato; o `serve.py` responde com as capas locais.

## 4. Mapa do `Main.dc.html`

O template vem primeiro, na ordem da tela: barra de ícones (`nav.rail`), barra superior (`header.top`), palco
(`section.hero`, com as páginas Biblioteca, Loja e Estatísticas), painel (`aside.panel`, que mostra módulos, Ajustes
ou o detalhe da faixa), dock (`footer.dock`), tela cheia, mini player, paleta, modal de permissões e aviso.

O script é dividido por comentários `/* ---------- ... ---------- */`:

| Seção | Conteúdo |
| --- | --- |
| dados fixos | ícones, logos, temas, módulos, tom do dock (`toneFor`), ícone em pontos (`dotPath`), selo (`SEAL`) |
| faixas | `buildTracks` com as seções e a curva de energia de cada faixa |
| simulação do diretor | `step`: energia, batidas, drops, cores do visual e tom do dock; `pushUI` a cada 100 ms |
| WebGL | o visual Spectral (pontos, rastro, bloom) |
| mapa de energia | `drawMap`, `mapDown` (arrastar), `mapMove` (dica) |
| ações | handlers, abrir e fechar páginas, segurar para confirmar, volume elástico, aviso arrastável, Loja |
| modo edição | trocar módulos de lugar, redimensionar o painel, desfazer |
| valores do template | `renderVals` |

## 5. Regras de design que não podem quebrar

- Contêiner ganha linha (`--hair`), estado ganha preenchimento (`--hover`, `--sel`). Nada de card dentro de card; a
  caixa dos módulos só aparece no modo edição.
- Escala fixa de 7 tamanhos: 12/16, 14/20, 16/24, 20/28, 28/32, 44/44 e 88/80. Raios 28, 16, 10, pílula e círculo.
- Contraste mínimo de 4,5:1 em texto. No dock e na seleção, o texto de apoio nunca fica abaixo de `--fg-2`.
- O dock é a única faixa com cor: o tom do visual até luminância 0,016. No modo seguro fica neutro e parado.
- Cor só com significado: energia, marca do serviço, capa, confirmação.
- Nenhum ponto do meio (·) separando dados e nenhum rótulo em caixa alta acima de título.
- Movimento só para explicar mudança: pressionar 120 ms, menus 200 ms, painéis 240 ms, saída mais rápida que a
  entrada, nada anima em atalho de teclado, e tudo respeita o movimento reduzido.
- Abaixo de 1060 px o painel vira gaveta; abaixo de 760 px tudo empilha e o dock gruda embaixo.
- Spotify só entra pelo Modo Escuta (o app ouve o som do sistema). Não usar a API do Spotify para o visual.

## 6. Conferir uma mudança

Com a prévia aberta, rode no console do navegador:

```js
(0, eval)(await (await fetch('/tools/auditoria.js')).text())
```

O resultado lista vazamento horizontal, texto cortado, alvos menores que 24 px, texto abaixo de 4,5:1, tamanhos de
fonte e raios usados. Dois avisos são falsos: o botão ativo dos seletores (o marcador fica atrás do texto e o script
não vê) e, abaixo de 1060 px, a largura da gaveta escondida. Confira em 1440×900, 1280×720, 1024×700, 768×1024 e
390×844.

Para capturas: `bash mockup/tools/capturar.sh nome "f=Main.dc.html&momento=44&estatico=true" 1440 900` (Edge no
Windows). As imagens vão para `mockup/.capturas/`, que o git ignora.

## 7. Limites da prévia local

- O interpretador recria a página inteira a cada atualização de estado (10 vezes por segundo). Por isso ele desliga
  transições e animações: o movimento só aparece de verdade no artefato.
- O canvas do visual e os mapas são preservados entre atualizações; o resto não. Um elemento guardado numa variável
  pode deixar de existir 100 ms depois.
- Em aba escondida o navegador pausa o `requestAnimationFrame`; use `raf=timeout`.

## 8. Levar mudanças de volta ao artefato

O artefato só é publicado pelo Claude Code, que tem a ferramenta de artefatos do claude.ai. O caminho é:

1. Editar os arquivos de `mockup/project/` em qualquer IDE, conferir com a prévia e fazer commit e push.
2. No Claude Code, pedir para puxar o repositório e publicar `mockup/project/` no artefato.

Se alguém editar direto no canvas do artefato, peça ao Claude Code para baixar a versão publicada e fazer commit
antes de mexer aqui, para não sobrescrever o trabalho.
