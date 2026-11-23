//! A mistura: de onde sai o som que vai para a placa.
//!
//! - **Sem intervalo**: a próxima música já fica aberta. Quando a de agora acaba, a seguinte começa no quadro
//!   seguinte, sem silêncio no meio (discos ao vivo e mixados ficam inteiros).
//! - **Crossfade**: com um tempo escolhido, a seguinte entra enquanto a de agora sai, com curvas de mesma
//!   potência (o volume total não afunda no meio).
//! - **Ganho por faixa**: o volume igualado (ReplayGain) vem pronto de quem pede a música.
//! - **Equalizador** de 10 bandas e um **limitador suave** no fim, para o ganho e o crossfade não estourarem.
//!
//! Nada aqui fala com a placa: [`Mistura::produzir`] devolve quadros estéreo e os marcos (quando uma música
//! começou, quando tudo acabou). Assim dá para testar com fontes de mentira.

use crate::Origem;
use std::f32::consts::{FRAC_PI_2, PI};

/// O que a mistura precisa de uma fonte de som (o arquivo decodificado, ou uma de teste).
pub trait Leitor: Send {
    /// Acrescenta mais som a `saida` (estéreo intercalado, na taxa da saída). Falso quando acabou.
    fn proximo(&mut self, saida: &mut Vec<f32>) -> bool;
    /// 0 quando o arquivo não informa.
    fn duracao_s(&self) -> f64;
    /// Vai para `segundos`; devolve onde ficou de verdade.
    fn ir(&mut self, segundos: f64) -> f64;
}

impl Leitor for crate::fonte::Fonte {
    fn proximo(&mut self, saida: &mut Vec<f32>) -> bool {
        crate::fonte::Fonte::proximo(self, saida)
    }
    fn duracao_s(&self) -> f64 {
        crate::fonte::Fonte::duracao_s(self)
    }
    fn ir(&mut self, segundos: f64) -> f64 {
        crate::fonte::Fonte::ir(self, segundos)
    }
}

/// Uma música aberta na mistura.
pub struct Faixa {
    pub origem: Origem,
    leitor: Box<dyn Leitor>,
    ganho: f32,
    pendente: Vec<f32>,
    /// Quadros já entregues.
    lidos: u64,
    /// Quadros que a música tem (0 = não se sabe).
    total: u64,
    acabou: bool,
}

impl Faixa {
    pub fn nova(origem: Origem, leitor: Box<dyn Leitor>, ganho: f32, taxa: u32) -> Self {
        let total = (leitor.duracao_s().max(0.0) * f64::from(taxa)) as u64;
        let ganho = if ganho.is_finite() { ganho.clamp(0.0, 4.0) } else { 1.0 };
        Self { origem, leitor, ganho, pendente: Vec::new(), lidos: 0, total, acabou: false }
    }

    pub fn duracao_s(&self) -> f64 {
        self.leitor.duracao_s()
    }

    pub fn ganho(&self) -> f32 {
        self.ganho
    }

    /// Tira até `n` quadros para `destino`. Devolve quantos tirou (menos que `n` só no fim da música).
    fn tirar(&mut self, n: usize, destino: &mut Vec<f32>) -> usize {
        while self.pendente.len() < n * 2 && !self.acabou {
            if !self.leitor.proximo(&mut self.pendente) {
                self.acabou = true;
            }
        }
        let k = (self.pendente.len() / 2).min(n);
        let ganho = self.ganho;
        destino.extend(self.pendente.drain(..k * 2).map(|v| v * ganho));
        self.lidos += k as u64;
        k
    }

    /// Quadros que faltam, quando a duração é conhecida.
    fn restante(&self) -> Option<u64> {
        (self.total > 0).then(|| self.total.saturating_sub(self.lidos))
    }

    fn ir(&mut self, segundos: f64, taxa: u32) -> f64 {
        let onde = self.leitor.ir(segundos);
        self.pendente.clear();
        self.acabou = false;
        self.lidos = (onde.max(0.0) * f64::from(taxa)) as u64;
        onde
    }
}

/// Um arquivo e o ganho com que ele toca.
pub type ArquivoComGanho = (Origem, f32);

/// O que aconteceu dentro de um pedaço produzido, no quadro indicado.
#[derive(Clone, Debug, PartialEq)]
pub enum Marco {
    /// Uma música começou a soar (troca sem intervalo ou início do crossfade).
    Comecou { origem: Origem, duracao_s: f64 },
    /// Não há mais nada para tocar.
    Fim,
}

/// Bandas do equalizador, em Hz.
pub const BANDAS: [f32; 10] = [31.0, 62.0, 125.0, 250.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0];
/// Ganho máximo de cada banda, para cima ou para baixo, em dB.
pub const TETO_EQ_DB: f32 = 12.0;

/// Um filtro biquad (forma direta I), com o estado dos dois canais.
#[derive(Clone, Copy, Default)]
struct Biquad {
    b: [f32; 3],
    a: [f32; 2],
    x: [[f32; 2]; 2],
    y: [[f32; 2]; 2],
}

impl Biquad {
    /// Pico (peaking EQ) do livro de receitas de Robert Bristow-Johnson.
    fn pico(taxa: f32, frequencia: f32, ganho_db: f32, q: f32) -> Self {
        let a = 10f32.powf(ganho_db / 40.0);
        let w0 = 2.0 * PI * (frequencia / taxa).min(0.49);
        let alfa = w0.sin() / (2.0 * q);
        let a0 = 1.0 + alfa / a;
        Self {
            b: [(1.0 + alfa * a) / a0, -2.0 * w0.cos() / a0, (1.0 - alfa * a) / a0],
            a: [-2.0 * w0.cos() / a0, (1.0 - alfa / a) / a0],
            ..Default::default()
        }
    }

    fn passar(&mut self, canal: usize, x: f32) -> f32 {
        let [x1, x2] = self.x[canal];
        let [y1, y2] = self.y[canal];
        let y = self.b[0] * x + self.b[1] * x1 + self.b[2] * x2 - self.a[0] * y1 - self.a[1] * y2;
        self.x[canal] = [x, x1];
        // Valores minúsculos viram zero: evita números "denormais", que deixam a CPU lenta no silêncio.
        self.y[canal] = [if y.abs() < 1e-20 { 0.0 } else { y }, y1];
        y
    }

    fn trocar_coeficientes(&mut self, outro: &Biquad) {
        self.b = outro.b;
        self.a = outro.a;
    }
}

/// Equalizador de 10 bandas. Desligado (ou com tudo em 0 dB) não toca no som.
pub struct Equalizador {
    taxa: f32,
    ganhos: [f32; 10],
    filtros: [Biquad; 10],
    ativo: bool,
}

impl Equalizador {
    pub fn novo(taxa: u32) -> Self {
        Self { taxa: taxa as f32, ganhos: [0.0; 10], filtros: [Biquad::default(); 10], ativo: false }
    }

    /// Ganhos em dB, um por banda (o que faltar fica em 0; o que passar do teto é cortado).
    pub fn ajustar(&mut self, ganhos: &[f32]) {
        let mut novos = [0.0f32; 10];
        for (n, g) in novos.iter_mut().zip(ganhos) {
            *n = if g.is_finite() { g.clamp(-TETO_EQ_DB, TETO_EQ_DB) } else { 0.0 };
        }
        if novos == self.ganhos && (self.ativo || novos.iter().all(|g| *g == 0.0)) {
            return;
        }
        self.ganhos = novos;
        self.ativo = novos.iter().any(|g| g.abs() > 0.05);
        for (i, f) in self.filtros.iter_mut().enumerate() {
            // Mantém o estado: mexer no equalizador com a música tocando não dá estalo.
            f.trocar_coeficientes(&Biquad::pico(self.taxa, BANDAS[i], novos[i], 1.41));
        }
    }

    fn passar(&mut self, estereo: &mut [f32]) {
        if !self.ativo {
            return;
        }
        for quadro in estereo.as_chunks_mut::<2>().0 {
            for (canal, v) in quadro.iter_mut().enumerate() {
                let mut s = *v;
                for f in &mut self.filtros {
                    s = f.passar(canal, s);
                }
                *v = s;
            }
        }
    }
}

/// Acima deste nível o limitador começa a dobrar o som, em vez de cortar seco.
const JOELHO: f32 = 0.9;

/// Limitador suave: até o joelho não muda nada; acima, aproxima-se de 1 sem nunca passar.
pub fn limitar(v: f32) -> f32 {
    let a = v.abs();
    if a <= JOELHO {
        return v;
    }
    let folga = 1.0 - JOELHO;
    (JOELHO + folga * ((a - JOELHO) / folga).tanh()).copysign(v)
}

/// A faixa que está saindo durante um crossfade.
struct Saindo {
    faixa: Faixa,
    total: u64,
    feito: u64,
}

pub struct Mistura {
    taxa: u32,
    atual: Option<Faixa>,
    saindo: Option<Saindo>,
    seguinte: Option<Faixa>,
    crossfade_s: f32,
    pub eq: Equalizador,
    rascunho: Vec<f32>,
}

impl Mistura {
    pub fn nova(taxa: u32) -> Self {
        Self {
            taxa,
            atual: None,
            saindo: None,
            seguinte: None,
            crossfade_s: 0.0,
            eq: Equalizador::novo(taxa),
            rascunho: Vec::new(),
        }
    }

    pub fn taxa(&self) -> u32 {
        self.taxa
    }

    /// 0 = sem crossfade (a troca continua sem intervalo). No máximo 12 segundos.
    pub fn ajustar_crossfade(&mut self, segundos: f32) {
        self.crossfade_s = if segundos.is_finite() { segundos.clamp(0.0, 12.0) } else { 0.0 };
    }

    /// Começa a tocar `faixa` agora, largando tudo o que havia (a seguinte também).
    pub fn tocar(&mut self, faixa: Faixa) {
        self.atual = Some(faixa);
        self.saindo = None;
        self.seguinte = None;
    }

    /// A música que entra quando a de agora acabar (`None` = nenhuma).
    pub fn preparar(&mut self, faixa: Option<Faixa>) {
        self.seguinte = faixa;
    }

    pub fn seguinte(&self) -> Option<&Origem> {
        self.seguinte.as_ref().map(|f| &f.origem)
    }

    /// A de agora e a seguinte (arquivo e ganho), para reabrir tudo numa saída de som nova.
    pub fn para_retomar(&self) -> (Option<ArquivoComGanho>, Option<ArquivoComGanho>) {
        let par = |f: &Faixa| (f.origem.clone(), f.ganho());
        (self.atual.as_ref().map(par), self.seguinte.as_ref().map(par))
    }

    pub fn parar(&mut self) {
        self.atual = None;
        self.saindo = None;
        self.seguinte = None;
    }

    pub fn vazia(&self) -> bool {
        self.atual.is_none() && self.saindo.is_none()
    }

    /// Vai para um ponto da música de agora (o crossfade em andamento é interrompido).
    pub fn ir(&mut self, segundos: f64) -> Option<f64> {
        self.saindo = None;
        let taxa = self.taxa;
        self.atual.as_mut().map(|f| f.ir(segundos, taxa))
    }

    fn quadros_de_fade(&self) -> u64 {
        (f64::from(self.crossfade_s) * f64::from(self.taxa)) as u64
    }

    /// Produz até `n` quadros estéreo em `saida` (acrescentando). Os marcos levam o quadro, contado a partir do
    /// começo deste pedaço. Devolve quantos quadros saíram (menos que `n` só quando tudo acabou).
    pub fn produzir(&mut self, n: usize, saida: &mut Vec<f32>, marcos: &mut Vec<(usize, Marco)>) -> usize {
        let inicio = saida.len();
        let mut feitos = 0usize;
        let fade = self.quadros_de_fade();
        while feitos < n {
            let Some(atual) = self.atual.as_mut() else { break };
            // Hora de começar o crossfade: a de agora está nos últimos segundos e há uma seguinte pronta.
            if self.saindo.is_none()
                && fade > 0
                && self.seguinte.is_some()
                && atual.restante().is_some_and(|r| r <= fade && r > 0)
            {
                let restante = atual.restante().unwrap_or(0);
                if let (Some(saindo), Some(entra)) = (self.atual.take(), self.seguinte.take()) {
                    marcos
                        .push((feitos, Marco::Comecou { origem: entra.origem.clone(), duracao_s: entra.duracao_s() }));
                    self.saindo = Some(Saindo { faixa: saindo, total: restante, feito: 0 });
                    self.atual = Some(entra);
                }
                continue;
            }
            let mut m = n - feitos;
            // O pedaço para exatamente onde o crossfade deve começar.
            if self.saindo.is_none()
                && fade > 0
                && self.seguinte.is_some()
                && let Some(r) = atual.restante()
                && r > fade
            {
                m = m.min((r - fade) as usize);
            }
            let antes = saida.len();
            let tirados = atual.tirar(m, saida);
            if let Some(s) = self.saindo.as_mut() {
                self.rascunho.clear();
                let deles = s.faixa.tirar(tirados, &mut self.rascunho);
                let total = s.total.max(1) as f32;
                for (i, quadro) in saida[antes..].as_chunks_mut::<2>().0.iter_mut().enumerate() {
                    let t = ((s.feito + i as u64) as f32 / total).min(1.0);
                    let (entra, sai) = ((t * FRAC_PI_2).sin(), (t * FRAC_PI_2).cos());
                    let (e, d) = if i < deles { (self.rascunho[i * 2], self.rascunho[i * 2 + 1]) } else { (0.0, 0.0) };
                    quadro[0] = quadro[0] * entra + e * sai;
                    quadro[1] = quadro[1] * entra + d * sai;
                }
                s.feito += tirados as u64;
                if s.feito >= s.total || (deles < tirados && s.faixa.acabou) {
                    self.saindo = None;
                }
            }
            feitos += tirados;
            if tirados < m {
                // A de agora acabou: a seguinte entra no quadro seguinte, ou tudo para.
                match self.seguinte.take() {
                    Some(entra) => {
                        marcos.push((
                            feitos,
                            Marco::Comecou { origem: entra.origem.clone(), duracao_s: entra.duracao_s() },
                        ));
                        self.atual = Some(entra);
                    }
                    None => {
                        self.atual = None;
                        self.saindo = None;
                        marcos.push((feitos, Marco::Fim));
                    }
                }
            }
        }
        self.eq.passar(&mut saida[inicio..]);
        for v in &mut saida[inicio..] {
            *v = limitar(*v);
        }
        feitos
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod testes {
    use super::*;

    /// Uma fonte de teste: `quadros` quadros com o mesmo valor, entregues aos poucos.
    struct Constante {
        valor: f32,
        quadros: u64,
        lidos: u64,
        taxa: u32,
        informa_duracao: bool,
    }

    impl Leitor for Constante {
        fn proximo(&mut self, saida: &mut Vec<f32>) -> bool {
            let k = (self.quadros - self.lidos).min(300);
            if k == 0 {
                return false;
            }
            saida.extend(std::iter::repeat_n(self.valor, k as usize * 2));
            self.lidos += k;
            true
        }
        fn duracao_s(&self) -> f64 {
            if self.informa_duracao { self.quadros as f64 / f64::from(self.taxa) } else { 0.0 }
        }
        fn ir(&mut self, segundos: f64) -> f64 {
            self.lidos = ((segundos * f64::from(self.taxa)) as u64).min(self.quadros);
            self.lidos as f64 / f64::from(self.taxa)
        }
    }

    const TAXA: u32 = 1000;

    fn faixa(nome: &str, valor: f32, quadros: u64) -> Faixa {
        let leitor = Constante { valor, quadros, lidos: 0, taxa: TAXA, informa_duracao: true };
        Faixa::nova(Origem::from(nome), Box::new(leitor), 1.0, TAXA)
    }

    fn tudo(m: &mut Mistura) -> (Vec<f32>, Vec<(usize, Marco)>) {
        let (mut som, mut marcos, mut base) = (Vec::new(), Vec::new(), 0);
        loop {
            let mut aqui = Vec::new();
            let k = m.produzir(128, &mut som, &mut aqui);
            marcos.extend(aqui.into_iter().map(|(q, marco)| (q + base, marco)));
            base += k;
            if k < 128 {
                return (som, marcos);
            }
        }
    }

    #[test]
    fn sem_intervalo_a_seguinte_entra_no_quadro_seguinte() {
        let mut m = Mistura::nova(TAXA);
        m.tocar(faixa("a", 0.5, 1000));
        m.preparar(Some(faixa("b", -0.5, 500)));
        let (som, marcos) = tudo(&mut m);
        assert_eq!(som.len(), 1500 * 2, "nenhum quadro de silêncio entre as duas");
        assert_eq!(som[999 * 2], 0.5);
        assert_eq!(som[1000 * 2], -0.5);
        assert_eq!(marcos, [(1000, Marco::Comecou { origem: "b".into(), duracao_s: 0.5 }), (1500, Marco::Fim)]);
    }

    #[test]
    fn crossfade_sobrepoe_e_mantem_o_volume() {
        let mut m = Mistura::nova(TAXA);
        m.ajustar_crossfade(0.2);
        m.tocar(faixa("a", 0.5, 1000));
        m.preparar(Some(faixa("b", 0.5, 1000)));
        let (som, marcos) = tudo(&mut m);
        // As duas se sobrepõem por 200 quadros: 1000 + 1000 - 200.
        assert_eq!(som.len(), 1800 * 2);
        assert_eq!(marcos[0], (800, Marco::Comecou { origem: "b".into(), duracao_s: 1.0 }));
        // No meio do crossfade, mesma potência: as duas a 0,5 somam ~0,707 (sin + cos de 45°, vezes 0,5).
        let meio = som[900 * 2];
        assert!((meio - 0.5 * std::f32::consts::SQRT_2).abs() < 0.02, "meio do crossfade = {meio}");
        assert!((som[799 * 2] - 0.5).abs() < 1e-6 && (som[1000 * 2] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn sem_duracao_conhecida_nao_ha_crossfade_mas_segue_sem_intervalo() {
        let mut m = Mistura::nova(TAXA);
        m.ajustar_crossfade(0.2);
        let leitor = Constante { valor: 0.3, quadros: 400, lidos: 0, taxa: TAXA, informa_duracao: false };
        m.tocar(Faixa::nova("a".into(), Box::new(leitor), 1.0, TAXA));
        m.preparar(Some(faixa("b", 0.3, 400)));
        let (som, _) = tudo(&mut m);
        assert_eq!(som.len(), 800 * 2);
    }

    #[test]
    fn ganho_e_limitador() {
        let mut m = Mistura::nova(TAXA);
        let leitor = Constante { valor: 0.5, quadros: 10, lidos: 0, taxa: TAXA, informa_duracao: true };
        m.tocar(Faixa::nova("a".into(), Box::new(leitor), 0.5, TAXA));
        let (som, _) = tudo(&mut m);
        assert!((som[0] - 0.25).abs() < 1e-6);
        // Ganho alto não passa de 1.
        let leitor = Constante { valor: 0.8, quadros: 10, lidos: 0, taxa: TAXA, informa_duracao: true };
        m.tocar(Faixa::nova("b".into(), Box::new(leitor), 3.0, TAXA));
        let (som, _) = tudo(&mut m);
        assert!(som.iter().all(|v| *v <= 1.0 && *v > 0.9));
        assert_eq!(limitar(0.5), 0.5);
        assert!(limitar(-5.0) >= -1.0 && limitar(-0.95) > -0.95 && limitar(-0.95) < -0.9);
    }

    #[test]
    fn ir_para_um_ponto_e_parar() {
        let mut m = Mistura::nova(TAXA);
        m.tocar(faixa("a", 0.5, 1000));
        assert_eq!(m.ir(0.75), Some(0.75));
        let (som, marcos) = tudo(&mut m);
        assert_eq!(som.len(), 250 * 2);
        assert_eq!(marcos, [(250, Marco::Fim)]);
        assert!(m.vazia());
    }

    #[test]
    fn equalizador_reforca_a_banda_e_desligado_nao_mexe() {
        let taxa = 48_000;
        let tom = |hz: f32| -> Vec<f32> {
            (0..taxa as usize / 4)
                .flat_map(|i| {
                    let v = (i as f32 * hz * 2.0 * PI / taxa as f32).sin() * 0.25;
                    [v, v]
                })
                .collect()
        };
        let energia = |v: &[f32]| v[v.len() / 2..].iter().map(|x| x * x).sum::<f32>();
        let mut eq = Equalizador::novo(taxa);
        let original = tom(1000.0);
        let mut som = original.clone();
        eq.passar(&mut som);
        assert_eq!(som, original, "sem ganho, o som passa igual");
        eq.ajustar(&[0.0, 0.0, 0.0, 0.0, 0.0, 6.0]);
        let mut reforcado = original.clone();
        eq.passar(&mut reforcado);
        let razao = energia(&reforcado) / energia(&original);
        // +6 dB em potência é ~4x.
        assert!((razao - 4.0).abs() < 0.6, "razão de energia {razao}");
        // Outra frequência, longe da banda, quase não muda.
        let mut eq = Equalizador::novo(taxa);
        eq.ajustar(&[0.0, 0.0, 0.0, 0.0, 0.0, 6.0]);
        let grave = tom(62.0);
        let mut g = grave.clone();
        eq.passar(&mut g);
        assert!((energia(&g) / energia(&grave) - 1.0).abs() < 0.1);
    }
}
