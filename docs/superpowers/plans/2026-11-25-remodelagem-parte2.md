# Remodelagem, parte 2: navegação e busca única

Desenho: [`../specs/2026-11-25-remodelagem-streaming-design.md`](../specs/2026-11-25-remodelagem-streaming-design.md), seções 6, 7 e 9.
Execução nesta sessão, sem subagentes, com os testes da lógica escritos antes.

**Objetivo:** três destinos (Início, Buscar, Biblioteca) iguais em toda janela, o serviço como filtro no alto da página,
uma busca só (o campo do topo) e uma pilha só de voltar.

## Decisões desta parte

- Trocar o filtro ou a aba **não** vira passo do voltar: muda o passo de agora (e começa do topo). Cada passo guarda a rota
  (com a aba da Biblioteca), o filtro e a rolagem; voltar devolve os três.
- `Ctrl Tab` e `Ctrl 1..9` numa página sem filtro (álbum, Loja) ou com a folha fechada abrem o Início naquele filtro.
- Ordem dos filtros: Tudo, os serviços, Neste PC (antes: Tudo, computador, serviços).
- Voltar e avançar vão para o topo da janela; a folha perde a barra dela. O × da página só existe no Palco (na Estante
  sempre há página), e nas páginas que já o tinham (Loja, Configurações).
- A Biblioteca tem um filtro de texto em todas as abas; a Fila é uma página à parte (vai para o painel na parte 4).
- Barra de comandos por cima só com o visual sozinho (tela cheia, expandido, letra). Nas outras, `Ctrl K` e `/` levam ao campo.
- Um comando escolhido na Busca devolve a pessoa à página de antes, se ela veio pelo atalho.
- "Abertos há pouco" sai da pilha da sessão; o `recentes` do banco é da parte 3. As buscas recentes ficam em
  `ajustes.buscas_recentes` (até 8).
- Editar e Trocar layout saem da lateral e do trilho: continuam no teclado, na busca (`>`) e em Configurações › Aparência.

## Tarefas

1. **`navegacao.rs`**: `Rota::Inicio`, `Rota::Biblioteca(AbaDaBiblioteca)`, `Passo {id, rota, espaco}`, uma pilha;
   `ir`, `trocar_filtro`, `trocar_aba`, `voltar`, `avancar`, `fechar`, `garantir_pagina`, `soltar_preenchida`,
   `esquecer_servico`, `visitados`, `passo`; `Campo`, `Buscas`, `so_comandos`, `lembrar_busca`, `abas_da_biblioteca`, `aba_valida`.
   Testes: a pilha guarda rota, filtro e rolagem (id do passo); voltar depois de trocar o filtro; filtro em página sem
   filtro; provedor que sai no meio (filtro e detalhes dele somem da pilha, sem passos repetidos); `>` só comandos; a
   busca recente não repete (acento e caixa); aba que não vale no filtro.
2. **`local.rs`**: `Pedido::Procurar` → `Lista.achadas` e `Lista.procura` (a busca do topo não mexe no filtro da Biblioteca). Teste.
3. **`nucleo`**: `ajustes.buscas_recentes` com `validar`; `/` como atalho; nomes dos comandos.
4. **`app.rs`, `app/navegar.rs`, `ui/campo.rs`**: `campo: Option<Campo>`, texto selecionado (a próxima letra troca tudo),
   `abrir_busca`, `ir_ao_destino`, `trocar_espaco`, teclas da busca (↑ ↓ Enter nos comandos), rolagem por passo.
5. **`ui/destinos.rs`** (novo): lateral, trilho com nomes e barra de baixo saem da mesma lista, nos dois layouts.
   **`ui/espacos.rs`**: vira o filtro. **`ui/casca.rs`**: o topo novo. **`arranjo.rs`**: trilho 92, lateral na altura toda.
6. **`ui/busca.rs`**: vazio (recentes, abertos há pouco), abas sublinhadas, comandos, Neste PC.
   **`ui/biblioteca.rs`**: a Biblioteca única (abas, filtro de texto, ordem, grade ou lista). `ui/paginas.rs`: Início com Neste PC.
7. `fmt`, `clippy`, testes; cópia muda em 1920, 1440, 958 × 560, 700, 410; README, AGENTS; release.
