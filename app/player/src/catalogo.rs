//! O catálogo da Loja: os manifestos dos módulos que vêm com o app (`recursos/modulos/*.toml`).
//!
//! Nesta versão o catálogo é embutido. Os manifestos são os mesmos arquivos que um módulo de fora teria, lidos
//! e conferidos pelo mesmo código, então a tela de instalação e o aceite já funcionam como vão funcionar quando
//! a Loja baixar módulos de terceiros.

use modulos::Manifesto;

/// O módulo da letra: o único que usa a internet, e por isso o primeiro a passar pelo aceite.
pub const ID_LETRA: &str = "player.letra";

pub const ID_LASTFM: &str = "player.lastfm";
pub const ID_DISCORD: &str = "player.discord";

const FONTES: [(&str, &str); 3] = [
    ("letra", include_str!("../../recursos/modulos/letra.toml")),
    ("lastfm", include_str!("../../recursos/modulos/lastfm.toml")),
    ("discord", include_str!("../../recursos/modulos/discord.toml")),
];

/// Os módulos do catálogo, na ordem em que aparecem. Um manifesto com erro fica de fora (e registrado).
pub fn catalogo() -> Vec<Manifesto> {
    FONTES
        .iter()
        .filter_map(|(nome, texto)| {
            Manifesto::ler(texto).map_err(|p| log::error!("manifesto {nome} inválido: {}", p.join("; "))).ok()
        })
        .collect()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn todos_os_manifestos_embutidos_sao_validos() {
        let c = catalogo();
        assert_eq!(c.len(), FONTES.len());
        let mut ids: Vec<&str> = c.iter().map(|m| m.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), c.len(), "ids únicos");
        let letra = c.iter().find(|m| m.id == ID_LETRA);
        assert!(letra.is_some_and(|m| m.pronto && !m.tem_critica()), "a letra pode ser instalada");
        assert!(c.iter().all(|m| m.pronto), "todos os módulos do catálogo têm código");
        // Entrar numa conta e falar com outro programa são ações críticas: a ficha destaca.
        assert!(c.iter().find(|m| m.id == ID_LASTFM).is_some_and(modulos::Manifesto::tem_critica));
        assert!(c.iter().find(|m| m.id == ID_DISCORD).is_some_and(modulos::Manifesto::tem_critica));
    }
}
