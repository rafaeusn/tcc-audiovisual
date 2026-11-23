# F4: Letras 2.0

Feita em 22/11/2026. Plano em [`plano-v2.md`](plano-v2.md), fase F4.

## O que entrou

| Parte | Onde |
| --- | --- |
| Fontes em cadeia: Better Lyrics, Unison e LRCLIB, na ordem dos ajustes | `crates/letras/src/fontes.rs` (`Fonte`, `Achada`, `em_cadeia`) |
| TTML (Better Lyrics, Unison) convertido em LRC com o tempo de cada palavra; vocais de fundo entre parênteses | `crates/letras/src/ttml.rs` |
| Tradução pela Unison, até 200 linhas por pedido | `crates/letras/src/traducao.rs` (`Cliente::traduzir`) |
| Letras latinas feitas no computador (`any_ascii`) | `crates/letras/src/traducao.rs` (`romanizar`) |
| A letra guardada com a fonte; acerto de sincronia e traduções guardados | `crates/historico` (colunas e tabelas `letras.fonte`, `atrasos_de_letra`, `traducoes_de_letra`) |
| A thread das letras: cadeia, acerto, tradução | `player/src/letrista.rs` |
| Configurações › Letras | `player/src/ui/configuracoes.rs` (`bloco_letras`) e os ajustes `letra-*` |
| Texto embaixo de cada linha (módulo, Ilha, tela cheia) | `Player::extra_da_linha`, `ui/painel/letra.rs`, `ui/ilha/conteudo.rs` |
| Letra em tela cheia (Y) | `player/src/ui/letra_cheia.rs` |

## Como a cadeia decide

- Cada fonte devolve uma `Achada` com nota: 3 palavra por palavra, 2 por linha ou instrumental, 1 só o texto, 0 nada.
- A primeira com a letra sincronizada vence. Com "Preferir palavra por palavra", a cadeia continua até achar uma
  com nota 3 e, se não achar, fica com a melhor que viu.
- Uma fonte que falha pela rede é pulada. Só quando todas falham a busca volta como "sem conexão" (e tenta de novo).
- Uma letra guardada antes da F4 (sem fonte, só do LRCLIB, sem palavras) aparece na hora e a cadeia roda uma vez; a
  nova só entra se for melhor.

## Os serviços

- **Better Lyrics**: `GET api.betterlyrics.org/getLyrics?s=&a=&al=&d=`, devolve `{ttml}`. 401, 404 e 422 querem dizer
  "não tem". Precisa do álbum e da duração para acertar.
- **Unison**: `GET unison.betterlyrics.org/lyrics?v=<id do vídeo>` ou por `song`, `artist`, `album` e `duration`.
  `POST /translate` com `{lines, to, videoId?}` traduz pelo Google e devolve a tradução e se a linha precisava.
- **LRCLIB**: como antes.
- O agente é genérico (`Player-Escuta/<versão>`); nada da pessoa vai junto. O manifesto do módulo Letra (versão 2)
  declara os três endereços, e quem já tinha o módulo aceita de novo na Loja.

## Testado

- Testes de unidade: TTML (palavras, vocais de fundo, entidades, tradução e romanização embutidas ignoradas), a nota
  e a regra da cadeia, letras latinas (cirílico, grego, kana, hangul, hanzi, latino com acento), os ajustes e o banco
  (acerto e tradução por idioma).
- Ao vivo, pelo exemplo `cargo run -p letras --example buscar_letra -- "<título>" "<artista>" "<álbum>" <s>`: a Better
  Lyrics devolveu uma faixa palavra por palavra e a tradução voltou para quase todas as linhas.
- Na cópia de teste: a faixa do YouTube Music com a letra palavra por palavra da Better Lyrics, a tradução no módulo e
  na tela cheia, o acerto guardado e a seção Letras.

## Ficou para depois

- Estilos da tela cheia e a bolinha nas pausas.
- O cartão para compartilhar um trecho.
- A letra desenhada no visual.
- Leitura japonesa do kanji (hoje vira a leitura chinesa).
