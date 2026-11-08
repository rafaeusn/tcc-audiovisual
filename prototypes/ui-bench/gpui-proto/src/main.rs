//! Protótipo GPUI (o framework do editor Zed).
//!
//! Mesmo layout em árvore, mesmo shader e mesmas medições do protótipo Slint, para comparar só a UI.
//! Diferenças que fazem parte do resultado:
//! - A interface é código Rust compilado: troca de tema funciona ao vivo, mas não existe "skin" carregada em
//!   tempo de execução (para mudar o desenho é preciso recompilar).
//! - O GPUI não aceita uma textura wgpu externa: o shader roda num device wgpu separado, os pixels voltam para a
//!   CPU e sobem de novo como imagem a cada quadro.
//! - Animações de posição são feitas à mão (interpolação com a mesma curva Bézier do Hyprland).
//!
//! Medição: `gpui-proto --bench 20 --synthetic --out bench-results/gpui.json`.
//!
//! Por padrão o visual vai numa camada do DirectComposition por baixo da UI (zero cópia, ver `underlay.rs`).
//! `--readback` usa o caminho antigo (pixels pela CPU), para comparar.

mod skin;
#[cfg(windows)]
mod underlay;

use skin::{Ctx, Val};

use gpui::{
    AnyElement, App, Application, Bounds, Context, CursorStyle, FocusHandle, Hsla, ImageSource, KeyDownEvent,
    MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ObjectFit, RenderImage, Rgba, SharedString,
    TitlebarOptions, Window, WindowBounds, WindowOptions, div, img, prelude::*, px, size,
};
use proto_core::audio::{Analyzer, Listener};
use proto_core::layout::{self, LayoutFile, Rect};
use proto_core::metrics::{self, BenchArgs, FrameClock};
use proto_core::visual::{self, Params, Visual};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};

const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

#[derive(Clone)]
struct Theme {
    name: String,
    bg: Hsla,
    surface: Hsla,
    surface2: Hsla,
    border: Hsla,
    text: Hsla,
    muted: Hsla,
    accent: Hsla,
    accent2: Hsla,
    radius: f32,
    border_width: f32,
    anim_ms: f32,
    font: SharedString,
}

fn hex(s: &str) -> Option<Hsla> {
    let h = s.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok().map(|v| v as f32 / 255.0);
    let a = if h.len() == 8 { p(6)? } else { 1.0 };
    Some(Rgba { r: p(0)?, g: p(2)?, b: p(4)?, a }.into())
}

impl skin::Tokens for Theme {
    fn color(&self, token: &str) -> Option<Hsla> {
        Some(match token {
            "bg" => self.bg,
            "surface" => self.surface,
            "surface-2" => self.surface2,
            "border" => self.border,
            "text" => self.text,
            "muted" => self.muted,
            "accent" => self.accent,
            "accent-2" => self.accent2,
            _ => return None,
        })
    }
    fn radius(&self) -> f32 {
        self.radius
    }
    fn border_width(&self) -> f32 {
        self.border_width
    }
    fn font(&self) -> SharedString {
        self.font.clone()
    }
}

impl Theme {
    fn load(path: &Path) -> Option<Theme> {
        let t: toml::Table = toml::from_str(&std::fs::read_to_string(path).ok()?).ok()?;
        let c = |k: &str| t.get(k).and_then(|v| v.as_str()).and_then(hex).unwrap_or(gpui::white());
        let n = |k: &str, d: f32| t.get(k).and_then(|v| v.as_float().or(v.as_integer().map(|i| i as f64))).map(|v| v as f32).unwrap_or(d);
        Some(Theme {
            name: t.get("name").and_then(|v| v.as_str()).unwrap_or("?").into(),
            bg: c("bg"),
            surface: c("surface"),
            surface2: c("surface-2"),
            border: c("border"),
            text: c("text"),
            muted: c("muted"),
            accent: c("accent"),
            accent2: c("accent-2"),
            radius: n("radius", 14.0),
            border_width: n("border-width", 1.0),
            anim_ms: n("anim", 280.0),
            font: t.get("font").and_then(|v| v.as_str()).unwrap_or("Segoe UI").to_string().into(),
        })
    }
}

/// Curva cubic-bezier (x1, y1, x2, y2) avaliada em x (Newton + bisseção).
fn bezier(x1: f32, y1: f32, x2: f32, y2: f32, x: f32) -> f32 {
    let bx = |t: f32| 3.0 * (1.0 - t) * (1.0 - t) * t * x1 + 3.0 * (1.0 - t) * t * t * x2 + t * t * t;
    let by = |t: f32| 3.0 * (1.0 - t) * (1.0 - t) * t * y1 + 3.0 * (1.0 - t) * t * t * y2 + t * t * t;
    let (mut lo, mut hi, mut t) = (0.0f32, 1.0f32, x);
    for _ in 0..20 {
        let v = bx(t);
        if (v - x).abs() < 1e-4 {
            break;
        }
        if v < x {
            lo = t;
        } else {
            hi = t;
        }
        t = (lo + hi) / 2.0;
    }
    by(t)
}

struct Tween {
    from: Rect,
    to: Rect,
    start: Instant,
}

impl Tween {
    fn at(&self, dur_ms: f32, instant: bool) -> Rect {
        if instant {
            return self.to;
        }
        let k = (self.start.elapsed().as_secs_f32() * 1000.0 / dur_ms.max(1.0)).min(1.0);
        let e = bezier(0.05, 0.9, 0.1, 1.05, k);
        let l = |a: f32, b: f32| a + (b - a) * e;
        Rect { x: l(self.from.x, self.to.x), y: l(self.from.y, self.to.y), w: l(self.from.w, self.to.w), h: l(self.from.h, self.to.h) }
    }
}

struct Readback {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: String,
    visual: Visual,
}

impl Readback {
    fn new() -> Self {
        let (device, queue, adapter) = visual::headless_device().expect("wgpu");
        let visual = Visual::with_format(&device, wgpu::TextureFormat::Bgra8Unorm);
        Self { device, queue, adapter, visual }
    }
}

enum Drag {
    Divider(usize),
    Panel(String),
    Floating { id: String, dx: f32, dy: f32 },
    Resize(String),
}

struct Proto {
    focus: FocusHandle,
    layout: LayoutFile,
    themes: Vec<PathBuf>,
    theme_idx: usize,
    theme: Theme,
    theme_mtime: Option<SystemTime>,
    listener: Listener,
    analyzer: Analyzer,
    clock: FrameClock,
    /// Caminho com cópia (só existe com `--readback` ou se a camada falhar): device wgpu próprio + leitura.
    readback: Option<Readback>,
    pixels: Vec<u8>,
    image: Option<Arc<RenderImage>>,
    tweens: HashMap<String, Tween>,
    edit: bool,
    drag: Option<Drag>,
    preview: Option<Rect>,
    win: (f32, f32),
    params: Params,
    last_text: Instant,
    last_watch: Instant,
    clock_text: SharedString,
    stats_text: SharedString,
    hint: Option<(SharedString, Instant)>,
    progress: f32,
    bench: BenchArgs,
    bench_started: bool,
    started: Instant,
    ui_hz: Option<f32>,
    ui_accum: f32,
    #[cfg(windows)]
    underlay: Option<underlay::Underlay>,
    skins: Vec<PathBuf>,
    skin_idx: usize,
    skin: skin::Skin,
    skin_mtime: Option<SystemTime>,
    skin_error: Option<String>,
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl Proto {
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ws = Path::new(CRATE_DIR).parent().unwrap().to_path_buf();
        let mut themes: Vec<PathBuf> = std::fs::read_dir(ws.join("themes"))
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "toml")).collect())
            .unwrap_or_default();
        themes.sort();
        let theme = Theme::load(&themes[0]).expect("tema");
        // `--layout arquivo.toml` troca o layout (cenários de medição).
        let args0: Vec<String> = std::env::args().collect();
        let layout_path = args0.iter().position(|x| x == "--layout").and_then(|i| args0.get(i + 1)).map(PathBuf::from).unwrap_or(ws.join("layout.user.toml"));
        let layout_text = std::fs::read_to_string(&layout_path).unwrap_or_else(|_| proto_core::DEFAULT_LAYOUT.to_string());
        let layout = LayoutFile::parse(&layout_text).or_else(|_| LayoutFile::parse(proto_core::DEFAULT_LAYOUT)).unwrap();
        let bench = BenchArgs::parse("bench-results/gpui.json");
        let focus = cx.focus_handle();
        window.focus(&focus);
        if args0.iter().any(|a| a == "--fullscreen") {
            window.toggle_fullscreen();
        }
        let args: Vec<String> = std::env::args().collect();
        let ui_hz = args.iter().position(|x| x == "--ui-hz").and_then(|i| args.get(i + 1)).and_then(|v| v.parse().ok());
        #[cfg(windows)]
        let underlay = if args.iter().any(|a| a == "--readback") {
            None
        } else {
            use raw_window_handle::{HasWindowHandle, RawWindowHandle};
            let scale = window.scale_factor();
            let vp = window.viewport_size();
            let (w, h) = ((f32::from(vp.width) * scale) as u32, (f32::from(vp.height) * scale) as u32);
            match window.window_handle().map(|h| h.as_raw()) {
                Ok(RawWindowHandle::Win32(hw)) => match underlay::Underlay::new(hw.hwnd.get(), w, h) {
                    Ok(u) => Some(u),
                    Err(e) => {
                        eprintln!("camada sem cópia indisponível ({e}); usando readback");
                        None
                    }
                },
                _ => None,
            }
        };
        Self {
            focus,
            layout,
            theme_mtime: mtime(&themes[0]),
            themes,
            theme_idx: 0,
            theme,
            listener: Listener::start(bench.synthetic),
            analyzer: Analyzer::default(),
            clock: FrameClock::default(),
            readback: None,
            pixels: Vec::new(),
            image: None,
            tweens: HashMap::new(),
            edit: false,
            drag: None,
            preview: None,
            win: (0.0, 0.0),
            params: Params::default(),
            last_text: Instant::now(),
            last_watch: Instant::now(),
            clock_text: "".into(),
            stats_text: "".into(),
            hint: None,
            progress: 0.0,
            bench,
            bench_started: false,
            started: Instant::now(),
            ui_hz,
            ui_accum: 0.0,
            #[cfg(windows)]
            underlay,
            skins: Vec::new(),
            skin_idx: 0,
            skin: skin::Skin::default(),
            skin_mtime: None,
            skin_error: None,
        }
        .with_readback_if_needed()
        .with_skins()
    }

    fn with_skins(mut self) -> Self {
        let dir = Path::new(CRATE_DIR).join("skins");
        let mut v: Vec<PathBuf> = std::fs::read_dir(&dir)
            .map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "kdl")).collect())
            .unwrap_or_default();
        v.sort();
        // `--skin nome` escolhe a skin inicial (cenários de medição).
        let args: Vec<String> = std::env::args().collect();
        let want = args.iter().position(|x| x == "--skin").and_then(|i| args.get(i + 1)).cloned().unwrap_or("default".into());
        self.skin_idx = v.iter().position(|p| p.file_stem().is_some_and(|s| s == want.as_str())).unwrap_or(0);
        self.skins = v;
        self.load_skin();
        self
    }

    /// Lê a skin atual. Com erro, a anterior continua e a mensagem aparece no topo da janela.
    fn load_skin(&mut self) {
        let Some(path) = self.skins.get(self.skin_idx).cloned() else { return };
        self.skin_mtime = mtime(&path);
        match std::fs::read_to_string(&path).map_err(|e| e.to_string()).and_then(|t| skin::Skin::parse(&t)) {
            Ok(sk) => {
                self.skin = sk;
                self.skin_error = None;
            }
            Err(e) => self.skin_error = Some(format!("{}: {e}", path.file_name().unwrap().to_string_lossy())),
        }
    }

    fn with_readback_if_needed(mut self) -> Self {
        if !self.zero_copy() {
            self.readback = Some(Readback::new());
        }
        self
    }

    fn zero_copy(&self) -> bool {
        #[cfg(windows)]
        {
            self.underlay.is_some()
        }
        #[cfg(not(windows))]
        {
            false
        }
    }

    /// Alvos do layout → interpolações (cada painel anima da posição atual até a nova).
    fn sync_tweens(&mut self) {
        let (panels, _) = self.layout.compute(self.win.0, self.win.1);
        let dur = self.theme.anim_ms;
        let instant = self.drag.as_ref().is_some_and(|d| !matches!(d, Drag::Panel(_)));
        for p in panels {
            match self.tweens.get_mut(&p.panel) {
                Some(t) if t.to != p.rect => {
                    t.from = t.at(dur, instant);
                    t.to = p.rect;
                    t.start = Instant::now();
                }
                Some(_) => {}
                None => {
                    self.tweens.insert(p.panel.clone(), Tween { from: p.rect, to: p.rect, start: Instant::now() });
                }
            }
        }
    }

    fn set_hint(&mut self, msg: impl Into<SharedString>) {
        self.hint = Some((msg.into(), Instant::now() + Duration::from_millis(2500)));
    }

    fn on_key(&mut self, key: &str, window: &mut Window) {
        match key {
            "e" => {
                self.edit = !self.edit;
            }
            "escape" => self.edit = false,
            "t" => {
                self.theme_idx = (self.theme_idx + 1) % self.themes.len();
                if let Some(t) = Theme::load(&self.themes[self.theme_idx]) {
                    self.theme = t;
                    self.theme_mtime = mtime(&self.themes[self.theme_idx]);
                    self.set_hint(format!("Tema: {}", self.theme.name));
                }
            }
            "s" => {
                if !self.skins.is_empty() {
                    self.skin_idx = (self.skin_idx + 1) % self.skins.len();
                    self.load_skin();
                    let name = self.skins[self.skin_idx].file_stem().unwrap().to_string_lossy().to_string();
                    self.set_hint(format!("Skin: {name} · Tema: {}", self.theme.name));
                }
            }
            "f" => window.toggle_fullscreen(),
            _ => {}
        }
    }

    fn on_move(&mut self, x: f32, y: f32) {
        let (w, h) = self.win;
        match &self.drag {
            Some(Drag::Divider(i)) => {
                let (_, divs) = self.layout.compute(w, h);
                if let Some(d) = divs.get(*i) {
                    self.layout.drag_divider(w, d, x, y);
                }
            }
            Some(Drag::Panel(id)) => {
                let (rects, _) = self.layout.compute(w, h);
                self.preview = rects
                    .iter()
                    .find(|r| !r.floating && &r.panel != id && r.rect.contains(x, y))
                    .map(|t| layout::drop_preview(&t.rect, x, y).1);
            }
            Some(Drag::Floating { id, dx, dy }) => {
                let id = id.clone();
                if let Some(f) = self.layout.floating.iter().find(|f| f.panel == id) {
                    let (fw, fh) = (f.w * w, f.h * h);
                    self.layout.move_floating(&id, x - dx, y - dy, fw, fh, w, h);
                }
            }
            Some(Drag::Resize(id)) => {
                let id = id.clone();
                if let Some(f) = self.layout.floating.iter().find(|f| f.panel == id) {
                    let (fx, fy) = (f.x * w, f.y * h);
                    self.layout.move_floating(&id, fx, fy, (x - fx).max(60.0), (y - fy).max(40.0), w, h);
                }
            }
            None => {}
        }
    }

    fn on_up(&mut self, x: f32, y: f32) {
        if let Some(Drag::Panel(id)) = self.drag.take() {
            let (w, h) = self.win;
            self.layout.drop_panel(w, h, &id, x, y);
        }
        self.preview = None;
    }

    fn frame(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let dt = self.clock.tick();
        self.ui_accum += dt;
        let p = self.analyzer.update(dt, &self.listener);
        // Sem `--ui-hz`, a UI recebe os números novos a todo quadro (igual ao Slint sem limite).
        if self.ui_hz.is_none_or(|hz| self.ui_accum >= 1.0 / hz) {
            self.params = p;
            self.ui_accum = 0.0;
        }
        let vp = window.viewport_size();
        self.win = (f32::from(vp.width), f32::from(vp.height));
        self.sync_tweens();

        let scale = window.scale_factor();
        #[cfg(windows)]
        if let Some(u) = self.underlay.as_mut() {
            // Zero cópia: fundo + visual direto na camada de baixo, no retângulo que o painel ocupa agora.
            let instant = self.drag.as_ref().is_some_and(|d| !matches!(d, Drag::Panel(_)));
            let full = self.skin.app.get("visual").is_some_and(|v| v == "fullscreen");
            let (r, radius) = if full {
                (Rect { x: 0.0, y: 0.0, w: self.win.0, h: self.win.1 }, 0.0)
            } else {
                let r = self.tweens.get("visual").map(|t| t.at(self.theme.anim_ms, instant)).unwrap_or(Rect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 });
                let rad = match self.skin.frame.get("radius").map(String::as_str) {
                    Some("theme") | None => self.theme.radius,
                    Some(v) => v.parse().unwrap_or(self.theme.radius),
                };
                (r, rad)
            };
            let bg_token = self.skin.app.get("bg").cloned().unwrap_or("bg".into());
            let bg = skin::color(&self.theme, &bg_token).unwrap_or(self.theme.bg).to_rgb();
            u.resize((self.win.0 * scale) as u32, (self.win.1 * scale) as u32);
            u.render([bg.r, bg.g, bg.b, 1.0], (r.x * scale, r.y * scale, r.w * scale, r.h * scale), radius * scale, &p);
        }
        // Readback: desenha no device próprio, lê de volta e sobe como imagem.
        let vr = self.tweens.get("visual").map(|t| t.to).unwrap_or(Rect { x: 0.0, y: 0.0, w: 64.0, h: 64.0 });
        let mut size = None;
        if let Some(rb) = self.readback.as_mut() {
            rb.visual.ensure_size(&rb.device, (vr.w * scale) as u32, (vr.h * scale) as u32);
            rb.visual.render_and_read(&rb.device, &rb.queue, &p, &mut self.pixels);
            size = Some(rb.visual.size);
        }
        if let Some((tw, th)) = size
            && let Some(buf) = image::RgbaImage::from_raw(tw, th, std::mem::take(&mut self.pixels))
        {
            let frame = image::Frame::new(buf);
            let new = Arc::new(RenderImage::new(smallvec::SmallVec::from_elem(frame, 1)));
            if let Some(old) = self.image.replace(new) {
                // Sem isso o atlas de imagens do GPUI cresce a cada quadro.
                let _ = window.drop_image(old);
            }
        }
        self.progress = (self.progress + dt / 200.0).fract();

        if self.last_text.elapsed() > Duration::from_millis(500) {
            self.last_text = Instant::now();
            let (ws, _) = metrics::memory_mb();
            self.clock_text = metrics::local_clock().into();
            self.stats_text = format!("{:.0} fps · {:.0} MB", self.clock.fps, ws).into();
        }
        if self.last_watch.elapsed() > Duration::from_millis(300) {
            self.last_watch = Instant::now();
            if let Some(sp) = self.skins.get(self.skin_idx)
                && mtime(sp) != self.skin_mtime
            {
                self.load_skin();
            }
            let path = &self.themes[self.theme_idx];
            let m = mtime(path);
            if m != self.theme_mtime {
                self.theme_mtime = m;
                if let Some(t) = Theme::load(path) {
                    self.theme = t;
                }
            }
        }
        if let Some(secs) = self.bench.seconds {
            let el = self.started.elapsed().as_secs_f32();
            if !self.bench_started && el > 2.0 {
                self.bench_started = true;
                self.clock.start_bench();
            }
            if self.bench_started && el > 2.0 + secs {
                let b = window.bounds();
                let sz = ((f32::from(b.size.width) * scale) as u32, (f32::from(b.size.height) * scale) as u32);
                #[cfg(windows)]
                let renderer = match &self.underlay {
                    Some(u) => format!("gpui DirectX + camada DirectComposition (zero cópia): {}", u.adapter),
                    None => format!("gpui DirectX + wgpu readback: {}", self.readback.as_ref().map(|r| r.adapter.as_str()).unwrap_or("?")),
                };
                #[cfg(not(windows))]
                let renderer = format!("gpui + wgpu readback: {}", self.readback.as_ref().map(|r| r.adapter.as_str()).unwrap_or("?"));
                let r = self.clock.report("gpui 0.2.2", &renderer, &self.listener.source, sz);
                r.save(&self.bench.out);
                cx.quit();
            }
        }
        window.request_animation_frame();
    }

    /// Valores ao vivo que as skins leem (`"{bass}"`, `each "queue"` …).
    fn skin_vars(&self) -> HashMap<String, Val> {
        let p = &self.params;
        let mut v = HashMap::new();
        for (k, x) in [("bass", p.bass), ("mid", p.mid), ("high", p.high), ("level", p.level), ("intensity", p.intensity), ("pulse", p.pulse), ("progress", self.progress)] {
            v.insert(k.to_string(), Val::N(x));
        }
        let title = if self.listener.is_synthetic() { "Sinal sintético (120 BPM)" } else { "Modo Escuta" };
        v.insert("now.title".into(), Val::S(title.into()));
        v.insert("now.source".into(), Val::S(self.listener.source.clone()));
        v.insert("clock".into(), Val::S(self.clock_text.to_string()));
        v.insert("stats".into(), Val::S(self.stats_text.to_string()));
        v.insert("theme.name".into(), Val::S(self.theme.name.clone()));
        let rows = [
            ("Faixa local — exemplo.mp3", "arquivo", true),
            ("Qualquer app tocando", "Modo Escuta", false),
            ("Audius — faixa", "módulo", false),
            ("Jamendo — faixa", "módulo", false),
        ];
        v.insert(
            "queue".into(),
            Val::L(
                rows.iter()
                    .map(|(t, a, pl)| {
                        HashMap::from([("title".to_string(), Val::S((*t).into())), ("artist".to_string(), Val::S((*a).into())), ("playing".to_string(), Val::B(*pl))])
                    })
                    .collect(),
            ),
        );
        v
    }

    fn visual_fullscreen(&self) -> bool {
        self.zero_copy() && self.skin.app.get("visual").is_some_and(|v| v == "fullscreen")
    }

    fn panel(&self, id: &str, r: Rect, floating: bool, vars: &HashMap<String, Val>, cx: &mut Context<Self>) -> impl IntoElement {
        let t = &self.theme;
        let sk = &self.skin;
        let ctx = Ctx { vars, item: None };
        let ps = sk.panels.get(id);
        let id_s: SharedString = id.to_string().into();
        let header_id = id_s.clone();

        // Cabeçalho: estilo da skin + alça de arrastar no modo edição.
        let mut hp = sk.header.clone();
        let hh = hp.remove("height").map(|h| ctx.num(&h)).unwrap_or(30.0);
        let transform_upper = hp.remove("transform").is_some_and(|v| v == "upper");
        let title = ps.map(|p| p.title.clone()).unwrap_or_default();
        let title = if transform_upper { title.to_uppercase() } else { title };
        let mut header = skin::apply_box(div().absolute().top_0().left_0().w_full().h(px(hh)).flex().items_center().gap(px(8.0)), &hp, t, &ctx, (r.w, r.h))
            .when(self.edit, |d| d.child(div().text_color(t.accent).child("⠿")))
            .child(title);
        if self.edit {
            header = header.cursor(CursorStyle::OpenHand).on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                    let id = header_id.to_string();
                    this.drag = Some(if floating {
                        let rr = this.tweens.get(&id).map(|t| t.to).unwrap_or(r);
                        Drag::Floating { id, dx: f32::from(ev.position.x) - rr.x, dy: f32::from(ev.position.y) - rr.y }
                    } else {
                        Drag::Panel(id)
                    });
                    cx.stop_propagation();
                }),
            );
        }

        // Conteúdo: o visual (imagem no modo com cópia; nada no modo camada) ou a árvore da skin.
        let body: AnyElement = if id == "visual" {
            match (&self.image, self.zero_copy()) {
                (Some(im), false) => div().size_full().child(img(ImageSource::Render(im.clone())).size_full().object_fit(ObjectFit::Cover)).into_any_element(),
                _ => div().into_any_element(),
            }
        } else {
            div().size_full().children(ps.map(|p| skin::build(&p.body, t, &ctx, (r.w, r.h))).unwrap_or_default()).into_any_element()
        };

        // Moldura: estilo da skin; no modo edição, borda de destaque.
        let mut fp = sk.frame.clone();
        let edit_border = fp.remove("edit-border").unwrap_or("accent".into());
        let shadow = fp.remove("shadow").unwrap_or("floating".into());
        if self.edit {
            fp.insert("border".into(), edit_border);
            fp.insert("border-width".into(), "2".into());
        }
        if id == "visual" {
            // O visual é desenhado na camada de baixo (ou como imagem): a moldura não pinta fundo.
            fp.insert("bg".into(), if self.zero_copy() { "none".into() } else { "#000000".into() });
            if self.visual_fullscreen() && !self.edit {
                fp.insert("border".into(), "none".into());
            }
        }
        if shadow == "all" || (shadow == "floating" && floating) {
            fp.insert("shadow".into(), "true".into());
        }
        let resize_id = id_s.clone();
        let frame = div().absolute().left(px(r.x)).top(px(r.y)).w(px(r.w)).h(px(r.h)).font_family(t.font.clone());
        skin::apply_box(frame, &fp, t, &ctx, (r.w, r.h))
            .child(body)
            .child(header)
            .when(self.edit && floating, |d| {
                d.child(
                    div()
                        .absolute()
                        .right(px(4.0))
                        .bottom(px(4.0))
                        .size(px(10.0))
                        .rounded(px(2.0))
                        .bg(t.accent)
                        .cursor(CursorStyle::ResizeUpLeftDownRight)
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                                this.drag = Some(Drag::Resize(resize_id.to_string()));
                                cx.stop_propagation();
                            }),
                        ),
                )
            })
    }
}

impl Render for Proto {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.frame(window, cx);
        let t = self.theme.clone();
        let dur = t.anim_ms;
        let instant = self.drag.as_ref().is_some_and(|d| !matches!(d, Drag::Panel(_)));
        let (rects, divs) = self.layout.compute(self.win.0, self.win.1);
        let vars = self.skin_vars();
        let mut order: Vec<_> = rects.iter().collect();
        order.sort_by(|a, b| a.floating.cmp(&b.floating).then_with(|| a.panel.cmp(&b.panel)));
        let panels: Vec<_> = order
            .iter()
            .map(|p| {
                let r = self.tweens.get(&p.panel).map(|tw| tw.at(dur, instant)).unwrap_or(p.rect);
                self.panel(&p.panel, r, p.floating, &vars, cx).into_any_element()
            })
            .collect();
        let dividers: Vec<_> = if self.edit {
            divs.iter()
                .enumerate()
                .map(|(i, d)| {
                    let row = d.dir == layout::Dir::Row;
                    div()
                        .absolute()
                        .left(px(d.rect.x))
                        .top(px(d.rect.y))
                        .w(px(d.rect.w))
                        .h(px(d.rect.h))
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor(if row { CursorStyle::ResizeLeftRight } else { CursorStyle::ResizeUpDown })
                        .child(div().w(px(if row { 4.0 } else { 48.0 })).h(px(if row { 48.0 } else { 4.0 })).rounded(px(2.0)).bg(t.muted))
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                                if std::env::var_os("PROTO_DEBUG").is_some() { eprintln!("down divisoria {i}"); }
                                this.drag = Some(Drag::Divider(i));
                                cx.stop_propagation();
                            }),
                        )
                        .into_any_element()
                })
                .collect()
        } else {
            Vec::new()
        };
        let hint = match &self.hint {
            Some((h, until)) if Instant::now() < *until => Some(h.clone()),
            _ if self.edit => Some("Modo edição · arraste cabeçalhos e divisórias · E para sair".into()),
            _ => None,
        };
        let zero_copy = self.zero_copy();
        div()
            .size_full()
            .relative()
            // Com a camada de baixo, o fundo é desenhado lá; aqui fica transparente para o visual aparecer.
            .when(!zero_copy, |d| d.bg(self.skin.app.get("bg").and_then(|b| skin::color(&t, b)).unwrap_or(t.bg)))
            .font_family(t.font.clone())
            .track_focus(&self.focus)
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, window, cx| {
                this.on_key(ev.keystroke.key.as_str(), window);
                cx.notify();
            }))
            .on_mouse_move(cx.listener(|this, ev: &MouseMoveEvent, _, _| {
                if std::env::var_os("PROTO_DEBUG").is_some() { eprintln!("move {:?} drag={}", ev.position, this.drag.is_some()); }
                if this.drag.is_some() {
                    this.on_move(f32::from(ev.position.x), f32::from(ev.position.y));
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, ev: &MouseUpEvent, _, _| {
                    this.on_up(f32::from(ev.position.x), f32::from(ev.position.y));
                    this.drag = None;
                }),
            )
            .children(panels)
            .children(dividers)
            .when_some(self.preview, |d, r| {
                d.child(
                    div()
                        .absolute()
                        .left(px(r.x))
                        .top(px(r.y))
                        .w(px(r.w))
                        .h(px(r.h))
                        .rounded(px(t.radius))
                        .bg(t.accent.opacity(0.18))
                        .border_2()
                        .border_color(t.accent),
                )
            })
            .when_some(self.skin_error.clone(), |d, e| {
                d.child(
                    div()
                        .absolute()
                        .top(px(16.0))
                        .left(px(16.0))
                        .right(px(16.0))
                        .p(px(10.0))
                        .rounded(px(10.0))
                        .bg(skin::parse_hex("#4a1020").unwrap())
                        .text_color(skin::parse_hex("#ffd7de").unwrap())
                        .text_size(px(12.0))
                        .child(format!("Skin com erro (a anterior continua ativa): {e}")),
                )
            })
            .when_some(hint, |d, h| {
                d.child(
                    div().absolute().bottom(px(14.0)).w_full().flex().justify_center().child(
                        div()
                            .px(px(14.0))
                            .py(px(6.0))
                            .rounded(px(15.0))
                            .bg(t.surface2)
                            .border_1()
                            .border_color(t.border)
                            .text_color(t.text)
                            .text_size(px(12.0))
                            .child(h),
                    ),
                )
            })
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(1280.0), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions { title: Some("Protótipo GPUI — layout em árvore".into()), ..Default::default() }),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Proto::new(window, cx)),
        )
        .expect("janela");
        cx.on_window_closed(|cx| cx.quit()).detach();
        cx.activate(true);
    });
}
