# F6, parte 2: os dados do Spotify — plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** O espaço do Spotify mostra o Para você, a Busca, as páginas (álbum, artista, playlist) e a Biblioteca da conta
(playlists, Curtidas, álbuns, artistas), tudo pela sessão do librespot.

**Architecture:** Um transporte só, o Pathfinder (`api-partner.spotify.com/pathfinder/v2/query`), com o portador do
Login5 e o client token da sessão. A conversão do JSON para os modelos é pura (`conversao.rs`); o `dados.rs` junta as
páginas. Os códigos das consultas vêm embutidos e, quando o Spotify os troca (HTTP 412), são relidos do texto do site
(como o limusic). No Player entram `PedidoDePagina::Biblioteca` e a lista longa que monta só as linhas à vista.

**Tech Stack:** Rust 2024, `reqwest` 0.12 (o do `librespot-oauth`), `serde_json`, `tokio`, GPUI 0.2.2.

**Spec:** [`docs/superpowers/specs/2026-11-23-f6-spotify-design.md`](../specs/2026-11-23-f6-spotify-design.md), seções 5.2 e
5.7.

**Referências:** Spotifast (`session_reads.rs`, `made_for_you.rs`: Pathfinder com Login5 e client token), Prism
(`SpotifyWeb.kt`: `searchDesktop`, `libraryV3`, `fetchLibraryTracks`, `fetchPlaylist`), Metrolist (`Pathfinder.kt`), limusic
(`spotify.rs`: reler o código do site). Nenhum código copiado. **Sonda de 23/11/2026, na conta do Rafael:** as 11 consultas
responderam 200; código desconhecido responde 412.

## Global Constraints

- Só leitura: nenhum pedido que mude a conta. Operações permitidas: `searchDesktop`, `home`, `libraryV3`,
  `fetchLibraryTracks`, `fetchPlaylist`, `getAlbum`, `queryArtistOverview`, `queryArtistDiscographyAll`.
- Códigos embutidos (de 23/11/2026): `searchDesktop` `eef7cc54888d91bdd6802623477873caa3948ae173a0c34fd86827b267e94c03`;
  `home` `5fb7da7a03ee2776c0670856609cb85a189a3cecc36cc160b91cde79063b9cf1`; `libraryV3`
  `390c78e5b951029bad359785e69b07b536a509c581cbcd0aded5e5067f187455`; `fetchLibraryTracks`
  `087278b20b743578a6262c2b0b4bcd20d879c503cc359a2285baf083ef944240`; `fetchPlaylist`
  `21ea11a6ac75a5f3e01fd365d29c18e0a4a8b7dc09f89bbb5d73910190c6acaf`; `getAlbum`
  `0b6ce3511e1b02e4a40fa95440f992acbe85080eb308af9194322d708fc1914f`; `queryArtistOverview`
  `eafb1620d783ece118e72fbfede2b0064483e029d3c1aafafa1c2de0e25a89e7`; `queryArtistDiscographyAll`
  `5e07d323febb57b4a56a42abbf781490e58764aa45feb6e3dc0591564fc56599`.
- Do site só se lê texto (a página, o pacote principal, os pedaços `xpui-routes-search` e `xpui-routes-artist`); só vale um
  código de 64 hexadecimais. Nada é executado. O relido fica em `<dados>/codigos.json`.
- Nenhum token, nome de playlist ou resposta da conta vai para o registro. Amostras reais da conta não entram no repositório.
- Uma consulta que falha vira erro com frase; nunca uma lista vazia como se não houvesse nada.
- Até 2000 faixas por playlist ou Curtidas (páginas de 100, 4 por vez); a página inteira cabe em 18 s (o Player espera 20 s).
- Podcasts, episódios e arquivos locais ficam de fora.

## Review Focus

1. A sessão caiu (o PC dormiu, a rede trocou): o pedido seguinte reentra sozinho uma vez e responde; sem rede, erro com frase
   e a conta continua `dentro`.
2. O Spotify revogou o login no meio do uso: a conta vira `fora` com o motivo, e o Player mostra o cartão.
3. O Spotify trocou um código: o pedido relê, tenta de novo e guarda; se não achar, diz que o Spotify mudou e não repete a
   leitura do site a cada pedido (uma vez a cada 10 min).
4. Curtidas ou playlist com milhares de faixas: abre, rola liso e o Aleatório usa todas as carregadas.
5. Campos faltando ou `null` (álbum sem capa, artista sem foto, item indisponível, tipo novo): o item é pulado ou aparece sem
   o campo; o resto da página continua.

---

### Task 1: O transporte e os códigos (`codigos.rs`, `pathfinder.rs`)

**Files:** Create `app/provedores/spotify/src/codigos.rs`, `src/pathfinder.rs`; Test nos próprios arquivos.

**Interfaces — Produces:**
- `codigos::Codigos::novo(arquivo: PathBuf)`, `.de(op) -> Option<String>`, `.guardar(op, codigo)`; puros
  `pacote_principal(html) -> Option<String>` (o nome do arquivo), `codigo_no_texto(js, op) -> Option<String>`,
  `pedacos_da_rota(js, rota) -> Vec<String>`.
- `pathfinder::Acesso { portador, cliente }`; `Falha { Sessao, Limite(u64), Codigo, Rede(String), Resposta(String) }` com
  `.frase()`; `Pathfinder::novo(Enderecos, Codigos)`, `async consultar(&self, &Acesso, op, variaveis: Value) -> Result<Value, Falha>`
  (devolve o `data`).

- [ ] Testes: os três puros com trechos sintéticos (código com 63 dígitos ou com letra fora do hexa não vale; rota com dois
  códigos devolve os dois arquivos); `corpo` com a operação, as variáveis e o código; com um servidor HTTP local de roteiro:
  `consulta_manda_o_portador_e_devolve_o_data`, `codigo_trocado_e_relido_do_site_e_guardado` (412 → página → pacote → 200),
  `codigo_que_vive_num_pedaco_e_achado`, `sem_codigo_novo_nao_le_o_site_de_novo` (a segunda consulta não bate no site),
  `limite_espera_sem_bater_de_novo` (429 com `Retry-After: 7` → `Limite(7)`, e o pedido seguinte falha sem rede),
  `portador_vencido_e_sessao` (401 → `Sessao`), `erro_no_corpo_nao_vira_vazio` (`errors` com `data` nulo → `Resposta`).
- [ ] Ver falhar; implementar; `cargo test -p provedor-spotify`.

### Task 2: A conversão (`conversao.rs`)

**Files:** Create `app/provedores/spotify/src/conversao.rs`; fixtures pequenas e inventadas (nomes fictícios) nos testes.

**Interfaces — Produces** (todas sobre o `data`): `busca(&Value, limite) -> Result<Vec<Item>, String>`,
`inicio(&Value) -> Result<Vec<Prateleira>, String>`, `album(&Value) -> Result<(Vec<Faixa>, u32), String>` (o total),
`populares(&Value) -> Result<Vec<Faixa>, String>`, `discografia(&Value) -> Result<(Vec<Album>, u32), String>`,
`playlist(&Value) -> Result<(Vec<Faixa>, u32), String>`, `curtidas(&Value) -> Result<(Vec<Faixa>, u32), String>`,
`biblioteca(&Value) -> Result<(Vec<Item>, u32), String>`; `capa(&Value) -> Option<String>` (a menor com 300 px ou mais; sem
medidas, a primeira).

- [ ] Testes: faixa completa (id da `uri`, artistas com ids, álbum com id, duração de `duration` ou `trackDuration`, capa);
  busca com o melhor primeiro e sem repetir, `SearchSectionEntity` pulado, limite por tipo; início com título de
  `transformedLabel`, a fileira sem título vira "Para começar", as de um item só (`HomeFeedBaselineSectionData`) juntas em
  "Sugestões para você", podcasts fora, prateleira vazia fora; álbum com o álbum, a capa e o artista do cabeçalho em cada
  faixa; discografia com o ano; playlist e Curtidas pulando episódio e `spotify:local:`, sem mexer no total; Biblioteca com
  as Curtidas primeiro (`Playlist` de id `curtidas`, nome "Músicas Curtidas", `quantas`); campo nulo não derruba a página;
  sem o nó esperado → `Err`.
- [ ] Ver falhar; implementar; verde.

### Task 3: A sessão que volta (`conta.rs`) e os dados (`dados.rs`, `main.rs`)

**Interfaces — Produces:** `Login::reconectar(&self) -> Result<(), String>` (só em `Dentro`; renova, guarda e conecta na
thread de quem chama; `Recusada` → `Fora` com o motivo e sem credencial; `Passageira` → continua `Dentro`);
`dados::paginas_que_faltam(total, por_pagina, maximo) -> Vec<u32>`; `Dados::{buscar, inicio, album, artista, playlist, biblioteca}`;
capacidades `Busca, Inicio, Album, Artista, Playlists, Biblioteca, Conta`.

- [ ] Testes: `reconectar_troca_o_portador_e_continua_dentro`, `reconectar_recusado_sai_com_o_motivo`,
  `reconectar_sem_rede_continua_dentro`, `sair_durante_o_reconectar_nao_volta`; `paginas_que_faltam` (0, exata, sobra, teto);
  `fora_da_conta_a_busca_vem_vazia_e_a_pagina_diz_para_entrar`; `tocar_diz_que_chega_na_proxima_parte`.
- [ ] Ver falhar; implementar (`consultar` tenta de novo uma vez depois de `Sessao`, com `reconectar`); verde.

### Task 4: O protocolo e o Player

**Files:** Modify `crates/provedores/src/protocolo.rs` (`CorpoDoPedido::Biblioteca`), `provedor-exemplo.rs`
(`Biblioteca` responde uma playlist), `docs/f1-provedores.md`; `player/src/remoto.rs`
(`PedidoDePagina::Biblioteca { servico }`; depois de um erro, pergunta a conta se o provedor tem), `player/src/paginas.rs`
(`juntar_biblioteca`, `janela_de_linhas`), `ui/espacos.rs`, `ui/paginas.rs`, `ui/estante.rs`, `ui/heroi.rs`.

**Interfaces — Produces:** `paginas::NaBiblioteca { item: Item, salvo: bool, fixado: bool }`,
`paginas::juntar_biblioteca(salvos: &[(Salvo, Option<Item>)], da_conta: &[Item], servico: Option<&str>) -> Vec<NaBiblioteca>` (fixados, depois a
conta na ordem dela, depois o resto dos salvos; sem repetir por tipo e id); `paginas::janela_de_linhas(topo_da_lista: Option<f32>,
visivel: (f32, f32), linha: f32, total: usize) -> Range<usize>` (folga de 12 linhas; sem medida ainda, as 40 primeiras).

- [ ] Testes das duas puras, do pedido no fio, da página da Biblioteca pelo `provedor-exemplo` e de
  `erro_de_pagina_atualiza_a_conta`. Ver falhar; implementar; verde.
- [ ] Telas: a Biblioteca do serviço e a lateral da Estante mostram a conta e os salvos; o filtro vale para os dois; o alfinete
  salva antes de fixar o que só está na conta; carregando, esqueleto; erro, "Tentar de novo" sem esconder os salvos. A
  lista da página de detalhe monta só as linhas de `janela_de_linhas`.

### Task 5: Conferir e documentar

- [ ] Tirar a sonda; bateria inteira; cópia de teste (`PrintWindow`) com a conta: Para você, Busca, álbum, artista, playlist,
  Biblioteca, uma playlist longa rolando até o fim, a lateral; `README`, `AGENTS.md`, `THIRD-PARTY-NOTICES.md`; revisão final.
