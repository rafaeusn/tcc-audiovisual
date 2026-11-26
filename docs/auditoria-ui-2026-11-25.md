# Auditoria das telas de streaming (25/11/2026)

Telas: Início ("Para você"), Busca, Biblioteca, artista, álbum, playlist e faixa, em Tudo, YouTube Music e Spotify; a
barra de baixo; os layouts Estante e Palco. Só leitura: nada do app mudou. O que fazer com isto está em
[`superpowers/specs/2026-11-25-remodelagem-streaming-design.md`](superpowers/specs/2026-11-25-remodelagem-streaming-design.md),
e o protótipo clicável em [`../mockup/remodelagem/`](../mockup/remodelagem/index.html)
(publicado em https://claude.ai/artifact/EGKMxFWu6fMk1rAwpzCuUF, privado).

**Método.** Revisão de design pela skill impeccable (`audit` e `critique`, variantes de app nativo), com as skills
ui-ux-pro-max, emil-design-eng e apple-design como critério extra. O detector automático da impeccable não se aplica
(ele lê HTML e CSS; aqui é Rust + GPUI) e o programa dela não foi rodado, porque precisaria ser baixado. Fotos de uma
cópia muda do Player, com a conta do Spotify e o YouTube Music de verdade, em 1920, 1440, 1280 × 720, 1000, 958 × 560,
700 e 410 px, na Estante e no Palco; medidas tiradas do código (`app/player/src/ui/*.rs`); respostas cruas do Spotify
lidas por uma sonda descartável, fora do repositório. Referências lidas no código ou em tela: MixMusic, Spotifast,
limusic, Prism, Metrolist, OpenTune e o redesenho do Figma (Juxtopposed); Sonos e Roon pela documentação e pelos
fóruns.

**Não conferido:** leitores de tela (o GPUI 0.2 não publica árvore de acessibilidade no Windows), o som, e o Player
com mais de dois serviços.

---

**Parte 8 (25/11/2026):** o tamanho da interface (90 a 125%), o "só depois uma função para diminuir" do pedido.

**Resolvidos na parte 7 (25/11/2026):** 19 (o Tab, o anel, Enter e espaço; rolar até o focado ficou) e 28 (os vazios com ação).

**Resolvidos na parte 6 (25/11/2026):** 11, 12 e 13.

**Resolvidos na parte 5 (25/11/2026):** 1 e 23.

**Resolvidos na parte 4 (25/11/2026):** 8, 10, 18, 21 e 33, e em parte 23 (as abas; o painel continua aberto por padrão).

**Resolvidos na parte 3 (25/11/2026):** 3, 28 (a capa das Curtidas) e 29.

**Resolvidos na parte 2 (25/11/2026):** 2, 6, 14, 15, 16, 20, 22, 24, 30, 31, 32, e em parte 18 (as ações da linha não ocupam lugar) e 21 (Histórico).

**Resolvidos na parte 1 (25/11/2026):** 4, 5, 7, 9, 15, 17, 25, 26, 27, e em parte 6, 8, 16 e 22.

## 1. Veredito

O Player passa por um app de verdade no que é dele (o visual, a Ilha, os módulos, o desfazer, os atalhos). As telas de
streaming, não: parecem um site de capas dentro do app. Três coisas explicam quase tudo:

1. **O serviço virou lugar, e deveria ser filtro.** Cada espaço tem o seu "Para você", a sua Busca e a sua Biblioteca.
   A pessoa precisa saber onde a música está antes de procurar.
2. **O visual, que é a razão do app, sai de cena quando se navega.** Na Estante ele é um quadro de 340 px.
3. **As medidas nasceram soltas em cada tela.** Nove tamanhos de texto, onze raios, dez tamanhos de ícone.

## 2. Placar

| Dimensão técnica | Nota | Por quê |
| --- | --- | --- |
| Acessibilidade | 2/4 | Contraste testado em código, movimento reduzido e modo seguro respeitados. Nenhum botão recebe o Tab; ações só com o mouse em cima; 92% do texto com 14 px ou menos; serviços sem nome visível |
| Desempenho | 3/4 | Nada espera na thread da tela; capas e páginas em cache. A busca é reagrupada a cada quadro, a lateral monta todas as linhas, e capas de 640 px são abertas para mostrar 36 |
| Aparência e temas | 1/4 | Cinco tamanhos de texto fora da nossa escala; capas sem arredondar; a cor do serviço disputa com a da energia |
| Padrões da plataforma | 2/4 | Nenhum menu de botão direito; a barra de baixo não leva a lugar nenhum; × numa página que não é janela |
| Adaptação | 2/4 | Três modos com teste, mas cartão de largura fixa, o visual some de 761 a 1060 px e os serviços ficam fora de alcance a 410 px no Palco |
| **Total** | **10/20** | Aceitável: dá para usar, com trabalho importante pela frente |

| Heurística de Nielsen | Nota | Por quê |
| --- | --- | --- |
| Estado visível | 2 | Nada diz de que serviço é a música que toca, nem que ela veio pelo Match |
| Linguagem da pessoa | 2 | "Espaço", Tudo como ícone de grade, Biblioteca que muda de sentido conforme o espaço |
| Controle e liberdade | 3 | Voltar, avançar, Esc e "Desfazer" funcionam bem |
| Consistência | 1 | Três buscas, "Para você" três vezes, × e voltar na mesma página, linhas de 52, 56 e 64 px |
| Prevenção de erro | 3 | Confirmações onde precisa; o login diz o motivo |
| Reconhecer em vez de lembrar | 1 | Bolinhas sem nome; Fila e Histórico como abas escondidas; ações só no mouse |
| Flexibilidade | 2 | Atalhos e barra de comandos ótimos; sem menu de contexto nem teclado nas listas |
| Estética e economia | 2 | Prateleiras iguais uma embaixo da outra; metade da folha vazia em várias telas |
| Recuperar de erro | 3 | "Tentar de novo" e a causa escrita |
| Ajuda | 2 | Dicas em quase tudo; nada explica o Match nem os espaços |
| **Total** | **21/40** | Aceitável |

**Especificidade.** O visual, os números em matriz de pontos, os títulos estreitos e a parede preta são nossos e
reconhecíveis. As páginas de streaming não usam nada disso: trocando a fonte, seriam de qualquer clone.

## 3. Achados

P1: resolver antes da próxima versão. P2: em seguida. P3: acabamento. "Parte" é a do plano de remodelagem. Nenhum P0:
tudo tem algum caminho, mesmo ruim, nas janelas comuns.

### P1

| # | Achado | Onde | O que causa | O que fazer | Parte |
| --- | --- | --- | --- | --- | --- |
| 1 | **O visual perdeu o centro na Estante.** 340 × 255 px numa janela de 1440 × 900 (6,7%); zero entre 761 e 1060 px com a gaveta fechada (a janela que o Rafael usa, 958 × 560); tira de 72 px abaixo de 760 | `arranjo.rs` (`estante`, `TOCANDO`) | A razão do app some justamente ao escolher música | Visual no centro, página ao lado (a opção B do protótipo, escolhida). `layout` | 5 |
| 2 | **A navegação se repete e confunde.** "Para você" aparece na lateral, no topo da folha e no título; abas em texto por espaço; três buscas (a barra de comandos, a página Busca e o campo da Biblioteca); no Palco, o streaming fica atrás do ícone Biblioteca | `ui/espacos.rs:16`, `ui/estante.rs`, `navegacao.rs` (`abas_do_espaco`) | Não se sabe por onde começar, nem o que cada busca procura | Três destinos (Início, Buscar, Biblioteca), uma busca, o serviço como filtro. `distill` | 2 |
| 3 | **"Tudo" não mistura nem marca.** Empilha as prateleiras do YouTube e, telas abaixo, as do Spotify. Cartões, linhas, fila, histórico, barra e Ilha não dizem o serviço | `ui/paginas.rs` (`ver_para_voce`), `ui/dock.rs:157` | Tudo é a pior das quatro páginas, quando devia ser a melhor | Prateleiras por tipo, selo em cada item. `layout`, `clarify` | 3, 4 |
| 4 | **Texto e símbolos pequenos e fora da escala.** `texto()`: 11, 12, 13, 14, 15, 16, 18, 20 e 22 px (65 usos de 12, 24 de 13, 114 de 14); títulos em 22, 24, 28, 32, 38 e 40; 74% dos ícones com 18 px ou menos, com traço de 1,6 em 24 (1,2 px na tela) | `ui/mod.rs` (`texto`, `titulo`, `icone`) e todos os `ui/*.rs` | Cansa, e nenhuma tela combina com a outra | Papéis de texto num arquivo só, com teste. `typeset`, `extract` | 1 |
| 5 | **Capas e fotos de artista quadradas.** O raio está na caixa com `overflow_hidden`, e o GPUI só recorta em retângulo; a imagem não tem raio. O design system diz 16 | `ui/paginas.rs:40-64` | Parece velho; o círculo do artista, que diz o tipo, não existe | `.rounded()` na própria imagem. `polish` | 1 |
| 6 | **Serviços fora de alcance a 410 px no Palco.** A fileira de espaços tem `overflow_hidden` e é cortada antes do YouTube e do Spotify | `ui/espacos.rs:22` | Não há como chegar neles nessa largura | O filtro rola e encolhe para as marcas. `adapt` | 2 |
| 7 | **A busca vaza entre páginas.** Um texto só (`busca_musicas`) serve à Busca dos serviços e à Biblioteca: buscar no Spotify e abrir Tudo › Biblioteca mostra "0 músicas encontradas" | `app.rs:320`, `ui/campo.rs:63`, `ui/musicas.rs:202` | Parece que as músicas sumiram | Cada campo com o seu texto. `harden` | 1 |

### P2

| # | Achado | Onde | O que fazer | Parte |
| --- | --- | --- | --- | --- |
| 8 | **A barra de baixo está torta.** O play (44 px) encosta na borda de cima; os ícones da direita estão noutro eixo; "3:46" fica embaixo do botão do painel; a linha do tempo quase some parada | `ui/dock.rs` | 96 px, um eixo, play de 52. `layout` | 4 |
| 9 | **Capa de vídeo achatada na barra e na sessão.** `img` sem `object_fit` usa `Contain` | `ui/dock.rs:160` | `ObjectFit::Cover`. `polish` | 1 |
| 10 | **A barra não leva a lugar nenhum.** Capa, título e artista não são clicáveis; não há coração | `ui/dock.rs:157` | Capa e título abrem o álbum; o nome, o artista; coração ao lado. `clarify` | 4 |
| 11 | **Artista e álbum sem imagem.** O Spotify já manda, na mesma resposta que o Player pede, a foto larga (1920 × 1080, até 2660 × 1140), ouvintes do mês, seguidores, biografia, cores extraídas, álbuns, singles e coletâneas separados, último lançamento e artistas parecidos; o YouTube Music, uma foto de 2880 × 1200, descrição, inscritos e parecidos. O protocolo não tem campo para nada disso | `provedores/*/src/conversao.rs`, `crates/provedores/src/modelo.rs` | Campos opcionais de detalhe e cabeçalhos com imagem. `bolder` | 6 |
| 12 | **Artista aberto por um link fica sem foto**, e álbuns e singles vêm misturados, com a mesma capa repetida em cada linha do álbum | `ui/paginas.rs` (`ver_detalhe`) | O mesmo de cima; no álbum, número no lugar da capa. `layout` | 6 |
| 13 | **Faltam os números.** Sem quantas músicas nem duração total; "2024" sozinho no subtítulo | `ui/paginas.rs` | Ano, quantas, quanto tempo. `clarify` | 6 |
| 14 | **Duas cores grandes brigando.** A folha e a lateral inteiras ficam vermelhas ou verdes (22% no alto, 7% embaixo) ao lado da barra azul ou vermelha da energia. A cor da energia é a nossa identidade | `espacos.rs` (`TINTA_DA_FOLHA`, `ALFA_DO_TOPO`) | Cor de serviço só no selo e no filtro; nas páginas de detalhe, a cor da imagem. `quieter` | 3 |
| 15 | **A folha tingida tem cantos quadrados** aparecendo fora da borda arredondada | `ui/paginas.rs` (degradê do topo) | Some com o achado 14 | 3 |
| 16 | **Cartões de 156 px em qualquer janela.** Minúsculos em 1920, uma fileira e meia em 1280 × 720, buracos na grade | `ui/paginas.rs` (`CARTAO`) | Quantos couberem, preenchendo a largura. `adapt` | 7 |
| 17 | **Texto cortado sem reticências** nos cartões, na lateral, no módulo Energia ("Energ"), no topo ("Ouvindo Alto-falantes (Realtek") | `.truncate()` sem largura definida; `ui/casca.rs:104` | Largura no próprio texto ou `line_clamp(1)`. `harden` | 1 |
| 18 | **Ações só com o mouse em cima** (coração, tocar, mais), e nenhum clique com o botão direito em todo o app | `ui/musicas.rs`, `ui/paginas.rs` | Coração e ⋯ sempre visíveis; menu de contexto. `harden` | 4 |
| 19 | **Sem teclado nas páginas.** Nenhum `tab_index` na interface; o GPUI 0.2 tem | todos os `ui/*.rs` | Tab, anel de foco, Enter e espaço. `audit` de novo no fim | 7 |
| 20 | **"Biblioteca" são duas coisas.** Em Tudo, são as pastas do computador; a lateral, ao lado, lista as playlists do Spotify | `ui/biblioteca.rs`, `ui/estante.rs` | Uma Biblioteca só, com filtro de serviço e de tipo. `distill` | 2 |
| 21 | **Fila e Histórico escondidos** como abas da Biblioteca | `navegacao.rs` | Fila no painel da direita; Histórico como aba da Biblioteca com nome. `layout` | 2, 4 |
| 22 | **O melhor resultado da busca é um cartão quase vazio**, e só fica ao lado das músicas a partir de 960 px | `ui/busca.rs` (`LADO_A_LADO`) | Cabe ao lado a partir de 820 px de conteúdo; embaixo disso, vira linha. `layout` | 2 |
| 23 | **O painel da direita fica vazio parado**: três avisos de "aparece aqui" em 340 px | `ui/painel.rs` | Painel fechado por padrão, com Agora, Fila e Letra. `distill` | 4 |
| 24 | **A busca é reagrupada e a lateral é montada inteira a cada quadro** | `ui/busca.rs`, `ui/estante.rs` | Guardar o agrupamento por versão; só as linhas visíveis. `optimize` | 2 |
| 25 | **Capa de 640 px para mostrar 36** | `capas.rs` | Miniatura de 128 px ao lado do arquivo. `optimize` | 1 |

### P3

| # | Achado | Onde | Parte |
| --- | --- | --- | --- |
| 26 | Rótulos em caixa alta ("SUA BIBLIOTECA", "ÁLBUM", "PLAYLIST", o nome do serviço, "CLIPE", "DROPS"), que o nosso sistema proíbe | `ui/estante.rs:191`, `ui/paginas.rs:411`, `ui/busca.rs:191`, `ui/faixa.rs` | 1 |
| 27 | Pontos do meio ("Playlist·Spotify", "Ré menor·7A"), também proibidos | vários | 1 |
| 28 | "Músicas Curtidas" sem capa; Biblioteca vazia do YouTube Music sem ação | `provedores/spotify/src/conversao.rs`, `ui/paginas.rs` | 3, 7 |
| 29 | As setas das prateleiras ficam por cima da capa | `ui/paginas.rs` (`ver_prateleira`) | 3 |
| 30 | × numa página que não é janela, ao lado de voltar | `ui/biblioteca.rs:219` | 2 |
| 31 | "Tudo" é um ícone de grade com um +, que parece "adicionar" | `ui/espacos.rs` | 2 |
| 32 | No rodapé da lateral da Estante, Loja, Ajustes, Editar e Layout são quatro ícones sem nome | `ui/estante.rs:145` | 2 |
| 33 | Seis ícones sem rótulo na direita da barra, três deles (Ilha, mini e tela cheia) de uso raro | `ui/dock.rs:96` | 4 |

## 4. O que se repete

- **Números soltos.** Cada tela escolhe o seu tamanho, raio e altura. Foi assim que capas com raio 8, 10 e 0 passaram.
  Um arquivo de papéis e um teste que falha com número solto resolvem de uma vez.
- **Informação que chega e é jogada fora.** O provedor recebe foto, cor, ouvintes, total de músicas, e converte só
  título, artista e capa.
- **O espaço como dono de tudo.** Quarenta usos de `Espaco` fora da navegação decidem cor, abas, busca e biblioteca.
- **O mouse como único caminho.** Ações no `hover`, sem Tab, sem botão direito.

## 5. O que está bom e fica

- Nada na tela espera serviço, disco ou rede; as páginas vencidas ficam enquanto as novas chegam.
- O "Desfazer", a barra de comandos, os atalhos e as dicas.
- O contraste do acento testado nos quatro temas (`espacos.rs`).
- Voltar e avançar como num navegador, com a rolagem guardada.
- O visual, a Ilha, a Doto e os títulos estreitos: a cara do Player.
- O esqueleto enquanto carrega e o "Tentar de novo" com a causa.

## 6. Carga de quem usa

Falhas no teste de carga cognitiva: foco único (o Início em Tudo tem 9 prateleiras iguais), hierarquia (título de 28 e
prateleira de 18 quase iguais), uma coisa por vez (três lugares para buscar), escolhas (quatro bolinhas sem nome, mais
três abas, mais quatro ícones no rodapé). Passam: agrupamento, memória de trabalho (voltar guarda a rolagem) e
revelação aos poucos (Ajustes).

**Quem usa ouvindo, com a janela pequena ao lado de um jogo** (o uso real do Rafael, 958 × 560): abre a Estante e não
vê o visual; vê uma fileira de capas cortada; para achar o Spotify, escolhe entre quatro bolinhas.

**Quem chega do Spotify:** procura a lupa e acha "Busca" escrito em dois lugares e um campo no alto que procura outra
coisa; clica no nome da música na barra e nada acontece; clica com o botão direito e nada acontece.

**Quem tem visão cansada:** lê 12 e 13 px em cinza em quase toda linha, e não há como aumentar.

## 7. Referências: o que pegar de cada uma

| Referência | O que pegar |
| --- | --- |
| MixMusic | Navegação única, um campo de busca, o serviço como ficha ("Everything", "Yours", "Spotify", "YouTube Music"), selo pequeno em cada item e na barra |
| Roon | Biblioteca única que se divide por fonte com um clique. Cuidado com a reclamação dos usuários: o mesmo álbum três vezes, uma por fonte. Uma linha com dois selos evita isso |
| Sonos | Uma busca para todos os serviços, com o preferido na frente. Cuidado: sem filtro por tipo, as pessoas voltaram à busca antiga |
| Spotifast | Atalhos de "continuar" no alto; fila em painel |
| limusic | Cabeçalho com a capa desfocada |
| Prism | Busca vazia que mostra o que foi aberto há pouco |
| Figma (Juxtopposed) | Artista com foto larga, nome enorme e abas sublinhadas; abas empilhadas nas capas de playlist |
| Apple (WWDC 2024, sessão 10207) | Imagem grande no alto, conteúdo espiando embaixo, a imagem sumindo num degradê de 45% a 80% |

Fontes: [Sonos](https://www.whathifi.com/news/redesigned-sonos-app-promises-a-more-streamlined-design-and-one-screen-access-for-all-your-music-streaming-sources),
[Roon](https://roon.app/compatibility/music),
[selos no Roon](https://community.roonlabs.com/t/local-library-icon-in-album-view-grouped-versions/160108),
[WWDC 2024](https://developer.apple.com/videos/play/wwdc2024/10207/).

## 8. Ordem recomendada

`extract` e `typeset` (parte 1), `distill` (2), `layout` e `quieter` (3), `layout` e `clarify` (4), `layout` (5),
`bolder` (6), `adapt` e `harden` (7), e no fim `polish` e uma nova `audit`.
