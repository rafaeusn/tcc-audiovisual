//! Leitura do formato LRC: `[mm:ss.cc] texto da linha`.

/// Arquivo vindo da rede: limita o tamanho antes de guardar e mostrar.
const MAXIMO_DE_LINHAS: usize = 3000;
const MAXIMO_POR_LINHA: usize = 300;

/// Uma linha da letra e o instante em que ela começa, em segundos da faixa.
#[derive(Clone, Debug, PartialEq)]
pub struct Linha {
    pub t: f64,
    pub texto: String,
}

/// "01:23.45", "1:23", "01:23.456" → segundos.
fn tempo(marca: &str) -> Option<f64> {
    let (min, resto) = marca.split_once(':')?;
    if min.is_empty() || min.len() > 3 || !min.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    // Alguns arquivos usam "mm:ss:cc".
    let resto = resto.replacen(':', ".", 1);
    if resto.is_empty() || !resto.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    let seg: f64 = resto.parse().ok()?;
    let min: f64 = min.parse().ok()?;
    (seg < 60.0).then_some(min * 60.0 + seg)
}

/// Tira as marcas de tempo por palavra (`<00:12.30>`) que alguns arquivos trazem no meio do texto.
fn sem_marcas_de_palavra(texto: &str) -> String {
    let mut limpo = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(abre) = resto.find('<') {
        let Some(fecha) = resto[abre..].find('>') else { break };
        let dentro = &resto[abre + 1..abre + fecha];
        limpo.push_str(&resto[..abre]);
        if tempo(dentro).is_none() {
            limpo.push_str(&resto[abre..=abre + fecha]);
        }
        resto = &resto[abre + fecha + 1..];
    }
    limpo.push_str(resto);
    limpo
}

/// Lê um texto LRC. Linhas sem tempo e etiquetas (`[ar:...]`) são ignoradas; `[offset:±ms]` desloca tudo.
/// O resultado vem em ordem de tempo. Texto que não é LRC devolve uma lista vazia.
pub fn ler_lrc(texto: &str) -> Vec<Linha> {
    let mut linhas = Vec::new();
    let mut atraso_s = 0.0;
    for bruta in texto.lines() {
        let mut resto = bruta.trim();
        let mut tempos: Vec<f64> = Vec::new();
        while let Some(depois) = resto.strip_prefix('[') {
            let Some(fim) = depois.find(']') else { break };
            let marca = depois[..fim].trim();
            if let Some(t) = tempo(marca) {
                tempos.push(t);
            } else if let Some(ms) = marca.strip_prefix("offset:").and_then(|v| v.trim().parse::<f64>().ok()) {
                // Pela convenção do formato, um valor positivo adianta a letra.
                if ms.is_finite() && ms.abs() < 600_000.0 {
                    atraso_s = -ms / 1000.0;
                }
            } else if tempos.is_empty() {
                // Etiqueta de informação: a linha inteira não é letra.
                break;
            }
            resto = depois[fim + 1..].trim_start();
        }
        if tempos.is_empty() {
            continue;
        }
        let limpo: String = sem_marcas_de_palavra(resto)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .filter(|c| !c.is_control())
            .take(MAXIMO_POR_LINHA)
            .collect();
        for t in tempos {
            if linhas.len() < MAXIMO_DE_LINHAS {
                linhas.push(Linha { t, texto: limpo.clone() });
            }
        }
    }
    for l in &mut linhas {
        l.t = (l.t + atraso_s).max(0.0);
    }
    linhas.sort_by(|a, b| a.t.total_cmp(&b.t));
    linhas
}

/// A linha que vale no instante `t`: a última que já começou. `None` antes da primeira.
pub fn linha_em(linhas: &[Linha], t: f64) -> Option<usize> {
    linhas.partition_point(|l| l.t <= t).checked_sub(1)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_tempos_e_ordena() {
        let l = ler_lrc(
            "[ar:Banda de Teste]\n[00:12.50]segunda linha\n[00:05.00] primeira   linha \nsem tempo\n[01:02.345]terceira",
        );
        assert_eq!(l.len(), 3);
        assert_eq!(l[0], Linha { t: 5.0, texto: "primeira linha".into() });
        assert_eq!(l[1].t, 12.5);
        assert!((l[2].t - 62.345).abs() < 1e-9);
    }

    #[test]
    fn uma_linha_com_varios_tempos_se_repete() {
        let l = ler_lrc("[00:10.00][00:40.00]refrão de teste\n[00:20.00]\n");
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].texto, "refrão de teste");
        assert_eq!(l[1], Linha { t: 20.0, texto: String::new() }, "linha vazia marca uma pausa");
        assert_eq!(l[2].t, 40.0);
    }

    #[test]
    fn offset_e_marcas_por_palavra() {
        let l = ler_lrc("[offset:+500]\n[00:10.00]<00:10.00>uma <00:10.40>palavra <3\n[00:00.20]começo");
        assert_eq!(l[0].t, 0.0, "nunca fica negativo");
        assert_eq!(l[1].t, 9.5);
        assert_eq!(l[1].texto, "uma palavra <3");
    }

    #[test]
    fn texto_que_nao_e_lrc() {
        assert!(ler_lrc("").is_empty());
        assert!(ler_lrc("só um texto\ncomum").is_empty());
        assert!(ler_lrc("[99:99.99]tempo impossível\n[abc]x\n[00:0a.00]y").is_empty());
        assert_eq!(ler_lrc("[1:05]curto\n[00:07:50]com dois pontos").len(), 2);
    }

    #[test]
    fn linha_do_instante() {
        let l = ler_lrc("[00:05.00]a\n[00:10.00]b\n[00:15.00]c");
        assert_eq!(linha_em(&l, 0.0), None);
        assert_eq!(linha_em(&l, 5.0), Some(0));
        assert_eq!(linha_em(&l, 9.99), Some(0));
        assert_eq!(linha_em(&l, 10.0), Some(1));
        assert_eq!(linha_em(&l, 999.0), Some(2));
        assert_eq!(linha_em(&[], 3.0), None);
    }

    #[test]
    fn entrada_enorme_e_limitada() {
        let texto: String =
            (0..5000).map(|i| format!("[{:02}:{:02}.00]{}\n", i / 60 % 100, i % 60, "x".repeat(900))).collect();
        let l = ler_lrc(&texto);
        assert_eq!(l.len(), MAXIMO_DE_LINHAS);
        assert!(l.iter().all(|x| x.texto.chars().count() == MAXIMO_POR_LINHA));
    }
}
