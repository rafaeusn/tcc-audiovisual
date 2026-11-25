//! O Soloist ligado: a thread que o abre, conversa com a API local dele, acompanha a faixa pedida e o reabre se cair.
//! Quem chama só manda e lê o estado; nada aqui espera a rede.

use super::instalar::{EXE, Preparo};
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
/// Depois do fim de uma faixa, quanto tempo calar o que o Spotify emendar sozinho (a fila é do Player).
const CALAR_NO_FIM: Duration = Duration::from_secs(5);
/// Depois de quanto tempo tocando (somado, e não numa faixa só: a pessoa pode pular) as quedas de antes não contam mais.
/// Menos que isto não prova nada: o build que quebra chega a avisar que está tocando, e cai em seguida.
const TOCOU_BEM: Duration = Duration::from_secs(15);
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
            && let Err(e) = s.ligar_como(None, false, false)
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
        self.ligar_como(chave, true, false)
    }

    /// Liga com o outro Soloist deste computador (o da `alternativa`), copiado para cá.
    pub fn trocar(&mut self) -> Result<(), String> {
        self.ligar_como(None, true, true)
    }

    /// `pedida`: foi a pessoa, agora (e não a abertura do Player). `trazer`: antes, copia o da alternativa.
    fn ligar_como(&mut self, chave: Option<&str>, pedida: bool, trazer: bool) -> Result<(), String> {
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
            pedida,
            trazer,
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

/// O programa quebrou (uma exceção do Windows), e não foi fechado por alguém, pelo fim da sessão ou por conta própria.
fn quebrou(codigo: i32) -> bool {
    const CTRL_C: u32 = 0xC000_013A;
    let codigo = codigo as u32;
    codigo >= 0xC000_0000 && codigo != CTRL_C
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
    pedida: bool,
    trazer: bool,
}

/// O que o laço sabe de um Soloist aberto.
#[derive(Default)]
struct Aberta {
    leitura: Leitura,
    pedida: Option<Pedida>,
    /// Até quando calar o que soar sem ter sido pedido, e a última pausa mandada por isso (quando, e em que faixa).
    calar_ate: Option<Instant>,
    pausou: Option<(Instant, String)>,
}

impl Aberta {
    fn calar(&mut self) {
        self.calar_ate = Some(Instant::now() + CALAR);
        self.pausou = None;
    }

    /// A faixa acabou: a próxima costuma vir em seguida, então só se cala se ela demorar, ou se o Spotify emendar outra.
    fn calar_depois_do_fim(&mut self) {
        self.calar_ate = Some(Instant::now() + CALAR_NO_FIM);
        self.pausou = Some((Instant::now(), self.leitura.uri().to_string()));
    }

    /// A próxima que foi adiantada e não vai mais entrar: da fila do Spotify ela só sai passando por ela (o que vem em
    /// seguida, tocar outra ou calar, não a deixa soar).
    fn tirar_da_fila(&self) -> Option<Value> {
        self.pedida.as_ref().is_some_and(|p| p.deixou_na_fila(&self.leitura)).then(|| comando("skip_next"))
    }

    /// O que soa sem ter sido pedido (uma faixa que começou depois de a pessoa desistir dela) é pausado.
    fn pausa_devida(&mut self) -> Option<Value> {
        let agora = Instant::now();
        self.calar_ate = self.calar_ate.filter(|ate| agora < *ate);
        self.calar_ate?;
        let uri = self.leitura.uri();
        let ja = |(quando, qual): &(Instant, String)| agora.duration_since(*quando) < ENTRE_PAUSAS && qual == uri;
        if !self.leitura.soando() || self.pausou.as_ref().is_some_and(ja) {
            return None;
        }
        self.pausou = Some((agora, uri.to_string()));
        Some(comando("pause"))
    }
}

impl Laco {
    fn situar(&self, situacao: SituacaoDoTocador, erro: Option<String>) {
        let mut e = travar(&self.estado);
        (e.situacao, e.erro, e.progresso) = (situacao, erro, None);
        e.processo = (situacao == SituacaoDoTocador::Pronto).then(|| EXE.to_string());
        e.adianta = situacao == SituacaoDoTocador::Pronto;
    }

    /// Não há um programa que sirva: pode haver um bom, de outro, neste computador.
    fn sem_programa(&self, motivo: String) {
        // A chave colada ainda não abriu nada, mas também não apaga nenhuma: sem ela, não daria para usar o outro.
        if let Some(cofre) = self.guardar.as_ref().filter(|c| !c.is_file()) {
            let _ = servicos::cofre::guardar(cofre, &self.chave);
        }
        let alternativa = self.programa.alternativa(&self.parar);
        travar(&self.estado).alternativa = alternativa;
        self.desistir(motivo);
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
        if self.trazer
            && let Err(e) = self.programa.trazer(&self.parar)
        {
            return self.sem_programa(e);
        }
        let mut pedida = self.pedida;
        loop {
            // (Só está baixando se o download começou: quase sempre o programa já está aí.)
            let estado = self.estado.clone();
            let progresso = move |p: f32| {
                let mut e = travar(&estado);
                (e.situacao, e.progresso) = (SituacaoDoTocador::Baixando, Some(p));
            };
            // O pedido da pessoa vale para a primeira abertura; depois dela, o ritmo é o de sempre.
            let como = Preparo { de_novo: venceu, pedida: std::mem::take(&mut pedida) };
            if let Err(e) = self.programa.preparar(como, &self.parar, &progresso) {
                return self.sem_programa(e);
            }
            travar(&self.estado).versao = self.programa.versao();
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
                // Um que só cai não é aberto de novo: o Match toca, e o preparo acha outro (o anterior, ou um publicado
                // depois) ou desiste.
                Fim::Fechou(codigo) if quebrou(codigo) && self.programa.caiu() => {
                    let motivo = format!("o Soloist caiu ({})", nome_do_codigo(codigo));
                    eprintln!("soloist: {motivo}, e não é a primeira vez: procurando outro");
                    self.faixa_falhou(&motivo);
                    (guardada, quedas) = (None, 0);
                    self.situar(SituacaoDoTocador::Falhou, Some(motivo));
                }
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
        let (mut tocado, mut tocou_bem, mut volta_anterior) = (Duration::ZERO, false, Instant::now());
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
                let (falhou_agora, fim_agora) = (!antes && p.falhou_mesmo(), !antes && p.chegou_ao_fim());
                let publicar = p.publicar(&a.leitura);
                let passou = volta_anterior.elapsed();
                if publicar.situacao == SituacaoDaFaixa::Tocando {
                    tocado += passou;
                    if !tocou_bem && tocado >= TOCOU_BEM {
                        tocou_bem = true;
                        self.programa.funcionou();
                    }
                }
                if falhou_agora {
                    // Ela ainda pode começar atrasada, por baixo do que o Player já toca no lugar.
                    a.calar();
                } else if fim_agora {
                    a.calar_depois_do_fim();
                }
                let mut e = travar(&self.estado);
                // Uma faixa pedida agora mesmo (a ordem ainda no canal) não volta para a de antes.
                if e.tocando.as_ref().is_none_or(|t| t.vez <= publicar.vez) {
                    e.tocando = Some(publicar);
                }
            }
            volta_anterior = Instant::now();
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
                // O que já há na fila do Spotify (a próxima só vai para lá se estiver vazia).
                let mut c = comando("get_queue");
                c["limit"] = json!(5);
                let _ = api.mandar(&c);
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
                // A pessoa pulou para a que já está na fila dele, carregada: basta passar para ela.
                let e_ela = a.pedida.as_ref().is_some_and(|p| p.e_a_seguinte(&faixa, &a.leitura, agora_ms()));
                mandar.extend(a.tirar_da_fila());
                let (nova, tocar) = Pedida::nova(&faixa, vez, &mut a.leitura, agora_ms());
                mandar.extend(tocar.filter(|_| !e_ela));
                a.pedida = Some(nova);
                a.calar_ate = None;
            }
            Acao::Preparar { faixa } => {
                mandar.extend(a.pedida.as_mut().and_then(|p| p.adiantar(&faixa, &a.leitura)));
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
                mandar.extend(a.tirar_da_fila());
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
        /// O que o Soloist de mentira conta em seguida, por conta própria.
        eventos: Arc<Mutex<Vec<Value>>>,
        baixando_sem_fim: AtomicBool,
        /// Nunca funcionou: na segunda queda é um quebrado.
        novo: AtomicBool,
        quedas: AtomicU32,
        quebrado: AtomicBool,
        /// Com um quebrado, há um anterior para abrir.
        tem_anterior: AtomicBool,
        /// Há um Soloist de outro programa.
        de_outro: AtomicBool,
        trazidos: AtomicU32,
        funcionou: AtomicU32,
        pedidas: Mutex<Vec<bool>>,
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
            let quebrou = self.quedas.fetch_add(1, Ordering::SeqCst) >= 1 && self.novo.load(Ordering::SeqCst);
            self.quebrado.store(quebrou, Ordering::SeqCst);
            quebrou
        }

        fn funcionou(&self) {
            self.funcionou.fetch_add(1, Ordering::SeqCst);
        }

        fn versao(&self) -> Option<String> {
            Some(if self.trazidos.load(Ordering::SeqCst) > 0 { "1.3.10.51".into() } else { "1.4.0.7".into() })
        }

        fn alternativa(&self, _: &AtomicBool) -> Option<String> {
            self.de_outro.load(Ordering::SeqCst).then(|| "1.3.10.51 (do Spotifast)".into())
        }

        fn trazer(&self, _: &AtomicBool) -> Result<(), String> {
            if !self.de_outro.load(Ordering::SeqCst) {
                return Err("não há outro".into());
            }
            self.trazidos.fetch_add(1, Ordering::SeqCst);
            self.quebrado.store(false, Ordering::SeqCst);
            Ok(())
        }

        fn preparar(&self, como: Preparo, parar: &AtomicBool, progresso: &dyn Fn(f32)) -> Result<(), String> {
            travar(&self.preparos).push(como.de_novo);
            travar(&self.pedidas).push(como.pedida);
            if self.quebrado.load(Ordering::SeqCst) && !self.tem_anterior.swap(false, Ordering::SeqCst) {
                return Err(super::super::QUEBRADO.into());
            }
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
            let (comecos, eventos) = (self.comecos.clone(), self.eventos.clone());
            std::thread::spawn(move || {
                let Ok((fio, _)) = escuta.accept() else { return };
                let Ok(mut ws) = tungstenite::accept(fio) else { return };
                let mut entrou = !fora.load(Ordering::SeqCst);
                let auth = |dentro: bool| json!({"type":"auth_state","logged_in":dentro,"is_active":false}).to_string();
                let _ = ws.send(Message::text(auth(entrou)));
                let _ = ws.get_ref().set_read_timeout(Some(Duration::from_millis(20)));
                let mut comecar: Option<(Instant, Value)> = None;
                let mut na_fila: Vec<Value> = Vec::new();
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
                    for v in std::mem::take(&mut *travar(&eventos)) {
                        match v["type"].as_str() {
                            Some("queue_changed") => na_fila.clear(),
                            _ => soando.store(v["status"] == "playing", Ordering::SeqCst),
                        }
                        let _ = ws.send(Message::text(v.to_string()));
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
                            if let "get_queue" | "add_to_queue" | "skip_next" = nome {
                                match nome {
                                    "add_to_queue" => na_fila.push(v["uri"].clone()),
                                    // Já carregada: começa logo.
                                    "skip_next" if !na_fila.is_empty() => {
                                        comecar = Some((Instant::now(), na_fila.remove(0)));
                                    }
                                    _ => {}
                                }
                                let u: Vec<Value> = na_fila.iter().map(|u| json!({"item":{"uri":u}})).collect();
                                let fila = json!({"type":"queue_changed","previous":[],"upcoming":u});
                                let _ = ws.send(Message::text(fila.to_string()));
                            }
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

    /// O fim como o Soloist o conta, e o que o Spotify emenda depois se a fila do Player não traz outra.
    #[test]
    fn o_que_o_spotify_emenda_depois_do_fim_e_calado_mas_a_seguinte_toca() {
        const OUTRA: &str = "6rqhFgbbKwnb9MLmUQDhG6";
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("emenda"));
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        let conta = |id: &str, status: &str, posicao: u64| {
            let v = json!({"type":"playback_state","status":status,"is_active":true,
                "item":{"uri":format!("spotify:track:{id}"),"decorations":{"playback":{"duration_ms":200_000}}},
                "position":{"position_ms":posicao,"timestamp_ms":agora_ms(),"speed":0.0}});
            travar(&falso.eventos).push(v);
        };
        let pausas = || travar(&falso.recebidos).iter().filter(|v| v["command"] == "pause").count();
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)));
        conta(FAIXA, "buffering", 199_700);
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Acabou)));
        // A seguinte vem logo: nenhuma pausa no caminho dela.
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)));
        std::thread::sleep(Duration::from_millis(700));
        assert_eq!((pausas(), falso.soando.load(Ordering::SeqCst)), (0, true));
        // Na última da fila não vem nenhuma, e o Spotify emenda uma dele: calada assim que começa.
        conta(FAIXA, "buffering", 199_700);
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Acabou)));
        conta(OUTRA, "playing", 0);
        assert!(esperar(|| pausas() >= 1 && !falso.soando.load(Ordering::SeqCst)), "calada");
        drop(s);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    /// A próxima vai para a fila do Soloist e entra sozinha; pular antes disso a tira de lá.
    #[test]
    fn a_proxima_adiantada_entra_sozinha_e_pular_antes_a_tira_da_fila() {
        const OUTRA: &str = "6rqhFgbbKwnb9MLmUQDhG6";
        const TERCEIRA: &str = "3n3Ppam7vgaVa1iaRUc9Lp";
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("adianta"));
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        assert!(s.estado().adianta);
        let comandos = || -> Vec<String> {
            let r = travar(&falso.recebidos);
            let nomes = ["add_to_queue", "skip_next", "play"];
            r.iter().filter_map(|v| v["command"].as_str()).filter(|c| nomes.contains(c)).map(String::from).collect()
        };
        assert!(esperar(|| travar(&falso.recebidos).iter().any(|v| v["command"] == "get_queue")), "pergunta a fila");
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)));
        s.controlar(Acao::Preparar { faixa: OUTRA.into() }).expect("adianta");
        assert!(esperar(|| comandos() == ["play", "add_to_queue"]), "{:?}", comandos());
        assert_eq!(s.estado().tocando.map(|t| t.faixa), Some(FAIXA.into()), "a de agora continua");
        // O Soloist passa para ela.
        travar(&falso.eventos).push(json!({"type":"queue_changed","previous":[],"upcoming":[]}));
        travar(&falso.eventos).push(json!({"type":"playback_state","status":"playing","is_active":true,
            "item":{"uri":format!("spotify:track:{OUTRA}"),"decorations":{"playback":{"duration_ms":90_000}}},
            "position":{"position_ms":0,"timestamp_ms":agora_ms(),"speed":1.0}}));
        assert!(esperar(|| s
            .estado()
            .tocando
            .is_some_and(|t| t.faixa == OUTRA && t.vez == 1 && t.situacao == SituacaoDaFaixa::Tocando)));
        assert_eq!(comandos(), ["play", "add_to_queue"], "sem nenhuma ordem");
        assert!(falso.soando.load(Ordering::SeqCst), "e sem pausa");
        // A pessoa pula para a que já está na fila: é só passar (ela já está carregada).
        s.controlar(Acao::Preparar { faixa: FAIXA.into() }).expect("adianta");
        assert!(esperar(|| comandos().len() == 3));
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("pula");
        assert!(esperar(|| s
            .estado()
            .tocando
            .is_some_and(|t| t.faixa == FAIXA && t.vez == 2 && t.situacao == SituacaoDaFaixa::Tocando)));
        assert_eq!(comandos(), ["play", "add_to_queue", "add_to_queue", "skip_next"]);
        // Para outra: a adiantada é consumida, e toca a que ela quer.
        s.controlar(Acao::Preparar { faixa: OUTRA.into() }).expect("adianta");
        assert!(esperar(|| comandos().len() == 5));
        s.controlar(Acao::Tocar { faixa: TERCEIRA.into() }).expect("pula");
        assert!(esperar(|| s
            .estado()
            .tocando
            .is_some_and(|t| t.faixa == TERCEIRA && t.situacao == SituacaoDaFaixa::Tocando)));
        assert_eq!(comandos()[4..], ["add_to_queue", "skip_next", "play"]);
        // Parar com uma adiantada: ela também sai da fila, calada.
        s.controlar(Acao::Preparar { faixa: OUTRA.into() }).expect("adianta");
        assert!(esperar(|| comandos().len() == 8));
        s.controlar(Acao::Parar).expect("para");
        assert!(esperar(|| comandos().last().is_some_and(|c| c == "skip_next")));
        assert!(esperar(|| !falso.soando.load(Ordering::SeqCst)));
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
        assert_eq!(s.estado().alternativa, None, "não há outro neste computador");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn quebrado_volta_ao_anterior_sozinho() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("anterior"));
        falso.novo.store(true, Ordering::SeqCst);
        falso.quedas.store(1, Ordering::SeqCst);
        falso.tem_anterior.store(true, Ordering::SeqCst);
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        s.controlar(Acao::Tocar { faixa: FAIXA.into() }).expect("toca");
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Tocando)));
        *travar(&falso.sair) = Some(-1_073_741_819);
        assert!(esperar(|| da_faixa(&s) == Some(SituacaoDaFaixa::Falhou)), "a faixa vai para o Match");
        assert!(esperar(|| falso.aberturas.load(Ordering::SeqCst) == 2 && situacao(&s) == SituacaoDoTocador::Pronto));
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn fechado_por_alguem_nao_e_um_programa_quebrado() {
        for codigo in [0, 1, 0xC000_013A_u32 as i32, 0x4001_0004] {
            assert!(!quebrou(codigo), "{codigo:x}");
        }
        assert!(quebrou(-1_073_741_819), "0xc0000005");
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("fechado"));
        falso.novo.store(true, Ordering::SeqCst);
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        *travar(&falso.sair) = Some(1);
        assert!(esperar(|| falso.aberturas.load(Ordering::SeqCst) == 2 && situacao(&s) == SituacaoDoTocador::Pronto));
        assert_eq!(falso.quedas.load(Ordering::SeqCst), 0, "ele abre de novo, e não conta como quebra");
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn com_um_quebrado_oferece_o_de_outro_programa_e_troca_quando_a_pessoa_pede() {
        let (falso, pasta) = (Arc::new(Falso::default()), pasta("alternativa"));
        falso.quebrado.store(true, Ordering::SeqCst);
        falso.de_outro.store(true, Ordering::SeqCst);
        let mut s = Soloist::novo(falso.clone(), pasta.clone());
        assert!(s.trocar().is_err(), "sem chave não há o que ligar");
        s.ligar(Some("k")).expect("liga");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Falhou));
        assert_eq!(s.estado().alternativa.as_deref(), Some("1.3.10.51 (do Spotifast)"));
        assert_eq!(falso.trazidos.load(Ordering::SeqCst), 0, "só quando a pessoa pede");
        assert!(s.estado().tem_chave, "a primeira chave fica, para dar para usar o outro");
        s.trocar().expect("troca");
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        let e = s.estado();
        assert_eq!((e.versao.as_deref(), e.alternativa), (Some("1.3.10.51"), None));
        // Pela abertura do Player, não é um pedido da pessoa.
        drop(s);
        let s = Soloist::novo(falso.clone(), pasta.clone());
        assert!(esperar(|| situacao(&s) == SituacaoDoTocador::Pronto));
        assert_eq!(*travar(&falso.pedidas), [true, true, false]);
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
