//! Pool edges and shared lane boundaries, independent of canvas coordinates.

use super::*;

pub const MIN_POOL_WIDTH: f32 = 200.0;
pub const MIN_LANE_HEIGHT: f32 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolResize {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    /// The shared boundary below this lane; its next neighbor absorbs the change.
    LaneBoundary(usize),
}

impl PoolResize {
    pub fn edge(self) -> Option<crate::geom::ResizeEdge> {
        use crate::geom::ResizeEdge as E;
        Some(match self {
            Self::Left => E::Left,
            Self::Right => E::Right,
            Self::Top => E::Top,
            Self::Bottom => E::Bottom,
            Self::TopLeft => E::TopLeft,
            Self::TopRight => E::TopRight,
            Self::BottomLeft => E::BottomLeft,
            Self::BottomRight => E::BottomRight,
            Self::LaneBoundary(_) => return None,
        })
    }

    /// Reference point used to retain the pointer's grab offset during a gesture.
    pub fn anchor(self, pool: &Pool) -> Option<Pos> {
        let r = pool.rect();
        let c = r.center();
        Some(match self {
            Self::Left => Pos::new(r.x, c.y),
            Self::Right => Pos::new(r.right(), c.y),
            Self::Top => Pos::new(c.x, r.y),
            Self::Bottom => Pos::new(c.x, r.bottom()),
            Self::TopLeft => Pos::new(r.x, r.y),
            Self::TopRight => Pos::new(r.right(), r.y),
            Self::BottomLeft => Pos::new(r.x, r.bottom()),
            Self::BottomRight => Pos::new(r.right(), r.bottom()),
            Self::LaneBoundary(i) if i < pool.lanes.len().saturating_sub(1) => Pos::new(c.x, pool.lane_rect(i).bottom()),
            Self::LaneBoundary(_) => return None,
        })
    }
}

impl From<crate::geom::ResizeEdge> for PoolResize {
    fn from(edge: crate::geom::ResizeEdge) -> Self {
        use crate::geom::ResizeEdge as E;
        match edge {
            E::Left => Self::Left,
            E::Right => Self::Right,
            E::Top => Self::Top,
            E::Bottom => Self::Bottom,
            E::TopLeft => Self::TopLeft,
            E::TopRight => Self::TopRight,
            E::BottomLeft => Self::BottomLeft,
            E::BottomRight => Self::BottomRight,
        }
    }
}

/// Constrain a moving border and, when possible, keep it on the requested grid.
/// Include the current border in the limits to tolerate undersized imported geometry.
fn constrain(value: f32, min: f32, max: f32, current: f32, grid: Option<f32>) -> f32 {
    let min = min.min(current);
    let max = max.max(current);
    if let Some(step) = grid {
        let lo = (min / step).ceil();
        let hi = (max / step).floor();
        if lo > hi {
            return current;
        }
        (value / step).round().clamp(lo, hi) * step
    } else {
        value.clamp(min, max)
    }
}

fn in_lane(index: usize, bounds: Rect, position: Pos) -> bool {
    // A center exactly on a separator belongs to the lane above, as in `lane_at`.
    bounds.contains(position) && (index == 0 || position.y > bounds.y)
}

impl BpmnDiagram {
    /// Resize a pool without moving its elements or cutting into their bounds.
    /// The top/bottom edges affect the first/last lane; an internal boundary
    /// redistributes two adjacent heights, keeping the pool and other lanes fixed.
    pub fn resize_pool(&mut self, id: Id, target: PoolResize, position: Pos, grid: Option<f32>) {
        if ![position.x, position.y].iter().all(|v| v.is_finite()) || grid.is_some_and(|step| !step.is_finite() || step <= 0.0) {
            return;
        }
        let Some(pool) = self.pool(id) else { return };
        if pool.lanes.is_empty()
            || ![pool.pos.x, pool.pos.y, pool.width, pool.height()].iter().all(|v| v.is_finite())
            || pool.width <= 0.0
            || pool.lanes.iter().any(|l| !l.height.is_finite() || l.height <= 0.0)
        {
            return;
        }
        let r = pool.rect();
        let mut left_max = r.right() - MIN_POOL_WIDTH;
        let mut right_min = r.x + MIN_POOL_WIDTH;
        let first_lane = pool.lane_rect(0);
        let mut top_max = first_lane.bottom() - MIN_LANE_HEIGHT;
        let last = pool.lanes.len() - 1;
        let last_lane = pool.lane_rect(last);
        let mut bottom_min = last_lane.y + MIN_LANE_HEIGHT;
        let header = POOL_HEADER + if last > 0 { LANE_HEADER } else { 0.0 };
        let boundary = match target {
            PoolResize::LaneBoundary(i) if i < last => Some((i, pool.lane_rect(i), pool.lane_rect(i + 1))),
            PoolResize::LaneBoundary(_) => return,
            _ => None,
        };
        let mut boundary_min = boundary.map_or(0.0, |(_, above, _)| above.y + MIN_LANE_HEIGHT);
        let mut boundary_max = boundary.map_or(0.0, |(_, _, below)| below.bottom() - MIN_LANE_HEIGHT);
        for node in self.nodes.iter().filter(|n| r.contains(n.pos)) {
            let nr = node.rect();
            left_max = left_max.min(nr.x - header);
            right_min = right_min.max(nr.right());
            if in_lane(0, first_lane, node.pos) {
                top_max = top_max.min(nr.y);
            }
            if in_lane(last, last_lane, node.pos) {
                bottom_min = bottom_min.max(nr.bottom());
            }
            if let Some((i, above, below)) = boundary {
                if in_lane(i, above, node.pos) {
                    boundary_min = boundary_min.max(nr.bottom());
                } else if in_lane(i + 1, below, node.pos) {
                    boundary_max = boundary_max.min(nr.y);
                }
            }
        }
        let mut left = r.x;
        let mut right = r.right();
        let mut top = r.y;
        let mut bottom = r.bottom();
        use PoolResize::*;
        if matches!(target, Left | TopLeft | BottomLeft) {
            left = constrain(position.x, f32::NEG_INFINITY, left_max, r.x, grid);
        }
        if matches!(target, Right | TopRight | BottomRight) {
            right = constrain(position.x, right_min, f32::INFINITY, r.right(), grid);
        }
        if matches!(target, Top | TopLeft | TopRight) {
            top = constrain(position.y, f32::NEG_INFINITY, top_max, r.y, grid);
        }
        if matches!(target, Bottom | BottomLeft | BottomRight) {
            bottom = constrain(position.y, bottom_min, f32::INFINITY, r.bottom(), grid);
        }
        let pool = self.pool_mut(id).unwrap();
        if let Some((i, above, below)) = boundary {
            let y = constrain(position.y, boundary_min, boundary_max, above.bottom(), grid);
            pool.lanes[i].height = y - above.y;
            pool.lanes[i + 1].height = below.bottom() - y;
        } else {
            if matches!(target, Left | Right | TopLeft | TopRight | BottomLeft | BottomRight) {
                pool.pos.x = left;
                pool.width = right - left;
            }
            if matches!(target, Top | TopLeft | TopRight) {
                pool.pos.y = top;
                pool.lanes[0].height += r.y - top;
            }
            if matches!(target, Bottom | BottomLeft | BottomRight) {
                let last_top = pool.pos.y + pool.lanes[..last].iter().map(|l| l.height).sum::<f32>();
                pool.lanes[last].height = bottom - last_top;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagram() -> (BpmnDiagram, Id) {
        let mut d = BpmnDiagram::new("Resize pools");
        let id = d.add_pool(Pos::new(100.0, 100.0));
        d.add_lane(id);
        d.add_lane(id);
        let p = d.pool_mut(id).unwrap();
        p.width = 600.0;
        for lane in &mut p.lanes {
            lane.height = 160.0;
        }
        (d, id)
    }

    #[test]
    fn all_pool_edges_and_corners_keep_opposite_edges_and_other_lanes_fixed() {
        use PoolResize::*;
        for target in [Left, Right, Top, Bottom, TopLeft, TopRight, BottomLeft, BottomRight] {
            let (mut d, id) = diagram();
            let original = d.pool(id).unwrap().clone();
            let anchor = target.anchor(&original).unwrap();
            let dx = if matches!(target, Left | TopLeft | BottomLeft) {
                -43.0
            } else {
                43.0
            };
            let dy = if matches!(target, Top | TopLeft | TopRight) { -27.0 } else { 27.0 };
            d.resize_pool(id, target, anchor + Pos::new(dx, dy), Some(10.0));
            let p = d.pool(id).unwrap();
            let r = p.rect();
            assert_eq!(
                r.x,
                if matches!(target, Left | TopLeft | BottomLeft) {
                    60.0
                } else {
                    100.0
                }
            );
            assert_eq!(
                r.right(),
                if matches!(target, Right | TopRight | BottomRight) {
                    740.0
                } else {
                    700.0
                }
            );
            assert_eq!(r.y, if matches!(target, Top | TopLeft | TopRight) { 70.0 } else { 100.0 });
            assert_eq!(
                r.bottom(),
                if matches!(target, Bottom | BottomLeft | BottomRight) {
                    610.0
                } else {
                    580.0
                }
            );
            assert_eq!(p.lanes[1], original.lanes[1]);
            assert_eq!(p.name, original.name);
            let resized = d.clone();
            d.resize_pool(id, target, anchor + Pos::new(dx, dy), Some(10.0));
            assert_eq!(d, resized, "repeated drag frames are stable: {target:?}");
            d.check_storage().unwrap();
        }
    }

    #[test]
    fn lane_boundary_redistributes_neighbors_and_respects_minimum_heights() {
        let (mut d, id) = diagram();
        let original = d.pool(id).unwrap().clone();
        d.resize_pool(id, PoolResize::LaneBoundary(0), Pos::new(999.0, 297.5), None);
        let p = d.pool(id).unwrap();
        assert_eq!(p.lanes[0].height, 197.5);
        assert_eq!(p.lanes[1].height, 122.5);
        assert_eq!(p.lanes[2], original.lanes[2]);
        assert_eq!(p.rect(), original.rect());
        for (requested, expected) in [(-1000.0, [60.0, 260.0]), (1000.0, [260.0, 60.0])] {
            d.resize_pool(id, PoolResize::LaneBoundary(0), Pos::new(0.0, requested), Some(10.0));
            let p = d.pool(id).unwrap();
            assert_eq!([p.lanes[0].height, p.lanes[1].height], expected);
            assert_eq!(p.rect(), original.rect());
        }
        d.check_storage().unwrap();
    }

    #[test]
    fn pool_and_lane_shrinking_stops_at_elements_without_moving_or_reassigning_them() {
        let (mut d, id) = diagram();
        let a = d.add_node(NodeKind::Task(TaskType::User), Pos::new(240.0, 205.0));
        let b = d.add_node(NodeKind::Task(TaskType::Manual), Pos::new(580.0, 330.0));
        let c = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(450.0, 505.0));
        d.add_flow(a, b).unwrap();
        let original = d.clone();
        let owners: Vec<_> = d.nodes.iter().map(|n| d.lane_of(n.id)).collect();
        use PoolResize::*;
        for (target, position) in [
            (LaneBoundary(0), Pos::new(0.0, -1000.0)),
            (LaneBoundary(0), Pos::new(0.0, 1000.0)),
            (Left, Pos::new(1000.0, 0.0)),
            (Right, Pos::new(-1000.0, 0.0)),
            (Top, Pos::new(0.0, 1000.0)),
            (Bottom, Pos::new(0.0, -1000.0)),
        ] {
            let mut resized = original.clone();
            resized.resize_pool(id, target, position, Some(10.0));
            assert_eq!(resized.nodes, original.nodes);
            assert_eq!(resized.flows, original.flows);
            assert_eq!(resized.nodes.iter().map(|n| resized.lane_of(n.id)).collect::<Vec<_>>(), owners);
            let pool = resized.pool(id).unwrap();
            for node in &resized.nodes {
                let (_, lid) = resized.lane_of(node.id).unwrap();
                let i = pool.lanes.iter().position(|l| l.id == lid).unwrap();
                let lane = pool.lane_rect(i);
                assert!(lane.contains(Pos::new(node.rect().x, node.rect().y)));
                assert!(lane.contains(Pos::new(node.rect().right(), node.rect().bottom())));
            }
            let anchor = target.anchor(pool).unwrap();
            let coordinate = if matches!(target, Left | Right) { anchor.x } else { anchor.y };
            assert_eq!(coordinate.rem_euclid(10.0), 0.0, "content limits round outward to the grid");
            resized.check_storage().unwrap();
        }
        assert_eq!(d.node(c), original.node(c));
    }

    #[test]
    fn single_lane_resize_free_coordinates_and_invalid_requests() {
        let (mut d, id) = diagram();
        d.pool_mut(id).unwrap().lanes.truncate(1);
        d.resize_pool(id, PoolResize::TopLeft, Pos::new(77.5, 83.25), None);
        let r = d.pool(id).unwrap().rect();
        assert_eq!(r, Rect::new(77.5, 83.25, 622.5, 176.75));
        let original = d.clone();
        for (target, position, grid) in [
            (PoolResize::LaneBoundary(0), Pos::new(0.0, 0.0), None),
            (PoolResize::BottomRight, Pos::new(f32::NAN, 0.0), None),
            (PoolResize::Bottom, Pos::new(0.0, f32::INFINITY), None),
            (PoolResize::Right, Pos::new(100.0, 100.0), Some(0.0)),
            (PoolResize::Right, Pos::new(100.0, 100.0), Some(f32::NAN)),
        ] {
            d.resize_pool(id, target, position, grid);
            assert_eq!(d, original);
        }
        let encoded = serde_json::to_string(&d).unwrap();
        assert_eq!(serde_json::from_str::<BpmnDiagram>(&encoded).unwrap(), d);
        d.resize_pool(id, PoolResize::BottomRight, Pos::new(-1000.0, -1000.0), None);
        assert_eq!(d.pool(id).unwrap().width, MIN_POOL_WIDTH);
        assert_eq!(d.pool(id).unwrap().height(), MIN_LANE_HEIGHT);
    }
}
