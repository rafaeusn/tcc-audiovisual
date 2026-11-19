//! Leitura do formato LRC: `[mm:ss.cc] texto da linha`.

/// Arquivo vindo da rede: limita o tamanho antes de guardar e mostrar.
const MAXIMO_DE_LINHAS: usize = 3000;
const MAXIMO_POR_LINHA: usize = 300;

/// Uma linha da letra e o instante em que ela começa, em segundos da faixa.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Linha {
    pub t: f64,
    pub texto: String,
    /// Quando cada palavra começa, se o arquivo traz as marcas por palavra (`<00:12.30>`, o "LRC com palavras").
    /// Sem elas, o andamento dentro da linha é estimado (ver [`cantado`]).
    pub palavras: Vec<Palavra>,
}

/// Uma marca de tempo dentro da linha: a partir de `t`, a palavra que começa na letra `letra` está sendo cantada.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palavra {
    pub t: f64,
    /// Posição em letras (não em bytes) no texto da linha.
    pub letra: usize,
}

/// Sem marcas por palavra, a linha se enche no tempo até a próxima, mas nunca mais devagar que isso (para não
/// esticar o preenchimento por um trecho instrumental).
const LETRAS_POR_SEGUNDO_NO_MINIMO: f64 = 6.0;
/// Caractere de uso privado onde cada marca de palavra fica enquanto o texto é arrumado.
const MARCA: u32 = 0xE000;
const MAXIMO_DE_MARCAS: usize = 6000;

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

/// Arruma o texto da linha (espaços, caracteres de controle, tamanho) e separa as marcas de tempo por palavra
/// (`<00:12.30>`): cada uma vira a posição, em letras, onde a palavra começa.
fn texto_e_palavras(texto: &str) -> (String, Vec<Palavra>) {
    // Cada marca vira um caractere de uso privado; depois de arrumar os espaços, a posição dele diz a letra.
    let mut bruto = String::with_capacity(texto.len());
    let mut tempos = Vec::new();
    let mut resto = texto;
    while let Some(abre) = resto.find('<') {
        let Some(fecha) = resto[abre..].find('>') else { break };
        let dentro = &resto[abre + 1..abre + fecha];
        bruto.push_str(&resto[..abre]);
        match tempo(dentro) {
            Some(t) if tempos.len() < MAXIMO_DE_MARCAS => {
                bruto.push(char::from_u32(MARCA + tempos.len() as u32).unwrap_or(' '));
                tempos.push(t);
            }
            Some(_) => {}
            None => bruto.push_str(&resto[abre..=abre + fecha]),
        }
        resto = &resto[abre + fecha + 1..];
    }
    bruto.push_str(resto);
    let arrumado = bruto.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut limpo = String::with_capacity(arrumado.len());
    let mut palavras = Vec::new();
    let mut letras = 0usize;
    for c in arrumado.chars() {
        let codigo = c as u32;
        if (MARCA..MARCA + MAXIMO_DE_MARCAS as u32).contains(&codigo) {
            if let Some(&t) = tempos.get((codigo - MARCA) as usize) {
                palavras.push(Palavra { t, letra: letras });
            }
        } else if !c.is_control() && letras < MAXIMO_POR_LINHA {
            limpo.push(c);
            letras += 1;
        }
    }
    // As marcas nas pontas deixam espaços sobrando: apara e acerta as posições.
    let antes = limpo.chars().take_while(|c| *c == ' ').count();
    let limpo = limpo.trim().to_string();
    let letras = limpo.chars().count();
    // Uma marca logo antes de um espaço aponta para a palavra seguinte.
    let caracteres: Vec<char> = limpo.chars().collect();
    for p in &mut palavras {
        p.letra = p.letra.saturating_sub(antes);
        if caracteres.get(p.letra) == Some(&' ') {
            p.letra += 1;
        }
        p.letra = p.letra.min(letras);
    }
    (limpo, palavras)
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
        let (limpo, palavras) = texto_e_palavras(resto);
        for t in tempos {
            if linhas.len() < MAXIMO_DE_LINHAS {
                // Uma linha repetida (vários tempos) só leva as marcas por palavra na primeira vez: elas são do
                // primeiro tempo.
                let marcas = if palavras.first().is_some_and(|p| (p.t - t).abs() < 30.0) {
                    palavras.clone()
                } else {
                    Vec::new()
                };
                linhas.push(Linha { t, texto: limpo.clone(), palavras: marcas });
            }
        }
    }
    for l in &mut linhas {
        l.t = (l.t + atraso_s).max(0.0);
        for p in &mut l.palavras {
            p.t = (p.t + atraso_s).max(0.0);
        }
    }
    linhas.sort_by(|a, b| a.t.total_cmp(&b.t));
    linhas
}

/// A linha que vale no instante `t`: a última que já começou. `None` antes da primeira.
pub fn linha_em(linhas: &[Linha], t: f64) -> Option<usize> {
    linhas.partition_point(|l| l.t <= t).checked_sub(1)
}

/// Quantas letras da linha `i` já foram cantadas no instante `t`, com fração (7,4 = sete letras e quase metade da
/// oitava), para encher a linha como num karaokê. Com marcas por palavra, segue as marcas; sem elas, enche no tempo
/// até a próxima linha (um pouco antes dela), sem passar de um ritmo de canto plausível.
pub fn cantado(linhas: &[Linha], i: usize, t: f64) -> f32 {
    let Some(l) = linhas.get(i) else { return 0.0 };
    let n = l.texto.chars().count();
    if n == 0 || t < l.t {
        return 0.0;
    }
    let proxima = linhas.get(i + 1).map(|p| p.t).filter(|p| *p > l.t);
    if !l.palavras.is_empty() {
        let k = l.palavras.partition_point(|p| p.t <= t);
        let Some(atual) = k.checked_sub(1).and_then(|k| l.palavras.get(k)) else { return 0.0 };
        let (fim_t, fim_letra) = match l.palavras.get(k) {
            Some(p) => (p.t, p.letra),
            None => (proxima.unwrap_or(atual.t + 0.8).min(atual.t + 2.0), n),
        };
        let duracao = (fim_t - atual.t).max(0.05);
        let feito = ((t - atual.t) / duracao).clamp(0.0, 1.0);
        let letras = atual.letra as f64 + feito * fim_letra.saturating_sub(atual.letra) as f64;
        return letras.min(n as f64) as f32;
    }
    let mais_devagar = (n as f64 / LETRAS_POR_SEGUNDO_NO_MINIMO).max(1.0);
    let duracao = match proxima {
        Some(p) => ((p - l.t) * 0.85).min(mais_devagar),
        None => mais_devagar,
    }
    .max(0.2);
    (((t - l.t) / duracao).clamp(0.0, 1.0) * n as f64) as f32
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
        assert_eq!(l[0], Linha { t: 5.0, texto: "primeira linha".into(), palavras: Vec::new() });
        assert_eq!(l[1].t, 12.5);
        assert!((l[2].t - 62.345).abs() < 1e-9);
    }

    #[test]
    fn uma_linha_com_varios_tempos_se_repete() {
        let l = ler_lrc("[00:10.00][00:40.00]refrão de teste\n[00:20.00]\n");
        assert_eq!(l.len(), 3);
        assert_eq!(l[0].texto, "refrão de teste");
        assert_eq!(l[1], Linha { t: 20.0, texto: String::new(), palavras: Vec::new() }, "linha vazia marca uma pausa");
        assert_eq!(l[2].t, 40.0);
    }

    #[test]
    fn offset_e_marcas_por_palavra() {
        let l = ler_lrc("[offset:+500]\n[00:10.00]<00:10.00>uma <00:10.40>palavra <3\n[00:00.20]começo");
        assert_eq!(l[0].t, 0.0, "nunca fica negativo");
        assert_eq!(l[1].t, 9.5);
        assert_eq!(l[1].texto, "uma palavra <3");
        assert_eq!(l[1].palavras, vec![Palavra { t: 9.5, letra: 0 }, Palavra { t: 9.9, letra: 4 }]);
    }

    #[test]
    fn marcas_por_palavra_viram_posicoes() {
        let l = ler_lrc("[00:10.00]<00:10.00>uma <00:10.40>palavra   <00:11.00>");
        assert_eq!(l[0].texto, "uma palavra");
        assert_eq!(
            l[0].palavras,
            vec![Palavra { t: 10.0, letra: 0 }, Palavra { t: 10.4, letra: 4 }, Palavra { t: 11.0, letra: 11 }]
        );
    }

    #[test]
    fn enche_pelas_marcas_por_palavra() {
        let l = ler_lrc("[00:10.00]<00:10.00>uma <00:10.40>palavra<00:11.00>\n[00:13.00]outra");
        assert_eq!(cantado(&l, 0, 9.0), 0.0);
        assert!((cantado(&l, 0, 10.2) - 2.0).abs() < 1e-3, "metade de 'uma '");
        assert!((cantado(&l, 0, 10.7) - 7.5).abs() < 1e-3, "metade de 'palavra'");
        assert_eq!(cantado(&l, 0, 12.0), 11.0);
    }

    #[test]
    fn sem_marcas_enche_no_tempo_da_linha_sem_esticar_o_intervalo() {
        // 12 letras e 4 s até a próxima: enche em 2 s (o ritmo mais lento aceito), não em 4.
        let l = ler_lrc("[00:05.00]doze letras!\n[00:09.00]b\n[00:09.30]c");
        assert_eq!(cantado(&l, 0, 5.0), 0.0);
        assert!((cantado(&l, 0, 6.0) - 6.0).abs() < 1e-3);
        assert_eq!(cantado(&l, 0, 7.5), 12.0);
        // Linha curta seguida logo de outra: enche antes da próxima chegar.
        assert_eq!(cantado(&l, 1, 9.0 + 0.3 * 0.85), 1.0);
        assert_eq!(cantado(&l, 7, 1.0), 0.0, "linha que não existe");
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
