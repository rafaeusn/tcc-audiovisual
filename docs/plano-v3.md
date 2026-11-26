# Plano v3: o que falta, do zero

Feito e **aprovado em 26/11/2026**. **Fase 1 feita em 26/11/2026** (o CI passa no GitHub; a conferência está em
[`conferencia-fase1.md`](conferencia-fase1.md), com 2 defeitos achados e consertados, e o que só o Rafael
confere). Substitui a tabela de fases do [plano v2](plano-v2.md) (seção 7)
a partir da F7. O v2 continua valendo como pesquisa: a varredura dos repositórios (seção 3), a arquitetura dos
provedores (seção 6) e os riscos (seção 8).

Por que um plano novo: as fases F0 a F6 e a remodelagem das telas (8 partes, 25/11/2026) mudaram o app por baixo. Parte
do que o v2 previa já está feita, parte ficou sem sentido, e cada fase deixou sobras espalhadas pelo `AGENTS.md`. Aqui
tudo vira uma lista só, em fases com começo e fim.

---

## 1. Onde estamos

**Feito:** F0 (código aberto), F1 (provedores), F2 (páginas), F3 (YouTube Music), F4 (Letras 2.0), F5 (a Ilha), F6
(Spotify, pelo Soloist ou pelo Match) e a remodelagem (medidas, busca única, Tudo misturado com selos, barra e menu
novos, visual no centro da Estante, artista e álbum com imagem, teclado, tamanho da interface).

**O que a remodelagem já resolveu do v2:**

| Do v2 | Situação |
| --- | --- |
| Uma busca em todos os provedores, com o selo em cada item | Feito (parte 2 e 3) |
| Uma biblioteca, com "Neste computador" junto dos serviços | Feito: "Neste PC" é um filtro, como os serviços (parte 2) |
| Mixes (playlists com faixas de vários serviços) | Feito (F2 e parte 3) |
| Agrupar a mesma música de dois serviços | Feito: um `Junto` com dois selos, e "Não são a mesma música" (parte 3 e 4) |
| Densidade, tamanho do texto e zoom | Feito: o tamanho da interface, 90 a 125% (parte 8) |
| Molas no `anim.rs` | Feito (F5) |
| Artista com abas, foto larga, ouvintes e "Sobre" | Feito (parte 6) |
| "Neste computador" como um provedor igual aos outros (F1) | **Sai do plano**: virou o filtro "Neste PC", que já faz o papel |
| Os layouts Expressivo e Compacto | **Sai do plano**: o tamanho da interface e a Estante cobrem o que eles queriam. Layouts novos voltam como itens da Loja (fase 8) |
| Downloads offline cifrados (F12) | **Sai do plano**: a regra 2 não deixa salvar faixas de catálogo em arquivo. Fica só o cache que já existe, para tocar sem intervalo e pré-analisar |
| Tocar o Spotify pelo librespot | **Fora** desde 23/11 (regra 2): o librespot fica só no login e nos dados |

---

## 2. Regras para tudo o que vier

O que a remodelagem deixou como padrão, e que toda tela nova segue (detalhe no `AGENTS.md`, seção 4):

- Tamanhos só de `ui/medidas.rs`, com o `px` de lá (o tamanho da interface multiplica). Posições do mouse e das áreas
  medidas convertem com `medidas::logico`.
- Toda música na tela diz de onde vem (selo); serviço novo traz a marca e a cor na `Identidade`.
- Todo botão, aba, cartão e linha passa por `focavel` (o Tab chega nele).
- Vazio sempre com uma ação; carregando com esqueleto; erro com "Tentar de novo".
- Conferir na cópia muda em 1920, 1440, **958 × 560** (a janela do Rafael) e 410, e em 90 e 125%.

---

## 3. O inventário das sobras

Cada sobra tem uma fase (seção 4). As riscadas já foram resolvidas depois e só aparecem aqui para mostrar que não se
perderam.

### 3.1 Das fases F1 a F6

| Sobra | De onde | O que é | Fase |
| --- | --- | --- | --- |
| Página de podcast | F2 | Um programa com a lista de episódios, capítulos e velocidade | 3 |
| Rádio | F2, F3 | Tocar sem fim músicas parecidas a partir de uma faixa ou artista; a aba "Parecidas" da faixa hoje fica vazia | 3 |
| Conta do YouTube Music | F3 | Entrar na conta: biblioteca, curtidas e histórico de lá | 3 |
| Legendas do YouTube | F3 | Letra das versões ao vivo e dos clipes | 4 |
| Estilos e bolinha da letra em tela cheia | F4 | Escolher o estilo; a bolinha que anda nas pausas | 4 |
| Cartão para compartilhar | F4 | Um trecho da letra virando imagem com a capa | 4 |
| A letra no visual | F4 | A letra desenhada dentro do visual, e não por cima | 6 |
| Mais fontes de letra | v2 3.3 | Spotify (Musixmatch), NetEase, KuGou, QQ, AMLL, Genius (com os significados) | 4 |
| Canvas do Spotify | F6 | Os vídeos curtos em laço atrás das músicas | 6 |
| Letras do Spotify | F6 | A letra sincronizada do próprio Spotify | 4 |
| ISRC | F6 | O código de cada gravação; o Spotify não manda, e o casamento entre serviços é por nota | 2 (o Shazam devolve o ISRC) e 3 (sem perdas casa por ele) |
| Posição ao cair para o Match | F6 | Quando o Soloist falha no meio, a música recomeça do zero pelo YouTube Music | 1 |
| Duas instâncias do Player na mesma pasta; Soloist e Spotifast abertos juntos | F6 | Casos que podem confundir a captura por processo | 1 |
| Lista cortada em 2000 sem aviso; lista que falha inteira quando o serviço cai | F6 | Playlists enormes e quedas no meio | 1 |
| A faixa de provedor baixada duas vezes | F1 | O tocador e a pré-análise abrem cada um a sua conexão | 3 |
| "Tocar" na página da faixa troca a fila inteira | F2 | Deveria tocar a faixa e manter a fila (ou perguntar) | 1 |
| Curtir e descurtir rápido pisca o coração | F2 | Antes de o serviço confirmar | 1 |
| Ids de linha que colidem com mais de 10 artistas ou 100 faixas | F2 | Um bug de tela em listas grandes | 1 |
| ~~Lista longa de playlist sem `uniform_list`~~ | F2 | ~~Resolvido na F6 (`janela_de_linhas`)~~ | — |
| ~~A foto do artista aberto por um link~~ | F2 | ~~Resolvido na parte 6~~ | — |
| ~~O visual estica durante a expansão~~ | F2 | ~~Resolvido na parte 5 (recorte)~~ | — |

### 3.2 Da remodelagem

| Sobra | Parte | O que é | Fase |
| --- | --- | --- | --- |
| Arrastar para reordenar a fila | 4 | Hoje só pelo menu (subir, descer) | 5 |
| X a partir do visual baixo dá um salto | 5 | No empilhado, a forma muda de tamanho de repente | 1 |
| Página larga sem animar a largura | 5 | Só o visual anda; a página pula | 1 |
| A biografia em inglês | 6 | O Spotify manda no idioma dele; dá para pedir em português ou traduzir pela Unison | 3 |
| O número da faixa no protocolo | 6 | O álbum numera pela ordem; discos duplos recomeçam errado | 1 |
| Soltar as fotos largas da placa de vídeo | 6 | Cada artista aberto deixa a foto na memória da placa | 1 |
| Rolar até o elemento focado | 7 | Com o Tab, o focado pode ficar fora da tela | 1 |
| Ações do mouse fora do Tab | 7 | Tocar a seguir e pôr na fila aparecem só com o mouse em cima (estão no ⋯) | 1 |
| BPM 240 numa música silenciosa | 7 | A análise inventa ritmo no silêncio | 1 |
| O esticão da barra de volume fora de 100% | 8 | Medido em pixels de verdade, sai um pouco maior | 1 |

### 3.3 De antes da v2

| Sobra | O que é | Fase |
| --- | --- | --- |
| Renomear playlist | Hoje não dá para trocar o nome | 1 |
| Arrastar para reordenar playlists | Como a fila | 5 |
| Capas nos álbuns do computador | Os álbuns locais aparecem sem capa | 2 (o reconhecimento traz a capa) |
| Capa da música local no controle de mídia do Windows | `cover_url: None` | 1 |
| ReplayGain de álbum | Igualar por disco, mantendo as diferenças entre as faixas | 5 |
| R128 | Medir o volume de quem não tem etiqueta | 5 |
| Compassos que não são 4/4 (*Beat This!*) | A batida forte cai no lugar errado em valsa e afins | 2 |
| Importar histórico | Do Spotify, Last.fm e Google | 9 |
| Instalador, CI, medições, teste com usuários | O acabamento | 1 (CI) e 10 |

### 3.4 Dívidas e o que nunca foi visto funcionando

| Item | Fase |
| --- | --- |
| Sem CI (nada roda os testes sozinho) | 1 |
| O teste de desenho do visual travou uma vez com um jogo aberto | 1 (prazo no teste) |
| O exe de teste dos provedores bloqueado pelo erro 4551 na bateria inteira | 1 (documentar o jeito; não mexer na segurança do Windows) |
| O medidor de flashes do modo seguro | 6 |
| O Spectral do mockup é o antigo | 6 |
| **Nunca visto funcionando:** a capa no Discord; o som de verdade do player local com MP3, FLAC e M4A; a janela de escolher pasta; a capa embutida das músicas locais; os botões da miniatura da barra de tarefas; o menu da bandeja; digitar com IME num teclado de verdade; `Ctrl +`, `Ctrl −`, `Ctrl Tab`, `Ctrl 1..9` e `Alt ←/→` de verdade; o tema Papel e o Mono nas telas novas | 1 (uma sessão de conferência com o Rafael no teclado e no mouse) |

---

## 4. As fases

Tamanho: **P** (alguns dias), **M** (uma a duas semanas), **G** (mais de duas semanas). Cada fase tem desenho, plano e
partes, como a remodelagem.

| # | Fase | O que entra | Tamanho |
| --- | --- | --- | --- |
| **1** | **Arrumar a casa** | CI no GitHub (fmt, clippy e testes no Windows); a sessão de conferência (3.4); os consertos pequenos de 3.1 e 3.2 marcados com 1 (rolar até o focado, BPM no silêncio, fotos largas, número da faixa, a posição ao cair para o Match, a página da faixa, o coração que pisca, ids que colidem, renomear playlist, a capa local no Windows, as duas animações da Estante, o esticão do volume) | M |
| **2** | **Modo Escuta preciso + reconhecer** (a F7 do v2) | Reconhecer pelo som (o algoritmo do SongRec) ou pelo SMTC; achar a gravação num provedor ou no computador; pré-analisar; alinhar a análise com o som ao vivo (realinhar a cada 10 s e quando a posição pula); conduzir o visual pelo modo arquivo; voltar ao ao vivo se a confiança cair. Junto: o botão "o que está tocando?" na Ilha e no topo; etiquetar e pôr capa nas músicas locais sem etiqueta; o ISRC das faixas reconhecidas; compassos que não são 4/4 | G |
| **3** | **Mais provedores** (a F11 do v2) | Em partes: o cache de pedaços compartilhado (a faixa baixada uma vez); rádio (a aba Parecidas e "Tocar rádio" no menu); SoundCloud; rádios ao vivo (Radio Browser); podcasts (RSS e YouTube Music) com a página de podcast; a conta do YouTube Music; sem perdas (experimental, casando por ISRC); Deezer; a biografia em português | G |
| **4** | **Letras 3** | Fontes novas (Spotify, NetEase, KuGou, QQ, AMLL, Genius com significados, legendas do YouTube); estilos e bolinha na tela cheia; o cartão para compartilhar; o editor de sincronia | M |
| **5** | **Som e fila** (a F8 do v2) | Arrastar para reordenar a fila e as playlists; crossfade no compasso; modo DJ (tom e energia); aleatório que espalha artistas; lixeira; repetir trecho A-B; timer de dormir; pular silêncio; velocidade e tom; ReplayGain de álbum e R128; AutoEq e EQ desenhável; a saída de som só do Player | M |
| **6** | **Visual** | MilkDrop (libprojectM) conduzido pelo nosso diretor; Canvas do Spotify e capas animadas como camada; a letra no visual; o medidor de flashes do modo seguro; o mockup com o Spectral novo | G |
| **7** | **Juntos e controle** (a F9 do v2) | Salas na rede local; controle pelo celular (QR); MCP; overlay para o OBS; passar o que toca para outro PC; salas pela internet e Chromecast depois | G |
| **8** | **Loja e personalização** (a F10 do v2) | Registro no GitHub com assinatura e atualização dos provedores; temas em arquivo; Material dinâmico pela capa; cores do papel de parede; plugins com permissões (e atividades na Ilha); Looks; editar a Início (arrastar prateleiras); layouts novos como itens da Loja | G |
| **9** | **Extras** (a F12 do v2) | Retrospectiva e cartões; backup num zip; atalhos globais; importar histórico; SponsorBlock; filtro de explícito; karaokê (separar a voz); skins do Winamp na janela mini; "Descobrir" em feed vertical | M cada |
| **10** | **Entrega** | Instalador e atualização automática; medições de desempenho (quadros, memória, CPU) e o relatório; teste com usuários; o material do TCC | M |

**De-para com o v2:** F7 → fase 2; F11 → fase 3; F4 (sobras) → fase 4; F8 → fase 5; o visual da F12 → fase 6; F9 → fase 7;
F10 → fase 8; o resto da F12 → fase 9; o acabamento → fases 1 e 10.

---

## 5. A ordem e o porquê

1. **Fase 1 antes de tudo, e curta.** O CI pega regressão nas fases grandes que vêm; os consertos são pequenos e já
   estão mapeados; a sessão de conferência fecha o que só foi visto em foto.
2. **Fase 2 (reconhecer) antes da 3 (provedores).** O reconhecimento devolve o ISRC, que o sem perdas da fase 3 usa
   para casar; e o Modo Escuta preciso é o diferencial do TCC.
3. **Fase 3 em partes independentes.** Cada provedor é um programa à parte (seção 6 do v2): dá para entregar o cache
   e o rádio primeiro e um provedor por vez.
4. **Fases 4 a 6 em qualquer ordem** depois da 3; a 6 (visual) é a mais ligada ao tema do TCC.
5. **Fases 7 a 9 por último**, e a 10 quando o Rafael decidir fechar a versão do TCC.

**Em paralelo:** a sessão de conferência da fase 1 depende do Rafael; o resto pode andar enquanto ela não acontece.

---

## 6. Riscos novos

| Risco | Cuidado |
| --- | --- |
| O Shazam muda o protocolo | O SongRec acompanha; o reconhecimento é opcional e o Modo Escuta continua sem ele |
| Alinhar a análise com o som ao vivo erra em versões diferentes (ao vivo, remix) | A confiança do alinhamento decide; abaixo dela, o visual volta ao ao vivo |
| SoundCloud e os servidores de sem perdas param de funcionar | Programas à parte; o Player segue sem eles |
| O MilkDrop (C++) no Windows | Compilar o libprojectM cedo, numa sonda, antes de desenhar a fase |
| O CI no Windows custa minutos | Rodar só no `main` e nos pedidos de mudança |
