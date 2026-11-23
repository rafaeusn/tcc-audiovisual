# Plano v2: núcleo + provedores, código aberto

Feito em 15/11/2026 e **aprovado em 16/11/2026** (substitui a lista de fases antiga do `AGENTS.md`). Base: releitura dos 12
repositórios de antes, mais o Spotifast, o Spicetify, o núcleo do SimpMusic (`maxrave-dev/core`) e os dois novos
(Spotifry e MixMusic); as 16 telas do redesign do Spotify no Figma (lidas de novo pelo conector do Figma); e uma
busca na web sobre letras animadas, "Dynamic Island" no Windows e o estado das APIs. Nada foi executado dos
repositórios: só leitura.

Documentos ao lado: [relatorio-codigo-aberto.md](relatorio-codigo-aberto.md) (riscos e licenças),
[pesquisa-repos-streaming.md](pesquisa-repos-streaming.md) (a varredura de 11/11),
[repertorio-ui.md](repertorio-ui.md) (o redesign do Figma em números).

---

## 0. Resumo

1. **A ideia:** o Player vem com o que já temos (Modo Escuta, player local, visual, análise, temas). Os serviços
   (YouTube Music, Spotify, SoundCloud, Shazam, letras de outras fontes, sem perdas) chegam como **provedores na
   Loja**. Desde 21/11/2026, sem limitação: já vêm prontos, usam APIs oficiais e não oficiais e não pedem o Client
   ID do usuário (ver a seção 1).
2. **A referência mais forte é o MixMusic** (novo, MIT ou GPL-3, então copiável sem restrição). Ele já faz quase
   tudo o que queremos para Windows:
   - YouTube Music e Spotify num player só, com uma busca, uma fila e uma biblioteca;
   - o Spotify tocando dentro do app pelo player oficial numa janela WebView2 escondida, com o visual ouvindo o
     som pela mesma captura por processo que já temos;
   - letra palavra por palavra com bolinha saltitante, tradução e romanização;
   - salas de ouvir junto e controle remoto;
   - MCP para assistentes de IA e overlay para o OBS;
   - 4 layouts e 14 temas, Material dinâmico pela capa e loja de temas e plugins.
3. **Em Rust, o que dá para pegar quase pronto:**
   - do limusic (GPL-3): o cliente de YouTube Music (`crates/innertube`), o PO token, o karaokê com separação de
     voz, o servidor de sincronia e a romanização;
   - do Spotifast (MIT): o cliente do Spotify (librespot), o Chromecast, a lógica de abrir e fechar do notch, o
     MilkDrop e as skins do Winamp.
4. **Layouts:** uma casca só e as mesmas páginas para todos os provedores. O usuário escolhe o layout; o provedor
   só traz os dados e as prateleiras dele. Clonar o layout de cada serviço, como o Spotifast faz com o Spotify, não
   funciona quando a fila e a biblioteca misturam serviços. Um dos layouts nasce do redesign do Figma.
5. **A Ilha** (nossa Dynamic Island) é viável no GPUI: janela flutuante sem moldura, transparente e com
   desfoque, que fica sempre no topo e não rouba o foco. Ela tem estados compacto, alerta e expandido, e cresce e
   encolhe com molas.
6. **O diferencial que ninguém tem:** o **Modo Escuta preciso**. O Player reconhece a música que toca em
   qualquer app, acha a gravação num provedor, faz a pré-análise e alinha com o som ao vivo. Assim o visual
   acerta drops e batidas até no Spotify oficial ou num vídeo do navegador.
7. **13 fases** (seção 7). A espinha é F0 (abrir o código) → F1 (núcleo de provedores) → F2 (páginas unificadas) →
   F3 (YouTube Music). A Ilha anda em paralelo desde já.

---

## 1. Decisões e regras novas

**Decidido por você (15/11):**
- Projeto gratuito; o "Studio pago" sai do plano.
- Núcleo + provedores. O padrão é o que já existe; provedores ficam na Loja, conectados pelo usuário.
- Código aberto, se ajudar o projeto. Este plano assume **GPL-3.0-or-later** (motivo no relatório).

**Decisão de arquitetura (aprovada com o plano):**
- **Provedor = programa à parte**, que a Loja baixa do repositório dele e atualiza sozinho. Ele conversa com o
  Player por um protocolo simples (seção 6).
- Assim o YouTube pode quebrar e ser consertado sem release nova do Player, um provedor que trava não derruba o
  app, e o repositório principal fica limpo.
- Fontes leves (uma fonte de letra, metadados, uma rádio) ficam como plugins de script, como no MixMusic e no
  Orpheon.

**Regras do `AGENTS.md` que mudaram** (aplicadas em 16/11/2026):

| Hoje | Proposta |
| --- | --- |
| 1. Nunca baixar áudio do YouTube ou do Spotify | 1. **O núcleo não usa nenhum serviço de streaming sozinho.** Tudo o que vem de um serviço passa por um provedor da Loja, instalado e conectado pelo usuário, desligado por padrão e com aviso de responsabilidade. Cache e download offline só se o provedor oferecer e o usuário ligar, sempre cifrados no computador dele. O áudio nunca é enviado a terceiros: ouvir junto sincroniza, não transmite |
| 2. Spotify só pelo Modo Escuta | 2. **Spotify por provedor**, com o login do app do próprio usuário (Client ID dele). O Modo Escuta continua funcionando sem provedor nenhum |
| 3 a 9 | Iguais (segredos fora do git, sem letra no chat, commit só quando pedir, não fechar o seu Player...) |

**Regras 1 e 2 de novo, em 21/11/2026** (a orientadora liberou, para fins acadêmicos, o uso sem as limitações):

| 16/11 | 21/11 |
| --- | --- |
| Provedor desligado por padrão, com aviso de responsabilidade | **Sem limitação:** APIs oficiais e não oficiais, leitura de páginas, provedores prontos por padrão, sem aviso. Pré-análise, visual sincronizado e cache local do áudio valem para qualquer serviço |
| Spotify com o Client ID do próprio usuário | Spotify pelo jeito que funcionar melhor: player web, API interna, Connect ou librespot, sem pedir que o usuário crie um app |
| Cache e download offline só se o provedor oferecer | **Único limite:** nada de quebrar DRM nem de salvar em arquivo (ou redistribuir) faixas de catálogo. O app toca pelo serviço, com a conta do usuário |

---

## 2. O que temos hoje e o que acontece com cada coisa

| Hoje | Decisão | Como fica |
| --- | --- | --- |
| Modo Escuta (captura do PC ou de um app, SMTC) | **Manter e melhorar** | Ganha o modo preciso (F7) e o reconhecimento pelo som |
| Player local (biblioteca, fila, mistura, crossfade, EQ) | **Manter** | Vira o provedor "Neste computador" dentro da biblioteca única |
| Pré-análise + condutor (modo arquivo) | **Manter e ampliar** | Passa a valer para músicas de streaming. A análise fica guardada pelo ID da faixa e pelo ISRC, então a mesma música vinda de dois serviços usa a mesma análise |
| Visuais (Spectral, Fluxo, Barras, modo seguro) | **Manter e somar** | MilkDrop (milhares de presets), Canvas do Spotify, capa animada e clipe como camadas do palco |
| Letra (LRCLIB, por linha) | **Trocar** | Fontes em cadeia, palavra por palavra, tela cheia com estilos, tradução e romanização (F4) |
| Histórico e estatísticas | **Melhorar** | Retrospectiva do ano, cartões para compartilhar, fatia por serviço |
| Loja (manifesto TOML, ações com motivo, aceite, bloqueio) | **Melhorar** | Registro no GitHub, provedores, temas em arquivo, plugins (F10). O formato de ações com motivo continua |
| Temas no código (4 temas, 2 skins) | **Trocar** | Temas em `theme.json`, Material dinâmico pela capa, cores do papel de parede; as skins viram layouts |
| Animações (`anim.rs`, aproximação exponencial) | **Melhorar** | Somar molas com velocidade (para a Ilha e os arrastos), mantendo a interrupção |
| Last.fm e Discord | **Manter** | Last.fm guarda scrobbles offline e manda depois; Discord ganha botões |
| Janela mini, bandeja, botões da miniatura | **Manter** | A Ilha soma, não substitui |
| `servicos::capas` (capa do Deezer para o Discord) | **Trocar** | Capa do provedor quando houver; o Deezer fica de reserva |
| Studio pago | **Remover** | Fora do plano |
| AcoustID (plano antigo de reconhecimento) | **Trocar** | Shazam pelo algoritmo do SongRec |
| Audius e Jamendo (plano antigo) | **Tirar do núcleo** | Podem virar provedores da Loja |
| Skins KDL carregadas de arquivo (fase 1 antiga) | **Remover** | Vira `theme.json` + layouts |
| ListenBrainz | **Continua fora** | Pedido seu |
| Medidor de flashes do modo seguro | **Manter no plano** | Fica mais importante com Canvas, MilkDrop e clipes |

---

## 3. A varredura

### 3.1 Os repositórios

| Repositório | O que é | Linguagem | Licença | O que pegar agora |
| --- | --- | --- | --- | --- |
| **MixMusic** (novo) | YouTube Music + Spotify num player de desktop desenhado na GPU | Go | **MIT ou GPL-3** | Quase tudo: provedores, casador Spotify→YouTube, letras palavra por palavra, salas, remoto, MCP, overlay OBS, layouts e temas, Material dinâmico, cache offline, SponsorBlock, Chromecast, recap |
| **Spotifry** (novo) | YouTube (NewPipe) para Android e desktop | Kotlin | GPL-3 | Crossfade alinhado na batida, pular silêncio, cortar início e fim (com SponsorBlock), smart shuffle que espalha artistas, importar playlist pública do Spotify, sem perdas (Monochrome), backup em zip, EQ desenhável com espectro ao vivo atrás, AutoEq |
| **Spotifast** | Cliente de Spotify | **Rust** | **MIT** | librespot e Soloist, Spotify Connect, Chromecast (`cast.rs`), notch (`notch.rs`, lógica pura testada), MilkDrop (libprojectM), skins do Winamp e do Windows Media Player, "Feito para você" pela API interna, rádio |
| **limusic** | YouTube Music para desktop | **Rust** + Tauri | GPL-3 | `crates/innertube` (cliente do YouTube), PO token (`rustypipe-botguard`), cifra, `crates/karaoke` (separar voz com ONNX), `crates/listen-protocol` e `crates/sync-server` (ouvir junto), romanização (lindera, wana_kana, pinyin, any_ascii), 9 fontes de letra, editor de sincronia, vários logins |
| **núcleo do SimpMusic** (novo para nós) | Os módulos centrais do SimpMusic | Kotlin | GPL-3 | Canvas do Spotify, letras do Spotify, letras traduzidas e romanizadas, Tidal (busca e stream), Discord completo, ouvir junto (compatível com o Metrolist), passagem de login por QR, SponsorBlock, Return YouTube Dislike, AutoEq |
| SimpMusic | YouTube Music para Android e desktop | Kotlin | GPL-3 | Três estilos de "tocando agora" (um igual ao Apple Music), reconhecer música, escrever letra sincronizada, comentários do YouTube, Wrapped mensal, widgets |
| Prism | YouTube Music para Android e TV | Kotlin | GPL-3 | Sing (tira a voz), modo DJ (Camelot e energia), Looks por código, capas animadas (Apple Music, Tidal), reconhecer (Shazam), letras de 9 fontes, Replay e Rewind, Blend por QR, clipe com letra sincronizada, importar histórico, modo desempenho |
| Orpheon | Spotify para Android | Kotlin | GPL-3 | Extensões que trazem áudio e downloads, letra palavra por palavra, significados do Genius, playlists colaborativas, Spotify Codes, Jam, smart shuffle, AutoMix |
| Meld | Spotify + YouTube Music | Kotlin | GPL-3 | Login no Spotify sem app de desenvolvedor (API interna), motor de recomendação próprio, sem perdas pelo Qobuz casando por ISRC, troca manual do par |
| nuclear | Player de desktop movido a plugins | Tauri + React | AGPL-3 | Loja de plugins, várias fontes de stream por faixa (o usuário escolhe), API HTTP local, MCP, visualizador de log, "Novidades" |
| Metrolist | YouTube Music para Android | Kotlin | GPL-3 | Ouvir junto (servidor), letras, scrobble. O clone novo falhou no Windows (nomes de arquivo longos); usei a cópia de 11/11 |
| Outify | Spotify para Android com núcleo em Rust | Kotlin + Rust | GPL-3 | librespot no celular, Jam (iniciar e convidar) |
| spotify-player | Spotify no terminal | **Rust** | **MIT** | Configuração em arquivo, atalhos, ganchos de evento, comandos de terminal |
| Spicetify | Temas e extensões para o Spotify | JS | LGPL | Lixeira (nunca mais tocar), Shuffle+ (aleatório de verdade), repetir trecho A-B, marcadores, letra em janela flutuante, lyrics-plus (Genius, NetEase, tradução), WebNowPlaying (Rainmeter) |
| Simple-Music-Player, OpenTune-Web, lista | — | — | — | Nada novo |

### 3.2 Provedores e streaming

**YouTube Music**
- **Como todos fazem:** a API interna do YouTube (InnerTube), o PO token (BotGuard) e a cifra da assinatura, com o
  yt-dlp de reserva. Funciona sem conta. A conta, opcional, traz a biblioteca, as curtidas e o histórico.
- **Para nós:** o `innertube` e o `potoken` do limusic, que já são Rust. O MixMusic documenta formatos, links que
  expiram e o erro "não é um robô" (`docs/streams.md`), e instala o yt-dlp só quando o usuário pede.
- **Formato:** AAC 128 kbps (itag 140), que o nosso `symphonia` já decodifica. O Opus (itag 251) pede um
  decodificador a mais, o libopus.
- **Pré-análise:** o stream inteiro tem uns 3 a 5 MB; o analista baixa e analisa em 1 a 2 s, igual a um arquivo
  local.

**Spotify: quatro jeitos, como no MixMusic, mais o librespot**

| Jeito | Quem toca | Precisa | Visual | Pré-análise |
| --- | --- | --- | --- | --- |
| **Aqui, pelo player oficial**: o player web (ou o Web Playback SDK) numa janela WebView2 escondida, com Widevine | O Spotify, na qualidade dele | Premium (desde 21/11, sem pedir o Client ID: usa o token do player web) | Captura do processo do WebView2 (já temos) | Pela gravação "par" do YouTube Music, alinhada ao som (F7) |
| **Spotify Connect** | O app ou aparelho do usuário | Premium | Captura, se tocar neste PC | Igual à de cima |
| **Pelo YouTube Music** (o "Match" do MixMusic) | O nosso motor | Nada (sem Premium) | Completo | Completa |
| **librespot** (opcional, como no Spotifast) | O nosso motor | Premium | Completo | Completa |

Observações sobre o Spotify:
- **Limites:** o modo de desenvolvimento da API oficial aceita 5 usuários por app. Desde 21/11 não dependemos dele:
  o token do player web e a API interna fazem o mesmo, sem cada pessoa criar um app.
- **Risco:** o librespot quebrou para muita gente em agosto de 2026. Fica como opção, não como padrão.
- **Extras pela API interna, com o login do usuário:** o Canvas (os vídeos curtos em loop), as letras do Spotify
  (Musixmatch, por palavra), o "Feito para você" e a rádio.

**Casar uma faixa entre serviços** (do MixMusic, MIT):
- A nota vai de 0 a 1: 45% para o título limpo (sem "Remastered", "Radio Edit"...), 35% para o artista, 20% para
  a duração (até 2 s conta inteiro) e +0,05 se for música em vez de vídeo.
- Toca quando a nota passa de 0,6. O par fica guardado, e o usuário pode trocar o par à mão.
- Com ISRC, o casamento é exato (Meld).

**Outros provedores**

| Provedor | Como | Valor |
| --- | --- | --- |
| SoundCloud | O `client_id` público do site, como o yt-dlp faz, ou a API oficial com Artist Pro | Remixes, sets e hardstyle que só existem lá |
| Sem perdas (Tidal/Qobuz) | Servidores de terceiros, casando por ISRC (Meld, Spotifry, núcleo do SimpMusic) | FLAC; experimental, pode parar a qualquer dia |
| Rádios | Radio Browser (Orpheon, Prism) | Fácil e legal; o visual reage ao som de verdade |
| Podcasts | RSS e os podcasts do YouTube Music | Capítulos, velocidade, transcrição (MixMusic) |
| JioSaavn | Site público, 320 kbps (Spotifry) | Catálogo indiano; baixa prioridade |

### 3.3 Letras

**Fontes, numa ordem que o usuário muda e desliga** (MixMusic, Prism, limusic):
1. **Better Lyrics** (`api.betterlyrics.org`): TTML palavra por palavra.
2. **Unison** (`unison.betterlyrics.org`): base comunitária, muita coisa palavra por palavra.
3. **LRCLIB**: por linha, às vezes por palavra.
4. **Spotify/Musixmatch**: com o login do provedor Spotify, por palavra.
5. **NetEase, KuGou, QQ**: as melhores para música asiática.
6. **AMLL TTML DB**: dezenas de milhares de letras por sílaba, feitas à mão (conferir a licença dos dados).
7. **YouTube Music** (texto) e as **legendas do YouTube** (para versões ao vivo).
8. **Genius**: texto e os **significados** de cada trecho (Orpheon).

Regras que vale copiar:
- Sincronizada ganha de texto puro.
- Uma opção "preferir palavra por palavra" pergunta a mais fontes.
- O usuário ajusta o atraso de cada música.
- A letra fica guardada.

O que mostrar:

| Recurso | De onde | No nosso app |
| --- | --- | --- |
| Palavra por palavra: cada palavra se enche do acento do começo ao fim dela; vocais de fundo e duetos | MixMusic `lyricswords.go`, limusic, AMLL | No painel, na tela cheia, na Ilha e no visual |
| **Letra em tela cheia** com 6 estilos (Spotlight, Glow, Focus, Simple, Bold, Calm): as linhas longe desfocam e apagam, e a letra desliza até a linha cantada | MixMusic `lyricsstage*.go` | Um modo do palco, com o nosso visual atrás em vez da capa desfocada |
| **Bolinha saltitante** de palavra em palavra, com achatamento e um anel ao pousar | MixMusic `lyricsball*.go` | Opcional na tela cheia |
| Tradução embaixo de cada linha (47 línguas, guardada por música) | MixMusic, SimpMusic (com IA), Spicetify | Opcional |
| Romanização (romaji, pinyin, hangul e outras) feita no computador | limusic (lindera, wana_kana, pinyin, any_ascii), MixMusic | Opcional |
| Cartão para compartilhar trechos (com a fonte da letra, como as licenças pedem) | MixMusic, SimpMusic | No menu da letra |
| Editor para sincronizar uma letra e mandar à base comunitária | limusic, SimpMusic | Fase futura |
| **A letra mexendo no visual** | Ideia nossa | Partículas pulsam na sílaba cantada; o refrão muda a paleta |

Formatos: TTML, LRC e LRC com palavras, YRC, QRC e Lyricify Syllable. Para o TTML existe a crate `ttml_processor`, e
o MixMusic tem o leitor em Go (MIT) para portar.

### 3.4 Reconhecer pelo som

- **Todos usam o Shazam:** Prism, SimpMusic, Metrolist e Meld, pelo algoritmo do **SongRec** (Rust, GPL-3). Só a
  impressão digital é enviada, nunca o som.
- **Para nós:** 10 a 12 s da nossa captura viram título, artista, ISRC, capa e o ponto da música.
- **Usos:**
  - um botão "o que está tocando?" na Ilha;
  - Modo Escuta sem SMTC (vídeo no navegador, jogo, set de DJ);
  - etiquetar as músicas locais que aparecem como "Artista desconhecido";
  - a primeira etapa do modo preciso (F7).

### 3.5 Som e fila

| Recurso | De onde | Observação |
| --- | --- | --- |
| **Crossfade no compasso**: alinha as batidas das duas músicas | Spotifry, Orpheon (AutoMix) | Já temos batidas e tempo forte da pré-análise: é o passo natural |
| **Modo DJ**: ordena a fila por tom (Camelot) e energia | Prism | Temos tom e energia |
| Smart shuffle que espalha artistas; "toca todas antes de repetir" | Spotifry, Orpheon, Spicetify | Na nossa fila |
| Lixeira: nunca mais tocar a música ou o artista | Spicetify | Simples e muito pedido |
| Repetir trecho A-B; marcadores na música | Spicetify | Bom para estudar e dançar |
| Timer de dormir que abaixa o volume aos poucos | Spotifry, SimpMusic | Pequeno |
| Pular silêncio; cortar início e fim; SponsorBlock nos clipes | Spotifry, MixMusic | Para provedores de vídeo |
| Velocidade e tom separados | Spotifry, limusic | Já está meio caminho no `tocador` |
| **AutoEq** (correção por modelo de fone, dados MIT) e EQ desenhável com espectro ao vivo atrás | Spotifry, SimpMusic, Prism | Soma ao nosso EQ |
| Reverb e delay | SimpMusic, limusic | Opcional |
| Escolher a saída de som só do Player | MixMusic, SimpMusic | Pequeno |

### 3.6 Visual (o nosso foco)

| Recurso | De onde | No nosso app |
| --- | --- | --- |
| **MilkDrop** pelo libprojectM (LGPL): milhares de presets da comunidade | Spotifast (`milkdrop/`) | Mais um visual, recebendo o nosso diretor (energia, drops) para trocar de preset na hora certa |
| **Canvas do Spotify** e **capas animadas** (Apple Music, Tidal) | núcleo do SimpMusic, Prism | Uma camada do palco, misturada ao visual |
| **Clipe** da música, mudo, no lugar da capa | MixMusic, Prism | Camada opcional |
| **Descobrir em feed vertical** com prévias | Figma (tela Descobrir), MixMusic | Cada cartão toca 15 s com Canvas e o nosso visual |
| Barra de progresso ondulada | MixMusic | Combina com o nosso mapa de energia por seções |
| Skins clássicas do Winamp no mini player | Spotifast | Divertido; um item da Loja |

### 3.7 Juntos e controle

| Recurso | De onde | Como fica |
| --- | --- | --- |
| **Ouvir junto na rede local**: sala com código de 6 letras e convite por link; cada um toca da própria fonte | MixMusic, Prism | Primeiro passo |
| Ouvir junto pela internet | limusic (`sync-server`, Rust), Metrolist, SimpMusic | Depois. Precisa de um servidor (LGPD: guarda só apelido e IP, com aviso) |
| **Sala de visual**: todos veem as mesmas viradas | Ideia nossa | Encaixa por cima da sala |
| **Controle pelo celular**: página local com QR, sem app | nuclear (Jam), MixMusic (pareamento por código) | Trocar música, visual e tema pelo celular |
| Passar o que toca para outro computador | MixMusic, limusic | Junto do remoto |
| **MCP**: uma IA controla o player e o visual | nuclear, MixMusic | Como tudo já é `Comando`, a lista de ferramentas sai pronta |
| **Overlay para o OBS**: página local com o que toca | MixMusic (`docs/stream-overlay.md`) | E ainda dá para mostrar o nosso visual na live |
| Chromecast e Google TV | Spotifast (`cast.rs`, MIT), Prism | Fase futura |

### 3.8 Personalização

| Recurso | De onde | No nosso app |
| --- | --- | --- |
| **Layout + tema separados**: 4 layouts × 14 temas, um tema para o claro e outro para o escuro | MixMusic | Seção 4 |
| `theme.json` com tokens documentados, prévia e pasta de temas | MixMusic (`docs/themes.md`), nuclear | Substitui os temas no código |
| **Material dinâmico pela capa** (cores em HCT) | MixMusic (`internal/hct`), SimpMusic | Junta com as nossas cores reativas à energia |
| Cores do papel de parede (pywal, matugen, wallust) | limusic, MixMusic | Já estava no plano; agora com código pronto |
| "Look" compartilhado como código de texto | Prism | Os nossos pacotes |
| Editar a Início: arrastar prateleiras | MixMusic (`homeedit.go`) | Na Início unificada |
| Densidade, tamanho do texto e zoom | MixMusic | Ajuste simples |
| Ícone do app à escolha | limusic | Pequeno |
| Configuração em arquivo que vale na hora ao salvar | MixMusic (`settings.json`) | O nosso `ajustes.toml` ganha recarga ao vivo |

### 3.9 Biblioteca e dados

| Recurso | De onde |
| --- | --- |
| **Uma busca** em todos os provedores, com o selo do serviço em cada resultado | MixMusic |
| **Uma biblioteca**: "Neste computador" + cada serviço; curtidas juntas e devolvidas ao serviço | MixMusic |
| **Mixes**: playlists com faixas de vários serviços | MixMusic |
| Seguir artistas e uma prateleira de lançamentos (com contagem até o lançamento) | MixMusic, Spotifast |
| Importar playlist por link (Spotify público → casar) | Spotifry, Meld |
| Importar histórico (export do Spotify, Last.fm, Takeout) | Prism (já estava na fase 7) |
| Agrupar versões de um álbum e cópias da mesma música | MixMusic |
| Filtro de conteúdo explícito | MixMusic, Spicetify |
| Playlists colaborativas (pelo serviço) | MixMusic, Orpheon |
| **Retrospectiva do ano** + cartões quadrados ou de story | MixMusic, SimpMusic, Prism |
| Backup e restauração num zip | Spotifry |
| Significados do Genius e comentários do YouTube | Orpheon, SimpMusic |

### 3.10 Desktop

Atalhos globais configuráveis (MixMusic, limusic), atualização automática com assinatura (MixMusic, limusic),
relatório de diagnóstico sem segredos (limusic, Orpheon), links `tccplayer://` (MixMusic), aviso de faixa nova,
iniciar com o Windows, manter no topo e WebNowPlaying para o Rainmeter (Spicetify).

---

## 4. Layouts por provedor

### 4.1 As três saídas

| | A. Um layout nosso para tudo | B. O layout de cada serviço (como o Spotifast) | C. Casca única + layouts à escolha do usuário |
| --- | --- | --- | --- |
| Como é | O provedor só traz dados; as páginas são sempre as nossas | Com Spotify, telas iguais às do Spotify; com YouTube, iguais às do YouTube Music | As páginas são as mesmas para todos; o usuário escolhe o layout (o desenho delas); o provedor traz dados e prateleiras próprias |
| Fila e biblioteca misturadas | Funciona | **Quebra**: a página é de quem quando a playlist tem faixas dos dois? | Funciona |
| Trabalho | Menor | N layouts × 10 páginas, e cada serviço muda o próprio app | Médio: os layouts são variações das mesmas páginas |
| Personalização | Pouca | Nenhuma | **Muita**, que é o espírito do projeto (Spicetify, Hyprland) |
| Quem faz assim | nuclear | Spotifast, Orpheon | MixMusic |

**Recomendo a C.** O usuário escolhe o layout uma vez e ele vale para todos os serviços. O provedor colabora com:
- **prateleiras próprias** ("Feito para você" no Spotify, "Mixes" no YouTube Music, "Reposts" no SoundCloud);
- o **selo** do serviço em cada item;
- **capacidades declaradas**: a interface esconde o que o provedor não tem (o nuclear faz assim).

Quem quiser algo parecido com o Spotify escolhe o layout "Estante", que nasce do redesign do Figma, sem copiar a
marca.

**Layouts propostos:**
- **Palco** (o de hoje): o visual em primeiro lugar e o título gigante.
- **Estante**: o redesign do Figma, com a biblioteca na lateral, folhas pretas separadas por filete, trilho de
  informação e dock na cor da capa.
- **Expressivo**: Material 3 Expressive, com preenchimentos tonais e pílulas.
- **Compacto**: linhas densas, para quem tem tela pequena.

### 4.2 As páginas, a partir das 16 telas do Figma

| Página | Tela do Figma | O que muda com o nosso app |
| --- | --- | --- |
| **Início** | Início (com e sem painel de amigos) | Prateleiras de todos os provedores, editáveis; uma prateleira "para este momento" pela energia do que você ouve |
| **Busca** | Busca (recentes, explorar, gêneros) | Uma busca só, com o selo do serviço; gêneros com BPM e energia médios |
| **Biblioteca** | Biblioteca em grade com fixar | "Neste computador" + serviços + Mixes; silhueta por tipo (playlist, álbum, pasta, artista) |
| **Lista** (curtidas e playlists) | Músicas curtidas | Coração sempre à vista; ações no hover; barras na faixa que toca, movidas pela energia real |
| **Faixa** | Página da faixa (Letra, Créditos, Parecidas) | Abas **Letra**, **Mapa** (estrutura e energia da pré-análise), **Créditos**, **Parecidas** |
| **Álbum** | Álbum com trilho de informação | Chips de BPM, tom (Camelot) e energia ao lado dos gêneros |
| **Playlist / Mix** | Mix tingido pela cor da capa | O tom vem da capa ou do visual, com o teto de contraste que já definimos |
| **Artista** | Artista: Início, Sobre (com ranking), Álbuns em grade e em lista | Igual, com lançamentos e "mais tocadas por você" |
| **Podcast e episódio** | Programa e episódio (com vídeo) | Capítulos, velocidade e transcrição |
| **Descobrir** | Feed vertical | Prévias com Canvas + o nosso visual (3.6) |
| **Palco** | — (é nosso) | O visual, a letra em tela cheia e os módulos |

O arquivo do Figma também tem uma página Mobile e uma de Componentes. Os tokens e as medidas já estão no
`repertorio-ui.md` (seção 2.1).

---

## 5. A Ilha (nossa Dynamic Island)

### 5.1 Estados

| Estado | Tamanho (aprox.) | O que mostra | Quando |
| --- | --- | --- | --- |
| **Oculta** | — | Nada | Nada tocando há 30 s, outro app em tela cheia (jogo, vídeo, apresentação) ou desligada nos ajustes |
| **Repouso** | 200 × 34 | Capa de 22 px à esquerda; 5 barras do espectro real à direita (graves a agudos); um ponto que acende no tempo forte (apagado no modo seguro) | Tocando |
| **Alerta** | cresce para os lados por 2,5 s | Um de cada vez, por prioridade: troca de faixa (título rolando), **"drop em 3, 2, 1"** (pela pré-análise, com um pulso no drop), volume, música reconhecida, curtida ou scrobble feito, amigo entrou na sala, download pronto, atualização | Eventos |
| **Expandida** | 440 × 176 | Capa de 64, título, artista e o selo do serviço; o **mapa de energia por seções** como barra de progresso arrastável; anterior, tocar e próxima; curtir; a **linha da letra palavra por palavra**, com a próxima apagada; atalhos: reconhecer, letra em tela cheia, trocar visual, saída de som, timer de dormir, abrir o app | Ponteiro parado 350 ms em cima, clique ou atalho |
| **Mínima** | círculo ao lado | Com duas atividades (timer, sala, reconhecimento), a música vira um círculo separado, como na Apple | Duas atividades |
| **Arrastando** | — | Segue o ponteiro 1:1; ao soltar, projeta pelo impulso e encaixa no ponto mais perto (topo centro ou cantos) | Arrastar |

### 5.2 Movimento (pela skill de design da Apple)

- **Molas, não durações.** Uma mola para cada medida (largura, altura, raio do canto), com amortecimento 1,0 e
  resposta 0,35 s. Quique leve (0,8) só ao soltar um arrasto, porque aí houve impulso.
- **Interrompível de verdade.** Se o ponteiro sai no meio da abertura, a forma volta do tamanho atual com a
  velocidade atual, sem salto. Hoje o `anim.rs` persegue o alvo sem guardar velocidade; ganha molas com
  velocidade.
- **Ordem do conteúdo.** Ele entra depois que a forma passou de 60% e sai antes de ela encolher.
- **Mesmo caminho.** A ilha cresce a partir da pílula e volta para ela, ancorada no topo.
- **Vidro que se materializa.** No modo vidro, o desfoque e a escala entram juntos.
- **Movimento reduzido** (o "Efeitos de animação" do Windows desligado, ou o modo seguro): esmaecimento curto,
  sem mudar de tamanho e sem pulso.
- **Custo.** 60 fps só enquanto anima, 30 fps para as barras e parada quando pausa.
- **Resposta no aperto.** O botão reage ao pressionar, não ao soltar.

### 5.3 Como se faz no Windows

- **A janela.** É uma janela GPUI própria (`WindowKind::PopUp`, sem moldura, fundo `Transparent` ou `Blurred`):
  conferi no código do GPUI 0.2.2 que ela existe e já faz o desfoque acrílico. A subclasse do crate `integracao`
  soma três coisas:
  - `HWND_TOPMOST`, para ficar sempre no topo;
  - `WS_EX_NOACTIVATE`, para não roubar o foco;
  - `WM_NCHITTEST` respondendo `HTTRANSPARENT` fora da pílula, para o clique passar para o que está atrás.
- **O tamanho.** Enquanto anima, a janela fica no tamanho expandido e a forma é desenhada dentro; quando assenta
  compacta, a janela encolhe (como o MixMusic faz).
- **Tela cheia.** O `SHQueryUserNotificationState` diz quando outro app está em tela cheia, e a ilha se esconde.
- **Monitores.** A ilha segue o monitor do Player e se reposiciona quando a tela ou o DPI mudam.
- **Dados.** Ela lê o mesmo estado do Player (retrato, pré-análise, letra), sem thread nova.
- **Plugins.** Na F10, os plugins podem registrar atividades próprias.
- **Reuso.**
  - as regras de abrir e fechar com temporizadores do `notch.rs` do Spotifast (MIT, funções puras com testes);
  - o fechamento depois de 2 s com o ponteiro longe e a devolução do teclado, do `notch.go` do MixMusic (MIT);
  - ideias do DynamicWin, do WinIsland e do Yata Notch.

---

## 6. Arquitetura dos provedores

```
 Player (núcleo)                                   Provedor (programa à parte, instalado pela Loja)
 ───────────────                                   ────────────────────────────────────────────────
 Loja: instala, atualiza, liga e desliga  ──────►  provedor-youtube.exe (repositório próprio, assinado)
 "buscar: daft punk"               ── JSON ──────►  InnerTube
                                   ◄─────────────  faixas (id, serviço, ISRC, duração, capa)
 "tocar: id"                       ── JSON ──────►  resolve o stream
                                   ◄─────────────  1. endereço + cabeçalhos + formato  → o nosso motor baixa,
                                                      decodifica, mistura e pré-analisa
                                                   2. som decodificado por memória compartilhada (librespot)
                                                   3. "toco eu" + PID (WebView2 do Spotify) → captura por
                                                      processo para o visual
```

- **Capacidades** que um provedor declara: busca, início (prateleiras), biblioteca, playlists (ler e editar),
  curtir, artista, álbum, rádio, letra, áudio (endereço, som ou toca sozinho), conta, podcast, vídeo, Canvas,
  reconhecer.
- **Modelos comuns:** `Faixa { serviço, id, isrc, título, artistas, álbum, duração, capa }`, `Álbum`, `Artista`,
  `Playlist`, `Prateleira`.
- **Sessões e segredos:** cifrados com o DPAPI (o cofre que já existe). Nada passa por servidor nosso.
- **Cache de análise** pelo ID e pelo ISRC, em `analises.db`.
- **Por que programa à parte, e não código dentro do Player:**
  - o YouTube muda toda semana, e o conserto sai sem release nova do Player;
  - um provedor que trava não derruba o app;
  - cada provedor pode ser escrito na linguagem em que já existe código pronto (até um pedaço do MixMusic em Go,
    se valer a pena);
  - o repositório principal não carrega o código de nenhum serviço.
- **O custo:** um protocolo para manter e um processo a mais por provedor ligado.

---

## 7. Fases

Tamanho: **P** (alguns dias), **M** (uma a duas semanas), **G** (mais de duas semanas).

| # | Fase | O que entra | Reuso | Tamanho |
| --- | --- | --- | --- | --- |
| **F0** | **Base aberta** | Licença GPL-3, `LICENSE`, `THIRD-PARTY-NOTICES`, as regras novas, `cargo deny`, README público e guia de contribuição | — | P |
| **F1** | **Núcleo de provedores** | O protocolo, os modelos, a fila com vários serviços, o motor tocando streams (HTTP com range, AAC), o cache de análise por ID e ISRC, o casador entre serviços e "Neste computador" como provedor. A Loja instala provedores de uma pasta (o registro completo vem na F10) | MixMusic (casador, MIT), nuclear (capacidades) | G |
| **F2** | **Páginas unificadas e layouts** | Início, Busca, Biblioteca, Lista, Faixa, Álbum, Playlist, Artista e Podcast; layouts Palco e Estante; molas no `anim.rs` | Figma, MixMusic | G |
| **F3** | **Provedor YouTube Music** | Sem conta: busca, artista, álbum, playlist, rádio, explorar, tocar com pré-análise, legendas. Com conta (opcional): biblioteca, curtidas, histórico | limusic (`innertube`, `potoken`, cifra), MixMusic (`streams.md`) | M |
| **F4** | **Letras 2.0** | Fontes em cadeia, palavra por palavra, tela cheia com estilos e bolinha, tradução, romanização, cartão, ajuste de atraso, a letra no visual | MixMusic (`lyrics/`, MIT), limusic (romanização), `ttml_processor` | M — **feita em 22/11/2026** (fontes, palavra por palavra, atraso guardado, tradução, letras latinas, tela cheia); ficaram estilos e bolinha, cartão e a letra no visual ([f4-letras.md](f4-letras.md)) |
| **F5** | **A Ilha** (pode começar já, em paralelo) | Seção 5 | Spotifast (`notch.rs`), MixMusic (`notch.go`) | M |
| **F6** | **Provedor Spotify** | Login com o app do usuário (PKCE), biblioteca, busca e início; tocar aqui, por Connect ou pelo YouTube Music; Canvas e letras do Spotify; librespot como opção depois | MixMusic (Web Playback SDK, Match), Spotifast (librespot, Connect), núcleo do SimpMusic (Canvas) | G |
| **F7** | **Modo Escuta preciso + Shazam** | Reconhecer (SMTC ou SongRec); achar a gravação num provedor ou no computador; pré-analisar; alinhar pela curva de ataques com o som ao vivo (realinhando a cada 10 s e quando a posição pula); conduzir pelo modo arquivo; voltar ao ao vivo se a confiança cair; etiquetar arquivos locais | SongRec (GPL-3), nossa pré-análise | M |
| **F8** | **Som e fila profissional** | Crossfade no compasso, modo DJ, smart shuffle, lixeira, A-B, timer de dormir, pular silêncio, velocidade e tom, AutoEq e EQ desenhável | Spotifry, Orpheon, Prism, Spicetify | M |
| **F9** | **Juntos e controle** | Salas na rede local, controle pelo celular (QR), MCP, overlay OBS, passar para outro PC; salas pela internet e Chromecast depois | MixMusic, limusic (`sync-server`), nuclear, Spotifast (`cast.rs`) | M a G |
| **F10** | **Loja de verdade** | Registro no GitHub, assinatura, atualização dos provedores, temas em arquivo, Material dinâmico, cores do papel de parede, plugins de script com permissões, Looks | MixMusic (registro, SDK, HCT), Orpheon (permissões), limusic (Matugen) | G |
| **F11** | **Mais provedores** | SoundCloud, rádios, podcasts RSS, sem perdas (experimental), Deezer | yt-dlp (como referência), Orpheon (rádio), Meld (Qobuz) | M cada |
| **F12** | **Extras** | Retrospectiva e cartões, backup, atualização automática, atalhos globais, downloads offline cifrados, SponsorBlock, filtro explícito, MilkDrop, karaokê (separar voz), medidor de flashes, diagnóstico, skins do Winamp | MixMusic, Spotifry, Spotifast, limusic (`karaoke`) | P a M cada |

**Marcos:**
- Depois da **F3 + F4**, o Player toca o YouTube Music inteiro, de graça, com o visual preciso da pré-análise e a
  letra palavra por palavra. É o primeiro "uau" completo.
- Depois da **F6 + F7**, o Spotify entra pelos quatro jeitos, e até o app oficial tem visual preciso.

**O que anda em paralelo:** a F5 (Ilha) e partes da F8 não dependem de provedores. A F4 pode começar com o LRCLIB e
as fontes abertas antes da F3.

---

## 8. Riscos e cuidados

| Risco | Cuidado |
| --- | --- |
| YouTube e Spotify mudam e quebram os provedores | Provedor à parte, com atualização própria; plano B automático (o YouTube Music quando o Spotify falha); um aviso claro na Ilha e no painel |
| Limites do Spotify (5 usuários por app, Premium do dono) | Cada usuário cria o próprio app, com um guia passo a passo, como no MixMusic e no Spotifast |
| Widevine no WebView2 | O MixMusic diz que funciona no Windows; testar cedo na F6 |
| Conta do usuário | Aviso ao ligar o provedor; tocar por streaming, sem baixar à força |
| Pedido de remoção no GitHub | Provedores em repositório separado; o núcleo continua de pé |
| LGPD | Sessões cifradas, nada de telemetria; aviso de privacidade se houver servidor de salas |
| Desempenho | O WebView2 só sobe com o Spotify "aqui" ligado; provedores dormem quando não usados; a Ilha só desenha quando precisa |
| Limites do GPUI 0.2 | Desfoque por elemento não é garantido (a Ilha usa o da janela); o resto já foi provado no app |
| Escopo grande para uma pessoa | Seguir a ordem das fases e copiar antes de escrever do zero |

---

## Fontes

- MixMusic: [github.com/fzn0x/mixmusic](https://github.com/fzn0x/mixmusic) (docs: `spotify-playback.md`, `lyrics.md`,
  `look-and-feel.md`, `listen-with-friends.md`, `remote-control.md`, `stream-overlay.md`, `ai-assistants.md`,
  `streams.md`, `architecture/notch.md`).
- Spotifry: [github.com/Alnyz/spotifry](https://github.com/Alnyz/spotifry). Spotifast:
  [github.com/crmne/spotifast](https://github.com/crmne/spotifast). Spicetify:
  [github.com/spicetify/cli](https://github.com/spicetify/cli). Núcleo do SimpMusic:
  [github.com/maxrave-dev/core](https://github.com/maxrave-dev/core). Os demais, em
  [pesquisa-repos-streaming.md](pesquisa-repos-streaming.md).
- Letras: [Apple Music-like Lyrics](https://raw.githubusercontent.com/amll-dev/applemusic-like-lyrics/main/packages/core/README.md),
  [Better Lyrics](https://github.com/better-lyrics), [Better Lyrics API](https://gh.r-mj.com/better-lyrics/api),
  [ttml_processor](https://docs.rs/crate/ttml_processor/0.2.2),
  [Beautiful Lyrics](https://www.GitHub.com/surfbryce/beautiful-lyrics),
  [pipeline de letras do SimpMusic](https://simpmusic.org/docs/dev/lyrics-pipeline).
- Ilha: [DynamicWin](https://github.com/FlorianButz/DynamicWin),
  [WinIsland](https://alternativeto.net/software/winisland/about/),
  [Yata Notch](https://alternativeto.net/software/yata-notch/about/),
  [Windows Central sobre a Dynamic Island no Windows](https://www.windowscentral.com/software-apps/want-your-pc-to-be-a-bit-more-like-a-mac-the-dynamic-island-for-windows-just-got-a-big-update).
- APIs (2026): ver as fontes de [relatorio-codigo-aberto.md](relatorio-codigo-aberto.md).
