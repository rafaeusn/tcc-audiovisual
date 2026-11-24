# F2, parte 2: curtir, salvar, fixar e playlists mistas — plano

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** o acervo guarda o que a pessoa curtiu e salvou de qualquer serviço (com fixar) e as playlists do Player
aceitam faixas de provedor ao lado das locais.

**Architecture:** tudo no crate `biblioteca` (SQLite do `biblioteca.db`), sem depender do crate `provedores`: o salvo
guarda o item do provedor como texto opaco (JSON escrito pelo Player). As faixas de provedor das playlists ficam numa
tabela própria de metadados (`faixas_remotas`, chave `servico:id`, a mesma do `Musica::caminho` remoto) e entram na
consulta da playlist ao lado das locais, com `Musica::remota` preenchida. O Player passa a mandar `Musica` (e não só o
caminho) ao criar playlist e ao pôr músicas nela. Telas de curtir e fixar ficam para as partes 3 e 4.

**Tech Stack:** Rust 2024, rusqlite 0.37.

**Spec:** `docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md` (5.4, 5.7 "Playlists do Player" e
"Fixar"; seção 6, item 2).

## Global Constraints

- Nada de áudio no banco: só metadados (regra 2 do `AGENTS.md`).
- Banco existente abre sem perder nada: tabelas novas com `CREATE TABLE IF NOT EXISTS`; nenhuma tabela antiga muda de
  forma.
- Nomes e comentários em português; `cargo fmt`, `clippy` sem aviso, testes passando. Sem commit.

## Review Focus

- Música de provedor numa playlist, depois a mesma faixa adicionada de novo: não duplica (teste na Task 2).
- Playlist só com faixas de provedor (nenhum arquivo local): aparece com a contagem certa e lista na ordem (Task 2).
- Busca dentro de uma playlist mista acha a faixa remota pelo título (Task 2).
- Salvar de novo um item já salvo e fixado: atualiza os dados, mantém fixado e a data de quando foi salvo (Task 1).
- Fixar um item que não está salvo: não cria nada e devolve `false` (Task 1).

---

### Task 1: salvos (curtir, salvar, fixar)

**Files:**
- Modify: `app/crates/biblioteca/src/acervo.rs` (esquema e métodos, testes)
- Modify: `app/crates/biblioteca/src/lib.rs` (tipos `TipoSalvo`, `Salvo`)

**Interfaces:**
- Produces:
  - `#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)] pub enum TipoSalvo { Faixa, Album, Artista, Playlist }` com
    `pub fn chave(self) -> &'static str` (`"faixa"`, `"album"`, `"artista"`, `"playlist"`) e
    `pub fn da_chave(s: &str) -> Option<Self>`.
  - `#[derive(Clone, Debug, PartialEq)] pub struct Salvo { pub servico: String, pub tipo: TipoSalvo, pub id: String, pub dados: String, pub quando: i64, pub fixado: bool }`
    (`dados`: o item em JSON, escrito por quem salva; `quando`: segundos Unix). Faixa local: `servico = "computador"`,
    `id` = caminho.
  - Tabela `salvos(servico TEXT, tipo TEXT, id TEXT, dados TEXT, quando INTEGER, fixado INTEGER, PRIMARY KEY (servico, tipo, id))`.
  - `impl Acervo`:
    - `pub fn salvar(&self, s: &Salvo) -> Resultado<()>`: novo entra como veio; já salvo troca só `dados` (mantém
      `quando` e `fixado`).
    - `pub fn tirar_salvo(&self, servico: &str, tipo: TipoSalvo, id: &str) -> Resultado<Option<Salvo>>`: devolve o que
      saiu (para o "Desfazer").
    - `pub fn esta_salvo(&self, servico: &str, tipo: TipoSalvo, id: &str) -> Resultado<bool>`.
    - `pub fn salvos(&self, servico: Option<&str>, tipo: Option<TipoSalvo>) -> Resultado<Vec<Salvo>>`: fixados
      primeiro, depois do mais novo ao mais antigo.
    - `pub fn fixar(&self, servico: &str, tipo: TipoSalvo, id: &str, fixado: bool) -> Resultado<bool>`: `false` se não
      está salvo.

- [ ] **Step 1: testes que falham** (em `acervo.rs`):

```rust
#[test]
fn salvar_tirar_e_fixar() {
    let a = Acervo::em_memoria().expect("acervo");
    let s = |servico: &str, tipo, id: &str, quando| Salvo {
        servico: servico.into(), tipo, id: id.into(), dados: format!("{{\"id\":\"{id}\"}}"), quando, fixado: false,
    };
    a.salvar(&s("youtube-music", TipoSalvo::Faixa, "f1", 10)).expect("salvar");
    a.salvar(&s("youtube-music", TipoSalvo::Album, "a1", 20)).expect("salvar");
    a.salvar(&s("computador", TipoSalvo::Faixa, "C:/m.mp3", 30)).expect("salvar");
    assert!(a.esta_salvo("youtube-music", TipoSalvo::Faixa, "f1").expect("ler"));
    let ids = |v: Vec<Salvo>| v.into_iter().map(|s| s.id).collect::<Vec<_>>();
    assert_eq!(ids(a.salvos(None, None).expect("ler")), ["C:/m.mp3", "a1", "f1"], "mais novo primeiro");
    assert_eq!(ids(a.salvos(Some("youtube-music"), Some(TipoSalvo::Faixa)).expect("ler")), ["f1"]);
    assert!(a.fixar("youtube-music", TipoSalvo::Faixa, "f1", true).expect("fixar"));
    assert_eq!(ids(a.salvos(None, None).expect("ler")), ["f1", "C:/m.mp3", "a1"], "fixado primeiro");
    // Salvar de novo troca os dados e mantém quando e fixado.
    let mut de_novo = s("youtube-music", TipoSalvo::Faixa, "f1", 99);
    de_novo.dados = "{\"novo\":true}".into();
    a.salvar(&de_novo).expect("salvar");
    let f1 = a.salvos(Some("youtube-music"), Some(TipoSalvo::Faixa)).expect("ler").remove(0);
    assert_eq!((f1.quando, f1.fixado, f1.dados.as_str()), (10, true, "{\"novo\":true}"));
    assert!(!a.fixar("youtube-music", TipoSalvo::Artista, "nada", true).expect("fixar"), "não salvo: nada");
    let saiu = a.tirar_salvo("youtube-music", TipoSalvo::Faixa, "f1").expect("tirar").expect("estava");
    assert!(saiu.fixado);
    assert!(!a.esta_salvo("youtube-music", TipoSalvo::Faixa, "f1").expect("ler"));
    assert_eq!(a.tirar_salvo("youtube-music", TipoSalvo::Faixa, "f1").expect("tirar"), None);
}
```

- [ ] **Step 2:** `cargo test -p biblioteca` — falha (tipos não existem).
- [ ] **Step 3:** implementar.
- [ ] **Step 4:** `cargo test -p biblioteca` — passa.

### Task 2: playlists com faixas de provedor

**Files:**
- Modify: `app/crates/biblioteca/src/acervo.rs` (tabela `faixas_remotas`, `guardar_remotas`, consulta da playlist,
  `COLUNAS`/`musica_da_linha`, `playlists()`)
- Modify: `app/crates/biblioteca/src/lib.rs` (`Playlist.remotas: u32`)
- Modify: `app/player/src/local.rs`, `app/player/src/app.rs` (`JanelaPlaylist.musicas`), `app/player/src/ui/musicas.rs`,
  `app/player/src/ui/playlist.rs`

**Interfaces:**
- Produces:
  - Tabela `faixas_remotas(caminho TEXT PRIMARY KEY, titulo, artista, album, duracao_s REAL, servico, remoto_id, isrc, capa, fonte, busca)`.
  - `pub fn guardar_remotas(&mut self, musicas: &[Musica]) -> Resultado<usize>`: as que têm `remota` entram ou são
    atualizadas (chave `caminho`); devolve quantas. As locais são ignoradas.
  - `Origem::Playlist(id)` lista locais (do acervo) e remotas (de `faixas_remotas`) na ordem de `posicao`, com
    `Musica::remota` preenchida nas remotas; a busca por palavras vale para as duas.
  - `Playlist { id, nome, quantas, remotas }`: `quantas` conta locais presentes + remotas guardadas; `remotas` só as
    remotas (Computador mostra as playlists com `remotas == 0`).
  - `local.rs`: `Pedido::CriarPlaylist { nome, musicas: Vec<Musica>, abrir }` e `Pedido::AdicionarAPlaylist(i64, Vec<Musica>)`
    chamam `guardar_remotas` antes de `adicionar_a_playlist(id, &caminhos)`. `Local::criar_playlist` e
    `Local::adicionar_a_playlist` recebem `Vec<Musica>`. `JanelaPlaylist.musicas: Vec<Musica>`.

- [ ] **Step 1: testes que falham** (em `acervo.rs`):

```rust
#[test]
fn playlist_com_faixas_de_provedor() {
    let mut a = acervo(); // o ajudante de teste que já existe (pasta C:/Musicas com músicas)
    let remota = |id: &str, titulo: &str| Musica {
        caminho: PathBuf::from(format!("youtube-music:{id}")),
        titulo: titulo.into(), artista: "Daft Punk".into(), duracao_s: 320.0,
        remota: Some(Remota { servico: "youtube-music".into(), id: id.into(), isrc: None,
            capa: Some("https://capa".into()), fonte: "YouTube Music".into() }),
        ..Default::default()
    };
    let id = a.criar_playlist("Mista").expect("criar");
    assert_eq!(a.guardar_remotas(&[remota("r1", "Around the World"), musica("C:/x.mp3", "Local", "A", "", 1)]).expect("guardar"), 1);
    a.adicionar_a_playlist(id, &[PathBuf::from("youtube-music:r1"), PathBuf::from("C:/Musicas/a.mp3")]).expect("adicionar");
    a.adicionar_a_playlist(id, &[PathBuf::from("youtube-music:r1")]).expect("de novo não duplica");
    let lista = Filtro { origem: Origem::Playlist(id), ..Default::default() };
    let m = a.buscar(&lista, 100).expect("buscar");
    assert_eq!(m.len(), 2);
    assert_eq!(m[0].titulo, "Around the World");
    assert_eq!(m[0].remota.as_ref().map(|r| r.id.as_str()), Some("r1"));
    assert_eq!(m[0].remota.as_ref().and_then(|r| r.capa.as_deref()), Some("https://capa"));
    assert!(m[1].remota.is_none());
    let p = a.playlists().expect("playlists").into_iter().find(|p| p.id == id).expect("tem");
    assert_eq!((p.quantas, p.remotas), (2, 1));
    let achou = a.buscar(&Filtro { consulta: "around".into(), ..lista.clone() }, 100).expect("buscar");
    assert_eq!(achou.len(), 1, "a busca acha a remota");
    let so_remota = a.criar_playlist("Só serviço").expect("criar");
    a.guardar_remotas(&[remota("r2", "One More Time")]).expect("guardar");
    a.adicionar_a_playlist(so_remota, &[PathBuf::from("youtube-music:r2"), PathBuf::from("youtube-music:r1")]).expect("adicionar");
    let titulos: Vec<String> = a.buscar(&Filtro { origem: Origem::Playlist(so_remota), ..Default::default() }, 100)
        .expect("buscar").into_iter().map(|m| m.titulo).collect();
    assert_eq!(titulos, ["One More Time", "Around the World"], "na ordem em que entraram");
}
```

  O teste antigo `playlists` compara `Playlist { id, nome, quantas }`: ganha `remotas: 0`.
- [ ] **Step 2:** `cargo test -p biblioteca` — falha.
- [ ] **Step 3:** implementar o acervo; depois a troca para `Vec<Musica>` no Player (o compilador aponta os lugares;
  na linha de música, `numa_lista` passa a ser a `Musica`).
- [ ] **Step 4:** `cargo test -p biblioteca`, `cargo build -p player`, `cargo clippy --workspace --all-targets` sem
  aviso e `cargo test --workspace --exclude visual --no-fail-fast`.

### Task 3: revisão e anotações

- [ ] Revisor novo no fim (como na parte 1); corrigir os importantes com teste.
- [ ] `AGENTS.md` seção 7 (parte 2) e a contagem de testes.
