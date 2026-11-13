//! Recursos embutidos no executável (ícones). O GPUI pede por caminho; quem responde é a tabela gerada no build.

use gpui::{AssetSource, Result, SharedString};
use std::borrow::Cow;

include!(concat!(env!("OUT_DIR"), "/icones.rs"));

pub struct Recursos;

impl AssetSource for Recursos {
    fn load(&self, caminho: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(icone(caminho).map(Cow::Borrowed))
    }

    fn list(&self, _caminho: &str) -> Result<Vec<SharedString>> {
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn icones_usados_pela_interface_existem() {
        for nome in [
            "bolt",
            "check",
            "drop",
            "expand",
            "eye",
            "full",
            "globe",
            "headphones",
            "history",
            "home",
            "home-f",
            "info",
            "keyboard",
            "lib",
            "lib-f",
            "logo",
            "lyrics",
            "max",
            "metro",
            "min",
            "minus",
            "next",
            "panel",
            "pause",
            "pip",
            "play",
            "prev",
            "shield",
            "sliders",
            "store",
            "store-f",
            "tone",
            "wave",
            "x",
        ] {
            let svg = icone(&format!("icones/{nome}.svg"));
            assert!(svg.is_some_and(|b| b.starts_with(b"<svg")), "ícone ausente: {nome}");
        }
        for nome in
            ["search", "grid", "plus", "vol", "mute", "grip", "swap", "clock", "pack", "puzzle", "layout", "skin"]
        {
            assert!(icone(&format!("icones/{nome}.svg")).is_some(), "ícone ausente: {nome}");
        }
        assert!(icone("icones/nao-existe.svg").is_none());
        assert!(icone("../segredo").is_none());
    }

    #[test]
    fn fontes_embutidas_sao_truetype() {
        let fontes = fontes();
        assert!(fontes.len() >= 7, "esperava as 7 fontes da interface, achei {}", fontes.len());
        for f in fontes {
            assert!(f.starts_with(&[0, 1, 0, 0]), "arquivo de fonte inválido");
        }
    }
}
