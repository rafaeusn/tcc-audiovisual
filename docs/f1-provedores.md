# F1: núcleo de provedores

Começada em 22/11/2026. Parte do [plano v2](plano-v2.md) (seções 6 e 7). Regras dos serviços: `AGENTS.md`, seção 2
(sem limitação de API; o único limite é não quebrar DRM nem baixar música para arquivo).

## O que é

Um **provedor** é um programa à parte que conhece um serviço (YouTube Music, Spotify, SoundCloud...). O Player o
abre, conversa com ele pelo protocolo abaixo e recebe faixas, prateleiras e o endereço do áudio. O motor do Player
toca, mistura e pré-analisa; o provedor só sabe falar com o serviço.

Por que um programa à parte: o serviço muda e o conserto sai sem release nova do Player; um provedor que trava não
derruba o app; cada provedor pode ser escrito na linguagem em que já existe código pronto.

## As partes

| Parte | O que entra | Situação |
| --- | --- | --- |
| 1 | Crate `provedores`: modelos, protocolo, casador entre serviços, cliente (abre o processo e conversa), registro (provedores instalados numa pasta), o lado do provedor (`provedores::servir`) e o provedor de exemplo | 22/11 |
| 2 | Tocador tocando por endereço (HTTP com pedidos por faixa de bytes, AAC/M4A) e o cache de análise por ID e ISRC | 22/11 |
| 3 | No Player (`player/src/remoto.rs`): uma thread por provedor, busca em todos (bloco "Nos serviços" na Biblioteca), fila com faixas de vários serviços, tocar pela rede com pré-análise e a seguinte sem intervalo | 22/11 ("Neste computador" como provedor igual aos outros ficou para a F2) |
| 4 | A Loja (aba Provedores): situação de cada um, ligar e desligar, instalar de uma pasta (atualizar), remover com "Desfazer" | 22/11 |

## Onde ficam

`<pasta de dados>/provedores/<nome>/provedor.toml`, com o executável na mesma pasta:

```toml
nome = "Exemplo"
servico = "exemplo"             # identificador curto, vai em toda faixa (exemplo:123)
executavel = "provedor-exemplo.exe"
argumentos = []                 # opcional
versao = "0.1.0"
```

O executável tem que estar dentro da pasta do provedor (caminho relativo, sem `..`). Provedores vêm ligados; os
desligados ficam em `provedores-desligados` no `ajustes.toml`. Instalar pela Loja copia a pasta escolhida para
`provedores/<servico>`; remover a move para `provedores-removidos/<servico>` (o "Desfazer" a traz de volta).

### Testar com o provedor de exemplo

1. `cargo build --release -p provedores --bin provedor-exemplo`.
2. Copie `provedor-exemplo.exe` para `<pasta de dados>/provedores/exemplo/` e ponha ao lado um `provedor.toml` com
   `argumentos = ["--url", "http://127.0.0.1:8765"]`.
3. Sirva três arquivos de áudio chamados `1`, `2` e `3` (WAV): `python -m http.server 8765 --bind 127.0.0.1`.
4. Abra o Player, vá à Biblioteca e busque "faixa".

O tocador sozinho também tem um teste manual: `cargo run -p tocador --example tocar_endereco -- <url1> <url2>`.

## O protocolo (versão 1)

Uma mensagem JSON por linha. O Player escreve os pedidos na entrada do provedor e lê as respostas na saída; o que o
provedor escreve na saída de erro vai para o registro do Player. Cada pedido tem um `id`; a resposta repete o `id`.
O provedor pode responder fora de ordem.

O primeiro pedido é sempre `ola`. O provedor se apresenta e diz o que sabe fazer:

```json
{"id":1,"pedido":"ola","protocolo":1}
{"id":1,"resposta":"apresentacao","nome":"Exemplo","servico":"exemplo","versao":"0.1.0","protocolo":1,"capacidades":["busca","inicio","audio"]}
```

| Pedido | Campos | Resposta |
| --- | --- | --- |
| `ola` | `protocolo` | `apresentacao` (`nome`, `servico`, `versao`, `protocolo`, `capacidades`) |
| `buscar` | `texto`, `limite` | `itens` (`itens`: faixas, álbuns, artistas, playlists; o primeiro é o melhor resultado, que o Player mostra em destaque; `limite` vale para cada tipo) |
| `inicio` | — | `prateleiras` (`prateleiras`: `titulo` e `itens`) |
| `album`, `artista`, `playlist` | `album`, `artista` ou `playlist` (o id dele) | `itens` (as faixas, ou os álbuns do artista) |
| `biblioteca` (só com a capacidade `biblioteca`) | — | `itens` (as playlists, os álbuns e os artistas que a conta tem salvos, na ordem do serviço) |
| `resolver` | `faixa` (o id dela) | `fonte` |
| `tchau` | — | `ok`, e o provedor sai |
| `entrar` (só com a capacidade `conta`) | — | `ok` na hora (o navegador abriu; o login continua no provedor) ou `erro` com o motivo |
| `sair` | — | `ok` (a credencial guardada é apagada) |
| `tocador` (só com a capacidade `toca-sozinho`) | — | `tocador` (`situacao`: `desligado`, `baixando`, `abrindo`, `pareando`, `pronto` ou `falhou`; `tem_chave`; `nome`, `progresso`, `erro`, `processo`, `tocando`, `versao`, `alternativa` e `adianta` opcionais) |
| `ligar-tocador` | `chave` (opcional: sem ela, vale a guardada) | `ok` na hora (o resto continua no provedor) ou `erro` |
| `desligar-tocador` | — | `ok` |
| `trocar-tocador` | — | `ok` na hora (passa a usar o programa de `alternativa` e liga com ele) ou `erro` |
| `controlar` | `acao`: `tocar` (com `faixa`), `preparar` (com `faixa`), `pausar`, `continuar`, `ir` (com `segundos`) ou `parar` | `ok` ou `erro` |
| `conta` | — | `conta` (`situacao`: `fora`, `entrando` ou `dentro`; `nome`, `premium`, `erro` e `navegador` opcionais; `navegador` em `entrando` diz que o navegador está aberto esperando a pessoa) |

O Player abre o provedor na pasta dele e passa em `TCCPLAYER_PROVEDOR_DADOS` uma pasta só dele
(`<pasta de dados>/provedores-dados/<serviço>`), para guardar cache ou login. Provedores que vêm com o app (hoje,
o YouTube Music e o Spotify) ficam ao lado do `player.exe` e entram na lista sem instalar.

Nenhum campo do pedido se chama `id` além do número do pedido (por isso `resolver` usa `faixa`, e não `id`).
Qualquer pedido pode ser respondido com `{"id":7,"resposta":"erro","mensagem":"..."}`. Um pedido que o provedor
não conhece também (assim versões novas do Player falam com provedores antigos).

O erro pode vir com **`causa`** (opcional, F6): `rede`, `limite`, `indisponivel`, `sem-conta`, `sem-premium`,
`sessao-recusada`, `chave-de-audio` ou `quebrado`. Ela diz ao Player o que fazer com a faixa (a regra está em
`player/src/cadeia.rs`): `rede` e `limite` são tentadas de novo (2, 5 e 10 s depois) e, se não voltarem, a fila **para** na
música, sem passar pelas outras; as demais pulam só aquela faixa. Sem `causa`, ou com uma que o Player não conhece, vale o
tratamento de antes (pula). O prazo estourado e o provedor que fechou contam como `rede`.

A **faixa** é igual em todo serviço:

```json
{"tipo":"faixa","servico":"exemplo","id":"123","isrc":"BRXXX2600001","titulo":"Nome","artistas":["Artista"],
 "album":"Álbum","duracao_s":201.5,"capa":"https://...","video":false}
```

**Campos opcionais da F2** (o protocolo continua na versão 1; provedor antigo não manda e tudo funciona):

- Na `apresentacao`, `identidade`: `{"cor":"#FF0000","logo":"<svg ...>...</svg>"}`. A cor da marca, em `#RRGGBB`, e
  a logo em SVG (texto, até 32 KB, num quadrado). O Player guarda a logo em
  `<pasta de dados>/provedores-dados/<serviço>/logo.svg` e usa os dois no espaço do serviço. Cor ou logo fora do
  formato são ignoradas (o espaço usa a inicial do nome e uma cor tirada dele).
- Na `apresentacao`, `conta` (F6, só com a capacidade `conta`): como a conta está ao abrir, no mesmo formato da
  resposta `conta`, por exemplo `{"situacao":"entrando"}` enquanto a credencial guardada é renovada. Depois de
  `entrar`, o Player pergunta `conta` a cada 2 s até ela sair de `entrando`.
- Na faixa, `album_id` (o id do álbum) e `artistas_ids` (o id de cada artista, na mesma ordem de `artistas`; `""`
  quando não tem). No álbum, `artistas_ids`. Com eles, o nome do álbum e dos artistas vira link para a página deles.

A **fonte** diz como tocar:

- `{"fonte":"endereco","url":"https://...","cabecalhos":[["User-Agent","..."]],"formato":"m4a","duracao_s":201.5}`:
  o motor do Player baixa (aos pedaços, com `Range`), decodifica, mistura e pré-analisa.
- `{"fonte":"toca-sozinho","pid":1234}`: o provedor toca (o player web do Spotify numa WebView2, por exemplo); o
  Player captura o som desse processo para o visual, como no Modo Escuta.

**O tocador do serviço** (F6, capacidade `toca-sozinho`): um provedor sem `audio` pode tocar por um programa do próprio serviço
(o Soloist do Spotify). `tocando` traz `faixa`, `vez` (sobe a cada `tocar`), `situacao` (`carregando`, `tocando`, `pausada`, `acabou`,
`em-outro-lugar` ou `falhou`), `posicao_s`, `duracao_s` e `erro`. Com ele pronto, o Player manda `controlar` e acompanha pelo `tocador`;
se a faixa falha, ela vai para outro serviço (o Match). `versao` é a do programa em uso. `alternativa` vem quando o programa que o
serviço publica não serve e há outro neste computador (a versão e de onde é): o Player oferece, e `trocar-tocador` passa a usá-lo.
Com **`adianta`**, a 10 s do fim o Player manda `preparar` com a próxima: o tocador passa para ela sozinho, sem intervalo e sem `tocar`
(em `tocando`, a `faixa` muda e a `vez` não), e a fila do Player anda junto. Um provedor em Rust pode responder as páginas lentas de outra thread
(`Servico::em_segundo_plano`), para a pausa não esperar por elas.

## Casar uma faixa entre serviços

Do MixMusic (MIT): nota de 0 a 1. Com o mesmo ISRC, 1. Senão, 45% para o título limpo (sem "Remastered", "Radio
Edit", "Official Video"...; "Remix", "Ao Vivo", "Acústico" continuam, porque são outra gravação), 35% para o
artista (o canal "Artista - Topic" do YouTube conta como o artista) e 20% para a duração (até 2 s de diferença
conta inteiro; 12 s ou mais, nada), mais 0,05 se for música em vez de vídeo. Toca quando a nota passa de 0,6. Título ou artista
muito diferentes perdem 0,4 (a duração sozinha não basta), e outra versão (ao vivo, karaokê, remix...) perde 0,25.
