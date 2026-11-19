//! Os módulos da Loja: instalar, desinstalar, aceitar o que pedem e arrumar no painel (modo edição).

use super::*;

impl Player {
    /// Grava a lista de instalados fora da thread da interface.
    pub(super) fn salvar_instalados(&self) {
        let Some(caminho) = self.arquivo_modulos.clone() else { return };
        let lista = self.instalados.clone();
        let gravando = std::thread::Builder::new().name("modulos".into()).spawn(move || {
            if let Err(e) = lista.salvar(&caminho) {
                log::warn!("não foi possível salvar os módulos instalados: {e}");
            }
        });
        if let Err(e) = gravando {
            log::warn!("não foi possível salvar os módulos instalados: {e}");
        }
    }

    /// Quem já tinha posto a Letra no painel antes de existir a ficha de instalação escolheu usá-la sabendo o
    /// que ela faz (a Loja já dizia): a escolha é mantida, registrada como instalação.
    pub(super) fn aceitar_letra_ja_escolhida(&mut self) {
        use crate::catalogo::ID_LETRA;
        if self.ajustes.modulos.contains(&Modulo::Letra)
            && self.instalados.achar(ID_LETRA).is_none()
            && let Some(m) = self.catalogo.iter().find(|m| m.id == ID_LETRA)
        {
            self.instalados.instalar(m);
            self.salvar_instalados();
            log::info!("módulo Letra registrado como instalado (já estava no painel)");
        }
    }

    /// O que a versão do catálogo pede além do que a pessoa aceitou ao instalar (vazio = nada novo, ou não
    /// instalado).
    pub(crate) fn pendencias(&self, m: &modulos::Manifesto) -> Vec<modulos::Mudanca> {
        match self.instalados.achar(&m.id) {
            Some(i) if i.codigo != m.codigo => i.aceite.mudancas(m),
            _ => Vec::new(),
        }
    }

    /// O módulo está instalado, ligado e não espera a pessoa aceitar uma versão nova.
    pub(crate) fn modulo_ativo(&self, id: &str) -> bool {
        self.instalados.ativo(id)
            && self.catalogo.iter().find(|m| m.id == id).is_none_or(|m| self.pendencias(m).is_empty())
    }

    /// Versões novas que não pedem nada além do que foi aceito se aplicam sozinhas. As que pedem mais esperam a
    /// pessoa aceitar na ficha (até lá, o módulo fica parado).
    pub(super) fn atualizar_modulos(&mut self) {
        let mut mudou = false;
        for m in self.catalogo.clone() {
            let Some(i) = self.instalados.achar(&m.id) else { continue };
            if i.codigo == m.codigo {
                continue;
            }
            if self.pendencias(&m).is_empty() {
                let ligado = i.ligado;
                self.instalados.instalar(&m);
                self.instalados.ligar(&m.id, ligado);
                mudou = true;
                log::info!("módulo {} atualizado para {} (não pede nada novo)", m.nome, m.versao);
            } else {
                log::info!("módulo {} {} pede mais do que foi aceito: espera a pessoa aceitar", m.nome, m.versao);
            }
        }
        if mudou {
            self.salvar_instalados();
        }
    }

    /// Diz às threads do Last.fm e do Discord se o módulo de cada uma está instalado e ligado.
    pub(super) fn avisar_servicos(&self) {
        self.servicos.ativar_lastfm(self.modulo_ativo(crate::catalogo::ID_LASTFM));
        self.servicos.ativar_discord(self.modulo_ativo(crate::catalogo::ID_DISCORD));
    }

    /// Abre a ficha do módulo `id` do catálogo.
    pub(crate) fn abrir_ficha(&mut self, id: &str) {
        self.ficha = self.catalogo.iter().position(|m| m.id == id);
        self.aceite_marcado = false;
    }

    pub(crate) fn fechar_ficha(&mut self) {
        self.ficha = None;
        self.aceite_marcado = false;
    }

    /// Instala o módulo da ficha aberta. Só vale com a caixa de aceite marcada e para módulo que já tem código.
    pub(crate) fn instalar_modulo_aberto(&mut self) {
        let Some(m) = self.ficha.and_then(|i| self.catalogo.get(i)).cloned() else { return };
        if !m.pronto || !self.aceite_marcado {
            return;
        }
        self.instalados.instalar(&m);
        self.salvar_instalados();
        self.avisar_servicos();
        self.aceite_marcado = false;
        if m.id == crate::catalogo::ID_LETRA {
            self.fechar_ficha();
            self.adicionar_modulo(Modulo::Letra);
        }
        // Os módulos de serviço ficam com a ficha aberta: é nela que se conecta a conta e se vê a situação.
        self.avisar(format!("Módulo {} instalado", m.nome));
    }

    pub(crate) fn desinstalar_modulo_aberto(&mut self) {
        let Some(m) = self.ficha.and_then(|i| self.catalogo.get(i)).cloned() else { return };
        if self.instalados.remover(&m.id) {
            self.salvar_instalados();
        }
        // Desinstalar o Last.fm também esquece a conta: a autorização guardada é apagada.
        if m.id == crate::catalogo::ID_LASTFM {
            self.servicos.desconectar_lastfm();
        }
        self.avisar_servicos();
        if m.id == crate::catalogo::ID_LETRA {
            self.remover_modulo(Modulo::Letra);
        }
        self.fechar_ficha();
        self.avisar(format!("Módulo {} desinstalado", m.nome));
    }

    pub(crate) fn adicionar_modulo(&mut self, m: Modulo) {
        // A Letra usa a internet: antes de entrar no painel, a pessoa vê o que ela faz e aceita.
        if m == Modulo::Letra && !self.modulo_ativo(crate::catalogo::ID_LETRA) {
            self.abrir_ficha(crate::catalogo::ID_LETRA);
            return;
        }
        if !self.ajustes.modulos.contains(&m) {
            self.ajustes.modulos.push(m);
            self.salvar();
        }
    }

    /// Começa a arrastar um módulo (modo edição).
    pub(crate) fn pegar_modulo(&mut self, modulo: Modulo, onde: Point<Pixels>) {
        if self.edicao {
            self.arrasto = Some(Arrasto { modulo, inicio: onde, atual: onde, alvo: None });
        }
    }

    /// Deslocamento atual de um módulo em relação ao lugar dele: segue o ponteiro enquanto é arrastado e volta
    /// (ou assenta no lugar novo) com uma aproximação suave quando é solto.
    pub(crate) fn deslocamento_do_modulo(&self, m: Modulo) -> (f32, f32) {
        let (kx, ky) = (("modulo-x", m as u32), ("modulo-y", m as u32));
        match &self.arrasto {
            Some(a) if a.modulo == m => {
                let (dx, dy) = (f32::from(a.atual.x - a.inicio.x), f32::from(a.atual.y - a.inicio.y));
                self.anim.definir(kx, dx);
                self.anim.definir(ky, dy);
                (dx, dy)
            }
            _ => (self.anim.seguir(kx, 0.0, 0.36), self.anim.seguir(ky, 0.0, 0.36)),
        }
    }

    pub(super) fn soltar_modulo(&mut self, a: Arrasto) {
        let Some(alvo) = a.alvo else { return };
        let (Some(ia), Some(ib)) = (
            self.ajustes.modulos.iter().position(|m| *m == a.modulo),
            self.ajustes.modulos.iter().position(|m| *m == alvo),
        ) else {
            return;
        };
        // Cada um parte de onde está na tela e vai até o lugar novo, em vez de pular.
        {
            let limites = self.limites.borrow();
            if let (Some(la), Some(lb)) = (limites.get(&a.modulo), limites.get(&alvo)) {
                let arrastado = (f32::from(a.atual.x - a.inicio.x), f32::from(a.atual.y - a.inicio.y));
                let entre = (f32::from(la.origin.x - lb.origin.x), f32::from(la.origin.y - lb.origin.y));
                self.anim.definir(("modulo-x", a.modulo as u32), entre.0);
                self.anim.definir(("modulo-y", a.modulo as u32), entre.1);
                self.anim.definir(("modulo-x", alvo as u32), arrastado.0 - entre.0);
                self.anim.definir(("modulo-y", alvo as u32), arrastado.1 - entre.1);
            }
        }
        self.ajustes.modulos.swap(ia, ib);
        self.salvar();
    }
}
