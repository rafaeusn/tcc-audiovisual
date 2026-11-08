# Pesquisa de referências para o app (Spotifast, Spicetify, Hyprland/end4)

Pesquisa feita em 07/11/2026 para o planejamento geral. Base: leitura do código do Spotifast (v0.17.1, ~176 mil
linhas de Rust) e do Spicetify CLI, documentação oficial do Spotify (Soloist e Developer Policy) e das APIs de
SoundCloud, YouTube Music e Last.fm.

---

## 0. Resumo em 5 linhas

1. O Spotifast é uma ótima **escola de engenharia** (Rust, rápido, leve), mas **não é "tudo dentro das diretrizes"**:
   o player padrão dele é o librespot (não oficial) e ele usa endpoints privados do Spotify.
2. A **Developer Policy do Spotify proíbe exatamente o nosso núcleo**: sincronizar gravações com mídia visual (III.6),
   analisar o conteúdo (III.13) e uso comercial em apps de streaming (IV.2). Isso vale para quem usa a plataforma
   de desenvolvedor deles (Web API, SDKs, Soloist).
3. Caminho seguro: **não usar a plataforma do Spotify** para o visual. O usuário toca no app oficial e nós
   **escutamos só o áudio daquele processo** (técnica que o próprio Spotifast usa no Windows) e lemos o "tocando
   agora" pelo Windows (SMTC). Nosso player toca arquivos locais, Audius e Jamendo.
4. Do Spicetify: o **modelo de customização** (cores por variáveis, CSS do usuário, extensões, apps, marketplace,
   recarga ao vivo). Do Hyprland/end4: **tudo é configuração, recarrega na hora, paleta gerada da imagem, presets
   compartilhados**.
5. A decisão técnica mais importante é **como desenhar a UI** (nativa em Rust ou Rust + WebView). Ver a seção 5.

---

## 1. Spotifast (github.com/crmne/spotifast)

### O que é
Cliente Spotify nativo para Linux, macOS e Windows. Abre em menos de 1 s e usa 100–250 MB de RAM, contra 600 MB–1 GB do
app oficial. Licença MIT, sem servidor próprio e sem telemetria.

### Como ele "traz o Spotify para dentro"
São três conexões independentes:

| Conexão | O que faz | Status |
| --- | --- | --- |
| **Web API** (Client ID do próprio usuário) | Biblioteca, busca, playlists, controle de dispositivos | Oficial, mas o *Development Mode* aceita **no máximo 5 contas por app**. Cada usuário precisa registrar o próprio app no painel do Spotify (e ter Premium). A cota ampliada exige empresa registrada com centenas de milhares de ouvintes. Apps criados após nov/2024 não acessam *audio features*, recomendações nem playlists do Spotify. |
| **librespot** (player padrão) | Toca o áudio (até 320 kbps), Spotify Connect, letras, rádio, "Made for you", contagem de plays | **Não oficial** (protocolo reverso). Também usa endpoints privados (Pathfinder, spclient). O próprio projeto admite que o Spotify às vezes recusa ("audio key error") e que endpoints podem mudar sem aviso. |
| **Soloist** (opcional desde a v0.15) | Player oficial do Spotify para terceiros (sem interface, controlado por WebSocket local) | Oficial. Exige Premium e uma chave pessoal por usuário, que não pode ser compartilhada. Builds expiram em 90 dias e não podem ser redistribuídas (o app baixa do CDN do Spotify). **A API não entrega o áudio.** |

### Como ele faz visualizador com o Soloist no Windows
`src/soloist/native/windows.cpp` usa **WASAPI process loopback**
(`AUDIOCLIENT_ACTIVATION_TYPE_PROCESS_LOOPBACK`, `PROCESS_LOOPBACK_MODE_INCLUDE_TARGET_PROCESS_TREE`). Ele captura
**só o áudio de um processo e dos filhos** (o PID do Soloist), não o som do sistema todo. Funciona no Windows 10 2004+
(build 19041+).
→ **Isso resolve o nosso Modo Escuta**: dá para capturar só o `Spotify.exe` (ou o navegador com o YouTube Music), sem
pegar notificação, Discord ou jogo.

### Lições de arquitetura para copiar
| Lição | Onde no código | Para nós |
| --- | --- | --- |
| A thread da UI nunca bloqueia; rede e áudio rodam num runtime tokio e conversam por canais | `src/backend.rs` | Núcleo Rust assíncrono, UI só desenha |
| **UI otimista**: o clique mostra o resultado na hora e o backend confirma depois; nada "pisca e volta" | `AGENTS.md` | Regra de UX desde o dia 1 |
| Visualizador (MilkDrop/projectM) num **processo filho**, com áudio por memória compartilhada (ring buffer) | `src/milkdrop.rs`, `src/milkdrop/child.rs` | Renderer de shaders isolado: se travar, o player continua |
| Visualizador lê o sinal **depois do EQ e antes do volume** (volume zero ainda dança) e compensa ~180–200 ms do buffer de saída | `src/vis.rs` | Mesma ideia da nossa compensação de latência |
| Skins Winamp (`.wsz`) e Windows Media Player (`.wmz`) com **interpretador de script próprio com orçamento de passos** (um skin com bug não trava o app) | `src/skin/`, `src/skin/script/interp.rs` | Modelo para plugins seguros |
| Paleta única (`Palette`), tema claro/escuro, cor tirada da capa e integração com o tema do desktop (Omarchy) | `src/theme.rs` | Tokens de tema |
| Credenciais no cofre do sistema (Windows Credential Manager + DPAPI) | `src/credentials.rs` | Tokens do Last.fm, Audius etc. |
| Arquivos de configuração gravados de forma atômica (temporário, depois troca) | `fastframe-file` | Layouts e presets nunca corrompem |
| Media keys / overlay de mídia do Windows (SMTC), bandeja, autostart, instância única, modo portátil | crates `fastframe-*` | "Pacote desktop" pronto como referência |
| Economia de energia: no máximo 30 fps na bateria e 1 fps com a tela bloqueada | `docs/_guide/milkdrop.md` | Orçamento de desempenho do visual |
| Atualização automática com manifesto assinado (Ed25519) + SHA-256 | `fastframe-update` | Distribuição segura |
| UI em **egui + glow** (OpenGL), sem wgpu: binário pequeno | `Cargo.toml` | Ver seção 5 |
| i18n por arquivos `.po` compilados no build (tem pt-BR) | `assets/i18n/` | Multilíngue barato |

### O que ele NÃO tem (espaço para nós)
Pré-análise de estrutura (seções, drops), diretor automático, templates próprios (só MilkDrop e Winamp), editor de
layout, estatísticas tipo Last.fm, outras fontes de música e plugins de código.

---

## 2. Termos do Spotify: o ponto crítico

Trechos da Spotify Developer Policy (developer.spotify.com/policy), consultada em 07/11/2026:

- **III.6**: "Do not synchronize any sound recordings with any visual media"
- **III.13**: "Do not analyze the Spotify Content or the Spotify Service for any purpose"
- **III.11**: não imitar nem substituir uma experiência central do Spotify
- **IV.2**: uso comercial não é permitido em apps de streaming (não pode vender assinatura, anúncio ou patrocínio no app)
- **III.14**: não treinar ML com conteúdo do Spotify

**Consequência:** um app que usa a Web API ou o Soloist para tocar Spotify **e** gera visual sincronizado **e** cobra
assinatura viola três cláusulas ao mesmo tempo. O Spotifast escapa porque é gratuito, de uso pessoal, e o visualizador
é um extra.

**Caminho recomendado:**
- Não usar a plataforma do Spotify para tocar nem para o "tocando agora".
- O usuário toca no **app oficial**. Nós lemos o título pelo **SMTC do Windows** (a caixinha de mídia do sistema) e
  capturamos só o áudio do processo dele (process loopback). Isso é o Modo Escuta.
- Gosto, histórico e playlists vêm do **export de dados do Spotify** (o usuário baixa) + **Last.fm**, como já estava no
  `CONTEXTO-TCC.md`.
- Opcional: um "modo pessoal" gratuito com Soloist só para controle (play/pause/fila), **sem** ler o áudio dele para o
  visual. Isso precisa de validação com um advogado antes de existir na versão paga.

> Não sou advogado. Antes de cobrar assinatura, vale uma consulta jurídica, principalmente sobre o Modo Escuta captar
> áudio de outro app (a captura em si é um recurso do Windows; o risco está nos termos de consumidor de cada serviço).

---

## 3. Outras fontes

| Fonte | API oficial? | Tocar dentro do app? | Recomendação |
| --- | --- | --- | --- |
| **Arquivos locais** | — | Sim | Principal. Análise completa (modo arquivo) |
| **Audius** | Sim, aberta | Sim (stream liberado) | Streaming legal no player |
| **Jamendo** | Sim | Sim, uso não comercial (comercial exige acordo) | Player |
| **Last.fm** | Sim (chave gratuita) | Não (é só metadado) | Scrobble, histórico e gosto. Uso comercial pede permissão |
| **SoundCloud** | Sim, mas **fechada**: chave nova passa por revisão caso a caso e exige Artist Pro. Proíbe capturar áudio e apps tipo rádio; assinantes Go exigem contrato | Só com aprovação | Modo Escuta. Pedir chave depois, com o produto pronto |
| **YouTube Music** | **Não existe API oficial.** O `ytmusicapi` é não oficial (imita o site com cookies) | Não | Modo Escuta (captura do processo do navegador/app + SMTC) |
| **YouTube** (vídeos) | Data API + IFrame Player oficiais | Só o player embutido visível; os termos proíbem separar áudio e vídeo | Evitar para o visual |
| **Spotify** | Ver seção 2 | Não para o nosso caso | Modo Escuta + export de dados |

**Conclusão:** o "matcher" que já estava planejado continua certo. O que muda é que Spotify, SoundCloud e YouTube Music
entram **pelo Modo Escuta**, e por isso o Modo Escuta passa a ser **o recurso mais importante do app**. Daí a importância
do cache de timeline (ideia 3 do documento de diferenciais): é o que faz o Modo Escuta ficar tão sincronizado quanto um
arquivo.

---

## 4. Spicetify (github.com/spicetify/cli)

### Como funciona
- CLI em **Go** que **modifica o app oficial do Spotify**. O app oficial é uma interface web (CEF + React) empacotada em
  `.spa`. O Spicetify extrai, aplica patches por expressão regular no JS e no CSS e reempacota
  (`src/preprocess/preprocess.go`, `src/apply/apply.go`).
- **Temas**: `color.ini` (esquemas de cor viram variáveis CSS) + `user.css` + assets.
- **Extensões**: arquivos JS injetados que usam um objeto global com API (`globals.d.ts`, 2.400 linhas):
  `Spicetify.Player`, `Platform`, `Menu`, `ContextMenu`, `Topbar`, `Playbar`, `PopupModal`, `CosmosAsync`, GraphQL.
- **Custom Apps**: páginas inteiras novas, como `lyrics-plus`, `new-releases` e `reddit`.
- **Marketplace**: loja comunitária de temas e extensões hospedados no GitHub.
- **`css-map.json`**: traduz as classes ofuscadas do Spotify para nomes estáveis. É o maior sofrimento do projeto,
  porque **cada atualização do Spotify quebra tudo**.
- `spicetify watch`: recarrega ao vivo enquanto se edita o tema.
- Também desliga telemetria (Sentry, logging). Modificar o cliente é contra os termos do Spotify.

### Lições
1. **API pública e estável desde o dia 1**: o Spicetify precisa adivinhar classes ofuscadas; nós podemos dar a cada
   componente um **ID e nomes de partes estáveis**, documentados.
2. **Três níveis de customização**: cores (fácil) → CSS (médio) → código (extensão/app). Funciona para públicos
   diferentes.
3. **Marketplace dentro do app**: instalar com um clique.
4. **Recarga ao vivo** no editor.
5. O que **não** copiar: JS arbitrário com acesso total. Lá, uma extensão pode fazer qualquer coisa. Nós devemos isolar
   plugins (sandbox + permissões), como o Spotifast faz com os scripts de skin.

---

## 5. UI modular (Hyprland, end4/illogical-impulse, end4-pC)

### O que esses projetos fazem
- **Hyprland**: compositor *tiling* (janelas se organizam em árvore, sem sobreposição), com bordas, cantos, *gaps* e
  **animações por curvas Bézier**, tudo num **arquivo de configuração que recarrega na hora**.
- **end4/dots-hyprland (illogical-impulse)**: o "shell" (barra, painéis laterais, overview, configurações) é feito em
  **Quickshell (QtQuick/QML)**. **Material You**: escolhe-se um papel de parede e a paleta inteira é gerada dele.
  Tem painel de IA, atalhos para tudo (`Super+/` lista os atalhos) e anti-flashbang.
- **end4-pC (pctrade)**: fork com visual "Material 3 Expressive", launcher que busca páginas de configuração, transições
  com shader e **galeria comunitária de presets do desktop** (baixar, instalar, enviar).

Não consegui abrir os posts do Instagram (pedem login). Ver as perguntas no fim.

### O que levar para o nosso editor de layout
| Ideia | Como aplicar |
| --- | --- |
| **Layout em árvore (tiling)** | A tela é uma árvore de divisões (horizontal/vertical) com proporções. Arrastar a divisória redimensiona. Arrastar um painel para a borda de outro cria uma nova divisão (como o Blender, o VS Code e o egui_tiles) |
| **Camada flutuante** | Widgets livres por cima (relógio, letra, mini player), com *snap* numa grade |
| **Modo edição** | Um atalho liga o modo edição: os componentes ganham alças, a música continua tocando (como o "editar tela" do iOS ou do KDE) |
| **Responsivo de verdade** | Um layout por **faixa de tamanho** (janela pequena, média, grande, tela cheia, segundo monitor). Cada painel declara tamanho mínimo e o que esconder quando aperta |
| **Tudo é arquivo** | O layout é salvo num arquivo legível (TOML/JSON) que recarrega ao vivo; dá para editar pela UI **ou** pelo arquivo, como no Hyprland |
| **Paleta gerada** | Paleta tipo Material You gerada da **capa do álbum** ou do **template visual** ativo |
| **Animações configuráveis** | Curvas Bézier nomeadas para abrir, fechar e mover painéis |
| **Presets compartilháveis** | Um "rice" = layout + tema + templates + plugins num pacote. Galeria dentro do app |
| **Teclado primeiro** | Paleta de comandos (Ctrl+K) que busca ações e configurações, além da lista de atalhos |
| **Desktop widgets (Windows)** | O Rainmeter é a referência no Windows: widgets presos na área de trabalho, configurados por INI. Pode virar um "modo desktop" do visual |

### Componentes (os "blocos de montar")
Player bar, fila, biblioteca, busca, capa grande, **canvas do visual** (pode haver mais de um), letra sincronizada,
mapa da música (energia e drops), espectro/osciloscópio, stats tipo Last.fm, "tocando agora" do Modo Escuta, relógio,
visualizador de BPM e seção, notas e marcações (validação colaborativa).

---

## 6. A decisão de stack (Windows primeiro, desempenho máximo)

| | **A. Rust nativo** (egui ou Slint/Iced + wgpu) | **B. Tauri 2** (núcleo Rust + UI web no WebView2) | **C. Híbrido** (núcleo Rust + UI web + visual nativo wgpu) |
| --- | --- | --- | --- |
| Inicialização e RAM | Melhor (o Spotifast fica em 100–250 MB) | WebView2 soma ~100–200 MB | Parecido com B |
| Customização tipo Spicetify (CSS, temas, plugins JS) | Difícil: tudo é sistema próprio | **Natural**: CSS variables, web components, iframes isolados | Natural na UI |
| Editor de layout | Existe (egui_tiles, egui_dock), mas o visual é menos refinado | Muitas bibliotecas maduras (dockview, golden-layout, gridstack, react-grid-layout) | Igual a B |
| Nossos templates (GLSL/WebGL já prontos) | Portar para wgpu (o naga traduz GLSL→WGSL) | **Reaproveita tudo** | Portar, com o melhor desempenho no visual |
| Tempo até o TCC | Mais longo | **Mais curto** | Médio |
| Exemplo real | Spotifast, Zed (GPUI) | Muitos apps modernos | — |

**Em qualquer opção o núcleo fica em Rust**: captura WASAPI por processo, decodificação, pré-análise e diretor (portar
`preanalysis.js`/`director.js`), fontes (Audius, Jamendo, Last.fm), cache de timeline, cofre de credenciais e SMTC.

**Minha recomendação:** **B agora, com uma fronteira limpa para virar C**. O renderer do visual fica atrás de uma
interface (recebe parâmetros do diretor e devolve frames). Assim, se as medições mostrarem gargalo, trocamos o WebGL
por wgpu nativo sem mexer na UI. Isso entrega a customização máxima e o editor de layout rápido, sem abrir mão de um
núcleo nativo. Para o TCC, medir e documentar o desempenho de B contra o Spotifast (A) vira um resultado.

### Metas de desempenho sugeridas (para medir, não só prometer)
- Abrir em < 1 s, com < 250 MB de RAM em repouso
- Visual a 60 fps (120 se o monitor permitir) com < 8 ms de GPU por quadro em placa integrada
- Pré-análise de uma faixa de 4 min em < 2 s (em Rust deve cair bem abaixo dos ~2–3 s atuais em JS)
- Latência áudio → imagem < 30 ms no modo arquivo
- Na bateria: 30 fps; minimizado: 0 fps

---

## 7. Plugins e temas (proposta inicial)

| Nível | Quem usa | Formato |
| --- | --- | --- |
| 1. Tema | Qualquer usuário | Tokens (cores, raio, fonte, blur, animações) num arquivo; editor visual com prévia ao vivo |
| 2. Estilo | Usuário avançado | CSS por **partes estáveis** dos componentes (`::part(player-bar)`), sem classes ofuscadas |
| 3. Layout | Qualquer usuário | Árvore + flutuantes, editável arrastando ou pelo arquivo |
| 4. Template visual | Artistas (TouchDesigner) | Shader + manifesto com os parâmetros do "contrato semântico" (energia, drop, subida, fase…) |
| 5. Plugin | Desenvolvedores | JS/TS isolado (iframe/Worker) ou WASM, com **permissões declaradas** (ler a fila, controlar o player, rede…) e API versionada |
| Pacote ("rice") | Comunidade | Tema + layout + templates + plugins, instalado pela galeria |

---

## 8. Modelo de produto (decidido em 07/11/2026)

Dois apps, um motor só (núcleo Rust compartilhado: áudio, captura por processo, pré-análise, diretor, renderer wgpu,
templates, temas, layout).

| | **Player** (grátis) | **Studio** (pago, baixado à parte) |
| --- | --- | --- |
| Público | Quem quer ouvir com visual | Quem quer criar vídeo/conteúdo e se apresentar ao vivo |
| Fontes | Arquivos, Audius, Jamendo, Modo Escuta + **módulos de serviço** (Spotify estilo Spotifast, SoundCloud, Last.fm, YT Music) | Só Modo Escuta e arquivos importados. **Nenhum código de Spotify** |
| Templates | Todos, menos os de câmera/corpo | Todos, incluindo os premium de câmera/corpo |
| Controle | Diretor automático + ajustes simples (cor, acessibilidade, velocidade) | Todos os parâmetros, linha do tempo e regras ("no drop → …") |
| Saída | Assistir | Ao vivo (tela, projetor, Spout/NDI/câmera virtual para OBS) e exportação de vídeo |

**Os três ajustes acordados:**
1. **O Studio não grava áudio de streaming.** Vídeo com som só a partir de arquivos importados (com aviso "tenho os
   direitos"). Do Modo Escuta, só saída ao vivo ou vídeo **só com o visual** sincronizado, para o criador pôr o som
   licenciado do TikTok/Reels.
2. **O Player não anuncia o Studio dentro do módulo Spotify** (Policy IV.2.3). A divulgação fica nas áreas gerais do app
   e no site.
3. **Templates de câmera/corpo funcionam com webcam** (pose, recorte e profundidade estimados por IA na GPU). Câmeras de
   profundidade (Kinect, Orbbec, RealSense) são opcionais. Referência: Alvie Starr, "Audio Reactive Guitar Visual –
   TouchDesigner POPs Particle + Kinect".

**Studio como "TouchDesigner fácil":** linha do tempo com a curva de energia e as seções da pré-análise; o diretor
monta um primeiro rascunho automático; o usuário edita com regras simples; um modo de nós fica opcional para quem é
avançado. Exportação 9:16, 16:9 e 1:1 até 4K60, pela GPU e pelo codificador de hardware do Windows, mais rápida que o
tempo real.

**Licença do TouchDesigner:** a Non-Commercial não permite uso comercial. Para o Studio pago, os templates precisam
nascer com a licença Commercial (ou Educational durante o TCC); vale checar também a licença de templates inspirados
em tutoriais.

### Módulos de serviço ("separados, apesar de juntos")
**O Player é Modo Escuta primeiro.** Por padrão ele não pede conexão com nenhum serviço: basta tocar música em qualquer
app. Os serviços são **módulos baixados à parte numa loja de módulos dentro do app** (como o Marketplace do Spicetify),
e nenhum vem instalado. Todos falam a mesma interface
com o núcleo e aparecem com o mesmo visual do app. Cada módulo declara as próprias capacidades e regras:

| Módulo | Capacidades | Regras próprias |
| --- | --- | --- |
| Spotify | Biblioteca, busca, controle; tocar via Soloist/librespot com as chaves do próprio usuário | Sem promoção paga no contexto dele; pode ser cortado pelo Spotify a qualquer momento |
| SoundCloud | Depende da aprovação da API (Artist Pro + revisão) | Sem captura nem modo rádio |
| Last.fm | Só metadados: scrobble, histórico, gosto | Uso comercial pede permissão |
| YT Music | Sem API oficial: Modo Escuta + SMTC | Evitar API não oficial (risco para a conta do usuário) |
| Audius / Jamendo | Tocar dentro do app | Jamendo comercial exige acordo |

Se um serviço fechar a porta, só o módulo dele para de funcionar; o resto continua pelo Modo Escuta.

---

## 9. Framework de UI em Rust (para customização estilo Hyprland/end4)

**Requisitos:** layout em árvore arrastável e redimensionável + widgets flutuantes; skins feitas pelo usuário,
carregadas em tempo de execução e com recarga ao vivo (como o QML do Quickshell no end4); animações com curvas;
temas por tokens; canvas de shader wgpu embutido na UI; plugins isolados; Windows primeiro; máximo desempenho.

| Candidato | A favor | Contra |
| --- | --- | --- |
| **Slint** | Linguagem declarativa `.slint` com prévia ao vivo; o `slint-interpreter` **carrega arquivos de UI em tempo de execução** (skins do usuário, como o QML do Quickshell); animações e estados nativos; integração com texturas wgpu para o canvas; licença royalty-free para desktop proprietário | Sem dock/tiling pronto (construímos o nosso, que é o coração do produto); integração wgpu ainda marcada como instável |
| **GPUI** (Zed) + gpui-component | O mais rápido (o Zed roda a 120 fps); `DockArea` com tiles e redimensionamento; o Zed prova o modelo de temas JSON + extensões WASM + marketplace | Pré-lançamento (via git, API muda); UI só em código Rust, sem skins em tempo de execução; canvas wgpu próprio é difícil (usa outro renderer) |
| Qt/QML via cxx-qt | É a tecnologia do Quickshell/end4; maduro; ShaderEffect e blur | Dependência C++ grande; ponte Rust↔C++; LGPL |
| Makepad | A UI inteira é shader, com live design | Comunidade e documentação pequenas |
| Dioxus Native (Blitz) | CSS de verdade sem navegador | Experimental |
| egui | Simples (usado pelo Spotifast) | Visual de "ferramenta"; customização visual limitada |

**Recomendação:** **Slint** como base, com o nosso próprio motor de layout em árvore (estilo dwindle do Hyprland) e o
canvas wgpu dos templates. **GPUI** entra como finalista no protótipo comparativo, para medir se a diferença de
desempenho compensa perder as skins em tempo de execução.

**Resultado do protótipo (07/11/2026):** o GPUI venceu em desempenho (144 fps estáveis com ~1/3 da CPU, contra
104–138 fps e um núcleo cheio no Slint). O Slint venceu em skins prontas em tempo de execução. A recomendação passou a
ser **GPUI como base + camada de skin própria**. Os dois testes decisivos passaram: visual sem cópia por uma camada
do DirectComposition (CPU ~14%, 144 fps) e skins KDL recarregadas ao vivo. **Decisão: GPUI.** Detalhes em
[`prototipo-ui-resultados.md`](prototipo-ui-resultados.md).

**Critérios do protótipo** (os dois finalistas, mesmo teste): painéis em árvore arrastáveis a 120 fps; canvas wgpu a
60 fps com UI por cima; skin do usuário carregada e recarregada ao vivo; troca de tema; animação com curva; RAM e tempo
de abertura.

---

## Fontes
- Spotifast: código em github.com/crmne/spotifast (docs `_reference/how-it-connects.md`, `what-spotify-allows.md`, `soloist.md`; `src/`)
- Spotify Soloist: https://developer.spotify.com/documentation/soloist
- Spotify Developer Policy: https://developer.spotify.com/policy
- Spicetify: https://github.com/spicetify/cli (`src/apply/apply.go`, `src/preprocess/preprocess.go`, `globals.d.ts`)
- end4 / illogical-impulse: https://github.com/end-4/dots-hyprland ; end4-pC: https://github.com/pctrade/end4-pC
- SoundCloud: https://developers.soundcloud.com/docs/api/terms-of-use , https://developers.soundcloud.com/blog/api-sign-up-changes , https://developers.soundcloud.com/docs/api/register-app
- ytmusicapi (não oficial): https://ytmusicapi.readthedocs.io/
- Comparativos de GUI em Rust: https://docs.rs/gpui-ce/0.3.0/gpui/_documentation/comparison/index.html , https://users.rust-lang.org/t/which-gui-framwork-should-i-choose/137581
