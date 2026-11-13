# Player (app em Rust)

Visualizador de música em tempo real. Ele funciona de dois jeitos: no **Modo Escuta**, analisa o som que o PC
está tocando (qualquer app) e move o visual; e como **player**, toca as músicas das pastas do computador.
Windows primeiro.

## Rodar

```powershell
cd app
cargo run -p player            # desenvolvimento
cargo run -p player --release  # como o usuário final vai usar
cargo test --workspace         # 165 testes (3 deles desenham os visuais na placa de vídeo)
cargo clippy --workspace --all-targets
```

Requisitos: Rust 1.88+, Windows 10/11 com DirectX 12. O `fxc.exe` do Windows SDK é usado pelo GPUI no build
(caminho em `.cargo/config.toml`).

Se o Windows bloquear o executável ou uma DLL do build ("Uma política de Controle de Aplicativo bloqueou este
arquivo", erro 4551), é o Controle Inteligente de Aplicativos avaliando um arquivo recém-compilado. Costuma passar
ao tentar de novo; para uma DLL de build, `cargo clean -p <pacote>` e compilar de novo.

## O que tem

| Parte | Onde | O que faz |
| --- | --- | --- |
| `nucleo` | `crates/nucleo` | Erros, pastas do usuário, ajustes (TOML, gravação atômica), comandos, registro em arquivo |
| `audio` | `crates/audio` | Captura do som do sistema (WASAPI loopback via `cpal`) ou de um app só (process loopback), num anel sem trava; volume do sistema |
| `analise` | `crates/analise` | FFT, descritores, ritmo (BPM, batida) e o diretor automático (port de `preview/*.js`) |
| `sessao` | `crates/sessao` | "Tocando agora" e controles de mídia do Windows (SMTC), com a capa |
| `historico` | `crates/historico` | O que foi ouvido, num SQLite local: execuções, estatísticas e as letras já buscadas |
| `letras` | `crates/letras` | Leitura do formato LRC e busca de letras no LRCLIB (aberto, sem chave) |
| `modulos` | `crates/modulos` | O formato dos módulos da Loja: manifesto, ações pedidas, aceite, lista de bloqueio, instalados |
| `tocador` | `crates/tocador` | Toca arquivos: decodifica (MP3, FLAC, WAV, OGG, M4A), converte a taxa e entrega à saída de som por uma fila sem trava |
| `biblioteca` | `crates/biblioteca` | As músicas do computador: varre as pastas, lê as etiquetas e guarda num acervo SQLite com busca |
| `servicos` | `crates/servicos` | Last.fm (autorização e scrobble), Discord (cartão "ouvindo"), a regra de quando a música conta e o cofre da conta |
| `integracao` | `crates/integracao` | O app dentro do Windows: botões na miniatura da barra de tarefas, ícone na bandeja, janela mini |
| `visual` | `crates/visual` | Os três visuais em wgpu: Spectral (262 mil partículas), Fluxo e Barras, mais o modo seguro |
| `janela` | `crates/janela` | Camada do DirectComposition por baixo da interface, onde o visual é desenhado sem cópia |
| `player` | `player` | A interface em GPUI: arranjo responsivo, temas, skins, painel, dock, Loja, Biblioteca, barra de comandos |

Linhas de execução: `escuta` (captura + análise a 60 Hz), `sessao` (SMTC), `gravador` (salva os ajustes),
`historico` (soma o que toca e grava no banco), `letras` (busca a letra), `lastfm` e `discord` (os módulos de
serviço), `tocador` (decodifica e alimenta a saída de som), `biblioteca` (varre as pastas e responde às buscas)
e a da interface, que só lê o resultado
mais recente das outras e nunca espera por elas.

### Interface

- **Painel em módulos**: Ritmo (BPM em matriz de pontos), Energia, Estrutura da faixa (nível de agora e "desde
  quando"), Letra, Nesta sessão, Visual e Relógio. No **modo edição** (`E`) dá para arrastar um módulo sobre outro para
  trocar de lugar, tirar, adicionar e pôr o painel do outro lado. A ordem fica salva.
- **Letra** (módulo opcional, fora do painel por padrão): a linha de agora, a anterior e as próximas, rolando com
  a música. A letra vem do [LRCLIB](https://lrclib.net) e fica guardada no banco local. A busca usa o título, o
  artista, o álbum e a duração da faixa, e só acontece enquanto o módulo está no painel. Precisa que o app de
  origem informe a posição da música; os botões − e + acertam a sincronia em passos de meio segundo. Quando a
  busca não acha (ou acha a música errada), a lupa abre a busca manual: digita-se a música, escolhe-se um
  resultado e ele fica guardado como a letra daquela faixa.
- **O que o Modo Escuta ouve** (Ajustes): "Todo o PC" (o padrão) ou um app só, escolhido entre os que estão
  com som aberto. Com um app escolhido, o visual ouve só ele (captura por processo do Windows, com os processos
  que o app abre) e o "tocando agora" segue só ele: um anúncio no navegador não mexe em nada de quem escolheu o
  Spotify. Se o app fecha, o topo mostra "Esperando..." e a captura volta sozinha quando ele reabre. Tocando uma
  música local com um app escolhido, a escuta ouve o próprio Player. Fica em `escuta-app` no `ajustes.toml`.
  Teste manual: `cargo run -p audio --example ouvir_app -- opera.exe spotify.exe` abre e fecha a captura de cada
  app e mostra o nível do som.
- **Músicas** (aba da Biblioteca): escolha pastas ("Adicionar pasta") e o app lê as músicas delas e das pastas
  de dentro. Os arquivos ficam onde estão; o acervo (`biblioteca.db`) só aponta para eles. Cada pasta vira uma
  categoria, como uma playlist, com a contagem ao lado; "Todas" junta as pastas sem repetir música (mesmo título,
  artista e duração contam como a mesma, inclusive cópias e pastas uma dentro da outra). O botão de tirar aparece
  na pasta escolhida. A busca acha por título, artista ou álbum, sem acento nem maiúscula, dentro da categoria
  escolhida; aceita acentos com tecla morta e colar (`Ctrl V`). Clicar numa música toca, e as seguintes da lista
  viram a fila. O dock controla (tocar/pausar, anterior, próxima; "anterior" volta ao começo depois de 3 segundos) e um
  clique no mapa de energia vai para aquele ponto. "Atualizar" varre de novo: o que é novo entra, o que sumiu
  do disco sai.
- **A música local aparece no Windows** como a de qualquer app (controle de mídia do sistema): as teclas de mídia
  do teclado funcionam, e o histórico, a letra, o Last.fm e o Discord a enxergam sem código próprio. O visual
  reage porque ouve o que sai do computador.
- **Histórico** (aba da Biblioteca): o que tocou com o Player aberto, por período (hoje, 7 dias, 30 dias, tudo): tempo
  ouvido e comparação com o período anterior, músicas, artistas, BPM e energia médios, dias seguidos com música,
  horas do dia, mais ouvidos e as últimas músicas. Uma música entra depois de 30 segundos ouvidos (ou da metade,
  se for curta); pausa e silêncio não contam. Funciona com a janela minimizada.
- **Barra de tarefas**: ao passar o mouse no ícone do app, a miniatura tem os botões anterior, tocar/pausar e
  próxima.
- **Bandeja**: um ícone ao lado do relógio mostra o nome da música; clicar traz a janela, e o botão direito abre
  o menu (mostrar, tocar ou pausar, próxima, anterior, sair). Com "Continuar na bandeja" ligado nos Ajustes
  (desligado por padrão), fechar a janela só a esconde: o app segue ouvindo e contando o histórico.
- **Janela mini** (`N`, ou o botão no dock): a janela encolhe para o canto da tela e fica por cima das outras,
  só com o visual e a faixa. Com o mouse em cima aparecem os controles (anterior, tocar/pausar, próxima) no
  canto de baixo à direita, o nome do visual (clicar troca) e o botão de voltar ao tamanho de antes; ao tirar o
  mouse eles somem na hora. Em janela estreita os controles sobem para o meio e o título fica com a largura
  toda; em janela baixa o artista e a origem saem. Arrastar em qualquer outro ponto move a janela; `N` ou `Esc`
  também voltam, e `F` sai direto para a janela maximizada.
- **Dicas nos ícones**: parar o mouse num botão de ícone (dock, barra lateral, topo, visual, Letra) mostra o
  nome curto do que ele faz e o atalho, quando há. O texto acompanha o estado ("Tocar" vira "Pausar"). A dica
  fica acima do botão, centralizada nele, para não cobrir os vizinhos; desce para baixo só quando não há espaço
  em cima (os botões da janela) e vai para o lado na barra lateral. Não aparece na janela mini. Dá para
  desligar em Ajustes, "Dicas dos botões" (`ajuda.dicas` na barra de comandos).
- **Relatório de diagnóstico** (Ajustes, em Sobre): copia versão, sistema, placa de vídeo, saída de som, ajustes
  e o fim do registro, com o nome do usuário do Windows apagado.
- **Barra de comandos** (`Ctrl K`): busca e executa qualquer comando pelo nome, sem acento nem maiúscula.
- **Loja**: temas, skins, layouts e pacotes se aplicam com um clique; os módulos do painel entram e saem por lá.
- **Módulos com ficha**: quem faz mais que desenhar (usa a internet, entra numa conta) se descreve num manifesto
  (`recursos/modulos/*.toml`): identidade e a lista de ações que pede, de um catálogo fechado, cada uma com o
  motivo escrito pelo autor. A ficha na Loja mostra isso, destaca as ações críticas e só libera "Instalar"
  depois de marcada a caixa "Li o que este módulo faz". O que foi aceito fica em `modulos.toml`; uma versão nova
  só se aplica sozinha se não pedir nada além disso (`Aceite::mudancas`). A Letra é o primeiro módulo assim:
  não busca nada antes de instalada.
- **Last.fm** (módulo): depois de instalado, "Conectar conta" na ficha abre a página do Last.fm no navegador para
  autorizar; o app nunca vê a senha. A autorização fica em `lastfm.cofre`, cifrada pelo Windows para o usuário
  atual. O app avisa quando a música começa e faz o scrobble quando ela passa da metade ou de 4 minutos (faixas
  de até 30 s não contam; pausa e silêncio também não). Sem internet, até 50 scrobbles esperam e são reenviados
  a cada minuto. Sem artista (título vindo do navegador) a faixa não é enviada. Se o app de origem já envia ao
  Last.fm, a música conta duas vezes: desligue num dos dois.
- **Discord** (módulo): mostra a música no perfil ("Ouvindo"), com a barra de progresso, pelo canal local do
  Discord aberto neste computador. Sem login e sem rede. O cartão some quando a música pausa.
- **Chaves dos serviços**: não ficam no código. O Last.fm precisa de `LASTFM_API_KEY` e `LASTFM_API_SECRET`
  (criadas em <https://www.last.fm/api/account/create>) e o Discord de `DISCORD_APP_ID` (o "Application ID" de
  um aplicativo criado em <https://discord.com/developers/applications>; o nome dele é o que aparece no cartão).
  Valem as variáveis de ambiente presentes ao abrir o app; se não houver, as que existiam na compilação. Sem
  elas a ficha do módulo avisa que ele não está configurado.
- **Skin imersiva** (`I`): o visual ocupa a janela inteira e o painel, o dock e o topo viram vidro por cima dele.
- **Volume** no dock: é o volume do sistema. O Modo Escuta analisa o som antes dele, então baixar o volume (ou
  deixar no mudo) não enfraquece o visual.
- **Cores reativas**: o dock e o alto do painel acompanham a cor da energia. No tema Mono tudo fica sem cor,
  inclusive as capas.
- **Micro-interações** (artefato "Micro-interações do Player"): botões afundam ao pressionar, o play vira pause
  ponto a ponto, marcadores de aba deslizam, os dígitos do BPM rolam, o título entra a cada troca de faixa, o
  logo e o ponto da energia pulsam uma vez por compasso (nunca no modo seguro), o volume estica nas pontas.
- **Fontes**: Instrument Sans (texto e títulos estreitos), Doto (números) e Fragment Mono, embutidas. Ficam em
  `recursos/fontes`, geradas por `gerar.py` a partir das fontes do Google Fonts (licença OFL).

Para mexer nas animações: `player/src/anim.rs` guarda os valores que perseguem um alvo, e cada tela pede o valor
do quadro com `anim.seguir(chave, alvo, tempo)`.

## Comportamento em falhas

- Sem saída de áudio, ou saída trocada/desconectada: avisa no topo e tenta de novo a cada 2 segundos.
- Sem placa de vídeo compatível, ou driver reiniciado: a interface abre e explica no lugar do visual; tenta recriar.
  A skin imersiva volta à padrão enquanto o visual não existe.
- Ajustes corrompidos: o arquivo é guardado como `.corrompido` e o app abre com os padrões.
- Sem pasta de dados: o app funciona, só não lembra os ajustes.
- Sem controle de volume no sistema: a barra de volume fica apagada.
- Sem pasta de dados ou com o banco estragado: o histórico fica desligado (ou recomeça vazio, guardando o
  arquivo antigo como `.corrompido`) e a Biblioteca avisa.
- Sem internet: o módulo Letra avisa e tenta de novo a cada 20 segundos. "Não tem letra" vale por um dia.
- Sem barra de tarefas ou bandeja (o Windows recusou a ligação): o app abre sem os botões, sem o ícone e sem a
  janela mini, e fechar a janela sempre encerra.
- Só uma cópia do app roda por vez. A variável de ambiente `TCCPLAYER_DADOS=<pasta>` troca a pasta de dados e
  deixa uma segunda cópia rodar ao lado (para testar sem tocar nos ajustes e no histórico de sempre). Para
  testar o player: `TCCPLAYER_MUDO=1` toca sem som e `TCCPLAYER_PASTA=<pasta>` já põe uma pasta na biblioteca.

Dados do usuário ficam em `%APPDATA%\TccPlayer` (`ajustes.toml`, `modulos.toml`, `historico.db`, `biblioteca.db`, `lastfm.cofre` e `registros\player.log`). Nenhum
áudio é gravado nem enviado para fora. O histórico fica só no computador. A única coisa que sai é a busca da
letra (título, artista, álbum e duração), e só com o módulo Letra no painel.

## Atalhos

Espaço tocar/pausar · Ctrl ←/→ faixa anterior/próxima · Ctrl ↑/↓ volume · M mudo · F tela cheia · P painel ·
Ctrl K comandos · Ctrl , ajustes · V próximo visual · 1/2/3 escolher o visual · T próximo tema · I imersivo ·
E editar o layout · N janela mini · Esc fechar/voltar. Cada atalho chama um comando com nome (`player.tocar-pausar`,
`layout.editar`…), o mesmo que os botões e a barra de comandos usam. A lista inteira está em
`crates/nucleo/src/comando.rs`.

## Ainda não tem

Captura por app (hoje é o som do sistema inteiro), no player: tela da fila, tocar sem intervalo entre faixas,
normalização de volume, equalizador, crossfade, capa da música local no controle do Windows, playlists e
visões por álbum e artista; letra palavra por palavra, ajuste de sincronia da letra lembrado por música, retrospectiva do ano, download de módulos
pela Loja (catálogo assinado), skins carregadas de arquivo, ir a um ponto da música pelo mapa de energia (e a
lupa ao passar o mouse), arrastar o visual para trocar de lado (há o botão "Trocar de lado"), medidor de flashes,
ajuste de sincronia para fone Bluetooth e CI. A Biblioteca abre uma folha dizendo o que vem.
