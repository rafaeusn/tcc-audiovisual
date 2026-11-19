# Como contribuir

Obrigado pelo interesse. O projeto é escrito e documentado em **português do Brasil**: código, comentários,
mensagens de commit e textos da interface.

## Antes de começar

- Leia o [`AGENTS.md`](AGENTS.md): arquitetura, convenções, armadilhas já resolvidas e o estado atual.
- O plano em vigor está em [`docs/plano-v2.md`](docs/plano-v2.md). Para uma ideia grande, abra uma issue antes.

## Regras

1. **O núcleo não usa nenhum serviço de streaming sozinho.** Spotify, YouTube Music, SoundCloud e afins entram só
   como provedores da Loja, desligados por padrão, instalados e conectados pelo usuário.
2. **Nenhum segredo no código nem no git.** Chaves de API vêm de variável de ambiente.
3. **Sem arquivos de áudio no repositório** (o `.gitignore` já bloqueia MP3, FLAC e WAV).
4. **Código de outro projeto:** só se a licença for compatível com a GPL-3. Mantenha no topo do arquivo um
   comentário com a origem e a licença, e registre em [`THIRD-PARTY-NOTICES.md`](THIRD-PARTY-NOTICES.md).
5. Toda ação do usuário é um `Comando` (`app/crates/nucleo/src/comando.rs`), com nome, atalho e entrada na barra
   de comandos.

## Antes de mandar

Dentro de `app/`:

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets   # sem nenhum aviso
cargo test --workspace --exclude visual
```

Escreva testes para a lógica nova. Se mexeu na interface, diga no pull request o que conferiu no app e o que não
conferiu.

## Licença

Ao contribuir, você concorda que a sua contribuição seja distribuída sob a [GPL-3.0-or-later](LICENSE).
