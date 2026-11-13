//! Deterministic left-to-right layout. Only presentation changes: all node/flow
//! identities, properties and original pool/lane memberships are preserved.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const PAD: f32 = 40.0;
const COLUMN_GAP: f32 = 80.0;
const ROW_GAP: f32 = 60.0;

pub fn auto_arrange(diagram: &mut BpmnDiagram) -> Result<(), String> {
    if diagram
        .nodes
        .iter()
        .any(|n| ![n.pos.x, n.pos.y, n.w, n.h].iter().all(|v| v.is_finite()) || n.w <= 0.0 || n.h <= 0.0)
        || diagram.pools.iter().any(|p| {
            ![p.pos.x, p.pos.y, p.width, p.height()].iter().all(|v| v.is_finite())
                || p.width <= POOL_HEADER
                || p.lanes.iter().any(|l| !l.height.is_finite() || l.height <= 0.0)
        })
    {
        return Err(tr("Cannot arrange a diagram with invalid geometry.").into());
    }
    if diagram.pools.iter().any(|p| p.lanes.is_empty()) {
        return Err(tr("Add a lane to each pool before arranging.").into());
    }
    let owners: BTreeMap<_, _> = diagram
        .nodes
        .iter()
        .map(|n| (n.id, (diagram.pool_of(n.id), diagram.lane_of(n.id))))
        .collect();
    if owners
        .values()
        .any(|(pool, lane)| pool.is_some() && lane.is_none() || lane.is_some_and(|(id, _)| Some(id) != *pool))
    {
        return Err(tr("Move elements out of pool headers and separate overlapping pools before arranging.").into());
    }
    let info: BTreeMap<_, _> = diagram.nodes.iter().map(|n| (n.id, n)).collect();
    if info.len() != diagram.nodes.len()
        || diagram
            .flows
            .iter()
            .any(|f| !info.contains_key(&f.from) || !info.contains_key(&f.to))
    {
        return Err(tr("Cannot arrange a diagram with invalid element references.").into());
    }
    let mut ranks = flow_ranks(diagram);
    let orphan_rank = ranks.values().copied().max().map_or(0, |rank| rank + 1);
    for n in diagram.nodes.iter().filter(|n| !n.kind.is_flow_node()) {
        let rank = diagram
            .flows
            .iter()
            .filter(|f| f.kind == FlowKind::Association)
            .filter_map(|f| {
                let other = if f.from == n.id {
                    f.to
                } else if f.to == n.id {
                    f.from
                } else {
                    return None;
                };
                info.get(&other)
                    .filter(|n| n.kind.is_flow_node())
                    .and_then(|_| ranks.get(&other))
                    .copied()
            })
            .min()
            .unwrap_or(orphan_rank);
        ranks.insert(n.id, rank);
    }
    let columns = ranks.values().copied().max().map_or(0, |rank| rank + 1);
    let mut widths = vec![0.0f32; columns];
    for n in &diagram.nodes {
        widths[ranks[&n.id]] = widths[ranks[&n.id]].max(n.w);
    }
    let mut centers = vec![];
    let mut x = PAD;
    for width in widths {
        centers.push(x + width / 2.0);
        x += width + COLUMN_GAP;
    }
    let body_width = if columns == 0 { 2.0 * PAD } else { x - COLUMN_GAP + PAD };
    let group_columns = |ids: Vec<Id>| {
        let mut groups: BTreeMap<usize, Vec<Id>> = BTreeMap::new();
        for id in ids {
            groups.entry(ranks[&id]).or_default().push(id);
        }
        for ids in groups.values_mut() {
            ids.sort_unstable();
        }
        groups
    };
    let required_height = |groups: &BTreeMap<usize, Vec<Id>>| {
        groups
            .values()
            .map(|ids| ids.iter().map(|id| info[id].h).sum::<f32>() + ROW_GAP * ids.len().saturating_sub(1) as f32 + 2.0 * PAD)
            .fold(140.0, f32::max)
    };
    let mut positions = BTreeMap::new();
    let mut next = diagram.clone();
    let mut pool_y = PAD;
    for pool in &mut next.pools {
        pool.pos = Pos::new(PAD, pool_y);
        pool.width = pool.width.max(body_width + POOL_HEADER);
        let mut lane_y = pool_y;
        for lane in &mut pool.lanes {
            let groups = group_columns(
                owners
                    .iter()
                    .filter(|(_, (_, owner))| *owner == Some((pool.id, lane.id)))
                    .map(|(&id, _)| id)
                    .collect(),
            );
            lane.height = lane.height.max(required_height(&groups));
            for (rank, ids) in groups {
                let mut y = lane_y + PAD;
                for id in ids {
                    let n = info[&id];
                    positions.insert(id, Pos::new(pool.pos.x + POOL_HEADER + centers[rank], y + n.h / 2.0));
                    y += n.h + ROW_GAP;
                }
            }
            lane_y += lane.height;
        }
        pool_y = lane_y + 2.0 * PAD;
    }
    let unpooled = group_columns(owners.iter().filter(|(_, owner)| owner.0.is_none()).map(|(&id, _)| id).collect());
    for (rank, ids) in unpooled {
        let mut y = pool_y + PAD;
        for id in ids {
            let n = info[&id];
            positions.insert(id, Pos::new(PAD + centers[rank], y + n.h / 2.0));
            y += n.h + ROW_GAP;
        }
    }
    for n in &mut next.nodes {
        n.pos = positions[&n.id];
    }
    if next.nodes.iter().any(|n| !n.pos.x.is_finite() || !n.pos.y.is_finite())
        || next
            .pools
            .iter()
            .any(|p| ![p.pos.x, p.pos.y, p.width, p.height()].iter().all(|v| v.is_finite()))
    {
        return Err(tr("Cannot arrange a diagram with invalid geometry.").into());
    }
    if next.nodes.iter().any(|n| owners[&n.id] != (next.pool_of(n.id), next.lane_of(n.id))) {
        return Err(tr("Move elements out of pool headers and separate overlapping pools before arranging.").into());
    }
    *diagram = next;
    Ok(())
}

/// Kahn layering, with a stable break when only a cyclic component remains.
/// Back edges stay valid and return to an earlier column; layout always terminates.
fn flow_ranks(d: &BpmnDiagram) -> BTreeMap<Id, usize> {
    let mut remaining: BTreeSet<_> = d.nodes.iter().filter(|n| n.kind.is_flow_node()).map(|n| n.id).collect();
    let mut incoming: BTreeMap<_, _> = remaining.iter().map(|&id| (id, 0usize)).collect();
    let mut outgoing: BTreeMap<Id, Vec<Id>> = BTreeMap::new();
    for f in d.flows.iter().filter(|f| f.kind == FlowKind::Sequence) {
        if remaining.contains(&f.from) && remaining.contains(&f.to) {
            *incoming.get_mut(&f.to).unwrap() += 1;
            outgoing.entry(f.from).or_default().push(f.to);
        }
    }
    let mut ready: BTreeSet<_> = incoming.iter().filter(|(_, count)| **count == 0).map(|(&id, _)| id).collect();
    let mut ranks: BTreeMap<_, _> = remaining.iter().map(|&id| (id, 0)).collect();
    while !remaining.is_empty() {
        let id = ready.pop_first().unwrap_or_else(|| *remaining.first().unwrap());
        remaining.remove(&id);
        for &to in outgoing.get(&id).into_iter().flatten() {
            if remaining.contains(&to) {
                ranks.insert(to, ranks[&to].max(ranks[&id] + 1));
                let count = incoming.get_mut(&to).unwrap();
                *count = count.saturating_sub(1);
                if *count == 0 {
                    ready.insert(to);
                }
            }
        }
    }
    ranks
}

#[cfg(test)]
mod tests {
    use super::*;
    fn semantics(d: &BpmnDiagram) -> BpmnDiagram {
        let mut d = d.clone();
        for n in &mut d.nodes {
            n.pos = Pos::default();
        }
        for p in &mut d.pools {
            p.pos = Pos::default();
            p.width = 0.0;
            for l in &mut p.lanes {
                l.height = 0.0;
            }
        }
        d
    }
    fn assert_separated(d: &BpmnDiagram) {
        for (i, a) in d.nodes.iter().enumerate() {
            for b in &d.nodes[i + 1..] {
                let a = a.rect();
                let b = b.rect();
                assert!(a.right() <= b.x || b.right() <= a.x || a.bottom() <= b.y || b.bottom() <= a.y);
            }
        }
    }
    #[test]
    fn pools_lanes_and_business_properties_survive_arrangement() {
        let mut d = crate::examples::order_process();
        d.nodes[0].documentation = "Keep this note".into();
        d.nodes[1].cost = 12.0;
        d.nodes[1].duration = 3.0;
        let owners: Vec<_> = d.nodes.iter().map(|n| (n.id, d.pool_of(n.id), d.lane_of(n.id))).collect();
        let business = semantics(&d);
        auto_arrange(&mut d).unwrap();
        assert_eq!(semantics(&d), business);
        assert_separated(&d);
        for (id, pool, lane) in owners {
            assert_eq!(d.pool_of(id), pool);
            assert_eq!(d.lane_of(id), lane);
        }
        let once = d.clone();
        auto_arrange(&mut d).unwrap();
        assert_eq!(d, once, "layout is idempotent");
    }
    #[test]
    fn disconnected_nodes_cycles_and_associations_are_deterministic() {
        let mut d = BpmnDiagram::new("cycle");
        let a = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(0.0, 0.0));
        let b = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(0.0, 0.0));
        d.add_flow(a, b).unwrap();
        d.add_flow(b, a).unwrap();
        d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(0.0, 0.0));
        let note = d.add_node(NodeKind::TextAnnotation, Pos::new(0.0, 0.0));
        d.add_flow(a, note).unwrap();
        let business = semantics(&d);
        auto_arrange(&mut d).unwrap();
        assert_separated(&d);
        assert_eq!(semantics(&d), business);
        let once = d.clone();
        auto_arrange(&mut d).unwrap();
        assert_eq!(d, once);
    }
    #[test]
    fn ambiguous_pool_headers_are_rejected_without_partial_changes() {
        let mut d = BpmnDiagram::new("header");
        d.add_pool(Pos::default());
        d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(10.0, 80.0));
        let original = d.clone();
        assert!(auto_arrange(&mut d).is_err());
        assert_eq!(d, original);
    }
}
