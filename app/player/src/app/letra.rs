//! A letra da faixa: a busca (automática e à mão), o acerto da sincronia e o que a Ilha e o painel leem.

use super::*;

impl Player {
    /// Abre a busca manual de letra, já com o nome da faixa que está tocando.
    pub(crate) fn abrir_busca_de_letra(&mut self) {
        let consulta: String = self
            .midia
            .faixa
            .as_ref()
            .map(|f| {
                let (titulo, artista) = letras::limpar(&f.titulo, &f.artista);
                format!("{titulo} {artista}").trim().chars().take(80).collect()
            })
            .unwrap_or_default();
        if !consulta.is_empty() {
            self.letrista.procurar(consulta.clone());
        }
        self.paleta = Some(Paleta { consulta, selecionado: 0, letra: true });
    }

    /// Procura no LRCLIB o que está digitado na busca de letra.
    pub(crate) fn procurar_letra(&mut self) {
        if let Some(consulta) = self.paleta.as_ref().map(|p| p.consulta.trim().to_string()).filter(|c| !c.is_empty()) {
            self.letrista.procurar(consulta);
        }
    }

    /// Usa o resultado `i` da busca como a letra da faixa que está tocando.
    pub(crate) fn fixar_letra(&mut self, i: usize) {
        let letrista::Achados::Prontos(lista) = &self.procura.achados else { return };
        let (Some(chave), Some(registro)) = (self.chave_letra.clone(), lista.get(i).cloned()) else { return };
        self.letrista.fixar(chave, registro);
        self.paleta = None;
        self.avisar("Letra trocada nesta música");
    }

    /// Adianta (positivo) ou atrasa a letra em relação à música. Fica guardado para esta música.
    pub(crate) fn deslocar_letra(&mut self, segundos: f32) {
        self.atraso_letra = (self.atraso_letra + segundos).clamp(-10.0, 10.0);
        if self.atraso_letra.abs() < 0.01 {
            self.atraso_letra = 0.0;
        }
        if let Some(chave) = self.chave_letra.clone() {
            self.letrista.guardar_atraso(chave, self.atraso_letra);
        }
    }

    /// Pede a tradução da letra da música de agora quando a pessoa ligou "Traduzir a letra" (uma vez por música e
    /// idioma; a thread guarda e devolve a guardada nas próximas).
    fn pedir_traducao(&mut self) {
        if !self.ajustes.letra_traduzir {
            return;
        }
        let Some(chave) = self.chave_letra.clone().filter(|c| *c == self.letra.chave) else { return };
        let letrista::Situacao::Achou(letra) = &self.letra.situacao else { return };
        if letra.linhas.is_empty() {
            return;
        }
        let pedido = (chave.clone(), self.ajustes.letra_idioma.clone());
        if self.traducao_pedida.as_ref() == Some(&pedido) {
            return;
        }
        let linhas: Vec<String> = letra.linhas.iter().map(|l| l.texto.clone()).collect();
        let video_id = self.video_do_youtube_music();
        self.letrista.traduzir(chave, pedido.1.clone(), linhas, video_id);
        self.traducao_pedida = Some(pedido);
    }

    /// O que vai embaixo da linha `k` da letra: as letras latinas e a tradução, quando ligadas e prontas.
    pub(crate) fn extra_da_linha(&self, k: usize, texto: &str) -> Option<String> {
        let mut partes: Vec<String> = Vec::new();
        if self.ajustes.letra_romanizar
            && let Some(r) = letras::romanizar(texto)
        {
            partes.push(r);
        }
        let desta = self.chave_letra.as_deref() == Some(self.traducao.chave.as_str())
            && self.traducao.idioma == self.ajustes.letra_idioma;
        if self.ajustes.letra_traduzir
            && desta
            && let Some(t) = self.traducao.linhas.as_ref().and_then(|l| l.get(k).cloned().flatten())
        {
            partes.push(t);
        }
        (!partes.is_empty()).then(|| partes.join("  ·  "))
    }

    /// O texto de baixo das linhas perto do instante `t` (as que a tela mostra), por linha.
    pub(crate) fn extras_perto(&self, linhas: &[letras::Linha], t: f64) -> Vec<(usize, String)> {
        if !self.letra_tem_extras() {
            return Vec::new();
        }
        let i = letras::linha_em(linhas, t).unwrap_or(0);
        (i.saturating_sub(3)..(i + 7).min(linhas.len()))
            .filter_map(|k| self.extra_da_linha(k, &linhas[k].texto).map(|e| (k, e)))
            .collect()
    }

    /// Alguma linha vai ter texto embaixo (para a tela dar espaço a ela).
    pub(crate) fn letra_tem_extras(&self) -> bool {
        self.ajustes.letra_traduzir || self.ajustes.letra_romanizar
    }

    /// Sobe (`passo` -1) ou desce (+1) uma fonte de letra na ordem.
    pub(crate) fn mover_fonte_de_letra(&mut self, chave: &str, passo: i32) {
        let fontes = &mut self.ajustes.letra_fontes;
        let Some(i) = fontes.iter().position(|f| f == chave) else { return };
        let j = i as i32 + passo;
        if j < 0 || j as usize >= fontes.len() {
            return;
        }
        fontes.swap(i, j as usize);
        self.salvar();
        self.configurar_letras();
    }

    /// Liga ou desliga uma fonte de letra (a última ligada não desliga).
    pub(crate) fn alternar_fonte_de_letra(&mut self, chave: &str) {
        let desligadas = &mut self.ajustes.letra_fontes_desligadas;
        if let Some(i) = desligadas.iter().position(|f| f == chave) {
            desligadas.remove(i);
        } else if desligadas.len() + 1 < self.ajustes.letra_fontes.len() {
            desligadas.push(chave.to_string());
        } else {
            self.avisar("Pelo menos uma fonte de letra fica ligada");
            return;
        }
        self.salvar();
        self.configurar_letras();
    }

    /// Manda à thread das letras as fontes ligadas, na ordem, e a preferência pela palavra por palavra.
    pub(crate) fn configurar_letras(&self) {
        let ordem: Vec<letras::Fonte> = self
            .ajustes
            .letra_fontes
            .iter()
            .filter(|f| !self.ajustes.letra_fontes_desligadas.contains(f))
            .filter_map(|f| letras::Fonte::da_chave(f))
            .collect();
        self.letrista.configurar(ordem, self.ajustes.letra_palavra_por_palavra);
    }

    /// A letra sincronizada da faixa de agora e o instante da música para ela (já com o ajuste manual), para a
    /// página Letra da Ilha. Sem ela, a frase que a Ilha mostra no lugar.
    pub(crate) fn letra_sincronizada(&self) -> Result<(Arc<letrista::Letra>, f64), &'static str> {
        let desta_faixa = self.chave_letra.as_deref().is_some_and(|c| c == self.letra.chave);
        let letra = match (&self.letra.situacao, desta_faixa) {
            (letrista::Situacao::Achou(l), true) => l.clone(),
            (letrista::Situacao::SemLetra, true) => return Err("Não achei a letra desta música."),
            (letrista::Situacao::SemRede, true) => return Err("Sem conexão para buscar a letra."),
            _ => return Err("Procurando a letra desta música…"),
        };
        if letra.linhas.is_empty() {
            return Err("Faixa instrumental.");
        }
        if !letra.sincronizada {
            return Err("A letra desta música não tem os tempos marcados.");
        }
        let (t, _) = self.tempo_da_letra().ok_or("O app que está tocando não informa a posição da música.")?;
        Ok((letra, t))
    }

    /// Quanto falta para o próximo drop da música local pré-analisada que está tocando.
    pub(crate) fn drop_em_s(&self) -> Option<f32> {
        if !self.origem_local || !self.local.situacao.tocando {
            return None;
        }
        let analise = self.local.analise_atual()?;
        let t = self.local.situacao.posicao_s;
        analise.proximo_drop(t).map(|d| (d - t) as f32)
    }

    /// Posição e duração da música para a letra, já com o ajuste manual. Vazio quando o app de origem não informa.
    pub(crate) fn tempo_da_letra(&self) -> Option<(f64, f64)> {
        let p = self.midia.posicao.filter(|p| p.duracao_s > 1.0)?;
        Some((p.agora(self.midia.tocando) + f64::from(self.atraso_letra), p.duracao_s))
    }

    /// Pede a letra da faixa atual quando o módulo Letra está no painel. Sem o módulo, nada vai para a rede.
    pub(super) fn atualizar_letra(&mut self, dt: f32) {
        let versao = self.letrista.versao();
        if versao != self.versao_letra {
            self.versao_letra = versao;
            let nova = self.letrista.resposta();
            // O acerto guardado para a música chega junto com a letra dela.
            if nova.chave != self.letra.chave && self.chave_letra.as_deref() == Some(nova.chave.as_str()) {
                self.atraso_letra = nova.atraso_s;
            }
            if nova != self.letra {
                self.letra = nova;
                // A letra nova entra do começo, sem rolar a partir da linha da música anterior.
                self.anim.definir_mola(("letra", 0), -1.0);
            }
            self.procura = self.letrista.procura();
            self.traducao = self.letrista.traducao();
        }
        self.t_letra += dt;
        if self.t_letra < 0.25 {
            return;
        }
        self.t_letra = 0.0;
        let faixa = self.midia.faixa.as_ref().filter(|f| !f.titulo.is_empty());
        // A letra da Ilha também pede: só quando a pessoa abre a letra nela.
        let ligada = (self.ajustes.modulos.contains(&Modulo::Letra) || self.letra_pela_ilha || self.letra_cheia)
            && self.modulo_ativo(crate::catalogo::ID_LETRA);
        let (Some(faixa), true) = (faixa, ligada) else {
            self.chave_letra = None;
            return;
        };
        let duracao = self.midia.posicao.map(|p| p.duracao_s).filter(|d| *d > 1.0);
        if duracao.is_none() && self.troca_de_faixa.elapsed() < ESPERA_PELA_DURACAO {
            return;
        }
        let (titulo, artista) = letras::limpar(&faixa.titulo, &faixa.artista);
        let chave = letras::chave(&titulo, &artista, duracao);
        let respondida = self.letra.chave == chave && self.letra.situacao != letrista::Situacao::SemRede;
        let pedida = self
            .letra_pedida
            .as_ref()
            .is_some_and(|(c, quando)| *c == chave && (respondida || quando.elapsed() < NOVA_BUSCA_DA_LETRA));
        if !pedida {
            // Faixa do YouTube Music: o id do vídeo acha a letra na Unison.
            let video_id = self.video_do_youtube_music();
            let pedido = letras::Pedido { titulo, artista, album: faixa.album.clone(), duracao_s: duracao, video_id };
            self.letrista.pedir(chave.clone(), pedido);
            self.letra_pedida = Some((chave.clone(), Instant::now()));
        }
        self.chave_letra = Some(chave);
        self.pedir_traducao();
    }

    /// O id do vídeo da faixa do YouTube Music que está tocando no Player (a Unison acha a letra e guarda a
    /// tradução por ele).
    fn video_do_youtube_music(&self) -> Option<String> {
        self.local
            .atual()
            .filter(|_| self.origem_local)
            .and_then(|m| m.remota.as_ref())
            .filter(|r| r.servico == "youtube-music")
            .map(|r| r.id.clone())
    }
}
