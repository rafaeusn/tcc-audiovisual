//! A conversa com o Windows. Todo o código `unsafe` da integração fica neste arquivo.
//!
//! A janela é do GPUI; aqui entramos na fila de mensagens dela (uma "subclasse" do Windows) para ver as poucas
//! mensagens que nos interessam e deixar todas as outras passarem.

use crate::Acao;
use crate::desenho::{Forma, desenhar};
use std::cell::RefCell;
use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateBitmap, DeleteObject, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow,
};
use windows::Win32::System::Com::{CLSCTX_INPROC_SERVER, CoCreateInstance};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Shell::{
    DefSubclassProc, ITaskbarList3, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    RemoveWindowSubclass, SetWindowSubclass, Shell_NotifyIconW, THB_FLAGS, THB_ICON, THB_TOOLTIP, THBF_ENABLED,
    THUMBBUTTON, TaskbarList,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreateIconIndirect, CreatePopupMenu, DestroyIcon, DestroyMenu, GA_ROOT, GetAncestor, GetCursorPos,
    GetSystemMetrics, GetWindowPlacement, HICON, HWND_NOTOPMOST, HWND_TOPMOST, ICONINFO, IsIconic, MF_SEPARATOR,
    MF_STRING, MINMAXINFO, PostMessageW, RegisterWindowMessageW, SM_CXSMICON, SW_HIDE, SW_MAXIMIZE, SW_RESTORE,
    SW_SHOW, SWP_NOMOVE, SWP_NOSIZE, SetForegroundWindow, SetWindowPlacement, SetWindowPos, ShowWindow,
    TPM_BOTTOMALIGN, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WINDOWPLACEMENT, WM_APP, WM_CLOSE, WM_COMMAND,
    WM_GETMINMAXINFO, WM_LBUTTONUP, WM_NCDESTROY, WM_RBUTTONUP, WindowFromPoint,
};
use windows::core::{PCWSTR, w};

/// Número da nossa subclasse na janela e do ícone na bandeja.
const ID: usize = 0x5443;
/// Mensagem que a bandeja manda para a janela quando mexem no ícone.
const MSG_BANDEJA: u32 = WM_APP + 0x4A1;
/// Mensagem nossa: entrar na janela mini (1) ou sair dela (0).
const MSG_MINI: u32 = WM_APP + 0x4A2;
/// Código de "botão da miniatura clicado" dentro de `WM_COMMAND`.
const THBN_CLICKED: usize = 0x1800;

const BOTAO_ANTERIOR: u32 = 1;
const BOTAO_TOCAR: u32 = 2;
const BOTAO_PROXIMA: u32 = 3;
const MENU_MOSTRAR: usize = 10;
const MENU_SAIR: usize = 11;

/// Tamanho da janela mini e o menor tamanho a que ela pode ser reduzida, em pixels a 100% de escala.
const MINI: (i32, i32) = (400, 240);
const MINI_MINIMO: (i32, i32) = (260, 160);
const MINI_MARGEM: i32 = 24;

struct Estado {
    hwnd: HWND,
    ao_pedir: Box<dyn Fn(Acao)>,
    barra: Option<ITaskbarList3>,
    botoes_postos: bool,
    na_bandeja: bool,
    anterior: HICON,
    tocar: HICON,
    pausar: HICON,
    proxima: HICON,
    app: HICON,
    tocando: bool,
    dica: String,
    fechar_esconde: bool,
    /// A pessoa pediu para sair pelo menu: o próximo fechar é de verdade.
    saindo: bool,
    /// Lugar e tamanho da janela antes de virar mini.
    antes_do_mini: Option<WINDOWPLACEMENT>,
    msg_botao_criado: u32,
    msg_barra_criada: u32,
}

thread_local! {
    static ESTADO: RefCell<Option<Estado>> = const { RefCell::new(None) };
}

/// Lê ou muda o estado. Devolve `None` se a integração não está instalada (ou se já estamos dentro de outra
/// chamada, o que acontece quando o Windows reentra na janela durante um menu).
fn com_estado<T>(f: impl FnOnce(&mut Estado) -> T) -> Option<T> {
    ESTADO.with(|e| e.try_borrow_mut().ok().and_then(|mut g| g.as_mut().map(f)))
}

fn icone(forma: Forma, lado: i32) -> HICON {
    let pixels = desenhar(forma, lado.max(1) as usize);
    let mascara = vec![0u8; (lado.max(1) * lado.max(1)) as usize];
    // SAFETY: os dois bitmaps são criados a partir de vetores vivos do tamanho pedido (32 e 8 bits por pixel) e
    // apagados logo depois de `CreateIconIndirect` copiar o conteúdo.
    unsafe {
        let cor = CreateBitmap(lado, lado, 1, 32, Some(pixels.as_ptr().cast::<c_void>()));
        let transparencia = CreateBitmap(lado, lado, 1, 8, Some(mascara.as_ptr().cast::<c_void>()));
        let info = ICONINFO { fIcon: true.into(), xHotspot: 0, yHotspot: 0, hbmMask: transparencia, hbmColor: cor };
        let criado = CreateIconIndirect(&info).unwrap_or_default();
        let _ = DeleteObject(cor.into());
        let _ = DeleteObject(transparencia.into());
        criado
    }
}

fn texto_fixo<const N: usize>(texto: &str) -> [u16; N] {
    let mut saida = [0u16; N];
    for (destino, unidade) in saida.iter_mut().take(N - 1).zip(texto.encode_utf16()) {
        *destino = unidade;
    }
    saida
}

fn dados_da_bandeja(e: &Estado) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW {
        cbSize: size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: e.hwnd,
        uID: ID as u32,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: MSG_BANDEJA,
        hIcon: e.app,
        szTip: texto_fixo(&e.dica),
        ..Default::default()
    }
}

fn por_na_bandeja(e: &mut Estado) {
    let dados = dados_da_bandeja(e);
    // SAFETY: `dados` é uma estrutura completa e viva durante a chamada.
    e.na_bandeja = unsafe { Shell_NotifyIconW(NIM_ADD, &dados) }.as_bool();
    if !e.na_bandeja {
        log::debug!("o Windows não aceitou o ícone na bandeja");
    }
}

fn botoes(e: &Estado) -> [THUMBBUTTON; 3] {
    let botao = |id: u32, icone: HICON, dica: &str| THUMBBUTTON {
        dwMask: THB_ICON | THB_TOOLTIP | THB_FLAGS,
        iId: id,
        iBitmap: 0,
        hIcon: icone,
        szTip: texto_fixo(dica),
        dwFlags: THBF_ENABLED,
    };
    let (meio, dica) = if e.tocando { (e.pausar, "Pausar") } else { (e.tocar, "Tocar") };
    [
        botao(BOTAO_ANTERIOR, e.anterior, "Faixa anterior"),
        botao(BOTAO_TOCAR, meio, dica),
        botao(BOTAO_PROXIMA, e.proxima, "Próxima faixa"),
    ]
}

/// Põe os botões na miniatura. Só funciona depois que o Windows criou o botão do app na barra de tarefas.
fn por_botoes(e: &mut Estado) {
    let Some(barra) = e.barra.as_ref() else { return };
    let lista = botoes(e);
    // SAFETY: a interface COM foi criada nesta thread e a lista vive durante a chamada.
    let feito = unsafe {
        if e.botoes_postos {
            barra.ThumbBarUpdateButtons(e.hwnd, &lista)
        } else {
            barra.ThumbBarAddButtons(e.hwnd, &lista)
        }
    };
    match feito {
        Ok(()) => e.botoes_postos = true,
        Err(erro) => log::debug!("botões da barra de tarefas ainda indisponíveis: {erro}"),
    }
}

pub fn instalar(hwnd: isize, ao_pedir: Box<dyn Fn(Acao)>) -> bool {
    let hwnd = HWND(hwnd as *mut c_void);
    // SAFETY: chamadas simples do Windows com a janela do app; o COM já foi iniciado nesta thread pelo GPUI.
    let (lado, barra, msg_botao_criado, msg_barra_criada) = unsafe {
        let barra: Option<ITaskbarList3> = CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)
            .and_then(|b: ITaskbarList3| b.HrInit().map(|()| b))
            .map_err(|e| log::debug!("barra de tarefas indisponível: {e}"))
            .ok();
        (
            GetSystemMetrics(SM_CXSMICON).clamp(16, 64),
            barra,
            RegisterWindowMessageW(w!("TaskbarButtonCreated")),
            RegisterWindowMessageW(w!("TaskbarCreated")),
        )
    };
    let mut estado = Estado {
        hwnd,
        ao_pedir,
        barra,
        botoes_postos: false,
        na_bandeja: false,
        anterior: icone(Forma::Anterior, lado),
        tocar: icone(Forma::Tocar, lado),
        pausar: icone(Forma::Pausar, lado),
        proxima: icone(Forma::Proxima, lado),
        app: icone(Forma::App, lado),
        tocando: false,
        dica: "Player".to_string(),
        fechar_esconde: false,
        saindo: false,
        antes_do_mini: None,
        msg_botao_criado,
        msg_barra_criada,
    };
    // SAFETY: `ao_receber` tem a assinatura que o Windows espera e vive pelo programa inteiro.
    if !unsafe { SetWindowSubclass(hwnd, Some(ao_receber), ID, 0) }.as_bool() {
        log::warn!("não foi possível ligar o Player à barra de tarefas e à bandeja");
        soltar_icones(&estado);
        return false;
    }
    por_na_bandeja(&mut estado);
    por_botoes(&mut estado);
    ESTADO.with(|e| *e.borrow_mut() = Some(estado));
    true
}

fn soltar_icones(e: &Estado) {
    for i in [e.anterior, e.tocar, e.pausar, e.proxima, e.app] {
        if !i.is_invalid() {
            // SAFETY: cada ícone foi criado por `CreateIconIndirect` e só é destruído aqui.
            let _ = unsafe { DestroyIcon(i) };
        }
    }
}

pub fn remover() {
    let Some(e) = ESTADO.with(|e| e.try_borrow_mut().ok().and_then(|mut g| g.take())) else { return };
    // SAFETY: desfaz o que `instalar` fez, com os mesmos identificadores, na mesma thread.
    unsafe {
        if e.na_bandeja {
            let _ = Shell_NotifyIconW(NIM_DELETE, &dados_da_bandeja(&e));
        }
        let _ = RemoveWindowSubclass(e.hwnd, Some(ao_receber), ID);
    }
    soltar_icones(&e);
}

pub fn tocando(tocando: bool) {
    com_estado(|e| {
        if e.tocando != tocando {
            e.tocando = tocando;
            if e.botoes_postos {
                por_botoes(e);
            }
        }
    });
}

pub fn dica(texto: &str) {
    com_estado(|e| {
        if e.dica != texto {
            e.dica = texto.to_string();
            if e.na_bandeja {
                let dados = dados_da_bandeja(e);
                // SAFETY: `dados` é uma estrutura completa e viva durante a chamada.
                let _ = unsafe { Shell_NotifyIconW(NIM_MODIFY, &dados) };
            }
        }
    });
}

pub fn fechar_esconde(ligado: bool) {
    // Sem ícone na bandeja não haveria como trazer a janela de volta: aí fechar continua fechando.
    com_estado(|e| e.fechar_esconde = ligado && e.na_bandeja);
}

/// Pede a troca para a janela mini (ou de volta). A troca acontece logo depois, fora do evento que a pediu:
/// mudar o tamanho da janela de dentro de um evento do GPUI faz ele perder o aviso de que o tamanho mudou.
pub fn mini(ligado: bool) -> bool {
    let Some(hwnd) = com_estado(|e| e.hwnd) else { return false };
    // SAFETY: põe uma mensagem nossa na fila da própria janela.
    unsafe { PostMessageW(Some(hwnd), MSG_MINI, WPARAM(usize::from(ligado)), LPARAM(0)) }.is_ok()
}

/// O ponteiro do mouse está em cima da janela do app agora (e não de outra janela por cima dela)?
pub fn ponteiro_dentro() -> bool {
    let Some(hwnd) = com_estado(|e| e.hwnd) else { return false };
    // SAFETY: só pergunta ao Windows onde o ponteiro está e que janela há nesse ponto.
    unsafe {
        let mut ponto = POINT::default();
        if GetCursorPos(&mut ponto).is_err() {
            return false;
        }
        let embaixo = WindowFromPoint(ponto);
        embaixo == hwnd || GetAncestor(embaixo, GA_ROOT) == hwnd
    }
}

pub fn mini_para_maximizada() -> bool {
    let Some(hwnd) = com_estado(|e| e.hwnd) else { return false };
    // SAFETY: põe uma mensagem nossa na fila da própria janela.
    unsafe { PostMessageW(Some(hwnd), MSG_MINI, WPARAM(2), LPARAM(0)) }.is_ok()
}

fn aplicar_mini(ligado: bool) -> bool {
    let Some((hwnd, antes)) = com_estado(|e| (e.hwnd, e.antes_do_mini)) else { return false };
    // SAFETY: chamadas do Windows sobre a janela do app, com estruturas locais do tamanho certo.
    unsafe {
        if ligado {
            if antes.is_some() {
                return true;
            }
            let mut lugar = WINDOWPLACEMENT { length: size_of::<WINDOWPLACEMENT>() as u32, ..Default::default() };
            if GetWindowPlacement(hwnd, &mut lugar).is_err() {
                return false;
            }
            // Maximizada ou minimizada, a janela não aceita o tamanho novo.
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let escala = |v: i32| v * GetDpiForWindow(hwnd).max(96) as i32 / 96;
            let mut tela = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
            if !GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut tela).as_bool() {
                return false;
            }
            let (w, h) = (escala(MINI.0), escala(MINI.1));
            let (x, y) = (tela.rcWork.right - w - escala(MINI_MARGEM), tela.rcWork.bottom - h - escala(MINI_MARGEM));
            // O estado muda antes: é ele que libera o tamanho pequeno na resposta a WM_GETMINMAXINFO.
            com_estado(|e| e.antes_do_mini = Some(lugar));
            if SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, w, h, Default::default()).is_err() {
                com_estado(|e| e.antes_do_mini = None);
                return false;
            }
            true
        } else {
            let Some(lugar) = antes else { return true };
            com_estado(|e| e.antes_do_mini = None);
            let _ = SetWindowPos(hwnd, Some(HWND_NOTOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            let _ = SetWindowPlacement(hwnd, &lugar);
            true
        }
    }
}

fn mostrar(hwnd: HWND) {
    // SAFETY: chamadas simples do Windows sobre a janela do app.
    unsafe {
        let _ = ShowWindow(hwnd, if IsIconic(hwnd).as_bool() { SW_RESTORE } else { SW_SHOW });
        let _ = SetForegroundWindow(hwnd);
    }
}

/// O menu do ícone da bandeja. Devolve o item escolhido (0 = nenhum).
fn menu(hwnd: HWND) -> usize {
    // SAFETY: o menu é criado, mostrado e destruído aqui; os textos são constantes.
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return 0 };
        let item = |id: usize, texto: PCWSTR| {
            let _ = AppendMenuW(menu, MF_STRING, id, texto);
        };
        item(MENU_MOSTRAR, w!("Mostrar o Player"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        item(BOTAO_TOCAR as usize, w!("Tocar ou pausar"));
        item(BOTAO_PROXIMA as usize, w!("Próxima faixa"));
        item(BOTAO_ANTERIOR as usize, w!("Faixa anterior"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        item(MENU_SAIR, w!("Sair"));
        let mut ponto = POINT::default();
        let _ = GetCursorPos(&mut ponto);
        // Sem isto o menu não fecha ao clicar fora dele.
        let _ = SetForegroundWindow(hwnd);
        let escolhido =
            TrackPopupMenu(menu, TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN, ponto.x, ponto.y, None, hwnd, None);
        let _ = DestroyMenu(menu);
        escolhido.0.max(0) as usize
    }
}

fn pedir(id: usize) {
    let acao = match id as u32 {
        BOTAO_ANTERIOR => Acao::Anterior,
        BOTAO_TOCAR => Acao::TocarPausar,
        BOTAO_PROXIMA => Acao::Proxima,
        _ => return,
    };
    com_estado(|e| (e.ao_pedir)(acao));
}

/// Todas as mensagens da janela passam por aqui antes de chegarem ao GPUI.
unsafe extern "system" fn ao_receber(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _dado: usize,
) -> LRESULT {
    let (botao_criado, barra_criada) =
        com_estado(|e| (e.msg_botao_criado, e.msg_barra_criada)).unwrap_or((u32::MAX, u32::MAX));
    match msg {
        WM_COMMAND if (wparam.0 >> 16) & 0xFFFF == THBN_CLICKED => {
            pedir(wparam.0 & 0xFFFF);
            return LRESULT(0);
        }
        MSG_BANDEJA => {
            match lparam.0 as u32 {
                WM_LBUTTONUP => mostrar(hwnd),
                WM_RBUTTONUP => match menu(hwnd) {
                    MENU_MOSTRAR => mostrar(hwnd),
                    MENU_SAIR => {
                        com_estado(|e| e.saindo = true);
                        // SAFETY: pede à própria janela que feche, pelo caminho normal.
                        let _ = unsafe { PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0)) };
                    }
                    outro => pedir(outro),
                },
                _ => {}
            }
            return LRESULT(0);
        }
        MSG_MINI => {
            if !aplicar_mini(wparam.0 == 1) {
                log::warn!("não foi possível trocar para a janela mini (ou voltar dela)");
            }
            if wparam.0 == 2 {
                // SAFETY: maximiza a janela do app, já de volta ao lugar de antes.
                let _ = unsafe { ShowWindow(hwnd, SW_MAXIMIZE) };
            }
            return LRESULT(0);
        }
        WM_CLOSE if com_estado(|e| e.fechar_esconde && !e.saindo).unwrap_or(false) => {
            // SAFETY: esconde a janela do app; ela continua existindo e o ícone da bandeja a traz de volta.
            let _ = unsafe { ShowWindow(hwnd, SW_HIDE) };
            return LRESULT(0);
        }
        WM_GETMINMAXINFO if com_estado(|e| e.antes_do_mini.is_some()).unwrap_or(false) => {
            // SAFETY: nesta mensagem o `lparam` aponta para um MINMAXINFO válido, que o GPUI preenche primeiro.
            unsafe {
                let resposta = DefSubclassProc(hwnd, msg, wparam, lparam);
                if let Some(info) = (lparam.0 as *mut MINMAXINFO).as_mut() {
                    let escala = GetDpiForWindow(hwnd).max(96) as i32;
                    info.ptMinTrackSize = POINT { x: MINI_MINIMO.0 * escala / 96, y: MINI_MINIMO.1 * escala / 96 };
                }
                return resposta;
            }
        }
        WM_NCDESTROY => crate::sistema::remover(),
        _ if msg == botao_criado => {
            com_estado(|e| {
                e.botoes_postos = false;
                por_botoes(e);
            });
        }
        // O Explorer reiniciou: o ícone da bandeja precisa ser posto de novo.
        _ if msg == barra_criada => {
            com_estado(por_na_bandeja);
        }
        _ => {}
    }
    // SAFETY: repassa a mensagem, sem alterações, para o próximo da fila (o GPUI).
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}
