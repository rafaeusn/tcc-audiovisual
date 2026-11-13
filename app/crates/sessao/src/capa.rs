//! Capa da faixa, decodificada fora da thread da interface.
//!
//! Os bytes vêm de outro programa, então são tratados como entrada não confiável: há teto de tamanho antes de
//! decodificar, a decodificação pode falhar sem derrubar nada e a imagem é reduzida antes de ir para a GPU.

/// Maior arquivo de capa aceito.
pub const TETO_BYTES: usize = 8 * 1024 * 1024;
/// Lado máximo da capa guardada (a interface mostra no máximo 96 px, 2x para telas densas).
const LADO: u32 = 192;

#[derive(Clone, Debug, PartialEq)]
pub struct Capa {
    pub largura: u32,
    pub altura: u32,
    /// Pixels em BGRA (a ordem que o GPUI espera).
    pub bgra: Vec<u8>,
}

impl Capa {
    pub fn decodificar(bytes: &[u8]) -> Option<Capa> {
        if bytes.is_empty() || bytes.len() > TETO_BYTES {
            return None;
        }
        let mut leitor = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?;
        // Imagem "bomba" (poucos bytes, bilhões de pixels): recusa antes de alocar.
        let mut limites = image::Limits::default();
        limites.max_image_width = Some(8192);
        limites.max_image_height = Some(8192);
        limites.max_alloc = Some(256 * 1024 * 1024);
        leitor.limits(limites);
        let img = leitor.decode().ok()?;
        let img = img.thumbnail(LADO, LADO).to_rgba8();
        let (largura, altura) = img.dimensions();
        let mut bgra = img.into_raw();
        for px in bgra.as_chunks_mut::<4>().0 {
            px.swap(0, 2);
        }
        Some(Capa { largura, altura, bgra })
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    fn png(l: u32, a: u32, cor: [u8; 4]) -> Vec<u8> {
        let img = image::RgbaImage::from_pixel(l, a, image::Rgba(cor));
        let mut out = std::io::Cursor::new(Vec::new());
        let ok = img.write_to(&mut out, image::ImageFormat::Png);
        assert!(ok.is_ok());
        out.into_inner()
    }

    #[test]
    fn decodifica_reduz_e_troca_para_bgra() {
        let c = Capa::decodificar(&png(640, 320, [255, 10, 20, 255]));
        let c = c.unwrap_or(Capa { largura: 0, altura: 0, bgra: vec![] });
        assert_eq!((c.largura, c.altura), (192, 96));
        assert_eq!(&c.bgra[..4], &[20, 10, 255, 255]);
    }

    #[test]
    fn recusa_lixo_e_arquivo_grande() {
        assert!(Capa::decodificar(&[]).is_none());
        assert!(Capa::decodificar(b"isto nao e uma imagem").is_none());
        assert!(Capa::decodificar(&vec![0u8; TETO_BYTES + 1]).is_none());
    }
}
