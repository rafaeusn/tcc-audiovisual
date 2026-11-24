# AGENTS.md: contexto para qualquer agente de IA

Leia isto antes de mexer no projeto. Vale para Cursor, Claude Code, Codex ou um chat novo. Atualizado em
23/11/2026. **Responda sempre em português do Brasil.**

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

Desde 16/11/2026 o projeto é **gratuito e aberto (GPL-3.0-or-later)**. Os serviços de streaming entram como
provedores da Loja ([`docs/plano-v2.md`](docs/plano-v2.md)); desde 21/11/2026, sem as limitações de antes (seção 2). Código copiado de outro projeto mantém no
topo a origem e a licença e entra em [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md).

O código está em **`app/`** (workspace Cargo). O resto do repositório é material anterior: `preview/` (protótipo
WebGL em JS, gabarito do visual e do diretor), `mockup/` (mockup da UI), `prototypes/`, `touchdesigner-mcp/` e
`docs/`.

---

## 2. Regras que não se negociam

1. **Serviços sem limitação (21/11/2026, com o aval da orientadora, para fins acadêmicos).** O app usa todos os
   recursos que conseguir: APIs oficiais e não oficiais (InnerTube e PO token do YouTube Music, a API interna e o
   player web do Spotify, Spotify Connect, librespot, o `client_id` público do SoundCloud, o reconhecimento do Shazam
   pelo protocolo do SongRec, servidores de terceiros para sem perdas), leitura de páginas, provedores já ligados e
   prontos por padrão, sem aviso de responsabilidade e sem exigir o Client ID do usuário. Pré-analisar, sincronizar o
   visual com o som e guardar análise do que vem de qualquer serviço vale; cache local do áudio para tocar sem
   intervalo e para a pré-análise também.
2. **O único limite, que o agente não atravessa:** nada de quebrar DRM nem de transformar o app num baixador de
 música (salvar em arquivo, ou redistribuir, faixas de catálogo do Spotify, YouTube, Tidal...). O app toca pelo
 serviço, com a conta do próprio usuário. O Modo Escuta continua funcionando sem provedor nenhum. (Regras 1 e 2
 reescritas de novo em 21/11/2026; as de 16/11 pediam provedor desligado por padrão e o Client ID do usuário.)
 **Decidido com o Rafael em 24/11/2026:** tocar pelo `librespot` **vale** e não é quebrar DRM — ele pede a chave de
 áudio ao Spotify como um cliente autenticado, e o áudio decifrado vive **só na memória** (nunca em arquivo, nunca
 redistribuído). O que o `librespot` grava em disco é o temporário dele, ainda cifrado, apagado a cada abertura.
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
cargo test --workspace                   # 483 testes em 23/11/2026 (fora o `visual --test desenho`), todos passando
cargo clippy --workspace --all-targets   # tem que sair sem aviso
cargo fmt --all
cargo deny check licenses sources        # licenças e fontes (app/deny.toml)
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
  - `TCCPLAYER_ILHA_ABERTA=1`: a Ilha fica aberta como se o mouse estivesse em cima (para fotografar a forma
    aberta sem mexer no mouse); `TCCPLAYER_ILHA_PAGINA=<0 a 4>` abre numa página. A Ilha vem desligada: ponha
    `ilha = true` (e `ilha-posicao = 0.2`, para não cobrir a Ilha do Player do Rafael) no `ajustes.toml` da pasta de
    teste. Fotografe a Ilha com `PrintWindow` (a janela dela), não a tela: o Rafael costuma estar usando o PC.
  - `TCCPLAYER_ILHA_ESCONDIDA=1`: a janela da Ilha fica acima da borda de cima da tela (fora da vista) e não some
    com tela cheia de outro app; `PrintWindow` ainda a fotografa. Com `Start-Process -WindowStyle Hidden` e a janela
    principal posta fora da tela (`SetWindowPos` sem ativar, depois `ShowWindow(4)`), a cópia inteira roda sem
    cobrir a tela do Rafael nem roubar o foco. Se a Ilha da cópia não aparece, confira `SHQueryUserNotificationState`:
    com vídeo ou jogo em tela cheia (estado 2 ou 3) ela some de propósito.
  - `TCCPLAYER_ILHA_DIAGNOSTICO=1`: registra, quadro a quadro durante as transições da Ilha, as medidas da forma, a
    posição do conteúdo depois do layout (`LAYOUT`), as trocas de região (`REGIAO`) e os movimentos da janela
    (`MOVER`), com o prefixo `ilha-diag` no `player.log`.
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
| `letras` | LRC (com palavras), TTML (`ttml.rs`), as fontes em cadeia (`fontes.rs`: Better Lyrics, Unison, LRCLIB; `em_cadeia`), tradução pela Unison e letras latinas (`traducao.rs`, `any_ascii`) |
| `integracao` | Windows: botões na miniatura da barra de tarefas, bandeja, janela mini (subclasse da janela); a janela da Ilha (`ilha.rs` + `sistema_ilha.rs`: sempre por cima, sem ativar, os monitores (`monitores`, `monitor_no_ponto`, com o nome de cada um), tela cheia de outro app, movimento reduzido) |
| `modulos` | Formato dos módulos da Loja: manifesto TOML, catálogo fechado de ações com motivo, aceite, bloqueio, instalados |
| `servicos` | Last.fm (autorização desktop e scrobble), Discord (Rich Presence pelo canal local), cofre DPAPI |
| `tocador` | Decodifica (`symphonia`), converte a taxa (`rubato`), mistura (`mistura.rs`: sem intervalo, crossfade, ganho, EQ, limitador) e entrega à saída (`cpal`) por uma fila SPSC (`rtrb`). Toca uma `Origem`: arquivo ou endereço na internet (`rede.rs`: baixa em pedaços de 64 KB com `Range`, pula para qualquer ponto, retoma conexão caída; a seguinte da internet abre em segundo plano) |
| `provedores` | F1: o formato comum das faixas (`modelo.rs`), o protocolo JSON por linha com os programas dos serviços (`protocolo.rs`, descrito em `docs/f1-provedores.md`), o lado do Player (`cliente.rs`: abre o processo, pedidos com prazo de várias threads), o lado do provedor (`servidor.rs`), os instalados (`registro.rs`), o casador entre serviços (`casador.rs`) e o `provedor-exemplo` |
| `preanalise` | Pré-análise (porte de `preview/preanalysis.js`, em `pista.rs`): batidas por programação dinâmica, tempo forte, clareza do pulso, trechos por novidade, energia calibrada, drops e subida; tom (`tom.rs`); LUFS; o modo arquivo do diretor (`condutor.rs`); `cache::Cache` em `analises.db` (vale enquanto tamanho, data e `VERSAO` batem) |
| `biblioteca` | Varre pastas, lê etiquetas e ReplayGain (`lofty`) e guarda no acervo SQLite (`faixas` com chave `(caminho, pasta)`, `playlists`, `playlist_itens`); consultas por `Filtro` (origem, busca, álbum, artista) |
| `player` | O app em GPUI 0.2.2: estado em `app.rs`, telas em `ui/*.rs` |
| `provedores/spotify` | O provedor do Spotify (F6, um programa à parte, embutido; o Soloist em `soloist/`): login pelo navegador com o Client ID público do librespot (`spotify.rs`), a conta e o refresh token cifrado (`conta.rs`), quem usa a porta 8898 (`porta.rs`); os dados pelo Pathfinder com a sessão (`pathfinder.rs`, `codigos.rs`), a conversão pura (`conversao.rs`) e as páginas (`dados.rs`); e o **áudio** no motor do Player (`audio/`: o librespot pede a chave, decifra em memória, um cache com teto e um HTTP só em `127.0.0.1` com token por abertura entregam os bytes ao `tocador`). Compila só com `vergen` 9.0.6 no `Cargo.lock` |
| `provedores/youtube-music` | O provedor do YouTube Music (um programa à parte, GPL-3.0-only por causa do `rustypipe`): metadados pelo `rustypipe`, áudio pelo cliente visionOS (`visionos.rs`). Embutido: o Player o acha ao lado do `player.exe` (`remoto::EMBUTIDOS`) |

### Princípio das threads

Cada coisa lenta ou que pode travar tem uma thread própria: `escuta`, `sessao`, `gravador`, `historico`,
`letras`, `lastfm`, `discord`, `tocador`, `biblioteca` e `escuta-app`. Elas publicam o último resultado num
`Arc<Mutex<_>>`, às vezes com um contador de versão. **A interface só lê o que está pronto e nunca espera nenhuma
thread.** Pedidos seguem por canal (`mpsc`).

### Arquivos principais do `player`

- `app.rs`: o `Player` (estado de tudo), `executar(Comando)`, `quadro()` (roda a cada frame), teclado.
- `escuta.rs`: thread do Modo Escuta; `Fonte::{Tudo, App(exe), Proprio}`.
- `local.rs`: player local (thread da biblioteca, tocador com a seguinte pré-carregada, SMTC via `souvlaki` com
  a capa).
- `remoto.rs`: as músicas dos provedores (F1): uma thread por provedor instalado (`<dados>/provedores`), a busca
  em todos ao mesmo tempo (depois de 350 ms sem digitar) e o endereço de cada faixa. A faixa de provedor vira uma
  `biblioteca::Musica` com `remota` preenchida e a chave `servico:id` no `caminho`; o `local.rs` pede o endereço
  antes de tocar (a de agora toca quando ele chega; a seguinte é resolvida antes, para entrar sem intervalo).
- `preanalista.rs`: thread da pré-análise (prioridade: a de agora e as duas seguintes; depois a biblioteca, em
  prioridade baixa; análise da biblioteca interrompida quando chega prioridade).
- `fila.rs`: a fila (aleatório que guarda a ordem original, repetir, a seguir, mover, tirar), só lógica e testada.
- `ilha.rs`: a lógica da Ilha (formas, esperas para abrir e fechar, fila de avisos, medidas), testada; a vista é
  `ui/ilha.rs`, numa segunda janela GPUI (`WindowKind::PopUp`, fundo transparente, tamanho fixo da maior forma)
  aberta por `Player::sincronizar_ilha`. A forma é um caminho (`PathBuilder`) com ombros côncavos; a região da
  janela (`integracao::ilha::regiao`) acompanha a forma, para os cliques fora dela passarem. Aberta, tem páginas
  (`ilha::Pagina`); a página Visual usa uma segunda `janela::Camada` na janela da Ilha, preparada em outra thread
  quando a Ilha abre (`Camada::preparar_transparente`, depois `CamadaPendente::pronta` a cada quadro), e desenha o
  visual por baixo de um recorte (furo) da forma. A página Letra e o módulo Letra enchem a linha cantada com
  `ui::letra_enchendo` (destaques de cor do `StyledText`) a partir de `letras::cantado`. Arrastar muda `ajustes.ilha_posicao`. A Ilha chama
  `Player::atualizar_estado` (a parte do quadro que não depende da janela), então o estado anda com a janela
  principal minimizada ou na bandeja.
- `ui/configuracoes.rs`: a página Configurações (seções numa coluna, ou abas em janela estreita). Os blocos vêm de
  `painel::blocos_de_ajuste`; o painel lateral mostra só os módulos.
- `diario.rs`, `letrista.rs`, `servicos.rs`: as pontes para histórico, letras e Last.fm/Discord. A thread das letras
  pergunta às fontes na ordem dos ajustes (`Recado::Configurar`), guarda a letra com a fonte, o acerto de sincronia
  (`Recado::Atraso`) e a tradução por música e idioma (`Recado::Traduzir`, `EstadoDaTraducao`). O texto embaixo de
  cada linha (tradução, letras latinas) sai de `Player::extra_da_linha`; a letra em tela cheia é `ui/letra_cheia.rs`
  (`Player::letra_cheia`, comando `LetraTelaCheia`, tecla Y), que pede ao arranjo o visual na janela inteira.
- `arranjo.rs`: layout responsivo (largo, gaveta, empilhado, mini), no Palco e na Estante (`Pedido.estante`,
  `Pedido.expansao`; `Arranjo.lateral` e `Arranjo.tocando`).
- `ui/`: `casca` (topo e trilho), `heroi`, `painel` (módulos e Ajustes), `reproducao` (crossfade, volume
  igualado e equalizador nos Ajustes), `dock`, `paleta` (Ctrl K), `busca_letra`, `campo` (campo de texto),
  `biblioteca`, `musicas` (categorias, visões, ações da linha, `campo_de_busca`), `fila`, `playlist` (janela), `loja`,
  `modulo` (ficha), `espacos` (topo da folha: espaços e abas), `paginas` (Para você, álbum, playlist, artista,
  Biblioteca do serviço), `busca` (página Busca), `faixa` (página da faixa), `estante` (lateral, barra de ícones da gaveta, o quadro do
  visual no trilho Tocando e a navegação de baixo da Estante), `mod.rs` (botões, dicas e ajudantes).
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
- "Tocando agora": `Player::escolher_origem` decide entre a música local e os outros apps (quem toca vence; os
  dois tocando ou parados, quem tocou por último) e grava em `origem_local`. Todo controle (dock, atalhos,
  bandeja, miniatura) segue essa origem; nunca use `local.ativo()` para decidir a quem mandar o comando. A sessão
  do próprio Player (`local::NOME_NO_SISTEMA`) é tirada do "outro" em `ler_outro`. No crate `sessao`, sem filtro,
  `smtc::escolher` prefere um app tocando ao "atual" parado.
- Sessões de mídia sem música (launchers como o Riot Client) são cortadas no crate `sessao`
  (`e_midia_de_verdade`, usada em `smtc::escolher` e `do_app`); artista com cara de identificador de programa
  vira vazio. "Nesta sessão" só registra depois de `SOM_PARA_A_SESSAO` segundos tocando com som
  (`Player::contar_para_a_sessao`).
- A interface não desenha com a janela escondida ou minimizada. O que precisa andar assim (player local) é
  atendido também por `manter_o_player_local_andando` (relógio de 200 ms).
- `tocador`: se a saída de som cai, `Motor::saida_caiu` guarda arquivo, posição e estado, e `tentar_retomar`
  reabre na saída nova a cada 1 s.
- **Calibração do diretor**: o corpus usou mono do ffmpeg (+3 dB sobre a média dos canais). `analise::Motor`
  compensa com √2 e a pré-análise faz o mono `(L+R)·0,707`. Não mude isso sem recalibrar
  (`docs/diretor-calibracao.md`, seção 8). Para comparar com a prévia: `cargo run --release -p preanalise
  --example diagnostico -- faixa.mp3` e `node tools/simulate.mjs faixa.mp3` (precisa de ffmpeg).
- Música local pré-analisada: o visual é conduzido por `preanalise::condutor::Condutor` (modo arquivo), chamado
  em `Player::aplicar_preanalise`; o ao vivo só dá o espectro. O impulso da forma soma os dois sem salto
  (`impulso_saida`).
- Varredura: pasta que não existe (`!is_dir()`) é pulada; varrer vazia apagaria as músicas dela do acervo.
- **Duas janelas**: `on_window_closed` só encerra o app quando a janela principal some (fechar a Ilha não sai). O
  `Process.MainWindowHandle` do .NET passa a apontar às vezes para a Ilha: nos scripts de teste, ache a janela
  principal pelo título "Player" (ou pelo tamanho) antes de mandar cliques.
- **A Ilha se move por mensagem** (`integracao::ilha::mover`), nunca de dentro de um evento do GPUI. A janela tem
  o tamanho da maior forma e a forma desliza dentro dela. **O GPUI reserva uma borda de redimensionar** (8 px dos
  lados e embaixo, 1 px em cima) em janelas sem barra de título: a subclasse da Ilha responde `WM_NCCALCSIZE` (área
  de desenho = janela inteira) e `WM_NCHITTEST` (`HTCLIENT`). Sem isso, sobrava a "barra fina" em cima e a forma
  perdia as pontas. A troca de origem (local ↔ outro app) apaga a faixa por um
  instante: a Ilha só some depois de 2 s sem faixa (`ilha::SEM_FAIXA_SOME`).
- **GPUI arredonda cada elemento em relação ao pai.** Um pai numa posição fracionária que muda a cada quadro faz
  os filhos pularem 1 px para lá e para cá, mesmo parados na tela (era a tremida da Ilha, 17/11/2026). O que se move
  com mola e tem filhos parados vai em pixels inteiros (`ilha::caixas`).
- **A Ilha e a "tremida"**: a janela tem sempre o tamanho do maior tamanho da Ilha (trocar o tamanho não a
  redimensiona) e folga para o quique (`FOLGA_DOS_LADOS`, `FOLGA_EMBAIXO`; o teste
  `o_quique_cabe_na_janela` confere). Enquanto as molas andam, a região da janela só cresce (caixa de agora, a do
  alvo e uma folga, `ilha::unir_regioes`); só fica justa quando a forma assenta, e sem pedir redesenho
  (`SetWindowRgn(.., false)`). Encolher a região a cada quadro cortava a forma nos quadros em que a tela ainda
  mostrava o desenho anterior. O conteúdo de antes sai num esmaecimento curto, não some de um quadro para o outro.
- **O visual da Ilha (camada transparente)**: o recorte da forma só abre depois de o visual estar na tela há um
  quadro (`quadros_com_visual >= 2`); o visual é desenhado na área inteira da página, 4 px maior que o recorte e
  por baixo do preto (senão as bordas suavizadas das duas camadas deixam ver a área de trabalho); a camada é limpa
  um quadro depois de o recorte fechar e logo depois de criada ou redimensionada (o conteúdo novo de uma superfície
  é indefinido). Criar a camada na thread da janela travava a troca para a página Visual (abrir a placa de vídeo e
  compilar os shaders).
- Player local: a fila fica em `Local`; o tocador só conhece "esta" (`Tocar`) e "a seguinte" (`Seguinte`).
  Quando a seguinte entra sozinha, `Situacao::avancou` muda e a fila anda; quando nada vem depois (ou o arquivo
  não abre), `terminadas` muda. Os marcos da mistura (`Marco::Comecou`, `Marco::Fim`) só valem quando a placa
  chega neles (`painel.tocados`), não quando são produzidos, porque há meio segundo de som na fila da placa.

---

## 7. Onde estamos (21/11/2026)

**Pronto e testado no app:** Modo Escuta (som do PC ou de um app só, com "tocando agora" filtrado pelo mesmo
app); três visuais e modo seguro; interface responsiva com 4 temas e 2 skins; painel em módulos com modo edição;
histórico e estatísticas; Letra com busca automática e manual; bandeja, janela mini e dicas; barra de comandos;
Loja com fichas de módulo; player local com categorias por pasta, "Todas" sem repetir, busca e fila;
publicação da música local no SMTC; fila com aleatório, repetir, a seguir e reordenar; troca sem intervalo e
fim da fila; álbuns, artistas e playlists; Ajustes de crossfade, volume igualado e equalizador (gravados).

**Auditoria (13/11/2026):** volume do dock não passa mais da borda; "Desfazer" no aviso (fila, playlist, pasta);
origem do "tocando agora" refeita; bandeja e miniatura seguem a origem; player local atendido com a janela
escondida; retomada após troca de saída de som; pasta fora do ar não apaga músicas; contagem de playlist só
com músicas do acervo; clique numa música confere o arquivo (a lista pode ter mudado); janela mini fecha
janelas por cima.

**Análise musical (15/11/2026), conferida no app** com duas faixas de hardstyle reais: "Pré-analisada", BPM
150/170, tom com Camelot, tempo do compasso aceso, "Intensa" (vermelho) no instante de cada drop e o visual
explodindo nele; a troca para o Spotify (Modo Escuta) seguiu sozinha. O ganho automático do ao vivo só foi
medido no diagnóstico com arquivos (não com o Spotify ouvido por um tempo longo).

**F2, parte 1 (23/11/2026)**, desenho em
[`docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md`](docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md)
(espaços por serviço com a identidade de cada um, páginas unificadas, layouts Palco e Estante; o visual mora no trilho
"Tocando" da Estante e cresce até o Palco): a `Apresentacao` ganhou `identidade` (cor `#RRGGBB` e logo SVG de até
32 KB) e as faixas e álbuns, `album_id` e `artistas_ids` (opcionais, protocolo v1); o YouTube Music manda os dele
(ao vivo: 5 de 5 faixas com os ids); `remoto.rs` grava a logo em `<dados>/provedores-dados/<serviço>/logo.svg`, guarda
a cor em `InfoDoProvedor` e pede as páginas (`PedidoDePagina`, `PaginaRemota`, cache de 10 min, "Tentar de novo").
Conferido na cópia: a logo gravada. Ainda sem tela (partes 3 a 5); a API das páginas tem um `expect(dead_code)` que
deve sair quando as telas a usarem.
**Parte 2 (23/11/2026)**: o acervo guarda o que a pessoa curte e salva de qualquer serviço (`salvos`: `TipoSalvo`,
`Salvo`, `salvar`, `tirar_salvo`, `esta_salvo`, `salvos`, `fixar`; os fixados vêm primeiro; `dados` é o item em JSON,
nunca áudio) e as playlists do Player aceitam faixas de serviço (`faixas_remotas` com a chave `servico:id`,
`guardar_remotas`; a consulta da playlist junta as do acervo e as do serviço; `Playlist.remotas`). O Player manda
`Musica` ao criar playlist e pôr músicas nela (`Local::por_na_playlist`). Sem tela de curtir e fixar ainda (partes 3 e 4).
**Parte 3 (23/11/2026)**: `navegacao.rs` (`Espaco`, `Rota`, `Navegacao` com uma pilha por espaço, até 50 para trás) e
`espacos.rs` (a paleta do espaço tirada da cor do serviço, com contraste conferido nos 4 temas; a cor tirada do nome
para provedor sem cor); o `Player` trocou o campo `pagina` por `nav` e o método `pagina()` (a `Pagina` de hoje sai da
rota aberta); `app/navegar.rs` abre rotas, troca de espaço (grava `ajustes.espaco`, anima a cor em 250 ms) e volta a
Tudo quando o provedor do espaço sai (`conferir_espaco`). A folha do Palco ganhou o topo (`ui/espacos.rs`: voltar,
avançar e a fileira de espaços com a logo SVG do provedor por `img`) e a cor do espaço (folha, filete e degradê).
Comandos `Voltar`/`Avancar` (`Alt ←/→`, botões do mouse) e `ProximoEspaco` (`Ctrl Tab`). O início do YouTube Music
junta lançamentos, clipes novos e as paradas (só as paradas vinham vazias). "Para você" ainda é texto (a parte 4 faz o
desenho). Conferido na cópia: logo, cor vermelha, as 4 prateleiras, Tudo neutro, voltar e avançar, Esc e o espaço
gravado. Não conferido: os botões do mouse e `Alt ←/→` (as mensagens de teste não levam o Alt). Depois da revisão: a
paleta anda uma vez por quadro (`atualizar_paleta`, a troca dura mesmo 250 ms), escolher um espaço com a folha fechada
abre a primeira página dele, navegar não vale na janela mini nem com a barra de comandos ou a janela de playlist abertas
e desfaz o modo edição e a gaveta, a página vencida fica na tela enquanto renova (e um erro ao renovar não a apaga), e
um provedor que não abriu tira o espaço só nesta abertura (o lembrado continua). Essas correções não foram vistas na
cópia, só nos testes.
**Parte 4a (23/11/2026)**: `capas.rs` (capas da internet baixadas por 2 threads e guardadas em `<dados>/capas`, a tela
desenha o arquivo com `img`), `paginas.rs` (o cartão de cada `Item`, a rota e o pedido de cada item, o link do
artista só com id e capacidade, `Item` ↔ `Salvo`), `app/salvos.rs` (curtir com clique otimista, tocar faixas de
provedor) e `ui/paginas.rs` ("Para você" em prateleiras de cartões com esqueleto; álbum, playlist e artista; a
Biblioteca do serviço com filtro e fixar). A thread da biblioteca atende salvar, tirar e fixar e publica `salvos` na
`Lista`. Conferido na cópia com o YouTube Music: as prateleiras com capa (118 capas guardadas), abrir um álbum, tocar,
curtir (coração cheio e o álbum na Biblioteca do serviço), a página do artista e fixar (gravado no banco). Ficou para a
4b: a busca com todos os tipos, a página da faixa, a Fila e o Histórico como páginas. Depois da revisão: "Aleatório" liga
o aleatório antes de tocar (antes perdia as faixas de antes da sorteada), a prateleira rola só de lado (a rodinha desce
a página) com setas nas pontas que não deixam o clique passar, a capa mostra a silhueta enquanto carrega ou se não
decodifica, só se guarda o que parece imagem e cabe em 8 MB, as capas prontas ficam na memória e a que falhou tenta de
novo depois de 1 minuto, os salvos chegam já lidos da thread da biblioteca, e cada página guarda a própria rolagem.
Ficou: lista longa de playlist sem `uniform_list`, curtir e descurtir antes da confirmação pisca o coração, o link do
artista no cabeçalho cobre o subtítulo inteiro, a foto do artista aberto por um link, tirar uma faixa da Biblioteca do
serviço (o cartão não tem coração) e curtir e fixar como comandos.
**Parte 4b (23/11/2026)**: o provedor do YouTube Music busca todos os tipos (principal + faixas, álbuns, artistas e
playlists ao mesmo tempo, 8 s de prazo cada; `conversao::juntar_busca` põe o melhor resultado primeiro, sem repetir, e
prefere a versão da busca por tipo, mais completa); `remoto::Busca` guarda `itens` e `respondidos`; `paginas.rs` agrupa
(`agrupar_busca`, `FiltroDaBusca`, `servico_buscando`, `rota_do_album`, `abas_da_faixa`). A folha ganhou as abas do
espaço (`navegacao::abas_do_espaco`: Para você, Busca, Biblioteca; o computador não tem); `ui/busca.rs` (melhor
resultado, faixas, prateleiras, chips; em Tudo um bloco por serviço) e `ui/faixa.rs` (Letra da faixa que toca, Mapa
da pré-análise em memória, Parecidas só com `Radio`). `Rota::Fila` e `Rota::Historico` (a aba da Biblioteca sai da
rota, `Player::aba_biblioteca()`; num serviço, a Biblioteca mostra só Fila e Histórico); o bloco "Nos serviços" virou o
atalho para a Busca. O teclado só vai ao campo de busca com ele na tela (`navegacao::tem_campo_de_busca`), e
`Alt ←/→` vale com o campo ativo. Conferido na cópia com o YouTube Music: busca "daft punk" (artista em destaque,
faixas, álbuns), chip Álbuns, página da faixa, tocar e o Mapa (107 BPM, Lá menor 8A, a posição andando), a Fila pelo
dock no espaço do serviço e voltar, o Histórico, a Busca em Tudo e, depois da revisão, o campo que não perde a
largura, o empilhado em folha estreita e Backspace numa página de álbum sem mexer na busca. Não conferido: a aba Letra
com uma letra (o módulo não estava ligado na cópia), dois serviços respondendo ao mesmo tempo e `Alt ←/→` (as
mensagens de teste não levam o Alt). Ficou: `Busca.faixas` só serve a um teste, o agrupamento roda a cada quadro, o
"Tocar" da página da faixa troca a fila inteira, voltar depois de "Buscar nos serviços" (que passa para Tudo) não volta
ao computador, e ids de linha que colidem com mais de 10 artistas ou 100 faixas por serviço.
**Parte 5, o layout Estante (23/11/2026)**: `ajustes.layout` (`Palco`, o padrão, ou `Estante`), `Comando::TrocarLayout`
(`Ctrl L`) e `Comando::ExpandirVisual` (`X`); Configurações › Aparência › Layout e o cartão Estante na aba Layouts da
Loja. `arranjo::estante`: largo = lateral (248) + folha (o herói) + trilho Tocando (340: o visual 4:3 no alto, os
módulos embaixo), dock sob a folha e o trilho; gaveta = a barra de ícones e o Tocando ao lado da página (ela encolhe:
por cima, a folha esconderia o visual, que é desenhado por baixo da interface); empilhado = a faixa do visual (72 px)
acima do dock, que sai em janela baixa. `ui/estante.rs` (lateral com espaços, páginas e salvos, os fixados primeiro;
barra de ícones; o quadro do visual; a navegação de baixo). Na Estante sempre há página (`Navegacao::garantir_pagina`;
voltar ao Palco devolve a folha fechada, `soltar_preenchidas`; voltar e avançar pulam a folha fechada,
`voltar_sem_vazias`); H e o × levam à primeira página, Esc volta uma página. Expandir: `Pedido.expansao` leva o
retângulo do visual até a janela (as regiões esmaecem) e, no fim, o arranjo é o da tela cheia (título grande, encolher,
arrastar a janela pelo visual); enquanto anima, o motor desenha no tamanho da janela e estica
(`Destino.interno`, `visual::tamanho_dos_alvos`), sem recriar os alvos a cada quadro. A skin imersiva não vale na
Estante; o erro do visual aparece no quadro, não sobre a página. Conferido na cópia com o YouTube Music: 1280 px
(lateral com o espaço vermelho, prateleiras, o Tocando com o visual e os módulos, tocar), expandir com X (visual na
janela inteira, título grande, botão encolher) e Esc, 1000 px (barra de ícones; o X com a gaveta fechada abre o
Tocando e expande dele; ao encolher ele fica aberto) e 600 px (espaços no topo da folha, a faixa do visual, as abas
embaixo). Não conferido: a animação sentida na tela (só fotos paradas), o `Ctrl L` (as mensagens de teste não levam o
Ctrl; o layout foi trocado nos ajustes), clicar nos salvos da lateral e o erro do visual. Ficou: o visual estica
(deforma) durante os 0,3 s da expansão, o modo edição no empilhado da Estante (sem painel), a lista da lateral monta as
faixas a cada quadro, e o topo da folha monta a fileira de espaços mesmo escondida.
**Parte 6, acabamento e documentos (23/11/2026)**: `Comando::AbrirEspaco(n)` (`Ctrl 1` a `Ctrl 9`; fora de `TODOS`),
`Comando::CurtirFaixa` (`C`: a faixa de serviço que toca, a versão mais completa da página aberta ou da busca, senão
`paginas::faixa_da_musica`) e `Comando::FixarPagina` (o item da rota aberta, `paginas::item_da_rota`; salva antes, se
preciso). `Busca.faixas` saiu (as telas leem `itens`). O resumo da fase está em
[`docs/f2-paginas.md`](docs/f2-paginas.md). Os três comandos não foram vistos na cópia (só os testes da lógica).
**Com isso a F2 está completa.**

**F6, o provedor do Spotify**, desenho aprovado em
[`docs/superpowers/specs/2026-11-23-f6-spotify-design.md`](docs/superpowers/specs/2026-11-23-f6-spotify-design.md): o
Spotify toca pelo **librespot** (parte 5, 24/11/2026), com o Soloist (o player oficial) e o Match do YouTube Music como
reservas. (A revisão de 23/11/2026 tinha tirado o librespot por achar que ele quebrava a regra 2; a decisão de 24/11/2026
com o Rafael desfez isso — ver a seção 2. Ele fica na sessão, nos dados e no áudio.) **Parte 1 (23/11/2026)**,
o login: o protocolo ganhou `entrar`, `sair`, `conta` e a `conta` na apresentação (capacidade `Conta`); o Player pergunta
`conta` a cada 2 s enquanto ela está `entrando` (`remoto.rs`, `InfoDoProvedor.conta`, `Remoto::entrar/sair`); o cartão de
entrar fica em Para você, Busca e Biblioteca do espaço (`contas.rs`, `ui/paginas.rs::cartao_da_conta`) e a Loja mostra a
conta com Sair. Duas armadilhas do librespot: (1) o `librespot-oauth` escreve na saída padrão antes de escutar na porta, e
com a saída presa (`stdout().lock()`) ele trava para sempre e o navegador volta para uma porta fechada (o `main` passa
`stdout()` sem lock); (2) com conta que não é Premium, o librespot **fecha o programa** (`exit(1)`): a marca `conectando`
ao lado da credencial evita o laço de reabrir e cair (e a credencial sai, para dar para entrar com outra conta). O plano
(`type`) chega depois de entrar. Depois da revisão: `sair` cancela a tentativa no meio (a geração é conferida antes de
guardar, conectar e apagar; `na_sessao` deixa uma entrar por vez), solta a porta e encerra a sessão; "Cancelar" enquanto o
navegador está aberto (`Conta.navegador`); o Player percebe um provedor que fecha ou emudece no meio do login e diz o
motivo (`depois_de_perguntar`); as páginas do serviço são esquecidas quando a conta muda; a Loja mostra o erro. Conferido na cópia, na
conta do Rafael: entrar pelo navegador, `conta.dpapi` guardada, reabrir e entrar sozinho, "Conectado como rafa ·
Premium". Também conferido: Sair e a porta ocupada (por um programa de
teste, no cartão e na Loja). Não conferido ao vivo: "Cancelar" com o navegador aberto (só o teste com a porta real, sem
navegador), e uma conta gratuita. Ficou: o `librespot-oauth` aceita a primeira conexão na porta (uma
ociosa do navegador prende o login; o certo é escrever a própria escuta), recusar no navegador mostra um erro em inglês, e
`situacao` desconhecida invalida a apresentação.

**F6, parte 2 (23/11/2026)**, os dados: tudo pelo Pathfinder (`api-partner.spotify.com/pathfinder/v2/query`) com o portador
do Login5 e o client token da sessão, **sem Client ID** (a sonda provou as 11 leituras na conta do Rafael; a reserva do
Client ID do desenho não entrou). Cada operação tem um código (`codigos.rs`); código trocado responde 412, e o novo é
relido do texto do site em segundo plano (o embutido antes; uma procura em vão a cada 10 min por operação). Um pedido
inteiro tem 16 s (`dados.rs`); sessão caída reentra em segundo plano (`Login::reconectar`, no máximo uma a cada 30 s, sem a
marca `conectando` em conta Premium). Lista pela metade é erro, nunca vazia; até 2000 faixas, 200 lançamentos, 1000 itens
da biblioteca. Protocolo: `biblioteca`. No Player: `PedidoDePagina::Biblioteca`, `paginas::juntar_biblioteca` (a conta e os
salvos, na página e na lateral), `janela_de_linhas` (a lista de detalhe monta só o que aparece), `avisar_se_nao_toca`
(serviço sem `Audio`), e um erro de página ou de busca faz perguntar a conta de novo. Conferido na cópia com a conta do
Rafael: Para você em português, busca, artista, álbum, Biblioteca, a lateral e as Curtidas (721 de 722) roladas até o fim.
Não conferido ao vivo: sessão que cai e volta, código trocado, login revogado, playlist com mais de 100 faixas que não
seja as Curtidas. Ficou: o ISRC das faixas (o Pathfinder não manda; parte 4), a lista que falha inteira na fila quando o
serviço cai (parte 4), o corte em 2000 sem aviso. Para reler os códigos à mão: o pacote
`web-player.<versão>.js` de open.spotify.com traz `"operação","query","<64 hex>"`; a busca e a discografia ficam nos
pedaços `xpui-routes-search` e `xpui-routes-artist`.

**F6, partes 3 e 4 (23/11/2026)**, tocar, em [`docs/f6-spotify.md`](docs/f6-spotify.md). `pares::quem_toca` diz quem toca cada
faixa: o próprio provedor, o tocador do serviço (`Sozinho`: o Soloist), outro serviço (o Match), alguém abrindo (a faixa espera) ou
ninguém. **Match**: `Tarefa::Casar` na thread do serviço que tem áudio; `<dados>/pares.json`; `FonteLida` guarda de que gravação é o
endereço (a pré-análise usa essa chave, e nunca a da faixa do Spotify). O casador ganhou pisos (título 0,5; artista 0,3) e desconto de
outra versão: antes, outra música do mesmo artista com a mesma duração passava. **Soloist** (`provedores/spotify/src/soloist/`:
`leitura.rs` pura, `instalar.rs`, `programa.rs`, `motor.rs`): fica aberto e recebe `play` pela API local; nada do áudio passa pelo
Player. No Player, `externo.rs` (a thread que pergunta e manda) e `local.rs` (`Externo`, `acompanhar_externo`, `com_o_pedido`: o que
a pessoa mandou fica na tela até a leitura confirmar). `servir` responde as páginas lentas de outra thread
(`Servico::em_segundo_plano`), então ele recebe a saída **sem** `lock()`. Armadilhas: o Spotify emenda outra faixa no fim (o motor
pausa 250 ms antes); uma faixa abandonada pode começar depois (`Aberta::calar` a pausa por 20 s); `desligar` no meio do download
precisa de `parar`. Configurações › Serviços (`ui/servicos.rs`): "Colar a chave copiada" lê a área de transferência e manda direto ao
provedor. Conferido na cópia com a conta do Rafael, muda: as Curtidas e um artista tocando pelo Match com pré-análise (remixes,
*slowed* e *sped up* casados certo), "Procurar outra gravação", a página da faixa e a seção Serviços. **O Soloist de verdade, com
a chave do Rafael (23/11/2026)**: baixou, o Windows não bloqueou, abriu, pareou, voltou sozinho à conta e a escuta achou o processo dele;
mas o build 1.4.0.7 do Spotify (publicado no mesmo dia) cai com 0xc0000005 ao começar a tocar, sempre em `soloist.exe+0x2120d1`, também
fora do Player, sem a API local, no modo de uma faixa e em todas as saídas de som (a pasta `D:/Rust/soloist-teste`; relato igual em
crmne/spotifast#1412). Por isso um Soloist que cai duas vezes sem nunca ter tocado 20 s é dado por quebrado: não é mais aberto, o Match
toca e outro build é procurado uma vez por dia (`Instalado::caiu`, `funcionou`). Falta ver tocar, pausar, pular, o fim e o visual com um
build que funcione. Ficou: duas instâncias do Player na mesma pasta, trazer a posição ao cair para o Match, e o Soloist e o
Spotifast abertos juntos (dois `soloist.exe` para a captura por nome).

**F6, parte 5 (24/11/2026), o áudio do Spotify pelo librespot**: o Spotify passa a tocar com o som do próprio Spotify **no motor
do Player** — pré-análise, troca sem intervalo, crossfade e equalizador, como nas faixas do YouTube Music. Decidido com o Rafael
nesse dia: o librespot **não quebra DRM** (pede a chave ao Spotify como um cliente autenticado), e o áudio decifrado vive só na
memória; a regra 2 do `AGENTS.md` foi anotada com isso. Onde: `provedores/spotify/src/audio/` — `librespot.rs` (o melhor formato
que a conta tem, a chave, a decifra em memória, e o erro com causa), `cache.rs` (os pedaços decifrados, teto de 8 MB e 4 faixas,
um download por pedaço mesmo com dois leitores), `servidor.rs` (HTTP só em `127.0.0.1`, porta sorteada, token por abertura,
`Range`/`206`), `pedidos.rs` (as contas do `Range` e do caminho, puras) e `mod.rs` (liga o servidor quando a sessão entra,
desliga quando a conta sai). O `resolver` do provedor agora devolve um endereço local, e o motor do Player não mudou. O protocolo
ganhou **`causa`** no erro (opcional, retrocompatível) e o núcleo, `Erro::ProvedorComCausa`. **A cadeia de reserva** é
`player/src/cadeia.rs` (`decidir`, `aviso`, puros e testados): librespot → Soloist → Match, com o aviso uma vez por troca; o
`Remoto` guarda os jeitos que caíram (`caiu`, `jeito_de`, `caidos`, `caiu_para`), e é **por jeito**: o Soloist cair não
derruba o áudio do próprio serviço, e vice-versa (`pares::quem_toca` recebe o conjunto dos que caíram). O `local.rs` troca de
jeito em vez de pular a faixa, e uma faixa que falha no Soloist **não** derruba o jeito (só o programa que caiu derruba).
Configurações › Serviços diz por que o áudio do Spotify parou, quando parou. Testes: 150
no provedor (com o servidor HTTP de verdade e a concorrência do cache) e 189 no `player`; `clippy` sem avisos nesses pacotes.
**Primeiro teste com a conta de verdade (24/11, à noite, numa cópia):** o login entrou, a busca funcionou e o `resolver`
devolveu o endereço local com `.ogg` (Ogg 320 escolhido, a conta é Premium). **O áudio não tocou:** o Spotify recusou a
chave (`chave-de-audio`) em todos os formatos. **A causa foi achada na documentação do Spotifast:** contas novas do Spotify
(as com nome inventado, uma sequência longa de letras e números — a do Rafael é `31axzsjldxrjw3er75tdfai2f5ou`) têm **toda**
chave recusada pelo librespot, e "nothing in Spotifast or librespot can change that today" (librespot#1649). Um Client ID
próprio **não** muda isso ("only speeds up library and search requests, not audio keys"). O caminho é o **Soloist**, que a
mesma documentação recomenda e que já temos (`soloist/`). Tudo o que foi testado e não era a causa está em
`docs/f6-spotify.md`, para não repetir. **Também achado e corrigido nesse teste:** o `resolver` devolvia um endereço local
mesmo quando a faixa não abria, e o primeiro pedaço dava `404` sem motivo visível; agora o erro sai daí com a causa certa.
**Não conferido ao vivo:** o Ogg no `symphonia`, a pré-análise pelo endereço local, a troca de faixa sem intervalo, e uma
conta antiga (que deveria tocar com o librespot).

**F4, Letras 2.0 (22/11/2026)**, em [`docs/f4-letras.md`](docs/f4-letras.md): fontes em cadeia (Better Lyrics, Unison,
LRCLIB; ordem e liga/desliga em Configurações › Letras; "Preferir palavra por palavra"), TTML convertido em LRC com
palavras, o rodapé diz a fonte, o acerto de sincronia guardado por música, tradução pela Unison (5 idiomas, guardada
por música e idioma), letras latinas no computador e a letra em tela cheia sobre o visual (Y). O manifesto do módulo
Letra foi para a versão 2 (três endereços): quem já tinha o módulo precisa aceitar de novo na Loja. Conferido na cópia
de teste com "Blinding Lights" pelo YouTube Music: Better Lyrics palavra por palavra, a tradução em português embaixo
de cada linha (módulo e tela cheia), o acerto +1,0 s, a tradução e a fonte gravados no banco, e a seção Letras. Não
conferido: a página Letra da Ilha com a tradução, as letras latinas na tela (só nos testes de unidade), a Unison e a
romanização com uma música de verdade e a troca de uma letra antiga do LRCLIB por uma melhor. Ficaram para depois:
estilos e a bolinha da tela cheia, o cartão para compartilhar e a letra no visual.

**F1, partes 1 e 2 (22/11/2026)**, desenho em [`docs/f1-provedores.md`](docs/f1-provedores.md): crate `provedores`
(modelos, protocolo versão 1, cliente, servidor, registro em `<dados>/provedores/<nome>/provedor.toml`, casador com
ISRC e nota título/artista/duração) e o `provedor-exemplo`, testado de ponta a ponta como programa à parte (pedidos
ao mesmo tempo, prazo, provedor que fecha no meio). O tocador toca endereços (`tocador::Origem`, `rede::LeitorHttp`)
e a pré-análise também (`preanalise::analisar_origem`); o cache guarda as análises das faixas pela chave e pelo ISRC
(`Cache::ler_faixa`, `guardar_faixa`).

**F1, parte 3 (22/11/2026)**: no Player, `remoto.rs` (uma thread por provedor, busca em todos, endereço de cada
faixa, reabre o provedor que fecha até 3 vezes); a fila aceita faixas de provedor ao lado das locais; a pré-análise
recebe alvos remotos (`preanalista::Alvo`); a busca da Biblioteca mostra o bloco "Nos serviços". Conferido numa cópia
de teste com o `provedor-exemplo` instalado na pasta de dados e um servidor HTTP local (Python) servindo três WAVs:
a busca chegou ao provedor, a faixa tocou pela rede com BPM 128 e 140 vindos da pré-análise remota, a seguinte entrou
sem intervalo aos 20,2 s (cronometrado), a fila acabou certo, e um provedor que fechou mostrou o erro e reabriu na
busca seguinte. Não conferido: um serviço de verdade (o primeiro é a F3) e a capa baixada de um endereço. "Neste
computador" continua sendo o player local de sempre, e não um provedor como os outros (fica para a F2, com as
páginas unificadas).

**F3, YouTube Music (23/11/2026)**, em [`docs/f3-youtube-music.md`](docs/f3-youtube-music.md): o provedor busca,
dá o início, álbuns, artistas e playlists pelo `rustypipe` e o áudio pelo cliente visionOS do YouTube (sem conta,
sem decifrar nada, sem PO token), o caminho que MixMusic, limusic e Prism usam (lido no código deles). Vem com o app
(`remoto::EMBUTIDOS`: o programa ao lado do `player.exe` entra na lista; dá para desligar, não remover) e recebe a
pasta `<dados>/provedores-dados/<serviço>` em `TCCPLAYER_PROVEDOR_DADOS`. Testado: iOS só serve o primeiro ~1 MB,
TV/web do `rustypipe` não decifram mais, Android VR pede login; visionOS serve o arquivo inteiro. O tocador pulou
para os 410 s de "Around the World" e tocou até o fim; no app (cópia de teste), a busca trouxe 20 faixas e a
primeira tocou com capa e pré-análise (121 BPM, Ré maior). Não testado ao vivo: início, álbum, artista, playlist
(sem tela até a F2). Rodar o JavaScript do YouTube (youtubei.js, yt-dlp) foi bloqueado pela segurança do agente e
não foi preciso.

**F1, parte 4 (22/11/2026)**: a Loja tem a aba Provedores (situação, versão, capacidades, ligar e desligar,
instalar de uma pasta, remover com "Desfazer", abrir a pasta). `provedores::instalar` copia a pasta para
`<dados>/provedores/<servico>` por uma pasta temporária (atualizar troca o instalado); `remover` move para
`<dados>/provedores-removidos/<servico>` (de onde `restaurar` traz de volta) e só mexe em pasta de dentro da pasta dos
provedores. `Remoto` fecha a thread do provedor antes de mexer na pasta dele (o Windows não deixa mexer na pasta de
um programa aberto) e faz a cópia em segundo plano; os desligados ficam em `ajustes.provedores_desligados`.
Conferido na cópia de teste: a aba com o Exemplo pronto, desligar (o processo fechou e o ajuste gravou), religar,
remover (aviso com "Desfazer", a pasta foi para os removidos) e desfazer (voltou e o provedor reabriu). Não
conferido pela tela: instalar pela janela de escolher pasta (o teste de unidade instala, atualiza e remove).
**Com isso a F1 está completa.**

**A Ilha, quinta passada (21/11/2026, a pedido do Rafael)**: arrastar é sempre livre; o cadeado só decide se ela
vai a qualquer borda (destravada) ou desliza pela borda dela (travada, `ilha::lugar_travado`, esticando ao ser puxada
para dentro). Vários monitores: arrastando, ela passa para o monitor do ponteiro (`VistaIlha::trocar_de_monitor`
converte as molas para a forma não pular) e o nome do monitor fica em `ajustes.ilha_monitor` (vazio = o da janela
principal); também por Configurações › Ilha › Monitor, pelo bloco Monitor na página Ajustes da Ilha e pelo comando
`IlhaOutroMonitor`. Em pé (barra lateral), as páginas foram refeitas: abas num controle segmentado, Tocando numa
coluna com a capa do maior tamanho que cabe, Ritmo e Ajustes enchendo a altura (8 blocos: 4 por linha deitada, 2 em
pé). As regras 1 e 2 (seção 2) perderam as limitações de serviço; o único limite é não quebrar DRM nem baixar música.

**Auditoria (17/11/2026)**, em [`docs/auditoria-2026-11-17.md`](docs/auditoria-2026-11-17.md): a tremida da Ilha
achada e corrigida com o diagnóstico quadro a quadro (posições fracionárias arredondadas pelo GPUI, região trocada a
cada quadro e a janela redimensionada na troca de tamanho); contraste do texto terciário corrigido sobre blocos e
seleção; a janela principal respeita o "Efeitos de animação" do Windows e o modo seguro; atalhos H, B e L; piso de
11 px no texto da Ilha; `normalizar` e `smoothstep` repetidos juntados; no empilhado, Biblioteca e Loja ficam com o
meio inteiro (`Pedido::pagina_cheia`) e o cabeçalho delas fica compacto em janela estreita. Conferido: os números
antes e depois, os testes e a cópia de teste por `PrintWindow` (380, 900 e 1280 px). Não conferido: a tremida e a
fresta na tela de verdade.

**A Ilha, quarta passada (16/11/2026, a pedido do Rafael)**: o visual da página Visual é preparado em outra
thread quando a Ilha abre (a troca de página não trava), desenhado 4 px maior por baixo do recorte (sem fresta), e o
recorte só abre com ele já na tela; a janela ganhou folga para o quique e a região só fica justa quando a forma
assenta (sem a "tremida" e sem cortar os cantos de baixo); o conteúdo de antes sai num esmaecimento curto; os
blocos ligados dos Ajustes ficam brancos; a letra (Ilha e painel) rola numa mola e a linha cantada enche como num
karaokê, palavra por palavra quando o LRC traz os tempos (`letras::Palavra`, `letras::cantado`,
`ui::letra_enchendo`). Conferido na cópia de teste fora da vista (`TCCPLAYER_ILHA_ESCONDIDA`), por `PrintWindow`:
o karaokê na Ilha e no painel com uma letra inventada posta no cache de teste (enchendo no meio da palavra certa),
o Spectral animando no recorte, "Cores reativas" branco nos Ajustes, a página Tocando, a camada da Ilha nascendo em
segundo plano e a janela no tamanho novo (640x242). Não conferido: a fresta e a tremida na tela de verdade (o
`PrintWindow` pinta o transparente de preto e foto parada não mostra tremida), a troca para a página Visual sentida
com o mouse e uma letra real do LRCLIB com tempos por palavra.

**A Ilha, terceira versão (16/11/2026)**: arrastar pelo topo (ímã no centro, lembra o lugar), some com o Player
maximizado ou em tela cheia na frente, tamanhos mínima/padrão/grande, páginas Tocando, Letra, Visual (o visual de
verdade num recorte), Ritmo e Ajustes, abre em 30 ms e fecha em 200 ms, encostada de verdade no topo (sem a borda do
GPUI). Configurações responsivas (no painel em janela estreita, fichas no topo em janela média) e troca de tema
rápida. Conferido na cópia de teste: a borda do topo (a primeira linha da tela fica preta), as cinco páginas (o
Spectral aparece dentro da Ilha), a área de desenho igual à janela, o arrasto com ímã e a posição gravada (o mouse
do Rafael passou por ela durante o teste) e as Configurações a 380 px. Não conferido: sumir com o Player
maximizado na frente (precisaria trazer a janela de teste para a frente), o hover real e a troca de tema na tela.

**A Ilha refeita (16/11/2026, a pedido do Rafael: "muito delay", "nada UX")**, no estilo do Nusa e do island:
entalhe preso ao topo com ombros, abre em 60 ms e fecha em 250 ms, cresce com quique, letra ao lado, × para
fechar, opcional (desligada por padrão) com botão no dock e seção nas Configurações, que viraram uma página. Na
cópia de teste: forma em repouso e aberta vistas na tela de verdade (ombros conferidos ampliando), página
Configurações e seção Ilha. Não conferido: o hover real com o mouse, a letra ao lado com uma música que tenha letra
sincronizada, e a região de clique deixando passar cliques para outro app.

**A primeira versão da Ilha (16/11/2026), conferida numa cópia de teste** (com a tela do PC bloqueada, só por captura da própria
janela): aparece no topo ao tocar; avisa a música nova (alarga para 340 px) e o drop ("Drop em 3" a 2,7 s do drop
da pré-análise); abre com um clique; pausar e tocar por ela agem no Player; com a janela principal minimizada ela
segue andando (o tempo avança e toca pela Ilha); "abrir o Player" restaura a janela; fechar a janela principal
encerra o app junto com a Ilha; some após 30 s parada. **Não conferido:** abrir com o mouse parado em cima (não
dá para mexer no mouse do Rafael), a transparência e os cantos na tela de verdade, sumir com jogo ou vídeo em
tela cheia, o aviso de volume e o "Efeitos de animação" do Windows desligado.

**Player local, parte 2 (13/11/2026), sem conferir no app:** o som de verdade do crossfade e do equalizador (as
cópias de teste rodam mudas; a mistura tem testes de unidade), o ReplayGain em arquivos reais, a capa da música
local no controle de mídia do Windows e arrastar nas barras do equalizador (só o clique foi testado).

**Feito, mas não visto funcionando de ponta a ponta:**
- Last.fm e Discord: as chaves existem desde 14/11/2026 (variáveis de usuário do Windows; programas abertos antes
  não as veem). **Last.fm conferido de ponta a ponta em 14/11/2026** (autorização no navegador, conta
  conectada, scrobble visto na conta). O Discord ainda não: o app de desktop do Discord precisa estar aberto (a
  versão no navegador não tem o canal local); o Player procura o Discord a cada 10 s. **Cartão visto no perfil
  do Rafael em 14/11/2026** (sem capa). A capa (Deezer, `servicos::capas`) e a leitura das respostas do Discord
  estão no código e nos testes, mas ainda não numa release testada: falta compilar e conferir no perfil.
- Som real do player local e arquivos MP3, FLAC e M4A: os testes usaram WAVs sintéticos, sem som.
- Janela de escolher pasta, capa embutida nas músicas locais, contagem das músicas locais no histórico.
- Clique nos botões da miniatura da barra de tarefas e aparência do menu da bandeja.
- Digitação num teclado físico com IME (foi testado com mensagens simuladas).
- Captura por app em Windows anteriores ao 10 21H1 (deve cair num aviso, sem travar).

**Dívidas conhecidas:**
- Faixa de provedor é baixada duas vezes (o tocador e a pré-análise abrem cada um o seu `LeitorHttp`); juntar num
  cache de pedaços compartilhado.
- O exe de teste do `provedor-youtube-music` costuma ser bloqueado pelo erro 4551 quando roda com o workspace; ele
  passa sozinho (`cargo test -p provedor-youtube-music`). Use `--no-fail-fast` para ver o resto.
- O teste `visual --test desenho` (usa a placa de vídeo) travou uma vez por 50 min em 14/11/2026, com um jogo
  aberto; os outros testes passam. Se travar, rode `cargo test --workspace --exclude visual` e o dele à parte.
- Sem CI.
- Skins KDL carregadas de arquivo (plano da fase 1) não existem; os temas estão no código.
- Medidor de flashes do modo seguro (fase 3) não existe.
- O Spectral do mockup (`mockup/project/Main.dc.html`) ainda é o antigo.
- Nada de capa da música local no controle de mídia do Windows (`cover_url: None`).

---

## 8. Próximas fases (plano v2, aprovado em 16/11/2026)

O plano completo, com o que copiar de cada projeto, está em [`docs/plano-v2.md`](docs/plano-v2.md). Ordem:

| # | Fase | Situação |
| --- | --- | --- |
| F0 | Base aberta: GPL-3, `LICENSE`, `THIRD-PARTY-NOTICES.md`, regras novas, `app/deny.toml`, README e CONTRIBUTING | Feita em 16/11/2026 (o `cargo-deny` não instala neste PC: o erro 4551 bloqueia os build scripts dele; as licenças foram conferidas pelo `cargo metadata`) |
| F1 | Núcleo de provedores: protocolo, modelos, fila com vários serviços, motor tocando streams, cache de análise por ID/ISRC, casador entre serviços | Feita em 22/11/2026 (crate `provedores`, tocador por endereço, cache por ID/ISRC, `remoto.rs`, busca "Nos serviços", fila mista, aba Provedores na Loja); "Neste computador" como provedor ficou para a F2 |
| F2 | Páginas unificadas e layouts (Palco, Estante), molas no `anim.rs` | Feita em 23/11/2026, em 6 partes: identidade e páginas dos provedores; curtir, salvar, fixar e playlists mistas; espaços e navegação; páginas, busca de todos os tipos e página da faixa; o layout Estante; comandos e documentos ([`docs/f2-paginas.md`](docs/f2-paginas.md)). Ficaram a página de podcast e o rádio |
| F3 | Provedor YouTube Music | Feita em 23/11/2026: `app/provedores/youtube-music`, embutido; metadados pelo `rustypipe` e áudio pelo cliente visionOS, como MixMusic e limusic ([`docs/f3-youtube-music.md`](docs/f3-youtube-music.md)) |
| F4 | Letras 2.0: fontes em cadeia, palavra por palavra, tela cheia, tradução, romanização | Feita em 22/11/2026 (`letras::fontes`, `ttml`, `traducao`; Configurações › Letras; `ui/letra_cheia.rs`); ficaram estilos e bolinha, cartão e a letra no visual ([`docs/f4-letras.md`](docs/f4-letras.md)) |
| F5 | **A Ilha** (Dynamic Island): janela flutuante no topo, estados repouso/alerta/expandida | Feita em 16/11/2026 (ver seção 7) |
| F6 | Provedor Spotify: o áudio pelo librespot, o Soloist ou o Match pelo YouTube Music | Feita em 23 e 24/11/2026, em 5 partes: login, dados, Match, Soloist e o áudio pelo librespot no motor do Player ([`docs/f6-spotify.md`](docs/f6-spotify.md)). O Soloist ligou e pareou, mas o build 1.4.0.7 do Spotify cai ao tocar (seção 7); ficaram Canvas, as letras do Spotify e o ISRC |
| F7 | Modo Escuta preciso + Shazam (SongRec): reconhecer, pré-analisar e alinhar ao som ao vivo | |
| F8 | Som e fila: crossfade no compasso, modo DJ, smart shuffle, lixeira, A-B, timer, AutoEq | |
| F9 | Juntos e controle: salas, controle pelo celular, MCP, overlay OBS, Chromecast | |
| F10 | Loja de verdade: registro no GitHub, temas em arquivo, Material dinâmico, plugins | |
| F11 | Mais provedores: SoundCloud, rádios, podcasts, sem perdas | |
| F12 | Extras: retrospectiva, backup, atualização automática, atalhos globais, offline, MilkDrop, karaokê, medidor de flashes | |

Sobras das fases antigas que continuam valendo: renomear playlist, arrastar para reordenar, capas nos álbuns,
ReplayGain de álbum, R128 para quem não tem etiqueta, *Beat This!* e compassos que não são 4/4, importar
histórico (vai na F12), acabamento (instalador, CI, medições, teste com usuários). O ListenBrainz saiu do plano a
pedido do Rafael.

---

## 9. Como entregar uma tarefa

1. Leia os arquivos que vai mudar antes de editar; siga o estilo que já existe.
2. Escreva testes para a lógica nova (há testes em quase todos os crates).
3. Rode `cargo fmt --all`, `cargo clippy --workspace --all-targets` (zero avisos) e `cargo test --workspace`.
4. Se mexeu na interface, abra uma cópia de teste (seção 5) e confira.
5. Atualize o `app/README.md` (o que o usuário vê) e, se mudou o estado do projeto, as seções 7 e 8 deste
   arquivo.
6. Diga com clareza o que foi testado e o que não foi.
