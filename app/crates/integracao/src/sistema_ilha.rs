//! A conversa com o Windows para a janela da Ilha. O código `unsafe` da Ilha fica todo neste arquivo.

use std::ffi::c_void;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DWMWA_BORDER_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_DONOTROUND, DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    ClientToScreen, CreateRectRgn, DeleteObject, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR,
    MONITOR_DEFAULTTONEAREST, MONITORINFO, MONITORINFOEXW, MonitorFromPoint, MonitorFromWindow, SetWindowRgn,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON};
use windows::Win32::UI::Shell::{
    DefSubclassProc, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN, RemoveWindowSubclass,
    SHQueryUserNotificationState, SetWindowSubclass,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GA_ROOTOWNER, GWL_EXSTYLE, GWL_STYLE, GetAncestor, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW,
    GetWindowRect, HTCLIENT, HWND_TOPMOST, IsIconic, IsWindowVisible, IsZoomed, MA_NOACTIVATE, PostMessageW,
    SPI_GETCLIENTAREAANIMATION, SW_HIDE, SW_SHOWNOACTIVATE, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, WM_APP, WM_MOUSEACTIVATE, WM_NCCALCSIZE,
    WM_NCDESTROY, WM_NCHITTEST, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_VISIBLE,
};
use windows::core::BOOL;

/// Número da subclasse na janela da Ilha.
const ID: usize = 0x494C;
/// Mensagem nossa: mostrar (wparam = 1) ou esconder (0).
/// Mensagem nossa: recalcular a moldura (a área de desenho vira a janela inteira). Vai por mensagem porque, chamado
/// direto enquanto o GPUI cria a janela, o recálculo manda avisos de tamanho que o GPUI não consegue atender ali
/// (e registra "RefCell already borrowed").
const MSG_MOLDURA: u32 = WM_APP + 0x4B1;
const MSG_MOSTRAR: u32 = WM_APP + 0x4B2;
/// Mensagem nossa: a região que recebe cliques (wparam = altura, lparam = esquerda | direita << 16, lógicos).
const MSG_REGIAO: u32 = WM_APP + 0x4B3;
/// Mensagem nossa: mover (wparam = x | y << 16 em 16 bits com sinal, lparam = largura | altura << 16, lógicos).
const MSG_MOVER: u32 = WM_APP + 0x4B4;
/// "Sem borda" para a cor da borda do Windows 11.
const SEM_BORDA: u32 = 0xFFFF_FFFE;

fn janela(hwnd: isize) -> HWND {
    HWND(hwnd as *mut c_void)
}

pub fn preparar(hwnd: isize) -> bool {
    let hwnd = janela(hwnd);
    // SAFETY: chamadas simples do Windows sobre a janela da Ilha, criada pelo GPUI nesta thread.
    unsafe {
        let visivel = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & WS_VISIBLE.0;
        SetWindowLongPtrW(hwnd, GWL_STYLE, (WS_POPUP.0 | visivel) as isize);
        let ex = GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32;
        SetWindowLongPtrW(hwnd, GWL_EXSTYLE, (ex | WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0 | WS_EX_TOPMOST.0) as isize);
        // A pílula desenha a própria forma: nada de cantos arredondados nem borda do Windows 11 em volta.
        let cantos = DWMWCP_DONOTROUND;
        let _ = DwmSetWindowAttribute(
            hwnd,
            DWMWA_WINDOW_CORNER_PREFERENCE,
            (&raw const cantos).cast(),
            size_of_val(&cantos) as u32,
        );
        let borda = SEM_BORDA;
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, (&raw const borda).cast(), size_of_val(&borda) as u32);
        // A subclasse entra antes de recalcular a moldura: é ela que diz que a área de desenho é a janela inteira. O
        // recálculo em si vai por mensagem (ver `MSG_MOLDURA`).
        if !SetWindowSubclass(hwnd, Some(ao_receber), ID, 0).as_bool() {
            return false;
        }
        let _ = PostMessageW(Some(hwnd), MSG_MOLDURA, WPARAM(0), LPARAM(0));
        true
    }
}

pub fn mostrar(hwnd: isize, visivel: bool) {
    // SAFETY: põe uma mensagem nossa na fila da janela da Ilha.
    let _ = unsafe { PostMessageW(Some(janela(hwnd)), MSG_MOSTRAR, WPARAM(usize::from(visivel)), LPARAM(0)) };
}

pub fn tela_cheia_de_outro() -> bool {
    // SAFETY: só pergunta ao Windows o estado de "não perturbe" da sessão.
    match unsafe { SHQueryUserNotificationState() } {
        Ok(estado) => [QUNS_BUSY, QUNS_RUNNING_D3D_FULL_SCREEN, QUNS_PRESENTATION_MODE].contains(&estado),
        Err(_) => false,
    }
}

pub fn regiao(hwnd: isize, esquerda: f32, topo: f32, direita: f32, baixo: f32) {
    let v = |x: f32| x.clamp(0.0, 8000.0) as usize;
    let (e, t, d, b) = (v(esquerda), v(topo), v(direita), v(baixo));
    // SAFETY: põe uma mensagem nossa na fila da janela da Ilha.
    let _ =
        unsafe { PostMessageW(Some(janela(hwnd)), MSG_REGIAO, WPARAM(t | b << 16), LPARAM((e | d << 16) as isize)) };
}

pub fn ponteiro_relativo(hwnd: isize) -> Option<(f32, f32)> {
    let hwnd = janela(hwnd);
    // SAFETY: só pergunta onde está o ponteiro, o retângulo e o DPI da janela.
    unsafe {
        let mut ponto = POINT::default();
        let mut caixa = RECT::default();
        if GetCursorPos(&mut ponto).is_err() || GetWindowRect(hwnd, &mut caixa).is_err() {
            return None;
        }
        let escala = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
        Some(((ponto.x - caixa.left) as f32 / escala, (ponto.y - caixa.top) as f32 / escala))
    }
}

/// Lê o retângulo, a área de trabalho, o DPI e o nome de um monitor.
fn ler_monitor(monitor: HMONITOR) -> Option<crate::ilha::Monitor> {
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = size_of::<MONITORINFOEXW>() as u32;
    // SAFETY: a estrutura estendida é local e começa pela comum; o tamanho dela vai em `cbSize`, como o Windows pede.
    unsafe {
        if !GetMonitorInfoW(monitor, (&raw mut info).cast()).as_bool() {
            return None;
        }
        let (mut dpi, mut dpi_y) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y);
        let (r, t) = (info.monitorInfo.rcMonitor, info.monitorInfo.rcWork);
        let fim = info.szDevice.iter().position(|c| *c == 0).unwrap_or(info.szDevice.len());
        Some(crate::ilha::Monitor {
            esquerda: r.left,
            topo: r.top,
            direita: r.right,
            baixo: r.bottom,
            area: (t.left, t.top, t.right, t.bottom),
            escala: dpi.max(96) as f32 / 96.0,
            nome: String::from_utf16_lossy(&info.szDevice[..fim]),
            // MONITORINFOF_PRIMARY
            principal: info.monitorInfo.dwFlags & 1 != 0,
        })
    }
}

pub fn monitor(hwnd: isize) -> Option<crate::ilha::Monitor> {
    // SAFETY: só pergunta em que monitor a janela está.
    ler_monitor(unsafe { MonitorFromWindow(janela(hwnd), MONITOR_DEFAULTTONEAREST) })
}

pub fn monitor_no_ponto(x: i32, y: i32) -> Option<crate::ilha::Monitor> {
    // SAFETY: só pergunta em que monitor está o ponto.
    ler_monitor(unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) })
}

pub fn monitores() -> Vec<crate::ilha::Monitor> {
    unsafe extern "system" fn cada(monitor: HMONITOR, _: HDC, _: *mut RECT, dado: LPARAM) -> BOOL {
        // SAFETY: `dado` é o endereço do vetor abaixo, vivo durante toda a enumeração (que é síncrona).
        let lista = unsafe { &mut *(dado.0 as *mut Vec<crate::ilha::Monitor>) };
        lista.extend(ler_monitor(monitor));
        BOOL(1)
    }
    let mut lista: Vec<crate::ilha::Monitor> = Vec::new();
    // SAFETY: a função de volta só é chamada durante esta chamada, com o endereço do vetor local.
    let _ = unsafe { EnumDisplayMonitors(None, None, Some(cada), LPARAM((&raw mut lista) as isize)) };
    lista
}

pub fn mover(hwnd: isize, x: i32, y: i32, largura: f32, altura: f32) {
    let pos = (x as i16 as u16 as usize) | ((y as i16 as u16 as usize) << 16);
    let w = largura.clamp(1.0, 4000.0) as usize;
    let h = altura.clamp(1.0, 4000.0) as usize;
    // SAFETY: põe uma mensagem nossa na fila da janela da Ilha.
    let _ = unsafe { PostMessageW(Some(janela(hwnd)), MSG_MOVER, WPARAM(pos), LPARAM((w | h << 16) as isize)) };
}

pub fn na_frente_ocupando_a_tela(hwnd: isize) -> bool {
    let hwnd = janela(hwnd);
    // SAFETY: perguntas simples ao Windows sobre janelas existentes, com estruturas locais.
    unsafe {
        if hwnd.0.is_null() || !IsWindowVisible(hwnd).as_bool() || IsIconic(hwnd).as_bool() {
            return false;
        }
        let frente = GetForegroundWindow();
        if frente != hwnd && GetAncestor(frente, GA_ROOTOWNER) != hwnd {
            return false;
        }
        if IsZoomed(hwnd).as_bool() {
            return true;
        }
        // Tela cheia: a janela cobre o monitor inteiro.
        let mut caixa = RECT::default();
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, ..Default::default() };
        if GetWindowRect(hwnd, &mut caixa).is_err() || !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return false;
        }
        let m = info.rcMonitor;
        caixa.left <= m.left && caixa.top <= m.top && caixa.right >= m.right && caixa.bottom >= m.bottom
    }
}

pub fn botao_apertado() -> bool {
    // SAFETY: só lê o estado do botão do mouse.
    (unsafe { GetAsyncKeyState(VK_LBUTTON.0 as i32) } as u16 & 0x8000) != 0
}

pub fn cursor() -> Option<(i32, i32)> {
    let mut ponto = POINT::default();
    // SAFETY: só pergunta onde o ponteiro está.
    unsafe { GetCursorPos(&mut ponto) }.ok().map(|()| (ponto.x, ponto.y))
}

pub fn movimento_reduzido() -> bool {
    let mut animar = BOOL(1);
    // SAFETY: lê uma configuração do sistema para uma variável local do tamanho certo.
    let lido = unsafe {
        SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, Some((&raw mut animar).cast()), Default::default())
    };
    lido.is_ok() && !animar.as_bool()
}

pub fn ponteiro_dentro(hwnd: isize) -> bool {
    // SAFETY: só pergunta onde está o ponteiro e o retângulo da janela.
    unsafe {
        let mut ponto = POINT::default();
        let mut caixa = RECT::default();
        if GetCursorPos(&mut ponto).is_err() || GetWindowRect(janela(hwnd), &mut caixa).is_err() {
            return false;
        }
        ponto.x >= caixa.left && ponto.x < caixa.right && ponto.y >= caixa.top && ponto.y < caixa.bottom
    }
}

/// As mensagens da janela da Ilha passam por aqui antes de chegarem ao GPUI.
unsafe extern "system" fn ao_receber(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    _id: usize,
    _dado: usize,
) -> LRESULT {
    match msg {
        MSG_MOLDURA => {
            // SAFETY: recalcula a moldura da própria janela, sem mover nem ativar.
            let _ = unsafe {
                SetWindowPos(
                    hwnd,
                    Some(HWND_TOPMOST),
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_FRAMECHANGED,
                )
            };
            return LRESULT(0);
        }
        // Clicar na Ilha não tira o foco do app que a pessoa está usando.
        WM_MOUSEACTIVATE => return LRESULT(MA_NOACTIVATE as isize),
        // Sem moldura nenhuma: o GPUI reserva uma borda de redimensionar em janelas sem barra de título, e aqui a
        // área de desenho tem que ser a janela inteira (senão a Ilha não encosta no topo e perde as pontas).
        WM_NCCALCSIZE if wparam.0 != 0 => return LRESULT(0),
        // Nenhuma parte da Ilha é borda de redimensionar nem barra de título.
        WM_NCHITTEST => return LRESULT(HTCLIENT as isize),
        MSG_MOVER => {
            let x = (wparam.0 & 0xFFFF) as u16 as i16 as i32;
            let y = ((wparam.0 >> 16) & 0xFFFF) as u16 as i16 as i32;
            let largura = (lparam.0 as usize & 0xFFFF) as f32;
            let altura = ((lparam.0 as usize >> 16) & 0xFFFF) as f32;
            // SAFETY: move a própria janela; as estruturas são locais.
            unsafe {
                // A escala do monitor para onde ela vai (indo para outro monitor, a janela ainda tem a do de antes).
                let destino = MonitorFromPoint(POINT { x: x + 1, y: y + 1 }, MONITOR_DEFAULTTONEAREST);
                let (mut dpi, mut dpi_y) = (GetDpiForWindow(hwnd).max(96), 96u32);
                let _ = GetDpiForMonitor(destino, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y);
                let escala = dpi.max(96) as f32 / 96.0;
                let (w, h) = ((largura * escala).round() as i32, (altura * escala).round() as i32);
                let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, w, h, SWP_NOACTIVATE);
                // Se a área desenhada não começa no topo da janela (alguma borda do sistema), sobe a janela o
                // tanto que falta para a Ilha encostar de verdade na borda da tela.
                let mut origem = POINT::default();
                let mut caixa = RECT::default();
                if ClientToScreen(hwnd, &mut origem).as_bool() && GetWindowRect(hwnd, &mut caixa).is_ok() {
                    let (dx, dy) = (origem.x - caixa.left, origem.y - caixa.top);
                    if dx != 0 || dy != 0 {
                        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x - dx, y - dy, w, h, SWP_NOACTIVATE);
                    }
                }
            }
            return LRESULT(0);
        }
        MSG_REGIAO => {
            let topo = (wparam.0 & 0xFFFF) as f32;
            let baixo = ((wparam.0 >> 16) & 0xFFFF) as f32;
            let esquerda = (lparam.0 as usize & 0xFFFF) as f32;
            let direita = ((lparam.0 as usize >> 16) & 0xFFFF) as f32;
            // SAFETY: cria uma região retangular e a entrega ao Windows, que passa a ser o dono dela. Sem pedir
            // redesenho: quem desenha é o DirectComposition, e o redesenho a cada troca fazia a forma piscar.
            unsafe {
                let escala = GetDpiForWindow(hwnd).max(96) as f32 / 96.0;
                let rgn = CreateRectRgn(
                    (esquerda * escala).floor() as i32,
                    (topo * escala).floor() as i32,
                    (direita * escala).ceil() as i32,
                    (baixo * escala).ceil() as i32,
                );
                if SetWindowRgn(hwnd, Some(rgn), false) == 0 {
                    let _ = DeleteObject(rgn.into());
                }
            }
            return LRESULT(0);
        }
        MSG_MOSTRAR => {
            // SAFETY: mostra sem ativar (e reafirma "sempre por cima") ou esconde a própria janela.
            unsafe {
                if wparam.0 == 1 {
                    let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                    let _ =
                        SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
                } else {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                }
            }
            return LRESULT(0);
        }
        WM_NCDESTROY => {
            // SAFETY: desfaz a subclasse com os mesmos identificadores, antes de a janela sumir.
            let _ = unsafe { RemoveWindowSubclass(hwnd, Some(ao_receber), ID) };
        }
        _ => {}
    }
    // SAFETY: repassa a mensagem, sem alterações, para o próximo da fila (o GPUI).
    unsafe { DefSubclassProc(hwnd, msg, wparam, lparam) }
}
