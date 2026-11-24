# F2, parte 4a: capas, cartões e as páginas dos serviços — plano

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** no Palco, "Para você" vira prateleiras de cartões com capa; clicar num álbum, numa playlist ou num artista
abre a página dele (com voltar), e dá para tocar dali, curtir e salvar; a Biblioteca de um serviço mostra o que foi
salvo dele, com fixar.

**Architecture:** um baixador de capas com cache em disco (`player/src/capas.rs`: threads próprias, a tela pede e lê
o caminho do arquivo pronto, desenhado com `img`); ajudantes puros (`player/src/paginas.rs`: o que mostrar de cada
`Item`, a rota e o pedido de cada item, a chave do salvo); a thread da biblioteca (`local.rs`) ganha os pedidos de
salvar, tirar e fixar e publica os salvos na `Lista`; as telas em `player/src/ui/paginas.rs`.

**Tech Stack:** Rust 2024, GPUI 0.2.2 (`img` de arquivo, `overflow_x_scroll`), `tocador::rede::baixar_pequeno`,
serde_json.

**Spec:** `docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md` (5.3, 5.4, 5.7; seção 6, item 4).

## Global Constraints

- Toda página abre na hora, com esqueleto, e se enche quando o provedor responde; erro vira frase com "Tentar de
  novo". Nada espera thread.
- Capacidade que o provedor não declara esconde o que depende dela (sem `Artista`, nome de artista não é link).
- Micro-interações: página nova entra deslizando ~24 px e aparecendo; capa cresce 3% no hover; o coração pulsa numa
  mola ao curtir; com movimento reduzido ou modo seguro, nada desliza.
- Capas: só a imagem (nunca áudio), no máximo 8 MB por capa, cache em `<dados>/capas/`.
- `cargo fmt`, `clippy` sem aviso, testes; cópia de teste por `PrintWindow`. Sem commit.

## Review Focus

- Item sem capa (ou capa que não baixa): o cartão mostra a silhueta do tipo, sem buraco e sem tremer (Task 1/3).
- A mesma capa pedida por vários cartões ao mesmo tempo: baixa uma vez só (Task 1).
- Álbum ou playlist que o provedor devolve vazio: a página diz isso, não fica no esqueleto (Task 4).
- Curtir duas vezes seguidas rápido: fica no estado da última, sem duplicar (Task 2).
- Faixa sem `id_do_artista`: o nome do artista é texto, não link (Task 1, `paginas.rs`).

---

### Task 1: `capas.rs` e `paginas.rs` (lógica)

**Files:** Create `app/player/src/capas.rs`, `app/player/src/paginas.rs`; Modify `app/player/src/main.rs`,
`app/player/Cargo.toml` (`serde_json = "1"`).

**Interfaces (Produces):**
- `capas.rs`:
  - `pub fn nome_do_arquivo(url: &str) -> String` (FNV-1a de 64 bits em hexadecimal + `.img`).
  - `pub struct Capas` com `pub fn iniciar(pasta: Option<PathBuf>) -> Self` (2 threads), `pub fn arquivo(&self, url: &str) -> Option<PathBuf>`:
    devolve o arquivo se já está em disco; senão pede (uma vez por endereço) e devolve `None`; `pub fn versao(&self) -> u64`
    (muda quando uma capa fica pronta, para a tela redesenhar). Sem pasta de dados, nada baixa.
- `paginas.rs`:
  - `pub struct Cartao { pub titulo: String, pub sub: String, pub capa: Option<String>, pub redondo: bool, pub tipo: TipoSalvo }`;
    `pub fn cartao(item: &Item) -> Cartao` (faixa: artistas; álbum: artistas · ano; artista: "Artista", redondo;
    playlist: dono · N faixas).
  - `pub fn rota_do_item(item: &Item) -> Option<Rota>` (álbum → `Album`, playlist → `Playlist`, artista → `Artista`;
    faixa → `None`: clicar toca).
  - `pub fn pedido_da_rota(rota: &Rota) -> Option<PedidoDePagina>`.
  - `pub fn chave_do_salvo(item: &Item) -> (String, TipoSalvo, String)` (serviço, tipo, id).
  - `pub fn rota_do_artista(faixa: &Faixa, i: usize, sabe_artista: bool) -> Option<Rota>` (só com id e com a
    capacidade).
  - `pub fn faixas_e_albuns(itens: &[Item]) -> (Vec<Faixa>, Vec<Item>)` (página do artista).

- [ ] **Step 1: testes que falham** — em `capas.rs`: `nome_do_arquivo` estável, diferente para endereços diferentes, só
  `[0-9a-f]` + `.img`; um teste com pasta temporária em que o arquivo já existe: `arquivo()` devolve o caminho sem
  pedir nada. Em `paginas.rs`: `cartao` dos 4 tipos (artista redondo; playlist com "12 faixas"); `rota_do_item` (faixa
  → `None`); `pedido_da_rota(Album)` → `PedidoDePagina::Album { servico, id }`; `rota_do_artista` sem id → `None`,
  sem capacidade → `None`, com os dois → `Some(Rota::Artista(..))`; `faixas_e_albuns` separa na ordem.
- [ ] **Step 2:** rodar e ver falhar. **Step 3:** implementar. **Step 4:** passar.

### Task 2: salvos na thread da biblioteca

**Files:** Modify `app/player/src/local.rs`.

**Interfaces:** `Pedido::Salvar(Salvo)`, `Pedido::TirarSalvo(String, TipoSalvo, String)`, `Pedido::Fixar(String, TipoSalvo, String, bool)`;
`Lista.salvos: Arc<Vec<Salvo>>` (todos, na ordem do acervo); `Local::salvar(&self, item: &Item)` (o `Item` em JSON,
`quando` = agora), `Local::tirar_salvo`, `Local::fixar`, `Local::esta_salvo(&self, item: &Item) -> bool` (procura em
`lista.salvos`). Depois de cada pedido, a lista é publicada de novo.

- [ ] Teste de unidade da conversão `Item` → `Salvo` e de volta (`salvo_de(item, quando)` e `item_do_salvo(&Salvo) -> Option<Item>`)
  que falha antes; depois implementar e passar.

### Task 3: cartões, prateleiras e "Para você"

**Files:** Create `app/player/src/ui/paginas.rs`; Modify `app/player/src/ui/espacos.rs` (troca o texto pelas
prateleiras), `app/player/src/ui/mod.rs`, `app/player/src/app.rs` (o `Player` ganha `capas: Capas`).

- Cartão de 160 px: capa (raio 7, ou círculo no artista) com a silhueta do tipo enquanto não há capa; título (14,
  semibold, 1 linha) e sub (12, 2 linhas). Hover: capa a 1,03 (mola). Clicar: faixa toca a prateleira a partir dela
  (`tocar_lista` com as faixas da prateleira); os outros abrem a rota.
- Prateleira: título (18) e a fileira rolando na horizontal.
- Esqueleto: 6 cartões cinza com brilho lento enquanto `Carregando`.
- Em Tudo, cada serviço é uma seção com o selo e o nome.

### Task 4: álbum, playlist e artista

**Files:** Modify `app/player/src/ui/paginas.rs`, `app/player/src/ui/heroi.rs` (as rotas de detalhe vão para as
páginas novas), `app/player/src/app/navegar.rs` (pedir a página da rota aberta).

- Cabeçalho: capa 160 (círculo no artista), tipo em versalete, título 32–40 (encolhe com o tamanho), o artista como
  link quando há id e capacidade; botões Tocar (acento do espaço), Aleatório e Salvar/Seguir (coração que pulsa).
- Lista de faixas (linha de 56 px: número, título, artista, duração, coração; "a seguir" e "fila" no hover); clicar
  toca a partir dela com a lista inteira na fila.
- Artista: "Populares" (5, com "Ver mais") e "Álbuns" em grade de cartões.
- Vazio: "O {serviço} não mandou faixas para este álbum."; erro: frase e "Tentar de novo".
- Página nova entra deslizando 24 px e aparecendo (240 ms).

### Task 5: a Biblioteca do espaço e fixar

**Files:** Modify `app/player/src/ui/paginas.rs`, `app/player/src/ui/heroi.rs`.

- No espaço de um serviço, a rota `Biblioteca` mostra os salvos dele: chips Tudo/Faixas/Álbuns/Artistas/Playlists e a
  grade de cartões (fixados primeiro, com um alfinete); no hover, o botão de fixar (círculo de 32 px no canto da
  capa). Sem salvos: "Curta faixas e salve álbuns do {serviço}: eles aparecem aqui." Em Tudo e no computador, a
  Biblioteca de hoje.

### Task 6: conferir e anotar

- [ ] Cópia de teste com o YouTube Music: as prateleiras com capa, um álbum aberto e tocado, voltar, curtir (o
  coração e a Biblioteca do serviço), fixar; revisor novo; `AGENTS.md`, `app/README.md`.
