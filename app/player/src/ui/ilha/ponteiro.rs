//! De onde vem o ponteiro da Ilha: o mouse de verdade ou, num teste, um roteiro.
//!
//! O roteiro (`TCCPLAYER_ILHA_ROTEIRO=<arquivo>`) deixa testar o hover, os cliques e os arrastos sem mexer no mouse de
//! ninguém. Cada linha é `segundos x y botão`: o tempo desde que a Ilha apareceu, o ponto em frações da área de
//! trabalho (0 a 1; um pouco fora vale, para testar a beira) e o botão (1 apertado, 0 solto). Entre duas linhas o
//! ponteiro anda em linha reta. `#` começa um comentário. Uma linha `segundos comando nome` dá um comando à Ilha
//! nesse instante (`trava`, `topo`, `baixo`, `esquerda`, `direita`, `monitor`). O relógio do roteiro começa quando
//! aparece o arquivo `<roteiro>.go` (quem testa o cria na hora certa). As frações valem para a área de trabalho do
//! monitor em que a Ilha estava nessa hora (passando de 0 ou de 1, o ponteiro vai para o monitor ao lado). Com o
//! roteiro, o mouse de verdade não conta.

use std::path::PathBuf;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Passo {
    t: f32,
    x: f32,
    y: f32,
    apertado: bool,
}

pub(super) enum Ponteiro {
    Sistema,
    Roteiro {
        passos: Vec<Passo>,
        comandos: Vec<(f32, String)>,
        gatilho: PathBuf,
        inicio: Option<Instant>,
        /// A área de trabalho (pixels da tela) quando o roteiro começou: as frações valem para ela.
        area: (i32, i32, i32, i32),
        apertado_antes: bool,
    },
}

impl Ponteiro {
    /// O mouse de verdade, ou o roteiro de `TCCPLAYER_ILHA_ROTEIRO` quando ele existe e é válido.
    pub(super) fn do_ambiente() -> Self {
        let Some(caminho) = std::env::var_os("TCCPLAYER_ILHA_ROTEIRO") else { return Self::Sistema };
        match std::fs::read_to_string(&caminho) {
            Ok(texto) => {
                let (passos, comandos) = (ler(&texto), ler_comandos(&texto));
                log::info!("ilha-diag roteiro do ponteiro com {} passos e {} comandos", passos.len(), comandos.len());
                let mut gatilho = PathBuf::from(&caminho).into_os_string();
                gatilho.push(".go");
                Self::Roteiro {
                    passos,
                    comandos,
                    gatilho: gatilho.into(),
                    inicio: None,
                    area: (0, 0, 1, 1),
                    apertado_antes: false,
                }
            }
            Err(e) => {
                log::warn!("roteiro do ponteiro da Ilha ilegível ({}): {e}", caminho.to_string_lossy());
                Self::Sistema
            }
        }
    }

    /// Começa o relógio do roteiro quando o arquivo de partida (`<roteiro>.go`) aparece, com as frações valendo para
    /// a área de trabalho `area_agora` (pixels da tela). Devolve se começou agora.
    pub(super) fn vigiar(&mut self, area_agora: (i32, i32, i32, i32)) -> bool {
        if let Self::Roteiro { gatilho, inicio, area, .. } = self
            && inicio.is_none()
            && gatilho.exists()
        {
            *inicio = Some(Instant::now());
            *area = area_agora;
            return true;
        }
        false
    }

    /// Onde está o ponteiro, em pixels da tela. No roteiro, o ponto vem em frações da área de trabalho de quando ele
    /// começou; antes do primeiro passo, o ponteiro está longe.
    pub(super) fn cursor(&self) -> Option<(i32, i32)> {
        match self {
            Self::Sistema => integracao::ilha::cursor(),
            Self::Roteiro { passos, inicio, area, .. } => {
                let p = agora(passos, *inicio)?;
                let (w, h) = ((area.2 - area.0) as f32, (area.3 - area.1) as f32);
                Some(((area.0 as f32 + p.x * w).round() as i32, (area.1 as f32 + p.y * h).round() as i32))
            }
        }
    }

    /// O botão esquerdo está apertado.
    pub(super) fn apertado(&self) -> bool {
        match self {
            Self::Sistema => integracao::ilha::botao_apertado(),
            Self::Roteiro { passos, inicio, .. } => agora(passos, *inicio).is_some_and(|p| p.apertado),
        }
    }

    /// O botão desceu desde a última pergunta. Só o roteiro: o clique do mouse de verdade chega pelo GPUI.
    pub(super) fn desceu(&mut self) -> bool {
        let apertado = self.apertado();
        match self {
            Self::Sistema => false,
            Self::Roteiro { apertado_antes, .. } => {
                let desceu = apertado && !*apertado_antes;
                *apertado_antes = apertado;
                desceu
            }
        }
    }

    pub(super) fn e_roteiro(&self) -> bool {
        matches!(self, Self::Roteiro { .. })
    }

    /// Os comandos do roteiro cuja hora chegou (cada um sai uma vez só).
    pub(super) fn comandos_da_hora(&mut self) -> Vec<String> {
        let Self::Roteiro { comandos, inicio: Some(inicio), .. } = self else { return Vec::new() };
        let t = inicio.elapsed().as_secs_f32();
        let (prontos, depois): (Vec<_>, Vec<_>) = comandos.drain(..).partition(|(quando, _)| *quando <= t);
        *comandos = depois;
        prontos.into_iter().map(|(_, nome)| nome).collect()
    }
}

/// As linhas `segundos comando nome` do roteiro.
fn ler_comandos(texto: &str) -> Vec<(f32, String)> {
    texto
        .lines()
        .filter_map(|l| {
            let mut partes = l.split('#').next().unwrap_or("").split_whitespace();
            let t: f32 = partes.next()?.parse().ok()?;
            (partes.next()? == "comando" && t.is_finite()).then_some(())?;
            Some((t, partes.next()?.to_string()))
        })
        .collect()
}

/// Lê as linhas do roteiro (as estragadas são puladas) em ordem de tempo.
fn ler(texto: &str) -> Vec<Passo> {
    let mut passos: Vec<Passo> = texto
        .lines()
        .filter_map(|l| {
            let l = l.split('#').next().unwrap_or("").trim();
            let v: Vec<f32> = l.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            match v.as_slice() {
                [t, x, y, b] if [*t, *x, *y].iter().all(|v| v.is_finite()) => {
                    Some(Passo { t: *t, x: *x, y: *y, apertado: *b > 0.5 })
                }
                _ => None,
            }
        })
        .collect();
    passos.sort_by(|a, b| a.t.total_cmp(&b.t));
    passos
}

/// O ponto do roteiro agora: entre dois passos, em linha reta; o botão é o do passo de antes.
fn agora(passos: &[Passo], inicio: Option<Instant>) -> Option<Passo> {
    let t = inicio?.elapsed().as_secs_f32();
    passo_em(passos, t)
}

fn passo_em(passos: &[Passo], t: f32) -> Option<Passo> {
    let i = passos.partition_point(|p| p.t <= t).checked_sub(1)?;
    let a = passos[i];
    let Some(b) = passos.get(i + 1) else { return Some(a) };
    let k = if b.t > a.t { ((t - a.t) / (b.t - a.t)).clamp(0.0, 1.0) } else { 1.0 };
    Some(Passo { t, x: a.x + (b.x - a.x) * k, y: a.y + (b.y - a.y) * k, apertado: a.apertado })
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_o_roteiro_e_anda_em_linha_reta() {
        let passos = ler("# hover e arrasto\n0.0 0.5 0.0 0\n1.0 0.5 0.5 1  # aperta\nlixo aqui\n2.0 0.1 0.5 1\n");
        assert_eq!(passos.len(), 3);
        assert_eq!(passo_em(&passos, -1.0), None, "antes do começo, longe");
        let meio = passo_em(&passos, 0.5).unwrap_or(passos[0]);
        assert!((meio.y - 0.25).abs() < 1e-6 && !meio.apertado);
        let arrastando = passo_em(&passos, 1.5).unwrap_or(passos[0]);
        assert!((arrastando.x - 0.3).abs() < 1e-6 && arrastando.apertado);
        assert_eq!(passo_em(&passos, 9.0).map(|p| (p.x, p.apertado)), Some((0.1, true)), "depois do fim, parado");
        assert_eq!(
            ler_comandos(
                "0 0.5 0.5 0
2.5 comando trava
3 comando
"
            ),
            vec![(2.5, "trava".to_string())]
        );
    }
}
