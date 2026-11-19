//! O volume do sistema no dock: mudar, silenciar e arrastar (também os gestos de arrastar da janela).

use super::*;

impl Player {
    pub(crate) fn mudar_volume(&mut self, alvo: f32) {
        if !alvo.is_finite() {
            return;
        }
        let alvo = alvo.clamp(0.0, 1.0);
        if audio::volume::definir(alvo) {
            self.volume = alvo;
            self.volume_disponivel = true;
            // Mexer no volume com o som mudo devolve o som, como no sistema.
            if self.mudo && alvo > 0.0 && audio::volume::silenciar(false) {
                self.mudo = false;
            }
        }
    }

    pub(super) fn silenciar(&mut self, mudo: bool) {
        if audio::volume::silenciar(mudo) {
            self.mudo = mudo;
        }
    }

    /// Leva o volume até onde o ponteiro está na pista. Além das pontas a barra estica com resistência.
    pub(super) fn volume_no_ponteiro(&mut self, x: Pixels) {
        let pista = self.pista_volume.get();
        let largura = f32::from(pista.size.width);
        if largura < 1.0 {
            return;
        }
        let pos = f32::from(x - pista.origin.x);
        let sobra = if pos < 0.0 { pos } else { (pos - largura).max(0.0) };
        // O esticão cabe na margem do controle (8 px à direita, 4 à esquerda): nunca passa da borda do dock.
        self.volume_esticado = elastico(sobra, ESTICA_VOLUME).clamp(-4.0, ESTICA_VOLUME);
        self.mudar_volume(pos / largura);
    }

    pub(crate) fn pegar_volume(&mut self, x: Pixels) {
        if self.volume_disponivel {
            self.arrastando_volume = true;
            self.volume_no_ponteiro(x);
        }
    }

    /// O ponteiro se moveu em qualquer lugar da janela. Devolve se algo mudou.
    pub(super) fn ao_mover(&mut self, pos: Point<Pixels>) -> bool {
        self.ponteiro_em = Instant::now();
        let mut mudou = false;
        // O botão da dica saiu de baixo do ponteiro sem avisar (a tela mudou, por exemplo): a dica some.
        if let Some(d) = &self.dica
            && !self.areas.borrow().get(d.id).is_some_and(|a| a.contains(&pos))
        {
            self.dica = None;
            mudou = true;
        }
        if self.arrastando_volume {
            self.volume_no_ponteiro(pos.x);
            mudou = true;
        }
        if let Some(a) = &mut self.arrasto {
            a.atual = pos;
            a.alvo = self.limites.borrow().iter().find(|(m, l)| **m != a.modulo && l.contains(&pos)).map(|(m, _)| *m);
            mudou = true;
        }
        mudou
    }

    /// O botão do mouse foi solto em qualquer lugar da janela.
    pub(super) fn ao_soltar(&mut self) -> bool {
        let mudou = self.pressionado.is_some() || self.arrastando_volume || self.arrasto.is_some();
        self.pressionado = None;
        if self.arrastando_volume {
            self.arrastando_volume = false;
            // A barra volta do ponto em que estava esticada.
            self.anim.definir(("volume-esticado", 0), self.volume_esticado);
            self.volume_esticado = 0.0;
        }
        if let Some(a) = self.arrasto.take() {
            self.soltar_modulo(a);
        }
        mudou
    }
}
