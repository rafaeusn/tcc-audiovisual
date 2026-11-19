//! Pré-análise das músicas do computador.
//!
//! Antes de a música tocar, o arquivo inteiro é analisado com a receita da prévia (`preview/preanalysis.js`,
//! calibrada no corpus de `docs/diretor-calibracao.md`): batidas por programação dinâmica, tempo forte do
//! compasso, clareza do pulso, trechos pela novidade, energia calibrada, drops no ataque exato e a subida antes de
//! cada um, mais o tom e a sonoridade (LUFS, para igualar o volume de quem não tem ReplayGain). Durante a música,
//! o [`condutor`] lê essa linha do tempo no instante exato, e o visual sabe o que vem pela frente.
//!
//! O resultado vai para um cache ([`cache`]): cada música é analisada uma vez.

pub mod cache;
pub mod condutor;
pub mod pista;
pub mod sonoridade;
pub mod tom;

use analise::Rotulo;
use nucleo::{Erro, Resultado};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tocador::fonte::Fonte;

/// Muda quando o jeito de analisar muda: análises de versão antiga são refeitas.
pub const VERSAO: u32 = 2;
/// A taxa em que o arquivo é analisado.
const TAXA: u32 = 44_100;
/// Músicas mais longas que isto (mixagens de horas) não são analisadas: a análise guarda o áudio na memória.
const DURACAO_MAXIMA_S: f64 = 20.0 * 60.0;

/// Uma batida: quando, a força (0–1, já com a clareza do pulso ali) e se é o tempo forte do compasso.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Batida {
    pub t: f64,
    pub forca: f32,
    pub forte: bool,
}

/// Um ataque (bumbo, caixa, nota, consoante): quando, a força e quanto dele é grave.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Ataque {
    pub t: f64,
    pub forca: f32,
    pub grave: f32,
}

/// Um drop: o instante exato e a força.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Drop {
    pub t: f64,
    pub forca: f32,
}

/// Um trecho da música (da análise de novidade), com a energia dele.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Secao {
    pub inicio_s: f64,
    pub fim_s: f64,
    pub energia: f32,
}

/// Um trecho para mostrar: de quando começa e que nível tem (trechos vizinhos do mesmo nível já juntos).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Trecho {
    pub inicio_s: f64,
    /// 0 = calma, 1 = agitada, 2 = intensa.
    pub nivel: u8,
}

impl Trecho {
    pub fn rotulo(&self) -> Rotulo {
        match self.nivel {
            0 => Rotulo::Calma,
            1 => Rotulo::Ativa,
            _ => Rotulo::Intensa,
        }
    }
}

/// O tom da música.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Tom {
    /// "Lá menor".
    pub nome: String,
    /// Na roda de Camelot ("8A"), a notação dos DJs para mixar tons compatíveis.
    pub camelot: String,
    /// 0–1: perto de zero, o tom é ambíguo.
    pub confianca: f32,
}

/// O que a pré-análise sabe de uma música.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Analise {
    pub versao: u32,
    pub duracao_s: f64,
    /// Andamento em BPM.
    pub bpm: f32,
    /// Clareza do pulso da música (0–1): perto de zero, não há batida (fala, piano livre, pads).
    pub clareza_da_batida: f32,
    /// Taxa das curvas abaixo (amostras por segundo).
    pub fps: f32,
    /// Energia final (0–1): cor, luz e distorção.
    pub energia: Vec<f32>,
    /// Volume (0–1): amplitude da forma.
    pub volume: Vec<f32>,
    /// Atividade (0–1): velocidade da forma.
    pub atividade: Vec<f32>,
    /// Subida antes de cada drop (0–1).
    pub preparo: Vec<f32>,
    /// Clareza do pulso em cada instante (0–1).
    pub conf: Vec<f32>,
    pub batidas: Vec<Batida>,
    pub ataques: Vec<Ataque>,
    pub drops: Vec<Drop>,
    pub secoes: Vec<Secao>,
    pub trechos: Vec<Trecho>,
    pub tom: Option<Tom>,
    /// Sonoridade integrada (LUFS), ou `None` se a música é quase só silêncio.
    pub lufs: Option<f32>,
    /// O maior valor de amostra (1 = o máximo sem distorcer).
    pub pico: f32,
}

/// O nível (calma, agitada, intensa) de uma energia, com os mesmos limiares do rótulo do diretor.
pub fn nivel_da_energia(e: f32) -> u8 {
    if e >= 0.74 {
        2
    } else if e >= 0.42 {
        1
    } else {
        0
    }
}

/// Os trechos para mostrar: cada seção vira um nível, e seções vizinhas do mesmo nível viram um trecho só.
pub fn trechos_das_secoes(secoes: &[Secao]) -> Vec<Trecho> {
    let mut saida: Vec<Trecho> = Vec::new();
    for s in secoes {
        let nivel = nivel_da_energia(s.energia);
        if saida.last().is_none_or(|u| u.nivel != nivel) {
            saida.push(Trecho { inicio_s: s.inicio_s, nivel });
        }
    }
    saida
}

impl Analise {
    /// O valor de uma curva no instante `t`, interpolado.
    pub fn curva_em(&self, curva: &[f32], t: f64) -> f32 {
        if curva.is_empty() {
            return 0.0;
        }
        let kf = t.max(0.0) * f64::from(if self.fps > 0.0 { self.fps } else { 10.0 });
        let k0 = kf.floor() as usize;
        if k0 + 1 >= curva.len() {
            return curva[curva.len() - 1];
        }
        let fr = (kf - k0 as f64) as f32;
        curva[k0] + (curva[k0 + 1] - curva[k0]) * fr
    }

    /// A energia no instante `t`.
    pub fn energia_em(&self, t: f64) -> f32 {
        self.curva_em(&self.energia, t)
    }

    /// O trecho que está tocando em `t`.
    pub fn trecho_em(&self, t: f64) -> Option<&Trecho> {
        self.trechos.iter().rev().find(|s| s.inicio_s <= t)
    }

    /// O próximo trecho depois de `t`.
    pub fn proximo_trecho(&self, t: f64) -> Option<&Trecho> {
        self.trechos.iter().find(|s| s.inicio_s > t)
    }

    /// O próximo drop depois de `t` (o instante).
    pub fn proximo_drop(&self, t: f64) -> Option<f64> {
        self.drops.iter().map(|d| d.t).find(|d| *d > t)
    }

    /// O último drop até `t`.
    pub fn ultimo_drop(&self, t: f64) -> Option<f64> {
        self.drops.iter().map(|d| d.t).rev().find(|d| *d <= t)
    }

    /// A energia em `n` células iguais, para o mapa do dock.
    pub fn mapa(&self, n: usize) -> Vec<f32> {
        if self.energia.is_empty() || n == 0 {
            return vec![-1.0; n];
        }
        let m = self.energia.len();
        (0..n)
            .map(|i| {
                let (a, b) = (i * m / n, ((i + 1) * m / n).max(i * m / n + 1).min(m));
                let fatia = &self.energia[a.min(m - 1)..b];
                fatia.iter().sum::<f32>() / fatia.len().max(1) as f32
            })
            .collect()
    }
}

/// Analisa o arquivo. `parar` interrompe no meio (o app está fechando, ou chegou uma prioridade).
pub fn analisar(caminho: &Path, parar: &AtomicBool) -> Resultado<Analise> {
    let mut fonte = Fonte::abrir(caminho, TAXA)?;
    let mut medidor = sonoridade::Medidor::novo(TAXA);
    let maximo = (DURACAO_MAXIMA_S * f64::from(TAXA)) as usize;
    let mut mono: Vec<f32> =
        Vec::with_capacity((fonte.duracao_s().max(0.0) * f64::from(TAXA)) as usize + TAXA as usize);
    let mut pedaco: Vec<f32> = Vec::new();
    loop {
        if parar.load(Ordering::Relaxed) {
            return Err(Erro::Audio("análise interrompida".into()));
        }
        pedaco.clear();
        let continua = fonte.proximo(&mut pedaco);
        medidor.empurrar(&pedaco);
        // Mono pela soma com -3 dB por canal (o mesmo que o ffmpeg faz e que a calibração usou).
        mono.extend(pedaco.as_chunks::<2>().0.iter().map(|q| (q[0] + q[1]) * std::f32::consts::FRAC_1_SQRT_2));
        if mono.len() > maximo {
            return Err(Erro::Audio("música longa demais para analisar".into()));
        }
        if !continua {
            break;
        }
    }
    if mono.len() < TAXA as usize {
        return Err(Erro::Audio("o arquivo não tem som para analisar".into()));
    }
    let parar_fn = || parar.load(Ordering::Relaxed);
    let tom = tom::tom_do_croma(&tom::croma(&mono, TAXA));
    let linha =
        pista::analisar_pcm(&mono, TAXA, &parar_fn).ok_or_else(|| Erro::Audio("análise interrompida".into()))?;
    let trechos = trechos_das_secoes(&linha.secoes);
    Ok(Analise {
        versao: VERSAO,
        duracao_s: if fonte.duracao_s() > 0.0 { fonte.duracao_s() } else { linha.duracao_s },
        bpm: linha.bpm,
        clareza_da_batida: linha.clareza_da_batida,
        fps: pista::FPS_CURVA,
        energia: linha.energia,
        volume: linha.volume,
        atividade: linha.atividade,
        preparo: linha.preparo,
        conf: linha.conf,
        batidas: linha.batidas,
        ataques: linha.ataques,
        drops: linha.drops,
        secoes: linha.secoes,
        trechos,
        tom,
        lufs: medidor.lufs(),
        pico: medidor.pico(),
    })
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    #[test]
    fn trechos_juntam_secoes_do_mesmo_nivel() {
        let s = |a: f64, e: f32| Secao { inicio_s: a, fim_s: a + 10.0, energia: e };
        let t = trechos_das_secoes(&[s(0.0, 0.2), s(10.0, 0.3), s(20.0, 0.9), s(30.0, 0.8), s(40.0, 0.5)]);
        assert_eq!(t.iter().map(|x| (x.inicio_s, x.nivel)).collect::<Vec<_>>(), [(0.0, 0), (20.0, 2), (40.0, 1)]);
        assert!(trechos_das_secoes(&[]).is_empty());
    }

    #[test]
    fn consultas_da_analise() {
        let a = Analise {
            fps: 10.0,
            energia: vec![0.1, 0.2, 0.9, 0.8],
            trechos: vec![Trecho { inicio_s: 0.0, nivel: 0 }, Trecho { inicio_s: 0.2, nivel: 2 }],
            drops: vec![Drop { t: 0.2, forca: 1.0 }, Drop { t: 30.0, forca: 0.5 }],
            ..Default::default()
        };
        assert!((a.energia_em(0.15) - 0.55).abs() < 1e-6, "interpolada");
        assert_eq!(a.energia_em(99.0), 0.8);
        assert_eq!(a.trecho_em(0.1).map(|t| t.nivel), Some(0));
        assert_eq!(a.trecho_em(0.3).map(|t| t.nivel), Some(2));
        assert_eq!(a.proximo_trecho(0.05).map(|t| t.inicio_s), Some(0.2));
        assert_eq!(a.proximo_drop(0.2), Some(30.0));
        assert_eq!(a.ultimo_drop(5.0), Some(0.2));
        assert_eq!(a.mapa(2), vec![(0.1 + 0.2) / 2.0, (0.9 + 0.8) / 2.0]);
        assert_eq!(Analise::default().mapa(3), vec![-1.0; 3]);
    }

    /// Grava um WAV: `partes` em sequência, cada uma (segundos, amplitude, com bumbo).
    fn wav(nome: &str, partes: &[(f32, f32, bool)]) -> std::path::PathBuf {
        let taxa = 44_100u32;
        let mut amostras: Vec<i16> = Vec::new();
        let mut t0 = 0usize;
        let mut semente = 7u32;
        for (seg, amp, bumbo) in partes {
            let n = (taxa as f32 * seg) as usize;
            for i in 0..n {
                let t = (t0 + i) as f32 / taxa as f32;
                let fase = t % 0.5;
                semente = semente.wrapping_mul(1664525).wrapping_add(1013904223);
                let chiado = (semente >> 8) as f32 / 8388608.0 - 1.0;
                let grave = if *bumbo { (-fase * 18.0).exp() * (std::f32::consts::TAU * 60.0 * t).sin() } else { 0.0 };
                let v = amp * (0.8 * grave + 0.15 * chiado + 0.2 * (std::f32::consts::TAU * 330.0 * t).sin());
                amostras.push((v.clamp(-1.0, 1.0) * 30000.0) as i16);
            }
            t0 += n;
        }
        let dados = (amostras.len() * 2) as u32;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + dados).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&taxa.to_le_bytes());
        b.extend_from_slice(&(taxa * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&dados.to_le_bytes());
        for a in amostras {
            b.extend_from_slice(&a.to_le_bytes());
        }
        let pasta = std::env::temp_dir().join(format!("tccplayer-preanalise-{}", std::process::id()));
        std::fs::create_dir_all(&pasta).expect("pasta");
        let caminho = pasta.join(nome);
        std::fs::write(&caminho, b).expect("gravar");
        caminho
    }

    #[test]
    fn analisa_um_arquivo_com_calma_drop_e_batidas_na_grade() {
        // 14 s baixinho e sem grave, 20 s alto com bumbo a 120 BPM, 10 s calmo, 16 s alto de novo.
        let caminho =
            wav("drop.wav", &[(14.0, 0.08, false), (20.0, 0.9, true), (10.0, 0.1, false), (16.0, 0.95, true)]);
        let a = analisar(&caminho, &AtomicBool::new(false)).expect("analisa");
        assert!((a.duracao_s - 60.0).abs() < 0.5, "duração {}", a.duracao_s);
        assert!((a.bpm - 120.0).abs() < 3.0, "bpm {}", a.bpm);
        // As batidas caem na grade de 0,5 s (o bumbo do teste começa no 0 de cada período).
        let na_grade = a
            .batidas
            .iter()
            .filter(|b| b.t > 15.0 && b.t < 33.0)
            .filter(|b| (b.t * 2.0 - (b.t * 2.0).round()).abs() < 0.08)
            .count();
        let no_trecho = a.batidas.iter().filter(|b| b.t > 15.0 && b.t < 33.0).count();
        assert!(no_trecho > 30 && na_grade * 10 >= no_trecho * 9, "{na_grade} de {no_trecho} na grade");
        assert!(a.drops.iter().any(|d| (13.0..15.5).contains(&d.t)), "drops {:?}", a.drops);
        assert!(a.drops.iter().any(|d| (43.0..45.5).contains(&d.t)), "drops {:?}", a.drops);
        let calma = a.energia_em(7.0);
        let forte = a.energia_em(25.0);
        assert!(forte > calma + 0.3, "calma {calma}, forte {forte}");
        assert!(a.trechos.iter().any(|t| t.nivel >= 1), "trechos {:?}", a.trechos);
        assert!(a.preparo.iter().any(|p| *p > 0.3), "subida antes do drop");
        assert!(a.lufs.is_some() && a.pico > 0.5);
        assert!(analisar(&caminho, &AtomicBool::new(true)).is_err());
        let _ = std::fs::remove_file(caminho);
    }
}
