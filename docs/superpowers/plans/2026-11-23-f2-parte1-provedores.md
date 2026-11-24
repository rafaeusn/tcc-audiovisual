# F2, parte 1: identidade e páginas dos provedores — plano

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** os provedores mandam a identidade (cor e logo) e os ids de álbum e artista, e o Player pede e guarda as
páginas deles (início, álbum, artista, playlist), prontas para as telas das partes 3 e 4.

**Architecture:** campos novos e opcionais no protocolo v1 (`provedores::protocolo` e `provedores::modelo`); o
provedor do YouTube Music os preenche; `player/src/remoto.rs` ganha uma tarefa de página por provedor, um cache por
pedido e grava a logo na pasta de dados do provedor. Nada de tela nesta parte.

**Tech Stack:** Rust 2024, serde/serde_json, rustypipe (só no provedor), threads + `Arc<Mutex<_>>` como o resto do
`remoto.rs`.

**Spec:** `docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md` (seções 5.1, 5.7 e 6, item 1).

## Global Constraints

- Protocolo continua na versão 1: todo campo novo é opcional (`#[serde(default)]`, `skip_serializing_if`); JSON antigo
  sem os campos tem de ser lido.
- Nomes de código, comentários e textos em português; comentários dizem o porquê.
- A interface nunca espera thread: `remoto.rs` publica em `Estado` e a tela lê a cópia feita em `quadro()`.
- Logo: SVG em texto, até 32 KB (32 * 1024 bytes); cor em `#RRGGBB`.
- `cargo fmt --all`, `cargo clippy --workspace --all-targets` sem aviso, `cargo test --workspace` passando (o teste do
  `provedor-youtube-music` pode ser bloqueado pelo erro 4551: rodar à parte com `cargo test -p provedor-youtube-music`).
- Sem commit: o Rafael pede quando quiser.

## Review Focus

- Provedor antigo sem `identidade` e sem ids: tudo funciona, sem logo e sem links (teste na Task 1).
- Logo maior que 32 KB ou que não é SVG: ignorada, sem gravar arquivo (teste na Task 1 e na Task 3).
- Pedir a página de um serviço desligado ou que não abriu: vira `Erro` com frase, não fica "Carregando" para sempre
  (teste na Task 3).
- Pedir a mesma página duas vezes seguidas: a segunda usa o cache e não vai ao provedor (teste na Task 3).
- Faixa com mais nomes de artista que ids (ou o contrário): `id_do_artista(i)` devolve `None` em vez de errar o
  artista (teste na Task 1).

---

### Task 1: protocolo — identidade e ids

**Files:**
- Modify: `app/crates/provedores/src/protocolo.rs` (struct `Apresentacao`, testes)
- Modify: `app/crates/provedores/src/modelo.rs` (`Faixa`, `Album`, testes)
- Modify: `app/crates/provedores/src/lib.rs` (reexportar `Identidade`)
- Modify: `docs/f1-provedores.md` (descrever os campos novos)

**Interfaces:**
- Produces:
  - `pub struct Identidade { pub cor: Option<String>, pub logo: Option<String> }` (Serialize, Deserialize, Default,
    Clone, Debug, PartialEq; `#[serde(default)]`).
  - `impl Identidade { pub fn cor_rgb(&self) -> Option<u32>; pub fn logo_valida(&self) -> Option<&str> }`:
    `cor_rgb` aceita só `#RRGGBB` (maiúsculas ou minúsculas); `logo_valida` devolve o SVG se tiver até 32 KB e, sem
    espaços e sem declaração `<?xml ...?>` no começo, começar por `<svg`.
  - `pub const MAXIMO_DA_LOGO: usize = 32 * 1024;`
  - `Apresentacao.identidade: Option<Identidade>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`).
  - `Faixa.album_id: Option<String>`, `Faixa.artistas_ids: Vec<String>` (mesma ordem de `artistas`; `""` = sem id;
    pulado no JSON quando vazio).
  - `Album.artistas_ids: Vec<String>` (mesma regra).
  - `impl Faixa { pub fn id_do_artista(&self, i: usize) -> Option<&str> }` e o mesmo em `Album`: `None` se o índice
    não existir nos dois vetores ou o id for vazio.

- [ ] **Step 1: testes que falham** (em `protocolo.rs` e `modelo.rs`)

```rust
#[test]
fn apresentacao_antiga_sem_identidade_e_nova_com() {
    let antiga = r#"{"nome":"X","servico":"x","protocolo":1}"#;
    let a: Apresentacao = serde_json::from_str(antiga).expect("lê");
    assert_eq!(a.identidade, None);
    let nova = r##"{"nome":"X","servico":"x","protocolo":1,"identidade":{"cor":"#1ed760","logo":"<svg viewBox='0 0 1 1'/>"}}"##;
    let a: Apresentacao = serde_json::from_str(nova).expect("lê");
    let i = a.identidade.expect("tem");
    assert_eq!(i.cor_rgb(), Some(0x1ED760));
    assert!(i.logo_valida().is_some());
}

#[test]
fn cor_e_logo_invalidas_sao_ignoradas() {
    let i = |cor: &str, logo: &str| Identidade { cor: Some(cor.into()), logo: Some(logo.into()) };
    assert_eq!(i("verde", "").cor_rgb(), None);
    assert_eq!(i("#12345", "").cor_rgb(), None);
    assert_eq!(i("#GG0000", "").cor_rgb(), None);
    assert!(i("#000000", "<png/>").logo_valida().is_none());
    assert!(i("#000000", "<?xml version='1.0'?>\n<svg/>").logo_valida().is_some());
    let grande = format!("<svg>{}</svg>", "a".repeat(MAXIMO_DA_LOGO));
    assert!(i("#000000", &grande).logo_valida().is_none());
}

#[test]
fn ids_de_album_e_artista_opcionais() {
    let f: Faixa = serde_json::from_str(r#"{"servico":"s","id":"1","titulo":"T","artistas":["A","B"]}"#).expect("lê");
    assert_eq!(f.album_id, None);
    assert_eq!(f.id_do_artista(0), None);
    let f = Faixa { artistas_ids: vec!["a1".into(), String::new()], ..f };
    assert_eq!(f.id_do_artista(0), Some("a1"));
    assert_eq!(f.id_do_artista(1), None, "id vazio");
    assert_eq!(f.id_do_artista(5), None, "fora da lista");
    let json = serde_json::to_string(&Faixa { artistas_ids: Vec::new(), ..f.clone() }).expect("escreve");
    assert!(!json.contains("artistas_ids"), "vazio não vai no JSON");
}
```

- [ ] **Step 2:** `cargo test -p provedores` — falha (tipos e campos não existem).
- [ ] **Step 3:** implementar as interfaces acima. Atualizar os construtores de `Faixa`/`Album`/`Apresentacao` que o
  compilador apontar no workspace (`provedor-exemplo`, `remoto.rs` testes, `conversao.rs`) com `..Default::default()`
  ou os campos novos vazios. Em `docs/f1-provedores.md`, uma subseção "Identidade e ids (opcionais)" com o formato.
- [ ] **Step 4:** `cargo test -p provedores` — passa; `cargo build --workspace` compila.

### Task 2: o provedor do YouTube Music manda identidade e ids

**Files:**
- Modify: `app/provedores/youtube-music/src/conversao.rs`
- Modify: `app/provedores/youtube-music/src/main.rs` (`apresentacao`, `testar`)
- Create: `app/provedores/youtube-music/src/logo.svg`

**Interfaces:**
- Consumes: `Identidade`, `Faixa.album_id`, `Faixa.artistas_ids`, `Album.artistas_ids` (Task 1).
- Produces: `pub const COR: &str = "#FF0000";` em `conversao.rs`; `pub fn ids(artistas: &[(Option<&str>, &str)]) -> Vec<String>`
  (um id por nome, `""` sem id; vetor vazio se nenhum tiver id).

- [ ] **Step 1: teste que falha** em `conversao.rs`:

```rust
#[test]
fn ids_dos_artistas_na_mesma_ordem() {
    assert_eq!(ids(&[(Some("UC1"), "A"), (None, "B")]), vec!["UC1".to_string(), String::new()]);
    assert!(ids(&[(None, "A"), (None, "B")]).is_empty(), "nenhum id: vetor vazio");
}
```

- [ ] **Step 2:** `cargo test -p provedor-youtube-music` — falha.
- [ ] **Step 3:** `ids` e os campos: em `faixa()` preencher `album_id` com `t.album.as_ref().map(|a| a.id.clone())` e
  `artistas_ids` com `ids` sobre `t.artists` (`ArtistId { id: Option<String>, name }`), filtrando os mesmos nomes vazios
  que `artistas` já filtra; em `album_item()`, `artistas_ids` do mesmo jeito. Em `album()` do `main.rs`, as faixas
  herdam também `album_id = Some(id)`. `logo.svg`: a marca do serviço desenhada em SVG simples (círculo vermelho com
  anel e triângulo de play brancos, `viewBox="0 0 24 24"`), carregada com `include_str!`. `apresentacao()` ganha
  `identidade: Some(Identidade { cor: Some(COR.into()), logo: Some(include_str!("logo.svg").into()) })`. `testar()`
  imprime também quantas faixas vieram com `album_id` e com id de artista.
- [ ] **Step 4:** `cargo test -p provedor-youtube-music` — passa. Ao vivo:
  `cargo run -p provedor-youtube-music -- --teste "daft punk"` mostra as faixas com ids contados (> 0).

### Task 3: `remoto.rs` — logo, cor e páginas

**Files:**
- Modify: `app/player/src/remoto.rs`
- Modify: `app/player/src/ui/loja.rs` só se o compilador pedir (campos novos em `InfoDoProvedor`)

**Interfaces:**
- Consumes: `Identidade::cor_rgb`, `Identidade::logo_valida` (Task 1); `Provedor::inicio`, `Provedor::itens` (já
  existem em `provedores::cliente`).
- Produces:
  - `InfoDoProvedor.cor: Option<u32>` e `InfoDoProvedor.logo: Option<PathBuf>` (o arquivo
    `<dados>/provedores-dados/<serviço>/logo.svg`, gravado quando o provedor abre com logo válida; apagado se a logo
    sumir).
  - `#[derive(Clone, Debug, PartialEq, Eq, Hash)] pub enum PedidoDePagina { Inicio { servico: String }, Album { servico: String, id: String }, Artista { servico: String, id: String }, Playlist { servico: String, id: String } }` com `fn servico(&self) -> &str`.
  - `#[derive(Clone, Debug, PartialEq)] pub enum PaginaRemota { Carregando, Prateleiras(Vec<Prateleira>), Itens(Vec<Item>), Erro(String) }`.
  - `impl Remoto { pub fn abrir_pagina(&mut self, pedido: PedidoDePagina); pub fn pagina(&self, pedido: &PedidoDePagina) -> Option<PaginaRemota>; pub fn de_novo(&mut self, pedido: &PedidoDePagina) }`:
    `abrir_pagina` usa o cache se a página tiver menos de `VALIDADE_DA_PAGINA` (10 min) e não for `Erro`; senão marca
    `Carregando` e manda `Tarefa::Pagina` à thread do serviço (sem thread: `Erro("o provedor X não está instalado ou
    está desligado")` na hora). `de_novo` esquece o cache e pede outra vez. `pagina` lê a cópia feita em `quadro()`.
  - `fn vencida(desde: Instant, agora: Instant) -> bool` (pura, para o teste).
  - `fn guardar_logo(pasta: &Path, identidade: Option<&Identidade>) -> Option<PathBuf>`: grava `pasta/logo.svg` com a
    logo válida (criando a pasta) e devolve o caminho; sem logo válida, apaga o arquivo antigo e devolve `None`.
  - Sem a capacidade (`Inicio`, `Album`, `Artista`, `Playlists`), a thread responde `Erro("o X não tem esta página")`
    sem ir ao provedor.

- [ ] **Step 1: testes que falham** (no módulo `testes` de `remoto.rs`):

```rust
#[test]
fn pagina_de_servico_que_nao_existe_vira_erro_na_hora() {
    let mut r = Remoto::iniciar(None, &[]);
    let p = PedidoDePagina::Inicio { servico: "nenhum".into() };
    r.abrir_pagina(p.clone());
    r.quadro();
    assert!(matches!(r.pagina(&p), Some(PaginaRemota::Erro(m)) if m.contains("não está instalado")));
}

#[test]
fn pagina_vence_em_dez_minutos() {
    let agora = Instant::now();
    assert!(!vencida(agora, agora + Duration::from_secs(9 * 60)));
    assert!(vencida(agora, agora + Duration::from_secs(10 * 60 + 1)));
}

#[test]
fn provedor_que_nao_abre_responde_erro_na_pagina() {
    // Mesmo arranjo do teste de instalar: um "provedor" que é um arquivo qualquer.
    // Instala, espera a situação Falhou, pede Album { servico: "teste", id: "x" } e espera Some(PaginaRemota::Erro(_)).
}

#[test]
fn logo_valida_e_gravada_e_invalida_nao() {
    // Chama a função que grava a logo (`guardar_logo(pasta: &Path, identidade: Option<&Identidade>) -> Option<PathBuf>`)
    // com um SVG válido: o arquivo existe e o retorno aponta para ele; com "<png/>": retorna None e apaga o anterior.
}
```

- [ ] **Step 2:** `cargo test -p player remoto` — falha.
- [ ] **Step 3:** implementar. `Estado` ganha `paginas: HashMap<PedidoDePagina, (Instant, PaginaRemota)>`; `Remoto`
  ganha a cópia `paginas` atualizada em `quadro()` quando a versão muda. `Tarefa::Pagina(PedidoDePagina)`; na thread,
  `Inicio` usa `p.inicio(PRAZO)`, os outros `p.itens(CorpoDoPedido::Album { album: id }, PRAZO)` (e `Artista`,
  `Playlist`). `situar` grava `cor` e `logo` (por `guardar_logo` na pasta `dados` do provedor) quando ele abre.
- [ ] **Step 4:** `cargo test -p player remoto` — passa. Depois `cargo fmt --all`, `cargo clippy --workspace
  --all-targets` (zero avisos) e `cargo test --workspace --exclude visual --no-fail-fast`.

### Task 4: conferir de ponta a ponta e anotar

**Files:**
- Modify: `AGENTS.md` (seção 7: F2 parte 1; contagem de testes)
- Modify: `docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md` só se algo do desenho mudou

- [ ] **Step 1:** release (`cargo build --release -p player -p provedor-youtube-music`, renomeando o exe em uso do
  Rafael antes, `AGENTS.md` seção 5) e cópia de teste: o registro `player.log` ou o arquivo
  `<dados>/provedores-dados/youtube-music/logo.svg` mostra que a logo chegou. As páginas ainda não têm tela: conferir
  pelo teste ao vivo da Task 2 e pelo log.
- [ ] **Step 2:** anotar no `AGENTS.md` o que entrou, o que foi testado e o que não foi.
