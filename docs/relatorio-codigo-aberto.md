# Abrir o código e soltar as amarras: o que ganhamos e o que custa

Relatório de 15/11/2026, a pedido do Rafael. Base: as duas pesquisas anteriores
([`pesquisa-referencias-app.md`](pesquisa-referencias-app.md), com o Spotifast, e
[`pesquisa-repos-streaming.md`](pesquisa-repos-streaming.md), com os 12 repositórios) e uma busca nova sobre o estado
das APIs em novembro de 2026 (fontes no fim). Nenhuma regra do `AGENTS.md` foi mudada: isto é material para decidir.

---

## 0. Resumo em uma página

O que hoje nos limita são **duas coisas diferentes**, que costumam ser tratadas como uma só:

1. **A licença (ser código fechado).** Quase todos os projetos que estudamos são GPL-3 ou AGPL-3. Enquanto o nosso
   código for fechado (por causa da ideia do Studio pago), não podemos copiar uma linha deles: só ler e reescrever.
   **Abrir o código como GPL-3 resolve isso por completo, sem risco nenhum.** Passamos a poder usar, direto, o
   karaokê em Rust do limusic, o servidor de "ouvir junto" dele, o reconhecimento do SongRec (Shazam), o cliente de
   YouTube RustyPipe, a loja de plugins do nuclear e tudo o mais. É o maior ganho com o menor custo.
2. **Os termos de uso dos serviços** (Spotify, YouTube, SoundCloud, Shazam). Isso não muda com a licença. Usar o áudio
   desses serviços por fora das APIs oficiais é o que todos os outros fazem, e funciona, mas tem três custos reais:
   **quebra com frequência** (o serviço muda e o recurso para de funcionar até alguém consertar), **risco para a conta
   do usuário** e **risco para o repositório** (pedido de remoção no GitHub). Não é burocracia: é manutenção e
   estabilidade.

**Recomendação:**

- **Abrir o código como GPL-3 agora.** Libera o código dos outros projetos, e o projeto continua gratuito.
- **Montar o Player como "núcleo + provedores"**, no modelo do nuclear: o núcleo (análise, visual, player, interface)
  fica limpo e é o que o TCC defende.
- **Fontes como Spotify (librespot), YouTube Music (RustyPipe), SoundCloud e o Shazam viram provedores opcionais,
  desligados por padrão.** O usuário liga se quiser, e cada um pode até morar num repositório separado.
- Assim temos o produto completo e unificado, e quando um serviço quebra ou pede remoção, só aquele provedor cai.

**O ganho técnico que mais importa para o foco do projeto (precisão e imersão):** hoje, a pré-análise que deixou o
hardstyle perfeito só vale para arquivos do computador. Com o áudio do Spotify, do YouTube Music ou do SoundCloud
**passando pelo nosso tocador**, toda música ganha a mesma precisão de drop, batida e trecho. Nenhum caminho
"correto" dá isso: a política do Spotify proíbe sincronizar visual com a API oficial, e o Modo Escuta só ouve ao vivo.

---

## 1. As duas amarras, separadas

### 1.1 Licença: o que muda ao abrir o código

| | Fechado (hoje) | GPL-3 |
| --- | --- | --- |
| Copiar código de limusic, Orpheon, Metrolist, Prism, SimpMusic, Outify, SongRec, RustyPipe (GPL-3) | Não. Só ler e reescrever | **Sim**, mantendo os créditos |
| Copiar do nuclear (AGPL-3) | Não | Sim. A AGPL só pesa para quem roda o programa como serviço na rede; num app de desktop é igual à GPL |
| Copiar do spotify-player, librespot (MIT) | Sim | Sim |
| As nossas dependências (GPUI Apache-2.0, wgpu MIT/Apache, symphonia MPL-2.0, rusqlite MIT...) | — | Todas compatíveis com GPL-3 (conferir com `cargo deny` antes de publicar) |
| Vender | Sim | Pode, mas qualquer pessoa pode redistribuir de graça. Na prática, a ideia do Studio pago acaba |
| Outros podem copiar o nosso código | Não | Sim, mas o que fizerem com ele também tem que ser GPL |
| Contribuições de fora, tradução pelo Weblate (grátis para projeto livre), visibilidade | Não | Sim |
| Para o TCC | Neutro | Bom: a banca pode ler o código, e reaproveitar com crédito é prática reconhecida. Só precisa deixar claro no texto o que é nosso e o que veio de fora |

O medo de "ter que abrir" faz sentido para quem vai vender. Como o projeto vai ser gratuito, abrir quase só traz
ganho. **O que se perde é só a exclusividade**: alguém pode fazer um fork. Isso acontece com todos eles, e o original
continua sendo a referência.

**Escolha de licença:** GPL-3.0-or-later. É a mesma da maioria dos projetos de onde vamos copiar, e deixa juntar com
AGPL-3 (nuclear). MIT ou Apache seriam mais "generosas", mas aí não poderíamos copiar o código GPL deles.

### 1.2 Termos de uso: o que cada caminho custa

Os riscos reais, sem burocracia:

| Risco | O que é na prática | Quão provável |
| --- | --- | --- |
| **Quebrar** | O serviço muda o protocolo e o recurso para até sair conserto. Exemplos de 2026: o librespot parou de tocar para muita gente em agosto; o YouTube passou a exigir *PO token* e o yt-dlp vive correndo atrás | **Certo**, várias vezes por ano |
| **Conta do usuário** | O serviço suspende contas que usam clientes não oficiais. É raro com o Spotify (a ação de 2026 mirou o "Premium pirata" e a API pública), mais comum com contas que baixam muito | Baixo a médio |
| **Remoção do repositório** | Pedido de remoção (DMCA) no GitHub. Precedente: o youtube-dl foi tirado do ar em 2020 e voltou depois. Afeta mais quem **baixa** áudio do que quem só toca | Baixo para tocar; médio para baixar |
| **Lei brasileira** | A Lei 9.610/98 (art. 107) proíbe contornar proteção técnica contra cópia. Tocar por streaming dentro do app é zona cinzenta; **baixar e guardar o áudio** é o que mais se aproxima da proibição | Quase nunca aplicado contra usuário ou projeto pequeno, mas é o ponto que um avaliador pode levantar |
| **LGPD** | Guardar tokens e cookies das contas do usuário (só no computador dele, cifrados com DPAPI, como o Last.fm já faz), não mandar nada a servidor nosso, e um aviso de privacidade se houver servidor de sala | Fácil de cumprir |

---

## 2. Serviço por serviço

Cada linha: o caminho "correto" (oficial) e o caminho "igual aos outros". O esforço está em tamanho relativo para nós:
pequeno (P), médio (M), grande (G).

### 2.1 Spotify

| | Correto | Igual aos outros |
| --- | --- | --- |
| Como | Modo Escuta (o que temos) + Web API para dados | **librespot** (crate MIT em Rust, o mesmo motor do Spotifast e do spotify-player): o usuário entra com a conta Premium dele pelo navegador, e o áudio chega como som decodificado direto no nosso tocador |
| Situação em 2026 | A Web API ficou muito restrita: em fevereiro, o modo de desenvolvimento passou a pedir Premium e a ter no máximo 5 usuários, e 15 endpoints saíram (top do artista, lançamentos, categorias). O acesso amplo é só para empresas. E a política proíbe sincronizar visual com o conteúdo | Funciona com Premium; quebrou em agosto de 2026 para muitos e é consertado de tempos em tempos pela comunidade |
| O que dá | Tocando agora, controles, visual ao vivo | **Tudo dentro do nosso app**: biblioteca, playlists, busca, fila misturada com músicas locais, e **pré-análise de toda música do Spotify** (drop exato, batidas reais, compasso). Crossfade e equalizador nossos. Sem a janela do Spotify aberta |
| Extras pela API interna (com o login do usuário) | — | Letras sincronizadas do próprio Spotify, **Canvas** (os vídeos curtos em loop, que podem entrar no visual), playlists por link, recomendações |
| Esforço | — | M para tocar e navegar; G para a interface completa de biblioteca (o Spotifast tem 176 mil linhas, mas boa parte é o que já temos) |
| Risco | Nenhum | Quebrar: alto. Conta: baixo. Remoção: baixo (o librespot existe desde 2016) |

**Sobre "entrar no Spotify dentro do nosso app":** é exatamente o librespot. Uma alternativa é abrir o Web Player
oficial escondido numa janela WebView (o que o Orpheon faz no Android). Ele quebra menos, mas a reprodução protegida
(Widevine) dentro do WebView2 precisaria de teste, e o áudio continua saindo pelo navegador (o visual voltaria a
depender da captura).

### 2.2 YouTube Music

| | Correto | Igual aos outros |
| --- | --- | --- |
| Como | Player oficial embutido (IFrame), visível, sem separar áudio e vídeo | **RustyPipe** (Rust, GPL-3, inspirado no NewPipe): busca, playlists, biblioteca da conta, letras e o link do áudio. Ou o yt-dlp como programa à parte |
| Situação em 2026 | Funciona, mas é um vídeo numa janela | A exigência de *PO token* quebra os clientes de tempos em tempos; cada atualização pede conserto |
| O que dá | Pouco para nós | O catálogo inteiro (inclusive remixes e versões que não estão no Spotify), **grátis e sem conta**, com pré-análise e tudo dentro do app. É o motor do limusic, do Metrolist, do SimpMusic e do Prism |
| Esforço | — | M (com o RustyPipe) |
| Risco | Nenhum | Quebrar: alto. Conta: baixo (dá para usar sem conta). Remoção: baixo para tocar, médio se baixar |

O truque mais usado pelos outros (Meld) é a **biblioteca do Spotify com o áudio do YouTube**: lê as playlists do
usuário no Spotify e acha cada faixa no YouTube Music, casando por ISRC, título e duração. Dá o Spotify "inteiro" sem
Premium.

### 2.3 SoundCloud

| | Correto | Igual aos outros |
| --- | --- | --- |
| Como | API oficial: o cadastro de apps reabriu em maio de 2026, mas só para assinantes **Artist Pro** (pago) | O `client_id` público do site, como o yt-dlp e o scdl fazem |
| O que dá | Busca, faixas, playlists, stream das faixas liberadas | O mesmo, sem a assinatura. Bom para remixes, sets de DJ e hardstyle que só existem lá |
| Esforço | P a M | P a M |
| Risco | Nenhum (mas pago) | Quebrar: médio. Conta: nenhum (não precisa de conta). Remoção: baixo |

### 2.4 Shazam (reconhecer pelo som)

| | Correto | Igual aos outros |
| --- | --- | --- |
| Como | Chromaprint + AcoustID (aberto; precisa de uma chave grátis) e, sem internet, impressão digital da nossa biblioteca | **SongRec** (Rust, GPL-3): gera a impressão digital do Shazam e consulta o endereço deles. É o que Metrolist, Prism e SimpMusic usam |
| O que dá | AcoustID: bom para músicas conhecidas e etiquetas; fraco para lançamentos e remixes | **O catálogo do Shazam**: reconhece quase tudo, inclusive vídeo no navegador, jogo ou live. Devolve título, artista, capa, ISRC e o ponto da música |
| Esforço | M | **P** (com o código do SongRec já em Rust) |
| Risco | Nenhum | Quebrar: baixo (funciona há anos). Conta e remoção: quase nenhum. Só manda alguns segundos de impressão digital, nunca áudio |

O "ponto da música" é o que liga o reconhecimento à precisão: se a música reconhecida existir em algum provedor (ou
no computador), o Modo Escuta passa a usar a pré-análise completa dela, alinhada no tempo certo.

### 2.5 Letras

| | Correto | Igual aos outros |
| --- | --- | --- |
| Fonte | LRCLIB (o que temos): sincronizada por linha, catálogo comunitário | Musixmatch (pela API interna do Spotify), Apple Music (**sincronizada por palavra/sílaba**), NetEase, QQ, Genius |
| O que dá a mais | — | Bem mais músicas com letra, e a sincronia **por palavra**: o visual pode reagir a cada sílaba cantada, e dá para fazer o karaokê de verdade |
| Esforço | — | P por fonte |
| Risco | Nenhum | Quebrar: médio. O resto é baixo |

### 2.6 Outros serviços

| Serviço | Correto | Igual aos outros | Comentário |
| --- | --- | --- | --- |
| Deezer | API pública: metadados, capas (já usamos no Discord) e prévias de 30 s | Faixas completas pelo cookie da conta (o "ARL", estilo deemix) | O deemix foi alvo de remoções. Para tocar, o YouTube Music cobre melhor |
| Tidal / Qobuz | Não há API aberta para tocar | Clientes não oficiais; o Meld usa servidores de terceiros para o Qobuz | Sem perdas, mas de baixo valor para o visual. Fica para depois |
| Apple Music | MusicKit pede conta de desenvolvedor paga | Token tirado do web player | Só vale pelas letras por palavra |
| Rádios (Radio Browser), Audius, Jamendo | Legais e grátis | — | Já estavam no plano; continuam valendo |

---

## 3. O que reaproveitar de cada repositório, com o código aberto

Com GPL-3, o "ler para entender, escrever o nosso" vira **"copiar, adaptar e dar crédito"** em tudo abaixo.

| De onde | O quê | Ganho para nós | Esforço com cópia |
| --- | --- | --- | --- |
| **limusic** (Rust, GPL-3) | `crates/karaoke`: separação de voz e instrumental com ONNX e DirectML, em blocos, trocando para o instrumental antes de terminar | A fase 10 (stems) quase pronta: visual reagindo à voz separada da base, e o karaokê | M (antes: G) |
| limusic | `crates/listen-protocol` + `crates/sync-server`: ouvir junto, em Rust | A sala de visual/escuta | M (antes: G) |
| limusic | `diagnostics.rs`, `appicon.rs`, `hotkeys.rs`, atualização automática, paleta do papel de parede (Matugen) | Acabamento | P cada |
| **SongRec** (Rust, GPL-3) | Impressão digital do Shazam | Reconhecer qualquer música pelo som | P |
| **RustyPipe** (Rust, GPL-3) | Cliente completo de YouTube/YouTube Music | Provedor YouTube Music | M |
| **librespot** (Rust, MIT) | Cliente do Spotify | Provedor Spotify | M |
| spotify-player (Rust, MIT) | Interface de biblioteca, busca e playlists sobre o librespot, atalhos e config | Base do provedor Spotify | M |
| **nuclear** (AGPL-3) | Sistema de plugins e provedores com capacidades declaradas, loja de temas, API HTTP local, servidor MCP, controle pelo celular por QR | A arquitetura "núcleo + provedores" e as fases 5 e 9 | M cada |
| **Orpheon** (GPL-3) | AutoMix (Beat This!, grade de batidas, transição no compasso), extensões com permissões, rádio | Mixagem automática estilo DJ entre faixas de qualquer fonte | M |
| **Meld** (GPL-3) | `SpotifyYouTubeMapper`: casar uma faixa do Spotify com o YouTube | A biblioteca do Spotify com áudio do YouTube | P |
| **Prism** (GPL-3) | Modo DJ (Camelot, energia, fila por tom), Looks como código, importação (Spotify, Last.fm, Takeout), equalizador paramétrico, áudio espacial, passeio guiado | Recursos de "app premium" | P a M cada |
| **Metrolist / SimpMusic** (GPL-3) | Várias fontes de letras, Wrapped e retrospectiva, AutoEq, sala compatível entre eles | Letras e estatísticas | P a M |

---

## 4. A grande comparação

### 4.1 Os dois caminhos lado a lado

| | **Caminho correto** (oficial, como estamos) | **Caminho igual aos outros** (código aberto + provedores não oficiais) |
| --- | --- | --- |
| Fontes de música dentro do app | Arquivos locais, rádios, Audius, Jamendo, SoundCloud (com Artist Pro) | Isso **mais** Spotify (Premium), YouTube Music (grátis), SoundCloud (grátis), Deezer |
| Spotify | Só ouvido de fora (Modo Escuta), com o app do Spotify aberto | Dentro do app, com a nossa interface, fila misturada e as nossas funções |
| **Precisão do visual** | Perfeita só em arquivos locais; o resto é ao vivo (bom, mas sem saber o futuro) | **Perfeita em quase tudo** que o usuário ouvir, porque todo áudio passa pela pré-análise |
| Reconhecer música | AcoustID (catálogo menor) e a nossa biblioteca | Catálogo do Shazam |
| Letras | LRCLIB, por linha | Várias fontes, **por palavra** |
| Separar voz, ouvir junto, loja | Escrito do zero (G cada) | Copiado e adaptado (M cada) |
| Velocidade de desenvolvimento | Mais lenta: tudo reescrito | **Muito mais rápida**: semanas viram dias em várias áreas |
| Manutenção | Baixa: as APIs oficiais quase não mudam | **Alta**: provedores quebram várias vezes por ano e pedem conserto |
| Estabilidade para o usuário | Alta | O núcleo é estável; os provedores, não |
| Risco para contas e repositório | Nenhum | Baixo a médio, concentrado nos provedores |
| Para a banca do TCC | Fácil de defender | O núcleo é fácil; os provedores pedem uma explicação (por que existem, que são opcionais, quem é responsável) |
| Comparado aos outros projetos | Menos fontes que todos, porém o único com o visual e a análise | **Tudo que eles têm, mais o que ninguém tem** (o visual conduzido pela análise) |

### 4.2 O que perdemos se ficarmos só no correto

- **A precisão fora dos arquivos locais.** É o ponto mais sério, porque é o foco do projeto. A maioria das pessoas ouve
  Spotify ou YouTube, não MP3. Para elas, o visual seria sempre o "ao vivo".
- **O app como player principal.** Sem fontes de streaming dentro dele, a pessoa sempre precisa de outro programa
  aberto, e o nosso vira "o visual do lado".
- **Reconhecimento bom** (Shazam), **letras por palavra** e o **Canvas** como material visual.
- **Velocidade**: reescrever do zero o que limusic, nuclear e SongRec já resolveram custa a maior parte do tempo que
  sobra até o TCC.

### 4.3 O que perdemos se formos pelo caminho dos outros

- **Previsibilidade**: algum provedor estará quebrado em algum momento, e o conserto depende de nós ou da comunidade
  (librespot, RustyPipe).
- **A ideia do Studio pago** (já abandonada).
- **Um pouco de tranquilidade na defesa**: alguém da banca pode perguntar sobre os termos de uso. A resposta boa é a
  arquitetura (abaixo).
- **Distribuição em lojas oficiais** (Microsoft Store) com esses provedores dentro. Pelo GitHub e pelo winget, segue
  normal.

---

## 5. A proposta: núcleo limpo + provedores

O desenho que junta os dois mundos, o mesmo do nuclear e do Spicetify:

```
┌────────────────────────── Núcleo (o TCC) ──────────────────────────┐
│ tocador · pré-análise · condutor · visual · fila · histórico ·     │
│ letras · Modo Escuta · interface · loja                            │
│                                                                    │
│   trait Provedor: busca · biblioteca · playlists · áudio (PCM) ·  │
│                   letras · capa · reconhecer                       │
└───────────────┬────────────┬───────────┬───────────┬───────────────┘
                │            │           │           │
          Arquivos      Rádios/       Spotify    YouTube Music
          locais        Audius/      (librespot) (RustyPipe)
          (padrão)      Jamendo                  SoundCloud
                        (padrão)     opcionais, desligados por padrão,
                                     podem morar em outro repositório
```

- **Cada provedor declara o que oferece** (como já fazem os módulos da Loja), e a interface esconde o que o provedor
  ativo não tem.
- **Todo áudio entra no mesmo tocador**, então crossfade, equalizador, volume igualado e **pré-análise** valem para
  qualquer fonte. A análise fica guardada pelo ISRC: a mesma música vinda do Spotify ou do YouTube usa a mesma análise.
- **Fila misturada**: uma música local, depois uma do Spotify, depois um remix do SoundCloud, com crossfade no compasso
  (o AutoMix do Orpheon).
- **Os provedores não oficiais ficam desligados**, com um aviso curto ao ligar ("usa um cliente não oficial; pode
  parar de funcionar; sua conta é sua responsabilidade"). Se um serviço pedir remoção, sai aquele provedor; o
  Player continua de pé.
- **Tocar, não baixar**: os provedores tocam por streaming e guardam só a análise, nunca o arquivo de áudio. É o que
  mantém o risco jurídico baixo e o que todos esses projetos fazem.

### Ordem sugerida, se a decisão for abrir

| # | O quê | Por quê | Tamanho |
| --- | --- | --- | --- |
| 1 | Licença GPL-3, arquivo `LICENSE`, créditos, `cargo deny` | Libera todo o resto | P |
| 2 | `trait Provedor` e os arquivos locais passando por ele | A base da unificação | M |
| 3 | Reconhecer pelo som com o SongRec + a impressão digital da nossa biblioteca | Pequeno e liga direto à precisão do Modo Escuta | P |
| 4 | Provedor Spotify (librespot): entrar com a conta, biblioteca, tocar com pré-análise | O pedido central; o maior ganho de imersão | M a G |
| 5 | Provedor YouTube Music (RustyPipe) + casar a biblioteca do Spotify com o YouTube | Tudo grátis, sem Premium | M |
| 6 | Separar voz (o karaokê do limusic) + letras por palavra | O diferencial visual que ninguém tem | M |
| 7 | SoundCloud, ouvir junto (sync-server do limusic), AutoMix | Completa o "premium unificado" | M cada |

---

## 6. O que fica igual em qualquer caminho

- **LGPD**: tudo no computador do usuário; tokens cifrados (DPAPI); nada de telemetria; aviso de privacidade se
  houver servidor de sala.
- **Nenhum segredo no git** (as chaves continuam em variáveis de ambiente).
- **Nada de reproduzir letras** em documentos ou no chat (exibir no app é o que o módulo Letra já faz).
- Se a decisão for abrir, as regras 1 e 2 do `AGENTS.md` (não baixar do YouTube/Spotify, Spotify só pelo Modo
  Escuta) precisam ser reescritas, por exemplo: "provedores não oficiais só tocam por streaming, ficam desligados por
  padrão e nunca guardam o áudio".

---

## Fontes

- Spotify: [Update on Developer Access and Platform Security (06/02/2026)](https://developer.spotify.com/blog/2026-02-06-update-on-developer-access-and-platform-security),
  [Guia de migração de fevereiro de 2026](https://developer.spotify.com/documentation/web-api/tutorials/february-2026-migration-guide),
  [Quotas do modo de desenvolvimento (23/07/2026)](https://developer.spotify.com/blog/2026-07-23-web-api-quota-updates),
  [Changelog de julho de 2026](https://developer.spotify.com/documentation/web-api/references/changes/july-2026),
  [Headphonesty: a ação contra apps de terceiros](https://www.headphonesty.com/2026/02/spotify-crackdown-thousands-third-party-music-apps/),
  [Techzine](https://www.techzine.eu/news/devops/138608/spotify-puts-the-brakes-on-developer-mode-with-stricter-api-rules),
  [Music Business Worldwide: o bloqueio do Premium pirata](https://www.musicbusinessworldwide.com/no-spotify-hasnt-gone-down-unless-you-were-pirating-the-premium-service).
- librespot em 2026: [Mopidy, quebra em agosto](https://discourse.mopidy.com/t/mopidy-spotify-not-working-gstreamer-no-resource-found/6324),
  [Music Assistant #4392](https://github.com/music-assistant/support/issues/4392),
  [Arch Wiki: Spotify](https://wiki.archlinux.org/title/Spotify).
- SoundCloud: [Vibe Coding with SoundCloud (cadastro para Artist Pro)](https://developers.soundcloud.com/blog/vibe-coding-ai-agent-docs-self-serve-api-keys),
  [API Credentials from Your Terminal](https://developers.soundcloud.com/blog/api-credentials-cli-openapi-github).
- YouTube: [PiunikaWeb: checagem de token](https://piunikaweb.com/2025/09/25/youtube-token-checks-third-party-issues/),
  [yt-dlp #17395](https://github.com/yt-dlp/yt-dlp/issues/17395),
  [Integração do bgutil PO token (05/2026)](https://www.nicolas-riquier.com.amigaland.com/notes/ExpSoft_SaveVideos/2026-05-17-bgutil-po-token-integration.html),
  [RustyPipe no docs.rs](https://docs.rs/rustypipe), [repositório do RustyPipe](https://code.thetadev.de/ThetaDev/rustypipe).
- Shazam: [SongRec no Arch (GPL-3.0)](https://archlinux.org/packages/extra/x86_64/songrec),
  [songrec-lib](https://docs.rs/crate/songrec-lib/0.5.3).
- Os 12 repositórios e o Spotifast: links em [`pesquisa-repos-streaming.md`](pesquisa-repos-streaming.md) e
  [`pesquisa-referencias-app.md`](pesquisa-referencias-app.md).
