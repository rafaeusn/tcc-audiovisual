# O Spotify tocando no Player: o que descobrimos (24/11/2026)

Este arquivo é para quem for continuar o trabalho. Ele guarda **tudo** o que foi testado, o que funcionou, o que
falhou e por quê — inclusive os caminhos que parecem certos e não são. A ideia é que ninguém repita a investigação.

Comece pela seção **"Onde parou"** e depois leia **"As duas diferenças que faltam"**. O resto é o diário da
investigação, útil quando algo aqui mudar.

---

## Onde parou

**O áudio do Spotify por dentro do Player (librespot) não toca nesta conta, e não é culpa do nosso código.** A causa
está documentada pelo próprio Spotifast (ver seção "A causa do librespot") e não tem conserto do nosso lado.

**O Soloist (o player oficial do Spotify) funciona nesta máquina** — foi provado tocando áudio de verdade. O que falta
é a nossa integração usar o modo certo. Duas diferenças concretas, ambas pequenas de corrigir (seção seguinte).

---

## As duas diferenças que faltam

### 1. A versão do Soloist

O Spotifast está tocando **agora** com o build **1.3.10**. Nós instalamos o **1.4.0**. O 1.4.0 é o build que cai com
`0xc0000005` ao tocar — o mesmo problema que o Rafael já tinha anotado em 23/11/2026.

| | Nossa instalação | A do Spotifast (funcionando) |
| --- | --- | --- |
| Versão | 1.4.0 (build `1795456923`) | **1.3.10** (hash `f466661c06c65c8f98d3c854b8ad30ea0ceb64b5907dfe00281266999b826339`) |
| Tamanho do `soloist.exe` | 20,2 MB | **22,8 MB** |
| Onde | `<dados>/provedores-dados/spotify/soloist/programa/1795456923/` | `%LOCALAPPDATA%\paolino\spotifast\data\soloist\builds\f466661c…/` |

**O nosso download pega sempre a última versão** (`https://soloist-builds.spotifycdn.com/soloist_release_windows_x86_64.zip`,
constantes `SERVIDOR` e `PACOTE` em `soloist/instalar.rs`) — e a última é justamente a que quebra. O Spotifast usa a
1.3.10 porque ela está no CDN dele, fixa.

**O que fazer:** fixar o build 1.3.10 no download (o servidor aceita versão por URL; é preciso descobrir o formato — o
Spotifast guarda o hash, então o caminho provavelmente é
`https://soloist-builds.spotifycdn.com/builds/<hash>/soloist_release_windows_x86_64.zip`, a confirmar). Enquanto isso
não for feito, o Soloist **não deve** ser aberto com o 1.4.0: a regra de "duas quedas = quebrado" já faz isso, e ela
está certa.

### 2. O comando `play` nunca leva a faixa

O Spotifast manda **só** `{"command":"play"}` — nunca `{"command":"play", "uri": "..."}`. Está no código dele, com o
motivo:

```rust
// src/soloist/protocol.rs
RemoteAction::Play if play.is_none() => json!({"command":"play"}),
RemoteAction::Play => return None, // Preserve context, ordering and selected occurrence.
```

Ou seja: **quem escolhe a faixa é o Spotify Connect, não o Player.** O Spotifast deixa o Soloist tocar o que já está na
conta (do app do Spotify, do celular, do que a pessoa escolheu) e usa a API local só para pausar, pular, buscar posição
e volume.

O nosso código faz diferente: manda `{"command":"play","uri":"spotify:track:…"}` e acompanha a faixa. Foi assim que
tomamos o `0xc0000005` em 127 ms — mas **atenção**: o teste foi no build 1.4.0. Ainda não sabemos se o `play` com URI
funciona no 1.3.10. **Teste isso antes de mudar a arquitetura.** Se funcionar, o nosso modelo (processo longo + URI)
continua e a correção é só a versão. Se não funcionar, o modelo passa a ser "o Soloist é um aparelho do Connect; o
Player manda play/pause/skip e mostra o que ele toca".

---

## O que foi testado e o que deu

### O Soloist (provado a tocar)

| Teste | Comando | Resultado |
| --- | --- | --- |
| `--single-track` com a conta pareada | `soloist -n "Player (Soloist)" -k <chave> -D <conta> -s spotify:track:69kOkLUCkxIZYexIgSG8rq -v` | **TOCOU.** Log: `AudioRendererImpl started driver playback`, `onPlaybackStarted`, áudio por 20 s |
| WebSocket sozinho (sem comando) | `soloist -n … -k … -w 127.0.0.1:0 -v` | **Estável** por 15 s. `logged in as 31axzsjldxrjw3er75tdfai2f5ou` |
| WebSocket + `{"command":"play","uri":…}` | pelo nosso Player | **Caiu** com `0xc0000005` em 127 ms (`posicao_s: 0.127`) |
| `-s` **e** `-w` juntos | | Não toca (o modo de faixa não convive com a API) |
| `--single-track` com pasta nova (sem conta) | | Só fica `ready`, não toca |

**Conclusão:** o Soloist 1.4.0 abre, entra na conta, e **toca com `--single-track`**. O que derruba é o `play` pela API.

### O librespot (não toca nesta conta)

Todas as combinações testadas em 24/11/2026, com a conta do Rafael (Premium Universitário), faixa *Get Lucky*:

| Teste | Resultado |
| --- | --- |
| `Credentials::with_access_token` (o padrão) | `audio key error` |
| `Credentials { username: Some(conta), AUTHENTICATION_SPOTIFY_TOKEN, token }` (exatamente o `playback_credentials` do Spotifast) | `audio key error` |
| `device_id` fixo (UUID persistido em arquivo) | `audio key error` |
| `device_id` fixo + username | `audio key error` |
| Credencial reutilizável do librespot (`cache.credentials()`, 2ª sessão) | `audio key error` |
| Esperar o plano chegar antes de pedir a chave | `Some("premium")` desde 0,0 s — **e a chave continuou recusada** |
| Escopo OAuth só `streaming` (com `show_dialog=true`) | Login OK, chave recusada |
| Client ID do Android (`9a8d2f0c…`) | `400 Bad Request` — o `SessionConfig` do 0.8 do crates.io não tem `os_version` |
| Todos os formatos (OGG 96, 160, 320) | Recusados igualmente |
| Três faixas diferentes | Recusadas igualmente |

**A lista de formatos oferecidos inclui `OGG_VORBIS_320`** — ou seja, a conta é reconhecida como Premium e o catálogo
funciona. Só a **chave** é negada.

---

## A causa do librespot (documentada pelo Spotifast)

Está em `docs/_guide/troubleshooting.md` do Spotifast, seção **"Spotify refused the audio key"**:

> **Newer accounts, refused every time.** If your Spotify username is a long string of random letters and numbers that
> Spotify made up for you, Spotify may refuse every key. **Nothing in Spotifast or librespot can change that today.**

A conta do Rafael é exatamente isso: `31axzsjldxrjw3er75tdfai2f5ou`.

E o que **não** resolve, segundo o mesmo documento:

> A [Client ID of your own](/make-it-even-faster/#make-a-spotify-app) doesn't change it either: that only speeds up
> library and search requests, not audio keys.

> Changing audio settings, clearing the cache, signing out and in, or reinstalling won't fix it, because nothing on your
> computer is wrong.

Referência: [librespot#1649](https://github.com/librespot-org/librespot/issues/1649).

**O que resolve, segundo eles:** o Soloist ("*may* avoid the refused-key problem with the built-in player") ou tocar
noutro aparelho. É o caminho que estamos seguindo.

---

## O que foi implementado e está pronto

Tudo abaixo está no código, testado (`cargo test`), com `clippy` e `fmt` limpos:

### `provedores/spotify/src/oauth.rs` (novo)

O fluxo OAuth escrito por nós, com **dois grants separados** — como o Spotifast. O `librespot-oauth` foi **removido**
do `Cargo.toml` porque não deixa escolher o `show_dialog` nem separar os escopos.

| Grant | Client ID | Porta | Escopos | `show_dialog` |
| --- | --- | --- | --- | --- |
| Áudio | Keymaster `65b70807…` | 8898 | **só `streaming`** | **sim** |
| Dados | o app da pessoa | 8989 | Web API | não |

Pontos que valem saber:

- **`show_dialog=true` só no áudio.** Sem ele o Spotify reaproveita a tela antiga e não concede o `streaming`.
- **`rundll32 url.dll,FileProtocolHandler`**, e **nunca** `cmd /C start`: o `&` da URL é separador de comandos para o
  `cmd`, que entregava ao navegador só o pedaço antes do primeiro `&` — o Spotify respondia "response_type must be
  code". Já corrigido e coberto por teste.
- O `state` e o verificador do PKCE são sorteados a cada tentativa, e o retorno confere o `state`.
- O token do áudio **não** serve para `/me` (o escopo é só `streaming`); o username vem da sessão do librespot.

### `provedores/spotify/src/audio/` (parte 3 do librespot)

Cinco arquivos, ~1.100 linhas: `librespot.rs` (formato, chave, decifra em memória), `cache.rs` (pedaços decifrados,
teto de 8 MB e 4 faixas), `servidor.rs` (HTTP local com `Range`), `pedidos.rs` (contas puras), `mod.rs` (liga/desliga).

**Continua válido** e é o caminho para uma conta **antiga** (que não sofra a recusa). Não jogue fora.

Dois defeitos achados e corrigidos no teste ao vivo:

1. O `resolver` devolvia um endereço local **mesmo quando a faixa não abria** — o primeiro pedaço tomava `404` sem
   motivo visível. Agora o erro sai dali com a `causa` certa.
2. `Cache::formato` engolia o erro com `.ok()`; passou a devolver `Result`.

### `player/src/cadeia.rs` (novo)

A cadeia de reserva, pura e testada: **librespot → Soloist → Match**. A `causa` do erro decide:

| Causa | Decisão |
| --- | --- |
| `rede`, `limite` | tenta de novo o mesmo jeito |
| `indisponivel` | só aquela faixa vai para o próximo |
| `chave-de-audio`, `quebrado` | derruba o jeito nesta abertura |
| `sem-premium`, `sessao-recusada`, `sem-conta` | direto ao Match (o Soloist depende da mesma sessão) |

O `Remoto` guarda os jeitos que caíram **por jeito** (`caidos`: `HashSet<Jeito>` por serviço), então o Soloist falhar
não derruba o librespot e vice-versa.

### Protocolo (`crates/provedores`)

O erro ganhou **`causa`** opcional (retrocompatível). Um provedor antigo não manda; uma causa que o Player não conhece
vira erro comum, sem quebrar a resposta.

---

## Armadilhas que custaram tempo

1. **`cmd /C start` corta a URL no `&`.** Causou "response_type must be code". Use `rundll32`.
2. **O `librespot-playback` não compila no Windows** neste projeto: conflito de `links = "alsa"` com o `cpal` do
   tocador. Qualquer coisa que dependa dele (`Spirc`) está fora — a não ser que se resolva o conflito de ALSA.
3. **Os arquivos `ws.port`/`ws.addr` ficam para trás** quando o Soloist morre, apontando para uma porta morta. O Player
   tem que conferir o `soloist.pid` do arquivo contra o PID do processo que ele mesmo abriu (corrigido em
   `programa.rs`).
4. **Erro 4551 do Windows** (Controle Inteligente de Aplicativos) bloqueia exe/DLL recém-compilados. Solução:
   `cargo clean -p <pacote>` e compilar de novo. Aconteceu com `provedor-spotify`, `player`, `schemars`,
   `async-trait`, `rust-embed-impl`.
5. **O `TCCPLAYER_MUDO=1` chega ao Soloist** como `--initial-volume 0`. Bom para testar sem som.
6. **A pasta de dados do provedor tem o build e a conta em pastas separadas:** `programa/<build>/soloist.exe` e
   `conta/` (onde ficam `.device_id`, `soloist.pid`, `ws.port`, `ws.addr`). O `--data-dir` aponta para `conta`; o
   `--cache-dir` para `cache`.
7. **A chave do Soloist (`spak_…`) é pessoal e vai na linha de comando** — é o que o programa aceita. Fica cifrada em
   `soloist.dpapi` (DPAPI) e nunca vai para registro.

---

## Como testar o Soloist à mão (sem o Player)

Útil para separar "é o nosso código" de "é o programa".

```powershell
$p   = "$env:APPDATA\TccPlayer\provedores-dados\spotify\soloist"
$exe = "$p\programa\<build>\soloist.exe"
$chave = "<a chave spak_...>"

# 1) Tocar uma faixa e sair (o modo que FUNCIONA)
& $exe -n "Teste" -k $chave -D "$p\conta" --cache-dir "$p\cache" `
       -s "spotify:track:69kOkLUCkxIZYexIgSG8rq" -v

# 2) Abrir com a API local (para pausa/posição) — NÃO toca com -s junto
& $exe -n "Teste" -k $chave -D "$p\conta" --cache-dir "$p\cache" -w "127.0.0.1:0" -v
# a porta sai em "$p\conta\ws.port" e o endereço em "ws.addr"

# 3) Falar com um Soloist já aberto
& $exe ctl status
& $exe ctl now --json
& $exe ctl play            # continua o que está na conta
& $exe ctl pause
& $exe ctl seek 30000
```

No log (`-v` ou o `saida.log` do provedor), o que indica que **tocou** é:

```
AudioRendererImpl … started driver playback
Audio@… onPlaybackStarted
```

E o que indica queda:

```
Error: …                (ou o código 0xc0000005 no processo)
```

---

## Decisões e regras que continuam valendo

- **A regra 2 do `AGENTS.md`** foi anotada em 24/11/2026 com o aval do Rafael: o librespot **não quebra DRM** (pede a
  chave ao Spotify como cliente autenticado), e o áudio decifrado vive **só na memória** — nunca em arquivo, nunca
  redistribuído. O temporário do librespot (bytes ainda cifrados) é apagado a cada abertura.
- **O Player nunca espera o Spotify.** Tudo o que é lento roda na thread do provedor; a interface só lê o que está
  pronto.
- **Não fechar o Player do Rafael** para compilar ou testar. Usar `TCCPLAYER_DADOS` para uma cópia.
- **A chave do Soloist não aparece** na tela, no registro nem em mensagem de erro.

---

## O que ainda não foi conferido

- **O `play` com URI no build 1.3.10.** É o teste que decide a arquitetura (ver "As duas diferenças").
- Se o CDN aceita pedir uma versão específica do Soloist (e qual é o formato da URL).
- O `--single-track` tocando **pelo Player** (o teste manual funcionou; a integração não usa esse modo ainda).
- Uma conta **antiga** no librespot (que não sofra a recusa) — não temos uma para testar.
- O Ogg do Spotify decodificado pelo `symphonia`, a pré-análise pelo endereço local e a troca sem intervalo: nada disso
  foi exercitado com áudio real do Spotify.

---

## Arquivos tocados nesta investigação

```
app/crates/nucleo/src/erro.rs              CausaDoProvedor, Erro::ProvedorComCausa
app/crates/provedores/src/protocolo.rs     Causa (campo opcional no erro)
app/crates/provedores/src/cliente.rs       tradução da causa
app/player/src/cadeia.rs                   NOVO — a cadeia de reserva
app/player/src/remoto.rs                   jeitos que caíram, avaliar_falha, caiu_para
app/player/src/local.rs                    troca de jeito em vez de pular a faixa
app/provedores/spotify/src/oauth.rs        NOVO — OAuth com dois grants
app/provedores/spotify/src/audio/*         NOVO — librespot em memória + servidor local
app/provedores/spotify/src/spotify.rs      sessão, device_id, username
app/provedores/spotify/src/soloist/        programa.rs (pid), motor.rs (TOCOU_BEM, abrir_tocando)
docs/f6-spotify.md                         a história e a causa
AGENTS.md                                  regra 2 e seção 7
THIRD-PARTY-NOTICES.md                     librespot-audio e -metadata
```
