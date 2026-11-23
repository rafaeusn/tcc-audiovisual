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
| `buscar` | `texto`, `limite` | `itens` (`itens`: faixas, álbuns, artistas, playlists) |
| `inicio` | — | `prateleiras` (`prateleiras`: `titulo` e `itens`) |
| `album`, `artista`, `playlist` | `album`, `artista` ou `playlist` (o id dele) | `itens` (as faixas, ou os álbuns do artista) |
| `resolver` | `faixa` (o id dela) | `fonte` |
| `tchau` | — | `ok`, e o provedor sai |

O Player abre o provedor na pasta dele e passa em `TCCPLAYER_PROVEDOR_DADOS` uma pasta só dele
(`<pasta de dados>/provedores-dados/<serviço>`), para guardar cache ou login. Provedores que vêm com o app (hoje,
o YouTube Music) ficam ao lado do `player.exe` e entram na lista sem instalar.

Nenhum campo do pedido se chama `id` além do número do pedido (por isso `resolver` usa `faixa`, e não `id`).
Qualquer pedido pode ser respondido com `{"id":7,"resposta":"erro","mensagem":"..."}`. Um pedido que o provedor
não conhece também (assim versões novas do Player falam com provedores antigos).

A **faixa** é igual em todo serviço:

```json
{"tipo":"faixa","servico":"exemplo","id":"123","isrc":"BRXXX2600001","titulo":"Nome","artistas":["Artista"],
 "album":"Álbum","duracao_s":201.5,"capa":"https://...","video":false}
```

A **fonte** diz como tocar:

- `{"fonte":"endereco","url":"https://...","cabecalhos":[["User-Agent","..."]],"formato":"m4a","duracao_s":201.5}`:
  o motor do Player baixa (aos pedaços, com `Range`), decodifica, mistura e pré-analisa.
- `{"fonte":"toca-sozinho","pid":1234}`: o provedor toca (o player web do Spotify numa WebView2, por exemplo); o
  Player captura o som desse processo para o visual, como no Modo Escuta.

## Casar uma faixa entre serviços

Do MixMusic (MIT): nota de 0 a 1. Com o mesmo ISRC, 1. Senão, 45% para o título limpo (sem "Remastered", "Radio
Edit", "Official Video"...; "Remix", "Ao Vivo", "Acústico" continuam, porque são outra gravação), 35% para o
artista (o canal "Artista - Topic" do YouTube conta como o artista) e 20% para a duração (até 2 s de diferença
conta inteiro; 12 s ou mais, nada), mais 0,05 se for música em vez de vídeo. Toca quando a nota passa de 0,6.
