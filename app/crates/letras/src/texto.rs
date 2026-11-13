//! Arruma os nomes que os apps publicam, para comparar e para perguntar.

/// Palavras que, dentro de parênteses ou depois de um traço, não fazem parte do nome da música.
const RUIDO: [&str; 16] = [
    "official",
    "oficial",
    "video",
    "vídeo",
    "audio",
    "áudio",
    "lyric",
    "lyrics",
    "letra",
    "legendado",
    "visualizer",
    "remaster",
    "remastered",
    "remasterizado",
    "hd",
    "4k",
];

fn sem_acento(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        outro => outro,
    }
}

/// Forma de comparar: minúsculas, sem acento, só letras e números, um espaço entre as palavras.
pub fn normalizar(s: &str) -> String {
    let mut saida = String::with_capacity(s.len());
    for c in s.chars().flat_map(char::to_lowercase).map(sem_acento) {
        if c.is_alphanumeric() {
            saida.push(c);
        } else if !saida.ends_with(' ') && !saida.is_empty() {
            saida.push(' ');
        }
    }
    saida.trim_end().to_string()
}

fn tem_ruido(trecho: &str) -> bool {
    let n = normalizar(trecho);
    n.split(' ').any(|palavra| RUIDO.iter().any(|r| normalizar(r) == palavra))
}

/// Tira os trechos entre `abre` e `fecha` que só trazem ruído ("(Official Video)", "[4K]").
fn sem_trechos(texto: &str, abre: char, fecha: char) -> String {
    let mut saida = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(i) = resto.find(abre) {
        let Some(f) = resto[i..].find(fecha) else { break };
        let dentro = &resto[i + abre.len_utf8()..i + f];
        saida.push_str(&resto[..i]);
        if !tem_ruido(dentro) {
            saida.push_str(&resto[i..i + f + fecha.len_utf8()]);
        }
        resto = &resto[i + f + fecha.len_utf8()..];
    }
    saida.push_str(resto);
    saida
}

fn arrumar_espacos(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Título e artista prontos para a busca.
///
/// - "(Official Video)", "[Lyrics]", " - Remastered 2011" saem do título.
/// - Canais do YouTube: "Fulano - Topic" e "FulanoVEVO" viram "Fulano".
/// - Título no formato "Artista - Música" (comum no navegador): separa, quando o artista está vazio ou é o
///   mesmo nome do canal.
pub fn limpar(titulo: &str, artista: &str) -> (String, String) {
    let mut artista = arrumar_espacos(artista);
    if let Some(a) = artista.strip_suffix(" - Topic") {
        artista = a.to_string();
    }
    if let Some(a) = artista.strip_suffix("VEVO").filter(|a| !a.is_empty()) {
        artista = a.trim_end().to_string();
    }

    let mut titulo = arrumar_espacos(&sem_trechos(&sem_trechos(titulo, '(', ')'), '[', ']'));
    // Sufixos depois de um traço: "Música - Remastered 2011".
    while let Some((antes, depois)) = titulo.rsplit_once(" - ") {
        if tem_ruido(depois) && !antes.trim().is_empty() {
            titulo = antes.trim().to_string();
        } else {
            break;
        }
    }
    if let Some((esquerda, direita)) = titulo.split_once(" - ") {
        let (e, a) = (normalizar(esquerda).replace(' ', ""), normalizar(&artista).replace(' ', ""));
        let mesmo_nome = !e.is_empty() && (a.contains(&e) || e.contains(&a));
        if !direita.trim().is_empty() && (a.is_empty() || mesmo_nome) {
            artista = esquerda.trim().to_string();
            titulo = direita.trim().to_string();
        }
    }
    (titulo, artista)
}

/// Identidade de uma faixa para guardar a letra: artista, título e a duração arredondada (0 = desconhecida).
/// A duração entra porque a mesma música existe em vários cortes, e a letra de um não serve no outro.
pub fn chave(titulo: &str, artista: &str, duracao_s: Option<f64>) -> String {
    let duracao = duracao_s.filter(|d| d.is_finite() && *d > 0.0).map_or(0, |d| d.round() as i64);
    format!("{}|{}|{}", normalizar(artista), normalizar(titulo), duracao)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn normaliza_para_comparar() {
        assert_eq!(normalizar("  Coração  Vagabundo!! "), "coracao vagabundo");
        assert_eq!(normalizar("AC/DC"), "ac dc");
        assert_eq!(normalizar("É..."), "e");
        assert_eq!(normalizar(""), "");
    }

    #[test]
    fn tira_o_ruido_do_titulo() {
        assert_eq!(limpar("Nome da Música (Official Video)", "Banda").0, "Nome da Música");
        assert_eq!(
            limpar("Nome da Música [Lyrics] (feat. Outra Pessoa)", "Banda").0,
            "Nome da Música (feat. Outra Pessoa)"
        );
        assert_eq!(limpar("Nome da Música - Remastered 2011", "Banda").0, "Nome da Música");
        assert_eq!(limpar("Nome da Música - 2009 Remaster", "Banda").0, "Nome da Música");
        assert_eq!(limpar("Parte Um - Parte Dois", "Banda"), ("Parte Um - Parte Dois".into(), "Banda".into()));
        assert_eq!(
            limpar("Ao Vivo (Ao Vivo em Teste)", "Banda").0,
            "Ao Vivo (Ao Vivo em Teste)",
            "versão ao vivo é outra"
        );
    }

    #[test]
    fn separa_artista_do_titulo_do_navegador() {
        assert_eq!(
            limpar("Banda Tal - Nome da Música (Vídeo Oficial)", ""),
            ("Nome da Música".into(), "Banda Tal".into())
        );
        assert_eq!(limpar("Banda Tal - Nome da Música", "BandaTalVEVO"), ("Nome da Música".into(), "Banda Tal".into()));
        assert_eq!(limpar("Nome da Música", "Banda Tal - Topic"), ("Nome da Música".into(), "Banda Tal".into()));
        // O canal é de outra pessoa: não dá para saber quem é o artista, fica como veio.
        assert_eq!(limpar("Um - Dois", "Canal Qualquer"), ("Um - Dois".into(), "Canal Qualquer".into()));
    }

    #[test]
    fn chave_ignora_caixa_e_acento() {
        assert_eq!(chave("Canção", "BANDA", Some(200.4)), "banda|cancao|200");
        assert_eq!(chave("cancao", "Banda", Some(200.0)), chave("Canção", "banda", Some(199.6)));
        assert_eq!(chave("x", "y", None), "y|x|0");
        assert_eq!(chave("x", "y", Some(f64::NAN)), "y|x|0");
    }
}
