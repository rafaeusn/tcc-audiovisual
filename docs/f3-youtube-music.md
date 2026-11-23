# F3: provedor do YouTube Music

Feito em 22–23/11/2026. Código em `app/provedores/youtube-music`: um programa à parte que fala o protocolo da F1
e vem pronto com o app (o Player o acha ao lado do `player.exe`).

## Em uma frase

A busca, os álbuns e as playlists vêm da API do YouTube Music (pela biblioteca `rustypipe`), e o áudio vem pelo
**cliente do YouTube para visionOS**: o YouTube entrega a esse cliente o endereço do áudio pronto para tocar,
inteiro, sem assinatura para decifrar e sem PO token. É o mesmo caminho do MixMusic, do limusic e do Prism.

## Por que o áudio do YouTube é difícil

Para tocar uma faixa, o app pede ao YouTube o "player" do vídeo, se apresentando como um dos apps oficiais (o
site, o app de celular, o de TV...). O YouTube responde com os endereços do áudio, mas, para quase todos os apps,
ele protege esses endereços:

- **Assinatura embaralhada**: o endereço só funciona depois de passar por uma conta escondida no JavaScript do
  player do YouTube, que muda com frequência.
- **PO token**: uma prova de que quem pede é um navegador de verdade, gerada por uma máquina virtual em JavaScript
  do YouTube (a "Botguard"). Sem ela, o endereço toca só o primeiro pedaço (cerca de 1 MB, uns 55 s) e depois
  responde 403.

Os dois exigem rodar código do próprio YouTube, e é o caminho mais frágil: quebra a cada mudança deles.

## O que cada projeto de referência faz (lido no código em 23/11/2026)

| Projeto | Como toca o YouTube Music |
| --- | --- |
| **MixMusic** (Go, desktop) | Pede o player como o **app de visionOS**, sem conta: endereços que tocam como vêm. Se o YouTube recusa, usa o yt-dlp (que roda o JavaScript do YouTube num Deno/Node instalado à parte) |
| **limusic** (Rust, desktop) | Também **visionOS** primeiro. Diz no código que o iOS só serve faixas fechadas e que o Android VR parou de servir em 22/09/2026 |
| **Prism** (Android) | **visionOS** como principal, TV de reserva |
| **Metrolist** e **SimpMusic** (Android) | O cliente web, com o PO token gerado rodando a Botguard numa WebView (o caminho pesado) |

## O que testei aqui

| Caminho | Resultado |
| --- | --- |
| Cliente iOS (o padrão do `rustypipe`) | Só o primeiro ~1 MB; o resto, 403 |
| Clientes TV, web e Android do `rustypipe` | A decodificação de assinaturas dele está parada desde abril de 2025: não acompanha o YouTube de agora |
| Cliente Android VR | O YouTube pede para confirmar que não é um robô |
| **Cliente visionOS** | **Funciona**: AAC (itag 140) e Opus com endereço direto; começo, meio, fim e o arquivo inteiro respondem 206 |

E de ponta a ponta:

- O tocador pulou para os 410 s de "Around the World" (Daft Punk, 7:10) e tocou até o fim.
- No app (cópia de teste, sem som): a busca "daft punk around the world" trouxe 20 faixas em "Nos serviços". A
  primeira tocou com a capa do álbum, e a pré-análise deu **121 BPM** (o andamento real da música), tom Ré maior e
  a estrutura da faixa.

## Como o nosso provedor faz

- **Busca, início (mais tocadas e em alta), álbum, playlist e artista**: `rustypipe`, como o app web do YouTube
  Music. Só leitura de dados públicos, sem conta.
- **Áudio** (`visionos.rs`):
  1. pede um ID de visitante (`youtubei/v1/visitor_id`), guardado por 6 horas;
  2. pede o player do vídeo com esse ID (`youtubei/v1/player`);
  3. escolhe o AAC (itag 140), que o motor do Player decodifica;
  4. entrega o endereço e o agente do visionOS ao Player, que baixa aos pedaços, toca e pré-analisa.

  Se o YouTube pede para confirmar que não é um robô, o ID é trocado e o pedido é refeito uma vez. Nada roda código
  do YouTube.
- **Pasta de dados**: `<dados>/provedores-dados/youtube-music` (o cache do `rustypipe`).

## Riscos e o que fazer quando quebrar

- O YouTube muda o que pede a cada cliente (o Android VR caiu em setembro de 2026). Os valores do cliente visionOS
  ficam no topo do `visionos.rs`, e o MixMusic, o limusic e o yt-dlp acompanham essas mudanças: quando parar,
  comparar com eles e trocar os valores.
- **Plano B**, se o visionOS cair de vez: o caminho do MixMusic, usar o yt-dlp quando a pessoa o tiver instalado
  (ele resolve as assinaturas com o JavaScript do YouTube fora do Player). Exige a decisão do Rafael, porque
  executa o código do YouTube.
- Faixas com DRM ou só para quem tem conta não tocam (o provedor diz o motivo).
- Ainda por fazer: o Opus (o AAC basta hoje), o áudio da conta Premium (256 kbit/s), as telas de álbum, artista e
  playlist (vêm com a F2) e a faixa baixada duas vezes (tocador e pré-análise).
