//! Skins em dados para o GPUI: um arquivo `.kdl` descreve a moldura dos painéis e o conteúdo de cada um
//! (linhas, colunas, textos, barras, listas), com tokens de tema e valores ao vivo (`"{bass}"`).
//!
//! O GPUI reconstrói a árvore de elementos a cada quadro a partir de dados; aqui esses dados vêm de um arquivo que
//! o usuário edita, e o app recarrega ao salvar. É a mesma ideia das skins do Slint e do QML do Quickshell, sem
//! precisar recompilar o app.
//!
//! Elementos: `column`, `row`, `box`, `text "…"`, `progress "{valor}"`, `bar "{valor}"`, `each "lista" {…}`, `spacer`.
//! Estilos: `w`/`h` (número, `fill` ou `50%`), `grow`, `gap`, `padding`, `padding-x/y/top/bottom/left/right`,
//! `align`, `justify`, `bg` (token, `#hex`, `token/0.5` ou `gradient(a, b)`), `radius`, `border`, `border-width`,
//! `opacity`, `shadow`, `size`, `weight`, `color`, `font`. Um nó com `active="{item.playing}"` usa as propriedades
//! `active-*` quando o valor é verdadeiro.

use gpui::{AnyElement, Div, FontWeight, Hsla, IntoElement, ParentElement, SharedString, Styled, div, px, relative};
use std::collections::HashMap;

pub type Props = HashMap<String, String>;

#[derive(Clone, Debug, Default)]
pub struct Node {
    pub kind: String,
    pub arg: Option<String>,
    pub props: Props,
    pub children: Vec<Node>,
}

#[derive(Clone, Debug, Default)]
pub struct PanelSkin {
    pub title: String,
    pub body: Vec<Node>,
}

#[derive(Clone, Debug, Default)]
pub struct Skin {
    pub app: Props,
    pub frame: Props,
    pub header: Props,
    pub panels: HashMap<String, PanelSkin>,
}

fn value_str(v: &kdl::KdlValue) -> String {
    match v {
        kdl::KdlValue::String(s) => s.clone(),
        kdl::KdlValue::Integer(i) => i.to_string(),
        kdl::KdlValue::Float(f) => f.to_string(),
        kdl::KdlValue::Bool(b) => b.to_string(),
        kdl::KdlValue::Null => String::new(),
    }
}

fn node_from(n: &kdl::KdlNode) -> Node {
    let mut out = Node { kind: n.name().value().to_string(), ..Default::default() };
    for e in n.entries() {
        match e.name() {
            Some(k) => {
                out.props.insert(k.value().to_string(), value_str(e.value()));
            }
            None if out.arg.is_none() => out.arg = Some(value_str(e.value())),
            None => {}
        }
    }
    if let Some(ch) = n.children() {
        out.children = ch.nodes().iter().map(node_from).collect();
    }
    out
}

impl Skin {
    pub fn parse(text: &str) -> Result<Skin, String> {
        let doc: kdl::KdlDocument = text.parse().map_err(|e: kdl::KdlError| {
            match e.diagnostics.first() {
                Some(d) => {
                    // Posição em linha:coluna (1-based), mais útil que o deslocamento em bytes.
                    let off = d.span.offset().min(text.len());
                    let before = &text[..off];
                    let line = before.matches('\n').count() + 1;
                    let col = before.rsplit('\n').next().map(|l| l.chars().count()).unwrap_or(0) + 1;
                    format!("linha {line}, coluna {col}: {}", d.message.clone().unwrap_or_default())
                }
                None => e.to_string(),
            }
        })?;
        let mut skin = Skin::default();
        for n in doc.nodes() {
            let node = node_from(n);
            match node.kind.as_str() {
                "app" => skin.app = node.props,
                "frame" => skin.frame = node.props,
                "header" => skin.header = node.props,
                "panel" => {
                    let id = node.arg.clone().ok_or("`panel` precisa de um nome: panel \"player\" { … }")?;
                    let title = node.props.get("title").cloned().unwrap_or_default();
                    skin.panels.insert(id, PanelSkin { title, body: node.children });
                }
                other => return Err(format!("nó desconhecido no topo: `{other}` (use app, frame, header ou panel)")),
            }
        }
        Ok(skin)
    }
}

/// Valores ao vivo que a skin pode ler: `"{bass}"`, `"{now.title}"`, `each "queue"` → `"{item.title}"`.
#[derive(Clone, Debug)]
pub enum Val {
    S(String),
    N(f32),
    B(bool),
    L(Vec<HashMap<String, Val>>),
}

pub struct Ctx<'a> {
    pub vars: &'a HashMap<String, Val>,
    pub item: Option<&'a HashMap<String, Val>>,
}

impl Ctx<'_> {
    fn get(&self, key: &str) -> Option<&Val> {
        match key.strip_prefix("item.") {
            Some(k) => self.item.and_then(|i| i.get(k)),
            None => self.vars.get(key),
        }
    }
    fn binding(s: &str) -> Option<&str> {
        s.strip_prefix('{').and_then(|r| r.strip_suffix('}'))
    }
    /// Texto com `{chave}` trocadas pelos valores.
    pub fn text(&self, s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(i) = rest.find('{') {
            out.push_str(&rest[..i]);
            match rest[i..].find('}') {
                Some(j) => {
                    let key = &rest[i + 1..i + j];
                    match self.get(key) {
                        Some(Val::S(v)) => out.push_str(v),
                        Some(Val::N(v)) => out.push_str(&format!("{v:.2}")),
                        Some(Val::B(v)) => out.push_str(if *v { "sim" } else { "não" }),
                        _ => {}
                    }
                    rest = &rest[i + j + 1..];
                }
                None => {
                    out.push_str(&rest[i..]);
                    rest = "";
                }
            }
        }
        out.push_str(rest);
        out
    }
    pub fn num(&self, s: &str) -> f32 {
        match Self::binding(s).and_then(|k| self.get(k)) {
            Some(Val::N(v)) => *v,
            Some(Val::B(b)) => *b as u8 as f32,
            _ => s.parse().unwrap_or(0.0),
        }
    }
    pub fn truthy(&self, s: &str) -> bool {
        match Self::binding(s).map(|k| self.get(k)).unwrap_or_else(|| self.get(s)) {
            Some(Val::B(b)) => *b,
            Some(Val::N(n)) => *n != 0.0,
            Some(Val::S(v)) => !v.is_empty(),
            Some(Val::L(l)) => !l.is_empty(),
            None => s == "true",
        }
    }
}

/// O que a skin precisa do tema: cores por nome (token) e alguns números.
pub trait Tokens {
    fn color(&self, token: &str) -> Option<Hsla>;
    fn radius(&self) -> f32;
    fn border_width(&self) -> f32;
    fn font(&self) -> SharedString;
}

pub fn parse_hex(s: &str) -> Option<Hsla> {
    let h = s.trim_start_matches('#');
    let p = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok().map(|v| v as f32 / 255.0);
    let a = if h.len() == 8 { p(6)? } else { 1.0 };
    Some(gpui::Rgba { r: p(0)?, g: p(2)?, b: p(4)?, a }.into())
}

/// `accent`, `#ff0055`, `surface/0.6` (com transparência) ou `none`.
pub fn color(t: &dyn Tokens, s: &str) -> Option<Hsla> {
    let s = s.trim();
    if s == "none" || s.is_empty() {
        return None;
    }
    let (base, alpha) = match s.split_once('/') {
        Some((b, a)) => (b.trim(), a.trim().parse::<f32>().ok()),
        None => (s, None),
    };
    let c = if base.starts_with('#') { parse_hex(base) } else { t.color(base) }?;
    Some(match alpha {
        Some(a) => c.opacity(a),
        None => c,
    })
}

fn weight(s: &str) -> FontWeight {
    match s {
        "thin" => FontWeight::THIN,
        "light" => FontWeight::LIGHT,
        "medium" => FontWeight::MEDIUM,
        "semibold" => FontWeight::SEMIBOLD,
        "bold" => FontWeight::BOLD,
        "black" => FontWeight::BLACK,
        _ => FontWeight::NORMAL,
    }
}

/// Tamanho em px; aceita `36%h` / `20%w` relativo ao painel.
fn size_px(s: &str, ctx: &Ctx, panel: (f32, f32)) -> f32 {
    if let Some(v) = s.strip_suffix("%h") {
        return v.parse::<f32>().unwrap_or(0.0) / 100.0 * panel.1;
    }
    if let Some(v) = s.strip_suffix("%w") {
        return v.parse::<f32>().unwrap_or(0.0) / 100.0 * panel.0;
    }
    ctx.num(s)
}

/// Propriedades efetivas: as `active-*` vencem quando `active` é verdadeiro.
fn effective(n: &Node, ctx: &Ctx) -> Props {
    let mut p = n.props.clone();
    if let Some(a) = n.props.get("active")
        && ctx.truthy(a)
    {
        for (k, v) in &n.props {
            if let Some(base) = k.strip_prefix("active-") {
                p.insert(base.to_string(), v.clone());
            }
        }
    }
    p
}

pub fn apply_box(mut d: Div, p: &Props, t: &dyn Tokens, ctx: &Ctx, panel: (f32, f32)) -> Div {
    let len = |v: &str| -> Option<gpui::DefiniteLength> {
        if v == "fill" {
            Some(relative(1.0))
        } else if let Some(pc) = v.strip_suffix('%') {
            pc.parse::<f32>().ok().map(|x| relative(x / 100.0))
        } else {
            Some(px(size_px(v, ctx, panel)).into())
        }
    };
    if let Some(v) = p.get("w").and_then(|v| len(v)) {
        d = d.w(v);
    }
    if let Some(v) = p.get("h").and_then(|v| len(v)) {
        d = d.h(v);
    }
    if p.get("grow").is_some_and(|v| v == "true") {
        d = d.flex_1();
    }
    let n = |k: &str| p.get(k).map(|v| size_px(v, ctx, panel));
    if let Some(v) = n("padding") {
        d = d.p(px(v));
    }
    if let Some(v) = n("padding-x") {
        d = d.px(px(v));
    }
    if let Some(v) = n("padding-y") {
        d = d.py(px(v));
    }
    if let Some(v) = n("padding-top") {
        d = d.pt(px(v));
    }
    if let Some(v) = n("padding-bottom") {
        d = d.pb(px(v));
    }
    if let Some(v) = n("padding-left") {
        d = d.pl(px(v));
    }
    if let Some(v) = n("padding-right") {
        d = d.pr(px(v));
    }
    if let Some(v) = n("gap") {
        d = d.gap(px(v));
    }
    match p.get("align").map(String::as_str) {
        Some("center") => d = d.items_center(),
        Some("end") => d = d.items_end(),
        Some("start") => d = d.items_start(),
        _ => {}
    }
    match p.get("justify").map(String::as_str) {
        Some("center") => d = d.justify_center(),
        Some("end") => d = d.justify_end(),
        Some("between") => d = d.justify_between(),
        _ => {}
    }
    if let Some(bg) = p.get("bg") {
        if let Some(inner) = bg.strip_prefix("gradient(").and_then(|r| r.strip_suffix(')')) {
            let mut it = inner.split(',');
            if let (Some(a), Some(b)) = (it.next().and_then(|a| color(t, a)), it.next().and_then(|b| color(t, b))) {
                d = d.bg(gpui::linear_gradient(135.0, gpui::linear_color_stop(a, 0.0), gpui::linear_color_stop(b, 1.0)));
            }
        } else if let Some(c) = color(t, bg) {
            d = d.bg(c);
        }
    }
    if let Some(r) = p.get("radius") {
        let r = if r == "theme" { t.radius() } else { size_px(r, ctx, panel) };
        d = d.rounded(px(r)).overflow_hidden();
    }
    if let Some(c) = p.get("border").and_then(|c| color(t, c)) {
        let w = p.get("border-width").map(|w| if w == "theme" { t.border_width() } else { ctx.num(w) }).unwrap_or(1.0);
        d = d.border(px(w)).border_color(c);
    }
    if let Some(o) = p.get("opacity") {
        d = d.opacity(ctx.num(o));
    }
    if p.get("shadow").is_some_and(|v| v == "true") {
        d = d.shadow_lg();
    }
    if let Some(c) = p.get("color").and_then(|c| color(t, c)) {
        d = d.text_color(c);
    }
    if let Some(s) = p.get("size") {
        d = d.text_size(px(size_px(s, ctx, panel)));
    }
    if let Some(w) = p.get("weight") {
        d = d.font_weight(weight(w));
    }
    if let Some(f) = p.get("font") {
        d = d.font_family(if f == "theme" { t.font() } else { SharedString::from(f.clone()) });
    }
    d
}

pub fn build(nodes: &[Node], t: &dyn Tokens, ctx: &Ctx, panel: (f32, f32)) -> Vec<AnyElement> {
    let mut out = Vec::new();
    for n in nodes {
        if let Some(cond) = n.props.get("when")
            && !ctx.truthy(cond)
        {
            continue;
        }
        if let Some(cond) = n.props.get("unless")
            && ctx.truthy(cond)
        {
            continue;
        }
        let p = effective(n, ctx);
        match n.kind.as_str() {
            "column" | "row" | "box" => {
                let mut d = div().flex();
                d = if n.kind == "row" { d.flex_row() } else { d.flex_col() };
                d = apply_box(d, &p, t, ctx, panel);
                out.push(d.children(build(&n.children, t, ctx, panel)).into_any_element());
            }
            "spacer" => out.push(div().flex_1().into_any_element()),
            "text" => {
                let s = ctx.text(n.arg.as_deref().unwrap_or(""));
                let s = if p.get("transform").is_some_and(|v| v == "upper") { s.to_uppercase() } else { s };
                let mut d = apply_box(div(), &p, t, ctx, panel);
                if p.get("text-align").is_some_and(|v| v == "center") {
                    d = d.flex().justify_center();
                }
                out.push(d.child(s).into_any_element());
            }
            "progress" => {
                let v = ctx.num(n.arg.as_deref().unwrap_or("0")).clamp(0.0, 1.0);
                let mut track = p.clone();
                track.remove("fill");
                track.insert("bg".into(), p.get("track").cloned().unwrap_or("surface-2".into()));
                track.entry("w".into()).or_insert("fill".into());
                let fill = color(t, p.get("fill").map(String::as_str).unwrap_or("accent"));
                let r = p.get("radius").map(|r| ctx.num(r)).unwrap_or(2.0);
                let mut bar = div().h_full().w(relative(v)).rounded(px(r));
                if let Some(c) = fill {
                    bar = bar.bg(c);
                }
                out.push(apply_box(div(), &track, t, ctx, panel).child(bar).into_any_element());
            }
            "bar" => {
                let v = ctx.num(n.arg.as_deref().unwrap_or("0")).clamp(0.0, 1.0);
                let fill = color(t, p.get("fill").map(String::as_str).unwrap_or("accent"));
                let track = color(t, p.get("track").map(String::as_str).unwrap_or("surface-2"));
                let r = p.get("radius").map(|r| ctx.num(r)).unwrap_or(6.0);
                let mut col = div().flex_1().h_full().flex().flex_col().gap(px(4.0));
                let mut tr = div().flex_1().w_full().rounded(px(r)).flex().flex_col().justify_end();
                if let Some(c) = track {
                    tr = tr.bg(c);
                }
                let mut fl = div().w_full().h(relative(v)).rounded(px(r));
                if let Some(c) = fill {
                    fl = fl.bg(c);
                }
                col = col.child(tr.child(fl));
                if let Some(label) = p.get("label") {
                    let mut lab = div().flex().justify_center().text_size(px(11.0)).child(ctx.text(label));
                    if let Some(c) = color(t, p.get("label-color").map(String::as_str).unwrap_or("muted")) {
                        lab = lab.text_color(c);
                    }
                    col = col.child(lab);
                }
                out.push(col.into_any_element());
            }
            "each" => {
                let key = n.arg.clone().unwrap_or_default();
                if let Some(Val::L(items)) = ctx.vars.get(&key) {
                    for item in items {
                        let c2 = Ctx { vars: ctx.vars, item: Some(item) };
                        out.extend(build(&n.children, t, &c2, panel));
                    }
                }
            }
            other => {
                out.push(div().text_color(gpui::red()).child(format!("skin: elemento desconhecido `{other}`")).into_any_element());
            }
        }
    }
    out
}
