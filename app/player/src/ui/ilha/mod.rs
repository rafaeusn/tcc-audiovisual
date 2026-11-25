//! A Ilha (a vista): um entalhe preto preso a uma borda da tela (ou solto nela), numa janela própria.
//!
//! A forma é um traço só: o corpo com os cantos de longe arredondados e, presa, dois "ombros" côncavos que o ligam à
//! borda da tela; solta, os quatro cantos redondos. Ela é desenhada a cada quadro com as medidas que as molas trazem:
//! cresce com um leve quique e encolhe sem quique. As molas guardam a forma na tela (a âncora: presa, o meio do lado
//! colado na borda; solta, o centro), então ela anda, muda de borda e de tamanho sem saltos. A janela é quadrada,
//! do tamanho da maior forma, e segue a forma sem sair do monitor; só a forma aparece e recebe cliques (a região da
//! janela acompanha a forma).
//!
//! Arrastar é sempre livre. Travada, a Ilha só desliza ao longo da borda em que está (puxada para dentro da tela, ela
//! estica um pouco e volta). Destravada, vai para qualquer lugar: perto de uma borda, gruda nela (o ímã); longe, fica
//! solta onde for largada. Travar uma Ilha solta a devolve à última borda. Com mais de um monitor, ela vai junto com o
//! ponteiro para o outro (travada, na mesma borda) e lembra em qual ficou.
//!
//! Aberta, a Ilha tem páginas (Tocando, Letra, Visual, Ritmo, Ajustes): as abas no alto, arrastar o conteúdo para o
//! lado ou a roda do mouse passam de uma para outra. Dos lados, ela vira uma barra lateral, com as páginas na vertical.
//! Na página Visual, o visual de verdade aparece num recorte da forma, desenhado por uma segunda camada do `janela`
//! (preparada em outra thread quando a Ilha abre). Na página Letra, a linha cantada enche como num karaokê.
//!
//! A lógica das formas e dos lugares fica em [`crate::ilha`]; a parte do Windows em `integracao::ilha`. A vista
//! mantém o estado do Player atualizado, para tudo andar com a janela principal minimizada ou na bandeja, e manda os
//! comandos pela janela principal.

use crate::anim::{Animacoes, elastico};
use crate::app::{Player, hwnd_da_janela};
use crate::ilha::{self, Alerta, Caixa, Controle, Entrada, Forma, Lugar, Medidas, Modo, Pagina};
use crate::letrista::Letra;
use crate::tema;
use crate::ui::{FONTE, com_alfa, icone, letra_enchendo, linha_da_letra, numero, tempo, texto};
use analise::{BINS, Retrato, Rotulo};
use gpui::{
    AnyElement, App, Bounds, ClickEvent, Context, CursorStyle, Div, Entity, FontWeight, Hsla, MouseButton,
    MouseDownEvent, Path, PathBuilder, Pixels, Point, RenderImage, ScrollWheelEvent, SharedString, Stateful, Window,
    WindowBackgroundAppearance, WindowBounds, WindowHandle, WindowKind, WindowOptions, canvas, div, hsla, img,
    linear_color_stop, linear_gradient, point, prelude::*, px, size,
};
use nucleo::{BordaIlha, Comando, Modelo, Pagina as PaginaDoApp, TamanhoIlha};
use std::sync::Arc;
use std::time::{Duration, Instant};

fn preto() -> Hsla {
    hsla(0.0, 0.0, 0.0, 1.0)
}

fn branco(a: f32) -> Hsla {
    hsla(0.0, 0.0, 1.0, a)
}

fn cinza() -> Hsla {
    hsla(0.0, 0.0, 0.64, 1.0)
}

/// Texto pequeno (legendas, tempos): acompanha o tamanho da Ilha, mas nunca abaixo de 11 px.
fn pequeno(s: f32) -> f32 {
    (11.0 * s).max(11.0)
}

#[cfg(windows)]
mod camada;
mod conteudo;
mod forma;
mod ponteiro;

#[cfg(windows)]
use camada::CAMADA_SEM_USO;
use forma::{Recorte, forma};
use ponteiro::Ponteiro;

/// O que a vista precisa do Player neste quadro (copiado, para não segurar o Player enquanto desenha).
struct Dados {
    titulo: String,
    artista: String,
    origem: String,
    capa: Option<Arc<RenderImage>>,
    tocando: bool,
    barras: [f32; 4],
    tom: Hsla,
    volume: f32,
    drop_em_s: Option<f32>,
    /// A letra sincronizada e o instante dela, ou a frase que vai no lugar.
    letra: Result<(Arc<Letra>, f64), &'static str>,
    /// A tradução e as letras latinas embaixo das linhas perto da de agora (vazio quando desligadas), e se a
    /// pessoa ligou alguma delas (a letra abre espaço para elas).
    extras: Vec<(usize, String)>,
    com_extras: bool,
    progresso: Option<(f64, f64)>,
    bpm: Option<u32>,
    energia: f32,
    rotulo: Rotulo,
    cor_do_rotulo: Hsla,
    mapa: Vec<f32>,
    tom_musical: Option<SharedString>,
    modelo: Modelo,
    cores_reativas: bool,
    modo_seguro: bool,
    tema: &'static str,
    tamanho: TamanhoIlha,
    travada: bool,
    ao_pairar: bool,
    /// Em que monitor a Ilha está e quantos há (com mais de um).
    monitor: Option<(usize, usize)>,
    retrato: Retrato,
    paleta_fixa: Option<([f32; 3], [f32; 3])>,
    cinza_no_visual: bool,
}

impl Dados {
    fn do_player(p: &Player) -> Self {
        let faixa = p.midia.faixa.clone().unwrap_or_default();
        // Quatro faixas do espectro (já em bandas logarítmicas), dos graves aos agudos.
        let mut barras = [0.0f32; 4];
        let fim = BINS * 9 / 10;
        for (i, b) in barras.iter_mut().enumerate() {
            let (de, ate) = (fim * i / 4, fim * (i + 1) / 4);
            *b = p.retrato.espectro[de..ate].iter().copied().max().unwrap_or(0) as f32 / 255.0;
        }
        let cores = p.cores();
        let rotulo = p.retrato.rotulo;
        let paleta = p.ajustes.paleta_atual();
        let letra = p.letra_sincronizada();
        Self {
            // A música local diz de onde vem: o computador ou o provedor da faixa.
            origem: match p.local.atual().filter(|_| p.origem_local) {
                Some(m) => m.remota.as_ref().map_or_else(|| "Neste computador".to_string(), |r| r.fonte.clone()),
                None => faixa.app.clone(),
            },
            titulo: faixa.titulo,
            artista: faixa.artista,
            capa: p.capa.clone(),
            tocando: p.midia.tocando,
            barras,
            tom: tema::cor_de(p.tom),
            volume: if p.mudo { 0.0 } else { p.volume },
            drop_em_s: p.drop_em_s(),
            extras: match &letra {
                Ok((l, t)) => p.extras_perto(&l.linhas, *t),
                Err(_) => Vec::new(),
            },
            com_extras: p.letra_tem_extras(),
            letra,
            progresso: p
                .midia
                .posicao
                .filter(|po| po.duracao_s > 1.0)
                .map(|po| (po.agora(p.midia.tocando).min(po.duracao_s), po.duracao_s)),
            bpm: (p.retrato.conf_batida > 0.3 && p.retrato.bpm > 40.0).then(|| p.retrato.bpm.round() as u32),
            energia: p.retrato.energia,
            rotulo,
            cor_do_rotulo: match rotulo {
                Rotulo::Calma => cores.calma,
                Rotulo::Ativa => cores.ativa,
                Rotulo::Intensa => cores.intensa,
            },
            mapa: p.mapa.clone(),
            tom_musical: p.tom_da_musica(),
            modelo: p.ajustes.modelo,
            cores_reativas: p.ajustes.cores_reativas,
            modo_seguro: p.ajustes.modo_seguro,
            tema: p.ajustes.tema.nome(),
            tamanho: p.ajustes.ilha_tamanho,
            travada: p.ajustes.ilha_travada,
            ao_pairar: p.ajustes.ilha_ao_pairar,
            monitor: None,
            retrato: p.retrato,
            paleta_fixa: (!p.ajustes.cores_reativas)
                .then(|| (paleta.a.map(|v| v as f32 / 255.0), paleta.b.map(|v| v as f32 / 255.0))),
            cinza_no_visual: p.ajustes.tema == nucleo::Tema::Mono,
        }
    }
}

/// Um arrasto que começou na Ilha: vira mudança de lugar (travada, só ao longo da borda) ou troca de página (no
/// conteúdo da Ilha aberta).
#[derive(Clone, Copy, Debug)]
struct Arrasto {
    /// Onde o ponteiro estava ao apertar, em pixels lógicos do monitor.
    inicio: (f32, f32),
    /// Onde o ponteiro pegou a Ilha, em relação ao centro dela (ela não pula para baixo do ponteiro).
    pega: (f32, f32),
    modo: ModoDoArrasto,
    /// Último ponto e quando, para a velocidade na hora de soltar.
    ultimo: (f32, Instant),
    velocidade: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ModoDoArrasto {
    /// Ainda não andou o bastante para saber.
    Indefinido,
    /// A Ilha vai junto: destravada, para qualquer borda ou solta; travada, ao longo da borda dela.
    Mover,
    Paginas,
}

/// O relógio leve da Ilha: lê o ponteiro e o estado mesmo quando ela não está desenhando.
const TIQUE: Duration = Duration::from_millis(16);

/// Quanto a Ilha precisa desenhar. Desenhar a cada quadro do monitor parada gastava a placa de vídeo à toa.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ritmo {
    /// Algo anima (a forma, a página, o arrasto, o visual): quadro a quadro.
    Cheio,
    /// Tocando, com as barras do som ou a letra à mostra: uns 30 quadros por segundo bastam.
    Lento,
    /// Nada se move: só para o relógio da música e o estado.
    Parado,
}

impl Ritmo {
    fn intervalo(self) -> Duration {
        match self {
            Self::Cheio => Duration::ZERO,
            Self::Lento => Duration::from_millis(33),
            Self::Parado => Duration::from_millis(250),
        }
    }
}

/// Quanto o ponteiro precisa andar para um clique virar arrasto.
const LIMIAR_DO_ARRASTO: f32 = 6.0;
/// Altura das abas no alto da Ilha aberta, na escala 1: arrastar dali muda a Ilha de lugar; abaixo, passa as páginas.
const CABECA: f32 = 54.0;
/// As molas que, paradas, querem dizer que a forma assentou.
const MOLAS_DA_FORMA: [&str; 9] =
    ["ilha-w", "ilha-h", "ilha-r", "ilha-rn", "ilha-e", "ilha-ax", "ilha-ay", "ilha-puxao-x", "ilha-puxao-y"];

/// O monitor da Ilha em pixels lógicos: o retângulo dele (a partir de 0, 0), a área de trabalho dentro dele e a escala.
#[derive(Clone, Copy, Debug)]
struct Telas {
    monitor: Caixa,
    area: Caixa,
    escala: f32,
    /// O canto do monitor e a área de trabalho em pixels da tela.
    canto: (i32, i32),
    area_na_tela: (i32, i32, i32, i32),
}

impl Telas {
    fn do_monitor(m: Option<integracao::ilha::Monitor>) -> Self {
        let m = m.unwrap_or(integracao::ilha::Monitor {
            esquerda: 0,
            topo: 0,
            direita: 1920,
            baixo: 1080,
            area: (0, 0, 1920, 1040),
            escala: 1.0,
            nome: String::new(),
            principal: true,
        });
        let e = if m.escala.is_finite() { m.escala.max(0.5) } else { 1.0 };
        let l = |v: i32| v as f32 / e;
        let (w, h) = (l(m.direita - m.esquerda).max(1.0), l(m.baixo - m.topo).max(1.0));
        let (a0, a1, a2, a3) = m.area;
        let area = Caixa::new(l(a0 - m.esquerda), l(a1 - m.topo), l(a2 - a0).max(1.0), l(a3 - a1).max(1.0));
        Self { monitor: Caixa::new(0.0, 0.0, w, h), area, escala: e, canto: (m.esquerda, m.topo), area_na_tela: m.area }
    }

    /// Um ponto da tela (pixels) em pixels lógicos do monitor.
    fn logico(&self, p: (i32, i32)) -> (f32, f32) {
        ((p.0 - self.canto.0) as f32 / self.escala, (p.1 - self.canto.1) as f32 / self.escala)
    }

    /// Um ponto em pixels lógicos do monitor `outro` visto deste (a forma passando de um monitor para outro).
    fn de_outro(&self, outro: &Telas, p: (f32, f32)) -> (f32, f32) {
        let tela = (outro.canto.0 as f32 + p.0 * outro.escala, outro.canto.1 as f32 + p.1 * outro.escala);
        ((tela.0 - self.canto.0 as f32) / self.escala, (tela.1 - self.canto.1 as f32) / self.escala)
    }
}

pub struct VistaIlha {
    principal: WindowHandle<Player>,
    player: Entity<Player>,
    hwnd: Option<isize>,
    hwnd_principal: Option<isize>,
    controle: Controle,
    anim: Animacoes,
    anterior: Instant,
    /// Clicou na Ilha desde o último quadro.
    clicou: bool,
    forma: Forma,
    alerta: Option<Alerta>,
    pagina: Pagina,
    /// A janela está à mostra (a Ilha some encolhendo: `saindo` até a forma sumir).
    visivel: bool,
    saindo: bool,
    /// As medidas pedidas por último, para saber se a forma cresce (com quique) ou encolhe (sem).
    alvo_visto: Medidas,
    crescendo: bool,
    /// O que está desenhado (forma, aviso, tamanho da Ilha e se está em pé), para o conteúdo novo entrar do zero.
    desenho_visto: (Forma, Option<Alerta>, TamanhoIlha, bool),
    /// Onde a Ilha fica pelos ajustes (presa a uma borda ou solta) e se ela está travada.
    lugar: Lugar,
    travada: bool,
    /// Onde ela está durante um arrasto (destravada), antes de soltar.
    lugar_no_arrasto: Option<Lugar>,
    /// O modo da forma (presa a qual borda, ou solta): diz como as molas da âncora seguram a forma.
    modo: Modo,
    /// De que lado os ombros estão desenhados (só muda quando eles somem).
    lado: Option<BordaIlha>,
    /// O corpo da forma neste quadro, em pixels lógicos do monitor.
    corpo_na_tela: Caixa,
    /// A forma com os ombros neste quadro, na janela (o ponteiro em cima e a região).
    caixa: Caixa,
    regiao_vista: ilha::Regiao,
    /// Onde a janela está (pixels da tela e tamanho) e o canto dela em pixels lógicos do monitor.
    janela_posta: Option<(i32, i32, i32, i32)>,
    origem: (f32, f32),
    /// O monitor, a escala máxima da Ilha nele e o tamanho da janela (quadrada) nessa escala.
    telas: Telas,
    escala_maxima: f32,
    tamanho_janela: (f32, f32),
    /// Travada e puxada para dentro da tela: quanto ela estica (pixels lógicos), voltando ao soltar.
    puxao: (f32, f32),
    /// Depois de largar a Ilha com o ponteiro em cima, ela só abre de novo depois que o ponteiro sair.
    pairar_bloqueado: bool,
    ponteiro: Ponteiro,
    arrasto: Option<Arrasto>,
    arrastou_em: Option<Instant>,
    /// Quanto a página foi arrastada (pixels), enquanto arrasta.
    deslize: f32,
    rolagem: f32,
    rolou_em: Instant,
    faixa_vista: Option<String>,
    /// De quem era a barra (a música ou o sistema) e onde estava.
    volume_visto: Option<(bool, f32)>,
    parada_desde: Option<Instant>,
    /// Quando o sistema foi consultado (tela cheia, Player na frente, movimento reduzido, monitor).
    sistema_em: Instant,
    tela_cheia: bool,
    principal_na_frente: bool,
    reduzido_no_sistema: bool,
    monitor: Option<integracao::ilha::Monitor>,
    /// Os nomes dos monitores ligados (em ordem), para a página Ajustes dizer "Monitor 1 de 2".
    monitores: Vec<String>,
    /// A última faixa vista: na troca de origem a faixa some por um instante, e a Ilha segue mostrando esta.
    ultima_faixa: (String, String),
    /// Ajuda de teste (`TCCPLAYER_ILHA_ABERTA=1`): a Ilha fica aberta como se o mouse estivesse em cima, para
    /// conferir a forma aberta sem mexer no mouse de ninguém. `TCCPLAYER_ILHA_PAGINA=<n>` abre numa página.
    aberta_para_teste: bool,
    /// Ajuda de teste (`TCCPLAYER_ILHA_ESCONDIDA=1`): a janela fica acima da borda da tela, fora da vista, e não
    /// some com tela cheia de outro app. Dá para fotografá-la (`PrintWindow`) sem cobrir a tela de quem usa o PC.
    escondida_para_teste: bool,
    /// Ajuda de teste (`TCCPLAYER_ILHA_FUNDO=1`): a janela ganha fundo magenta, para a forma preta aparecer nas fotos
    /// (`PrintWindow` pinta o transparente de preto). Só com a Ilha escondida, fora da vista.
    fundo_de_teste: bool,
    /// Quanto a Ilha precisa desenhar agora (ver [`Ritmo`]) e quando desenhou por último.
    ritmo: Ritmo,
    desenhado_em: Instant,
    /// Ajuda de teste (`TCCPLAYER_ILHA_DIAGNOSTICO=1`): registra, quadro a quadro durante as transições, as medidas
    /// da forma, onde o conteúdo foi posto, as trocas de região, os movimentos da janela e os lugares do arrasto.
    diagnostico: Option<Instant>,
    /// A segunda camada do visual: preparada em outra thread quando a Ilha abre e mantida enquanto ela é usada
    /// (sai depois de [`CAMADA_SEM_USO`] sem a Ilha aberta).
    #[cfg(windows)]
    camada: Option<janela::Camada>,
    #[cfg(windows)]
    camada_pendente: Option<janela::CamadaPendente>,
    camada_usada: Instant,
    camada_falhou: bool,
    /// Quadros seguidos com o visual desenhado e sem ele: o recorte só abre com o visual já na tela, e a camada é
    /// limpa um quadro depois de o recorte fechar (abrir ou limpar antes mostraria a área de trabalho num quadro).
    quadros_com_visual: u32,
    quadros_sem_visual: u32,
    /// A camada acabou de nascer ou mudou de tamanho: precisa de um quadro vazio (o conteúdo novo é indefinido).
    camada_suja: bool,
    /// A escala do tamanho escolhido (mínima, padrão, grande), lida do Player e limitada ao que cabe no monitor.
    player_escala: f32,
}

impl VistaIlha {
    /// Abre a janela da Ilha (escondida até haver o que mostrar).
    pub fn abrir(
        cx: &mut App,
        principal: WindowHandle<Player>,
        player: Entity<Player>,
        hwnd_principal: Option<isize>,
    ) -> Option<WindowHandle<VistaIlha>> {
        let (w, h) = ilha::tamanho_da_janela(TamanhoIlha::Grande.escala());
        let opcoes = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds {
                origin: point(px(0.0), px(0.0)),
                size: size(px(w), px(h)),
            })),
            titlebar: None,
            focus: false,
            show: false,
            kind: WindowKind::PopUp,
            is_movable: false,
            is_resizable: false,
            is_minimizable: false,
            window_background: WindowBackgroundAppearance::Transparent,
            app_id: Some("TccPlayer".into()),
            ..Default::default()
        };
        match cx.open_window(opcoes, |window, cx| cx.new(|cx| Self::new(principal, player, hwnd_principal, window, cx)))
        {
            Ok(janela) => Some(janela),
            Err(e) => {
                log::warn!("não foi possível abrir a Ilha: {e}");
                None
            }
        }
    }

    fn new(
        principal: WindowHandle<Player>,
        player: Entity<Player>,
        hwnd_principal: Option<isize>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let hwnd = hwnd_da_janela(window);
        if let Some(h) = hwnd {
            if !integracao::ilha::preparar(h) {
                log::warn!("a Ilha não conseguiu ficar sempre por cima das outras janelas");
            }
            integracao::ilha::regiao(h, 0.0, 0.0, 0.0, 0.0);
        }
        // Um relógio leve: escondida, decide quando ela volta a aparecer; à mostra e sem animação, lê o ponteiro e o
        // estado e só pede um desenho quando algo muda (a Ilha econômica, ver `Ritmo`).
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(TIQUE).await;
                if this.update(cx, |this, cx| this.tique(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        let agora = Instant::now();
        let pagina = std::env::var("TCCPLAYER_ILHA_PAGINA")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .and_then(|i| Pagina::TODAS.get(i).copied())
            .unwrap_or_default();
        let telas = Telas::do_monitor(hwnd_principal.and_then(integracao::ilha::monitor));
        let escala_maxima = ilha::escala_que_cabe((telas.area.w, telas.area.h), (telas.monitor.w, telas.monitor.h));
        Self {
            principal,
            player,
            hwnd,
            hwnd_principal,
            controle: Controle::default(),
            anim: Animacoes::default(),
            anterior: agora,
            clicou: false,
            forma: Forma::Oculta,
            alerta: None,
            pagina,
            visivel: false,
            saindo: false,
            alvo_visto: ilha::medidas(Forma::Repouso, None, 1.0, false),
            crescendo: true,
            desenho_visto: (Forma::Oculta, None, TamanhoIlha::Padrao, false),
            lugar: Lugar::Presa(BordaIlha::Topo, 0.5),
            travada: true,
            lugar_no_arrasto: None,
            modo: Modo::Presa(BordaIlha::Topo),
            lado: Some(BordaIlha::Topo),
            corpo_na_tela: Caixa::default(),
            caixa: Caixa::default(),
            regiao_vista: (0, 0, 0, 0),
            janela_posta: None,
            origem: (0.0, 0.0),
            telas,
            escala_maxima,
            tamanho_janela: ilha::tamanho_da_janela(escala_maxima),
            puxao: (0.0, 0.0),
            pairar_bloqueado: false,
            ponteiro: Ponteiro::do_ambiente(),
            arrasto: None,
            arrastou_em: None,
            deslize: 0.0,
            rolagem: 0.0,
            rolou_em: agora,
            faixa_vista: None,
            volume_visto: None,
            parada_desde: None,
            sistema_em: agora - Duration::from_secs(5),
            tela_cheia: false,
            principal_na_frente: false,
            reduzido_no_sistema: false,
            monitor: None,
            monitores: Vec::new(),
            ultima_faixa: (String::new(), String::new()),
            aberta_para_teste: std::env::var_os("TCCPLAYER_ILHA_ABERTA").is_some_and(|v| v == "1"),
            escondida_para_teste: std::env::var_os("TCCPLAYER_ILHA_ESCONDIDA").is_some_and(|v| v == "1"),
            fundo_de_teste: std::env::var_os("TCCPLAYER_ILHA_ESCONDIDA").is_some_and(|v| v == "1")
                && std::env::var_os("TCCPLAYER_ILHA_FUNDO").is_some_and(|v| v == "1"),
            ritmo: Ritmo::Cheio,
            desenhado_em: agora,
            diagnostico: std::env::var_os("TCCPLAYER_ILHA_DIAGNOSTICO").is_some_and(|v| v == "1").then_some(agora),
            #[cfg(windows)]
            camada: None,
            #[cfg(windows)]
            camada_pendente: None,
            camada_usada: agora,
            camada_falhou: false,
            quadros_com_visual: 0,
            quadros_sem_visual: 0,
            camada_suja: false,
            player_escala: 1.0,
        }
    }

    /// Registra uma linha do diagnóstico (só com `TCCPLAYER_ILHA_DIAGNOSTICO=1`).
    fn diag(&self, o_que: std::fmt::Arguments<'_>) {
        if let Some(t0) = self.diagnostico {
            log::info!("ilha-diag {:.1} {o_que}", t0.elapsed().as_secs_f64() * 1000.0);
        }
    }

    /// A camada do visual sai depois de um tempo sem a Ilha aberta (a placa de vídeo descansa).
    fn soltar_camada_sem_uso(&mut self) {
        #[cfg(windows)]
        if self.camada.is_some() && self.camada_usada.elapsed() > CAMADA_SEM_USO {
            self.camada = None;
            self.quadros_com_visual = 0;
        }
    }

    /// Acabou de arrastar: o clique que vem junto não vale (não abre nem aperta botão).
    fn acabou_de_arrastar(&self) -> bool {
        self.arrastou_em.is_some_and(|t| t.elapsed() < Duration::from_millis(300))
    }

    /// Onde está o ponteiro, em pixels lógicos do monitor.
    fn ponteiro_no_monitor(&self) -> Option<(f32, f32)> {
        self.ponteiro.cursor().map(|p| self.telas.logico(p))
    }

    /// Onde está o ponteiro, em pixels lógicos da janela.
    fn ponteiro_na_janela(&self) -> Option<(f32, f32)> {
        self.ponteiro_no_monitor().map(|p| (p.0 - self.origem.0, p.1 - self.origem.1))
    }

    /// O botão desceu em cima da Ilha: começa um arrasto (ou um clique, se o ponteiro não andar).
    fn comecar_arrasto(&mut self) {
        let Some(ponto) = self.ponteiro_no_monitor() else {
            self.clicou = true;
            return;
        };
        let (cx, cy) = self.corpo_na_tela.centro();
        self.arrasto = Some(Arrasto {
            inicio: ponto,
            pega: (ponto.0 - cx, ponto.1 - cy),
            modo: ModoDoArrasto::Indefinido,
            ultimo: (0.0, Instant::now()),
            velocidade: 0.0,
        });
    }

    /// Acompanha um arrasto em andamento: muda a Ilha de lugar (destravada), estica e volta (travada) ou passa as
    /// páginas. Ao soltar, decide.
    fn acompanhar_arrasto(&mut self, cx: &mut Context<Self>) {
        let Some(mut a) = self.arrasto else { return };
        let agora = Instant::now();
        let Some(ponto) = self.ponteiro_no_monitor() else { return };
        let (dx, dy) = (ponto.0 - a.inicio.0, ponto.1 - a.inicio.1);
        let dt = agora.duration_since(a.ultimo.1).as_secs_f32();
        if dt > 0.004 {
            a.velocidade = a.velocidade * 0.3 + 0.7 * (dx - a.ultimo.0) / dt;
            a.ultimo = (dx, agora);
        }
        if a.modo == ModoDoArrasto::Indefinido && dx.hypot(dy) > LIMIAR_DO_ARRASTO {
            // Aberta, arrastar o conteúdo passa as páginas; as abas no alto e a Ilha fechada mudam de lugar (ou, travada,
            // só esticam).
            let s = self.escala();
            let topo_do_conteudo = self.corpo_na_tela.y + CABECA * s;
            let comecou_no_conteudo = self.forma == Forma::Expandida && a.inicio.1 > topo_do_conteudo;
            // Deitada (no topo, embaixo ou solta), as páginas passam de lado; puxar o conteúdo na vertical leva a
            // Ilha (destravada). Em pé, as páginas e a borda estão no mesmo eixo: leva-se pelas abas.
            let leva_pelo_conteudo = !self.travada && !self.modo.de_pe() && dy.abs() > dx.abs();
            a.modo = if comecou_no_conteudo && !leva_pelo_conteudo {
                ModoDoArrasto::Paginas
            } else {
                // Fechada para arrastar: o ponteiro fica perto do meio da forma pequena.
                let repouso = ilha::medidas_no_modo(self.modo, Forma::Repouso, None, s, false);
                a.pega = (
                    a.pega.0.clamp(-repouso.largura / 2.0, repouso.largura / 2.0),
                    a.pega.1.clamp(-repouso.altura / 2.0, repouso.altura / 2.0),
                );
                self.lugar_no_arrasto = Some(self.lugar);
                self.diag(format_args!("ARRASTO comecou em {:?}", self.lugar));
                ModoDoArrasto::Mover
            };
        }
        if !self.ponteiro.apertado() {
            // Soltou.
            match a.modo {
                ModoDoArrasto::Indefinido => self.clicou = true,
                ModoDoArrasto::Mover => {
                    self.puxao = (0.0, 0.0);
                    if let Some(lugar) = self.lugar_no_arrasto.take() {
                        self.diag(format_args!("SOLTOU em {lugar:?}"));
                        self.lugar = lugar;
                        // O monitor onde ela foi largada fica guardado; no da janela principal, sem escolha guardada,
                        // ela continua seguindo a janela.
                        let aqui = self.monitor.as_ref().map(|m| m.nome.clone());
                        let do_player = self.hwnd_principal.and_then(integracao::ilha::monitor).map(|m| m.nome);
                        self.player.update(cx, |p, _| {
                            if p.ajustes.ilha_monitor.is_some() || aqui != do_player {
                                p.ajustes.ilha_monitor = aqui.or(p.ajustes.ilha_monitor.take());
                            }
                            match lugar {
                                Lugar::Presa(borda, posicao) => {
                                    p.ajustes.ilha_borda = borda;
                                    p.ajustes.ilha_posicao = posicao;
                                    p.ajustes.ilha_solta = None;
                                }
                                Lugar::Solta(x, y) => p.ajustes.ilha_solta = Some([x, y]),
                            }
                            p.salvar();
                        });
                    }
                    self.pairar_bloqueado = true;
                    self.arrastou_em = Some(agora);
                }
                ModoDoArrasto::Paginas => {
                    let largura = Self::largura_da_pagina(self.escala(), self.modo.de_pe());
                    self.pagina = ilha::pagina_depois_do_arrasto(self.pagina, self.deslize, a.velocidade, largura);
                    let atual = self.pagina.indice() as f32 - self.deslize / largura;
                    self.anim.definir_mola(("ilha-pagina", 0), atual);
                    self.deslize = 0.0;
                    self.arrastou_em = Some(agora);
                }
            }
            self.arrasto = None;
            return;
        }
        match a.modo {
            ModoDoArrasto::Mover => {
                let area = self.telas.area;
                let ponteiro = (ponto.0 - area.x, ponto.1 - area.y);
                let centro = (ponto.0 - a.pega.0 - area.x, ponto.1 - a.pega.1 - area.y);
                let lugar = if self.travada {
                    // Travada: desliza pela borda dela; puxada para dentro da tela, estica resistindo e volta.
                    let borda = match (self.lugar_no_arrasto, self.lugar) {
                        (Some(Lugar::Presa(b, _)), _) | (_, Lugar::Presa(b, _)) => b,
                        _ => BordaIlha::Topo,
                    };
                    let para_dentro = match borda {
                        BordaIlha::Topo => ponteiro.1,
                        BordaIlha::Baixo => area.h - ponteiro.1,
                        BordaIlha::Esquerda => ponteiro.0,
                        BordaIlha::Direita => area.w - ponteiro.0,
                    };
                    let estica = elastico((para_dentro - 24.0 * self.escala()).max(0.0), 160.0) * 0.35;
                    self.puxao = match borda {
                        BordaIlha::Topo => (0.0, estica),
                        BordaIlha::Baixo => (0.0, -estica),
                        BordaIlha::Esquerda => (estica, 0.0),
                        BordaIlha::Direita => (-estica, 0.0),
                    };
                    ilha::lugar_travado(centro, (area.w, area.h), borda)
                } else {
                    self.puxao = (0.0, 0.0);
                    ilha::lugar_no_arrasto(ponteiro, centro, (area.w, area.h), self.lugar_no_arrasto)
                };
                if lugar.modo() != self.lugar_no_arrasto.map_or(self.modo, Lugar::modo) {
                    self.diag(format_args!("LUGAR {lugar:?}"));
                }
                self.lugar_no_arrasto = Some(lugar);
            }
            ModoDoArrasto::Paginas => {
                // Nas pontas, o conteúdo resiste (elástico) em vez de parar seco.
                let largura = Self::largura_da_pagina(self.escala(), self.modo.de_pe());
                let ultimo = Pagina::TODAS.len() - 1;
                let i = self.pagina.indice();
                let sobra = if (i == 0 && dx > 0.0) || (i == ultimo && dx < 0.0) { elastico(dx, largura) } else { dx };
                self.deslize = sobra;
            }
            ModoDoArrasto::Indefinido => {}
        }
        self.arrasto = Some(a);
    }

    /// A escala da Ilha: a do tamanho escolhido, sem passar do que cabe no monitor.
    fn escala(&self) -> f32 {
        self.player_escala.min(self.escala_maxima).max(0.5)
    }

    /// A largura de cada página da Ilha aberta, em pixels inteiros (a fileira desliza sem pular pixel).
    fn largura_da_pagina(s: f32, de_pe: bool) -> f32 {
        let modo = if de_pe { Modo::Presa(BordaIlha::Esquerda) } else { Modo::Presa(BordaIlha::Topo) };
        (ilha::medidas_no_modo(modo, Forma::Expandida, None, s, false).largura - 40.0 * s).round()
    }

    /// Um tique do relógio leve. Escondida: decide se a Ilha volta a aparecer. À mostra, sem animação: lê o ponteiro
    /// e o estado e só pede desenho quando a forma muda ou o ritmo dela pede (a cada [`Ritmo::intervalo`]).
    fn tique(&mut self, cx: &mut Context<Self>) {
        if !self.visivel {
            self.decidir(cx);
            self.soltar_camada_sem_uso();
            return;
        }
        if self.ritmo == Ritmo::Cheio {
            // O próprio quadro pede o próximo.
            return;
        }
        let antes = (self.forma, self.alerta, self.saindo, self.arrasto.is_some(), self.lugar);
        self.decidir(cx);
        let mudou = (self.forma, self.alerta, self.saindo, self.arrasto.is_some(), self.lugar) != antes;
        if mudou || self.desenhado_em.elapsed() >= self.ritmo.intervalo() {
            cx.notify();
        }
    }

    /// Atualiza o Player, detecta os avisos e decide a forma, o lugar e se a janela aparece.
    fn decidir(&mut self, cx: &mut Context<Self>) {
        let agora = Instant::now();
        self.player.update(cx, |p, _| p.atualizar_estado());
        let levando = self.arrasto.is_some_and(|a| a.modo == ModoDoArrasto::Mover);
        if levando {
            // Levando a Ilha, ela vai para o monitor em que o ponteiro está.
            if let Some((x, y)) = self.ponteiro.cursor()
                && !self.monitor.as_ref().is_some_and(|m| m.contem(x, y))
                && let Some(m) = integracao::ilha::monitor_no_ponto(x, y)
            {
                self.trocar_de_monitor(Some(m), true);
            }
        } else if agora.duration_since(self.sistema_em) >= Duration::from_millis(150) {
            self.sistema_em = agora;
            self.tela_cheia = integracao::ilha::tela_cheia_de_outro();
            self.principal_na_frente = self.hwnd_principal.is_some_and(integracao::ilha::na_frente_ocupando_a_tela);
            self.reduzido_no_sistema = integracao::ilha::movimento_reduzido();
            // O monitor escolhido (se ainda estiver ligado) ou o da janela principal.
            let todos = integracao::ilha::monitores();
            let escolhido = self.player.read(cx).ajustes.ilha_monitor.clone();
            let monitor = escolhido
                .and_then(|n| todos.iter().find(|m| m.nome == n).cloned())
                .or_else(|| self.hwnd_principal.and_then(integracao::ilha::monitor));
            self.monitores = todos.into_iter().map(|m| m.nome).collect();
            if monitor.is_some() && monitor != self.monitor {
                self.trocar_de_monitor(monitor, false);
            }
        }
        // O ponteiro em cima da forma de agora (com uma folga), e o roteiro de teste apertando o botão nela.
        let ponteiro = self.ponteiro_na_janela();
        let em_cima = self.visivel
            && !self.saindo
            && ponteiro.is_some_and(|p| self.caixa.contem(p, 4.0))
            && self.forma != Forma::Oculta;
        if self.ponteiro.vigiar(self.telas.area_na_tela) {
            self.diag(format_args!("ROTEIRO começou"));
        }
        if self.ponteiro.desceu() && em_cima {
            self.comecar_arrasto();
        }
        for nome in self.ponteiro.comandos_da_hora() {
            let comando = match nome.as_str() {
                "trava" => Some(Comando::AlternarTravaIlha),
                "topo" => Some(Comando::PrenderIlha(BordaIlha::Topo)),
                "baixo" => Some(Comando::PrenderIlha(BordaIlha::Baixo)),
                "esquerda" => Some(Comando::PrenderIlha(BordaIlha::Esquerda)),
                "direita" => Some(Comando::PrenderIlha(BordaIlha::Direita)),
                "monitor" => Some(Comando::IlhaOutroMonitor),
                _ => None,
            };
            self.diag(format_args!("COMANDO do roteiro {nome}"));
            if let Some(c) = comando {
                let _ = self.principal.update(cx, |p, window, cx| p.executar(c, window, cx));
            }
        }
        if self.pairar_bloqueado && !em_cima {
            self.pairar_bloqueado = false;
        }
        self.acompanhar_arrasto(cx);
        let p = self.player.read(cx);
        self.player_escala = p.ajustes.ilha_tamanho.escala();
        self.travada = p.ajustes.ilha_travada;
        self.lugar = match p.ajustes.ilha_solta {
            Some([x, y]) if !self.travada => Lugar::Solta(x, y),
            _ => Lugar::Presa(p.ajustes.ilha_borda, p.ajustes.ilha_posicao),
        };
        let tocando = p.midia.tocando;
        let chave = p.midia.faixa.as_ref().map(|f| format!("{}\u{1f}{}", f.titulo, f.artista));
        let volume = if p.mudo { 0.0 } else { p.volume };

        // Avisos pela diferença com o quadro anterior (o primeiro valor visto não avisa; a faixa sumir por um
        // instante, na troca de origem, também não).
        if chave.is_some() && chave != self.faixa_vista {
            if self.faixa_vista.is_some() {
                self.controle.alertar(Alerta::Faixa, agora);
            }
            self.faixa_vista = chave;
        }
        let da_musica = p.volume_e_da_musica();
        if self.volume_visto.is_some_and(|(de, v)| de == da_musica && (v - volume).abs() > 0.004) {
            self.controle.alertar(Alerta::Volume, agora);
        }
        self.volume_visto = Some((da_musica, volume));
        self.parada_desde = if tocando { None } else { Some(self.parada_desde.unwrap_or(agora)) };

        let arrastando = self.arrasto.is_some_and(|a| a.modo == ModoDoArrasto::Mover);
        let ponteiro_dentro = (em_cima && !self.pairar_bloqueado)
            || self.arrasto.is_some_and(|a| a.modo != ModoDoArrasto::Mover)
            || (self.aberta_para_teste && self.visivel);
        let entrada = Entrada {
            ligada: p.ajustes.ilha,
            abre_ao_pairar: p.ajustes.ilha_ao_pairar,
            avisos: p.ajustes.ilha_avisos,
            tem_faixa: p.midia.faixa.is_some(),
            tocando,
            ponteiro_dentro,
            // O Player maximizado ou em tela cheia, na frente, já mostra tudo: a Ilha sai do caminho.
            tela_cheia_de_outro: (self.tela_cheia || self.principal_na_frente) && !self.escondida_para_teste,
            clicou: std::mem::take(&mut self.clicou) && !self.acabou_de_arrastar(),
            parada_ha_s: self.parada_desde.map_or(0.0, |d| agora.duration_since(d).as_secs_f32()),
            drop_em_s: p.drop_em_s(),
            arrastando,
        };
        let (forma, alerta) = self.controle.passo(&entrada, agora);
        let pede_letra = forma == Forma::Expandida && self.pagina == Pagina::Letra;
        if p.letra_pela_ilha != pede_letra {
            self.player.update(cx, |p, _| p.letra_pela_ilha = pede_letra);
        }

        let quer_ver = forma != Forma::Oculta;
        if quer_ver {
            self.forma = forma;
            self.alerta = alerta;
            self.saindo = false;
        } else if self.visivel {
            // Some encolhendo para a borda; a janela esconde quando a forma acaba de sumir.
            self.saindo = true;
        }
        if quer_ver && !self.visivel {
            self.aparecer();
            cx.notify();
        }
    }

    /// A Ilha passa para outro monitor (ou o dela mudou de resolução ou de barra de tarefas): ela se refaz para caber
    /// nele. Levada pelo ponteiro, a forma continua onde está na tela (sem pular); indo por escolha, ela nasce de novo
    /// no lugar dela no monitor novo.
    fn trocar_de_monitor(&mut self, monitor: Option<integracao::ilha::Monitor>, levada: bool) {
        let antes = self.telas;
        self.telas = Telas::do_monitor(monitor.clone());
        self.monitor = monitor;
        let t = self.telas;
        self.escala_maxima = ilha::escala_que_cabe((t.area.w, t.area.h), (t.monitor.w, t.monitor.h));
        self.tamanho_janela = ilha::tamanho_da_janela(self.escala_maxima);
        self.janela_posta = None;
        self.diag(format_args!(
            "MONITOR {:?} {:?} escala máxima {:.2}",
            self.monitor.as_ref().map(|m| m.nome.as_str()),
            t.monitor,
            self.escala_maxima
        ));
        if !self.visivel {
            return;
        }
        if levada {
            if let (Some(x), Some(y)) = (self.anim.mola_atual(("ilha-ax", 0)), self.anim.mola_atual(("ilha-ay", 0))) {
                let (x, y) = t.de_outro(&antes, (x, y));
                self.anim.definir_mola(("ilha-ax", 0), x);
                self.anim.definir_mola(("ilha-ay", 0), y);
            }
            let c = self.corpo_na_tela;
            let (x, y) = t.de_outro(&antes, (c.x, c.y));
            self.corpo_na_tela = Caixa::new(x, y, c.w, c.h);
            if let Some(a) = self.arrasto.as_mut() {
                a.inicio = t.de_outro(&antes, a.inicio);
            }
        } else if !self.saindo {
            self.aparecer();
        }
    }

    /// A Ilha volta a aparecer: nasce como um risco na borda (solta, um ponto no lugar dela) e cresce até a forma.
    fn aparecer(&mut self) {
        self.visivel = true;
        let s = self.escala();
        let modo = self.lugar.modo();
        self.modo = modo;
        self.lado = modo.borda();
        let risco = risco(modo, s);
        let alvo = ilha::medidas_no_modo(modo, Forma::Repouso, None, s, false);
        let caixa = ilha::caixa_no_lugar(self.lugar, (alvo.largura, alvo.altura), alvo.ombro, self.telas.area);
        let (ax, ay) = ilha::ancora(modo, caixa);
        for (chave, valor) in [
            ("ilha-w", risco.largura),
            ("ilha-h", risco.altura),
            ("ilha-r", 1.0),
            ("ilha-rn", 0.0),
            ("ilha-e", 0.0),
            ("ilha-ax", ax),
            ("ilha-ay", ay),
            ("ilha-puxao-x", 0.0),
            ("ilha-puxao-y", 0.0),
        ] {
            self.anim.definir_mola((chave, 0), valor);
        }
        self.corpo_na_tela = ilha::caixa_da_ancora(modo, (ax, ay), (risco.largura, risco.altura));
        self.crescendo = true;
        self.janela_posta = None;
        self.colocar_janela();
        if let Some(hwnd) = self.hwnd {
            integracao::ilha::mostrar(hwnd, true);
        }
    }

    /// Põe a janela (quadrada) em volta da forma, sem sair do monitor; presa a uma borda, ela encosta na borda e fica
    /// parada enquanto a forma muda de tamanho.
    fn colocar_janela(&mut self) {
        let t = self.telas;
        let (lw, lh) = self.tamanho_janela;
        let o = ilha::janela_para(self.corpo_na_tela, (lw, lh), t.monitor);
        let x = t.canto.0 + (o.0 * t.escala).round() as i32;
        let y = t.canto.1 + (o.1 * t.escala).round() as i32;
        // A origem de verdade (depois de arredondar para pixels da tela), para a forma ficar onde a janela está.
        self.origem = ((x - t.canto.0) as f32 / t.escala, (y - t.canto.1) as f32 / t.escala);
        let y = if self.escondida_para_teste { t.canto.1 - (lh * t.escala).ceil() as i32 - 64 } else { y };
        let posta = (x, y, lw.round() as i32, lh.round() as i32);
        if self.janela_posta != Some(posta)
            && let Some(hwnd) = self.hwnd
        {
            self.janela_posta = Some(posta);
            self.diag(format_args!("MOVER janela {posta:?}"));
            integracao::ilha::mover(hwnd, x, y, lw, lh);
        }
    }

    /// Manda um comando para o Player, pela janela principal.
    fn comandar(&mut self, comando: Comando, cx: &mut Context<Self>) {
        if self.acabou_de_arrastar() {
            return;
        }
        let _ = self.principal.update(cx, |p, window, cx| {
            p.executar(comando, window, cx);
            cx.notify();
        });
    }
}

/// O risco de onde a Ilha nasce e para onde some: presa, uma linha na borda; solta, um ponto.
fn risco(modo: Modo, s: f32) -> Medidas {
    match modo {
        Modo::Presa(b) if b.de_pe() => Medidas { largura: 0.0, altura: 90.0 * s, raio: 0.0, ombro: 0.0 },
        Modo::Presa(_) => Medidas { largura: 90.0 * s, altura: 0.0, raio: 0.0, ombro: 0.0 },
        Modo::Solta => Medidas { largura: 0.0, altura: 0.0, raio: 0.0, ombro: 0.0 },
    }
}

impl Render for VistaIlha {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let agora = Instant::now();
        let dt = agora.duration_since(self.anterior).as_secs_f32().clamp(0.0, 0.1);
        self.anterior = agora;
        self.decidir(cx);
        let p = self.player.read(cx);
        let reduzido = self.reduzido_no_sistema || p.ajustes.modo_seguro;
        let mut dados = Dados::do_player(p);
        if dados.titulo.is_empty() {
            (dados.titulo, dados.artista) = self.ultima_faixa.clone();
        } else {
            self.ultima_faixa = (dados.titulo.clone(), dados.artista.clone());
        }
        if self.monitores.len() > 1 {
            let aqui = self.monitor.as_ref().and_then(|m| self.monitores.iter().position(|n| *n == m.nome));
            dados.monitor = aqui.map(|i| (i, self.monitores.len()));
        }
        let s = self.escala();
        let minima = dados.tamanho == TamanhoIlha::Minima;
        self.anim.quadro(dt, reduzido);

        // O lugar e o modo da forma: durante o arrasto, onde o ponteiro a leva; senão, o dos ajustes. Trocando de modo
        // (de solta para presa, de uma borda para outra), a âncora é refeita na forma de agora: ela não pula.
        let lugar = self.lugar_no_arrasto.unwrap_or(self.lugar);
        let modo = lugar.modo();
        if modo != self.modo {
            let (ax, ay) = ilha::ancora(modo, self.corpo_na_tela);
            self.anim.definir_mola(("ilha-ax", 0), ax);
            self.anim.definir_mola(("ilha-ay", 0), ay);
            self.diag(format_args!("MODO {:?} -> {modo:?}", self.modo));
            self.modo = modo;
        }

        // O alvo: a forma pedida no modo, ou o risco quando a Ilha está saindo.
        let alvo =
            if self.saindo { risco(modo, s) } else { ilha::medidas_no_modo(modo, self.forma, self.alerta, s, minima) };
        if alvo != self.alvo_visto {
            self.crescendo = alvo.largura * alvo.altura > self.alvo_visto.largura * self.alvo_visto.altura;
            self.alvo_visto = alvo;
        }
        let area = self.telas.area;
        let alvo_na_tela = ilha::caixa_no_lugar(lugar, (alvo.largura, alvo.altura), alvo.ombro, area);
        let ancora_alvo = ilha::ancora(modo, alvo_na_tela);

        // Crescer tem impulso (um leve quique, como o Nusa); encolher assenta sem passar.
        let (resposta, amortecimento) = if self.crescendo { ilha::MOLA_CRESCER } else { ilha::MOLA_ENCOLHER };
        let w = self.anim.mola_amortecida(("ilha-w", 0), alvo.largura, resposta, amortecimento).max(0.0);
        let h = self.anim.mola_amortecida(("ilha-h", 0), alvo.altura, resposta, amortecimento).max(0.0);
        let longe = self.anim.mola_amortecida(("ilha-r", 0), alvo.raio, resposta, 1.0).max(0.0);
        // A âncora: solta e arrastada, vai junto com o ponteiro (1:1); grudando numa borda, voa até ela; no resto
        // (travar, prender por comando), anda numa mola sem quique.
        let movendo = self.arrasto.is_some_and(|a| a.modo == ModoDoArrasto::Mover);
        let (ax, ay) = if movendo && (modo == Modo::Solta || self.travada) {
            self.anim.definir_mola(("ilha-ax", 0), ancora_alvo.0);
            self.anim.definir_mola(("ilha-ay", 0), ancora_alvo.1);
            ancora_alvo
        } else {
            let r = if movendo { 0.16 } else { 0.32 };
            (
                self.anim.mola_amortecida(("ilha-ax", 0), ancora_alvo.0, r, 1.0),
                self.anim.mola_amortecida(("ilha-ay", 0), ancora_alvo.1, r, 1.0),
            )
        };
        let puxao = (
            self.anim.mola_amortecida(("ilha-puxao-x", 0), self.puxao.0, 0.22, 0.62),
            self.anim.mola_amortecida(("ilha-puxao-y", 0), self.puxao.1, 0.22, 0.62),
        );
        // O puxão (travada): presa, puxar para dentro da tela estica a forma, que continua colada na borda; ao longo
        // da borda, ela desliza um pouco. Sempre resistindo, e volta ao soltar.
        let (mut w, mut h, mut ancora) = (w, h, (ax, ay));
        match modo {
            Modo::Presa(BordaIlha::Topo) => {
                ancora.0 += puxao.0;
                h += puxao.1.max(0.0);
            }
            Modo::Presa(BordaIlha::Baixo) => {
                ancora.0 += puxao.0;
                h += (-puxao.1).max(0.0);
            }
            Modo::Presa(BordaIlha::Esquerda) => {
                ancora.1 += puxao.1;
                w += puxao.0.max(0.0);
            }
            Modo::Presa(BordaIlha::Direita) => {
                ancora.1 += puxao.1;
                w += (-puxao.0).max(0.0);
            }
            Modo::Solta => ancora = (ax + puxao.0, ay + puxao.1),
        }
        let corpo_na_tela = ilha::caixa_da_ancora(modo, ancora, (w, h));
        self.corpo_na_tela = corpo_na_tela;

        // Os ombros e os cantos do lado da borda: presa, os cantos endireitam e os ombros crescem quando a forma
        // encosta na borda; largando, os ombros somem antes de os cantos arredondarem. O lado dos ombros só muda
        // quando eles já sumiram.
        let toca = match modo {
            Modo::Presa(BordaIlha::Topo) => (corpo_na_tela.y - area.y).abs() < 1.0,
            Modo::Presa(BordaIlha::Baixo) => (area.base() - corpo_na_tela.base()).abs() < 1.0,
            Modo::Presa(BordaIlha::Esquerda) => (corpo_na_tela.x - area.x).abs() < 1.0,
            Modo::Presa(BordaIlha::Direita) => (area.direita() - corpo_na_tela.direita()).abs() < 1.0,
            Modo::Solta => false,
        };
        let ombro_antes = self.anim.mola_atual(("ilha-e", 0)).unwrap_or(0.0);
        let perto_antes = self.anim.mola_atual(("ilha-rn", 0)).unwrap_or(0.0);
        let perto_alvo = if modo == Modo::Solta && ombro_antes < 0.5 { alvo.raio } else { 0.0 };
        let ombro_alvo =
            if modo != Modo::Solta && perto_antes < 0.5 && toca && !self.saindo { alvo.ombro } else { 0.0 };
        let perto = self.anim.mola_amortecida(("ilha-rn", 0), perto_alvo, 0.14, 1.0).max(0.0);
        let ombro = self.anim.mola_amortecida(("ilha-e", 0), ombro_alvo, 0.18, 1.0).max(0.0);
        if ombro < 0.5 {
            self.lado = modo.borda().or(self.lado);
        }
        let raios = ilha::raios(self.lado, perto, longe);

        // A janela em volta da forma, e a forma vista da janela.
        if self.visivel {
            self.colocar_janela();
        }
        let origem = self.origem;
        let corpo_na_janela = corpo_na_tela.menos(origem);
        self.caixa = ilha::caixa_com_ombros(corpo_na_janela, self.lado, ombro);

        // Saindo: quando a forma acaba de sumir, a janela esconde.
        if self.saindo && w.min(h) < 1.5 {
            self.saindo = false;
            self.visivel = false;
            self.forma = Forma::Oculta;
            if let Some(hwnd) = self.hwnd {
                integracao::ilha::mostrar(hwnd, false);
            }
        }
        // Só a forma recebe cliques e aparece: a região da janela segue a caixa dela, sem trocar a cada quadro
        // enquanto a forma anda (ver `ilha::regiao_do_quadro`).
        let assentou = MOLAS_DA_FORMA.into_iter().all(|k| self.anim.parada((k, 0)));
        let caixa_do_alvo = ilha::caixa_com_ombros(alvo_na_tela.menos(origem), modo.borda(), alvo.ombro);
        let regiao =
            ilha::regiao_do_quadro(assentou, self.caixa, caixa_do_alvo, ilha::FOLGA_DA_REGIAO * s, self.regiao_vista);
        if regiao != self.regiao_vista
            && let Some(hwnd) = self.hwnd
        {
            self.diag(format_args!("REGIAO {regiao:?}"));
            self.regiao_vista = regiao;
            integracao::ilha::regiao(hwnd, regiao.0 as f32, regiao.1 as f32, regiao.2 as f32, regiao.3 as f32);
        }

        // Troca de conteúdo (outra forma, outro aviso, outro tamanho da Ilha, deitada ou em pé): o de antes sai num
        // esmaecimento curto, no tamanho dele, e o novo entra quando a forma está quase no tamanho.
        let pedido = (self.forma, self.alerta, dados.tamanho, modo.de_pe());
        let (opacidade, opacidade_alvo) = if pedido != self.desenho_visto {
            let o = self.anim.seguir(("ilha-conteudo", 0), 0.0, 0.05);
            if o < 0.04 {
                self.desenho_visto = pedido;
                self.anim.definir(("ilha-conteudo", 0), 0.0);
            }
            (o, 0.0)
        } else {
            // O conteúdo novo entra quando a forma já chegou perto do tamanho (o recorte o esconde fora dela).
            let pronta = !self.saindo
                && (w - alvo.largura).abs() < alvo.largura.max(1.0) * 0.25
                && (h - alvo.altura).abs() < alvo.altura.max(1.0) * 0.35;
            let alvo_da_opacidade = if pronta { 1.0 } else { 0.0 };
            (self.anim.seguir(("ilha-conteudo", 0), alvo_da_opacidade, 0.07), alvo_da_opacidade)
        };
        let desenho = self.desenho_visto;
        let de_pe = desenho.3;
        let s = desenho.2.escala().min(self.escala_maxima).max(0.5);
        let minima = desenho.2 == TamanhoIlha::Minima;
        // As medidas do conteúdo desenhado: as do alvo ou, enquanto o de antes sai, as da forma dele.
        let modo_do_conteudo = if de_pe { Modo::Presa(BordaIlha::Esquerda) } else { Modo::Presa(BordaIlha::Topo) };
        let alvo_do_conteudo = ilha::medidas_no_modo(modo_do_conteudo, desenho.0, desenho.1, s, minima);
        let aberta = desenho.0 == Forma::Expandida && self.forma == Forma::Expandida && !self.saindo;
        // O corpo, o recorte e o conteúdo em pixels inteiros: o conteúdo, segurado pela âncora, fica parado enquanto a
        // forma muda de tamanho.
        let conteudo_na_tela =
            ilha::caixa_da_ancora(modo, ancora, (alvo_do_conteudo.largura, alvo_do_conteudo.altura)).menos(origem);
        let caixas = ilha::caixas(corpo_na_janela, raios, (conteudo_na_tela.x, conteudo_na_tela.y));

        // As páginas da Ilha aberta: uma fileira que desliza (com a mola, ou com o dedo enquanto arrasta).
        let largura_pagina = Self::largura_da_pagina(s, de_pe);
        let indice = self.pagina.indice() as f32;
        let deslize = if self.arrasto.is_some_and(|a| a.modo == ModoDoArrasto::Paginas) {
            let posicao = indice - self.deslize / largura_pagina;
            self.anim.definir_mola(("ilha-pagina", 0), posicao);
            posicao
        } else {
            self.anim.mola_amortecida(("ilha-pagina", 0), indice, 0.36, 0.88)
        };
        let altura_conteudo = alvo_do_conteudo.altura - (CABECA + 16.0) * s;
        let topo_conteudo = caixas.conteudo_na_janela.1 + CABECA * s;
        let esquerda_conteudo = caixas.conteudo_na_janela.0 + (20.0 * s).round();

        // O visual da página Visual: a camada o desenha na área inteira do conteúdo, um pouco maior, por baixo do
        // preto (sem fresta entre a forma e o visual), e o recorte da forma mostra a parte da página que está à
        // vista na fileira. O recorte só abre com o corpo já grande o bastante e com o visual já na tela.
        let iv = Pagina::Visual.indice() as f32;
        let corpo_cabe =
            h >= alvo_do_conteudo.altura - 16.0 * s + 8.0 && w >= alvo_do_conteudo.largura - 40.0 * s + 16.0;
        let mut buraco = None;
        if aberta && opacidade > 0.05 && corpo_cabe {
            let x = esquerda_conteudo + (iv - deslize) * largura_pagina;
            let (x_a, x_b) = (x.max(esquerda_conteudo), (x + largura_pagina).min(esquerda_conteudo + largura_pagina));
            if x_b - x_a > 2.0 {
                buraco = Some(Recorte { x: x_a, y: topo_conteudo, w: x_b - x_a, h: altura_conteudo, raio: 22.0 * s });
            }
        }
        let area_do_visual = buraco.map(|_| {
            Recorte { x: esquerda_conteudo, y: topo_conteudo, w: largura_pagina, h: altura_conteudo, raio: 22.0 * s }
                .inflado(4.0)
        });
        #[cfg(windows)]
        let visual_no_ar = self.desenhar_visual(window, area_do_visual, &dados, dt);
        #[cfg(not(windows))]
        let visual_no_ar = {
            let _ = area_do_visual;
            false
        };
        let furo = buraco.filter(|_| visual_no_ar);

        let conteudo = match (desenho.0, desenho.1) {
            (Forma::Expandida, _) => {
                let mut fileira =
                    div().absolute().top_0().left(px((-deslize * largura_pagina).round())).h_full().flex();
                for pagina in Pagina::TODAS {
                    let corpo = match pagina {
                        Pagina::Tocando => self.pagina_tocando(&dados, s, de_pe, altura_conteudo, cx),
                        Pagina::Letra => self.pagina_letra(&dados, s, altura_conteudo),
                        Pagina::Visual => self.pagina_visual(&dados, s, cx),
                        Pagina::Ritmo => self.pagina_ritmo(&dados, s, de_pe),
                        Pagina::Ajustes => self.pagina_ajustes(&dados, s, de_pe, altura_conteudo, cx),
                    };
                    fileira = fileira.child(div().w(px(largura_pagina)).h_full().flex_none().child(corpo));
                }
                div()
                    .size_full()
                    .px(px((20.0 * s).round()))
                    .pt(px(12.0 * s))
                    .flex()
                    .flex_col()
                    .gap(px(10.0 * s))
                    .child(self.abas(&dados, s, de_pe, cx))
                    .child(
                        div().relative().w(px(largura_pagina)).h(px(altura_conteudo)).overflow_hidden().child(fileira),
                    )
                    .into_any_element()
            }
            (Forma::Alerta, Some(a)) => self.aviso(&dados, a, s),
            _ => self.repouso(&dados, s, minima, de_pe),
        };
        // Diagnóstico das transições: as medidas deste quadro e, depois do layout, onde o conteúdo ficou de verdade.
        let diag = self.diagnostico.filter(|_| !assentou || (opacidade > 0.0 && opacidade < 1.0));
        if diag.is_some() {
            self.diag(format_args!(
                "dt={:.1} forma={:?}/{:?} modo={modo:?} corpo=({:.2},{:.2} {:.2}x{:.2}) r={longe:.2}/{perto:.2} \
                 e={ombro:.2} conteudo={:?} op={opacidade:.3} s={s} janela={:?}",
                dt * 1000.0,
                self.forma,
                desenho.0,
                corpo_na_tela.x,
                corpo_na_tela.y,
                w,
                h,
                caixas.conteudo_na_janela,
                self.janela_posta,
            ));
        }
        let sonda = diag.map(|t0| {
            canvas(
                move |b, _, _| {
                    log::info!(
                        "ilha-diag {:.1} LAYOUT conteudo=({:.3},{:.3}) {:.2}x{:.2}",
                        t0.elapsed().as_secs_f64() * 1000.0,
                        f32::from(b.origin.x),
                        f32::from(b.origin.y),
                        f32::from(b.size.width),
                        f32::from(b.size.height),
                    )
                },
                |_, (), _, _| {},
            )
            .absolute()
            .top_0()
            .left_0()
            .size_full()
        });
        // O conteúdo tem o tamanho do alvo e fica parado no lugar dele: durante a mudança ele não se rearruma nem
        // anda. O corpo recebe os cliques; o recorte, dentro dele, esconde o conteúdo que sairia da forma.
        let (c, r) = (caixas.corpo, caixas.recorte);
        let destravada = !self.travada;
        let mao = if movendo { CursorStyle::ClosedHand } else { CursorStyle::OpenHand };
        let corpo = div()
            .id("ilha")
            .absolute()
            .left(px(c.x))
            .top(px(c.y))
            .w(px(c.w))
            .h(px(c.h))
            // Destravada, a mão mostra que dá para pegar e levar a Ilha.
            .when(destravada, |d| d.cursor(mao))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                    if !this.ponteiro.e_roteiro() {
                        this.comecar_arrasto();
                    }
                    cx.notify();
                }),
            )
            .on_scroll_wheel(cx.listener(|this, ev: &ScrollWheelEvent, _, cx| {
                if this.forma != Forma::Expandida {
                    return;
                }
                let d = ev.delta.pixel_delta(px(24.0));
                let (x, y) = (f32::from(d.x), f32::from(d.y));
                this.rolagem += if x.abs() > y.abs() { -x } else { -y };
                if this.rolagem.abs() > 40.0 && this.rolou_em.elapsed() > Duration::from_millis(220) {
                    this.pagina = this.pagina.andar(if this.rolagem > 0.0 { 1 } else { -1 });
                    this.rolagem = 0.0;
                    this.rolou_em = Instant::now();
                    cx.notify();
                }
            }))
            .child(
                div().absolute().left(px(r.x)).top(px(r.y)).w(px(r.w)).h(px(r.h)).overflow_hidden().child(
                    div()
                        .absolute()
                        .left(px(caixas.conteudo.0))
                        .top(px(caixas.conteudo.1))
                        .w(px(alvo_do_conteudo.largura))
                        .h(px(alvo_do_conteudo.altura))
                        .opacity(opacidade)
                        .child(conteudo)
                        .children(sonda),
                ),
            );
        // A Ilha econômica: quadro a quadro só enquanto algo anima; tocando com som ou letra à mostra, uns 30 por
        // segundo; parada, quase nada (o relógio segue lendo o ponteiro, então o hover continua na hora).
        let animando = !assentou
            || self.saindo
            || (opacidade - opacidade_alvo).abs() > 0.002
            || self.arrasto.is_some()
            || buraco.is_some()
            || !self.anim.parada(("ilha-pagina", 0))
            || !self.anim.parada(("ilha-letra", 0));
        let com_movimento = dados.tocando
            && match desenho.0 {
                Forma::Expandida => matches!(self.pagina, Pagina::Tocando | Pagina::Letra | Pagina::Ritmo),
                _ => true,
            };
        self.ritmo = if animando {
            Ritmo::Cheio
        } else if com_movimento {
            Ritmo::Lento
        } else {
            Ritmo::Parado
        };
        self.desenhado_em = agora;
        if self.visivel && self.ritmo == Ritmo::Cheio {
            window.request_animation_frame();
        }
        let lado = self.lado;
        div()
            .size_full()
            .relative()
            .when(self.fundo_de_teste, |d| d.bg(hsla(0.83, 1.0, 0.5, 1.0)))
            .font_family(FONTE)
            .text_color(branco(1.0))
            .text_size(px(13.0))
            .child(
                canvas(
                    |_, _, _| (),
                    move |limites, (), window, _| {
                        if let Some(caminho) = forma(limites.origin, corpo_na_janela, raios, lado, ombro, furo) {
                            window.paint_path(caminho, preto());
                        }
                    },
                )
                .absolute()
                .top_0()
                .left_0()
                .size_full(),
            )
            .child(corpo)
    }
}
