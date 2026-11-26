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
use crate::navegacao::Rota;
use crate::tema::{self, Cores};
use analise::{Retrato, Rotulo};
use gpui::{
    Bounds, Context, DispatchPhase, FocusHandle, KeyDownEvent, MouseMoveEvent, MouseUpEvent, Pixels, Point,
    RenderImage, SharedString, Window, WindowHandle, canvas, div, prelude::*,
};
use nucleo::{Ajustes, Comando, Modelo, Modulo, Pagina, Skin, Tema};
use sessao::{Capa, Controle, Estado, Faixa, Sessao};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

mod letra;
mod loja;
mod menu;
mod midia;
mod navegar;
pub(crate) use menu::AbaDoPainel;
pub(crate) use navegar::ALTURA_DO_COMANDO;
mod salvos;
mod volume;

/// Colunas do medidor de energia do painel (uma a cada meio segundo).
pub const COLUNAS_ENERGIA: usize = 16;
/// Células do mapa de energia do dock.
pub const CELULAS_MAPA: usize = 96;
const MAXIMO_NA_SESSAO: usize = 30;
/// Uma música entra em "Nesta sessão" depois de tocar com som por este tempo (segundos). Sem isso, qualquer app
/// que abre uma sessão de mídia em silêncio, ou uma faixa pulada na hora, enchia a lista.
const SOM_PARA_A_SESSAO: f32 = 4.0;
/// Trechos guardados no módulo Estrutura (os mais antigos saem).
const MAXIMO_DE_SECOES: usize = 48;
/// Um nível de energia precisa durar isto para virar um trecho novo (evita pisca-pisca na fronteira).
const SECAO_MINIMA_S: f32 = 2.0;
const AVISO: Duration = Duration::from_millis(2600);
/// Com "Desfazer", o aviso fica mais tempo: dá para ler e clicar.
const AVISO_COM_DESFAZER: Duration = Duration::from_secs(6);
const PASSO_VOLUME: f32 = 0.05;
/// Quanto a barra de volume estica, no máximo, ao puxar além do fim (em pixels).
const ESTICA_VOLUME: f32 = 6.0;
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

/// O aviso que aparece acima do dock por alguns segundos, às vezes com "Desfazer".
pub struct Aviso {
    pub texto: SharedString,
    pub ate: Instant,
    pub desfazer: Option<Desfazer>,
}

/// O que o "Desfazer" do aviso (ou `Ctrl Z`) devolve.
pub enum Desfazer {
    /// A música tirada da fila volta para a posição em que estava.
    TirarDaFila { indice: usize, musica: Box<biblioteca::Musica> },
    /// As músicas tiradas por "Limpar a seguir" voltam para o fim da fila.
    LimparASeguir(Vec<biblioteca::Musica>),
    /// A pasta tirada volta (e é lida de novo).
    TirarPasta(std::path::PathBuf),
    /// Playlist apagada ou música tirada de playlist: a thread da biblioteca guarda os detalhes.
    NaBiblioteca,
    /// O provedor removido (o serviço) volta.
    RestaurarProvedor(String),
}

/// A janela de pôr músicas numa playlist (ou criar uma nova).
pub struct JanelaPlaylist {
    /// As músicas que vão entrar (do computador ou de um serviço).
    pub musicas: Vec<biblioteca::Musica>,
    /// O nome da playlist nova, enquanto a pessoa escreve.
    pub nome: String,
    /// Só criar (sem músicas) e abrir a playlist nova.
    pub so_criar: bool,
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
    Provedores,
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
    /// Quanto a faixa de agora já tocou com som (segundos), até entrar em "Nesta sessão".
    som_da_faixa: f32,
    pub(crate) retrato: Retrato,
    pub(crate) situacao: Situacao,
    #[cfg(windows)]
    camada: Option<janela::Camada>,
    pub(crate) erro_visual: Option<SharedString>,
    pub(crate) placa: SharedString,
    proxima_camada: Instant,
    /// As páginas abertas: a pilha e o filtro de serviço. Sem página, só o visual (no Palco).
    pub(crate) nav: crate::navegacao::Navegacao,
    /// As capas das páginas dos serviços (baixadas e guardadas em disco).
    pub(crate) capas: crate::capas::Capas,
    /// Curtidas e salvos que a biblioteca ainda não confirmou (para dois cliques rápidos ficarem no último).
    pub(crate) salvos_pendentes: std::collections::HashMap<salvos::ChaveDoSalvo, bool>,
    /// As faixas e os álbuns da página de detalhe aberta, já separados (uma playlist longa não é copiada a cada quadro).
    pub(crate) detalhe_lido: RefCell<Option<salvos::DetalheLido>>,
    /// Já disse, nesta abertura, por onde as músicas de um serviço sem áudio tocam.
    pub(crate) avisou_do_match: bool,
    /// Já disse, nesta abertura, que o serviço tocava em outro app e para ao tocar aqui.
    pub(crate) avisou_que_toma_a_vez: bool,
    /// A biblioteca de cada filtro que a tela mostra (o da página e Tudo, na lateral), já junta.
    pub(crate) biblioteca_lida: RefCell<HashMap<Option<String>, salvos::BibliotecaLida>>,
    /// Os resultados da busca e o Início, juntos uma vez por resposta que chega.
    pub(crate) busca_lida: RefCell<Option<salvos::BuscaLida>>,
    pub(crate) inicio_lido: RefCell<Option<salvos::InicioLido>>,
    /// Um álbum, playlist ou artista para tocar quando as faixas chegarem (o botão de tocar de um cartão).
    pub(crate) tocar_ao_chegar: Option<(crate::remoto::PedidoDePagina, Instant, menu::AoChegar)>,
    /// O menu aberto (do botão direito ou do ⋯).
    pub(crate) menu: Option<menu::MenuAberto>,
    /// O que o painel mostra: os módulos, a fila ou a letra.
    pub(crate) aba_do_painel: AbaDoPainel,
    /// A rolagem da fila no painel (ela abre na música de agora).
    pub(crate) rolagem_da_fila: gpui::UniformListScrollHandle,
    /// O que a pessoa disse que não é a mesma música.
    pub(crate) separadas: crate::juntar::Separadas,
    /// A última música de serviço que entrou nos recentes.
    recente_visto: Option<std::path::PathBuf>,
    /// O que a Biblioteca aberta mostra, já filtrado e em ordem.
    pub(crate) biblioteca_arrumada: RefCell<Option<salvos::Arrumada>>,
    pub(crate) ordem_da_biblioteca: crate::paginas::Ordem,
    /// A Biblioteca em linhas (ou em cartões).
    pub(crate) biblioteca_em_lista: bool,
    /// A aba em que a Biblioteca reabre.
    pub(crate) aba_lembrada: crate::navegacao::AbaDaBiblioteca,
    /// Quantos cartões as grades longas (Biblioteca do serviço, discografia) mostram; cresce no "Ver mais".
    pub(crate) cartoes_a_mostrar: usize,
    /// A aba escolhida na página Buscar e, nos comandos, o que o Enter executa.
    pub(crate) aba_da_busca: crate::paginas::AbaDaBusca,
    pub(crate) comando_escolhido: usize,
    /// A busca foi aberta pelo atalho: um comando escolhido nela devolve a página de antes.
    pub(crate) busca_de_passagem: bool,
    /// A aba aberta na página da faixa.
    pub(crate) aba_da_faixa: crate::paginas::AbaDaFaixa,
    /// A aba aberta na página de um artista.
    pub(crate) aba_do_artista: crate::detalhes::AbaDoArtista,
    /// O passo da navegação cujo item já foi completado nos recentes.
    pub(crate) recente_completo: Option<u64>,
    /// O cartão sob o mouse (a capa dele cresce).
    pub(crate) cartao_sob: Option<usize>,
    /// A página do artista mostra todas as populares.
    pub(crate) ver_mais: bool,
    /// A rolagem de cada prateleira de cartões.
    pub(crate) rolagens: RefCell<HashMap<usize, gpui::ScrollHandle>>,
    /// A aba escolhida em cada fileira de abas, e em que passo: ela só volta à vista quando muda.
    pub(crate) abas_vistas: RefCell<HashMap<&'static str, (u64, usize)>>,
    /// A rolagem de cada passo da navegação (voltar devolve a página onde ela estava), e a das listas de músicas.
    pub(crate) rolagens_das_paginas: RefCell<HashMap<u64, gpui::ScrollHandle>>,
    pub(crate) rolagens_das_listas: RefCell<HashMap<u64, gpui::UniformListScrollHandle>>,
    /// A seção aberta na página Configurações.
    pub(crate) secao_config: crate::ui::configuracoes::Secao,
    pub(crate) gaveta_aberta: bool,
    pub(crate) arranjo: Arranjo,
    anterior: Instant,
    /// Energia recente, uma amostra a cada meio segundo (a mais nova no fim).
    pub(crate) energia_recente: VecDeque<f32>,
    t_energia: f32,
    /// Energia de cada trecho da faixa (ou das últimas células, quando o app não informa a posição).
    /// Negativo = trecho ainda não ouvido.
    pub(crate) mapa: Vec<f32>,
    /// O mapa veio da pré-análise desta análise (o endereço dela): aí ele não é mais preenchido ao vivo.
    mapa_da_analise: Option<usize>,
    /// O modo arquivo do diretor: com a música local pré-analisada, conduz o visual pela linha do tempo.
    condutor: preanalise::condutor::Condutor,
    /// O impulso da forma: a soma do ao vivo (quando é ele que conduz) e do condutor (quando é a análise), para
    /// não dar salto na troca entre os dois.
    impulso_vivo: f64,
    impulso_saida: f64,
    /// Em que tempo do compasso estamos, quando a análise sabe (o módulo Ritmo acende o tempo forte certo).
    compasso_da_analise: Option<u8>,
    t_mapa: f32,
    /// Batida atual do compasso (0 a 3), só para o indicador do painel.
    pub(crate) batida: usize,
    fase_anterior: f32,
    /// Vai a 1 no primeiro tempo do compasso e cai rápido: o pulso do logo e do ponto da energia.
    pub(crate) pulso_compasso: f32,
    pub(crate) tom: [f32; 3],
    pub(crate) aviso: Option<Aviso>,
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
    /// O volume da música mudou há tantos segundos e ainda não foi guardado.
    t_volume_salvo: Option<f32>,
    /// A música do Player está muda (o resto do PC, não).
    mudo_da_musica: bool,
    /// O volume da música que toca por fora do Player.
    mesa: crate::mesa::Mesa,

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
    /// "Efeitos de animação" desligado no Windows: a interface vai direto ao estado final (como a Ilha).
    movimento_reduzido: bool,

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
    /// O "tocando agora" dos outros apps (sem a sessão do próprio Player), lido quando muda.
    outro: Estado,
    /// Tocando no quadro anterior: a música local e o outro app (para saber quem começou por último).
    local_tocava: bool,
    outro_tocava: bool,
    /// Quem tocou por último: a música local (verdadeiro) ou outro app. Desempata quando nada toca.
    ultimo_foi_local: bool,
    /// O que a tela mostra agora é a música local (os controles valem para ela).
    pub(crate) origem_local: bool,
    /// A mesma informação, para a bandeja e os botões da barra de tarefas (que agem fora da interface).
    origem_local_fora: Arc<std::sync::atomic::AtomicBool>,
    /// Os textos dos campos das páginas, e o campo com o teclado (vale enquanto ele está na tela).
    pub(crate) buscas: crate::navegacao::Buscas,
    pub(crate) campo: Option<crate::navegacao::Campo>,
    /// O texto do campo está todo selecionado: a próxima letra troca tudo.
    pub(crate) campo_selecionado: bool,
    /// No campo de texto ativo: o pedaço ainda provisório (a tecla morta esperando a letra), em bytes.
    pub(crate) marcado: Option<std::ops::Range<usize>>,
    /// A janela de playlist aberta (pôr músicas numa playlist ou criar uma).
    pub(crate) janela_playlist: Option<JanelaPlaylist>,
    /// A playlist com o "Apagar?" à mostra, esperando a confirmação.
    pub(crate) apagar_playlist: Option<i64>,
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
    /// A tradução da letra (quando ligada) e o que foi pedido: a chave da música e o idioma.
    pub(crate) traducao: letrista::EstadoDaTraducao,
    pub(crate) traducao_pedida: Option<(String, String)>,
    /// Quando o ponteiro se mexeu pela última vez (os controles da janela mini somem com ele parado).
    pub(crate) ponteiro_em: Instant,
    /// Barra de tarefas, bandeja e janela mini (vazio se o sistema não aceitou a ligação).
    sistema: Option<integracao::Integracao>,
    /// A janela está no modo mini (pequena, no canto, por cima das outras).
    pub(crate) mini: bool,
    /// Estante: o visual na janela inteira, a pedido da pessoa.
    pub(crate) expandido: bool,
    /// Quanto o visual já cresceu (0 ao lado da página, 1 na janela), lido pela tela para esmaecer o resto.
    pub(crate) expansao: f32,
    /// Ninguém mexe há um tempo: a página da Estante sai da frente do visual.
    pub(crate) descanso: crate::descanso::Descanso,
    /// A divisória da Estante está sendo arrastada.
    pub(crate) arrastando_divisao: bool,
    /// No empilhado da Estante, a página do Início foi rolada e o visual virou tira.
    recolhido: bool,
    /// A pessoa anda pela tela com o Tab: o anel de foco aparece, e espaço e Enter são do elemento focado.
    pub(crate) foco_pelo_teclado: bool,
    /// Quando o estado (som, mídia, threads) foi atualizado pela última vez: a Ilha também atualiza, e a guarda
    /// evita rodar duas vezes no mesmo quadro.
    estado_em: Instant,
    /// A janela da Ilha, quando aberta.
    ilha: Option<WindowHandle<crate::ui::ilha::VistaIlha>>,
    abrindo_ilha: bool,
    /// A Ilha está mostrando a letra (então a letra é buscada mesmo sem o módulo Letra no painel).
    pub(crate) letra_pela_ilha: bool,
    /// A letra em tela cheia, sobre o visual.
    pub(crate) letra_cheia: bool,
    /// O tema e a skin vistos por último, e até quando o tom do dock troca rápido depois de uma mudança.
    estilo_visto: (Tema, bool),
    troca_de_estilo_ate: Instant,
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
        let fonte = fonte_de(&ajustes.escuta_app, false, None);
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
            som_da_faixa: 0.0,
            retrato: Retrato::default(),
            situacao: Situacao::Iniciando,
            #[cfg(windows)]
            camada: None,
            erro_visual: None,
            placa: "".into(),
            proxima_camada: agora,
            nav: crate::navegacao::Navegacao::new(crate::navegacao::Espaco::da_chave(&ajustes.espaco)),
            capas: crate::capas::Capas::iniciar(nucleo::caminhos::pasta_dados().ok().map(|d| d.join("capas"))),
            salvos_pendentes: std::collections::HashMap::new(),
            detalhe_lido: RefCell::new(None),
            avisou_do_match: false,
            avisou_que_toma_a_vez: false,
            biblioteca_lida: RefCell::default(),
            busca_lida: RefCell::default(),
            inicio_lido: RefCell::default(),
            tocar_ao_chegar: None,
            menu: None,
            aba_do_painel: AbaDoPainel::default(),
            rolagem_da_fila: gpui::UniformListScrollHandle::new(),
            separadas: crate::juntar::Separadas::abrir(
                nucleo::caminhos::pasta_dados().ok().map(|d| d.join("separadas.json")),
            ),
            recente_visto: None,
            biblioteca_arrumada: RefCell::default(),
            ordem_da_biblioteca: crate::paginas::Ordem::default(),
            biblioteca_em_lista: false,
            aba_lembrada: crate::navegacao::AbaDaBiblioteca::default(),
            cartoes_a_mostrar: crate::ui::CARTOES_POR_VEZ,
            aba_da_busca: crate::paginas::AbaDaBusca::default(),
            comando_escolhido: 0,
            busca_de_passagem: false,
            aba_da_faixa: crate::paginas::AbaDaFaixa::Letra,
            aba_do_artista: Default::default(),
            recente_completo: None,
            cartao_sob: None,
            ver_mais: false,
            rolagens: RefCell::default(),
            abas_vistas: RefCell::default(),
            rolagens_das_paginas: RefCell::default(),
            rolagens_das_listas: RefCell::default(),
            secao_config: Default::default(),
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
            mapa_da_analise: None,
            condutor: preanalise::condutor::Condutor::default(),
            impulso_vivo: 0.0,
            impulso_saida: 0.0,
            compasso_da_analise: None,
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
            t_volume_salvo: None,
            mudo_da_musica: false,
            mesa: crate::mesa::Mesa::iniciar(ajustes.volume_musica),
            secoes: Vec::new(),
            candidata: None,
            silencio_s: 0.0,
            inicio_da_faixa: agora,
            troca_de_faixa: agora,
            hora: hora_local(),
            t_hora: 0.0,
            movimento_reduzido: integracao::ilha::movimento_reduzido(),
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
                ajustes.aleatorio,
                ajustes.repetir,
                ajustes.analisar_biblioteca,
                &ajustes.provedores_desligados,
            ),
            era_local: false,
            outro: Estado::default(),
            local_tocava: false,
            outro_tocava: false,
            ultimo_foi_local: false,
            origem_local: false,
            origem_local_fora: Arc::default(),
            buscas: crate::navegacao::Buscas::default(),
            campo: None,
            campo_selecionado: false,
            marcado: None,
            janela_playlist: None,
            apagar_playlist: None,
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
            expandido: false,
            expansao: 0.0,
            descanso: crate::descanso::Descanso::new(agora),
            arrastando_divisao: false,
            recolhido: false,
            foco_pelo_teclado: false,
            estado_em: agora,
            ilha: None,
            abrindo_ilha: false,
            letra_pela_ilha: false,
            letra_cheia: false,
            traducao: letrista::EstadoDaTraducao::default(),
            traducao_pedida: None,
            estilo_visto: (ajustes.tema, ajustes.skin == Skin::Imersiva),
            troca_de_estilo_ate: agora,
            ajustes,
        };
        player.criar_camada(window);
        player.ligar_ao_sistema(window);
        player.aceitar_letra_ja_escolhida();
        player.atualizar_modulos();
        player.avisar_servicos();
        player.aplicar_reproducao();
        player.aplicar_volume_da_musica();
        player.escuta.ligar_ao_player(player.local.derivacao(), player.mesa.ganho());
        player.manter_o_player_local_andando(cx);
        player.configurar_letras();
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
                let local = self.local.controle();
                let origem_local = self.origem_local_fora.clone();
                // Os botões agem no que a tela mostra: a música local ou o app que está tocando.
                let sistema = integracao::Integracao::instalar(hw.hwnd.get(), move |acao| {
                    if origem_local.load(std::sync::atomic::Ordering::Acquire) {
                        let _ = local.send(match acao {
                            Acao::Anterior => souvlaki::MediaControlEvent::Previous,
                            Acao::TocarPausar => souvlaki::MediaControlEvent::Toggle,
                            Acao::Proxima => souvlaki::MediaControlEvent::Next,
                        });
                    } else {
                        controle.controlar(match acao {
                            Acao::Anterior => Controle::Anterior,
                            Acao::TocarPausar => Controle::TocarPausar,
                            Acao::Proxima => Controle::Proxima,
                        });
                    }
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
            // Na Estante a página fica aberta (a janela mini só não a mostra).
            if self.ajustes.layout != nucleo::Layout::Estante {
                self.nav.fechar();
            }
            self.expandido = false;
            self.gaveta_aberta = false;
            self.menu = None;
            self.paleta = None;
            // Janelas por cima e campos de texto não cabem na janela mini.
            self.janela_playlist = None;
            self.ficha = None;
            self.largar_campo();
            self.apagar_playlist = None;
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
        self.ajustes.skin == Skin::Imersiva && self.erro_visual.is_none() && !self.estante()
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
        self.aviso = Some(Aviso { texto: texto.into(), ate: Instant::now() + AVISO, desfazer: None });
    }

    /// Aviso com o botão "Desfazer" (também vale `Ctrl Z` enquanto ele está na tela).
    pub(crate) fn avisar_com_desfazer(&mut self, texto: impl Into<SharedString>, desfazer: Desfazer) {
        self.aviso =
            Some(Aviso { texto: texto.into(), ate: Instant::now() + AVISO_COM_DESFAZER, desfazer: Some(desfazer) });
    }

    /// Desfaz a ação do aviso que está na tela. Falso se não havia nada para desfazer.
    pub(crate) fn desfazer(&mut self) -> bool {
        let Some(d) = self.aviso.take().and_then(|a| a.desfazer) else { return false };
        match d {
            Desfazer::TirarDaFila { indice, musica } => self.local.fila.inserir(indice, *musica),
            Desfazer::RestaurarProvedor(servico) => self.local.remoto.restaurar(servico),
            Desfazer::LimparASeguir(musicas) => self.local.fila.adicionar(musicas),
            Desfazer::TirarPasta(pasta) => self.local.adicionar_pasta(pasta),
            Desfazer::NaBiblioteca => self.local.desfazer_na_biblioteca(),
        }
        self.avisar("Desfeito");
        true
    }

    pub(crate) fn salvar(&self) {
        self.gravador.salvar(&self.ajustes);
    }

    /// Executa um comando. Botões, atalhos e a barra de comandos passam todos por aqui.
    pub(crate) fn executar(&mut self, comando: Comando, window: &mut Window, cx: &mut Context<Self>) {
        log::debug!("comando {}", comando.id());
        // O botão acabou de ser usado: a dica dele já cumpriu o papel.
        self.dica = None;
        match comando {
            // Com uma música local carregada, os controles valem para ela; senão, para o app que está tocando.
            Comando::TocarPausar if self.origem_local => self.local.alternar(),
            Comando::Proxima if self.origem_local => self.local.proxima(),
            Comando::Anterior if self.origem_local => self.local.anterior(),
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
            Comando::AlternarIlha => {
                self.ajustes.ilha = !self.ajustes.ilha;
                self.avisar(if self.ajustes.ilha {
                    "Ilha ligada: ela aparece no topo da tela enquanto a música toca"
                } else {
                    "Ilha fechada. Para reabrir: tecla D, o botão no dock ou as Configurações"
                });
                self.salvar();
            }
            Comando::AlternarIlhaAoPairar => {
                self.ajustes.ilha_ao_pairar = !self.ajustes.ilha_ao_pairar;
                self.salvar();
            }
            Comando::CentralizarIlha => {
                self.ajustes.ilha_borda = nucleo::BordaIlha::Topo;
                self.ajustes.ilha_posicao = 0.5;
                self.ajustes.ilha_solta = None;
                self.avisar("A Ilha voltou ao centro do topo");
                self.salvar();
            }
            Comando::AlternarTravaIlha => {
                self.ajustes.ilha_travada = !self.ajustes.ilha_travada;
                if self.ajustes.ilha_travada {
                    // Travar uma Ilha solta a devolve à última borda em que ela esteve.
                    self.ajustes.ilha_solta = None;
                    self.avisar("Ilha travada");
                } else {
                    self.avisar("Ilha destravada: arraste para qualquer borda ou monitor");
                }
                self.salvar();
            }
            Comando::PrenderIlha(borda) => {
                self.ajustes.ilha = true;
                self.ajustes.ilha_borda = borda;
                self.ajustes.ilha_posicao = 0.5;
                self.ajustes.ilha_solta = None;
                self.avisar(format!("A Ilha agora fica {}", borda.nome().to_lowercase()));
                self.salvar();
            }
            Comando::IlhaOutroMonitor => {
                // O próximo monitor depois do que a Ilha usa agora (o escolhido ou, sem escolha, o da janela).
                let todos = integracao::ilha::monitores();
                if todos.len() < 2 {
                    self.avisar("Só há um monitor ligado");
                } else {
                    let atual = self
                        .ajustes
                        .ilha_monitor
                        .clone()
                        .or_else(|| hwnd_da_janela(window).and_then(integracao::ilha::monitor).map(|m| m.nome));
                    let i =
                        todos.iter().position(|m| Some(&m.nome) == atual.as_ref()).map_or(0, |i| (i + 1) % todos.len());
                    self.ajustes.ilha = true;
                    self.ajustes.ilha_monitor = Some(todos[i].nome.clone());
                    self.avisar(format!("A Ilha foi para o monitor {}", i + 1));
                    self.salvar();
                }
            }
            Comando::TrocarTamanhoIlha(t) => {
                if self.ajustes.ilha_tamanho != t {
                    self.ajustes.ilha_tamanho = t;
                    self.avisar(format!("Ilha: tamanho {}", t.nome().to_lowercase()));
                    self.salvar();
                }
            }
            Comando::AlternarIlhaAvisos => {
                self.ajustes.ilha_avisos = !self.ajustes.ilha_avisos;
                self.salvar();
            }
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
            Comando::AlternarAleatorio => {
                self.local.alternar_aleatorio();
                self.ajustes.aleatorio = self.local.fila.aleatorio;
                self.salvar();
                self.avisar(if self.ajustes.aleatorio {
                    "Ordem aleatória ligada"
                } else {
                    "Ordem aleatória desligada"
                });
            }
            Comando::AlternarRepetir => {
                self.local.alternar_repetir();
                self.ajustes.repetir = self.local.fila.repetir;
                self.salvar();
                self.avisar(match self.ajustes.repetir {
                    nucleo::Repetir::Nada => "Repetir desligado",
                    nucleo::Repetir::Fila => "Repetindo a fila",
                    nucleo::Repetir::Faixa => "Repetindo a música",
                });
            }
            Comando::AbrirFila => {
                // A janela mini não mostra páginas: volta ao tamanho normal antes.
                if self.mini {
                    self.alternar_mini(window);
                }
                // No painel, ao lado da página; sem lugar para ele, a página da Fila.
                if !self.alternar_no_painel(AbaDoPainel::Fila) {
                    self.abrir_rota(Rota::Fila);
                    self.edicao = false;
                }
            }
            Comando::AbrirLetra => {
                if self.mini {
                    self.alternar_mini(window);
                }
                if !self.alternar_no_painel(AbaDoPainel::Letra) {
                    self.executar(Comando::LetraTelaCheia, window, cx);
                }
            }
            Comando::AlternarDicas => {
                self.ajustes.dicas = !self.ajustes.dicas;
                self.avisar(if self.ajustes.dicas {
                    "Dicas dos botões ligadas"
                } else {
                    "Dicas dos botões desligadas"
                });
                self.salvar();
            }
            Comando::AlternarPainel if self.estante() => self.gaveta_aberta = !self.gaveta_aberta,
            Comando::AlternarPainel => match self.arranjo.modo {
                Modo::Largo => {
                    self.ajustes.painel_aberto = !self.ajustes.painel_aberto;
                    self.salvar();
                }
                Modo::Gaveta => self.gaveta_aberta = !self.gaveta_aberta,
                Modo::Empilhado => {}
            },
            Comando::AbrirAjustes => {
                if self.pagina() == Pagina::Configuracoes {
                    self.fechar_folha();
                } else {
                    self.abrir_rota(Rota::Configuracoes);
                }
                self.tirar_o_painel_da_frente();
                self.edicao = false;
            }
            Comando::AbrirPagina(p) => {
                match p {
                    Pagina::Inicio => self.ir_ao_destino(Rota::Inicio),
                    Pagina::Biblioteca => self.ir_ao_destino(Rota::Biblioteca(self.aba_lembrada)),
                    Pagina::Loja => self.abrir_rota(Rota::Loja),
                    Pagina::Configuracoes => self.abrir_rota(Rota::Configuracoes),
                }
                self.tirar_o_painel_da_frente();
                self.edicao = false;
            }
            Comando::AbrirPaleta => {
                // Só o visual na tela (tela cheia, letra, mini): a barra de comandos por cima. Senão, o campo do topo.
                if self.paleta.is_some() {
                    self.paleta = None;
                } else if self.arranjo.so_visual || self.letra_cheia {
                    self.paleta = Some(Paleta::default());
                } else if self.campo_no_teclado() == Some(crate::navegacao::Campo::Busca) {
                    self.largar_campo();
                } else {
                    self.abrir_busca();
                }
            }
            Comando::Fechar => {
                if self.paleta.is_some() {
                    self.paleta = None;
                } else if self.ficha.is_some() {
                    self.fechar_ficha();
                } else if self.letra_cheia {
                    self.letra_cheia = false;
                } else if self.expandido {
                    self.expandido = false;
                } else if self.mini {
                    self.alternar_mini(window);
                } else if window.is_fullscreen() {
                    window.toggle_fullscreen();
                } else if self.edicao {
                    self.sair_da_edicao();
                } else if self.gaveta_aberta {
                    self.gaveta_aberta = false;
                } else if self.estante() {
                    // Na Estante a página não fecha: o Esc volta uma página.
                    self.andar(true);
                } else if self.pagina() != Pagina::Inicio {
                    self.nav.fechar();
                }
            }
            Comando::TrocarLayout => {
                let novo = match self.ajustes.layout {
                    nucleo::Layout::Palco => nucleo::Layout::Estante,
                    nucleo::Layout::Estante => nucleo::Layout::Palco,
                };
                self.trocar_layout(novo);
            }
            Comando::ExpandirVisual => {
                if self.mini || self.letra_cheia || window.is_fullscreen() {
                    // O visual já está na janela inteira.
                } else if self.estante() {
                    self.expandido = !self.expandido;
                } else {
                    self.avisar("Expandir o visual vale no layout Estante (Ctrl L troca o layout)");
                }
            }
            Comando::AumentarInterface | Comando::DiminuirInterface => {
                let passo: isize = if comando == Comando::AumentarInterface { 1 } else { -1 };
                let tamanhos = nucleo::ajustes::TAMANHOS;
                let agora = tamanhos.iter().position(|t| *t == self.ajustes.tamanho).unwrap_or(1) as isize;
                let novo = tamanhos[(agora + passo).clamp(0, tamanhos.len() as isize - 1) as usize];
                self.mudar_tamanho(novo);
            }
            Comando::AlternarPaginaLarga => {
                if self.estante() && self.arranjo.modo != Modo::Empilhado {
                    self.ajustes.pagina_larga = !self.ajustes.pagina_larga;
                    self.expandido = false;
                    self.tirar_o_painel_da_frente();
                    self.salvar();
                } else if self.estante() {
                    self.avisar("A página já está na largura da janela");
                } else {
                    self.avisar("A página larga vale no layout Estante (Ctrl L troca o layout)");
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
                    self.aba_do_painel = AbaDoPainel::Agora;
                    if !self.estante() {
                        self.nav.fechar();
                    }
                    self.mostrar_painel();
                    self.avisar("Modo edição: arraste um módulo sobre outro para trocar");
                }
            }
            Comando::TrocarLado => {
                self.ajustes.painel_a_esquerda = !self.ajustes.painel_a_esquerda;
                self.salvar();
            }
            Comando::Voltar => {
                self.andar(true);
            }
            Comando::Avancar => {
                self.andar(false);
            }
            Comando::AbrirEspaco(n) => {
                if let Some((e, _)) = self.espacos().get(usize::from(n)).cloned() {
                    self.trocar_espaco(e);
                }
            }
            Comando::CurtirFaixa => self.curtir_a_que_toca(),
            Comando::FixarPagina => self.fixar_a_pagina(),
            Comando::ProximoEspaco => {
                let espacos = self.espacos();
                let i = espacos.iter().position(|(e, _)| e == self.nav.espaco()).map_or(0, |i| (i + 1) % espacos.len());
                if let Some((e, _)) = espacos.get(i).cloned() {
                    self.trocar_espaco(e);
                }
            }
            Comando::LetraTelaCheia => {
                if self.mini {
                    self.alternar_mini(window);
                }
                self.letra_cheia = !self.letra_cheia;
                if self.letra_cheia && !self.modulo_ativo(crate::catalogo::ID_LETRA) {
                    self.avisar("O módulo Letra está desligado na Loja");
                }
                self.paleta = None;
                self.edicao = false;
            }
        }
        cx.notify();
    }

    fn sair_da_edicao(&mut self) {
        self.edicao = false;
        self.arrasto = None;
    }

    fn mostrar_painel(&mut self) {
        if self.estante() {
            self.gaveta_aberta = true;
            return;
        }
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
    /// Passa ao player local os ajustes de reprodução (volume igualado, crossfade, equalizador).
    pub(crate) fn aplicar_reproducao(&mut self) {
        let aj = &self.ajustes;
        self.local.preanalista().analisar_biblioteca(aj.analisar_biblioteca);
        self.local.volume_igualado = aj.volume_igualado;
        self.local.mixagem(aj.crossfade_s, if aj.eq_ligado { aj.eq.clone() } else { Vec::new() });
    }

    /// Muda um ajuste de reprodução, grava e aplica.
    pub(crate) fn mudar_reproducao(&mut self, mudar: impl FnOnce(&mut nucleo::Ajustes)) {
        mudar(&mut self.ajustes);
        self.ajustes = self.ajustes.clone().validar();
        self.salvar();
        self.aplicar_reproducao();
    }

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

    /// Toca a música `i` da lista da Biblioteca; as seguintes viram a fila.
    pub(crate) fn tocar_musica(&mut self, i: usize, caminho: &std::path::Path) {
        self.largar_campo();
        self.local.tocar(i, caminho);
    }

    /// Toca uma música achada pela busca do topo; as outras achadas viram a fila.
    pub(crate) fn tocar_achada(&mut self, caminho: &std::path::Path) {
        let achadas = self.local.lista.achadas.as_ref().clone();
        // Pelo caminho: a lista pode ter mudado entre o desenho e o clique.
        let Some(i) = achadas.iter().position(|m| m.caminho == caminho) else { return };
        self.lembrar_a_busca();
        self.local.tocar_lista(achadas, i);
    }

    /// Com a janela escondida (bandeja) ou minimizada, a interface não desenha, e o player local era atendido
    /// só no desenho: a fila parava no fim da música e as teclas de mídia não respondiam. Este relógio atende o
    /// player local cinco vezes por segundo, desenhando ou não. Atender duas vezes no mesmo quadro não faz mal.
    fn manter_o_player_local_andando(&self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(200)).await;
                if this.update(cx, |this, _| this.local.quadro()).is_err() {
                    break;
                }
            }
        })
        .detach();
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
                    this.avisar("Lendo as músicas da pasta...");
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Escolhe a pasta de um provedor para instalar.
    pub(crate) fn escolher_provedor(&mut self, cx: &mut Context<Self>) {
        let escolha = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Instalar provedor".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(pastas))) = escolha.await {
                let _ = this.update(cx, |this, cx| {
                    for pasta in pastas {
                        this.local.remoto.instalar(pasta);
                    }
                    this.avisar("Instalando o provedor...");
                    cx.notify();
                });
            }
        })
        .detach();
    }

    /// Liga ou desliga um provedor e guarda a escolha.
    pub(crate) fn ligar_provedor(&mut self, servico: &str, ligado: bool) {
        self.local.remoto.ligar(servico, ligado);
        self.ajustes.provedores_desligados = self.local.remoto.desligados();
        self.salvar();
    }

    /// Abre a pasta dos provedores no Explorador (criando-a, se ainda não existe).
    pub(crate) fn abrir_pasta_dos_provedores(&mut self) {
        let Some(pasta) = self.local.remoto.pasta().map(std::path::Path::to_path_buf) else { return };
        if let Err(e) = std::fs::create_dir_all(&pasta) {
            self.avisar(format!("Não deu para criar a pasta dos provedores ({e})"));
            return;
        }
        #[cfg(windows)]
        let aberto = std::process::Command::new("explorer").arg(&pasta).spawn();
        #[cfg(not(windows))]
        let aberto = std::process::Command::new("xdg-open").arg(&pasta).spawn();
        if let Err(e) = aberto {
            log::warn!("não deu para abrir a pasta dos provedores: {e}");
        }
    }

    /// Clique no mapa do dock: vai para aquele ponto da música (só para a música local, que é a que o app toca).
    pub(crate) fn ir_no_mapa(&mut self, x: Pixels) {
        let Some(area) = self.areas.borrow().get("dock-mapa").copied() else { return };
        let largura = f32::from(area.size.width);
        let duracao = self.local.situacao.duracao_s;
        if self.origem_local && largura > 1.0 && duracao > 1.0 {
            let parte = (f32::from(x - area.origin.x) / largura).clamp(0.0, 1.0);
            self.local.ir(duracao * f64::from(parte));
        }
    }

    // ------------------------------------------------------------------ módulos da Loja

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
        if self.nav.aba() == crate::navegacao::AbaDaBiblioteca::Historico {
            self.t_relatorio += dt;
            if self.t_relatorio >= RENOVAR_RESUMO_S {
                self.pedir_relatorio();
            }
        }
    }

    /// A parte do quadro que não depende da janela: o som, a mídia, a origem, a pré-análise e as threads. A Ilha
    /// também chama, para tudo seguir andando com a janela principal minimizada ou na bandeja; a guarda evita rodar
    /// duas vezes no mesmo quadro quando as duas janelas desenham.
    pub(crate) fn atualizar_estado(&mut self) {
        let agora = Instant::now();
        let passado = agora.duration_since(self.estado_em);
        if passado < Duration::from_millis(4) {
            return;
        }
        let dt = passado.as_secs_f32().clamp(0.0, 0.1);
        self.estado_em = agora;

        self.retrato = self.escuta.retrato();
        self.situacao = self.escuta.situacao();
        self.apps_com_som = self.escuta.apps();
        self.local.quadro();
        self.conferir_espaco();
        self.conferir_salvos_pendentes();
        self.conferir_tocar_ao_chegar();
        self.lembrar_o_que_toca();
        self.completar_o_recente();
        self.pedir_paginas_da_rota();
        if let Some(aviso) = self.local.aviso.take() {
            self.avisar(aviso);
        }
        if let Some(aviso) = self.local.remoto.aviso.take() {
            match self.local.remoto.removido.take() {
                Some(servico) => self.avisar_com_desfazer(aviso, Desfazer::RestaurarProvedor(servico)),
                None => self.avisar(aviso),
            }
        }
        let outro_mudou = self.ler_outro();
        let local = self.escolher_origem();
        if std::mem::replace(&mut self.origem_local, local) != local {
            self.ler_volume();
        }
        self.origem_local_fora.store(self.origem_local, std::sync::atomic::Ordering::Release);
        // Com um app escolhido, a música local ainda move o visual: enquanto ela é a origem, a escuta ouve o Player.
        let processo = self.local.processo_externo();
        self.mesa.programa(processo.clone());
        let fonte = fonte_de(&self.ajustes.escuta_app, self.origem_local, processo.as_deref());
        if fonte != self.fonte_pedida {
            self.escuta.escolher(fonte.clone());
            self.fonte_pedida = fonte;
        }
        match self.local.estado_de_midia().filter(|_| self.origem_local) {
            Some(estado) => {
                self.era_local = true;
                self.atualizar_midia(estado);
            }
            None => {
                if self.era_local || outro_mudou {
                    self.era_local = false;
                    self.atualizar_midia(self.outro.clone());
                }
            }
        }
        self.aplicar_preanalise(dt);
        if let Some(s) = &self.sistema {
            s.tocando(self.midia.tocando);
        }
        self.atualizar_medidores(dt);
        self.atualizar_sistema(dt);
        self.atualizar_relatorio(dt);
        self.atualizar_letra(dt);
        if self.aviso.as_ref().is_some_and(|a| agora >= a.ate) {
            self.aviso = None;
        }
    }

    /// Um quadro: lê o que mudou, atualiza os números da tela e desenha o visual.
    fn quadro(&mut self, window: &mut Window) {
        let agora = Instant::now();
        let dt = agora.duration_since(self.anterior).as_secs_f32().clamp(0.0, 0.1);
        self.anterior = agora;
        // Com "Efeitos de animação" desligado no Windows ou o modo seguro, nada desliza: vai direto ao fim.
        self.anim.quadro(dt, self.movimento_reduzido || self.ajustes.modo_seguro);
        self.atualizar_estado();

        let vp = window.viewport_size();
        let (real_w, real_h) = (f32::from(vp.width), f32::from(vp.height));
        // O tamanho da interface: tudo é medido na janela dividida pela escala (e o `px` das telas multiplica de volta).
        let escala = crate::ui::medidas::escala_que_cabe(self.ajustes.tamanho, real_w, real_h);
        crate::ui::medidas::definir_escala(escala);
        let (w, h) = (real_w / escala, real_h / escala);
        let tela_cheia = window.is_fullscreen();
        // A Estante sempre mostra uma página; o visual cresce até a janela numa curva curta.
        let estante = self.estante();
        if estante {
            self.nav.garantir_pagina();
        } else {
            self.expandido = false;
            self.arrastando_divisao = false;
        }
        self.descansar(window, agora);
        // Fora do modo do teclado, o foco é da raiz (um clique num elemento focável o leva para ele).
        if !self.foco_pelo_teclado && !self.foco.is_focused(window) {
            window.focus(&self.foco);
        }
        let so_visual = f32::from(u8::from(self.so_visual_na_estante()));
        let expansao = self.anim.seguir(("estante-expande", 0), so_visual, 0.32);
        // No fim da expansão, o arranjo é o da tela cheia (o herói com o título grande).
        let expandido_de_vez = so_visual > 0.0 && expansao >= 0.999;
        self.expansao = if estante { expansao } else { 0.0 };
        // O painel no lugar da página precisa da largura dela: a página larga espera ele fechar.
        let larga = self.ajustes.pagina_larga && !(self.gaveta_aberta && w < arranjo::PAINEL_AO_LADO);
        let larga_em = self.anim.seguir(("estante-larga", 0), f32::from(u8::from(larga)), 0.32);
        let alta = estante && self.nav.atual() == Some(&Rota::Inicio) && !self.recolher_o_visual();
        let alta = self.anim.seguir(("estante-alta", 0), f32::from(u8::from(alta)), 0.24);
        self.arranjo = arranjo::calcular(
            w,
            h,
            Pedido {
                painel_aberto: self.ajustes.painel_aberto,
                gaveta_aberta: self.gaveta_aberta,
                // A letra em tela cheia usa o visual na janela inteira, como a tela cheia.
                tela_cheia: tela_cheia || self.letra_cheia || expandido_de_vez,
                painel_a_esquerda: self.ajustes.painel_a_esquerda,
                imersivo: self.imersivo(),
                mini: self.mini,
                pagina_cheia: matches!(self.pagina(), Pagina::Biblioteca | Pagina::Loja),
                estante,
                expansao,
                divisao: self.ajustes.divisao,
                arrastando: self.arrastando_divisao,
                larga,
                larga_em,
                alta,
            },
        );
        if self.arranjo.modo != Modo::Gaveta && !estante {
            self.gaveta_aberta = false;
        }
        if self.arranjo.so_visual {
            self.sair_da_edicao();
        }
        self.lembrar_tamanho(real_w, real_h, tela_cheia || self.mini || window.is_maximized());
        self.contar_para_a_sessao(dt, window);
        self.desenhar_visual(window, dt);
        window.request_animation_frame();
    }

    /// Tab e Shift Tab andam pelos elementos da tela; com um deles focado, espaço e Enter são dele (o GPUI clica no soltar
    /// da tecla) e `Esc` volta ao jeito de sempre. Devolve se a tecla foi tratada aqui.
    fn tecla_de_foco(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) -> bool {
        let m = &ev.keystroke.modifiers;
        let tecla = ev.keystroke.key.as_str();
        let janela_por_cima = self.paleta.is_some() || self.janela_playlist.is_some() || self.ficha.is_some();
        if tecla == "tab" && !m.control && !m.alt && !m.platform && !janela_por_cima {
            self.largar_campo();
            self.foco_pelo_teclado = true;
            if m.shift {
                window.focus_prev();
            } else {
                window.focus_next();
            }
            cx.notify();
            return true;
        }
        if !self.foco_pelo_teclado || self.foco.is_focused(window) {
            return false;
        }
        match tecla {
            "space" | "enter" if !m.modified() => true,
            "escape" => {
                self.foco_pelo_teclado = false;
                window.focus(&self.foco);
                cx.notify();
                true
            }
            _ => false,
        }
    }

    /// Troca o tamanho da interface (gravado) e diz qual ficou.
    pub(crate) fn mudar_tamanho(&mut self, tamanho: u16) {
        if self.ajustes.tamanho != tamanho {
            self.ajustes.tamanho = tamanho;
            self.ajustes = self.ajustes.clone().validar();
            self.salvar();
        }
        self.avisar(format!("Tamanho da interface: {}%", self.ajustes.tamanho));
    }

    /// Na Estante, só o visual na janela: a pessoa pediu, ou a página saiu da frente.
    pub(crate) fn so_visual_na_estante(&self) -> bool {
        self.estante() && (self.expandido || self.descanso.dormindo())
    }

    /// A página sai da frente do visual quando ninguém mexe (só com ele e ela à vista) e volta quando alguém mexe.
    fn descansar(&mut self, window: &Window, agora: Instant) {
        let a = &self.arranjo;
        let a_vista = self.estante()
            && !a.so_visual
            && !self.expandido
            && a.visual.w >= 2.0
            && a.painel.is_none()
            && !self.letra_cheia
            && !window.is_fullscreen();
        let ocupado = !self.estante()
            || self.campo_no_teclado().is_some()
            || self.menu.is_some()
            || self.paleta.is_some()
            || self.ficha.is_some()
            || self.janela_playlist.is_some()
            || self.edicao
            || self.arrasto.is_some()
            || self.arrastando_volume
            || self.arrastando_divisao;
        let entrada = crate::descanso::Entrada {
            espera_s: self.ajustes.descanso_s,
            tocando: self.midia.tocando,
            na_frente: window.is_window_active(),
            a_vista,
            ocupado,
        };
        self.descanso.passo(&entrada, agora);
        if self.descanso.dormindo() {
            self.dica = None;
        }
    }

    /// No empilhado da Estante, rolar o Início encolhe o visual até a tira; no alto da página ele volta.
    fn recolher_o_visual(&mut self) -> bool {
        if self.arranjo.modo != Modo::Empilhado {
            self.recolhido = false;
            return false;
        }
        let rolagem = self.rolagem_da_pagina();
        let (rolado, sobra) =
            (-crate::ui::medidas::logico(rolagem.offset().y), crate::ui::medidas::logico(rolagem.max_offset().height));
        let ganho = arranjo::ganho_ao_recolher(&self.arranjo.miolo);
        self.recolhido = arranjo::recolhe(self.recolhido, rolado, sobra, ganho);
        self.recolhido
    }

    /// Abre ou fecha a janela da Ilha conforme o ajuste. Abrir e fechar janelas fica para depois do quadro.
    fn sincronizar_ilha(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // A janela pode ter sido fechada por fora: esquece o identificador.
        if let Some(h) = self.ilha
            && !cx.windows().iter().any(|w| w.window_id() == h.window_id())
        {
            self.ilha = None;
        }
        if self.ajustes.ilha && self.ilha.is_none() && !self.abrindo_ilha {
            let Some(principal) = window.window_handle().downcast::<Player>() else { return };
            self.abrindo_ilha = true;
            let player = cx.entity();
            let hwnd = hwnd_da_janela(window);
            cx.defer(move |cx| {
                let aberta = crate::ui::ilha::VistaIlha::abrir(cx, principal, player.clone(), hwnd);
                player.update(cx, |p, _| {
                    p.ilha = aberta;
                    p.abrindo_ilha = false;
                });
            });
        } else if !self.ajustes.ilha
            && let Some(h) = self.ilha.take()
        {
            cx.defer(move |cx| {
                let _ = h.update(cx, |_, window, _| window.remove_window());
            });
        }
    }

    /// Traz a janela principal para a frente (mesmo escondida na bandeja ou minimizada).
    pub(crate) fn trazer_janela(&self) {
        if let Some(s) = &self.sistema {
            s.trazer();
        }
    }

    fn ao_teclar(&mut self, ev: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        // Com a página fora da frente, qualquer tecla a traz de volta; Esc e X (que fariam outra coisa) só isso.
        if self.descanso.atividade(Instant::now()) {
            cx.notify();
            let m = &ev.keystroke.modifiers;
            let comando = Comando::do_atalho(ev.keystroke.key.as_str(), m.control || m.platform);
            if matches!(comando, Some(Comando::Fechar | Comando::ExpandirVisual)) {
                return;
            }
        }
        if self.menu.is_some() {
            self.tecla_no_menu(ev, window, cx);
            return;
        }
        if self.tecla_de_foco(ev, window, cx) {
            return;
        }
        if self.paleta.is_some() {
            self.tecla_na_paleta(ev, window, cx);
            return;
        }
        if self.janela_playlist.is_some() {
            self.tecla_na_janela_playlist(ev, cx);
            return;
        }
        // Alt ← e Alt → (voltar e avançar) valem também com o teclado no campo de busca.
        let m = &ev.keystroke.modifiers;
        if m.alt && !m.shift && !m.control {
            if let Some(comando) = Comando::do_atalho_alt(ev.keystroke.key.as_str()) {
                self.executar(comando, window, cx);
            }
            return;
        }
        if let Some(campo) = self.campo_no_teclado() {
            self.tecla_no_campo(campo, ev, window, cx);
            return;
        }
        if m.alt || m.shift {
            return;
        }
        if ev.keystroke.key == "z" && (m.control || m.platform) {
            if self.desfazer() {
                cx.notify();
            }
            return;
        }
        if let Some(comando) = Comando::do_atalho(ev.keystroke.key.as_str(), m.control || m.platform) {
            if comando == Comando::AbrirPaleta {
                // A tecla que abre o campo (a barra "/") não pode ser a primeira letra dele.
                cx.stop_propagation();
            }
            self.executar(comando, window, cx);
        }
    }
}

/// A fonte do Modo Escuta para o app escolhido nos ajustes (`""` = todo o som). Com a música do Player, ouve só ela (antes do
/// volume, que assim não muda o visual): a do tocador daqui, ou a do `processo` que a toca (o Soloist).
fn fonte_de(exe: &str, local_tocando: bool, processo: Option<&str>) -> Fonte {
    match (local_tocando, processo) {
        (true, Some(processo)) => Fonte::DoPlayer(processo.to_string()),
        (true, None) => Fonte::Proprio,
        (false, _) if exe.is_empty() => Fonte::Tudo,
        (false, _) => Fonte::App(exe.to_string()),
    }
}

/// A regra de quem a tela mostra, sem estado (testada à parte).
fn origem_e_local(tem_local: bool, tem_outro: bool, local_toca: bool, outro_toca: bool, ultimo_local: bool) -> bool {
    match (tem_local, tem_outro) {
        (true, false) => true,
        (false, _) => false,
        (true, true) => match (local_toca, outro_toca) {
            (true, false) => true,
            (false, true) => false,
            _ => ultimo_local,
        },
    }
}

/// O nome do app que o "tocando agora" segue (`""` = o app atual do Windows).
fn filtro_de(exe: &str) -> String {
    if exe.is_empty() { String::new() } else { sessao::nome_do_app(exe) }
}

/// Converte a capa decodificada (BGRA) para a imagem que o GPUI desenha.
fn imagem_da_capa(capa: &Capa) -> Option<Arc<RenderImage>> {
    let mut buffer = image::RgbaImage::from_raw(capa.largura, capa.altura, capa.bgra.clone())?;
    // A capa de um vídeo é larga: fica o quadrado do meio.
    if let Some((x, y, lado)) = crate::capas::recorte_quadrado(capa.largura, capa.altura) {
        buffer = image::imageops::crop_imm(&buffer, x, y, lado, lado).to_image();
    }
    Some(Arc::new(RenderImage::new(smallvec::SmallVec::from_elem(image::Frame::new(buffer), 1))))
}

/// O identificador que o Windows dá à janela (para a integração com o sistema). `None` fora do Windows.
pub(crate) fn hwnd_da_janela(window: &Window) -> Option<isize> {
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
pub(crate) fn hora_local() -> (u8, u8) {
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
        self.sincronizar_ilha(window, cx);
        let c = self.cores();
        let a = self.arranjo;
        let mut raiz = div()
            .size_full()
            .relative()
            .font_family(crate::ui::FONTE)
            .text_color(c.texto)
            .text_size(crate::ui::medidas::px(14.0))
            .line_height(crate::ui::medidas::px(20.0))
            .track_focus(&self.foco)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| this.ao_teclar(ev, window, cx)));
        // Sem a camada de baixo ninguém pinta o fundo: a interface pinta.
        if self.erro_visual.is_some() {
            raiz = raiz.bg(c.parede);
        }
        let estante = self.estante();
        if self.letra_cheia {
            raiz = raiz.child(self.ver_letra_cheia(cx));
        } else if estante && !a.so_visual {
            raiz = raiz.child(self.ver_estante(cx));
        } else {
            raiz = raiz.child(self.ver_heroi(cx));
        }
        if self.descanso.dormindo() {
            // Por cima de tudo: o cursor some, e nada embaixo recebe o mouse.
            raiz = raiz.child(div().absolute().size_full().occlude().cursor(gpui::CursorStyle::None));
        }
        if !a.so_visual && !estante {
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
        raiz = raiz.children(self.ver_aviso(cx));
        if self.paleta.is_some() {
            raiz = raiz.child(self.ver_paleta(cx));
        }
        if let Some(m) = self.ficha.and_then(|i| self.catalogo.get(i)) {
            raiz = raiz.child(self.ver_ficha_do_modulo(m, cx));
        }
        raiz = raiz.children(self.ver_janela_playlist(cx));
        raiz = raiz.children(self.ver_menu(cx));
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
                    // Com a página fora da frente, o clique e a rodinha só a trazem de volta.
                    let ao_clicar = entidade.clone();
                    window.on_mouse_event(move |_: &gpui::MouseDownEvent, fase, _, cx| {
                        if fase != DispatchPhase::Capture {
                            return;
                        }
                        // O mouse tira do modo do teclado (o foco volta à raiz no quadro seguinte).
                        let acordou = ao_clicar.update(cx, |this, _| {
                            this.foco_pelo_teclado = false;
                            this.descanso.atividade(Instant::now())
                        });
                        if acordou.unwrap_or(false) {
                            cx.stop_propagation();
                        }
                    });
                    let ao_rolar = entidade.clone();
                    window.on_mouse_event(move |_: &gpui::ScrollWheelEvent, fase, _, cx| {
                        if fase == DispatchPhase::Capture
                            && ao_rolar.update(cx, |this, _| this.descanso.atividade(Instant::now())).unwrap_or(false)
                        {
                            cx.stop_propagation();
                        }
                    });
                    // Os botões "voltar" e "avançar" do mouse navegam as páginas, como no navegador.
                    let ao_navegar = entidade.clone();
                    window.on_mouse_event(move |ev: &gpui::MouseDownEvent, fase, window, cx| {
                        if fase != DispatchPhase::Bubble {
                            return;
                        }
                        let comando = match ev.button {
                            gpui::MouseButton::Navigate(gpui::NavigationDirection::Back) => Comando::Voltar,
                            gpui::MouseButton::Navigate(gpui::NavigationDirection::Forward) => Comando::Avancar,
                            _ => return,
                        };
                        let _ = ao_navegar.update(cx, |this, cx| this.executar(comando, window, cx));
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

#[cfg(test)]
mod testes_da_origem {
    use super::{Fonte, fonte_de, origem_e_local};

    #[test]
    fn o_visual_ouve_quem_faz_o_som() {
        // A música do Player é ouvida sozinha: o volume dela não muda o visual, e o resto do PC não entra.
        for exe in ["", "opera.exe"] {
            assert_eq!(fonte_de(exe, true, Some("soloist.exe")), Fonte::DoPlayer("soloist.exe".into()));
            assert_eq!(fonte_de(exe, true, None), Fonte::Proprio);
        }
        assert_eq!(fonte_de("", false, None), Fonte::Tudo);
        assert_eq!(fonte_de("opera.exe", false, Some("soloist.exe")), Fonte::App("opera.exe".into()));
    }

    #[test]
    fn quem_toca_vence_e_o_ultimo_desempata() {
        // Só uma das fontes existe.
        assert!(origem_e_local(true, false, false, false, false));
        assert!(!origem_e_local(false, true, false, false, true));
        assert!(!origem_e_local(false, false, false, false, true));
        // Música local pausada e o Spotify tocando: a tela vai para o Spotify.
        assert!(!origem_e_local(true, true, false, true, true));
        // Música local tocando e o outro pausado: a local.
        assert!(origem_e_local(true, true, true, false, false));
        // Os dois tocando, ou os dois parados: quem tocou por último.
        assert!(origem_e_local(true, true, true, true, true));
        assert!(!origem_e_local(true, true, true, true, false));
        assert!(origem_e_local(true, true, false, false, true));
    }
}
