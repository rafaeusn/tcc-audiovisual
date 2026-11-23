//! Leitura do TTML (o XML de texto com tempos em que vêm as letras palavra por palavra da Better Lyrics e da
//! Unison) e conversão para o LRC com palavras (`[mm:ss.cc]<mm:ss.cc>palavra <mm:ss.cc>palavra<fim>`), que é como
//! o resto do app guarda e mostra as letras.
//!
//! Cada `<p begin>` é uma linha; os `<span begin>` dentro dela são as palavras. Os vocais de fundo
//! (`ttm:role="x-bg"`) vão no fim da linha, entre parênteses, como no MixMusic. Os trechos que não são cantados
//! (tradução, romanização embutidas, metadados) ficam de fora. Uma linha sem tempo, numa letra que tem outros
//! tempos, fica de fora; uma letra sem nenhum tempo não é sincronizada (devolve `None`).

use quick_xml::events::Event;
use quick_xml::reader::Reader;

/// Papéis de trechos que não são cantados.
const NAO_CANTADOS: [&str; 4] = ["x-translation", "x-roman", "x-romanization", "x-transliteration"];

/// "10.5", "10.500s", "1:01.44", "00:01:01.440" → segundos.
pub fn tempo_ttml(v: &str) -> Option<f64> {
    let v = v.trim().trim_end_matches('s');
    if v.is_empty() {
        return None;
    }
    let mut total = 0.0;
    for parte in v.split(':') {
        let n: f64 = parte.parse().ok()?;
        if !n.is_finite() || n < 0.0 {
            return None;
        }
        total = total * 60.0 + n;
    }
    Some(total)
}

/// Segundos → "mm:ss.cc" (minutos com dois dígitos ou mais).
fn marca(t: f64) -> String {
    let cs = (t.max(0.0) * 100.0).round() as u64;
    format!("{:02}:{:02}.{:02}", cs / 6000, (cs / 100) % 60, cs % 100)
}

#[derive(Default)]
struct LinhaTtml {
    t: Option<f64>,
    fim: Option<f64>,
    /// O texto da linha principal, já com as marcas de palavra.
    principal: String,
    /// Os vocais de fundo, com marcas.
    fundo: String,
    /// O fim da última palavra de cada parte (para a marca que fecha).
    fim_principal: Option<f64>,
    fim_fundo: Option<f64>,
    /// O tempo da última marca posta (as marcas não podem voltar no tempo).
    ultima: f64,
    tem_palavra: bool,
}

impl LinhaTtml {
    fn por_marca(&mut self, fundo: bool, t: f64) {
        let t = t.max(self.ultima);
        self.ultima = t;
        self.tem_palavra = true;
        let alvo = if fundo { &mut self.fundo } else { &mut self.principal };
        alvo.push('<');
        alvo.push_str(&marca(t));
        alvo.push('>');
    }

    fn por_texto(&mut self, fundo: bool, texto: &str) {
        if fundo { &mut self.fundo } else { &mut self.principal }.push_str(texto);
    }

    /// A linha em LRC com palavras, ou `None` se ela não tem tempo ou texto.
    fn em_lrc(&self) -> Option<String> {
        let t = self.t?;
        let mut texto = self.principal.split_whitespace().collect::<Vec<_>>().join(" ");
        let fundo = self.fundo.split_whitespace().collect::<Vec<_>>().join(" ");
        let fecha = |fim: Option<f64>, ultima: f64| fim.map(|f| format!("<{}>", marca(f.max(ultima))));
        if fundo.is_empty() {
            if self.tem_palavra
                && let Some(f) = fecha(self.fim_principal.or(self.fim), self.ultima)
            {
                texto.push_str(&f);
            }
        } else {
            texto.push_str(" (");
            texto.push_str(&fundo);
            if let Some(f) = fecha(self.fim_fundo.or(self.fim), self.ultima) {
                texto.push_str(&f);
            }
            texto.push(')');
        }
        let so_letras: String = texto.chars().filter(|c| !c.is_whitespace()).collect();
        // Sem nada além das marcas, a linha não tem o que mostrar.
        let tem_texto = so_letras.split(['<', '>']).enumerate().any(|(i, p)| i % 2 == 0 && !p.is_empty());
        tem_texto.then(|| format!("[{}]{}", marca(t), texto))
    }
}

/// O valor de um atributo pelo nome local (sem o prefixo: `ttm:role` → `role`).
fn atributo(e: &quick_xml::events::BytesStart<'_>, nome: &[u8]) -> Option<String> {
    e.attributes()
        .flatten()
        .find(|a| a.key.local_name().as_ref() == nome)
        .and_then(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0).ok().map(|v| v.into_owned()))
}

/// O texto de uma referência (`&amp;`, `&#39;`...).
fn referencia(nome: &str) -> String {
    match nome {
        "amp" => "&".into(),
        "lt" => "<".into(),
        "gt" => ">".into(),
        "quot" => "\"".into(),
        "apos" => "'".into(),
        n => n
            .strip_prefix("#x")
            .and_then(|h| u32::from_str_radix(h, 16).ok())
            .or_else(|| n.strip_prefix('#').and_then(|d| d.parse().ok()))
            .and_then(char::from_u32)
            .map(String::from)
            .unwrap_or_default(),
    }
}

/// Converte uma letra em TTML para o LRC com palavras. `None` quando o XML não se lê ou nenhuma linha tem tempo.
pub fn ttml_para_lrc(ttml: &str) -> Option<String> {
    let mut leitor = Reader::from_str(ttml);
    leitor.config_mut().trim_text(false);
    let mut linhas: Vec<(f64, String)> = Vec::new();
    let mut atual: Option<LinhaTtml> = None;
    // A pilha dos `span` abertos: (é fundo, ignora, tem tempo próprio e o fim dele).
    let mut pilha: Vec<(bool, bool, Option<f64>)> = Vec::new();
    let mut nos_metadados = 0usize;
    loop {
        let evento = match leitor.read_event() {
            Ok(e) => e,
            Err(_) => return None,
        };
        match evento {
            Event::Eof => break,
            Event::Start(e) => match e.local_name().as_ref() {
                b"metadata" | b"head" => nos_metadados += 1,
                b"p" if nos_metadados == 0 => {
                    let t = atributo(&e, b"begin").as_deref().and_then(tempo_ttml);
                    let fim = atributo(&e, b"end").as_deref().and_then(tempo_ttml);
                    atual = Some(LinhaTtml { t, fim, ultima: t.unwrap_or(0.0), ..Default::default() });
                    pilha.clear();
                }
                b"span" if atual.is_some() => {
                    let papel = atributo(&e, b"role").unwrap_or_default();
                    let (fundo_de_cima, ignora_de_cima) = pilha.last().map_or((false, false), |&(f, i, _)| (f, i));
                    let fundo = fundo_de_cima || papel == "x-bg";
                    let ignora = ignora_de_cima || NAO_CANTADOS.contains(&papel.as_str());
                    let comeco = atributo(&e, b"begin").as_deref().and_then(tempo_ttml);
                    let fim = atributo(&e, b"end").as_deref().and_then(tempo_ttml);
                    if let (Some(l), Some(t), false) = (atual.as_mut(), comeco, ignora) {
                        if l.t.is_none() {
                            l.t = Some(t);
                        }
                        l.por_marca(fundo, t);
                    }
                    pilha.push((fundo, ignora, if comeco.is_some() { fim } else { None }));
                }
                _ => {}
            },
            Event::End(e) => match e.local_name().as_ref() {
                b"metadata" | b"head" => nos_metadados = nos_metadados.saturating_sub(1),
                b"p" => {
                    if let Some(l) = atual.take()
                        && let (Some(t), Some(texto)) = (l.t, l.em_lrc())
                    {
                        linhas.push((t, texto));
                    }
                }
                b"span" => {
                    if let (Some((fundo, ignora, fim)), Some(l)) = (pilha.pop(), atual.as_mut())
                        && !ignora
                        && let Some(f) = fim
                    {
                        if fundo {
                            l.fim_fundo = Some(f);
                        } else {
                            l.fim_principal = Some(f);
                        }
                    }
                }
                _ => {}
            },
            Event::Text(t) => {
                if let (Some(l), Ok(texto)) = (atual.as_mut(), t.decode()) {
                    let (fundo, ignora) = pilha.last().map_or((false, false), |&(f, i, _)| (f, i));
                    if !ignora {
                        l.por_texto(fundo, &texto);
                    }
                }
            }
            Event::GeneralRef(r) => {
                if let (Some(l), Ok(nome)) = (atual.as_mut(), r.decode()) {
                    let (fundo, ignora) = pilha.last().map_or((false, false), |&(f, i, _)| (f, i));
                    if !ignora {
                        l.por_texto(fundo, &referencia(&nome));
                    }
                }
            }
            _ => {}
        }
    }
    if linhas.is_empty() {
        return None;
    }
    linhas.sort_by(|a, b| a.0.total_cmp(&b.0));
    Some(linhas.into_iter().map(|(_, l)| l).collect::<Vec<_>>().join("\n"))
}

/// A letra tem tempo por palavra (alguma linha com marcas `<..>`).
pub fn por_palavra(lrc: &str) -> bool {
    crate::ler_lrc(lrc).iter().any(|l| !l.palavras.is_empty())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use crate::ler_lrc;

    // Letras inventadas para os testes (nenhuma letra de música de verdade).
    const PALAVRA_POR_PALAVRA: &str = r#"<tt xmlns="http://www.w3.org/ns/ttml" xmlns:ttm="http://www.w3.org/ns/ttml#metadata">
<head><metadata><ttm:agent type="person" xml:id="v1"/><songwriters><songwriter>Alguém</songwriter></songwriters></metadata></head>
<body dur="1:01.440"><div begin="10.500" end="28.300">
<p begin="10.500" end="14.100" ttm:agent="v1"><span begin="10.500" end="11.200">bola</span> <span begin="11.200" end="11.800">azul</span> <span begin="11.800" end="14.100">gira</span></p>
<p begin="1:15.000" end="1:19.400"><span begin="1:15.000" end="1:16.000">sol</span> <span begin="1:16.000" end="1:19.400">&amp; mar</span><span ttm:role="x-bg"><span begin="1:17.500" end="1:19.000">eco</span></span></p>
<p begin="20.000" end="21.000"><span ttm:role="x-translation">não cantado</span><span begin="20.000" end="21.000">lua</span></p>
</div></body></tt>"#;

    #[test]
    fn tempos_em_todos_os_formatos() {
        assert_eq!(tempo_ttml("10.5"), Some(10.5));
        assert_eq!(tempo_ttml("10.500s"), Some(10.5));
        assert_eq!(tempo_ttml("1:01.44"), Some(61.44));
        assert_eq!(tempo_ttml("00:01:01.440"), Some(61.44));
        assert_eq!(tempo_ttml(""), None);
        assert_eq!(tempo_ttml("abc"), None);
        assert_eq!(tempo_ttml("-1"), None);
        assert_eq!(marca(61.444), "01:01.44");
        assert_eq!(marca(3725.0), "62:05.00", "mais de uma hora continua em minutos");
    }

    #[test]
    fn palavra_por_palavra_vira_lrc_com_palavras() {
        let lrc = ttml_para_lrc(PALAVRA_POR_PALAVRA).expect("lê");
        let linhas = ler_lrc(&lrc);
        assert_eq!(linhas.len(), 3, "{lrc}");
        // Em ordem de tempo; os metadados (o compositor) não viram linha.
        assert_eq!(linhas[0].texto, "bola azul gira");
        assert_eq!(linhas[0].t, 10.5);
        let inicios: Vec<(f64, usize)> = linhas[0].palavras.iter().map(|p| (p.t, p.letra)).collect();
        assert_eq!(inicios, vec![(10.5, 0), (11.2, 5), (11.8, 10), (14.1, 14)], "a última marca fecha a linha");
        // O trecho não cantado fica de fora.
        assert_eq!(linhas[1].texto, "lua");
        // O fundo entre parênteses, a referência resolvida, e as marcas nunca voltam no tempo.
        assert_eq!(linhas[2].texto, "sol & mar (eco)");
        assert!(linhas[2].palavras.windows(2).all(|w| w[0].t <= w[1].t && w[0].letra <= w[1].letra));
        assert!(por_palavra(&lrc));
    }

    #[test]
    fn por_linha_e_estragado() {
        let por_linha = r#"<tt><body><div><p begin="00:00:05.000" end="00:00:08.000">uma linha só</p><p>sem tempo</p></div></body></tt>"#;
        let lrc = ttml_para_lrc(por_linha).expect("lê");
        assert_eq!(lrc, "[00:05.00]uma linha só");
        assert!(!por_palavra(&lrc));
        assert_eq!(
            ttml_para_lrc("<tt><body><p>sem tempo nenhum</p></body></tt>"),
            None,
            "sem tempo não é sincronizada"
        );
        assert_eq!(ttml_para_lrc("<tt><body><p begin=\"1\">"), None.or(Some("[00:01.00]".into())).filter(|_| false));
        assert_eq!(ttml_para_lrc("isto não é xml <<<"), None);
        assert_eq!(ttml_para_lrc(""), None);
    }
}
