//! Thread que conversa com o Windows (WinRT). Todo o código que pode travar ou falhar fica aqui.

use crate::{Capa, Controle, Estado, Faixa, Posicao, e_midia_de_verdade, nome_do_app, parece_identificador};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as Sessao, GlobalSystemMediaTransportControlsSessionManager as Gerente,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status,
};
use windows::Storage::Streams::DataReader;
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize};

const INTERVALO: Duration = Duration::from_millis(400);
/// Depois de uma falha ao falar com o Windows, espera antes de tentar de novo.
const ESPERA_APOS_ERRO: Duration = Duration::from_secs(5);
/// Quantas consultas seguidas ainda procuram a capa depois que a faixa muda (os apps publicam a capa depois).
const TENTATIVAS_DE_CAPA: u32 = 12;
/// Segundos entre 1601 (época do Windows) e 1970 (época Unix).
const EPOCA_WINDOWS_S: f64 = 11_644_473_600.0;
/// Textos que vêm de outro programa: limita o tamanho antes de guardar e mostrar.
const TETO_TEXTO: usize = 300;

type R<T> = windows::core::Result<T>;

/// O nome com que o próprio Player aparece para o Windows: a música local passa mesmo com um filtro.
const PROPRIO: &str = "Player";

pub fn rodar(estado: Arc<Mutex<Estado>>, versao: Arc<AtomicU64>, fila: Receiver<Controle>, filtro: Arc<Mutex<String>>) {
    // SAFETY: inicializa o WinRT nesta thread, uma vez, antes de qualquer chamada WinRT. Não há ponteiros nossos.
    if let Err(e) = unsafe { RoInitialize(RO_INIT_MULTITHREADED) } {
        log::warn!("WinRT indisponível ({e}); tocando agora desligado");
        return;
    }
    let publicar = |novo: Estado| {
        let mut g = match estado.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        *g = novo;
        versao.fetch_add(1, Ordering::Release);
    };

    let mut gerente: Option<Gerente> = None;
    let mut proxima_tentativa = Instant::now();
    let mut atual = Estado::default();
    let mut capa_de: Option<Faixa> = None;
    let mut tentativas_capa = 0u32;

    loop {
        // A espera pelo próximo comando é também o intervalo entre consultas.
        let controle = match fila.recv_timeout(INTERVALO) {
            Ok(c) => Some(c),
            Err(RecvTimeoutError::Timeout) => None,
            Err(RecvTimeoutError::Disconnected) => return,
        };

        if gerente.is_none() && Instant::now() >= proxima_tentativa {
            match Gerente::RequestAsync().and_then(|op| op.join()) {
                Ok(g) => gerente = Some(g),
                Err(e) => {
                    log::warn!("sessões de mídia indisponíveis: {e}");
                    proxima_tentativa = Instant::now() + ESPERA_APOS_ERRO;
                }
            }
        }
        let Some(g) = gerente.as_ref() else {
            if atual.disponivel {
                atual = Estado::default();
                publicar(atual.clone());
            }
            continue;
        };

        // `GetCurrentSession` falha quando nenhum app está publicando mídia: é o estado "vazio", não um erro.
        let quer = match filtro.lock() {
            Ok(f) => f.clone(),
            Err(p) => p.into_inner().clone(),
        };
        let sessao = if quer.is_empty() { escolher(g) } else { do_app(g, &quer) };
        if let (Some(c), Some(s)) = (controle, sessao.as_ref())
            && let Err(e) = enviar(s, c)
        {
            log::warn!("comando {c:?} recusado pelo app de origem: {e}");
        }

        let mut novo = Estado { disponivel: true, ..Default::default() };
        if let Some(s) = sessao.as_ref() {
            match ler(s) {
                Ok((faixa, tocando, posicao)) => {
                    novo.tocando = tocando;
                    novo.posicao = posicao;
                    if capa_de.as_ref() != Some(&faixa) {
                        capa_de = Some(faixa.clone());
                        tentativas_capa = TENTATIVAS_DE_CAPA;
                        atual.capa = None;
                    }
                    novo.capa = atual.capa.clone();
                    if novo.capa.is_none() && tentativas_capa > 0 {
                        tentativas_capa -= 1;
                        novo.capa = ler_capa(s).map(Arc::new);
                    }
                    novo.faixa = Some(faixa);
                }
                Err(e) => {
                    // O app fechou no meio da leitura, por exemplo. Volta ao estado vazio e tenta na próxima.
                    log::debug!("leitura da sessão falhou: {e}");
                    capa_de = None;
                }
            }
        } else {
            capa_de = None;
        }

        // A posição muda a cada leitura (o relógio anda): só publica quando algo visível mudou de verdade.
        let pulou = match (&atual.posicao, &novo.posicao) {
            (Some(a), Some(b)) => {
                (a.agora(atual.tocando) - b.agora(novo.tocando)).abs() > 1.5 || (a.duracao_s - b.duracao_s).abs() > 0.5
            }
            (None, None) => false,
            _ => true,
        };
        let mudou = atual.faixa != novo.faixa
            || atual.tocando != novo.tocando
            || atual.disponivel != novo.disponivel
            || atual.capa.as_ref().map(Arc::as_ptr) != novo.capa.as_ref().map(Arc::as_ptr)
            || pulou;
        if mudou {
            atual = novo;
            publicar(atual.clone());
        }
    }
}

fn esta_tocando(s: &Sessao) -> bool {
    s.GetPlaybackInfo().and_then(|i| i.PlaybackStatus()).is_ok_and(|status| status == Status::Playing)
}

fn e_proprio(s: &Sessao) -> bool {
    nome_do_app(&texto(s.SourceAppUserModelId())) == PROPRIO
}

/// Sem filtro: qual sessão acompanhar. O Windows aponta uma como "atual", mas ela pode estar parada enquanto
/// outro app toca (pausar a música local e dar play no Spotify, por exemplo). A ordem:
/// 1. um outro app tocando (a atual primeiro);
/// 2. o próprio Player tocando (o histórico conta a música local por aqui);
/// 3. a atual, se não for o próprio Player;
/// 4. qualquer outro app;
/// 5. o próprio Player.
fn escolher(g: &Gerente) -> Option<Sessao> {
    // Sessões sem mídia de verdade (o Riot Client, por exemplo) nem entram na conta.
    let atual = g.GetCurrentSession().ok().filter(tem_midia);
    let mut todas: Vec<Sessao> = g.GetSessions().map(|l| l.into_iter().filter(tem_midia).collect()).unwrap_or_default();
    if let Some(a) = atual.as_ref() {
        // A atual vai na frente: em empate, é ela que vale.
        let id = texto(a.SourceAppUserModelId());
        todas.retain(|s| texto(s.SourceAppUserModelId()) != id);
        todas.insert(0, a.clone());
    }
    let tocando: Vec<bool> = todas.iter().map(esta_tocando).collect();
    let proprio: Vec<bool> = todas.iter().map(e_proprio).collect();
    let escolhida = (0..todas.len())
        .find(|&i| tocando[i] && !proprio[i])
        .or_else(|| (0..todas.len()).find(|&i| tocando[i]))
        .or_else(|| (!todas.is_empty() && atual.is_some() && !proprio[0]).then_some(0))
        .or_else(|| (0..todas.len()).find(|&i| !proprio[i]))
        .or_else(|| (!todas.is_empty()).then_some(0))?;
    Some(todas.swap_remove(escolhida))
}

/// A sessão do app `quer` (ou do próprio Player), preferindo a que está tocando.
fn do_app(g: &Gerente, quer: &str) -> Option<Sessao> {
    let lista = g.GetSessions().ok()?;
    let mut achadas: Vec<Sessao> = lista
        .into_iter()
        .filter(|s| {
            let nome = nome_do_app(&texto(s.SourceAppUserModelId()));
            (nome == quer || nome == PROPRIO) && tem_midia(s)
        })
        .collect();
    match achadas.iter().position(esta_tocando) {
        Some(i) => Some(achadas.swap_remove(i)),
        None => achadas.into_iter().next(),
    }
}

fn enviar(s: &Sessao, c: Controle) -> R<()> {
    let op = match c {
        Controle::TocarPausar => s.TryTogglePlayPauseAsync()?,
        Controle::Proxima => s.TrySkipNextAsync()?,
        Controle::Anterior => s.TrySkipPreviousAsync()?,
    };
    if !op.join()? {
        log::debug!("o app de origem não aceitou {c:?}");
    }
    Ok(())
}

fn texto(h: R<windows::core::HSTRING>) -> String {
    let s = h.map(|h| h.to_string_lossy()).unwrap_or_default();
    let limpo: String = s.chars().filter(|c| !c.is_control()).take(TETO_TEXTO).collect();
    limpo.trim().to_string()
}

fn ler(s: &Sessao) -> R<(Faixa, bool, Option<Posicao>)> {
    let props = s.TryGetMediaPropertiesAsync()?.join()?;
    let mut artista = texto(props.Artist());
    // Alguns apps põem o próprio identificador no lugar do artista.
    if parece_identificador(&artista) {
        artista.clear();
    }
    let faixa = Faixa {
        titulo: texto(props.Title()),
        artista,
        album: texto(props.AlbumTitle()),
        app: nome_do_app(&texto(s.SourceAppUserModelId())),
    };
    let tocando = s.GetPlaybackInfo()?.PlaybackStatus()? == Status::Playing;
    Ok((faixa, tocando, ler_posicao(s, tocando)))
}

/// A sessão tem música (ou vídeo) de verdade, não só um app que abriu a sessão de mídia à toa.
fn tem_midia(s: &Sessao) -> bool {
    ler(s).is_ok_and(|(faixa, _, posicao)| {
        e_midia_de_verdade(&faixa, &texto(s.SourceAppUserModelId()), posicao.is_some())
    })
}

/// Posição e duração, quando o app informa. Muitos apps (navegadores, por exemplo) não informam: `None`.
fn ler_posicao(s: &Sessao, tocando: bool) -> Option<Posicao> {
    let t = s.GetTimelineProperties().ok()?;
    let seg = |ticks: i64| ticks as f64 / 1e7;
    let inicio = seg(t.StartTime().ok()?.Duration);
    let duracao = seg(t.EndTime().ok()?.Duration) - inicio;
    if !duracao.is_finite() || duracao <= 1.0 || duracao > 24.0 * 3600.0 {
        return None;
    }
    let mut posicao = seg(t.Position().ok()?.Duration) - inicio;
    // O valor é de quando o app atualizou pela última vez: soma o tempo passado desde então.
    if tocando
        && let Ok(quando) = t.LastUpdatedTime()
        && let Ok(agora) = SystemTime::now().duration_since(UNIX_EPOCH)
    {
        let atraso = agora.as_secs_f64() + EPOCA_WINDOWS_S - seg(quando.UniversalTime);
        if (0.0..duracao).contains(&atraso) {
            posicao += atraso;
        }
    }
    Some(Posicao { posicao_s: posicao.clamp(0.0, duracao), duracao_s: duracao, lida_em: Instant::now() })
}

fn ler_capa(s: &Sessao) -> Option<Capa> {
    let ler = || -> R<Vec<u8>> {
        let props = s.TryGetMediaPropertiesAsync()?.join()?;
        let stream = props.Thumbnail()?.OpenReadAsync()?.join()?;
        let tamanho = stream.Size()?;
        if tamanho == 0 || tamanho > crate::capa::TETO_BYTES as u64 {
            return Ok(Vec::new());
        }
        let leitor = DataReader::CreateDataReader(&stream)?;
        let lidos = leitor.LoadAsync(tamanho as u32)?.join()? as usize;
        let mut bytes = vec![0u8; lidos.min(tamanho as usize)];
        leitor.ReadBytes(&mut bytes)?;
        Ok(bytes)
    };
    Capa::decodificar(&ler().ok()?)
}
