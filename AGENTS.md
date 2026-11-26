# AGENTS.md: contexto para qualquer agente de IA

Leia isto antes de mexer no projeto. Vale para Cursor, Claude Code, Codex ou um chat novo. Atualizado em
26/11/2026. **Responda sempre em português do Brasil.**

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
cargo test --workspace                   # 604 testes em 26/11/2026 (fora os 3 do `visual --test desenho`), todos passando
cargo clippy --workspace --all-targets   # tem que sair sem aviso
cargo fmt --all
cargo deny check licenses sources        # licenças e fontes (app/deny.toml)
```

- **CI** (`.github/workflows/ci.yml`, desde 26/11/2026): a cada envio ao `main` e a cada pedido de mudança, no Windows
  (Rust 1.99.1, o mesmo daqui): `fmt --check`, `clippy -D warnings` e `cargo test --workspace --locked`; numa máquina Linux,
  `cargo deny check licenses sources`. O servidor não tem placa de vídeo: `TCCPLAYER_SEM_GPU=1` faz o teste que desenha os
  visuais terminar sem falhar (e, com placa, ele sai com erro se passar de 3 min, em vez de travar).
- A pasta de build é `D:/Rust/target`, e o cache do cargo fica em `D:/Rust/cargo`.
- `app/.cargo/config.toml` aponta o `fxc.exe` do Windows SDK 10.0.22621, que o GPUI usa para os shaders.
- Rust edition 2024, MSRV 1.88. Lints do workspace: `unwrap_used`, `expect_used`, `panic`, `todo` e `dbg_macro`
  geram aviso; `unsafe_op_in_unsafe_fn` é erro. Em teste, use `#[allow(clippy::expect_used)]` no módulo.
- Variáveis de ambiente para teste:
  - `TCCPLAYER_DADOS=<pasta>`: pasta de dados separada (ajustes, bancos, registro). Permite rodar uma cópia ao
    lado do Player do Rafael. Uma cópia por pasta: abrir outra com a mesma pasta traz a aberta para a frente e fecha
    (o mutex tem a chave da pasta, `nucleo::caminhos::chave_da_pasta`).
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
| `audio` | Captura do som: loopback da saída padrão (`cpal`) ou de um app só (process loopback em ponto flutuante, `processo.rs`); anel sem trava; a `Derivacao` (a música do Player antes do volume); volume do sistema e o de um app no misturador (`volume.rs`) |
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
| `provedores/spotify` | O provedor do Spotify (F6, um programa à parte, embutido; o Soloist em `soloist/`): login pelo navegador com o Client ID público do librespot (`spotify.rs`, o fluxo com PKCE em `oauth.rs`), a conta e o refresh token cifrado (`conta.rs`), quem usa a porta 8898 (`porta.rs`); os dados pelo Pathfinder com a sessão (`pathfinder.rs`, `codigos.rs`), a conversão pura (`conversao.rs`) e as páginas (`dados.rs`). Compila só com `vergen` 9.0.6 no `Cargo.lock` |
| `provedores/youtube-music` | O provedor do YouTube Music (um programa à parte, GPL-3.0-only por causa do `rustypipe`): metadados pelo `rustypipe`, áudio pelo cliente visionOS (`visionos.rs`). Embutido: o Player o acha ao lado do `player.exe` (`remoto::EMBUTIDOS`) |

### Princípio das threads

Cada coisa lenta ou que pode travar tem uma thread própria: `escuta`, `sessao`, `gravador`, `historico`,
`letras`, `lastfm`, `discord`, `tocador`, `biblioteca` e `escuta-app`. Elas publicam o último resultado num
`Arc<Mutex<_>>`, às vezes com um contador de versão. **A interface só lê o que está pronto e nunca espera nenhuma
thread.** Pedidos seguem por canal (`mpsc`).

### Arquivos principais do `player`

- `app.rs`: o `Player` (estado de tudo), `executar(Comando)`, `quadro()` (roda a cada frame), teclado.
- `escuta.rs`: thread do Modo Escuta; `Fonte::{Tudo, App(exe), Proprio, DoPlayer(exe)}`. Com uma música do Player, ouve só ela:
  `Proprio` lê a `Derivacao` do tocador (antes do volume, sem o Windows); `DoPlayer` é o Soloist, com o ganho da mesa.
- `mesa.rs`: o volume só da música que toca por fora (o Soloist), no misturador do Windows, e o ganho que o desfaz para o visual.
- `local.rs`: player local (thread da biblioteca, tocador com a seguinte pré-carregada, SMTC via `souvlaki` com
  a capa).
- `remoto.rs`: as músicas dos provedores (F1): uma thread por provedor instalado (`<dados>/provedores`), a busca
  em todos ao mesmo tempo (depois de 350 ms sem digitar) e o endereço de cada faixa. A faixa de provedor vira uma
  `biblioteca::Musica` com `remota` preenchida e a chave `servico:id` no `caminho`; o `local.rs` pede o endereço
  antes de tocar (a de agora toca quando ele chega; a seguinte é resolvida antes, para entrar sem intervalo).
- `preanalista.rs`: thread da pré-análise (prioridade: a de agora e as duas seguintes; depois a biblioteca, em
  prioridade baixa; análise da biblioteca interrompida quando chega prioridade).
- `cadeia.rs`: o que fazer quando uma faixa de serviço não toca, pela causa do erro (tentar de novo, esperar ou pular).
- `menu.rs`: o menu do botão direito e do ⋯, só lógica e testada (`Alvo`, `Acao`, `entradas`, `lugar`); `app/menu.rs` abre e executa (e as abas do
  painel, `AbaDoPainel`), `ui/menu.rs` desenha (`ao_menu`, `botao_mais`).
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
  `Pedido.expansao`; `Arranjo.miolo`, `divisoria`, `painel_na_pagina` e `interno`, o tamanho em que o visual é desenhado).
- `detalhes.rs`: as contas das páginas de artista, álbum e playlist (abas do artista, números, datas, o véu sobre a imagem, `mesmo_nome`);
  a tela é `ui/detalhe.rs` (o cabeçalho com imagem, `fundo_do_cabecalho`, `cantos_da_folha`).
- `descanso.rs`: a página da Estante que sai da frente quando ninguém mexe (só a lógica; `Player::descansar` e `so_visual_na_estante`).
- `ui/`: `casca` (topo e trilho), `heroi`, `painel` (módulos e Ajustes), `reproducao` (crossfade, volume
  igualado e equalizador nos Ajustes), `dock`, `paleta` (Ctrl K), `busca_letra`, `campo` (campo de texto),
  `biblioteca`, `musicas` (categorias, visões, ações da linha, `campo_de_busca`), `fila`, `playlist` (janela), `loja`,
  `modulo` (ficha), `destinos` (lateral, trilho e barra de baixo, e as abas sublinhadas), `espacos` (o filtro de serviço), `paginas` (Início, álbum,
  playlist, artista e as peças), `biblioteca` (a Biblioteca única; as abas Pastas e Histórico são `musicas` e `historico`), `busca` (página
  Buscar), `faixa` (página da faixa), `estante` (a lateral, o visual no centro e a divisória), `selo` (a marca dos serviços, o disco e o botão de tocar), `linha` (texto com "…"), `mod.rs`
  (botões, dicas e ajudantes).
- `recursos/`: fontes, ícones SVG e os manifestos dos módulos (`recursos/modulos/*.toml`).

### Convenções

- Nomes de código, comentários e textos em **português** (`tocar_musica`, `Acervo::buscar`). Comentários dizem o
  porquê, em frases curtas.
- Toda ação do usuário é um `Comando` (`crates/nucleo/src/comando.rs`), com nome, atalho e entrada na barra de
  comandos.
- Botões passam por `self.botao(...)` e `self.botao_icone(...)`; as animações por `self.anim.seguir(chave, alvo,
  tempo)`; as dicas por `marca(id)` e `ao_pairar(...)`.
- **Medidas:** todo tamanho de texto, ícone e raio sai de `ui/medidas.rs` (`medidas::corpo()`, `ICONE`, `raio_da_capa`...). O teste
  `sem_numero_solto` falha com número solto nas telas (a Ilha tem escala própria). Texto com "…" usa `ui::linha(texto)`
  (ou `linhas(texto, n)`), nunca o do GPUI: o `truncate` corta sem "…", e o `text_ellipsis` fecha o app em texto com acento
  (`ui/linha.rs`; um teste confere). Sem caixa alta e sem " · " separando dados.
- **Tamanho da interface:** nas telas, o `px` é o de `ui/medidas.rs` (multiplica pela escala do ajuste `tamanho`), e o arranjo é calculado
  na janela dividida por ela. O que vem do mouse, das áreas medidas e da rolagem está em pixels de verdade: converta com `medidas::logico`
  (e use `gpui::px` para escrever um valor real, como dentro de um `canvas`). A Ilha desenha em escala 1 e devolve a de antes.
- **Imagem de canto redondo:** o raio vai no `img`, e a imagem precisa ser quadrada (`capas::arquivo_para` recorta): o GPUI só recorta
  filhos em retângulo, e `ObjectFit::Cover` desenha a imagem para fora do quadrado.
- Ajustes novos: um campo em `Ajustes` com `#[serde(default)]` e correção em `validar()`.
- Edições de texto: um campo novo usa `ui/campo.rs` (`receber_texto`, `conteudo_do_campo`, `EntityInputHandler`).
  Não trate letras pela tecla (`key_char`), porque acento com tecla morta deixa de funcionar.

---

## 5. Ambiente do PC do Rafael (importante)

- **Erro 4551 (Controle Inteligente de Aplicativos)** bloqueia de vez em quando um exe ou uma DLL de proc-macro
  recém-compilados ("can't find crate for X", "LoadLibraryExW ... 4551"). Solução: `cargo clean [--release] -p
  <pacote>` e compilar de novo. Se quase todas as bibliotecas somem de uma vez, o bloqueado é um proc-macro comum a elas
  (em 25/11/2026, o `thiserror-impl`). "can't find crate for `gpui`": `cargo clean -p gpui -p gpui-macros`. Um build
  script bloqueado (o do `librespot-protocol` é frequente): `cargo clean -p <pacote>`. Um exe de teste bloqueado ("could
  not execute process ... 4551"): rode o pacote sozinho (`cargo test -p <pacote>`) ou mude um arquivo dele para o exe sair
  com outro conteúdo. **Confira sempre o código de saída do cargo**, porque um `| tail` esconde a falha e o exe antigo
  continua lá. O CI roda tudo sem esse bloqueio. **Cuidado:** `cargo clean -p <pacote>` sem `--release` limpa o pacote também
  na release; em 26/11/2026 a release depois quebrou em cascata ("can't find crate for `symphonia`", `reqwest`... com os
  arquivos lá). Resolveu guardar os `.exe` de `D:/Rust/target/release` e fazer `cargo clean --release` inteiro. Para achar a
  DLL bloqueada: `LoadLibraryExW(dll, 0, 1)` em cada `deps/*.dll` (erro 4551).
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
- **Volume e visual**: a captura de um app vem **depois** do volume dele no misturador (medido: proporcional, até 0,00001, em ponto
  flutuante; a mudança chega uns 50 ms depois). A `mesa` desfaz pelo **maior** volume dos últimos 80 ms: o visual pode piscar para baixo,
  nunca estourar. Medindo no mesmo programa que toca, a conta sai ao quadrado (ele se ouve com o próprio volume): meça de outro processo.
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
Spotify toca pelo Soloist (o player oficial) ou pelo Match do YouTube Music. Tocar pelo librespot saiu na revisão de 23/11/2026
(decifra as faixas protegidas fora do player do Spotify, o que a regra 2 não deixa): ele fica só no login e nos dados. **Parte 1 (23/11/2026)**,
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
navegador), e uma conta gratuita. Ficou: `situacao` desconhecida invalida a apresentação. (O `librespot-oauth` saiu em 24/11.)

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
crmne/spotifast#1412). Um Soloist quebrado não é mais aberto, e o Match toca (regras no parágrafo de 24/11). Ficou: duas instâncias do Player na mesma pasta, trazer a posição ao cair para o Match, e o Soloist e o
Spotifast abertos juntos (dois `soloist.exe` para a captura por nome).

**F6, a branch do Rafael e o Soloist (24/11/2026)**: a `feat/spotify-audio-e-investigacao` (do Rafael) foi revista e o útil veio
para o `main`, refeito; a parte `audio/` (tocar pelo librespot) ficou na branch. **Por que o Spotifast toca e o nosso não**: a versão.
Ele usa o Soloist 1.3.10.51, baixado em 07/11; o Player baixou o 1.4.0.7, que cai. O servidor do Spotify só tem o último
([`docs/spotify-investigacao-2026-11-24.md`](docs/spotify-investigacao-2026-11-24.md)). Agora (`soloist/instalar.rs`): volta ao build
anterior (que fica guardado), dá por quebrado o que cai 2 vezes seguidas sem 15 s tocando ou 4 vezes no dia (só exceções do Windows
contam, `motor::quebrou`), nunca abre o 1.4.0.7, só baixa se o pacote mudou (ETag), oferece copiar o Soloist do Spotifast
(`EstadoDoTocador.alternativa`, `trocar-tocador`, "Usar esse Soloist") e confere a assinatura do Spotify antes de abrir
(`assinatura.rs`). Login próprio (`oauth.rs`, sem `librespot-oauth`): uma conexão parada não prende mais a porta, o `state` é sempre
conferido, e o id do aparelho é estável. Erro com `causa` (`Causa`, `nucleo::CausaDoProvedor`): sem rede, a faixa é tentada de novo e
a fila para nela (`cadeia.rs`, `Local::falhou_o_endereco`). Conferido neste PC: achar, conferir e copiar o 1.3.10.51 do Spotifast (um
teste `#[ignore]` em `instalar.rs`). **Tocou no Player do Rafael**: o Spotify publicou o 1.4.0.12 em
24/11 às 21:02, o Player o baixou sozinho e ele tocou 80 min (umas 30 músicas) sem cair, com login, pausar, continuar, pular, o
fim das faixas e o visual ouvindo só o Soloist. O fim de cada faixa deixava 1,2 a 1,6 s de silêncio (o Soloist diz `buffering` 0,95 s antes do fim e a
posição para, com o som saindo: `leitura::na_cauda`); corrigido e medido em 25/11: de 0,70 a 0,87 s. **Troca sem intervalo (25/11)**:
a 10 s do fim o Player manda `Acao::Preparar` com a próxima, o provedor a põe na fila do Spotify (`add_to_queue`, só com ela vazia) e
o Soloist passa para ela sozinho (`Pedida::adiantar`, `passar_para`; `Local::avancou_no_externo`); pular ou parar antes a consome com
`skip_next`. Conferido no Player do Rafael, quase mudo (4 trocas): a próxima começa de 30 a 50 ms depois do fim, com a fila e a tela
andando juntas, também depois de pausar e continuar; pular nos últimos 10 s toca a certa e deixa a fila do Spotify vazia. Para testar sem
incomodar: o volume só do Soloist pela API local (`set_volume`; o medidor não vê abaixo de 6). Não conferido ao vivo: o fim da fila (o
que o Spotify emenda depois da última), o botão "Usar esse Soloist" na tela, tocar com o 1.3.10.51 pelo Player e outro aparelho tomando a vez.

**Volume só da música (25/11/2026, a pedido do Rafael)**: com a música do Player na tela, a barra do dock, `Ctrl ↑/↓` e `M` mexem só
nela (`ajustes.volume_musica`; o ganho é o quadrado da barra): no tocador daqui, na saída (`saida::preencher`, aos poucos, sem estalo);
no Soloist, pela sessão dele no misturador (`mesa.rs`, só o processo que o Player abriu). O visual não sente (ver "Volume e visual" na
seção 6), e a escuta ouve só a música do Player, mesmo com "todo o som". Conferido: na cópia muda, uma música local pré-analisada
continua "Intensa" 0,84 com a barra em 100, em 9 e no mudo; no Soloist do Rafael, quase mudo, o que se captura com o volume do programa
em 100%, 25%, 1%, 0,01% e 0,001% e o ganho da conta é igual. O Rafael testou no Player dele, no Spotify: ficou bom.
Não conferido: a barra seguindo o misturador do Windows, e a Ilha.

**Remodelagem, parte 1 (25/11/2026)**, medidas e consertos ([plano](docs/superpowers/plans/2026-11-25-remodelagem-parte1.md)):
`ui/medidas.rs` com os sete papéis de texto (16 de base, 14 de apoio), ícones de 24 (traço 1,8), linhas de 64, fichas de 40, botões de 44;
topo 64, dock 96 (88 em janela baixa), navegação 72, lateral 264. Capas arredondadas (e redondas as de artista), recortadas e com miniatura
de 128 px (`capas::Versoes`: `.quad` e `.mini` ao lado da capa). A busca dos serviços e a da Biblioteca têm textos separados
(`navegacao::Buscas`). No dock, o meio fica com o que sobra dos lados (o tempo não fica mais embaixo de um botão); Ilha e mini saem abaixo
de 860 px, aleatório e repetir abaixo de 440. Na folha estreita os espaços e as abas viram só ícones e cabem dois cartões por linha.
Conferido na cópia muda com o YouTube Music e o Spotify: Estante em 1920, 1440, 958 × 560, 700, 410 e 400 × 420, Palco em 1440, 958 × 560 e
410, Loja, Configurações, busca, playlist, uma faixa de vídeo tocando, a busca que não vaza mais, Fila vazia, Histórico, modo edição, o tema Papel e a barra de
comandos. Não conferido: a janela mini, a letra em tela cheia com uma letra, a ficha de um módulo na Loja, as janelas de playlist e de
letra, uma pasta de músicas do computador e a Ilha (a capa dela agora é recortada). Ficou: na linha, as ações escondidas ainda ocupam lugar e a lista de artistas é cortada sem "…" (parte 4).

**Remodelagem, parte 2 (25/11/2026)**, navegação e busca única ([plano](docs/superpowers/plans/2026-11-25-remodelagem-parte2.md)):
`navegacao.rs` refeito: uma pilha só de passos (`id`, rota, filtro); `Espaco` virou o filtro de serviço (`trocar_filtro` não cria passo;
numa página sem filtro abre o Início); `Rota::Inicio`, `Rota::Biblioteca(AbaDaBiblioteca)`, sem `Historico`; `esquecer_servico` limpa a pilha;
a rolagem é um `ScrollHandle` por passo (`rolagem_da_pagina`: o GPUI esquece a de um elemento que sai da tela). O campo com o teclado é
`Player::campo` (`Campo::{Busca, Biblioteca}`, `campo_no_teclado`), com seleção total (`campo_selecionado`). A busca do topo procura também
no computador por `Pedido::Procurar` (`Lista::achadas`), sem mexer no filtro da Biblioteca; `ajustes.buscas_recentes`. Os comandos moram na
busca (`>`, aba Comandos, `executar_da_busca` volta à página de antes se `busca_de_passagem`); a barra por cima só com `so_visual`. A tinta do
serviço saiu das telas (era da parte 3), os cartões preenchem a largura (`lado_do_cartao_em`, da parte 7) e as ações da linha ficam por cima
(`acoes_por_cima`, da parte 4). A busca
é agrupada uma vez por resposta (`grupos_da_busca`) e a lateral só monta as linhas à vista. **Queda achada e corrigida**: o `uma_linha` da parte 1 (`line_clamp` + `text_ellipsis`) fechava o app em texto
com acento medido em duas larguras (`direct_write.rs`, "not a char boundary"); agora é o elemento `ui/linha.rs`. Conferido na cópia muda com o
YouTube Music e o Spotify: Estante em 1920, 1440, 958 × 560, 700 e 410, Palco em 1440, tema Papel; busca, `>` com setas e Enter (volta à página
de antes), a barra `/` do teclado, buscas recentes, o texto selecionado, trocar o filtro, voltar com a rolagem, as abas da Biblioteca, ordem por
nome, Histórico, Fila, uma pasta de músicas (WAVs de teste) na busca e na Biblioteca, e um álbum do YouTube Music tocando. Não conferido:
`Ctrl Tab`, `Ctrl 1..9` e `Alt ←/→` (as mensagens de teste não levam Ctrl nem Alt), a Biblioteca em linhas, Curtidas com faixas, uma playlist do
Player nos cartões, a skin imersiva, a barra de comandos na tela cheia, um provedor que sai no meio (só os testes). No primeiro teste o YouTube
respondeu 403 a todas as faixas por um instante e depois voltou sozinho (a fila acabou vazia): não é da navegação, e não foi investigado.
Ficou: Principais em blocos por serviço (parte 3 mistura), "Abertos há pouco" só desta abertura (parte 3 guarda).

**Remodelagem, parte 3 (25/11/2026)**, Tudo misturado e selos ([plano](docs/superpowers/plans/2026-11-25-remodelagem-parte3.md)):
`juntar.rs` (lógica): `juntar_inicio` (a k-ésima prateleira de um tipo com a k-ésima do outro serviço; as sem tipo intercaladas), `intercalar` (a
mesma música, por ISRC ou nota a partir de 0,9 no `casador`, vira um `Junto` com `tambem`), `juntar_busca`, `saudacao`, `servico_do_app`. O protocolo
ganhou `Prateleira.tipo` (`TipoDaPrateleira`, desconhecido vira `Outras`); o Spotify adivinha o tipo pelo nome da seção, pelo título e pelos itens
(`tipo_da_secao`). `biblioteca.db` ganhou `recentes` (até 60: `registrar_recente`, `Local::recente`), escrito ao abrir álbum, playlist ou artista e
quando uma música de serviço começa a tocar. `ui/selo.rs`: `Marca`, `disco`, `discos`, `com_selo`, `selo_do_cartao` (o selo que vira tocar);
`Player::tocar_item` espera a página e toca (`tocar_ao_chegar`, 25 s). `ajustes.servico_preferido`. O Histórico não ganhou coluna: o serviço sai do
nome do app de cada execução. Conferido na cópia muda com o YouTube Music e o Spotify: Início em Tudo (atalhos, prateleiras, "Lançamentos" juntos),
tocar uma playlist do Spotify pelo cartão (pelo Match, com os dois selos na barra e na Fila), a busca com a mesma música em dois selos, "Abertos há
pouco", Configurações › Serviços, a Ilha aberta com o selo, em 1920, 1440, 958 × 560 e 410. Não conferido: escolher o serviço preferido na tela (só a
lógica), tocar um álbum ou artista pelo cartão (só playlist), "Tocadas há pouco" com três músicas, um provedor sem marca própria, o selo no Histórico e o tema Mono.

**Remodelagem, parte 4 (25/11/2026)**, barra, painel e menu ([plano](docs/superpowers/plans/2026-11-25-remodelagem-parte4.md)): na barra, a capa e o
título levam ao álbum (sem o id dele, à página da faixa) e o artista à página dele (`paginas::destino_do_titulo`, `destino_do_artista`; `Remota` ganhou
`album_id`, `artistas` e `artistas_ids`, só na memória), o coração, e à direita letra, fila, painel, volume e ⋯; abaixo de 1100 px a coluna do nome fica
com pelo menos 264, e abaixo de 800 (`barra_apertada`) a letra, o coração e o mudo vão para o ⋯. O painel tem as abas Agora, Fila e Letra
(`aba_do_painel`, só na memória; `alternar_no_painel`): `Q` e `U` (comando novo `AbrirLetra`) abrem a aba, e sem lugar (Estante com menos de 660 px de
altura, janela estreita) a página da Fila e a letra em tela cheia. O menu: cartões, linhas, fila, barra, melhor resultado e visual; "Não são a mesma
música" grava em `<dados>/separadas.json` (`juntar::Separadas`, `Regras`). O cabeçalho do álbum, do artista e da faixa põe os botões numa linha própria
em folha com menos de 640 px. Conferido na cópia muda com o YouTube Music e o Spotify: o menu no cartão, na linha, na fila, na barra e no visual (setas,
Enter, Esc), tocar em seguida, separar (o arquivo e as duas linhas), os links da barra (faixa, álbum, artista), as abas Fila (abre na de agora) e Letra
(com letra de verdade), `Q` e `U`, em Estante 1920, 1440, 958 × 560 e 410 e em Palco 1440 e 958 × 560. Não conferido: pôr na playlist, tocar em ordem aleatória e pôr
um álbum na fila pelo menu, fixar pelo menu, "Tocar pelo outro", o menu de uma música do computador (só os testes da lógica), o ⋯ com o mouse de
verdade, "Saída de som do Windows", o tema Mono. Ficou: a aba Fila não arrasta para reordenar (subir e descer estão no menu).

**Remodelagem, parte 5 (25/11/2026)**, o visual no centro ([plano](docs/superpowers/plans/2026-11-25-remodelagem-parte5.md)): na Estante, `[navegação]
[visual] [divisória] [página]` a partir de 761 px (lateral a partir de 1200, trilho abaixo), 52 : 48 a partir de 1680, 46 : 54 a partir de 1200 e 48 : 52
abaixo; `ajustes.divisao` (dois cliques na divisória voltam ao padrão), `ajustes.pagina_larga` (comando `AlternarPaginaLarga`, tecla W: o visual vira
uma tira de 96 ou 72). O painel da Estante é `gaveta_aberta` em toda janela (vem fechado): ao lado a partir de 1680 (`arranjo::PAINEL_AO_LADO`), senão no
lugar da página, e abrir uma página o fecha (`tirar_o_painel_da_frente`); `P`, `Q` e `U` valem em qualquer altura. Até 760 px o visual fica no alto: 40% no
Início (vira tira ao rolar, `arranjo::recolhe`), tira de 72 nas outras. `Destino.interno` agora **recorta** em vez de esticar (`motor::recorte`,
`U.v[3]` do `saida.wgsl`); o Spectral passeia menos e encolhe em visual estreito (`passeio_que_cabe`, `escala_que_cabe`; em 16:9 nada muda). A página sai
da frente (`descanso.rs`, `ajustes.descanso_s`, 30 s): só com música tocando, a janela na frente, sem painel, campo, menu ou edição; o ponteiro precisa
andar mais de 3 px, e o clique que acorda não vale como clique. Loja: as abas viraram sublinhadas. Conferido na cópia muda com o YouTube Music: Estante em
1920, 1440, 958 × 560, 800, 410 e 410 × 560 (lado a lado, larga, painel ao lado e no lugar da página, Loja, Configurações, Biblioteca, busca, álbum, edição),
arrastar a divisória e o ajuste gravado, dois cliques, X no meio e no fim, dormir, mouse parado, acordar com o mouse e com o clique, rolar o Início em 410,
os três visuais, o menu do visual e o Palco em 1440 e 958 × 560. Não conferido: a fluidez de arrastar e das transições na tela de verdade (só fotos), o cursor sumindo,
o descanso com o foco de verdade (a mensagem de ativar foi simulada), o custo do recorte em quadros, o tema Papel e o Mono. Ficou: no empilhado, X a partir
do visual baixo dá um salto de tamanho na forma; a página larga não anima a largura da página (só o visual anda).

**Remodelagem, parte 6 (25/11/2026)**, artista, álbum e playlist ([plano](docs/superpowers/plans/2026-11-25-remodelagem-parte6.md)): o protocolo ganhou
`Detalhe` (opcional, em `CorpoDaResposta::Itens { itens, detalhe }`; um estragado vira `None`): o item completo, a imagem larga, a cor, a descrição, as
fotos, seguidores, ouvintes, total, duração, data, gravadora, tipo, dono e prateleiras (`TipoDaPrateleira::{UltimoLancamento, Parecidos,
MaisDoArtista}`); `Album.tipo` (`TipoDeAlbum`, no JSON `tipo_de_album`: a divisão em álbuns, singles e coletâneas é feita no Player, sem repetir a
lista) e `Faixa.ouvidas`. O Spotify tira tudo do que já pedia (`detalhe_do_artista`, `_do_album`, `_da_playlist`; biografia sem HTML, `sem_html`); o
YouTube Music, do `rustypipe`. `PaginaRemota::Itens(itens, detalhe)`. `capas.rs` faz os fundos (`TipoDeFundo::{Desfocado, Largo}`: 480 × 270 desfocado,
ou a foto até 1280 de largura) e mede o brilho de onde fica o texto (percentis 95 e 5); o véu da folha por cima sai de `detalhes::veu`, para o texto do
tema ler com 4,5:1 nos dois tipos de tema. A folha ganhou os cantos por cima (`cantos_da_folha`: o GPUI só recorta em retângulo). Linhas: o álbum sem
a capa repetida e com o número; as ouvidas a partir de 640 px; o álbum de cada faixa da playlist a partir de 760; abaixo de 480, número e coração saem.
"Ver no YouTube Music" e "Ver no Spotify": `Remoto::artista_em` busca o nome no outro serviço, `pares::mesmo_artista` só aceita o primeiro resultado
sendo um artista com o mesmo nome, e o par fica em `pares.json` (`artista:<serviço>:<id>:<destino>`). O artista aberto por um link entra nos recentes
com a foto quando ela chega (`completar_o_recente`). Conferido na cópia muda com o YouTube Music e o Spotify (conta do Rafael, só dados): o artista
nos dois serviços (foto larga, ouvintes ou seguidores, o link de um para o outro e de volta, Populares, Coletâneas, Sobre com os números e a
biografia, rolar), o álbum do YouTube Music, a faixa com a capa desfocada atrás e uma playlist do Spotify (245 músicas, 9 h 47 min, 5 seguidores), em
1440 e 958 × 560. Não conferido: o tema Papel, um álbum do Spotify (a gravadora no rodapé), as fotos do Sobre, um artista sem foto nenhuma, um provedor
antigo na tela (só os testes), a memória da placa com muitas fotos largas abertas. Ficou: a biografia vem no idioma que o Spotify manda (inglês), o
número da faixa (`numero`) não entrou no protocolo, e as fotos largas não são soltas da placa ao sair da página.

**Remodelagem, parte 7 (25/11/2026)**, tamanhos e teclado ([plano](docs/superpowers/plans/2026-11-25-remodelagem-parte7.md)): o Tab anda pela tela
(`Player::focavel`: `tab_index(0)` e um anel de 2 px, uma borda no foco; a sombra do GPUI pinta por baixo de quem não tem fundo, e sem desfoque nem é
desenhada) nos botões, pílulas, opções, destinos, abas, fichas do filtro, cartões, atalhos, linhas, corações, ⋯ e nas seções das Configurações; a ordem é a
do desenho (na Estante: navegação, topo, página e painel, barra, visual). O GPUI clica no Enter e no espaço (ao soltar) e foca no clique do mouse: o anel só
aparece depois de um Tab (`foco_pelo_teclado`), o clique sai do modo e o foco volta à raiz no quadro seguinte (os atalhos de uma letra seguem valendo), e
com um elemento focado espaço e Enter são dele (`tecla_de_foco`), `Esc` volta. A Fila vazia (painel e página) e a lateral vazia ganham "Buscar músicas"
(`buscar_musicas`). A janela vai até 360 × 400 (antes 560 de altura); no empilhado baixo a barra tem uma linha só, de 72 (`DOCK_EMPILHADO_BAIXO`).
Testes: 10 mil linhas (`janela_de_linhas`), a Biblioteca de 10 mil arrumada em menos de 1,5 s, o título de 200 letras sem espaço, 400 × 400 no arranjo.
Conferido na cópia muda: o Tab pela Estante em 958 × 560 (o anel no Início, nas fichas), Enter numa ficha, espaço num botão focado sem pausar a música,
duas músicas de nome enorme (uma sem espaço) na lista, no visual e na barra, e 400 × 400 com a barra de uma linha. Não conferido: `Esc` saindo do modo
(só a lógica), o Tab entrando numa página longa (o GPUI não rola até o elemento focado: ele pode ficar fora da vista), Shift Tab, um leitor de tela (o
GPUI 0.2 não publica a árvore de acessibilidade no Windows). Ficou: rolar até o focado, as ações que só aparecem com o mouse em cima continuam fora do Tab
(estão no ⋯), e o BPM de uma música em silêncio aparece como 240.

**Remodelagem, parte 8 (25/11/2026)**, o tamanho da interface ([plano](docs/superpowers/plans/2026-11-25-remodelagem-parte8.md)): `Ajustes.tamanho` (90, 100,
110 ou 125; um número de fora vai para o mais perto), Configurações › Aparência › Tamanho da interface e `Ctrl +`/`Ctrl −` (`Comando::AumentarInterface`,
`DiminuirInterface`). `medidas::px` multiplica pela escala (em 100% passa como está; nas outras, de 2 px para cima vira pixel inteiro), e todas as telas
importam esse `px` (fora a Ilha e `ui/linha.rs`, que mede em pixels de verdade); a escala fica num estático, definido no começo do desenho de cada janela
(a Ilha põe 1 e devolve a de antes). `quadro` calcula o arranjo na janela dividida pela escala, que baixa sozinha numa janela pequena demais
(`escala_que_cabe`, 320 × 400 lógicos); o visual é desenhado com o fator do Windows vezes ela. Convertem pixels de verdade: a divisória, o menu, as
dicas, o arrasto dos módulos, as setas das prateleiras, a rolagem até o comando, a lista longa e o recolher do empilhado; dentro dos `canvas` (o tocar, a
onda, o mapa) as contas são reais e usam `gpui::px`. Testes: o ajuste, o arredondamento e a janela mínima, e cada tamanho em cada modo do arranjo.
Conferido na cópia: 90% e 125% em 1440 e 958 × 560, 110% em 958 × 560 (o menu do visual abrindo no ponto do clique, o ícone de tocar e a onda no lugar) e a
Ilha igual com a interface em 125%. Não conferido: `Ctrl +` e `Ctrl −` de verdade (as mensagens de teste não levam o Ctrl; o tamanho foi trocado nos
ajustes), arrastar a divisória e os módulos fora de 100%, o tema Papel e uma tela com fator do Windows diferente de 100%. Ficou: o esticão da barra de
volume é medido em pixels de verdade (sai um pouco maior fora de 100%), e em 125% a fileira de filtros de uma folha média rola para o lado.

**Fase 1 do plano v3, arrumar a casa (26/11/2026)** ([plano](docs/superpowers/plans/2026-11-26-fase1-arrumar-a-casa.md)): o CI
(seção 3); o teste da placa sai com erro depois de 3 min. Tab rola a página até o focado (`focos.rs`: um `FocusHandle` por elemento,
pelo id, de um estoque refeito a cada quadro; o focado anota onde foi desenhado e `rolar_a_pagina_ate` em `ui/mod.rs` decide se ele é
da página pelo recorte dele; `focavel` agora recebe `Stateful<Div>`). As fotos largas saem da placa 5 s depois de sair da tela
(`capas::FundosNaTela`, `Player::soltar_fundos`). BPM 0 ("—") sem pulso (`pista::tem_pulso`; antes dava 240). `Faixa.numero`
no protocolo (o Spotify pelo `trackNumber`, o YouTube Music pelo `track_nr`; vale só na lista do álbum). "Tocar" na página da
faixa usa `Local::tocar_agora` (entra depois da de agora, a fila continua). O coração espera a biblioteca atender o último clique
(`Lista.atendidos`, `Local::enviados`). Ids de artista na linha sem colisão (`id_do_artista_na_linha`). Renomear playlist (o chip
"Renomear" da playlist escolhida, a janela de playlist com `renomear`, `Acervo::renomear_playlist`). A capa local vai ao Windows
também pausada. A música que cai do Soloist para o Match continua da posição (`Local::comecar_em`). Uma cópia por pasta de dados:
o mutex tem `caminhos::chave_da_pasta`, e a segunda cópia manda a mensagem registrada "TccPlayer.aparecer" com a chave
(`integracao::pedir_que_apareca`; a janela com a mesma chave faz `mostrar`). A escuta segue só o `soloist.exe` aberto pelo Player
(sem cair no do Spotifast pelo nome). A lista cortada diz "Mostrando N de M músicas" (`detalhes::aviso_de_corte`). Página larga em
dois tempos, sem um passar por cima do outro (o visual baixa e a página desce ao lado; depois o visual alarga e a página desliza por
baixo); no empilhado a cena é a da janela inteira (o X não salta). O esticão do volume em medida das telas. O `main.rs` tinha 20
trechos com acento estragado (desde o commit anterior), consertados. "A lista que falha inteira" já estava resolvido pela
`cadeia.rs` (24/11); tocar a seguir e pôr na fila ficam no ⋯ (que o Tab alcança). Conferido na cópia muda com o YouTube Music e o
Spotify: Tab rolando o Início em 958 × 560 até um cartão da prateleira, a página larga indo e voltando (fotos no meio do caminho),
o X no empilhado em 410 (a forma encolhe sem salto) e o Início do empilhado. Não conferido: a segunda cópia chamando a primeira (traria
a janela para a frente, roubando o foco), renomear na tela, o coração com cliques rápidos, as fotos saindo da placa (só o teste da
lógica), a posição ao cair para o Match, o Spotifast aberto junto, o aviso de corte com uma lista real, o volume em 125% e o CI rodando
no GitHub (precisa do push). O roteiro da sessão com o Rafael está em [`docs/conferencia-fase1.md`](docs/conferencia-fase1.md).

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
  aberto. Desde 26/11/2026 ele sai com erro depois de 3 min.
- Skins KDL carregadas de arquivo (plano da fase 1) não existem; os temas estão no código.
- Medidor de flashes do modo seguro (fase 3) não existe.
- O Spectral do mockup (`mockup/project/Main.dc.html`) ainda é o antigo.

---

## 8. Próximas fases (plano v3, aprovado em 26/11/2026)

O plano está em [`docs/plano-v3.md`](docs/plano-v3.md): o que a remodelagem já resolveu do v2, o que saiu do plano, as regras
para as telas novas e o **inventário de todas as sobras** (seção 3), cada uma com a fase dela. O [plano v2](docs/plano-v2.md)
continua valendo como pesquisa (repositórios, arquitetura dos provedores, riscos). Feitas antes do v3: F0 a F6 do v2 (seção 7
acima) e a remodelagem.

| # | Fase | Situação |
| --- | --- | --- |
| 1 | Arrumar a casa: CI no GitHub, os consertos pequenos e uma sessão de conferência com o Rafael | Feita em 26/11/2026; falta a sessão de conferência ([roteiro](docs/conferencia-fase1.md)) e ver o CI rodar no GitHub |
| 2 | Modo Escuta preciso + reconhecer (a F7 do v2): SongRec, alinhar ao som ao vivo, etiquetar as locais, compassos que não são 4/4 | |
| 3 | Mais provedores (a F11 do v2): cache de pedaços, rádio, SoundCloud, rádios ao vivo, podcasts, conta do YouTube Music, sem perdas, Deezer | |
| 4 | Letras 3: fontes novas, estilos e bolinha, cartão, editor de sincronia | |
| 5 | Som e fila (a F8 do v2): arrastar para reordenar, crossfade no compasso, modo DJ, lixeira, A-B, timer, ReplayGain de álbum, R128, AutoEq | |
| 6 | Visual: MilkDrop, Canvas e capas animadas, a letra no visual, medidor de flashes | |
| 7 | Juntos e controle (a F9 do v2): salas, celular, MCP, OBS, Chromecast | |
| 8 | Loja e personalização (a F10 do v2): registro, temas em arquivo, Material dinâmico, plugins, editar a Início | |
| 9 | Extras (a F12 do v2): retrospectiva, backup, atalhos globais, importar histórico, karaokê | |
| 10 | Entrega: instalador, medições, teste com usuários, material do TCC | |

Saíram do plano: "Neste computador" como provedor (virou o filtro "Neste PC"), os layouts Expressivo e Compacto, os downloads
offline (regra 2) e o ListenBrainz (pedido do Rafael).

**Remodelagem das telas de streaming e da Estante** (decidida em 25/11/2026; as 8 partes estão prontas desde 25/11/2026, seção 7): o serviço vira filtro
(Início, Buscar, Biblioteca), Tudo misturado com selo em cada item, o visual no centro com a página ao lado, medidas num
arquivo só, barra de baixo nova e artista e álbum com imagem, em 8 partes. Auditoria em
[`docs/auditoria-ui-2026-11-25.md`](docs/auditoria-ui-2026-11-25.md), desenho em
[`docs/superpowers/specs/2026-11-25-remodelagem-streaming-design.md`](docs/superpowers/specs/2026-11-25-remodelagem-streaming-design.md),
protótipo em `mockup/remodelagem/`.

## 9. Como entregar uma tarefa

1. Leia os arquivos que vai mudar antes de editar; siga o estilo que já existe.
2. Escreva testes para a lógica nova (há testes em quase todos os crates).
3. Rode `cargo fmt --all`, `cargo clippy --workspace --all-targets` (zero avisos) e `cargo test --workspace`.
4. Se mexeu na interface, abra uma cópia de teste (seção 5) e confira.
5. Atualize o `app/README.md` (o que o usuário vê) e, se mudou o estado do projeto, as seções 7 e 8 deste
   arquivo.
6. Diga com clareza o que foi testado e o que não foi.
