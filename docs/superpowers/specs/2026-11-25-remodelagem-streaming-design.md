# Remodelagem das telas de streaming e da Estante: desenho

Data: 25/11/2026. Situação: **decisões tomadas (seção 3); as 8 partes prontas em 25/11/2026**.
A auditoria que deu origem a isto: [`../../auditoria-ui-2026-11-25.md`](../../auditoria-ui-2026-11-25.md).
O protótipo clicável, com as opções em quatro tamanhos de janela: [`../../../mockup/remodelagem/`](../../../mockup/remodelagem/index.html),
publicado em https://claude.ai/artifact/EGKMxFWu6fMk1rAwpzCuUF (privado).

## 1. Por quê

O Rafael pediu (25/11/2026): o fluxo de hoje confunde (cada serviço com o seu "Para você" e a busca em texto, sem
lupa); Tudo precisa trazer de verdade coisas do YouTube Music e do Spotify, cada uma com a marca do serviço embaixo à
direita, também na barra de baixo; a Estante agrada, mas tira o visual do centro; fontes fora do padrão, letras e
símbolos pequenos; tudo um pouco maior e padronizado, e só depois uma função para diminuir; capas arredondadas e
modernas; imagem de fundo em artista e álbum; responsividade; respeitar o nosso design system.

## 2. Objetivo e limites

**Dá certo se:** qualquer música conhecida, de qualquer serviço, é achada e tocada em até três ações a partir de
qualquer tela; o visual aparece grande em toda janela a partir de 761 px, também navegando; nenhum texto de leitura
tem menos de 14 px; nenhum número de medida fica fora do arquivo de medidas; toda música na tela diz de onde vem.

**Fica de fora:** tocar vídeo, Canvas, podcasts, rádio, salas, temas em arquivo, arrastar para playlists, letra no
visual. O Palco, a Ilha, os módulos, a Loja e as Configurações só recebem as medidas novas.

**Seria um fracasso se:** virasse um clone do Spotify (perder o visual, a parede preta, a Doto e os títulos estreitos);
a tela esperasse a rede em algum momento; a Estante ficasse bonita em 1920 e ruim em 958 × 560.

## 3. Decisões

Escolhidas pelo Rafael em 25/11/2026, com o protótipo na mão:

| # | Decisão | Escolha | As outras opções (seção 13) |
| --- | --- | --- | --- |
| 1 | Onde fica o visual na Estante | **B, lado a lado, com divisória** | A: folha que sobe sobre o visual. C: vidro sobre o visual |
| 2 | Como escolher o serviço | **Filtro no alto da página** | Grupos por serviço na lateral. Seletor na busca |
| 3 | Quanta cor de serviço | **Só no selo e no filtro** | Um véu leve no alto. Como hoje |
| 4 | Extras | **Os quatro:** mesma música com dois selos (parte 3), menu do botão direito (4), a página sai da frente (5), ver o artista no outro serviço (6) | Nenhum ficou de fora |

Decididas por mim, para corrigir se discordar: tamanho-base 16 com apoio de 14; Palco e Estante continuam dois layouts;
uma pilha só de voltar; Fila e Letra no painel da direita; Histórico na Biblioteca; a Ilha e os botões da barra de
tarefas não mudam de função.

## 4. Princípios

1. **O serviço é filtro, não lugar.** Três destinos que nunca mudam; o serviço só muda o que aparece neles.
2. **Cor grande é da energia; cor pequena é do serviço.** A barra e o visual têm a cor da música. O serviço tem um
   selo. Álbum e artista têm a cor da própria imagem.
3. **O visual não sai de cena para a pessoa navegar.**
4. **Uma escala, sem exceção.** Medida solta não compila no teste.
5. **Tudo leva a algum lugar.** Capa, título e artista são links em toda parte.
6. **Contêiner ganha linha, estado ganha preenchimento.** (A regra que já é nossa.)

## 5. Medidas

Um arquivo novo, `app/player/src/ui/medidas.rs`, é o único lugar com números, em `px`. Na parte 8, o `px` das telas passa a
ser um do próprio arquivo, que multiplica pela escala, e o arranjo é calculado na janela dividida por ela (decidido na parte 1,
no lugar do `rem`: uma conta só, e os modos responsivos acompanham a escala).

| Papel de texto | Tamanho/linha | Fonte e peso | Onde |
| --- | --- | --- | --- |
| Cartaz | 88/88 (encolhe para 72, 56, 44) | Título 700 | Artista, visual inteiro |
| Página | 44/48 | Título 700 | Início, Biblioteca, álbum |
| Seção | 28/34 | Título 700 | Prateleiras, módulos grandes |
| Destaque | 20/28 | 600 | Artista sobre o visual |
| Corpo | 16/24 | 450 ou 600 | Nomes, navegação, botões, texto corrido |
| Apoio | 14/20 | 450 ou 600 | Artista, dados, fichas |
| Micro | 12/16 | 500, ou mono | Teclas, tempos, nome sob o ícone do trilho |

De-para: 12 e 13 de leitura viram 14; 14 principal vira 16, 14 secundário fica; 15 vira 16; 18 vira 20 ou Seção; 22 e
24 viram 28; 32, 38 e 40 viram 44; 11 só na Ilha, como 12.

| Medida | Valores |
| --- | --- |
| Ícone | 24 (caixa de 44), 20 ao lado de um texto, 16 só dentro de ficha pequena, 28 no tocar. Traço de 1,8 em 24 (os 70 SVGs são regravados) |
| Raio | 28: regiões e capa de cabeçalho. 16: capa a partir de 64, cartão, linha, menu. 10: capa menor e controle. Pílula e círculo |
| Espaço | 4, 8, 12, 16, 24, 32, 48. Entre regiões da janela, 12; entre cartões, 16 |
| Altura | Topo 64, busca 48, item de navegação 48, ficha 40, botão 44, linha 64, atalho 72, barra 96 (88 baixa, 72 estreita) |
| Largura | Lateral 264, trilho 92, painel 380 (até 42% do conteúdo), barra de navegação 72 de altura |

**Teste `medidas::sem_numero_solto`:** lê `src/ui/*.rs` e falha com `texto(<número>)`, `titulo(<número>)`,
`.rounded(px(<número>`, `icone(.., <número>`, `.text_size(px(<número>`. Começa com a lista das exceções de hoje por
arquivo e só pode diminuir; termina vazia na parte 7.

## 6. Navegação e fluxo

**Destinos** (lateral, trilho com nome, ou barra embaixo): **Início**, **Buscar**, **Biblioteca**. Embaixo: Loja e
Configurações, com nome. Editar e Trocar layout vão para o menu da janela e a barra de comandos.

**Filtro de serviço** (Início, Buscar, Biblioteca): fichas de 40 px, "Tudo", "YouTube Music", "Spotify", "Neste PC" e
um por provedor instalado, com marca e nome; abaixo de 520 px de conteúdo, só a marca das não escolhidas. É o
`ajustes.espaco` de hoje. Trocar o filtro não troca de página. Páginas de detalhe não têm filtro.

**Busca única:** o campo do topo. Clicar, `Ctrl K`, `/` ou "Buscar" levam à página Buscar com o foco nele. Vazio: as
buscas recentes e o que foi aberto há pouco. Com texto: abas Principais, Músicas, Álbuns, Artistas, Playlists,
Comandos; começar com `>` mostra só comandos. A barra de comandos por cima só existe onde não há topo (só o visual,
tela cheia). Cada campo tem o seu texto: a busca dos serviços, o filtro da Biblioteca e o da letra não se misturam.

**Biblioteca:** uma só, com filtro de serviço e abas Tudo, Playlists, Álbuns, Artistas, Curtidas, Pastas, Histórico.
"Neste PC" mostra as categorias por pasta de hoje.

**Voltar e avançar:** uma pilha; cada passo guarda a rota, o filtro e a rolagem.

| Tarefa | Hoje | Depois |
| --- | --- | --- |
| Achar uma música do Spotify estando no YouTube | Escolher a bolinha, aba Busca, campo, texto | Campo, texto |
| Ver as playlists de todos os serviços | Não existe | Biblioteca › Playlists |
| Ir ao álbum da música que toca | Buscar o nome | Clicar na capa da barra |
| Saber de onde vem a música que toca | Não dá | Selo na capa da barra |
| Ver o visual grande navegando | Tela cheia e voltar | Já está lá |

## 7. Layout

### 7.1 Estante (opção B)

`janela = [lateral ou trilho] [visual] [divisória] [página]`, com o topo em cima e a barra embaixo dos dois.

| Janela | Navegação | Visual : página | Painel |
| --- | --- | --- | --- |
| 1680 ou mais | Lateral 264 | 52 : 48 | Abre ao lado (380) |
| 1200 a 1679 | Lateral 264 | 46 : 54 | Toma o lugar da página |
| 761 a 1199 | Trilho 92 com nomes | 48 : 52 | Toma o lugar da página |
| até 760 | Barra embaixo | Visual no alto (40% no Início, tira de 72 nas outras) e a página como folha que sobe | Folha inteira |
| altura até 640 | (igual) | Topo 64, barra 88, cabeçalhos baixos | (igual) |

- A divisória (12 px, pega de 4 × 48) arrasta de 28% a 68%; dois cliques voltam ao padrão; grava `ajustes.divisao`.
  Enquanto arrasta, o motor desenha num tamanho só (`Destino.interno`), com recorte em vez de esticar: `saida.wgsl`
  ganha um retângulo de recorte em `U.v[3]` e lê `t0` por ele.
- **Página larga** (botão no canto da página, `ajustes.pagina_larga`): a página toma o miolo e o visual vira uma tira de
  96 px (72 baixa) no alto, recortada no centro.
- **Só o visual** (X, o `ExpandirVisual` de hoje): a página sai para a direita e volta de lá.
- Sobre o visual: título (Página), artista (Destaque), BPM na Doto quando cabem (520 px) e o botão de expandir.
- O painel substituindo a página guarda a página e a rolagem; fechar devolve.
- **A página sai da frente** (extra): com música tocando, a janela na frente, o visual e a página à vista e o mouse e o
  teclado parados por 30 s, a página sai para a direita e fica só o visual; o cursor some. Mexer o mouse, tecla ou
  música que para trazem de volta a página e a rolagem, sem contar esse movimento como clique. Não vale com campo de
  texto ativo, menu aberto ou modo edição. Ajuste em Configurações › Aparência (`ajustes.descanso_s`: 0 desliga; vem em
  30).

### 7.2 Palco

Continua: só o visual, com a página abrindo por cima. Ganha o trilho com nomes, o topo com a busca única, o filtro e as
peças novas. A folha perde a tinta do serviço.

## 8. Peças

- **Capa:** a imagem recebe o raio (`img().rounded()`), nunca só a caixa. Álbum: quadrado de 16. Playlist: duas abas
  atrás, no alto (a silhueta que já é nossa). Artista: círculo. Sem imagem: silhueta do tipo.
- **Selo:** disco escuro de 28 (20 em linhas, 24 na barra), marca em linha na cor do serviço, embaixo à direita. Aparece
  quando a tela mistura serviços. Com o mouse em cima do cartão, o selo cresce para 48, ganha a cor do serviço e vira
  o botão de tocar (140 ms; volta em 100). A pessoa vê onde vai tocar antes de clicar.
- **Menu** (extra): o mesmo para o ⋯ e para o botão direito, em capas, linhas, na barra, na fila e no visual. Faixa:
  tocar, tocar em seguida, pôr na fila, pôr na playlist, curtir, ir ao álbum, ir ao artista, abrir no outro serviço,
  separar (quando juntou errado). Álbum, playlist e artista: tocar, aleatório, pôr na fila, salvar, fixar. Só o que
  vale para o item aparece; abre onde o mouse está, cabe na janela, setas e Enter andam, Esc fecha.
- **Par de selos:** duas marcas sobrepostas quando a mesma música está em dois serviços (extra) e, na barra, quando o
  Match toca por outro: Spotify atrás, YouTube Music na frente, com a dica "tocando pelo YouTube Music".
- **Cartão:** quantos couberem com no mínimo 180 (160 com conteúdo menor que 760, 150 abaixo de 520), 16 entre eles,
  preenchendo a largura. Nome em Corpo 600 com reticências, linha de baixo em Apoio.
- **Atalho:** 72 de altura, capa colada à esquerda, nome em até duas linhas. Duas, três ou quatro colunas.
- **Linha de música:** 64, capa de 48 (número no álbum), nome e artista, álbum, ouvidas, coração, tempo, ⋯. Perde
  colunas, e nunca o nome: abaixo de 640 saem álbum e ouvidas; abaixo de 480, número e coração.
- **Ficha** (filtro) e **aba** sublinhada (tipo) são diferentes de propósito: duas fileiras de fichas iguais confundem.
- **Cabeçalho com imagem:** artista, foto larga com degradê (cheia até 45%, sumindo em 98%) e véu à esquerda; álbum e
  playlist, a capa desfocada e escurecida atrás, capa de 232 (176 ou 120) com raio 28. Nome, dados separados por
  espaço (sem ponto do meio), Tocar, Aleatório, Seguir ou Salvar, ⋯.
- **Barra de baixo:** três colunas. Capa de 64 com selo, nome, artista e coração; embaralhar, anterior, tocar (52),
  próxima, repetir, e a onda com os tempos; letra, fila, painel, volume e ⋯ (mini, Ilha, tela cheia, saída de som).
- **Painel:** Agora (os módulos de hoje), Fila, Letra. Fechado por padrão na Estante.
- **Vazio, carregando, erro:** esqueleto na forma da peça; vazio sempre com uma ação ("Entrar no Spotify", "Buscar
  músicas"); erro com a causa e "Tentar de novo" (como hoje).

## 9. Páginas

- **Início, Tudo:** saudação (a do serviço, quando há; senão pela hora); até 8 atalhos; prateleiras juntas por tipo; as
  sem tipo intercaladas, com o nome do serviço ao lado do título. **Num serviço:** as prateleiras dele, sem selos.
- **Buscar:** seção 6. Melhor resultado ao lado das músicas a partir de 820 px de conteúdo; embaixo disso, vira linha.
- **Biblioteca:** grade de cartões ou lista; ordenar por recentes, nome ou artista.
- **Artista:** cabeçalho com foto; abas Visão geral, Álbuns, Singles e EPs, Coletâneas, Sobre (só as que têm conteúdo).
  Visão geral: populares com ouvidas, último lançamento, álbuns, parecidos. Sobre: biografia e fotos. "Ver no YouTube
  Music" ou "Ver no Spotify" (extra) procura o mesmo nome no outro serviço e só aparece quando o primeiro resultado é um
  artista com o mesmo nome, sem acento e sem caixa; o par fica guardado em `pares.json`.
- **Álbum:** ano, quantas, quanto tempo, gravadora no fim; faixas com número; mais do artista.
- **Playlist:** dono, descrição, quantas, quanto tempo, quem segue; faixas com capa.
- **Faixa:** o fundo da capa; abas Letra, Mapa e Parecidas, como hoje.

## 10. Dados

Protocolo versão 1, tudo opcional (quem não conhece ignora):

- `Prateleira.tipo`: `atalhos`, `para-voce`, `lancamentos`, `recentes`, `artistas`, `clipes`, `paradas`, `outras`.
- A resposta `itens` ganha `detalhe`: `imagem` (larga), `cor` (`#RRGGBB`), `descricao`, `seguidores`, `ouvintes`,
  `total`, `duracao_s`, `data`, `gravadora`, `tipo` (álbum, single, EP, coletânea), `dono`, `secoes` (`Prateleira`s:
  álbuns, singles, coletâneas, parecidos). `Faixa` ganha `ouvidas` e `numero`.

De onde vem (conferido em 25/11/2026 com uma sonda): Spotify, `queryArtistOverview` (`headerImage`, `stats`,
`profile.biography`, `discography.{albums,singles,compilations,latest,popularReleasesAlbums}`, `relatedContent`),
`getAlbum` (`coverArt.extractedColors`, `date`, `label`, `type`, `playcount`), `fetchPlaylist` (`description`,
`followers`), `home` (`greeting`, `HomeShortsSectionData`); tudo já vem no pedido que o Player faz. YouTube Music,
`rustypipe::MusicArtist` (`header_image` de até 2880 × 1200, `description`, `subscriber_count`, `similar_artists`),
`MusicAlbum` (`album_type`, `year`, `track_count`), `MusicPlaylist` (`description`, `track_count`). Neste PC: a capa
embutida; artista, um mosaico das capas.

- `capas.rs` grava, nas mesmas threads, ao lado do arquivo: miniatura de 128, fundo desfocado (`image::imageops::fast_blur`,
  480 × 270, escurecido) e a cor dominante (a do serviço, quando vem).
- `biblioteca.db` ganha `recentes` (serviço, tipo, id, o item em JSON, quando; até 60), para os atalhos e o Buscar vazio.
  `historico.db`: `execucoes.servico`, opcional (coluna nova, como foi a fonte da letra).
- Juntar o mesmo item de dois serviços (extra): o `casador`, com ISRC igual ou nota a partir de 0,9; toca no serviço do
  filtro, senão no preferido (Configurações › Serviços).

## 11. Movimento

Nada anima em atalho de teclado. Tudo interrompível (`anim.seguir` parte do valor de agora) e desligado com movimento
reduzido ou modo seguro.

| O quê | Como |
| --- | --- |
| Apertar | 120 ms, escala 0,97 |
| Selo que vira tocar | 140 ms para abrir, 100 para voltar |
| Página larga, só o visual, painel | 320 ms, curva de gaveta (0,32 0,72 0 1), pelo mesmo caminho na volta |
| Divisória | segue o mouse, sem atraso |
| Trocar de página | o conteúdo esmaece em 120 ms; nada desliza |
| Cor do cabeçalho | 240 ms |

## 12. Acessibilidade

4,5:1 em todo texto, também sobre foto: o véu do cabeçalho é calculado pela luminância da faixa da imagem atrás do
texto. Tab por navegação, busca, filtro, conteúdo e barra; anel de 2 px; Enter e espaço. Alvos de 44 (40 em fichas).
Nada só com o mouse em cima. Serviço sempre com marca e nome (na dica, quando só a marca cabe). O tamanho da
interface vem na parte 8.

## 13. Alternativas consideradas

**Visual.** *A, folha sobre o visual:* página larga e bonita, mas ao rolar o visual vira uma tira, e na janela de
958 × 560 ele fica com 180 px; fica parecida com o Palco. *B, lado a lado:* o visual fica grande o tempo todo, e em 958 × 560
a página tem a largura de um celular, que todo mundo sabe usar; custo: metade da largura, que a divisória e a página
larga resolvem. *C, vidro:* a página cobre a forma, e texto legível exige escurecer até ela sumir.

**Serviço.** *Filtro no alto:* como o MixMusic; vale nas três páginas e some nas de detalhe. *Grupos na lateral:* mostra
tudo de uma vez, mas cresce com cada provedor e volta a fazer do serviço um lugar. *Seletor na busca:* enxuto, mas
esconde a escolha.

**Cor.** *Só selo e filtro:* a energia continua a identidade. *Véu leve:* bonito numa página só, e briga com a barra.
*Como hoje:* o achado 14.

## 14. Partes

Cada uma: testes antes, `fmt`, `clippy` sem aviso, testes, cópia muda conferida em 1920, 1440, 958 × 560 e 410, README e
AGENTS atualizados. O plano passo a passo de cada parte é escrito quando ela começar.

| # | Parte | Muda | Tamanho |
| --- | --- | --- | --- |
| 1 | Fundação e consertos | `ui/medidas.rs` e o teste; todos os `ui/*.rs` nas medidas novas; `arranjo.rs` (alturas); ícones; capas redondas; `object_fit`; reticências; busca separada; caixa alta e pontos do meio; miniaturas em `capas.rs` | grande |
| 2 | Navegação e busca única | `navegacao.rs` (uma pilha, `Passo`), `ui/estante.rs` e `ui/casca.rs` (lateral, trilho, topo), `ui/busca.rs`, `ui/paleta.rs` (dentro da busca), `ui/biblioteca.rs` (uma só), `ui/espacos.rs` (vira o filtro) | grande |
| 3 | Tudo misturado e selos | `paginas::juntar_inicio`, `Prateleira.tipo` nos dois provedores, `recentes`, `historico.servico`, `ui/selo.rs`, `espacos.rs` sem tinta, Ilha; a mesma música com dois selos (`casador`) | grande |
| 4 | Barra, painel e menu | `ui/dock.rs`, `ui/painel.rs` (Agora, Fila, Letra), links, `ui/menu.rs` (⋯ e botão direito em tudo) | grande |
| 5 | Visual no centro | `arranjo.rs` (B), divisória, página larga, recorte em `saida.wgsl` e `motor.rs`, `app/midia.rs`; a página sai da frente (`descanso.rs`) | média |
| 6 | Artista, álbum, playlist | `detalhe` no protocolo, conversões, `ui/detalhe.rs`, fundo desfocado, abas; o artista no outro serviço | grande |
| 7 | Tamanhos e teclado | Cartões fluidos, regras por altura, `tab_stop` e anel de foco, vazios com ação, nomes compridos e listas enormes | média |
| 8 | Tamanho da interface | `ajustes.tamanho` (90, 100, 110, 125), o `px` de `medidas.rs`, `arranjo` escalado, Configurações › Aparência | pequena |

Ordem: 1 antes de todas; 2 antes de 3 e 5; 4 e 6 a qualquer momento depois da 1; 7 depois da 6; 8 por último.

**Testes que cada parte precisa ter:** 1: `sem_numero_solto`, a busca não vaza, a miniatura é usada abaixo de 128. 2: a
pilha guarda rota, filtro e rolagem; voltar depois de trocar o filtro; `>` só comandos; provedor que sai no meio. 3:
juntar por tipo, intercalar, não repetir, serviço que não respondeu, serviço só com uma prateleira; dois selos: ISRC igual, nota baixa, remix e ao vivo
separados. 4: o destino de cada clique com faixa local, de serviço, do Match e do Modo Escuta (sem link); o menu de cada
tipo de item e o menu que não cabe na janela. 5: os retângulos em todos os tamanhos (não se cruzam, o visual nunca
abaixo de 220 px de largura), a divisória nos limites, o recorte, o descanso (o que conta e o que não conta como parado).
6: provedor antigo sem `detalhe`, artista sem foto, álbum de uma faixa, playlist de 2000, artista que não existe no
outro serviço e homônimo. 7: nome de 200 letras, título sem espaço, 10 mil
faixas, janela 400 × 400. 8: cada tamanho em cada modo.

## 15. Riscos

| Risco | O que fazer |
| --- | --- |
| A parte 1 mexe em todas as telas | Um arquivo por vez, com foto antes e depois; a lista de exceções do teste mostra o que falta |
| A escala da parte 8 cai em pixels quebrados | Arredondar no `px` de `medidas.rs`; conferir a Ilha (tremida de 17/11) e as linhas de 1 px, que não escalam |
| Recortar o visual custa quadro | Medir na placa do Rafael antes e depois; o recorte lê a mesma textura |
| Foto larga pesa na memória da placa | Redimensionar para a largura da página e largar ao sair dela |
| Marcas dos serviços em linha não são as oficiais | Uso acadêmico, com o nome ao lado; trocar pela oficial é um arquivo por serviço |
| Duas versões da mesma música juntadas errado (remix, ao vivo) | O `casador` já desconta outra versão; nota 0,9 e "separar" no menu |

## 16. Fica para depois

Estilos de cartão por tema, arrastar músicas, layouts de terceiros na Loja, a opção A como layout extra, e o visual
usando a imagem do artista.
