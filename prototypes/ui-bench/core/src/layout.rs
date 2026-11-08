//! Layout em árvore, no estilo "dwindle" do Hyprland.
//!
//! A tela é uma árvore: cada nó ou é um painel (folha) ou uma divisão em duas partes (lado a lado ou empilhadas)
//! com uma proporção. Arrastar a divisória muda a proporção; soltar um painel na borda de outro cria uma nova
//! divisão. Cada faixa de largura de janela (breakpoint) tem a sua própria árvore: é isso que deixa o layout
//! responsivo. Widgets flutuantes ficam numa camada por cima, em coordenadas relativas (0-1).
//!
//! Este módulo não sabe nada de UI: devolve retângulos, e qualquer framework (Slint, GPUI…) desenha.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Dir {
    /// Lado a lado (a à esquerda, b à direita).
    Row,
    /// Empilhado (a em cima, b embaixo).
    Column,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Node {
    Leaf { panel: String },
    Split { split: Dir, ratio: f32, a: Box<Node>, b: Box<Node> },
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Floating {
    pub panel: String,
    /// Posição e tamanho relativos à janela (0-1).
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Breakpoint {
    /// A árvore vale a partir desta largura de janela (px lógicos).
    pub min_width: f32,
    pub root: Node,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LayoutFile {
    /// Espaço entre painéis e entre os painéis e a borda da janela.
    #[serde(default = "default_gap_in")]
    pub gaps_in: f32,
    #[serde(default = "default_gap_out")]
    pub gaps_out: f32,
    #[serde(rename = "breakpoint")]
    pub breakpoints: Vec<Breakpoint>,
    #[serde(default)]
    pub floating: Vec<Floating>,
}

fn default_gap_in() -> f32 {
    6.0
}
fn default_gap_out() -> f32 {
    10.0
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub fn contains(&self, px: f32, py: f32) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelRect {
    pub panel: String,
    pub rect: Rect,
    pub floating: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Divider {
    /// Caminho até a divisão na árvore ('a'/'b' a cada nível).
    pub path: String,
    pub dir: Dir,
    /// Área clicável da divisória.
    pub rect: Rect,
    /// Retângulo da divisão inteira: a proporção é calculada sobre ele.
    pub parent: Rect,
}

/// Onde um painel foi solto em cima de outro.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
    /// No meio: os dois trocam de lugar.
    Center,
}

pub const MIN_RATIO: f32 = 0.12;

impl LayoutFile {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut f: LayoutFile = toml::from_str(text).map_err(|e| e.to_string())?;
        if f.breakpoints.is_empty() {
            return Err("o layout precisa de pelo menos um [[breakpoint]]".into());
        }
        f.breakpoints.sort_by(|a, b| a.min_width.total_cmp(&b.min_width));
        Ok(f)
    }

    pub fn to_toml(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_default()
    }

    /// Índice da árvore que vale para esta largura.
    pub fn active(&self, width: f32) -> usize {
        let mut idx = 0;
        for (i, b) in self.breakpoints.iter().enumerate() {
            if width >= b.min_width {
                idx = i;
            }
        }
        idx
    }

    /// Retângulos dos painéis e das divisórias para uma janela de `w` × `h`.
    pub fn compute(&self, w: f32, h: f32) -> (Vec<PanelRect>, Vec<Divider>) {
        let bp = &self.breakpoints[self.active(w)];
        let go = self.gaps_out;
        let area = Rect { x: go, y: go, w: (w - 2.0 * go).max(1.0), h: (h - 2.0 * go).max(1.0) };
        let mut panels = Vec::new();
        let mut dividers = Vec::new();
        walk(&bp.root, area, self.gaps_in, &mut String::new(), &mut panels, &mut dividers);
        for f in &self.floating {
            // Um painel que já está na árvore não aparece duas vezes.
            if panels.iter().any(|p| p.panel == f.panel) {
                continue;
            }
            panels.push(PanelRect {
                panel: f.panel.clone(),
                rect: Rect { x: f.x * w, y: f.y * h, w: f.w * w, h: f.h * h },
                floating: true,
            });
        }
        (panels, dividers)
    }

    /// Arrastar uma divisória até o ponto (px, py) da janela.
    pub fn drag_divider(&mut self, width: f32, div: &Divider, px: f32, py: f32) {
        let ratio = match div.dir {
            Dir::Row => (px - div.parent.x) / div.parent.w.max(1.0),
            Dir::Column => (py - div.parent.y) / div.parent.h.max(1.0),
        }
        .clamp(MIN_RATIO, 1.0 - MIN_RATIO);
        let idx = self.active(width);
        if let Some(Node::Split { ratio: r, .. }) = node_at_mut(&mut self.breakpoints[idx].root, &div.path) {
            *r = ratio;
        }
    }

    /// Soltar o painel `panel` no ponto (px, py). Devolve true se o layout mudou.
    pub fn drop_panel(&mut self, width: f32, height: f32, panel: &str, px: f32, py: f32) -> bool {
        let (rects, _) = self.compute(width, height);
        let Some(target) = rects.iter().find(|r| !r.floating && r.panel != panel && r.rect.contains(px, py)) else {
            return false;
        };
        let edge = edge_at(&target.rect, px, py);
        let target_id = target.panel.clone();
        let idx = self.active(width);
        let in_tree = contains_panel(&self.breakpoints[idx].root, panel);
        // Um painel flutuante solto na árvore passa a ser um painel da árvore.
        let was_floating = if in_tree {
            None
        } else {
            let pos = self.floating.iter().position(|f| f.panel == panel);
            pos.map(|i| self.floating.remove(i))
        };
        let root = &mut self.breakpoints[idx].root;
        if edge == Edge::Center {
            if in_tree {
                swap_panels(root, panel, &target_id);
            } else {
                // Troca de camada: o flutuante entra na árvore e o alvo vira flutuante no lugar dele.
                rename_panel(root, &target_id, panel);
                if let Some(mut f) = was_floating {
                    f.panel = target_id;
                    self.floating.push(f);
                }
            }
            return true;
        }
        remove_panel(root, panel);
        let new_leaf = Node::Leaf { panel: panel.to_string() };
        insert_at_panel(root, &target_id, new_leaf, edge);
        true
    }

    /// Mover um widget flutuante (coordenadas absolutas da janela).
    pub fn move_floating(&mut self, panel: &str, x: f32, y: f32, w: f32, h: f32, win_w: f32, win_h: f32) {
        if let Some(f) = self.floating.iter_mut().find(|f| f.panel == panel) {
            f.w = (w / win_w).clamp(0.05, 1.0);
            f.h = (h / win_h).clamp(0.05, 1.0);
            f.x = (x / win_w).clamp(0.0, 1.0 - f.w);
            f.y = (y / win_h).clamp(0.0, 1.0 - f.h);
        }
    }
}

fn walk(node: &Node, r: Rect, gap: f32, path: &mut String, panels: &mut Vec<PanelRect>, divs: &mut Vec<Divider>) {
    match node {
        Node::Leaf { panel } => panels.push(PanelRect { panel: panel.clone(), rect: r, floating: false }),
        Node::Split { split, ratio, a, b } => {
            let ratio = ratio.clamp(MIN_RATIO, 1.0 - MIN_RATIO);
            let (ra, rb, rd) = match split {
                Dir::Row => {
                    let wa = (r.w - gap) * ratio;
                    (
                        Rect { x: r.x, y: r.y, w: wa, h: r.h },
                        Rect { x: r.x + wa + gap, y: r.y, w: r.w - wa - gap, h: r.h },
                        Rect { x: r.x + wa - 3.0, y: r.y, w: gap + 6.0, h: r.h },
                    )
                }
                Dir::Column => {
                    let ha = (r.h - gap) * ratio;
                    (
                        Rect { x: r.x, y: r.y, w: r.w, h: ha },
                        Rect { x: r.x, y: r.y + ha + gap, w: r.w, h: r.h - ha - gap },
                        Rect { x: r.x, y: r.y + ha - 3.0, w: r.w, h: gap + 6.0 },
                    )
                }
            };
            divs.push(Divider { path: path.clone(), dir: *split, rect: rd, parent: r });
            path.push('a');
            walk(a, ra, gap, path, panels, divs);
            path.pop();
            path.push('b');
            walk(b, rb, gap, path, panels, divs);
            path.pop();
        }
    }
}

fn edge_at(r: &Rect, px: f32, py: f32) -> Edge {
    let u = (px - r.x) / r.w.max(1.0);
    let v = (py - r.y) / r.h.max(1.0);
    if (0.3..0.7).contains(&u) && (0.3..0.7).contains(&v) {
        return Edge::Center;
    }
    // A borda mais próxima vence.
    let d = [(u, Edge::Left), (1.0 - u, Edge::Right), (v, Edge::Top), (1.0 - v, Edge::Bottom)];
    d.iter().min_by(|a, b| a.0.total_cmp(&b.0)).map(|x| x.1).unwrap_or(Edge::Center)
}

/// Retângulo de prévia (meia área do alvo) para mostrar onde o painel vai cair.
pub fn drop_preview(target: &Rect, px: f32, py: f32) -> (Edge, Rect) {
    let e = edge_at(target, px, py);
    let t = *target;
    let r = match e {
        Edge::Left => Rect { w: t.w / 2.0, ..t },
        Edge::Right => Rect { x: t.x + t.w / 2.0, w: t.w / 2.0, ..t },
        Edge::Top => Rect { h: t.h / 2.0, ..t },
        Edge::Bottom => Rect { y: t.y + t.h / 2.0, h: t.h / 2.0, ..t },
        Edge::Center => t,
    };
    (e, r)
}

fn node_at_mut<'a>(node: &'a mut Node, path: &str) -> Option<&'a mut Node> {
    let mut cur = node;
    for c in path.chars() {
        cur = match cur {
            Node::Split { a, b, .. } => {
                if c == 'a' {
                    a.as_mut()
                } else {
                    b.as_mut()
                }
            }
            Node::Leaf { .. } => return None,
        };
    }
    Some(cur)
}

fn contains_panel(node: &Node, id: &str) -> bool {
    match node {
        Node::Leaf { panel } => panel == id,
        Node::Split { a, b, .. } => contains_panel(a, id) || contains_panel(b, id),
    }
}

fn swap_panels(node: &mut Node, x: &str, y: &str) {
    match node {
        Node::Leaf { panel } => {
            if panel == x {
                *panel = y.to_string();
            } else if panel == y {
                *panel = x.to_string();
            }
        }
        Node::Split { a, b, .. } => {
            swap_panels(a, x, y);
            swap_panels(b, x, y);
        }
    }
}

fn rename_panel(node: &mut Node, from: &str, to: &str) {
    match node {
        Node::Leaf { panel } => {
            if panel == from {
                *panel = to.to_string();
            }
        }
        Node::Split { a, b, .. } => {
            rename_panel(a, from, to);
            rename_panel(b, from, to);
        }
    }
}

/// Tira o painel da árvore: a divisão que o continha é trocada pelo irmão.
fn remove_panel(node: &mut Node, id: &str) -> bool {
    if let Node::Split { a, b, .. } = node {
        if matches!(a.as_ref(), Node::Leaf { panel } if panel == id) {
            *node = std::mem::replace(b.as_mut(), Node::Leaf { panel: String::new() });
            return true;
        }
        if matches!(b.as_ref(), Node::Leaf { panel } if panel == id) {
            *node = std::mem::replace(a.as_mut(), Node::Leaf { panel: String::new() });
            return true;
        }
        return remove_panel(a, id) || remove_panel(b, id);
    }
    false
}

fn insert_at_panel(node: &mut Node, target: &str, leaf: Node, edge: Edge) -> bool {
    match node {
        Node::Leaf { panel } if panel == target => {
            let old = std::mem::replace(node, Node::Leaf { panel: String::new() });
            let (split, first) = match edge {
                Edge::Left => (Dir::Row, true),
                Edge::Right => (Dir::Row, false),
                Edge::Top => (Dir::Column, true),
                _ => (Dir::Column, false),
            };
            let (a, b) = if first { (leaf, old) } else { (old, leaf) };
            *node = Node::Split { split, ratio: 0.5, a: Box::new(a), b: Box::new(b) };
            true
        }
        Node::Leaf { .. } => false,
        Node::Split { a, b, .. } => {
            // `leaf` só pode ser movido uma vez: tenta o lado a, e se não achar, o lado b.
            if contains_panel(a, target) {
                insert_at_panel(a, target, leaf, edge)
            } else {
                insert_at_panel(b, target, leaf, edge)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
gaps_in = 6
gaps_out = 10

[[breakpoint]]
min_width = 0
root = { split = "column", ratio = 0.8, a = { panel = "visual" }, b = { panel = "player" } }

[[breakpoint]]
min_width = 900
root = { split = "row", ratio = 0.7, a = { split = "column", ratio = 0.8, a = { panel = "visual" }, b = { panel = "player" } }, b = { panel = "queue" } }

[[floating]]
panel = "clock"
x = 0.02
y = 0.02
w = 0.15
h = 0.1
"#;

    #[test]
    fn breakpoints_e_retangulos() {
        let l = LayoutFile::parse(SAMPLE).unwrap();
        let (p, d) = l.compute(1200.0, 800.0);
        assert_eq!(p.len(), 4);
        assert_eq!(d.len(), 2);
        let (p, _) = l.compute(600.0, 800.0);
        assert_eq!(p.iter().filter(|r| !r.floating).count(), 2);
    }

    #[test]
    fn soltar_na_borda_cria_divisao() {
        let mut l = LayoutFile::parse(SAMPLE).unwrap();
        // Solta a fila na borda esquerda do visual.
        let (p, _) = l.compute(1200.0, 800.0);
        let vis = p.iter().find(|r| r.panel == "visual").unwrap().rect;
        assert!(l.drop_panel(1200.0, 800.0, "queue", vis.x + 5.0, vis.y + vis.h / 2.0));
        let (p, _) = l.compute(1200.0, 800.0);
        let q = p.iter().find(|r| r.panel == "queue").unwrap().rect;
        let v = p.iter().find(|r| r.panel == "visual").unwrap().rect;
        assert!(q.x < v.x);
    }

    #[test]
    fn soltar_no_meio_troca() {
        let mut l = LayoutFile::parse(SAMPLE).unwrap();
        let (p, _) = l.compute(1200.0, 800.0);
        let vis = p.iter().find(|r| r.panel == "visual").unwrap().rect;
        assert!(l.drop_panel(1200.0, 800.0, "queue", vis.x + vis.w / 2.0, vis.y + vis.h / 2.0));
        let (p, _) = l.compute(1200.0, 800.0);
        let q = p.iter().find(|r| r.panel == "queue").unwrap().rect;
        assert_eq!(q, vis);
    }

    #[test]
    fn divisoria_respeita_limites() {
        let mut l = LayoutFile::parse(SAMPLE).unwrap();
        let (_, d) = l.compute(1200.0, 800.0);
        let top = d.iter().find(|d| d.path.is_empty()).unwrap().clone();
        l.drag_divider(1200.0, &top, 5000.0, 0.0);
        let (_, d) = l.compute(1200.0, 800.0);
        let top = d.iter().find(|d| d.path.is_empty()).unwrap();
        assert!(top.rect.x < 1200.0 * (1.0 - MIN_RATIO) + 1.0);
    }

    #[test]
    fn ida_e_volta_toml() {
        let l = LayoutFile::parse(SAMPLE).unwrap();
        let again = LayoutFile::parse(&l.to_toml()).unwrap();
        assert_eq!(l, again);
    }
}
