//! O áudio do Spotify pelo librespot: o mesmo protocolo que os clientes oficiais usam, com a conta da pessoa.
//!
//! O caminho de uma faixa: o metadado diz quais arquivos ela tem (Ogg 320, 160, 96...); o melhor que a conta tem direito
//! dá um `FileId`; a sessão pede a chave daquela faixa; o arquivo vem do CDN do Spotify **cifrado** e é decifrado em
//! memória. Só os bytes decifrados, na memória, saem daqui — quem os guarda é o [`super::cache`].
//!
//! **Nada é gravado em disco com áudio.** O `AudioFile` do librespot usa um arquivo temporário para os bytes ainda
//! cifrados (é o jeito dele de baixar aos pedaços); a pasta desse temporário é apagada a cada abertura do provedor
//! (`spotify.rs`) e vive dentro da pasta de dados dele. O áudio decifrado nunca toca o disco: ele existe só no
//! `Vec<u8>` que o cache devolve ao servidor local.
//!
//! Cada faixa fica **aberta** enquanto está em uso (um `AudioFile` por faixa, não um por pedaço): abrir de novo a cada
//! 128 KB faria o Spotify reabrir a conexão e pedir outra chave à toa.

use super::cache::Baixador;
use librespot_audio::AudioFile;
use librespot_audio::{AudioDecrypt, StreamLoaderController};
use librespot_core::error::ErrorKind;
use librespot_core::{FileId, Session, SpotifyId, SpotifyUri};
use librespot_metadata::Metadata;
use librespot_metadata::audio::file::{AudioFileFormat, AudioFiles};
use librespot_metadata::track::Track;
use provedores::Causa;
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

/// Quanto um pedaço pode demorar (o tocador desiste depois de 30 s sem pedaço).
const PRAZO_DO_PEDACO: Duration = Duration::from_secs(20);
/// Quanto o metadado da faixa pode demorar.
const PRAZO_DO_METADADO: Duration = Duration::from_secs(8);
/// Quanto a chave de áudio pode demorar (o librespot já desiste sozinho em 1,5 s).
const PRAZO_DA_CHAVE: Duration = Duration::from_secs(5);
/// Quantas faixas ficam com o arquivo aberto. Mais que isto, a menos usada fecha.
const FAIXAS_ABERTAS: usize = 6;

fn travar<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A falha do áudio com a causa do protocolo, para o Player decidir o que fazer (a cadeia em `player/src/cadeia.rs`).
#[derive(Clone, Debug, PartialEq)]
pub struct FalhaDeAudio {
    pub causa: Causa,
    pub mensagem: String,
}

impl FalhaDeAudio {
    pub fn nova(causa: Causa, mensagem: impl Into<String>) -> Self {
        Self { causa, mensagem: mensagem.into() }
    }
}

/// O melhor arquivo da faixa para esta conta: Ogg 320 (Premium), depois 160, 96, e MP3 (contas antigas).
///
/// O Spotify só manda os formatos que a conta permite, então pegar o melhor presente na lista é o que dá a maior
/// qualidade a que a pessoa tem direito.
pub fn escolher(files: &AudioFiles) -> Option<(AudioFileFormat, FileId)> {
    const PREFERENCIA: [AudioFileFormat; 6] = [
        AudioFileFormat::OGG_VORBIS_320,
        AudioFileFormat::OGG_VORBIS_160,
        AudioFileFormat::OGG_VORBIS_96,
        AudioFileFormat::MP3_320,
        AudioFileFormat::MP3_256,
        AudioFileFormat::MP3_160,
    ];
    PREFERENCIA.iter().find_map(|f| files.get(f).map(|id| (*f, *id)))
}

/// A extensão que o decodificador do Player entende (`ogg` para Vorbis, `mp3` para MP3).
pub fn extensao(formato: AudioFileFormat) -> &'static str {
    if AudioFiles::is_ogg_vorbis(formato) { "ogg" } else { "mp3" }
}

/// O nome do formato para a pessoa ler.
#[cfg_attr(not(test), expect(dead_code, reason = "a tela diz a qualidade que toca (Configurações › Serviços)"))]
pub fn nome_do_formato(formato: AudioFileFormat) -> &'static str {
    match formato {
        AudioFileFormat::OGG_VORBIS_320 => "Ogg 320",
        AudioFileFormat::OGG_VORBIS_160 => "Ogg 160",
        AudioFileFormat::OGG_VORBIS_96 => "Ogg 96",
        AudioFileFormat::MP3_320 => "MP3 320",
        AudioFileFormat::MP3_256 => "MP3 256",
        AudioFileFormat::MP3_160 => "MP3 160",
        AudioFileFormat::MP3_96 => "MP3 96",
        _ => "áudio do Spotify",
    }
}

/// A causa de um erro do librespot. O tipo do erro não basta: é o texto que separa "o Spotify recusou a chave" (derruba
/// o jeito de tocar) de "a rede caiu" (só pede para tentar de novo), e o librespot escreve os dois como `Unavailable`.
pub fn causa_do_erro(tipo: ErrorKind, texto: &str) -> Causa {
    let t = texto.to_ascii_lowercase();
    if t.contains("audio key") || t.contains("aes key") {
        return Causa::ChaveDeAudio;
    }
    if t.contains("premium") && t.contains("required") {
        return Causa::SemPremium;
    }
    if t.contains("bad credentials") || t.contains("unauthenticated") {
        return Causa::SessaoRecusada;
    }
    if t.contains("not available") || t.contains("unavailable") || t.contains("restricted") {
        return Causa::Indisponivel;
    }
    match tipo {
        ErrorKind::DeadlineExceeded | ErrorKind::Cancelled | ErrorKind::Aborted => Causa::Rede,
        ErrorKind::ResourceExhausted => Causa::Limite,
        ErrorKind::Unauthenticated | ErrorKind::PermissionDenied => Causa::SessaoRecusada,
        ErrorKind::Unavailable => Causa::Indisponivel,
        _ => Causa::Quebrado,
    }
}

/// O erro do librespot já com a causa e uma frase para a pessoa.
pub fn falha_do_librespot(e: librespot_core::Error) -> FalhaDeAudio {
    let causa = causa_do_erro(e.kind, &e.to_string());
    let mensagem = match causa {
        Causa::ChaveDeAudio => "o Spotify recusou a chave do áudio desta faixa".to_string(),
        Causa::SemPremium => "esta conta do Spotify não é Premium".to_string(),
        Causa::SessaoRecusada => "o Spotify recusou a sessão".to_string(),
        Causa::Indisponivel => "esta faixa não está disponível nesta conta".to_string(),
        Causa::Rede => "sem resposta do Spotify".to_string(),
        Causa::Limite => "o Spotify pediu para esperar".to_string(),
        _ => format!("o Spotify respondeu de um jeito inesperado ({e})"),
    };
    FalhaDeAudio::nova(causa, mensagem)
}

/// Uma faixa com o arquivo já aberto: o decifrador (que já tem a chave), o tamanho e o formato.
struct Aberta {
    decifrador: AudioDecrypt<AudioFile>,
    /// Para fechar a conexão do Spotify quando a faixa sai de uso.
    controlador: StreamLoaderController,
    tamanho: u64,
    formato: AudioFileFormat,
    usado: u64,
}

/// De onde vem a sessão do librespot. O `spotify.rs` entrega a de agora; os testes, uma de mentira (ou nenhuma).
pub trait FonteDaSessao: Send + Sync + 'static {
    fn agora(&self) -> Option<Session>;
}

/// O áudio do Spotify pela sessão de agora.
pub struct PelaSessao {
    /// Pedida a cada uso: a sessão pode cair e voltar (o `Login` reentra sozinho).
    sessao: Arc<dyn FonteDaSessao>,
    rt: Arc<tokio::runtime::Runtime>,
    abertas: Mutex<Abertas>,
}

#[derive(Default)]
struct Abertas {
    faixas: HashMap<String, Aberta>,
    relogio: u64,
}

impl PelaSessao {
    pub fn novo(sessao: Arc<dyn FonteDaSessao>, rt: Arc<tokio::runtime::Runtime>) -> Self {
        Self { sessao, rt, abertas: Mutex::default() }
    }

    fn sessao(&self) -> Result<Session, FalhaDeAudio> {
        self.sessao
            .agora()
            .filter(|s| !s.is_invalid())
            .ok_or_else(|| FalhaDeAudio::nova(Causa::SemConta, "Entre na sua conta do Spotify."))
    }

    /// O metadado da faixa, com prazo: um Spotify lento não pode prender a thread do tocador.
    fn metadado(&self, id: &str) -> Result<(SpotifyId, Track), FalhaDeAudio> {
        let (spotify_id, uri) = Self::identidade(id)?;
        let sessao = self.sessao()?;
        self.rt
            .block_on(async move {
                match tokio::time::timeout(PRAZO_DO_METADADO, Track::get(&sessao, &uri)).await {
                    Ok(Ok(t)) => Ok(t),
                    Ok(Err(e)) => Err(falha_do_librespot(e)),
                    Err(_) => Err(FalhaDeAudio::nova(Causa::Rede, "o Spotify não respondeu em 8 s")),
                }
            })
            .map(|t| (spotify_id, t))
    }

    /// O `SpotifyId` e o `SpotifyUri` de uma faixa pelo id base62.
    fn identidade(id: &str) -> Result<(SpotifyId, SpotifyUri), FalhaDeAudio> {
        let spotify_id = SpotifyId::from_base62(id)
            .map_err(|_| FalhaDeAudio::nova(Causa::Indisponivel, "este id não é de uma faixa do Spotify"))?;
        Ok((spotify_id, SpotifyUri::Track { id: spotify_id }))
    }

    /// Abre a faixa (se ainda não está aberta), com a chave e o arquivo. Fecha a menos usada quando passa do teto.
    fn abrir(&self, id: &str) -> Result<(u64, AudioFileFormat), FalhaDeAudio> {
        {
            let mut abertas = travar(&self.abertas);
            abertas.relogio += 1;
            let relogio = abertas.relogio;
            if let Some(a) = abertas.faixas.get_mut(id) {
                a.usado = relogio;
                return Ok((a.tamanho, a.formato));
            }
        }

        // A sessão antes do id: sem conta, o motivo útil é "entre na conta", e não "este id não é do Spotify".
        self.sessao()?;
        let (spotify_id, track) = self.metadado(id)?;
        let (formato, file_id) = escolher(&track.files)
            .ok_or_else(|| FalhaDeAudio::nova(Causa::Indisponivel, "o Spotify não oferece esta faixa nesta conta"))?;
        let sessao = self.sessao()?;
        let por_segundo = bytes_por_segundo(formato);
        let (chave, arquivo) = self.rt.block_on(async move {
            let chave = tokio::time::timeout(PRAZO_DA_CHAVE, sessao.audio_key().request(spotify_id, file_id))
                .await
                .map_err(|_| FalhaDeAudio::nova(Causa::Rede, "o Spotify não deu a chave em 5 s"))?
                .map_err(falha_do_librespot)?;
            let arquivo = tokio::time::timeout(PRAZO_DO_PEDACO, AudioFile::open(&sessao, file_id, por_segundo))
                .await
                .map_err(|_| FalhaDeAudio::nova(Causa::Rede, "o Spotify não abriu o arquivo em 20 s"))?
                .map_err(falha_do_librespot)?;
            Ok::<_, FalhaDeAudio>((chave, arquivo))
        })?;
        let controlador = arquivo.get_stream_loader_controller().map_err(falha_do_librespot)?;
        let tamanho = controlador.len() as u64;
        let mut abertas = travar(&self.abertas);
        abertas.relogio += 1;
        let relogio = abertas.relogio;
        abertas.faixas.insert(
            id.to_string(),
            Aberta {
                decifrador: AudioDecrypt::new(Some(chave), arquivo),
                controlador,
                tamanho,
                formato,
                usado: relogio,
            },
        );
        // Fecha as que sobraram (e solta a conexão do Spotify de cada uma).
        let mut fechar = Vec::new();
        while abertas.faixas.len() > FAIXAS_ABERTAS {
            let Some(velha) = abertas.faixas.iter().filter(|(k, _)| k.as_str() != id).min_by_key(|(_, a)| a.usado)
            else {
                break;
            };
            let velha = velha.0.clone();
            if let Some(a) = abertas.faixas.remove(&velha) {
                fechar.push(a.controlador);
            }
        }
        drop(abertas);
        for c in fechar {
            c.close();
        }
        Ok((tamanho, formato))
    }
}

/// Uns bytes por segundo por formato, para o librespot calcular quanto baixar de antemão.
fn bytes_por_segundo(formato: AudioFileFormat) -> usize {
    match formato {
        AudioFileFormat::OGG_VORBIS_320 | AudioFileFormat::MP3_320 => 40_000,
        AudioFileFormat::OGG_VORBIS_160
        | AudioFileFormat::MP3_160
        | AudioFileFormat::OGG_VORBIS_96
        | AudioFileFormat::MP3_96 => 20_000,
        _ => 12_000,
    }
}

impl Baixador for PelaSessao {
    fn tamanho(&self, id: &str) -> Result<u64, FalhaDeAudio> {
        // A sessão primeiro: sem conta, o motivo é "entre na conta", e não "id estranho".
        self.sessao()?;
        self.abrir(id).map(|(t, _)| t)
    }

    fn pedaco(&self, id: &str, de: u64, quantos: u64) -> Result<Vec<u8>, FalhaDeAudio> {
        self.abrir(id)?;
        let mut abertas = travar(&self.abertas);
        abertas.relogio += 1;
        let relogio = abertas.relogio;
        let a = abertas
            .faixas
            .get_mut(id)
            .ok_or_else(|| FalhaDeAudio::nova(Causa::Quebrado, "a faixa fechou no meio da leitura"))?;
        a.usado = relogio;
        a.decifrador
            .seek(SeekFrom::Start(de))
            .map_err(|e| FalhaDeAudio::nova(Causa::Rede, format!("o Spotify não entregou o arquivo ({e})")))?;
        let mut saida = vec![0u8; quantos as usize];
        let mut lidos = 0;
        while lidos < saida.len() {
            match a.decifrador.read(&mut saida[lidos..]) {
                Ok(0) => break,
                Ok(n) => lidos += n,
                Err(e) => return Err(FalhaDeAudio::nova(Causa::Rede, format!("o áudio parou ({e})"))),
            }
        }
        saida.truncate(lidos);
        Ok(saida)
    }

    fn formato(&self, id: &str) -> Result<String, FalhaDeAudio> {
        // Abre a faixa de verdade: um erro aqui (conta sem Premium, chave recusada, faixa fora do catálogo) tem de sair
        // antes de o Player receber um endereço que depois responderia 404.
        self.abrir(id).map(|(_, f)| extensao(f).to_string())
    }
}

/// A pasta temporária do librespot (só bytes **cifrados**): limpa a cada abertura, para nada sobreviver a uma queda.
pub fn limpar_temporaria(pasta: &std::path::Path) {
    let _ = std::fs::remove_dir_all(pasta);
    let _ = std::fs::create_dir_all(pasta);
}

/// Uma pasta temporária só deste provedor, vazia.
pub fn pasta_temporaria(base: &std::path::Path) -> PathBuf {
    let pasta = base.join("librespot-tmp");
    limpar_temporaria(&pasta);
    pasta
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use std::collections::HashMap;

    fn com(files: &[(AudioFileFormat, u8)]) -> AudioFiles {
        AudioFiles(files.iter().map(|(f, i)| (*f, FileId([*i; 20]))).collect::<HashMap<_, _>>())
    }

    #[test]
    fn escolhe_a_melhor_qualidade_que_a_conta_tem() {
        let premium = com(&[
            (AudioFileFormat::OGG_VORBIS_96, 1),
            (AudioFileFormat::OGG_VORBIS_160, 2),
            (AudioFileFormat::OGG_VORBIS_320, 3),
        ]);
        assert_eq!(escolher(&premium).map(|(f, _)| f), Some(AudioFileFormat::OGG_VORBIS_320));
        // Só 160 e 96: fica o 160.
        let dois = com(&[(AudioFileFormat::OGG_VORBIS_96, 1), (AudioFileFormat::OGG_VORBIS_160, 2)]);
        assert_eq!(escolher(&dois).map(|(f, _)| f), Some(AudioFileFormat::OGG_VORBIS_160));
        // Uma conta antiga, só com MP3.
        assert_eq!(escolher(&com(&[(AudioFileFormat::MP3_160, 5)])).map(|(f, _)| f), Some(AudioFileFormat::MP3_160));
        // Nenhum formato conhecido: não dá para tocar.
        assert_eq!(escolher(&com(&[])), None);
    }

    #[test]
    fn a_chave_recusada_derruba_o_jeito_e_a_rede_nao() {
        // O texto vem do librespot (o mesmo que o Spotifast lê).
        assert_eq!(causa_do_erro(ErrorKind::Unavailable, "audio key error"), Causa::ChaveDeAudio);
        assert_eq!(causa_do_erro(ErrorKind::Unavailable, "error audio key 00 01"), Causa::ChaveDeAudio);
        assert_eq!(causa_do_erro(ErrorKind::DeadlineExceeded, "deadline exceeded"), Causa::Rede);
        assert_eq!(causa_do_erro(ErrorKind::ResourceExhausted, "too many requests"), Causa::Limite);
    }

    #[test]
    fn a_conta_que_nao_serve_apaga_a_sessao() {
        assert_eq!(causa_do_erro(ErrorKind::PermissionDenied, "Premium account required"), Causa::SemPremium);
        assert_eq!(causa_do_erro(ErrorKind::Unauthenticated, "Bad credentials"), Causa::SessaoRecusada);
    }

    #[test]
    fn a_faixa_fora_do_catalogo_e_da_faixa_so() {
        assert_eq!(causa_do_erro(ErrorKind::Unavailable, "track unavailable"), Causa::Indisponivel);
        assert_eq!(causa_do_erro(ErrorKind::NotFound, "not found"), Causa::Quebrado);
    }

    #[test]
    fn os_nomes_dos_formatos_para_a_pessoa() {
        assert_eq!(nome_do_formato(AudioFileFormat::OGG_VORBIS_320), "Ogg 320");
        assert_eq!(extensao(AudioFileFormat::MP3_160), "mp3");
        assert_eq!(extensao(AudioFileFormat::OGG_VORBIS_96), "ogg");
    }

    struct SemSessao;

    impl FonteDaSessao for SemSessao {
        fn agora(&self) -> Option<Session> {
            None
        }
    }

    #[test]
    fn sem_conta_o_audio_diz_para_entrar() {
        let rt = Arc::new(tokio::runtime::Builder::new_current_thread().enable_all().build().expect("rt"));
        let p = PelaSessao::novo(Arc::new(SemSessao), rt);
        let e = p.tamanho("abc").expect_err("sem conta");
        assert_eq!((e.causa, e.mensagem.as_str()), (Causa::SemConta, "Entre na sua conta do Spotify."));
        assert_eq!(p.formato("abc").expect_err("sem conta").causa, Causa::SemConta);
    }

    #[test]
    fn a_pasta_temporaria_nasce_vazia() {
        let base = std::env::temp_dir().join(format!("provedor-spotify-tmp-{}", std::process::id()));
        let pasta = base.join("librespot-tmp");
        std::fs::create_dir_all(pasta.join("velho")).expect("velho");
        std::fs::write(pasta.join("sobra.bin"), "x").expect("sobra");
        let limpa = pasta_temporaria(&base);
        assert!(limpa.is_dir());
        assert_eq!(std::fs::read_dir(&limpa).expect("lê").count(), 0, "nada sobrevive à abertura");
        let _ = std::fs::remove_dir_all(&base);
    }
}
