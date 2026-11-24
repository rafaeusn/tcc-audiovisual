# F6: o provedor do Spotify

> **Revisto em 24/11/2026 (à tarde).** Tocar pelo librespot **voltou a valer**, a pedido do Rafael: o librespot pede a chave de
> áudio ao Spotify como um cliente autenticado (não quebra DRM), e o áudio decifrado vive **só na memória**, nunca em arquivo e
> nunca redistribuído. É o jeito principal; o Soloist e o Match ficam como reserva. A regra 2 do `AGENTS.md` foi anotada com
> isso. A revisão da noite de 23/11 (abaixo, que tirou o librespot) fica como história da decisão.
>
> **Revisto em 23/11/2026, à noite.** Tocar pelo librespot (o jeito A abaixo) **saiu**: ele decifra as faixas protegidas do
> Spotify fora do player dele, o que a regra 2 do `AGENTS.md` não deixa, e o desenho tinha passado por cima disso. O librespot
> fica só no login e nos dados. O Spotify toca pelo **Soloist** (o player oficial, com o som do Spotify) ou pelo **Match** (a
> mesma música no YouTube Music). As seções 2, 4, 5.3 a 5.5, 5.7 e 6 valem com a seção 8 por cima. **No fim, o jeito principal é
> o librespot (24/11).**
>
> **Revisto em 24/11/2026, a pedido do Rafael:** o librespot **voltou** e é o jeito principal. Ele não quebra DRM — pede a
> chave ao Spotify como um cliente autenticado, e o áudio decifrado vive **só na memória** (nada em arquivo, nada
> redistribuído), que é o que a regra 2 proíbe. A decisão ficou registrada na seção 2 do `AGENTS.md`. A cadeia é
> librespot → Soloist → Match, decidida pela **causa** do erro (`provedores::Causa`); como tocar, de fato, está em
> [`docs/f6-spotify.md`](../../f6-spotify.md).

Desenho de 23/11/2026, conversado com o Rafael. Plano de origem: [`docs/plano-v2.md`](../../plano-v2.md), fase F6.
Referências lidas para este desenho: o Spotifast (Rust, MIT: `src/auth.rs`, `docs/_reference/how-it-connects.md`,
`what-spotify-allows.md`, `soloist.md`, `src/soloist/`) e o MixMusic (`docs/spotify-playback.md`,
`docs/architecture/spotify-here.md`).

## 1. Intenção

O Spotify entra como mais um serviço do Player, igual ao YouTube Music: um espaço com a cor e a logo dele, Para
você, Busca, a Biblioteca da conta (playlists, curtidas, álbuns, artistas), as páginas de álbum, artista e playlist,
e tocar. O Rafael tem Premium e quer o **som do próprio Spotify** tocando no Player, com o visual e a pré-análise
exatos, como nas músicas locais e do YouTube Music. Quando o jeito principal falhar, o Player troca sozinho para o
seguinte e diz o motivo.

## 2. Metas, o que não é meta, o que seria fracasso

**Metas**
- Entrar na conta uma vez, pelo navegador (quem digita a senha é o Rafael), sem criar app de desenvolvedor e sem
  Client ID.
- Dados da conta e do catálogo: busca (todos os tipos), início ("Feito para você" e as prateleiras do Spotify),
  álbum, artista, playlist (as dele e as do Spotify) e a Biblioteca da conta.
- Tocar com o áudio do Spotify **no motor do Player**: troca sem intervalo, crossfade, equalizador, volume igualado,
  pré-análise completa e o visual conduzido por ela.
- **A cadeia de reserva**, decidida pela causa do erro: A (librespot) → C (Soloist) → Match (YouTube Music). O
  aviso diz o motivo uma vez, com palavras simples.

**Não é meta (fica para depois)**
- Escrever na conta do Spotify: curtir, seguir, editar playlists. Na F6, curtir e salvar continuam no acervo local
  (F2); a Biblioteca do serviço mostra as duas coisas.
- Spotify Connect como jeito de tocar, Canvas, letras do Spotify, rádio e podcasts. Os pedidos internos que eles
  usam são os mesmos, então ficam fáceis depois, mas não entram agora.
- O player web numa WebView2 (o caminho do MixMusic): ficou de fora na conversa (o visual só ao vivo, sem Client ID
  a chave fica frágil).

**Fracasso, mesmo funcionando**
- Áudio do Spotify gravado em arquivo, no cache ou em qualquer pasta (regra 2 do `AGENTS.md`). O áudio decifrado
  vive só na memória, para tocar e para a pré-análise.
- Pedir a senha no Player ou guardar a senha. Só a credencial que o próprio Spotify devolve, cifrada (DPAPI).
- Trocar de jeito sem dizer por quê, ou trocar por um erro passageiro (a rede piscou).
- O Player travar esperando o Spotify (toda espera é na thread do provedor; a interface só lê o que está pronto).

## 3. Restrições

- Regras do `AGENTS.md`: serviços sem limitação (APIs internas valem), mas nada de quebrar DRM nem baixar música;
  nenhum segredo no código (o Client ID público do librespot, o mesmo do Spotifast, não é segredo); commits só
  quando o Rafael pedir; não fechar o Player do Rafael; código e textos em português.
- O provedor é um programa à parte, como o do YouTube Music (`app/provedores/spotify`), embutido ao lado do
  `player.exe` (`remoto::EMBUTIDOS`), falando o protocolo v1 por linhas de JSON (`docs/f1-provedores.md`). Campos e
  pedidos novos entram como opcionais, sem subir a versão.
- Licenças: librespot é MIT; o código do Spotifast é MIT (copiar mantém a origem no topo e entra em
  `THIRD-PARTY-NOTICES.md`). O provedor pode ser GPL-3.0-or-later.
- O Rafael costuma usar o Spotifast neste PC. Ele ocupa a porta 8898 do login do librespot; o nosso login precisa
  dizer isso com clareza quando a porta estiver ocupada.
- Testes ao vivo só numa cópia de teste (`TCCPLAYER_DADOS`), com `TCCPLAYER_MUDO=1` ou sem tocar por cima da
  música do Rafael.

## 4. Abordagens consideradas

| | Como | A favor | Contra |
| --- | --- | --- | --- |
| **A. librespot (o padrão do Spotifast)** | Login pelo navegador com o Client ID público do librespot; dados pelos pedidos internos da sessão (Pathfinder, spclient, metadados); áudio decifrado na memória, entregue ao motor do Player | Sem Client ID; o único jeito com o som do Spotify e com tudo do motor (pré-análise, sem intervalo, crossfade, EQ) | O librespot e os pedidos internos podem quebrar quando o Spotify muda algo; Premium |
| B. Player web numa WebView2 (MixMusic) | O Web Playback SDK escondido | Player oficial | Visual só ao vivo, sem crossfade e EQ; sem Client ID, a chave vem de lugar frágil |
| **C. Soloist (Spotifast 0.15)** | O player oficial do Spotify, baixado do Spotify, controlado pela API local dele | Oficial, sem perdas | Uma chave pessoal do painel do Spotify (o Rafael copia e cola); no Windows o visual só ouve o som ao vivo (captura do processo) |
| **Match (MixMusic)** | A mesma música no YouTube Music, pelo casador da F1 | Sem Premium, pré-análise completa | Outra gravação às vezes |

**Escolhido pelo Rafael:** A como padrão; se A falhar por uma causa que não é passageira, C; se A e C estiverem
fora, o Match. Descartado: B.

## 5. O desenho

### 5.1 O provedor e o login

- Programa `provedor-spotify` (crate em `app/provedores/spotify`), com `librespot-core`, `librespot-metadata`,
  `librespot-audio` e `librespot-protocol` 0.8 (as versões do Spotifast). A thread do Player não espera nada dele.
- **Login** como o Spotifast (`auth.rs`): PKCE no navegador com o Client ID público do librespot, retorno em
  `http://127.0.0.1:8898/login`, escopo de streaming. O token vira a sessão do librespot, que devolve uma credencial
  reutilizável; ela fica cifrada com DPAPI em `<dados>/provedores-dados/spotify/`. Nas próximas aberturas, a sessão
  entra sozinha.
- **Pedido novo `entrar`** (opcional; capacidade `Conta`): o Player pede, o provedor abre o navegador e responde
  quando a pessoa aprovou (ou com o erro, por exemplo "a porta 8898 está ocupada: feche o Spotifast e tente de novo").
  Pedido `sair` apaga a credencial. A `apresentacao` ganha `conta` opcional: `{"nome":"...","premium":true}` ou
  ausente (sem login). Sem login, o espaço do Spotify mostra só o botão "Entrar no Spotify".
- Identidade: a cor verde do Spotify e a logo, como o YouTube Music manda as dele.

### 5.2 Os dados

Pela sessão do librespot, com o portador do Login5 e o client token, como o Spotifast faz no início, no artista e nos
lançamentos:

| Pedido do protocolo | De onde vem |
| --- | --- |
| `buscar` | Pathfinder (a busca do app do Spotify), todos os tipos, o melhor resultado primeiro |
| `inicio` | A seção pessoal do início (o "Feito para você" e as outras prateleiras), Pathfinder |
| `album` | `getAlbum` (Pathfinder) ou os metadados da sessão |
| `artista` | `queryArtistOverview` (populares e lançamentos) |
| `playlist` | As playlists pela sessão (as da pessoa e as do Spotify, como o Spotifast lê) |
| **`biblioteca`** (novo, opcional; capacidade `Biblioteca`) | A lista de playlists da conta (rootlist), as Músicas Curtidas, os álbuns e os artistas salvos (coleção) |

- Faixas com `album_id`, `artistas_ids`, ISRC (dos metadados) e capa, para os links da F2 e para o Match.
- No Player, `PedidoDePagina::Biblioteca { servico }`; a Biblioteca do serviço junta o que veio da conta (por
  cima) com o que foi salvo no Player (o acervo local). Playlist da conta abre como as outras.
- Os pedidos internos podem mudar sem aviso. Cada um que falha vira erro com causa (5.4) e a página mostra "Tentar
  de novo"; nunca uma lista vazia como se não houvesse nada.

### 5.3 Tocar pelo librespot (A)

- `resolver` devolve `{"fonte":"endereco","url":"http://127.0.0.1:<porta>/<token>/<id>","formato":"ogg",...}`. O
  provedor serve, só em `127.0.0.1` e com um token sorteado a cada abertura, o arquivo Ogg do Spotify **decifrado em
  memória**, aos pedaços e com `Range`. Assim o tocador (`rede::LeitorHttp`), a troca sem intervalo, a seguinte
  pré-carregada e a pré-análise (`analisar_origem`) funcionam sem mudar nada no motor.
- Qualidade: a maior que a conta permite (Ogg 320 com Premium).
- Nada vai para o disco: os pedaços ficam num cache em memória do provedor, com limite de tamanho, e somem quando a
  faixa sai da fila.
- Pedir o mesmo arquivo duas vezes (tocador e pré-análise, a dívida da F1) é aceitável aqui, porque o provedor
  serve da memória; juntar os leitores continua sendo dívida.

### 5.4 Erros com causa e a cadeia de reserva

- A resposta `erro` ganha `causa` opcional: `rede`, `sem-conta`, `sem-premium`, `sessao-recusada`,
  `chave-de-audio` (o "audio key error" do librespot), `indisponivel` (a faixa não toca no país ou saiu do
  catálogo), `limite` (o Spotify pediu para esperar) e `quebrado` (resposta que o provedor não entendeu). Sem
  `causa`, o Player trata como antes.
- No Player, a regra fica numa função pura e testada (`cadeia::proximo_jeito(causa, jeitos_prontos)`):
  - `rede` e `limite`: tenta de novo o mesmo jeito (com espera), sem trocar.
  - `indisponivel`: só aquela faixa vai para o Match.
  - `chave-de-audio` e `quebrado` ao tocar: o jeito cai nesta abertura. A → C se o Soloist estiver pronto, senão o
    Match. C caído → o Match.
  - `sem-premium` e `sessao-recusada`: a sessão do librespot não serve, e o Soloist depende dela (5.7): o Match direto.
- O aviso sai uma vez por troca, com o motivo ("O Spotify recusou o áudio do librespot; tocando pelo Soloist"). A
  Loja (aba Provedores) e Configurações mostram o jeito de agora e o porquê.
- Com o Match, o par fica guardado (o casador da F1, nota a partir de 0,6, ISRC primeiro), e a faixa que não acha par
  é pulada com aviso.

### 5.5 Tocar pelo Soloist (C)

- Como o Spotifast (`src/soloist/`): o provedor baixa o Soloist do Spotify quando a pessoa escolhe configurá-lo, pede a
  chave pessoal (a pessoa cola num campo do Player; ela fica cifrada com DPAPI), abre o programa e fala com a API
  local dele. O Soloist aparece como um aparelho do Spotify; parear é uma vez, pelo app do Spotify.
- `resolver` devolve `{"fonte":"toca-sozinho","pid":<pid do Soloist>}`. O Player passa a saber acompanhar esse tipo
  de fonte (hoje é erro): pedidos novos opcionais `controlar` (`tocar`, `pausar`, `continuar`, `pular-para`, `parar`,
  com a faixa) e `situacao` (posição, tocando, acabou), e o visual ouve o processo do Soloist (a captura por app que
  já existe).
- Sem pré-análise exata com o Soloist até a F7 (alinhar a gravação ao som); o visual reage ao vivo.

### 5.6 O que a pessoa vê

- Configurações › Serviços › Spotify: entrar e sair, a conta e se é Premium, o jeito de tocar agora (librespot,
  Soloist ou YouTube Music) e o motivo da última troca, "Configurar o Soloist" (chave, baixar, parear) e "Trocar o par
  do YouTube Music" na faixa tocada pelo Match.
- O espaço do Spotify sem login: um cartão com "Entrar no Spotify".

### 5.7 Decisões depois da revisão

**Do Rafael:**
- **O Player tem a prioridade.** O Spotify toca em um lugar por vez por conta; tocar pelo Player pode parar o
  Spotifast (ou o app do Spotify) e isso é aceito. Antes de tomar a reprodução, o Player confere se outro aparelho da
  conta está tocando (o estado do Connect pela sessão) e, se estiver, avisa uma vez: "O Spotify está tocando no
  <aparelho>; ao tocar aqui, ele para". O mesmo para a porta 8898 do login: ocupada (o Spotifast aberto), o aviso diz
  quem a usa e pede para fechá-lo antes de entrar. Os testes ao vivo podem ser feitos em cada parte, na conta dele,
  com a mesma regra.
- **Client ID pessoal como reserva opcional.** Se o Spotify recusar a busca, as curtidas ou os álbuns salvos pela
  sessão do librespot, Configurações › Serviços › Spotify oferece um campo para colar um Client ID próprio (como no
  Spotifast, PKCE com retorno local), usado só para o que a sessão não conseguir. Ninguém precisa dele para começar.
  Sem ele, a parte recusada mostra o motivo e "Tentar de novo".

**Minhas (para o Rafael conferir):**
- **O Soloist depende da sessão do librespot.** A API local do Soloist só pausa, continua, pula e informa; quem manda
  tocar uma faixa nele é a sessão do librespot (pelo Connect), sem Web API. Então a cadeia fica: problema no áudio
  (`chave-de-audio`, `quebrado` ao tocar) → C; a sessão inteira recusada (`sessao-recusada`, `sem-premium`) → o Match
  direto.
- **Arquivo temporário cifrado vale.** O `librespot-audio` guarda os pedaços baixados num arquivo temporário, ainda
  cifrados. Ele fica na pasta do provedor e é apagado ao fechar (e ao abrir, se sobrou). Áudio decifrado nunca vai para
  o disco.
- **librespot 0.8 do crates.io primeiro.** A causa `chave-de-audio` sai da mensagem de erro dele; só se não der para
  distinguir é que entra o fork do Spotifast (por git, o que muda o `cargo deny` e as notícias de licença).
- **Um Soloist nosso**, com o nome de aparelho "Player (Soloist)", para não brigar com o do Spotifast.

## 6. Partes (ordem de entrega)

1. **Provedor e login**: o programa, o login pelo navegador, a credencial cifrada, `entrar`/`sair`, `conta` na
   apresentação, identidade, embutido no Player; a tela de entrar.
2. **Os dados**: busca, início, álbum, artista, playlist e `biblioteca` (com a página no Player).
3. **Tocar pelo librespot**: o servidor local em memória, `resolver`, pré-análise e sem intervalo conferidos.
4. **Erros com causa e o Match**: `causa` no protocolo, a regra da cadeia, o Match no Player, os avisos.
5. **O Soloist**: baixar, chave, parear, `toca-sozinho` no Player com `controlar` e `situacao`.
6. **Testes, cópia de teste e documentos** (`docs/f6-spotify.md`, `app/README.md`, `AGENTS.md`, `plano-v2.md`,
   `THIRD-PARTY-NOTICES.md`).

Cada parte termina com `cargo fmt`, `clippy` sem aviso, testes e, se mexeu na tela, a cópia de teste por
`PrintWindow`.

## 7. Deixado para quem constrói

- Os nomes, a divisão em arquivos e quais pedidos do Pathfinder usar para cada página (copiando do Spotifast o que ele
  já usa; os hashes das consultas podem precisar ser lidos do site do Spotify quando mudarem).
- O tamanho do cache em memória e como o servidor local entrega os pedaços.
- O texto exato dos avisos, dentro do que 5.4 diz.
- O desenho da tela de entrar e da seção nas Configurações, seguindo o que já existe.

## 8. Como o Spotify toca (revisão de 23/11/2026)

**A cadeia:** o Soloist, se a pessoa o configurou e ele está pronto; senão (ou se ele falhar), o Match. Configurações › Serviços ›
Spotify escolhe entre "Soloist, com o Match de reserva" e "Sempre pelo YouTube Music". A causa do erro (5.4) continua decidindo:
`rede` e `limite` tentam de novo; `indisponivel` manda só aquela faixa para o Match; o resto derruba o Soloist nesta abertura. O
aviso diz o motivo uma vez.

**Match.** No Player: a faixa de um serviço sem áudio é procurada num provedor que tem busca e áudio (título e artista), o
casador da F1 escolhe (ISRC, ou nota a partir de 0,6) e o endereço é o do par. O par fica guardado em `<dados>/pares.json` (só
ids e títulos). "Procurar outro par" recusa o de agora e pega o seguinte. A tela diz por onde a faixa toca. Sem par, a faixa é
pulada com aviso. Pré-análise, fila sem intervalo, crossfade e equalizador valem como para qualquer faixa do YouTube Music.

**Soloist** (documentação oficial: developer.spotify.com/documentation/soloist). O provedor baixa o programa do Spotify quando a
pessoa escolhe configurá-lo (nada é redistribuído), guarda a chave pessoal dela cifrada (DPAPI) e a pessoa pareia uma vez pelo
app do Spotify (`--pair`). Cada faixa toca com `--single-track spotify:track:<id>` e a API local (`--ws 127.0.0.1:0`) dá posição,
pausa, pulo e o fim; o programa sai sozinho quando a faixa acaba. Não precisa de Client ID nem da sessão do librespot. A chave vai na
linha de comando do programa (é o que o Spotify pede): fica visível a quem já pode ver os processos deste usuário. As versões do
Soloist vencem em 90 dias (código de saída 10): baixa outra. O visual ouve o processo ao vivo; sem equalizador, crossfade e
pré-análise exata até a F7.

**Mudou ao construir:** em vez de um processo por faixa (`--single-track`), o Soloist fica aberto e recebe `play` com a faixa pela API
local, como no Spotifast: a troca de faixa não espera ele abrir e entrar na conta.

**Partes:** 3, o Match; 4, o Soloist; 5, testes, cópia de teste e documentos.
