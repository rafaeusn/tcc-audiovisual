# F2, parte 5: o layout Estante

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** um segundo layout, a Estante (lateral com espaços, páginas e salvos; a folha da página; o trilho "Tocando"
com o visual ao vivo e os módulos), com expandir o visual até a janela inteira e o responsivo.

**Architecture:** `arranjo::calcular` ganha `Pedido.estante` e `Pedido.expansao` (0..1) e devolve `lateral` e
`tocando` (o retângulo do visual fica no alto do trilho; o painel de módulos embaixo). A camada do visual já desenha
num retângulo qualquer (`arranjo.visual`), e o que está por cima precisa deixar esse retângulo livre. Expandir anima
o retângulo até a janela (as outras regiões esmaecem) e, no fim, vira o arranjo de tela cheia (o herói com o título
grande).

**Tech Stack:** Rust 2024, GPUI 0.2.2.

**Spec:** `docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md` (5.5, 5.6, 5.7).

## Global Constraints

- `ajustes.layout` (`"palco"`, o padrão, ou `"estante"`), com `#[serde(default)]`; o Palco não muda em nada.
- Na Estante sempre há uma rota aberta (começa na primeira página do espaço); fechar a folha não vale.
- Medidas: lateral 248 px; trilho Tocando 340 px (largo); raio e filete do repertório.
- Movimento reduzido: expandir e encolher sem animação.
- Camada do visual por baixo da interface: nada opaco por cima do retângulo do visual.

## Decisões (minhas, para o Rafael conferir)

- Empilhado (≤ 760 px): o visual vira uma faixa de 72 px acima do dock (o dock é opaco: o visual não aparece
  dentro dele); tocar na faixa expande.
- Gaveta (761–1060): a lateral vira o trilho de ícones (espaços e páginas); o Tocando é a gaveta da direita (`P` ou
  o botão do dock), com o visual no alto dela; fechada, o visual não aparece.
- Comandos novos: `TrocarLayout` (`Ctrl L`) e `ExpandirVisual` (`X`; no Palco, só avisa).
- Expandir passa pelo arranjo de tela cheia no fim (o herói com o título grande e o botão encolher); `Esc` encolhe.

## Review Focus

1. Trocar de layout com a folha fechada (Palco) e voltar: a Estante abre numa página; o Palco volta como estava.
2. Esc, H e o botão fechar da folha na Estante: não deixam a folha vazia nem a tela sem página.
3. Expandir no meio de uma troca de tamanho da janela, ou com a janela mini / letra em tela cheia.
4. Gaveta e empilhado: o visual some e volta sem deixar a camada desenhando por cima de nada.
5. Salvos na lateral: dados estragados (`None`) não quebram e os fixados vêm primeiro.

---

### Task 1: ajuste, comandos e o arranjo da Estante

**Files:** `crates/nucleo/src/ajustes.rs`, `crates/nucleo/src/comando.rs`, `player/src/arranjo.rs`

- [ ] Testes: `estante_larga_tem_lateral_folha_e_trilho` (sem sobrepor, visual no alto do trilho, dock sob folha e
  trilho), `estante_gaveta_visual_so_com_a_gaveta`, `estante_empilhada_visual_acima_do_dock`,
  `expandir_leva_o_visual_a_janela_inteira`, e nos ajustes `layout_padrao_e_palco_e_volta_do_toml`.
- [ ] Implementar `Layout`, `Comando::{TrocarLayout, ExpandirVisual}`, `Pedido.{estante, expansao}`,
  `Arranjo.{lateral, tocando}`.

### Task 2: o Player na Estante

**Files:** `app.rs`, `app/navegar.rs`, `ui/heroi.rs`, `ui/espacos.rs`

- [ ] `Player::estante()`, rota garantida na Estante (`navegacao::Navegacao::garantir_pagina` testado), Esc/H/fechar
  na Estante, a mola da expansão e o arranjo de tela cheia no fim, esmaecer as regiões.

### Task 3: as telas da Estante

**Files:** `ui/estante.rs` (novo), `ui/casca.rs`, `ui/dock.rs`

- [ ] Lateral (espaços, páginas, salvos do espaço com os fixados primeiro, Loja e Configurações), o trilho de ícones
  da gaveta, o quadro do visual no trilho (nome do visual, título, expandir), a faixa do visual no empilhado, o
  encolher no herói expandido, a navegação de baixo com as abas do espaço.

### Task 4: escolher o layout

**Files:** `ui/configuracoes.rs`, `ui/loja.rs`

- [ ] Configurações › Aparência: Palco | Estante; a aba Layouts da Loja ganha o cartão Estante.

### Task 5: revisão, cópia e documentos

- [ ] fmt, clippy, testes, revisor novo; cópia de teste a 1280, 900 e 600 px, expandir e encolher; README e AGENTS.md.
