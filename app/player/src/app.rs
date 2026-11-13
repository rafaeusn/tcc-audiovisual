//! O estado do Player e o que acontece a cada quadro.
//!
//! A interface roda na thread principal e nunca espera ninguém: a cada quadro ela lê o retrato do som (thread da
//! escuta) e o estado da mídia (thread da sessão), atualiza os números que mostra, manda desenhar o visual na
//! camada de baixo e monta a tela. O desenho de cada região fica em `ui/`.

use crate::anim::{Animacoes, elastico};
use crate::arranjo::{self, Arranjo, Modo, Pedido};
use crate::diario::{Diario, Periodo, Relatorio};
use crate::escuta::{Escuta, Fonte, Situacao};
use crate::gravador::Gravador;
use crate::letrista::{self, Letrista};
use crate::tema::{self, Cores};
use analise::{Retrato, Rotulo};
use gpui::{
    Bounds, Context, DispatchPhase, FocusHandle, KeyDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    RenderImage, SharedString, Window, canvas, div, prelude::*, px,
};
use nucleo::{Ajustes, Comando, Modelo, Modulo, Pagina, Skin, Tema};
use sessao::{Capa, Controle, Estado, Faixa, Sessao};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Colunas do medidor de energia do painel (uma a cada meio segundo).
pub const COLUNAS_ENERGIA: usize = 16;
/// Células do mapa de energia do dock.
pub const CELULAS_MAPA: usize = 96;
const MAXIMO_NA_SESSAO: usize = 30;
/// Trechos guardados no módulo Estrutura (os mais antigos saem).
const MAXIMO_DE_SECOES: usize = 48;
/// Um nível de energia precisa durar isto para virar um trecho novo (evita pisca-pisca na fronteira).
const SECAO_MINIMA_S: f32 = 2.0;
const AVISO: Duration = Duration::from_millis(2600);
const PASSO_VOLUME: f32 = 0.05;
/// De quanto em quanto tempo o resumo da Biblioteca é refeito enquanto a página está aberta.
const RENOVAR_RESUMO_S: f32 = 5.0;
/// Sem a duração da faixa (os apps informam um pouco depois do título), a busca da letra espera isto.
const ESPERA_PELA_DURACAO: Duration = Duration::from_millis(1500);
/// Sem internet, a busca da letra é refeita de tanto em tanto tempo.
const NOVA_BUSCA_DA_LETRA: Duration = Duration::from_secs(20);
/// Quanto cada toque em "adiantar" ou "atrasar" desloca a letra, em segundos.
pub const PASSO_DA_LETRA: f32 = 0.5;

/// Uma faixa ouvida nesta sessão.
pub struct Ouvida {
    pub faixa: Faixa,
    pub capa: Option<Arc<RenderImage>>,
    pub quando: Instant,
}

/// A barra de comandos, enquanto está aberta.
#[derive(Default)]
pub struct Paleta {
    pub consulta: String,
    pub selecionado: usize,
    /// A barra está no modo "buscar a letra" (em vez de buscar comandos).
    pub letra: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AbaBiblioteca {
    /// As músicas das pastas do computador.
    #[default]
    Musicas,
    /// O que foi ouvido, em números.
    Historico,
}

/// O botão em que o mouse está parado, para a dica (nome e atalho) dele.
pub struct DicaAtiva {
    pub id: &'static str,
    /// Com comando, o texto é o nome curto dele; sem, é `rotulo`.
    pub comando: Option<Comando>,
    pub rotulo: &'static str,
    pub desde: Instant,
}

/// Um módulo sendo arrastado no modo edição.
pub struct Arrasto {
    pub modulo: Modulo,
    pub inicio: Point<Pixels>,
    pub atual: Point<Pixels>,
    /// Módulo sob o ponteiro: soltar aqui troca os dois de lugar.
    pub alvo: Option<Modulo>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbaLoja {
    Modulos,
    Temas,
    Skins,
    Layouts,
    Pacotes,
}

/// Um trecho da faixa com o mesmo nível de energia (módulo Estrutura).
#[derive(Clone, Copy, Debug)]
pub struct Secao {
    pub rotulo: Rotulo,
    /// Quando começou, em segundos da faixa.
    pub inicio_s: f64,
}

pub struct Player {
    pub(crate) foco: FocusHandle,
    pub(crate) ajustes: Ajustes,
    gravador: Gravador,
    escuta: Escuta,
    /// A fonte do Modo Escuta pedida por último (só pede de novo quando muda).
    fonte_pedida: Fonte,
    /// Os apps com som aberto agora (executáveis), para escolher o que o Modo Escuta ouve.
    pub(crate) apps_com_som: Vec<String>,
    sessao: Sessao,
    versao_sessao: u64,
    pub(crate) midia: Estado,
    pub(crate) capa: Option<Arc<RenderImage>>,
    pub(crate) historico: VecDeque<Ouvida>,
    pub(crate) retrato: Retrato,
    pub(crate) situacao: Situacao,
    #[cfg(windows)]
    camada: Option<janela::Camada>,
    pub(crate) erro_visual: Option<SharedString>,
    pub(crate) placa: SharedString,
    proxima_camada: Instant,
    pub(crate) pagina: Pagina,
    pub(crate) painel_ajustes: bool,
    pub(crate) gaveta_aberta: bool,
    pub(crate) arranjo: Arranjo,
    anterior: Instant,
    /// Energia recente, uma amostra a cada meio segundo (a mais nova no fim).
    pub(crate) energia_recente: VecDeque<f32>,
    t_energia: f32,
    /// Energia de cada trecho da faixa (ou das últimas células, quando o app não informa a posição).
    /// Negativo = trecho ainda não ouvido.
    pub(crate) mapa: Vec<f32>,
    t_mapa: f32,
    /// Batida atual do compasso (0 a 3), só para o indicador do painel.
    pub(crate) batida: usize,
    fase_anterior: f32,
    /// Vai a 1 no primeiro tempo do compasso e cai rápido: o pulso do logo e do ponto da energia.
    pub(crate) pulso_compasso: f32,
    pub(crate) tom: [f32; 3],
    pub(crate) aviso: Option<(SharedString, Instant)>,
    tamanho_visto: (f32, f32),
    tamanho_desde: Instant,

    pub(crate) anim: Animacoes,
    /// Botão que está sendo apertado agora (para o "afundar ao pressionar").
    pub(crate) pressionado: Option<&'static str>,
    pub(crate) paleta: Option<Paleta>,
    /// Rolagem da lista da barra de comandos (para o item escolhido pelas setas ficar à vista).
    pub(crate) rolagem_paleta: gpui::ScrollHandle,
    pub(crate) edicao: bool,
    pub(crate) arrasto: Option<Arrasto>,
    /// Onde cada módulo foi desenhado no último quadro (para saber em cima de qual o ponteiro está).
    pub(crate) limites: Rc<RefCell<HashMap<Modulo, Bounds<Pixels>>>>,
    pub(crate) aba_loja: AbaLoja,

    pub(crate) volume: f32,
    pub(crate) mudo: bool,
    /// O sistema respondeu ao pedido de volume (sem saída de som, o controle fica apagado).
    pub(crate) volume_disponivel: bool,
    pub(crate) arrastando_volume: bool,
    /// Quanto a barra de volume está esticada além do limite, em pixels.
    pub(crate) volume_esticado: f32,
    pub(crate) pista_volume: Rc<Cell<Bounds<Pixels>>>,
    t_volume: f32,

    pub(crate) secoes: Vec<Secao>,
    candidata: Option<(Rotulo, f32)>,
    /// Há quanto tempo está em silêncio.
    silencio_s: f32,
    inicio_da_faixa: Instant,
    /// Quando a faixa mudou pela última vez (para a entrada do título).
    pub(crate) troca_de_faixa: Instant,
    /// Hora local (horas, minutos), para o módulo Relógio.
    pub(crate) hora: (u8, u8),
    t_hora: f32,

    diario: Diario,
    versao_diario: u64,
    /// O último resumo que a thread do histórico entregou.
    pub(crate) relatorio: Option<Arc<Relatorio>>,
    pub(crate) periodo: Periodo,
    t_relatorio: f32,

    letrista: Letrista,
    versao_letra: u64,
    /// A última resposta da thread das letras (pode ser de uma faixa anterior: comparar a chave).
    pub(crate) letra: letrista::Resposta,
    /// Identidade da faixa atual para a letra (vazio sem faixa ou sem o módulo Letra no painel).
    pub(crate) chave_letra: Option<String>,
    letra_pedida: Option<(String, Instant)>,
    t_letra: f32,
    /// Ajuste manual da sincronia da letra, em segundos (positivo adianta a letra).
    pub(crate) atraso_letra: f32,

    /// O player de arquivos locais: lista, fila e tocador.
    pub(crate) local: crate::local::Local,
    /// Neste quadro o "tocando agora" veio do player local (e não dos outros apps).
    era_local: bool,
    pub(crate) aba_biblioteca: AbaBiblioteca,
    /// O texto da busca de músicas e se o teclado está nela.
    pub(crate) busca_musicas: String,
    pub(crate) buscando_musicas: bool,
    /// No campo de texto ativo: o pedaço ainda provisório (a tecla morta esperando a letra), em bytes.
    pub(crate) marcado: Option<std::ops::Range<usize>>,
    /// As threads do Last.fm e do Discord.
    pub(crate) servicos: crate::servicos::Servicos,
    /// Os módulos da Loja (manifestos que vêm com o app).
    pub(crate) catalogo: Vec<modulos::Manifesto>,
    /// O que está instalado e o que foi aceito de cada módulo.
    pub(crate) instalados: modulos::Instalados,
    arquivo_modulos: Option<std::path::PathBuf>,
    /// A ficha de módulo aberta (posição no catálogo).
    pub(crate) ficha: Option<usize>,
    /// A caixa "Li o que este módulo faz" da ficha.
    pub(crate) aceite_marcado: bool,
    pub(crate) dica: Option<DicaAtiva>,
    /// Onde cada botão com dica foi desenhado no último quadro.
    pub(crate) areas: Rc<RefCell<HashMap<&'static str, Bounds<Pixels>>>>,
    /// O resultado da última busca manual de letra.
    pub(crate) procura: letrista::Procura,
    /// Quando o ponteiro se mexeu pela última vez (os controles da janela mini somem com ele parado).
    pub(crate) ponteiro_em: Instant,
    /// Barra de tarefas, bandeja e janela mini (vazio se o sistema não aceitou a ligação).
    sistema: Option<integracao::Integracao>,
    /// A janela está no modo mini (pequena, no canto, por cima das outras).
    pub(crate) mini: bool,
}

impl Player {
    pub fn new(
        ajustes: Ajustes,
        gravador: Gravador,
        banco: Option<std::path::PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let foco = cx.focus_handle();
        window.focus(&foco);
        let agora = Instant::now();
        let vp = window.viewport_size();
        let tamanho = (f32::from(vp.width), f32::from(vp.height));
        let cores = tema::cores(ajustes.tema);
        let fonte = fonte_de(&ajustes.escuta_app, false);
        let (escuta, sessao) = (Escuta::iniciar(fonte.clone()), Sessao::iniciar());
        sessao.filtrar(&filtro_de(&ajustes.escuta_app));
        let diario = Diario::iniciar(banco.clone(), sessao.leitor(), escuta.leitor());
        let servicos =
            crate::servicos::Servicos::iniciar(nucleo::caminhos::pasta_dados().ok(), sessao.leitor(), escuta.leitor());
        let arquivo_modulos = nucleo::caminhos::arquivo_modulos().ok();
        let instalados = arquivo_modulos.as_deref().map(modulos::Instalados::carregar).unwrap_or_default();
        let mut player = Self {
            foco,
            gravador,
            escuta,
            fonte_pedida: fonte,
            apps_com_som: Vec::new(),
            sessao,
            versao_sessao: u64::MAX,
            midia: Estado::default(),
            capa: None,
            historico: VecDeque::new(),
            retrato: Retrato::default(),
            situacao: Situacao::Iniciando,
            #[cfg(windows)]
            camada: None,
            erro_visual: None,
            placa: "".into(),
            proxima_camada: agora,
            pagina: Pagina::Inicio,
            painel_ajustes: false,
            gaveta_aberta: false,
            arranjo: arranjo::calcular(
                tamanho.0,
                tamanho.1,
                Pedido { painel_aberto: ajustes.painel_aberto, ..Default::default() },
            ),
            anterior: agora,
            energia_recente: std::iter::repeat_n(0.0, COLUNAS_ENERGIA).collect(),
            t_energia: 0.0,
            mapa: vec![-1.0; CELULAS_MAPA],
            t_mapa: 0.0,
            batida: 0,
            fase_anterior: 0.0,
            pulso_compasso: 0.0,
            tom: cores.dock,
            aviso: None,
            tamanho_visto: tamanho,
            tamanho_desde: agora,
            anim: Animacoes::default(),
            pressionado: None,
            paleta: None,
            rolagem_paleta: gpui::ScrollHandle::new(),
            edicao: false,
            arrasto: None,
            limites: Rc::default(),
            aba_loja: AbaLoja::Modulos,
            volume: 0.0,
            mudo: false,
            volume_disponivel: false,
            arrastando_volume: false,
            volume_esticado: 0.0,
            pista_volume: Rc::default(),
            t_volume: 1.0,
            secoes: Vec::new(),
            candidata: None,
            silencio_s: 0.0,
            inicio_da_faixa: agora,
            troca_de_faixa: agora,
            hora: hora_local(),
            t_hora: 0.0,
            diario,
            versao_diario: 0,
            relatorio: None,
            periodo: Periodo::default(),
            t_relatorio: 0.0,
            letrista: Letrista::iniciar(banco),
            versao_letra: 0,
            letra: letrista::Resposta::default(),
            chave_letra: None,
            letra_pedida: None,
            t_letra: 1.0,
            atraso_letra: 0.0,
            local: crate::local::Local::iniciar(
                nucleo::caminhos::pasta_dados().ok(),
                hwnd_da_janela(window),
                std::env::var_os("TCCPLAYER_MUDO").is_some(),
            ),
            era_local: false,
            aba_biblioteca: AbaBiblioteca::default(),
            busca_musicas: String::new(),
            buscando_musicas: false,
            marcado: None,
            servicos,
            catalogo: crate::catalogo::catalogo(),
            instalados,
            arquivo_modulos,
            ficha: None,
            aceite_marcado: false,
            dica: None,
            areas: Rc::default(),
            procura: letrista::Procura::default(),
            ponteiro_em: agora,
            sistema: None,
            mini: false,
            ajustes,
        };
        player.criar_camada(window);
        player.ligar_ao_sistema(window);
        player.aceitar_letra_ja_escolhida();
        player.avisar_servicos();
        // Ajuda de teste: pastas de músicas (separadas por `;`) para já entrar na biblioteca ao abrir.
        if let Some(pastas) = std::env::var_os("TCCPLAYER_PASTA") {
            for pasta in std::env::split_paths(&pastas).filter(|p| !p.as_os_str().is_empty()) {
                player.local.adicionar_pasta(pasta);
            }
        }
        player
    }

    /// Botões na miniatura da barra de tarefas e ícone na bandeja. Os cliques vão direto para a sessão de mídia:
    /// funcionam mesmo com a janela escondida, quando a interface não está desenhando.
    fn ligar_ao_sistema(&mut self, window: &mut Window) {
        #[cfg(windows)]
        {
            use integracao::Acao;
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            if let Ok(RawWindowHandle::Win32(hw)) = window.window_handle().map(|h| h.as_raw()) {
                let controle = self.sessao.controlador();
                let sistema = integracao::Integracao::instalar(hw.hwnd.get(), move |acao| {
                    controle.controlar(match acao {
                        Acao::Anterior => Controle::Anterior,
                        Acao::TocarPausar => Controle::TocarPausar,
                        Acao::Proxima => Controle::Proxima,
                    });
                });
                sistema.fechar_esconde(self.ajustes.fechar_na_bandeja);
                self.sistema = Some(sistema);
            }
        }
        #[cfg(not(windows))]
        {
            let _ = window;
        }
    }

    /// O ponteiro está em cima da janela agora? Sem a resposta do sistema, vale "mexeu há pouco".
    pub(crate) fn ponteiro_na_janela(&self) -> bool {
        self.sistema
            .as_ref()
            .and_then(integracao::Integracao::ponteiro_dentro)
            .unwrap_or_else(|| self.ponteiro_em.elapsed() < Duration::from_millis(1500))
    }

    /// Entra na janela mini ou sai dela.
    fn alternar_mini(&mut self, window: &mut Window) {
        if window.is_fullscreen() {
            self.avisar("Saia da tela cheia para usar a janela mini");
            return;
        }
        let alvo = !self.mini;
        if !self.sistema.as_ref().is_some_and(|s| s.mini(alvo)) {
            self.avisar("A janela mini não está disponível neste sistema");
            return;
        }
        self.mini = alvo;
        if alvo {
            self.pagina = Pagina::Inicio;
            self.painel_ajustes = false;
            self.gaveta_aberta = false;
            self.paleta = None;
            self.sair_da_edicao();
        }
    }

    /// Monta o relatório de diagnóstico e põe na área de transferência.
    fn copiar_diagnostico(&mut self, cx: &mut Context<Self>) {
        use crate::diagnostico;
        let aj = &self.ajustes;
        let dados = diagnostico::Dados {
            versao: env!("CARGO_PKG_VERSION").to_string(),
            sistema: diagnostico::sistema(),
            placa: self.placa.to_string(),
            som: match &self.situacao {
                Situacao::Iniciando => "iniciando".to_string(),
                Situacao::Ouvindo { dispositivo } => format!("ouvindo {dispositivo}"),
                Situacao::EsperandoApp { app } => format!("esperando {app} abrir"),
                Situacao::SemAudio { motivo } => format!("sem saída de áudio ({motivo})"),
            },
            ajustes: format!(
                "visual {:?}, tema {:?}, skin {:?}, cores reativas {}, modo seguro {}, bandeja {}, módulos {:?}",
                aj.modelo, aj.tema, aj.skin, aj.cores_reativas, aj.modo_seguro, aj.fechar_na_bandeja, aj.modulos
            ),
            historico: self.diario.ativo(),
        };
        let registro = nucleo::caminhos::pasta_registros()
            .map(|p| diagnostico::fim_do_registro(&p.join("player.log")))
            .unwrap_or_default();
        let usuario = std::env::var("USERNAME").or_else(|_| std::env::var("USER")).unwrap_or_default();
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(diagnostico::montar(&dados, &registro, &usuario)));
        self.avisar("Relatório copiado: cole onde for relatar o problema");
    }

    /// A skin imersiva depende do visual: sem ele, a interface volta à forma padrão.
    pub(crate) fn imersivo(&self) -> bool {
        self.ajustes.skin == Skin::Imersiva && self.erro_visual.is_none()
    }

    pub(crate) fn cores(&self) -> Cores {
        tema::cores_da_skin(self.ajustes.tema, self.imersivo())
    }

    /// Cria (ou recria) a camada do visual. Se falhar, a interface continua e mostra o motivo no lugar do visual.
    fn criar_camada(&mut self, window: &mut Window) {
        self.proxima_camada = Instant::now() + Duration::from_secs(3);
        #[cfg(windows)]
        {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let escala = window.scale_factor();
            let vp = window.viewport_size();
            let (w, h) = ((f32::from(vp.width) * escala) as u32, (f32::from(vp.height) * escala) as u32);
            let criada = match window.window_handle().map(|h| h.as_raw()) {
                Ok(RawWindowHandle::Win32(hw)) => janela::Camada::new(hw.hwnd.get(), w, h),
                Ok(_) => Err(nucleo::Erro::Gpu("tipo de janela não suportado".into())),
                Err(e) => Err(nucleo::Erro::Gpu(e.to_string())),
            };
            match criada {
                Ok(c) => {
                    self.placa = c.adaptador.clone().into();
                    self.camada = Some(c);
                    self.erro_visual = None;
                }
                Err(e) => {
                    if self.erro_visual.is_none() {
                        log::error!("visual indisponível: {e}");
                    }
                    self.camada = None;
                    self.erro_visual = Some("Não foi possível iniciar o visual nesta placa de vídeo.".into());
                }
            }
        }
        #[cfg(not(windows))]
        {
            let _ = window;
            self.erro_visual = Some("O visual ainda só funciona no Windows.".into());
        }
    }

    pub(crate) fn avisar(&mut self, texto: impl Into<SharedString>) {
        self.aviso = Some((texto.into(), Instant::now() + AVISO));
    }

    fn salvar(&self) {
        self.gravador.salvar(&self.ajustes);
    }

    /// Executa um comando. Botões, atalhos e a barra de comandos passam todos por aqui.
    pub(crate) fn executar(&mut self, comando: Comando, window: &mut Window, cx: &mut Context<Self>) {
        log::debug!("comando {}", comando.id());
        // O botão acabou de ser usado: a dica dele já cumpriu o papel.
        self.dica = None;
        match comando {
            // Com uma música local carregada, os controles valem para ela; senão, para o app que está tocando.
            Comando::TocarPausar if self.local.ativo() => self.local.alternar(),
            Comando::Proxima if self.local.ativo() => self.local.proxima(),
            Comando::Anterior if self.local.ativo() => self.local.anterior(),
            Comando::TocarPausar => self.sessao.controlar(Controle::TocarPausar),
            Comando::Proxima => self.sessao.controlar(Controle::Proxima),
            Comando::Anterior => self.sessao.controlar(Controle::Anterior),
            Comando::AumentarVolume => self.mudar_volume(self.volume + PASSO_VOLUME),
            Comando::DiminuirVolume => self.mudar_volume(self.volume - PASSO_VOLUME),
            Comando::Silenciar => self.silenciar(!self.mudo),
            // Da janela mini, "tela cheia" é sair dela já com a janela maximizada.
            Comando::TelaCheia if self.mini => {
                if self.sistema.as_ref().is_some_and(integracao::Integracao::mini_para_maximizada) {
                    self.mini = false;
                }
            }
            Comando::TelaCheia => window.toggle_fullscreen(),
            Comando::AlternarMini => self.alternar_mini(window),
            Comando::AlternarBandeja => {
                self.ajustes.fechar_na_bandeja = !self.ajustes.fechar_na_bandeja;
                if let Some(s) = &self.sistema {
                    s.fechar_esconde(self.ajustes.fechar_na_bandeja);
                }
                self.avisar(if self.ajustes.fechar_na_bandeja {
                    "Fechar a janela agora deixa o Player na bandeja"
                } else {
                    "Fechar a janela agora encerra o Player"
                });
                self.salvar();
            }
            Comando::CopiarDiagnostico => self.copiar_diagnostico(cx),
            Comando::AlternarDicas => {
                self.ajustes.dicas = !self.ajustes.dicas;
                self.avisar(if self.ajustes.dicas {
                    "Dicas dos botões ligadas"
                } else {
                    "Dicas dos botões desligadas"
                });
                self.salvar();
            }
            Comando::AlternarPainel => match self.arranjo.modo {
                Modo::Largo => {
                    self.ajustes.painel_aberto = !self.ajustes.painel_aberto;
                    self.salvar();
                }
                Modo::Gaveta => self.gaveta_aberta = !self.gaveta_aberta,
                Modo::Empilhado => {}
            },
            Comando::AbrirAjustes => {
                self.painel_ajustes = !self.painel_ajustes || !self.painel_visivel();
                if self.painel_ajustes {
                    self.pagina = Pagina::Inicio;
                    self.edicao = false;
                    self.mostrar_painel();
                }
            }
            Comando::AbrirPagina(p) => {
                if p == Pagina::Biblioteca {
                    self.pedir_relatorio();
                }
                self.buscando_musicas = false;
                self.pagina = p;
                self.painel_ajustes = false;
                self.gaveta_aberta = false;
                self.edicao = false;
            }
            Comando::AbrirPaleta => {
                self.paleta = if self.paleta.is_some() { None } else { Some(Paleta::default()) };
            }
            Comando::Fechar => {
                if self.paleta.is_some() {
                    self.paleta = None;
                } else if self.ficha.is_some() {
                    self.fechar_ficha();
                } else if self.mini {
                    self.alternar_mini(window);
                } else if window.is_fullscreen() {
                    window.toggle_fullscreen();
                } else if self.edicao {
                    self.sair_da_edicao();
                } else if self.gaveta_aberta {
                    self.gaveta_aberta = false;
                } else if self.pagina != Pagina::Inicio {
                    self.pagina = Pagina::Inicio;
                } else {
                    self.painel_ajustes = false;
                }
            }
            Comando::TrocarVisual(m) => self.trocar_visual(m),
            Comando::ProximoVisual => self.trocar_visual(self.ajustes.modelo.proximo()),
            Comando::AlternarCoresReativas => {
                self.ajustes.cores_reativas = !self.ajustes.cores_reativas;
                self.salvar();
            }
            Comando::AlternarModoSeguro => {
                self.ajustes.modo_seguro = !self.ajustes.modo_seguro;
                self.avisar(if self.ajustes.modo_seguro {
                    "Modo seguro ligado: sem clarões"
                } else {
                    "Modo seguro desligado"
                });
                self.salvar();
            }
            Comando::ProximoTema => self.trocar_tema(self.ajustes.tema.proximo()),
            Comando::TrocarTema(t) => self.trocar_tema(t),
            Comando::AlternarImersivo => {
                let nova = if self.ajustes.skin == Skin::Imersiva { Skin::Padrao } else { Skin::Imersiva };
                self.trocar_skin(nova);
            }
            Comando::EditarLayout => {
                if self.edicao {
                    self.sair_da_edicao();
                } else if self.arranjo.so_visual {
                    self.avisar("Saia da tela cheia para personalizar o layout");
                } else {
                    self.edicao = true;
                    self.pagina = Pagina::Inicio;
                    self.painel_ajustes = false;
                    self.mostrar_painel();
                    self.avisar("Modo edição: arraste um módulo sobre outro para trocar");
                }
            }
            Comando::TrocarLado => {
                self.ajustes.painel_a_esquerda = !self.ajustes.painel_a_esquerda;
                self.salvar();
            }
        }
        cx.notify();
    }

    fn sair_da_edicao(&mut self) {
        self.edicao = false;
        self.arrasto = None;
    }

    fn painel_visivel(&self) -> bool {
        match self.arranjo.modo {
            Modo::Largo => self.ajustes.painel_aberto,
            Modo::Gaveta => self.gaveta_aberta,
            Modo::Empilhado => true,
        }
    }

    fn mostrar_painel(&mut self) {
        match self.arranjo.modo {
            Modo::Largo if !self.ajustes.painel_aberto => {
                self.ajustes.painel_aberto = true;
                self.salvar();
            }
            Modo::Gaveta => self.gaveta_aberta = true,
            _ => {}
        }
    }

    fn trocar_visual(&mut self, m: Modelo) {
        if self.ajustes.modelo != m {
            self.ajustes.modelo = m;
            self.avisar(format!("Visual: {}", m.nome()));
            self.salvar();
        }
    }

    pub(crate) fn trocar_tema(&mut self, t: Tema) {
        if self.ajustes.tema != t {
            self.ajustes.tema = t;
            self.avisar(format!("Tema: {}", t.nome()));
            self.salvar();
        }
    }

    pub(crate) fn trocar_skin(&mut self, s: Skin) {
        if self.ajustes.skin != s {
            self.ajustes.skin = s;
            self.avisar(if s == Skin::Imersiva {
                "Modo imersivo: o visual vira o fundo"
            } else {
                "Modo imersivo desligado"
            });
            self.salvar();
        }
    }

    /// Escolhe o que o Modo Escuta ouve: `""` = todo o som do PC; senão, o executável de um app.
    pub(crate) fn escolher_fonte(&mut self, exe: &str) {
        self.ajustes.escuta_app = exe.to_string();
        self.ajustes = self.ajustes.clone().validar();
        self.sessao.filtrar(&filtro_de(&self.ajustes.escuta_app));
        self.salvar();
        let frase = if exe.is_empty() {
            "O Modo Escuta volta a ouvir todo o som do PC".to_string()
        } else {
            format!("O Modo Escuta agora ouve só {}", sessao::nome_do_app(exe))
        };
        self.avisar(frase);
    }

    pub(crate) fn escolher_paleta(&mut self, i: usize) {
        if i < nucleo::ajustes::PALETAS.len() && self.ajustes.paleta != i {
            self.ajustes.paleta = i;
            self.salvar();
        }
    }

    /// Layouts prontos da Loja: mostra ou esconde o painel e escolhe o lado dele.
    pub(crate) fn aplicar_layout(&mut self, painel_aberto: bool, a_esquerda: bool) {
        self.ajustes.painel_aberto = painel_aberto;
        self.ajustes.painel_a_esquerda = a_esquerda;
        self.salvar();
    }

    // ------------------------------------------------------------------ módulos do painel

    pub(crate) fn remover_modulo(&mut self, m: Modulo) {
        self.ajustes.modulos.retain(|x| *x != m);
        self.limites.borrow_mut().remove(&m);
        self.salvar();
    }

    // ------------------------------------------------------------------ músicas locais

    pub(crate) fn abrir_aba_da_biblioteca(&mut self, aba: AbaBiblioteca) {
        self.aba_biblioteca = aba;
        self.buscando_musicas = false;
        if aba == AbaBiblioteca::Historico {
            self.pedir_relatorio();
        }
    }

    /// Toca a música `i` da lista da Biblioteca; as seguintes viram a fila.
    pub(crate) fn tocar_musica(&mut self, i: usize) {
        self.buscando_musicas = false;
        self.local.tocar(i);
    }

    /// Abre a janela do sistema para escolher pastas de música.
    pub(crate) fn escolher_pasta(&mut self, cx: &mut Context<Self>) {
        let escolha = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: true,
            prompt: Some("Adicionar à biblioteca".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(pastas))) = escolha.await {
                let _ = this.update(cx, |this, cx| {
                    for pasta in pastas {
                        this.local.adicionar_pasta(pasta);
                    }
                    this.aba_biblioteca = AbaBiblioteca::Musicas;
                    this.avisar("Lendo as músicas da pasta...");
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Tecla com a busca de músicas ativa: digitar filtra a lista; Esc ou Enter devolvem os atalhos.
    fn tecla_na_busca_de_musicas(&mut self, ev: &KeyDownEvent, cx: &mut Context<Self>) {
        let k = &ev.keystroke;
        let ctrl = k.modifiers.control || k.modifiers.platform;
        // As letras chegam pelo caminho de texto do sistema (ver `ui::campo`); aqui só as teclas de controle.
        match k.key.as_str() {
            "escape" | "enter" => {
                self.buscando_musicas = false;
                self.marcado = None;
            }
            "backspace" => self.apagar_no_campo(ctrl),
            "v" if ctrl => self.colar_no_campo(cx),
            _ => {}
        }
        cx.notify();
    }

    /// Clique no mapa do dock: vai para aquele ponto da música (só para a música local, que é a que o app toca).
    pub(crate) fn ir_no_mapa(&mut self, x: Pixels) {
        let Some(area) = self.areas.borrow().get("dock-mapa").copied() else { return };
        let largura = f32::from(area.size.width);
        let duracao = self.local.situacao.duracao_s;
        if self.local.ativo() && largura > 1.0 && duracao > 1.0 {
            let parte = (f32::from(x - area.origin.x) / largura).clamp(0.0, 1.0);
            self.local.ir(duracao * f64::from(parte));
        }
    }

    // ------------------------------------------------------------------ módulos da Loja

    /// Grava a lista de instalados fora da thread da interface.
    fn salvar_instalados(&self) {
        let Some(caminho) = self.arquivo_modulos.clone() else { return };
        let lista = self.instalados.clone();
        let gravando = std::thread::Builder::new().name("modulos".into()).spawn(move || {
            if let Err(e) = lista.salvar(&caminho) {
                log::warn!("não foi possível salvar os módulos instalados: {e}");
            }
        });
        if let Err(e) = gravando {
            log::warn!("não foi possível salvar os módulos instalados: {e}");
        }
    }

    /// Quem já tinha posto a Letra no painel antes de existir a ficha de instalação escolheu usá-la sabendo o
    /// que ela faz (a Loja já dizia): a escolha é mantida, registrada como instalação.
    fn aceitar_letra_ja_escolhida(&mut self) {
        use crate::catalogo::ID_LETRA;
        if self.ajustes.modulos.contains(&Modulo::Letra)
            && self.instalados.achar(ID_LETRA).is_none()
            && let Some(m) = self.catalogo.iter().find(|m| m.id == ID_LETRA)
        {
            self.instalados.instalar(m);
            self.salvar_instalados();
            log::info!("módulo Letra registrado como instalado (já estava no painel)");
        }
    }

    /// Diz às threads do Last.fm e do Discord se o módulo de cada uma está instalado e ligado.
    fn avisar_servicos(&self) {
        self.servicos.ativar_lastfm(self.instalados.ativo(crate::catalogo::ID_LASTFM));
        self.servicos.ativar_discord(self.instalados.ativo(crate::catalogo::ID_DISCORD));
    }

    /// Abre a ficha do módulo `id` do catálogo.
    pub(crate) fn abrir_ficha(&mut self, id: &str) {
        self.ficha = self.catalogo.iter().position(|m| m.id == id);
        self.aceite_marcado = false;
    }

    pub(crate) fn fechar_ficha(&mut self) {
        self.ficha = None;
        self.aceite_marcado = false;
    }

    /// Instala o módulo da ficha aberta. Só vale com a caixa de aceite marcada e para módulo que já tem código.
    pub(crate) fn instalar_modulo_aberto(&mut self) {
        let Some(m) = self.ficha.and_then(|i| self.catalogo.get(i)).cloned() else { return };
        if !m.pronto || !self.aceite_marcado {
            return;
        }
        self.instalados.instalar(&m);
        self.salvar_instalados();
        self.avisar_servicos();
        self.aceite_marcado = false;
        if m.id == crate::catalogo::ID_LETRA {
            self.fechar_ficha();
            self.adicionar_modulo(Modulo::Letra);
        }
        // Os módulos de serviço ficam com a ficha aberta: é nela que se conecta a conta e se vê a situação.
        self.avisar(format!("Módulo {} instalado", m.nome));
    }

    pub(crate) fn desinstalar_modulo_aberto(&mut self) {
        let Some(m) = self.ficha.and_then(|i| self.catalogo.get(i)).cloned() else { return };
        if self.instalados.remover(&m.id) {
            self.salvar_instalados();
        }
        // Desinstalar o Last.fm também esquece a conta: a autorização guardada é apagada.
        if m.id == crate::catalogo::ID_LASTFM {
            self.servicos.desconectar_lastfm();
        }
        self.avisar_servicos();
        if m.id == crate::catalogo::ID_LETRA {
            self.remover_modulo(Modulo::Letra);
        }
        self.fechar_ficha();
        self.avisar(format!("Módulo {} desinstalado", m.nome));
    }

    pub(crate) fn adicionar_modulo(&mut self, m: Modulo) {
        // A Letra usa a internet: antes de entrar no painel, a pessoa vê o que ela faz e aceita.
        if m == Modulo::Letra && !self.instalados.ativo(crate::catalogo::ID_LETRA) {
            self.abrir_ficha(crate::catalogo::ID_LETRA);
            return;
        }
        if !self.ajustes.modulos.contains(&m) {
            self.ajustes.modulos.push(m);
            self.salvar();
        }
    }

    /// Começa a arrastar um módulo (modo edição).
    pub(crate) fn pegar_modulo(&mut self, modulo: Modulo, onde: Point<Pixels>) {
        if self.edicao {
            self.arrasto = Some(Arrasto { modulo, inicio: onde, atual: onde, alvo: None });
        }
    }

    /// Deslocamento atual de um módulo em relação ao lugar dele: segue o ponteiro enquanto é arrastado e volta
    /// (ou assenta no lugar novo) com uma aproximação suave quando é solto.
    pub(crate) fn deslocamento_do_modulo(&self, m: Modulo) -> (f32, f32) {
        let (kx, ky) = (("modulo-x", m as u32), ("modulo-y", m as u32));
        match &self.arrasto {
            Some(a) if a.modulo == m => {
                let (dx, dy) = (f32::from(a.atual.x - a.inicio.x), f32::from(a.atual.y - a.inicio.y));
                self.anim.definir(kx, dx);
                self.anim.definir(ky, dy);
                (dx, dy)
            }
            _ => (self.anim.seguir(kx, 0.0, 0.36), self.anim.seguir(ky, 0.0, 0.36)),
        }
    }

    fn soltar_modulo(&mut self, a: Arrasto) {
        let Some(alvo) = a.alvo else { return };
        let (Some(ia), Some(ib)) = (
            self.ajustes.modulos.iter().position(|m| *m == a.modulo),
            self.ajustes.modulos.iter().position(|m| *m == alvo),
        ) else {
            return;
        };
        // Cada um parte de onde está na tela e vai até o lugar novo, em vez de pular.
        {
            let limites = self.limites.borrow();
            if let (Some(la), Some(lb)) = (limites.get(&a.modulo), limites.get(&alvo)) {
                let arrastado = (f32::from(a.atual.x - a.inicio.x), f32::from(a.atual.y - a.inicio.y));
                let entre = (f32::from(la.origin.x - lb.origin.x), f32::from(la.origin.y - lb.origin.y));
                self.anim.definir(("modulo-x", a.modulo as u32), entre.0);
                self.anim.definir(("modulo-y", a.modulo as u32), entre.1);
                self.anim.definir(("modulo-x", alvo as u32), arrastado.0 - entre.0);
                self.anim.definir(("modulo-y", alvo as u32), arrastado.1 - entre.1);
            }
        }
        self.ajustes.modulos.swap(ia, ib);
        self.salvar();
    }

    // ------------------------------------------------------------------ histórico e letra

    /// O histórico está gravando (há pasta de dados e o banco abriu).
    pub(crate) fn historico_ativo(&self) -> bool {
        self.diario.ativo()
    }

    fn pedir_relatorio(&mut self) {
        self.t_relatorio = 0.0;
        self.diario.pedir(self.periodo);
    }

    pub(crate) fn escolher_periodo(&mut self, p: Periodo) {
        if self.periodo != p {
            self.periodo = p;
            self.pedir_relatorio();
        }
    }

    /// Lê o resumo que ficou pronto e, com a Biblioteca aberta, pede um novo de tempos em tempos.
    fn atualizar_relatorio(&mut self, dt: f32) {
        let versao = self.diario.versao();
        if versao != self.versao_diario {
            self.versao_diario = versao;
            self.relatorio = self.diario.relatorio();
        }
        if self.pagina == Pagina::Biblioteca {
            self.t_relatorio += dt;
            if self.t_relatorio >= RENOVAR_RESUMO_S {
                self.pedir_relatorio();
            }
        }
    }

    /// Abre a busca manual de letra, já com o nome da faixa que está tocando.
    pub(crate) fn abrir_busca_de_letra(&mut self) {
        let consulta: String = self
            .midia
            .faixa
            .as_ref()
            .map(|f| {
                let (titulo, artista) = letras::limpar(&f.titulo, &f.artista);
                format!("{titulo} {artista}").trim().chars().take(80).collect()
            })
            .unwrap_or_default();
        if !consulta.is_empty() {
            self.letrista.procurar(consulta.clone());
        }
        self.paleta = Some(Paleta { consulta, selecionado: 0, letra: true });
    }

    /// Procura no LRCLIB o que está digitado na busca de letra.
    pub(crate) fn procurar_letra(&mut self) {
        if let Some(consulta) = self.paleta.as_ref().map(|p| p.consulta.trim().to_string()).filter(|c| !c.is_empty()) {
            self.letrista.procurar(consulta);
        }
    }

    /// Usa o resultado `i` da busca como a letra da faixa que está tocando.
    pub(crate) fn fixar_letra(&mut self, i: usize) {
        let letrista::Achados::Prontos(lista) = &self.procura.achados else { return };
        let (Some(chave), Some(registro)) = (self.chave_letra.clone(), lista.get(i).cloned()) else { return };
        self.letrista.fixar(chave, registro);
        self.paleta = None;
        self.avisar("Letra trocada nesta música");
    }

    /// Adianta (positivo) ou atrasa a letra em relação à música. Vale até a faixa mudar.
    pub(crate) fn deslocar_letra(&mut self, segundos: f32) {
        self.atraso_letra = (self.atraso_letra + segundos).clamp(-10.0, 10.0);
    }

    /// Posição e duração da música para a letra, já com o ajuste manual. Vazio quando o app de origem não informa.
    pub(crate) fn tempo_da_letra(&self) -> Option<(f64, f64)> {
        let p = self.midia.posicao.filter(|p| p.duracao_s > 1.0)?;
        Some((p.agora(self.midia.tocando) + f64::from(self.atraso_letra), p.duracao_s))
    }

    /// Pede a letra da faixa atual quando o módulo Letra está no painel. Sem o módulo, nada vai para a rede.
    fn atualizar_letra(&mut self, dt: f32) {
        let versao = self.letrista.versao();
        if versao != self.versao_letra {
            self.versao_letra = versao;
            let nova = self.letrista.resposta();
            if nova != self.letra {
                self.letra = nova;
                // A letra nova entra do começo, sem rolar a partir da linha da música anterior.
                self.anim.definir(("letra", 0), -1.0);
            }
            self.procura = self.letrista.procura();
        }
        self.t_letra += dt;
        if self.t_letra < 0.25 {
            return;
        }
        self.t_letra = 0.0;
        let faixa = self.midia.faixa.as_ref().filter(|f| !f.titulo.is_empty());
        let ligada = self.ajustes.modulos.contains(&Modulo::Letra) && self.instalados.ativo(crate::catalogo::ID_LETRA);
        let (Some(faixa), true) = (faixa, ligada) else {
            self.chave_letra = None;
            return;
        };
        let duracao = self.midia.posicao.map(|p| p.duracao_s).filter(|d| *d > 1.0);
        if duracao.is_none() && self.troca_de_faixa.elapsed() < ESPERA_PELA_DURACAO {
            return;
        }
        let (titulo, artista) = letras::limpar(&faixa.titulo, &faixa.artista);
        let chave = letras::chave(&titulo, &artista, duracao);
        let respondida = self.letra.chave == chave && self.letra.situacao != letrista::Situacao::SemRede;
        let pedida = self
            .letra_pedida
            .as_ref()
            .is_some_and(|(c, quando)| *c == chave && (respondida || quando.elapsed() < NOVA_BUSCA_DA_LETRA));
        if !pedida {
            let pedido = letras::Pedido { titulo, artista, album: faixa.album.clone(), duracao_s: duracao };
            self.letrista.pedir(chave.clone(), pedido);
            self.letra_pedida = Some((chave.clone(), Instant::now()));
        }
        self.chave_letra = Some(chave);
    }

    // ------------------------------------------------------------------ volume

    pub(crate) fn mudar_volume(&mut self, alvo: f32) {
        if !alvo.is_finite() {
            return;
        }
        let alvo = alvo.clamp(0.0, 1.0);
        if audio::volume::definir(alvo) {
            self.volume = alvo;
            self.volume_disponivel = true;
            // Mexer no volume com o som mudo devolve o som, como no sistema.
            if self.mudo && alvo > 0.0 && audio::volume::silenciar(false) {
                self.mudo = false;
            }
        }
    }

    fn silenciar(&mut self, mudo: bool) {
        if audio::volume::silenciar(mudo) {
            self.mudo = mudo;
        }
    }

    /// Leva o volume até onde o ponteiro está na pista. Além das pontas a barra estica com resistência.
    fn volume_no_ponteiro(&mut self, x: Pixels) {
        let pista = self.pista_volume.get();
        let largura = f32::from(pista.size.width);
        if largura < 1.0 {
            return;
        }
        let pos = f32::from(x - pista.origin.x);
        let sobra = if pos < 0.0 { pos } else { (pos - largura).max(0.0) };
        self.volume_esticado = elastico(sobra, largura);
        self.mudar_volume(pos / largura);
    }

    pub(crate) fn pegar_volume(&mut self, x: Pixels) {
        if self.volume_disponivel {
            self.arrastando_volume = true;
            self.volume_no_ponteiro(x);
        }
    }

    // ------------------------------------------------------------------ ponteiro (ouvintes da janela inteira)

    /// O ponteiro se moveu em qualquer lugar da janela. Devolve se algo mudou.
    fn ao_mover(&mut self, pos: Point<Pixels>) -> bool {
        self.ponteiro_em = Instant::now();
        let mut mudou = false;
        // O botão da dica saiu de baixo do ponteiro sem avisar (a tela mudou, por exemplo): a dica some.
        if let Some(d) = &self.dica
            && !self.areas.borrow().get(d.id).is_some_and(|a| a.contains(&pos))
        {
            self.dica = None;
            mudou = true;
        }
        if self.arrastando_volume {
            self.volume_no_ponteiro(pos.x);
            mudou = true;
        }
        if let Some(a) = &mut self.arrasto {
            a.atual = pos;
            a.alvo = self.limites.borrow().iter().find(|(m, l)| **m != a.modulo && l.contains(&pos)).map(|(m, _)| *m);
            mudou = true;
        }
        mudou
    }

    /// O botão do mouse foi solto em qualquer lugar da janela.
    fn ao_soltar(&mut self) -> bool {
        let mudou = self.pressionado.is_some() || self.arrastando_volume || self.arrasto.is_some();
        self.pressionado = None;
        if self.arrastando_volume {
            self.arrastando_volume = false;
            // A barra volta do ponto em que estava esticada.
            self.anim.definir(("volume-esticado", 0), self.volume_esticado);
            self.volume_esticado = 0.0;
        }
        if let Some(a) = self.arrasto.take() {
            self.soltar_modulo(a);
        }
        mudou
    }

    // ------------------------------------------------------------------ o quadro

    /// Um quadro: lê o que mudou, atualiza os números da tela e desenha o visual.
    fn quadro(&mut self, window: &mut Window) {
        let agora = Instant::now();
        let dt = agora.duration_since(self.anterior).as_secs_f32().clamp(0.0, 0.1);
        self.anterior = agora;
        self.anim.quadro(dt, false);

        self.retrato = self.escuta.retrato();
        self.situacao = self.escuta.situacao();
        self.apps_com_som = self.escuta.apps();
        // O "tocando agora" vem do player local quando ele tem uma música carregada; senão, dos outros apps.
        self.local.quadro();
        // Com um app escolhido, a música local ainda move o visual: enquanto ela toca, a escuta ouve o Player.
        let fonte = fonte_de(&self.ajustes.escuta_app, self.local.ativo());
        if fonte != self.fonte_pedida {
            self.escuta.escolher(fonte.clone());
            self.fonte_pedida = fonte;
        }
        match self.local.estado_de_midia() {
            Some(estado) => {
                self.era_local = true;
                self.atualizar_midia(estado, window);
            }
            None => {
                let versao = self.sessao.versao();
                if self.era_local || versao != self.versao_sessao {
                    self.era_local = false;
                    self.versao_sessao = versao;
                    let mut estado = self.sessao.estado();
                    // O resto da nossa própria fila, já parada, não é "tocando agora".
                    if estado.faixa.as_ref().is_some_and(|f| self.local.e_resto(f)) {
                        estado = Estado { disponivel: estado.disponivel, ..Default::default() };
                    }
                    self.atualizar_midia(estado, window);
                }
            }
        }

        let vp = window.viewport_size();
        let (w, h) = (f32::from(vp.width), f32::from(vp.height));
        let tela_cheia = window.is_fullscreen();
        self.arranjo = arranjo::calcular(
            w,
            h,
            Pedido {
                painel_aberto: self.ajustes.painel_aberto,
                gaveta_aberta: self.gaveta_aberta,
                tela_cheia,
                painel_a_esquerda: self.ajustes.painel_a_esquerda,
                imersivo: self.imersivo(),
                mini: self.mini,
            },
        );
        if self.arranjo.modo != Modo::Gaveta {
            self.gaveta_aberta = false;
        }
        if self.arranjo.so_visual {
            self.sair_da_edicao();
        }
        self.lembrar_tamanho(w, h, tela_cheia || self.mini || window.is_maximized());
        if let Some(s) = &self.sistema {
            s.tocando(self.midia.tocando);
        }

        self.atualizar_medidores(dt);
        self.atualizar_sistema(dt);
        self.atualizar_relatorio(dt);
        self.atualizar_letra(dt);
        self.desenhar_visual(window, dt);

        if self.aviso.as_ref().is_some_and(|(_, ate)| agora >= *ate) {
            self.aviso = None;
        }
        window.request_animation_frame();
    }

    /// A faixa, a capa ou o estado de reprodução mudaram.
    fn atualizar_midia(&mut self, novo: Estado, window: &mut Window) {
        let mudou_faixa = novo.faixa.as_ref().map(Faixa::chave) != self.midia.faixa.as_ref().map(Faixa::chave);
        let mudou_capa = novo.capa.as_ref().map(Arc::as_ptr) != self.midia.capa.as_ref().map(Arc::as_ptr);
        if mudou_faixa {
            self.mapa.fill(-1.0);
            self.capa = None;
            self.secoes.clear();
            self.candidata = None;
            self.inicio_da_faixa = Instant::now();
            self.troca_de_faixa = Instant::now();
            self.atraso_letra = 0.0;
        }
        if mudou_faixa || mudou_capa {
            self.capa = novo.capa.as_deref().and_then(imagem_da_capa);
        }
        if let Some(faixa) = novo.faixa.as_ref().filter(|f| !f.titulo.is_empty()) {
            match self.historico.front_mut() {
                // A capa chega um pouco depois do título: completa a entrada que já existe.
                Some(o) if o.faixa.chave() == faixa.chave() => o.capa = self.capa.clone().or(o.capa.take()),
                _ => {
                    self.historico.push_front(Ouvida {
                        faixa: faixa.clone(),
                        capa: self.capa.clone(),
                        quando: Instant::now(),
                    });
                    while self.historico.len() > MAXIMO_NA_SESSAO {
                        // Sem isso o atlas de imagens do GPUI guardaria a capa para sempre.
                        if let Some(Ouvida { capa: Some(img), .. }) = self.historico.pop_back() {
                            let _ = window.drop_image(img);
                        }
                    }
                }
            }
        }
        self.midia = novo;
        if let Some(s) = &self.sistema {
            let (titulo, artista) =
                self.midia.faixa.as_ref().map_or(("", ""), |f| (f.titulo.as_str(), f.artista.as_str()));
            s.dica(&integracao::texto_da_dica(titulo, artista));
        }
    }

    /// Onde a faixa está agora, em segundos: a posição que o app informa ou, sem ela, o tempo desde que começou.
    pub(crate) fn tempo_da_faixa(&self) -> f64 {
        match self.midia.posicao {
            Some(p) if p.duracao_s > 1.0 => p.agora(self.midia.tocando),
            _ => self.inicio_da_faixa.elapsed().as_secs_f64(),
        }
    }

    fn atualizar_medidores(&mut self, dt: f32) {
        let r = self.retrato;
        self.t_energia += dt;
        if self.t_energia >= 0.5 {
            self.t_energia = 0.0;
            self.energia_recente.pop_front();
            self.energia_recente.push_back(r.energia * r.gate);
        }
        // Batida do compasso: avança quando a fase da batida dá a volta.
        if r.conf_batida > 0.15 && r.gate > 0.5 && r.fase_batida < self.fase_anterior - 0.5 {
            self.batida = (self.batida + 1) % 4;
            // Pulso só no primeiro tempo, e nunca no modo seguro.
            if self.batida == 0 && !self.ajustes.modo_seguro {
                self.pulso_compasso = 1.0;
            }
        }
        self.fase_anterior = r.fase_batida;
        self.pulso_compasso *= (-dt / 0.09).exp();

        // Mapa de energia: com a posição da faixa, cada célula é um trecho; sem ela, as células rolam.
        let n = self.mapa.len();
        match self.midia.posicao.filter(|p| p.duracao_s > 1.0) {
            Some(p) if self.midia.tocando => {
                let i = ((p.agora(true) / p.duracao_s) * n as f64) as usize;
                if let Some(c) = self.mapa.get_mut(i.min(n - 1)) {
                    *c = if *c < 0.0 { r.energia } else { *c + (r.energia - *c) * (dt * 2.0).min(1.0) };
                }
            }
            Some(_) => {}
            None => {
                self.t_mapa += dt;
                if self.t_mapa >= 0.5 && r.gate > 0.5 {
                    self.t_mapa = 0.0;
                    self.mapa.rotate_left(1);
                    self.mapa[n - 1] = r.energia;
                }
            }
        }

        self.atualizar_secoes(dt);

        // Tom do dock: segue a cor do visual devagar; neutro no modo seguro ou sem cores reativas.
        let cores = self.cores();
        let segue = self.ajustes.cores_reativas && !self.ajustes.modo_seguro && r.gate > 0.3;
        let alvo =
            if segue { tema::tom_do_dock(analise::cores(r.energia, 0.0).0, self.ajustes.tema) } else { cores.dock };
        let k = 1.0 - (-dt / 2.0).exp();
        for (tom, alvo) in self.tom.iter_mut().zip(alvo) {
            *tom += (alvo - *tom) * k;
        }
    }

    /// Estrutura da faixa: registra quando o nível de energia muda e fica.
    fn atualizar_secoes(&mut self, dt: f32) {
        let r = self.retrato;
        if r.gate < 0.5 {
            self.candidata = None;
            self.silencio_s += dt;
            return;
        }
        // Sem faixa informada pelo app, "a faixa" começa quando o som volta depois de um silêncio.
        if self.silencio_s > 3.0 && self.midia.posicao.is_none() {
            self.secoes.clear();
            self.inicio_da_faixa = Instant::now();
        }
        self.silencio_s = 0.0;
        let t = self.tempo_da_faixa();
        // A pessoa voltou a faixa: o que estava registrado depois desse ponto não vale mais.
        while self.secoes.last().is_some_and(|s| s.inicio_s > t + 1.0) {
            self.secoes.pop();
        }
        let Some(atual) = self.secoes.last().map(|s| s.rotulo) else {
            self.secoes.push(Secao { rotulo: r.rotulo, inicio_s: t });
            return;
        };
        if r.rotulo == atual {
            self.candidata = None;
            return;
        }
        let durou = match self.candidata {
            Some((rotulo, tempo)) if rotulo == r.rotulo => tempo + dt,
            _ => 0.0,
        };
        self.candidata = Some((r.rotulo, durou));
        if durou >= SECAO_MINIMA_S {
            self.candidata = None;
            self.secoes.push(Secao { rotulo: r.rotulo, inicio_s: (t - f64::from(durou)).max(0.0) });
            if self.secoes.len() > MAXIMO_DE_SECOES {
                self.secoes.remove(0);
            }
        }
    }

    /// Coisas do sistema que mudam devagar: o volume (outro app pode ter mexido) e a hora.
    fn atualizar_sistema(&mut self, dt: f32) {
        self.t_volume += dt;
        if self.t_volume >= 0.5 && !self.arrastando_volume {
            self.t_volume = 0.0;
            match audio::volume::ler() {
                Some(n) => {
                    self.volume = n.volume;
                    self.mudo = n.mudo;
                    self.volume_disponivel = true;
                }
                None => self.volume_disponivel = false,
            }
        }
        self.t_hora += dt;
        if self.t_hora >= 1.0 {
            self.t_hora = 0.0;
            self.hora = hora_local();
        }
    }

    fn desenhar_visual(&mut self, window: &mut Window, dt: f32) {
        #[cfg(windows)]
        {
            if self.camada.as_ref().is_some_and(janela::Camada::perdida) {
                self.camada = None;
                self.erro_visual = Some("A placa de vídeo foi reiniciada. Recuperando o visual...".into());
            }
            if self.camada.is_none() && Instant::now() >= self.proxima_camada {
                self.criar_camada(window);
            }
            let escala = window.scale_factor();
            let a = &self.arranjo;
            let cores = self.cores();
            let fundo = tema::tripla(cores.parede);
            let v = a.visual;
            let raio = a.raio_visual * escala;
            let paleta = self.ajustes.paleta_atual();
            let fixas = (!self.ajustes.cores_reativas)
                .then(|| (paleta.a.map(|v| v as f32 / 255.0), paleta.b.map(|v| v as f32 / 255.0)));
            let quadro = visual::Quadro {
                retrato: &self.retrato,
                modelo: self.ajustes.modelo,
                cores: fixas,
                modo_seguro: self.ajustes.modo_seguro,
                cinza: self.ajustes.tema == Tema::Mono,
                dt,
            };
            if let Some(c) = self.camada.as_mut() {
                c.redimensionar((a.janela.w * escala) as u32, (a.janela.h * escala) as u32);
                c.desenhar(fundo, (v.x * escala, v.y * escala, v.w * escala, v.h * escala), raio, &quadro);
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (window, dt);
        }
    }

    /// Guarda o tamanho da janela depois que ele fica parado por um segundo (para reabrir igual).
    fn lembrar_tamanho(&mut self, w: f32, h: f32, especial: bool) {
        if especial {
            return;
        }
        if (w, h) != self.tamanho_visto {
            self.tamanho_visto = (w, h);
            self.tamanho_desde = Instant::now();
        } else if self.tamanho_desde.elapsed() > Duration::from_secs(1)
            && ((self.ajustes.janela_largura - w).abs() > 0.5 || (self.ajustes.janela_altura - h).abs() > 0.5)
        {
            self.ajustes.janela_largura = w;
            self.ajustes.janela_altura = h;
            self.ajustes = self.ajustes.clone().validar();
            self.salvar();
        }
    }

    fn ao_teclar(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        if self.paleta.is_some() {
            self.tecla_na_paleta(ev, window, cx);
            return;
        }
        if self.buscando_musicas && self.pagina == Pagina::Biblioteca {
            self.tecla_na_busca_de_musicas(ev, cx);
            return;
        }
        let m = &ev.keystroke.modifiers;
        if m.alt || m.shift {
            return;
        }
        if let Some(comando) = Comando::do_atalho(ev.keystroke.key.as_str(), m.control || m.platform) {
            self.executar(comando, window, cx);
        }
    }
}

/// A fonte do Modo Escuta para o app escolhido nos ajustes (`""` = todo o som). Com um app escolhido e uma
/// música local tocando, ouve o próprio Player.
fn fonte_de(exe: &str, local_tocando: bool) -> Fonte {
    match (exe.is_empty(), local_tocando) {
        (true, _) => Fonte::Tudo,
        (false, true) => Fonte::Proprio,
        (false, false) => Fonte::App(exe.to_string()),
    }
}

/// O nome do app que o "tocando agora" segue (`""` = o app atual do Windows).
fn filtro_de(exe: &str) -> String {
    if exe.is_empty() { String::new() } else { sessao::nome_do_app(exe) }
}

/// Converte a capa decodificada (BGRA) para a imagem que o GPUI desenha.
fn imagem_da_capa(capa: &Capa) -> Option<Arc<RenderImage>> {
    let buffer = image::RgbaImage::from_raw(capa.largura, capa.altura, capa.bgra.clone())?;
    Some(Arc::new(RenderImage::new(smallvec::SmallVec::from_elem(image::Frame::new(buffer), 1))))
}

/// O identificador que o Windows dá à janela (para a integração com o sistema). `None` fora do Windows.
fn hwnd_da_janela(window: &Window) -> Option<isize> {
    #[cfg(windows)]
    {
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};
        match HasWindowHandle::window_handle(window).map(|h| h.as_raw()) {
            Ok(RawWindowHandle::Win32(hw)) => Some(hw.hwnd.get()),
            _ => None,
        }
    }
    #[cfg(not(windows))]
    {
        let _ = window;
        None
    }
}

/// Hora local do sistema (horas, minutos).
fn hora_local() -> (u8, u8) {
    #[cfg(windows)]
    {
        // SAFETY: função do Windows sem parâmetros, que só devolve a hora atual.
        let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
        ((t.wHour % 24) as u8, (t.wMinute % 60) as u8)
    }
    #[cfg(not(windows))]
    {
        let s = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        (((s / 3600) % 24) as u8, ((s / 60) % 60) as u8)
    }
}

impl Render for Player {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.quadro(window);
        let c = self.cores();
        let a = self.arranjo;
        let mut raiz = div()
            .size_full()
            .relative()
            .font_family(crate::ui::FONTE)
            .text_color(c.texto)
            .text_size(px(14.0))
            .line_height(px(20.0))
            .track_focus(&self.foco)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| this.ao_teclar(ev, window, cx)));
        // Sem a camada de baixo ninguém pinta o fundo: a interface pinta.
        if self.erro_visual.is_some() {
            raiz = raiz.bg(c.parede);
        }
        raiz = raiz.child(self.ver_heroi(cx));
        if !a.so_visual {
            raiz = raiz.child(self.ver_topo(cx)).child(self.ver_dock(cx));
            if let Some(r) = a.trilho {
                raiz = raiz.child(self.ver_trilho(r, cx));
            }
            if let Some(r) = a.painel {
                raiz = raiz.child(self.ver_painel(r, false, cx));
            }
            if let Some(r) = a.gaveta {
                raiz = raiz.child(self.ver_fundo_da_gaveta(cx)).child(self.ver_painel(r, true, cx));
            }
            if let Some(r) = a.nav {
                raiz = raiz.child(self.ver_nav(r, cx));
            }
        }
        raiz = raiz.children(self.ver_aviso());
        if self.paleta.is_some() {
            raiz = raiz.child(self.ver_paleta(cx));
        }
        if let Some(m) = self.ficha.and_then(|i| self.catalogo.get(i)) {
            raiz = raiz.child(self.ver_ficha_do_modulo(m, cx));
        }
        // Por último, por cima de tudo.
        raiz = raiz.children(self.ver_dica());

        // Ouvintes da janela inteira: soltar o botão e mover o ponteiro valem mesmo fora de quem começou o gesto
        // (arrastar o volume, arrastar um módulo, soltar um botão apertado).
        let entidade = cx.entity().downgrade();
        raiz.child(
            canvas(
                |_, _, _| (),
                move |_, (), window, _| {
                    let ao_mover = entidade.clone();
                    window.on_mouse_event(move |ev: &MouseMoveEvent, fase, _, cx| {
                        if fase == DispatchPhase::Bubble {
                            let _ = ao_mover.update(cx, |this, cx| {
                                if this.ao_mover(ev.position) {
                                    cx.notify();
                                }
                            });
                        }
                    });
                    let ao_soltar = entidade.clone();
                    window.on_mouse_event(move |_: &MouseUpEvent, fase, _, cx| {
                        if fase == DispatchPhase::Bubble {
                            let _ = ao_soltar.update(cx, |this, cx| {
                                if this.ao_soltar() {
                                    cx.notify();
                                }
                            });
                        }
                    });
                },
            )
            .absolute()
            .size_0(),
        )
    }
}
