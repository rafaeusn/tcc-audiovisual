# F2: espaços dos serviços, páginas unificadas e o layout Estante

Design acertado com o Rafael em 23/11/2026. Base: `docs/plano-v2.md` (seções 4 e 7, fase F2), o redesign do Figma
(`docs/repertorio-ui.md`, seção 2.1) e o app de hoje (`app/`).

## 1. Intenção

O Player já toca o YouTube Music (F3) e vai receber outros serviços (Spotify na F6, SoundCloud e rádios na F11).
Hoje não há como **navegar** neles: a busca da Biblioteca mostra o bloco "Nos serviços", e só. Os provedores já
respondem início, álbum, artista e playlist (protocolo versão 1), mas o Player nunca pede.

A F2 dá ao Player uma **casca única que vai se enchendo** com cada serviço instalado, e deixa a pessoa **sentir em
que serviço está**: cada serviço tem um "espaço" com a logo, a cor e os tons dele. Tudo responsivo, bem distribuído,
com micro-interações, animações e transições, no mesmo cuidado das fases anteriores (Ilha, letra em tela cheia).

## 2. Metas, o que não é meta, o que seria fracasso

**Metas**
- Espaços na navegação: **Tudo**, **Neste computador** e um por provedor instalado e ligado.
- A identidade de cada serviço (nome, cor, logo) vem do próprio provedor; o Player não guarda marca no código.
- Páginas iguais para todos os serviços: Início, Busca, Biblioteca do espaço, Álbum, Playlist, Artista e Faixa.
- Navegação com voltar e avançar.
- Curtir e salvar faixas, álbuns, artistas e playlists de qualquer serviço, guardado no Player.
- Dois layouts: **Palco** (o de hoje) e **Estante** (o redesign do Figma), escolhidos pela pessoa.
- Na Estante, o visual mora no trilho "Tocando" e cresce até o Palco com um clique.
- Responsivo nos modos que o `arranjo.rs` já tem (largo, gaveta, empilhado, mini).

**Não é meta**
- Podcast e Descobrir (não há provedor; ficam para a F11).
- Login em serviço e biblioteca da conta (vem com o provedor que tiver conta, F6).
- Os layouts Expressivo e Compacto do plano v2 (depois; a estrutura deve deixar espaço para eles).
- Reescrever a Loja e as Configurações (continuam como estão; só ganham a escolha de layout).
- Molas novas no `anim.rs`: `mola` e `mola_amortecida` já existem e servem.

**Seria fracasso, mesmo funcionando**
- Copiar a cara de um serviço (telas iguais às do Spotify). O sotaque é cor, logo e prateleiras; o desenho é nosso.
- Uma página "de quem" quebrada: fila ou playlist com faixas de dois serviços mostrada no espaço errado ou sem selo.
- O visual sumir de quem usa a Estante.
- A interface esperar a rede: toda página abre na hora, com esqueleto, e se enche quando o provedor responde.
- Animação que atrapalha: nada desliza com o "Efeitos de animação" do Windows desligado ou no modo seguro (o
  `anim.quadro(dt, parado)` já resolve isso).

## 3. Restrições

- Rust + GPUI 0.2.2, Windows. Regras do `AGENTS.md`: nomes e textos em português, toda ação do usuário é um
  `Comando`, botões por `self.botao`/`botao_icone`, animações por `self.anim`, dicas por `marca`/`ao_pairar`,
  `uniform_list` para listas longas, a interface nunca espera thread nenhuma.
- Regra 2 do `AGENTS.md`: nada de baixar música. Curtir e salvar guardam **metadados** (ids, títulos, capas), nunca o
  áudio.
- Protocolo dos provedores: a versão 1 continua valendo. Campos novos são opcionais, para provedores antigos
  funcionarem (`#[serde(default)]`).
- Contraste: texto secundário sobre a cor do espaço e no dock usa `--fg-2` com o teto de luminância do repertório
  (seção 2.1, item 2). A cor do serviço é acento, nunca fundo cheio de texto.

## 4. Abordagens consideradas

| | Como é | Por que sim ou não |
| --- | --- | --- |
| A. Casca única neutra | Páginas nossas; o provedor só traz dados | Escala, mas o serviço quase não "aparece" |
| B. Uma cópia por serviço (como o Spotifast) | Com o Spotify, telas iguais às dele | Quebra com dois serviços (fila e playlist mistas), N × 10 páginas, copia marca |
| **C. Casca única com espaços (escolhida)** | Páginas nossas; cada serviço tem um espaço com a identidade dele; Tudo junta os serviços no neutro | Dá a sensação de "estar no Spotify" sem cópia, escala para qualquer serviço, funciona com fila mista |

Intensidade da identidade (mockups): discreto, sotaque ou **espaço do serviço** (escolhido): dentro do espaço, as
folhas, os filetes, o dock e o acento se tingem com a cor do serviço.

Onde fica o visual na Estante (mockups): **no trilho "Tocando", expandindo para o Palco** (escolhido); atrás da página
(atrapalha a leitura e briga com a cor do espaço); só no layout Palco (o visual sumiria da Estante).

## 5. O desenho

### 5.1 Identidade do provedor

A `Apresentacao` ganha um campo opcional `identidade`:
- `cor`: a cor da marca, em `#RRGGBB`.
- `logo`: o desenho da logo em SVG (texto), de até 32 KB, num quadrado. É o provedor que manda a dele.

O Player guarda a logo em `<dados>/provedores-dados/<serviço>/logo.svg` e a mostra num círculo (selo), do tamanho do
lugar: 12 px no selo do item, 20 px na fileira de espaços, 28 px no cabeçalho da página. Sem identidade (provedor
antigo, o exemplo), o espaço usa a inicial do nome num círculo e uma cor derivada do nome do serviço.

**Neste computador** é um espaço nosso: ícone de pasta e uma cor própria da casca (não a de um serviço). **Tudo** é
neutro: as cores do tema de hoje.

**Derivar os tons**: da `cor` sai uma pequena paleta: acento (a cor, clareada se o contraste com o preto for menor que
3:1), tom das folhas (a cor a 6–8% sobre `#060606`), filete (a cor a 18%), degradê do topo das páginas (a cor a 22%
indo ao transparente) e o dock (a cor sob o teto de luminância 0,045). A troca de espaço anima essa paleta numa mola de
cerca de 250 ms (`anim`), sem piscar.

### 5.2 Espaços e navegação

- **Espaço** (`Espaco`): `Tudo`, `Computador` ou `Servico(String)`. Só aparecem os provedores instalados e ligados.
  O espaço aberto é lembrado nos ajustes.
- **Rota** (`Rota`): `Inicio`, `Busca`, `Biblioteca`, `Album(Item)`, `Playlist(Item)`, `Artista(Item)`,
  `Faixa(Item)`, mais as páginas de hoje que continuam (`Loja`, `Configuracoes`, `Fila`, `Historico`). Uma rota de
  detalhe leva o item de onde veio (com o serviço), então abrir um álbum do YouTube Music em Tudo funciona.
- **Pilha**: voltar e avançar como num navegador: botão ← no topo da folha, botão "voltar" e "avançar" do mouse
  (`MouseButton::Navigate`), `Alt ←` e `Alt →`. Cada espaço guarda a própria pilha; trocar de espaço e voltar traz a
  página onde você estava nele.
- `nucleo::Pagina` continua existindo para os comandos (`AbrirPagina`), mapeada para as rotas.
- Comandos novos (com nome, atalho quando fizer sentido e entrada na barra de comandos): voltar, avançar, abrir
  espaço N (`Ctrl 1..9`, sem conflitar com `1/2/3` dos visuais), expandir o visual (o Palco), trocar de layout.

### 5.3 As páginas

Todas abrem **na hora**, com esqueleto (blocos cinza no lugar das capas e das linhas, com um brilho lento) e se
enchem quando o provedor responde. Erro do provedor vira uma frase com "Tentar de novo". Capacidade que o provedor
não declara esconde o que depende dela (sem `Artista`, nomes de artista não são links).

- **Início**: as prateleiras do serviço (pedido `inicio`), cada uma uma fileira de cards rolando na horizontal, com
  setas que somem nas pontas. Em Tudo: "Continue ouvindo" (do histórico local) e as prateleiras de cada serviço, com o
  título "· YouTube Music" e o selo. Em Computador: álbuns e pastas recentes do acervo.
- **Busca**: no espaço aberto (em Tudo, em todos, como a busca de hoje). Resultados agrupados: melhor resultado,
  faixas, álbuns, artistas, playlists. Filtro por tipo em chips. Substitui o bloco "Nos serviços" da Biblioteca.
- **Biblioteca do espaço**: o que a pessoa curtiu e salvou dele (faixas, álbuns, artistas, playlists), em grade ou
  lista, com filtro por tipo. Em Computador: a Biblioteca de hoje (pastas, álbuns, artistas, playlists locais). Em
  Tudo: o salvo de todos, com selo.
- **Álbum e Playlist**: cabeçalho com a capa, o título grande, o artista (link) e o degradê do tom da capa (ou da cor
  do espaço, sem capa); botões tocar, aleatório, salvar; a lista das faixas (linha de 56–67 px: número, título,
  artista, duração; coração sempre à vista; adicionar à fila e "a seguir" no hover). No modo largo, o trilho à direita
  mostra o que a pré-análise souber das faixas (BPM médio, tom, energia).
- **Artista**: foto em círculo, nome, seguir (salvar), populares (5, com "ver mais") e álbuns em grade.
- **Faixa**: capa, título, artista, álbum; abas **Letra** (a letra da F4), **Mapa** (estrutura e energia da
  pré-análise, quando houver) e **Parecidas** (se o provedor tiver rádio; senão a aba some).
- Clicar numa faixa toca a partir dela, com o resto da lista na fila (como a Biblioteca faz hoje).

**Micro-interações**: página nova entra da direita (desliza ~24 px e aparece) e a anterior recua de leve; voltar faz o
inverso. Os itens de uma lista aparecem em sequência rápida (até ~8 itens, 20 ms entre eles). Capa cresce 3% no hover;
linha ganha o fundo de hover e mostra as ações. Curtir pulsa o coração numa mola. Trocar de espaço desliza o
indicador da fileira de espaços até o novo e anima a paleta. Tudo para com movimento reduzido.

### 5.4 Curtir e salvar

Uma tabela nova no acervo (`biblioteca.db`): `salvos(servico, tipo, id, dados_json, quando, fixado)`, chave
`(servico, tipo, id)`. `dados_json` é o `Item` do provedor (título, artistas, capa...), para a Biblioteca abrir sem
rede. Faixas locais podem ser curtidas também (`servico = "computador"`, `id` = caminho). Não guarda áudio.

### 5.5 Layout Palco e layout Estante

- **Palco** (padrão, o de hoje): o visual no herói, o painel de módulos, o dock. As páginas abrem na folha sobre o
  visual, como a Biblioteca abre hoje. A fileira de espaços fica no topo da folha (logos em círculo, a ativa com o
  nome). A cor do espaço tinge a folha e o acento; o dock segue o tom do visual, como hoje.
- **Estante**: a casca do Figma, nas medidas do repertório (moldura e folhas `#060606`, filete de 1 px, raio 10).
  - **Lateral** (240–260 px): a fileira de espaços no alto; Início, Busca e Biblioteca; abaixo, o salvo do espaço
    (fixados primeiro).
  - **Folha de conteúdo**: a rota aberta, com o cabeçalho fixo (voltar, avançar, título).
  - **Trilho "Tocando"** (300–360 px): o **visual ao vivo** no alto (a `janela::Camada` é desenhada no retângulo do
    trilho), capa, título, artista, BPM e tom, e abaixo os módulos do painel que couberem.
  - **Dock** só sob a folha e o trilho, na cor do espaço (sob o teto).
  - **Expandir para o Palco**: clicar no visual do trilho (ou o comando) faz o retângulo do visual crescer numa mola
    até a janela inteira, com o título grande da faixa, e as folhas esmaecem. Esc ou o botão encolher fazem o caminho
    de volta, para a mesma página. A letra em tela cheia (Y) funciona por cima.
- **Escolher o layout**: Configurações › Aparência (e a aba Layouts da Loja, que já existe). Fica em `ajustes.layout`.

### 5.6 Responsivo (Estante)

| Modo (`arranjo`) | Estante |
| --- | --- |
| Largo (> 1060 px) | Lateral + folha + trilho com o visual |
| Gaveta (761–1060 px) | A lateral vira um trilho de ícones (espaços e páginas, com dica); o "Tocando" vira gaveta pela direita; o visual fica no topo da gaveta |
| Empilhado (≤ 760 px) | Fileira de espaços no topo; Início, Busca e Biblioteca em abas embaixo; o visual vira a capa animada no dock (toque nela expande para o Palco) |
| Mini | Igual a hoje (só o visual) |

O Palco segue o `arranjo` de hoje.

### 5.7 Decisões depois da revisão (chamadas minhas, para o Rafael conferir)

- **Palco sem página aberta**: no Palco a rota é opcional (`Option<Rota>`). Sem rota, só o visual, como hoje; `H` e
  `AbrirPagina(Inicio)` continuam fechando a folha. O Início do serviço (as prateleiras) é a primeira aba da folha,
  "Para você", ao lado de Busca e Biblioteca. A fileira de espaços aparece com a folha aberta. Na Estante sempre há
  uma rota (começa no Início do espaço).
- **Links de artista e álbum**: a `Faixa` ganha `album_id` e `artistas_ids` opcionais e o `Album` ganha
  `artistas_ids` (campos novos e opcionais, protocolo v1). O provedor do YouTube Music os preenche (o `rustypipe` traz
  os ids). Sem id, o nome não é link.
- **Parecidas**: a aba só aparece se o provedor declarar `Radio`. A F2 não cria o pedido de rádio (seria pedido novo
  no protocolo); fica para a F8 ou para quando um provedor precisar.
- **Playlists do Player com faixas de vários serviços** (acertado na conversa): `playlist_itens` passa a guardar
  também faixas de provedor (a chave `servico:id`; o que mostrar delas fica numa tabela nova, `faixas_remotas`, sem
  mexer nas tabelas antigas). Essas playlists moram em
  **Tudo**; em **Computador** aparecem só as que têm apenas músicas locais. Cada linha leva o selo do serviço.
- **Fila, Histórico, Loja e Configurações**: Fila e Histórico viram rotas de Tudo, abertas de qualquer espaço (Fila
  pelo botão do dock, como hoje; Histórico pela Biblioteca de Tudo). Loja e Configurações ficam no pé da lateral da
  Estante (ícones com dica), como no trilho de hoje; no modo gaveta, no pé do trilho de ícones; no empilhado, numa aba
  "Mais".
- **Fixar**: entra. A tabela `salvos` tem a coluna `fixado`; fixar e desafixar ficam no hover do item (o círculo de
  40 px do Figma) e no comando; os fixados vêm primeiro na lateral.

## 6. Partes (ordem de entrega)

1. **Identidade e páginas dos provedores**: `identidade` na `Apresentacao` e os ids de álbum e artista nos itens (o
   provedor do YouTube Music manda os dele), `remoto.rs` pede início, álbum, artista e playlist e publica o resultado
   por rota, como já faz com a busca.
2. **Curtir, salvar e fixar** no acervo, e playlists do Player com faixas de provedor (migração), com testes.
3. **Espaços e navegação**: `Espaco`, `Rota`, pilhas, comandos, a paleta derivada e a transição de cor.
4. **As páginas** no layout Palco, com esqueleto, erro, capacidades e as micro-interações.
5. **O layout Estante**: o arranjo novo, o visual no trilho, expandir e encolher, o responsivo.
6. **Testes, cópia de teste e documentos** (`app/README.md`, `AGENTS.md`, `docs/plano-v2.md`, um `docs/f2-paginas.md`).

Cada parte termina com `cargo fmt`, `clippy` sem aviso, testes e, se mexeu na tela, a cópia de teste por
`PrintWindow`.

## 7. Deixado para quem constrói

- Nomes dos tipos e a divisão em arquivos (seguindo `ui/*.rs` e `app/*.rs`).
- Como a logo SVG é desenhada no GPUI (arquivo em disco + `img`, ou outro caminho), desde que nítida em 12–28 px.
- A fórmula exata da paleta derivada, dentro dos contrastes da seção 3.
- O esqueleto (formato e brilho) e os tempos finos das animações, dentro do que a seção 5.3 diz.
- Como o pedido de página no `remoto.rs` é cancelado ou reaproveitado quando a pessoa volta rápido (cache curto por
  rota é bem-vindo).
