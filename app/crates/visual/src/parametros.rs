//! Os números de cada quadro: do [`Retrato`] do som para o que os shaders recebem.
//!
//! Tudo aqui é conta pura (sem GPU), para poder ser testado. A parte do Spectral é um porte de `render` e
//! `updateMotion` de `preview/spectralTD.js`; os valores são os que foram ajustados na prévia.

use analise::{Dirigidos, Retrato};

/// Câmera do .toe: t = (0.71147, 0.8994, 3.55031), sem rotação, FOV 45°.
const CAM: [f32; 3] = [0.71147, 0.8994, 3.55031];

/// A prévia foi ajustada a 60 quadros por segundo. Efeitos "por quadro" (rastro, giro do rastro) são convertidos
/// para "por tempo", senão um monitor de 144 Hz mostraria um rastro mais curto.
pub fn por_quadro(dt: f32) -> f32 {
    (dt * 60.0).clamp(0.05, 4.0)
}

fn seguir(atual: f32, alvo: f32, dt: f32, tau: f32) -> f32 {
    atual + (alvo - atual) * (1.0 - (-dt / tau).exp())
}

/// A parte de luz do quadro: o que o modo seguro pode limitar.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Luz {
    /// Pump do ataque aplicado ao brilho e ao bloom.
    pub pulso: f32,
    /// Clarão do drop.
    pub drop: f32,
    /// Batida simples (aberração e flash das Barras).
    pub batida: f32,
    pub exposicao: f32,
    pub bloom: f32,
    pub cor_a: [f32; 3],
    pub cor_b: [f32; 3],
    /// Multiplicador do tremor e do zoom do bumbo (1 = normal).
    pub movimento: f32,
}

/// Modo seguro para fotossensibilidade: fica entre o diretor e os shaders, como o limitador de velocidade de um
/// carro. O diretor pede o que quiser; a luz não passa do limite.
///
/// - Sem clarão no drop nem no ataque: eles viram movimento (explosão, zoom), não luz.
/// - Brilho, exposição e bloom mudam devagar (taxa máxima por segundo).
/// - A cor muda devagar e o vermelho perde saturação.
/// - Tremor e zoom do bumbo reduzidos.
#[derive(Default)]
pub struct Limitador {
    exposicao: Option<f32>,
    bloom: Option<f32>,
    cor_a: Option<[f32; 3]>,
    cor_b: Option<[f32; 3]>,
}

impl Limitador {
    /// Variação máxima de exposição e de bloom por segundo.
    pub const TAXA_LUZ: f32 = 0.25;
    /// Variação máxima de cada canal de cor por segundo.
    pub const TAXA_COR: f32 = 0.5;
    pub const EXPOSICAO_MAXIMA: f32 = 1.6;

    pub fn aplicar(&mut self, seguro: bool, dt: f32, bruto: Luz) -> Luz {
        if !seguro {
            *self = Self::default();
            return bruto;
        }
        let dt = dt.clamp(0.0, 0.1);
        let devagar = |guardado: &mut Option<f32>, alvo: f32, taxa: f32| {
            let atual = guardado.unwrap_or(alvo);
            let novo = atual + (alvo - atual).clamp(-taxa * dt, taxa * dt);
            *guardado = Some(novo);
            novo
        };
        let cor_devagar = |guardada: &mut Option<[f32; 3]>, alvo: [f32; 3]| {
            let alvo = sem_vermelho_forte(alvo);
            let atual = guardada.unwrap_or(alvo);
            let passo = Self::TAXA_COR * dt;
            let nova = [0, 1, 2].map(|i| atual[i] + (alvo[i] - atual[i]).clamp(-passo, passo));
            *guardada = Some(nova);
            nova
        };
        Luz {
            pulso: 0.0,
            drop: 0.0,
            batida: 0.0,
            exposicao: devagar(&mut self.exposicao, bruto.exposicao.min(Self::EXPOSICAO_MAXIMA), Self::TAXA_LUZ),
            bloom: devagar(&mut self.bloom, bruto.bloom, Self::TAXA_LUZ),
            cor_a: cor_devagar(&mut self.cor_a, bruto.cor_a),
            cor_b: cor_devagar(&mut self.cor_b, bruto.cor_b),
            movimento: 0.4,
        }
    }
}

/// Vermelho saturado é a cor mais arriscada em flashes: quanto mais vermelha, mais a cor é puxada para o cinza.
fn sem_vermelho_forte(c: [f32; 3]) -> [f32; 3] {
    let vermelhidao = (c[0] - c[1].max(c[2])).clamp(0.0, 1.0);
    let cinza = 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
    let k = 0.45 * vermelhidao;
    c.map(|v| v + (cinza - v) * k)
}

/// O que o Spectral manda para a GPU num quadro.
#[derive(Clone, Copy, Debug)]
pub struct QuadroSpectral {
    pub offset: f32,
    pub deriva: f32,
    pub distorcao: f32,
    pub mvp: [f32; 16],
    pub ancora: [f32; 2],
    pub desloc: [f32; 2],
    pub escala: f32,
    pub explosao: f32,
    pub tremor: f32,
    pub fluxo: f32,
    pub giro: f32,
    pub tempo: f32,
    pub tamanho_px: f32,
    pub brilho: f32,
    pub forca_bloom: f32,
    pub rastro: f32,
}

/// Estado do Spectral que anda de um quadro para o outro.
pub struct Spectral {
    rapido: f32,
    lento: f32,
    fase: f32,
    passeio: f32,
    tempo: f32,
}

impl Default for Spectral {
    fn default() -> Self {
        Self { rapido: 0.0, lento: 0.25, fase: 0.0, passeio: 0.0, tempo: 0.0 }
    }
}

/// Trajetória suave e quase aleatória (soma de senos incomensuráveis), em [-1, 1].
fn trajetoria(p: f32) -> f32 {
    (p.sin() + 0.6 * (p * 1.73 + 1.3).sin() + 0.4 * (p * 2.9 + 2.1).sin()) / 2.0
}

impl Spectral {
    /// `impulso` é a soma dos ataques desde o quadro anterior. `largura` e `altura` são do alvo, em pixels.
    #[allow(clippy::too_many_arguments)]
    pub fn quadro(
        &mut self,
        dt: f32,
        r: &Retrato,
        impulso: f32,
        p: &Dirigidos,
        luz: &Luz,
        largura: f32,
        altura: f32,
    ) -> QuadroSpectral {
        let b = &r.bandas;
        // "Slow Spectral Density" do original, mais a morfose rápida conduzida pelo áudio.
        self.rapido = seguir(self.rapido, b.level, dt, 0.25);
        self.lento = seguir(self.lento, self.rapido, dt, 5.0);

        // Morfose: base sempre viva + atividade do áudio. Com batida clara, acelera logo depois de cada batida.
        let arranque = 1.0 + 0.6 * r.conf_batida * (-6.0 * r.fase_batida).exp();
        self.fase += dt * (0.45 + 2.0 * r.atividade * r.gate) * p.speed * arranque;
        self.fase += impulso * 0.85 * p.react;
        // Passeio pela tela: sem batida, ritmo livre; com batida, um ciclo a cada 4 compassos.
        let livre = 0.3 + 0.6 * r.loud;
        let preso = if r.bpm > 0.0 { std::f32::consts::TAU * (r.bpm / 60.0) / 16.0 } else { livre };
        self.passeio += dt * (livre + (preso - livre) * r.conf_batida) * p.speed;
        self.tempo += dt * p.speed;
        // Os acumuladores só entram em senos: mantê-los pequenos preserva a precisão em sessões longas.
        self.fase %= 4096.0 * std::f32::consts::TAU;
        self.passeio %= 4096.0 * std::f32::consts::TAU;
        self.tempo %= 3600.0;

        let amplitude = (0.12 + 0.25 * r.loud) * p.motion * r.gate;
        let meia_altura = CAM[2] * (22.5f32).to_radians().tan() * 0.75;
        let desloc = [
            (self.passeio * 0.9).sin() * amplitude * 0.9 * meia_altura,
            (self.passeio * 1.3 + 1.1).sin() * amplitude * 0.6 * meia_altura,
        ];
        let mut escala = p.scale * (1.0 + p.spread * (0.5 * r.loud + 0.4 * r.atividade) * r.gate);
        let mut tremor = p.spread * b.high * 0.02 * r.gate;
        let mut brilho_forma = 1.0 + r.atividade * 0.2 * p.spread;

        // O pump dá um estalo na forma (movimento): vale com ou sem modo seguro. O brilho usa `luz.pulso`.
        escala *= 1.0 + 0.07 * r.pulso * luz.movimento;
        // Subida (antes do drop): a forma se contrai e treme, acumulando tensão; o drop solta tudo.
        escala *= 1.0 - 0.12 * r.build;
        tremor += r.build * 0.03 + p.spread * r.build * 0.02;
        tremor *= luz.movimento;
        brilho_forma *= 1.0 + 0.25 * r.build * luz.movimento;

        let amp = (0.045 + 0.09 * r.loud * r.gate) * p.react;
        let offset = (0.49 + (self.lento - 0.2) * 0.15 * p.intensity + trajetoria(self.fase) * amp).clamp(0.3, 0.7);

        let fov = 45.0 * (1.0 - p.punch * r.pulso * 0.1 * luz.movimento - r.drop * 0.15 * luz.movimento);
        let aspecto = (largura / altura.max(1.0)).clamp(0.1, 10.0);
        let mvp = mat_mul(&perspectiva(fov.to_radians(), aspecto, 0.1, 100.0), &translacao(-CAM[0], -CAM[1], -CAM[2]));

        QuadroSpectral {
            offset,
            deriva: self.tempo / 23.0,
            distorcao: p.warp * (1.0 + p.spread * b.mid * 0.3),
            mvp,
            ancora: [CAM[0], CAM[1]],
            desloc,
            escala,
            explosao: p.spread * b.flux * 0.08 * r.gate + r.drop * 0.6 * p.spread,
            tremor,
            fluxo: p.spread * (0.015 + 0.05 * r.loud),
            giro: self.passeio * 0.1 * p.motion,
            tempo: self.tempo,
            tamanho_px: (1.2 * altura / 1280.0).max(1.0),
            brilho: brilho_forma * (1.0 + luz.pulso * 0.5 * p.react + luz.drop * 0.8),
            forca_bloom: luz.bloom * 0.45 * (1.0 + 0.4 * luz.pulso + 0.5 * luz.drop),
            rastro: p.feedback.powf(por_quadro(dt)),
        }
    }
}

/// Matrizes 4x4 em ordem de colunas (como o WGSL espera).
fn perspectiva(fov_y: f32, aspecto: f32, perto: f32, longe: f32) -> [f32; 16] {
    let f = 1.0 / (fov_y / 2.0).tan();
    let k = longe / (perto - longe);
    // Profundidade de 0 a 1 (convenção do wgpu, diferente do WebGL).
    [f / aspecto, 0.0, 0.0, 0.0, 0.0, f, 0.0, 0.0, 0.0, 0.0, k, -1.0, 0.0, 0.0, perto * k, 0.0]
}

fn translacao(x: f32, y: f32, z: f32) -> [f32; 16] {
    [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, x, y, z, 1.0]
}

fn mat_mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut o = [0.0; 16];
    for c in 0..4 {
        for l in 0..4 {
            o[c * 4 + l] = (0..4).map(|k| a[k * 4 + l] * b[c * 4 + k]).sum();
        }
    }
    o
}

#[cfg(test)]
mod testes {
    use super::*;

    fn luz(exposicao: f32, cor: [f32; 3]) -> Luz {
        Luz { pulso: 0.9, drop: 0.8, batida: 1.0, exposicao, bloom: 1.5, cor_a: cor, cor_b: cor, movimento: 1.0 }
    }

    #[test]
    fn sem_modo_seguro_nada_muda() {
        let l = luz(2.3, [1.0, 0.1, 0.2]);
        assert_eq!(Limitador::default().aplicar(false, 1.0 / 60.0, l), l);
    }

    #[test]
    fn modo_seguro_tira_os_clarões_e_limita_a_exposicao() {
        let s = Limitador::default().aplicar(true, 1.0 / 60.0, luz(2.3, [1.0, 1.0, 1.0]));
        assert_eq!((s.pulso, s.drop, s.batida), (0.0, 0.0, 0.0));
        assert!(s.exposicao <= Limitador::EXPOSICAO_MAXIMA);
        assert!(s.movimento < 1.0);
    }

    #[test]
    fn modo_seguro_limita_a_variacao_por_segundo() {
        // O pior caso: o diretor alterna entre calmo e intenso 10 vezes por segundo.
        let mut lim = Limitador::default();
        let dt = 1.0 / 144.0;
        let mut anterior = lim.aplicar(true, dt, luz(0.8, [1.0, 1.0, 1.0]));
        let (mut var_exp, mut var_cor, mut tempo) = (0.0f32, 0.0f32, 0.0f32);
        for i in 0..144 * 3 {
            let alto = (i / 14) % 2 == 0;
            let bruto = if alto { luz(2.3, [1.0, 0.1, 0.22]) } else { luz(0.8, [1.0, 1.0, 1.0]) };
            let s = lim.aplicar(true, dt, bruto);
            var_exp += (s.exposicao - anterior.exposicao).abs();
            var_cor += (s.cor_a[1] - anterior.cor_a[1]).abs();
            anterior = s;
            tempo += dt;
        }
        assert!(var_exp / tempo <= Limitador::TAXA_LUZ + 1e-3, "exposição variou {} por segundo", var_exp / tempo);
        assert!(var_cor / tempo <= Limitador::TAXA_COR + 1e-3, "cor variou {} por segundo", var_cor / tempo);
    }

    #[test]
    fn vermelho_perde_saturacao_e_branco_fica_igual() {
        let v = sem_vermelho_forte([1.0, 0.1, 0.22]);
        assert!(v[0] < 1.0 && v[1] > 0.1);
        assert_eq!(sem_vermelho_forte([1.0, 1.0, 1.0]), [1.0, 1.0, 1.0]);
        assert_eq!(sem_vermelho_forte([0.18, 0.42, 1.0]), [0.18, 0.42, 1.0]);
    }

    #[test]
    fn spectral_fica_dentro_dos_limites() {
        let mut s = Spectral::default();
        let l = luz(1.5, [1.0, 1.0, 1.0]);
        let mut r = Retrato {
            energia: 1.0,
            loud: 1.0,
            atividade: 1.0,
            gate: 1.0,
            pulso: 1.0,
            drop: 1.0,
            bpm: 128.0,
            conf_batida: 1.0,
            ..Default::default()
        };
        r.bandas.level = 1.0;
        for _ in 0..2000 {
            let q = s.quadro(1.0 / 60.0, &r, 0.5, &Dirigidos::para(1.0), &l, 1920.0, 1080.0);
            assert!((0.3..=0.7).contains(&q.offset));
            assert!(q.mvp.iter().all(|v| v.is_finite()));
            assert!(q.escala.is_finite() && q.escala > 0.5 && q.escala < 6.0);
            assert!(q.tamanho_px >= 1.0);
        }
    }

    #[test]
    fn silencio_deixa_a_forma_parada_no_centro() {
        let mut s = Spectral::default();
        let l = luz(0.8, [1.0, 1.0, 1.0]);
        let r = Retrato::default();
        let q = s.quadro(1.0 / 60.0, &r, 0.0, &Dirigidos::para(0.0), &l, 1280.0, 720.0);
        assert_eq!(q.desloc, [0.0, 0.0]);
        assert_eq!(q.explosao, 0.0);
    }

    #[test]
    fn rastro_independe_da_taxa_de_quadros() {
        // Um segundo de rastro a 60 e a 144 quadros por segundo sobra a mesma fração.
        let sobra = |fps: f32| (0..fps as u32).fold(1.0f32, |a, _| a * 0.8f32.powf(por_quadro(1.0 / fps)));
        assert!((sobra(60.0) - sobra(144.0)).abs() < 1e-4);
    }

    #[test]
    fn projecao_poe_a_ancora_no_centro() {
        let m = mat_mul(&perspectiva(45f32.to_radians(), 1.0, 0.1, 100.0), &translacao(-CAM[0], -CAM[1], -CAM[2]));
        // O ponto (ancora, z = 0) fica bem na frente da câmera: x e y do clip valem zero.
        let p = [CAM[0], CAM[1], 0.0, 1.0];
        let clip: Vec<f32> = (0..4).map(|l| (0..4).map(|k| m[k * 4 + l] * p[k]).sum()).collect();
        assert!(clip[0].abs() < 1e-5 && clip[1].abs() < 1e-5);
        assert!((clip[3] - CAM[2]).abs() < 1e-5);
        assert!(clip[2] > 0.0 && clip[2] < clip[3]);
    }
}
