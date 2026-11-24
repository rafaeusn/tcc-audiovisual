# F6: o Spotify no Player

O desenho completo (com o que foi pensado e descartado) está em
[`superpowers/specs/2026-11-23-f6-spotify-design.md`](superpowers/specs/2026-11-23-f6-spotify-design.md); a seção 8 dele é a
que vale para tocar.

## O que é

O `provedor-spotify` (`app/provedores/spotify`) é um programa à parte que vem com o Player. Ele cuida de três coisas:

1. **A conta** (parte 1): a pessoa entra pelo navegador; o refresh token fica cifrado (DPAPI).
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
- **O fim da faixa**: o Player dá a faixa por acabada 250 ms antes do fim e pausa, porque o Spotify emendaria outra
  (a fila é do Player). Se ele emendar mesmo assim, a faixa nova é calada.
- **Outro aparelho**: a conta toca num lugar só. Se o Spotify passa a tocar em outro lugar, o Player pausa e avisa; continuar traz
  a faixa de volta de onde parou. Se o Spotify (ou o Spotifast) toca neste computador quando a pessoa dá o play, o Player avisa
  que ele para.
- **Vence em 90 dias**: a partir de 60 o provedor procura outro uma vez por dia; vencido (código de saída 10), baixa na hora.
- **Build quebrado**: o Spotify já publicou builds que caem ao tocar no Windows (o 1.4.0.7, de 23/11/2026, é um). Um que cai duas
  vezes sem nunca ter tocado 20 s não é mais aberto: o Match toca, a tela diz o motivo e outro build é procurado uma vez por dia.
- A chave vai na linha de comando do Soloist, que é o que ele aceita: quem pode ver os processos deste usuário a vê.
- O que o Soloist escreve fica em `provedores-dados/spotify/soloist/saida.log`, sem a chave, e não no registro do Player.
- O visual ouve o processo do Soloist quando o Modo Escuta está num app só; com "todo o som", ouve tudo, como sempre.
- Sem equalizador, crossfade, troca sem intervalo nem pré-análise exata (até a F7).

## O protocolo

Capacidade `toca-sozinho`; pedidos `tocador`, `ligar-tocador`, `desligar-tocador` e `controlar`; resposta `tocador`
([`f1-provedores.md`](f1-provedores.md)). Serve a qualquer serviço com um player próprio. No Player, `externo.rs` pergunta o
estado (a cada 200 ms tocando) e manda os comandos numa thread; o `local.rs` acompanha.
