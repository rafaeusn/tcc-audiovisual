# F2, parte 4b: busca de todos os tipos, página da faixa, Fila e Histórico como páginas

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans. Steps use checkbox (`- [ ]`) syntax.

**Goal:** a Busca vira uma página da folha (faixas, álbuns, artistas e playlists, com filtro por tipo), a faixa ganha
página própria (Letra, Mapa e, com `Radio`, Parecidas), e Fila e Histórico viram rotas abertas de qualquer espaço.

**Architecture:** o provedor do YouTube Music passa a responder a busca com todos os tipos (o primeiro item é o melhor
resultado); `remoto::Busca` guarda os itens; `paginas.rs` agrupa (lógica testada); `ui/busca.rs` e `ui/faixa.rs`
desenham. A folha ganha as abas do espaço (Para você, Busca, Biblioteca).

**Tech Stack:** Rust 2024, GPUI 0.2.2, `rustypipe` 0.11.4.

**Spec:** `docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md` (5.2, 5.3 Busca e Faixa, 5.7 Fila e
Histórico, Parecidas).

## Global Constraints

- Protocolo v1: nada de campo obrigatório novo; "o primeiro item da busca é o melhor resultado" é convenção opcional
  (documentada em `docs/f1-provedores.md`).
- A Busca substitui o bloco "Nos serviços" da Biblioteca (lá fica um atalho para a página Busca com o mesmo texto).
- Parecidas só aparece com `Capacidade::Radio`; a F2 não cria o pedido de rádio.
- Letra e Mapa mostram o que o Player já tem (a letra da faixa que toca; a pré-análise que já existe na memória);
  sem isso, uma frase que diz quando aparece.
- Movimento reduzido para as animações (o `anim` já respeita); textos em português; `cargo clippy` sem aviso.

## Review Focus

1. Busca com dois provedores respondendo em tempos diferentes: a página não pisca nem perde o que já chegou.
2. Texto da busca trocado no meio: os resultados velhos não aparecem como se fossem do texto novo.
3. Item repetido entre a busca principal e a busca por tipo (mesmo id): aparece uma vez só.
4. Fila aberta pelo dock num espaço de serviço: abre a Fila (não a Biblioteca do serviço) e voltar volta à página
   de antes.
5. Página de uma faixa que não toca: Letra e Mapa dizem o que falta, sem travar nem mostrar a letra de outra faixa.

---

### Task 1: a busca do YouTube Music com todos os tipos

**Files:** `app/provedores/youtube-music/src/conversao.rs`, `app/provedores/youtube-music/src/main.rs`

**Interfaces:** Produces `conversao::juntar_busca(principal: Vec<Item>, por_tipo: Vec<Vec<Item>>, limite: usize) ->
Vec<Item>`: o primeiro da principal na frente; depois faixas, álbuns, artistas e playlists (de `por_tipo` e da
principal), sem repetir (tipo, id), no máximo `limite` de cada tipo.

- [ ] Teste `juntar_busca_poe_o_melhor_na_frente_e_nao_repete` (melhor = primeiro da principal; um álbum que vem nas
  duas listas aparece uma vez; `limite` corta por tipo).
- [ ] Ver falhar; implementar; `buscar` usa `music_search_main` + `music_search_tracks/albums/artists/playlists`
  ao mesmo tempo (`tokio::join!`), cada falha só fica de fora; erro só se todas falharem.
- [ ] `cargo test -p provedor-youtube-music`; `--teste "daft punk"` mostra os tipos.

### Task 2: os grupos da busca

**Files:** `app/player/src/remoto.rs`, `app/player/src/paginas.rs`

**Interfaces:** `remoto::Busca.itens: Vec<Item>` (na ordem de cada provedor; `faixas` continua). `paginas::FiltroDaBusca
{Tudo, Faixas, Albuns, Artistas, Playlists}` (`TODOS`, `nome()`), `paginas::GruposDaBusca { melhor: Option<Item>,
faixas: Vec<Faixa>, albuns, artistas, playlists: Vec<Item> }`, `paginas::agrupar_busca(itens: &[Item], servico:
Option<&str>) -> GruposDaBusca` (melhor = o primeiro item de cada serviço, o do primeiro serviço que respondeu),
`GruposDaBusca::filtros()` (Tudo e os tipos que têm resultado).

- [ ] Testes `agrupar_busca_por_tipo_e_servico`, `filtros_so_dos_tipos_com_resultado`.
- [ ] Implementar; o trabalhador guarda `itens` junto com `faixas`.

### Task 3: Fila e Histórico como rotas; as abas do espaço

**Files:** `navegacao.rs`, `app/navegar.rs`, `app.rs`, `ui/musicas.rs`, `ui/biblioteca.rs`, `ui/espacos.rs`,
`ui/heroi.rs`, `ui/dock.rs`

**Interfaces:** `Rota::Fila`, `Rota::Historico`; `Player::aba_biblioteca() -> AbaBiblioteca` (da rota, em vez do
campo); `navegacao::abas_do_espaco(&Espaco) -> Vec<(Rota, &'static str)>` (Computador: nenhuma; os outros: Para
você, Busca, Biblioteca).

- [ ] Testes `abas_de_cada_espaco` e `fila_abre_em_qualquer_espaco_e_voltar_volta`.
- [ ] `AbrirFila` → `abrir_rota(Rota::Fila)`; as abas Músicas, Fila e Histórico da Biblioteca navegam; num espaço
  de serviço Fila e Histórico mostram só o título (sem a aba Músicas). O topo da folha mostra as abas do espaço.

### Task 4: a página Busca

**Files:** `ui/busca.rs` (novo), `ui/musicas.rs`, `ui/heroi.rs`, `app.rs`, `ui/campo.rs`

- [ ] O campo de busca sai de `ver_musicas` para um ajudante (`campo_de_busca`), usado nas duas páginas (o texto é
  o mesmo `busca_musicas`). Abrir a Busca põe o teclado no campo.
- [ ] Vazia: a frase do que dá para buscar. Buscando: esqueleto. Erros: a frase com o nome do serviço. Resultados:
  "Melhor resultado" (cartão grande) e as 4 primeiras faixas lado a lado (empilhados em folha estreita), depois as
  prateleiras de álbuns, artistas e playlists; chips filtram (faixas em lista, os outros em grade). Em Tudo, cada
  serviço no seu bloco, e "N músicas neste computador" leva à Biblioteca.
- [ ] O bloco "Nos serviços" da Biblioteca vira o atalho "Buscar \"texto\" nos serviços".

### Task 5: a página da faixa

**Files:** `ui/faixa.rs` (novo), `paginas.rs`, `ui/paginas.rs`

**Interfaces:** `paginas::AbaDaFaixa {Letra, Mapa, Parecidas}`, `paginas::abas_da_faixa(sabe_radio: bool) ->
Vec<AbaDaFaixa>`, `paginas::rota_do_album(faixa, sabe_album) -> Option<Rota>`.

- [ ] Testes `parecidas_so_com_radio`, `link_do_album_so_com_id_e_capacidade`.
- [ ] Cabeçalho (capa, título, artistas e álbum como links, Tocar, coração, a seguir, fila); abas com indicador.
  Letra: as linhas da letra quando a faixa é a que toca (a cantada em destaque), senão a frase. Mapa: a energia da
  pré-análise em barras, trechos (Calma, Agitada, Intensa), drops, BPM e tom; senão a frase.
- [ ] O título na linha de faixa e no cartão do melhor resultado abre a página da faixa.

### Task 6: revisão, cópia de teste e documentos

- [ ] `cargo fmt`, `clippy` sem aviso, `cargo test --workspace`; revisor novo (opus) no diff da parte; corrigir o
  que for Crítico ou Importante com teste.
- [ ] Cópia de teste com o YouTube Music: busca com os tipos, filtro, melhor resultado, página da faixa (Letra e
  Mapa da que toca), Fila pelo dock num espaço de serviço, Histórico.
- [ ] `app/README.md`, `AGENTS.md` (seções 7 e 8), `docs/f1-provedores.md`.
