//! Casar uma faixa entre serviços: achar, numa lista de resultados de outro serviço, a mesma gravação.
//!
//! A conta vem do MixMusic (MIT): com o mesmo ISRC, é a mesma; senão, uma nota de 0 a 1 que pesa o título limpo
//! (45%), o artista (35%) e a duração (20%), com um bônus pequeno para a faixa de álbum em vez de vídeo. Toca
//! quando a nota passa de [`LIMIAR`].

use crate::modelo::Faixa;
use nucleo::texto::normalizar;
use std::collections::HashMap;

/// A partir desta nota, o par é a mesma música.
pub const LIMIAR: f32 = 0.6;

/// Palavras que, num parêntese ou depois de " - ", não mudam a gravação ("Remastered", "Official Video"...).
/// "Remix", "Ao Vivo", "Acústico", "Sped Up" ficam: são outra gravação.
const RUIDO: [&str; 22] = [
    "remaster",
    "radio edit",
    "official",
    "oficial",
    "video",
    "vídeo",
    "audio",
    "áudio",
    "lyric",
    "letra",
    "visualizer",
    "explicit",
    "clean",
    "hd",
    "hq",
    "4k",
    "mv",
    "single version",
    "album version",
    "original mix",
    "feat",
    "prod",
];

fn e_ruido(trecho: &str) -> bool {
    let t = trecho.to_lowercase();
    let palavras: Vec<&str> = t.split(|c: char| !c.is_alphanumeric()).filter(|p| !p.is_empty()).collect();
    RUIDO.iter().any(|r| {
        if r.contains(' ') {
            t.contains(r)
        } else {
            // Palavra inteira (ou começo dela: "remastered", "lyrics"), para "hd" não pegar "shdow".
            palavras.iter().any(|p| p.starts_with(r))
        }
    }) || t.trim_start().starts_with("ft.")
}

/// Tira os parênteses e colchetes de ruído e o "feat." do título.
fn sem_ruido(titulo: &str) -> String {
    let mut saida = String::with_capacity(titulo.len());
    let mut resto = titulo;
    while let Some(i) = resto.find(['(', '[']) {
        let fecha = if resto[i..].starts_with('(') { ')' } else { ']' };
        saida.push_str(&resto[..i]);
        match resto[i + 1..].find(fecha) {
            Some(j) => {
                let dentro = &resto[i + 1..i + 1 + j];
                if !e_ruido(dentro) {
                    saida.push_str(&resto[i..i + 2 + j]);
                }
                resto = &resto[i + 2 + j..];
            }
            None => {
                resto = &resto[i..];
                break;
            }
        }
    }
    saida.push_str(resto);
    // "Nome - Remastered 2011", "Nome - Radio Edit": o trecho depois do traço sai se for ruído.
    let partes: Vec<&str> = saida.split(" - ").collect();
    let mut titulo: Vec<&str> = vec![partes[0]];
    titulo.extend(partes[1..].iter().filter(|p| !e_ruido(p)));
    let junto = titulo.join(" - ");
    // "Nome feat. Fulano" / "Nome ft. Fulano": o convidado já está nos artistas.
    let minusculo = junto.to_lowercase();
    let corte = [" feat. ", " feat ", " ft. ", " featuring "].iter().filter_map(|m| minusculo.find(m)).min();
    match corte {
        Some(c) => junto[..c].to_string(),
        None => junto,
    }
}

/// O título para comparar: sem ruído, minúsculas, sem acento.
pub fn limpar_titulo(titulo: &str) -> String {
    normalizar(&sem_ruido(titulo))
}

/// O artista para comparar: sem o " - Topic" dos canais automáticos do YouTube nem o "VEVO" grudado no nome.
pub fn limpar_artista(artista: &str) -> String {
    let mut a = normalizar(artista);
    for sufixo in [" topic", "vevo", " official", " oficial"] {
        if let Some(sem) = a.strip_suffix(sufixo) {
            a = sem.trim_end().to_string();
        }
    }
    a
}

/// Os nomes dentro de uma lista de artistas ("A & B", "A, B", "A x B", "A feat. B" viram A e B).
fn nomes(artistas: &[String]) -> Vec<String> {
    let mut saida = Vec::new();
    for a in artistas {
        let mut texto = format!(" {} ", a.to_lowercase());
        for sep in [" feat. ", " feat ", " ft. ", " featuring ", " & ", " x ", " e ", " and ", ","] {
            texto = texto.replace(sep, "\u{1f}");
        }
        saida.extend(texto.split('\u{1f}').map(limpar_artista).filter(|n| !n.is_empty()));
    }
    saida
}

fn bigramas(s: &str) -> HashMap<(char, char), u32> {
    let cs: Vec<char> = s.chars().collect();
    let mut m = HashMap::new();
    for par in cs.windows(2) {
        *m.entry((par[0], par[1])).or_insert(0) += 1;
    }
    m
}

/// Quanto dois textos (já limpos) se parecem, de 0 a 1 (coeficiente de Dice nos pares de letras).
pub fn parecido(a: &str, b: &str) -> f32 {
    if a == b {
        return if a.is_empty() { 0.0 } else { 1.0 };
    }
    let (ba, bb) = (bigramas(a), bigramas(b));
    let (na, nb): (u32, u32) = (ba.values().sum(), bb.values().sum());
    if na + nb == 0 {
        return 0.0;
    }
    let comuns: u32 = ba.iter().map(|(k, v)| (*v).min(*bb.get(k).unwrap_or(&0))).sum();
    2.0 * comuns as f32 / (na + nb) as f32
}

/// A nota da duração: até 2 s de diferença, inteira; de 12 s para cima, nada. Sem as duas, meio termo.
fn nota_da_duracao(a: Option<f64>, b: Option<f64>) -> f32 {
    match (a.filter(|d| d.is_finite() && *d > 0.0), b.filter(|d| d.is_finite() && *d > 0.0)) {
        (Some(a), Some(b)) => (1.0 - ((a - b).abs() - 2.0) / 10.0).clamp(0.0, 1.0) as f32,
        _ => 0.5,
    }
}

/// A nota (0 a 1) de `candidato` ser a mesma gravação que `alvo`.
pub fn nota(alvo: &Faixa, candidato: &Faixa) -> f32 {
    if let (Some(a), Some(b)) = (alvo.isrc_limpo(), candidato.isrc_limpo()) {
        return if a == b { 1.0 } else { 0.0_f32.max(nota_sem_isrc(alvo, candidato) - 0.3) };
    }
    nota_sem_isrc(alvo, candidato)
}

fn nota_sem_isrc(alvo: &Faixa, candidato: &Faixa) -> f32 {
    let (artistas_alvo, artistas_cand) = (nomes(&alvo.artistas), nomes(&candidato.artistas));
    let mut artista =
        artistas_alvo.iter().flat_map(|a| artistas_cand.iter().map(move |b| parecido(a, b))).fold(0.0f32, f32::max);
    let titulo_alvo = limpar_titulo(&alvo.titulo);
    let mut titulo = parecido(&titulo_alvo, &limpar_titulo(&candidato.titulo));
    // Vídeos do YouTube: "Artista - Nome (Official Video)". Se o começo é o artista, o resto é o título.
    if let Some((antes, depois)) = sem_ruido(&candidato.titulo).split_once(" - ") {
        let quem = limpar_artista(antes);
        let do_artista = artistas_alvo.iter().map(|a| parecido(a, &quem)).fold(0.0f32, f32::max);
        if do_artista > 0.8 {
            titulo = titulo.max(parecido(&titulo_alvo, &normalizar(depois)));
            artista = artista.max(do_artista);
        }
    }
    let duracao = nota_da_duracao(alvo.duracao_s, candidato.duracao_s);
    let bonus = if candidato.video { 0.0 } else { 0.05 };
    (0.45 * titulo + 0.35 * artista + 0.20 * duracao + bonus).clamp(0.0, 1.0)
}

/// O melhor par para `alvo` entre os `candidatos` (o índice e a nota), se algum passar de [`LIMIAR`].
pub fn melhor_par(alvo: &Faixa, candidatos: &[Faixa]) -> Option<(usize, f32)> {
    candidatos
        .iter()
        .enumerate()
        .map(|(i, c)| (i, nota(alvo, c)))
        .filter(|(_, n)| *n >= LIMIAR)
        .max_by(|a, b| a.1.total_cmp(&b.1))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn f(titulo: &str, artistas: &[&str], duracao: f64, video: bool) -> Faixa {
        Faixa {
            servico: "s".into(),
            id: titulo.into(),
            titulo: titulo.into(),
            artistas: artistas.iter().map(|a| a.to_string()).collect(),
            duracao_s: Some(duracao),
            video,
            ..Default::default()
        }
    }

    #[test]
    fn limpa_o_ruido_e_guarda_o_que_muda_a_gravacao() {
        assert_eq!(limpar_titulo("Here Comes the Sun - Remastered 2009"), "here comes the sun");
        assert_eq!(limpar_titulo("Blinding Lights (Official Video)"), "blinding lights");
        assert_eq!(limpar_titulo("Levels [Radio Edit]"), "levels");
        assert_eq!(limpar_titulo("Song (feat. Someone) [Lyrics]"), "song");
        assert_eq!(limpar_titulo("Song feat. Someone"), "song");
        assert_eq!(limpar_titulo("Song (Remix)"), "song remix");
        assert_eq!(limpar_titulo("Canção (Ao Vivo)"), "cancao ao vivo");
        assert_eq!(limpar_titulo("Shadow (HD)"), "shadow");
        assert_eq!(limpar_titulo("Parêntese (aberto"), "parentese aberto");
        assert_eq!(limpar_artista("Daft Punk - Topic"), "daft punk");
        assert_eq!(limpar_artista("TaylorSwiftVEVO"), "taylorswift");
        assert_eq!(nomes(&["A & B".into(), "C feat. D".into()]), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn a_mesma_musica_em_outro_servico_passa() {
        let spotify = f("Around the World", &["Daft Punk"], 429.0, false);
        let topico = f("Around the World", &["Daft Punk - Topic"], 428.0, false);
        let clipe = f("Daft Punk - Around The World (Official Music Video)", &["DaftPunkVEVO"], 431.0, true);
        assert!(nota(&spotify, &topico) > 0.95, "{}", nota(&spotify, &topico));
        assert!(nota(&spotify, &clipe) > LIMIAR, "{}", nota(&spotify, &clipe));
        assert!(nota(&spotify, &topico) > nota(&spotify, &clipe), "a faixa do álbum ganha do clipe");
    }

    #[test]
    fn outra_musica_ou_outra_versao_perde() {
        let alvo = f("One More Time", &["Daft Punk"], 320.0, false);
        let outra = f("Harder Better Faster Stronger", &["Daft Punk"], 224.0, false);
        let remix = f("One More Time (Remix)", &["Daft Punk"], 410.0, false);
        let homonima = f("One More Time", &["Britney Spears"], 211.0, false);
        assert!(nota(&alvo, &outra) < LIMIAR, "{}", nota(&alvo, &outra));
        assert!(nota(&alvo, &remix) < nota(&alvo, &f("One More Time", &["Daft Punk"], 321.0, false)));
        assert!(nota(&alvo, &homonima) < LIMIAR, "{}", nota(&alvo, &homonima));
    }

    #[test]
    fn isrc_decide() {
        let mut a = f("Nome", &["X"], 100.0, false);
        let mut b = f("Outro nome qualquer", &["Y"], 300.0, true);
        a.isrc = Some("BR-ABC-26-00001".into());
        b.isrc = Some("brabc2600001".into());
        assert_eq!(nota(&a, &b), 1.0);
        b.isrc = Some("BRABC2600002".into());
        assert!(nota(&a, &b) < LIMIAR);
    }

    #[test]
    fn melhor_par_escolhe_ou_desiste() {
        let alvo = f("Strobe", &["deadmau5"], 637.0, false);
        let candidatos = vec![
            f("Strobe (Radio Edit)", &["deadmau5"], 213.0, false),
            f("Strobe", &["deadmau5"], 636.0, false),
            f("Ghosts n Stuff", &["deadmau5"], 330.0, false),
        ];
        assert_eq!(melhor_par(&alvo, &candidatos).map(|(i, _)| i), Some(1));
        assert_eq!(melhor_par(&alvo, &candidatos[2..]), None);
        assert_eq!(melhor_par(&alvo, &[]), None);
    }

    #[test]
    fn numeros_e_textos_estranhos_nao_quebram() {
        let vazio = Faixa::default();
        let n = nota(&vazio, &vazio);
        assert!((0.0..=1.0).contains(&n));
        assert_eq!(parecido("", ""), 0.0);
        assert_eq!(parecido("a", "b"), 0.0);
        assert_eq!(nota_da_duracao(Some(f64::NAN), Some(3.0)), 0.5);
        let estranho = f("((([[[", &["", "&&&"], -5.0, false);
        assert!((0.0..=1.0).contains(&nota(&estranho, &estranho)));
    }
}
