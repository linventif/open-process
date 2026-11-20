//! Typed graph mechanics shared by EPC and UML activity models.
//! The node type owns connection rules; this module owns identity and geometry.
use crate::geom::{Pos, Rect};
use crate::i18n::{key, tr};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub type Id = u64;

/// Flows to merge when `deleted` nodes are removed. Deleted nodes linked by
/// flows form one block; a block with exactly one flow entering from outside
/// and one leaving to outside yields `(entering flow, outside target)`, so
/// A→B→C→D without B and C becomes A→D. `edges` are `(flow, from, to)`.
pub fn bridges<N: Copy + Ord, F: Copy>(deleted: &BTreeSet<N>, edges: &[(F, N, N)]) -> Vec<(F, N)> {
    // Union-find over the deleted nodes, joined by the flows between them.
    let nodes: Vec<N> = deleted.iter().copied().collect();
    let index = |n: N| nodes.binary_search(&n).ok();
    let mut parent: Vec<usize> = (0..nodes.len()).collect();
    fn root(parent: &mut [usize], mut i: usize) -> usize {
        while parent[i] != i {
            parent[i] = parent[parent[i]];
            i = parent[i];
        }
        i
    }
    for &(_, from, to) in edges {
        if let (Some(a), Some(b)) = (index(from), index(to)) {
            let (a, b) = (root(&mut parent, a), root(&mut parent, b));
            parent[a] = b;
        }
    }
    let mut entering = std::collections::BTreeMap::<usize, Vec<F>>::new();
    let mut leaving = std::collections::BTreeMap::<usize, Vec<N>>::new();
    for &(flow, from, to) in edges {
        match (index(from), index(to)) {
            (None, Some(b)) => entering.entry(root(&mut parent, b)).or_default().push(flow),
            (Some(a), None) => leaving.entry(root(&mut parent, a)).or_default().push(to),
            _ => {}
        }
    }
    entering
        .into_iter()
        .filter_map(|(block, flows)| match (flows.as_slice(), leaving.get(&block).map(Vec::as_slice)) {
            ([flow], Some([to])) => Some((*flow, *to)),
            _ => None,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Action,
    Terminator,
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

/// A transient selection, independent of document storage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment<K> {
    pub nodes: Vec<Node<K>>,
    pub flows: Vec<Flow>,
}

impl<K: FlowKind> Node<K> {
    pub fn anchor(&self, toward: Pos) -> Pos {
        let r = self.bounds;
        let c = r.center();
        let d = toward - c;
        if d.x.abs() + d.y.abs() < 0.001 {
            return crate::geom::Port::Right.point(r);
        }
        let x = d.x / (r.w / 2.0);
        let y = d.y / (r.h / 2.0);
        let scale = match self.kind.shape() {
            Shape::Initial | Shape::Final | Shape::FlowFinal | Shape::Connector(_) => 1.0 / x.hypot(y),
            _ => return crate::geom::Port::toward(r, toward).point(r),
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
    /// Arranges nodes left to right along their flows. Loops return leftward;
    /// identities, kinds, names, sizes and flows are unchanged.
    pub fn auto_arrange(&mut self, grid: Option<f32>) -> Result<(), String> {
        let index: std::collections::HashMap<Id, usize> = self.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let sizes: Vec<_> = self.nodes.iter().map(|n| (n.bounds.w, n.bounds.h)).collect();
        let edges: Vec<_> = self
            .flows
            .iter()
            .filter_map(|f| Some((*index.get(&f.from)?, *index.get(&f.to)?)))
            .collect();
        let options = crate::layout::Options::new(crate::layout::Direction::LeftToRight, grid);
        let centers = crate::layout::layered(&sizes, &edges, options).map_err(|e| e.to_string())?;
        for (n, c) in self.nodes.iter_mut().zip(centers) {
            n.bounds = Rect::from_center(c, n.bounds.w, n.bounds.h);
        }
        Ok(())
    }

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
    /// Deletes like [`Self::delete`], then merges the flows around removed
    /// nodes (see [`bridges`]) when the notation allows the direct connection
    /// and it does not exist yet. The entering flow keeps its label.
    pub fn delete_bridging(&mut self, nodes: &BTreeSet<Id>, flow: Option<Id>) {
        self.flows.retain(|f| Some(f.id) != flow);
        let edges: Vec<_> = self.flows.iter().map(|f| (f.id, f.from, f.to)).collect();
        let mut merged = vec![];
        for (entering, to) in bridges(nodes, &edges) {
            let Some(from) = self.flows.iter().find(|f| f.id == entering).map(|f| f.from) else {
                continue;
            };
            let allowed = from != to
                && self.node(from).zip(self.node(to)).is_some_and(|(a, b)| a.kind.can_connect(b.kind))
                && !self.flows.iter().any(|f| f.from == from && f.to == to)
                && !merged.contains(&(from, to));
            if allowed && let Some(f) = self.flows.iter_mut().find(|f| f.id == entering) {
                f.to = to;
                merged.push((from, to));
            }
        }
        self.delete(nodes, None);
    }

    /// Moves one end of a flow to `node` when the notation allows it.
    pub fn reconnect(&mut self, id: Id, end: crate::geom::Endpoint, node: Id) -> bool {
        let Some(flow) = self.flows.iter().find(|f| f.id == id) else {
            return false;
        };
        let (from, to) = end.moved((flow.from, flow.to), node);
        let allowed = from != to
            && self.node(from).zip(self.node(to)).is_some_and(|(a, b)| a.kind.can_connect(b.kind))
            && !self.flows.iter().any(|f| f.id != id && f.from == from && f.to == to);
        if allowed && let Some(flow) = self.flows.iter_mut().find(|f| f.id == id) {
            (flow.from, flow.to) = (from, to);
        }
        allowed
    }

    pub fn delete(&mut self, nodes: &BTreeSet<Id>, flow: Option<Id>) {
        self.nodes.retain(|n| !nodes.contains(&n.id));
        self.flows
            .retain(|f| !nodes.contains(&f.from) && !nodes.contains(&f.to) && flow != Some(f.id));
    }
    pub fn fragment(&self, ids: &BTreeSet<Id>) -> Fragment<K> {
        Fragment {
            nodes: self.nodes.iter().filter(|n| ids.contains(&n.id)).cloned().collect(),
            flows: self
                .flows
                .iter()
                .filter(|f| ids.contains(&f.from) && ids.contains(&f.to))
                .cloned()
                .collect(),
        }
    }
    pub fn paste(&mut self, fragment: &Fragment<K>, delta: Pos) -> Result<BTreeSet<Id>, String> {
        Self {
            name: String::new(),
            nodes: fragment.nodes.clone(),
            flows: fragment.flows.clone(),
        }
        .check_storage()?;
        let mut result = self.clone();
        let mut map = std::collections::BTreeMap::new();
        for mut node in fragment.nodes.clone() {
            let id = result.next_id();
            map.insert(node.id, id);
            node.id = id;
            node.bounds.x += delta.x;
            node.bounds.y += delta.y;
            result.nodes.push(node);
        }
        for flow in &fragment.flows {
            if let (Some(&from), Some(&to)) = (map.get(&flow.from), map.get(&flow.to)) {
                result.connect(from, to, flow.label.clone());
            }
        }
        result.check_storage()?;
        *self = result;
        Ok(map.into_values().collect())
    }
    pub fn duplicate(&mut self, ids: &BTreeSet<Id>) -> BTreeSet<Id> {
        self.paste(&self.fragment(ids), Pos::new(30.0, 30.0)).unwrap_or_default()
    }
    /// Storage invariants, distinct from an incomplete model's editable diagnostics.
    pub fn check_storage(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        let mut kinds = std::collections::HashMap::new();
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
            kinds.insert(n.id, n.kind);
        }
        let mut ends = BTreeSet::new();
        for f in &self.flows {
            if f.id == 0
                || f.id == u64::MAX
                || !ids.insert(f.id)
                || f.from == f.to
                || !ends.insert((f.from, f.to))
                || !kinds.get(&f.from).zip(kinds.get(&f.to)).is_some_and(|(a, b)| a.can_connect(*b))
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
                Shape::Terminator => format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"{}\" {attr}/>",
                    r.x,
                    r.y,
                    r.w,
                    r.h,
                    r.h / 2.0
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
            let y = if matches!(n.kind.shape(), Shape::Action | Shape::Terminator | Shape::Event) {
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
