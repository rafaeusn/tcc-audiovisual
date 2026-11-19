//! Texto para comparar e buscar, igual em todo o app (biblioteca, letras).

/// A letra sem acento (as do português e mais algumas comuns em nomes de música).
pub fn sem_acento(c: char) -> char {
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

/// Forma de comparar e buscar: minúsculas, sem acento, só letras e números, um espaço entre as palavras. As chaves
/// das letras e o índice de busca da biblioteca dependem disto: mudar a conta muda as chaves já gravadas.
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

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn normaliza_para_comparar() {
        assert_eq!(normalizar("  Coração  Vagabundo!! "), "coracao vagabundo");
        assert_eq!(normalizar("  Coração (Ao Vivo) "), "coracao ao vivo");
        assert_eq!(normalizar("AC/DC"), "ac dc");
        assert_eq!(normalizar("Señor Å"), "senor a");
        assert_eq!(normalizar(""), "");
    }
}
