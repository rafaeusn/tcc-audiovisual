# Sessão de conferência (fase 1 do plano v3)

O que só tinha sido visto em foto ou em teste. Em 26/11/2026 o Claude conferiu sozinho tudo o que dava sem mexer no mouse nem
no teclado do Rafael: numa cópia muda, fora da vista, com mensagens do Windows, fotos da janela e músicas de teste geradas
(MP3, FLAC e M4A de 40 s a 120 BPM, com capa embutida, e 20 s de silêncio). O que precisa de mão, ouvido ou de um programa
aberto ficou na parte 3. ✓ funcionou, ✗ não funcionou, — não deu para testar.

## 1. Conferido pelo Claude

| | Item | Como | O que se viu |
| --- | --- | --- | --- |
| ✓ | Tab rola até o focado | 22 Tabs no Início em 958 × 560 | A página rolou até "Tocadas há pouco", com o cartão focado inteiro à vista |
| ✓ | Página larga (W) | Fotos 70 e 170 ms depois do W, na ida e na volta | Em dois tempos, sem a página passar por cima do visual |
| ✓ | X no empilhado (410 px) | Fotos durante a expansão | A forma encolhe aos poucos, sem o salto do começo |
| ✓ | Esticão do volume | Puxar a barra além da ponta em 100% e 125% (1920 × 1080) | 7 px e 9 px: o mesmo na medida das telas; volta ao soltar |
| ✓ | "Tocar" na página da faixa | Fila com 4 músicas, tocando a 3ª; Tocar no "One More Time" | Ela entrou depois da de agora e a 4ª continuou na fila |
| ✓ | Coração sem piscar | Três cliques em menos de 100 ms, fotos entre eles | Vazio, cheio, vazio, cheio, e ficou cheio |
| ✓ | Renomear playlist | "+ Playlist", depois "Renomear" e "Corrida à noite" | A janela abre com o nome; chip, lateral e aviso mudam; continua depois de reabrir |
| ✓ | Número da faixa | O "White Album" (Spotify, 30 faixas) | O disco 1 vai até 17 e o disco 2 recomeça do 1 |
| ✓ | Fotos largas fora da placa | 4 artistas na versão de depuração | O registro diz "solto da placa" para cada fundo, e a memória cai 5 MB nessa hora |
| ✓ | Segunda cópia na mesma pasta | Cópia minimizada; outra aberta na mesma pasta | A segunda fechou em 0,2 s e a primeira saiu do minimizado (sem roubar o foco do Opera) |
| ✓ | Capa no controle de mídia do Windows | Leitura das sessões de mídia (WinRT) | A sessão do Player tem capa tocando e pausada |
| ✓ | Lista cortada em 2000 | Uma playlist pública de 9.999 músicas | "Mostrando 2.000 de 9.999 músicas" no fim (mas ver o defeito 1 abaixo) |
| ✓ | BPM no silêncio | O arquivo de silêncio | O BPM não aparece (antes era 240) |
| ✓ | MP3, FLAC e M4A | Tocar os três (mudo) | Tocam, a posição anda, 120 BPM certo e a fila segue sozinha para a próxima |
| ✓ | Capa embutida | Os três arquivos | A capa aparece na barra de baixo e no controle de mídia do Windows |
| ✓ | Botões da miniatura da barra | A mensagem que o Windows manda no clique | Tocar/pausar e próxima agem na música da tela |
| ✓ | `Ctrl +` e `Ctrl −` | Ctrl marcado só na fila de entrada da cópia | 100 → 110 → 125 → 110 → 100%, gravado nos ajustes |
| ✓ | `Ctrl Tab`, `Ctrl 1` | Idem | Passa ao próximo filtro (abre o Início numa página sem filtro); Ctrl 1 vai para Tudo |
| ✓ | `Alt ←/→` e os botões do mouse | Idem, e os botões X1/X2 | Voltam e avançam (a rolagem do álbum é lembrada) |
| ✓ | Texto com acento e Unicode | "ação", tecla morta + é, 日本語, 中文, ñ e um emoji | Tudo entra certo |
| ✓ | Temas Papel e Mono | Início, busca "daft punk" e a página do artista em 1440 × 900 | Tudo se lê; no Papel o véu deixa o nome escuro sobre a foto clara; o Mono deixa capas e selos cinza |

## 2. Achados

1. **Playlist muito grande do Spotify podia não abrir na primeira vez** (✓ consertado no mesmo dia). A de 9.999 músicas deu
   "Sem resposta do Spotify (demorou demais)"; em "Tentar de novo" abriu. Agora até 6 páginas ficam sempre em andamento (antes,
   levas de 4 que esperavam a mais lenta), e se o prazo acabar no meio vale o que chegou, com o total para o aviso "N de M"
   (só nas listas de faixas; a biblioteca continua dando erro). O total desconta as faixas que o Player esconde, para uma
   playlist inteira com episódios não parecer cortada. Conferido: a de 9.999 abriu em menos de 5 s na primeira vez, com as 2000.
2. **A memória da placa crescia com as capas** (✓ consertado). As capas das páginas também saem da placa, 30 s depois de saírem
   da tela (as fotos largas, 5 s). Conferido na versão de depuração: 263 capas soltas depois de sair da playlist enorme, e os
   mesmos 4 artistas foram de 304 para 316 MB (antes, de 384 para 1516 MB).
3. As capas das faixas de uma playlist enorme ficaram no desenho de espera. Na segunda vez apareceram: era só demora.
4. As músicas do computador aparecem sem capa nas listas e na busca (a capa embutida só aparece na barra e no Windows). Já
   estava no plano (fase 2).
5. No teste dos botões da miniatura, com a música da cópia pausada, a tela passou a mostrar o que tocava no Opera, e um
   tocar/pausar seguinte vai para ele. É o comportamento combinado ("quem toca vence"), mas vale lembrar ao testar.

## 3. Para o Rafael (precisa de ouvido, de mão ou de um programa aberto)

| | O que fazer | O que deve acontecer |
| --- | --- | --- |
| ☐ | Ouvir um MP3, um FLAC e um M4A seus, com som, trocando de uma para outra | Som limpo, sem estalo; o crossfade e o equalizador mudam o som |
| ☐ | Biblioteca › Pastas › adicionar pasta | Abre a janela do Windows para escolher a pasta, e as músicas entram |
| ☐ | Fechar para a bandeja e clicar com o botão direito no ícone | O menu aparece bonito, e "Mostrar" e "Sair" funcionam |
| ☐ | Com o Discord de desktop aberto, tocar algo | O cartão no seu perfil mostra a música **com a capa** |
| ☐ | Se usar, escrever com um IME (japonês, chinês) de verdade | A composição aparece e o texto entra certo |
| ☐ | Os botões da miniatura com o mouse de verdade | Agem na música da tela |
| ☐ | Fique de olho: o Soloist caindo no meio de uma música do Spotify | Ela continua pelo YouTube Music do ponto onde estava |
| ☐ | Fique de olho: o Spotifast aberto junto com o Player | O visual segue só a música do Player |
