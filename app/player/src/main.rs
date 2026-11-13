//! Player: visualizador de música em tempo real (Modo Escuta), com interface em GPUI.
//!
//! Ordem de partida: registro → ajustes → uma instância só → janela. Nada aqui derruba o app: sem pasta de
//! dados ele abre sem lembrar ajustes, sem placa de vídeo compatível ele abre sem o visual, sem áudio ele espera.

// Em release, sem janela de terminal atrás do app.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod anim;
mod app;
mod arranjo;
mod catalogo;
mod diagnostico;
mod diario;
mod escuta;
mod gravador;
mod letrista;
mod local;
mod recursos;
mod servicos;
mod tema;
mod ui;

use gpui::{App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size};
use nucleo::Ajustes;

/// Menor janela em que o arranjo ainda cabe (ver `arranjo::calcular`).
const JANELA_MINIMA: (f32, f32) = (360.0, 560.0);

/// Garante uma instância só: duas ouvindo e desenhando ao mesmo tempo só gastariam a máquina.
/// Devolve `None` se o Player já está aberto. O guarda precisa viver até o fim do programa.
#[cfg(windows)]
fn instancia_unica() -> Option<impl Drop> {
    use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::w;

    struct Guarda(HANDLE);
    impl Drop for Guarda {
        fn drop(&mut self) {
            // SAFETY: o handle veio de `CreateMutexW` e só é fechado aqui, uma vez.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    // SAFETY: chamada simples da API do Windows, com um nome constante terminado em nulo.
    let criado = unsafe { CreateMutexW(None, false, w!("Local\\TccPlayer.instancia-unica")) };
    match criado {
        Ok(handle) => {
            // SAFETY: lê o código de erro da chamada acima, na mesma thread.
            let ja_existe = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            let guarda = Guarda(handle);
            (!ja_existe).then_some(guarda)
        }
        Err(e) => {
            // Sem o mutex não dá para saber: é melhor abrir do que recusar.
            log::warn!("não foi possível checar se o Player já está aberto: {e}");
            Some(Guarda(HANDLE::default()))
        }
    }
}

#[cfg(not(windows))]
fn instancia_unica() -> Option<impl Drop> {
    struct Guarda;
    impl Drop for Guarda {
        fn drop(&mut self) {}
    }
    Some(Guarda)
}

fn main() {
    let pasta_registros = nucleo::caminhos::pasta_registros().ok();
    nucleo::registro::iniciar(pasta_registros.as_deref());
    log::info!("Player {} iniciando", env!("CARGO_PKG_VERSION"));

    // Um pânico em qualquer thread fica registrado (em release não há terminal para ver).
    let padrao = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("pânico: {info}");
        log::logger().flush();
        padrao(info);
    }));

    // Com outra pasta de dados (`TCCPLAYER_DADOS`) esta cópia não disputa nada com a que já estiver aberta.
    let _instancia = if nucleo::caminhos::pasta_alternativa().is_some() {
        None
    } else {
        let Some(guarda) = instancia_unica() else {
            log::info!("o Player já está aberto; encerrando esta cópia");
            return;
        };
        Some(guarda)
    };

    let arquivo_ajustes = match nucleo::caminhos::arquivo_ajustes() {
        Ok(caminho) => Some(caminho),
        Err(e) => {
            log::warn!("pasta de dados indisponível ({e}); os ajustes não serão lembrados");
            None
        }
    };
    let ajustes = arquivo_ajustes.as_deref().map(Ajustes::carregar).unwrap_or_default();
    let gravador = gravador::Gravador::iniciar(arquivo_ajustes);
    // Sem pasta de dados o app abre do mesmo jeito, só sem histórico e sem guardar as letras.
    let banco = nucleo::caminhos::arquivo_banco().ok();

    Application::new().with_assets(recursos::Recursos).run(move |cx: &mut App| {
        // Sem as fontes o app abre do mesmo jeito, com a fonte do sistema.
        let fontes = recursos::fontes().into_iter().map(std::borrow::Cow::Borrowed).collect();
        if let Err(e) = cx.text_system().add_fonts(fontes) {
            log::warn!("não foi possível carregar as fontes da interface: {e}");
        }
        let tamanho = size(px(ajustes.janela_largura), px(ajustes.janela_altura));
        let opcoes = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, tamanho, cx))),
            // Sem a barra do sistema: o topo do app arrasta a janela e tem os botões de minimizar e fechar.
            titlebar: Some(TitlebarOptions {
                title: Some("Player".into()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            window_min_size: Some(size(px(JANELA_MINIMA.0), px(JANELA_MINIMA.1))),
            app_id: Some("TccPlayer".into()),
            ..Default::default()
        };
        let aberta = cx
            .open_window(opcoes, move |window, cx| cx.new(|cx| app::Player::new(ajustes, gravador, banco, window, cx)));
        if let Err(e) = aberta {
            log::error!("não foi possível abrir a janela: {e}");
            cx.quit();
            return;
        }
        cx.on_window_closed(|cx| cx.quit()).detach();
        cx.activate(true);
    });
    log::info!("Player encerrado");
    log::logger().flush();
}
