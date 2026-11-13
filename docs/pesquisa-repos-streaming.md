# Varredura de 12 repositórios de streaming de música

Feita em 11/11/2026. Complementa [`pesquisa-referencias-app.md`](pesquisa-referencias-app.md) (Spotifast, Spicetify,
Hyprland). Base: leitura dos READMEs, da documentação e dos cabeçalhos e trechos do código de cada repositório, em
cópia rasa. Nada foi compilado nem executado. Dois limites: o núcleo do SimpMusic fica em outro repositório
(`maxrave-dev/core`) e o `OpenTune-Web` é só o site do OpenTune (o app está no Codeberg); esses dois não foram lidos.

---

## 0. Resumo

1. **Quase tudo que esses apps têm de "streaming" não serve para nós**: 9 dos 12 tocam áudio do YouTube ou do Spotify
   por API privada, o que as nossas regras e os termos dos serviços proíbem. O valor está no que fica **em volta** do
   áudio: letras, scrobble, Discord, estatísticas, loja de extensões, temas, análise de batida e integração com o
   Windows.
2. **Licença**: 10 dos 12 são GPL-3 ou AGPL-3. Copiar código deles obriga o nosso app (e o Studio pago, que usa o mesmo
   motor) a ser GPL também. A saída é **reimplementar a partir do comportamento e dos protocolos públicos**. Só o
   `spotify-player` (MIT) pode ser copiado com crédito.
3. **As três melhores referências para o nosso caso**: `limusic` (é Rust para desktop, com as mesmas integrações que
   queremos), `Orpheon` (extensões com permissões e análise de batida para mixagem) e `nuclear` (loja de plugins e de
   temas, API local, servidor MCP).
4. **O que dá para ter já, dentro do Modo Escuta**: letras sincronizadas (LRCLIB), histórico e estatísticas locais,
   scrobble no Last.fm e no ListenBrainz, cartão no Discord, botões na miniatura da barra de tarefas, bandeja e mini
   player.
5. **O que muda a nossa análise**: o modelo aberto *Beat This!* (batida e primeiro tempo do compasso), detecção de tom
   e a roda de Camelot. Estão no AutoMix do Orpheon e no modo DJ do Prism.

---

## 1. Os repositórios

| Repositório | O que é | Tecnologia | Licença | Estado | Valor para nós |
| --- | --- | --- | --- | --- | --- |
| [limusic](https://github.com/SimoHypers/limusic) | Cliente de YouTube Music para desktop | Rust + Tauri + Svelte, libmpv | GPL-3 | Ativo | **Alto**: integrações de desktop em Rust |
| [Orpheon](https://github.com/kota-zo/Orpheon) | Cliente de Spotify para Android (dirige o Web Player oficial escondido) | Kotlin | GPL-3 | Ativo | **Alto**: extensões, AutoMix, rádio |
| [nuclear](https://github.com/nukeop/nuclear) | Player de desktop movido a plugins | Tauri (Rust) + React | AGPL-3 | Ativo | **Alto**: loja de plugins e temas, API local, MCP |
| [Prism](https://github.com/Yigder/Prism) | Cliente de YouTube Music para Android e TV | Kotlin | GPL-3 | Ativo | **Alto** em ideias: modo DJ, Looks, Replay, importação |
| [spotify-player](https://github.com/aome510/spotify-player) | Cliente de Spotify no terminal | Rust, librespot | **MIT** | Ativo | Médio: configuração, temas, CLI; único copiável |
| [SimpMusic](https://github.com/maxrave-dev/SimpMusic) | Cliente de YouTube Music, Android e desktop | Kotlin Multiplatform | GPL-3 | Ativo | Médio: Wrapped, estatísticas, AutoEq |
| [Metrolist](https://github.com/MetrolistGroup/Metrolist) | Cliente de YouTube Music para Android | Kotlin | GPL-3 | Só manutenção | Médio: letras, scrobble, sala de escuta |
| [Meld](https://github.com/FrancescoGrazioso/Meld) | Metrolist + conta do Spotify + Qobuz | Kotlin | GPL-3 | Ativo | Baixo: casador de faixas e recomendação |
| [Outify](https://github.com/iTomKo/Outify) | Cliente de Spotify para Android | Kotlin + Rust (librespot) | GPL-3 | Início | Baixo: é todo librespot |
| [Simple-Music-Player](https://github.com/SimpleMobileTools/Simple-Music-Player) | Player de arquivos locais para Android | Kotlin | GPL-3 | Parado desde 06/2024 | Baixo: lista básica do player local |
| [OpenTune-Web](https://github.com/Arturo254/OpenTune-Web) | Site do app OpenTune | HTML | GPL-3 | Ativo | Nenhum para o código |
| [Open-Source-Music-Streaming-Apps](https://github.com/chayotic/Open-Source-Music-Streaming-Apps) | Lista de apps | Markdown | CC0 | Ativo | Índice para achar mais referências |

Da lista, valem uma olhada futura os de Windows que não estavam entre os 12: Harmonoid e Nora (players locais),
BitChord (o visual que o Orpheon e o Prism citam como inspiração), Pear Desktop, Spotube, muffon e sonora.

---

## 2. Duas regras antes de reaproveitar

### 2.1 Licença

- **GPL-3 e AGPL-3** (todos menos `spotify-player` e a lista): código copiado ou traduzido linha a linha vira obra
  derivada. O nosso motor é compartilhado entre o Player gratuito e o Studio pago, então código GPL dentro dele
  obrigaria o Studio a abrir o código inteiro. Regra prática: **ler para entender, escrever o nosso**. Ideias,
  formatos de arquivo e protocolos públicos não têm dono.
- **MIT** (`spotify-player`): pode copiar, mantendo o aviso de direitos.
- O que os apps usam por baixo e tem licença permissiva pode entrar direto: as crates `lofty`, `symphonia`,
  `souvlaki`, `discord-rich-presence`, `ort` (ONNX Runtime), `rusqlite`, `strsim`; o modelo *Beat This!* (MIT); os
  modelos de separação de voz UVR-MDX e Spleeter (MIT); os perfis do AutoEq (MIT).
- O nosso repositório ainda **não tem arquivo de licença**. Vale decidir isso antes da primeira versão pública.

### 2.2 O que fica de fora pelas nossas regras

| O que eles fazem | Quem faz | Por que não |
| --- | --- | --- |
| Tocar e baixar áudio do YouTube (API interna, quebra de assinatura, BotGuard, yt-dlp) | limusic, Metrolist, Meld, Prism, SimpMusic, nuclear | Regra nossa: nada de áudio do YouTube fora do Modo Escuta |
| Tocar Spotify com librespot | Outify, spotify-player | Protocolo não oficial; regra nossa: Spotify só pelo Modo Escuta |
| Entrar na conta do Spotify por WebView e usar a API privada (GraphQL, token por TOTP) | Meld, Orpheon, Prism (parte), SimpMusic (Canvas e letras) | API privada, risco para a conta do usuário |
| Áudio sem perdas do Qobuz por servidores de terceiros | Meld | Contorna a assinatura do serviço |
| Silenciar ou pular anúncios | Orpheon | Contra os termos do Spotify |
| Letras da Apple Music, Netease, QQ, Kugou, Musixmatch, Genius por raspagem | limusic, Metrolist, Prism, Orpheon | Fontes sem licença para terceiros; ficamos com o LRCLIB |
| Reconhecer música pela API privada do Shazam | Metrolist, Prism, SimpMusic | Endpoint não oficial (ver 3.6) |
| Capas animadas da Apple Music, Tidal e Canvas do Spotify | Prism, SimpMusic | Conteúdo de API privada |

---

## 3. O que reaproveitar, por área

Cada item diz de onde vem, como eles fazem e como fica no nosso app. "Ideia nossa" marca o que é sugestão minha e não
está em nenhum dos repositórios.

### 3.1 Letras sincronizadas

- **De onde**: limusic (`src-tauri/src/lyrics.rs`), Metrolist (`com/metrolist/lrclib/LrcLib.kt`), nuclear, Prism.
- **Como fazem**: vários provedores numa lista que o usuário ordena e desliga. O que serve para nós é o **LRCLIB**
  (`lrclib.net/api`): gratuito, sem chave, pede só que o app se identifique no cabeçalho `User-Agent`. Primeiro
  `/api/get` com título, artista, álbum e duração (resposta exata); se não achar, `/api/search` e escolhe pela
  duração mais próxima (±2 s, depois ±5 s). A duração é o que evita pegar a letra de outra versão da mesma música.
- **Detalhes que valem copiar como comportamento**: letra sincronizada ganha da simples; resultado guardado em SQLite
  para sempre e "não tem letra" por 24 h; falha de rede não guarda nada; ajuste de atraso por música (letra adiantada
  ou atrasada) e escolha manual da fonte ficam fixados; clicar numa linha pula para aquele ponto.
- **No nosso app**: um módulo novo do painel, "Letra". No Modo Escuta já temos título, artista, duração e posição pelo
  Windows (SMTC), que é tudo que o LRCLIB precisa. A linha atual pode também alimentar o visual (ideia nossa).
- **Atenção**: mostrar letra em produto pago tem risco de direito autoral. No Player gratuito é o que todos fazem; no
  Studio, deixar de fora.

### 3.2 Scrobble: Last.fm e ListenBrainz

- **De onde**: limusic (`lastfm.rs`, `listenbrainz.rs`), Metrolist (`ScrobbleManager.kt`, `lastfm/LastFM.kt`).
- **Como fazem**: um relógio só para os dois serviços. "Tocando agora" quando a faixa começa e um scrobble quando
  passa da **metade ou de 4 minutos**, o que vier primeiro (regra oficial do Last.fm); faixas com menos de 30 s não
  contam. O relógio pausa junto com a música.
  - Last.fm: `auth.getToken` → abre o navegador na página de autorização → consulta `auth.getSession` até aprovar. A
    chave de sessão não expira. Chamadas assinadas com MD5 dos parâmetros em ordem + segredo.
  - ListenBrainz: o usuário cola um token; `GET /1/validate-token` e `POST /1/submit-listens` (`playing_now` e
    `single`). É aberto e não tem restrição de uso comercial, ao contrário do Last.fm.
- **Detalhes**: tudo "melhor esforço" (falha vira linha de log, nunca erro na tela); regras de limpeza do título e do
  artista editáveis pelo usuário, com prévia de como a faixa vai ser contada; chave da API fora do repositório.
- **No nosso app**: é o módulo Last.fm que já está no plano (fase 5), e o ListenBrainz entra quase de graça junto.
  **Cuidado próprio do Modo Escuta**: o Spotify e outros apps já fazem scrobble sozinhos; precisamos de uma opção por
  app de origem para não contar duas vezes (ideia nossa).

### 3.3 Cartão no Discord

- **De onde**: limusic (`discord.rs`, 1.400 linhas só de casos de borda), nuclear (`src-tauri/src/discord.rs`).
- **Como fazem**: no desktop não precisa de login. O Discord aberto expõe um canal local e a crate
  `discord-rich-presence` manda o cartão ("Ouvindo…", capa, barra de progresso).
- **Lições do limusic**: mostrar só enquanto toca; sem barra quando pausado (o Discord não tem estado de pausa);
  limitar a frequência de envio, porque o Discord descarta em silêncio atualizações muito próximas; juntar pulos
  rápidos de faixa num envio só; abrir a conexão ao ligar o recurso, não na primeira música.
- **No nosso app**: módulo opcional da Loja. Precisa de um ID de aplicativo do Discord (gratuito).

### 3.4 Integração com o Windows

| Recurso | De onde | Como | No nosso app |
| --- | --- | --- | --- |
| Botões anterior / tocar / próxima na miniatura da barra de tarefas | limusic `taskbar.rs` | `ITaskbarList3::ThumbBarAddButtons`, cliques chegam como `WM_COMMAND` | Serve já: mandam os mesmos comandos de mídia que o dock |
| Bandeja do sistema | limusic `tray.rs` | Menu Mostrar / Tocar / Próxima / Sair; fechar a janela mantém o app | Serve já |
| Atalhos globais | limusic `hotkeys.rs` | `RegisterHotKey`, configuráveis | Encaixa no nosso sistema de comandos |
| Mini player sempre no topo | limusic `mini.rs` | Janela pequena que substitui a principal | Um "layout" a mais na Loja; para nós, uma janelinha só com o visual |
| Publicar no controle de mídia do Windows | limusic `media.rs` | Crate `souvlaki` numa thread dona | Só quando tivermos player próprio (fase 4); hoje nós **lemos** o SMTC |
| Aviso de troca de faixa | limusic `notify.rs` | Só com a janela sem foco, desligado por padrão | Opcional |
| Iniciar com o Windows, instância única, lembrar posição da janela | limusic (plugins do Tauri) | — | Instância única já temos |
| Ícone do app escolhido pelo usuário | limusic `appicon.rs` | Troca na janela e na barra de tarefas | Combina com temas e pacotes |
| Escolher a saída de áudio só do app | SimpMusic | Não move o som dos outros apps | Fase 4 |

### 3.5 Análise musical: batida, compasso, tom e energia

É a área onde os repositórios mais acrescentam ao que já temos em `crates/analise`.

- **Orpheon, AutoMix** (`docs/automix/README.md`, código em `base/automix`). Analisa a música inteira uma vez:
  - ataques por fluxo espectral em 40 bandas logarítmicas, mais as bandas abaixo de 150 Hz (bumbo);
  - o modelo **Beat This!** (MIT, ONNX pequeno em int8) dá o nível métrico e o **primeiro tempo de cada compasso**.
    Eles explicam por que só autocorrelação não basta: ela hesita entre o tempo real e 3/4 ou 4/3 dele em hi-hats
    de trap. É o mesmo tipo de erro que o nosso detector de BPM pode ter;
  - uma grade de batidas constante ajustada por mínimos quadrados e conferida (parcela de batidas na grade, desvio em
    8 tempos, concordância com o modelo);
  - por compasso: nível, graves e agudos, "novidade" (4 compassos contra os 4 anteriores), fronteiras de seção e frases
    de 8 compassos; **tom** (Krumhansl–Kessler sobre o cromagrama), volume percebido, onde o som começa e termina.
- **Prism** (`playback/MixTempo.kt`, `MusicKey.kt`, `Energy.kt`, `DjSequencer.kt`): BPM com nota de confiança (música
  ambiente pontua baixo e o app então não tenta alinhar batidas), BPM comparado "sem oitava" (85 = 170), duração do
  fade final e da introdução calma, tom com a **roda de Camelot** para ordenar a fila, e uma energia de 0 a 1 por
  faixa (volume + clareza da batida + brilho).
- **No nosso app**:
  - no modo arquivo e no cache de linha do tempo, rodar o Beat This! pela crate `ort` (no Windows com DirectML, como o
    karaokê do limusic) para ter compasso e primeiro tempo confiáveis: o logo e os pulsos "uma vez por compasso"
    passam a cair no tempo certo;
  - a "novidade" por compasso e as fronteiras de seção melhoram o módulo Estrutura e as viradas do diretor;
  - confiança do BPM: quando for baixa, o diretor para de pulsar na batida em vez de pulsar errado;
  - tom → paleta (ideia nossa): cor do visual ligada ao tom e ao modo maior/menor;
  - no Studio, a mesma análise serve para cortes de vídeo no compasso.
- **Limite**: o Beat This! lê a música inteira, então não vale para o Modo Escuta ao vivo sem cache. Lá continua o
  nosso detector em tempo real.

### 3.6 Descobrir que música está tocando

- **Como fazem** (Metrolist, Prism, SimpMusic): geram a "impressão digital" do áudio com o algoritmo portado do
  SongRec (GPL-3) e mandam para um endereço não oficial do Shazam (`amp.shazam.com/discovery/...`).
- **Por que interessa**: no Modo Escuta, nem todo app informa a faixa ao Windows (um vídeo no navegador, um jogo).
  Reconhecer pelo som preencheria título, capa e letra.
- **Recomendação**: não usar o Shazam (API privada + código GPL). A alternativa aberta é **Chromaprint + AcoustID**
  (ideia nossa; os termos de uso comercial do AcoustID precisam ser conferidos antes). Fica como módulo futuro.

### 3.7 Separar voz e instrumental

- **De onde**: limusic (`crates/karaoke`, 9 mil linhas de Rust), Prism (`playback/sing`).
- **Como fazem**: modelos UVR-MDX-NET (limusic) ou Spleeter (Prism) rodando em ONNX Runtime, com DirectML no Windows.
  O limusic separa em blocos e grava um WAV "ainda crescendo", para trocar para o instrumental antes de terminar.
- **No nosso app** (futuro, modo arquivo e Studio): o visual reagindo em separado à voz e à base, por exemplo
  partículas com a voz e pulso com a bateria. É um diferencial que nenhum deles tem.

### 3.8 Histórico, estatísticas e retrospectiva

- **De onde**: limusic (`stats.rs`), nuclear (`history/stats`), SimpMusic (Wrapped, "recibo" de escuta), Prism (Replay
  e Rewind), Orpheon (recap).
- **Como fazem**: cada execução vira uma linha no SQLite; as estatísticas são calculadas na hora de abrir a tela, sem
  manter contadores (um ano de uso pesado agrupa em milissegundos). Tudo fica no aparelho.
- **O que mostram**: tempo ouvido e comparação com o período anterior, execuções, músicas e artistas novos, top
  músicas / artistas / álbuns, relógio de 24 horas, dia da semana, calendário estilo GitHub, sequência de dias,
  retrospectiva do ano em cartões para compartilhar, playlists mensais.
- **No nosso app**: o módulo "Nesta sessão" vira a porta de uma página de histórico. Como temos a análise, dá para
  somar o que ninguém tem (ideia nossa): BPM médio do dia, curva de energia da semana, horário em que o usuário ouve
  música mais agitada.

### 3.9 Importar histórico e playlists

- **De onde**: Prism (`data/imports`: `SpotifyExport.kt`, `LastFmHistory.kt`, `TakeoutHistory.kt`,
  `TrackMatcher.kt`), limusic (`spotify.rs`, `import.rs`), Meld (`SpotifyYouTubeMapper.kt`).
- **O que serve**: ler o **zip do export de dados do Spotify** (JSON com todas as playlists e curtidas, lido sem
  rede), CSV do Exportify / TuneMyMusic / Soundiiz, histórico do Last.fm e do Google Takeout. Já estava no nosso plano;
  o código deles mostra o formato dos arquivos.
- **Casar faixas entre catálogos**: nota por título + artista + duração, com desconto para versões "ao vivo",
  "karaokê" e afins; **ISRC** quando existe (casamento exato); resultado guardado; o usuário pode corrigir à mão e a
  correção vale para sempre. O nuclear leva isso além com "verificação" compartilhada entre usuários.
- **O que não serve**: ler playlist por link usando a API privada do Spotify (limusic, Meld).

### 3.10 Loja de extensões

A melhor referência é o **Orpheon** (`docs/extensions/API.md`, `POLICY.md`, `UI-LOOKS.md`); o **nuclear** completa com
o modelo de provedores.

| Ideia | De onde | Como aplicar na nossa Loja |
| --- | --- | --- |
| **Catálogo fechado de ações**: a extensão declara o que faz (dar letras, ler o que toca, usar a rede…) e cada ação tem ícone, nome e nível normal ou crítico | Orpheon | O nosso `modulo.toml` ganha `acoes`, cada uma com um **motivo escrito pelo autor** (até 80 caracteres), mostrado antes de instalar |
| Rede só para os endereços listados no manifesto; `*` é crítico | Orpheon | Igual |
| **Orçamento por chamada**: 3 s para carregar, 3 a 30 s por gancho, checagem a cada 5.000 instruções; três falhas seguidas desligam a extensão; erro de rede não conta como falha | Orpheon | Mesmo princípio que já anotamos do Spotifast |
| Limites: 120 requisições/min, 5 MB por resposta, só https, 512 KB de armazenamento | Orpheon | Igual |
| **Atualização só se aplica sozinha se não pedir nada novo**; ação, endereço, autor ou motivo diferente espera o OK do usuário, mostrando só o que muda; botão "Voltar para a versão X" | Orpheon | Igual |
| **Lista de bloqueio** lida uma vez por dia (`blocked.json` com id e hash) | Orpheon | Igual, no nosso repositório de registro |
| Extensão só de aparência, sem código: cores, partes escondidas, ordem das abas | Orpheon | São os nossos temas e skins |
| `replaces` e `conflictsWith`; ganchos "somados" (todas respondem) contra "em ordem" (a primeira que responder) | Orpheon | Útil quando houver dois módulos de letra ou de fonte |
| Traduções dentro do manifesto, com queda de `pt-BR` para `pt` | Orpheon | Igual |
| Ferramenta de linha de comando para o autor testar no computador | Orpheon | Igual |
| **Registro estático no GitHub**: a loja lê um índice; o código fica no repositório do autor, baixado do release | nuclear, Orpheon | Sem servidor nosso. O plano já pede catálogo assinado |
| Plugin "de desenvolvimento" carregado de uma pasta, com botão de recarregar | nuclear | Bom para quem cria |
| **Tipos de provedor** com capacidades declaradas (metadados, áudio, página inicial, descoberta, playlists); a interface esconde a seção que o provedor ativo não oferece | nuclear | É o nosso "cada módulo declara as próprias capacidades" |
| Política pública: o que bloqueia uma extensão e como denunciar | Orpheon | Copiar a estrutura do texto, não o texto |

### 3.11 Temas e personalização

| Ideia | De onde | No nosso app |
| --- | --- | --- |
| Tema em JSON com superfícies nomeadas (`background`, `card`, `popover`, `topbar`, `sidebar-left`…), fonte, raio, borda e sombra, com prévia ao vivo | nuclear `docs/themes` | Formato dos nossos temas em arquivo |
| Loja de temas com amostra da paleta em faixas diagonais | nuclear | Cartões da aba Temas |
| **"Look" compartilhado como um código de texto** (`PRISMLOOK1:…`) ou arquivo com as fotos; galeria da comunidade é um `index.json` aceito por pull request; um Look só pode mudar a aparência | Prism `themes/` | Os nossos pacotes: colar um código aplica tema + skin + layout |
| Cor tirada da capa que está tocando | limusic, Metrolist, Prism | Já temos a cor pela energia; a capa vira mais uma origem |
| **Paleta do papel de parede**: lê um arquivo gerado por Matugen, wallust ou pywal e recarrega ao vivo | limusic `docs/MATUGEN.md` | É o "Material You" do item end4 do nosso plano, já resolvido |
| Estilos por componente no tema e **atalhos em arquivo**, com sequências de teclas | spotify-player `docs/config.md` (MIT) | O nosso `atalhos.toml` |
| Três estilos de tela "tocando agora"; letra em tela cheia como vídeo | SimpMusic, Prism | Layouts e skins |
| Fontes de leitura fácil (Atkinson Hyperlegible, OpenDyslexic), ambas OFL | Prism | Acessibilidade, junto do modo seguro |
| "Modo desempenho": um botão que alivia o app em máquina fraca | Prism | Combina com o nosso orçamento de quadros |
| Zoom da interface com Ctrl + roda; tela com todos os atalhos | limusic | Barato de fazer |
| Regras de desempenho de interface | limusic `docs/UI-PERFORMANCE.md` | Leitura útil: a página deles também redesenha o tempo todo |

### 3.12 Player de arquivos locais (a nossa fase 4)

- **Biblioteca**: limusic `local.rs` varre pastas e lê etiquetas com a crate `lofty`; arquivo que ela não entende
  entra na lista pelo nome. O arquivo local usa as mesmas telas e a mesma fila do resto. Simple Music Player:
  importar e exportar M3U, editar etiquetas.
- **Motor**: o limusic usa libmpv; para nós, `symphonia` + `cpal` evita uma dependência grande (o karaokê do limusic
  já decodifica com `symphonia`).
- **Recursos que todos têm e o usuário espera**: fila com arrastar, fila restaurada ao reabrir, sem intervalo entre
  faixas, normalização de volume (com limitador para não distorcer), **crossfade** (o do Prism escolhe a duração
  pelo fade da música que acaba e pela introdução da que entra), velocidade e tom, pular silêncio, temporizador
  para dormir, "estatísticas para nerds" (codec, taxa, bits).
- **Equalizador**: 10 bandas com predefinições e, melhor, **paramétrico com perfis do AutoEq** (correção por modelo
  de fone, MIT). Metrolist (`eq/`), Prism (`ParametricEq.kt`), SimpMusic. O filtro biquad é matemática pública.
- **Efeito espacial por software** (Prism `SpatialAudioProcessor.kt`): alargamento, cruzamento entre canais e sala.
- **Para o visual**: o Prism tira o sinal **depois** da normalização e sem microfone; é a mesma decisão que tomamos
  no Modo Escuta (antes do volume).

### 3.13 Fontes de música que são legais

- **Rádios pela internet**: o Orpheon usa o [Radio Browser](https://www.radio-browser.info) (`radio/RadioBrowserApi.kt`),
  um diretório comunitário e gratuito. Descobre o servidor por `all.api.radio-browser.info`, aceita só https e guarda
  os favoritos no aparelho. **Para nós é a fonte nova mais fácil**: toca dentro do app, o visual reage ao som de
  verdade e não há conta nem chave.
- **Podcasts por RSS** (Prism): velocidade, botões de pular, lembrar onde parou.
- Audius e Jamendo continuam como no plano; nenhum dos 12 usa.
- **Metadados**: o Prism busca gênero no iTunes Search e no Deezer e data de lançamento no Deezer. A opção mais
  limpa é o MusicBrainz com o Cover Art Archive (ideia nossa; dados abertos).

### 3.14 Controle remoto e automação

| Ideia | De onde | No nosso app |
| --- | --- | --- |
| **Controle pelo celular**: o app sobe um servidor na rede local, mostra um QR e qualquer navegador vira controle. Só aceita a rede local | nuclear ("Jam") | Trocar visual, tema e cor pelo celular numa festa |
| **API HTTP local** (`/api/playback`, `/api/queue`, `/api/settings/{id}`…) | nuclear `docs/integrations/http-api.md` | Como tudo no nosso app já é comando com nome, a API é uma lista dos comandos |
| **Servidor MCP**: quatro ferramentas (listar métodos, detalhar, descrever tipo, chamar) em `127.0.0.1:8800` | nuclear `src-tauri/src/mcp` | Uma IA dirigindo o visual. Mesmo desenho, sobre os nossos comandos |
| Servidor compatível com MPD | nuclear | Baixa prioridade |
| Comandos de terminal que falam com o app aberto por uma porta local | spotify-player (MIT) | `player visual proximo` |
| **Gancho de evento**: roda um programa do usuário a cada troca de faixa | spotify-player (MIT) | Automação simples, no espírito do Hyprland |
| Passar a sessão do celular para o desktop por QR | SimpMusic, limusic `devicelink.rs` | Depende de app de celular |

### 3.15 Ouvir junto

- **De onde**: Metrolist (`listentogether/`, com o servidor `metroserver`), limusic (`crates/listen-protocol` e
  `crates/sync-server`, em Rust), SimpMusic (compatível com o Metrolist), Prism (só na mesma rede).
- **Como fazem**: cada pessoa toca o próprio áudio; a sala só repassa tocar, pausar, pular e a fila. Um anfitrião
  manda, o servidor calcula a posição, a sala tem código e o anfitrião aprova quem entra. Mensagens JSON por
  WebSocket; o anfitrião é trocado 20 s depois de cair.
- **No nosso app** (futuro, ideia nossa): sala de **visual** em vez de sala de música. Todos veem o mesmo visual, o
  mesmo tema e as mesmas viradas, cada um com a própria música tocando. Encaixa no Studio ao vivo.

### 3.16 Acabamento

- **Relatório de diagnóstico num botão** (limusic `diagnostics.rs`, Orpheon): junta versão, sistema e o fim do log
  com os segredos apagados, pronto para colar num relato de erro. A regra deles: apagar demais custa uma pergunta,
  vazar custa a conta do usuário.
- Tela "Novidades" e visualizador de log dentro do app (nuclear).
- Tradução pelo Weblate, gratuito para projeto livre (limusic, Metrolist, Outify); inglês editado à mão, o resto só
  pela plataforma.
- Atualização automática do app (limusic, nuclear, Orpheon).
- Não deixar o PC dormir enquanto toca (limusic `inhibit.rs` diz que no Windows o próprio áudio já segura).
- Perfis separados no mesmo computador (nuclear `profile.rs`).
- Passeio guiado na primeira abertura (Prism `ui/tour`).
- Temas por época do ano e pelo horário do sol (Prism `data/looks/Seasons.kt`, `SunClock.kt`).

---

## 4. Ordem sugerida

| Quando | O quê | Por quê |
| --- | --- | --- |
| **Agora**, dentro do Modo Escuta | Histórico em SQLite + página de estatísticas (3.8) | Base para scrobble, Replay e importação |
| | Módulo Letra com LRCLIB (3.1) | Maior ganho visível com menor risco |
| | Botões na barra de tarefas, bandeja, mini player (3.4) | Trabalho pequeno, cara de app pronto |
| | Relatório de diagnóstico (3.16) | Ajuda nos testes com usuários do TCC |
| **Fase 5**, Loja | Manifesto com ações e motivos, atualização segura, lista de bloqueio, registro no GitHub (3.10) | Define o formato antes de existir o primeiro módulo |
| | Last.fm + ListenBrainz (3.2), Discord (3.3) | Primeiros módulos com código |
| | Pacote como código de texto, paleta do papel de parede (3.11) | Customização no estilo que queremos |
| **Análise** | Beat This!, novidade por compasso, tom, confiança do BPM (3.5) | Melhora o diretor e vira resultado do TCC |
| **Fase 4**, player local | Biblioteca, sem intervalo, normalização, crossfade, AutoEq (3.12); rádios (3.13) | O conjunto que todo player tem |
| **Depois** | API local e controle por QR, MCP, gancho de evento (3.14) | Barato quando tudo já é comando |
| | Importar export do Spotify e Last.fm (3.9) | Depende do histórico |
| | Stems (3.7), reconhecimento por som (3.6), sala de visual (3.15) | Diferenciais, mais pesados |

---

## 5. Onde olhar em cada repositório

| Assunto | Caminho |
| --- | --- |
| Letras (LRCLIB e cache) | `limusic/src-tauri/src/lyrics.rs` · `Metrolist/app/src/main/kotlin/com/metrolist/lrclib/LrcLib.kt` |
| Last.fm e ListenBrainz | `limusic/src-tauri/src/lastfm.rs`, `listenbrainz.rs` · `Metrolist/.../utils/ScrobbleManager.kt` |
| Discord | `limusic/src-tauri/src/discord.rs` |
| Barra de tarefas, bandeja, atalhos, mini player | `limusic/src-tauri/src/taskbar.rs`, `tray.rs`, `hotkeys.rs`, `mini.rs` |
| Estatísticas | `limusic/src-tauri/src/stats.rs` · `nuclear/packages/player/src-tauri/src/history/stats/` · `SimpMusic/.../ui/screen/home/wrapped/` |
| Diagnóstico | `limusic/src-tauri/src/diagnostics.rs` |
| Arquivos locais | `limusic/src-tauri/src/local.rs` |
| Karaokê (ONNX, DirectML) | `limusic/crates/karaoke/` |
| Sala de escuta | `limusic/crates/listen-protocol/`, `crates/sync-server/` · `Metrolist/.../listentogether/` |
| Paleta do papel de parede, desempenho da interface | `limusic/docs/MATUGEN.md`, `docs/UI-PERFORMANCE.md` |
| Extensões: API, política, aparência | `Orpheon/docs/extensions/API.md`, `POLICY.md`, `UI-LOOKS.md` |
| AutoMix: análise e plano de transição | `Orpheon/docs/automix/README.md`, `segue-design.md` |
| Rádio | `Orpheon/base/app/src/main/java/com/music/spotui/radio/` |
| Plugins, provedores, publicação | `nuclear/packages/docs/plugins/`, `core-concepts/plugins-and-providers.md`, `packages/plugin-sdk/` |
| Temas e loja de temas | `nuclear/packages/docs/themes/` · `Prism/themes/README.md` |
| MCP, API HTTP, MPD, controle remoto | `nuclear/packages/docs/integrations/`, `development/mcp-architecture.md`, `user-manual/remote-control.md` |
| Modo DJ, tom, energia, crossfade | `Prism/app/src/main/java/com/prism/music/playback/` (`MixTempo.kt`, `MusicKey.kt`, `Energy.kt`, `DjSequencer.kt`, `Mix.kt`) |
| Importação de histórico | `Prism/.../data/imports/` · `limusic/src-tauri/src/spotify.rs` (só o trecho do zip e do CSV) |
| Equalizador e AutoEq | `Metrolist/app/src/main/kotlin/com/metrolist/music/eq/` · `Prism/.../playback/ParametricEq.kt` |
| Casador de faixas | `Meld/app/src/main/kotlin/com/metrolist/music/playback/SpotifyYouTubeMapper.kt` |
| Configuração, temas, atalhos, gancho de evento (MIT) | `spotify-player/docs/config.md`, `spotify_player/src/config/` |
