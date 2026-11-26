# Remodelagem, parte 5: o visual no centro

**Objetivo:** na Estante, o visual fica grande ao lado da página em toda janela a partir de 761 px, com uma divisória que se arrasta,
a página larga, e a página que sai da frente com o mouse parado.

**Desenho:** [`../specs/2026-11-25-remodelagem-streaming-design.md`](../specs/2026-11-25-remodelagem-streaming-design.md), seção 7.1 (opção B).

**Execução:** nesta sessão, sem commit (o Rafael pede quando quiser).

## Regras

- Medidas em `ui/medidas.rs` e `arranjo.rs`; contas puras e testadas; textos em português; nada de número solto nas telas.
- O Palco não muda de arranjo.
- Nada fica só com o mouse em cima; botões de 40 ou 44.

## O que precisa dar certo (foco da revisão)

1. 958 × 560: visual e página à vista, sem se cruzar, a página com pelo menos 320 px e o visual com pelo menos 220.
2. Abrir uma página com o painel no lugar dela: o painel sai.
3. Janela que muda de tamanho com a divisória arrastada: nos limites, nada se cruza.
4. O mouse não mexeu de verdade (o Windows manda `WM_MOUSEMOVE` parado): não conta como atividade.
5. Lendo a letra no painel, escrevendo num campo, com o menu aberto ou editando: a página não sai.

## Tarefas

### 1. Recorte no visual (`crates/visual`)
- `motor::recorte(lw, lh, iw, ih) -> [f32; 4]`: (u0, v0, du, dv) do meio da cena, cobrindo o retângulo sem deformar.
- `saida.wgsl` lê `t0` por `U.v[3]`; `Destino.interno` passa a recortar (antes esticava).
- Testes: igual → inteiro; mais estreito → corta os lados; tira → corta em cima e embaixo; valores estragados → inteiro.

### 2. Ajustes e comando (`crates/nucleo`)
- `divisao: Option<f32>` (0,28 a 0,68), `pagina_larga: bool`, `descanso_s: u32` (30; 0 desliga; senão 5 a 600).
- `Comando::AlternarPaginaLarga` ("Página larga (Estante)", tecla `W`, grupo Layout).

### 3. Arranjo (`player/src/arranjo.rs`)
- `Pedido` ganha `divisao`, `larga` (0 a 1), `arrastando`, `alta` (Início do empilhado, 0 a 1); sai `tocando`.
- Estante: lateral a partir de 1200; trilho de 761 a 1199; barra embaixo até 760. Visual : página = 52 : 48 (1680+), 46 : 54, 48 : 52.
- Painel (fechado por padrão, `gaveta_aberta`): ao lado a partir de 1680 (380); abaixo disso, no lugar da página.
- `Arranjo.divisoria: Option<Ret>`, `interno: Option<(f32, f32)>`, `painel_na_pagina: bool`.
- Testes em todos os tamanhos: nada se cruza, mínimos, limites da divisória, página larga, painel nos três modos, expansão.

### 4. Estante na tela (`ui/estante.rs`, `app.rs`, `app/menu.rs`, `app/midia.rs`)
- `ver_quadro_do_visual`: título, artista, BPM (520+), botões; tira com "Lado a lado"; o alto do empilhado.
- Só o visual: a página sai para a direita. Divisória com pega, dois cliques voltam ao padrão.
- Painel: `P`, `Q`, `U` valem em toda janela; sai `painel_fixo`, `ALTURA_PARA_A_ABA`, `fechar_gaveta_fora_da_estante` → `sair_da_frente_da_pagina`.
- Empilhado: no Início o visual tem 40%; rolando, vira tira (`recolhe`, com folga para não ficar indo e voltando).

### 5. A página sai da frente (`player/src/descanso.rs`)
- `Descanso { ultimo, ponteiro, dormindo }`: `mexeu(pos)` (só se andou mais de 3 px), `atividade()`, `passo(&Entrada) -> bool`.
- Dormindo: só o visual e o nome da música, cursor escondido; clique e as teclas Esc, X só acordam; as outras valem.
- Configurações › Aparência: Desligado, 15 s, 30 s, 1 min, 2 min.

### 6. Conferência e documentos
- fmt, clippy, testes, release, cópia muda (1920, 1440, 958 × 560, 800, 410; Palco em 1440).
- README, AGENTS (seções 3, 4, 7 e 8), spec, auditoria, memória.
