//! Protótipo Slint.
//!
//! - A interface inteira vem de `skins/<nome>/app.slint`, compilada em tempo de execução pelo slint-interpreter.
//!   Salvar o arquivo recarrega a skin na hora; `S` troca de skin.
//! - Tema por tokens (`../themes/*.toml`), aplicado no global `Theme` da skin; `T` troca.
//! - Layout em árvore (proto-core) com divisórias arrastáveis e painéis que trocam de lugar (`E` = modo edição).
//! - O visual é desenhado com wgpu no mesmo device do Slint e entra na UI como imagem, sem cópia.
//!
//! Medição: `slint-proto --bench 20 --synthetic --out ../bench-results/slint.json`.

use proto_core::audio::{Analyzer, Listener};
use proto_core::layout::{self, Divider, LayoutFile, PanelRect};
use proto_core::metrics::{self, BenchArgs, FrameClock};
use proto_core::visual::Visual;
use slint::{ComponentHandle, GraphicsAPI, Model, RenderingState, VecModel};
use slint_interpreter::{Compiler, ComponentInstance, Struct, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime};

const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");

fn workspace_dir() -> PathBuf {
    Path::new(CRATE_DIR).parent().unwrap().to_path_buf()
}

struct State {
    inst: Option<ComponentInstance>,
    layout: LayoutFile,
    layout_user: PathBuf,
    layout_dirty: bool,
    skins: Vec<PathBuf>,
    skin: usize,
    themes: Vec<PathBuf>,
    theme: usize,
    theme_name: String,
    panels: Rc<VecModel<Value>>,
    divs: Rc<VecModel<Value>>,
    queue: Rc<VecModel<Value>>,
    dividers: Vec<Divider>,
    win: (f32, f32),
    scale: f32,
    visual: Option<Visual>,
    visual_fullscreen: bool,
    fullscreen: bool,
    listener: Listener,
    analyzer: Analyzer,
    clock: FrameClock,
    edit: bool,
    last_drag: Option<Instant>,
    hint_until: Option<Instant>,
    bench: BenchArgs,
    bench_started: bool,
    started: Instant,
    last_text: Instant,
    mtimes: HashMap<PathBuf, SystemTime>,
    progress: f32,
    /// Diagnóstico: `--static-ui` não atualiza os números da UI a cada quadro; `--no-visual` não redesenha o shader.
    static_ui: bool,
    /// `--ui-hz N`: atualiza os números da UI no máximo N vezes por segundo (o visual continua a todo quadro).
    ui_hz: Option<f32>,
    ui_accum: f32,
    no_visual: bool,
}

fn num(v: f32) -> Value {
    Value::Number(v as f64)
}
fn text(s: impl Into<slint::SharedString>) -> Value {
    Value::String(s.into())
}
fn field(name: &str, v: Value) -> (String, Value) {
    (name.to_string(), v)
}

fn panel_value(p: &PanelRect) -> Value {
    Value::Struct(Struct::from_iter([
        field("id", text(p.panel.as_str())),
        field("kind", text(p.panel.as_str())),
        field("x", num(p.rect.x)),
        field("y", num(p.rect.y)),
        field("w", num(p.rect.w)),
        field("h", num(p.rect.h)),
        field("floating", Value::Bool(p.floating)),
    ]))
}

fn div_value(d: &Divider) -> Value {
    Value::Struct(Struct::from_iter([
        field("x", num(d.rect.x)),
        field("y", num(d.rect.y)),
        field("w", num(d.rect.w)),
        field("h", num(d.rect.h)),
        field("row", Value::Bool(d.dir == layout::Dir::Row)),
    ]))
}

/// Atualiza um modelo trocando só as linhas que mudaram (assim a skin anima em vez de recriar os elementos).
fn sync_model(model: &VecModel<Value>, rows: Vec<Value>) {
    if model.row_count() != rows.len() {
        model.set_vec(rows);
        return;
    }
    for (i, v) in rows.into_iter().enumerate() {
        if model.row_data(i).as_ref() != Some(&v) {
            model.set_row_data(i, v);
        }
    }
}

fn list_dir(dir: &Path, want_dir: bool, ext: Option<&str>) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    v.retain(|p| if want_dir { p.is_dir() } else { p.extension().and_then(|e| e.to_str()) == ext });
    v.sort();
    v
}

fn mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

impl State {
    fn inst(&self) -> &ComponentInstance {
        self.inst.as_ref().expect("instância")
    }

    fn set(&self, name: &str, v: Value) {
        if let Some(i) = &self.inst {
            let _ = i.set_property(name, v);
        }
    }

    fn skin_file(&self) -> PathBuf {
        self.skins[self.skin].join("app.slint")
    }

    fn relayout(&mut self) {
        let (w, h) = self.win;
        let (mut panels, divs) = self.layout.compute(w, h);
        // Ordem estável por id: o mesmo painel continua sendo a mesma linha do modelo (e anima ao mudar de lugar).
        // Flutuantes por último: são desenhados por cima.
        panels.sort_by(|a, b| a.floating.cmp(&b.floating).then_with(|| a.panel.cmp(&b.panel)));
        sync_model(&self.panels, panels.iter().map(panel_value).collect());
        sync_model(&self.divs, divs.iter().map(div_value).collect());
        self.dividers = divs;
    }

    fn hint(&mut self, msg: String) {
        self.set("hint-text", text(msg));
        self.hint_until = Some(Instant::now() + Duration::from_millis(2500));
    }

    fn edit_hint(&mut self) {
        if self.edit {
            self.set("hint-text", text("Modo edição · arraste cabeçalhos e divisórias · E para sair"));
            self.hint_until = None;
        } else {
            self.set("hint-text", text(""));
        }
    }

    fn apply_theme(&mut self) {
        let Some(inst) = &self.inst else { return };
        let path = &self.themes[self.theme];
        let Ok(src) = std::fs::read_to_string(path) else { return };
        let table: toml::Table = match toml::from_str(&src) {
            Ok(t) => t,
            Err(e) => {
                let _ = inst.set_property("error-text", text(format!("tema {}: {e}", path.display())));
                return;
            }
        };
        for (k, v) in &table {
            let value = match v {
                toml::Value::String(s) if k == "name" => {
                    self.theme_name = s.clone();
                    continue;
                }
                toml::Value::String(s) if s.starts_with('#') => match parse_color(s) {
                    Some(c) => Value::Brush(slint::Brush::SolidColor(c)),
                    None => continue,
                },
                toml::Value::String(s) => text(s.as_str()),
                toml::Value::Integer(n) => Value::Number(*n as f64),
                toml::Value::Float(n) => Value::Number(*n),
                _ => continue,
            };
            // Chaves que a skin não conhece são ignoradas: o tema pode ter mais tokens que a skin usa.
            let _ = inst.set_global_property("Theme", k, value);
        }
    }

    fn save_layout(&mut self) {
        if self.layout_dirty {
            let text = self.layout.to_toml();
            let _ = std::fs::write(&self.layout_user, &text);
            if let Some(t) = mtime(&self.layout_user) {
                self.mtimes.insert(self.layout_user.clone(), t);
            }
            self.layout_dirty = false;
        }
    }

    /// Um quadro: áudio → parâmetros → visual na textura → números da UI.
    fn frame(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let dt = self.clock.tick();
        let params = self.analyzer.update(dt, &self.listener);
        let inst = self.inst().clone_strong();
        let window = inst.window();
        let scale = window.scale_factor();
        let size = window.size().to_logical(scale);
        if (size.width, size.height) != self.win || scale != self.scale {
            self.win = (size.width, size.height);
            self.scale = scale;
            self.relayout();
        }

        let (vw, vh) = if self.visual_fullscreen {
            (self.win.0, self.win.1)
        } else {
            let (p, _) = self.layout.compute(self.win.0, self.win.1);
            p.iter().find(|r| r.panel == "visual").map(|r| (r.rect.w, r.rect.h)).unwrap_or((64.0, 64.0))
        };
        let visual = self.visual.get_or_insert_with(|| Visual::new(device));
        if visual.ensure_size(device, (vw * scale) as u32, (vh * scale) as u32)
            && let Some(tex) = visual.texture()
            && let Ok(img) = slint::Image::try_from(tex.clone())
        {
            let _ = inst.set_property("visual", Value::Image(img));
        }
        if !self.no_visual {
            visual.render(device, queue, &params);
        }
        self.ui_accum += dt;
        let skip_ui = self.ui_hz.is_some_and(|hz| self.ui_accum < 1.0 / hz);
        if self.static_ui || skip_ui {
            return self.frame_tail(&inst);
        }
        self.ui_accum = 0.0;

        for (k, v) in [
            ("bass", params.bass),
            ("mid", params.mid),
            ("high", params.high),
            ("level", params.level),
            ("intensity", params.intensity),
        ] {
            let _ = inst.set_property(k, num(v));
        }
        self.progress = (self.progress + dt / 200.0).fract();
        let _ = inst.set_property("progress", num(self.progress));
        self.frame_tail(&inst);
    }

    fn frame_tail(&mut self, inst: &ComponentInstance) {
        let window = inst.window();

        if self.last_text.elapsed() > Duration::from_millis(500) {
            self.last_text = Instant::now();
            let (ws, _) = metrics::memory_mb();
            let _ = inst.set_property("clock-text", text(metrics::local_clock()));
            let _ = inst.set_property("stats-text", text(format!("{:.0} fps · {:.0} MB", self.clock.fps, ws)));
            if self.hint_until.is_some_and(|t| Instant::now() > t) {
                self.hint_until = None;
                self.edit_hint();
            }
        }

        // Fim de um arraste (divisória ou flutuante): volta a animar e grava o layout.
        if let Some(t) = self.last_drag
            && t.elapsed() > Duration::from_millis(180)
        {
            self.last_drag = None;
            let _ = inst.set_property("dragging", Value::Bool(false));
            self.save_layout();
        }

        if let Some(secs) = self.bench.seconds {
            let el = self.started.elapsed().as_secs_f32();
            if !self.bench_started && el > 2.0 {
                self.bench_started = true;
                self.clock.start_bench();
            }
            if self.bench_started && el > 2.0 + secs {
                let s = window.size();
                let report =
                    self.clock.report("slint 1.18 (interpreter)", "femtovg-wgpu", &self.listener.source, (s.width, s.height));
                report.save(&self.bench.out);
                let _ = slint::quit_event_loop();
            }
        }
    }

    fn on_key(&mut self, key: &str) -> Option<Action> {
        match key {
            "e" | "E" => {
                self.edit = !self.edit;
                self.set("edit-mode", Value::Bool(self.edit));
                self.edit_hint();
            }
            "\u{1b}" if self.edit => {
                self.edit = false;
                self.set("edit-mode", Value::Bool(false));
                self.edit_hint();
            }
            "t" | "T" => {
                self.theme = (self.theme + 1) % self.themes.len();
                self.apply_theme();
                self.hint(format!("Tema: {}", self.theme_name));
            }
            "s" | "S" => {
                self.skin = (self.skin + 1) % self.skins.len();
                return Some(Action::ReloadSkin);
            }
            "r" | "R" => return Some(Action::ReloadSkin),
            "f" | "F" => {
                self.fullscreen = !self.fullscreen;
                self.inst().window().set_fullscreen(self.fullscreen);
            }
            _ => {}
        }
        None
    }
}

enum Action {
    ReloadSkin,
}

fn parse_color(s: &str) -> Option<slint::Color> {
    let h = s.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    match h.len() {
        6 => Some(slint::Color::from_rgb_u8(p(0)?, p(2)?, p(4)?)),
        8 => Some(slint::Color::from_argb_u8(p(6)?, p(0)?, p(2)?, p(4)?)),
        _ => None,
    }
}

fn arg_f32(args: &[Value], i: usize) -> f32 {
    match args.get(i) {
        Some(Value::Number(n)) => *n as f32,
        _ => 0.0,
    }
}
fn arg_str(args: &[Value], i: usize) -> String {
    match args.get(i) {
        Some(Value::String(s)) => s.to_string(),
        _ => String::new(),
    }
}

/// Compila a skin atual e troca a janela. Se a skin tiver erro, a janela antiga continua e mostra o erro.
fn load_skin(state: &Rc<RefCell<State>>) {
    let path = state.borrow().skin_file();
    let compiler = Compiler::new();
    let result = spin_on::spin_on(compiler.build_from_path(&path));
    let errors: Vec<String> = result
        .diagnostics()
        .filter(|d| d.level() == slint_interpreter::DiagnosticLevel::Error)
        .map(|d| {
            let (line, col) = d.line_column();
            format!("{}:{line}:{col}: {}", path.file_name().unwrap().to_string_lossy(), d.message())
        })
        .collect();
    let created = if errors.is_empty() {
        result.component("App").ok_or_else(|| "a skin precisa exportar `component App`".to_string()).and_then(|d| d.create().map_err(|e| e.to_string()))
    } else {
        Err(errors.join("\n"))
    };
    let new = match created {
        Ok(i) => i,
        Err(e) => {
            eprintln!("skin com erro:\n{e}");
            let s = state.borrow();
            match &s.inst {
                Some(_) => s.set("error-text", text(format!("Skin com erro (a anterior continua ativa):\n{e}"))),
                None => {
                    // Primeira skin não compila: não há o que mostrar.
                    std::process::exit(1);
                }
            }
            return;
        }
    };
    if let Some(t) = mtime(&path) {
        state.borrow_mut().mtimes.insert(path.clone(), t);
    }
    wire(state, &new);

    let old = state.borrow_mut().inst.replace(new.clone_strong());
    {
        let mut s = state.borrow_mut();
        s.visual = None;
        s.visual_fullscreen = matches!(new.get_property("visual-fullscreen"), Ok(Value::Bool(true)));
        let (p, d, q) = (s.panels.clone(), s.divs.clone(), s.queue.clone());
        let _ = new.set_property("panels", Value::Model(p.into()));
        let _ = new.set_property("dividers", Value::Model(d.into()));
        let _ = new.set_property("queue", Value::Model(q.into()));
        let _ = new.set_property("edit-mode", Value::Bool(s.edit));
        let now_title = if s.listener.is_synthetic() { "Sinal sintético (120 BPM)" } else { "Modo Escuta" };
        let _ = new.set_property("now-title", text(now_title));
        let _ = new.set_property("now-artist", text(s.listener.source.as_str()));
        s.apply_theme();
        let name = s.skins[s.skin].file_name().unwrap().to_string_lossy().to_string();
        let msg = format!("Skin: {name} · Tema: {}", s.theme_name);
        s.hint(msg);
    }
    if let Some(old) = old {
        new.window().set_size(old.window().size());
        new.window().set_position(old.window().position());
        new.window().set_fullscreen(state.borrow().fullscreen);
        new.show().ok();
        old.hide().ok();
    } else {
        if state.borrow().bench.seconds.is_some() {
            new.window().set_size(slint::LogicalSize::new(1280.0, 800.0));
        }
        new.show().ok();
    }
}

fn wire(state: &Rc<RefCell<State>>, inst: &ComponentInstance) {
    let weak = Rc::downgrade(state);
    inst.window().set_rendering_notifier(move |rs, api| {
        let Some(state) = weak.upgrade() else { return };
        match (rs, api) {
            (RenderingState::BeforeRendering, GraphicsAPI::WGPU30 { device, queue, .. }) => {
                if let Ok(mut s) = state.try_borrow_mut() {
                    s.frame(device, queue);
                }
            }
            (RenderingState::AfterRendering, _) => {
                if let Ok(s) = state.try_borrow() {
                    // Redesenho contínuo: o visual anima a todo quadro.
                    s.inst().window().request_redraw();
                }
            }
            _ => {}
        }
    })
    .expect("o renderer precisa ser wgpu");

    let st = Rc::downgrade(state);
    inst.set_callback("divider-drag", move |a| {
        if let Some(st) = st.upgrade() {
            let mut s = st.borrow_mut();
            let i = arg_f32(a, 0) as usize;
            if let Some(d) = s.dividers.get(i).cloned() {
                let w = s.win.0;
                s.layout.drag_divider(w, &d, arg_f32(a, 1), arg_f32(a, 2));
                s.layout_dirty = true;
                s.last_drag = Some(Instant::now());
                s.set("dragging", Value::Bool(true));
                s.relayout();
            }
        }
        Value::Void
    })
    .ok();
    inst.set_callback("divider-released", |_| Value::Void).ok();

    let st = Rc::downgrade(state);
    inst.set_callback("panel-drag", move |a| {
        if let Some(st) = st.upgrade() {
            let s = st.borrow();
            let (id, x, y) = (arg_str(a, 0), arg_f32(a, 1), arg_f32(a, 2));
            let (rects, _) = s.layout.compute(s.win.0, s.win.1);
            match rects.iter().find(|r| !r.floating && r.panel != id && r.rect.contains(x, y)) {
                Some(t) => {
                    let (_, r) = layout::drop_preview(&t.rect, x, y);
                    s.set("preview-visible", Value::Bool(true));
                    for (k, v) in [("preview-x", r.x), ("preview-y", r.y), ("preview-w", r.w), ("preview-h", r.h)] {
                        s.set(k, num(v));
                    }
                }
                None => s.set("preview-visible", Value::Bool(false)),
            }
        }
        Value::Void
    })
    .ok();

    let st = Rc::downgrade(state);
    inst.set_callback("panel-drop", move |a| {
        if let Some(st) = st.upgrade() {
            let mut s = st.borrow_mut();
            s.set("preview-visible", Value::Bool(false));
            let (w, h) = s.win;
            if s.layout.drop_panel(w, h, &arg_str(a, 0), arg_f32(a, 1), arg_f32(a, 2)) {
                s.layout_dirty = true;
                s.relayout();
                s.save_layout();
            }
        }
        Value::Void
    })
    .ok();

    let st = Rc::downgrade(state);
    inst.set_callback("floating-move", move |a| {
        if let Some(st) = st.upgrade() {
            let mut s = st.borrow_mut();
            let (w, h) = s.win;
            let id = arg_str(a, 0);
            s.layout.move_floating(&id, arg_f32(a, 1), arg_f32(a, 2), arg_f32(a, 3), arg_f32(a, 4), w, h);
            s.layout_dirty = true;
            s.last_drag = Some(Instant::now());
            s.set("dragging", Value::Bool(true));
            s.relayout();
        }
        Value::Void
    })
    .ok();

    let st = Rc::downgrade(state);
    inst.set_callback("key", move |a| {
        let Some(st) = st.upgrade() else { return Value::Void };
        let action = st.borrow_mut().on_key(&arg_str(a, 0));
        if let Some(Action::ReloadSkin) = action {
            // Fora do callback: trocar de janela aqui dentro mexeria na instância que está tratando o evento.
            let st = Rc::downgrade(&st);
            slint::Timer::single_shot(Duration::ZERO, move || {
                if let Some(st) = st.upgrade() {
                    load_skin(&st);
                }
            });
        }
        Value::Void
    })
    .ok();

    inst.window().on_close_requested(|| {
        let _ = slint::quit_event_loop();
        slint::CloseRequestResponse::HideWindow
    });
}

/// Recarga ao vivo: confere a data dos arquivos a cada 300 ms (skin, tema e layout).
fn watch(state: &Rc<RefCell<State>>) -> slint::Timer {
    let timer = slint::Timer::default();
    let weak = Rc::downgrade(state);
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(300), move || {
        let Some(st) = weak.upgrade() else { return };
        let (skin, theme, layout) = {
            let s = st.borrow();
            (s.skin_file(), s.themes[s.theme].clone(), s.layout_user.clone())
        };
        let changed = |p: &Path| {
            let now = mtime(p);
            let mut s = st.borrow_mut();
            let old = s.mtimes.get(p).copied();
            match now {
                Some(t) if old != Some(t) => {
                    s.mtimes.insert(p.to_path_buf(), t);
                    old.is_some()
                }
                _ => false,
            }
        };
        if changed(&skin) {
            load_skin(&st);
        }
        if changed(&theme) {
            st.borrow_mut().apply_theme();
        }
        if changed(&layout) {
            let mut s = st.borrow_mut();
            match std::fs::read_to_string(&layout).map_err(|e| e.to_string()).and_then(|t| LayoutFile::parse(&t)) {
                Ok(l) => {
                    s.layout = l;
                    s.relayout();
                    s.set("error-text", text(""));
                }
                Err(e) => s.set("error-text", text(format!("layout.user.toml: {e}"))),
            }
        }
    });
    timer
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let bench = BenchArgs::parse("bench-results/slint.json");
    slint::BackendSelector::new().require_wgpu_30(slint::wgpu_30::WGPUConfiguration::default()).select()?;

    let ws = workspace_dir();
    let layout_user = ws.join("layout.user.toml");
    let layout_text = std::fs::read_to_string(&layout_user).unwrap_or_else(|_| proto_core::DEFAULT_LAYOUT.to_string());
    let layout = LayoutFile::parse(&layout_text).or_else(|_| LayoutFile::parse(proto_core::DEFAULT_LAYOUT))?;
    if !layout_user.exists() {
        std::fs::write(&layout_user, layout.to_toml())?;
    }
    let skins = list_dir(&Path::new(CRATE_DIR).join("skins"), true, None);
    let themes = list_dir(&ws.join("themes"), false, Some("toml"));
    let queue: Vec<Value> = [
        ("Faixa local — exemplo.mp3", "arquivo", true),
        ("Qualquer app tocando", "Modo Escuta", false),
        ("Audius — faixa", "módulo", false),
        ("Jamendo — faixa", "módulo", false),
    ]
    .iter()
    .map(|(t, a, p)| {
        Value::Struct(Struct::from_iter([field("title", text(*t)), field("artist", text(*a)), field("playing", Value::Bool(*p))]))
    })
    .collect();

    let mut mtimes = HashMap::new();
    for p in themes.iter().chain(std::iter::once(&layout_user)) {
        if let Some(t) = mtime(p) {
            mtimes.insert(p.clone(), t);
        }
    }
    let synthetic = bench.synthetic;
    let state = Rc::new(RefCell::new(State {
        inst: None,
        layout,
        layout_user,
        layout_dirty: false,
        skin: skins.iter().position(|p| p.ends_with("default")).unwrap_or(0),
        skins,
        theme: 0,
        themes,
        theme_name: String::new(),
        panels: Rc::new(VecModel::default()),
        divs: Rc::new(VecModel::default()),
        queue: Rc::new(VecModel::from(queue)),
        dividers: Vec::new(),
        win: (0.0, 0.0),
        scale: 1.0,
        visual: None,
        visual_fullscreen: false,
        fullscreen: false,
        listener: Listener::start(synthetic),
        analyzer: Analyzer::default(),
        clock: FrameClock::default(),
        edit: false,
        last_drag: None,
        hint_until: None,
        bench,
        bench_started: false,
        started: Instant::now(),
        last_text: Instant::now(),
        mtimes,
        progress: 0.0,
        static_ui: std::env::args().any(|a| a == "--static-ui"),
        no_visual: std::env::args().any(|a| a == "--no-visual"),
        ui_hz: {
            let a: Vec<String> = std::env::args().collect();
            a.iter().position(|x| x == "--ui-hz").and_then(|i| a.get(i + 1)).and_then(|v| v.parse().ok())
        },
        ui_accum: 0.0,
    }));
    load_skin(&state);
    let _watch = watch(&state);
    slint::run_event_loop_until_quit()?;
    let mut s = state.borrow_mut();
    s.save_layout();
    Ok(())
}
