'use strict';
/* Protótipo da remodelagem: monta a janela do Player nas opções e tamanhos escolhidos. Nomes e capas são inventados. */
const IC = {"home": "<path d=\"M4 10.6 12 4l8 6.6V19a1 1 0 0 1-1 1h-4.5v-5.5h-5V20H5a1 1 0 0 1-1-1z\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "home-f": "<path d=\"M3.6 10.2 11.1 4.1a1.4 1.4 0 0 1 1.8 0l7.5 6.1c.4.3.6.7.6 1.2V19a1.5 1.5 0 0 1-1.5 1.5H15v-5.5a1 1 0 0 0-1-1h-4a1 1 0 0 0-1 1v5.5H4.5A1.5 1.5 0 0 1 3 19v-7.6c0-.5.2-.9.6-1.2z\" fill=\"currentColor\"/>", "search": "<path d=\"M10.75 17.5a6.75 6.75 0 1 0 0-13.5 6.75 6.75 0 0 0 0 13.5zM15.75 15.75 20 20\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "lib": "<path d=\"M7 4.5h10M5.5 7.5h13M5 10.5h14a1 1 0 0 1 1 1v7a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1v-7a1 1 0 0 1 1-1z\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "lib-f": "<path d=\"M7 3.75h10a.75.75 0 0 1 0 1.5H7a.75.75 0 0 1 0-1.5zM5.5 6.75h13a.75.75 0 0 1 0 1.5h-13a.75.75 0 0 1 0-1.5zM5 9.75h14a1.75 1.75 0 0 1 1.75 1.75v7A1.75 1.75 0 0 1 19 20.25H5a1.75 1.75 0 0 1-1.75-1.75v-7A1.75 1.75 0 0 1 5 9.75z\" fill=\"currentColor\"/>", "store": "<path d=\"M5.2 8h13.6l-1 11a1.5 1.5 0 0 1-1.5 1.4H7.7a1.5 1.5 0 0 1-1.5-1.4zM9 8V7a3 3 0 0 1 6 0v1\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "sliders": "<path d=\"M4 7h9M17 7h3M15 4.8v4.4M4 17h3M11 17h9M9 14.8v4.4\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "heart": "<path d=\"M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linejoin=\"round\"/>", "heart-f": "<path d=\"M12 20s-7-4.4-7-10a4 4 0 0 1 7-2.6A4 4 0 0 1 19 10c0 5.6-7 10-7 10z\" fill=\"currentColor\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linejoin=\"round\"/>", "shuffle": "<path d=\"M4 7h3.2c1.6 0 3.1.8 4 2.1l1.6 2.3M4 17h3.2c1.6 0 3.1-.8 4-2.1l2.6-3.8c.9-1.3 2.4-2.1 4-2.1H20M17.5 6.5 20 9l-2.5 2.5M15.6 15.1c.9 1.2 2.3 1.9 3.8 1.9H20M17.5 14.5 20 17l-2.5 2.5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "prev": "<path d=\"M6.25 5.5a1.25 1.25 0 0 1 2.5 0v13a1.25 1.25 0 0 1-2.5 0zM19 6.4v11.2a1.1 1.1 0 0 1-1.7.92l-7.8-5.6a1.1 1.1 0 0 1 0-1.84l7.8-5.6A1.1 1.1 0 0 1 19 6.4z\" fill=\"currentColor\"/>", "next": "<path d=\"M17.75 5.5a1.25 1.25 0 0 0-2.5 0v13a1.25 1.25 0 0 0 2.5 0zM5 6.4v11.2a1.1 1.1 0 0 0 1.7.92l7.8-5.6a1.1 1.1 0 0 0 0-1.84l-7.8-5.6A1.1 1.1 0 0 0 5 6.4z\" fill=\"currentColor\"/>", "play": "<path d=\"M8 5.65v12.7a1.1 1.1 0 0 0 1.66.95l10.55-6.35a1.1 1.1 0 0 0 0-1.9L9.66 4.7A1.1 1.1 0 0 0 8 5.65z\" fill=\"currentColor\"/>", "pause": "<path d=\"M7.5 5h2a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1zm7 0h2a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1h-2a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z\" fill=\"currentColor\"/>", "repeat": "<path d=\"M5 11V9.5A2.5 2.5 0 0 1 7.5 7H19M16.5 4.5 19 7l-2.5 2.5M19 13v1.5a2.5 2.5 0 0 1-2.5 2.5H5M7.5 19.5 5 17l2.5-2.5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "queue": "<path d=\"M4 6h12M4 11h12M4 16h7M15 15v5l4-2.5z\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "lyrics": "<path d=\"M4.5 6.5h15M4.5 11h9M4.5 15.5h15M4.5 20h6\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "panel": "<path d=\"M4.5 6a1.5 1.5 0 0 1 1.5-1.5h12A1.5 1.5 0 0 1 19.5 6v12a1.5 1.5 0 0 1-1.5 1.5H6A1.5 1.5 0 0 1 4.5 18zM14.5 4.5v15\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "expand": "<path d=\"M14 4.5h5.5V10M10 19.5H4.5V14M19.5 4.5l-6 6M4.5 19.5l6-6\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "vol": "<path d=\"M4 9.5h3.2L12 5.6v12.8l-4.8-3.9H4zM15.5 9a4 4 0 0 1 0 6M18.2 6.4a7.6 7.6 0 0 1 0 11.2\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "chev-l": "<path d=\"M14 7l-5 5 5 5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "chev-r": "<path d=\"M10 7l5 5-5 5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "chev-u": "<path d=\"M7 14l5-5 5 5\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "album": "<path d=\"M12 3.5a8.5 8.5 0 1 0 0 17 8.5 8.5 0 0 0 0-17zM12 10a2 2 0 1 0 0 4 2 2 0 0 0 0-4z\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "plus": "<path d=\"M12 5v14M5 12h14\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "x": "<path d=\"M6.5 6.5l11 11M17.5 6.5l-11 11\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "clock": "<path d=\"M12 20.5a8.5 8.5 0 1 0 0-17 8.5 8.5 0 0 0 0 17zM12 7.5V12l3 2\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "min": "<path d=\"M5 12h14\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "max": "<path d=\"M6 6h12v12H6z\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "full": "<path d=\"M4.5 9V4.5H9M15 4.5h4.5V9M19.5 15v4.5H15M9 19.5H4.5V15\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "pip": "<path d=\"M20 10.5V7a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v10a2 2 0 0 0 2 2h4.5M14.5 14h5a1 1 0 0 1 1 1v3.5a1 1 0 0 1-1 1h-5a1 1 0 0 1-1-1V15a1 1 0 0 1 1-1z\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>", "grid": "<path d=\"M4.5 5.5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1zM13.5 5.5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1zM4.5 14.5a1 1 0 0 1 1-1h4a1 1 0 0 1 1 1v4a1 1 0 0 1-1 1h-4a1 1 0 0 1-1-1zM16.5 13.5v6M13.5 16.5h6\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"1.8\" stroke-linecap=\"round\" stroke-linejoin=\"round\"/>"};
const ic = (n, c = '') => `<svg class="ic ${c}" viewBox="0 0 24 24" aria-hidden="true">${IC[n] || ''}</svg>`;

/* as marcas dos serviços, desenhadas em linha para formarem uma família */
const GLIFO = {
  sp: '<g fill="none" stroke="#1ed760" stroke-linecap="round"><path d="M4.6 8.2c4.9-1.5 10.6-1.1 14.9 1.4" stroke-width="2.4"/><path d="M5.6 12.3c4.1-1.1 8.8-.7 12.4 1.3" stroke-width="2.1"/><path d="M6.6 16.2c3.3-.8 7-.5 9.8 1" stroke-width="1.8"/></g>',
  yt: '<circle cx="12" cy="12" r="8.2" fill="none" stroke="#ff2d4a" stroke-width="2.2"/><path d="M10 8.6v6.8l5.6-3.4z" fill="#ff2d4a"/>',
  pc: '<path d="M3.5 7.2a1.6 1.6 0 0 1 1.6-1.6h4l2 2.2h7.8a1.6 1.6 0 0 1 1.6 1.6v7.4a1.6 1.6 0 0 1-1.6 1.6H5.1a1.6 1.6 0 0 1-1.6-1.6z" fill="none" stroke="#f2f2f3" stroke-width="2" stroke-linejoin="round"/>',
};
const glifo = (s, c = '') => `<svg class="${c}" viewBox="0 0 24 24" aria-hidden="true">${GLIFO[s]}</svg>`;
const NOME = { sp: 'Spotify', yt: 'YouTube Music', pc: 'Neste PC' };
const PASTA = /^(localhost|127\.0\.0\.1)$/.test(location.hostname) || location.protocol === 'file:' ? '../assets/covers/' : 'covers/';
const cp = (n) => `${PASTA}${n}.jpg`;

/* [nome, linha de baixo, tipo, serviço, capa] */
const ATALHOS = [
  ['Músicas curtidas', '', 'pl', 'sp', 'glow'], ['Treino pesado', '', 'pl', 'sp', 'bars'], ['Maré Alta', '', 'al', 'yt', 'silk'], ['Demos de 2024', '', 'pl', 'pc', 'scan'],
  ['Mix do dia', '', 'pl', 'yt', 'flow'], ['Lume', '', 'ar', 'sp', 'orbit'], ['Chuva na serra', '', 'pl', 'sp', 'contour'], ['Baile de garagem', '', 'al', 'pc', 'halftone'],
];
const PRAT = [
  ['Feito para você', [['Mix diário 1', 'Lume, Vera Solar, Telha e mais', 'pl', 'sp', 'sphere'], ['Seu supermix', 'Toca sem parar, do seu jeito', 'pl', 'yt', 'warpgrid'], ['Radar de novidades', 'Atualizada toda sexta', 'pl', 'sp', 'moire'], ['Descobertas da semana', 'Feita para você', 'pl', 'sp', 'dither'], ['Mix de foco', 'Instrumentais calmos', 'pl', 'yt', 'silk'], ['Voltando no tempo', 'O que você ouvia em 2022', 'pl', 'sp', 'flow'], ['Para correr', 'Entre 150 e 170 BPM', 'pl', 'yt', 'bars']]],
  ['Lançamentos', [['Sal na Janela', 'Vera Solar', 'al', 'yt', 'contour'], ['Noite Elétrica', 'Lume', 'al', 'sp', 'glow'], ['Quando a Cidade Apaga as Luzes', 'Ondas Curtas', 'al', 'yt', 'orbit'], ['Ferrugem', 'Telha', 'al', 'sp', 'scan'], ['Meia-Volta', 'Casa Verde', 'al', 'yt', 'halftone'], ['Mapa Mudo', 'Nó Cego', 'al', 'sp', 'sphere'], ['Longe Perto', 'Iara Mar', 'al', 'yt', 'moire']]],
  ['Seus artistas', [['Lume', 'Artista', 'ar', 'sp', 'orbit'], ['Vera Solar', 'Artista', 'ar', 'yt', 'silk'], ['Ondas Curtas', 'Artista', 'ar', 'sp', 'flow'], ['Telha', 'Artista', 'ar', 'yt', 'contour'], ['Iara Mar', 'Artista', 'ar', 'pc', 'glow'], ['Nó Cego', 'Artista', 'ar', 'sp', 'bars'], ['Casa Verde', 'Artista', 'ar', 'yt', 'sphere']]],
  ['Tocadas há pouco', [['Corrente', 'Lume', 'fx', 'sp', 'glow'], ['Vidro', 'Vera Solar', 'fx', 'yt', 'silk'], ['Ensaio 07', 'Gravação sua', 'fx', 'pc', 'scan'], ['Farol', 'Ondas Curtas', 'fx', 'sp', 'orbit'], ['Trilha do ônibus', 'Telha', 'fx', 'yt', 'halftone'], ['Chão', 'Iara Mar', 'fx', 'pc', 'dither'], ['Contramão', 'Nó Cego', 'fx', 'sp', 'moire']]],
];
const FAIXAS = [
  ['Corrente', 'Lume', 'Noite Elétrica', ['sp', 'yt'], 'glow', '3:46', '84,2 mi'], ['Noite Elétrica', 'Lume', 'Noite Elétrica', ['sp'], 'glow', '4:02', '61,9 mi'], ['Farol baixo', 'Lume', 'Farol', ['sp', 'yt'], 'orbit', '3:18', '40,3 mi'],
  ['Segunda pele', 'Lume', 'Farol', ['yt'], 'silk', '2:57', '22,7 mi'], ['Marcha lenta', 'Lume', 'Demos', ['pc'], 'scan', '5:11', ''], ['Alta voltagem', 'Lume', 'Noite Elétrica', ['sp', 'yt'], 'glow', '3:33', '18,0 mi'], ['Curto', 'Lume', 'Noite Elétrica', ['sp'], 'glow', '2:41', '15,4 mi'],
  ['Subestação', 'Lume', 'Noite Elétrica', ['sp'], 'glow', '4:27', '11,8 mi'], ['Fio terra', 'Lume', 'Noite Elétrica', ['sp'], 'glow', '3:09', '9,1 mi'],
];
const LATERAL = [
  ['Músicas curtidas', 'Playlist, 722 músicas', 'pl', 'sp', 'glow'], ['Treino pesado', 'Playlist', 'pl', 'sp', 'bars'], ['Lume', 'Artista', 'ar', 'sp', 'orbit'], ['Seu supermix', 'Playlist', 'pl', 'yt', 'warpgrid'], ['Demos de 2024', 'Pasta, 38 músicas', 'pl', 'pc', 'scan'],
  ['Noite Elétrica', 'Álbum, Lume', 'al', 'sp', 'glow'], ['Sal na Janela', 'Álbum, Vera Solar', 'al', 'yt', 'contour'], ['Chuva na serra', 'Playlist', 'pl', 'sp', 'contour'],
];
const TEXTOS = {
  B: ['B. Lado a lado', 'Biblioteca à esquerda, o visual no meio e as páginas à direita, com uma divisória que você arrasta.', ['O visual fica grande e central o tempo todo, também enquanto você procura música: 518 × 708 px na janela de 1440 (4,2 vezes a área de hoje).', 'Na sua janela de 958 × 560 ele tem 404 × 384 px; hoje, nessa janela, ele nem aparece.', 'A divisória muda a proporção e o app lembra. Há um botão para a página larga (o visual vira uma tira) e outro para só o visual.', 'O Palco continua diferente: lá é só o visual, e a página abre por cima.'], ['A página tem metade da largura: de duas a quatro capas por linha, em vez de cinco.', 'Com Fila, Letra ou Módulos abertos, eles tomam o lugar da página (em janelas de 1680 px para cima, abrem ao lado).']],
  A: ['A. Palco vivo', 'O visual ocupa o miolo da janela, e a página é uma folha que sobe e desce por cima dele.', ['No Início, o visual tem 1140 × 340 px na janela de 1440 (4,5 vezes a área de hoje).', 'A página usa a largura toda, e a foto do artista fica bem larga.'], ['Ao rolar, o visual vira uma tira de 96 px: enquanto você procura música, ele quase some de novo.', 'Na sua janela de 958 × 560 sobra pouca altura: o visual fica com 180 px e aparece uma fileira só.', 'Fica parecida com o Palco.']],
  C: ['C. Vidro', 'O visual ocupa a janela inteira e as páginas são um vidro escuro por cima.', ['O visual nunca muda de lugar nem de tamanho.', 'É o que o Palco de hoje já faz, com a lateral da Estante.'], ['A página cobre justamente o centro, que é onde a forma está.', 'Texto sobre imagem em movimento cansa. Para ler bem, o vidro precisa ficar tão escuro que o visual some.']],
  hoje: ['Como é hoje', 'O visual fica num quadro de 340 px à direita; a página ocupa o meio.', ['A página tem bastante espaço.'], ['O visual ocupa 6,7% da janela de 1440 e some entre 761 e 1060 px de largura.', '“Para você” aparece três vezes, os serviços são ícones sem nome e há três buscas.', 'As capas saem quadradas, o texto é cortado sem reticências e o play encosta na borda da barra.']],
};

const S = { op: 'B', w: 1440, h: 900, pag: 'inicio', filtro: 'tudo', painel: false, aba: 'agora', baixa: false, larga: false, div: null, en: 'agitada', volta: [], busca: '' };
const EN = { calma: ['Calma', '#e9eefb', '#9fb2e6', '#16171b', .35, .12, '0,24', '92'], agitada: ['Agitada', '#3d7bff', '#9cc0ff', '#10182f', .75, .55, '0,52', '106'], intensa: ['Intensa', '#ff3348', '#ff9a8a', '#2c0d13', 1.15, 1.1, '0,86', '150'] };
const $ = (s, r = document) => r.querySelector(s);

/* ---------- medidas da janela ---------- */
function medir() {
  const { w, h, op } = S, M = 12;
  if (op === 'hoje') {
    const est = w <= 760, med = w > 760 && w <= 1060, topo = 52;
    const nav = est ? { x: 0, y: h - 64, w, h: 64 } : med ? { x: 0, y: 0, w: 72, h } : { x: M, y: topo + M, w: 248, h: h - topo - 2 * M };
    const x0 = est ? M : med ? 72 + M : M + 248 + M, dh = est ? 116 : 80;
    const dock = { x: x0, y: est ? h - 64 - 8 - dh : h - M - dh, w: w - M - x0, h: dh };
    const meio = dock.y - M - topo - M, largo = !est && !med;
    const centro = { x: x0, y: topo + M, w: largo ? w - M - 340 - M - x0 : w - M - x0, h: est ? meio - 80 : meio };
    const vis = largo ? { x: w - M - 340, y: topo + M, w: 340, h: Math.min(255, meio * .45) } : est ? { x: M, y: dock.y - 80, w: w - 2 * M, h: 72 } : null;
    return { hoje: true, est, med, topo, nav, dock, centro, vis, M };
  }
  const est = w <= 760, med = w > 760 && w < 1200, baixa = h < 640;
  const topo = est ? 64 : baixa ? 64 : 72, dh = est ? 72 : baixa ? 88 : 96;
  const nav = est ? { x: 0, y: h - 72, w, h: 72 } : med ? { x: 0, y: 0, w: 92, h } : { x: M, y: M, w: 264, h: h - 2 * M };
  const x0 = est ? 8 : med ? 92 : 2 * M + 264;
  const dock = est ? { x: 8, y: h - 72 - dh - 8, w: w - 16, h: dh } : { x: x0, y: h - M - dh, w: w - M - x0, h: dh };
  // na B, abaixo de 1680 px o painel toma o lugar da página; acima, abre ao lado
  const troca = op === 'B' && w < 1680, pw = S.painel && !est && !troca ? Math.min(380, Math.round((w - x0) * .42)) : 0;
  const centro = { x: x0, y: topo, w: w - (est ? 8 : M) - x0 - (pw ? pw + M : 0), h: dock.y - M - topo };
  const tira = est ? 72 : baixa ? 72 : 96;
  const r = { est, med, baixa, topo, nav, dock, centro, tira, M, painel: pw ? { x: w - M - pw, y: topo, w: pw, h: centro.h } : null };
  r.folhaSobe = op === 'A' || (op === 'B' && (S.larga || est));
  if (r.folhaSobe) {
    r.abre = S.baixa ? centro.h + 40 : op === 'A' && S.pag === 'inicio' ? Math.round(centro.h * .48) : op === 'B' && est && S.pag === 'inicio' ? Math.round(centro.h * .4) : tira;
    r.rolo = { x: centro.x, y: centro.y + tira, w: centro.w, h: centro.h - tira };
  } else if (op === 'B') {
    const dv = S.div ?? (centro.w >= 1300 ? .52 : centro.w >= 1000 ? .46 : .48);
    const vw = Math.round(centro.w * dv) - 6;
    r.vis = S.baixa ? centro : { ...centro, w: vw };
    r.rolo = { ...centro, x: centro.x + vw + M + (S.baixa ? centro.w : 0), w: centro.w - vw - M };
    r.divisor = S.baixa ? null : { x: centro.x + vw - 2, y: centro.y, w: M + 4, h: centro.h };
    if (S.painel && troca && !S.baixa) r.painel = { ...r.rolo };
  } else { r.vis = { x: 0, y: 0, w, h }; r.rolo = S.baixa ? { ...centro, y: h + 20 } : centro; }
  r.lado = r.rolo.w < 520 ? 20 : r.rolo.w < 900 ? 28 : 36;
  const util = r.rolo.w - 2 * r.lado;
  r.n = Math.max(2, Math.floor((util + 16) / ((util < 520 ? 150 : util < 760 ? 160 : 180) + 16)));
  r.a = util >= 980 ? 4 : util >= 660 ? 3 : 2;
  r.util = util;
  return r;
}
const cx = (r) => `left:${r.x}px;top:${r.y}px;width:${r.w}px;height:${r.h}px`;

/* ---------- pedaços ---------- */
const selo = (s) => `<span class="selo ${s}">${glifo(s, 'gl')}${ic('play', 'tc')}</span>`;
const capa = (c, tipo, s, mini = '') => `<span class="capa ${tipo} ${mini}"><i style="background-image:url(${cp(c)})"></i>${s}</span>`;
const mostra = () => S.filtro === 'tudo';
const passa = (i) => S.filtro === 'tudo' || i[3] === S.filtro;
const ida = (t) => (t === 'ar' ? 'artista' : 'album');
function cartao([n, sub, t, s, c], extra = '') {
  return `<button class="cart ${t === 'ar' ? 'ar' : ''} ${extra}" data-ir="${ida(t)}">${capa(c, t, selo(s))}<span><span class="el nome">${n}</span><span class="el ap">${sub}</span></span></button>`;
}
function chips(m) {
  const so = m.util < 520;
  return ['tudo', 'yt', 'sp', 'pc'].map((f) => {
    const on = S.filtro === f, g = f === 'tudo' ? '' : glifo(f);
    const t = f === 'tudo' ? 'Tudo' : NOME[f];
    return `<button class="ch sv ${on ? 'on' : ''} ${so && !on && f !== 'tudo' ? 'so' : ''}" data-filtro="${f}" aria-pressed="${on}" aria-label="${t}" ${f === 'tudo' ? 'style="padding:0 16px!important"' : ''}>${on && f !== 'tudo' ? g.replace(/#1ed760|#ff2d4a|#f2f2f3/g, f === 'sp' ? '#0b7a34' : f === 'yt' ? '#d1002a' : '#0b0b0c') : g}${so && !on && f !== 'tudo' ? '' : t}</button>`;
  }).join('');
}
const barra = (m, extra = '') => `<div class="barra">${chips(m)}${extra}</div>`;
function linha(f, i, m, o = {}) {
  // a linha perde colunas, e não o nome: em lista estreita saem o número, o coração e o álbum (ficam no menu)
  const [t, a, d, ss, c, dur, pl] = f, cab = m.util > 640, pq = m.util < 480, num = o.num && !(pq && !o.semCapa);
  const cols = [num ? '28px' : '', o.semCapa ? '' : '48px', 'minmax(0,1.5fr)', cab && !o.curta ? 'minmax(0,1fr)' : '', cab ? '76px' : '', pq ? '' : '44px', '48px', '44px'].filter(Boolean).join(' ');
  return `<div class="fx ${i === 0 && o.agora ? 'agora' : ''}" style="--fc:${cols}${pq ? ';gap:12px;padding:0 4px!important' : ''}">${num ? `<span class="num">${i + 1}</span>` : ''}${o.semCapa ? '' : capa(c, 'al', (o.selos ?? mostra()) ? ss.map(selo).join('') : '', 'mini')}<span style="min-width:0;display:grid"><span class="el forte">${t}</span><button class="el ap lk" data-ir="artista">${a}</button></span>${cab && !o.curta ? `<button class="el ap lk" data-ir="album">${d}</button>` : ''}${cab ? `<span class="ap dir">${pl}</span>` : ''}${pq ? '' : `<button class="btn-ic" aria-label="Curtir">${ic(i % 3 === 0 ? 'heart-f' : 'heart')}</button>`}<span class="ap dir">${dur}</span><button class="btn-ic" aria-label="Mais ações">${pontos()}</button></div>`;
}
const pontos = () => '<svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><g fill="currentColor"><circle cx="5" cy="12" r="1.9"/><circle cx="12" cy="12" r="1.9"/><circle cx="19" cy="12" r="1.9"/></g></svg>';
const prateleira = (t, its, m) => its.length ? `<section class="prat"><h2 class="cond">${t}<button>Ver tudo</button></h2><div class="fila-c" style="--n:${m.n}">${its.slice(0, m.n).map((i) => cartao(i)).join('')}</div></section>` : '';

/* ---------- páginas ---------- */
const PAG = {
  inicio(m) {
    const at = ATALHOS.filter(passa).slice(0, m.a * 2);
    return `${barra(m)}<div class="corpo">${m.baixa && S.op === 'A' ? '' : '<h1 class="cond">Boa tarde</h1>'}
      <div class="atalhos" style="--a:${m.a}">${at.map(([n, , t, s, c]) => `<button class="at" data-ir="${ida(t)}">${capa(c, '', mostra() ? selo(s) : '')}<span>${n}</span></button>`).join('')}</div>
      ${PRAT.map(([t, its]) => prateleira(t, its.filter(passa), m)).join('')}</div>`;
  },
  buscar(m) {
    const ts = ['Principais', 'Músicas', 'Álbuns', 'Artistas', 'Playlists', 'Comandos'];
    if (!S.busca) return `${barra(m)}<div class="corpo"><h1 class="cond">Buscar</h1><section class="prat"><h2 class="cond">Buscas recentes</h2><div style="display:flex;flex-wrap:wrap;gap:8px">${['lume', 'vera solar', 'ondas curtas', 'foco'].map((b) => `<button class="ch" style="height:44px" data-busca="${b}">${ic('clock', 'p20')}${b}</button>`).join('')}</div></section>${prateleira('Abertos há pouco', PRAT[1][1].filter(passa), m)}</div>`;
    const fs = FAIXAS.filter((f) => S.filtro === 'tudo' || f[3].includes(S.filtro)).slice(0, 6);
    return `${barra(m)}<div class="corpo" style="gap:32px"><div class="abas" style="padding:0;margin-top:-16px">${ts.map((t, i) => `<button class="${i ? '' : 'on'}">${t}</button>`).join('')}</div>
      <div class="duas" style="--d:${m.util > 820 ? 'minmax(0,1fr) minmax(0,1.7fr)' : 'minmax(0,1fr)'}"><section class="prat"><h2 class="cond">Melhor resultado</h2><button class="li" data-ir="artista" style="border:1px solid var(--linha)!important;padding:20px!important;gap:24px!important">${capa('orbit', 'ar', '', 'mini').replace('class="capa ar mini"', 'class="capa ar mini" style="width:112px;height:112px"')}<span style="min-width:0;display:grid;gap:8px"><span class="cond" style="font-size:44px;line-height:44px">Lume</span><span class="ap" style="display:flex;gap:10px;align-items:center">Artista, no ${glifo('sp', 'ic p20')} e no ${glifo('yt', 'ic p20')}</span></span></button></section>
      <section class="prat"><h2 class="cond">Músicas</h2><div class="fxs">${fs.map((f, i) => linha(f, i, { ...m, util: m.util > 820 ? m.util * .62 : m.util }, { selos: true, curta: true })).join('')}</div></section></div>${prateleira('Álbuns', PRAT[1][1].filter(passa), m)}</div>`;
  },
  biblioteca(m) {
    const its = [...ATALHOS, ...PRAT[0][1], ...PRAT[1][1], ...PRAT[2][1]].filter(passa).map((i) => (i[1] ? i : [i[0], i[2] === 'ar' ? 'Artista' : i[2] === 'al' ? 'Álbum' : 'Playlist', i[2], i[3], i[4]]));
    const un = its.filter((i, k) => its.findIndex((j) => j[0] === i[0]) === k);
    return `${barra(m)}<div class="corpo" style="gap:28px"><h1 class="cond">Biblioteca</h1><div class="abas" style="padding:0;margin-top:-8px">${['Tudo', 'Playlists', 'Álbuns', 'Artistas', 'Curtidas', 'Pastas', 'Histórico'].map((t, i) => `<button class="${i ? '' : 'on'}">${t}</button>`).join('')}</div><div class="grade" style="--n:${m.n}">${un.map((i) => cartao(i)).join('')}</div></div>`;
  },
  artista(m) {
    const p = m.rolo.w < 520, alto = m.est ? 300 : m.baixa ? (p ? 232 : 220) : 380, ct = p && m.baixa ? 44 : p || m.baixa ? 56 : m.rolo.w < 900 ? 72 : 104;
    return `<header class="capaf" style="height:${alto}px;--ct:${ct}px"><div class="img" style="background-image:url(${foto()})"></div><div class="tx"><h1 class="cond">Lume</h1><div class="meta">${glifo('sp')}<span class="el">10,2 milhões de ouvintes por mês</span>${p ? '' : '<button class="lk">Ver no YouTube Music</button>'}</div><div class="acoes"><button class="pil cheia">${ic('play')}Tocar</button>${m.rolo.w < 520 ? '' : `<button class="pil">${ic('shuffle')}Aleatório</button>`}<button class="pil">${ic('heart')}Seguir</button><button class="btn-ic" aria-label="Mais ações">${pontos()}</button></div></div></header>
      <div class="abas">${['Visão geral', 'Álbuns', 'Singles e EPs', 'Coletâneas', 'Sobre'].map((t, i) => `<button class="${i ? '' : 'on'}">${t}</button>`).join('')}</div>
      <div class="corpo" style="padding-top:28px"><div class="duas" style="--d:${m.util > 820 ? 'minmax(0,1.7fr) minmax(0,1fr)' : 'minmax(0,1fr)'}"><section class="prat"><h2 class="cond">Populares</h2><div class="fxs">${FAIXAS.slice(0, 5).map((f, i) => linha(f, i, { ...m, util: m.util * .6 }, { num: true, curta: true, agora: true, selos: false })).join('')}</div></section><section class="prat" style="max-width:340px"><h2 class="cond">Último lançamento</h2>${cartao(['Noite Elétrica', 'Álbum, novembro de 2026', 'al', 'sp', 'glow'])}</section></div>${prateleira('Álbuns', PRAT[1][1], m)}${prateleira('Quem ouve também curte', PRAT[2][1].slice(1), m)}</div>`;
  },
  album(m) {
    const p = m.rolo.w < 480, lado = p ? 120 : m.baixa ? 140 : m.rolo.w < 760 ? 176 : 232;
    return `<header class="capaf" style="--ct:${p ? 28 : m.baixa || m.rolo.w < 760 ? 44 : 64}px;padding-top:${m.baixa ? 24 : 36}px;gap:${p ? 20 : 32}px"><div class="img borra" style="background-image:url(${cp('glow')})"></div><span class="capa" style="width:${lado}px;height:${lado}px;border-radius:${p ? 16 : 28}px;box-shadow:0 24px 60px -20px rgba(0,0,0,.8)"><i style="background-image:url(${cp('glow')})"></i></span><div class="tx"><h1 class="cond">Noite Elétrica</h1><div class="meta"><button class="lk forte" data-ir="artista">Lume</button><span>2026</span><span>9 músicas</span><span>31 min</span>${glifo('sp')}</div>${p ? '' : `<div class="acoes"><button class="pil cheia">${ic('play')}Tocar</button>${m.rolo.w < 760 ? `<button class="btn-ic" aria-label="Aleatório">${ic('shuffle')}</button><button class="btn-ic" aria-label="Salvar">${ic('heart')}</button>` : `<button class="pil">${ic('shuffle')}Aleatório</button><button class="pil">${ic('heart')}Salvar</button>`}<button class="btn-ic" aria-label="Mais ações">${pontos()}</button></div>`}</div></header>${p ? `<div class="acoes" style="padding:0 var(--lado) 12px"><button class="pil cheia">${ic('play')}Tocar</button><button class="btn-ic" aria-label="Aleatório">${ic('shuffle')}</button><button class="btn-ic" aria-label="Salvar">${ic('heart')}</button><button class="btn-ic" aria-label="Mais ações">${pontos()}</button></div>` : ''}
      <div class="corpo"><div class="fxs">${FAIXAS.map((f, i) => linha(f, i, m, { num: true, semCapa: true, curta: true, agora: true })).join('')}</div>${prateleira('Mais de Lume', PRAT[1][1], m)}</div>`;
  },
};

/* ---------- a janela ---------- */
function htmlNav(m) {
  const dest = ['inicio', 'buscar', 'biblioteca'], raiz = dest.includes(S.pag) ? S.pag : [...S.volta].reverse().find((p) => dest.includes(p)) || 'inicio';
  const it = (p, n, i) => `<button class="ni ${raiz === p ? 'on' : ''}" data-ir="${p}">${ic(raiz === p && IC[i + '-f'] ? i + '-f' : i)}<span>${n}</span></button>`;
  const tres = it('inicio', 'Início', 'home') + it('buscar', 'Buscar', 'search') + it('biblioteca', 'Biblioteca', 'lib');
  if (m.est) return `<nav class="ab nav" style="${cx(m.nav)}">${tres}<button class="ni">${ic('sliders')}<span>Ajustes</span></button></nav>`;
  if (m.med) return `<nav class="ab tri" style="${cx(m.nav)}">${tres}<span style="flex:1"></span><button class="ni">${ic('store')}<span>Loja</span></button><button class="ni">${ic('sliders')}<span>Ajustes</span></button></nav>`;
  const q = Math.max(0, Math.floor((m.nav.h - 436) / 64));
  return `<nav class="ab lat" style="${cx(m.nav)}">${tres}<hr><div class="tit">Sua biblioteca<button class="btn-ic" aria-label="Nova playlist">${ic('plus')}</button></div><div class="chips"><button class="ch">Playlists</button><button class="ch">Artistas</button></div><div class="rol">${LATERAL.filter(passa).slice(0, q).map(([n, s, t, sv, c]) => `<button class="li" data-ir="${ida(t)}">${capa(c, t === 'ar' ? 'ar' : '', mostra() ? selo(sv) : '', 'mini')}<span><span class="el forte">${n}</span><span class="el ap">${s}</span></span></button>`).join('')}</div><hr><button class="ni">${ic('store')}<span>Loja</span></button><button class="ni">${ic('sliders')}<span>Configurações</span></button></nav>`;
}
function htmlTopo(m) {
  const x = m.est ? 8 : m.med ? 92 : m.centro.x, w = S.w - x - (m.est ? 8 : 12);
  const txt = S.pag === 'buscar' && S.busca ? `<span style="color:var(--fg)">${S.busca}</span>` : '<span class="el">' + (w < 520 ? 'Buscar' : 'Buscar músicas, artistas e comandos') + '</span>';
  return `<header class="ab topo" style="left:${x}px;top:0;width:${w}px;height:${m.topo}px"><button class="btn-ic" data-volta aria-label="Voltar" style="${S.volta.length ? '' : 'opacity:.4'}">${ic('chev-l')}</button>${m.est ? '' : `<button class="btn-ic" aria-label="Avançar" style="opacity:.4">${ic('chev-r')}</button>`}<button class="busca ${S.pag === 'buscar' ? 'on' : ''}" data-ir="buscar" data-limpa>${ic('search')}${txt}${w > 620 ? '<span class="tec">Ctrl K</span>' : ''}</button>${w > 900 ? '<span class="ouv">Ouvindo o Player</span>' : ''}<span class="jan">${m.est ? '' : `<span>${ic('min', 'p20')}</span><span>${ic('max', 'p20')}</span>`}<span>${ic('x', 'p20')}</span></span></header>`;
}
function ondas(n) { let o = ''; for (let i = 0; i < n; i++) { const a = .25 + .75 * Math.abs(Math.sin(i * .31) * Math.cos(i * .071 + 1)); o += `<i class="${i < n * .22 ? 'f' : ''}" style="height:${Math.round(6 + a * 18)}px"></i>`; } return o; }
function htmlDock(m) {
  const faixa = `<div class="dk-f"><button class="capa" data-ir="album" aria-label="Abrir o álbum" style="${m.est ? '' : 'width:64px;height:64px'}"><i style="background-image:url(${cp('glow')})"></i>${selo('sp')}</button><span class="tx"><button class="el forte lk" data-ir="album">Corrente</button><button class="el ap lk" data-ir="artista" style="color:#c9cbd3">Lume</button></span>${m.est ? '' : `<button class="btn-ic on" style="background:none" aria-label="Curtir">${ic('heart-f')}</button>`}</div>`;
  if (m.est) return `<footer class="ab dock mini" style="${cx(m.dock)}">${faixa}<div class="dk-d"><button class="btn-ic" aria-label="Curtir">${ic('heart-f')}</button><button class="tocar" style="width:48px;height:48px" aria-label="Pausar">${ic('pause')}</button><button class="btn-ic" aria-label="Próxima">${ic('next')}</button></div></footer>`;
  const pouco = m.dock.w < 940;
  return `<footer class="ab dock" style="${cx(m.dock)};${pouco ? 'grid-template-columns:minmax(0,1fr) minmax(0,360px) auto;gap:16px' : ''}">${faixa}<div class="dk-c"><div class="bt"><button class="btn-ic" aria-label="Aleatório">${ic('shuffle')}</button><button class="btn-ic" aria-label="Anterior">${ic('prev')}</button><button class="tocar" aria-label="Pausar">${ic('pause', 'p28')}</button><button class="btn-ic" aria-label="Próxima">${ic('next')}</button><button class="btn-ic" aria-label="Repetir">${ic('repeat')}</button></div><div class="tempo"><span style="text-align:right">0:48</span><span class="onda">${ondas(pouco ? 46 : 72)}</span><span>3:46</span></div></div><div class="dk-d"><button class="btn-ic ${S.painel && S.aba === 'letra' ? 'on' : ''}" data-painel="letra" aria-label="Letra">${ic('lyrics')}</button><button class="btn-ic ${S.painel && S.aba === 'fila' ? 'on' : ''}" data-painel="fila" aria-label="Fila">${ic('queue')}</button><button class="btn-ic ${S.painel && S.aba === 'agora' ? 'on' : ''}" data-painel="agora" aria-label="Painel de módulos">${ic('panel')}</button><button class="btn-ic" aria-label="Volume">${ic('vol')}</button>${pouco ? '' : '<span class="volu"></span>'}<button class="btn-ic" aria-label="Mais">${pontos()}</button></div></footer>`;
}
function htmlPainel(m) {
  if (!m.painel) return '';
  const e = EN[S.en];
  const corpo = {
    agora: `<div class="mod"><div class="cab"><span>Ritmo</span><span>Ré menor</span></div><div style="display:flex;align-items:end;gap:12px"><span class="bpm">${e[7]}</span><span class="ap">BPM</span></div></div><div class="mod"><div class="cab"><span>Energia</span><span>${e[6]}</span></div><span class="cond" style="font-size:28px;line-height:32px;color:var(--en)">${e[0]}</span><div class="pontos-e">${Array.from({ length: 72 }, (_, i) => `<i class="${i % 24 < 24 * e[4] / 1.25 - Math.floor(i / 24) * 2 ? 'f' : ''}"></i>`).join('')}</div></div><div class="mod"><div class="cab"><span>Estrutura</span><span>Drop em 0:14</span></div><div class="trechos">${[8, 16, 22, 10, 26, 18].map((w, i) => `<i style="flex:${w};background:${['#6f7380', '#3d7bff', '#ff3348', '#6f7380', '#ff3348', '#3d7bff'][i]};opacity:${i === 1 ? 1 : .45}"></i>`).join('')}</div></div>`,
    fila: `<div style="padding:16px 12px">${FAIXAS.slice(0, 7).map((f, i) => `<button class="li">${capa(f[4], '', selo(f[3][0]), 'mini')}<span><span class="el forte" style="${i ? '' : 'color:var(--en)'}">${f[0]}</span><span class="el ap">${f[1]}</span></span></button>`).join('')}</div>`,
    letra: `<div style="padding:28px 24px;display:grid;gap:20px">${[70, 92, 54, 84, 64, 76, 48].map((w, i) => `<span style="height:22px;width:${w}%;border-radius:8px;background:${i === 2 ? '#f2f2f3' : 'rgba(255,255,255,.16)'}"></span>`).join('')}<span class="ap">A letra da faixa entra aqui.</span></div>`,
  }[S.aba];
  return `<aside class="ab painel" style="${cx(m.painel)}"><div class="abas">${[['agora', 'Agora'], ['fila', 'Fila'], ['letra', 'Letra']].map(([k, n]) => `<button class="${S.aba === k ? 'on' : ''}" data-aba="${k}">${n}</button>`).join('')}<button class="btn-ic" data-painel="${S.aba}" aria-label="Fechar o painel" style="margin-left:auto;border:0!important;padding:0!important">${ic('x')}</button></div>${corpo}</aside>`;
}
function htmlJanela(m) {
  const e = EN[S.en], B = S.op === 'B';
  const bpm = `<span class="bpm" style="color:var(--en)">${e[7]}</span><span class="sub" style="font-size:16px;line-height:24px">BPM</span>`;
  const bvis = `<button class="vidrinho" data-baixa>${ic(S.baixa ? 'shrink' : 'expand', 'p20')}${m.est ? '' : S.baixa ? 'Voltar à página' : 'Ver o visual'}</button>`;
  const nome = `<div style="min-width:0;flex:1"><h1 class="cond el" style="${m.est ? 'font-size:28px;line-height:32px' : ''}">Corrente</h1><div class="sub el">Lume</div></div>`;
  const legenda = `<div class="sobre" style="${m.est || m.baixa ? 'left:20px;right:20px;bottom:44px' : ''}">${nome}${m.est ? '' : bpm}${bvis}</div>`;
  const pag = PAG[S.pag](m);
  let meio;
  if (m.folhaSobe) {
    meio = `<section class="ab rolo" style="${cx(m.rolo)};--lado:${m.lado}px" data-rolo><div class="vao" style="height:${m.abre - m.tira}px"><div data-legenda>${m.abre > m.tira + 40 ? legenda : ''}</div></div><div class="folha" data-folha>${pag}</div></section>
      <div class="ab tira" style="left:${m.centro.x}px;top:${m.centro.y}px;width:${m.centro.w}px;height:${m.tira}px" data-tira><span style="flex:1"></span>${B && !m.est ? `<button class="vidrinho" data-larga>${ic('panel', 'p20')}Lado a lado</button>` : ''}${bvis}</div>`;
  } else if (B) {
    const estreito = m.vis.w < 520;
    meio = `<div class="ab" style="${cx(m.vis)};pointer-events:none"><div class="sobre" style="left:28px;right:28px;bottom:28px">${nome}${estreito && !S.baixa ? '' : bpm}</div><div style="position:absolute;right:16px;top:16px;pointer-events:auto">${estreito && !S.baixa ? bvis.replace('Ver o visual', '').replace('class="vidrinho"', 'class="vidrinho" style="width:40px;padding:0!important;justify-content:center" aria-label="Ver o visual"') : bvis}</div></div>` +
      (m.painel && S.w < 1680 ? '' : `<section class="ab rolo" style="${cx(m.rolo)};--lado:${m.lado}px" data-rolo><div class="folha">${pag}</div></section>${S.baixa ? '' : `<button class="ab btn-ic vidrinho" style="z-index:7;left:${m.rolo.x + m.rolo.w - 56}px;top:${m.rolo.y + 16}px;width:40px;padding:0!important;justify-content:center" data-larga aria-label="Página larga" title="Página larga">${ic('full', 'p20')}</button>`}`) +
      (m.divisor ? `<div class="ab divisor" style="${cx(m.divisor)}" data-divisor role="separator" aria-label="Arraste para mudar a largura do visual"><i></i></div>` : '');
  } else {
    meio = `<section class="ab rolo vidro" style="${cx(m.rolo)};--lado:${m.lado}px" data-rolo><div class="folha">${pag}</div></section>` + (S.baixa ? `<div class="ab" style="left:${m.centro.x}px;top:${m.centro.y}px;width:${m.centro.w}px;height:${m.centro.h}px">${legenda}</div>` : `<button class="ab vidrinho" style="right:28px;top:${m.topo + 16}px" data-baixa>${ic('expand', 'p20')}${m.est ? '' : 'Ver o visual'}</button>`);
  }
  return htmlNav(m) + meio + htmlTopo(m) + htmlPainel(m) + htmlDock(m);
}

/* ---------- como é hoje ---------- */
function htmlHoje(m) {
  const sv = S.filtro === 'tudo' ? 'nt' : S.filtro === 'sp' ? 'sp' : S.filtro === 'yt' ? 'yt' : 'nt';
  const its = PRAT[1][1].filter(passa), c156 = ([n, s, t, , c]) => `<div style="width:156px;flex:none"><span class="capa" style="width:156px"><i style="background-image:url(${cp(c)});border-radius:0"></i></span><div class="corta" style="font-weight:600;margin-top:8px">${n}</div><div class="corta" style="font-size:12px;line-height:16px;color:var(--fg-2)">${s}</div></div>`;
  const bol = [['tudo', ic('grid', 'p20')], ['pc', ic('lib', 'p20')], ['yt', '<span style="width:22px;height:22px;border-radius:99px;background:#f00"></span>'], ['sp', '<span style="width:22px;height:22px;border-radius:99px;background:#1db954"></span>']];
  const bolas = bol.map(([f, g]) => `<button data-filtro="${f}" aria-label="${f === 'tudo' ? 'Tudo' : NOME[f]}" style="width:36px;height:36px;border-radius:99px;display:grid;place-items:center;${S.filtro === f ? 'box-shadow:0 0 0 2px #fff' : ''}">${g}</button>`).join('');
  const nomeE = S.filtro === 'tudo' ? 'Tudo' : NOME[S.filtro];
  const pags = [['inicio', 'Para você', 'home'], ['buscar', 'Busca', 'search'], ['biblioteca', 'Biblioteca', 'lib']];
  const lat = m.est ? `<nav class="ab nav" style="${cx(m.nav)}">${pags.map(([p, n, i]) => `<button class="ni" data-ir="${p}">${ic(i)}<span>${n}</span></button>`).join('')}</nav>` : m.med ? `<nav class="ab" style="${cx(m.nav)};display:flex;flex-direction:column;align-items:center;gap:12px;padding:64px 0 16px">${bolas}<span style="height:12px"></span>${pags.map(([p, n, i]) => `<button class="btn-ic" data-ir="${p}" aria-label="${n}">${ic(i, 'p20')}</button>`).join('')}</nav>`
    : `<nav class="ab" style="${cx(m.nav)};border:1px solid var(--linha);border-radius:28px;padding:16px;display:flex;flex-direction:column;gap:6px"><div style="display:flex;gap:8px">${bolas}</div><div style="font-weight:600;font-size:18px;margin:10px 0 8px">${nomeE}</div>${pags.map(([p, n, i]) => `<button data-ir="${p}" style="display:flex;gap:12px;align-items:center;height:40px;padding:0 12px;border-radius:10px;font-weight:600;${S.pag === p ? 'background:var(--sel)' : 'color:var(--fg-2)'}">${ic(i, 'p20')}${n}</button>`).join('')}<div style="font-size:12px;font-weight:600;color:var(--fg-3);margin-top:14px">SUA BIBLIOTECA</div>${LATERAL.slice(0, 6).map(([n, s, , , c]) => `<div style="display:flex;gap:10px;align-items:center;height:52px"><i style="width:36px;height:36px;background:url(${cp(c)}) center/cover"></i><div style="min-width:0"><div class="corta" style="font-weight:600">${n}</div><div style="font-size:12px;line-height:16px;color:var(--fg-3)">${s.split(',')[0]}·Spotify</div></div></div>`).join('')}</nav>`;
  const folha = `<section class="ab folha-h ${sv}" style="${cx(m.centro)}"><div style="height:52px;display:flex;align-items:center;gap:28px;padding:0 24px;border-bottom:1px solid var(--linha);font-size:13px;font-weight:600">${ic('chev-l', 'p20')}${ic('chev-r', 'p20')}${m.est ? '' : '<span style="font-size:14px">Para você</span>'}${m.med || m.est ? '<span>Busca</span><span>Biblioteca</span>' : ''}</div><div style="padding:24px;display:grid;gap:20px;overflow:hidden"><div class="cond" style="font-size:28px;line-height:34px">Para você</div>${[0, 1].map((k) => `<div style="font-size:18px;font-weight:600">${['Lançamentos', 'Clipes novos'][k]}</div><div style="display:flex;gap:16px;overflow:hidden">${(k ? [...its].reverse() : its).concat(its).slice(0, 7).map(c156).join('')}</div>`).join('')}</div></section>`;
  const trilho = m.vis && !m.est ? `<div class="ab" style="left:${m.vis.x}px;top:${m.vis.y}px;width:${m.vis.w}px;height:${m.vis.h}px;border:1px solid var(--linha);border-radius:28px"><div style="position:absolute;left:16px;bottom:12px"><div style="font-weight:600;font-size:16px">Corrente</div><div style="font-size:12px;line-height:16px">Lume</div></div></div><div class="ab" style="left:${m.vis.x}px;top:${m.vis.y + m.vis.h + 12}px;width:${m.vis.w}px;height:${m.centro.h - m.vis.h - 12}px;border:1px solid var(--linha);border-radius:28px;padding:22px 24px;display:grid;align-content:start;gap:22px"><div style="display:flex;justify-content:space-between"><div><div style="color:var(--fg-2);font-weight:600">Ritmo</div><div class="bpm" style="font-size:40px">106</div></div><div style="width:120px"><div style="color:var(--fg-2);font-weight:600">Energ</div><div class="cond" style="font-size:22px;color:var(--en)">${EN[S.en][0]}</div></div></div><div style="color:var(--fg-2);font-weight:600">Estrutura da faixa</div><div class="trechos">${[8, 16, 22, 10, 26].map((w, i) => `<i style="flex:${w};background:${['#6f7380', '#3d7bff', '#ff3348', '#6f7380', '#ff3348'][i]};opacity:.5"></i>`).join('')}</div></div>` : '';
  const topo = `<header class="ab" style="left:12px;top:0;width:${S.w - 24}px;height:52px;display:flex;align-items:center;gap:12px"><span class="ouv" style="height:32px;overflow:hidden">Ouvindo o Player</span><span style="margin:0 auto;height:40px;width:${Math.min(440, S.w - 420)}px;border:1px solid var(--linha-2);border-radius:99px;display:flex;align-items:center;gap:12px;padding:0 14px;color:var(--fg-2);font-size:14px;overflow:hidden;white-space:nowrap">${ic('search', 'p20')}Buscar ou executar um comando</span><span class="jan" style="margin:0"><span>${ic('min', 'p20')}</span><span>${ic('x', 'p20')}</span></span></header>`;
  const dock = `<footer class="ab" style="${cx(m.dock)};border-radius:28px;background:var(--tom);padding:0 18px"><div style="position:absolute;left:18px;top:16px;display:flex;gap:12px;align-items:center"><i style="width:48px;height:48px;border-radius:10px;background:url(${cp('glow')}) center/cover"></i><div><div style="font-weight:600">Corrente</div><div style="color:var(--fg-2)">Lume</div></div></div><div style="position:absolute;left:50%;top:1px;transform:translateX(-50%);display:flex;gap:8px;align-items:center"><span class="btn-ic" style="width:40px;height:40px">${ic('prev', 'p20')}</span><span class="tocar" style="width:44px;height:44px">${ic('pause')}</span><span class="btn-ic" style="width:40px;height:40px">${ic('next', 'p20')}</span></div>${m.est ? '' : `<div style="position:absolute;left:50%;top:50px;transform:translateX(-50%);width:${Math.min(440, m.dock.w * .4)}px" class="tempo"><span>0:48</span><span class="onda" style="height:20px">${ondas(60)}</span><span>3:46</span></div><div style="position:absolute;right:16px;top:20px;display:flex">${['panel', 'queue', 'pip', 'full', 'vol'].map((i) => `<span class="btn-ic" style="width:40px;height:40px">${ic(i, 'p20')}</span>`).join('')}</div>`}</footer>`;
  return lat + folha + trilho + topo + dock;
}

/* ---------- desenhar ---------- */
let M = null, rolagem = {};
function desenhar(manter = true) {
  const mk = $('#mk'), ui = $('#ui'), mold = $('#mold');
  const antes = $('[data-rolo]', ui), topoAntes = manter && antes ? antes.scrollTop : 0;
  M = medir();
  mk.style.width = S.w + 'px'; mk.style.height = S.h + 'px';
  mk.style.setProperty('--en', EN[S.en][1]); mk.style.setProperty('--tom', EN[S.en][3]);
  document.documentElement.style.setProperty('--en', EN[S.en][1]); document.documentElement.style.setProperty('--tom', EN[S.en][3]);
  ui.className = M.hoje ? 'hj' : '';
  ui.innerHTML = M.hoje ? htmlHoje(M) : htmlJanela(M);
  const r = $('[data-rolo]', ui); if (r) r.scrollTop = topoAntes;
  const largura = mold.clientWidth, k = Math.min(1, largura / S.w);
  mk.style.transform = `scale(${k})`; mk.style.left = Math.max(0, (largura - S.w * k) / 2) + 'px';
  mold.style.height = Math.round(S.h * k) + 'px';
  const t = TEXTOS[S.op];
  $('#explica').innerHTML = `<div><h3>${t[0]}</h3><p>${t[1]}</p></div><div><h3>A favor</h3><ul>${t[2].map((x) => `<li>${x}</li>`).join('')}</ul></div><div><h3>Contra</h3><ul>${t[3].map((x) => `<li>${x}</li>`).join('')}</ul></div>`;
  document.querySelectorAll('[data-set]').forEach((b) => { const [k2, v] = b.dataset.set.split('='); const on = k2 === 'tam' ? v === `${S.w}x${S.h}` : String(S[k2]) === v; b.setAttribute('aria-pressed', on); });
  tam = null;
}
function ir(p) {
  if (p !== S.pag) { S.volta.push(S.pag); S.pag = p; }
  S.baixa = false; desenhar(false);
}
document.addEventListener('click', (ev) => {
  const a = ev.target.closest('[data-set],[data-ir],[data-filtro],[data-volta],[data-painel],[data-aba],[data-baixa],[data-larga],[data-busca]');
  if (!a) return;
  const d = a.dataset;
  if (d.set) { const [k, v] = d.set.split('='); if (k === 'tam') { [S.w, S.h] = v.split('x').map(Number); if (S.w <= 760) S.painel = false; } else S[k] = v; if (k === 'op') { S.baixa = false; S.larga = false; S.div = null; } desenhar(k === 'en'); return; }
  if (!a.closest('#mk')) return;
  if (d.busca) { S.busca = d.busca; return desenhar(false); }
  if ('limpa' in d) { if (S.pag === 'buscar') S.busca = S.busca ? '' : 'lume'; else S.busca = ''; }
  if (d.ir) return ir(d.ir);
  if (d.filtro) { S.filtro = d.filtro; return desenhar(false); }
  if ('volta' in d) { if (S.volta.length) { S.pag = S.volta.pop(); S.baixa = false; desenhar(false); } return; }
  if (d.painel) { S.painel = !(S.painel && S.aba === d.painel); S.aba = d.painel; return desenhar(); }
  if (d.aba) { S.aba = d.aba; return desenhar(); }
  if ('larga' in d) { S.larga = !S.larga; S.baixa = false; S.painel = false; return desenhar(false); }
  if ('baixa' in d) {
    S.baixa = !S.baixa;
    const v = $('.vao'), r = $('[data-rolo]');
    if (M.folhaSobe && v) { r.scrollTop = 0; M = medir(); v.style.height = `${M.abre - M.tira}px`; setTimeout(() => desenhar(false), matchMedia('(prefers-reduced-motion: reduce)').matches ? 0 : 430); } else desenhar();
  }
});
// a divisória da B: arrastar muda a proporção; dois cliques voltam ao padrão
document.addEventListener('pointerdown', (ev) => {
  const d = ev.target.closest('[data-divisor]'); if (!d) return;
  ev.preventDefault();
  const mk = $('#mk'); let pede = false;
  const mover = (e) => {
    const b = mk.getBoundingClientRect(), x = (e.clientX - b.left) / (b.width / S.w);
    S.div = Math.max(.28, Math.min(.68, (x - M.centro.x) / M.centro.w));
    if (!pede) { pede = true; requestAnimationFrame(() => { pede = false; desenhar(); }); }
  };
  const fim = () => { document.removeEventListener('pointermove', mover); document.removeEventListener('pointerup', fim); };
  document.addEventListener('pointermove', mover); document.addEventListener('pointerup', fim);
});
document.addEventListener('dblclick', (ev) => { if (ev.target.closest('[data-divisor]')) { S.div = null; desenhar(); } });
new ResizeObserver(() => desenhar()).observe($('#mold'));

/* ---------- o visual ---------- */
const cv = $('#vis'), gl = cv.getContext('webgl', { alpha: false, antialias: true, preserveDrawingBuffer: true });
let prog, tam = null, cor = [.24, .48, 1], cor2 = [.6, .75, 1], amp = .75, disp = .55;
function hex(h) { return [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255); }
if (gl) {
  const sh = (t, s) => { const o = gl.createShader(t); gl.shaderSource(o, s); gl.compileShader(o); if (!gl.getShaderParameter(o, gl.COMPILE_STATUS)) console.error(gl.getShaderInfoLog(o)); return o; };
  prog = gl.createProgram();
  gl.attachShader(prog, sh(gl.VERTEX_SHADER, `attribute vec4 a;uniform float t,amp,disp;uniform vec2 res;uniform vec3 ca,cb;varying vec4 vc;
  void main(){float u=a.x*6.2831853,v=a.y*2.-1.;
  vec3 p=vec3(v*1.15+.22*sin(u+v*2.2+t*.3),cos(u)*(.5+.38*sin(v*2.6+t*.21)),sin(u)*(.22+.32*cos(v*3.1-t*.23)));
  p.y+=.42*sin(v*2.3+t*.35+u*.5)*amp;p.x+=.2*sin(u*2.+t*.5)*amp;p.z+=.25*sin(u*3.+v*4.+t*.6)*amp;
  float s=step(.84,a.z);p+=s*disp*vec3(a.w*1.5-.5,(.5-a.x)*1.4,a.w-.5)*(.6+.4*sin(t*.8+a.w*20.));
  float c=cos(t*.16+.7),n=sin(t*.16+.7);p.xz=mat2(c,-n,n,c)*p.xz;c=cos(.42);n=sin(.42);p.xy=mat2(c,-n,n,c)*p.xy;
  float z=p.z+3.1;vec2 q=p.xy*2.1/z;float k=res.x>res.y*1.2?1.06:res.x/res.y*.95;q*=k;q.y+=.06;q.x*=res.y/res.x;gl_Position=vec4(q,0.,1.);
  gl_PointSize=max(1.,(1.1+1.5*a.w)*res.y/620.*3.1/z);float l=smoothstep(4.,2.3,z);vc=vec4(mix(ca,cb,a.w*.55+l*.35),(.26+.62*l)*(1.-s*.35));}`));
  gl.attachShader(prog, sh(gl.FRAGMENT_SHADER, `precision mediump float;varying vec4 vc;void main(){float d=length(gl_PointCoord-.5);float a=smoothstep(.5,.05,d)*vc.a;gl_FragColor=vec4(vc.rgb*a,1.);}`));
  gl.linkProgram(prog); gl.useProgram(prog);
  const N = 30000, b = new Float32Array(N * 4); let s = 7; const rnd = () => ((s = (s * 16807) % 2147483647) / 2147483647);
  for (let i = 0; i < b.length; i++) b[i] = rnd();
  gl.bindBuffer(gl.ARRAY_BUFFER, gl.createBuffer()); gl.bufferData(gl.ARRAY_BUFFER, b, gl.STATIC_DRAW);
  const l = gl.getAttribLocation(prog, 'a'); gl.enableVertexAttribArray(l); gl.vertexAttribPointer(l, 4, gl.FLOAT, false, 0, 0);
  gl.enable(gl.BLEND); gl.blendFunc(gl.ONE, gl.ONE); gl.clearColor(0, 0, 0, 1);
}
const parado = matchMedia('(prefers-reduced-motion: reduce)').matches;
const mistura = (a, b, k) => a.map((x, i) => x + (b[i] - x) * k);
function quadro(ms) {
  if (M) {
    // onde o visual está: no Palco vivo, do alto do miolo até a folha (que sobe e desce por cima dele)
    let r = M.vis, raio = M.hoje || S.op === 'B' ? 28 : 0, interno = r;
    if (M.folhaSobe) {
      const folha = $('[data-folha]'), mk = $('#mk').getBoundingClientRect(), k = mk.width / S.w;
      const y = folha ? (folha.getBoundingClientRect().top - mk.top) / k : M.centro.y + M.centro.h;
      const alto = Math.max(M.tira + 30, Math.min(M.centro.h, y - M.centro.y + 30));
      r = { ...M.centro, h: alto }; interno = { w: M.centro.w, h: Math.max(M.tira * 3, Math.min(M.centro.h, Math.round(M.centro.h * .62))) };
      if (S.baixa) interno = M.centro;
      raio = 28;
      const abre = Math.max(0, Math.min(1, (y - M.centro.y - M.tira - 20) / 90)), t = $('[data-tira]'), l = $('[data-legenda]');
      if (t) { t.style.opacity = 1 - abre; t.style.visibility = abre > .98 ? 'hidden' : 'visible'; }
      if (l) l.style.opacity = abre;
    }
    cv.style.display = r ? 'block' : 'none';
    if (r) {
      Object.assign(cv.style, { left: r.x + 'px', top: r.y + 'px', width: r.w + 'px', height: r.h + 'px', borderRadius: raio + 'px' });
      const dpr = Math.min(1.5, devicePixelRatio || 1), W = Math.round(interno.w * dpr), H = Math.round(interno.h * dpr);
      if (tam !== `${W}x${H}`) { cv.width = W; cv.height = H; tam = `${W}x${H}`; }
      const e = EN[S.en]; cor = mistura(cor, hex(e[1]), .06); cor2 = mistura(cor2, hex(e[2]), .06); amp += (e[4] - amp) * .04; disp += (e[5] - disp) * .04;
      if (gl) {
        const U = (n) => gl.getUniformLocation(prog, n), t = parado ? 8 : ms / 1000 * (.6 + amp * .5) + 8;
        gl.viewport(0, 0, W, H); gl.clear(gl.COLOR_BUFFER_BIT);
        gl.uniform1f(U('t'), t); gl.uniform1f(U('amp'), amp); gl.uniform1f(U('disp'), disp); gl.uniform2f(U('res'), W, H); gl.uniform3fv(U('ca'), cor); gl.uniform3fv(U('cb'), cor2);
        gl.drawArrays(gl.POINTS, 0, 30000);
      }
    }
  }
  requestAnimationFrame(quadro);
}

/* ---------- uma foto de palco inventada (no app vem a do serviço) ---------- */
let fotoGuardada = '';
function foto() {
  if (fotoGuardada) return fotoGuardada;
  const w = 1600, h = 720, c = document.createElement('canvas'); c.width = w; c.height = h;
  const g = c.getContext('2d'); let s = 11; const r = () => ((s = (s * 16807) % 2147483647) / 2147483647);
  let b = g.createLinearGradient(0, 0, 0, h); b.addColorStop(0, '#0a1622'); b.addColorStop(.55, '#12324a'); b.addColorStop(1, '#05080c'); g.fillStyle = b; g.fillRect(0, 0, w, h);
  g.globalCompositeOperation = 'lighter';
  for (let i = 0; i < 10; i++) {
    const x = w * (.08 + i * .1 + r() * .04), a = (r() - .5) * 1.1, L = h * 1.35, wd = 50 + r() * 120, x2 = x + Math.sin(a) * L, y2 = Math.cos(a) * L, m = 190 + r() * 70;
    b = g.createLinearGradient(x, 0, x2, y2); b.addColorStop(0, `hsla(${m},90%,78%,.55)`); b.addColorStop(1, `hsla(${m},90%,60%,0)`); g.fillStyle = b;
    g.beginPath(); g.moveTo(x - 5, -10); g.lineTo(x + 5, -10); g.lineTo(x2 + wd, y2); g.lineTo(x2 - wd, y2); g.fill();
  }
  for (let i = 0; i < 26; i++) { const x = r() * w, y = h * (.35 + r() * .6), R = 90 + r() * 260; b = g.createRadialGradient(x, y, 0, x, y, R); b.addColorStop(0, `hsla(${195 + r() * 40},70%,60%,.11)`); b.addColorStop(1, 'transparent'); g.fillStyle = b; g.fillRect(x - R, y - R, 2 * R, 2 * R); }
  g.globalCompositeOperation = 'source-over';
  const px = w * .66, py = h * .2; g.fillStyle = '#04070b'; g.shadowColor = 'rgba(150,215,255,.85)'; g.shadowBlur = 26;
  g.beginPath(); g.ellipse(px, py + 62, 40, 50, -.12, 0, 7); g.fill();
  g.beginPath(); g.moveTo(px - 22, py + 104); g.bezierCurveTo(px - 110, py + 128, px - 120, py + 170, px - 128, py + 300); g.lineTo(px - 92, py + 310); g.lineTo(px - 74, py + 220); g.lineTo(px - 70, h); g.lineTo(px + 84, h); g.lineTo(px + 72, py + 220); g.bezierCurveTo(px + 120, py + 190, px + 168, py + 120, px + 196, py + 30); g.lineTo(px + 172, py + 16); g.bezierCurveTo(px + 140, py + 70, px + 110, py + 120, px + 24, py + 106); g.fill();
  g.shadowBlur = 0; g.fillStyle = '#020304';
  for (let i = 0; i < 64; i++) { const x = (i / 63) * w + (r() - .5) * 30, y = h - 20 - r() * 70, R = 18 + r() * 14; g.beginPath(); g.arc(x, y, R, 0, 7); g.fill(); g.fillRect(x - R * 1.5, y + R * .7, R * 3, h); if (r() > .72) { g.save(); g.translate(x + R, y); g.rotate(.25 - r() * .5); g.fillRect(-5, -90 - r() * 50, 11, 140); g.restore(); } }
  fotoGuardada = c.toDataURL('image/jpeg', .86);
  return fotoGuardada;
}

/* ---------- as peças lado a lado ---------- */
function pecas() {
  const m = { util: 700 };
  const velho = (n, s, c) => `<div style="width:156px;flex:none;font-size:14px;line-height:20px"><i style="display:block;width:156px;height:156px;background:url(${cp(c)}) center/cover"></i><div class="corta" style="font-weight:600;margin-top:8px;overflow:hidden;white-space:nowrap">${n}</div><div style="font-size:12px;line-height:16px;color:var(--fg-2);white-space:nowrap;overflow:hidden">${s}</div></div>`;
  const novo = (i, x = '') => `<div class="mk" style="position:relative;width:188px;flex:none;background:none;overflow:visible">${cartao(i, x)}</div>`;
  $('#p-cartao').innerHTML = `<div class="caixa"><small>Hoje</small><div class="tela-x"><div style="display:flex;gap:16px">${velho('Quando a Cidade Apaga as Luzes', 'Ondas Curtas', 'orbit')}${velho('Treino pesado', 'Playlist·Spotify', 'bars')}</div></div></div><div class="caixa novo"><small>Proposta (o terceiro está com o mouse em cima)</small><div class="tela-x"><div style="display:flex;gap:20px;padding-top:12px">${novo(['Quando a Cidade Apaga as Luzes', 'Ondas Curtas', 'al', 'yt', 'orbit'])}${novo(['Treino pesado', 'Playlist', 'pl', 'sp', 'bars'])}${novo(['Noite Elétrica', 'Lume', 'al', 'sp', 'glow'], 'mostra')}${novo(['Lume', 'Artista', 'ar', 'sp', 'orbit'])}</div></div></div>`;
  const guarda = { ...S }; Object.assign(S, { painel: false, filtro: 'tudo' });
  const md = { est: false, dock: { x: 0, y: 0, w: 1120, h: 96 } };
  $('#p-barra').innerHTML = `<div class="caixa"><small>Hoje: 80 px de altura</small><div class="tela-x"><div class="mk" style="position:relative;width:1120px;height:80px;background:none">${htmlHoje({ est: false, med: false, nav: { x: -900, y: -900, w: 1, h: 1 }, centro: { x: -900, y: -900, w: 1, h: 1 }, vis: null, dock: { x: 0, y: 0, w: 1120, h: 80 } }).match(/<footer[\s\S]*<\/footer>/)[0]}</div></div></div><div class="caixa novo"><small>Proposta: 96 px, capa de 64, selo, coração, tudo com 24 px e clicável</small><div class="tela-x"><div class="mk" style="position:relative;width:1120px;height:96px;background:none">${htmlDock(md)}</div></div></div>`;
  $('#p-linha').innerHTML = `<div class="caixa"><small>Hoje: 56 px, ações só com o mouse em cima</small><div style="display:grid;font-size:14px;line-height:20px">${FAIXAS.slice(0, 3).map((f) => `<div style="display:grid;grid-template-columns:40px minmax(0,1fr) auto;gap:12px;align-items:center;height:56px"><i style="width:40px;height:40px;background:url(${cp(f[4])}) center/cover"></i><div style="min-width:0"><div style="font-weight:600">${f[0]}</div><div style="font-size:12px;line-height:16px;color:var(--fg-2)">${f[1]}</div></div><span style="font-size:12px;color:var(--fg-3)">${f[5]}</span></div>`).join('')}</div></div><div class="caixa novo"><small>Proposta: 64 px; a mesma música em dois serviços é uma linha com dois selos</small><div class="mk" style="position:relative;background:none;overflow:visible">${FAIXAS.slice(0, 3).map((f, i) => linha(f, i, m, { selos: true, curta: true })).join('')}</div></div>`;
  Object.assign(S, guarda);
  const uso = [[11, 1], [12, 65], [13, 24], [14, 114], [15, 1], [16, 8], [18, 6], [20, 1], [22, 1]];
  $('#p-texto').innerHTML = `<div class="caixa"><small>Hoje: 9 tamanhos, 5 fora da escala; 92% do texto tem 14 px ou menos</small><div style="display:grid;gap:8px">${uso.map(([t, q]) => `<div style="display:grid;grid-template-columns:44px minmax(0,1fr) 44px;gap:12px;align-items:center"><span style="font:400 12px/16px var(--mono);color:var(--fg-2)">${t} px</span><span style="height:10px;border-radius:5px;width:${Math.max(1.5, q / 114 * 100)}%;background:${[12, 14, 16, 20].includes(t) ? 'var(--fg-2)' : 'var(--intensa)'}"></span><span style="font:400 12px/16px var(--mono);color:var(--fg-2);text-align:right">${q}×</span></div>`).join('')}</div><small style="font-weight:450;color:var(--fg-2);font-size:14px;line-height:20px">Em vermelho, os que não existem no nosso design system.</small></div><div class="caixa novo"><small>Proposta: 7 papéis, todos da escala que já era nossa</small><div style="display:grid;gap:14px">${[['Cartaz', '88/80', 'cond', '64px/58px'], ['Página', '44/44', 'cond', '44px/44px'], ['Seção', '28/32', 'cond', '28px/32px'], ['Destaque', '20/28', 'forte', '20px/28px'], ['Corpo', '16/24', 'forte', '16px/24px'], ['Apoio', '14/20', '', '14px/20px'], ['Micro', '12/16, só teclas e tempos', '', '12px/16px']].map(([n, t, c, f]) => `<div style="display:flex;justify-content:space-between;gap:16px;align-items:baseline;min-width:0"><span class="${c}" style="font-size:${f.split('/')[0]};line-height:${f.split('/')[1]}">${n}</span><span style="font:400 12px/16px var(--mono);color:var(--fg-2);white-space:nowrap">${t}</span></div>`).join('')}</div></div>`;
  $('#p-selos').innerHTML = `<div class="caixa novo"><small>A família: disco escuro, marca em linha. Com o mouse em cima, o selo cresce e vira o botão de tocar na cor do serviço</small><div style="display:flex;flex-wrap:wrap;gap:36px;align-items:center">${['yt', 'sp', 'pc'].map((s) => `<span style="display:flex;align-items:center;gap:20px"><span class="mk" style="position:relative;display:block;width:104px;height:104px;background:none;overflow:visible">${capa({ yt: 'silk', sp: 'glow', pc: 'scan' }[s], '', selo(s))}</span><span class="mk cart mostra" style="position:relative;display:block!important;width:104px;height:104px;background:none;overflow:visible">${capa({ yt: 'silk', sp: 'glow', pc: 'scan' }[s], '', selo(s))}</span><span>${NOME[s]}</span></span>`).join('')}</div></div>`;
}

window.addEventListener('load', () => {
  // só para conferir um estado na prévia local: ?op=B&w=958&h=560&pag=artista&so=1
  const q = new URLSearchParams(location.search);
  for (const k of ['op', 'pag', 'filtro', 'en', 'aba', 'busca']) if (q.has(k)) S[k] = q.get(k);
  for (const k of ['w', 'h']) if (q.has(k)) S[k] = Number(q.get(k));
  for (const k of ['painel', 'baixa', 'larga']) if (q.has(k)) S[k] = q.get(k) === '1';
  if (q.has('so')) { document.querySelectorAll('.pg > :not([aria-label="Protótipo"]), .ctl, .explica').forEach((e) => { e.style.display = 'none'; }); document.querySelector('.pg').style.cssText = 'padding:0;max-width:none;gap:0'; document.querySelector('.mold').style.borderRadius = 0; }
  if (innerWidth < 720) { S.w = 410; S.h = 800; }
  desenhar(); pecas(); requestAnimationFrame(quadro);
  if (q.has('rol')) { const r = $('[data-rolo]'); if (r) r.scrollTop = Number(q.get('rol')); }
});
