//! O que toca e o que o visual mostra: a mídia, a origem (local ou outro app), a pré-análise e as medidas.

use super::*;

impl Player {
    /// Soma o tempo que a faixa de agora toca com som; ao passar de [`SOM_PARA_A_SESSAO`], ela entra em "Nesta
    /// sessão" (uma vez por faixa).
    pub(super) fn contar_para_a_sessao(&mut self, dt: f32, window: &mut Window) {
        let Some(faixa) = self.midia.faixa.as_ref().filter(|f| !f.titulo.is_empty()) else { return };
        if self.historico.front().is_some_and(|o| o.faixa.chave() == faixa.chave()) {
            return;
        }
        if self.midia.tocando && self.retrato.gate > 0.5 {
            self.som_da_faixa += dt;
        }
        if self.som_da_faixa < SOM_PARA_A_SESSAO {
            return;
        }
        let ouvida = Ouvida { faixa: faixa.clone(), capa: self.capa.clone(), quando: Instant::now() };
        self.historico.push_front(ouvida);
        while self.historico.len() > MAXIMO_NA_SESSAO {
            // Sem isso o atlas de imagens do GPUI guardaria a capa para sempre.
            if let Some(Ouvida { capa: Some(img), .. }) = self.historico.pop_back() {
                let _ = window.drop_image(img);
            }
        }
    }

    /// A faixa, a capa ou o estado de reprodução mudaram.
    pub(super) fn atualizar_midia(&mut self, novo: Estado) {
        let mudou_faixa = novo.faixa.as_ref().map(Faixa::chave) != self.midia.faixa.as_ref().map(Faixa::chave);
        let mudou_capa = novo.capa.as_ref().map(Arc::as_ptr) != self.midia.capa.as_ref().map(Arc::as_ptr);
        if mudou_faixa {
            self.mapa.fill(-1.0);
            self.mapa_da_analise = None;
            self.capa = None;
            self.secoes.clear();
            self.candidata = None;
            self.inicio_da_faixa = Instant::now();
            self.troca_de_faixa = Instant::now();
            self.atraso_letra = 0.0;
        }
        if mudou_faixa || mudou_capa {
            self.capa = novo.capa.as_deref().and_then(imagem_da_capa);
        }
        if mudou_faixa {
            self.som_da_faixa = 0.0;
        }
        // A capa chega um pouco depois do título: completa a entrada que já está em "Nesta sessão".
        if let (Some(faixa), Some(o)) = (novo.faixa.as_ref(), self.historico.front_mut())
            && o.faixa.chave() == faixa.chave()
        {
            o.capa = self.capa.clone().or(o.capa.take());
        }
        self.midia = novo;
        if let Some(s) = &self.sistema {
            let (titulo, artista) =
                self.midia.faixa.as_ref().map_or(("", ""), |f| (f.titulo.as_str(), f.artista.as_str()));
            s.dica(&integracao::texto_da_dica(titulo, artista));
        }
    }

    /// Onde a faixa está agora, em segundos: a posição que o app informa ou, sem ela, o tempo desde que começou.
    pub(crate) fn tempo_da_faixa(&self) -> f64 {
        match self.midia.posicao {
            Some(p) if p.duracao_s > 1.0 => p.agora(self.midia.tocando),
            _ => self.inicio_da_faixa.elapsed().as_secs_f64(),
        }
    }

    pub(super) fn atualizar_medidores(&mut self, dt: f32) {
        let r = self.retrato;
        self.t_energia += dt;
        if self.t_energia >= 0.5 {
            self.t_energia = 0.0;
            self.energia_recente.pop_front();
            self.energia_recente.push_back(r.energia * r.gate);
        }
        // Batida do compasso: avança quando a fase da batida dá a volta.
        if let Some(tempo) = self.compasso_da_analise {
            // A análise sabe o tempo do compasso: acende o certo (e pulsa no tempo forte).
            if u8::try_from(self.batida).ok() != Some(tempo) {
                self.batida = usize::from(tempo);
                if tempo == 0 && !self.ajustes.modo_seguro {
                    self.pulso_compasso = 1.0;
                }
            }
        } else if r.conf_batida > 0.15 && r.gate > 0.5 && r.fase_batida < self.fase_anterior - 0.5 {
            self.batida = (self.batida + 1) % 4;
            // Pulso só no primeiro tempo, e nunca no modo seguro.
            if self.batida == 0 && !self.ajustes.modo_seguro {
                self.pulso_compasso = 1.0;
            }
        }
        self.fase_anterior = r.fase_batida;
        self.pulso_compasso *= (-dt / 0.09).exp();

        // Mapa de energia: com a posição da faixa, cada célula é um trecho; sem ela, as células rolam. Com a
        // pré-análise, o mapa já está inteiro e não muda.
        let n = self.mapa.len();
        match self.midia.posicao.filter(|p| p.duracao_s > 1.0 && self.mapa_da_analise.is_none()) {
            Some(p) if self.midia.tocando => {
                let i = ((p.agora(true) / p.duracao_s) * n as f64) as usize;
                if let Some(c) = self.mapa.get_mut(i.min(n - 1)) {
                    *c = if *c < 0.0 { r.energia } else { *c + (r.energia - *c) * (dt * 2.0).min(1.0) };
                }
            }
            Some(_) => {}
            None => {
                self.t_mapa += dt;
                if self.t_mapa >= 0.5 && r.gate > 0.5 {
                    self.t_mapa = 0.0;
                    self.mapa.rotate_left(1);
                    self.mapa[n - 1] = r.energia;
                }
            }
        }

        self.atualizar_secoes(dt);

        // Tom do dock: segue a cor do visual devagar; neutro no modo seguro ou sem cores reativas.
        let cores = self.cores();
        let segue = self.ajustes.cores_reativas && !self.ajustes.modo_seguro && r.gate > 0.3;
        let alvo =
            if segue { tema::tom_do_dock(analise::cores(r.energia, 0.0).0, self.ajustes.tema) } else { cores.dock };
        // Logo depois de trocar o tema (ou a skin), o tom chega rápido; no resto do tempo segue a música devagar.
        let estilo = (self.ajustes.tema, self.imersivo());
        if estilo != self.estilo_visto {
            self.estilo_visto = estilo;
            self.troca_de_estilo_ate = Instant::now() + Duration::from_millis(500);
        }
        let constante = if Instant::now() < self.troca_de_estilo_ate { 0.06 } else { 2.0 };
        let k = 1.0 - (-dt / constante).exp();
        for (tom, alvo) in self.tom.iter_mut().zip(alvo) {
            *tom += (alvo - *tom) * k;
        }
    }

    /// Estrutura da faixa: registra quando o nível de energia muda e fica.
    pub(super) fn atualizar_secoes(&mut self, dt: f32) {
        let r = self.retrato;
        if r.gate < 0.5 {
            self.candidata = None;
            self.silencio_s += dt;
            return;
        }
        // Sem faixa informada pelo app, "a faixa" começa quando o som volta depois de um silêncio.
        if self.silencio_s > 3.0 && self.midia.posicao.is_none() {
            self.secoes.clear();
            self.inicio_da_faixa = Instant::now();
        }
        self.silencio_s = 0.0;
        let t = self.tempo_da_faixa();
        // A pessoa voltou a faixa: o que estava registrado depois desse ponto não vale mais.
        while self.secoes.last().is_some_and(|s| s.inicio_s > t + 1.0) {
            self.secoes.pop();
        }
        let Some(atual) = self.secoes.last().map(|s| s.rotulo) else {
            self.secoes.push(Secao { rotulo: r.rotulo, inicio_s: t });
            return;
        };
        if r.rotulo == atual {
            self.candidata = None;
            return;
        }
        let durou = match self.candidata {
            Some((rotulo, tempo)) if rotulo == r.rotulo => tempo + dt,
            _ => 0.0,
        };
        self.candidata = Some((r.rotulo, durou));
        if durou >= SECAO_MINIMA_S {
            self.candidata = None;
            self.secoes.push(Secao { rotulo: r.rotulo, inicio_s: (t - f64::from(durou)).max(0.0) });
            if self.secoes.len() > MAXIMO_DE_SECOES {
                self.secoes.remove(0);
            }
        }
    }

    /// Coisas do sistema que mudam devagar: o volume (outro app pode ter mexido) e a hora.
    pub(super) fn atualizar_sistema(&mut self, dt: f32) {
        self.t_volume += dt;
        if self.t_volume >= 0.5 && !self.arrastando_volume {
            self.t_volume = 0.0;
            self.ler_volume();
        }
        // Arrastando a barra o volume muda a cada quadro: ele é guardado quando sossega.
        if let Some(t) = self.t_volume_salvo.as_mut() {
            *t += dt;
            if *t >= 1.0 {
                self.t_volume_salvo = None;
                self.salvar();
            }
        }
        self.t_hora += dt;
        if self.t_hora >= 1.0 {
            self.t_hora = 0.0;
            self.hora = hora_local();
            self.movimento_reduzido = integracao::ilha::movimento_reduzido();
        }
    }

    pub(super) fn desenhar_visual(&mut self, window: &mut Window, dt: f32) {
        #[cfg(windows)]
        {
            if self.camada.as_ref().is_some_and(janela::Camada::perdida) {
                self.camada = None;
                self.erro_visual = Some("A placa de vídeo foi reiniciada. Recuperando o visual...".into());
            }
            if self.camada.is_none() && Instant::now() >= self.proxima_camada {
                self.criar_camada(window);
            }
            // Os pixels da tela: o fator do Windows vezes o tamanho da interface.
            let escala = window.scale_factor() * crate::ui::medidas::escala();
            let a = &self.arranjo;
            let cores = self.cores();
            let fundo = tema::tripla(cores.parede);
            let v = a.visual;
            let raio = a.raio_visual * escala;
            let paleta = self.ajustes.paleta_atual();
            let fixas = (!self.ajustes.cores_reativas)
                .then(|| (paleta.a.map(|v| v as f32 / 255.0), paleta.b.map(|v| v as f32 / 255.0)));
            let quadro = visual::Quadro {
                retrato: &self.retrato,
                modelo: self.ajustes.modelo,
                cores: fixas,
                modo_seguro: self.ajustes.modo_seguro,
                cinza: self.ajustes.tema == Tema::Mono,
                dt,
            };
            // Enquanto o retângulo do visual muda (cresce, a divisória anda, vira tira), ele é desenhado num tamanho só e o
            // meio aparece no retângulo: recriar os alvos a cada quadro engasgava e apagava o rastro do visual.
            let interno = a.interno.map(|(w, h)| (w * escala, h * escala));
            if let Some(c) = self.camada.as_mut() {
                c.redimensionar((a.janela.w * escala) as u32, (a.janela.h * escala) as u32);
                let r = (v.x * escala, v.y * escala, v.w * escala, v.h * escala);
                c.desenhar_recortado(fundo, r, interno, raio, &quadro);
            }
        }
        #[cfg(not(windows))]
        {
            let _ = (window, dt);
        }
    }

    /// Com a música local pré-analisada, o visual é conduzido pela linha do tempo (o modo arquivo do diretor):
    /// energia no degrau do trecho, pump em cada batida, drop no ataque exato, subida antes dele, fase pelas
    /// batidas reais. O espectro continua vindo do som ao vivo. O mapa do dock aparece inteiro desde o começo.
    pub(super) fn aplicar_preanalise(&mut self, dt: f32) {
        // O impulso do ao vivo deste quadro (o motor recomeça do zero quando a captura reabre: nunca negativo).
        let delta_vivo = (self.retrato.impulso_total - self.impulso_vivo).max(0.0);
        self.impulso_vivo = self.retrato.impulso_total;
        let analise = if self.origem_local { self.local.analise_atual() } else { None };
        let chave = analise.as_ref().map(|a| Arc::as_ptr(a) as usize);
        if chave != self.mapa_da_analise {
            match &analise {
                Some(a) => self.mapa = a.mapa(CELULAS_MAPA),
                None if self.mapa_da_analise.is_some() => self.mapa.fill(-1.0),
                None => {}
            }
            self.mapa_da_analise = chave;
        }
        match analise.filter(|_| self.local.situacao.tocando) {
            Some(a) => {
                // O instante que está sendo ouvido: a posição entregue à placa, menos a latência da saída.
                const LATENCIA_S: f64 = 0.02;
                let t = (self.local.situacao.posicao_s - LATENCIA_S).max(0.0);
                let q = self.condutor.conduzir(&a, dt, t, 1.0);
                let r = &mut self.retrato;
                r.energia = q.energia;
                r.loud = q.volume;
                r.atividade = q.atividade;
                r.pulso = q.pulso;
                r.drop = q.drop;
                r.build = q.preparo;
                r.conf_batida = q.conf_batida;
                r.bpm = q.bpm;
                r.fase_batida = q.fase_batida;
                r.rotulo = q.rotulo;
                // A música está tocando: o som existe mesmo se a captura não ouvir (sistema mudo, outra saída).
                if q.volume > 0.02 {
                    r.gate = 1.0;
                }
                self.impulso_saida += f64::from(q.impulso);
                self.compasso_da_analise = q.tempo_do_compasso;
            }
            None => {
                self.condutor.reiniciar();
                self.impulso_saida += delta_vivo;
                self.compasso_da_analise = None;
            }
        }
        self.retrato.impulso_total = self.impulso_saida;
    }

    /// Lê o "tocando agora" dos outros apps quando ele muda. A sessão do próprio Player (a música local
    /// publicada no Windows) não conta: a música local vem direto do player local. Devolve se mudou.
    pub(super) fn ler_outro(&mut self) -> bool {
        let versao = self.sessao.versao();
        if versao == self.versao_sessao {
            return false;
        }
        self.versao_sessao = versao;
        let mut estado = self.sessao.estado();
        if estado.faixa.as_ref().is_some_and(|f| e_o_player(&f.app)) {
            estado = Estado { disponivel: estado.disponivel, ..Default::default() };
        }
        self.outro = estado;
        true
    }

    /// Quem a tela mostra (e os controles comandam): quem está tocando; se os dois tocam, quem começou por
    /// último; se nenhum toca, quem tocou por último.
    pub(super) fn escolher_origem(&mut self) -> bool {
        let tem_local = self.local.ativo();
        let tem_outro = self.outro.faixa.is_some();
        let local_toca = tem_local && self.local.situacao.tocando;
        let outro_toca = tem_outro && self.outro.tocando;
        if local_toca && !self.local_tocava {
            self.ultimo_foi_local = true;
        }
        if outro_toca && !self.outro_tocava {
            self.ultimo_foi_local = false;
        }
        self.local_tocava = local_toca;
        self.outro_tocava = outro_toca;
        origem_e_local(tem_local, tem_outro, local_toca, outro_toca, self.ultimo_foi_local)
    }

    /// Guarda o tamanho da janela depois que ele fica parado por um segundo (para reabrir igual).
    pub(super) fn lembrar_tamanho(&mut self, w: f32, h: f32, especial: bool) {
        if especial {
            return;
        }
        if (w, h) != self.tamanho_visto {
            self.tamanho_visto = (w, h);
            self.tamanho_desde = Instant::now();
        } else if self.tamanho_desde.elapsed() > Duration::from_secs(1)
            && ((self.ajustes.janela_largura - w).abs() > 0.5 || (self.ajustes.janela_altura - h).abs() > 0.5)
        {
            self.ajustes.janela_largura = w;
            self.ajustes.janela_altura = h;
            self.ajustes = self.ajustes.clone().validar();
            self.salvar();
        }
    }
}

/// A sessão é do Player, ou do Soloist que ele abre: não é "outro app".
fn e_o_player(app: &str) -> bool {
    app == crate::local::NOME_NO_SISTEMA || app.to_lowercase().contains("soloist")
}
