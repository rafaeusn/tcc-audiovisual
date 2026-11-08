# Plano da v1: Player (Rust + GPUI, Windows)

Planejamento de 08/11/2026 para a primeira versão do app gratuito (Player). Junta tudo o que foi decidido até
agora: [`CONTEXTO-TCC.md`](../CONTEXTO-TCC.md), [`pesquisa-referencias-app.md`](pesquisa-referencias-app.md),
[`prototipo-ui-resultados.md`](prototipo-ui-resultados.md) e [`diretor-calibracao.md`](diretor-calibracao.md).

---

## 0. Resumo

1. **O que é a v1:** o Player para Windows, em Rust + GPUI. Funciona no **Modo Escuta** (qualquer música tocando no PC
   vira visual) e toca **arquivos locais**. Tem a UI modular (arrastar, redimensionar, skins, temas) e a **loja de
   módulos**.
2. **Visual:** um template de verdade, o **Spectral**, portado da prévia WebGL para wgpu. Ele roda só no **diretor
   automático**. O usuário pode trocar as cores reativas por cores próprias, ligar o **modo seguro para
   fotossensibilidade** e acertar a sincronia.
3. **A UI padrão lembra o Spotify** (barra lateral, área central, painel "tocando agora", barra do player), mas o
   centro é o visual, as páginas abrem por cima dele em vidro, e tudo pode ser rearrumado.
4. **A loja já nasce na v1:** temas, skins, layouts, perfis de Escuta (Spotify, YT Music, SoundCloud...) e o primeiro
   módulo com código, o **Last.fm** (scrobble, histórico e estatísticas).
5. **Fica para depois:** o Studio, o modo para surdos, outros templates, o Spotify com biblioteca dentro do app,
   macOS e Linux.

---

## 1. O que aproveitamos de cada referência

### 1.1 Spotifast: a engenharia

Aproveitamos **o motor do carro, não a estrada que ele usa**. A conexão dele com o Spotify (librespot não oficial,
Soloist e Web API) esbarra na Developer Policy justamente no que nos torna únicos: visual sincronizado com a música
(III.6) e análise do áudio (III.13). A engenharia dele, por outro lado, serve inteira:

| Lição do Spotifast | Na nossa v1 |
| --- | --- |
| A UI nunca espera rede nem disco. O trabalho pesado roda em outras threads e conversa por canais | Fase 0: núcleo com threads de áudio, análise e rede; a UI só lê um "retrato" do estado |
| UI otimista (o clique responde na hora e o núcleo confirma depois) | Regra de UX desde a Fase 1 |
| Captura do áudio **só de um processo** (WASAPI process loopback) | Fase 2: "ouvir só o Spotify" ou "só o navegador" |
| O visual lê o som antes do volume (volume zero ainda dança) e compensa o atraso da saída | Fase 4: player local com compensação de latência |
| Visualizador isolado: se travar, o player continua | Erro no template ou na skin mostra aviso e volta ao anterior (já funciona nas skins) |
| Cofre de senhas do Windows (Credential Manager) | Fase 5: sessão do Last.fm |
| Arquivo de configuração gravado de forma atômica (escreve num temporário e troca) | Fase 0: layouts, temas e ajustes nunca corrompem |
| SMTC (a caixinha de mídia do Windows), teclas de mídia, bandeja, instância única | Fases 1, 2 e 4 |
| Economia de energia: 30 fps na bateria, 0 fps minimizado | Fase 3 |
| Atualização com manifesto assinado (Ed25519) | Fase 5 (catálogo da loja) e Fase 6 (atualizações do app) |
| Tradução por arquivos, com pt-BR | Fase 1: pt-BR e inglês |

### 1.2 Spicetify: a customização

| Lição do Spicetify | Na nossa v1 |
| --- | --- |
| Três níveis: cores, estilo, código | **Tema** (tokens em TOML), **skin** (KDL) e **módulo** (código isolado) |
| Classes ofuscadas que quebram a cada atualização (`css-map.json`) | Evitamos desde o início: cada painel e cada parte têm um **nome estável e documentado** (`player-bar`, `player-bar.progress`) |
| Marketplace dentro do app | Loja na Fase 5 |
| `spicetify watch` (recarrega ao editar) | Já existe no protótipo: salvar o arquivo atualiza a tela em menos de 1 s |
| Extensões com acesso total | **Não copiar.** Nossos módulos rodam isolados e pedem permissões (como os apps de celular) |

### 1.3 Hyprland, end4 e end4-pC: a interface

| Lição | Na nossa v1 |
| --- | --- |
| Layout em árvore (tiling) com divisórias arrastáveis | Já existe no protótipo (`layout.rs`, 5 testes) |
| Widgets flutuantes por cima | Já existe (relógio flutuante) |
| Tudo é arquivo de configuração e recarrega na hora | Layout, tema, skin e atalhos são arquivos legíveis em `%APPDATA%` |
| Animações com curvas Bézier nomeadas | Fase 1: curvas com nome (`suave`, `elastica`) configuráveis no tema |
| Paleta gerada da imagem (Material You) | Fase 1: paleta opcional gerada da **capa** que está tocando |
| Pacotes compartilháveis ("rice") | Fase 5: pacote = tema + skin + layout, instalado pela loja |
| Teclado primeiro (`Super+/` lista os atalhos) | Fase 1: paleta de comandos `Ctrl+K` e lista de atalhos `F1` |
| Modo edição | Já existe no protótipo (tecla `E`) |

### 1.4 O que já é nosso

| De onde | O que vai para o app |
| --- | --- |
| `prototypes/ui-bench/core/src/layout.rs` | Motor de layout em árvore (vai quase igual) |
| `prototypes/ui-bench/gpui-proto/src/skin.rs` | Interpretador de skins KDL (cresce com botões, sliders, imagens e listas) |
| `prototypes/ui-bench/gpui-proto/src/underlay.rs` | Camada do visual sem cópia (DirectComposition) |
| `prototypes/ui-bench/core/src/metrics.rs` | Medições de fps, CPU e RAM (viram testes de desempenho) |
| `preview/spectralTD.js` | Template Spectral, portado para WGSL/wgpu |
| `preview/features.js`, `rhythm.js`, `preanalysis.js`, `director.js` | Diretor automático e pré-análise, portados para Rust |
| `tools/*.mjs` | Viram a **referência**: o resultado em Rust tem que bater com o do JS (testes de paridade) |

---

## 2. Escopo da v1

| Entra na v1 | Fica para depois |
| --- | --- |
| Modo Escuta: som do sistema ou de um app só, "tocando agora" pelo SMTC, controles (play/pause/próxima) | Studio (app pago, editor e exportação) |
| Player de arquivos locais com biblioteca, fila e pré-análise | Modo para surdos (pistas visuais do ritmo, vibração) |
| Template Spectral no diretor automático | Outros templates e templates de câmera/corpo |
| Cores reativas ligadas/desligadas, cores próprias, modo seguro, ajuste de sincronia | Controle manual dos parâmetros do template (é do Studio) |
| UI modular: layout, edição, temas, skins, atalhos, paleta de comandos | Plugins que desenham UI própria |
| Loja: temas, skins, layouts, pacotes, perfis de Escuta, módulo Last.fm | Spotify com biblioteca e busca dentro do app (ver 5.4) |
| pt-BR e inglês | Audius e Jamendo (v1.1: são os primeiros candidatos depois da v1) |
| Instalador e atualização automática | Contas, nuvem e sincronização entre PCs |
| | macOS, Linux e celular |

**Sem servidor na v1.** Como o Spotifast: tudo fica no PC do usuário, sem conta e sem telemetria. A loja é um
repositório no GitHub. Isso simplifica o TCC e é um argumento de privacidade.

---

## 3. A UI padrão

### 3.1 "Parecida com o Spotify, mas diferente"

A pessoa reconhece o esqueleto na hora, porque é o mesmo do Spotify. Mas o centro não é uma grade de capas: é o
**visual ao vivo**. As páginas (biblioteca, loja, ajustes) abrem como **folhas de vidro por cima do visual**, que
continua rodando, escurecido, atrás delas.

```
┌────────────────────────────────────────────────────────────────────────────┐
│ ◉  [ ⌕ Buscar ou comando…   Ctrl+K ]                          ⚙   ─  □  ✕  │  barra de título própria
├──────────┬─────────────────────────────────────────────────┬───────────────┤
│ ⌂ Início │                                                 │ TOCANDO AGORA │
│ ♪ Biblio │                                                 │ ┌───────────┐ │
│ ⊞ Loja   │                                                 │ │   capa    │ │
│ ▤ Stats  │          VISUAL (Spectral) ao vivo              │ └───────────┘ │
│──────────│          é o "herói" da tela                    │ Título        │
│ FONTES   │                                                 │ Artista       │
│ ● Escuta │                                                 │ via Spotify ● │
│ ○ Spotify│                               ┌──────────┐      │───────────────│
│ ○ Arquivo│                               │ 21:47    │      │ FILA          │
│ + módulos│                               └──────────┘      │ 1. …          │
│          │                          widget flutuante       │ 2. …          │
├──────────┴─────────────────────────────────────────────────┴───────────────┤
│ ⏮ ▶ ⏭   Título — Artista    ━━━━━━━━○────── 1:32 / 3:40    ◐ Visual  🔊 ━━ │  barra do player
└────────────────────────────────────────────────────────────────────────────┘
```

O que deixa diferente:
- **O visual é o centro.** Na tela cheia (`F`) sobra só ele. Com `H`, a UI some e volta ao mexer o mouse.
- **A cor da interface segue a música.** Um tema opcional gera a paleta da capa que está tocando (estilo Material
  You do end4).
- **Tudo se mexe.** No modo edição (`E`), qualquer painel pode ser arrastado, redimensionado, trocado de lugar ou
  virar flutuante, com a música tocando.
- **Responsiva de verdade.** Janela estreita esconde a barra lateral. Janela pequena vira mini player com visual. Um
  segundo monitor pode ficar só com o visual. Cada tamanho tem o próprio layout (breakpoints, já no protótipo).

### 3.2 Páginas da v1

| Página | Conteúdo |
| --- | --- |
| **Início** | Sem música: "Toque uma música em qualquer app e o visual começa sozinho" e a lista de apps de mídia abertos (pelo SMTC): "Spotify detectado. Ouvir só ele?" Com música: o visual. |
| **Biblioteca** | Pastas de música do usuário, faixas, álbuns e artistas, com busca. Lista virtualizada (milhares de faixas sem travar) |
| **Loja** | Abas Módulos, Temas, Skins, Layouts e Pacotes. Instalar, atualizar e remover com um clique |
| **Estatísticas** | Aparece com o módulo Last.fm: mais ouvidas, artistas, horários |
| **Ajustes** | Aparência (tema, skin, layout), Visual (ver seção 4), Fontes (qual app ouvir), Atalhos, Idioma, Sobre |

### 3.3 Componentes (os blocos de montar)

Cada componente tem um **ID estável** que skins, layouts e módulos usam:

`visual`, `tocando-agora`, `player-bar`, `fila`, `biblioteca`, `fontes`, `relogio`, `medidores` (grave, médio e agudo),
`busca`, `paginas` (onde as páginas abrem).

Existem dois tipos:
- **Nativos** (feitos em Rust): biblioteca, loja, ajustes, campo de busca. São complexos e precisam ser rápidos. A
  skin muda a aparência deles pelas **partes com nome** (`biblioteca.linha`, `biblioteca.linha:tocando`), como o
  `::part()` do CSS.
- **De skin** (feitos em KDL): tocando agora, relógio, medidores, cabeçalhos e molduras. A skin decide tudo: posição
  interna, textos, cores e barras.

### 3.4 Tudo é comando

Cada ação do app tem um nome: `player.tocar-pausar`, `player.proxima`, `visual.tela-cheia`, `layout.editar`,
`skin.proxima`, `pagina.abrir "loja"`. O **mesmo nome** é usado pelo botão da skin, pelo atalho, pela paleta
`Ctrl+K` e pelos módulos. É o que o Hyprland chama de *dispatcher* e o VS Code de *command*.

```kdl
// na skin: o botão chama um comando
button icon="play" on-click="player.tocar-pausar" active="{player.tocando}"
```

```toml
# atalhos.toml
"Espaço" = "player.tocar-pausar"
"F"      = "visual.tela-cheia"
"Ctrl+K" = "paleta.abrir"
```

Analogia: os comandos são os interruptores da casa. A skin decide onde fica cada interruptor e o atalho é um
controle remoto, mas a fiação é uma só.

---

## 4. O template Spectral na v1

### 4.1 O que o usuário controla

O template roda **só no diretor automático**: a música decide forma, movimento, luz e cor. Os ajustes da v1 são:

| Ajuste | O que faz |
| --- | --- |
| **Cores reativas** (padrão: ligado) | Ligado: branco no calmo, azul no agitado, vermelho no intenso, com salto no drop (como na prévia). Desligado: o usuário escolhe **duas cores** (principal e secundária) ou uma paleta pronta, e a intensidade continua mexendo na forma e na luz, mas não muda o tom |
| **Modo seguro** (fotossensibilidade) | Ver 4.2 |
| **Ajuste de sincronia** (avançado) | Atrasa ou adianta o visual em milissegundos, para fones Bluetooth |

Sem parâmetros manuais (velocidade, bloom, partículas): isso é do Studio.

### 4.2 Modo seguro para fotossensibilidade

**Por quê:** visual reativo pisca, e piscadas rápidas podem provocar crises em pessoas com epilepsia fotossensível. A
referência é a WCAG 2.3.1 (no máximo **3 flashes por segundo**) e as regras de TV (ITU-R BT.1702, teste Harding).

**Como funciona:** um "limitador" fica entre o diretor e o template. Funciona como o limitador de velocidade de um
carro: o motorista (diretor) acelera como quiser, mas o carro não passa do limite.

| Regra | Implementação |
| --- | --- |
| Sem flash no drop | O drop vira movimento (explosão de partículas, zoom), não clarão |
| Brilho muda devagar | Brilho, exposição e bloom têm taxa máxima de variação por segundo. Os pulsos de batida mexem na forma, não na luz |
| No máximo 3 mudanças fortes de luz por segundo | Contador de transições de brilho (≥ 10% de luminância) numa janela de 1 s; acima de 3, a variação é achatada |
| Sem flash vermelho saturado | Com o modo seguro, a transição para o vermelho é lenta e menos saturada |
| Menos movimento brusco | Tremor e giro do kick reduzidos |

**Medidor de flashes (vira resultado do TCC):** o app renderiza o visual sem janela, mede a luminância média de cada
quadro na GPU e conta os flashes por segundo. Um teste automático roda o corpus de calibração e confere que, com o
modo seguro ligado, **nenhum segundo passa de 3 flashes**. Fica medido, não prometido.

**Na primeira abertura**, o app pergunta: "Você ou alguém que vai assistir tem sensibilidade a luzes piscando?" Se o
Windows estiver com "efeitos de animação" desligados, o modo seguro já vem ligado.

### 4.3 Port para wgpu

| Prévia (WebGL2) | App (wgpu, WGSL) |
| --- | --- |
| `noise1`/`noise2` em textura 256×256 | Compute shader gera as duas texturas |
| 65.536 pontos com `gl.POINTS` | Pontos instanciados lendo a textura (ou quads de 1 px, para controlar o tamanho) |
| Bloom dual Kawase em cadeia de mips | Mesma cadeia, em passes de render |
| Rastro (feedback) e aberração | Duas texturas alternadas (ping-pong) |
| Camada espacial (kick, seção, médios, agudos) | Mesmos uniforms, vindos do diretor em Rust |

**Pronto quando:** prints lado a lado com a prévia na mesma faixa e no mesmo instante, e as estatísticas do ruído
(mediana e p5–p95, a ferramenta `stats` da prévia) iguais às da tabela de calibração de
[`port-spectral-td.md`](port-spectral-td.md). Meta de GPU: menos de 4 ms por quadro em 1080p na RTX 2060.

---

## 5. A loja de módulos

### 5.1 O que se instala

| Tipo | O que é | Tem código? |
| --- | --- | --- |
| Tema | Tokens de cor, raio, fonte, curvas de animação (`.toml`) | Não |
| Skin | Aparência e conteúdo dos painéis (`.kdl`) | Não |
| Layout | Árvore de painéis e flutuantes por tamanho de janela (`.toml`) | Não |
| Pacote ("rice") | Tema + skin + layout juntos | Não |
| Perfil de Escuta | Ensina o app a reconhecer um serviço: processo, sessão de mídia, ícone, controles | Não |
| Módulo de serviço | Integração com API (Last.fm na v1) | Sim, isolado |

### 5.2 Perfis de Escuta: o Spotify da v1

O módulo "Spotify" da v1 é um **perfil de Escuta**: um arquivo de dados, sem código e sem API do Spotify.

```toml
# modulo.toml
id = "spotify-escuta"
nome = "Spotify"
tipo = "escuta"
versao = "1.0.0"

[escuta]
processos = ["Spotify.exe"]       # captura só o áudio deste app
sessao-de-midia = ["Spotify.exe"] # "tocando agora" e controles pelo SMTC
controles = ["tocar", "pausar", "proxima", "anterior"]
```

Com ele instalado, o usuário:
1. abre o Spotify oficial normalmente;
2. o app detecta, mostra capa, título e artista no "tocando agora", com "via Spotify";
3. o visual ouve **só o Spotify** (notificação do Discord não mexe no visual);
4. play, pausa e próxima na nossa barra do player controlam o Spotify.

Para o usuário, parece integrado. Por dentro, nada usa a plataforma do Spotify, por isso fica dentro das regras. O
mesmo arquivo, com outro processo, serve para **YT Music** (navegador ou app), **SoundCloud**, **Deezer**, **Tidal**
e **Apple Music**. A loja da v1 já sai com esses perfis.

### 5.3 Módulo Last.fm (o primeiro com código)

- **Login** pelo navegador (o fluxo oficial de apps desktop do Last.fm). A sessão fica no cofre do Windows.
- **Scrobble** do que toca no Modo Escuta e no player local. Segue as regras do Last.fm: a faixa precisa tocar mais da
  metade ou 4 minutos.
- **Importar histórico** para a página Estatísticas.

**Módulos com código rodam em WebAssembly (WASM)**, o mesmo modelo das extensões do Zed (o editor que criou o GPUI).
O WASM é como uma "sala com paredes de vidro": o módulo faz o trabalho dele, mas só toca no que o app entrega pela
porta. O manifesto declara as permissões, e o usuário vê a lista antes de instalar:

```toml
id = "lastfm"
tipo = "servico"
permissoes = [
  "rede:ws.audioscrobbler.com",   # só esse endereço
  "credenciais",                  # guardar a sessão no cofre
  "ler:tocando-agora",
  "pagina:estatisticas",
]
```

A chave de API do Last.fm **não vai para o Git**: ela entra no build por variável de ambiente.

### 5.4 Por que o Spotify "completo" fica para depois

Biblioteca, busca e playlists do Spotify dentro do app, como no Spotifast, exigem a Web API ou o Soloist. Usar essa
plataforma junto com visual sincronizado é o que as cláusulas III.6 e III.13 proíbem. Além disso, o modo de
desenvolvimento aceita só 5 contas por app. O caminho:
1. **v1:** perfil de Escuta (5.2), que já entrega o "tocando agora", os controles e o visual.
2. **Depois:** um módulo "Spotify Biblioteca" só **após consulta jurídica**. Mesmo assim, ele seria um módulo à parte
   que pode ser desligado pelo Spotify sem afetar o resto.

### 5.5 Como a loja funciona por dentro

- **Catálogo:** um repositório no GitHub com um `index.json` listando os pacotes (como o repositório de extensões do
  Zed e o Marketplace do Spicetify).
- **Segurança:** o índice é **assinado** (Ed25519) e cada pacote tem hash SHA-256. O app recusa o que não bate.
- **Instalação:** baixa, confere, descompacta em `%APPDATA%\<app>\modulos\<id>\` e ativa sem reiniciar.
- **Na v1, só pacotes nossos (curados).** Envio pela comunidade vem depois, com revisão.
- **Regra da Policy IV.2.3:** nenhuma divulgação do Studio aparece no contexto do módulo Spotify.

---

## 6. Arquitetura

### 6.1 Threads (quem faz o quê)

```
 Áudio (tempo real)          Análise                  Rede/disco (tokio)
 ┌─────────────────┐        ┌───────────────────┐    ┌──────────────────────┐
 │ captura WASAPI  │──sinal─▶ features + ritmo   │    │ loja, Last.fm,       │
 │ (sistema/app)   │        │ diretor ao vivo   │    │ biblioteca (SQLite), │
 │ player local    │        │ pré-análise       │    │ capas, SMTC          │
 │ (cpal+symphonia)│        │ (rayon, cache)    │    └──────────┬───────────┘
 └─────────────────┘        └─────────┬─────────┘               │
                                      │ parâmetros               │ eventos
                                      ▼                          ▼
                     ┌──────────────────────────────────────────────────┐
                     │ UI (GPUI, thread principal): lê um retrato do     │
                     │ estado a cada quadro, nunca espera ninguém        │
                     │   └─ camada do visual (wgpu, DirectComposition)  │
                     └──────────────────────────────────────────────────┘
```

Analogia: é uma cozinha de restaurante. Cada cozinheiro tem a sua bancada (thread), e os pratos passam por um balcão
(canal). O garçom (a UI) nunca entra na cozinha: ele só pega o que está pronto no balcão. Por isso o salão nunca
trava.

### 6.2 Pastas do código

```
app/                          workspace Cargo do app de verdade
├── crates/
│   ├── nucleo/       tipos comuns, comandos, estado, configuração com gravação atômica
│   ├── audio/        captura (sistema e por processo), player local, buffers circulares
│   ├── analise/      features, ritmo, pré-análise, diretor (port do JS) + testes de paridade
│   ├── visual/       wgpu: template Spectral, limitador do modo seguro, medidor de flashes
│   ├── janela/       camada DirectComposition (do protótipo)
│   ├── sessao/       SMTC: "tocando agora" e controles
│   ├── biblioteca/   varredura de pastas, metadados (lofty), SQLite (rusqlite), capas
│   ├── interface/    layout em árvore, skins KDL, temas, animações, paleta de comandos
│   └── loja/         catálogo, assinatura, instalação, permissões, runtime WASM
├── player/           o executável (GPUI): junta tudo
├── modulos/          módulos nossos: lastfm (WASM), perfis de Escuta
└── recursos/         skins, temas e layouts padrão, ícones, traduções
```

O GPUI fica **só** em `interface/` e `player/`. Se a API dele mudar (ele ainda é pré-1.0), o estrago fica contido
nessas duas pastas. A versão é fixada no `Cargo.lock`.

`preview/` e `prototypes/ui-bench/` continuam no repositório como **referência**: a prévia gera os resultados
"gabarito" que o Rust precisa reproduzir.

### 6.3 Arquivos do usuário

```
%APPDATA%\<app>\
├── ajustes.toml         visual, fontes, idioma
├── atalhos.toml
├── layouts\  temas\  skins\     (editáveis à mão, recarregam na hora)
├── modulos\<id>\
├── biblioteca.db
└── cache\timelines\     pré-análise salva por faixa (abrir de novo = instantâneo)
```

---

## 7. Fases

As durações são estimativas. Sugestão: **v1 pronta por volta de junho de 2027**,
com o 2º semestre de 2027 livre para avaliação com usuários e escrita do TCC.

### Fase 0: Fundação (1–2 semanas)
- Workspace `app/` com as pastas da seção 6.2. Mover `layout.rs`, `skin.rs`, `underlay.rs` e `metrics.rs` do protótipo.
- GitHub Actions no Windows: compilar, testar, `clippy` (revisor automático) e `rustfmt` (formatação).
- Logs (`tracing`), pasta de configuração, gravação atômica, instância única.
- **Pronto quando:** a janela abre com o layout do protótipo, o CI passa e o app grava e lê ajustes.

### Fase 1: Casca da UI (4–6 semanas)
- A tela padrão da seção 3: barra de título própria, barra lateral, painel "tocando agora", barra do player, páginas
  em vidro, tema "Noite" padrão + 2 alternativos.
- Skin KDL v2: `button`, `icon`, `image`, `slider`, `scroll`, `input`, ações (`on-click`), estados (`hover`,
  `active`), animações com curva por nome.
- Sistema de comandos, atalhos, paleta `Ctrl+K`, lista de atalhos `F1`.
- Modo edição completo: arrastar, redimensionar, flutuar, salvar por breakpoint, desfazer.
- Paleta gerada da capa (opcional), traduções pt-BR e inglês.
- Visual provisório: o anel do protótipo, que é trocado pelo Spectral na Fase 3.
- **Pronto quando:** dá para usar e rearrumar a interface inteira com dados de exemplo, a 144 fps e com menos de 20%
  de CPU.

### Fase 2: Modo Escuta (3–4 semanas)
- Captura do som do sistema (já existe) e **por processo** (process loopback, Windows 10 2004+).
- SMTC: título, artista, capa, app de origem e controles. Detecção automática na página Início.
- Port do diretor ao vivo: `features.js`, `rhythm.js` e a parte ao vivo do `director.js`.
- Perfis de Escuta lidos de arquivo (ainda sem a loja).
- **Pronto quando:** com o Spotify oficial tocando, o app mostra a faixa, ouve só ele, os controles funcionam e a
  energia ao vivo bate com a do JS no corpus (testes de paridade).

### Fase 3: Spectral em wgpu + ajustes do visual (4–5 semanas)
- Port do template (seção 4.3), tela cheia, segundo monitor.
- Cores reativas ligadas/desligadas, cores próprias, paletas prontas.
- Limitador do modo seguro + medidor de flashes + teste automático no corpus.
- Economia de energia: 30 fps na bateria, 0 fps minimizado ou com a janela coberta.
- **Pronto quando:** paridade visual com a prévia, < 4 ms de GPU em 1080p e o teste de flashes passando.

### Fase 4: Player de arquivos locais (4–5 semanas)
- Biblioteca: escolher pastas, varrer, ler metadados e capas, SQLite, busca, fila.
- Reprodução: decodificação (`symphonia`), saída de áudio (`cpal`), posição exata para a sincronia.
- Port da pré-análise (`preanalysis.js`) em paralelo (`rayon`), com cache em disco por faixa.
- Diretor no modo arquivo, com compensação de latência e ajuste de sincronia.
- Teclas de mídia e SMTC do nosso próprio player.
- **Pronto quando:** pré-análise de uma faixa de 4 min em < 1 s, testes sintéticos com o mesmo acerto do JS (98–100%
  das batidas, ~5 ms) e 10 mil faixas na biblioteca sem travar a rolagem.

### Fase 5: Loja e Last.fm (4–6 semanas)
- Catálogo assinado, instalar, atualizar, remover, tela de permissões.
- Primeiros pacotes: 3 temas, 2 skins, 2 layouts, perfis de Escuta (Spotify, YT Music, SoundCloud, Deezer, Tidal,
  Apple Music).
- Runtime WASM (`wasmtime`, interface WIT) e o módulo Last.fm: login, scrobble, histórico e página Estatísticas.
- **Pronto quando:** instalar o pacote "Neon" troca tema, skin e layout sem reiniciar; o Last.fm faz scrobble de uma
  faixa ouvida pelo Spotify em Modo Escuta.

### Fase 6: Acabamento e avaliação (3–4 semanas)
- Instalador, atualização automática assinada, primeira abertura (pergunta do modo seguro, escolha de tema).
- Medições finais contra o app oficial do Spotify e o Spotifast: abertura, RAM, CPU.
- Preparação do teste com usuários: SUS (questionário de usabilidade) e notas de "sincronia" e "imersão" por trecho
  (validação do diretor, pendente em [`diretor-calibracao.md`](diretor-calibracao.md)).

---

## 8. Metas de desempenho e testes

| Meta | Valor | Como medir |
| --- | --- | --- |
| Abrir até o 1º quadro | < 1 s (protótipo: 0,72 s) | `metrics.rs` |
| RAM em uso | < 250 MB (protótipo: 177 MB) | `metrics.rs` |
| UI | 144 fps com < 20% de CPU | `--bench` |
| Visual Spectral | < 4 ms de GPU em 1080p (RTX 2060), < 8 ms em placa integrada | timestamp queries do wgpu |
| Pré-análise de 4 min | < 1 s (JS: 2–3 s) | teste |
| Atraso som → imagem | < 30 ms no arquivo, < 60 ms no Escuta | testes sintéticos |
| Minimizado | 0 fps | `--bench` |
| Modo seguro | ≤ 3 flashes/s em todo o corpus | medidor de flashes |

**Testes:**
- **Unidade:** layout, skin, comandos, análise.
- **Paridade JS × Rust:** a prévia gera um "gabarito" em JSON (energia, batidas, drops) para o corpus e os testes
  sintéticos. O Rust tem que bater dentro de uma tolerância.
- **Desempenho:** os benchmarks do protótipo viram um comando fixo, com os resultados salvos em `bench-results/` a
  cada fase.

---

## 9. Regras que continuam valendo

- Spotify, SoundCloud e YT Music entram pelo **Modo Escuta**, nunca pela plataforma deles, quando há visual.
- Nunca baixar áudio do YouTube ou do Spotify (yt-dlp etc.).
- Não versionar MP3, chaves nem segredos.
- Nenhuma divulgação do Studio dentro do contexto do módulo Spotify.
- Módulos de código sempre isolados e com permissões declaradas.
- Antes de cobrar o Studio: consulta jurídica (Modo Escuta e termos de cada serviço).

---

## 10. Decisões em aberto

| Decisão | Recomendação |
| --- | --- |
| Nome do app (vai para a pasta de configuração, o instalador e a loja) | Escolher antes da Fase 5. Até lá, nome provisório no código |
| Spotify na v1 | Perfil de Escuta (5.2). O "Spotify Biblioteca" só depois da consulta jurídica |
| Começar pela UI ou pelo visual | **Pela UI** (Fase 1), como você pediu. O visual provisório segura até a Fase 3 |
| Visual da tela padrão | Mockup feito em 08/11/2026: [`design-ui-v1.md`](design-ui-v1.md) (aguardando aprovação) |

---

## 11. O que dá para começar agora

1. **Mockup da tela padrão** (seção 3), para você aprovar cores, proporções e o vidro sobre o visual antes de virar
   código.
2. **Fase 0:** criar `app/`, mover os módulos do protótipo e ligar o CI.
3. **Fase 1, primeira parte:** a casca com barra lateral, "tocando agora" e barra do player com dados de exemplo,
   mais o sistema de comandos e os botões na skin KDL.
