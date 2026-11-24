# Avisos de terceiros

Este projeto é distribuído sob a **GPL-3.0-or-later** (arquivo [`LICENSE`](LICENSE)). Abaixo, o que ele usa de terceiros e sob que licença. Todas são compatíveis com a GPL-3.

Regra: código **copiado ou adaptado** de outro projeto entra na seção 3, com origem, licença e os arquivos onde está, e o arquivo copiado mantém no topo um comentário com a origem.

## 0. Provedores

- `app/provedores/youtube-music` usa a biblioteca **rustypipe** (GPL-3.0, <https://codeberg.org/ThetaDev/rustypipe>)
  para os metadados do YouTube Music; por isso esse programa é distribuído sob a GPL-3.0 (só a 3).
- A rota do áudio pelo cliente visionOS do YouTube (`visionos.rs`) segue o que fazem o **MixMusic**
  (<https://github.com/fzn0x/mixmusic>, MIT ou GPL-3) e o **limusic** (<https://github.com/SimoHypers/limusic>,
  GPL-3). Nenhum código foi copiado: só os valores do cliente (nome, versão, agente), que são do YouTube.
- `app/provedores/spotify` usa o **librespot** (`librespot-core`, `librespot-oauth`, `librespot-audio` e
  `librespot-metadata`, MIT, <https://github.com/librespot-org/librespot>) para o login, a sessão, os metadados das faixas,
  o arquivo de áudio e a chave que o decifra. O áudio decifrado vive só na memória do provedor e nunca é gravado nem
  redistribuído (regra 2 do `AGENTS.md`). O caminho (o Client ID público do librespot e a porta 8898) é o mesmo do
  **Spotifast** (<https://github.com/crmne/spotifast>, MIT). Os pedidos dos dados seguem o que
  fazem o Spotifast, o **Prism** (<https://github.com/Yigder/Prism>, GPL-3), o **Metrolist** (GPL-3) e o **limusic**
  (<https://github.com/SimoHypers/limusic>, GPL-3; a ideia de reler o código das consultas do site). Nenhum código deles
  foi copiado. Desde 24/11/2026 o áudio toca pelo próprio motor do Player (`audio/`): o librespot entrega o arquivo
  cifrado, a chave o decifra **na memória** e um servidor local em `127.0.0.1` (com token sorteado a cada abertura)
  entrega os bytes ao tocador. Nada de áudio vai para o disco — o temporário do `librespot-audio` guarda só bytes
  **cifrados** e é apagado a cada abertura (regra 2 do `AGENTS.md`).
- O **Soloist** (o player do Spotify) não vem com o Player nem está neste repositório: o programa é baixado do servidor do Spotify
  quando a pessoa o liga, com a chave dela, e vale a licença dele. O jeito de o abrir e de falar com a API local segue a
  documentação oficial e o **Spotifast** (MIT), sem código copiado. Para isso o provedor usa `tungstenite` 0.28 (MIT OR Apache-2.0)
  e `zip` 2.6 (MIT).
- A conta do casador entre serviços (Match) vem do **MixMusic** (MIT), com pisos de título, artista e versão nossos.

## 1. Fontes tipográficas

| Fonte | Licença | Arquivo da licença |
| --- | --- | --- |
| Instrument Sans | SIL Open Font License 1.1 | `app/recursos/fontes/OFL-InstrumentSans.txt` |
| Fragment Mono | SIL Open Font License 1.1 | `app/recursos/fontes/OFL-FragmentMono.txt` |
| Doto | SIL Open Font License 1.1 | `app/recursos/fontes/OFL-Doto.txt` |

Os ícones em `app/recursos/icones` foram desenhados para este projeto.

## 2. Bibliotecas Rust

Geradas de `cargo metadata` em 16/11/2026: 824 pacotes, agrupados pela licença declarada. O texto de cada licença vem junto do código-fonte de cada pacote (crates.io).

### MIT OR Apache-2.0 (425)

aes 0.8.4, ahash 0.8.12, aligned 0.4.3, allocator-api2 0.2.21, android_system_properties 0.1.6, anyhow 1.0.104, arbitrary 1.5.0, arrayvec 0.7.8, as-raw-xcb-connection 1.0.1, as-slice 0.2.1, ash 0.38.0+1.3.281, ash-window 0.13.0, async-broadcast 0.7.2, async-compression 0.4.52, async-recursion 1.2.0, async-trait 0.1.92, base64 0.22.1, bit_field 0.10.4, bitflags 1.3.2, bitflags 2.13.2, bitstream-io 4.10.0, block-buffer 0.10.4, block-buffer 0.12.1, block-padding 0.3.3, bstr 1.13.1, bumpalo 3.20.3, cbc 0.1.2, cc 1.11.1, cfg-if 1.0.5, cgl 0.3.2, chacha20 0.10.2, cipher 0.4.4, cocoa 0.24.1, cocoa 0.25.0, cocoa 0.26.0, cocoa-foundation 0.1.2, cocoa-foundation 0.2.0, compression-codecs 0.4.47, compression-core 0.4.34, const-random 0.1.18, const-random-macro 0.1.16, core-foundation 0.10.0, core-foundation 0.9.4, core-foundation-sys 0.8.7, core-graphics 0.22.3, core-graphics 0.23.2, core-graphics 0.24.0, core-graphics-helmer-fork 0.24.0, core-graphics-types 0.1.3, core-graphics-types 0.2.0, core-graphics2 0.4.1, core-text 21.0.0, core-video 0.4.3, core_detect 1.0.0, coreaudio-rs 0.14.2, cosmic-text 0.14.2, cpufeatures 0.2.17, cpufeatures 0.3.1, crc32fast 1.5.4, crossbeam-deque 0.8.8, crossbeam-epoch 0.9.21, crossbeam-queue 0.3.14, crossbeam-utils 0.8.23, crypto-common 0.1.7, crypto-common 0.2.2, dasp_sample 0.11.0, data-url 0.3.2, digest 0.10.7, digest 0.11.3, dirs 4.0.0, dirs 5.0.1, dirs-sys 0.3.7, dirs-sys 0.4.1, displaydoc 0.2.7, document-features 0.2.12, downcast-rs 1.2.1, dyn-clone 1.0.20, either 1.19.0, enumflags2 0.7.12, enumflags2_derive 0.7.12, erased-serde 0.4.11, errno 0.3.14, etagere 0.2.15, euclid 0.22.14, fallible-iterator 0.3.0, fallible-streaming-iterator 0.1.9, fdeflate 0.3.7, filetime 0.2.29, find-msvc-tools 0.1.15, flate2 1.1.11, float-ord 0.3.2, font-types 0.12.7, foreign-types 0.3.2, foreign-types 0.5.0, foreign-types-macros 0.2.4, foreign-types-shared 0.1.1, foreign-types-shared 0.3.1, form_urlencoded 1.2.2, futf 0.1.5, futures 0.3.34, futures-channel 0.3.34, futures-core 0.3.34, futures-executor 0.3.34, futures-io 0.3.34, futures-macro 0.3.34, futures-sink 0.3.34, futures-task 0.3.34, futures-util 0.3.34, getrandom 0.2.17, getrandom 0.3.4, getrandom 0.4.3, gif 0.14.2, glob 0.3.4, gloo-timers 0.3.0, gpu-alloc 0.6.2, gpu-alloc-ash 0.7.1, gpu-alloc-types 0.3.1, gpu-allocator 0.28.0, half 2.7.1, hashbrown 0.14.5, hashbrown 0.15.5, hashbrown 0.16.1, hashbrown 0.17.1, hashlink 0.10.0, heck 0.4.1, heck 0.5.0, hermit-abi 0.5.3, hex 0.4.3, hkdf 0.12.4, hmac 0.12.1, home 0.5.12, http 1.5.0, httparse 1.10.1, hybrid-array 0.4.15, idna 1.1.0, image 0.25.10, image-webp 0.2.4, inout 0.1.4, inventory 0.3.25, io-surface 0.16.1, ipnet 2.12.2, itertools 0.13.0, itertools 0.14.0, itoa 1.0.19, jni 0.22.4, jni-macros 0.22.4, jni-sys 0.3.1, jni-sys 0.4.1, jni-sys-macros 0.4.1, jobserver 0.1.35, js-sys 0.3.107, khronos-egl 6.0.0, kv-log-macro 1.0.7, lazy_static 1.5.1, leaky-cow 0.1.1, libc 0.2.191, litrs 1.0.1, lock_api 0.4.14, lofty 0.22.4, lofty_attr 0.11.1, log 0.4.35, lyon 1.0.19, lyon_algorithms 1.0.22, lyon_geom 1.0.20, lyon_path 1.0.19, lyon_tessellation 1.0.23, mac 0.1.1, md-5 0.10.6, memmap2 0.9.12, metal 0.29.0, mime 0.3.18, minimal-lexical 0.2.1, multiversion 0.8.1, multiversion-macros 0.8.1, naga 25.0.1, naga 30.0.1, naga-types 30.0.1, native-tls 0.2.18, ndk 0.9.0, ndk-context 0.1.1, ndk-sys 0.6.0+11769913, num 0.4.3, num-bigint 0.4.8, num-bigint-dig 0.8.6, num-complex 0.4.6, num-derive 0.4.2, num-integer 0.1.47, num-iter 0.1.46, num-rational 0.4.2, num-traits 0.2.19, num_cpus 1.17.0, ogg_pager 0.7.1, once_cell 1.21.4, openssl-macros 0.1.1, openssl-probe 0.2.2, ordered-stream 0.2.0, parking_lot 0.12.5, parking_lot_core 0.9.12, paste 1.0.15, pastey 0.1.1, pathfinder_geometry 0.5.1, pathfinder_simd 0.5.6, pbkdf2 0.12.2, percent-encoding 2.3.2, pin-utils 0.1.1, piper 0.2.5, pkg-config 0.3.34, png 0.17.16, png 0.18.1, ppv-lite86 0.2.21, presser 0.3.1, prettyplease 0.2.37, primal-check 0.3.4, proc-macro-crate 3.5.0, proc-macro-error-attr2 2.0.0, proc-macro-error2 2.0.1, proc-macro2 1.0.107, profiling 1.0.18, profiling-procmacros 1.0.18, psm 0.1.32, qoi 0.4.1, quick-error 2.0.1, quinn 0.11.12, quinn-proto 0.11.19, quinn-udp 0.5.16, quote 1.0.47, rand 0.10.3, rand 0.8.8, rand 0.9.5, rand_chacha 0.3.1, rand_chacha 0.9.0, rand_core 0.10.1, rand_core 0.6.4, rand_core 0.9.5, rand_pcg 0.10.2, range-alloc 0.1.5, rangemap 1.8.0, raw-window-metal 0.4.0, raw-window-metal 1.1.0, rayon 1.12.0, rayon-core 1.13.0, read-fonts 0.41.0, ref-cast 1.0.27, ref-cast-impl 1.0.27, regex 1.13.1, regex-automata 0.4.18, regex-syntax 0.8.11, renderdoc-sys 1.1.0, roxmltree 0.20.0, rtrb 0.3.6, rustc_version 0.4.1, rustfft 6.4.1, rustls-pki-types 1.16.0, rustversion 1.0.23, scoped-tls 1.0.1, scopeguard 1.2.0, screencapturekit 0.2.8, screencapturekit-sys 0.2.8, security-framework 3.7.0, security-framework-sys 2.17.0, semver 1.0.28, serde 1.0.229, serde_core 1.0.229, serde_derive 1.0.229, serde_derive_internals 0.30.0, serde_json 1.0.154, serde_json_lenient 0.2.4, serde_repr 0.1.21, serde_spanned 0.6.9, serde_spanned 1.1.3, serde_urlencoded 0.7.1, sha2 0.10.9, sha2 0.11.1, shlex 1.3.0, shlex 2.0.1, signal-hook-registry 1.4.8, simdutf8 0.1.5, siphasher 1.0.4, skrifa 0.44.0, smallvec 1.16.5, smol_str 0.2.2, socket2 0.6.5, stable_deref_trait 1.2.1, stacker 0.1.25, static_assertions 1.1.0, strength_reduce 0.2.4, svg_fmt 0.4.5, syn 1.0.109, syn 2.0.119, syn 3.0.9, sys-locale 0.3.2, system-configuration 0.6.1, system-configuration-sys 0.6.0, target-features 0.1.6, tempfile 3.27.0, tendril 0.4.3, thiserror 1.0.69, thiserror 2.0.21, thiserror-impl 1.0.69, thiserror-impl 2.0.21, tokio-rustls 0.26.6, toml 0.8.23, toml 1.2.1+spec-1.1.0, toml_datetime 0.6.11, toml_datetime 1.1.3+spec-1.1.0, toml_edit 0.22.27, toml_edit 0.25.19+spec-1.1.0, toml_parser 1.1.5+spec-1.1.0, toml_write 0.1.2, toml_writer 1.2.0+spec-1.1.0, transpose 0.2.3, ttf-parser 0.20.0, ttf-parser 0.21.1, ttf-parser 0.25.1, typeid 1.0.3, typenum 1.20.1, unicase 2.10.0, unicode-bidi 0.3.18, unicode-bidi-mirroring 0.2.0, unicode-bidi-mirroring 0.4.0, unicode-ccc 0.2.0, unicode-ccc 0.4.0, unicode-script 0.5.9, unicode-segmentation 1.13.3, unicode-vo 0.1.0, unicode-width 0.2.3, ureq 2.12.1, url 2.5.8, utf-8 0.7.6, vcpkg 0.2.15, version_check 0.9.5, wasm-bindgen 0.2.130, wasm-bindgen-futures 0.4.80, wasm-bindgen-macro 0.2.130, wasm-bindgen-macro-support 0.2.130, wasm-bindgen-shared 0.2.130, wasm-streams 0.4.2, web-sys 0.3.107, web-time 1.1.0, weezl 0.1.12, wgpu 30.0.1, wgpu-core 30.0.1, wgpu-core-deps-apple 30.0.1, wgpu-core-deps-emscripten 30.0.1, wgpu-core-deps-windows-linux-android 30.0.1, wgpu-hal 30.0.1, wgpu-naga-bridge 30.0.1, wgpu-types 30.0.1, winapi 0.3.9, winapi-i686-pc-windows-gnu 0.4.0, winapi-x86_64-pc-windows-gnu 0.4.0, windows 0.44.0, windows 0.57.0, windows 0.61.3, windows 0.62.2, windows-collections 0.2.0, windows-collections 0.3.2, windows-core 0.57.0, windows-core 0.61.2, windows-core 0.62.2, windows-future 0.2.1, windows-future 0.3.2, windows-implement 0.57.0, windows-implement 0.60.2, windows-interface 0.57.0, windows-interface 0.59.3, windows-link 0.1.3, windows-link 0.2.1, windows-numerics 0.2.0, windows-numerics 0.3.1, windows-registry 0.4.0, windows-registry 0.5.3, windows-result 0.1.2, windows-result 0.3.4, windows-result 0.4.1, windows-strings 0.3.1, windows-strings 0.4.2, windows-strings 0.5.1, windows-sys 0.48.0, windows-sys 0.52.0, windows-sys 0.59.0, windows-sys 0.61.2, windows-targets 0.42.2, windows-targets 0.48.5, windows-targets 0.52.6, windows-targets 0.53.5, windows-threading 0.1.0, windows-threading 0.2.1, windows_aarch64_gnullvm 0.42.2, windows_aarch64_gnullvm 0.48.5, windows_aarch64_gnullvm 0.52.6, windows_aarch64_gnullvm 0.53.1, windows_aarch64_msvc 0.42.2, windows_aarch64_msvc 0.48.5, windows_aarch64_msvc 0.52.6, windows_aarch64_msvc 0.53.1, windows_i686_gnu 0.42.2, windows_i686_gnu 0.48.5, windows_i686_gnu 0.52.6, windows_i686_gnu 0.53.1, windows_i686_gnullvm 0.52.6, windows_i686_gnullvm 0.53.1, windows_i686_msvc 0.42.2, windows_i686_msvc 0.48.5, windows_i686_msvc 0.52.6, windows_i686_msvc 0.53.1, windows_x86_64_gnu 0.42.2, windows_x86_64_gnu 0.48.5, windows_x86_64_gnu 0.52.6, windows_x86_64_gnu 0.53.1, windows_x86_64_gnullvm 0.42.2, windows_x86_64_gnullvm 0.48.5, windows_x86_64_gnullvm 0.52.6, windows_x86_64_gnullvm 0.53.1, windows_x86_64_msvc 0.42.2, windows_x86_64_msvc 0.48.5, windows_x86_64_msvc 0.52.6, windows_x86_64_msvc 0.53.1, wio 0.2.2, x11rb 0.13.2, x11rb-protocol 0.13.2, xattr 0.2.3, zed-async-tar 0.5.0-zed, zed-font-kit 0.14.1-zed, zed-reqwest 0.12.15-zed

### MIT (164)

aligned-vec 0.6.4, alsa-sys 0.4.0, arg_enum_proc_macro 0.3.4, ashpd 0.11.1, ashpd 0.12.3, async_zip 0.0.17, av-scenechange 0.14.1, blade-graphics 0.7.1, blade-macros 0.3.0, blade-util 0.3.0, block 0.1.6, block2 0.6.2, built 0.8.1, bytes 1.12.1, calloop 0.13.0, calloop-wayland-source 0.3.0, cfg_aliases 0.2.2, color_quant 1.1.0, combine 4.6.9, convert_case 0.4.0, core_maths 0.1.1, crunchy 0.2.4, data-encoding 2.11.1, deflate64 0.1.12, derive_more 0.99.20, discord-rich-presence 1.1.0, dispatch 0.2.0, dlib 0.5.3, embed-resource 3.0.12, endi 1.1.1, equator 0.4.2, equator-macro 0.4.2, extended 0.1.0, fax 0.2.7, filedescriptor 0.8.3, float-cmp 0.9.0, float_next_after 1.0.0, fontconfig-parser 0.5.8, fontdb 0.16.2, fontdb 0.23.0, freetype-sys 0.20.1, generic-array 0.14.7, grid 0.18.0, h2 0.4.20, hidden-trait 0.1.2, http-body 1.1.0, http-body-util 0.1.5, hyper 1.12.0, hyper-util 0.1.21, imagesize 0.13.0, interpolate_name 0.2.4, is-docker 0.2.0, is-wsl 0.4.0, libm 0.2.16, libredox 0.1.25, libsqlite3-sys 0.35.0, loop9 0.1.5, malloc_buf 0.0.6, maybe-rayon 0.1.1, memoffset 0.9.1, mime_guess 2.0.5, mint 0.5.9, mio 1.2.4, new_debug_unreachable 1.0.6, nix 0.29.0, nix 0.31.3, nom 7.1.3, nom 8.0.0, noop_proc_macro 0.3.0, objc 0.2.7, objc-foundation 0.1.1, objc2 0.6.5, objc2-encode 4.1.0, objc2-foundation 0.3.2, objc_exception 0.1.2, objc_id 0.1.1, oo7 0.5.0, open 5.4.5, openssl-sys 0.9.118, ordered-float 5.4.0, pico-args 0.5.0, postage 0.5.0, quick-xml 0.41.0, realfft 3.5.1, redox_syscall 0.2.16, redox_syscall 0.5.18, redox_users 0.4.6, rgb 0.8.53, rubato 0.16.2, rusqlite 0.37.0, rust-embed 8.13.0, rust-embed-impl 8.13.0, rust-embed-utils 8.13.0, rustybuzz 0.14.1, rustybuzz 0.20.1, schannel 0.1.30, schemars 1.2.3, schemars_derive 1.2.3, seahash 4.1.0, simd-adler32 0.3.10, simd_helpers 0.1.0, slab 0.4.12, souvlaki 0.8.3, spin 0.9.9, strict-num 0.1.1, strum 0.26.3, strum 0.27.2, strum_macros 0.26.4, strum_macros 0.27.2, synstructure 0.14.0, sysinfo 0.31.4, taffy 0.9.0, take-until 0.2.0, tao-core-video-sys 0.2.0, tiff 0.11.3, tokio 1.54.1, tokio-socks 0.5.3, tokio-util 0.7.20, tower 0.5.3, tower-layer 0.3.3, tower-service 0.3.3, tracing 0.1.44, tracing-attributes 0.1.31, tracing-core 0.1.36, try-lock 0.2.5, uds_windows 1.2.1, vswhom 0.1.0, vswhom-sys 0.1.3, want 0.3.2, wayland-backend 0.3.17, wayland-client 0.31.15, wayland-cursor 0.31.14, wayland-protocols 0.31.2, wayland-protocols 0.32.13, wayland-protocols-plasma 0.2.0, wayland-scanner 0.31.11, wayland-sys 0.31.11, which 6.0.3, windows-capture 1.5.0, winnow 0.7.15, winnow 1.0.5, winreg 0.56.0, winsafe 0.0.19, x11 2.21.0, x11-clipboard 0.9.3, xcb 1.7.1, xcursor 0.3.11, xim-ctext 0.3.0, xim-parser 0.2.2, xkbcommon 0.8.0, xml-rs 0.8.29, xmlwriter 0.1.0, y4m 0.8.0, yeslogic-fontconfig-sys 6.0.1, zbus 5.19.0, zbus_macros 5.19.0, zbus_names 4.3.5, zcheapstr 1.2.0, zed-scap 0.0.8-zed, zed-xim 0.4.0-zed, zmij 1.0.23, zvariant 5.15.1, zvariant_derive 5.15.1, zvariant_utils 4.2.0

### Apache-2.0 OR MIT (85)

alsa 0.11.0, async-channel 1.9.0, async-channel 2.5.0, async-executor 1.14.1, async-fs 2.2.0, async-global-executor 2.4.1, async-io 2.6.0, async-lock 3.5.0, async-net 2.0.0, async-process 2.5.0, async-signal 0.2.14, async-std 1.13.2, async-task 4.7.1, atomic 0.5.3, atomic-waker 1.1.2, autocfg 1.5.1, bit-set 0.10.0, bit-set 0.8.0, bit-vec 0.8.0, bit-vec 0.9.1, blocking 1.7.0, cexpr 0.6.0, concurrent-queue 2.5.0, const-oid 0.10.3, ctor 0.4.3, ctor-proc-macro 0.0.6, dtor 0.0.6, dtor-proc-macro 0.0.5, equivalent 1.0.3, event-listener 2.5.3, event-listener 5.4.2, event-listener-strategy 0.5.4, fastrand 1.9.0, fastrand 2.5.0, flume 0.11.1, fnv 1.0.7, futures-lite 1.13.0, futures-lite 2.6.1, idna_adapter 1.2.2, indexmap 2.14.2, kurbo 0.11.3, leak 0.1.2, multiversion_no_op 1.0.0, no_std_io2 0.9.4, ntapi 0.4.3, object 0.39.1, parking 2.2.1, pin-project 1.1.13, pin-project-internal 1.1.13, pin-project-lite 0.2.17, polling 3.11.0, pollster 0.2.5, pollster 1.0.1, portable-atomic 1.15.0, portable-atomic-util 0.2.8, resvg 0.45.1, rustc-hash 1.1.0, rustc-hash 2.1.3, serde_fmt 1.1.0, simd_cesu8 1.2.0, simplecss 0.2.2, smol 2.0.2, sval 2.22.0, sval_buffer 2.22.0, sval_dynamic 2.22.0, sval_fmt 2.22.0, sval_json 2.22.0, sval_nested 2.22.0, sval_ref 2.22.0, sval_serde 2.22.0, svgtypes 0.15.3, swash 0.2.10, usvg 0.45.1, utf8_iter 1.1.0, utf_types 1.0.0, uuid 0.8.2, uuid 1.26.1, value-bag 1.14.1, value-bag-serde1 1.14.1, value-bag-sval2 1.14.1, waker-fn 1.2.0, yazi 0.2.2, zeno 0.3.3, zeroize 1.9.1, zeroize_derive 1.5.0

### Apache-2.0 (28)

clang-sys 1.9.1, codespan-reporting 0.12.0, codespan-reporting 0.13.1, command-fds 0.3.3, cpal 0.18.2, gethostname 1.1.0, gl_generator 0.14.0, glutin_wgl_sys 0.6.1, gpui 0.2.2, gpui-macros 0.2.2, gpui_collections 0.2.2, gpui_derive_refineable 0.2.2, gpui_http_client 0.2.2, gpui_media 0.2.2, gpui_perf 0.2.2, gpui_refineable 0.2.2, gpui_semantic_version 0.2.2, gpui_sum_tree 0.2.2, gpui_util 0.2.2, gpui_util_macros 0.2.2, khronos_api 3.1.0, openssl 0.10.82, spirv 0.3.0+sdk-1.3.268.0, spirv 0.4.0+sdk-1.4.341.0, stacksafe 0.1.4, stacksafe-macro 0.1.4, sync_wrapper 1.0.2, unicode-linebreak 0.1.5

### Unicode-3.0 (18)

icu_collections 2.4.0, icu_locale_core 2.4.0, icu_normalizer 2.4.0, icu_normalizer_data 2.4.0, icu_properties 2.4.0, icu_properties_data 2.4.0, icu_provider 2.4.0, litemap 0.8.3, potential_utf 0.1.6, tinystr 0.8.5, writeable 0.6.4, yoke 0.8.4, yoke-derive 0.8.4, zerofrom 0.1.8, zerofrom-derive 0.1.8, zerotrie 0.2.5, zerovec 0.11.9, zerovec-derive 0.11.7

### MPL-2.0 (15)

cbindgen 0.28.0, dwrote 0.11.5, option-ext 0.2.0, symphonia 0.5.5, symphonia-bundle-flac 0.5.5, symphonia-bundle-mp3 0.5.5, symphonia-codec-aac 0.5.5, symphonia-codec-pcm 0.5.5, symphonia-codec-vorbis 0.5.5, symphonia-core 0.5.5, symphonia-format-isomp4 0.5.5, symphonia-format-ogg 0.5.5, symphonia-format-riff 0.5.5, symphonia-metadata 0.5.5, symphonia-utils-xiph 0.5.5

### Zlib OR Apache-2.0 OR MIT (15)

bytemuck 1.25.2, bytemuck_derive 1.12.1, dispatch2 0.3.1, objc2-app-kit 0.3.2, objc2-audio-toolbox 0.3.2, objc2-avf-audio 0.3.2, objc2-core-audio 0.3.2, objc2-core-audio-types 0.3.2, objc2-core-foundation 0.3.2, objc2-core-graphics 0.3.2, objc2-io-surface 0.3.2, objc2-metal 0.3.2, objc2-quartz-core 0.3.2, objc2-ui-kit 0.3.2, tinyvec 1.13.3

### BSD-3-Clause (10)

avif-serialize 0.8.9, bindgen 0.71.1, exr 1.74.3, instant 0.1.13, lebe 0.5.3, ravif 0.13.0, sha1_smol 1.0.1, subtle 2.6.1, tiny-skia 0.11.4, tiny-skia-path 0.11.4

### Unlicense OR MIT (9)

aho-corasick 1.1.5, byteorder 1.5.0, byteorder-lite 0.1.0, globset 0.4.21, memchr 2.8.3, same-file 1.0.6, termcolor 1.4.1, walkdir 2.5.0, winapi-util 0.1.11

### MIT OR Apache-2.0 OR Zlib (8)

glow 0.16.0, glow 0.17.0, lru-slab 0.1.3, raw-window-handle 0.6.2, xkeysym 0.2.1, zune-core 0.5.3, zune-inflate 0.2.54, zune-jpeg 0.5.15

### Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT (7)

linux-raw-sys 0.12.1, linux-raw-sys 0.4.15, rustix 0.38.44, rustix 1.1.6, wasi 0.11.1+wasi-snapshot-preview1, wasip2 1.0.4+wasi-0.2.12, wit-bindgen 0.57.1

### Zlib (5)

foldhash 0.1.5, foldhash 0.2.0, nanorand 0.7.0, slotmap 1.1.1, zlib-rs 0.6.8

### Apache-2.0 OR ISC OR MIT (4)

hyper-rustls 0.27.10, rustls 0.23.46, rustls-native-certs 0.8.5, rustls-pemfile 2.2.0

### BSD-2-Clause (4)

arrayref 0.3.9, av1-grain 0.2.5, rav1e 0.8.1, v_frame 0.3.9

### ISC (3)

libloading 0.8.9, rustls-webpki 0.103.16, untrusted 0.9.0

### (MIT OR Apache-2.0) AND Unicode-3.0 (2)

unicode-ident 1.0.27, unicode-properties 0.1.5

### BSD-2-Clause OR Apache-2.0 OR MIT (2)

zerocopy 0.8.63, zerocopy-derive 0.8.63

### BSD-3-Clause OR Apache-2.0 (2)

moxcms 0.8.1, pxfm 0.1.30

### BSD-3-Clause OR MIT OR Apache-2.0 (2)

num_enum 0.7.6, num_enum_derive 0.7.6

### CC0-1.0 (2)

hexf-parse 0.2.1, tiny-keccak 2.0.2

### MIT OR Apache-2.0 OR LGPL-2.1-or-later (2)

r-efi 5.3.0, r-efi 6.0.0

### MIT OR Zlib OR Apache-2.0 (2)

miniz_oxide 0.8.9, miniz_oxide 0.9.1

### (Apache-2.0 OR MIT) AND BSD-3-Clause (1)

encoding_rs 0.8.44

### (MIT OR Apache-2.0) AND NCSA (1)

libfuzzer-sys 0.4.13

### 0BSD OR MIT OR Apache-2.0 (1)

adler2 2.0.1

### Apache-2.0 AND ISC (1)

ring 0.17.14

### Apache-2.0 OR BSL-1.0 (1)

ryu 1.0.23

### Apache-2.0 OR GPL-2.0-only (1)

self_cell 1.3.0

### Apache-2.0 WITH LLVM-exception (1)

ar_archive_writer 0.5.3

### BSD-2-Clause OR MIT OR Apache-2.0 (1)

mach2 0.6.0

### CC0-1.0 OR Apache-2.0 (1)

imgref 1.12.3

### CC0-1.0 OR MIT-0 OR Apache-2.0 (1)

dunce 1.0.5

## 3. Código copiado ou adaptado de outros projetos

| Origem | Licença | O que veio | Onde está |
| --- | --- | --- | --- |
| `preview/` deste repositório (prévia em JS) | Mesmo autor | Pré-análise e diretor portados para Rust | `app/crates/preanalise`, `app/crates/analise` |

Projetos de referência que serão usados nas próximas fases (ver `docs/plano-v2.md`): MixMusic (MIT ou GPL-3), Spotifast (MIT), limusic (GPL-3), Spotifry (GPL-3), SimpMusic (GPL-3), SongRec (GPL-3). Cada trecho copiado entra na tabela acima quando for usado.
