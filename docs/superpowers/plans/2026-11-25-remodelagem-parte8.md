# Remodelagem, parte 8: o tamanho da interface

**Objetivo:** Configurações › Aparência › Tamanho da interface (90, 100, 110, 125%), `Ctrl +` e `Ctrl −`; tudo cresce junto, e os
modos responsivos acompanham.

**Desenho:** [`../specs/2026-11-25-remodelagem-streaming-design.md`](../specs/2026-11-25-remodelagem-streaming-design.md), seção 5 (decidido na
parte 1): o `px` das telas é o de `ui/medidas.rs`, que multiplica pela escala, e o arranjo é calculado na janela dividida por ela.

## Decisões

- A escala fica num estático de `medidas.rs`; cada janela a define no começo do desenho (a principal, a do ajuste; a Ilha, 1: ela tem o
  tamanho dela). O GPUI desenha uma janela inteira de cada vez.
- Em 100% o número passa como está; nas outras escalas, os tamanhos de 2 px ou mais viram pixels inteiros (nada de meio pixel tremendo).
- A janela pequena não força a escala: `escala_que_cabe` baixa até o arranjo caber (320 × 400 lógicos).
- O que vem do mouse e das áreas medidas está em pixels reais: a divisória, o menu, as dicas, o arrasto dos módulos, a rolagem até o
  comando, a lista longa e o visual (`desenhar_visual`) convertem.
- `ui/linha.rs` mede em pixels reais e fica com o `px` do GPUI.

## Tarefas

1. `Ajustes.tamanho` (u16, 100; `validar` leva ao mais perto da lista); `Comando::AumentarInterface` e `DiminuirInterface` (`Ctrl =`, `Ctrl -`).
2. `medidas::{px, escala, definir_escala, escalar, logico, escala_que_cabe}` e testes.
3. Trocar o `px` do GPUI pelo de `medidas` nas telas (fora a Ilha e `linha.rs`); `quadro` calcula o arranjo na janela dividida.
4. As conversões da lista acima; Configurações › Aparência.
5. Testes: cada tamanho em cada modo no arranjo; o arredondamento; o ajuste. Conferir 90, 110 e 125% em 1440 e 958 × 560, e a Ilha igual.
