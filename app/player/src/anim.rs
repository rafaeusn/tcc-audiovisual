//! Animações da interface: valores que perseguem um alvo, quadro a quadro.
//!
//! A interface é redesenhada a cada quadro (o visual pede isso), então animar é só guardar o valor atual de cada
//! coisa e aproximá-lo do alvo um pouco por quadro. A aproximação é exponencial: sai rápido e chega devagar
//! (a "curva de saída forte" das micro-interações), sem quique, e pode ser interrompida a qualquer momento.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

/// (nome, índice): o índice separa itens repetidos (o dígito 2 do BPM, o módulo 3).
pub type Chave = (&'static str, u32);

#[derive(Default)]
pub struct Animacoes {
    dt: Cell<f32>,
    valores: RefCell<HashMap<Chave, f32>>,
    /// Movimento reduzido: tudo vai direto para o alvo.
    parado: Cell<bool>,
}

impl Animacoes {
    /// Chamado uma vez por quadro, antes de desenhar.
    pub fn quadro(&self, dt: f32, parado: bool) {
        self.dt.set(if dt.is_finite() { dt.clamp(0.0, 0.1) } else { 0.0 });
        self.parado.set(parado);
    }

    /// Aproxima o valor de `chave` do `alvo` e devolve o valor deste quadro. `tempo` é quanto leva para chegar
    /// (99% do caminho), em segundos. Na primeira vez o valor já nasce no alvo. Chamar uma vez por quadro por chave.
    pub fn seguir(&self, chave: Chave, alvo: f32, tempo: f32) -> f32 {
        let mut valores = self.valores.borrow_mut();
        let atual = valores.entry(chave).or_insert(alvo);
        if self.parado.get() || tempo <= 0.0 || !atual.is_finite() || !alvo.is_finite() {
            *atual = if alvo.is_finite() { alvo } else { 0.0 };
        } else {
            let k = 1.0 - (-self.dt.get() * 4.6 / tempo).exp();
            *atual += (alvo - *atual) * k;
            if (alvo - *atual).abs() < 1e-3 {
                *atual = alvo;
            }
        }
        *atual
    }

    /// Põe o valor de `chave` num ponto de partida (para ele animar de lá até o alvo).
    pub fn definir(&self, chave: Chave, valor: f32) {
        self.valores.borrow_mut().insert(chave, valor);
    }
}

/// Resistência elástica ao puxar além do limite (a mesma conta do "elástico" da Apple): quanto mais longe,
/// menos anda. `sobra` é quanto passou do limite e `tamanho` o comprimento da pista, em pixels.
pub fn elastico(sobra: f32, tamanho: f32) -> f32 {
    const C: f32 = 0.55;
    if tamanho <= 0.0 || !sobra.is_finite() {
        return 0.0;
    }
    sobra.signum() * (sobra.abs() * tamanho * C) / (tamanho + C * sobra.abs())
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn nasce_no_alvo_e_persegue_depois() {
        let a = Animacoes::default();
        a.quadro(1.0 / 60.0, false);
        assert_eq!(a.seguir(("x", 0), 1.0, 0.2), 1.0);
        let v1 = a.seguir(("x", 0), 0.0, 0.2);
        assert!(v1 < 1.0 && v1 > 0.5, "anda um pouco por quadro: {v1}");
        for _ in 0..60 {
            a.seguir(("x", 0), 0.0, 0.2);
        }
        assert_eq!(a.seguir(("x", 0), 0.0, 0.2), 0.0, "chega e para exatamente no alvo");
    }

    #[test]
    fn movimento_reduzido_vai_direto() {
        let a = Animacoes::default();
        a.quadro(1.0 / 60.0, true);
        a.definir(("x", 0), 5.0);
        assert_eq!(a.seguir(("x", 0), 2.0, 0.3), 2.0);
    }

    #[test]
    fn valores_invalidos_nao_se_espalham() {
        let a = Animacoes::default();
        a.quadro(f32::NAN, false);
        a.definir(("x", 0), f32::NAN);
        assert_eq!(a.seguir(("x", 0), 1.0, 0.2), 1.0);
        assert_eq!(a.seguir(("y", 0), f32::INFINITY, 0.2), 0.0);
    }

    #[test]
    fn elastico_resiste_cada_vez_mais() {
        let pouco = elastico(10.0, 112.0);
        let muito = elastico(100.0, 112.0);
        assert!(pouco > 0.0 && pouco < 10.0);
        assert!(muito < 100.0 * 0.55 && muito > pouco);
        assert_eq!(elastico(-10.0, 112.0), -pouco);
        assert_eq!(elastico(10.0, 0.0), 0.0);
    }
}
