# Remodelagem, parte 7: tamanhos e teclado

**Objetivo:** andar pela tela com o Tab (navegação, busca, filtro, página, barra), com o anel de foco e Enter ou espaço; vazios
sempre com uma ação; nomes enormes, títulos sem espaço, listas de 10 mil e a janela de 400 × 400 sem quebrar.

**Desenho:** [`../specs/2026-11-25-remodelagem-streaming-design.md`](../specs/2026-11-25-remodelagem-streaming-design.md), seções 12 e 14.

## Decisões

- O GPUI 0.2.2 já tem `tab_index`, o estilo de foco (`.focus`) e o clique pelo Enter e pelo espaço (no soltar da tecla). Falta: o Tab
  chamar `focus_next`/`focus_prev`, e duas coisas que ele faz e o Player não quer.
- **O mouse também foca**: o anel só aparece depois de um Tab (`foco_pelo_teclado`); um clique sai desse modo e o foco volta à raiz
  no quadro seguinte (os atalhos de uma letra continuam valendo depois de um clique).
- **O espaço é tocar e pausar**: com um elemento focado pelo Tab, espaço e Enter são dele. `Esc` sai do modo e devolve o foco à raiz.
- O anel é uma sombra de 2 px por fora, no raio do elemento (borda mexeria no tamanho).
- A ordem segue a ordem de desenho: navegação, topo, página, painel, barra, o visual.

## Tarefas

1. `Player::focavel(el, raio)` e as peças: `botao`, `pilula`, `opcao`, destinos, abas, fichas do filtro, cartões, atalhos, linhas,
   coração, ⋯. Tab e Shift Tab em `ao_teclar`; o modo sai no clique.
2. Vazios: a Fila (painel e página) e a lateral ganham "Buscar músicas".
3. Testes: `janela_de_linhas` com 10 mil, arrumar a biblioteca com 10 mil (tempo), o título de 200 letras, `arranjo` em 400 × 400.
4. Conferir na cópia muda: Tab pela Estante em 958 × 560, Enter num cartão, espaço num botão sem pausar, Esc, um nome de 200 letras
   (uma pasta com um WAV de nome enorme), 400 × 400.
