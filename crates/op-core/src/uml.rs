//! UML class and use-case diagrams, independent of the GUI.

use crate::geom::{Pos, Rect};
use crate::i18n::{key, tr, trf};
use serde::{Deserialize, Serialize};

pub mod routing;

pub type Id = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramKind {
    Class,
    UseCase,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Class,
    Interface,
    Enumeration,
    Actor,
    UseCase,
    System,
}

impl NodeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Class => key("uml.class"),
            Self::Interface => key("uml.interface"),
            Self::Enumeration => key("uml.enumeration"),
            Self::Actor => key("uml.actor"),
            Self::UseCase => key("uml.use_case"),
            Self::System => key("uml.system_boundary"),
        }
    }

    pub fn is_classifier(self) -> bool {
        matches!(self, Self::Class | Self::Interface | Self::Enumeration)
    }
}

impl DiagramKind {
    pub fn node_kinds(self) -> &'static [NodeKind] {
        match self {
            Self::Class => &[NodeKind::Class, NodeKind::Interface, NodeKind::Enumeration],
            Self::UseCase => &[NodeKind::Actor, NodeKind::UseCase, NodeKind::System],
        }
    }

    pub fn relation_kinds(self) -> &'static [RelationKind] {
        match self {
            Self::Class => &[
                RelationKind::Association,
                RelationKind::DirectedAssociation,
                RelationKind::Aggregation,
                RelationKind::Composition,
                RelationKind::Generalization,
                RelationKind::Realization,
                RelationKind::Dependency,
            ],
            Self::UseCase => &[
                RelationKind::Association,
                RelationKind::Generalization,
                RelationKind::Include,
                RelationKind::Extend,
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub kind: NodeKind,
    pub name: String,
    pub bounds: Rect,
    /// UML member notation, e.g. `- name: String` and `+ submit(): bool`.
    pub attributes: String,
    pub operations: String,
    pub abstract_class: bool,
}

impl Node {
    /// Grow class compartments to keep every member visible.
    pub fn rect(&self) -> Rect {
        let mut r = self.bounds;
        r.w = r.w.max(80.0);
        r.h = r.h.max(if self.kind == NodeKind::Actor { 130.0 } else { 60.0 });
        if self.kind.is_classifier() {
            r.h = r.h.max(
                46.0 + self.attributes.lines().count().max(1) as f32 * 20.0 + self.operations.lines().count().max(1) as f32 * 20.0 + 16.0,
            );
        }
        r
    }

    /// Anchor on the visible outline in the direction of another point.
    pub fn anchor(&self, toward: Pos) -> Pos {
        let r = self.rect();
        if self.kind == NodeKind::Actor {
            let chest = Pos::new(r.center().x, r.y + 40.0);
            let d = toward - chest;
            return if d.x.abs() >= d.y.abs() {
                Pos::new(chest.x + 25.0 * d.x.signum(), chest.y)
            } else {
                Pos::new(chest.x, if d.y < 0.0 { r.y + 3.0 } else { r.y + 95.0 })
            };
        }
        let c = r.center();
        let d = toward - c;
        if d.x.abs() + d.y.abs() < 0.001 {
            return Pos::new(r.right(), c.y);
        }
        let scale = if self.kind == NodeKind::UseCase {
            1.0 / ((d.x / (r.w / 2.0)).powi(2) + (d.y / (r.h / 2.0)).powi(2)).sqrt()
        } else {
            ((r.w / 2.0) / d.x.abs()).min((r.h / 2.0) / d.y.abs())
        };
        Pos::new(c.x + d.x * scale, c.y + d.y * scale)
    }

    pub fn contains(&self, p: Pos) -> bool {
        let r = self.rect();
        if self.kind == NodeKind::UseCase {
            ((p.x - r.center().x) / (r.w / 2.0)).powi(2) + ((p.y - r.center().y) / (r.h / 2.0)).powi(2) <= 1.0
        } else if self.kind == NodeKind::System {
            // The interior stays available for selecting and creating use cases.
            r.contains(p) && (p.y < r.y + 30.0 || p.x < r.x + 8.0 || p.x > r.right() - 8.0 || p.y > r.bottom() - 8.0)
        } else {
            r.contains(p)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    Association,
    DirectedAssociation,
    Aggregation,
    Composition,
    Generalization,
    Realization,
    Dependency,
    Include,
    Extend,
}

impl RelationKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Association => key("bpmn.association"),
            Self::DirectedAssociation => key("uml.directed_association"),
            Self::Aggregation => key("uml.aggregation"),
            Self::Composition => key("uml.composition"),
            Self::Generalization => key("uml.generalization"),
            Self::Realization => key("uml.realization"),
            Self::Dependency => key("uml.dependency"),
            Self::Include => key("uml.include"),
            Self::Extend => key("uml.extend"),
        }
    }

    pub fn dashed(self) -> bool {
        matches!(self, Self::Realization | Self::Dependency | Self::Include | Self::Extend)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub id: Id,
    pub from: Id,
    pub to: Id,
    pub kind: RelationKind,
    pub label: String,
    pub source_multiplicity: String,
    pub target_multiplicity: String,
    pub source_role: String,
    pub target_role: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UmlDiagram {
    pub name: String,
    pub kind: DiagramKind,
    pub nodes: Vec<Node>,
    pub relations: Vec<Relation>,
}

/// An internal clipboard preserves members, sizes, labels and incident relations.
#[derive(Clone, Debug)]
pub struct DiagramFragment {
    pub kind: DiagramKind,
    pub nodes: Vec<Node>,
    pub relations: Vec<Relation>,
}

impl UmlDiagram {
    pub fn new(name: impl Into<String>, kind: DiagramKind) -> Self {
        Self {
            name: name.into(),
            kind,
            nodes: Vec::new(),
            relations: Vec::new(),
        }
    }

    fn next_id(&self) -> Id {
        self.nodes
            .iter()
            .map(|n| n.id)
            .chain(self.relations.iter().map(|r| r.id))
            .max()
            .unwrap_or(0)
            + 1
    }

    pub fn node(&self, id: Id) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn add_node(&mut self, kind: NodeKind, pos: Pos, name: impl Into<String>) -> Option<Id> {
        if !self.kind.node_kinds().contains(&kind) {
            return None;
        }
        let (w, h) = match kind {
            NodeKind::Actor => (90.0, 130.0),
            NodeKind::UseCase => (180.0, 85.0),
            NodeKind::System => (460.0, 340.0),
            _ => (220.0, 130.0),
        };
        let id = self.next_id();
        self.nodes.push(Node {
            id,
            kind,
            name: name.into(),
            bounds: Rect::new(pos.x, pos.y, w, h),
            attributes: String::new(),
            operations: String::new(),
            abstract_class: false,
        });
        Some(id)
    }

    /// Direction: child → parent, implementation → interface, including → included,
    /// extending → extended. Aggregation/composition diamonds are at the source (whole).
    pub fn can_connect(&self, from: Id, to: Id, kind: RelationKind) -> bool {
        self.can_connect_excluding(from, to, kind, None)
    }

    pub fn can_connect_excluding(&self, from: Id, to: Id, kind: RelationKind, excluded: Option<Id>) -> bool {
        let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
            return false;
        };
        if !self.kind.relation_kinds().contains(&kind) || a.kind == NodeKind::System || b.kind == NodeKind::System {
            return false;
        }
        if from == to {
            return self.kind == DiagramKind::Class && matches!(kind, RelationKind::Association | RelationKind::DirectedAssociation);
        }
        if kind == RelationKind::Generalization {
            // Reject inheritance cycles, including when changing or reversing an existing relation.
            let mut pending = vec![to];
            let mut visited = std::collections::HashSet::new();
            while let Some(id) = pending.pop() {
                if id == from {
                    return false;
                }
                if visited.insert(id) {
                    pending.extend(
                        self.relations
                            .iter()
                            .filter(|r| Some(r.id) != excluded && r.kind == RelationKind::Generalization && r.from == id)
                            .map(|r| r.to),
                    );
                }
            }
        }
        match self.kind {
            DiagramKind::Class => {
                a.kind.is_classifier()
                    && b.kind.is_classifier()
                    && (kind != RelationKind::Generalization || (a.kind == b.kind && a.kind != NodeKind::Enumeration))
                    && (kind != RelationKind::Realization || (a.kind == NodeKind::Class && b.kind == NodeKind::Interface))
            }
            DiagramKind::UseCase => match kind {
                RelationKind::Association => matches!(
                    (a.kind, b.kind),
                    (NodeKind::Actor, NodeKind::UseCase) | (NodeKind::UseCase, NodeKind::Actor)
                ),
                RelationKind::Generalization => a.kind == b.kind,
                RelationKind::Include | RelationKind::Extend => a.kind == NodeKind::UseCase && b.kind == NodeKind::UseCase,
                _ => false,
            },
        }
    }

    pub fn connect(&mut self, from: Id, to: Id, kind: RelationKind) -> Option<Id> {
        if !self.can_connect(from, to, kind) {
            return None;
        }
        let id = self.next_id();
        self.relations.push(Relation {
            id,
            from,
            to,
            kind,
            label: String::new(),
            source_multiplicity: String::new(),
            target_multiplicity: String::new(),
            source_role: String::new(),
            target_role: String::new(),
        });
        Some(id)
    }

    pub fn remove_node(&mut self, id: Id) {
        self.nodes.retain(|n| n.id != id);
        self.relations.retain(|r| r.from != id && r.to != id);
    }

    /// A system carries its contained use cases when moved by its header or border.
    pub fn move_node(&mut self, id: Id, pos: Pos) {
        let Some(node) = self.node(id) else { return };
        let delta = pos - Pos::new(node.bounds.x, node.bounds.y);
        let selected = std::collections::BTreeSet::from([id]);
        let ids = self.selection_closure(&selected);
        for n in &mut self.nodes {
            if ids.contains(&n.id) {
                n.bounds.x += delta.x;
                n.bounds.y += delta.y;
            }
        }
    }

    /// Resolve containment by complete bounds, using the smallest enclosing system.
    pub fn system_of(&self, node: &Node) -> Option<Id> {
        if node.kind != NodeKind::UseCase {
            return None;
        }
        let child = node.rect();
        self.nodes
            .iter()
            .filter(|n| {
                let r = n.rect();
                n.kind == NodeKind::System
                    && child.x >= r.x
                    && child.y >= r.y + 30.0
                    && child.right() <= r.right()
                    && child.bottom() <= r.bottom()
            })
            .min_by(|a, b| (a.rect().w * a.rect().h).total_cmp(&(b.rect().w * b.rect().h)))
            .map(|n| n.id)
    }

    /// Include contained use cases once, even when they are selected alongside their system.
    pub fn selection_closure(&self, selected: &std::collections::BTreeSet<Id>) -> std::collections::BTreeSet<Id> {
        let mut ids = selected.clone();
        for n in &self.nodes {
            if self.system_of(n).is_some_and(|system| selected.contains(&system)) {
                ids.insert(n.id);
            }
        }
        ids
    }

    pub fn fragment(&self, selected: &std::collections::BTreeSet<Id>) -> DiagramFragment {
        let ids = self.selection_closure(selected);
        DiagramFragment {
            kind: self.kind,
            nodes: self.nodes.iter().filter(|n| ids.contains(&n.id)).cloned().collect(),
            relations: self
                .relations
                .iter()
                .filter(|r| ids.contains(&r.from) && ids.contains(&r.to))
                .cloned()
                .collect(),
        }
    }

    /// Paste with fresh node and relation ids; external links never point back to the original.
    pub fn paste(&mut self, fragment: &DiagramFragment, delta: Pos) -> std::collections::BTreeSet<Id> {
        if fragment.kind != self.kind {
            return Default::default();
        }
        let mut remap = std::collections::HashMap::new();
        for old in &fragment.nodes {
            let id = self.next_id();
            let mut node = old.clone();
            node.id = id;
            node.bounds.x += delta.x;
            node.bounds.y += delta.y;
            remap.insert(old.id, id);
            self.nodes.push(node);
        }
        for old in &fragment.relations {
            if let (Some(&from), Some(&to)) = (remap.get(&old.from), remap.get(&old.to)) {
                let mut relation = old.clone();
                relation.id = self.next_id();
                relation.from = from;
                relation.to = to;
                self.relations.push(relation);
            }
        }
        remap.into_values().collect()
    }

    pub fn validate(&self) -> Vec<(Option<Id>, String)> {
        let mut issues = Vec::new();
        let mut ids = std::collections::HashSet::new();
        for n in &self.nodes {
            if !ids.insert(n.id) {
                issues.push((Some(n.id), tr("uml.duplicate_uml_identifier").into()));
            }
            if n.name.trim().is_empty() {
                issues.push((Some(n.id), tr("uml.element_has_no_name").into()));
            }
            if !self.kind.node_kinds().contains(&n.kind) {
                issues.push((Some(n.id), tr("uml.element_type_does_not_belong_diagram").into()));
            }
            if ![n.bounds.x, n.bounds.y, n.bounds.w, n.bounds.h].iter().all(|v| v.is_finite()) || n.bounds.w <= 0.0 || n.bounds.h <= 0.0 {
                issues.push((Some(n.id), tr("validation.invalid_dimensions").into()));
            }
        }
        for r in &self.relations {
            if !ids.insert(r.id) {
                issues.push((Some(r.id), tr("uml.duplicate_uml_identifier").into()));
            }
            if !self.can_connect_excluding(r.from, r.to, r.kind, Some(r.id)) {
                issues.push((Some(r.id), tr("uml.invalid_relation_endpoints_inheritance_cycle").into()));
            }
            for value in [&r.source_multiplicity, &r.target_multiplicity] {
                if !valid_multiplicity(value) {
                    issues.push((Some(r.id), trf("uml.invalid_multiplicity", &[value])));
                }
            }
        }
        issues
    }

    /// Export editable PlantUML source. Stable aliases keep duplicate names distinct.
    pub fn to_plantuml(&self) -> String {
        fn quote(s: &str) -> String {
            s.replace('\\', "\\\\").replace('"', "\\\"").replace('\r', "").replace('\n', "\\n")
        }
        let mut out = format!("@startuml\ntitle \"{}\"\n", quote(&self.name));
        fn declaration(n: &Node) -> String {
            let keyword = match n.kind {
                NodeKind::Class if n.abstract_class => "abstract class",
                NodeKind::Class => "class",
                NodeKind::Interface => "interface",
                NodeKind::Enumeration => "enum",
                NodeKind::Actor => "actor",
                NodeKind::UseCase => "usecase",
                NodeKind::System => "rectangle",
            };
            let mut text = format!("{keyword} \"{}\" as n{}", quote(&n.name), n.id);
            if n.kind.is_classifier() {
                text.push_str(" {\n");
                for (i, members) in [&n.attributes, &n.operations].iter().enumerate() {
                    if i > 0 {
                        text.push_str("  --\n");
                    }
                    for line in members.lines() {
                        // Member text cannot close a declaration or add PlantUML directives.
                        let safe = line.replace(['{', '}'], "").replace('@', "＠");
                        text.push_str(&format!("  {safe}\n"));
                    }
                }
                text.push('}');
            }
            text.push('\n');
            text
        }
        // Declare contained use cases inside their system from the outset.
        let mut assigned = std::collections::HashSet::new();
        for system in self.nodes.iter().filter(|n| n.kind == NodeKind::System) {
            out.push_str(&format!("rectangle \"{}\" as n{} {{\n", quote(&system.name), system.id));
            for n in self.nodes.iter().filter(|n| n.kind == NodeKind::UseCase) {
                if self.system_of(n) == Some(system.id) && assigned.insert(n.id) {
                    out.push_str(&declaration(n));
                }
            }
            out.push_str("}\n");
        }
        for n in self
            .nodes
            .iter()
            .filter(|n| n.kind != NodeKind::System && !assigned.contains(&n.id))
        {
            out.push_str(&declaration(n));
        }
        for r in &self.relations {
            if self.node(r.from).is_none() || self.node(r.to).is_none() {
                continue;
            }
            let arrow = match r.kind {
                RelationKind::Association => "--",
                RelationKind::DirectedAssociation => "-->",
                RelationKind::Aggregation => "o--",
                RelationKind::Composition => "*--",
                RelationKind::Generalization => "--|>",
                RelationKind::Realization => "..|>",
                _ => "..>",
            };
            let label = match r.kind {
                RelationKind::Include => format!("<<include>> {}", r.label),
                RelationKind::Extend => format!("<<extend>> {}", r.label),
                _ => r.label.clone(),
            };
            let source = format!("{} {}", r.source_role, r.source_multiplicity);
            let target = format!("{} {}", r.target_role, r.target_multiplicity);
            out.push_str(&format!(
                "n{} \"{}\" {arrow} \"{}\" n{} : {}\n",
                r.from,
                quote(source.trim()),
                quote(target.trim()),
                r.to,
                quote(&label)
            ));
        }
        out.push_str("@enduml\n");
        out
    }
}

/// Empty values are unspecified; accept integers, *, and ordered ranges.
pub fn valid_multiplicity(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() || value == "*" {
        return true;
    }
    if let Some((low, high)) = value.split_once("..") {
        let Ok(low) = low.trim().parse::<u64>() else {
            return false;
        };
        high.trim() == "*" || high.trim().parse::<u64>().is_ok_and(|high| high >= low)
    } else {
        value.parse::<u64>().is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{self, Model};

    #[test]
    fn both_diagram_types_roundtrip_and_delete_incident_relations() {
        for (kind, a, b, relation) in [
            (DiagramKind::Class, NodeKind::Class, NodeKind::Interface, RelationKind::Realization),
            (DiagramKind::UseCase, NodeKind::Actor, NodeKind::UseCase, RelationKind::Association),
        ] {
            let mut d = UmlDiagram::new("École", kind);
            let a = d.add_node(a, Pos::new(0.0, 0.0), "A").unwrap();
            let b = d.add_node(b, Pos::new(300.0, 0.0), "B").unwrap();
            d.connect(a, b, relation).unwrap();
            let model = Model::Uml(d.clone());
            assert_eq!(document::load_any(document::to_json(&model).as_bytes()).unwrap(), model);
            d.remove_node(a);
            assert!(d.relations.is_empty());
            assert_eq!(d.nodes.len(), 1);
        }
    }

    #[test]
    fn use_case_relations_enforce_endpoint_types() {
        let mut d = UmlDiagram::new("Shop", DiagramKind::UseCase);
        let a = d.add_node(NodeKind::Actor, Pos::default(), "Customer").unwrap();
        let u = d.add_node(NodeKind::UseCase, Pos::default(), "Order").unwrap();
        let v = d.add_node(NodeKind::UseCase, Pos::default(), "Pay").unwrap();
        let s = d.add_node(NodeKind::System, Pos::default(), "Shop").unwrap();
        assert!(d.connect(u, v, RelationKind::Include).is_some());
        assert!(d.connect(v, u, RelationKind::Extend).is_some());
        assert!(d.connect(a, u, RelationKind::Association).is_some());
        assert!(d.connect(a, u, RelationKind::Include).is_none());
        assert!(d.connect(u, v, RelationKind::Association).is_none());
        assert!(d.connect(s, u, RelationKind::Association).is_none());
        assert!(d.connect(u, u, RelationKind::Include).is_none());
        assert!(d.add_node(NodeKind::Class, Pos::default(), "No").is_none());
    }

    #[test]
    fn moving_system_carries_use_cases_and_export_keeps_them_nested() {
        let mut d = crate::examples::uml_use_cases();
        let before = d.clone();
        let system = d.nodes[0].clone();
        d.move_node(system.id, Pos::new(system.bounds.x + 100.0, system.bounds.y + 50.0));
        for (old, new) in before.nodes.iter().zip(&d.nodes) {
            if old.kind == NodeKind::Actor {
                assert_eq!(old, new, "actors stay outside the system");
            } else {
                assert_eq!(new.bounds.x, old.bounds.x + 100.0);
                assert_eq!(new.bounds.y, old.bounds.y + 50.0);
            }
        }
        let puml = d.to_plantuml();
        let boundary_end = puml.find("}\n").unwrap();
        let actor = d.nodes.iter().find(|n| n.kind == NodeKind::Actor).unwrap();
        assert!(puml.find(&format!("as n{}", actor.id)).unwrap() > boundary_end);
        for n in d.nodes.iter().filter(|n| n.kind == NodeKind::UseCase) {
            let alias = format!("as n{}", n.id);
            assert!(puml.find(&alias).unwrap() < boundary_end);
            assert_eq!(puml.matches(&alias).count(), 1);
        }
        assert!(puml.contains("..>"));
        assert!(puml.contains("<<include>>"));
        assert!(puml.contains("<<extend>>"));
    }

    #[test]
    fn class_compartments_and_ellipse_anchors() {
        let mut d = UmlDiagram::new("D", DiagramKind::Class);
        let a = d.add_node(NodeKind::Class, Pos::default(), "A").unwrap();
        let n = d.nodes.iter_mut().find(|n| n.id == a).unwrap();
        n.attributes = "x\ny\nz\na\nb\nc".into();
        assert!(n.rect().h > n.bounds.h);
        let mut d = UmlDiagram::new("U", DiagramKind::UseCase);
        d.add_node(NodeKind::UseCase, Pos::default(), "U");
        let n = &d.nodes[0];
        assert_eq!(n.anchor(Pos::new(500.0, n.rect().center().y)), Pos::new(180.0, 42.5));
        assert!(!n.contains(Pos::default()));
    }

    #[test]
    fn clipboard_preserves_members_and_relations_with_fresh_identifiers() {
        let mut d = crate::examples::uml_classes();
        let original_ids: std::collections::HashSet<_> = d.nodes.iter().map(|n| n.id).chain(d.relations.iter().map(|r| r.id)).collect();
        let selected = d.nodes.iter().map(|n| n.id).collect();
        let fragment = d.fragment(&selected);
        let pasted = d.paste(&fragment, Pos::new(30.0, 40.0));
        assert_eq!(pasted.len(), fragment.nodes.len());
        for n in d.nodes.iter().filter(|n| pasted.contains(&n.id)) {
            assert!(!original_ids.contains(&n.id));
        }
        for (original, copy) in fragment.nodes.iter().zip(d.nodes.iter().filter(|n| pasted.contains(&n.id))) {
            assert_eq!(copy.attributes, original.attributes);
            assert_eq!(copy.operations, original.operations);
            assert_eq!(copy.bounds.x, original.bounds.x + 30.0);
            assert_eq!(copy.bounds.y, original.bounds.y + 40.0);
        }
        assert_eq!(d.relations.len(), fragment.relations.len() * 2);
        for r in &d.relations[fragment.relations.len()..] {
            assert!(pasted.contains(&r.from) && pasted.contains(&r.to));
            assert!(!original_ids.contains(&r.id));
        }
        let ids: std::collections::HashSet<_> = d.nodes.iter().map(|n| n.id).chain(d.relations.iter().map(|r| r.id)).collect();
        assert_eq!(ids.len(), d.nodes.len() + d.relations.len());
        assert!(d.validate().is_empty());
    }

    #[test]
    fn containment_is_consistent_for_partial_overlap_selection_and_paste() {
        let mut d = crate::examples::uml_use_cases();
        let system = d.nodes[0].id;
        let child = d.nodes[2].id;
        let selected = std::collections::BTreeSet::from([system, child]);
        let closure = d.selection_closure(&selected);
        assert_eq!(closure.len(), 4);
        assert!(!closure.contains(&d.nodes[1].id));
        let fragment = d.fragment(&selected);
        assert_eq!(fragment.nodes.len(), 4);
        assert_eq!(fragment.relations.len(), 2);
        let new = d.paste(&fragment, Pos::new(1200.0, 0.0));
        for n in d.nodes.iter().filter(|n| new.contains(&n.id) && n.kind == NodeKind::UseCase) {
            assert!(d.system_of(n).is_some_and(|s| new.contains(&s)));
        }
        d.nodes[2].bounds.x = d.nodes[0].bounds.right() - 10.0;
        assert_eq!(d.system_of(&d.nodes[2]), None, "a partially overlapping use case is not owned");
        assert!(!d.selection_closure(&std::collections::BTreeSet::from([system])).contains(&child));
    }

    #[test]
    fn rejects_inheritance_cycles_and_reports_invalid_imported_relations() {
        let mut d = UmlDiagram::new("Inheritance", DiagramKind::Class);
        let a = d.add_node(NodeKind::Class, Pos::default(), "A").unwrap();
        let b = d.add_node(NodeKind::Class, Pos::default(), "B").unwrap();
        let c = d.add_node(NodeKind::Class, Pos::default(), "C").unwrap();
        d.connect(a, b, RelationKind::Generalization).unwrap();
        d.connect(b, c, RelationKind::Generalization).unwrap();
        assert!(d.connect(c, a, RelationKind::Generalization).is_none());
        assert!(d.can_connect_excluding(b, a, RelationKind::Generalization, Some(d.relations[0].id)));
        let i = d.add_node(NodeKind::Interface, Pos::default(), "I").unwrap();
        assert!(d.connect(a, i, RelationKind::Generalization).is_none());
        assert!(d.connect(a, i, RelationKind::Realization).is_some());
        assert!(d.validate().is_empty());
        d.relations[0].source_multiplicity = "3..1".into();
        d.relations[1].to = 999;
        assert_eq!(d.validate().len(), 2);
        for value in ["", "1", "*", "0..1", "1..*", "2..12"] {
            assert!(valid_multiplicity(value));
        }
        for value in ["-1", "*..1", "4..2", "1...2", "one"] {
            assert!(!valid_multiplicity(value));
        }
    }

    #[test]
    fn plantuml_preserves_relation_direction_and_escapes_names() {
        let mut d = UmlDiagram::new("A\nB", DiagramKind::Class);
        let a = d.add_node(NodeKind::Class, Pos::default(), "A\"B").unwrap();
        let b = d.add_node(NodeKind::Interface, Pos::default(), "I").unwrap();
        d.connect(a, b, RelationKind::Realization).unwrap();
        let p = d.to_plantuml();
        assert!(p.contains("class \"A\\\"B\" as n1"));
        assert!(p.contains("n1 \"\" ..|> \"\" n2"));
        assert!(p.contains("title \"A\\nB\""));
    }
}
