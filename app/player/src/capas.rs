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

/// O lado da miniatura (o dobro de 64, para telas com escala), e até que lado de desenho ela serve.
const MINIATURA: u32 = 128;
const ATE: f32 = 64.0;
/// O maior lado de uma capa recortada.
const MAIOR_LADO: u32 = 640;
/// O fundo desfocado (a capa atrás do cabeçalho do álbum) e a largura máxima da foto larga do artista: maior que a
/// página não aparece, e pesa na placa de vídeo.
const DESFOCADO: (u32, u32) = (480, 270);
const LARGA: u32 = 1280;

/// O fundo do cabeçalho de uma página.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TipoDeFundo {
    /// A capa desfocada (álbum, playlist, faixa, artista sem foto larga).
    Desfocado,
    /// A foto larga do artista, como veio.
    Largo,
}

/// Um fundo pronto: o arquivo e o brilho da imagem onde fica o texto (os percentis 95 e 5, de 0 a 1).
#[derive(Clone, Debug, PartialEq)]
pub struct Fundo {
    pub arquivo: PathBuf,
    pub claro: f32,
    pub escuro: f32,
}

/// Os percentis 95 e 5 do brilho (o `luma` de cada pixel, de 0 a 1). Sem pixel, o pior caso: (1, 0).
pub fn brilho(pixels: impl Iterator<Item = [u8; 3]>) -> (f32, f32) {
    let mut contagem = [0u32; 256];
    let mut total = 0u32;
    for [r, g, b] in pixels {
        let luma = (2126 * u32::from(r) + 7152 * u32::from(g) + 722 * u32::from(b)) / 10_000;
        contagem[luma.min(255) as usize] += 1;
        total += 1;
    }
    if total == 0 {
        return (1.0, 0.0);
    }
    let percentil = |p: f32| {
        let alvo = ((total as f32) * p).ceil().max(1.0) as u32;
        let mut soma = 0;
        contagem.iter().position(|c| {
            soma += c;
            soma >= alvo
        })
    };
    let em = |i: Option<usize>| i.unwrap_or(255) as f32 / 255.0;
    (em(percentil(0.95)), em(percentil(0.05)))
}

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
    /// As versões para desenhar: `None` foi pedida e ainda não ficou pronta.
    versoes: HashMap<String, Option<Versoes>>,
    /// Os fundos: `None` foi pedido; `Some(None)`, não deu.
    fundos: HashMap<(String, TipoDeFundo), Option<Option<Fundo>>>,
}

/// O que se desenha de uma capa, além dela mesma.
#[derive(Clone, Default)]
struct Versoes {
    /// Recortada, quando ela não era quadrada.
    quadrada: Option<PathBuf>,
    miniatura: Option<PathBuf>,
}

enum Tarefa {
    Baixar(String),
    Versoes(String, PathBuf),
    Fundo(String, TipoDeFundo, PathBuf),
}

pub struct Capas {
    pasta: Option<PathBuf>,
    pedidos: Option<Sender<Tarefa>>,
    estado: Arc<Mutex<Estado>>,
}

impl Capas {
    /// Sem pasta de dados, nada baixa (e nenhuma thread abre).
    pub fn iniciar(pasta: Option<PathBuf>) -> Self {
        let estado = Arc::new(Mutex::new(Estado::default()));
        let pedidos = pasta.as_ref().map(|pasta| {
            let (envio, chegada) = channel::<Tarefa>();
            let chegada = Arc::new(Mutex::new(chegada));
            for n in 0..AO_MESMO_TEMPO {
                let (chegada, pasta, estado) = (chegada.clone(), pasta.clone(), estado.clone());
                let criada = std::thread::Builder::new().name(format!("capas-{n}")).spawn(move || {
                    loop {
                        // Uma thread por vez espera o próximo pedido; a trava sai antes de baixar.
                        let pedido = chegada.lock().unwrap_or_else(PoisonError::into_inner).recv();
                        // A tela redesenha a cada quadro: a capa aparece assim que fica pronta.
                        match pedido {
                            Ok(Tarefa::Baixar(url)) => {
                                if let Some(arquivo) = baixar(&url, &pasta) {
                                    estado.lock().unwrap_or_else(PoisonError::into_inner).prontas.insert(url, arquivo);
                                }
                            }
                            Ok(Tarefa::Versoes(url, capa)) => {
                                let prontas = Some(Versoes::fazer(&capa));
                                estado.lock().unwrap_or_else(PoisonError::into_inner).versoes.insert(url, prontas);
                            }
                            Ok(Tarefa::Fundo(url, tipo, capa)) => {
                                let feito = Some(fazer_fundo(&capa, tipo));
                                estado.lock().unwrap_or_else(PoisonError::into_inner).fundos.insert((url, tipo), feito);
                            }
                            Err(_) => break,
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
            let _ = p.send(Tarefa::Baixar(url.to_string()));
        }
        None
    }

    /// O arquivo para desenhar a capa num quadrado deste lado: nas linhas, a miniatura (uma capa de 640 px aberta para
    /// mostrar 48 ocupa a placa de vídeo à toa); no resto, a capa, recortada se era larga (o GPUI desenha a imagem
    /// inteira para fora do quadrado, e o canto arredondado sai errado). `None` enquanto não fica pronta.
    pub fn arquivo_para(&self, url: &str, lado: f32) -> Option<PathBuf> {
        let inteira = self.arquivo(url)?;
        let mut e = self.estado.lock().unwrap_or_else(PoisonError::into_inner);
        let versoes = match e.versoes.get(url) {
            Some(v) => v.clone()?,
            None => match Versoes::em_disco(&inteira) {
                Some(v) => {
                    e.versoes.insert(url.to_string(), Some(v.clone()));
                    v
                }
                None => {
                    e.versoes.insert(url.to_string(), None);
                    drop(e);
                    if let Some(p) = &self.pedidos {
                        let _ = p.send(Tarefa::Versoes(url.to_string(), inteira));
                    }
                    return None;
                }
            },
        };
        let grande = versoes.quadrada.unwrap_or(inteira);
        Some(if lado <= ATE { versoes.miniatura.unwrap_or(grande) } else { grande })
    }
}

impl Capas {
    /// O fundo de um cabeçalho, quando pronto (a imagem baixa e é preparada em segundo plano; a tela pede a cada quadro).
    pub fn fundo(&self, url: &str, tipo: TipoDeFundo) -> Option<Fundo> {
        let inteira = self.arquivo(url)?;
        let chave = (url.to_string(), tipo);
        let mut e = self.estado.lock().unwrap_or_else(PoisonError::into_inner);
        match e.fundos.get(&chave) {
            Some(pronto) => pronto.clone().flatten(),
            None => {
                e.fundos.insert(chave, None);
                drop(e);
                if let Some(p) = &self.pedidos {
                    let _ = p.send(Tarefa::Fundo(url.to_string(), tipo, inteira));
                }
                None
            }
        }
    }
}

/// Prepara o fundo ao lado da capa (de outra abertura, ele já está lá: só mede o brilho).
fn fazer_fundo(capa: &std::path::Path, tipo: TipoDeFundo) -> Option<Fundo> {
    let destino = capa.with_extension(match tipo {
        TipoDeFundo::Desfocado => "fundo",
        TipoDeFundo::Largo => "larga",
    });
    let ler = |a: &std::path::Path| std::fs::read(a).ok().and_then(|b| image::load_from_memory(&b).ok());
    let imagem = match ler(&destino) {
        Some(pronta) => pronta,
        None => {
            let original = ler(capa)?;
            let feita = match tipo {
                TipoDeFundo::Desfocado => {
                    // Desfocar pequeno e aumentar: o mesmo efeito, uma fração da conta.
                    let filtro = image::imageops::FilterType::Triangle;
                    let (l, a) = DESFOCADO;
                    original.resize_to_fill(l / 3, a / 3, filtro).fast_blur(6.0).resize_exact(l, a, filtro)
                }
                TipoDeFundo::Largo if original.width() > LARGA => {
                    let altura = (u64::from(original.height()) * u64::from(LARGA) / u64::from(original.width())) as u32;
                    original.resize_exact(LARGA, altura.max(1), image::imageops::FilterType::Triangle)
                }
                TipoDeFundo::Largo => original,
            };
            gravar(&feita, &destino)?;
            feita
        }
    };
    let rgb = imagem.to_rgb8();
    let (largura, altura) = rgb.dimensions();
    // Na foto larga o texto fica embaixo, à esquerda; no desfocado, em qualquer lugar.
    let (x_fim, y_inicio) = match tipo {
        TipoDeFundo::Largo => (largura * 7 / 10, altura * 45 / 100),
        TipoDeFundo::Desfocado => (largura, 0),
    };
    let (claro, escuro) =
        brilho(rgb.enumerate_pixels().filter(|(x, y, _)| *x < x_fim && *y >= y_inicio).map(|(_, _, p)| p.0));
    Some(Fundo { arquivo: destino, claro, escuro })
}

impl Versoes {
    /// As versões já gravadas (a miniatura é a última a ser gravada: com ela, as outras estão prontas).
    fn em_disco(capa: &std::path::Path) -> Option<Self> {
        let miniatura = capa.with_extension("mini");
        miniatura.is_file().then(|| {
            let quadrada = capa.with_extension("quad");
            Self { quadrada: quadrada.is_file().then_some(quadrada), miniatura: Some(miniatura) }
        })
    }

    /// Grava as versões da capa. Uma que não se decodifica aqui fica só com a inteira.
    fn fazer(capa: &std::path::Path) -> Self {
        let lida = std::fs::read(capa).ok().and_then(|b| image::load_from_memory(&b).ok());
        let Some(imagem) = lida else { return Self::default() };
        let (largura, altura) = (imagem.width(), imagem.height());
        let recorte = recorte_quadrado(largura, altura);
        let quadrada = recorte.and_then(|(x, y, lado)| {
            let mut q = imagem.crop_imm(x, y, lado, lado);
            if lado > MAIOR_LADO {
                q = q.resize(MAIOR_LADO, MAIOR_LADO, image::imageops::FilterType::Triangle);
            }
            gravar(&q, &capa.with_extension("quad"))
        });
        // Larga e sem a quadrada (disco cheio): sem miniatura também, para tentar de novo na próxima abertura.
        if recorte.is_some() && quadrada.is_none() {
            return Self::default();
        }
        let pequena = imagem.resize_to_fill(MINIATURA, MINIATURA, image::imageops::FilterType::Triangle);
        Self { quadrada, miniatura: gravar(&pequena, &capa.with_extension("mini")) }
    }
}

/// O quadrado do meio (x, y, lado) de uma imagem que não é quadrada (2% de folga).
pub fn recorte_quadrado(largura: u32, altura: u32) -> Option<(u32, u32, u32)> {
    let (menor, maior) = (largura.min(altura), largura.max(altura));
    let quase = u64::from(maior - menor) * 50 <= u64::from(maior);
    (menor > 0 && !quase).then(|| ((largura - menor) / 2, (altura - menor) / 2, menor))
}

fn gravar(imagem: &image::DynamicImage, destino: &std::path::Path) -> Option<PathBuf> {
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90).encode_image(&imagem.to_rgb8()).ok()?;
    let temporario = destino.with_extension(format!("{}-parcial", destino.extension()?.to_str()?));
    std::fs::write(&temporario, &bytes).and_then(|()| std::fs::rename(&temporario, destino)).ok()?;
    Some(destino.to_path_buf())
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

/// Quanto um fundo grande fica na placa de vídeo depois de sair da tela (voltar à página logo depois não o recarrega).
pub const FUNDO_FICA: Duration = Duration::from_secs(5);
/// Quanto uma capa fica: são pequenas e muitas, e voltar a uma página é comum.
pub const CAPA_FICA: Duration = Duration::from_secs(30);

/// As imagens desenhadas nas páginas. O GPUI guarda toda imagem desenhada na placa até o fim: numa sessão longa, cada
/// página aberta deixava as capas e as fotos lá (uns 4 MB por artista). As que saem da tela pelo tempo de cada uma
/// ([`FUNDO_FICA`], [`CAPA_FICA`]) são soltas.
#[derive(Default)]
pub struct ImagensNaTela {
    vistas: HashMap<PathBuf, (Instant, Duration)>,
}

impl ImagensNaTela {
    pub fn vista(&mut self, arquivo: &std::path::Path, agora: Instant, fica: Duration) {
        match self.vistas.get_mut(arquivo) {
            Some(v) => *v = (agora, v.1.max(fica)),
            None => {
                self.vistas.insert(arquivo.to_path_buf(), (agora, fica));
            }
        }
    }

    /// As que não são desenhadas há mais que o tempo delas (e esquece delas).
    pub fn vencidas(&mut self, agora: Instant) -> Vec<PathBuf> {
        let mut fora = Vec::new();
        self.vistas.retain(|arquivo, (vista, fica)| {
            let fica = agora.saturating_duration_since(*vista) < *fica;
            if !fica {
                fora.push(arquivo.clone());
            }
            fica
        });
        fora
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn imagem_que_sai_da_tela_e_solta_depois_do_tempo_dela() {
        let mut f = ImagensNaTela::default();
        let t = Instant::now();
        let s = Duration::from_secs;
        let (a, b, capa) = (PathBuf::from("a.larga"), PathBuf::from("b.larga"), PathBuf::from("c.quad"));
        f.vista(&a, t, FUNDO_FICA);
        f.vista(&b, t, FUNDO_FICA);
        f.vista(&capa, t, CAPA_FICA);
        f.vista(&a, t + s(4), FUNDO_FICA);
        assert!(f.vencidas(t + s(4)).is_empty());
        assert_eq!(f.vencidas(t + s(6)), vec![b]);
        assert_eq!(f.vencidas(t + s(10)), vec![a]);
        assert!(f.vencidas(t + s(20)).is_empty(), "a capa fica mais");
        assert_eq!(f.vencidas(t + s(31)), vec![capa]);
        assert!(f.vencidas(t + s(60)).is_empty(), "solta uma vez só");
    }

    #[test]
    fn o_brilho_de_onde_fica_o_texto() {
        let (claro, escuro) = brilho((0..100u8).map(|i| [i * 2, i * 2, i * 2]));
        assert!((claro - 190.0 / 255.0).abs() < 0.01 && (escuro - 8.0 / 255.0).abs() < 0.01, "{claro} {escuro}");
        assert_eq!(brilho(std::iter::repeat_n([255, 255, 255], 10)), (1.0, 1.0));
        assert_eq!(brilho(std::iter::empty()), (1.0, 0.0), "sem pixel, o pior caso");
    }

    #[test]
    fn o_fundo_fica_pronto_ao_lado_da_capa() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-fundos-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        let url = "https://exemplo.invalido/larga.jpg";
        image::RgbImage::from_pixel(2000, 800, image::Rgb([250, 250, 250]))
            .save_with_format(pasta.join(nome_do_arquivo(url)), image::ImageFormat::Png)
            .expect("capa");
        let capas = Capas::iniciar(Some(pasta.clone()));
        for tipo in [TipoDeFundo::Largo, TipoDeFundo::Desfocado] {
            let limite = Instant::now() + Duration::from_secs(10);
            let fundo = loop {
                match capas.fundo(url, tipo) {
                    None if Instant::now() < limite => std::thread::sleep(Duration::from_millis(20)),
                    f => break f.expect("o fundo ficou pronto"),
                }
            };
            let m = image::load_from_memory(&std::fs::read(&fundo.arquivo).expect("lê")).expect("imagem");
            let esperado = if tipo == TipoDeFundo::Largo { (LARGA, 512) } else { DESFOCADO };
            assert_eq!((m.width(), m.height()), esperado);
            assert!(fundo.claro > 0.95, "imagem clara: {fundo:?}");
        }
        drop(capas);
        let _ = std::fs::remove_dir_all(&pasta);
    }

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
    fn linha_usa_a_miniatura_e_a_capa_larga_e_recortada() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-miniaturas-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&pasta);
        std::fs::create_dir_all(&pasta).expect("pasta");
        let capas = Capas::iniciar(Some(pasta.clone()));
        let esperar = |url: &str, lado: f32| {
            let limite = Instant::now() + Duration::from_secs(10);
            loop {
                match capas.arquivo_para(url, lado) {
                    None if Instant::now() < limite => std::thread::sleep(Duration::from_millis(20)),
                    a => return a.expect("a capa ficou pronta"),
                }
            }
        };
        let medidas = |a: &std::path::Path| {
            let m = image::load_from_memory(&std::fs::read(a).expect("lê")).expect("imagem");
            (m.width(), m.height())
        };
        let gravar = |url: &str, l: u32, a: u32| {
            let arquivo = pasta.join(nome_do_arquivo(url));
            image::RgbImage::from_pixel(l, a, image::Rgb([200, 40, 40]))
                .save_with_format(&arquivo, image::ImageFormat::Png)
                .expect("capa");
            arquivo
        };
        // Larga, como a capa de um vídeo: recortada no cartão, quadrada na linha.
        let larga = "https://exemplo.invalido/larga.png";
        let inteira = gravar(larga, 640, 360);
        assert_eq!(medidas(&esperar(larga, 176.0)), (360, 360));
        assert_eq!(medidas(&esperar(larga, 48.0)), (MINIATURA, MINIATURA));
        assert_ne!(esperar(larga, 176.0), inteira);
        // Quadrada: o cartão usa a própria capa.
        let quadrada = "https://exemplo.invalido/quadrada.png";
        let capa = gravar(quadrada, 300, 300);
        assert_eq!(esperar(quadrada, 176.0), capa);
        assert_eq!(medidas(&esperar(quadrada, 48.0)), (MINIATURA, MINIATURA));
        // O que não se decodifica fica com o arquivo como veio.
        let ruim = "https://exemplo.invalido/ruim.gif";
        std::fs::write(pasta.join(nome_do_arquivo(ruim)), b"GIF89a").expect("arquivo");
        assert_eq!(esperar(ruim, 48.0), pasta.join(nome_do_arquivo(ruim)));
        // Na próxima abertura, as versões já estão em disco.
        drop(capas);
        let outra = Capas::iniciar(Some(pasta.clone()));
        assert!(outra.arquivo_para(larga, 48.0).is_some_and(|a| a.extension().is_some_and(|x| x == "mini")));
        drop(outra);
        let _ = std::fs::remove_dir_all(&pasta);
    }

    #[test]
    fn recorte_e_o_quadrado_do_meio() {
        assert_eq!(recorte_quadrado(640, 360), Some((140, 0, 360)));
        assert_eq!(recorte_quadrado(100, 300), Some((0, 100, 100)));
        assert_eq!(
            (recorte_quadrado(640, 640), recorte_quadrado(640, 635), recorte_quadrado(0, 9)),
            (None, None, None)
        );
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
