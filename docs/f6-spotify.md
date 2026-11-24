# F6: o Spotify no Player

O desenho completo (com o que foi pensado e descartado) está em
[`superpowers/specs/2026-11-23-f6-spotify-design.md`](superpowers/specs/2026-11-23-f6-spotify-design.md); a seção 8 dele é a
que vale para tocar.

## O que é

O `provedor-spotify` (`app/provedores/spotify`) é um programa à parte que vem com o Player. Ele cuida de quatro coisas:

1. **A conta** (parte 1): a pessoa entra pelo navegador; o refresh token fica cifrado (DPAPI).
2. **Os dados** (parte 2): busca, início, páginas e biblioteca, pelo Pathfinder, com a sessão do librespot.
3. **O áudio** (parte 3): o Spotify toca com o som do próprio Spotify, decifrado em memória pelo librespot, no motor do
   Player.
4. **O Soloist** (parte 4): o player oficial do Spotify, sem janela, como reserva.

O librespot cuida da sessão, dos dados e do áudio. Nada é gravado em arquivo.

## Por onde uma música do Spotify toca

A cadeia, na ordem (a regra pura e testada é `player/src/cadeia.rs`):

| Situação | Quem toca |
| --- | --- |
| A conta está dentro e o áudio abriu (o normal) | O **librespot**: o áudio do Spotify no motor do Player, com pré-análise, troca sem intervalo, crossfade e equalizador |
| O librespot falhou nesta abertura (chave recusada, resposta quebrada) e o Soloist está pronto | O **Soloist**: o player oficial do Spotify, o som sai por ele |
| Sem librespot nem Soloist | O **Match**: a mesma música no YouTube Music |
| Nenhum | Ninguém: o Player avisa para ligar o YouTube Music na Loja |

A **causa** do erro decide (o protocolo ganhou o campo `causa`):
`rede` e `limite` tentam de novo o mesmo jeito; `indisponivel` manda só aquela faixa para o próximo jeito; `chave-de-audio`
e `quebrado` derrubam o jeito nesta abertura; `sem-premium`, `sessao-recusada` e `sem-conta` pulam direto para o Match (o
Soloist depende da mesma sessão). O aviso sai uma vez por troca, com o motivo em palavras simples.

### O librespot (o jeito principal)

- **Onde:** `app/provedores/spotify/src/audio/` — `librespot.rs` (formato, chave, decifra), `cache.rs` (os pedaços
  decifrados, **só na memória**, teto de 8 MB e 4 faixas), `servidor.rs` (HTTP local com `Range`), `pedidos.rs` (as
  contas do `Range` e do caminho), `mod.rs` (liga e desliga com a sessão).
- **Como o Player toca:** o `resolver` abre a faixa (é ali que a conta e a chave são conferidas) e devolve
  `{"fonte":"endereco","url":"http://127.0.0.1:<porta>/a/<token>/<id>.<ext>","formato":"ogg"}`. Para o motor do Player é
  um endereço como o do YouTube Music, então troca sem intervalo, crossfade, equalizador e pré-análise valem sem mudar
  nada no `tocador`.
- **Qualidade:** o melhor formato que a conta tem (Ogg 320 no Premium; Ogg 160/96 e MP3 numa conta antiga).
- **Segurança:** a porta escuta só em `127.0.0.1`, com um token sorteado a cada abertura; sem o token, `404`. Sem `CORS`.
- **A regra 2:** o áudio decifrado vive num `Arc<Vec<u8>>` que morre com o provedor; não existe caminho de código que
  grave áudio. O `AudioFile` do librespot usa um temporário para os bytes ainda **cifrados**, limpo a cada abertura.

**Primeiro teste de verdade (24/11/2026, à noite, numa cópia com a conta do Rafael):** o login entrou, a busca do Spotify
funcionou, e o `resolver` devolveu o endereço local com `.ogg` — ou seja, o formato Ogg 320 foi escolhido e o servidor
subiu. **O áudio não tocou:** o Spotify recusou a chave de áudio desta conta (`causa: "chave-de-audio"`), sempre e em
qualquer formato.

**A causa, achada na documentação do Spotifast (24/11/2026):** contas novas do Spotify — as que têm por nome uma sequência
longa de letras e números que o Spotify inventou — têm **toda** chave de áudio recusada pelo librespot. A conta do Rafael é
uma dessas (`31axzsjldxrjw3er75tdfai2f5ou`). A documentação do Spotifast é explícita:

> "If your Spotify username is a long string of random letters and numbers that Spotify made up for you, Spotify may refuse
> every key. **Nothing in Spotifast or librespot can change that today.**"
> (`docs/_guide/troubleshooting.md`, e a [issue #1649 do librespot](https://github.com/librespot-org/librespot/issues/1649))

E o que **não** resolve, segundo a mesma documentação: um Client ID próprio ("that only speeds up library and search
requests, not audio keys"), trocar as configurações de áudio, limpar o cache, sair e entrar de novo.

**Tudo o que foi testado e não era a causa** (para não repetir): o Client ID do Keymaster (é o que o Spotifast usa), o
`client_id` do Android (dá `400` sem a `os_version`, que só o fork do Spotifast tem), o `device_id` fixo, o `username` no
`Credentials` (o `playback_credentials` do Spotifast, exato), a credencial reutilizável do librespot, e esperar o plano
(`Some("premium")` desde o primeiro instante). As quatro combinações de `Credentials` × `device_id` recusam igual.

**O que a documentação do Spotifast recomenda:** o **Soloist** (o player oficial do Spotify), que "may avoid the refused-key
problem with the built-in player". É o caminho que já temos implementado (`soloist/`), e é o que a cadeia de reserva usa
quando o librespot cai.

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

Capacidade `audio` (o áudio no motor do Player, pelo librespot) e `toca-sozinho` (o Soloist); pedidos `resolver`,
`tocador`, `ligar-tocador`, `desligar-tocador` e `controlar`; resposta `tocador` ([`f1-provedores.md`](f1-provedores.md)).
Serve a qualquer serviço com um player próprio. No Player, `externo.rs` pergunta o estado (a cada 200 ms tocando) e manda
os comandos numa thread; o `local.rs` acompanha.

O erro do protocolo ganhou **`causa`** (opcional, F6): `rede`, `limite`, `indisponivel`, `sem-conta`, `sem-premium`,
`sessao-recusada`, `chave-de-audio`, `quebrado`. Provedor antigo não manda, e uma causa que o Player não conhece vira erro
comum (nada quebra). É ela que a cadeia de reserva lê.
