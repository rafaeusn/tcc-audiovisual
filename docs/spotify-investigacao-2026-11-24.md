# O Spotify tocando no Player: o que se sabe (24/11/2026)

Este arquivo junta a investigação do Rafael (branch `feat/spotify-audio-e-investigacao`, 24/11/2026 à tarde) com a conferência
feita em cima dela na noite do mesmo dia. A primeira parte é o que vale; o resto é o diário, para ninguém repetir os testes.

---

## A resposta: por que o Spotifast toca e o nosso Soloist não

**É a versão do Soloist.** Conferido no computador do Rafael em 24/11/2026:

| | Nosso Player | Spotifast |
| --- | --- | --- |
| Versão | 1.4.0.7 (compilado em 23/11/2026) | **1.3.10.51** (compilado em 06/11/2026, vale até 04/02/2027) |
| Assinatura | válida, Spotify AB | válida, Spotify AB |
| `soloist.exe` | 22.665.528 bytes | 22.796.600 bytes |
| O que acontece | cai (`0xc0000005`) ao começar a tocar | tocou das 19:54 às 21:20 daquele dia, na mesma conta, sem cair |

- **Por que o Spotifast tem a antiga:** ele baixou a dele em 07/11 e só procura outra nos últimos 30 dias da validade (90 dias
  depois de compilada). O nosso Player baixou a dele em 23/11, no dia em que o Spotify publicou a quebrada.
- **O que a [0.17.2 do Spotifast](https://github.com/crmne/spotifast/releases/tag/v0.17.2) ensina:** um build novo que fica caindo
  faz o Spotifast voltar para o anterior, que ele guarda até o novo se provar. O nosso apagava o anterior.
- **O servidor só tem a última versão.** Dez endereços com versão no nome (`1.3.10/…`, `builds/<número>/…`) responderam 404. Fixar o
  1.3.10 por endereço não dá; o que dá é usar o que já está no computador (o do Spotifast) ou o próximo que sair.
- **O Spotify consertou.** Em 24/11 saíram mais dois builds (15:31 e 21:02). O das 21:02, **1.4.0.12**, foi o que o Player baixou às
  22:34, e **tocou**: das 22:36 às 23:45, umas 30 músicas seguidas, nenhuma queda, com o visual ouvindo só o Soloist.
- A queda é do programa, e não do comando `play`: ela aconteceu também sem a API local, no modo `--single-track` e em todas as
  saídas de som (23/11), sempre dentro do mesmo trecho (`soloist.exe+0x211f55` a `+0x212387`). Ela não é certa a cada vez, e foi por isso
  que um teste com `--single-track` chegou a tocar 20 s.
- O mesmo defeito foi relatado por outra pessoa em [crmne/spotifast#1412](https://github.com/crmne/spotifast/issues/1412).

### O que mudou no Player por causa disso

1. **Volta ao anterior.** O Player guarda o build anterior; um que cai duas vezes seguidas (sem 15 s tocando entre elas), ou quatro
   vezes no mesmo dia, não é mais aberto. Só uma exceção do Windows conta como queda.
2. **Usa o Soloist do Spotifast.** Se o publicado só quebra, Configurações › Serviços oferece o outro que houver no computador; ele é
   copiado (nunca movido) quando a pessoa clica, e só se tiver a assinatura do Spotify.
3. **O 1.4.0.7 nem é aberto** (está na lista dos que quebram em todo Windows).
4. **Procura outro** uma vez por dia, ou na hora quando a pessoa liga (no máximo a cada 10 minutos), e só baixa se o pacote mudou.
5. **Todo `soloist.exe` tem a assinatura conferida** antes de abrir, o baixado também.

---

### O fim de cada música (medido em 24/11, ouvindo os eventos da API local e o nível do som do Soloist)

| Quando | O que acontece |
| --- | --- |
| 0,95 a 1,0 s antes do fim | O Soloist já decodificou tudo: passa a dizer `buffering` e a posição para. O som segue até o fim |
| 0,09 a 0,15 s depois do fim pela posição | O som acaba (é o atraso da placa) |
| cerca de 0,65 s depois do fim | O Spotify emenda outra música dele; ela não chega a soar |
| 1,2 a 1,6 s depois | A música seguinte da fila do Player começa |

O aviso de fim do Player (250 ms antes) só valia em `playing`, e por isso nunca disparava: ele esperava o Spotify emendar a outra.
Agora "carregando" perto do fim conta como o resto do som saindo, e a seguinte é pedida quando ele acaba. Medido de novo em 25/11 (4
trocas): do fim de uma até a outra começar, **de 0,70 a 0,87 s** (antes, de 1,29 a 1,50 s), e o Spotify não emenda mais nada. O que sobra
é o `play` (0,5 a 0,7 s para soar).

Com a próxima na fila do próprio Soloist (`add_to_queue`, a 10 s do fim), medido em 25/11 no Player do Rafael, quase mudo: em 4 trocas
(uma delas depois de pausar e continuar nos últimos segundos), a próxima começa **de 30 a 50 ms** depois do fim da outra, e a fila volta
a ficar vazia. Os eventos continuam iguais (`buffering` 0,95 s antes do fim). Cada item da fila diz de onde é: `source` `queue` (posto
por alguém) ou `context` (do Spotify: o `node_rules_placeholder` aparece por um instante e não conta). Pular nesses segundos com o
`skip_next` consome a adiantada, e ela começa na hora. A continuidade foi vista pelos eventos; o nível do som não deu para comparar,
porque as músicas do teste acabavam em silêncio.

Não era o crossfade: o do Player estava em 0 e é do misturador dele, por onde o som do Soloist não passa; e o fim de cada música tem
silêncio, e não duas soando juntas.

---

## Correções ao que estava na branch

- **"O `play` nunca leva a faixa" não é diferença.** A API local aceita `play` com a faixa nas duas versões (`soloist ctl --help`:
  `play [uri]`). O Spotifast não usa porque toca **listas inteiras** pela Web API, para manter a ordem ("preserve context, ordering").
  O nosso modelo (a fila é do Player, uma faixa por vez) continua.
- **Começar a tocar por 2 s não prova um build.** Com essa regra, o 1.4.0.7 foi marcado como bom às 19:42 de 24/11 e caiu às 19:51.
  Agora são 15 s tocando, somados entre as faixas, e isso só zera as quedas: não vale para sempre.
- **O `abrir_faixa` da branch não usa `--single-track`** (o comentário dizia que sim; o comando continuava com `--ws`).
- **O PKCE e o `state` do login eram sorteados a partir do relógio**, o que dá para adivinhar; agora vêm do sistema.
- **A espera do navegador não tinha prazo** (só era conferido quando chegava uma conexão), e qualquer programa do computador podia
  derrubar o login com um `error=` sem o `state`.
- **`rede` e `limite` diziam "tenta de novo", mas a faixa era pulada**, e `indisponivel` também. Agora a rede é tentada de novo (2, 5 e
  10 s) e, se não voltar, a fila para na música em vez de passar por todas.
- **Um jeito que caía não voltava** nem quando a pessoa ligava o Soloist de novo.

## O que veio da branch para o `main`

| O quê | Como entrou |
| --- | --- |
| `causa` no erro do protocolo | Igual, e agora o YouTube Music e o Spotify a mandam |
| Login próprio, sem o `librespot-oauth` | Refeito com a mesma ideia: sorteio do sistema, prazo, cancelar, `state` sempre conferido, sem o segundo login com Client ID |
| Id do aparelho estável | Entrou, e na sessão de verdade (na branch ele só valia na sessão que lia o nome da conta) |
| `soloist.pid` conferido | Entrou |
| A cadeia (`cadeia.rs`) | Só o que valia sem o librespot: tentar de novo, esperar ou pular |
| `audio/` (tocar pelo librespot) | **Ficou na branch.** Nesta conta o Spotify recusa toda chave, então cada abertura começaria com uma falha; e o agente (Claude) não escreve nem evolui a parte que decifra o áudio do Spotify |

---

# Diário da tarde (o que foi testado na branch)

## O que foi testado e o que deu

### O Soloist (provado a tocar)

| Teste | Comando | Resultado |
| --- | --- | --- |
| `--single-track` com a conta pareada | `soloist -n "Player (Soloist)" -k <chave> -D <conta> -s spotify:track:69kOkLUCkxIZYexIgSG8rq -v` | **TOCOU.** Log: `AudioRendererImpl started driver playback`, `onPlaybackStarted`, áudio por 20 s |
| WebSocket sozinho (sem comando) | `soloist -n … -k … -w 127.0.0.1:0 -v` | **Estável** por 15 s. entra na conta sozinho |
| WebSocket + `{"command":"play","uri":…}` | pelo nosso Player | **Caiu** com `0xc0000005` em 127 ms (`posicao_s: 0.127`) |
| `-s` **e** `-w` juntos | | Não toca (o modo de faixa não convive com a API) |
| `--single-track` com pasta nova (sem conta) | | Só fica `ready`, não toca |

**Conclusão da tarde:** o Soloist 1.4.0 abre, entra na conta e tocou com `--single-track`. (À noite: é sorte, e não o modo; ver o começo deste arquivo.)

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

A conta do Rafael é exatamente isso: um nome inventado pelo Spotify.

E o que **não** resolve, segundo o mesmo documento:

> A [Client ID of your own](/make-it-even-faster/#make-a-spotify-app) doesn't change it either: that only speeds up
> library and search requests, not audio keys.

> Changing audio settings, clearing the cache, signing out and in, or reinstalling won't fix it, because nothing on your
> computer is wrong.

Referência: [librespot#1649](https://github.com/librespot-org/librespot/issues/1649).

**O que resolve, segundo eles:** o Soloist ("*may* avoid the refused-key problem with the built-in player") ou tocar
noutro aparelho. É o caminho que estamos seguindo.

---

## O que foi implementado na branch (diário)

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

## Decisões e regras (diário da branch)

- **A regra 2 do `AGENTS.md`** foi anotada na branch em 24/11/2026 com o aval do Rafael (no `main` ela não mudou): o librespot **não quebra DRM** (pede a
  chave ao Spotify como cliente autenticado), e o áudio decifrado vive **só na memória** — nunca em arquivo, nunca
  redistribuído. O temporário do librespot (bytes ainda cifrados) é apagado a cada abertura.
- **O Player nunca espera o Spotify.** Tudo o que é lento roda na thread do provedor; a interface só lê o que está
  pronto.
- **Não fechar o Player do Rafael** para compilar ou testar. Usar `TCCPLAYER_DADOS` para uma cópia.
- **A chave do Soloist não aparece** na tela, no registro nem em mensagem de erro.

---

## O que não tinha sido conferido na branch (diário)

- **O `play` com URI no build 1.3.10.** É o teste que decide a arquitetura (ver "As duas diferenças").
- Se o CDN aceita pedir uma versão específica do Soloist (e qual é o formato da URL).
- O `--single-track` tocando **pelo Player** (o teste manual funcionou; a integração não usa esse modo ainda).
- Uma conta **antiga** no librespot (que não sofra a recusa) — não temos uma para testar.
- O Ogg do Spotify decodificado pelo `symphonia`, a pré-análise pelo endereço local e a troca sem intervalo: nada disso
  foi exercitado com áudio real do Spotify.

---

## Arquivos tocados na branch

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
