//! Obstacle-aware orthogonal routing for UML relations.
//!
//! A visibility grid uses obstacle edges and connection ports as coordinates.
//! Dijkstra's algorithm minimizes length plus bends, with four candidate ports
//! at each endpoint. System boundaries are containers, not obstacles.

use super::{Id, Node, NodeKind, Relation, UmlDiagram};
use crate::geom::{Pos, Rect};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

const CLEARANCE: f32 = 16.0;
const BEND_COST: f32 = 28.0;

#[derive(Clone, Copy)]
struct Port {
    anchor: Pos,
    stub: Pos,
    axis: usize,
}

fn ports(n: &Node, lane: f32, shared_y: Option<f32>, shared_x: Option<f32>) -> Vec<Port> {
    let r = n.rect();
    let c = r.center();
    let mut ports = Vec::new();
    for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
        if n.kind == NodeKind::Actor && dy > 0.0 {
            continue;
        }
        let mut anchor = n.anchor(Pos::new(c.x + dx * r.w * 2.0, c.y + dy * r.h * 2.0));
        if n.kind != NodeKind::Actor {
            if dx != 0.0 {
                anchor.y = (shared_y.unwrap_or(c.y) + lane).clamp(r.y + r.h * 0.2, r.bottom() - r.h * 0.2);
                if n.kind == NodeKind::UseCase {
                    let ratio = (anchor.y - c.y) / (r.h / 2.0);
                    anchor.x = c.x + dx * (r.w / 2.0) * (1.0 - ratio * ratio).sqrt();
                }
            } else {
                anchor.x = (shared_x.unwrap_or(c.x) + lane).clamp(r.x + r.w * 0.2, r.right() - r.w * 0.2);
                if n.kind == NodeKind::UseCase {
                    let ratio = (anchor.x - c.x) / (r.w / 2.0);
                    anchor.y = c.y + dy * (r.h / 2.0) * (1.0 - ratio * ratio).sqrt();
                }
            }
        }
        let stub = if dx != 0.0 {
            Pos::new(if dx > 0.0 { r.right() + CLEARANCE } else { r.x - CLEARANCE }, anchor.y)
        } else {
            Pos::new(anchor.x, if dy > 0.0 { r.bottom() + CLEARANCE } else { r.y - CLEARANCE })
        };
        ports.push(Port {
            anchor,
            stub,
            axis: usize::from(dy != 0.0),
        });
    }
    ports
}

fn inside(r: Rect, p: Pos) -> bool {
    p.x > r.x + 0.01 && p.x < r.right() - 0.01 && p.y > r.y + 0.01 && p.y < r.bottom() - 0.01
}

fn inside_any(point: Pos, obstacles: &[Rect]) -> bool {
    obstacles.iter().any(|r| inside(*r, point))
}

fn clear_segment(a: Pos, b: Pos, obstacles: &[Rect]) -> bool {
    obstacles.iter().all(|r| {
        if (a.y - b.y).abs() < 0.01 {
            !(a.y > r.y + 0.01 && a.y < r.bottom() - 0.01 && a.x.min(b.x) < r.right() - 0.01 && a.x.max(b.x) > r.x + 0.01)
        } else {
            !(a.x > r.x + 0.01 && a.x < r.right() - 0.01 && a.y.min(b.y) < r.bottom() - 0.01 && a.y.max(b.y) > r.y + 0.01)
        }
    })
}

#[derive(Clone, Copy)]
struct Visit {
    cost: f32,
    state: usize,
}
impl PartialEq for Visit {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost && self.state == other.state
    }
}
impl Eq for Visit {}
impl PartialOrd for Visit {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Visit {
    fn cmp(&self, other: &Self) -> Ordering {
        other.cost.total_cmp(&self.cost).then_with(|| other.state.cmp(&self.state))
    }
}

fn simplified(points: Vec<Pos>) -> Vec<Pos> {
    let mut out: Vec<Pos> = Vec::new();
    for p in points {
        if out.last().is_some_and(|v| v.dist(p) < 0.01) {
            continue;
        }
        while out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            let same_x = (a.x - b.x).abs() < 0.01 && (b.x - p.x).abs() < 0.01;
            let same_y = (a.y - b.y).abs() < 0.01 && (b.y - p.y).abs() < 0.01;
            if (same_x || same_y) && (b.x - a.x) * (p.x - b.x) + (b.y - a.y) * (p.y - b.y) >= 0.0 {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}

pub fn route(diagram: &UmlDiagram, relation: &Relation) -> Vec<Pos> {
    let (Some(a), Some(b)) = (diagram.node(relation.from), diagram.node(relation.to)) else {
        return Vec::new();
    };
    let siblings: Vec<_> = diagram
        .relations
        .iter()
        .filter(|r| (r.from == relation.from && r.to == relation.to) || (r.from == relation.to && r.to == relation.from))
        .collect();
    let rank = siblings.iter().position(|r| r.id == relation.id).unwrap_or(0);
    let lane = (rank as f32 - (siblings.len().saturating_sub(1) as f32 / 2.0)) * 22.0;
    if a.id == b.id {
        let r = a.rect();
        let distance = 44.0 + rank as f32 * 24.0;
        return vec![
            Pos::new(r.right(), r.y + r.h * 0.3),
            Pos::new(r.right() + distance, r.y + r.h * 0.3),
            Pos::new(r.right() + distance, r.y - distance),
            Pos::new(r.center().x, r.y - distance),
            Pos::new(r.center().x, r.y),
        ];
    }
    // Align ports across overlapping projections to avoid tiny zigzags when
    // classes have different compartment heights or an actor is off-center.
    let projection = |n: &Node, horizontal: bool| {
        let r = n.rect();
        if horizontal && n.kind == NodeKind::Actor {
            return (r.y + 40.0, r.y + 40.0);
        }
        if horizontal {
            (r.y + r.h * 0.2, r.bottom() - r.h * 0.2)
        } else {
            (r.x + r.w * 0.2, r.right() - r.w * 0.2)
        }
    };
    let shared = |horizontal: bool| {
        let (alo, ahi) = projection(a, horizontal);
        let (blo, bhi) = projection(b, horizontal);
        let lo = alo.max(blo);
        let hi = ahi.min(bhi);
        (lo <= hi).then(|| ((alo + ahi + blo + bhi) / 4.0).clamp(lo, hi))
    };
    let from = ports(a, lane, shared(true), shared(false));
    let to = ports(b, lane, shared(true), shared(false));
    let obstacles: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|n| n.kind != NodeKind::System)
        .map(|n| {
            let r = n.rect();
            Rect::new(r.x - CLEARANCE, r.y - CLEARANCE, r.w + 2.0 * CLEARANCE, r.h + 2.0 * CLEARANCE)
        })
        .collect();
    // Most edits need only a straight line or one elbow. Avoid building a grid
    // when such a path already clears every element.
    let mut simple: Option<(f32, Vec<Pos>)> = None;
    for source in &from {
        for target in &to {
            for corner in [Pos::new(source.stub.x, target.stub.y), Pos::new(target.stub.x, source.stub.y)] {
                let path = simplified(vec![source.anchor, source.stub, corner, target.stub, target.anchor]);
                if !inside_any(source.stub, &obstacles)
                    && !inside_any(target.stub, &obstacles)
                    && clear_segment(source.stub, corner, &obstacles)
                    && clear_segment(corner, target.stub, &obstacles)
                {
                    let cost = path.windows(2).map(|w| w[0].dist(w[1])).sum::<f32>() + path.len().saturating_sub(2) as f32 * BEND_COST;
                    if simple.as_ref().is_none_or(|(best, _)| cost < *best) {
                        simple = Some((cost, path));
                    }
                }
            }
        }
    }
    if let Some((_, path)) = simple {
        return path;
    }
    let mut xs: Vec<f32> = obstacles.iter().flat_map(|r| [r.x, r.right()]).collect();
    let mut ys: Vec<f32> = obstacles.iter().flat_map(|r| [r.y, r.bottom()]).collect();
    for p in from.iter().chain(&to) {
        xs.push(p.stub.x);
        ys.push(p.stub.y);
    }
    for coords in [&mut xs, &mut ys] {
        coords.sort_by(f32::total_cmp);
        coords.dedup_by(|a, b| (*a - *b).abs() < 0.001);
    }
    let width = xs.len();
    let index = |p: Pos| -> usize {
        let x = xs.iter().position(|x| (*x - p.x).abs() < 0.001).unwrap();
        let y = ys.iter().position(|y| (*y - p.y).abs() < 0.001).unwrap();
        y * width + x
    };
    let point = |i: usize| Pos::new(xs[i % width], ys[i / width]);
    let available: Vec<bool> = (0..xs.len() * ys.len())
        .map(|i| !obstacles.iter().any(|r| inside(*r, point(i))))
        .collect();
    let mut costs = vec![f32::INFINITY; available.len() * 2];
    let mut previous = vec![None; costs.len()];
    let mut roots = vec![None; costs.len()];
    let mut queue = BinaryHeap::new();
    for (i, p) in from.iter().enumerate() {
        let state = index(p.stub) * 2 + p.axis;
        if !available[state / 2] {
            continue;
        }
        let cost = p.anchor.dist(p.stub);
        if cost < costs[state] {
            costs[state] = cost;
            roots[state] = Some(i);
            queue.push(Visit { cost, state });
        }
    }
    let targets: Vec<_> = to.iter().map(|p| index(p.stub)).collect();
    let mut best: Option<(f32, usize, usize)> = None;
    while let Some(Visit { cost, state }) = queue.pop() {
        if cost > costs[state] {
            continue;
        }
        if best.is_some_and(|(best, _, _)| cost > best) {
            break;
        }
        let node = state / 2;
        let axis = state % 2;
        for (i, target) in targets.iter().enumerate() {
            if *target == node {
                let total = cost + to[i].stub.dist(to[i].anchor) + if axis != to[i].axis { BEND_COST } else { 0.0 };
                if best.is_none_or(|(best, _, _)| total < best) {
                    best = Some((total, state, i));
                }
            }
        }
        let (x, y) = (node % width, node / width);
        let neighbors = [
            (x.checked_sub(1).map(|x| y * width + x), 0),
            ((x + 1 < width).then_some(node + 1), 0),
            (y.checked_sub(1).map(|y| y * width + x), 1),
            ((y + 1 < ys.len()).then_some(node + width), 1),
        ];
        for (next, next_axis) in neighbors {
            let Some(next) = next else {
                continue;
            };
            if !available[next] || !clear_segment(point(node), point(next), &obstacles) {
                continue;
            }
            let new_cost = cost + point(node).dist(point(next)) + if axis != next_axis { BEND_COST } else { 0.0 };
            let next_state = next * 2 + next_axis;
            if new_cost + 0.001 < costs[next_state] {
                costs[next_state] = new_cost;
                previous[next_state] = Some(state);
                queue.push(Visit {
                    cost: new_cost,
                    state: next_state,
                });
            }
        }
    }
    if let Some((_, mut state, target)) = best {
        let mut points = vec![to[target].anchor];
        loop {
            points.push(point(state / 2));
            if let Some(prev) = previous[state] {
                state = prev;
            } else {
                points.push(from[roots[state].unwrap()].anchor);
                break;
            }
        }
        points.reverse();
        simplified(points)
    } else {
        // Overlapping endpoints can make every port inaccessible; keep the model editable.
        vec![a.anchor(b.rect().center()), b.anchor(a.rect().center())]
    }
}

pub fn route_all(diagram: &UmlDiagram) -> HashMap<Id, Vec<Pos>> {
    diagram.relations.iter().map(|r| (r.id, route(diagram, r))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::uml::{DiagramKind, RelationKind};

    #[test]
    fn routes_around_obstacles_and_keeps_parallel_links_distinct() {
        let mut d = UmlDiagram::new("Routing", DiagramKind::Class);
        let a = d.add_node(NodeKind::Class, Pos::new(0.0, 100.0), "A").unwrap();
        let b = d.add_node(NodeKind::Class, Pos::new(700.0, 100.0), "B").unwrap();
        d.add_node(NodeKind::Class, Pos::new(340.0, 40.0), "Obstacle");
        d.nodes[2].bounds.h = 300.0;
        d.connect(a, b, RelationKind::Association);
        d.connect(b, a, RelationKind::Dependency);
        let p = route(&d, &d.relations[0]);
        let q = route(&d, &d.relations[1]);
        assert_ne!(p, q.into_iter().rev().collect::<Vec<_>>());
        assert!(p.len() >= 4);
        for w in p.windows(2) {
            assert!((w[0].x - w[1].x).abs() < 0.01 || (w[0].y - w[1].y).abs() < 0.01);
            assert!(clear_segment(w[0], w[1], &[d.nodes[2].rect()]));
        }
        assert!((p[0].x - d.nodes[0].rect().right()).abs() < 0.01 || (p[0].y - d.nodes[0].rect().y).abs() < 0.01);
    }

    #[test]
    fn routing_handles_use_cases_self_relations_and_overlapping_nodes() {
        let mut d = crate::examples::uml_use_cases();
        for r in &d.relations {
            let points = route(&d, r);
            assert!(points.len() >= 2);
            assert!(points.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
        }
        d.nodes[2].bounds = d.nodes[3].bounds;
        assert!(!route(&d, &d.relations[1]).is_empty());
        let mut d = UmlDiagram::new("Self", DiagramKind::Class);
        let id = d.add_node(NodeKind::Class, Pos::default(), "A").unwrap();
        d.connect(id, id, RelationKind::Association);
        assert_eq!(route(&d, &d.relations[0]).len(), 5);
    }
}
