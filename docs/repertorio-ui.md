# Repertório de UI/UX e auditoria do mockup

Estudo feito em 08/11/2026, antes da v4 do mockup. Reúne o que tiramos das skills de design, do redesign do Spotify
no Figma, das imagens de referência e de bibliotecas de micro-interações, mais uma auditoria medida no navegador
das telas atuais ([design-ui-v1.md](design-ui-v1.md)). No fim, uma proposta de direção para a v4.

Laboratório interativo das micro-interações: https://claude.ai/artifact/GHU8FDbNsR5riSLrEY4zbX (privado; compartilhar
pelo menu Share).

---

## 1. Fontes

**Skills instaladas** em `~/.claude/skills` (valem para qualquer projeto):

| Skill | O que trouxe para cá |
| --- | --- |
| `impeccable` (Paul Bakaus) | Modos de superfície, piso de qualidade, auditoria em 5 dimensões, crítica por heurísticas de Nielsen |
| `frontend-design` (Anthropic) | Tiques de interface gerada por IA, um elemento memorável por tela, plano de tokens antes do código |
| `web-design-guidelines` (Vercel) | Lista de regras de interface (foco, números tabulares, `…`, estados, toque) |
| `ui-ux-pro-max` | Prioridades por categoria (acessibilidade e toque primeiro) e base de 120 regras de UX |
| `taste-skill` | Regras contra cara de template; é voltada a sites, então usei só os princípios |
| `emil-design-eng` e mais 10 do Emil Kowalski | Quando animar ou não, curvas, durações, coreografia, gestos, quebrar a UI com dados reais |

Não rodei scripts das skills. O lançador da `impeccable` baixa um binário próprio, então li as referências direto.

**Referências visuais:**
- O redesign do Spotify por Juxtopposed, página Desktop com 16 telas. O link enviado é de outra conta do Figma.
  Abri o original público da Comunidade, que tem o mesmo nome; o Figma deixou uma cópia nos seus Rascunhos. As
  medidas exatas vieram depois, do CSS exportado das 16 telas (seção 2.1).
- As três imagens anexadas e o print do nosso modal do Last.fm.
- Catálogos: React Bits (33 textos, 38 animações, 48 componentes, 34 micro-interações e 67 fundos), players e
  visualizadores do 21st.dev, botões de play e pause no CodePen (Aaron Iker, Jhey Tompkins, Osmo), coleção Music
  Interfaces do Awwwards.

---

## 2. O que o redesign do Figma ensina

| Conceito | Como aparece lá | No nosso app |
| --- | --- | --- |
| **Duas camadas** | Barra lateral e topo direto no preto; o conteúdo é uma folha do mesmo preto, separada por um filete de 1 px e canto de 10. Fundo diferente só em estado: aba ativa e item selecionado | **Adaptar.** Hoje tudo é bloco sobre preto. Na v4, moldura e painel ficam no mesmo preto e se separam por filete; o palco é o próprio visual |
| **Biblioteca na barra lateral** | Categorias com ícone (Fixados, Playlists, Curtidas, Álbuns, Pastas...), sem capas | **Adaptar.** Barra de ícones que expande com nomes (como no Hyprland) e uma seção de fixados |
| **Navegação no topo** | Início, Descobrir e Busca como abas de largura fixa, alinhadas às colunas de baixo. Com a Biblioteca ativa, o destaque cobre a lateral inteira: aba e painel viram uma peça só | **Não** como abas, porque a barra de ícones já resolve. Ficam duas regras: o topo se alinha às colunas, e o item ativo se liga ao painel que ele abre |
| **Player como dock** | Faixa sólida de 81 px, raio 10, na cor da capa que toca e só sob a coluna de conteúdo; a lateral desce até o rodapé | **Já temos.** Na v4 ganha o tom do visual com teto de contraste (2.1), fica só sob palco e painel e mostra capa, título e artista, para a faixa não sumir quando uma página cobre o palco |
| **Controles à esquerda, faixa no centro** | Inverte o padrão do Spotify | **Não.** Transporte no centro é o que o usuário espera |
| **Trilho de informação** | Álbum e playlist com a lista à esquerda; à direita, um trilho de 320 a 400 px com capa, tags e créditos | **Sim.** É a função da coluna de módulos; na Biblioteca vira o detalhe da faixa |
| **Chips de gênero** | Funk, Disco, Soft Rock como metadado | **Adaptar** com dados nossos: BPM, energia média, "drop em 1:12" e tags do Last.fm |
| **Página da faixa** | Abas Letra, Créditos e Parecidas; ferramentas em coluna vertical | **Adaptar** como abas Mapa, Créditos e Sessão. Sem letra, por licença; o efeito de karaokê vai para o mapa de energia |
| **Descobrir em feed vertical** | Vídeo em retrato (390×612) com uns 150 px do próximo à vista | **Agora não.** A ideia de mostrar a borda do próximo serve para "A seguir" |
| **Brilho tirado da capa** | Capa ampliada e desfocada atrás do conteúdo, a 6%; folha tingida de 12 a 18%; degradê da cor da capa até o preto no topo das páginas | **Sim.** O dock leva o tom do visual; o painel, só um degradê fraco do mesmo tom no topo |
| **Silhueta e cor do item** | O formato diz o tipo: abas empilhadas acima da capa (duas na playlist, uma no álbum, uma lateral na pasta), círculo no artista, retrato no audiolivro. A contagem de faixas vem na cor do item | **Adaptar.** Um formato por tipo na Biblioteca e na Loja, e uma cor de assinatura por skin nas capas geradas |
| **Painel de amigos** | Outra folha, de 358 px, que liga e desliga; a folha de conteúdo encolhe em vez de ser coberta | **Não** (social fora do escopo). O padrão já é o nosso botão Módulos, com o mesmo cuidado: abrir o painel encolhe o palco (corrige A6) |
| **Biblioteca com fixar, grade, lista, ordenar** | Alfinete em cada item; alternar grade e lista; ações da linha só no hover | **Sim** na Biblioteca e na Loja |

Detalhes que valem copiar no espírito: ícones de traço uniforme de 2 px, que passam de contorno a preenchido
quando ativos; sublinhado de 2 px na aba ativa; a linha atual da letra acesa como karaokê, com as outras
apagadas; e um único verde para todas as ações principais.

### 2.1 O Figma em números

O CSS exportado das 16 telas Desktop (1771×1024) confirma quase toda a tabela e corrige quatro leituras: a folha
de conteúdo não é cinza, o dock não é translúcido, as faixas de cor ficam acima da capa (não na base) e o
Descobrir é retrato de 390×612, não 9:16. A tabela acima já está corrigida.

**Casca**
- Moldura e folhas usam o mesmo preto, `#060606`. O que separa é um filete de 1 px (`#202020`, 1,2:1) com raio
  10. Preenchimento só aparece em estado: `#111111` na aba ativa e `#202020` no item selecionado.
- Lateral de 259 px e conteúdo de 1491 px, com 7 px entre eles. A lateral desce até o rodapé; o dock (1491×81,
  raio 10) fica só sob o conteúdo.
- As abas do topo têm largura fixa e se alinham às colunas de baixo. Com a Biblioteca ativa, o destaque cobre a
  lateral inteira (259×1010).
- O painel de amigos é outra folha, de 358 px. Ao abrir, a folha de conteúdo encolhe de 1491 para 1125 px.

**Tokens**

| Grupo | Valores |
| --- | --- |
| Fundo | `#060606` na moldura e nas folhas; `#111111` no ativo; `#202020` no selecionado e no filete |
| Texto | `#E0E0E0` principal e `#898989` secundário |
| Acento | `#1ED760`, só em tocar, curtir, fixar, aba ativa e "tocando agora" |
| Cor derivada | Dock na cor da capa que toca; folha tingida de 12 a 18% no Descobrir e na faixa; degradê da cor da capa (47%) até o preto no topo de álbum, playlist e podcast; capa ampliada e desfocada atrás de tudo, a 6% |
| Tipo | Satoshi 400, 500, 700 e 900; corpos 12, 14, 16, 18, 20, 28, 30, 36 e 50; entrelinha de cerca de 1,35 em tudo e 1,75 no texto longo, em coluna de 750 px; espaçamento de +0,03em, menos no nome do artista |
| Raio | 3 nas miniaturas, 5 nas linhas, 8 nos chips do topo, 10 nas folhas, no dock e nas capas grandes, 44 nas pílulas, e círculo |
| Ícone | Caixa de 40, desenho de 24 e traço de 2; contorno quando inativo, preenchido quando ativo |
| Espaço | Recuo de 25 px nas folhas; 30 entre seções; 15 do título à fileira; 11 entre cards |

**Componentes**
- **A silhueta diz o tipo.** É o achado mais forte. Playlist tem duas abas empilhadas acima da capa (a 15% e a
  30% da cor do item), álbum tem uma e pasta tem uma aba lateral; artista é círculo, audiolivro é retrato e
  podcast ganha uma placa com barra lateral. Na tela Início, a contagem de faixas é a única cor de texto, na cor
  do item.
- **Card de 170 px:** capa com raio 7, título de 14 em peso 500 com a contagem à direita e descrição de duas
  linhas em 12.
- **Cabeçalho fixo:** 59 px de preto a 80% com desfoque de 50 px. Chips de 35 px; o ativo se inverte, claro com
  texto escuro.
- **Linha de lista de 67 px:** número, capa de 51, título de 16 e artista de 14. O coração de curtida fica sempre
  à vista; as outras ações só aparecem no hover.
- **Estados:** 50% de opacidade no desabilitado (a seta "anterior" no começo da fileira) e um círculo de 40 px,
  sobreposto ao canto da capa, para remover ou fixar.
- **Dock de 81 px:** à esquerda, play verde de 38 px, transporte, progresso de 3 px e volume; no centro, capa de
  51 com faixa, artista e álbum; à direita, as ações, um divisor de 2×36 e a fila.
- **Letra em karaokê:** a linha atual se acende da esquerda para a direita, com um degradê recortado no texto; as
  próximas ficam a 20%.
- **Prévia em segmentos:** no Descobrir, a prévia de playlist mostra quatro segmentos, como nos stories, com o
  atual preenchido.
- **Selo de ranking:** na página Sobre do artista, uma estrela de 132 px com a posição no mundo.

**Onde o próprio Figma falha.** Por isso levamos o conceito, não o número. Contrastes calculados pela fórmula da
WCAG 2:
- **Texto secundário no dock:** `#898989` sobre `#333842` dá 3,4:1, abaixo dos 4,5:1 exigidos para texto de 14
  px. A cor do dock não é o problema (luminância 0,039); o cinza é que é escuro demais. Nosso `--fg-2`
  (`#A7A7AD`) daria 4,9:1 no mesmo dock.
- **Duas cores de contagem reprovam** sobre o preto: `#DD138B` (4,38:1) e `#27856B` (4,49:1). As outras 18
  passam de 6:1.
- **Letra:** as próximas linhas, a 20%, ficam em 1,7:1. Funciona como efeito, não como texto para ler.
- **Deriva de instâncias:** caixas de ícone de 37,3 a 40,41 px; raios de capa entre 5,6 e 7,9; fontes de 13,21,
  14,08, 15,85 e 30,02 px. São componentes redimensionados à mão.
- **Folgas soltas:** 3, 4, 5 e 7 px entre as peças da casca; o dock fica a 10 px do rodapé em 13 telas e a 7 px
  em 3; cerca de 20 valores de espaço diferentes.
- **Nomes:** camadas chamadas "Saves" em itens diferentes e a aba "Compilattions".

**O que muda na v4** (já aplicado na seção 8):
1. **Contêiner ganha linha; estado ganha preenchimento.** Moldura e painel no mesmo preto, separados por um
   filete `#1F1F22`; hover em `#18181B`; seleção em `#222226`. Os módulos se separam por espaço e título, e a
   caixa só aparece no modo edição. É a resposta ao "cards em cards".
2. **O dock é a única faixa com cor:** o tom do visual, com teto de luminância relativa de 0,045, que deixa o
   `--fg-2` acima de 4,5:1. Na prática, até cerca de 25% do tom calmo ou 45% do agitado e do intenso sobre o
   preto. O tom segue a energia por uma média lenta e fica parado no modo seguro e com as cores reativas
   desligadas.
3. **No dock e na seleção, texto secundário é `--fg-2`, nunca `--fg-3`.** O `--fg-3` cai para 2,8:1 no dock com o
   tom no teto e para 4,0:1 sobre `#222226`; no hover passa raspando (4,5:1). O `--fg-4` (2,2:1 sobre o preto)
   fica só para desabilitado e decoração.
4. **Entrelinha fixa por corpo, em múltiplos de 4:** 12/16, 14/20, 16/24, 20/28 e 28/32; títulos de exibição
   apertados (44/44 e 88/80); texto corrido a 1,6, em colunas de até 68 caracteres. O Figma usa cerca de 1,35 até
   no nome de 50 px, o que deixa o título frouxo.
5. **Ícone ativo:** de contorno para pontos (seção 6), numa caixa de 40 px com desenho de 24 e traço de 2.
6. **Silhueta por tipo** na Biblioteca e na Loja: faixa em quadrado, pasta com aba lateral, skin em 16:9, módulo
   em bloco, perfil e serviço em círculo com a marca.
7. **Progresso por seções:** o mapa de energia do dock se divide pela estrutura da faixa (intro, verso, refrão,
   drop), e a seção atual se acende como a letra em karaokê. Com arquivo local, as seções vêm da pré-análise; no
   Modo Escuta, aparecem conforme a faixa toca.
8. **Ações da linha** aparecem no hover e no foco do teclado; com tela de toque, ficam sempre à vista.
9. **Cabeçalho fixo:** o desfoque por elemento não é garantido no GPUI 0.2. O plano B é uma faixa opaca com a
   borda de baixo esmaecida.
10. **Selo raro em Estatísticas:** anel de pontos com o número em Doto, para marcos como 100 horas ouvidas.

Fica a nossa escala de 7 tamanhos; o Figma usa 9 corpos e ainda deriva.

---

## 3. O que as imagens anexadas ensinam

| Imagem | Ideia | No nosso app |
| --- | --- | --- |
| Painel colorido (1) | Cartões de cor chapada com recorte; painel que sai da moldura; anel de progresso no avatar e no pause da linha ativa | Anel de progresso no play do **mini player**. Recortes não: no Modo Escuta só temos a capa |
| Player escuro (2) | Arte grande com faixa de vidro; fila com abas; botões Minimizar e Tela cheia; itens fixados como miniaturas na barra | **Mini player** (janela compacta sempre no topo, ótimo no Windows) e **fixados na barra** (skins e perfis) |
| The Weeknd (3) | Nome gigante sobre foto que sangra; linha que toca em verde com barras de equalizador; contextos recentes no topo | Barras de equalizador na linha que toca, **movidas pela energia real**. O título gigante já é nossa assinatura |
| Nosso modal (4) | Ícones fora do centro e caixas quase invisíveis | É um bug de CSS, detalhado na auditoria (A5) |

---

## 4. Princípios das skills, aplicados aqui

- **O app tem dois modos.** O visual é experiência: pode ousar. A moldura é operação: precisão, padrões
  conhecidos, nada estranho sem motivo. A ousadia fica no visual, no título condensado e nos pontos de energia; o
  resto fica quieto.
- **Uma coisa memorável por tela.** Na tela principal é o título sobre o visual. Os módulos não competem com ele.
- **Tirar os tiques de IA ainda presentes.** São 19 textos com ponto do meio ("Spectral · automático", "Ao vivo ·
  só este app", "Chopin · Musopen"). Monoespaçada só em tempo e dados. Nenhum rótulo acima de título.
- **Escala tipográfica fixa e curta.** Para interface de operação, razão de 1,125 a 1,2. Hoje são 11 tamanhos,
  seis deles entre 12 e 15 px.
- **Raio concêntrico.** Raio de dentro = raio de fora menos o recuo. Hoje são 10 raios diferentes.
- **Frequência decide a animação.** Nada anima em atalho de teclado ou na paleta de comandos. Pressionar: 100 a
  160 ms. Menus: 150 a 250 ms. Painéis: 200 a 300 ms. A saída é mais rápida que a entrada.
- **Mola para o que se arrasta.** Começa do valor atual, herda a velocidade do gesto, resiste na borda e pode ser
  interrompida a qualquer momento.
- **Vidro só sobre o visual.** Escurecer o fundo quando a tarefa exige foco (modal); separar sem escurecer quando
  é paralela (painel). Esmaecer a borda do conteúdo rolável em vez de linha divisória.
- **Causa e efeito.** Se a interface pulsa com a música, o pulso sai do áudio, no mesmo quadro, e respeita o modo
  seguro.
- **Testar com dados ruins.** Exemplo: "Tchaikovsky: Sinfonia nº 6 em si menor, Op. 74 'Patética': IV. Finale".
  Também listas com 0 ou 1 item ("1 faixa", não "1 faixas") e faixa sem capa.
- **Risco do GPUI.** A versão 0.2 ainda não tem acessibilidade para leitor de tela. Foco visível e navegação por
  teclado ficam por nossa conta.

---

## 5. Micro-interações

Cada candidata passou pelos quatro filtros do Emil: frequência, propósito, velocidade e função. Todas dão para
fazer no GPUI, desenhando no `canvas` e animando quadro a quadro.

| # | Interação | Onde | Propósito | Receita | Origem |
| --- | --- | --- | --- | --- | --- |
| 1 | Botão afunda ao pressionar | Todos os botões | Retorno | Escala 0,97 em 120 ms; volta em 160 ms | Emil, Apple |
| 2 | Play e pause se transformam | Dock, tela cheia | Estado | Morph do desenho com 2 px de desfoque no meio; 180 ms | CodePen |
| 3 | **Mapa de energia como barra de tempo** | Dock | Retorno e estado | Passar o mouse mostra tempo e nível; colunas perto do cursor crescem; arrastar 1:1 e resistir nas pontas | React Bits (Wake Slider, Scrub Field), Apple |
| 4 | Volume elástico | Dock | Retorno | Puxar além do limite estica e volta com mola; roda ajusta 2%; valor aparece só ao arrastar | React Bits (Elastic Slider) |
| 5 | Números que rolam | BPM, relógio, contagens | Evitar troca brusca | Dígitos Doto sobem ou descem só quando o valor muda; 220 ms | React Bits (Count Up, Split Flap) |
| 6 | Troca de faixa | Título do palco e dock | Evitar troca brusca | Título sai com desfoque e desce 8 px; o novo entra; artista 40 ms depois; 260 ms | Coreografia do Emil |
| 7 | Instalar | Loja | Estado | Botão vira anel de progresso e depois um check verde; 200 ms por etapa | React Bits (Status Mark) |
| 8 | Segurar para confirmar | Desinstalar, Restaurar layout | Evitar engano | Preenche em 1,2 s linear; soltar volta em 200 ms | Emil, React Bits (Hold Button) |
| 9 | Seletor com marcador que desliza | Abas da Loja, Cores e Seguro, Faixas e Pastas | Continuidade | Marcador desliza 200 ms; sem efeito elástico, porque o uso é frequente | React Bits (Rubber Segment), sem o elástico |
| 10 | Aviso que se arrasta para fechar | Avisos | Continuidade | Sai pela mesma borda; um gesto rápido (velocidade acima de 0,11 px/ms) fecha | Sonner, React Bits (Swipe Toast) |
| 11 | Barras de equalizador na linha que toca | Biblioteca, Nesta sessão | Estado | Três barras seguem a energia real; param no pause; ficam paradas com movimento reduzido | Imagem 3 |
| 12 | Pulso no tempo forte | Ponto de energia, anel do logo | Estado | Um pulso a cada compasso, não a cada batida; desligado no modo seguro | Nosso |
| 13 | Módulo levanta no modo edição | Modo edição | Continuidade | Escala 1,02 e sombra ao pegar; vizinhos abrem espaço com mola sem quique | Apple, Emil |
| 14 | Página nasce do ícone | Biblioteca, Loja | Continuidade | Folha cresce a partir do ícone da barra; desfoque e escala juntos; 240 ms | Apple (materiais) |

**Recusadas:**
- **Animar a paleta de comandos:** é acionada pelo teclado e usada o dia todo, então anima nunca.
- **Entrada em cascata nas listas da Biblioteca:** frequência alta; a lista aparece pronta.
- **Cursor personalizado** (Splash, Blob e Target Cursor): é decoração numa superfície de operação e tira a atenção
  do visual.
- **Fundos animados do React Bits** (Aurora, Galaxy, Hyperspeed): o fundo já é o nosso visual.
- **Cartões inclinados e holográficos:** decoração que pode causar enjoo.
- **Texto embaralhado ou com glitch:** fantasia de "tecnologia", e o glitch pisca.
- **Galerias 3D e carrosséis circulares:** atrapalham a leitura.
- **Dock que amplia ao passar o mouse na barra de ícones:** a navegação é usada o tempo todo, e a ampliação
  atrasa a mira.

**Laboratório:** https://claude.ai/artifact/GHU8FDbNsR5riSLrEY4zbX. Ele demonstra as 14 interações e o ícone ativo
em pontos, com chaves para câmera lenta (4 vezes), movimento reduzido e modo seguro.

---

## 6. Identidade

**Temos:**
- o anel de pontos;
- os números em matriz;
- o mapa de energia em pontos;
- o título condensado;
- as capas geradas;
- as três cores de energia.

**Falta:**
- **Assinatura de movimento.** "Tudo nasce de um ponto": o carregamento é o anel do logo se completando; as páginas
  crescem a partir de um ponto do ícone; o pulso do tempo forte é o mesmo ponto.
- **Ícone ativo em pontos.** Hoje o ativo é o mesmo ícone preenchido, como em qualquer app. Na v4, o ícone ativo
  vira uma versão em matriz de pontos, a mesma linguagem dos números.
- **Superfícies.** Sair da "sopa de blocos" para moldura, palco e painel (seção 8).
- **Voz.** Tirar os pontos do meio e os textos técnicos ("corpus de calibração do diretor") das telas do usuário.
- **Cor de assinatura por skin**, nas capas e no anel do logo.

---

## 7. Auditoria

Medida no navegador em 1440×900, 1280×720, 1024×700, 768×1024 e 390×844, nas telas Principal, Loja, Biblioteca,
Ajustes, Modo edição e no modal. Um script verificou vazamentos, texto cortado, alvos menores que 24 px, contraste
e sobreposição.

| Dimensão | Nota (0 a 4) | Principal achado |
| --- | --- | --- |
| Acessibilidade | 3 | Nenhuma falha de contraste e nenhum alvo pequeno; o GPUI ainda não fala com leitor de tela |
| Desempenho | 3 | Visual em WebGL estável; o mockup não mede o app real |
| Temas | 3 | Tokens usados e tema Monocromático funcionando; alguns valores ainda soltos no código |
| Responsivo | 1 | Quebra em 1280×720, some abaixo de 1060 px, corta a tabela, vaza sob o dock no celular |
| Integridade visual | 2 | Sopa de blocos, 11 tamanhos de fonte, 10 raios, controles duplicados, pontos do meio |
| **Total** | **12/20** | Aceitável: precisa de trabalho antes de virar código |

**Achados, do mais grave ao mais leve:**

| # | Peso | Problema | Evidência | Correção |
| --- | --- | --- | --- | --- |
| A1 | P1 | Em 1280×720, o módulo "Nesta sessão" fica com 48 px e a lista some | Linhas fixas de 176, 158 e 124 px na coluna | Altura pelo conteúdo e consulta por altura: abaixo de 780 px a capa vira uma faixa de 72 px e a lista rola com borda esmaecida |
| A2 | P1 | Abaixo de 1060 px os módulos desaparecem sem substituto | Coluna escondida pela consulta de largura | Os módulos viram uma gaveta aberta pelo botão Módulos do dock, ou uma faixa compacta sob o palco |
| A3 | P1 | Quando uma página cobre o palco, a faixa que toca some da tela | Título só existe no palco | Dock com capa, título e artista ao lado da fonte, sempre visível |
| A4 | P1 | A Biblioteca corta a coluna Licença em 1280 px | "Domínio p", "CC BY-SA" cortados | Colunas com prioridade: Licença vai para o detalhe abaixo de 1100 px; Gênero some abaixo de 900 px |
| A5 | P1 | No modal do Last.fm, os ícones ficam 9 px fora do centro e as caixas quase não aparecem | A regra `.perm span` também pega a caixa do ícone e troca `grid` por `block`; contraste da caixa 1,1:1 | Classes próprias para os textos da linha e caixa com fundo visível, ou sem caixa |
| A6 | P2 | Em 1280 px, Ajustes flutua sobre o título do palco | Painel cobre "Vidro Notur…" | Ajustes ocupa o lugar da coluna de módulos, com a mesma largura; o visual continua à vista |
| A7 | P2 | No modo edição, as etiquetas cobrem o conteúdo dos módulos | "Ritmo" sobre as batidas, "Relógio" sobre a data, "Visual" sobre "Cores" | Etiqueta no lugar do título do módulo, não por cima do conteúdo |
| A8 | P2 | Em 390 px, o conteúdo aparece por baixo e abaixo do dock | "128 BPM" visível sob o player | Espaço no fim igual à altura do dock mais a margem segura, e borda esmaecida sob ele |
| A9 | P2 | Em 1280×720, a Loja mostra só um item acima da dobra | Cabeçalho ocupa cerca de 45% da folha | Cabeçalho compacto em telas baixas, com abas fixas no topo |
| A10 | P2 | Em 768 px, o texto da busca é cortado | "Buscar ou executar um co…" | Só "Buscar" e o atalho; texto completo a partir de 1100 px |
| A11 | P2 | Controles duplicados | Tela cheia no palco e no dock; ajustes no palco e na barra; "Início" no topo repete a barra | Um lugar para cada: tela cheia no dock; o botão do palco abre direto a aba Visual; o topo mostra o nome só quando há página aberta |
| A12 | P3 | 11 tamanhos de fonte | 12; 12,5; 13; 13,5; 14; 15; 21; 24; 42; 54; 88 | Escala de 7: 12, 14, 16, 20, 28, 44, 88 |
| A13 | P3 | 10 raios de canto | 4, 6, 8, 12, 14, 16, 18, 22, pílula, círculo | Quatro: 28 (palco e folhas), 16 (módulos), 10 (controles), pílula; mais o círculo |
| A14 | P3 | Desalinhamentos pequenos | Chip do palco em 124 px e título em 128 px; busca em 763 px e transporte em 761 px | Mesma margem esquerda no palco e o mesmo eixo para busca e transporte |
| A15 | P3 | 19 textos com ponto do meio | "Spectral · automático", "Chopin · Musopen" | Reescrever: estado vira ícone, autor e fonte em duas linhas |
| A16 | P3 | Relógio como módulo padrão é decoração | Não ajuda a ouvir | Vira módulo opcional da Loja; o lugar padrão fica para "A seguir" ou "Estrutura da faixa" |

**O que funciona:**
- Zero falha de contraste nas seis telas.
- Nenhum alvo de clique menor que 24 px.
- Nenhum vazamento horizontal.
- Ícones coerentes.
- Tema Monocromático correto.
- Significado claro das cores de energia.
- O título condensado é uma assinatura forte.

---

## 8. Direção proposta para a v4

**Tese espacial: moldura preta, palco vivo, painel de leitura.**

```
┌──────┬────────────────────────────────────────────┬───────────────┐
│  ◎   │          [ Buscar             Ctrl K ]    │       ─ □ ✕   │
│      │ ┌──────────────────────────────┐ ┌───────────────────────┐ │
│  ⌂   │ │                              │ │ Ritmo        Energia  │ │
│  ♫   │ │           palco              │ │ Estrutura da faixa    │ │
│  ⬡   │ │                              │ │ A seguir              │ │
│ ──── │ │  Vidro Noturno               │ │                       │ │
│ fixos│ │  Coletivo Sal                │ │                       │ │
│  ⚙   │ └──────────────────────────────┘ └───────────────────────┘ │
│      │ [capa] Vidro Noturno   Spotify    ⏮  ⏯  ⏭   mapa   🔊      │
└──────┴─────────────────────────────────────────────────────────────┘
```

- A barra de ícones e a busca ficam direto no preto. A barra desce até o rodapé; o dock fica só sob palco e
  painel.
- Nada é bloco. O painel é o mesmo preto da moldura, separado por um filete; o palco é o próprio visual; o
  preenchimento fica só para estado (hover, seleção, item ativo).
- O dock é a única faixa com cor, o tom do visual com teto de contraste, e passa a mostrar a faixa.
- O painel troca de conteúdo (Módulos, Ajustes, detalhe da Biblioteca). Acima de 1060 px, abrir o painel encolhe
  o palco em vez de cobri-lo.
- Abaixo de 1060 px, o painel vira gaveta.
- Abaixo de 760 px, tudo empilha e o dock gruda embaixo com espaço reservado.

**Tokens a consolidar:**

| Token | Valor |
| --- | --- |
| Tipografia | Instrument Sans em 12/16, 14/20, 16/24, 20/28, 28/32, 44/44 e 88/80; números em Doto; tempo em Fragment Mono |
| Texto | `#F2F2F3` principal; `#A7A7AD` secundário, o mínimo no dock e na seleção; `#808087` terciário, fora do dock e da seleção; `#46464C` só em desabilitado |
| Raios | 28, 16, 10, pílula e círculo |
| Espaço | Base 4: 4, 8, 12, 16, 24, 32, 48 |
| Superfícies | Moldura e painel `#000`, separados por filete `#1F1F22`; hover `#18181B`; seleção `#222226`; menus e modais `#111113`; o palco é o próprio visual; o dock leva o tom do visual até luminância 0,045 |
| Movimento | Pressionar 120 ms; menus 200 ms; painéis 240 ms; molas sem quique; saída mais rápida que a entrada |

**Entram na v4:** as correções A1 a A16, as micro-interações 1 a 14, as dez mudanças da seção 2.1, o ícone ativo
em pontos, as barras de equalizador reais, o tom do visual no dock e no painel, e o mini player.

**Feito em 08/11/2026.** O mockup v4 está no mesmo link e o resumo em [design-ui-v1.md](design-ui-v1.md). A
auditoria refeita no navegador, em 1440×900, 1280×720, 1024×700, 768×1024 e 390×844, não achou vazamento horizontal,
texto cortado, alvo menor que 24 px nem texto abaixo de 4,5:1 (fora o conteúdo esmaecido de propósito no modo
edição). Ficaram só os 7 tamanhos de fonte e nenhum ponto do meio. Em 1280×720, "Nesta sessão" passou de 48 para
204 px e o cabeçalho da Loja, de 45% para 20% da folha. Mudanças de rota na construção:

- O tom do dock ficou em luminância 0,016, não 0,024, e dessaturado 30%: no teto o vermelho do drop pesava demais.
- Ajustes perdeu a aba Sobre, que virou um botão de informação; três abas cabem no painel de 340 px.
- O Relógio voltou como módulo opcional, adicionado pela Loja.
