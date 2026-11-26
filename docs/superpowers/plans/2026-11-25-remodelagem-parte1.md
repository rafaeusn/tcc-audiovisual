# Remodelagem, parte 1: medidas e consertos

Desenho: [`../specs/2026-11-25-remodelagem-streaming-design.md`](../specs/2026-11-25-remodelagem-streaming-design.md), seções 5, 8 e 14.
Execução nesta sessão, uma revisão no fim. Commit só quando o Rafael pedir.

**Objetivo:** todo tamanho de texto, ícone e raio das telas sai de `app/player/src/ui/medidas.rs`, tudo cresce para os
papéis novos, e os defeitos fáceis da auditoria (5, 7, 9, 17, 25, 26, 27) somem.

**Mudança em relação ao desenho:** as medidas ficam em `px`, não em `rem`. Na parte 8 o `px` das telas passa a ser um do
próprio `medidas.rs`, que multiplica pela escala, e o arranjo é calculado na janela dividida por ela: uma conta só, sem
trocar cada medida por `rem` (e os modos responsivos acompanham a escala de graça).

## Regras

- A Ilha (`ui/ilha/`) tem escala própria e não muda.
- Linhas de 1 px, barras de gráfico e miniaturas dos temas na Loja não são texto, ícone nem cartão: o raio delas é
  `raio_fino(altura)`.
- O "· · ·" das pausas da letra fica (é desenho da pausa, não separador).

## Tarefas

1. **`ui/medidas.rs`** com os papéis (`micro`, `apoio`, `corpo`, `destaque`, `secao`, `pagina`), as constantes de ícone,
   raio e altura, e o teste `sem_numero_solto`, que lê `src/ui/**/*.rs` (menos a Ilha) e falha com `texto(<número>`,
   `titulo(<número>`, `numero(<número>`, `icone(.., <número>`, `tamanho_icone: <número>`, `.text_size(px(<número>` e
   `.rounded*(px(<número>`. Vermelho no começo; verde no fim da tarefa 3.
2. **Consertos com teste:** a busca dos serviços separada do filtro da Biblioteca; `capas::miniatura` (128 px gravada ao
   lado, usada até 64 px de lado). Sem teste possível: raio na imagem, `ObjectFit::Cover` na barra e na sessão,
   reticências, separadores e caixa alta.
3. **Telas nas medidas novas**, um grupo por vez: ajudantes (`mod.rs`), páginas, busca, lateral e topo, músicas e fila,
   barra, painel, Loja e Configurações, janelas. Compila a cada grupo.
4. **`arranjo.rs`:** topo 64, barra 96 (88 em janela baixa), navegação de baixo 72, lateral 264; testes ajustados.
5. **Ícones:** traço de 1,8.
6. **Conferir:** `fmt`, `clippy`, testes, cópia muda em 1920, 1440, 958 × 560 e 410, Estante e Palco, com foto antes e
   depois. README, AGENTS e o desenho.

## O que olhar na revisão

Texto que não cabe mais (botões, fichas, módulos, janelas); linhas com altura fixa onde o texto cresceu; a lista
rolando por altura de linha errada (`uniform_list`, `janela_de_linhas`); miniatura pedida antes de existir; janela 400 × 400.
