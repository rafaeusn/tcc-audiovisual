# F2: espaços, páginas unificadas e o layout Estante

Feita em 23/11/2026. Plano em [`plano-v2.md`](plano-v2.md), fase F2. Desenho aprovado em
[`superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md`](superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md),
e os planos de cada parte em [`superpowers/plans/`](superpowers/plans/).

## O que entrou

| Parte | Onde |
| --- | --- |
| Identidade do provedor (cor `#RRGGBB` e logo SVG de até 32 KB) e os ids de álbum e artista nas faixas e nos álbuns (campos opcionais, protocolo v1) | `crates/provedores/src/modelo.rs` (`Apresentacao.identidade`, `Faixa.album_id`, `artistas_ids`) |
| O YouTube Music manda a logo, a cor e os ids; o início junta lançamentos, clipes novos e as paradas; a busca pede todos os tipos ao mesmo tempo | `provedores/youtube-music/src/main.rs`, `conversao.rs` (`juntar_busca`) |
| Páginas dos provedores (início, álbum, artista, playlist) pedidas por rota, com cache de 10 min e "Tentar de novo" | `player/src/remoto.rs` (`PedidoDePagina`, `PaginaRemota`) |
| Curtir, salvar e fixar de qualquer serviço; playlists do Player com faixas de serviço | `crates/biblioteca/src/acervo.rs` (tabelas `salvos` e `faixas_remotas`) |
| Espaços (Tudo, o computador, cada serviço) com uma pilha de páginas cada, voltar e avançar | `player/src/navegacao.rs` (`Espaco`, `Rota`, `Navegacao`) |
| A paleta do espaço tirada da cor do serviço, com o contraste conferido nos 4 temas, e a troca de cor em 250 ms | `player/src/espacos.rs`, `player/src/app/navegar.rs` |
| Capas da internet baixadas em segundo plano e guardadas em `<dados>/capas` | `player/src/capas.rs` |
| Para você (prateleiras), álbum, playlist, artista e a Biblioteca do serviço | `player/src/paginas.rs` (lógica), `player/src/ui/paginas.rs` |
| Busca de todos os tipos, com chips e um bloco por serviço em Tudo | `player/src/ui/busca.rs` |
| Página da faixa: Letra, Mapa da pré-análise e Parecidas (só com `Radio`) | `player/src/ui/faixa.rs` |
| Fila e Histórico como páginas | `Rota::Fila`, `Rota::Historico` |
| O layout Estante: lateral, página, trilho Tocando com o visual, expandir até a janela | `player/src/arranjo.rs` (`estante`, `expandir`), `player/src/ui/estante.rs` |
| O visual desenhado num tamanho e esticado enquanto cresce | `crates/visual/src/motor.rs` (`Destino.interno`), `crates/janela/src/camada.rs` (`desenhar_esticado`) |

## Como a navegação funciona

- Cada espaço tem a própria pilha de páginas (até 50 para trás). Trocar de espaço lembra onde a pessoa estava em cada
  um, e o espaço aberto fica em `ajustes.espaco`.
- **Palco** (o padrão): a página é opcional. Sem página, só o visual, como antes. As abas da folha são Para você,
  Busca e Biblioteca; o computador não tem Para você.
- **Estante** (`ajustes.layout = "estante"`): sempre há página. Ao trocar para a Estante, um espaço sem página abre
  a primeira dele; ao voltar para o Palco, ele volta com a folha fechada. Voltar e avançar pulam a folha fechada do
  Palco. `H` e o × levam à primeira página do espaço; `Esc` volta uma página.
- Um provedor desligado ou removido tira o espaço dele (volta a Tudo). Um que não abriu tira só nesta abertura.
- Navegar não vale na janela mini nem com a barra de comandos ou a janela de playlist abertas. Navegar desfaz o modo
  edição, a busca de músicas e o visual expandido.

## A Estante por largura

| Modo | Como fica |
| --- | --- |
| Largo (> 1060 px) | Lateral (248 px: espaços, páginas, salvos com os fixados primeiro) + página + trilho Tocando (340 px: o visual 4:3 no alto, os módulos embaixo); dock sob a página e o trilho |
| Gaveta (761–1060 px) | A lateral vira uma barra de ícones; o Tocando abre ao lado da página e a empurra (por cima, ele esconderia o visual, desenhado por baixo da interface) |
| Empilhado (≤ 760 px) | Espaços no topo da página, as abas embaixo e o visual numa faixa de 72 px acima do dock (a faixa sai quando a janela fica baixa) |

Expandir (`X` ou clicar no visual): o retângulo do visual cresce até a janela em 0,32 s e as regiões esmaecem; no
fim, é o arranjo da tela cheia, com o título grande, o botão encolher e a janela arrastada pelo visual. Na gaveta, o
`X` abre o Tocando e expande dele. A skin imersiva não vale na Estante, e o erro do visual aparece no quadro do
visual, não sobre a página.

## Comandos novos

| Comando | Atalho |
| --- | --- |
| Voltar e avançar | `Alt ←` e `Alt →`, os botões laterais do mouse |
| Próximo espaço | `Ctrl Tab` |
| Abrir o espaço da posição (1 = Tudo, 2 = o computador, depois os serviços) | `Ctrl 1` a `Ctrl 9` |
| Trocar o layout (Palco ou Estante) | `Ctrl L` |
| Expandir o visual (Estante) | `X` |
| Curtir a faixa de serviço que toca | `C` |
| Fixar na lateral o álbum, a playlist, o artista ou a faixa da página aberta | pela barra de comandos |

## Testado

- Testes de unidade: o protocolo com os campos novos, a mistura da busca, os salvos e as playlists mistas no banco, a
  pilha de cada espaço, a paleta e o contraste nos 4 temas, os cartões, as rotas e os pedidos, o agrupamento da busca,
  o arranjo da Estante nas três larguras e na expansão, o tamanho dos alvos do visual e os comandos novos.
- Na cópia de teste, com o YouTube Music de verdade:
  - a logo e a cor vermelha do espaço, as 4 prateleiras, voltar e avançar, `Esc`;
  - abrir um álbum, tocar, curtir, a página do artista e fixar (gravado no banco);
  - a busca "daft punk" com os chips, a página da faixa com o Mapa (107 BPM, Lá menor, 8A) e a Fila e o Histórico como
    páginas;
  - a Estante a 1280, 1000 e 600 px; expandir com `X` e encolher com `Esc`; o `X` na gaveta.

## Não conferido

- A animação da expansão vista em movimento (só fotos paradas).
- Os atalhos com `Alt` e `Ctrl` (`Alt ←/→`, `Ctrl L`, `Ctrl 1–9`) e os botões do mouse: as mensagens de teste não
  levam os modificadores.
- Clicar nos salvos da lateral, a aba Letra da página da faixa com uma letra de verdade, dois serviços respondendo ao
  mesmo tempo e o erro do visual na tela.
- Curtir (`C`) e fixar pela barra de comandos (só os testes de unidade da lógica).

## Ficou para depois

- A página de podcast (nenhum provedor tem podcasts ainda; entra com a F11) e o pedido de rádio para a aba Parecidas.
- Curtir as músicas do computador (o desenho prevê `servico = "computador"`).
- O visual estica um pouco (deforma) durante os 0,3 s da expansão.
- O modo edição no empilhado da Estante (não há painel nele).
- Desempenho: a lateral monta a lista dos salvos e a busca é agrupada a cada quadro; a lista longa de playlist não
  usa `uniform_list`.
- Detalhes: o "Tocar" da página da faixa troca a fila inteira; voltar depois de "Buscar nos serviços" (que passa
  para Tudo) não volta ao computador; ids de linha podem colidir com mais de 10 artistas ou 100 faixas por serviço; o
  link do artista no cabeçalho cobre o subtítulo inteiro; a foto do artista aberto por um link; tirar uma faixa da
  Biblioteca do serviço (o cartão não tem coração).
