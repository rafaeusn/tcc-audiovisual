//! Lê um arquivo de música e entrega som estéreo na taxa da saída.
//!
//! A decodificação é do `symphonia` (MP3, FLAC, WAV, OGG Vorbis, AAC/M4A). Mono vira estéreo, mais de dois
//! canais ficam com os dois primeiros, e a taxa do arquivo é convertida para a da placa de som quando difere
//! (o Windows só aceita a taxa em que o dispositivo está).

use nucleo::{Erro, Resultado};
use rubato::{FastFixedIn, PolynomialDegree, Resampler};
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, Decoder, DecoderOptions};
use symphonia::core::errors::Error as ErroDeMidia;
use symphonia::core::formats::{FormatOptions, FormatReader, SeekMode, SeekTo};
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// Quadros que o conversor de taxa recebe por vez.
const BLOCO: usize = 1024;

pub struct Fonte {
    formato: Box<dyn FormatReader>,
    decodificador: Box<dyn Decoder>,
    faixa: u32,
    taxa: u32,
    duracao_s: f64,
    conversor: Option<FastFixedIn<f32>>,
    /// Som já decodificado esperando completar um bloco do conversor (esquerda, direita).
    espera: [Vec<f32>; 2],
    amostras: Option<SampleBuffer<f32>>,
    acabou: bool,
}

fn erro(e: impl std::fmt::Display) -> Erro {
    Erro::Audio(e.to_string())
}

impl Fonte {
    /// Abre `caminho` para tocar numa saída de `taxa_da_saida` Hz.
    pub fn abrir(caminho: &Path, taxa_da_saida: u32) -> Resultado<Self> {
        let arquivo = std::fs::File::open(caminho).map_err(|e| Erro::arquivo(caminho, e))?;
        let fluxo = MediaSourceStream::new(Box::new(arquivo), Default::default());
        let mut dica = Hint::new();
        if let Some(extensao) = caminho.extension().and_then(|e| e.to_str()) {
            dica.with_extension(extensao);
        }
        let sondado = symphonia::default::get_probe()
            .format(&dica, fluxo, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| Erro::Audio(format!("formato não reconhecido: {e}")))?;
        let formato = sondado.format;
        let faixa = formato
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| Erro::Audio("o arquivo não tem som".into()))?;
        let parametros = faixa.codec_params.clone();
        let id = faixa.id;
        let taxa =
            parametros.sample_rate.filter(|t| (8000..=384_000).contains(t)).ok_or_else(|| erro("taxa inválida"))?;
        let decodificador =
            symphonia::default::get_codecs().make(&parametros, &DecoderOptions::default()).map_err(erro)?;
        let duracao_s = parametros.n_frames.map_or(0.0, |n| n as f64 / f64::from(taxa));
        let conversor = if taxa == taxa_da_saida {
            None
        } else {
            let razao = f64::from(taxa_da_saida) / f64::from(taxa);
            Some(FastFixedIn::new(razao, 1.0, PolynomialDegree::Cubic, BLOCO, 2).map_err(erro)?)
        };
        Ok(Self {
            formato,
            decodificador,
            faixa: id,
            taxa,
            duracao_s,
            conversor,
            espera: [Vec::new(), Vec::new()],
            amostras: None,
            acabou: false,
        })
    }

    /// Duração da música em segundos (0 quando o arquivo não informa).
    pub fn duracao_s(&self) -> f64 {
        self.duracao_s
    }

    /// Decodifica mais um pedaço e acrescenta a `saida`, intercalado (esquerda, direita). Devolve falso quando a
    /// música acabou e não há mais nada para entregar.
    pub fn proximo(&mut self, saida: &mut Vec<f32>) -> bool {
        if self.acabou {
            return false;
        }
        let pacote = loop {
            match self.formato.next_packet() {
                Ok(p) if p.track_id() == self.faixa => break Some(p),
                Ok(_) => {}
                // Fim do arquivo, ou o formato pediu para recomeçar (fluxos encadeados): acaba aqui.
                Err(_) => break None,
            }
        };
        let Some(pacote) = pacote else {
            self.acabou = true;
            self.esvaziar(saida);
            return !saida.is_empty();
        };
        let decodificado = match self.decodificador.decode(&pacote) {
            Ok(d) => d,
            // Um pacote estragado é pulado; o resto da música segue.
            Err(ErroDeMidia::DecodeError(_) | ErroDeMidia::IoError(_)) => return true,
            Err(_) => {
                self.acabou = true;
                return false;
            }
        };
        let especificacao = *decodificado.spec();
        let canais = especificacao.channels.count().max(1);
        let capacidade = decodificado.capacity() as u64;
        let amostras = match &mut self.amostras {
            Some(a) if a.capacity() >= decodificado.frames() * canais => a,
            vazio => vazio.insert(SampleBuffer::new(capacidade, especificacao)),
        };
        amostras.copy_interleaved_ref(decodificado);
        let dados = amostras.samples();
        for quadro in dados.chunks_exact(canais) {
            let (esquerda, direita) = (quadro[0], if canais > 1 { quadro[1] } else { quadro[0] });
            if self.conversor.is_some() {
                self.espera[0].push(esquerda);
                self.espera[1].push(direita);
            } else {
                saida.extend_from_slice(&[esquerda, direita]);
            }
        }
        self.converter(saida, false);
        true
    }

    /// Passa pelo conversor de taxa os blocos completos que estão em espera (e o resto, no fim da música).
    fn converter(&mut self, saida: &mut Vec<f32>, ate_o_fim: bool) {
        let Some(conversor) = self.conversor.as_mut() else { return };
        loop {
            let disponivel = self.espera[0].len();
            if disponivel < BLOCO && !(ate_o_fim && disponivel > 0) {
                return;
            }
            // No fim, o último bloco é completado com silêncio e só a parte que corresponde ao som é aproveitada.
            let uteis = disponivel.min(BLOCO);
            let bloco: Vec<Vec<f32>> = self
                .espera
                .iter_mut()
                .map(|canal| {
                    let mut b: Vec<f32> = canal.drain(..uteis).collect();
                    b.resize(BLOCO, 0.0);
                    b
                })
                .collect();
            let Ok(convertido) = conversor.process(&bloco, None) else { return };
            let quadros = convertido[0].len() * uteis / BLOCO;
            for (esquerda, direita) in convertido[0].iter().zip(&convertido[1]).take(quadros) {
                saida.extend_from_slice(&[*esquerda, *direita]);
            }
        }
    }

    fn esvaziar(&mut self, saida: &mut Vec<f32>) {
        self.converter(saida, true);
    }

    /// Vai para o ponto `segundos` da música. Devolve onde ficou de verdade.
    pub fn ir(&mut self, segundos: f64) -> f64 {
        let alvo =
            if self.duracao_s > 0.0 { segundos.clamp(0.0, (self.duracao_s - 0.2).max(0.0)) } else { segundos.max(0.0) };
        let pedido = SeekTo::Time { time: Time::from(alvo), track_id: Some(self.faixa) };
        match self.formato.seek(SeekMode::Accurate, pedido) {
            Ok(onde) => {
                self.decodificador.reset();
                self.espera.iter_mut().for_each(Vec::clear);
                if let Some(c) = self.conversor.as_mut() {
                    c.reset();
                }
                self.acabou = false;
                onde.actual_ts as f64 / f64::from(self.taxa)
            }
            Err(e) => {
                log::debug!("não foi possível ir para {alvo:.1}s: {e}");
                alvo
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    /// Grava um WAV de teste: um tom de 440 Hz, 16 bits.
    fn wav(nome: &str, taxa: u32, canais: u16, segundos: f32) -> std::path::PathBuf {
        let quadros = (taxa as f32 * segundos) as u32;
        let dados = quadros * u32::from(canais) * 2;
        let mut b = Vec::with_capacity(44 + dados as usize);
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + dados).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&canais.to_le_bytes());
        b.extend_from_slice(&taxa.to_le_bytes());
        b.extend_from_slice(&(taxa * u32::from(canais) * 2).to_le_bytes());
        b.extend_from_slice(&(canais * 2).to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&dados.to_le_bytes());
        for i in 0..quadros {
            let v = ((i as f32 * 440.0 * std::f32::consts::TAU / taxa as f32).sin() * 12_000.0) as i16;
            for _ in 0..canais {
                b.extend_from_slice(&v.to_le_bytes());
            }
        }
        let pasta = std::env::temp_dir().join(format!("tccplayer-teste-fonte-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).expect("pasta");
        let caminho = pasta.join(nome);
        std::fs::write(&caminho, b).expect("gravar wav");
        caminho
    }

    fn tudo(f: &mut Fonte) -> Vec<f32> {
        let mut saida = Vec::new();
        while f.proximo(&mut saida) {}
        saida
    }

    #[test]
    fn mesma_taxa_passa_direto() {
        let caminho = wav("direto.wav", 48_000, 2, 1.0);
        let mut f = Fonte::abrir(&caminho, 48_000).expect("abrir");
        assert!((f.duracao_s() - 1.0).abs() < 0.01);
        let som = tudo(&mut f);
        assert_eq!(som.len(), 48_000 * 2);
        assert!(som.iter().any(|v| v.abs() > 0.2), "tem som");
        assert!(som.iter().all(|v| v.abs() <= 1.0));
    }

    #[test]
    fn mono_e_outra_taxa_viram_estereo_na_taxa_da_saida() {
        let caminho = wav("mono44.wav", 44_100, 1, 2.0);
        let mut f = Fonte::abrir(&caminho, 48_000).expect("abrir");
        let som = tudo(&mut f);
        let quadros = som.len() / 2;
        assert!((quadros as i64 - 96_000).abs() < 600, "2 s a 48 kHz, deu {quadros} quadros");
        // Mono: os dois lados são iguais.
        assert!(som.as_chunks::<2>().0.iter().all(|q| (q[0] - q[1]).abs() < 1e-6));
        assert!(som.iter().any(|v| v.abs() > 0.2));
    }

    #[test]
    fn ir_para_um_ponto() {
        let caminho = wav("ir.wav", 48_000, 2, 3.0);
        let mut f = Fonte::abrir(&caminho, 48_000).expect("abrir");
        let onde = f.ir(2.0);
        assert!((onde - 2.0).abs() < 0.1, "ficou em {onde}");
        let resto = tudo(&mut f).len() / 2;
        assert!((resto as i64 - 48_000).abs() < 4800, "sobra 1 s, deu {resto} quadros");
        // Depois do fim dá para voltar ao começo.
        assert!(f.ir(0.0) < 0.1);
        assert!(tudo(&mut f).len() / 2 > 140_000);
    }

    #[test]
    fn arquivo_que_nao_e_musica() {
        let pasta = std::env::temp_dir().join(format!("tccplayer-teste-fonte-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).expect("pasta");
        let caminho = pasta.join("texto.mp3");
        std::fs::write(&caminho, "isto não é música").expect("gravar");
        assert!(Fonte::abrir(&caminho, 48_000).is_err());
        assert!(Fonte::abrir(&pasta.join("nao-existe.flac"), 48_000).is_err());
    }
}
