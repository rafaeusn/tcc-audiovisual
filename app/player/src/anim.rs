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
    /// Molas: valor, velocidade e o alvo atual de cada chave.
    molas: RefCell<HashMap<Chave, (f32, f32, f32)>>,
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

    /// Uma mola criticamente amortecida (sem passar do alvo) que guarda a velocidade: ao trocar o alvo no meio do
    /// caminho, o valor muda de rumo sem salto, como os gestos da Apple. `resposta` é o período da mola em segundos
    /// (0,35 é o padrão de abrir e fechar). Nasce no alvo; com movimento reduzido vai direto.
    #[cfg(test)]
    pub fn mola(&self, chave: Chave, alvo: f32, resposta: f32) -> f32 {
        self.mola_amortecida(chave, alvo, resposta, 1.0)
    }

    /// Mola com amortecimento escolhido: 1,0 chega sem passar do alvo; abaixo disso passa um pouco e volta (um
    /// quique, para movimentos com impulso, como a Ilha crescendo).
    pub fn mola_amortecida(&self, chave: Chave, alvo: f32, resposta: f32, amortecimento: f32) -> f32 {
        let mut molas = self.molas.borrow_mut();
        let (x, v, a) = molas.entry(chave).or_insert((alvo, 0.0, alvo));
        *a = alvo;
        if self.parado.get() || resposta <= 0.0 || !x.is_finite() || !v.is_finite() || !alvo.is_finite() {
            *x = if alvo.is_finite() { alvo } else { 0.0 };
            *v = 0.0;
            return *x;
        }
        let w = std::f32::consts::TAU / resposta;
        // Passos curtos: a integração fica estável mesmo num quadro longo.
        let mut resto = self.dt.get();
        while resto > 0.0 {
            let h = resto.min(1.0 / 240.0);
            let aceleracao = w * w * (alvo - *x) - 2.0 * amortecimento.clamp(0.1, 2.0) * w * *v;
            *v += aceleracao * h;
            *x += *v * h;
            resto -= h;
        }
        if (alvo - *x).abs() < 1e-2 && v.abs() < 5e-2 {
            *x = alvo;
            *v = 0.0;
        }
        *x
    }

    /// Põe a mola de `chave` parada num ponto de partida (para ela animar de lá até o próximo alvo).
    pub fn definir_mola(&self, chave: Chave, valor: f32) {
        self.molas.borrow_mut().insert(chave, (valor, 0.0, valor));
    }

    /// Onde a mola de `chave` está agora, sem andar (`None` se ela ainda não existe).
    pub fn mola_atual(&self, chave: Chave) -> Option<f32> {
        self.molas.borrow().get(&chave).map(|(x, _, _)| *x)
    }

    /// A mola de `chave` chegou ao alvo e está parada (ou nunca existiu).
    pub fn parada(&self, chave: Chave) -> bool {
        self.molas.borrow().get(&chave).is_none_or(|(x, v, a)| x == a && *v == 0.0)
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
    fn mola_nasce_no_alvo_e_chega_sem_passar() {
        let a = Animacoes::default();
        a.quadro(1.0 / 60.0, false);
        assert_eq!(a.mola(("m", 0), 0.0, 0.35), 0.0);
        let mut maior = 0.0f32;
        let mut quadros = 0;
        loop {
            maior = maior.max(a.mola(("m", 0), 1.0, 0.35));
            quadros += 1;
            if a.parada(("m", 0)) || quadros >= 600 {
                break;
            }
        }
        assert!(maior <= 1.0, "criticamente amortecida não passa do alvo: {maior}");
        assert!(quadros < 90, "chega em menos de 1,5 s: {quadros} quadros");
        assert_eq!(a.mola(("m", 0), 1.0, 0.35), 1.0);
    }

    #[test]
    fn mola_reverte_sem_salto() {
        let a = Animacoes::default();
        a.quadro(1.0 / 60.0, false);
        a.mola(("m", 0), 0.0, 0.35);
        let mut anterior = 0.0;
        for _ in 0..8 {
            anterior = a.mola(("m", 0), 1.0, 0.35);
        }
        let depois = a.mola(("m", 0), 0.0, 0.35);
        // A velocidade continua: no primeiro quadro depois de inverter, ainda anda um pouco no sentido antigo.
        assert!(depois >= anterior, "não salta para trás: {anterior} -> {depois}");
        for _ in 0..120 {
            a.mola(("m", 0), 0.0, 0.35);
        }
        assert!(a.parada(("m", 0)));
    }

    #[test]
    fn mola_com_quique_passa_e_assenta() {
        let a = Animacoes::default();
        a.quadro(1.0 / 60.0, false);
        a.mola_amortecida(("q", 0), 0.0, 0.5, 0.7);
        let mut maior = 0.0f32;
        for _ in 0..180 {
            maior = maior.max(a.mola_amortecida(("q", 0), 1.0, 0.5, 0.7));
        }
        assert!(maior > 1.0 && maior < 1.15, "passa um pouco do alvo: {maior}");
        assert!(a.parada(("q", 0)), "e assenta em menos de 3 s");
    }

    #[test]
    fn mola_reduzida_vai_direto() {
        let a = Animacoes::default();
        a.quadro(1.0 / 60.0, false);
        a.mola(("m", 0), 0.0, 0.35);
        a.quadro(1.0 / 60.0, true);
        assert_eq!(a.mola(("m", 0), 5.0, 0.35), 5.0);
        assert!(a.parada(("m", 0)));
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
