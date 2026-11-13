# AGENTS.md: contexto para qualquer agente de IA

Leia isto antes de mexer no projeto. Vale para Cursor, Claude Code, Codex ou um chat novo. Atualizado em
13/11/2026. **Responda sempre em português do Brasil.**

Detalhes de cada função do app: [`app/README.md`](app/README.md). História do projeto e decisões de produto:
[`CONTEXTO-TCC.md`](CONTEXTO-TCC.md). Plano original da v1 (fases 0–6): [`docs/plano-v1.md`](docs/plano-v1.md).

---

## 1. O que é

TCC do Rafael: um **player audiovisual para Windows**, em Rust. Duas formas de uso:

- **Modo Escuta**: analisa o som que o PC está tocando (Spotify, navegador, qualquer app) e move um visual
  artístico em tempo real. Lê o "tocando agora" pela sessão de mídia do Windows (SMTC). Não usa a API de
  nenhum serviço.
- **Player local**: toca as músicas das pastas do computador (MP3, FLAC, WAV, OGG, M4A).

Por cima disso: painel em módulos (ritmo, energia, letra...), histórico e estatísticas locais, letras do LRCLIB,
Loja de temas e módulos, integração com o Windows (bandeja, janela mini, botões da barra de tarefas) e módulos
Last.fm e Discord.

O código está em **`app/`** (workspace Cargo). O resto do repositório é material anterior: `preview/` (protótipo
WebGL em JS, gabarito do visual e do diretor), `mockup/` (mockup da UI), `prototypes/`, `touchdesigner-mcp/` e
`docs/`.

---

## 2. Regras que não se negociam

1. **Nunca baixar áudio do YouTube ou do Spotify** (yt-dlp e parecidos). Viola os termos de uso.
2. **Spotify só pelo Modo Escuta** (captura do som + SMTC). Nada de API do Spotify nem de "Soloist", porque a
   política deles proíbe sincronizar visual com o conteúdo pela API.
3. **Nenhum segredo no código nem no git.** As chaves `LASTFM_API_KEY`, `LASTFM_API_SECRET` e `DISCORD_APP_ID`
   vêm só de variável de ambiente (lidas ao abrir o app ou por `option_env!` na compilação). Nunca pedir nem
   colar chave no chat.
4. **Sem MP3/FLAC/WAV no git** (o `.gitignore` já bloqueia).
5. **Não reproduzir letra de música** no chat nem em documentos.
6. **Não mexer nas configurações de segurança do Windows** (o Controle Inteligente de Aplicativos é escolha do
   usuário).
7. **Commit e push só quando o Rafael pedir**, direto no `main`. Mensagem em português; terminar com a linha de
   coautoria do agente que fez o trabalho.
8. **Não fechar o Player do Rafael** para compilar ou testar (ver seção 5).
9. O Rafael não quer aula de Rust. Faça o trabalho e termine com um resumo curto e simples.

---

## 3. Como compilar e testar

Dentro de `app/`:

```bash
cargo run -p player                      # abre o app (debug)
cargo build --release -p player          # release (D:/Rust/target/release/player.exe)
cargo test --workspace                   # 173 testes em 13/11/2026, todos passando
cargo clippy --workspace --all-targets   # tem que sair sem aviso
cargo fmt --all
```

- A pasta de build é `D:/Rust/target`, e o cache do cargo fica em `D:/Rust/cargo`.
- `app/.cargo/config.toml` aponta o `fxc.exe` do Windows SDK 10.0.22621, que o GPUI usa para os shaders.
- Rust edition 2024, MSRV 1.88. Lints do workspace: `unwrap_used`, `expect_used`, `panic`, `todo` e `dbg_macro`
  geram aviso; `unsafe_op_in_unsafe_fn` é erro. Em teste, use `#[allow(clippy::expect_used)]` no módulo.
- Variáveis de ambiente para teste:
  - `TCCPLAYER_DADOS=<pasta>`: pasta de dados separada (ajustes, bancos, registro). Permite rodar uma cópia ao
    lado do Player do Rafael.
  - `TCCPLAYER_MUDO=1`: o player local toca sem som.
  - `TCCPLAYER_PASTA=<pasta1>;<pasta2>`: pastas de música que entram na Biblioteca ao abrir.
- Teste manual da captura por app: `cargo run -p audio --example ouvir_app -- opera.exe spotify.exe`.
- Registro: `<pasta de dados>/registros/player.log`. A pasta de dados padrão é `%APPDATA%\TccPlayer`.

---

## 4. Arquitetura

### Crates (`app/crates/*` e `app/player`)

| Crate | O que faz |
| --- | --- |
| `nucleo` | Erros, caminhos, `Ajustes` (TOML com gravação atômica), `Comando` (todas as ações do app), registro |
| `audio` | Captura do som: loopback da saída padrão (`cpal`) ou de um app só (process loopback, `processo.rs`); anel sem trava; volume do sistema |
| `analise` | FFT, descritores, ritmo (BPM e batida) e o diretor automático (port de `preview/director.js`) |
| `sessao` | SMTC: "tocando agora", capa e controles; filtro por app (`Sessao::filtrar`) |
| `visual` | Os visuais em wgpu: Spectral (262 mil partículas), Fluxo e Barras, mais o modo seguro |
| `janela` | Camada DirectComposition por baixo da interface, onde o visual é desenhado sem cópia |
| `historico` | SQLite local: execuções, estatísticas e cache de letras |
| `letras` | Formato LRC e cliente do LRCLIB |
| `integracao` | Windows: botões na miniatura da barra de tarefas, bandeja, janela mini (subclasse da janela) |
| `modulos` | Formato dos módulos da Loja: manifesto TOML, catálogo fechado de ações com motivo, aceite, bloqueio, instalados |
| `servicos` | Last.fm (autorização desktop e scrobble), Discord (Rich Presence pelo canal local), cofre DPAPI |
| `tocador` | Decodifica (`symphonia`), converte a taxa (`rubato`) e entrega à saída (`cpal`) por uma fila SPSC (`rtrb`) |
| `biblioteca` | Varre pastas, lê etiquetas (`lofty`) e guarda no acervo SQLite (`faixas`, chave `(caminho, pasta)`) |
| `player` | O app em GPUI 0.2.2: estado em `app.rs`, telas em `ui/*.rs` |

### Princípio das threads

Cada coisa lenta ou que pode travar tem uma thread própria: `escuta`, `sessao`, `gravador`, `historico`,
`letras`, `lastfm`, `discord`, `tocador`, `biblioteca` e `escuta-app`. Elas publicam o último resultado num
`Arc<Mutex<_>>`, às vezes com um contador de versão. **A interface só lê o que está pronto e nunca espera nenhuma
thread.** Pedidos seguem por canal (`mpsc`).

### Arquivos principais do `player`

- `app.rs`: o `Player` (estado de tudo), `executar(Comando)`, `quadro()` (roda a cada frame), teclado.
- `escuta.rs`: thread do Modo Escuta; `Fonte::{Tudo, App(exe), Proprio}`.
- `local.rs`: player local (fila, tocador, publicação no SMTC via `souvlaki`, thread da biblioteca).
- `diario.rs`, `letrista.rs`, `servicos.rs`: as pontes para histórico, letras e Last.fm/Discord.
- `arranjo.rs`: layout responsivo (largo, gaveta, empilhado, mini).
- `ui/`: `casca` (topo e trilho), `heroi`, `painel` (módulos e Ajustes), `dock`, `paleta` (Ctrl K),
  `busca_letra`, `campo` (campo de texto), `biblioteca`, `musicas`, `loja`, `modulo` (ficha), `mod.rs`
  (botões, dicas e ajudantes).
- `recursos/`: fontes, ícones SVG e os manifestos dos módulos (`recursos/modulos/*.toml`).

### Convenções

- Nomes de código, comentários e textos em **português** (`tocar_musica`, `Acervo::buscar`). Comentários dizem o
  porquê, em frases curtas.
- Toda ação do usuário é um `Comando` (`crates/nucleo/src/comando.rs`), com nome, atalho e entrada na barra de
  comandos.
- Botões passam por `self.botao(...)` e `self.botao_icone(...)`; as animações por `self.anim.seguir(chave, alvo,
  tempo)`; as dicas por `marca(id)` e `ao_pairar(...)`.
- Ajustes novos: um campo em `Ajustes` com `#[serde(default)]` e correção em `validar()`.
- Edições de texto: um campo novo usa `ui/campo.rs` (`receber_texto`, `conteudo_do_campo`, `EntityInputHandler`).
  Não trate letras pela tecla (`key_char`), porque acento com tecla morta deixa de funcionar.

---

## 5. Ambiente do PC do Rafael (importante)

- **Erro 4551 (Controle Inteligente de Aplicativos)** bloqueia de vez em quando um exe ou uma DLL de proc-macro
  recém-compilados ("can't find crate for X", "LoadLibraryExW ... 4551"). Solução: `cargo clean [--release] -p
  <pacote>` e compilar de novo. **Confira sempre o código de saída do cargo**, porque um `| tail` esconde a falha
  e o exe antigo continua lá.
- **O Rafael costuma deixar o Player aberto.** Para testar: renomeie o exe em uso
  (`D:/Rust/target/release/player.exe` → `player-em-uso.exe`), compile e abra uma cópia com `TCCPLAYER_DADOS`
  apontando para uma pasta temporária. Guarde o PID do `Start-Process -PassThru` e feche só esse processo.
- Ele costuma estar ouvindo música (Spotifast, Opera). Não silencie o sistema nem toque som de teste por cima;
  use `TCCPLAYER_MUDO=1` ou a música que já está tocando.
- Para testar sem mexer no mouse dele: capture a janela com `PrintWindow(hwnd, hdc, 2)` e mande cliques e teclas
  com `PostMessage`. Texto vai como `WM_CHAR`; tecla morta, como `WM_DEADCHAR`.
- Prefira o PowerShell para abrir o exe; pelo Bash ele às vezes é bloqueado.

---

## 6. Armadilhas já resolvidas (não caia de novo)

- **Process loopback**: o bloco `AUDIOCLIENT_ACTIVATION_PARAMS` passado no `PROPVARIANT` precisa ser alocado com
  `CoTaskMemAlloc`, porque o Windows o libera sozinho. Na pilha ou numa `Box` do Rust, isso corrompe a memória
  (0xC0000374) ao começar ou ao fechar a captura.
- **GPUI e a janela**: redimensionar a janela de dentro de um evento do GPUI perde o aviso de resize. Poste uma
  mensagem própria (`MSG_MINI`) e redimensione na subclasse.
- **GPUI e os cliques**: a área de arrastar a janela "rouba" cliques. Os botões ali precisam de `.occlude()`.
- **GPUI e o texto**: letras chegam por `WM_CHAR` → `EntityInputHandler`, registrado com `window.handle_input`
  só enquanto um campo está ativo.
- Listas longas: `uniform_list` + `cx.processor` (só as linhas visíveis são montadas).
- O loopback do Windows para de entregar amostras quando nada toca: a escuta trata 80 ms sem amostra como
  silêncio.
- Depois que a fila local acaba, o Windows guarda a última faixa do Player como "parada". `Local::e_resto`
  impede que ela volte como "tocando agora".

---

## 7. Onde estamos (13/11/2026)

**Pronto e testado no app:** Modo Escuta (som do PC ou de um app só, com "tocando agora" filtrado pelo mesmo
app); três visuais e modo seguro; interface responsiva com 4 temas e 2 skins; painel em módulos com modo edição;
histórico e estatísticas; Letra com busca automática e manual; bandeja, janela mini e dicas; barra de comandos;
Loja com fichas de módulo; player local com categorias por pasta, "Todas" sem repetir, busca e fila;
publicação da música local no SMTC.

**Feito, mas não visto funcionando de ponta a ponta:**
- Last.fm e Discord: faltam as chaves (o Rafael ainda não criou; ver `app/README.md`, "Chaves dos serviços").
- Som real do player local e arquivos MP3, FLAC e M4A: os testes usaram WAVs sintéticos, sem som.
- Janela de escolher pasta, capa embutida nas músicas locais, contagem das músicas locais no histórico.
- Clique nos botões da miniatura da barra de tarefas e aparência do menu da bandeja.
- Digitação num teclado físico com IME (foi testado com mensagens simuladas).
- Captura por app em Windows anteriores ao 10 21H1 (deve cair num aviso, sem travar).

**Dívidas conhecidas:**
- Sem CI.
- Skins KDL carregadas de arquivo (plano da fase 1) não existem; os temas estão no código.
- Medidor de flashes do modo seguro (fase 3) não existe.
- O Spectral do mockup (`mockup/project/Main.dc.html`) ainda é o antigo.
- Nada de capa da música local no controle de mídia do Windows (`cover_url: None`).

---

## 8. Próximas fases (escolha do Rafael; nenhuma começou)

1. **Player local, parte 2**: sem intervalo entre faixas, volume igualado (ReplayGain/R128), crossfade,
   equalizador, tela da fila, visões por álbum e artista, playlists próprias, capa local no Windows.
2. **Pré-análise das músicas locais**: analisar o arquivo antes de tocar (BPM, seções, drop), com cache, para o
   diretor antecipar o drop.
3. **Análise musical melhor**: modelo *Beat This!* (batida e compasso), tom, confiança do BPM, novidade por
   compasso.
4. **Modo seguro completo**: medidor de flashes.
5. **Loja de verdade**: catálogo assinado, download de módulos, registro no GitHub, módulos WASM, skins de
   arquivo, paleta do papel de parede.
6. **Last.fm e Discord de ponta a ponta**, quando houver as chaves.
7. **Importar histórico** do Spotify (arquivo de dados pessoais) e do Last.fm.
8. **Rádios online legais.**
9. **Controle remoto**: API local, QR code no celular, MCP, ganchos de evento.
10. **Diferenciais pesados**: separar voz e instrumental, reconhecer música pelo som, ouvir junto.
11. **Acabamento e avaliação do TCC**: instalador, CI, medições de desempenho, teste com usuários (SUS).
12. **Mockup**: levar o Spectral TD para o mockup.

O que reaproveitar de outros projetos em cada área (com licenças) está em
[`docs/pesquisa-repos-streaming.md`](docs/pesquisa-repos-streaming.md). O ListenBrainz saiu do plano a pedido
do Rafael.

---

## 9. Como entregar uma tarefa

1. Leia os arquivos que vai mudar antes de editar; siga o estilo que já existe.
2. Escreva testes para a lógica nova (há testes em quase todos os crates).
3. Rode `cargo fmt --all`, `cargo clippy --workspace --all-targets` (zero avisos) e `cargo test --workspace`.
4. Se mexeu na interface, abra uma cópia de teste (seção 5) e confira.
5. Atualize o `app/README.md` (o que o usuário vê) e, se mudou o estado do projeto, as seções 7 e 8 deste
   arquivo.
6. Diga com clareza o que foi testado e o que não foi.
