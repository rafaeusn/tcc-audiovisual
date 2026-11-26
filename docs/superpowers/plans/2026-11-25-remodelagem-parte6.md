# Remodelagem, parte 6: artista, álbum e playlist

**Objetivo:** as páginas de artista, álbum, playlist e faixa com imagem de fundo, os números que os serviços já mandam, abas no
artista, e "Ver no outro serviço".

**Desenho:** [`../specs/2026-11-25-remodelagem-streaming-design.md`](../specs/2026-11-25-remodelagem-streaming-design.md), seções 8, 9, 10 e 12.

**Execução:** nesta sessão, sem commit.

## O que precisa dar certo (foco da revisão)

1. Provedor antigo (sem `detalhe`), `detalhe` estragado ou tipo desconhecido: a página aparece como antes.
2. Artista sem foto larga, sem foto nenhuma, sem biografia; álbum de uma faixa; playlist de 2000; nome de 200 letras.
3. Texto sobre qualquer imagem (capa branca, capa preta) com 4,5:1, no tema escuro e no Papel.
4. Homônimo no outro serviço: o link só aparece com o mesmo nome no primeiro resultado.
5. A página rolada: a imagem não vaza do canto redondo da folha.

## Tarefas

### 1. Formato (`crates/provedores`)
- `Detalhe` (tudo opcional): `item`, `imagem`, `cor`, `descricao`, `fotos`, `seguidores`, `ouvintes`, `total`, `duracao_s`, `data`,
  `gravadora`, `tipo`, `dono`, `secoes`. `item` é o item completo (o artista aberto por um link ganha a foto).
- `TipoDeAlbum` (`Album`, `Single`, `Ep`, `Coletanea`, `Outro`), em `Album.tipo`; `Faixa.ouvidas`; `TipoDaPrateleira` ganha
  `UltimoLancamento`, `Parecidos` e `MaisDoArtista`.
- `CorpoDaResposta::Itens { itens, detalhe }`: um `detalhe` que não se lê vira `None`; `Provedor::pagina` devolve os dois.
- A divisão em álbuns, singles e coletâneas é feita no Player por `Album.tipo` (não repete a lista na resposta).

### 2. Provedores
- Spotify: artista (`headerImage`, `stats`, `biography` sem HTML, `gallery`, `latest`, `relatedArtists`, `type` de cada lançamento,
  `playcount`), álbum (`label`, `date`, `type`, cor, `moreAlbumsByArtist`), playlist (`description`, `followers`, dono).
- YouTube Music: artista (`header_image`, `description`, `subscriber_count`, `similar_artists`, `playlists`), álbum (`album_type`, `year`,
  `track_count`, `variants`), playlist (`description`, `track_count`, dono, `related_playlists`).

### 3. Imagens (`player/src/capas.rs`)
- `Fundo { arquivo, claro, escuro }`: `fundo(url, Desfocado | Largo)`. Desfocado: 480 × 270, `fast_blur`. Largo: até 1280 de
  largura. `claro` e `escuro` são os percentis 95 e 5 do brilho, para o véu.
- `detalhes::veu(fundo, tema escuro?) -> alfa` (pura).

### 4. Telas (`ui/detalhe.rs`, `detalhes.rs` com a lógica)
- `detalhes.rs`: números ("28,3 milhões", "31 min", "1 h 12 min"), data, abas com conteúdo, divisão por tipo, véu.
- Cabeçalho com imagem; linha de faixa com ouvidas e álbum, sem capa no álbum; abas do artista; Sobre; cantos da folha por cima.

### 5. O artista no outro serviço (`pares.rs`, `remoto.rs`)
- `mesmo_artista(nome, resultados)`: o primeiro resultado é um artista com o mesmo nome (sem acento, sem caixa).
- `Tarefa::AcharArtista`; guardado em `pares.json` com a chave `artista:<serviço>:<id>:<destino>`.

### 6. Conferência e documentos
