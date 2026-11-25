# F6: o Spotify no Player

O desenho completo (com o que foi pensado e descartado) está em
[`superpowers/specs/2026-11-23-f6-spotify-design.md`](superpowers/specs/2026-11-23-f6-spotify-design.md); a seção 8 dele é a
que vale para tocar.

## O que é

O `provedor-spotify` (`app/provedores/spotify`) é um programa à parte que vem com o Player. Ele cuida de três coisas:

1. **A conta** (parte 1): a pessoa entra pelo navegador (`oauth.rs`, o nosso fluxo com PKCE); o refresh token fica cifrado (DPAPI).
2. **Os dados** (parte 2): busca, início, páginas e biblioteca, pelo Pathfinder, com a sessão do librespot.
3. **O Soloist** (parte 4): o player oficial do Spotify, sem janela, se a pessoa o ligar.

O librespot fica só na sessão e nos dados. Nada pede chave de áudio nem decifra faixa do Spotify.

## Por onde uma música do Spotify toca

`pares::quem_toca` decide, a cada faixa:

| Situação | Quem toca |
| --- | --- |
| O Soloist está pronto (ou abrindo) | O Soloist. O som sai por ele; o Player manda e acompanha |
| Sem o Soloist (desligado, baixando, pareando, falhou), ou ele não tocou esta faixa | O **Match**: a mesma música num serviço com busca e áudio (o YouTube Music) |
| Nenhum dos dois | Ninguém: o Player avisa para ligar o YouTube Music na Loja |

### O Match

- A busca é o título sem o ruído ("Remastered", "Official Video") e o primeiro artista; o casador da F1 escolhe (nota a
  partir de 0,6). O casador ganhou pisos: título ou artista diferentes não passam só porque a duração bate, e "ao vivo",
  karaokê, acústico, remix, cover e acelerada contam como outra gravação quando só um dos lados diz.
- O par fica em `<dados>/pares.json` (ids e títulos, nunca áudio). A página da faixa diz por onde ela toca e tem "Procurar
  outra gravação", que recusa a de agora e pega a seguinte; sem outra que sirva, fica a de antes, com aviso.
- A pré-análise é da gravação que toca (a chave do par), e não da faixa do Spotify.
- Fila sem intervalo, crossfade, equalizador e pré-análise valem como para qualquer faixa do YouTube Music.

### O Soloist

- Documentação: developer.spotify.com/documentation/soloist. Precisa de conta Premium e da chave pessoal da pessoa.
- **Ligar**: Configurações › Serviços › Spotify. "Criar a chave no Spotify" abre a página; "Colar a chave copiada" lê a área de
  transferência e manda direto ao provedor (a chave não aparece na tela, no registro nem em erro; fica em
  `soloist.dpapi`, cifrada). O provedor baixa o programa de `soloist-builds.spotifycdn.com` (só https, sem redirecionar, com
  limite de tamanho) e o abre.
- **Parear**, uma vez: com o app do Spotify na mesma rede e conta, tocar uma música e escolher "Player (Soloist)" na lista de
  aparelhos.
- O programa fica aberto enquanto ligado e recebe `play` com a faixa pela API local (WebSocket em 127.0.0.1, porta sorteada).
  Ele morre com o provedor (grupo de processos do Windows) e reabre sozinho até 3 vezes se cair.
- **O fim da faixa**: perto de 1 s antes dele o Soloist diz `buffering` e a posição para, mas o som segue. O Player conta esse resto
  (`leitura::na_cauda`), dá a faixa por acabada quando o som acaba e pede a seguinte. O que o Spotify emendar sozinho (a fila é do
  Player) é calado.
- **Sem intervalo**: a 10 s do fim o Player manda a próxima (`preparar`), e o provedor a põe na fila do Spotify (`add_to_queue`), só se
  a fila estiver vazia: dali nada se tira, e o que já estivesse lá tocaria antes. O Soloist passa para ela sozinho e a fila do Player
  anda junto (`Local::avancou_no_externo`). Se a pessoa pula ou para antes, o `skip_next` a consome (senão ela entraria no meio das
  outras depois; pulando para ela mesma, ele basta, e ela começa na hora); se a fila do Player muda nesses segundos, toca a de agora. A próxima adiantada aparece na fila do celular. Se ela
  não entra em 1,5 s, ou se é outra, o Player a pede como antes: aí sobram de 0,7 a 0,9 s (o que o `play` leva para soar).
- **Outro aparelho**: a conta toca num lugar só. Se o Spotify passa a tocar em outro lugar, o Player pausa e avisa; continuar traz
  a faixa de volta de onde parou. Se o Spotify (ou o Spotifast) toca neste computador quando a pessoa dá o play, o Player avisa
  que ele para.
- **Vence em 90 dias**: a partir de 60 o provedor procura outro uma vez por dia; vencido (código de saída 10), baixa na hora.
- **Build quebrado**: o Spotify já publicou builds que caem ao tocar no Windows (o 1.4.0.7, de 23/11/2026, é um e nem é aberto;
  ver [a investigação](spotify-investigacao-2026-11-24.md)). Um que cai duas vezes seguidas, sem 15 s tocando entre elas, ou quatro
  vezes no mesmo dia, não é mais aberto. Só uma exceção do Windows conta como queda (fechar o programa não).
- **Volta ao anterior**: o build de antes fica guardado; com o novo quebrado, é ele que abre.
- **Procurar outro**: com um quebrado, uma vez por dia, ou na hora quando a pessoa liga (no máximo a cada 10 min). O servidor diz
  se ainda é o mesmo pacote (ETag), e aí nada é baixado.
- **O Soloist de outro programa**: se só há quebrados, Configurações › Serviços oferece o do Spotifast (`%LOCALAPPDATA%\paolino\spotifast`),
  se houver: "Usar esse Soloist" **copia** o programa dele (a pasta dele não muda). Sem clicar, nada é copiado.
- **Assinatura**: todo `soloist.exe` (baixado ou copiado) só é aberto se tiver a assinatura válida do Spotify (`assinatura.rs`).
- Só se fala com o Soloist que este provedor abriu: o `soloist.pid` ao lado da porta é conferido com o processo dele.
- A chave vai na linha de comando do Soloist, que é o que ele aceita: quem pode ver os processos deste usuário a vê.
- O que o Soloist escreve fica em `provedores-dados/spotify/soloist/saida.log`, sem a chave, e não no registro do Player.
- O visual ouve só o processo do Soloist que o Player abriu, com "todo o som" também. A barra de volume mexe só nele (no misturador do
  Windows, `mesa.rs`), e o visual desfaz essa conta: baixar a música não muda o que ele vê. O volume do Spotify Connect fica em 100.
- Sem equalizador, crossfade nem pré-análise exata (até a F7).

### Quando não toca

O erro de um provedor diz a causa ([`f1-provedores.md`](f1-provedores.md)). Sem rede, a faixa é tentada de novo e, se não voltar, a
fila para nela: "Sem conexão com o YouTube Music: a fila ficou parada aqui". Uma faixa que parou no meio é pedida de novo uma vez
(o endereço pode ter vencido) antes de a fila seguir.

### O login

`oauth.rs` pede o código ao Spotify pelo navegador (PKCE, o `state` conferido em todo retorno, o sorteio vem do Windows) com o
Client ID público do librespot e a porta 8898, e troca pelo token. Cada conexão na porta é atendida à parte: uma que fica parada não
prende o login. O id do aparelho (`aparelho.txt`) é sempre o mesmo, e assim a conta não mostra um aparelho novo a cada abertura.

## O protocolo

Capacidade `toca-sozinho`; pedidos `tocador`, `ligar-tocador`, `desligar-tocador` e `controlar`; resposta `tocador`
([`f1-provedores.md`](f1-provedores.md)). Serve a qualquer serviço com um player próprio. No Player, `externo.rs` pergunta o
estado (a cada 200 ms tocando) e manda os comandos numa thread; o `local.rs` acompanha.
