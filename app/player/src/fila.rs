//! A fila de músicas do computador: o que toca agora, o que vem depois e em que ordem.
//!
//! Só lógica, sem tocador nem interface. A ordem aleatória guarda a ordem original: desligar o aleatório volta à
//! lista como estava, a partir da música de agora.

use biblioteca::Musica;
use nucleo::Repetir;

#[derive(Default)]
pub struct Fila {
    /// Na ordem em que vão tocar (já embaralhada, com o aleatório ligado).
    itens: Vec<Musica>,
    /// A ordem antes de embaralhar.
    original: Vec<Musica>,
    atual: Option<usize>,
    pub aleatorio: bool,
    pub repetir: Repetir,
    semente: u64,
}

impl Fila {
    pub fn nova(aleatorio: bool, repetir: Repetir) -> Self {
        let semente = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0x9E37_79B9, |d| d.as_nanos() as u64)
            | 1;
        Self { aleatorio, repetir, semente, ..Default::default() }
    }

    pub fn itens(&self) -> &[Musica] {
        &self.itens
    }

    pub fn atual(&self) -> Option<usize> {
        self.atual
    }

    pub fn musica_atual(&self) -> Option<&Musica> {
        self.atual.and_then(|i| self.itens.get(i))
    }

    pub fn vazia(&self) -> bool {
        self.itens.is_empty()
    }

    /// Um número pseudoaleatório (xorshift): basta para embaralhar, sem depender de outra biblioteca.
    fn sortear(&mut self, ate: usize) -> usize {
        let mut x = self.semente;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.semente = x;
        (x % ate.max(1) as u64) as usize
    }

    /// Embaralha tudo o que vem depois da música de agora (ela fica onde está).
    fn embaralhar_o_resto(&mut self) {
        let comeco = self.atual.map_or(0, |i| i + 1);
        let n = self.itens.len();
        for i in (comeco + 1..n).rev() {
            let j = comeco + self.sortear(i - comeco + 1);
            self.itens.swap(i, j);
        }
    }

    /// Troca a fila por `musicas` e começa da `indice`. Com o aleatório, essa toca primeiro e o resto embaralha.
    pub fn tocar(&mut self, musicas: Vec<Musica>, indice: usize) -> Option<&Musica> {
        let indice = indice.min(musicas.len().checked_sub(1)?);
        self.original = musicas.clone();
        self.itens = musicas;
        if self.aleatorio {
            self.itens.swap(0, indice);
            self.atual = Some(0);
            self.embaralhar_o_resto();
        } else {
            self.atual = Some(indice);
        }
        self.musica_atual()
    }

    /// Esvazia a fila (a música parou).
    pub fn limpar(&mut self) {
        self.itens.clear();
        self.original.clear();
        self.atual = None;
    }

    /// Qual entra depois da de agora. `sozinha` = a de agora acabou por si (aí o "repetir a música" vale);
    /// apertar "próxima" passa adiante mesmo repetindo a música.
    pub fn indice_seguinte(&self, sozinha: bool) -> Option<usize> {
        let i = self.atual?;
        if sozinha && self.repetir == Repetir::Faixa {
            return Some(i);
        }
        if i + 1 < self.itens.len() {
            Some(i + 1)
        } else if self.repetir != Repetir::Nada && !self.itens.is_empty() {
            Some(0)
        } else {
            None
        }
    }

    /// A anterior (com repetir a fila, da primeira volta para a última).
    pub fn indice_anterior(&self) -> Option<usize> {
        let i = self.atual?;
        if i > 0 {
            Some(i - 1)
        } else if self.repetir == Repetir::Fila {
            self.itens.len().checked_sub(1)
        } else {
            None
        }
    }

    /// Passa a tocar a `indice` da fila. Falso se não existe.
    pub fn ir_para(&mut self, indice: usize) -> bool {
        if indice < self.itens.len() {
            self.atual = Some(indice);
            true
        } else {
            false
        }
    }

    /// Liga ou desliga o aleatório. Desligar volta à ordem original, continuando da música de agora.
    pub fn alternar_aleatorio(&mut self) {
        self.aleatorio = !self.aleatorio;
        if self.aleatorio {
            self.embaralhar_o_resto();
        } else if let Some(agora) = self.musica_atual().cloned() {
            self.itens = self.original.clone();
            self.atual = self.itens.iter().position(|m| m.caminho == agora.caminho);
        }
    }

    /// Põe a música logo depois da de agora.
    pub fn tocar_a_seguir(&mut self, musica: Musica) {
        let lugar = self.atual.map_or(self.itens.len(), |i| i + 1);
        let lugar_original = self
            .musica_atual()
            .and_then(|a| self.original.iter().position(|m| m.caminho == a.caminho))
            .map_or(self.original.len(), |i| i + 1);
        self.itens.insert(lugar, musica.clone());
        self.original.insert(lugar_original.min(self.original.len()), musica);
    }

    /// Põe a música logo depois da de agora e diz o lugar dela, para ela tocar já (o resto da fila continua depois).
    pub fn tocar_agora(&mut self, musica: Musica) -> usize {
        let lugar = self.atual.map_or(self.itens.len(), |i| i + 1);
        self.tocar_a_seguir(musica);
        lugar
    }

    /// Põe as músicas no fim da fila.
    pub fn adicionar(&mut self, musicas: impl IntoIterator<Item = Musica>) {
        for m in musicas {
            self.original.push(m.clone());
            self.itens.push(m);
        }
    }

    /// Tira a música `indice` e a devolve (a de agora não sai: para ela, use parar ou próxima).
    pub fn tirar(&mut self, indice: usize) -> Option<Musica> {
        if Some(indice) == self.atual || indice >= self.itens.len() {
            return None;
        }
        let saiu = self.itens.remove(indice);
        if let Some(j) = self.original.iter().position(|m| m.caminho == saiu.caminho) {
            self.original.remove(j);
        }
        if let Some(a) = self.atual.as_mut()
            && indice < *a
        {
            *a -= 1;
        }
        Some(saiu)
    }

    /// Põe a música de volta na posição `indice` (desfazer um "tirar"). Fora do tamanho, vai para o fim.
    pub fn inserir(&mut self, indice: usize, musica: Musica) {
        let indice = indice.min(self.itens.len());
        if let Some(a) = self.atual.as_mut()
            && indice <= *a
        {
            *a += 1;
        }
        let no_original = indice.min(self.original.len());
        self.original.insert(no_original, musica.clone());
        self.itens.insert(indice, musica);
    }

    /// Move a música `de` para a posição `para` (a de agora também pode mudar de lugar).
    pub fn mover(&mut self, de: usize, para: usize) -> bool {
        let n = self.itens.len();
        if de >= n || para >= n || de == para {
            return false;
        }
        let m = self.itens.remove(de);
        self.itens.insert(para, m);
        if let Some(a) = self.atual {
            self.atual = Some(if a == de {
                para
            } else if de < a && para >= a {
                a - 1
            } else if de > a && para <= a {
                a + 1
            } else {
                a
            });
        }
        // Mover à mão vira a ordem nova também para quando o aleatório desligar.
        if !self.aleatorio {
            self.original = self.itens.clone();
        }
        true
    }

    /// Tira tudo o que vem depois da de agora e devolve o que saiu (para desfazer).
    pub fn limpar_a_seguir(&mut self) -> Vec<Musica> {
        let Some(i) = self.atual else { return Vec::new() };
        let saiu = self.itens.split_off(i + 1);
        self.original = self.itens.clone();
        saiu
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod testes {
    use super::*;
    use std::path::PathBuf;

    fn m(nome: &str) -> Musica {
        Musica { caminho: PathBuf::from(nome), titulo: nome.into(), ..Default::default() }
    }

    fn lista(n: usize) -> Vec<Musica> {
        (0..n).map(|i| m(&format!("{i}"))).collect()
    }

    fn nomes(f: &Fila) -> Vec<String> {
        f.itens().iter().map(|m| m.titulo.clone()).collect()
    }

    #[test]
    fn segue_em_ordem_e_para_no_fim() {
        let mut f = Fila::nova(false, Repetir::Nada);
        assert_eq!(f.tocar(lista(3), 1).map(|m| m.titulo.as_str()), Some("1"));
        assert_eq!(f.indice_seguinte(true), Some(2));
        f.ir_para(2);
        assert_eq!(f.indice_seguinte(true), None);
        assert_eq!(f.indice_anterior(), Some(1));
        assert!(f.tocar(Vec::new(), 0).is_none());
    }

    #[test]
    fn tocar_agora_mantem_o_resto_da_fila() {
        let mut f = Fila::nova(false, Repetir::Nada);
        f.tocar(lista(3), 1);
        let i = f.tocar_agora(m("nova"));
        f.ir_para(i);
        assert_eq!(f.musica_atual().map(|m| m.titulo.as_str()), Some("nova"));
        assert_eq!(nomes(&f), ["0", "1", "nova", "2"], "a fila de antes continua depois dela");
    }

    #[test]
    fn repetir_a_fila_e_a_musica() {
        let mut f = Fila::nova(false, Repetir::Fila);
        f.tocar(lista(3), 2);
        assert_eq!(f.indice_seguinte(true), Some(0));
        f.ir_para(0);
        assert_eq!(f.indice_anterior(), Some(2));
        f.repetir = Repetir::Faixa;
        assert_eq!(f.indice_seguinte(true), Some(0), "sozinha, repete");
        assert_eq!(f.indice_seguinte(false), Some(1), "apertando próxima, passa");
    }

    #[test]
    fn aleatorio_comeca_pela_escolhida_e_volta_a_ordem() {
        let mut f = Fila::nova(true, Repetir::Nada);
        f.tocar(lista(20), 7);
        assert_eq!(f.musica_atual().map(|m| m.titulo.as_str()), Some("7"));
        let mut todos = nomes(&f);
        todos.sort_by_key(|n| n.parse::<u32>().unwrap_or(0));
        assert_eq!(
            todos,
            nomes(&{
                let mut g = Fila::nova(false, Repetir::Nada);
                g.tocar(lista(20), 0);
                g
            }),
            "ninguém some nem repete"
        );
        assert_ne!(nomes(&f)[1..], (0..20).filter(|i| *i != 7).map(|i| i.to_string()).collect::<Vec<_>>()[..]);
        f.ir_para(3);
        let agora = f.musica_atual().map(|m| m.titulo.clone());
        f.alternar_aleatorio();
        assert_eq!(nomes(&f), (0..20).map(|i| i.to_string()).collect::<Vec<_>>());
        assert_eq!(f.musica_atual().map(|m| m.titulo.clone()), agora, "continua na mesma música");
    }

    #[test]
    fn a_seguir_fim_tirar_e_mover() {
        let mut f = Fila::nova(false, Repetir::Nada);
        f.tocar(lista(4), 1);
        f.tocar_a_seguir(m("x"));
        assert_eq!(nomes(&f), ["0", "1", "x", "2", "3"]);
        f.adicionar([m("y")]);
        assert_eq!(nomes(&f).last().map(String::as_str), Some("y"));
        assert!(f.tirar(1).is_none(), "a de agora não sai");
        let saiu = f.tirar(0).expect("tira");
        assert_eq!(saiu.titulo, "0");
        assert_eq!(f.atual(), Some(0), "a de agora continua apontada");
        assert_eq!(nomes(&f), ["1", "x", "2", "3", "y"]);
        assert!(f.mover(4, 1));
        assert_eq!(nomes(&f), ["1", "y", "x", "2", "3"]);
        assert!(f.mover(0, 2), "a de agora pode mudar de lugar");
        assert_eq!(f.atual(), Some(2));
        assert_eq!(f.musica_atual().map(|m| m.titulo.as_str()), Some("1"));
        let sairam = f.limpar_a_seguir();
        assert_eq!(nomes(&f), ["y", "x", "1"]);
        assert_eq!(sairam.len(), 2);
        assert_eq!(f.indice_seguinte(true), None);
        // Desfazer: voltam para o fim, na mesma ordem.
        f.adicionar(sairam);
        assert_eq!(nomes(&f), ["y", "x", "1", "2", "3"]);
    }

    #[test]
    fn desfazer_tirar_volta_ao_mesmo_lugar() {
        let mut f = Fila::nova(false, Repetir::Nada);
        f.tocar(lista(4), 2);
        let saiu = f.tirar(0).expect("tira");
        assert_eq!(f.atual(), Some(1));
        f.inserir(0, saiu);
        assert_eq!(nomes(&f), ["0", "1", "2", "3"]);
        assert_eq!(f.atual(), Some(2), "a de agora continua a mesma");
        let saiu = f.tirar(3).expect("tira");
        f.inserir(99, saiu);
        assert_eq!(nomes(&f), ["0", "1", "2", "3"]);
    }
}
