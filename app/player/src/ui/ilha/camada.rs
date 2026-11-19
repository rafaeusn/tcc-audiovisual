//! A camada do visual da Ilha: preparada em outra thread, desenhada por baixo do recorte da página Visual.

use super::*;

/// Um minuto sem a Ilha aberta, a camada do visual sai (a placa de vídeo descansa).
pub(super) const CAMADA_SEM_USO: Duration = Duration::from_secs(60);

impl VistaIlha {
    /// Cuida da camada do visual: começa a prepará-la (em outra thread) quando a Ilha abre, desenha o visual em
    /// `area` (a página Visual à vista) e limpa a camada quando a página sai. Devolve se o recorte da forma pode
    /// abrir: o visual já estava na tela no quadro anterior (abrir antes mostraria a área de trabalho num quadro).
    #[cfg(windows)]
    pub(super) fn desenhar_visual(&mut self, window: &Window, area: Option<Recorte>, dados: &Dados, dt: f32) -> bool {
        let escala = window.scale_factor();
        let vp = window.viewport_size();
        let (w, h) = ((f32::from(vp.width) * escala) as u32, (f32::from(vp.height) * escala) as u32);
        let aberta = self.visivel && self.forma == Forma::Expandida;
        if aberta {
            self.camada_usada = Instant::now();
        }
        // Prepara logo que a Ilha abre: quando a pessoa chega à página Visual, a camada já está pronta.
        if aberta
            && self.camada.is_none()
            && self.camada_pendente.is_none()
            && !self.camada_falhou
            && let Some(hwnd) = self.hwnd
        {
            match janela::Camada::preparar_transparente(hwnd, w, h) {
                Ok(pendente) => self.camada_pendente = Some(pendente),
                Err(e) => {
                    log::warn!("a Ilha não conseguiu abrir o visual: {e}");
                    self.camada_falhou = true;
                }
            }
        }
        if let Some(pronta) = self.camada_pendente.as_mut().and_then(|p| p.pronta()) {
            self.camada_pendente = None;
            match pronta {
                Ok(camada) => {
                    self.camada = Some(camada);
                    self.camada_suja = true;
                }
                Err(e) => {
                    log::warn!("a Ilha não conseguiu abrir o visual: {e}");
                    self.camada_falhou = true;
                }
            }
        }
        if self.camada.as_ref().is_some_and(|c| c.perdida()) {
            self.camada = None;
        }
        let Some(camada) = self.camada.as_mut() else {
            self.quadros_com_visual = 0;
            return false;
        };
        if camada.redimensionar(w, h) {
            self.camada_suja = true;
        }
        let quadro = visual::Quadro {
            retrato: &dados.retrato,
            modelo: dados.modelo,
            cores: dados.paleta_fixa,
            modo_seguro: dados.modo_seguro,
            cinza: dados.cinza_no_visual,
            dt,
        };
        match area {
            Some(r) => {
                camada.desenhar_recorte(
                    (r.x * escala, r.y * escala, r.w * escala, r.h * escala),
                    r.raio * escala,
                    &quadro,
                );
                self.camada_suja = false;
                self.quadros_sem_visual = 0;
                self.quadros_com_visual = self.quadros_com_visual.saturating_add(1);
            }
            None => {
                self.quadros_com_visual = 0;
                self.quadros_sem_visual = self.quadros_sem_visual.saturating_add(1);
                // O recorte fechou no quadro anterior: agora um quadro vazio tira o visual de baixo da forma. Só um:
                // fora da página Visual, a camada não gasta a placa de vídeo.
                if self.camada_suja || self.quadros_sem_visual == 2 {
                    camada.desenhar_recorte((0.0, 0.0, 0.0, 0.0), 0.0, &quadro);
                    self.camada_suja = false;
                }
                self.soltar_camada_sem_uso();
            }
        }
        self.quadros_com_visual >= 2
    }
}
