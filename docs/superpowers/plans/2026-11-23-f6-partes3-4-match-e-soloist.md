# F6, partes 3 e 4: o Match e o Soloist

> **Para agentes:** execução nativa (superpowers:executing-plans), como nas partes anteriores. Sem commits até o Rafael pedir.

**Meta:** as músicas do Spotify tocam no Player: pelo Soloist (o player oficial do Spotify, ligado pela pessoa) ou, sem ele,
pela mesma música no YouTube Music (o Match).

**Desenho:** `docs/superpowers/specs/2026-11-23-f6-spotify-design.md`, seção 8.

**Restrições:** nada pede chave de áudio nem decifra faixa do Spotify; a chave do Soloist é colada pela pessoa, guardada com DPAPI e
nunca vai para registro, tela ou erro; o programa do Spotify só é baixado quando a pessoa escolhe; nada de redistribuir.

**Decisão nova (o porquê):** o Soloist fica **aberto** (como no Spotifast) e recebe `play` com a faixa pela API local, em vez de abrir um
processo por faixa com `--single-track`: cada troca levaria segundos (abrir, entrar na sessão). O pareamento é o mesmo.

## Tarefas

- [x] **1. O Match** (`pares.rs`, `remoto.rs`, `preanalista.rs`, `local.rs`, `casador.rs`, `ui/faixa.rs`, `app/salvos.rs`).
  - O casador ganhou piso de título e de artista e desconto para outra versão: a mesma duração sozinha chegava ao limiar.
- [x] **2. O protocolo** (`protocolo.rs`, `cliente.rs`, `servidor.rs`, `modelo.rs`): capacidade `toca-sozinho`; pedidos `tocador`,
  `ligar-tocador` (com a chave num tipo que não aparece em `Debug`), `desligar-tocador`, `controlar`; resposta `tocador`
  (`EstadoDoTocador`: situação, nome do aparelho, progresso, erro, o que toca). `Servico::em_segundo_plano`: as páginas rodam numa
  thread, para um pedido lento não segurar a pausa.
- [x] **3. O Soloist no provedor** (`app/provedores/spotify/src/soloist/`): `leitura.rs` (os eventos da API local viram o estado, puro),
  `instalar.rs` (baixar, abrir o zip, a versão e o vencimento), `motor.rs` (o processo, o WebSocket, parear, controlar, reabrir).
- [x] **4. O Player**: `externo.rs` (pergunta o estado e manda os comandos, fora da interface), `QuemToca::Sozinho`, `local.rs` acompanha o
  externo (posição, fim, pausa, pular, falha → Match), `escuta` ouve `soloist.exe`, avisos.
- [x] **5. A tela**: Configurações › Serviços (Spotify: por onde toca, colar a chave, baixar, parear, desligar), a página da faixa.
- [x] **6. Bateria, release, cópia de teste, revisão final, documentos.**

## Pontos para a revisão

- A faixa acaba e o Spotify emenda outra (autoplay): o Player precisa parar e tocar a próxima da fila dele.
- O Spotify passa a tocar em outro aparelho no meio (a conta toca num lugar só).
- O Soloist vence (90 dias, saída 10), é bloqueado pelo Windows (4551) ou fecha no meio da faixa.
- A chave errada ou revogada; a chave em mensagens de erro e no registro.
- Pausar e pular enquanto uma página de 16 s carrega.
- Fila mista: Spotify, YouTube Music e arquivos, com repetir e aleatório.

## Revisão final

Um revisor à parte achou 1 problema crítico (uma faixa abandonada que começa depois tocava por baixo), 9 importantes e 12 menores. Os
10 primeiros foram corrigidos; os da leitura, do motor, do par e da faixa com o provedor abrindo com teste que falhou antes, os outros com
teste escrito junto (gravar fora da trava ficou sem teste) (a pausa que piscava, a vez devolvida pelo celular, desligar no meio do
download, a pré-análise da gravação errada, a faixa clicada com o provedor abrindo, gravar em disco com a trava da tela, o par trocado
por falha passageira, a posição da faixa anterior), e mais 10 dos menores. Ficaram: duas instâncias na mesma pasta e levar a posição ao
cair para o Match.
