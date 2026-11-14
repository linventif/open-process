//! Deterministic left-to-right layout. Only presentation changes: all node/flow
//! identities, properties and original pool/lane memberships are preserved.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const PAD: f32 = 40.0;
const COLUMN_GAP: f32 = 80.0;
const ROW_GAP: f32 = 60.0;

/// Align node centers and pool/lane geometry to `grid` when snapping is enabled.
pub fn auto_arrange(diagram: &mut BpmnDiagram, grid: Option<f32>) -> Result<(), String> {
    if grid.is_some_and(|step| !step.is_finite() || step <= 0.0)
        || diagram
            .nodes
            .iter()
            .any(|n| ![n.pos.x, n.pos.y, n.w, n.h].iter().all(|v| v.is_finite()) || n.w <= 0.0 || n.h <= 0.0)
        || diagram.pools.iter().any(|p| {
            ![p.pos.x, p.pos.y, p.width, p.height()].iter().all(|v| v.is_finite())
                || p.width <= POOL_HEADER
                || p.lanes.iter().any(|l| !l.height.is_finite() || l.height <= 0.0)
        })
    {
        return Err(tr("bpmn.cannot_arrange_diagram_invalid_geometry").into());
    }
    if diagram.pools.iter().any(|p| p.lanes.is_empty()) {
        return Err(tr("bpmn.add_lane_each_pool_before_arranging").into());
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
        return Err(tr("bpmn.move_elements_out_pool_headers_separate_overlapping").into());
    }
    let info: BTreeMap<_, _> = diagram.nodes.iter().map(|n| (n.id, n)).collect();
    if info.len() != diagram.nodes.len()
        || diagram
            .flows
            .iter()
            .any(|f| !info.contains_key(&f.from) || !info.contains_key(&f.to))
    {
        return Err(tr("bpmn.cannot_arrange_diagram_invalid_element_references").into());
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
    // Round outward so snapping cannot consume the spacing around a shape.
    let align = |value: f32| grid.map_or(value, |step| (value / step).ceil() * step);
    let mut centers = vec![];
    let mut x = PAD;
    for width in widths {
        let center = align(x + width / 2.0);
        centers.push(center);
        x = center + width / 2.0 + COLUMN_GAP;
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
    let place_rows = |groups: &BTreeMap<usize, Vec<Id>>, origin: Pos, positions: &mut BTreeMap<Id, Pos>| {
        let mut heights = vec![0.0f32; groups.values().map(Vec::len).max().unwrap_or(0)];
        for ids in groups.values() {
            for (row, id) in ids.iter().enumerate() {
                heights[row] = heights[row].max(info[id].h);
            }
        }
        let mut rows = Vec::with_capacity(heights.len());
        let mut bottom = origin.y;
        let mut y = origin.y + PAD;
        for height in heights {
            let center = align(y + height / 2.0);
            rows.push(center);
            bottom = center + height / 2.0;
            y = bottom + ROW_GAP;
        }
        for (&rank, ids) in groups {
            for (row, &id) in ids.iter().enumerate() {
                positions.insert(id, Pos::new(origin.x + centers[rank], rows[row]));
            }
        }
        align((bottom + PAD - origin.y).max(140.0))
    };
    let mut positions = BTreeMap::new();
    let mut next = diagram.clone();
    let mut pool_y = PAD;
    for pool in &mut next.pools {
        pool.pos = Pos::new(align(PAD), align(pool_y));
        let body_x = align(pool.pos.x + POOL_HEADER);
        pool.width = align(pool.width.max(body_x - pool.pos.x + body_width));
        let mut lane_y = pool.pos.y;
        for lane in &mut pool.lanes {
            let groups = group_columns(
                owners
                    .iter()
                    .filter(|(_, (_, owner))| *owner == Some((pool.id, lane.id)))
                    .map(|(&id, _)| id)
                    .collect(),
            );
            let required = place_rows(&groups, Pos::new(body_x, lane_y), &mut positions);
            lane.height = align(lane.height.max(required));
            lane_y += lane.height;
        }
        pool_y = lane_y + 2.0 * PAD;
    }
    let unpooled = group_columns(owners.iter().filter(|(_, owner)| owner.0.is_none()).map(|(&id, _)| id).collect());
    place_rows(&unpooled, Pos::new(align(PAD), align(pool_y)), &mut positions);
    for n in &mut next.nodes {
        n.pos = positions[&n.id];
    }
    if next.nodes.iter().any(|n| !n.pos.x.is_finite() || !n.pos.y.is_finite())
        || next
            .pools
            .iter()
            .any(|p| ![p.pos.x, p.pos.y, p.width, p.height()].iter().all(|v| v.is_finite()))
    {
        return Err(tr("bpmn.cannot_arrange_diagram_invalid_geometry").into());
    }
    if next.nodes.iter().any(|n| owners[&n.id] != (next.pool_of(n.id), next.lane_of(n.id))) {
        return Err(tr("bpmn.move_elements_out_pool_headers_separate_overlapping").into());
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
    fn mixed_shapes_align_centers_and_keep_flow_endpoints_on_their_shapes() {
        let mut d = BpmnDiagram::new("Mixed shapes");
        let ids = [
            d.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::default()),
            d.add_node(NodeKind::Task(TaskType::User), Pos::default()),
            d.add_node(NodeKind::Gateway(GatewayType::Parallel), Pos::default()),
            d.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::default()),
        ];
        d.node_mut(ids[1]).unwrap().w = 111.3;
        d.node_mut(ids[1]).unwrap().h = 73.4;
        for pair in ids.windows(2) {
            d.add_flow(pair[0], pair[1]).unwrap();
        }
        let business = semantics(&d);
        auto_arrange(&mut d, Some(10.0)).unwrap();
        assert!(d.nodes.iter().all(|n| n.pos.y == d.nodes[0].pos.y));
        assert!(d.nodes.iter().all(|n| n.pos == n.pos.snapped(10.0)));
        assert_separated(&d);
        assert_eq!(semantics(&d), business);
        for flow in &d.flows {
            let a = d.node(flow.from).unwrap();
            let b = d.node(flow.to).unwrap();
            assert_eq!(
                route(&d, flow),
                vec![Pos::new(a.rect().right(), a.pos.y), Pos::new(b.rect().x, b.pos.y)]
            );
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
        auto_arrange(&mut d, Some(10.0)).unwrap();
        assert_eq!(semantics(&d), business);
        assert_separated(&d);
        for (id, pool, lane) in owners {
            assert_eq!(d.pool_of(id), pool);
            assert_eq!(d.lane_of(id), lane);
        }
        let once = d.clone();
        auto_arrange(&mut d, Some(10.0)).unwrap();
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
        auto_arrange(&mut d, Some(10.0)).unwrap();
        assert_separated(&d);
        assert_eq!(semantics(&d), business);
        let once = d.clone();
        auto_arrange(&mut d, Some(10.0)).unwrap();
        assert_eq!(d, once);
    }
    #[test]
    fn ambiguous_pool_headers_are_rejected_without_partial_changes() {
        let mut d = BpmnDiagram::new("header");
        d.add_pool(Pos::default());
        d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(10.0, 80.0));
        let original = d.clone();
        assert!(auto_arrange(&mut d, Some(10.0)).is_err());
        assert_eq!(d, original);
    }

    #[test]
    fn fractional_sizes_snap_without_overlaps_or_changing_lane_membership() {
        use crate::{geom::Port, style::EdgeStyle};
        let mut original = crate::examples::order_process();
        original.pools[0].width += 0.3;
        original.pools[0].lanes[0].height += 0.7;
        original.pools[0].lanes[1].height += 3.1;
        let other = original.add_pool(Pos::new(1600.0, 0.0));
        original.add_lane(other);
        original.add_node(NodeKind::Task(TaskType::Receive), Pos::new(1800.0, 80.0));
        original.add_node(NodeKind::SubProcess, Pos::new(1800.0, 250.0));
        original.add_node(NodeKind::Task(TaskType::Manual), Pos::new(-200.0, -100.0));
        for n in original
            .nodes
            .iter_mut()
            .filter(|n| matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess))
        {
            n.w += 13.7;
            n.h += 27.3;
        }
        let owners: Vec<_> = original
            .nodes
            .iter()
            .map(|n| (n.id, original.pool_of(n.id), original.lane_of(n.id)))
            .collect();
        for grid in [10.0, 7.5, 32.0] {
            let mut d = original.clone();
            auto_arrange(&mut d, Some(grid)).unwrap();
            assert_eq!(semantics(&d), semantics(&original));
            assert_separated(&d);
            for n in &d.nodes {
                assert!(n.pos.dist(n.pos.snapped(grid)) < 0.001);
            }
            for pool in &d.pools {
                assert!(pool.pos.dist(pool.pos.snapped(grid)) < 0.001);
                assert!((pool.width / grid - (pool.width / grid).round()).abs() < 0.001);
                for (i, lane) in pool.lanes.iter().enumerate() {
                    assert!((lane.height / grid - (lane.height / grid).round()).abs() < 0.001);
                    let bounds = pool.lane_rect(i);
                    for n in d.nodes.iter().filter(|n| d.lane_of(n.id) == Some((pool.id, lane.id))) {
                        assert!(bounds.contains(Pos::new(n.rect().x, n.rect().y)));
                        assert!(bounds.contains(Pos::new(n.rect().right(), n.rect().bottom())));
                    }
                }
            }
            for &(id, pool, lane) in &owners {
                assert_eq!((d.pool_of(id), d.lane_of(id)), (pool, lane));
            }
            let once = d.clone();
            auto_arrange(&mut d, Some(grid)).unwrap();
            assert_eq!(d, once);
            for style in [EdgeStyle::Straight, EdgeStyle::Orthogonal, EdgeStyle::Curved] {
                d.style.edges = style;
                for flow in &d.flows {
                    let points = route(&d, flow);
                    for (id, point) in [(flow.from, points[0]), (flow.to, *points.last().unwrap())] {
                        let n = d.node(id).unwrap();
                        if n.kind.is_event() {
                            assert!((n.pos.dist(point) - n.w / 2.0).abs() < 0.001);
                        } else {
                            assert!(Port::ALL.iter().any(|port| port.point(n.rect()).dist(point) < 0.001));
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn disabling_snapping_keeps_fractional_centers_and_node_sizes() {
        let mut d = BpmnDiagram::new("Unsnapped");
        let id = d.add_node(NodeKind::Task(TaskType::Manual), Pos::default());
        d.node_mut(id).unwrap().w = 111.3;
        d.node_mut(id).unwrap().h = 73.4;
        auto_arrange(&mut d, None).unwrap();
        assert_ne!(d.nodes[0].pos, d.nodes[0].pos.snapped(10.0));
        assert_eq!((d.nodes[0].w, d.nodes[0].h), (111.3, 73.4));
        let once = d.clone();
        auto_arrange(&mut d, None).unwrap();
        assert_eq!(d, once);
    }

    #[test]
    fn invalid_grid_steps_leave_the_diagram_unchanged() {
        let original = crate::examples::order_process();
        for grid in [0.0, -10.0, f32::NAN, f32::INFINITY] {
            let mut d = original.clone();
            assert!(auto_arrange(&mut d, Some(grid)).is_err());
            assert_eq!(d, original);
        }
    }
}
