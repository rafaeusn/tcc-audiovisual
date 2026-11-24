//! O Soloist ligado: a thread que o abre, conversa com a API local dele, acompanha a faixa pedida e o reabre se cair.
//! Quem chama só manda e lê o estado; nada aqui espera a rede.

use super::instalar::EXE;
use super::leitura::{Leitura, Pedida, comando};
use super::programa::{APARELHO, Aberto, Falha, Programa};
use provedores::{Acao, EstadoDoTocador, SituacaoDaFaixa, SituacaoDoTocador};
use serde_json::{Value, json};
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tungstenite::{Message, WebSocket};

/// Quanto cada leitura da API local espera (é também o passo do laço).
const PASSO: Duration = Duration::from_millis(50);
const PRAZO_DA_API: Duration = Duration::from_secs(5);
/// Quanto a sessão guardada tem para voltar antes de se pedir para parear (já pareado uma vez, e nunca pareado).
const SESSAO_VOLTA: Duration = Duration::from_secs(8);
const SEM_SESSAO: Duration = Duration::from_millis(400);
/// As esperas antes de cada reabertura seguida.
const REABRIR: [Duration; 3] = [Duration::from_secs(2), Duration::from_secs(10), Duration::from_secs(30)];
/// Depois de quanto tempo de pé uma queda não conta mais como seguida.
const DE_PE: Duration = Duration::from_secs(120);
/// Depois de parar (ou de uma faixa que falhou), quanto tempo calar o que começar atrasado, e de quanto em quanto tempo.
const CALAR: Duration = Duration::from_secs(20);
/// Depois de quanto tempo tocando um Soloist novo é dado por bom neste computador.
const TOCOU_BEM: Duration = Duration::from_secs(20);
/// A API caiu: quanto esperar para ver se foi o programa que fechou, e com que código.
const ESPERA_DO_CODIGO: Duration = Duration::from_secs(3);
const ENTRE_PAUSAS: Duration = Duration::from_millis(500);

enum Ordem {
    /// Com a vez da faixa que o Player pediu por último.
    Fazer(Acao, u64),
    Fechar,
}

fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn agora_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64
}

struct Ligado {
    ordens: Sender<Ordem>,
    /// A pessoa desistiu: o que estiver demorando volta.
    parar: Arc<AtomicBool>,
    fim: JoinHandle<()>,
}

pub struct Soloist {
    programa: Arc<dyn Programa>,
    /// Onde ficam a chave cifrada e a marca de "ligado".
    pasta: PathBuf,
    estado: Arc<Mutex<EstadoDoTocador>>,
    ligado: Option<Ligado>,
    vez: u64,
}

impl Soloist {
    /// Ligado da última vez, já abre.
    pub fn novo(programa: Arc<dyn Programa>, pasta: PathBuf) -> Self {
        let mut s = Self { programa, pasta, estado: Arc::default(), ligado: None, vez: 0 };
        if s.marca().is_file()
            && let Err(e) = s.ligar(None)
        {
            travar(&s.estado).erro = Some(e);
        }
        s
    }

    fn cofre(&self) -> PathBuf {
        self.pasta.join("soloist.dpapi")
    }

    fn marca(&self) -> PathBuf {
        self.pasta.join("soloist.ligado")
    }

    /// Já foi pareado uma vez.
    fn pareado(&self) -> PathBuf {
        self.pasta.join("soloist.pareado")
    }

    pub fn estado(&self) -> EstadoDoTocador {
        EstadoDoTocador { tem_chave: self.cofre().is_file(), ..travar(&self.estado).clone() }
    }

    /// Liga com a chave colada (que fica guardada) ou com a guardada. Abrir, baixar e parear continuam na thread.
    pub fn ligar(&mut self, chave: Option<&str>) -> Result<(), String> {
        let nova = chave.map(str::trim).filter(|c| !c.is_empty());
        let (chave, guardar) = match nova {
            Some(c) => {
                if c.len() > 4096 || !c.bytes().all(|b| b.is_ascii_graphic()) {
                    return Err("o texto copiado não parece a chave do Soloist".into());
                }
                // Só fica guardada quando o Soloist abre com ela: um texto errado não apaga a boa.
                (c.to_string(), Some(self.cofre()))
            }
            None => (servicos::cofre::ler(&self.cofre()).ok_or("cole a chave do Soloist primeiro")?, None),
        };
        self.fechar();
        let _ = std::fs::create_dir_all(&self.pasta);
        if let Err(e) = std::fs::write(self.marca(), "") {
            eprintln!("soloist: ligado só desta vez ({e})");
        }
        *travar(&self.estado) =
            EstadoDoTocador { situacao: SituacaoDoTocador::Abrindo, nome: Some(APARELHO.into()), ..Default::default() };
        let (ordens, chegam) = channel();
        let parar = Arc::new(AtomicBool::new(false));
        let laco = Laco {
            parar: parar.clone(),
            marca: self.marca(),
            guardar,
            programa: self.programa.clone(),
            estado: self.estado.clone(),
            chave,
            ordens: chegam,
            pareado: self.pareado(),
        };
        let fim = std::thread::Builder::new().name("soloist".into()).spawn(move || laco.rodar());
        let fim = fim.map_err(|e| format!("o Soloist não abriu: {e}"))?;
        self.ligado = Some(Ligado { ordens, parar, fim });
        Ok(())
    }

    /// Desliga (a chave e o pareamento ficam para a próxima vez).
    pub fn desligar(&mut self) {
        let _ = std::fs::remove_file(self.marca());
        self.fechar();
    }

    /// Fecha o programa; na próxima abertura do Player ele volta, se estava ligado.
    pub fn fechar(&mut self) {
        if let Some(l) = self.ligado.take() {
            l.parar.store(true, Ordering::Release);
            let _ = l.ordens.send(Ordem::Fechar);
            let _ = l.fim.join();
        }
        *travar(&self.estado) = EstadoDoTocador::default();
    }

    pub fn controlar(&mut self, acao: Acao) -> Result<(), String> {
        let ligado = self.ligado.as_ref().ok_or("o Soloist está desligado")?;
        if let Acao::Tocar { faixa } = &acao {
            // Na hora: quem perguntar logo depois já vê esta faixa, e não o fim da anterior.
            self.vez += 1;
            let tocando = provedores::Tocando { faixa: faixa.clone(), vez: self.vez, ..Default::default() };
            travar(&self.estado).tocando = Some(tocando);
        }
        ligado.ordens.send(Ordem::Fazer(acao, self.vez)).map_err(|_| "o Soloist parou".to_string())
    }
}

impl Drop for Soloist {
    fn drop(&mut self) {
        self.fechar();
    }
}

/// A conversa com a API local.
struct Api(WebSocket<TcpStream>);

impl Api {
    fn conectar(porta: u16) -> Result<Self, String> {
        let falha = |e: &dyn std::fmt::Display| format!("a API do Soloist não respondeu: {e}");
        let endereco = SocketAddr::from((Ipv4Addr::LOCALHOST, porta));
        let fio = TcpStream::connect_timeout(&endereco, PRAZO_DA_API).map_err(|e| falha(&e))?;
        fio.set_read_timeout(Some(PRAZO_DA_API)).map_err(|e| falha(&e))?;
        fio.set_write_timeout(Some(PRAZO_DA_API)).map_err(|e| falha(&e))?;
        let limites = tungstenite::protocol::WebSocketConfig::default()
            .max_message_size(Some(4 * 1024 * 1024))
            .max_frame_size(Some(4 * 1024 * 1024));
        let (ws, _) = tungstenite::client::client_with_config(format!("ws://127.0.0.1:{porta}"), fio, Some(limites))
            .map_err(|e| falha(&e))?;
        ws.get_ref().set_read_timeout(Some(PASSO)).map_err(|e| falha(&e))?;
        Ok(Self(ws))
    }

    fn mandar(&mut self, v: &Value) -> Result<(), String> {
        self.0.send(Message::text(v.to_string())).map_err(|e| e.to_string())
    }

    /// Uma mensagem, ou `None` se nada chegou no passo.
    fn ler(&mut self) -> Result<Option<Value>, String> {
        use std::io::ErrorKind::{TimedOut, WouldBlock};
        match self.0.read() {
            Ok(Message::Text(t)) => Ok(Some(serde_json::from_str(&t).unwrap_or(Value::Null))),
            Ok(Message::Close(_)) => Err("a API do Soloist fechou".into()),
            Ok(_) => Ok(None),
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), TimedOut | WouldBlock) => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// O programa fechou: com que código, se fechar logo.
fn codigo_da_saida(aberto: &mut dyn Aberto) -> Option<i32> {
    let ate = Instant::now() + ESPERA_DO_CODIGO;
    loop {
        if let Some(codigo) = aberto.saiu() {
            return Some(codigo);
        }
        if Instant::now() >= ate {
            return None;
        }
        std::thread::sleep(PASSO);
    }
}

fn fim_com(codigo: i32) -> Fim {
    if codigo == 10 { Fim::Venceu } else { Fim::Fechou(codigo) }
}

/// O código de saída como o Windows o mostra (um erro do programa vem em hexadecimal).
fn nome_do_codigo(codigo: i32) -> String {
    if codigo < 0 { format!("0x{:08x}", codigo as u32) } else { codigo.to_string() }
}

/// Por que o laço de um Soloist aberto acabou.
enum Fim {
    Fechar,
    Venceu,
    /// O programa fechou sozinho, com este código.
    Fechou(i32),
    Caiu(String),
}

struct Laco {
    programa: Arc<dyn Programa>,
    estado: Arc<Mutex<EstadoDoTocador>>,
    chave: String,
    ordens: Receiver<Ordem>,
    pareado: PathBuf,
    marca: PathBuf,
    /// Onde guardar a chave (nova) quando o Soloist abrir com ela.
    guardar: Option<PathBuf>,
    parar: Arc<AtomicBool>,
}

/// O que o laço sabe de um Soloist aberto.
#[derive(Default)]
struct Aberta {
    leitura: Leitura,
    pedida: Option<Pedida>,
    /// Até quando calar o que soar sem ter sido pedido, e a última pausa mandada por isso.
    calar_ate: Option<Instant>,
    pausou: Option<Instant>,
}

impl Aberta {
    fn calar(&mut self) {
        self.calar_ate = Some(Instant::now() + CALAR);
        self.pausou = None;
    }

    /// O que soa sem ter sido pedido (uma faixa que começou depois de a pessoa desistir dela) é pausado.
    fn pausa_devida(&mut self) -> Option<Value> {
        let agora = Instant::now();
        self.calar_ate = self.calar_ate.filter(|ate| agora < *ate);
        self.calar_ate?;
        let vale = self.leitura.soando() && self.pausou.is_none_or(|p| agora.duration_since(p) >= ENTRE_PAUSAS);
        vale.then(|| {
            self.pausou = Some(agora);
            comando("pause")
        })
    }
}

impl Laco {
    fn situar(&self, situacao: SituacaoDoTocador, erro: Option<String>) {
        let mut e = travar(&self.estado);
        (e.situacao, e.erro, e.progresso) = (situacao, erro, None);
        e.processo = (situacao == SituacaoDoTocador::Pronto).then(|| EXE.to_string());
    }

    /// Falhou de vez: o que tocava acaba com o motivo, e as ordens seguintes também, até desligarem.
    fn desistir(&self, motivo: String) {
        eprintln!("soloist: {motivo}");
        if motivo.contains(super::BLOQUEADO) {
            // Não volta sozinho na próxima abertura: baixaria tudo de novo para ser bloqueado de novo.
            let _ = std::fs::remove_file(&self.marca);
        }
        self.faixa_falhou(&motivo);
        self.situar(SituacaoDoTocador::Falhou, Some(motivo.clone()));
        while let Ok(Ordem::Fazer(..)) = self.ordens.recv() {
            self.faixa_falhou(&motivo);
        }
    }

    fn faixa_falhou(&self, motivo: &str) {
        if let Some(t) = travar(&self.estado).tocando.as_mut()
            && matches!(t.situacao, SituacaoDaFaixa::Carregando | SituacaoDaFaixa::Tocando | SituacaoDaFaixa::Pausada)
        {
            (t.situacao, t.erro) = (SituacaoDaFaixa::Falhou, Some(motivo.to_string()));
        }
    }

    fn rodar(self) {
        let mut guardada: Option<(Acao, u64)> = None;
        let (mut quedas, mut venceu) = (0, false);
        loop {
            // (Só está baixando se o download começou: quase sempre o programa já está aí.)
            let estado = self.estado.clone();
            let progresso = move |p: f32| {
                let mut e = travar(&estado);
                (e.situacao, e.progresso) = (SituacaoDoTocador::Baixando, Some(p));
            };
            if let Err(e) = self.programa.preparar(venceu, &self.parar, &progresso) {
                return self.desistir(e);
            }
            self.situar(SituacaoDoTocador::Abrindo, None);
            let aberto_em = Instant::now();
            let fim = match self.programa.abrir(&self.chave, &self.parar) {
                Ok(mut aberto) => {
                    if let Some(cofre) = &self.guardar
                        && servicos::cofre::guardar(cofre, &self.chave).is_err()
                    {
                        eprintln!("soloist: a chave vale só desta vez (não deu para guardar)");
                    }
                    self.acompanhar(&mut *aberto, &mut guardada)
                }
                Err(Falha::Venceu) => Fim::Venceu,
                Err(Falha::Motivo(m)) => return self.desistir(m),
            };
            if aberto_em.elapsed() >= DE_PE {
                (quedas, venceu) = (0, false);
            }
            match fim {
                Fim::Fechar => return,
                Fim::Venceu if venceu => {
                    return self.desistir("o Soloist venceu, e o Spotify ainda não publicou outro".into());
                }
                Fim::Venceu => venceu = true,
                // Um que só cai não é aberto de novo: o Match toca, e outro é procurado uma vez por dia.
                Fim::Fechou(_) if self.programa.caiu() => return self.desistir(super::QUEBRADO.into()),
                Fim::Caiu(_) | Fim::Fechou(_) => {
                    let motivo = match fim {
                        Fim::Fechou(codigo) => format!("o Soloist caiu ({})", nome_do_codigo(codigo)),
                        Fim::Caiu(motivo) => motivo,
                        _ => String::new(),
                    };
                    self.faixa_falhou(&motivo);
                    guardada = None;
                    let Some(espera) = REABRIR.get(quedas) else { return self.desistir(motivo) };
                    quedas += 1;
                    eprintln!("soloist: {motivo}; abrindo de novo ({quedas}/{})", REABRIR.len());
                    // Parado até reabrir: enquanto isso, quem toca é o Match.
                    self.situar(SituacaoDoTocador::Falhou, Some(motivo));
                    if self.esperar(*espera, &mut guardada) {
                        return;
                    }
                }
            }
        }
    }

    /// Espera para reabrir, guardando a última faixa pedida. `true`: é para fechar.
    fn esperar(&self, quanto: Duration, guardada: &mut Option<(Acao, u64)>) -> bool {
        let ate = Instant::now() + quanto;
        loop {
            match self.ordens.recv_timeout(ate.saturating_duration_since(Instant::now())) {
                Ok(Ordem::Fazer(a, vez)) => guardar(guardada, a, vez),
                Ok(Ordem::Fechar) | Err(RecvTimeoutError::Disconnected) => return true,
                Err(RecvTimeoutError::Timeout) => return false,
            }
        }
    }

    fn acompanhar(&self, aberto: &mut dyn Aberto, guardada: &mut Option<(Acao, u64)>) -> Fim {
        let mut api = match Api::conectar(aberto.porta()) {
            Ok(a) => a,
            Err(e) => return Fim::Caiu(e),
        };
        let mut a = Aberta::default();
        let mut reconectou: Option<Instant> = None;
        let (mut tocando_desde, mut tocou_bem) = (None::<Instant>, false);
        // Uma sessão já guardada leva uns segundos para voltar: até lá ele está abrindo, e não pedindo para parear.
        let volta = if self.pareado.is_file() { SESSAO_VOLTA } else { SEM_SESSAO };
        let mut esperar_ate = Some(Instant::now() + volta);
        loop {
            if let Some(codigo) = aberto.saiu() {
                return fim_com(codigo);
            }
            let mut mandar: Vec<Value> = Vec::new();
            loop {
                match self.ordens.try_recv() {
                    Ok(Ordem::Fazer(acao, vez)) if a.leitura.dentro => self.fazer(acao, vez, &mut a, &mut mandar),
                    // Ainda sem conta: a faixa espera a sessão voltar (ou a pessoa parear).
                    Ok(Ordem::Fazer(acao, vez)) => guardar(guardada, acao, vez),
                    Ok(Ordem::Fechar) | Err(TryRecvError::Disconnected) => return Fim::Fechar,
                    Err(TryRecvError::Empty) => break,
                }
            }
            if a.leitura.dentro
                && let Some((acao, vez)) = guardada.take()
            {
                self.fazer(acao, vez, &mut a, &mut mandar);
            }
            if let Some(p) = a.pedida.as_mut() {
                let antes = p.acabou();
                mandar.extend(p.avaliar(&mut a.leitura, agora_ms()));
                let falhou_agora = !antes && p.falhou_mesmo();
                let publicar = p.publicar(&a.leitura);
                if publicar.situacao == SituacaoDaFaixa::Tocando {
                    let desde = *tocando_desde.get_or_insert_with(Instant::now);
                    if !tocou_bem && desde.elapsed() >= TOCOU_BEM {
                        tocou_bem = true;
                        self.programa.funcionou();
                    }
                } else {
                    tocando_desde = None;
                }
                if falhou_agora {
                    // Ela ainda pode começar atrasada, por baixo do que o Player já toca no lugar.
                    a.calar();
                }
                let mut e = travar(&self.estado);
                // Uma faixa pedida agora mesmo (a ordem ainda no canal) não volta para a de antes.
                if e.tocando.as_ref().is_none_or(|t| t.vez <= publicar.vez) {
                    e.tocando = Some(publicar);
                }
            }
            mandar.extend(a.pausa_devida());
            let trocou = mandar.iter().try_for_each(|v| api.mandar(v)).and_then(|()| api.ler());
            match trocou {
                Ok(Some(v)) => a.leitura.receber(&v),
                Ok(None) => {}
                // A API caiu com o programa de pé: uma reconexão; caindo de novo logo, reabre tudo.
                Err(e) => {
                    if let Some(codigo) = codigo_da_saida(aberto) {
                        return fim_com(codigo);
                    }
                    if reconectou.is_some_and(|r| r.elapsed() < Duration::from_secs(30)) {
                        return Fim::Caiu(e);
                    }
                    reconectou = Some(Instant::now());
                    match Api::conectar(aberto.porta()) {
                        Ok(nova) => api = nova,
                        Err(e) => return Fim::Caiu(e),
                    }
                }
            }
            let situacao = match esperar_ate {
                _ if a.leitura.dentro => SituacaoDoTocador::Pronto,
                Some(ate) if Instant::now() < ate => SituacaoDoTocador::Abrindo,
                _ => SituacaoDoTocador::Pareando,
            };
            if situacao == SituacaoDoTocador::Pronto && esperar_ate.take().is_some() {
                let _ = std::fs::write(&self.pareado, "");
                // Parear começa uma música pelo celular: o Player é quem decide o que toca.
                if a.pedida.is_none() {
                    a.calar();
                }
                // Sem repetir nem misturar: a fila é do Player. (Uma vez só: uma recusa aqui não é da faixa.)
                for nome in ["set_repeat_track", "set_repeat_context", "set_shuffle"] {
                    let mut c = comando(nome);
                    c["enabled"] = json!(false);
                    // (Se a API caiu, o próximo laço percebe.)
                    let _ = api.mandar(&c);
                }
            }
            if travar(&self.estado).situacao != situacao {
                if situacao == SituacaoDoTocador::Pareando {
                    self.faixa_falhou("o Soloist não está pareado com o Spotify");
                    a.pedida = None;
                }
                self.situar(situacao, None);
            }
        }
    }

    fn fazer(&self, acao: Acao, vez: u64, a: &mut Aberta, mandar: &mut Vec<Value>) {
        match acao {
            Acao::Tocar { faixa } => {
                let (nova, tocar) = Pedida::nova(&faixa, vez, &mut a.leitura, agora_ms());
                mandar.extend(tocar);
                a.pedida = Some(nova);
                a.calar_ate = None;
            }
            Acao::Pausar => mandar.extend(a.pedida.as_mut().and_then(Pedida::pausar)),
            Acao::Continuar => {
                // Tocava em outro aparelho: pede a faixa de novo, de onde parou.
                let parada = a.pedida.as_ref().filter(|p| p.em_outro_lugar());
                match parada.map(|p| (p.faixa().to_string(), p.posicao_s(), p.vez())) {
                    Some((faixa, posicao, vez)) => {
                        self.fazer(Acao::Tocar { faixa }, vez, a, mandar);
                        if let Some(p) = a.pedida.as_mut() {
                            p.ir_para_s = Some(posicao);
                        }
                    }
                    None => mandar.extend(a.pedida.as_mut().and_then(Pedida::continuar)),
                }
            }
            Acao::Ir { segundos } => mandar.extend(a.pedida.as_mut().and_then(|p| p.ir(segundos))),
            Acao::Parar => {
                // Pela leitura, e não pelo que se esperava da faixa: ela pode começar depois de a pessoa desistir.
                if a.pedida.take().is_some() {
                    a.calar();
                }
                let mut e = travar(&self.estado);
                if e.tocando.as_ref().is_some_and(|t| t.vez <= vez) {
                    e.tocando = None;
                }
            }
        }
    }
}

/// Com o Soloist ainda abrindo, só a última faixa pedida importa.
fn guardar(guardada: &mut Option<(Acao, u64)>, acao: Acao, vez: u64) {
    match acao {
        Acao::Tocar { .. } => *guardada = Some((acao, vez)),
        Acao::Parar => *guardada = None,
        _ => {}
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use std::net::TcpListener;
    use std::sync::atomic::AtomicU32;

    const FAIXA: &str = "2JRo0gjbX4GrCqBYdRohoo";

    /// Um Soloist de mentira: a API local de verdade (WebSocket), que toca o que mandam.
    #[derive(Default)]
    struct Falso {
        /// Não há conta nele.
        fora: Arc<AtomicBool>,
        recebidos: Arc<Mutex<Vec<Value>>>,
        aberturas: AtomicU32,
        chaves: Mutex<Vec<String>>,
        /// As próximas aberturas falham assim.
        falhas: Mutex<Vec<Falha>>,
        preparos: Mutex<Vec<bool>>,
        /// Fecha o programa aberto com este código.
        sair: Arc<Mutex<Option<i32>>>,
        /// A faixa só começa este tempo depois do `play` (ms).
        demora: Arc<AtomicU32>,
        /// O que soa agora no Soloist de mentira.
        soando: Arc<AtomicBool>,
        comecos: Arc<AtomicU32>,
        /// Recusa os comandos de repetir e misturar.
        recusa_os_set: Arc<AtomicBool>,
        baixando_sem_fim: AtomicBool,
        /// Nunca funcionou: na segunda queda é um quebrado.
        novo: AtomicBool,
        quedas: AtomicU32,
    }

    struct AbertoFalso {
        porta: u16,
        sair: Arc<Mutex<Option<i32>>>,
    }

    impl Aberto for AbertoFalso {
        fn porta(&self) -> u16 {
            self.porta
        }
        fn saiu(&mut self) -> Option<i32> {
            travar(&self.sair).take()
        }
    }

    impl Programa for Falso {
        fn caiu(&self) -> bool {
            self.quedas.fetch_add(1, Ordering::SeqCst) >= 1 && self.novo.load(Ordering::SeqCst)
        }

        fn funcionou(&self) {}

        fn preparar(&self, de_novo: bool, parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<(), String> {
            travar(&self.preparos).push(de_novo);
            progresso(0.5);
            while self.baixando_sem_fim.load(Ordering::SeqCst) && !parar.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(10));
            }
            if parar.load(Ordering::SeqCst) { Err("cancelado".into()) } else { Ok(()) }
        }

        fn abrir(&self, chave: &str, _: &AtomicBool) -> Result<Box<dyn Aberto>, Falha> {
            self.aberturas.fetch_add(1, Ordering::SeqCst);
            travar(&self.chaves).push(chave.to_string());
            let falha = travar(&self.falhas).pop();
            if let Some(f) = falha {
                return Err(f);
            }
            let escuta = TcpListener::bind("127.0.0.1:0").expect("porta");
            let porta = escuta.local_addr().expect("endereço").port();
            let (fora, recebidos) = (self.fora.clone(), self.recebidos.clone());
            let (demora, soando, recusa) = (self.demora.clone(), self.soando.clone(), self.recusa_os_set.clone());
            let comecos = self.comecos.clone();
            std::thread::spawn(move || {
                let Ok((fio, _)) = escuta.accept() else { return };
                let Ok(mut ws) = tungstenite::accept(fio) else { return };
                let mut entrou = !fora.load(Ordering::SeqCst);
                let auth = |dentro: bool| json!({"type":"auth_state","logged_in":dentro,"is_active":false}).to_string();
                let _ = ws.send(Message::text(auth(entrou)));
                let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(20)));
                let mut comecar: Option<(Instant, Value)> = None;
                loop {
                    if let Some((quando, uri)) = comecar.take() {
                        if Instant::now() < quando {
                            comecar = Some((quando, uri));
                        } else {
                            soando.store(true, Ordering::SeqCst);
                            comecos.fetch_add(1, Ordering::SeqCst);
                            let estado = json!({
                                "type":"playback_state","status":"playing","is_active":true,
                                "item":{"uri":uri,"decorations":{"playback":{"duration_ms":200_000}}},
                                "position":{"position_ms":1000,"timestamp_ms":agora_ms(),"speed":1.0}});
                            let _ = ws.send(Message::text(estado.to_string()));
                        }
                    }
                    let agora_dentro = !fora.load(Ordering::SeqCst);
                    if agora_dentro != entrou {
                        entrou = agora_dentro;
                        let _ = ws.send(Message::text(auth(entrou)));
                    }
                    match ws.read() {
                        Ok(Message::Text(t)) => {
                            let v: Value = serde_json::from_str(&t).expect("json");
                            if v["command"] == "play" && v["uri"].is_string() {
                                let quando =
                                    Instant::now() + Duration::from_millis(demora.load(Ordering::SeqCst).into());
                                comecar = Some((quando, v["uri"].clone()));
                            }
                            let nome = v["command"].as_str().unwrap_or_default();
                            if nome == "pause" && soando.swap(false, Ordering::SeqCst) {
                                let pausou = json!({"type":"playback_changed","status":"paused"});
                                let _ = ws.send(Message::text(pausou.to_string()));
                            }
                            if nome.starts_with("set_") && recusa.load(Ordering::SeqCst) {
                                let _ = ws.send(Message::text(json!({"type":"error","message":"no"}).to_string()));
                            }
                            travar(&recebidos).push(v);
                        }
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(e))
                            if matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock) => {}
                        Err(_) => return,
                    }
                }
            });
            Ok(Box::new(AbertoFalso { porta, sair: self.sair.clone() }))
        }
    }

    fn esperar(mut f: impl FnMut() -> bool) -> bool {
        let ate = Instant::now() + Duration::from_secs(8);
        while Instant::now() < ate {
            if f() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    fn pasta(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("tccplayer-motor-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("pasta");
        p
    }

    fn situacao(s: &Soloist) -> SituacaoDoTocador {
        s.estado().situacao
    }

    fn da_faixa(s: &Soloist) -> Option<SituacaoDaFaixa> {
        s.estado().tocando.map(|t| t.situacao)
    }

    #[test]
    fn liga_toca_pausa_e_volta_sozinho_na_proxima_abertura() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("toca"));
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        assert_eq!(situacao(&s), SituacaoDoTocador::Desligado);
        assert!(s.ligar(None).expect_err("sem chave").contains("cole a chave"));
        assert!(s.ligar(Some("duas palavras")).is_err());
        assert!(s.controlar(Acao::Pausar).is_err(), "desligado não toca");
        s.ligar(Some("  chave123\n")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        assert_eq!(*travar(&falso.chaves), ["chave123"]);
        let e = s.estado();
        assert!(e.tem_chave && e.processo.as_deref() == Some("soloist.exe"));
        assert!(!format!("{e:?}").contains("chave123"));
        let bruto = std::fs::read(pasta.join("soloist.dpapi")).expect("cofre");
        assert!(!bruto.windows(8).any(|j| j == b"chave123"), "a chave fica cifrada");

        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        assert_eq!(s.estado().tocando.map(|t| (t.faixa, t.vez)), Some((FAIXA.into(), 1)), "na hora");
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)));
        assert_eq!(s.estado().tocando.map(|t| t.duracao_s), Some(200.0));
        s.controlar(Acao::Pausar).expect("pausa");
        assert!(esperar(|| travar(&falso.recebidos).iter().any(|v| v["command"] == "pause")));
        s.controlar(Acao::Parar).expect("para");
        assert!(esperar(|| s.estado().tocando.is_none()));
        drop(s);

        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto), "volta ligado, sem pedir a chave");
        s.desligar();
        assert_eq!(situacao(&s), SituacaoDoTocador::Desligado);
        drop(s);
        let s = Soloist::novo(falso, pasta.clone());
        assert_eq!((situacao(&s), s.estado().tem_chave), (SituacaoDoTocador::Desligado, true), "desligado fica");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn sem_parear_a_faixa_espera_e_toca_quando_a_pessoa_pareia() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("parear"));
        falso.fora.store(true, Ordering::SeqCst);
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pareando));
        assert_eq!(s.estado().nome.as_deref(), Some(APARELHO));
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("guarda");
        falso.fora.store(false, Ordering::SeqCst);
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)));
        // O Spotify desfez o pareamento no meio: a faixa acaba com o motivo.
        falso.fora.store(true, Ordering::SeqCst);
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pareando));
        assert_eq!(da_faixa(&s), Some(SituacaoDaFaixa::Falhou));
        drop(s);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn parar_antes_de_comecar_cala_a_faixa_quando_ela_comeca() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("calar"));
        falso.demora.store(600, Ordering::SeqCst);
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        assert!(esperar(|| travar(&falso.recebidos).iter().any(|v| v["command"] == "play")));
        s.controlar(Acao::Parar).expect("para");
        assert!(esperar(|| falso.comecos.load(Ordering::SeqCst) == 1), "ela começa por baixo");
        assert!(esperar(|| !falso.soando.load(Ordering::SeqCst)), "e é calada");
        // Tocar e parar e tocar de novo, colados: vale a última.
        falso.demora.store(0, Ordering::SeqCst);
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        s.controlar(Acao::Parar).expect("para");
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        let vez = s.estado().tocando.map(|t| t.vez);
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)));
        assert_eq!(s.estado().tocando.map(|t| t.vez), vez);
        drop(s);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn recusar_o_repetir_nao_e_recusar_a_faixa() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("recusa"));
        falso.recusa_os_set.store(true, Ordering::SeqCst);
        falso.demora.store(300, Ordering::SeqCst);
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)), "{:?}", s.estado().tocando);
        drop(s);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn desligar_no_meio_do_download_volta_logo() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("cancelar"));
        falso.baixando_sem_fim.store(true, Ordering::SeqCst);
        let mut s = Soloist::novo(falso, pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Baixando));
        let comecou = Instant::now();
        s.desligar();
        assert!(comecou.elapsed() < Duration::from_secs(2));
        assert_eq!(situacao(&s), SituacaoDoTocador::Desligado);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn uma_chave_que_nao_abre_nao_apaga_a_boa() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("chaves"));
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("boa")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        travar(&falso.falhas).push(Falha::Motivo("confira a chave".into()));
        s.ligar(Some("outra-coisa")).expect("tenta");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Falhou));
        s.ligar(None).expect("a guardada");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        assert_eq!(travar(&falso.chaves).last().map(String::as_str), Some("boa"));
        drop(s);
        // Bloqueado pelo Windows: não volta sozinho na próxima abertura (baixaria tudo de novo).
        travar(&falso.falhas).push(Falha::Motivo(super::super::BLOQUEADO.into()));
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Falhou));
        s.fechar();
        let s = Soloist::novo(falso, pasta.clone());
        assert_eq!((situacao(&s), s.estado().tem_chave), (SituacaoDoTocador::Desligado, true));
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn vencido_baixa_outro_uma_vez() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("venceu"));
        travar(&falso.falhas).push(Falha::Venceu);
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        assert_eq!(*travar(&falso.preparos), [false, true]);
        s.desligar();
        travar(&falso.falhas).extend([Falha::Venceu, Falha::Venceu]);
        s.ligar(None).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Falhou));
        assert!(s.estado().erro.expect("motivo").contains("venceu"));
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn um_que_so_cai_para_de_ser_aberto_e_diz_o_motivo() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("so-cai"));
        falso.novo.store(true, Ordering::SeqCst);
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        *travar(&falso.sair) = Some(-1_073_741_819);
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Falhou));
        assert!(s.estado().erro.expect("motivo").contains("0xc0000005"));
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto), "uma queda: abre de novo");
        *travar(&falso.sair) = Some(-1_073_741_819);
        assert!(esperar(|| s.estado().erro.is_some_and(|e| e == super::super::QUEBRADO)));
        std::thread::sleep(Duration::from_millis(300));
        assert_eq!((falso.aberturas.load(Ordering::SeqCst), situacao(&s)), (2, SituacaoDoTocador::Falhou));
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn caiu_no_meio_da_faixa_diz_e_abre_de_novo() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("caiu"));
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)));
        *travar(&falso.sair) = Some(1);
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Falhou)));
        assert!(esperar(|| falso.aberturas.load(Ordering::SeqCst) == 2 && situacao(&s) == SituacaoDoTocador::Pronto));
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn chave_recusada_falha_com_o_motivo_e_quem_pede_depois_tambem_sabe() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("recusada"));
        travar(&falso.falhas).push(Falha::Motivo("confira a chave".into()));
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("manda");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Falhou));
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Falhou)));
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("manda");
        assert!(esperar(|| s.estado().tocando.is_some_and(|t| t.vez == 2 && t.situacao == SituacaoDaFaixa::Falhou)));
        s.ligar(Some("k")).expect("tenta de novo");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        let _ = std::fs::remove_dir_all(&pasta);
    }
}
