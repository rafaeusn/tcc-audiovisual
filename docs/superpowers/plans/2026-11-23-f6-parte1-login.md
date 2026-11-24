# F6, parte 1: o provedor do Spotify e o login — plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O Player abre o provedor do Spotify embutido, mostra "Entrar no Spotify" no espaço dele e, depois que a
pessoa aprova no navegador, sabe a conta (nome, Premium) e entra sozinho nas próximas aberturas.

**Architecture:** Um programa novo, `provedor-spotify` (`app/provedores/spotify`), fala o protocolo v1 com três
pedidos novos opcionais (`entrar`, `sair`, `conta`) e a `conta` na apresentação. O login corre numa thread do
provedor (o `servir` atende um pedido por vez e não pode esperar o navegador): `entrar` responde `ok` na hora e o
Player pergunta `conta` a cada 2 s até sair de `entrando`. O refresh token do OAuth fica cifrado com DPAPI
(`servicos::cofre`); a cada abertura ele vira um access token e uma sessão do librespot.

**Tech Stack:** Rust 2024, `librespot-oauth` e `librespot-core` 0.8 (crates.io), `tokio`, `windows` (dono da porta),
GPUI 0.2.2 no Player.

**Spec:** [`docs/superpowers/specs/2026-11-23-f6-spotify-design.md`](../specs/2026-11-23-f6-spotify-design.md), seções 5.1
e 5.7.

## Global Constraints

- Client ID público do librespot: `65b708073fc0480ea92a077233ca87bd`; retorno `http://127.0.0.1:8898/login`.
- Escopos pedidos: `streaming`, `user-read-private`, `playlist-read-private`, `playlist-read-collaborative`,
  `user-library-read`, `user-follow-read`, `user-read-playback-state`, `user-modify-playback-state` (as partes 2 a 5
  usam; pedir agora evita um segundo login).
- Nada de senha: quem digita é a pessoa, no navegador. Só o refresh token, cifrado, em
  `<TCCPLAYER_PROVEDOR_DADOS>/conta.dpapi`.
- Identidade: cor `#1DB954`; serviço `spotify`; nome `Spotify`.
- Protocolo continua na versão 1; campos e pedidos novos são opcionais. Um provedor sem `Capacidade::Conta` nunca
  recebe `entrar`, `sair` ou `conta`.
- Licença do provedor: `GPL-3.0-or-later` (librespot é MIT).
- Textos e nomes em português; `cargo fmt`, `clippy` sem aviso, testes; cópia de teste com `TCCPLAYER_DADOS`; não
  fechar o Player do Rafael (renomear o exe em uso).

## Review Focus

1. A porta 8898 ocupada (o Spotifast aberto): `entrar` responde erro na hora, com o nome do programa que a usa, e a
   conta volta a `fora` (não fica presa em `entrando`).
2. A pessoa fecha o navegador sem aprovar: o provedor desiste depois de 5 min, a conta volta a `fora` com o motivo,
   e o Player para de perguntar.
3. Refresh token guardado e recusado (revogado na conta): apaga o arquivo e fica `fora` com o motivo, sem laço.
4. Conta gratuita: `dentro` com `premium: false` (a parte 3 decide o que fazer); nada quebra.
5. Provedor antigo (o YouTube Music, sem `Conta`): o Player não pergunta `conta` e a apresentação sem o campo lê
   igual.

---

### Task 1: O protocolo da conta

**Files:**
- Modify: `app/crates/provedores/src/protocolo.rs` (pedidos, resposta, `Apresentacao.conta`)
- Modify: `app/crates/provedores/src/cliente.rs` (`conta`, `entrar`, `sair`)
- Modify: `app/crates/provedores/src/lib.rs` (reexportar `Conta`, `SituacaoDaConta`)
- Modify: `docs/f1-provedores.md` (a tabela de pedidos e a `conta`)

**Interfaces:**
- Produces:
  - `pub struct Conta { pub situacao: SituacaoDaConta, pub nome: Option<String>, pub premium: Option<bool>, pub erro: Option<String> }` (`Default` = `Fora`, opcionais com `skip_serializing_if`).
  - `pub enum SituacaoDaConta { Fora, Entrando, Dentro }` (`kebab-case`, `Default = Fora`).
  - `CorpoDoPedido::{Entrar, Sair, Conta}`; `CorpoDaResposta::Conta(Conta)`; `Apresentacao.conta: Option<Conta>` (`#[serde(default, skip_serializing_if = "Option::is_none")]`).
  - `Provedor::conta(&self, prazo) -> Resultado<Conta>`, `Provedor::entrar(&self, prazo) -> Resultado<()>`, `Provedor::sair(&self, prazo) -> Resultado<()>`.

- [ ] **Step 1: Testes** em `protocolo.rs`:

```rust
#[test]
fn conta_no_fio() {
    let p: Pedido = serde_json::from_str(r#"{"id":3,"pedido":"entrar"}"#).expect("entrar");
    assert_eq!(p.corpo, CorpoDoPedido::Entrar);
    let r = Resposta { id: 4, corpo: CorpoDaResposta::Conta(Conta {
        situacao: SituacaoDaConta::Dentro, nome: Some("rafael".into()), premium: Some(true), erro: None }) };
    assert_eq!(serde_json::to_string(&r).expect("json"),
        r#"{"id":4,"resposta":"conta","situacao":"dentro","nome":"rafael","premium":true}"#);
    let fora: Resposta = serde_json::from_str(r#"{"id":5,"resposta":"conta","situacao":"fora","erro":"x"}"#).expect("fora");
    assert!(matches!(fora.corpo, CorpoDaResposta::Conta(Conta { situacao: SituacaoDaConta::Fora, .. })));
}

#[test]
fn apresentacao_sem_conta_le_igual() {
    let a: Apresentacao = serde_json::from_str(r#"{"nome":"X","servico":"x","protocolo":1}"#).expect("antiga");
    assert_eq!(a.conta, None);
    let com: Apresentacao = serde_json::from_str(
        r#"{"nome":"S","servico":"spotify","protocolo":1,"conta":{"situacao":"entrando"}}"#).expect("nova");
    assert_eq!(com.conta.map(|c| c.situacao), Some(SituacaoDaConta::Entrando));
}
```

- [ ] **Step 2:** `cargo test -p provedores --lib protocolo` → não compila (tipos faltando).
- [ ] **Step 3:** Implementar os tipos e os três métodos do cliente (como `inicio`: `pedir` e casar a resposta; `Ok` para `entrar` e `sair`).
- [ ] **Step 4:** `cargo test -p provedores` → passa (os testes antigos também).
- [ ] **Step 5:** Documentar em `docs/f1-provedores.md`: linhas `entrar` (`ok` na hora, o login continua no provedor), `sair` (`ok`), `conta` (`conta`), e o campo `conta` da apresentação.

### Task 2: O provedor, a conta e o cofre (sem rede)

**Files:**
- Create: `app/provedores/spotify/Cargo.toml`, `src/main.rs`, `src/conta.rs`, `src/logo.svg`
- Test: no próprio `conta.rs` e `main.rs`

**Interfaces:**
- Consumes: Task 1 (`Conta`, `SituacaoDaConta`, pedidos).
- Produces:
  - `pub trait Autorizador: Send + Sync + 'static { fn autorizar(&self) -> Result<Tokens, String>; fn renovar(&self, refresh: &str) -> Result<Tokens, String>; fn conectar(&self, access: &str) -> Result<DadosDaConta, String>; fn porta_ocupada(&self) -> Option<String>; }` em `conta.rs`. `porta_ocupada` devolve `Some(frase)` quando a 8898 não está livre (a frase diz quem a usa).
  - `pub struct Tokens { pub access: String, pub refresh: Option<String> }`; `pub struct DadosDaConta { pub nome: String, pub premium: bool }`.
  - `pub struct Login { .. }` com `Login::novo(autorizador: Arc<dyn Autorizador>, cofre: PathBuf) -> Self` (se o cofre tem refresh token, já começa `Entrando` e renova numa thread), `estado(&self) -> Conta`, `entrar(&self) -> Result<(), String>`, `sair(&self)`.
  - Depois de `conectar`, `Login` guarda `DadosDaConta` e o access token para as partes seguintes: `Login::access(&self) -> Option<String>`.

- [ ] **Step 1: Testes** em `conta.rs`, com um `Autorizador` falso (contadores e respostas programadas):
  - `entrar_com_a_porta_ocupada_erra_na_hora`: `porta_ocupada` = `Some("a porta 8898 está com o Spotifast.exe")` → `entrar()` é `Err` com essa frase, `estado().situacao == Fora`, `autorizar` nunca chamado.
  - `entrar_e_aprovar_fica_dentro_e_guarda`: `autorizar` → `Tokens { access: "a", refresh: Some("r") }`, `conectar` → `("rafael", true)`; esperar (até 2 s) e conferir `Dentro`, nome, `premium == Some(true)`, e que `servicos::cofre::ler(cofre) == Some("r")` (no Windows; fora dele, só o estado).
  - `navegador_fechado_volta_para_fora`: `autorizar` → `Err("desistiu")` → `Fora` com `erro == Some("desistiu")`.
  - `refresh_recusado_apaga_e_fica_fora`: cofre com "velho", `renovar` → `Err(..)` → `Fora`, arquivo apagado, `renovar` chamado uma vez só.
  - `entrar_duas_vezes_nao_abre_dois_navegadores`: o segundo `entrar()` com `Entrando` devolve `Ok` sem chamar `autorizar` de novo.
  - `sair_apaga_o_cofre`: depois de `Dentro`, `sair()` → `Fora` e o arquivo some.
- [ ] **Step 2:** `cargo test -p provedor-spotify` → não compila.
- [ ] **Step 3:** Implementar `conta.rs` (estado num `Arc<Mutex<Conta>>`; o trabalho de `autorizar`/`renovar`/`conectar` numa thread por tentativa) e `Cargo.toml` (`provedores`, `nucleo`, `servicos`, `log`; licença `GPL-3.0-or-later`; `[lints] workspace = true`).
- [ ] **Step 4:** `main.rs`: `struct Spotify { login: Login }` com `Servico`: `apresentacao()` (nome, serviço, versão, `capacidades: vec![Capacidade::Conta]`, identidade com `#1DB954` e `logo.svg` — a marca do Spotify, um círculo com as três ondas, até 32 KB, e `conta: Some(login.estado())`), `atender` para `Conta`, `Entrar` (erro com a frase), `Sair`, e o erro de sempre para o resto. Teste `servir_responde_a_conta` com `servir` sobre linhas em memória (`ola`, `conta`) e um autorizador falso: a resposta da `conta` é `{"situacao":"fora"}`.
- [ ] **Step 5:** `cargo test -p provedor-spotify` → passa; `cargo clippy -p provedor-spotify --all-targets` sem aviso.

### Task 3: O Spotify de verdade (OAuth e sessão)

**Files:**
- Create: `app/provedores/spotify/src/spotify.rs` (o `Autorizador` real), `src/porta.rs` (quem usa a porta)
- Modify: `app/provedores/spotify/Cargo.toml`, `src/main.rs` (modo `--teste-login`)

**Interfaces:**
- Consumes: Task 2 (`Autorizador`, `Tokens`, `DadosDaConta`).
- Produces: `pub struct Real { .. }` com `Real::novo() -> Result<Self, String>` (runtime tokio próprio) implementando `Autorizador`; `porta::dono(porta: u16) -> Option<String>` (o nome do exe, pelo `GetExtendedTcpTable` + `QueryFullProcessImageNameW`; `None` fora do Windows ou sem dono).

- [ ] **Step 1:** Ler o código do `librespot-oauth` e do `librespot-core` 0.8 no registro (`D:/Rust/cargo/registry/src/*/librespot-oauth-0.8.0/src/lib.rs`) e confirmar os nomes: o construtor do cliente OAuth com Client ID, retorno e escopos; abrir o navegador; obter e renovar o token (e o prazo de espera, para usar 5 min); `Credentials::with_access_token`; `Session::new(SessionConfig, None)`, `connect(credentials, false)`, `username()` e o atributo `type` (`premium`). Se algum nome for outro, ledger `Ruling:` e seguir.
- [ ] **Step 2: Teste** `porta_livre_nao_tem_dono` em `porta.rs`: abrir um `TcpListener` em `127.0.0.1:0`, pegar a porta, conferir `dono(porta)` = o exe deste teste (termina com `.exe`); fechar e conferir `None`.
- [ ] **Step 3:** `cargo test -p provedor-spotify porta` → falha (não existe).
- [ ] **Step 4:** Implementar `porta.rs` e `spotify.rs`. `porta_ocupada`: tentar `TcpListener::bind("127.0.0.1:8898")`; se falhar, `Some(format!("A porta 8898, que o login do Spotify usa, está com o {dono}. Feche-o e tente de novo."))` (sem dono: "com outro programa"). `conectar` devolve o nome da conta e `premium` pelo atributo `type == "premium"`; a sessão fica guardada no `Real` (as partes 2 e 3 usam).
- [ ] **Step 5:** `cargo test -p provedor-spotify` → passa. `main` troca o falso pelo `Real` (o `Login` recebe `Arc<Real>`; se `Real::novo` falha, o provedor sai com a mensagem na saída de erro, como o do YouTube Music).
- [ ] **Step 6: Ao vivo (com o Rafael)** — `--teste-login`: imprime `porta_ocupada()`, chama `entrar`, espera e imprime a `conta`. Avisar antes: o navegador abre e ele aprova; o Spotifast precisa estar fechado (a porta). Esperado: `dentro`, o nome dele, `premium: true`; rodar de novo e entrar sem navegador (pelo refresh token).

### Task 4: O Player conhece a conta

**Files:**
- Modify: `app/player/src/remoto.rs` (`EMBUTIDOS`, `InfoDoProvedor.conta`, `Tarefa::{Entrar, Sair}`, perguntar a conta)
- Modify: `app/crates/provedores/src/bin/provedor-exemplo.rs` (conta simulada, para o teste)

**Interfaces:**
- Consumes: Task 1 (`Provedor::conta/entrar/sair`, `Apresentacao.conta`).
- Produces: `InfoDoProvedor.conta: Option<Conta>` (`None` = o provedor não tem `Conta`); `Remoto::entrar(&self, servico: &str)`, `Remoto::sair(&self, servico: &str)`; `EMBUTIDOS: [(&str, &str, &str); 2]` com `("spotify", "Spotify", "provedor-spotify.exe")`.

- [ ] **Step 1: Testes** em `remoto.rs`:
  - `embutidos_com_o_spotify`: numa pasta temporária com `provedor-youtube-music.exe` e `provedor-spotify.exe` vazios, `embutidos_em` devolve os dois serviços.
  - `entrar_ate_ficar_dentro`: com o `provedor-exemplo --conta` instalado (como os testes de hoje instalam o exemplo), a `conta` começa `Fora`; `entrar("exemplo")`; esperar (até 10 s, com `quadro()`) e conferir `Dentro` e o nome `"exemplo"`; `sair("exemplo")` → `Fora`.
- [ ] **Step 2:** `cargo test -p player remoto` → falha.
- [ ] **Step 3:** `provedor-exemplo --conta`: apresenta `Conta` nas capacidades e `conta: fora`; `entrar` → `ok` e passa a `entrando`; a segunda `conta` depois disso responde `dentro` (`nome: "exemplo"`, `premium: true`); `sair` → `fora`.
- [ ] **Step 4:** `remoto.rs`: ao abrir (`situar`), `info.conta = apresentacao.conta` só se `sabe(Capacidade::Conta)`. `Tarefa::Entrar` chama `entrar` e, com `ok`, pergunta `conta` a cada 2 s (no máximo 6 min) até sair de `entrando`, publicando cada resposta em `info.conta` e subindo a versão; erro do `entrar` vira `Conta { situacao: Fora, erro: Some(motivo) }`. `Tarefa::Sair` chama `sair` e publica `Fora`. Enquanto pergunta, a thread continua atendendo as outras tarefas (perguntar entre uma tarefa e outra, com `recv_timeout` de 2 s no laço).
- [ ] **Step 5:** `cargo test -p player remoto` → passa.

### Task 5: A tela de entrar

**Files:**
- Modify: `app/player/src/ui/paginas.rs` (Para você e Biblioteca do serviço)
- Modify: `app/player/src/ui/busca.rs` (o bloco do serviço)
- Modify: `app/player/src/ui/loja.rs` (a ficha do provedor na aba Provedores)
- Create: `app/player/src/contas.rs` (a lógica do que mostrar, testada)

**Interfaces:**
- Consumes: Task 4 (`InfoDoProvedor.conta`, `Remoto::entrar/sair`).
- Produces: `pub enum Aviso { Entrar { erro: Option<String> }, Entrando, Nada }` e `pub fn aviso_da_conta(conta: Option<&Conta>) -> Aviso` em `contas.rs`.

- [ ] **Step 1: Teste** `o_que_mostrar_da_conta` em `contas.rs`: `None` → `Nada`; `Fora` sem erro → `Entrar { erro: None }`; `Fora` com erro → `Entrar { erro: Some(..) }`; `Entrando` → `Entrando`; `Dentro` → `Nada`.
- [ ] **Step 2:** `cargo test -p player contas` → falha; implementar; passa.
- [ ] **Step 3:** No espaço de um serviço com `Aviso::Entrar`, Para você, Busca e Biblioteca mostram, no lugar do conteúdo do serviço, um cartão: a logo do serviço, "Entre na sua conta do <nome>", "Abre o navegador; quem entra é você. A senha não passa pelo Player.", o botão "Entrar no <nome>" (acento do espaço; chama `Remoto::entrar`) e, havendo, o erro embaixo em `texto_2`. Com `Aviso::Entrando`: "Aprove no navegador que abriu…" no lugar do botão. Em Tudo, o bloco do serviço sem conta mostra só uma linha "<nome>: entre na conta" com o mesmo botão.
- [ ] **Step 4:** Na aba Provedores da Loja, a ficha de um provedor com conta mostra "Conectado como <nome> · Premium" (ou "· Gratuita") e o botão "Sair" (chama `Remoto::sair`), ou "Não conectado" e "Entrar".
- [ ] **Step 5:** `cargo fmt --all`, `cargo clippy --workspace --all-targets` sem aviso, `cargo test --workspace --exclude visual` (e o `visual` à parte).
- [ ] **Step 6: Cópia de teste** (renomear o `player.exe` em uso; `TCCPLAYER_DADOS` numa pasta temporária; `PrintWindow`): o espaço do Spotify com a cor verde e a logo, o cartão "Entrar no Spotify", clicar → "Aprove no navegador…" (o Rafael aprova, com o Spotifast fechado) → as prateleiras vazias do Spotify (a parte 2 as enche) e a ficha da Loja com "Conectado como … · Premium"; fechar e abrir a cópia: entra sozinho. Com o Spotifast aberto: o erro da porta com o nome dele.
- [ ] **Step 7:** Atualizar `app/README.md` (o espaço do Spotify e o login), `AGENTS.md` (seção 4: o crate `provedores/spotify`; seção 7: a parte 1 da F6, o que foi e o que não foi conferido; seção 8: F6 em andamento).
