//! O áudio do Spotify tocando pelo próprio Player (F6, parte 3).
//!
//! O `librespot` fala com o Spotify como um cliente oficial: pega a chave de áudio da faixa com a conta da pessoa, baixa o
//! arquivo cifrado e o decifra **na memória**. O [`servidor`] entrega esses bytes, só em `127.0.0.1` e com um token
//! sorteado, a quem pedir; o `resolver` do provedor devolve esse endereço ao Player, e o motor dele toca como toca uma
//! faixa do YouTube Music: troca sem intervalo, crossfade, equalizador e pré-análise.
//!
//! **A regra 2 do `AGENTS.md`, aqui:** nada de áudio em arquivo. Os bytes decifrados só existem no [`cache`] (memória, com
//! teto) e o que o librespot grava em disco é o arquivo temporário dele, ainda cifrado, apagado a cada abertura. Decidido
//! com o Rafael em 24/11/2026; a decisão está registrada em `docs/f6-spotify.md`.

pub mod cache;
pub mod librespot;
pub mod pedidos;
pub mod servidor;

pub use cache::Cache;
pub use librespot::{FonteDaSessao, PelaSessao};
pub use servidor::Servidor;

/// Uma fonte de sessão que nunca tem sessão (os testes do `resolver`).
#[cfg(test)]
pub struct FonteNenhuma;

#[cfg(test)]
impl FonteDaSessao for FonteNenhuma {
    fn agora(&self) -> Option<librespot_core::Session> {
        None
    }
}

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use librespot::FalhaDeAudio;
use provedores::Causa;

/// O áudio de agora: o servidor local e o cache, montados quando o login termina e desmontados quando ele sai.
///
/// Enquanto a conta não está dentro, não há servidor (o `resolver` diz "entre na conta"). Quando a sessão cai e volta, o
/// servidor continua o mesmo: o `PelaSessao` pede a sessão de agora a cada uso.
pub struct Audio {
    /// O servidor e o cache, quando há sessão.
    ligado: Mutex<Option<Ligado>>,
    rt: Arc<tokio::runtime::Runtime>,
    pasta: std::path::PathBuf,
}

struct Ligado {
    servidor: Servidor,
    cache: Arc<Cache>,
}

fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Audio {
    /// `pasta` é a pasta de dados do provedor: o temporário do librespot mora nela (só bytes cifrados).
    pub fn novo(rt: Arc<tokio::runtime::Runtime>, pasta: std::path::PathBuf) -> Self {
        Self { ligado: Mutex::new(None), rt, pasta }
    }

    /// Liga o áudio com a fonte da sessão (o `Real`): o servidor local e o cache.
    ///
    /// Chamar de novo (a conta entrou, ou voltou) fecha o servidor anterior. Sem sessão, não liga.
    pub fn ligar(&self, sessao: Arc<dyn FonteDaSessao>) -> Result<(), String> {
        // O temporário do librespot (só bytes cifrados): nasce vazio a cada ligação.
        librespot::limpar_temporaria(&librespot::pasta_temporaria(&self.pasta));
        let baixador = Arc::new(PelaSessao::novo(sessao, Arc::clone(&self.rt)));
        let cache = Arc::new(Cache::novo(baixador));
        let servidor = Servidor::abrir(Arc::clone(&cache)).map_err(|e| format!("o áudio não abriu: {e}"))?;
        log::info!("áudio do Spotify pronto na porta {}", servidor.porta());
        // Fechar o anterior aqui solta a porta e o laço dele.
        *travar(&self.ligado) = Some(Ligado { servidor, cache });
        Ok(())
    }

    /// Desliga (a conta saiu): a porta fecha e o áudio guardado some.
    pub fn desligar(&self) {
        *travar(&self.ligado) = None;
    }

    pub fn ligado(&self) -> bool {
        travar(&self.ligado).is_some()
    }

    /// O endereço desta faixa para o Player, com o formato que toca nesta conta.
    ///
    /// Abre a faixa aqui **de propósito**: se o Spotify recusar (a chave, o catálogo, a conta), o erro sai daqui, com a
    /// causa certa, em vez de o Player receber um endereço que depois responde 404.
    pub fn endereco(&self, id: &str) -> Result<(String, Option<String>), FalhaDeAudio> {
        let guarda = travar(&self.ligado);
        let Some(ligado) = guarda.as_ref() else {
            return Err(FalhaDeAudio::nova(Causa::SemConta, "Entre na sua conta do Spotify."));
        };
        let formato = ligado.cache.formato(id)?;
        Ok((ligado.servidor.endereco(id, &formato), Some(formato)))
    }

    /// A porta do servidor (para o registro e os testes).
    #[cfg_attr(not(test), expect(dead_code, reason = "o registro da abertura e os testes"))]
    pub fn porta(&self) -> Option<u16> {
        travar(&self.ligado).as_ref().map(|l| l.servidor.porta())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    fn rt() -> Arc<tokio::runtime::Runtime> {
        Arc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().expect("rt"))
    }

    struct SemSessao;

    impl FonteDaSessao for SemSessao {
        fn agora(&self) -> Option<librespot_core::Session> {
            None
        }
    }

    #[test]
    fn sem_conta_o_audio_nao_liga_e_o_endereco_diz_para_entrar() {
        let pasta = std::env::temp_dir().join(format!("provedor-spotify-audio-{}", std::process::id()));
        let audio = Audio::novo(rt(), pasta);
        assert!(!audio.ligado());
        assert_eq!(audio.endereco("x").expect_err("sem conta").causa, Causa::SemConta);
        assert_eq!(audio.porta(), None);
    }

    #[test]
    fn ligar_abre_uma_porta_local_e_desligar_fecha() {
        let pasta = std::env::temp_dir().join(format!("provedor-spotify-audio-liga-{}", std::process::id()));
        let audio = Audio::novo(rt(), pasta);
        audio.ligar(Arc::new(SemSessao)).expect("liga");
        assert!(audio.ligado());
        let porta = audio.porta().expect("porta");
        assert!(porta > 0);
        // Sem sessão de verdade, a faixa não abre: o erro sai daqui, com a causa, em vez de um endereço que daria 404.
        let falha = audio.endereco("abc123").expect_err("sem sessão");
        assert_eq!(falha.causa, Causa::SemConta);
        // Uma nova ligação troca a porta (a anterior fechou).
        audio.ligar(Arc::new(SemSessao)).expect("religa");
        audio.desligar();
        assert!(!audio.ligado());
        assert_eq!(audio.porta(), None);
    }
}
