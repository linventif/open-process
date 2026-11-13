//! Typed graph mechanics shared by EPC and UML activity models.
//! The node type owns connection rules; this module owns identity and geometry.
use crate::geom::{Pos, Rect};
use crate::i18n::{key, tr};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub type Id = u64;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Action,
    Event,
    Diamond,
    Initial,
    Final,
    FlowFinal,
    Bar,
    Connector(&'static str),
}

pub trait FlowKind: Copy + Eq + std::fmt::Debug + 'static {
    fn all() -> &'static [Self];
    fn label(self) -> &'static str;
    fn shape(self) -> Shape;
    fn can_connect(self, target: Self) -> bool;
    fn degree_valid(self, incoming: usize, outgoing: usize) -> bool;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node<K> {
    pub id: Id,
    pub kind: K,
    pub name: String,
    pub bounds: Rect,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    pub id: Id,
    pub from: Id,
    pub to: Id,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagram<K> {
    pub name: String,
    pub nodes: Vec<Node<K>>,
    pub flows: Vec<Flow>,
}

impl<K: FlowKind> Node<K> {
    pub fn anchor(&self, toward: Pos) -> Pos {
        let r = self.bounds;
        let c = r.center();
        let d = toward - c;
        if d.x.abs() + d.y.abs() < 0.001 {
            return c;
        }
        let x = d.x / (r.w / 2.0);
        let y = d.y / (r.h / 2.0);
        let scale = match self.kind.shape() {
            Shape::Initial | Shape::Final | Shape::FlowFinal | Shape::Connector(_) => 1.0 / x.hypot(y),
            Shape::Diamond => 1.0 / (x.abs() + y.abs()),
            _ => 1.0 / x.abs().max(y.abs()),
        };
        Pos::new(c.x + d.x * scale, c.y + d.y * scale)
    }

    pub fn contains(&self, p: Pos) -> bool {
        let r = self.bounds;
        let x = (p.x - r.center().x) / (r.w / 2.0);
        let y = (p.y - r.center().y) / (r.h / 2.0);
        match self.kind.shape() {
            Shape::Initial | Shape::Final | Shape::FlowFinal | Shape::Connector(_) => x * x + y * y <= 1.0,
            Shape::Diamond => x.abs() + y.abs() <= 1.0,
            _ => r.contains(p),
        }
    }
}

impl<K: FlowKind> Diagram<K> {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            nodes: vec![],
            flows: vec![],
        }
    }
    fn next_id(&self) -> Id {
        self.nodes
            .iter()
            .map(|n| n.id)
            .chain(self.flows.iter().map(|f| f.id))
            .max()
            .unwrap_or(0)
            .saturating_add(1)
    }
    pub fn node(&self, id: Id) -> Option<&Node<K>> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub fn add_node(&mut self, kind: K, center: Pos, name: impl Into<String>) -> Id {
        let (w, h) = match kind.shape() {
            Shape::Initial | Shape::Final | Shape::FlowFinal | Shape::Connector(_) => (40.0, 40.0),
            Shape::Diamond => (60.0, 60.0),
            Shape::Bar => (140.0, 12.0),
            _ => (150.0, 65.0),
        };
        let id = self.next_id();
        self.nodes.push(Node {
            id,
            kind,
            name: name.into(),
            bounds: Rect::from_center(center, w, h),
        });
        id
    }
    pub fn can_connect(&self, from: Id, to: Id) -> bool {
        from != to
            && self.node(from).zip(self.node(to)).is_some_and(|(a, b)| a.kind.can_connect(b.kind))
            && !self.flows.iter().any(|f| f.from == from && f.to == to)
    }
    pub fn connect(&mut self, from: Id, to: Id, label: impl Into<String>) -> Option<Id> {
        if !self.can_connect(from, to) {
            return None;
        }
        let id = self.next_id();
        self.flows.push(Flow {
            id,
            from,
            to,
            label: label.into(),
        });
        Some(id)
    }
    pub fn delete(&mut self, nodes: &BTreeSet<Id>, flow: Option<Id>) {
        self.nodes.retain(|n| !nodes.contains(&n.id));
        self.flows
            .retain(|f| !nodes.contains(&f.from) && !nodes.contains(&f.to) && flow != Some(f.id));
    }
    pub fn duplicate(&mut self, ids: &BTreeSet<Id>) -> BTreeSet<Id> {
        let nodes: Vec<_> = self.nodes.iter().filter(|n| ids.contains(&n.id)).cloned().collect();
        let flows = self.flows.clone();
        let mut map = std::collections::BTreeMap::new();
        for mut n in nodes {
            let id = self.next_id();
            map.insert(n.id, id);
            n.id = id;
            n.bounds.x += 30.0;
            n.bounds.y += 30.0;
            self.nodes.push(n);
        }
        for f in flows {
            if let (Some(&from), Some(&to)) = (map.get(&f.from), map.get(&f.to)) {
                self.connect(from, to, f.label);
            }
        }
        map.into_values().collect()
    }
    /// Storage invariants, distinct from an incomplete model's editable diagnostics.
    pub fn check_storage(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        for n in &self.nodes {
            let r = n.bounds;
            if n.id == 0
                || n.id == u64::MAX
                || !ids.insert(n.id)
                || ![r.x, r.y, r.w, r.h].iter().all(|x| x.is_finite())
                || r.w <= 0.0
                || r.h <= 0.0
            {
                return Err("Invalid flow node identity or geometry".into());
            }
        }
        let mut ends = BTreeSet::new();
        for f in &self.flows {
            if f.id == 0
                || f.id == u64::MAX
                || !ids.insert(f.id)
                || f.from == f.to
                || !ends.insert((f.from, f.to))
                || !self
                    .node(f.from)
                    .zip(self.node(f.to))
                    .is_some_and(|(a, b)| a.kind.can_connect(b.kind))
            {
                return Err("Invalid flow identity or endpoints".into());
            }
        }
        Ok(())
    }
    pub fn issues(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter_map(|n| {
                let incoming = self.flows.iter().filter(|f| f.to == n.id).count();
                let outgoing = self.flows.iter().filter(|f| f.from == n.id).count();
                (!n.kind.degree_valid(incoming, outgoing)).then(|| {
                    format!(
                        "{} (#{}): {}",
                        tr(n.kind.label()),
                        n.id,
                        tr(key("flow.check_incoming_outgoing_flows"))
                    )
                })
            })
            .collect()
    }
    pub fn bounds(&self) -> Option<(Pos, Pos)> {
        if self.nodes.is_empty() {
            return None;
        }
        Some((
            Pos::new(
                self.nodes.iter().map(|n| n.bounds.x).fold(f32::INFINITY, f32::min),
                self.nodes.iter().map(|n| n.bounds.y).fold(f32::INFINITY, f32::min),
            ),
            Pos::new(
                self.nodes.iter().map(|n| n.bounds.right()).fold(f32::NEG_INFINITY, f32::max),
                self.nodes.iter().map(|n| n.bounds.bottom()).fold(f32::NEG_INFINITY, f32::max),
            ),
        ))
    }
    pub fn points(&self, f: &Flow) -> Option<(Pos, Pos)> {
        let (a, b) = (self.node(f.from)?, self.node(f.to)?);
        Some((a.anchor(b.bounds.center()), b.anchor(a.bounds.center())))
    }
    pub fn to_dot(&self) -> String {
        let quoted = |s: &str| serde_json::to_string(s).unwrap();
        let mut out = format!("digraph model {{\n  label={};\n", quoted(&self.name));
        for n in &self.nodes {
            let shape = match n.kind.shape() {
                Shape::Diamond => "diamond",
                Shape::Event => "hexagon",
                Shape::Initial | Shape::Final | Shape::FlowFinal | Shape::Connector(_) => "circle",
                _ => "box",
            };
            out.push_str(&format!("  n{} [label={}, shape={}];\n", n.id, quoted(&n.name), shape));
        }
        for f in &self.flows {
            out.push_str(&format!("  n{} -> n{} [label={}];\n", f.from, f.to, quoted(&f.label)));
        }
        out.push_str("}\n");
        out
    }
    pub fn to_svg(&self) -> String {
        use crate::xml::escape;
        let (min, max) = self.bounds().unwrap_or((Pos::new(0.0, 0.0), Pos::new(400.0, 240.0)));
        let mut out = format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {} {}\"><title>{}</title><defs><marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"9\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto\"><path d=\"M0 0L10 5L0 10Z\" fill=\"#475569\"/></marker></defs><g font-family=\"sans-serif\" font-size=\"13\" fill=\"#172033\">",
            min.x - 60.0,
            min.y - 60.0,
            max.x - min.x + 120.0,
            max.y - min.y + 140.0,
            escape(&self.name)
        );
        for f in &self.flows {
            if let Some((a, b)) = self.points(f) {
                out.push_str(&format!("<path d=\"M{} {}L{} {}\" fill=\"none\" stroke=\"#475569\" marker-end=\"url(#arrow)\"/><text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",a.x,a.y,b.x,b.y,(a.x+b.x)/2.0,(a.y+b.y)/2.0-7.0,escape(&f.label)));
            }
        }
        for n in &self.nodes {
            let r = n.bounds;
            let c = r.center();
            let attr = "fill=\"#f1f5ff\" stroke=\"#475569\" stroke-width=\"1.5\"";
            let element = match n.kind.shape() {
                Shape::Diamond => format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{}\" {attr}/>",
                    c.x,
                    r.y,
                    r.right(),
                    c.y,
                    c.x,
                    r.bottom(),
                    r.x,
                    c.y
                ),
                Shape::Event => format!(
                    "<polygon points=\"{},{} {},{} {},{} {},{} {},{} {},{}\" {attr}/>",
                    r.x + 15.0,
                    r.y,
                    r.right() - 15.0,
                    r.y,
                    r.right(),
                    c.y,
                    r.right() - 15.0,
                    r.bottom(),
                    r.x + 15.0,
                    r.bottom(),
                    r.x,
                    c.y
                ),
                Shape::Initial => format!(
                    "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"#172033\"/>",
                    c.x,
                    c.y,
                    r.w / 2.0,
                    r.h / 2.0
                ),
                Shape::Final | Shape::FlowFinal | Shape::Connector(_) => {
                    let mut s = format!(
                        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" {attr}/>",
                        c.x,
                        c.y,
                        r.w / 2.0,
                        r.h / 2.0
                    );
                    if n.kind.shape() == Shape::Final {
                        s.push_str(&format!(
                            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"#172033\"/>",
                            c.x,
                            c.y,
                            r.w * 0.3,
                            r.h * 0.3
                        ));
                    }
                    if n.kind.shape() == Shape::FlowFinal {
                        s.push_str(&format!(
                            "<path d=\"M{} {}l20 20m0 -20l-20 20\" stroke=\"#475569\"/>",
                            c.x - 10.0,
                            c.y - 10.0
                        ));
                    }
                    s
                }
                Shape::Bar => format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"#172033\"/>",
                    r.x, r.y, r.w, r.h
                ),
                _ => format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"10\" {attr}/>",
                    r.x, r.y, r.w, r.h
                ),
            };
            out.push_str(&element);
            if let Shape::Connector(symbol) = n.kind.shape() {
                out.push_str(&format!(
                    "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                    c.x,
                    c.y + 4.0,
                    escape(symbol)
                ));
            }
            let y = if matches!(n.kind.shape(), Shape::Action | Shape::Event) {
                c.y + 4.0
            } else {
                r.bottom() + 20.0
            };
            out.push_str(&format!(
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\">{}</text>",
                c.x,
                y,
                escape(&n.name)
            ));
        }
        out.push_str("</g></svg>");
        out
    }
}
