# F2, parte 3: espaços e navegação — plano

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** no layout Palco, a folha sobre o visual ganha espaços (Tudo, Neste computador e um por serviço, com a logo e a
cor dele), voltar e avançar, e a cor do espaço tinge a folha numa transição; as páginas de hoje continuam funcionando
dentro dela.

**Architecture:** lógica pura e testada em dois módulos novos do `player`: `navegacao.rs` (`Espaco`, `Rota`,
`Navegacao` com uma pilha por espaço) e `espacos.rs` (a paleta derivada da cor do serviço e a cor tirada do nome). O
`Player` troca o campo `pagina: Pagina` por `nav: Navegacao` e um método `pagina()` que deriva a `Pagina` de hoje da
rota aberta (os 43 lugares que leem `self.pagina` passam a chamar o método; os que escrevem passam a navegar). A folha
(`ui/heroi.rs::ver_folha`) ganha um cabeçalho (`ui/espacos.rs`): voltar, avançar e a fileira de espaços.

**Tech Stack:** Rust 2024, GPUI 0.2.2 (`img` para a logo SVG gravada pela parte 1), `anim` do app.

**Spec:** `docs/superpowers/specs/2026-11-23-f2-espacos-e-paginas-design.md` (5.1, 5.2, 5.5 "Palco", 5.7 "Palco sem
página aberta"; seção 6, item 3).

## Global Constraints

- Sem página aberta no Palco = só o visual, como hoje; `H`, `Esc` e `AbrirPagina(Inicio)` fecham a folha.
- A cor do serviço é acento, nunca fundo cheio de texto: folha tingida a 6–8%, filete a 18%, degradê do topo a 22%
  indo ao transparente; acento com contraste ≥ 3:1 sobre a folha (clareado no escuro, escurecido no tema Papel).
- A troca de espaço anima a paleta em ~250 ms com `anim`; com movimento reduzido ou modo seguro, vai direto.
- Toda ação é um `Comando` (nome, atalho, barra de comandos). Nada espera thread.
- `cargo fmt`, `clippy` sem aviso, testes passando; cópia de teste por `PrintWindow`. Sem commit.

## Review Focus

- Voltar com a pilha vazia, ou avançar sem nada à frente: nada muda e os botões ficam apagados (teste na Task 1).
- Ir para a mesma rota que já está aberta: não empilha uma cópia (Task 1).
- O espaço lembrado nos ajustes é de um provedor que foi desligado ou removido: abre em Tudo (Task 1 e Task 3).
- Trocar de espaço e voltar ao anterior: a página onde a pessoa estava nele volta (Task 1).
- Serviço sem cor (provedor antigo): a cor tirada do nome é estável entre aberturas e passa nos contrastes (Task 2).

---

### Task 1: `navegacao.rs`

**Files:** Create `app/player/src/navegacao.rs`; Modify `app/player/src/main.rs` (`mod navegacao;`).

**Interfaces (Produces):**
- `#[derive(Clone, Debug, PartialEq, Eq, Hash)] pub enum Espaco { Tudo, Computador, Servico(String) }` com
  `pub fn chave(&self) -> String` (`"tudo"`, `"computador"`, `"servico:<x>"`) e `pub fn da_chave(s: &str) -> Espaco`
  (desconhecido = `Tudo`).
- `#[derive(Clone, Debug, PartialEq)] pub enum Rota { ParaVoce, Busca, Biblioteca, Album(Item), Playlist(Item), Artista(Item), Faixa(Item), Loja, Configuracoes }`
  (`Item` = `provedores::Item`).
- `pub struct Navegacao` com:
  - `pub fn new(espaco: Espaco) -> Self` (nenhuma página aberta);
  - `pub fn espaco(&self) -> &Espaco`, `pub fn atual(&self) -> Option<&Rota>`;
  - `pub fn ir(&mut self, rota: Rota)`: a de agora vai para trás, o "avançar" se perde; a mesma rota não empilha;
  - `pub fn fechar(&mut self)`: fecha a folha (entra no histórico como `None`, para voltar reabrir);
  - `pub fn voltar(&mut self) -> bool`, `pub fn avancar(&mut self) -> bool`, `pub fn pode_voltar(&self) -> bool`,
    `pub fn pode_avancar(&self) -> bool`;
  - `pub fn trocar_espaco(&mut self, espaco: Espaco)`: cada espaço guarda a pilha dele; um espaço novo abre em
    `ParaVoce` (serviço e Tudo) ou `Biblioteca` (Computador);
  - `pub fn esquecer_servico(&mut self, servico: &str)`: some a pilha dele; se era o aberto, vai para Tudo.
  - No máximo 50 rotas para trás por espaço (as mais velhas saem).

- [ ] **Step 1: testes que falham** (no próprio arquivo):

```rust
#[test]
fn ir_voltar_e_avancar() {
    let mut n = Navegacao::new(Espaco::Tudo);
    assert_eq!(n.atual(), None);
    assert!(!n.voltar() && !n.pode_voltar() && !n.pode_avancar());
    n.ir(Rota::Biblioteca);
    n.ir(Rota::Biblioteca);
    n.ir(Rota::Loja);
    assert!(n.voltar());
    assert_eq!(n.atual(), Some(&Rota::Biblioteca), "a repetida não empilhou");
    assert!(n.voltar());
    assert_eq!(n.atual(), None, "volta ao visual");
    assert!(!n.voltar());
    assert!(n.avancar() && n.avancar());
    assert_eq!(n.atual(), Some(&Rota::Loja));
    assert!(!n.avancar());
    n.voltar();
    n.ir(Rota::Configuracoes);
    assert!(!n.pode_avancar(), "ir apaga o avançar");
}

#[test]
fn cada_espaco_guarda_a_sua_pilha() {
    let mut n = Navegacao::new(Espaco::Tudo);
    n.ir(Rota::Loja);
    n.trocar_espaco(Espaco::Servico("youtube-music".into()));
    assert_eq!(n.atual(), Some(&Rota::ParaVoce));
    n.ir(Rota::Busca);
    n.trocar_espaco(Espaco::Computador);
    assert_eq!(n.atual(), Some(&Rota::Biblioteca));
    n.trocar_espaco(Espaco::Servico("youtube-music".into()));
    assert_eq!(n.atual(), Some(&Rota::Busca), "volta onde estava");
    n.esquecer_servico("youtube-music");
    assert_eq!(n.espaco(), &Espaco::Tudo);
    assert_eq!(n.atual(), Some(&Rota::Loja));
}

#[test]
fn fechar_e_chaves() {
    let mut n = Navegacao::new(Espaco::Tudo);
    n.ir(Rota::Biblioteca);
    n.fechar();
    assert_eq!(n.atual(), None);
    assert!(n.voltar());
    assert_eq!(n.atual(), Some(&Rota::Biblioteca), "voltar reabre a folha");
    for e in [Espaco::Tudo, Espaco::Computador, Espaco::Servico("x".into())] {
        assert_eq!(Espaco::da_chave(&e.chave()), e);
    }
    assert_eq!(Espaco::da_chave("lixo"), Espaco::Tudo);
    for _ in 0..80 {
        n.ir(Rota::Loja);
        n.ir(Rota::Configuracoes);
    }
    let mut voltas = 0;
    while n.voltar() {
        voltas += 1;
    }
    assert!(voltas <= 50, "a pilha tem limite");
}
```

- [ ] **Step 2:** `cargo test -p player navegacao` — falha.
- [ ] **Step 3:** implementar.
- [ ] **Step 4:** `cargo test -p player navegacao` — passa.

### Task 2: `espacos.rs` — a paleta do espaço

**Files:** Create `app/player/src/espacos.rs`; Modify `app/player/src/main.rs`; `app/player/src/tema.rs` (tornar
`luminancia` e `contraste` `pub(crate)` sem `cfg(test)`).

**Interfaces (Produces):**
- `#[derive(Clone, Copy, Debug, PartialEq)] pub struct PaletaDoEspaco { pub acento: Hsla, pub texto_no_acento: Hsla, pub folha: Hsla, pub filete: Hsla, pub topo: Hsla }`
  (`topo`: a cor do começo do degradê, já com o alfa).
- `pub fn paleta(cor: [f32; 3], base: &tema::Cores) -> PaletaDoEspaco`: folha = `misturar(base.folha, cor, 0.07)`;
  filete = `misturar(base.linha, cor, 0.18)`; topo = cor com alfa 0,22; acento = a cor clareada (tema escuro) ou
  escurecida (Papel) até contraste ≥ 3:1 com a folha; `texto_no_acento` = preto ou branco, o de maior contraste.
- `pub fn neutra(base: &tema::Cores) -> PaletaDoEspaco` (Tudo): as cores do tema, acento = `base.inv`.
- `pub fn cor_do_nome(servico: &str) -> [f32; 3]`: matiz pelo hash FNV-1a do nome, saturação 0,65, luz 0,55.
- `pub const COR_DO_COMPUTADOR: u32 = 0xF2B84B;`
- `pub fn rgb(cor: u32) -> [f32; 3]`.
- `pub fn entre(a: &PaletaDoEspaco, b: &PaletaDoEspaco, t: f32) -> PaletaDoEspaco` (mistura campo a campo).

- [ ] **Step 1: testes que falham:**

```rust
#[test]
fn acento_tem_contraste_em_todos_os_temas() {
    for tema in nucleo::Tema::TODOS {
        let base = crate::tema::cores(tema);
        for cor in [rgb(0xFF0000), rgb(0x1ED760), rgb(0x0A0A0A), rgb(0xFFFFFF), rgb(COR_DO_COMPUTADOR), cor_do_nome("soundcloud")] {
            let p = paleta(cor, &base);
            let r = crate::tema::contraste(crate::tema::tripla(p.acento), crate::tema::tripla(p.folha));
            assert!(r >= 3.0, "{tema:?} {cor:?}: acento com contraste {r:.2}");
            let t = crate::tema::contraste(crate::tema::tripla(p.texto_no_acento), crate::tema::tripla(p.acento));
            assert!(t >= 4.5, "{tema:?} {cor:?}: texto no acento com {t:.2}");
            let texto = crate::tema::contraste(crate::tema::tripla(base.texto_2), crate::tema::tripla(p.folha));
            assert!(texto >= 4.5, "{tema:?} {cor:?}: texto secundário sobre a folha tingida com {texto:.2}");
        }
    }
}

#[test]
fn cor_do_nome_e_estavel_e_muda_com_o_nome() {
    assert_eq!(cor_do_nome("soundcloud"), cor_do_nome("soundcloud"));
    assert_ne!(cor_do_nome("soundcloud"), cor_do_nome("deezer"));
}

#[test]
fn transicao_vai_de_uma_a_outra() {
    let base = crate::tema::cores(nucleo::Tema::Noite);
    let (a, b) = (neutra(&base), paleta(rgb(0xFF0000), &base));
    assert_eq!(entre(&a, &b, 0.0), a);
    assert_eq!(entre(&a, &b, 1.0), b);
}
```

- [ ] **Step 2:** `cargo test -p player espacos` — falha. **Step 3:** implementar. **Step 4:** passa.

### Task 3: o Player navega

**Files:** Modify `app/player/src/app.rs`, `app/player/src/ui/*.rs` (os que leem `self.pagina`),
`app/crates/nucleo/src/ajustes.rs` (`espaco: String`, padrão `"tudo"`), `app/crates/nucleo/src/comando.rs`.

**Interfaces:**
- Consumes: Task 1 e Task 2.
- Produces:
  - `Player.nav: Navegacao` (no lugar de `pagina`), `pub(crate) fn pagina(&self) -> Pagina`: `None` → `Inicio`,
    `Loja` → `Loja`, `Configuracoes` → `Configuracoes`, o resto → `Biblioteca`.
  - `pub(crate) fn abrir_rota(&mut self, rota: Rota)`, `pub(crate) fn trocar_espaco(&mut self, e: Espaco)` (grava
    `ajustes.espaco`, começa a transição de cor), `pub(crate) fn espacos(&self) -> Vec<(Espaco, String)>` (Tudo,
    Computador e os provedores ligados e prontos, com o nome), `pub(crate) fn paleta_do_espaco(&self) -> PaletaDoEspaco`
    (com a transição aplicada).
  - Comandos novos: `Voltar` (`Alt ←` e o botão "voltar" do mouse), `Avancar` (`Alt →` e "avançar" do mouse),
    `ProximoEspaco` (`Ctrl Tab`). `Comando::do_atalho_alt(tecla) -> Option<Comando>` para as teclas com Alt; o teste
    de atalhos inclui as com Alt.
  - Ao abrir: `Espaco::da_chave(&ajustes.espaco)`; se for um serviço que não está na lista quando os provedores
    carregam, `nav.esquecer_servico`.

- [ ] **Step 1:** testes de `nucleo` (comandos com Alt e o ajuste `espaco` válido) que falham; rodar e ver falhar.
- [ ] **Step 2:** implementar; trocar todos os `self.pagina` (leitura → `self.pagina()`, escrita → `abrir_rota` ou
  `nav.fechar()`); `AbaBiblioteca` continua dentro da rota `Biblioteca`.
- [ ] **Step 3:** `cargo test --workspace --exclude visual --no-fail-fast` e `cargo clippy --workspace --all-targets`.

### Task 4: a folha com espaços (Palco)

**Files:** Create `app/player/src/ui/espacos.rs`; Modify `app/player/src/ui/heroi.rs` (`ver_folha`),
`app/player/src/ui/mod.rs`.

- Cabeçalho da folha (altura 52 px): ← e → (`botao_icone`, apagados a 50% quando não dá), a fileira de espaços
  (círculos de 28 px: Tudo com o ícone `grid`, Computador com `lib`, serviço com a logo (`img` do arquivo da parte 1)
  ou a inicial na cor tirada do nome; o ativo vira pílula com o nome, e o indicador desliza numa mola até ele), com
  dica no hover.
- A folha usa `paleta_do_espaco()`: fundo `folha`, borda `filete`, degradê `topo` → transparente nos primeiros 160 px.
- Rotas sem página própria ainda (`ParaVoce`, `Busca`, `Album`...) mostram, por enquanto, as prateleiras do serviço
  em texto (título da prateleira e os títulos dos itens) a partir de `remoto.pagina(Inicio)`, ou a frase de
  carregando/erro com "Tentar de novo"; a parte 4 troca pelo desenho final.

- [ ] Compilar, clippy, testes; cópia de teste com o YouTube Music: a fileira com a logo, a cor vermelha na folha, a
  troca para Tudo (cor neutra), voltar e avançar, e `Esc`/`H` fechando.

### Task 5: revisão e anotações

- [ ] Revisor novo no fim; corrigir os importantes com teste. `AGENTS.md` (seção 7, contagem de testes) e
  `app/README.md` (espaços, voltar e avançar, atalhos).
