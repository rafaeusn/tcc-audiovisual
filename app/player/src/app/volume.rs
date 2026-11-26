//! O volume no dock: mudar, silenciar e arrastar (também os gestos de arrastar da janela). Com a música do Player na tela, é o
//! volume só dela (o resto do PC não muda, e o visual não sente); com a de outro app, é o do sistema.

use super::*;

impl Player {
    pub(crate) fn mudar_volume(&mut self, alvo: f32) {
        if !alvo.is_finite() {
            return;
        }
        let alvo = alvo.clamp(0.0, 1.0);
        if self.volume_e_da_musica() {
            self.ajustes.volume_musica = alvo;
            (self.volume, self.volume_disponivel) = (alvo, true);
            if alvo > 0.0 {
                (self.mudo_da_musica, self.mudo) = (false, false);
            }
            self.aplicar_volume_da_musica();
            self.t_volume_salvo = Some(0.0);
            return;
        }
        if audio::volume::definir(alvo) {
            self.volume = alvo;
            self.volume_disponivel = true;
            // Mexer no volume com o som mudo devolve o som, como no sistema.
            if self.mudo && alvo > 0.0 && audio::volume::silenciar(false) {
                self.mudo = false;
            }
        }
    }

    /// A barra é a da música do Player, e não a do sistema.
    pub(crate) fn volume_e_da_musica(&self) -> bool {
        self.origem_local
    }

    /// Leva o volume da música a quem a toca: o tocador daqui e o programa do serviço.
    pub(super) fn aplicar_volume_da_musica(&self) {
        let v = if self.mudo_da_musica { 0.0 } else { self.ajustes.volume_musica };
        self.local.volume(crate::mesa::ganho_da_barra(v));
        self.mesa.volume(v);
    }

    /// Lê o volume da barra (outro programa pode ter mexido nele, e a barra pode ter trocado de dono).
    pub(super) fn ler_volume(&mut self) {
        if self.volume_e_da_musica() {
            if let Some(v) = self.mesa.mudou_de_fora() {
                // No mudo, o programa está no piso: só uma mudança de verdade vale.
                self.ajustes.volume_musica = v;
                self.mudo_da_musica = false;
                self.aplicar_volume_da_musica();
                self.t_volume_salvo = Some(0.0);
            }
            self.volume = self.ajustes.volume_musica;
            self.mudo = self.mudo_da_musica;
            self.volume_disponivel = true;
            return;
        }
        match audio::volume::ler() {
            Some(n) => {
                self.volume = n.volume;
                self.mudo = n.mudo;
                self.volume_disponivel = true;
            }
            None => self.volume_disponivel = false,
        }
    }

    pub(super) fn silenciar(&mut self, mudo: bool) {
        if self.volume_e_da_musica() {
            (self.mudo_da_musica, self.mudo) = (mudo, mudo);
            self.aplicar_volume_da_musica();
            return;
        }
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
        let (x, y) = (f32::from(pos.x), f32::from(pos.y));
        let mut mudou = self.descanso.mexeu(x, y, self.ponteiro_em);
        if self.arrastando_divisao {
            let x = x / crate::ui::medidas::escala();
            self.ajustes.divisao = Some(crate::arranjo::divisao_no_ponto(&self.arranjo.miolo, x));
            mudou = true;
        }
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
        if std::mem::take(&mut self.arrastando_divisao) {
            self.salvar();
        }
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
