# Sessão de conferência (fase 1 do plano v3)

O que só foi visto em foto ou em teste, para conferir com teclado, mouse e som de verdade. Leva uns 40 minutos. Marque cada
item com ✓ (funcionou), ✗ (não funcionou: diga o que aconteceu) ou — (não deu para testar).

**Antes de começar:** abra o Player da release nova (`D:/Rust/target/release/player.exe`), com o YouTube Music e o Spotify
ligados. Tenha à mão uma pasta com um MP3, um FLAC e um M4A seus (com capa embutida, de preferência) e o app de desktop do
Discord aberto.

## 1. O que mudou nesta fase

| | O que fazer | O que deve acontecer |
| --- | --- | --- |
| ☐ | No Início, aperte Tab várias vezes até o anel passar da parte de baixo da página | A página rola sozinha e o elemento com o anel fica sempre à vista. Shift Tab volta e rola para cima |
| ☐ | Na Estante, aperte W (página larga) e depois W de novo | O visual vira tira e a página desliza junto até a largura toda (sem pular); na volta, o mesmo ao contrário |
| ☐ | Janela estreita (uns 410 px), numa página que não seja o Início: aperte X | O visual cresce até a janela sem a forma dar um salto no começo |
| ☐ | Arraste a barra de volume além da ponta com a interface em 125% (`Ctrl +`) | Ela estica um pouco e volta ao soltar, do mesmo tamanho que em 100% |
| ☐ | Na página de uma faixa (com outra música tocando e a fila cheia), aperte Tocar | A faixa toca já; abra a Fila: o que vinha depois continua lá |
| ☐ | Clique no coração de uma faixa três vezes bem rápido | O coração termina cheio e não pisca no meio do caminho |
| ☐ | Biblioteca › Pastas: escolha uma playlist sua, clique em "Renomear", troque o nome e aperte Enter | O nome muda no chip e na lateral, e continua depois de fechar e abrir o app |
| ☐ | Abra um álbum (YouTube Music ou Spotify), de preferência um disco duplo | Os números são os do álbum (o disco 2 começa do 1 de novo, como no serviço) |
| ☐ | Abra uns 10 artistas seguidos (com foto larga) e volte ao Início | Nada muda na tela; a memória da placa de vídeo (Gerenciador de Tarefas › Desempenho › GPU) não fica subindo a cada artista |
| ☐ | Com o Player aberto, clique de novo no atalho dele (ou rode o `player.exe` outra vez) | Não abre uma segunda janela: a que já estava aberta vem para a frente, também se estava na bandeja |
| ☐ | Pause uma música do computador | O controle de mídia do Windows (o painel do volume, a tela de bloqueio) continua mostrando a capa |
| ☐ | Uma playlist do Spotify com mais de 2000 músicas, se tiver | No fim da lista aparece "Mostrando 2.000 de N músicas" |
| ☐ | Uma faixa silenciosa ou um arquivo de silêncio, pré-analisado | O BPM aparece como "—", e não 240 |

Não dá para provocar de propósito, mas fique de olho: se o Soloist cair no meio de uma música do Spotify, ela deve continuar
pelo YouTube Music **do ponto onde estava**, e não do começo. E com o Spotifast aberto ao mesmo tempo, o visual deve seguir só a
música do Player.

## 2. O que nunca foi visto funcionando

| | O que fazer | O que deve acontecer |
| --- | --- | --- |
| ☐ | Toque um MP3, um FLAC e um M4A seus, com som | Os três tocam limpos, sem estalo na troca, e o crossfade e o equalizador (Configurações › Reprodução) mudam o som |
| ☐ | Biblioteca › Pastas › adicionar pasta | Abre a janela do Windows para escolher a pasta, e as músicas entram |
| ☐ | Uma música com capa embutida | A capa aparece na lista, na barra de baixo e no controle de mídia do Windows |
| ☐ | Passe o mouse no ícone do Player na barra de tarefas e use os botões da miniatura (anterior, tocar, próxima) | Agem na música que está na tela |
| ☐ | Feche a janela com a opção de ir para a bandeja ligada e clique com o botão direito no ícone da bandeja | O menu aparece bonito, e "Mostrar" e "Sair" funcionam |
| ☐ | Com o Discord de desktop aberto, toque algo | O cartão no seu perfil mostra a música **com a capa** |
| ☐ | Escreva na busca com acento (ã, é, ç) e, se usar, com um IME (japonês, chinês) | As letras entram certas |
| ☐ | `Ctrl +` e `Ctrl −` | A interface cresce e diminui (90, 100, 110, 125%) |
| ☐ | `Ctrl Tab` e `Ctrl 1`, `Ctrl 2`... | Troca o filtro de serviço (Tudo, Neste PC, YouTube Music, Spotify) |
| ☐ | `Alt ←` e `Alt →`, e os botões de voltar e avançar do mouse | Voltam e avançam as páginas |
| ☐ | Configurações › Aparência › tema Papel, depois o Mono: passe pelo Início, uma busca, um artista e a barra de baixo | Tudo se lê bem, sem texto apagado nem cor estranha |

## 3. Depois

Mande a lista marcada. O que der ✗ entra como conserto antes da fase 2.
