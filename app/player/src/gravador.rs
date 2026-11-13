//! Grava os ajustes em disco fora da thread da interface. Vários pedidos seguidos viram uma gravação só.

use nucleo::Ajustes;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};

pub struct Gravador {
    fila: Option<Sender<Ajustes>>,
}

impl Gravador {
    /// `caminho = None` (pasta de dados indisponível) desliga a gravação: o app funciona, só não lembra.
    pub fn iniciar(caminho: Option<PathBuf>) -> Self {
        let Some(caminho) = caminho else { return Self { fila: None } };
        let (fila, pedidos) = channel::<Ajustes>();
        let criada = std::thread::Builder::new().name("gravador".into()).spawn(move || {
            while let Ok(mut ajustes) = pedidos.recv() {
                // Só o pedido mais novo interessa.
                while let Ok(mais_novo) = pedidos.try_recv() {
                    ajustes = mais_novo;
                }
                if let Err(e) = ajustes.salvar(&caminho) {
                    log::warn!("não foi possível salvar os ajustes: {e}");
                }
            }
        });
        match criada {
            Ok(_) => Self { fila: Some(fila) },
            Err(e) => {
                log::warn!("gravação de ajustes desligada: {e}");
                Self { fila: None }
            }
        }
    }

    pub fn salvar(&self, ajustes: &Ajustes) {
        if let Some(f) = &self.fila
            && f.send(ajustes.clone()).is_err()
        {
            log::warn!("a thread de gravação parou; ajustes não foram salvos");
        }
    }
}
