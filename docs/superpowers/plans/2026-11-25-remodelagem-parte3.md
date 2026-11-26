# Remodelagem, parte 3: Tudo misturado e selos

Desenho: [`../specs/2026-11-25-remodelagem-streaming-design.md`](../specs/2026-11-25-remodelagem-streaming-design.md), seções 8, 9 e 10.
Execução nesta sessão, sem subagentes, com os testes da lógica escritos antes.

**Objetivo:** em Tudo, as prateleiras e os resultados dos serviços aparecem juntos, e tudo na tela diz de onde vem por um selo.

## Decisões desta parte

- **Selo:** disco escuro com a marca em linha na cor do serviço, embaixo à direita da capa (28 no cartão, 20 na linha, 24 na barra). Só
  onde a tela mistura serviços. Nos cartões, com o mouse em cima, ele cresce para 44 e vira o botão de tocar na cor do
  serviço (no filtro de um serviço só, o botão aparece sem selo). Tocar um álbum, uma playlist ou um artista pelo cartão busca as
  faixas e toca quando chegam, sem abrir a página.
- **Marca:** as dos dois serviços em linha são nossas (`icones/marca-*.svg`); um provedor sem marca usa a logo que ele manda.
- **Prateleiras:** `Prateleira.tipo` (opcional) decide quais se juntam. A k-ésima de um tipo de um serviço junta com a k-ésima do
  mesmo tipo do outro, intercalando os itens; o título é o nosso quando junta de verdade, senão o do serviço, com o nome dele ao lado. As
  sem tipo se intercalam entre os serviços. O que o serviço não disse é adivinhado pelo conteúdo e pelo título (só em provedor antigo não há).
- **Mesma música, dois selos:** ISRC igual ou nota a partir de 0,9 no `casador`, nas faixas (álbuns e artistas continuam separados). Toca
  pelo serviço preferido (`ajustes.servico_preferido`, Configurações › Serviços; vazio = o primeiro que toca por ele mesmo).
- **Recentes:** tabela `recentes` em `biblioteca.db` (até 60), com o que foi aberto e tocado; alimenta os atalhos do Início, "Tocadas há
  pouco" e "Abertos há pouco".
- **Histórico:** sem coluna nova: o `app` de cada execução já é o nome do serviço, e vale também para o que já foi gravado.
- **Saudação** pela hora. A do Spotify seria a mesma frase, com um campo novo no protocolo.
- As setas das prateleiras vão para a linha do título (auditoria 29); "Músicas Curtidas" ganha capa (28).

## Tarefas

1. **Dados:** `TipoDaPrateleira` no `provedores` (desconhecido vira `outras`), nos dois provedores e no exemplo; `recentes` no `Acervo`
   (`registrar_recente`, `recentes`) e na `Lista`; `ajustes.servico_preferido`. Testes em cada crate.
2. **`player/src/juntar.rs`** (lógica): `juntar_inicio`, `intercalar`, `juntar_faixas`, `melhor`, `atalhos`, `saudacao`. Testes: juntar por tipo,
   intercalar, não repetir, serviço que não respondeu, serviço só com uma prateleira, ISRC igual, nota baixa, remix e ao vivo separados, o preferido na frente.
3. **`ui/selo.rs`:** a marca, o disco, o par e o botão de tocar; `app/salvos.rs`: tocar um item pelo cartão quando a página chega.
4. **Telas:** Início (saudação, atalhos, prateleiras juntas), Buscar (Principais e abas misturadas, o computador junto), Biblioteca, lateral, Fila,
   Histórico, Nesta sessão, barra, Ilha e o nome do serviço nas páginas de detalhe.
5. `fmt`, `clippy`, testes; cópia muda em 1920, 1440, 958 × 560 e 410; README, AGENTS, `docs/f1-provedores.md`; release (com os provedores).
