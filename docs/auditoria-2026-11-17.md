# Auditoria do Player (17/11/2026)

Varredura do app (`app/`, cerca de 30 mil linhas em Rust) atrás de: a tremida da Ilha, responsividade,
acessibilidade, UX/UI, lógica repetida ou descartável e melhorias de funcionalidade. O que foi corrigido já está no
código, com testes; o resto fica aqui como lista de melhorias, em ordem de prioridade.

## 1. A tremida da Ilha (corrigida)

**Sintoma:** ao passar o mouse (a Ilha cresce e encolhe) e ao trocar o tamanho nos Ajustes, os componentes "tremiam".

**Como foi achada:** uma ajuda de teste nova (`TCCPLAYER_ILHA_DIAGNOSTICO=1`) registra, quadro a quadro durante as
transições, as medidas da forma, onde o conteúdo ficou depois do layout, as trocas de região e os movimentos da
janela. A cópia de teste rodou fora da vista e as transições foram provocadas por cliques simulados.

**Três causas, com os números:**

| Causa | Evidência | Conserto |
| --- | --- | --- |
| O GPUI arredonda a posição de cada elemento em relação ao pai. O corpo da Ilha ficava numa posição fracionária que mudava a cada quadro, e o conteúdo, que devia ficar parado, pulava 1 px para lá e para cá. | Conteúdo em x = 42,4 na conta, mas 42, 43, 42, 43... depois do layout; altura alternando 278 e 279. | `ilha::caixas`: corpo, recorte e conteúdo em pixels inteiros. Depois: x = 104 em todos os 216 quadros. |
| A região da janela (o que aparece e recebe cliques) era trocada a cada quadro durante o quique. | 8 trocas seguidas, uma a cada 7 ms. | `ilha::regiao_do_quadro`: só troca no começo e quando a forma assenta. |
| Trocar o tamanho da Ilha redimensionava a janela, e o primeiro quadro saía no tamanho velho. | Janela indo de 640x242 para 781x295 e 550x208, com um quadro em 640x242 já na escala nova. | A janela tem sempre o tamanho do maior tamanho da Ilha; o conteúdo do tamanho velho esmaece e o novo entra. |

Também: o conteúdo é recortado longe dos cantos arredondados de baixo (não aparece mais fora da forma durante a
mudança) e não desliza mais 4 px ao entrar (só esmaece).

Testes novos: `conteudo_fica_parado_enquanto_a_forma_muda`, `recorte_fica_dentro_dos_cantos_arredondados` e
`regiao_nao_muda_a_cada_quadro_no_quique`.

## 2. Responsividade

O arranjo (`arranjo.rs`) já é bem coberto: quatro modos (largo acima de 1060 px, gaveta de 761 a 1060, empilhado até
760, mini), testados em dez tamanhos de janela, de 320x400 a 1920x1080, sem nada sair da janela nem se sobrepor. A
janela mínima (360x560) fica acima do mínimo do arranjo. Todas as páginas (Biblioteca, Músicas, Fila, Loja,
Configurações, painel, dock, topo) mudam de forma pela largura.

**Passada visual** (cópia de teste fora da vista, fotografada por `PrintWindow`), a 380x720 (empilhado), 900x640
(gaveta) e 1280x800 (largo), nas páginas Início, Biblioteca e Loja:

- **Corrigido: no empilhado, a Biblioteca e a Loja ficavam espremidas no cartão de cima** (uns 200 px), dividindo a
  altura com o painel de módulos: a lista de músicas e os módulos da Loja nem apareciam. Agora, com uma página com
  lista aberta, ela fica com o meio inteiro (`Pedido::pagina_cheia` no arranjo, com teste). O Início e as
  Configurações seguem como antes.
- **Corrigido: o cabeçalho da Biblioteca gastava a altura toda em janela estreita.** Agora a descrição sai (também
  na Loja) e "Atualizar", "Adicionar pasta" e "Tocar tudo" viram botões só de ícone, com a dica. A 380x720, a
  lista aparece; na janela larga nada mudou.
- Gaveta (900 px) e largo (1280 px): sem problema.

## 3. Acessibilidade

| Ponto | Situação |
| --- | --- |
| Contraste do texto | **Corrigido.** O teste só conferia o texto sobre a parede. Sobre a linha selecionada, o texto terciário ficava abaixo de 4,5:1 (Noite 4,04; Grafite 4,45). Clareado de leve nos dois temas, e o teste agora confere parede, blocos e seleção nos 4 temas. |
| Movimento reduzido | **Corrigido.** A janela principal ignorava o "Efeitos de animação" do Windows e o modo seguro (só a Ilha respeitava). Agora as duas vão direto ao estado final quando ele está desligado. |
| Teclado | **Melhorado.** Início, Biblioteca e Loja ganharam atalhos (H, B, L). Toda ação já está na barra de comandos (Ctrl K) e os atalhos não disparam enquanto um campo de texto está ativo. O GPUI 0.2.2 não tem navegação por Tab entre botões nem anel de foco. |
| Leitor de tela | Limitação do GPUI: no Windows ele não publica a árvore de acessibilidade (UI Automation). Não há o que fazer no app até o GPUI ter isso. |
| Texto pequeno | **Corrigido na Ilha:** na Ilha mínima as legendas caíam para 9,5 px; agora o piso é 11 px. |
| Tamanho dos alvos | Bom: os botões clicáveis têm 24 px ou mais (o mínimo do WCAG 2.5.8 para mouse). |
| Botões só com ícone | No app, todos têm dica (nome e atalho). **Na Ilha, não:** anterior, próxima, abrir o Player, fechar e as abas sem nome. Ver melhorias. |

## 4. UX/UI

- Estados vazios e de espera existem em todas as listas ("Lendo as músicas...", "Nenhuma música com esse nome.",
  fila vazia com instrução), e ações destrutivas têm "Desfazer" no aviso.
- O × da Ilha desliga a Ilha (o ajuste), como pedido; ela volta pela tecla D ou nas Configurações.
- A letra sem tempos marcados aparece no painel (rolando na proporção da música), mas a Ilha só mostra um aviso.

## 5. Código: repetição e o que dá para resumir

**Juntado nesta auditoria:**
- `normalizar` (texto para comparar e buscar) existia igual em `biblioteca` e `letras`: agora é uma só, em
  `nucleo::texto`. O resultado é o mesmo, então as chaves de letra e o índice da biblioteca já gravados continuam
  valendo. A da barra de comandos é diferente de propósito (mantém a pontuação) e ficou.
- `smoothstep` existia três vezes (`analise::diretor`, `analise::ritmo`, `preanalise::pista`): ficou só a do
  diretor. A conta é idêntica, então a calibração não muda.
- A Ilha tinha o próprio ajudante de transparência de cor; usa o do `ui`.
- Um comentário do `quadro()` estava em cima da função errada.

**Funções públicas sem uso:** nenhuma (307 funções públicas nos crates, todas usadas).

**Para resumir depois (sem pressa, sem mudar comportamento):**
- `app.rs` (2 mil linhas), `ui/ilha.rs` (1,6 mil) e `ui/painel.rs` (1,3 mil) cresceram demais. Dá para separar:
  as páginas da Ilha num `ui/ilha/paginas.rs`; no `app.rs`, as partes de letra, volume e módulos em arquivos
  próprios (como já foi feito com `fila.rs` e `local.rs`).
- A conversão de erro do SQLite (`fn erro(rusqlite::Error)`) se repete em `biblioteca`, `historico` e `preanalise`.

## 6. Desempenho

- A Ilha pede um quadro novo a cada quadro enquanto está à mostra, mesmo parada (música pausada, sem animação).
  Dá para cair para alguns quadros por segundo quando nada se move.
- A camada do visual da Ilha agora só desenha na página Visual (antes limpava a camada a cada quadro).

## 7. Melhorias sugeridas (em ordem)

1. **Dicas nos botões da Ilha** (anterior, próxima, abrir o Player, fechar e as abas) e o nome da aba ao passar o mouse.
2. **Ilha econômica parada:** menos quadros por segundo quando nada se move.
3. **Letra sem tempos na Ilha:** rolar na proporção da música, como o painel já faz.
4. **Separar os arquivos grandes** (item 5).
5. **CI:** o Controle Inteligente de Aplicativos bloqueou três arquivos de compilação hoje (duas macros e um
   executável de teste). Um CI no GitHub tiraria a dependência deste PC para saber se está tudo verde.
6. **Fresta e tremida na tela de verdade:** a foto da janela (`PrintWindow`) pinta o transparente de preto e não
   mostra movimento; a última palavra é do Rafael, na tela dele.

## 8. Como foi conferido

- `cargo fmt`, `cargo clippy --workspace --all-targets` sem aviso, 239 testes passando (5 novos).
- Diagnóstico quadro a quadro antes e depois do conserto da tremida (troca de tamanho e abrir/fechar).
- Cópia de teste fora da vista, fotografada por `PrintWindow` (ver a passada visual acima).
