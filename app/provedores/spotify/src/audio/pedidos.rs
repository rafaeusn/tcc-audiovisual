//! Os pedidos HTTP do servidor local de áudio, em funções puras: o caminho (com o token) e a faixa de bytes pedida.
//! Separado do servidor para os casos chatos (faixa invertida, fora do arquivo, lixo) terem teste sem abrir porta.

/// O que o cliente pediu, já conferido contra o tamanho do arquivo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Faixa {
    pub de: u64,
    /// Quantos bytes vão no corpo.
    pub quantos: u64,
    /// `206` (parte do arquivo) ou `200` (o arquivo inteiro, do começo).
    pub parcial: bool,
}

impl Faixa {
    /// O arquivo inteiro.
    pub fn inteira(tamanho: u64) -> Self {
        Self { de: 0, quantos: tamanho, parcial: false }
    }

    pub fn ate(&self) -> u64 {
        self.de + self.quantos
    }
}

/// A faixa de bytes de um cabeçalho `Range`, limitada a `tamanho` bytes. `None` quando não há cabeçalho; um cabeçalho
/// que não se entende (ou que não sobra nada para mandar) vale como o arquivo inteiro, que é o que a RFC manda fazer.
///
/// Só o que o tocador pede: `bytes=<de>-` e `bytes=<de>-<ate>`. Vários intervalos (`bytes=0-9,20-29`) não são
/// atendidos por inteiro: vale o primeiro, em vez de recusar e travar a música.
pub fn faixa(cabecalho: Option<&str>, tamanho: u64) -> Faixa {
    let Some(texto) = cabecalho else { return Faixa::inteira(tamanho) };
    let Some(lista) = texto.trim().strip_prefix("bytes=") else { return Faixa::inteira(tamanho) };
    let primeiro = lista.split(',').next().unwrap_or("").trim();
    let (de, ate) = primeiro.split_once('-').unwrap_or(("", ""));
    let de = de.trim();
    let ate = ate.trim();
    let inteira = Faixa::inteira(tamanho);
    if de.is_empty() {
        // `bytes=-N`: os últimos N bytes. Ninguém pede isso para tocar, e contar do fim só daria confusão.
        return inteira;
    }
    let Ok(de) = de.parse::<u64>() else { return inteira };
    if de >= tamanho {
        // Fora do arquivo: nada para mandar. O tocador prefere isto a um laço.
        return Faixa { de: tamanho, quantos: 0, parcial: true };
    }
    let ate = if ate.is_empty() {
        tamanho - 1
    } else {
        match ate.parse::<u64>() {
            Ok(n) if n >= de => n.min(tamanho - 1),
            Ok(_) => return inteira,
            Err(_) => return inteira,
        }
    };
    Faixa { de, quantos: ate - de + 1, parcial: true }
}

/// O token do caminho (`/a/<token>/<id>.<ext>`): o primeiro pedaço com letras e números depois da raiz. O token é
/// sorteado a cada abertura e só serve para o tocador não precisar de outra credencial; quem não o tem recebe 404.
#[cfg(test)]
pub fn token_do_caminho(caminho: &str) -> Option<&str> {
    caminho.split('/').find(|p| !p.is_empty()).filter(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// A extensão do arquivo no fim do caminho (`/a/<token>/abc.ogg` → `ogg`), quando parece uma. A parte depois de `?` ou
/// `#` (que o tocador não manda, mas um cliente qualquer pode) fica de fora.
pub fn extensao_do_caminho(caminho: &str) -> Option<String> {
    let caminho = caminho.split(['?', '#']).next().unwrap_or("");
    let ultimo = caminho.rsplit('/').find(|p| !p.is_empty())?;
    let (_, ext) = ultimo.rsplit_once('.')?;
    let ext = ext.to_ascii_lowercase();
    (!ext.is_empty() && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric())).then_some(ext)
}

/// Os pedaços do caminho, já sem a query: `/a/<token>/<id>.<ext>` → `["a", "<token>", "<id>.<ext>"]`.
pub fn partes_do_caminho(caminho: &str) -> Vec<&str> {
    let sem_query = caminho.split(['?', '#']).next().unwrap_or("");
    sem_query.trim_start_matches('/').split('/').collect()
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;

    const TAM: u64 = 1000;

    #[test]
    fn sem_cabecalho_vai_o_arquivo_inteiro() {
        assert_eq!(faixa(None, TAM), Faixa { de: 0, quantos: TAM, parcial: false });
        assert_eq!(faixa(Some("bytes=0-"), 0), Faixa { de: 0, quantos: 0, parcial: true });
    }

    #[test]
    fn as_faixas_que_o_tocador_pede() {
        assert_eq!(faixa(Some("bytes=0-"), TAM), Faixa { de: 0, quantos: TAM, parcial: true });
        assert_eq!(faixa(Some("bytes=100-"), TAM), Faixa { de: 100, quantos: 900, parcial: true });
        assert_eq!(faixa(Some("bytes=100-199"), TAM), Faixa { de: 100, quantos: 100, parcial: true });
        // Um fim além do arquivo é cortado no fim.
        assert_eq!(faixa(Some("bytes=900-9999"), TAM), Faixa { de: 900, quantos: 100, parcial: true });
        assert_eq!(faixa(Some("bytes=0-99999"), TAM), Faixa { de: 0, quantos: TAM, parcial: true });
    }

    #[test]
    fn lixo_e_faixa_invertida_valem_o_arquivo_inteiro() {
        for cabecalho in ["bytes=abc-", "bytes=200-100", "items=0-10", "bytes=-", "bytes=-100", "  "] {
            assert_eq!(faixa(Some(cabecalho), TAM), Faixa::inteira(TAM), "{cabecalho}");
        }
    }

    #[test]
    fn pedir_depois_do_fim_nao_manda_nada() {
        assert_eq!(faixa(Some("bytes=1000-"), TAM), Faixa { de: 1000, quantos: 0, parcial: true });
        assert_eq!(faixa(Some("bytes=5000-"), TAM), Faixa { de: 1000, quantos: 0, parcial: true });
    }

    #[test]
    fn varios_intervalos_vale_o_primeiro() {
        assert_eq!(faixa(Some("bytes=10-19,50-59"), TAM), Faixa { de: 10, quantos: 10, parcial: true });
    }

    #[test]
    fn o_token_e_a_extensao_do_caminho() {
        assert_eq!(token_do_caminho("/a/abc123/xyz.ogg"), Some("a"));
        assert_eq!(token_do_caminho("/"), None);
        assert_eq!(token_do_caminho(""), None);
        assert_eq!(extensao_do_caminho("/a/t/id.ogg"), Some("ogg".into()));
        assert_eq!(extensao_do_caminho("/a/t/id"), None);
        assert_eq!(extensao_do_caminho("/a/t/.ogg"), Some("ogg".into()));
        assert_eq!(extensao_do_caminho("/a/t/id.ogg?x=1"), Some("ogg".into()), "a query fica de fora");
        assert_eq!(partes_do_caminho("/a/t/id.ogg?x=1"), ["a", "t", "id.ogg"]);
        assert_eq!(partes_do_caminho("a/t"), ["a", "t"]);
    }
}
