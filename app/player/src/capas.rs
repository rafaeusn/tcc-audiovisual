//! As capas que vêm da internet (cartões das páginas dos serviços): baixadas em segundo plano, guardadas em disco
//! (`<pasta de dados>/capas`) e desenhadas pela tela a partir do arquivo. A tela pede e segue; nunca espera.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

/// Quantas capas baixam ao mesmo tempo.
const AO_MESMO_TEMPO: usize = 2;
/// O tamanho máximo de uma capa (maior que isso não é capa: fica de fora, em vez de guardada cortada).
const MAXIMO: u64 = 8 * 1024 * 1024;
/// Uma capa que não veio é pedida de novo depois disso.
const NOVA_TENTATIVA: Duration = Duration::from_secs(60);

/// O nome do arquivo de uma capa: o FNV-1a de 64 bits do endereço (nada do endereço vira caminho).
pub fn nome_do_arquivo(url: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in url.bytes() {
        h ^= u64::from(b);
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}.img")
}

/// Os bytes começam como JPEG, PNG, WebP ou GIF? (Uma página de login do Wi-Fi, por exemplo, não é capa.)
pub fn parece_imagem(b: &[u8]) -> bool {
    b.starts_with(&[0xFF, 0xD8, 0xFF])
        || b.starts_with(&[0x89, b'P', b'N', b'G'])
        || (b.len() >= 12 && b.starts_with(b"RIFF") && &b[8..12] == b"WEBP")
        || b.starts_with(b"GIF8")
}

/// Já passou o tempo de pedir de novo uma capa que não veio?
pub fn pode_tentar_de_novo(desde: Instant, agora: Instant) -> bool {
    agora.saturating_duration_since(desde) > NOVA_TENTATIVA
}

/// O que se sabe de cada endereço: pronto (o arquivo), ou pedido (quando).
#[derive(Default)]
struct Estado {
    prontas: HashMap<String, PathBuf>,
    pedidas: HashMap<String, Instant>,
}

pub struct Capas {
    pasta: Option<PathBuf>,
    pedidos: Option<Sender<String>>,
    estado: Arc<Mutex<Estado>>,
}

impl Capas {
    /// Sem pasta de dados, nada baixa (e nenhuma thread abre).
    pub fn iniciar(pasta: Option<PathBuf>) -> Self {
        let estado = Arc::new(Mutex::new(Estado::default()));
        let pedidos = pasta.as_ref().map(|pasta| {
            let (envio, chegada) = channel::<String>();
            let chegada = Arc::new(Mutex::new(chegada));
            for n in 0..AO_MESMO_TEMPO {
                let (chegada, pasta, estado) = (chegada.clone(), pasta.clone(), estado.clone());
                let criada = std::thread::Builder::new().name(format!("capas-{n}")).spawn(move || {
                    loop {
                        // Uma thread por vez espera o próximo pedido; a trava sai antes de baixar.
                        let pedido = chegada.lock().unwrap_or_else(PoisonError::into_inner).recv();
                        let Ok(url) = pedido else { break };
                        // A tela redesenha a cada quadro: a capa aparece assim que fica pronta.
                        if let Some(arquivo) = baixar(&url, &pasta) {
                            estado.lock().unwrap_or_else(PoisonError::into_inner).prontas.insert(url, arquivo);
                        }
                    }
                });
                if let Err(e) = criada {
                    log::warn!("a thread das capas não abriu: {e}");
                }
            }
            envio
        });
        Self { pasta, pedidos, estado }
    }

    /// O arquivo da capa, se está pronto. Se não está, pede (uma vez; de novo só um minuto depois de uma que não
    /// veio) e devolve `None`. O disco só é olhado na primeira vez de cada endereço.
    pub fn arquivo(&self, url: &str) -> Option<PathBuf> {
        let pasta = self.pasta.as_ref().filter(|_| !url.is_empty())?;
        let mut e = self.estado.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(a) = e.prontas.get(url) {
            return Some(a.clone());
        }
        let agora = Instant::now();
        match e.pedidas.get(url) {
            Some(desde) if !pode_tentar_de_novo(*desde, agora) => return None,
            Some(_) => {}
            None => {
                // De outra abertura do app: já está em disco.
                let arquivo = pasta.join(nome_do_arquivo(url));
                if arquivo.is_file() {
                    e.prontas.insert(url.to_string(), arquivo.clone());
                    return Some(arquivo);
                }
            }
        }
        e.pedidas.insert(url.to_string(), agora);
        drop(e);
        if let Some(p) = &self.pedidos {
            let _ = p.send(url.to_string());
        }
        None
    }
}

/// Baixa uma capa para a pasta, por um arquivo temporário (a tela nunca vê um arquivo pela metade). Só guarda o
/// que parece imagem e cabe no limite.
fn baixar(url: &str, pasta: &std::path::Path) -> Option<PathBuf> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return None;
    }
    let bytes = match tocador::rede::baixar_pequeno(url, MAXIMO + 1) {
        Ok(b) if b.len() as u64 <= MAXIMO && parece_imagem(&b) => b,
        Ok(b) => {
            log::debug!("capa recusada ({url}): {} bytes, não parece imagem ou é grande demais", b.len());
            return None;
        }
        Err(e) => {
            log::debug!("capa não baixada ({url}): {e}");
            return None;
        }
    };
    let destino = pasta.join(nome_do_arquivo(url));
    let temporario = destino.with_extension("parcial");
    let gravou = std::fs::create_dir_all(pasta)
        .and_then(|()| std::fs::write(&temporario, &bytes))
        .and_then(|()| std::fs::rename(&temporario, &destino));
    match gravou {
        Ok(()) => Some(destino),
        Err(e) => {
            log::debug!("capa não gravada: {e}");
            let _ = std::fs::remove_file(&temporario);
            None
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn nome_do_arquivo_estavel_e_seguro() {
        let a = nome_do_arquivo("https://i.ytimg.com/vi/abc/hq.jpg");
        assert_eq!(a, nome_do_arquivo("https://i.ytimg.com/vi/abc/hq.jpg"));
        assert_ne!(a, nome_do_arquivo("https://i.ytimg.com/vi/abd/hq.jpg"));
        assert!(a.ends_with(".img"));
        assert!(a.trim_end_matches(".img").chars().all(|c| c.is_ascii_hexdigit()), "{a}");
    }

    #[test]
    fn capa_ja_em_disco_volta_na_hora() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-capas-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        let url = "https://exemplo.invalido/capa.jpg";
        std::fs::write(pasta.join(nome_do_arquivo(url)), b"bytes").expect("arquivo");
        let capas = Capas::iniciar(Some(pasta.clone()));
        assert_eq!(capas.arquivo(url), Some(pasta.join(nome_do_arquivo(url))));
        assert_eq!(capas.arquivo(""), None, "sem endereço, nada");
        assert_eq!(Capas::iniciar(None).arquivo(url), None, "sem pasta de dados, nada");
        drop(capas);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn so_guarda_o_que_parece_imagem() {
        assert!(parece_imagem(&[0xFF, 0xD8, 0xFF, 0xE0, 0, 0]));
        assert!(parece_imagem(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]));
        assert!(parece_imagem(b"RIFF0000WEBPVP8 "));
        assert!(parece_imagem(b"GIF89a......"));
        assert!(!parece_imagem(b"<!doctype html><html>"), "página do Wi-Fi do hotel não vira capa");
        assert!(!parece_imagem(b""));
    }

    #[test]
    fn capa_que_falhou_tenta_de_novo_depois_de_um_minuto() {
        let agora = std::time::Instant::now();
        assert!(!pode_tentar_de_novo(agora, agora + std::time::Duration::from_secs(30)));
        assert!(pode_tentar_de_novo(agora, agora + std::time::Duration::from_secs(61)));
    }
}
