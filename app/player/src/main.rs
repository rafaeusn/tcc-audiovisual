//! Player: visualizador de mÃºsica em tempo real (Modo Escuta), com interface em GPUI.
//!
//! Ordem de partida: registro â†’ ajustes â†’ uma instÃ¢ncia sÃ³ â†’ janela. Nada aqui derruba o app: sem pasta de
//! dados ele abre sem lembrar ajustes, sem placa de vÃ­deo compatÃ­vel ele abre sem o visual, sem Ã¡udio ele espera.

// Em release, sem janela de terminal atrÃ¡s do app.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod anim;
mod app;
mod arranjo;
mod cadeia;
mod capas;
mod catalogo;
mod contas;
mod diagnostico;
mod diario;
mod escuta;
mod espacos;
mod externo;
mod fila;
mod gravador;
mod ilha;
mod letrista;
mod local;
mod mesa;
mod navegacao;
mod paginas;
mod pares;
mod preanalista;
mod recursos;
mod remoto;
mod servicos;
mod tema;
mod ui;

use gpui::{App, Application, Bounds, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px, size};
use nucleo::Ajustes;

/// Menor janela em que o arranjo ainda cabe (ver `arranjo::calcular`).
const JANELA_MINIMA: (f32, f32) = (360.0, 560.0);

/// Garante uma instÃ¢ncia sÃ³: duas ouvindo e desenhando ao mesmo tempo sÃ³ gastariam a mÃ¡quina.
/// Devolve `None` se o Player jÃ¡ estÃ¡ aberto. O guarda precisa viver atÃ© o fim do programa.
#[cfg(windows)]
fn instancia_unica() -> Option<impl Drop> {
    use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
    use windows::Win32::System::Threading::CreateMutexW;
    use windows::core::w;

    struct Guarda(HANDLE);
    impl Drop for Guarda {
        fn drop(&mut self) {
            // SAFETY: o handle veio de `CreateMutexW` e sÃ³ Ã© fechado aqui, uma vez.
            let _ = unsafe { CloseHandle(self.0) };
        }
    }

    // SAFETY: chamada simples da API do Windows, com um nome constante terminado em nulo.
    let criado = unsafe { CreateMutexW(None, false, w!("Local\\TccPlayer.instancia-unica")) };
    match criado {
        Ok(handle) => {
            // SAFETY: lÃª o cÃ³digo de erro da chamada acima, na mesma thread.
            let ja_existe = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            let guarda = Guarda(handle);
            (!ja_existe).then_some(guarda)
        }
        Err(e) => {
            // Sem o mutex nÃ£o dÃ¡ para saber: Ã© melhor abrir do que recusar.
            log::warn!("nÃ£o foi possÃ­vel checar se o Player jÃ¡ estÃ¡ aberto: {e}");
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

    // Um pÃ¢nico em qualquer thread fica registrado (em release nÃ£o hÃ¡ terminal para ver).
    let padrao = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        log::error!("pÃ¢nico: {info}");
        log::logger().flush();
        padrao(info);
    }));

    // Com outra pasta de dados (`TCCPLAYER_DADOS`) esta cÃ³pia nÃ£o disputa nada com a que jÃ¡ estiver aberta.
    let _instancia = if nucleo::caminhos::pasta_alternativa().is_some() {
        None
    } else {
        let Some(guarda) = instancia_unica() else {
            log::info!("o Player jÃ¡ estÃ¡ aberto; encerrando esta cÃ³pia");
            return;
        };
        Some(guarda)
    };

    let arquivo_ajustes = match nucleo::caminhos::arquivo_ajustes() {
        Ok(caminho) => Some(caminho),
        Err(e) => {
            log::warn!("pasta de dados indisponÃ­vel ({e}); os ajustes nÃ£o serÃ£o lembrados");
            None
        }
    };
    let ajustes = arquivo_ajustes.as_deref().map(Ajustes::carregar).unwrap_or_default();
    let gravador = gravador::Gravador::iniciar(arquivo_ajustes);
    // Sem pasta de dados o app abre do mesmo jeito, sÃ³ sem histÃ³rico e sem guardar as letras.
    let banco = nucleo::caminhos::arquivo_banco().ok();

    Application::new().with_assets(recursos::Recursos).run(move |cx: &mut App| {
        // Sem as fontes o app abre do mesmo jeito, com a fonte do sistema.
        let fontes = recursos::fontes().into_iter().map(std::borrow::Cow::Borrowed).collect();
        if let Err(e) = cx.text_system().add_fonts(fontes) {
            log::warn!("nÃ£o foi possÃ­vel carregar as fontes da interface: {e}");
        }
        let tamanho = size(px(ajustes.janela_largura), px(ajustes.janela_altura));
        let opcoes = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(Bounds::centered(None, tamanho, cx))),
            // Sem a barra do sistema: o topo do app arrasta a janela e tem os botÃµes de minimizar e fechar.
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
        let principal = match aberta {
            Ok(janela) => janela.window_id(),
            Err(e) => {
                log::error!("nÃ£o foi possÃ­vel abrir a janela: {e}");
                cx.quit();
                return;
            }
        };
        // SÃ³ a janela principal encerra o app (fechar a Ilha nÃ£o); com ela, a Ilha vai junto.
        cx.on_window_closed(move |cx| {
            if !cx.windows().iter().any(|w| w.window_id() == principal) {
                cx.quit();
            }
        })
        .detach();
        cx.activate(true);
    });
    log::info!("Player encerrado");
    log::logger().flush();
}
