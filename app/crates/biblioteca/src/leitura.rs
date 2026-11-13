//! Achar os arquivos de música de uma pasta e ler as etiquetas de cada um.

use crate::Musica;
use lofty::file::{AudioFile, TaggedFileExt};
use lofty::tag::Accessor;
use std::path::Path;

/// Os tipos de arquivo que o tocador sabe tocar.
pub const EXTENSOES: [&str; 6] = ["mp3", "flac", "wav", "ogg", "m4a", "aac"];
/// Pastas dentro de pastas: até onde a varredura desce.
const PROFUNDIDADE: usize = 12;
/// Texto de etiqueta vem de fora: limita o tamanho antes de guardar e mostrar.
const TETO_TEXTO: usize = 300;

fn limpo(texto: &str) -> String {
    texto.chars().filter(|c| !c.is_control()).take(TETO_TEXTO).collect::<String>().trim().to_string()
}

fn e_musica(caminho: &Path) -> bool {
    caminho.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSOES.iter().any(|x| x.eq_ignore_ascii_case(e)))
}

/// Lê as etiquetas de um arquivo. Nunca falha: o que não der para ler fica vazio, e o título cai para o nome
/// do arquivo.
pub fn ler(caminho: &Path) -> Musica {
    let mut m = Musica { caminho: caminho.to_path_buf(), ..Default::default() };
    if let Ok(arquivo) = lofty::read_from_path(caminho) {
        m.duracao_s = arquivo.properties().duration().as_secs_f64();
        if let Some(etiqueta) = arquivo.primary_tag().or_else(|| arquivo.first_tag()) {
            m.titulo = etiqueta.title().map(|t| limpo(&t)).unwrap_or_default();
            m.artista = etiqueta.artist().map(|t| limpo(&t)).unwrap_or_default();
            m.album = etiqueta.album().map(|t| limpo(&t)).unwrap_or_default();
            m.numero = etiqueta.track().unwrap_or(0);
        }
    }
    if m.titulo.is_empty() {
        m.titulo = caminho.file_stem().map(|n| limpo(&n.to_string_lossy())).unwrap_or_default();
    }
    m
}

/// A imagem da capa embutida no arquivo (os bytes do JPEG ou PNG), se houver.
pub fn capa(caminho: &Path) -> Option<Vec<u8>> {
    let arquivo = lofty::read_from_path(caminho).ok()?;
    let etiqueta = arquivo.primary_tag().or_else(|| arquivo.first_tag())?;
    etiqueta.pictures().first().map(|p| p.data().to_vec())
}

/// Percorre `pasta` e as pastas dentro dela, chamando `ao_achar` com o caminho de cada arquivo de música.
/// `continuar` é consultada a cada pasta: devolver falso interrompe a varredura (o app está fechando).
pub fn varrer(pasta: &Path, ao_achar: &mut dyn FnMut(&Path), continuar: &dyn Fn() -> bool) {
    descer(pasta, 0, ao_achar, continuar);
}

fn descer(pasta: &Path, nivel: usize, ao_achar: &mut dyn FnMut(&Path), continuar: &dyn Fn() -> bool) {
    if nivel > PROFUNDIDADE || !continuar() {
        return;
    }
    let Ok(itens) = std::fs::read_dir(pasta) else { return };
    let mut pastas = Vec::new();
    for item in itens.flatten() {
        let caminho = item.path();
        // Atalhos e ligações não são seguidos: poderiam dar a volta e nunca acabar.
        match item.file_type() {
            Ok(tipo) if tipo.is_dir() => pastas.push(caminho),
            Ok(tipo) if tipo.is_file() && e_musica(&caminho) => ao_achar(&caminho),
            _ => {}
        }
    }
    for p in pastas {
        descer(&p, nivel + 1, ao_achar, continuar);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
pub(crate) mod testes {
    use super::*;
    use std::path::PathBuf;

    /// Um WAV de teste com meio segundo de silêncio.
    pub fn wav(caminho: &Path) {
        let (taxa, quadros) = (8000u32, 4000u32);
        let dados = quadros * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + dados).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&taxa.to_le_bytes());
        b.extend_from_slice(&(taxa * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&dados.to_le_bytes());
        b.resize(b.len() + dados as usize, 0);
        std::fs::create_dir_all(caminho.parent().expect("pasta")).expect("criar pasta");
        std::fs::write(caminho, b).expect("gravar");
    }

    pub fn pasta_de_teste(nome: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("tccplayer-teste-bib-{nome}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).expect("pasta");
        p
    }

    #[test]
    fn acha_so_musicas_e_desce_nas_pastas() {
        let pasta = pasta_de_teste("varrer");
        wav(&pasta.join("uma.wav"));
        wav(&pasta.join("Disco/02 - Segunda.WAV"));
        wav(&pasta.join("Disco/Lado B/tres.wav"));
        std::fs::write(pasta.join("capa.jpg"), b"x").expect("gravar");
        std::fs::write(pasta.join("notas.txt"), b"x").expect("gravar");
        let mut achados = Vec::new();
        varrer(&pasta, &mut |p| achados.push(p.to_path_buf()), &|| true);
        assert_eq!(achados.len(), 3);
        // Interromper para a varredura antes de achar qualquer coisa.
        let mut nenhum = 0;
        varrer(&pasta, &mut |_| nenhum += 1, &|| false);
        assert_eq!(nenhum, 0);
        varrer(&pasta.join("nao-existe"), &mut |_| nenhum += 1, &|| true);
        assert_eq!(nenhum, 0);
        let _ = std::fs::remove_dir_all(pasta);
    }

    #[test]
    fn sem_etiqueta_o_titulo_e_o_nome_do_arquivo() {
        let pasta = pasta_de_teste("ler");
        let caminho = pasta.join("Minha Música.wav");
        wav(&caminho);
        let m = ler(&caminho);
        assert_eq!(m.titulo, "Minha Música");
        assert_eq!(m.artista, "");
        assert!((m.duracao_s - 0.5).abs() < 0.05, "duração {}", m.duracao_s);
        assert_eq!(capa(&caminho), None);
        // Arquivo estragado ainda entra, só com o nome.
        let ruim = pasta.join("estragado.mp3");
        std::fs::write(&ruim, b"nada").expect("gravar");
        let m = ler(&ruim);
        assert_eq!((m.titulo.as_str(), m.duracao_s), ("estragado", 0.0));
        let _ = std::fs::remove_dir_all(pasta);
    }
}
