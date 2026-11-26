# Remodelagem, parte 4: barra, painel e menu

Desenho: [`../specs/2026-11-25-remodelagem-streaming-design.md`](../specs/2026-11-25-remodelagem-streaming-design.md), seções 8 e 9.
Execução nesta sessão, sem subagentes, com os testes da lógica escritos antes.

**Objetivo:** da barra de baixo se chega a tudo da música que toca; a Fila e a Letra ficam no painel, ao lado da página; e o menu do
botão direito (ou do ⋯) vale em capas, linhas, fila, barra e visual.

## Decisões desta parte

- **Links da barra:** a capa e o título abrem o álbum (sem o id dele, a página da faixa); o artista, a página do artista (sem id,
  a busca pelo nome). Música do computador: o álbum e o artista dela na Biblioteca. Som de outro app (Modo Escuta): sem link.
- A fila passa a guardar os ids do álbum e dos artistas (`Remota`), só na memória.
- **Barra:** coração ao lado do nome; à direita, letra, fila, painel, volume e ⋯ (tela cheia, mini, Ilha, letra em tela cheia, saída de som). Abaixo
  de 1100 px, a coluna da esquerda tem pelo menos 264 px: o nome da música vale mais que o play exatamente no meio.
- **Painel:** abas Agora (os módulos), Fila e Letra. `Q` e o botão abrem a Fila no painel; onde não há painel (janela estreita da Estante),
  a página. Na Estante larga o trilho Tocando continua à vista (a parte 5 o refaz); a aba escolhida só troca o que há embaixo do visual.
- **Menu:** só o que vale para o item; abre onde o mouse está e cabe na janela; setas, Enter e Esc. As ações por cima das linhas
  continuam; o ⋯ fica sempre à vista no fim da linha e, nos cartões, com o mouse em cima (o alfinete vai para o menu).
- **Separar** (duas versões juntadas errado) fica em `<dados>/separadas.json`.
- "Abrir no outro serviço": se a música está nos dois, "Tocar pelo ..."; senão, "Procurar no ..." (o filtro muda e a busca vem pronta).

## Tarefas

1. **Lógica:** `paginas::destino_do_titulo`, `destino_do_artista` (faixa local, de serviço, do Match e sem ids); `menu.rs`: `Alvo`, `Acao`,
   `entradas`, `lugar`; `juntar::Separadas` e `Junto.tambem` com as faixas. Testes de cada tipo de item e do menu que não cabe.
2. **`ui/menu.rs`**: a caixa, o teclado, abrir pelo botão direito e pelo ⋯; `app/menu.rs`: executar as ações.
3. **`ui/dock.rs`** refeito; **`ui/painel/mod.rs`** com as abas; `ui/fila.rs` e `ui/painel/letra.rs` no painel.
4. `fmt`, `clippy`, testes; cópia muda em 1920, 1440, 958 × 560 e 410; README, AGENTS; release.
