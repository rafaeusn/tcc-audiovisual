# Fase 1 (plano v3): arrumar a casa

**Objetivo:** o CI no GitHub, os consertos pequenos do inventário (plano v3, seção 3) e o roteiro da sessão de conferência.

**Plano:** [`../../plano-v3.md`](../../plano-v3.md), fase 1. **Execução:** nesta sessão, sem commit até o Rafael pedir.

## Decisões

- **CI:** GitHub Actions no Windows (o app é só para Windows), com a mesma versão do Rust daqui (1.99.1), `fmt --check`, `clippy -D
  warnings` e os testes sem o `visual --test desenho` (sem placa de vídeo no servidor). Roda no `main` e nos pedidos de mudança.
- **O teste da placa** ganha um prazo (3 min): passou disso, ele sai com erro em vez de travar.
- **Rolar até o focado:** cada elemento focável ganha um foco próprio (`FocusHandle`), de um estoque refeito a cada quadro; com o Tab,
  ele anota onde foi desenhado, e a página rola até ele.
- **As fotos largas** saem da placa de vídeo (e do cache de imagens do GPUI) quando a página muda.
- **BPM no silêncio:** sem pulso nenhum na música (a curva de ataques sem variação), o BPM fica 0 ("—").
- **Número da faixa:** `Faixa.numero` (opcional) no protocolo; o álbum usa ele, e a posição na lista quando não vem.
- **"Tocar" na página da faixa:** com música tocando, ela entra agora e a fila continua depois dela.
- **O coração que pisca:** o pedido de curtir que chega depois de um descurtir não volta o coração.
- **Ids que colidem:** os ids das linhas e dos artistas passam a ter espaço para qualquer quantidade.
- **Renomear playlist:** no menu da playlist (e na página dela), pela janela de playlist que já existe.
- **A capa local no Windows:** a capa embutida vai para um arquivo em `<dados>/capas-locais` e o controle de mídia a recebe por `file://`.
- **Estante:** no empilhado, a cena é sempre a da janela inteira (o visual mostra o meio dela), então o X não muda o tamanho
  dela e a forma não salta. A página larga anda em dois tempos, sem um passar por cima do outro: o visual baixa até a altura
  da tira e a página desce ao lado; depois o visual alarga e a página desliza por baixo (a primeira versão, tudo junto, fazia
  a página cruzar o visual no meio do caminho).
- **O esticão do volume** conta em medida das telas.
- **Spotify:** a posição continua no Match quando o Soloist cai; a escuta segue o Soloist pelo processo que o provedor abriu (e não pelo
  nome, que confunde com o do Spotifast); uma segunda cópia do Player na mesma pasta de dados chama a primeira e fecha; a lista cortada em
  2000 diz quantas faltam.
- **As ações que aparecem só com o mouse** (tocar a seguir e pôr na fila) ficam fora do Tab: o ⋯ da mesma linha, que o Tab alcança,
  tem as duas. Decidido para não encher o Tab de paradas repetidas.

## Conferência

O roteiro da sessão com o Rafael fica em `docs/conferencia-fase1.md`: o que só foi visto em foto, item por item, com o que fazer e o que
deve acontecer.
