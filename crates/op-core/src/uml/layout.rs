//! Automatic UML layout built on [`crate::layout`].
//!
//! Class diagrams flow top to bottom with parents above their children. Use-case
//! diagrams flow left to right from actors; each system boundary is arranged
//! as one composite node whose use cases are laid out inside it, so every use
//! case stays in the same system. Only geometry changes.
use super::*;
use crate::layout::{self, Direction, Options};
use std::collections::BTreeMap;

/// Free space inside a system around its use cases, below the title band.
const SYSTEM_PAD: f32 = 30.0;
const SYSTEM_TITLE: f32 = 30.0;

impl UmlDiagram {
    pub fn auto_arrange(&mut self, grid: Option<f32>) -> Result<(), String> {
        let members: BTreeMap<Id, Option<Id>> = self.nodes.iter().map(|n| (n.id, self.system_of(n))).collect();
        let index: BTreeMap<Id, usize> = self.nodes.iter().enumerate().map(|(i, n)| (n.id, i)).collect();
        let size = |n: &Node| (n.rect().w, n.rect().h);
        let edges = |ids: &dyn Fn(Id) -> Option<usize>| -> Vec<(usize, usize)> {
            self.relations
                .iter()
                .filter_map(|r| {
                    let (from, to) = self.layout_direction(r);
                    Some((ids(from)?, ids(to)?))
                })
                .collect()
        };
        let align = |v: f32| grid.map_or(v, |g| (v / g).ceil() * g);

        // Arrange each system's use cases first, relative to the system's corner.
        let mut inner: BTreeMap<Id, Vec<(usize, Pos)>> = BTreeMap::new();
        let mut outer_size: Vec<(f32, f32)> = self.nodes.iter().map(size).collect();
        for system in self.nodes.iter().filter(|n| n.kind == NodeKind::System) {
            let inside: Vec<usize> = self
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| members[&n.id] == Some(system.id))
                .map(|(i, _)| i)
                .collect();
            if inside.is_empty() {
                continue;
            }
            let local: BTreeMap<Id, usize> = inside.iter().enumerate().map(|(k, &i)| (self.nodes[i].id, k)).collect();
            let sizes: Vec<_> = inside.iter().map(|&i| size(&self.nodes[i])).collect();
            let options = Options {
                margin: SYSTEM_PAD,
                layer_gap: 60.0,
                node_gap: 30.0,
                ..Options::new(Direction::LeftToRight, grid)
            };
            let arranged = layout::layered_bands(&sizes, &edges(&|id| local.get(&id).copied()), &vec![0; sizes.len()], options)
                .map_err(|e| e.to_string())?;
            let width = inside
                .iter()
                .zip(&arranged.centers)
                .map(|(&i, c)| c.x + self.nodes[i].rect().w / 2.0 + SYSTEM_PAD)
                .fold(0.0, f32::max);
            let height = arranged.extents.first().copied().unwrap_or(0.0) + SYSTEM_TITLE;
            let minimum = system.minimum_size();
            outer_size[index[&system.id]] = (align(width.max(minimum.x)), align(height.max(minimum.y)));
            inner.insert(
                system.id,
                inside
                    .iter()
                    .zip(arranged.centers)
                    .map(|(&i, c)| (i, Pos::new(c.x, c.y + SYSTEM_TITLE)))
                    .collect(),
            );
        }

        // Then the outer graph, where a system stands for everything it contains.
        let top: Vec<usize> = self
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| members[&n.id].is_none())
            .map(|(i, _)| i)
            .collect();
        let outer: BTreeMap<Id, usize> = top.iter().enumerate().map(|(k, &i)| (self.nodes[i].id, k)).collect();
        let owner = |id: Id| outer.get(&members.get(&id).copied().flatten().unwrap_or(id)).copied();
        let sizes: Vec<_> = top.iter().map(|&i| outer_size[i]).collect();
        let direction = match self.kind {
            DiagramKind::Class => Direction::TopToBottom,
            DiagramKind::UseCase => Direction::LeftToRight,
        };
        let centers = layout::layered(&sizes, &edges(&owner), Options::new(direction, grid)).map_err(|e| e.to_string())?;

        let mut next = self.clone();
        for (&i, c) in top.iter().zip(centers) {
            let (w, h) = sizes[outer[&self.nodes[i].id]];
            let corner = Pos::new(c.x - w / 2.0, c.y - h / 2.0);
            let node = &mut next.nodes[i];
            if let Some(children) = inner.get(&node.id) {
                let corner = Pos::new(align(corner.x), align(corner.y));
                node.bounds = Rect::new(corner.x, corner.y, w, h);
                for &(child, at) in children {
                    let r = self.nodes[child].rect();
                    next.nodes[child].bounds.x = corner.x + at.x - r.w / 2.0;
                    next.nodes[child].bounds.y = corner.y + at.y - r.h / 2.0;
                }
            } else {
                node.bounds.x = corner.x;
                node.bounds.y = corner.y;
            }
        }
        if next
            .nodes
            .iter()
            .any(|n| ![n.bounds.x, n.bounds.y, n.bounds.w, n.bounds.h].iter().all(|v| v.is_finite()))
            || next.nodes.iter().any(|n| next.system_of(n) != members[&n.id])
        {
            return Err(tr("canvas.cannot_arrange_invalid_geometry").into());
        }
        *self = next;
        Ok(())
    }

    /// Relation endpoints in reading order: parents before children, wholes
    /// before parts, actors before use cases and base use cases before extensions.
    fn layout_direction(&self, r: &Relation) -> (Id, Id) {
        let actor = |id| self.node(id).is_some_and(|n| n.kind == NodeKind::Actor);
        match r.kind {
            RelationKind::Generalization | RelationKind::Realization | RelationKind::Extend => (r.to, r.from),
            RelationKind::Association if actor(r.to) && !actor(r.from) => (r.to, r.from),
            _ => (r.from, r.to),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn semantics(d: &UmlDiagram) -> UmlDiagram {
        let mut d = d.clone();
        for n in &mut d.nodes {
            n.bounds.x = 0.0;
            n.bounds.y = 0.0;
            if n.kind == NodeKind::System {
                n.bounds.w = 0.0;
                n.bounds.h = 0.0;
            }
        }
        d
    }

    fn assert_separated(d: &UmlDiagram, ignore: impl Fn(&Node, &Node) -> bool) {
        for (i, a) in d.nodes.iter().enumerate() {
            for b in &d.nodes[i + 1..] {
                if ignore(a, b) {
                    continue;
                }
                let (a, b) = (a.rect(), b.rect());
                assert!(a.right() <= b.x || b.right() <= a.x || a.bottom() <= b.y || b.bottom() <= a.y);
            }
        }
    }

    #[test]
    fn class_hierarchies_put_parents_above_children() {
        let mut d = UmlDiagram::new("Shapes", DiagramKind::Class);
        let shape = d.add_node(NodeKind::Class, Pos::default(), "Shape").unwrap();
        let circle = d.add_node(NodeKind::Class, Pos::default(), "Circle").unwrap();
        let square = d.add_node(NodeKind::Class, Pos::default(), "Square").unwrap();
        let canvas = d.add_node(NodeKind::Class, Pos::default(), "Canvas").unwrap();
        d.connect(circle, shape, RelationKind::Generalization).unwrap();
        d.connect(square, shape, RelationKind::Generalization).unwrap();
        d.connect(canvas, shape, RelationKind::Composition).unwrap();
        let business = semantics(&d);
        d.auto_arrange(Some(10.0)).unwrap();
        let y = |id| d.node(id).unwrap().rect().center().y;
        assert!(y(shape) < y(circle) && y(circle) == y(square));
        assert!(y(canvas) < y(shape), "the whole precedes its part");
        assert_separated(&d, |_, _| false);
        assert_eq!(semantics(&d), business);
        let once = d.clone();
        d.auto_arrange(Some(10.0)).unwrap();
        assert_eq!(d, once, "layout is idempotent");
    }

    #[test]
    fn use_cases_stay_inside_their_system_and_actors_come_first() {
        let mut d = UmlDiagram::new("Shop", DiagramKind::UseCase);
        let system = d.add_node(NodeKind::System, Pos::new(300.0, 0.0), "Shop").unwrap();
        let other = d.add_node(NodeKind::System, Pos::new(300.0, 600.0), "Bank").unwrap();
        let client = d.add_node(NodeKind::Actor, Pos::new(1200.0, 900.0), "Client").unwrap();
        let order = d.add_node(NodeKind::UseCase, Pos::new(340.0, 60.0), "Order").unwrap();
        let pay = d.add_node(NodeKind::UseCase, Pos::new(340.0, 200.0), "Pay").unwrap();
        let card = d.add_node(NodeKind::UseCase, Pos::new(340.0, 660.0), "Check card").unwrap();
        let loose = d.add_node(NodeKind::UseCase, Pos::new(-400.0, -400.0), "Browse").unwrap();
        d.connect(order, client, RelationKind::Association).unwrap();
        d.connect(order, pay, RelationKind::Include).unwrap();
        d.connect(pay, card, RelationKind::Include).unwrap();
        d.connect(client, loose, RelationKind::Association).unwrap();
        let owners: Vec<_> = d.nodes.iter().map(|n| d.system_of(n)).collect();
        assert_eq!(owners[3..6], [Some(system), Some(system), Some(other)]);
        let business = semantics(&d);
        for grid in [Some(10.0), None] {
            let mut d = d.clone();
            d.auto_arrange(grid).unwrap();
            assert_eq!(d.nodes.iter().map(|n| d.system_of(n)).collect::<Vec<_>>(), owners);
            assert_eq!(semantics(&d), business);
            let x = |id| d.node(id).unwrap().rect().center().x;
            assert!(x(client) < d.node(system).unwrap().rect().x);
            assert!(x(order) < x(pay), "an including use case precedes the included one");
            assert!(d.node(system).unwrap().rect().right() < d.node(other).unwrap().rect().x);
            // Systems enclose their use cases; everything else is separated.
            assert_separated(&d, |a, b| {
                [(a, b), (b, a)]
                    .iter()
                    .any(|(s, u)| s.kind == NodeKind::System && d.system_of(u) == Some(s.id))
            });
            if let Some(g) = grid {
                let system = d.node(system).unwrap().rect();
                assert_eq!(Pos::new(system.x, system.y), Pos::new(system.x, system.y).snapped(g));
            }
        }
    }
}
