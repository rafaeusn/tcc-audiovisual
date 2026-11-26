//! As páginas de artista, álbum e playlist: as abas, os números como se leem, a data e o véu sobre a imagem do cabeçalho.
//! Só contas, testadas; a tela é `ui/detalhe.rs`.

use provedores::{Detalhe, Faixa, Item, TipoDeAlbum};

/// As abas da página do artista.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AbaDoArtista {
    #[default]
    VisaoGeral,
    Albuns,
    Singles,
    Coletaneas,
    Sobre,
}

impl AbaDoArtista {
    pub fn nome(self) -> &'static str {
        match self {
            Self::VisaoGeral => "Visão geral",
            Self::Albuns => "Álbuns",
            Self::Singles => "Singles e EPs",
            Self::Coletaneas => "Coletâneas",
            Self::Sobre => "Sobre",
        }
    }
}

/// De que aba um lançamento é (o que o serviço não disse é álbum).
fn aba_do_lancamento(item: &Item) -> Option<AbaDoArtista> {
    let Item::Album(a) = item else { return None };
    Some(match a.tipo {
        Some(TipoDeAlbum::Single | TipoDeAlbum::Ep) => AbaDoArtista::Singles,
        Some(TipoDeAlbum::Coletanea) => AbaDoArtista::Coletaneas,
        _ => AbaDoArtista::Albuns,
    })
}

/// Os lançamentos de uma aba, na ordem em que vieram.
pub fn lancamentos(albuns: &[Item], aba: AbaDoArtista) -> Vec<Item> {
    albuns.iter().filter(|i| aba_do_lancamento(i) == Some(aba)).cloned().collect()
}

/// As abas que têm o que mostrar (a Visão geral sempre).
pub fn abas_do_artista(albuns: &[Item], detalhe: Option<&Detalhe>) -> Vec<AbaDoArtista> {
    let mut abas = vec![AbaDoArtista::VisaoGeral];
    for aba in [AbaDoArtista::Albuns, AbaDoArtista::Singles, AbaDoArtista::Coletaneas] {
        if albuns.iter().any(|i| aba_do_lancamento(i) == Some(aba)) {
            abas.push(aba);
        }
    }
    let sobre = detalhe.is_some_and(|d| {
        d.descricao.as_deref().is_some_and(|t| !t.trim().is_empty())
            || !d.fotos.is_empty()
            || d.ouvintes.is_some()
            || d.seguidores.is_some()
    });
    if sobre {
        abas.push(AbaDoArtista::Sobre);
    }
    abas
}

/// `1.010.119.362`.
pub fn milhar(n: u64) -> String {
    let digitos = n.to_string();
    let mut saida = String::with_capacity(digitos.len() + digitos.len() / 3);
    for (i, c) in digitos.chars().enumerate() {
        if i > 0 && (digitos.len() - i).is_multiple_of(3) {
            saida.push('.');
        }
        saida.push(c);
    }
    saida
}

/// `28,3 milhões`, `1,2 bilhão`, `783.823`: só os grandes ficam resumidos.
pub fn resumido(n: u64) -> String {
    let em = |valor: f64, uma: &str, varias: &str| {
        let arredondado = (valor * 10.0).round() / 10.0;
        let texto = if arredondado.fract() == 0.0 {
            format!("{arredondado:.0}")
        } else {
            format!("{arredondado:.1}").replace('.', ",")
        };
        format!("{texto} {}", if arredondado < 2.0 { uma } else { varias })
    };
    match n {
        0..1_000_000 => milhar(n),
        1_000_000..1_000_000_000 => em(n as f64 / 1e6, "milhão", "milhões"),
        _ => em(n as f64 / 1e9, "bilhão", "bilhões"),
    }
}

/// `1 música`, `14 músicas`.
pub fn quantas(n: u64, uma: &str, varias: &str) -> String {
    if n == 1 { format!("1 {uma}") } else { format!("{} {varias}", milhar(n)) }
}

/// `31 min`, `1 h 12 min`, `2 h`.
pub fn duracao(segundos: f64) -> Option<String> {
    if !segundos.is_finite() || segundos < 1.0 {
        return None;
    }
    let minutos = (segundos / 60.0).round().max(1.0) as u64;
    Some(match (minutos / 60, minutos % 60) {
        (0, m) => format!("{m} min"),
        (h, 0) => format!("{h} h"),
        (h, m) => format!("{h} h {m} min"),
    })
}

const MESES: [&str; 12] = [
    "janeiro",
    "fevereiro",
    "março",
    "abril",
    "maio",
    "junho",
    "julho",
    "agosto",
    "setembro",
    "outubro",
    "novembro",
    "dezembro",
];

/// `12 de março de 2001`, `março de 2001`, `2001`. Uma data que não se entende fica como veio.
pub fn data_longa(data: &str) -> String {
    let partes: Vec<&str> = data.split('-').collect();
    let mes = partes.get(1).and_then(|m| m.parse::<usize>().ok()).and_then(|m| MESES.get(m.checked_sub(1)?));
    let dia = partes.get(2).and_then(|d| d.parse::<u8>().ok()).filter(|d| (1..=31).contains(d));
    match (partes.first(), mes, dia) {
        (Some(ano), Some(mes), Some(dia)) => format!("{dia} de {mes} de {ano}"),
        (Some(ano), Some(mes), None) => format!("{mes} de {ano}"),
        _ => data.to_string(),
    }
}

/// A duração da lista: a que o serviço mandou ou, com todas as faixas na mão, a soma delas.
pub fn duracao_da_lista(detalhe: Option<&Detalhe>, faixas: &[Faixa]) -> Option<f64> {
    if let Some(d) = detalhe.and_then(|d| d.duracao_s) {
        return Some(d);
    }
    let todas = detalhe.and_then(|d| d.total).is_none_or(|t| t as usize == faixas.len());
    let soma: f64 = faixas.iter().filter_map(|f| f.duracao_s).sum();
    (todas && !faixas.is_empty() && soma > 0.0).then_some(soma)
}

/// Os dados do cabeçalho de um álbum ou playlist, na ordem em que se leem: o ano, quantas e quanto tempo.
pub fn dados_da_lista(ano: Option<u16>, detalhe: Option<&Detalhe>, faixas: &[Faixa]) -> Vec<String> {
    let ano = detalhe.and_then(|d| d.data.as_deref()?.get(..4).map(str::to_string)).or(ano.map(|a| a.to_string()));
    let total = detalhe.and_then(|d| d.total).map(u64::from).or((!faixas.is_empty()).then_some(faixas.len() as u64));
    ano.into_iter()
        .chain(total.map(|n| quantas(n, "música", "músicas")))
        .chain(duracao_da_lista(detalhe, faixas).and_then(duracao))
        .collect()
}

/// A cor `#RRGGBB` em bytes.
pub fn cor(texto: &str) -> Option<[u8; 3]> {
    let hex = texto.strip_prefix('#').filter(|h| h.len() == 6 && h.bytes().all(|b| b.is_ascii_hexdigit()))?;
    let byte = |i: usize| u8::from_str_radix(&hex[i..i + 2], 16).ok();
    Some([byte(0)?, byte(2)?, byte(4)?])
}

/// Quanto da cor da folha vai por cima da imagem para o texto do tema ler bem nela (4,5:1). `claro` e `escuro` são os
/// percentis 95 e 5 do brilho da imagem (0 a 1), onde o texto fica. Tema escuro: o texto é claro, a imagem escurece até o
/// mais claro dela ficar em 0,38; tema claro: clareia até o mais escuro ficar em 0,9. Nunca some de todo nem fica intacta.
pub fn veu(claro: f32, escuro: f32, tema_escuro: bool) -> f32 {
    let (claro, escuro) = (claro.clamp(0.0, 1.0), escuro.clamp(0.0, 1.0));
    if !claro.is_finite() || !escuro.is_finite() {
        return 0.85;
    }
    if tema_escuro {
        (1.0 - 0.38 / claro.max(0.01)).clamp(0.35, 0.85)
    } else {
        ((0.9 - escuro) / (1.0 - escuro).max(0.01)).clamp(0.55, 0.92)
    }
}

/// O mesmo nome, sem acento, sem caixa e sem espaços a mais.
pub fn mesmo_nome(a: &str, b: &str) -> bool {
    let limpo = |s: &str| nucleo::texto::normalizar(s).split_whitespace().collect::<Vec<_>>().join(" ");
    let (a, b) = (limpo(a), limpo(b));
    !a.is_empty() && a == b
}

#[cfg(test)]
mod testes {
    use super::*;
    use provedores::Album;

    fn album(id: &str, tipo: Option<TipoDeAlbum>) -> Item {
        Item::Album(Album { id: id.into(), tipo, ..Default::default() })
    }

    #[test]
    fn as_abas_do_artista_so_com_conteudo() {
        let albuns = [
            album("a", Some(TipoDeAlbum::Album)),
            album("b", None),
            album("c", Some(TipoDeAlbum::Ep)),
            album("d", Some(TipoDeAlbum::Single)),
            album("e", Some(TipoDeAlbum::Outro)),
        ];
        assert_eq!(
            abas_do_artista(&albuns, None),
            [AbaDoArtista::VisaoGeral, AbaDoArtista::Albuns, AbaDoArtista::Singles]
        );
        let ids =
            |aba| lancamentos(&albuns, aba).iter().map(|i| crate::paginas::chave_do_salvo(i).2).collect::<Vec<_>>();
        assert_eq!(ids(AbaDoArtista::Albuns), ["a", "b", "e"], "o que o serviço não disse é álbum");
        assert_eq!(ids(AbaDoArtista::Singles), ["c", "d"]);
        let sobre = Detalhe { descricao: Some("bio".into()), ..Default::default() };
        assert_eq!(abas_do_artista(&[], Some(&sobre)), [AbaDoArtista::VisaoGeral, AbaDoArtista::Sobre]);
        let vazio = Detalhe { descricao: Some("  ".into()), ..Default::default() };
        assert_eq!(
            abas_do_artista(&[], Some(&vazio)),
            [AbaDoArtista::VisaoGeral],
            "provedor antigo ou artista sem nada"
        );
    }

    #[test]
    fn numeros_como_se_leem() {
        assert_eq!(milhar(1_010_119_362), "1.010.119.362");
        assert_eq!((milhar(0), milhar(999), milhar(1000)), ("0".into(), "999".into(), "1.000".into()));
        assert_eq!(resumido(28_289_213), "28,3 milhões");
        assert_eq!(resumido(1_000_000), "1 milhão");
        assert_eq!(resumido(1_960_000), "2 milhões");
        assert_eq!(resumido(1_200_000_000), "1,2 bilhão");
        assert_eq!(resumido(783_823), "783.823");
        assert_eq!(quantas(1, "música", "músicas"), "1 música");
        assert_eq!(quantas(2000, "música", "músicas"), "2.000 músicas");
        assert_eq!(duracao(1860.0).as_deref(), Some("31 min"));
        assert_eq!(duracao(4320.0).as_deref(), Some("1 h 12 min"));
        assert_eq!(duracao(7200.0).as_deref(), Some("2 h"));
        assert_eq!(duracao(20.0).as_deref(), Some("1 min"));
        assert_eq!((duracao(0.0), duracao(f64::NAN)), (None, None));
    }

    #[test]
    fn datas_em_portugues() {
        assert_eq!(data_longa("2001-03-12"), "12 de março de 2001");
        assert_eq!(data_longa("2001-12"), "dezembro de 2001");
        assert_eq!(data_longa("2001"), "2001");
        assert_eq!(data_longa("2001-13-40"), "2001-13-40", "o que não se entende fica como veio");
    }

    #[test]
    fn os_dados_da_lista() {
        let faixa = |s| Faixa { duracao_s: Some(s), ..Default::default() };
        let faixas = [faixa(1000.0), faixa(860.0)];
        assert_eq!(dados_da_lista(Some(2001), None, &faixas), ["2001", "2 músicas", "31 min"]);
        // A playlist tem 2000 faixas, e só vieram duas: a soma mentiria.
        let grande = Detalhe { total: Some(2000), ..Default::default() };
        assert_eq!(dados_da_lista(None, Some(&grande), &faixas), ["2.000 músicas"]);
        let datado = Detalhe { data: Some("1997-01-20".into()), duracao_s: Some(3600.0), ..Default::default() };
        assert_eq!(dados_da_lista(Some(2001), Some(&datado), &[]), ["1997", "1 h"]);
        assert!(dados_da_lista(None, None, &[]).is_empty(), "álbum que não veio");
        let uma = [faixa(200.0)];
        assert_eq!(dados_da_lista(None, None, &uma), ["1 música", "3 min"], "álbum de uma faixa");
    }

    #[test]
    fn o_veu_garante_a_leitura() {
        // Imagem branca no tema escuro: escurece até o branco virar 0,38.
        let branca = veu(1.0, 0.9, true);
        assert!((1.0 - branca - 0.38).abs() < 0.01, "{branca}");
        // Imagem escura: o mínimo, para não apagar a foto.
        assert_eq!(veu(0.2, 0.0, true), 0.35);
        // Tema claro, imagem preta: clareia até 0,9.
        let preta = veu(0.1, 0.0, false);
        assert!((preta - 0.9).abs() < 0.01, "{preta}");
        assert_eq!(veu(1.0, 1.0, false), 0.55);
        assert_eq!(veu(f32::NAN, 0.5, true), 0.85, "sem medida, o véu forte");
        assert_eq!(cor("#B0A840"), Some([0xB0, 0xA8, 0x40]));
        assert_eq!((cor("B0A840"), cor("#B0A84"), cor("#GGGGGG")), (None, None, None));
    }

    #[test]
    fn o_mesmo_nome_sem_acento_nem_caixa() {
        assert!(mesmo_nome("Beyoncé", "beyonce"));
        assert!(mesmo_nome("Daft  Punk", "daft punk"));
        assert!(!mesmo_nome("Daft Punk", "Daft Punk Tribute"));
        assert!(!mesmo_nome("", ""));
    }
}
