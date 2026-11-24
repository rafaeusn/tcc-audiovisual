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
| `historico` | `crates/historico` | O que foi ouvido, num SQLite local: execuções, estatísticas, as letras já buscadas (com a fonte), o acerto de sincronia de cada música e as traduções |
| `letras` | `crates/letras` | LRC (também com o tempo de cada palavra), TTML, as fontes em cadeia (Better Lyrics, Unison, LRCLIB), a tradução pela Unison e as letras latinas ([F4](../docs/f4-letras.md)) |
| `modulos` | `crates/modulos` | O formato dos módulos da Loja: manifesto, ações pedidas, aceite, lista de bloqueio, instalados |
| `tocador` | `crates/tocador` | Toca arquivos e endereços na internet (baixados aos pedaços, com pulo para qualquer ponto): decodifica (MP3, FLAC, WAV, OGG, M4A), mistura (sem intervalo, crossfade, ganho, equalizador, limitador) e entrega à saída de som por uma fila sem trava |
| `provedores` | `crates/provedores` | Os provedores de música (YouTube Music, Spotify... como programas à parte): formato comum das faixas, protocolo, casador entre serviços e o registro dos instalados. Ainda sem tela no app ([F1](../docs/f1-provedores.md)) |
| `preanalise` | `crates/preanalise` | Pré-análise das músicas do computador: o motor da análise roda sobre o arquivo inteiro (energia, trechos, drops, BPM) e mede a sonoridade (LUFS, ITU-R BS.1770); cache em `analises.db` |
| `biblioteca` | `crates/biblioteca` | As músicas do computador: varre as pastas, lê as etiquetas (com ReplayGain) e guarda num acervo SQLite com busca, álbuns, artistas e playlists |
| `servicos` | `crates/servicos` | Last.fm (autorização e scrobble), Discord (cartão "ouvindo"), a regra de quando a música conta e o cofre da conta |
| `integracao` | `crates/integracao` | O app dentro do Windows: botões na miniatura da barra de tarefas, ícone na bandeja, janela mini, a janela da Ilha |
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
  a música numa mola (passa de leve e assenta). A linha de agora enche como num karaokê, letra por letra, com uma
  borda macia; quando o arquivo traz o tempo de cada palavra (o "LRC com palavras"), enche palavra por palavra no
  tempo dela. As outras linhas apagam e diminuem com a distância, e uma pausa vira "· · ·" enchendo até a próxima
  linha. A letra vem de três fontes, perguntadas em ordem: Better Lyrics
  (muita letra palavra por palavra), Unison (a base aberta da Better Lyrics, que acha as faixas do YouTube Music
  pelo vídeo) e [LRCLIB](https://lrclib.net). A primeira com a letra sincronizada vence; com "Preferir palavra
  por palavra", a cadeia continua até achar uma assim. O rodapé diz de onde veio a letra. Ela fica guardada no
  banco local; uma letra guardada antes da F4 (só do LRCLIB) é trocada uma vez se outra fonte tiver uma melhor.
  A busca usa o título, o artista, o álbum, a duração e (na faixa do YouTube Music) o id do vídeo, e só acontece
  enquanto o módulo está no painel, na Ilha ou na letra em tela cheia. Precisa que o app de origem informe a
  posição da música; os botões − e + acertam a sincronia em passos de meio segundo, e o acerto fica guardado
  para a música. Quando a busca não acha (ou acha a música errada), a lupa abre a busca manual: digita-se a
  música, escolhe-se um resultado e ele fica guardado como a letra daquela faixa.
- **Tradução e letras latinas** (Configurações › Letras): com "Traduzir a letra", cada linha ganha a tradução
  embaixo (português, inglês, espanhol, francês ou japonês), feita pela Unison e guardada por música e idioma.
  "Letras latinas" mostra embaixo a letra de outra escrita (coreano, japonês, chinês, cirílico, grego...) em
  letras latinas, feita no próprio computador. Vale no módulo, na Ilha e na tela cheia.
- **Letra em tela cheia** (`Y`, o botão de expandir no módulo Letra ou a barra de comandos): o visual ocupa a
  janela inteira, escurecido, e a letra rola grande por cima, a linha de agora enchendo palavra por palavra, com
  a tradução embaixo, a fonte e o acerto de sincronia no rodapé. O topo arrasta a janela; Esc fecha.
- **Configurações › Serviços**: a conta, por onde as músicas de cada serviço tocam agora, o motivo se o áudio do Spotify
  parou nesta abertura, e o Soloist do Spotify (chave, ligar, parear, desligar).
- **Configurações › Letras**: a ordem das fontes (setas), ligar e desligar cada uma (uma fica sempre ligada),
  preferir palavra por palavra, traduzir (e o idioma) e letras latinas.
- **Espaços e navegação** (F2): a folha das páginas tem, no topo, voltar e avançar (também `Alt ←`/`Alt →` e os
  botões laterais do mouse) e os espaços: **Tudo**, **Neste computador** e um por serviço ligado, com a logo e a cor
  que o provedor manda. No espaço de um serviço a folha se tinge com a cor dele, numa transição curta, e abre em
  "Para você" (as prateleiras do serviço: no YouTube Music, lançamentos, clipes novos, artistas em alta e paradas).
  Cada espaço lembra as páginas dele; `Ctrl Tab` vai para o próximo e `Ctrl 1` a `Ctrl 9` abrem o da posição (1 é
  Tudo, 2 o computador, depois os serviços). O espaço aberto fica guardado. `C` curte a faixa de serviço que toca, e
  "Fixar na lateral o que está aberto" (na barra de comandos) fixa o álbum, a playlist, o artista ou a faixa da página.
- **Spotify** (F6): o espaço do Spotify vem com o app. Sem entrar, ele mostra o cartão "Entre na sua conta
  do Spotify": o botão abre o navegador, quem entra é você, e a senha não passa pelo Player. Depois disso o Player entra
  sozinho (a credencial fica cifrada pelo Windows) e a Loja › Provedores mostra "Conectado como … · Premium" com Sair. Se
  outro programa usa a porta do login (o Spotifast aberto), o aviso diz qual. Para você, Busca, álbuns, artistas,
  playlists e a Biblioteca da conta (com as Músicas Curtidas) vêm da sua conta, na página Biblioteca e na lateral da
  Estante, junto com o que você salvou no Player. As músicas tocam **com o som do próprio Spotify**, dentro do Player: a
  pré-análise, a troca sem intervalo, o crossfade e o equalizador valem como nas músicas do YouTube Music. Se o Spotify
  recusar o áudio desta conta (por exemplo, uma conta sem Premium), o Player avisa uma vez e segue sozinho: primeiro tenta
  o **Soloist** de Configurações › Serviços (o player oficial, baixado do Spotify, que pede Premium, uma chave pessoal sua e
  parear uma vez) e, sem ele, o YouTube Music (a mesma música, escolhida por título, artista e duração). Configurações ›
  Serviços diz por onde as músicas tocam agora e o motivo se o áudio do Spotify tiver parado. O Spotify toca num lugar por
  vez: tocar aqui para o outro aparelho ([F6](../docs/f6-spotify.md)).
- **Páginas dos serviços**: "Para você" mostra as prateleiras em cartões com capa (as capas baixam em segundo plano e
  ficam guardadas na pasta de dados, em `capas`); clicar numa faixa toca a fileira a partir dela, clicar num álbum,
  playlist ou artista abre a página dele, que entra deslizando. Álbum e playlist: capa, título, o artista como link,
  Tocar, Aleatório, curtir e a lista das faixas (a seguir e fila no hover, coração em cada uma). Artista: populares
  (com "Ver mais") e os álbuns. Enquanto o serviço não responde, a página mostra um esqueleto; erro vira frase com
  "Tentar de novo". **Biblioteca do serviço**: o que foi curtido e salvo dele, com filtro por tipo e fixar (o alfinete
  aparece sobre a capa). No topo da folha, as abas do espaço: **Para você**, **Busca** e **Biblioteca**.
- **Layout Estante** (F2; Configurações › Aparência › Layout, a aba Layouts da Loja ou `Ctrl L`): a **lateral** com os
  espaços (a marca de cada um; o aberto com o anel na cor dele), as páginas do espaço (Para você, Busca, Biblioteca) e
  o que foi salvo nele, os fixados primeiro (no computador, as playlists só de músicas locais); Loja, Configurações,
  personalizar e trocar o layout no pé. No meio, a **página**, com voltar, avançar e o título. À direita, o trilho
  **Tocando**: o **visual ao vivo** no alto, com o nome do visual e a faixa, e os módulos do painel embaixo. Clicar no
  visual (ou `X`) faz ele crescer até a janela inteira, com o título grande; `Esc` ou o botão encolher voltam para a
  mesma página. Em janela média a lateral vira uma barra de ícones e o Tocando abre ao lado da página (`P` ou o botão
  do dock); em janela estreita o visual vira uma faixa acima do dock e as páginas vão para a navegação de baixo. O
  Palco (o layout de sempre) continua o padrão.
- **Busca** (F2): o campo (o mesmo texto da Biblioteca) e o que os serviços acharam: o melhor resultado em destaque,
  as faixas, e prateleiras de álbuns, artistas e playlists; chips filtram por tipo (faixas em lista, o resto em
  grade). No espaço de um serviço, só ele; em Tudo, cada serviço no seu bloco e quantas músicas do computador batem.
  Cada serviço aparece assim que responde; resultado de um texto antigo fica apagado até o novo chegar.
- **Página da faixa**: o título de uma faixa (nas listas e no melhor resultado) abre a página dela: capa, artistas e
  álbum como links, Tocar (Pausar, se é ela que toca), A seguir, Na fila e curtir. Abas **Letra** (a letra da
  faixa que toca, com a tela cheia; com o módulo Letra à vista no painel, ela fica lá), **Mapa** (a pré-análise:
  energia por trecho calma, agitada ou intensa, drops, a posição de agora, BPM, tom e duração) e **Parecidas** (só
  com provedor que tenha rádio).
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
  escolhida; aceita acentos com tecla morta e colar (`Ctrl V`). Clicar numa música toca, e a lista inteira vira
  a fila ("Tocar tudo" começa do início). O dock controla (tocar/pausar, anterior, próxima; "anterior" volta ao
  começo depois de 3 segundos) e um clique no mapa de energia vai para aquele ponto. "Atualizar" varre de novo:
  o que é novo entra, o que sumiu do disco sai.
- **Álbuns e artistas**: o seletor Músicas | Álbuns | Artistas agrupa a categoria escolhida pelas etiquetas;
  escolher um álbum ou artista filtra a lista, com um chip para tirar o filtro.
- **Playlists**: "+ Playlist" cria uma (o nome aceita acentos e colar). Passando o mouse numa música aparecem
  três botões: tocar a seguir, pôr no fim da fila e pôr numa playlist (a janela lista as playlists e cria uma
  nova). Cada playlist vira uma categoria, na ordem em que as músicas entraram; dentro dela, o X da linha tira a
  música. Apagar pede confirmação ("Apagar?"). Ficam no `biblioteca.db` (tabelas `playlists` e
  `playlist_itens`), guardando o caminho dos arquivos.
- **Fila** (aba da Biblioteca, `Q`, ou o botão no dock; abre no espaço em que você está, e num serviço a Biblioteca
  mostra só Fila e Histórico): o que já tocou (esmaecido), a de agora e o que vem
  depois. Clicar pula para a música; com o mouse em cima, subir, descer e tirar. "Limpar a seguir" deixa só a de
  agora. Tirar uma música, limpar a fila, tirar uma música de playlist, tirar uma pasta ou apagar uma playlist
  mostram um aviso com **Desfazer** (6 segundos; `Ctrl Z` também vale). **Aleatório** (`S`) embaralha o resto
  mantendo a de agora, e desligar volta à ordem original.
  **Repetir** (`R`) passa por não, a fila e a música. Os dois ficam no dock quando uma música local toca e são
  lembrados nos ajustes.
- **Reprodução** (Ajustes, "Músicas do computador"): a próxima música já fica aberta e entra **sem intervalo**
  (discos ao vivo e mixados ficam inteiros). **Crossfade** de 2 a 12 s, com curvas de mesma potência (só quando
  o arquivo informa a duração). **Volume igualado** pelo ReplayGain das etiquetas (`REPLAYGAIN_TRACK_GAIN` e
  `REPLAYGAIN_TRACK_PEAK`), sem passar do pico; arquivo sem ReplayGain toca como está. **Equalizador** de 10
  bandas (31 Hz a 16 kHz, ±12 dB): presets, ou clique e arraste nas barras. No fim um limitador suave impede que
  ganho, equalizador e crossfade estourem. Valem só para as músicas do computador: o som dos outros apps não
  passa pelo tocador.
- **Sensibilidade do Modo Escuta**: o som entra no nível em que o diretor foi calibrado, e um ganho automático
  sobe fontes baixas (Spotify e YouTube igualam o volume em -14 LUFS; o volume do app baixa mais) até 12 dB,
  sem mexer em música já alta nem em silêncio. A fase da batida segue os bumbos.
- **De onde vem o "tocando agora"**: a tela mostra quem está tocando. Pausou a música local e deu play no
  Spotify, a tela e os controles (dock, teclas do app, bandeja e botões da barra de tarefas) passam para o
  Spotify; voltou a tocar a local, voltam para ela. Com as duas tocando, vale a que começou por último; com as
  duas paradas, a que tocou por último. A sessão que o próprio Player publica no Windows não conta como "outro
  app". Do lado do Windows, quando o app "atual" está parado e outro está tocando, o Player acompanha o que toca.
- **Só mídia de verdade**: apps que abrem uma sessão de mídia no Windows sem tocar música (o Riot Client, por
  exemplo: o título é o nome do app, sem artista, álbum nem duração) são ignorados na tela, no histórico, no
  Last.fm e no Discord. "Nesta sessão" só registra uma música depois de 4 segundos tocando com som.
- **Saída de som trocada** (fone desligado, outra caixa como padrão): a música local continua na saída nova, do
  mesmo ponto e no mesmo estado (tocando ou pausada).
- **Pasta fora do ar** (pendrive ou HD externo desligado): "Atualizar" pula a pasta e mantém o que já estava
  guardado, em vez de apagar as músicas dela.
- **Com a janela escondida ou minimizada**, o player local continua atendido (a fila anda, as teclas de mídia e a
  bandeja respondem): um relógio de 200 ms faz o que antes só acontecia no desenho.
- **Pré-análise** (músicas do computador): antes de tocar, o app analisa o arquivo inteiro com a receita da prévia
  (batidas por programação dinâmica, tempo forte do compasso, clareza do pulso, trechos pela novidade, energia
  calibrada no corpus, drops no ataque exato, tom) e guarda em `analises.db` (refaz se o arquivo mudar). Durante a
  música, o visual é conduzido por essa linha do tempo: o pump cai em cada batida, mais forte no tempo forte; o
  drop, no instante dele; a subida começa até 16 s antes; o módulo Ritmo acende o tempo certo do compasso e mostra
  o tom (com a notação Camelot). A música de agora e as duas seguintes vêm
  primeiro; o resto da biblioteca é analisado aos poucos, em prioridade baixa (Ajustes, "Analisar a biblioteca",
  com o andamento). Com a análise pronta: o mapa de energia do dock aparece inteiro desde o começo; a "Estrutura
  da faixa" mostra a música toda (o que tocou, o de agora, o que vem), marca os drops e diz quanto falta para o
  próximo; o BPM fica firme; e o visual se prepara nos 4 segundos antes do drop e acerta o momento dele (ao vivo,
  o drop só é percebido depois). Música sem ReplayGain ganha o volume igualado pela sonoridade medida (nível
  de referência -18 LUFS, sem passar do pico).
- **A música local aparece no Windows** como a de qualquer app (controle de mídia do sistema), com a capa
  embutida no arquivo (gravada na pasta temporária `TccPlayer`): as teclas de mídia do teclado funcionam, e o
  histórico, a letra, o Last.fm e o Discord a enxergam sem código próprio. O visual reage porque ouve o que sai
  do computador.
- **Histórico** (aba da Biblioteca): o que tocou com o Player aberto, por período (hoje, 7 dias, 30 dias, tudo): tempo
  ouvido e comparação com o período anterior, músicas, artistas, BPM e energia médios, dias seguidos com música,
  horas do dia, mais ouvidos e as últimas músicas. Uma música entra depois de 30 segundos ouvidos (ou da metade,
  se for curta); pausa e silêncio não contam. Funciona com a janela minimizada.
- **Barra de tarefas**: ao passar o mouse no ícone do app, a miniatura tem os botões anterior, tocar/pausar e
  próxima.
- **Bandeja**: um ícone ao lado do relógio mostra o nome da música; clicar traz a janela, e o botão direito abre
  o menu (mostrar, tocar ou pausar, próxima, anterior, sair). Com "Continuar na bandeja" ligado nos Ajustes
  (desligado por padrão), fechar a janela só a esconde: o app segue ouvindo e contando o histórico.
- **Ilha** (opcional, desligada por padrão: tecla `D`, o botão ao lado da janela mini no dock ou Configurações ›
  Ilha): um entalhe preto preso ao topo da tela, com "ombros" curvos que o ligam à borda (no estilo do Nusa e do
  island), por cima de tudo e sem roubar o foco. Fechada, mostra a capa e o som em barras na cor do visual. Avisa
  por um instante a música nova, o volume e, nas músicas pré-analisadas, a contagem "Drop em 3, 2, 1".
  - **Abre na hora** com o mouse em cima e fecha logo depois de ele sair (só o tempo da animação). Cresce com um
    leve quique e encolhe sem quique; some encolhendo para a borda.
  - **Páginas** quando aberta: *Tocando* (capa grande, título, progresso, controles, abrir o Player), *Letra* (a linha
    cantada grande, enchendo como num karaokê, entre a anterior e as próximas; a troca de linha rola numa mola e a
    linha que chega cresce e acende), *Visual* (o visual de verdade, reagindo ao som, num recorte da Ilha, com a
    escolha entre Spectral, Fluxo e Barras; ele é preparado em segundo plano quando a Ilha abre, então chegar à
    página não trava), *Ritmo* (BPM, energia, estrutura da faixa, contagem do drop e tom) e
    *Ajustes* (cores reativas, modo seguro, tema, tamanho, trava, monitor, abrir ao pairar e abrir as
    Configurações; o que está ligado fica branco, como no painel). Troca pelas abas, arrastando o conteúdo para o
    lado ou com a roda do mouse.
  - **Arrastar é sempre livre.** Travada (o padrão), a Ilha desliza ao longo da borda em que está; puxada para dentro
    da tela, estica um pouco e volta. **Destravada** (o cadeado no alto da Ilha aberta), vai para qualquer canto:
    perto de uma borda (topo, embaixo, esquerda, direita) gruda nela; largada no meio, fica solta; travar a devolve à
    última borda. Ela lembra onde ficou. "Voltar ao centro do topo" e as quatro bordas também nas Configurações.
  - **Dos lados, vira uma barra lateral**: fechada, uma aba estreita; aberta, uma coluna com as abas num controle
    segmentado, a capa do maior tamanho que cabe, nome, progresso, controles centrados e, embaixo, de onde vem o som
    e "abrir o Player". Ritmo e Ajustes também se arrumam na vertical.
  - **Mais de um monitor**: arrastando, a Ilha passa para o monitor em que o mouse está (travada, na mesma borda) e
    fica nele. Também dá para escolher nas Configurações (Do Player, Monitor 1, Monitor 2...), pelo bloco Monitor
    da página Ajustes da Ilha ou pela barra de comandos ("Mandar a Ilha para o outro monitor").
  - **Tamanhos**: mínima (fechada só com o som em barras), padrão e grande.
  - **Sai do caminho** com jogo, vídeo ou apresentação em tela cheia, com o Player maximizado ou em tela cheia na
    frente e depois de 30 s parada; segue funcionando com a janela minimizada ou na bandeja. Fora da forma, os
    cliques passam para o que está embaixo. O × da Ilha aberta a fecha. Respeita o modo seguro e o "Efeitos de
    animação" do Windows.
- **Nos serviços** (F1): com provedores instalados em `<pasta de dados>/provedores/<nome>/` (o `provedor.toml` e o
  programa), a busca também pergunta a eles. Na Biblioteca, "Buscar \"…\" nos serviços" leva à página Busca com o
  mesmo texto; lá, clicar numa faixa toca (os resultados viram a fila), e dá para pôr a seguir ou no fim da fila, ao
  lado das músicas do computador. A faixa toca pela rede, com a pré-análise (visual, BPM, drops) feita do mesmo endereço, e a
  seguinte entra sem intervalo. Um provedor que trava é reaberto sozinho.
- **YouTube Music** (F3): vem pronto com o app, sem conta. A busca traz faixas, álbuns, artistas e playlists dele
  (cada tipo pedido ao mesmo tempo, com prazo de 8 s cada), e as faixas tocam inteiras, com capa, visual, BPM, tom e drops da pré-análise. Na Loja, aba Provedores, ele
  aparece como "Vem com o app" (dá para desligar).
- **Configurações** (`Ctrl ,` ou o botão de controles na barra lateral): uma página com seções (Geral, Aparência,
  Visual, Som, Ilha, Atalhos, Sobre). Larga, as seções ficam numa coluna à esquerda; média, viram fichas no topo;
  estreita, a página vai para a área do painel e rola inteira, com o visual em cima. As mudanças valem na hora e
  ficam guardadas. Trocar o tema muda as cores na hora (o tom do dock acompanha em cerca de 0,15 s).
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
  A aba **Provedores** mostra os provedores instalados (situação, versão e o que cada um sabe fazer), liga e
  desliga cada um (desligado, o programa dele fecha e a busca não pergunta a ele; a escolha fica guardada), instala
  de uma pasta (a mesma pasta de novo atualiza) e remove, com "Desfazer" no aviso. "Abrir a pasta dos provedores"
  mostra onde eles ficam.
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
- **Discord** (módulo, versão 1.1.0): mostra a música no perfil ("Ouvindo"), com a capa do álbum, a foto do
  artista no canto, o nome do álbum ao passar o mouse e a barra de progresso; título e artista viram links para o
  Last.fm. Fala com o Discord aberto neste computador e busca a capa na busca pública do Deezer (vão o título e
  o artista; sem conta nem chave). O Player lê a resposta do Discord: se ele recusar o cartão, o motivo aparece
  na ficha. O cartão some quando a música pausa. Não aparece? No Discord, ligue "Privacidade de atividade >
  Compartilhar sua atividade detectada".
- **Versões novas de módulo**: a que não pede nada além do que foi aceito se aplica sozinha; a que pede mais
  (como o Discord 1.1.0, que passou a usar a internet) fica parada até a pessoa aceitar na ficha ("Aceitar e
  atualizar").
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
  logo e o ponto da energia pulsam uma vez por compasso (nunca no modo seguro), o volume estica nas pontas. Com o
  "Efeitos de animação" do Windows desligado (ou o modo seguro), nada desliza: tudo vai direto ao estado final.
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
- Sem internet: o módulo Letra avisa e tenta de novo a cada 20 segundos. "Não tem letra" vale por um dia. Uma
  fonte fora do ar é pulada; a letra só falha se todas falharem. Sem a tradução, a letra aparece sem ela.
- Sem barra de tarefas ou bandeja (o Windows recusou a ligação): o app abre sem os botões, sem o ícone e sem a
  janela mini, e fechar a janela sempre encerra.
- Só uma cópia do app roda por vez. A variável de ambiente `TCCPLAYER_DADOS=<pasta>` troca a pasta de dados e
  deixa uma segunda cópia rodar ao lado (para testar sem tocar nos ajustes e no histórico de sempre). Para
  testar o player: `TCCPLAYER_MUDO=1` toca sem som e `TCCPLAYER_PASTA=<pasta>` já põe uma pasta na biblioteca.

Dados do usuário ficam em `%APPDATA%\TccPlayer` (`ajustes.toml`, `modulos.toml`, `historico.db`, `biblioteca.db`, `lastfm.cofre` e `registros\player.log`). Nenhum
áudio é gravado nem enviado para fora. O histórico fica só no computador. Da letra saem a busca (título,
artista, álbum, duração e o id do vídeo do YouTube Music, para a Better Lyrics, a Unison e o LRCLIB) e, com a
tradução ligada, as linhas da letra para a Unison; só com o módulo Letra ligado.

- **Janela estreita**: com a Biblioteca ou a Loja aberta no layout empilhado, a página fica com o meio inteiro (o
  painel de módulos sai), sem a descrição, e os botões de texto do topo dela viram só ícone, com a dica.

## Atalhos

Espaço tocar/pausar · Ctrl ←/→ faixa anterior/próxima · Ctrl ↑/↓ volume · M mudo · F tela cheia · P painel ·
Ctrl K comandos · Ctrl , ajustes · V próximo visual · 1/2/3 escolher o visual · T próximo tema · I imersivo ·
E editar o layout · N janela mini · D Ilha · H Início · B Biblioteca · L Loja · S aleatório · R repetir · Q fila ·
Y letra em tela cheia · Alt ←/→ voltar e avançar · Ctrl Tab próximo espaço · Ctrl 1–9 abrir o espaço da posição ·
Ctrl L trocar o layout · X expandir o visual (Estante) · C curtir a faixa que toca · Esc fechar/voltar. As teclas de uma letra não valem enquanto um campo de texto está ativo. Cada atalho chama um comando com nome (`player.tocar-pausar`,
`layout.editar`…), o mesmo que os botões e a barra de comandos usam. A lista inteira está em
`crates/nucleo/src/comando.rs`.

## Ainda não tem

Captura por app (hoje é o som do sistema inteiro), no player: tela da fila, tocar sem intervalo entre faixas,
normalização de volume, equalizador, crossfade, capa da música local no controle do Windows, playlists e
visões por álbum e artista; retrospectiva do ano, download de módulos
pela Loja (catálogo assinado), skins carregadas de arquivo, ir a um ponto da música pelo mapa de energia (e a
lupa ao passar o mouse), arrastar o visual para trocar de lado (há o botão "Trocar de lado"), medidor de flashes,
ajuste de sincronia para fone Bluetooth e CI. A Biblioteca abre uma folha dizendo o que vem.
