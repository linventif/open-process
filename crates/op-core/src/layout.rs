//! Deterministic layered layout shared by graph notations.
//!
//! Nodes are ranked along the flow (Kahn layering with a stable cycle break),
//! ordered inside each layer by barycenters to reduce crossings, then placed
//! across the flow near their neighbours without overlapping. Only presentation
//! is computed: callers keep identities, labels and semantics unchanged.
use crate::geom::Pos;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    LeftToRight,
    TopToBottom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Options {
    pub direction: Direction,
    /// Free space around the arranged graph.
    pub margin: f32,
    /// Distance between consecutive layers along the flow.
    pub layer_gap: f32,
    /// Distance between neighbours of the same layer.
    pub node_gap: f32,
    /// Centers are rounded outward to this step, so spacing is never consumed.
    pub grid: Option<f32>,
}

impl Options {
    pub fn new(direction: Direction, grid: Option<f32>) -> Self {
        Self {
            direction,
            margin: 40.0,
            layer_gap: 80.0,
            node_gap: 40.0,
            grid,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// A size, spacing or grid step is not finite and positive.
    InvalidGeometry,
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(crate::i18n::tr("canvas.cannot_arrange_invalid_geometry"))
    }
}

const SWEEPS: usize = 4;

/// Result of [`layered_bands`].
#[derive(Clone, Debug, PartialEq)]
pub struct Banded {
    /// Node centers in input order. The cross-axis coordinate is relative to the
    /// start of the node's band; the main-axis coordinate is shared by all bands.
    pub centers: Vec<Pos>,
    /// Cross-axis extent each band needs, margins included (0 for an empty band).
    pub extents: Vec<f32>,
}

/// Centers for nodes of `(width, height)`, returned in input order.
///
/// `edges` are `(from, to)` node indices. Self-loops, duplicates and indices out
/// of range are ignored. Unconnected nodes are packed after the connected graph
/// so they never stretch a layer.
pub fn layered(sizes: &[(f32, f32)], edges: &[(usize, usize)], options: Options) -> Result<Vec<Pos>, LayoutError> {
    layered_bands(sizes, edges, &vec![0; sizes.len()], options).map(|banded| banded.centers)
}

/// Like [`layered`], with every node kept in a band (for example a BPMN lane).
/// Bands share the layers along the flow and are stacked by the caller.
pub fn layered_bands(sizes: &[(f32, f32)], edges: &[(usize, usize)], bands: &[usize], options: Options) -> Result<Banded, LayoutError> {
    let positive = |v: f32| v.is_finite() && v > 0.0;
    let spacing = [options.margin, options.layer_gap, options.node_gap];
    if sizes.len() != bands.len()
        || sizes.iter().any(|&(w, h)| !positive(w) || !positive(h))
        || spacing.iter().any(|v| !v.is_finite() || *v < 0.0)
        || options.grid.is_some_and(|g| !positive(g))
    {
        return Err(LayoutError::InvalidGeometry);
    }
    let n = sizes.len();
    let edges: BTreeSet<(usize, usize)> = edges.iter().copied().filter(|&(a, b)| a != b && a < n && b < n).collect();
    let mut succ = vec![vec![]; n];
    let mut pred = vec![vec![]; n];
    for &(a, b) in &edges {
        succ[a].push(b);
        pred[b].push(a);
    }
    let (main, cross): (Vec<f32>, Vec<f32>) = sizes
        .iter()
        .map(|&(w, h)| match options.direction {
            Direction::LeftToRight => (w, h),
            Direction::TopToBottom => (h, w),
        })
        .unzip();

    let mut layers = layers(&succ, &pred);
    pack_isolated(&mut layers, &succ, &pred);
    for layer in &mut layers {
        layer.sort_by_key(|&v| bands[v]);
    }
    let mut rank = vec![0; n];
    for (r, layer) in layers.iter().enumerate() {
        for &v in layer {
            rank[v] = r;
        }
    }
    // Coordinates of different bands are independent; only same-band neighbours attract.
    let neighbours = |v: usize| succ[v].iter().chain(&pred[v]).copied().filter(move |&u| bands[u] == bands[v]);
    let mut at = vec![0.0; n];
    for layer in &layers {
        for group in layer.chunk_by(|&a, &b| bands[a] == bands[b]) {
            let wanted = vec![0.0; group.len()];
            for (&v, c) in group.iter().zip(separate(&wanted, group, &cross, options.node_gap)) {
                at[v] = c;
            }
        }
    }
    for sweep in 0..SWEEPS {
        let down = sweep % 2 == 0;
        let order: Vec<usize> = if down {
            (1..layers.len()).collect()
        } else {
            (0..layers.len().saturating_sub(1)).rev().collect()
        };
        for r in order {
            let toward = |u: usize| if down { rank[u] < r } else { rank[u] > r };
            let target = |v: usize| barycenter(neighbours(v).filter(|&u| toward(u)).map(|u| at[u])).unwrap_or(at[v]);
            let mut keyed: Vec<(usize, f32)> = layers[r].iter().map(|&v| (v, target(v))).collect();
            keyed.sort_by(|a, b| bands[a.0].cmp(&bands[b.0]).then(a.1.total_cmp(&b.1)));
            layers[r] = keyed.iter().map(|&(v, _)| v).collect();
            for group in keyed.chunk_by(|a, b| bands[a.0] == bands[b.0]) {
                let members: Vec<usize> = group.iter().map(|&(v, _)| v).collect();
                let wanted: Vec<f32> = group.iter().map(|&(_, k)| k).collect();
                for (&v, c) in members.iter().zip(separate(&wanted, &members, &cross, options.node_gap)) {
                    at[v] = c;
                }
            }
        }
    }

    let align = |v: f32| options.grid.map_or(v, |g| (v / g).ceil() * g);
    let band_count = bands.iter().max().map_or(0, |b| b + 1);
    let mut low = vec![f32::INFINITY; band_count];
    for v in 0..n {
        low[bands[v]] = low[bands[v]].min(at[v] - cross[v] / 2.0);
    }
    let mut extents = vec![0.0f32; band_count];
    let mut out = vec![Pos::default(); n];
    let mut cursor = options.margin;
    for layer in &layers {
        let thickness = layer.iter().map(|&v| main[v]).fold(0.0, f32::max);
        let center = align(cursor + thickness / 2.0);
        cursor = center + thickness / 2.0 + options.layer_gap;
        let mut previous: Option<usize> = None;
        for &v in layer {
            let b = bands[v];
            let mut c = at[v] + options.margin - low[b];
            if let Some(p) = previous.filter(|&p| bands[p] == b) {
                c = c.max(at[p] + (cross[p] + cross[v]) / 2.0 + options.node_gap);
            }
            at[v] = align(c);
            extents[b] = extents[b].max(at[v] + cross[v] / 2.0 + options.margin);
            previous = Some(v);
            out[v] = match options.direction {
                Direction::LeftToRight => Pos::new(center, at[v]),
                Direction::TopToBottom => Pos::new(at[v], center),
            };
        }
    }
    if out.iter().any(|p| !p.x.is_finite() || !p.y.is_finite()) || extents.iter().any(|e| !e.is_finite()) {
        return Err(LayoutError::InvalidGeometry);
    }
    Ok(Banded { centers: out, extents })
}

/// Longest-path layering by Kahn's algorithm. When only cycles remain, the node
/// with the most already-placed predecessors (then the lowest index) is released:
/// back edges then return to an earlier layer and the loop always terminates.
fn layers(succ: &[Vec<usize>], pred: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let n = succ.len();
    let mut waiting: Vec<usize> = pred.iter().map(Vec::len).collect();
    let mut placed = vec![false; n];
    let mut rank = vec![0usize; n];
    let mut ready: BTreeSet<usize> = (0..n).filter(|&v| waiting[v] == 0).collect();
    for _ in 0..n {
        let v = ready.pop_first().unwrap_or_else(|| {
            (0..n)
                .filter(|&v| !placed[v])
                .max_by_key(|&v| (pred[v].iter().filter(|&&u| placed[u]).count(), std::cmp::Reverse(v)))
                .expect("a node remains while the loop runs")
        });
        placed[v] = true;
        for &w in &succ[v] {
            if !placed[w] {
                rank[w] = rank[w].max(rank[v] + 1);
                waiting[w] = waiting[w].saturating_sub(1);
                if waiting[w] == 0 {
                    ready.insert(w);
                }
            }
        }
    }
    // A source joins the flow just before its earliest successor instead of the
    // first layer, e.g. a data input or a message start in a later column.
    for v in (0..n).filter(|&v| pred[v].is_empty()) {
        if let Some(first) = succ[v].iter().map(|&w| rank[w]).min() {
            rank[v] = first.saturating_sub(1);
        }
    }
    let mut layers = vec![vec![]; rank.iter().max().map_or(0, |r| r + 1)];
    for v in 0..n {
        layers[rank[v]].push(v);
    }
    layers
}

/// Moves unconnected nodes out of the flow into trailing layers whose length
/// matches the widest connected layer (or a square block when nothing connects).
fn pack_isolated(layers: &mut Vec<Vec<usize>>, succ: &[Vec<usize>], pred: &[Vec<usize>]) {
    let lonely = |v: &usize| succ[*v].is_empty() && pred[*v].is_empty();
    let mut isolated: Vec<usize> = layers.iter().flatten().copied().filter(lonely).collect();
    if isolated.is_empty() {
        return;
    }
    isolated.sort_unstable();
    for layer in layers.iter_mut() {
        layer.retain(|v| !lonely(v));
    }
    layers.retain(|layer| !layer.is_empty());
    let widest = layers.iter().map(Vec::len).max().unwrap_or(0);
    let square = (isolated.len() as f32).sqrt().ceil() as usize;
    for chunk in isolated.chunks(widest.max(square).max(1)) {
        layers.push(chunk.to_vec());
    }
}

fn barycenter(values: impl Iterator<Item = f32>) -> Option<f32> {
    let (sum, count) = values.fold((0.0, 0usize), |(s, c), v| (s + v, c + 1));
    (count > 0).then(|| sum / count as f32)
}

/// Centers close to `wanted` (already sorted) that keep `gap` between nodes.
/// Pushing forward and backward then averaging preserves every separation; the
/// final shift keeps the layer's mean, so a split stays aligned with its join.
fn separate(wanted: &[f32], layer: &[usize], cross: &[f32], gap: f32) -> Vec<f32> {
    if wanted.is_empty() {
        return vec![];
    }
    let sep = |i: usize| (cross[layer[i - 1]] + cross[layer[i]]) / 2.0 + gap;
    let mut forward = wanted.to_vec();
    for i in 1..forward.len() {
        forward[i] = forward[i].max(forward[i - 1] + sep(i));
    }
    let mut backward = wanted.to_vec();
    for i in (1..backward.len()).rev() {
        backward[i - 1] = backward[i - 1].min(backward[i] - sep(i));
    }
    let placed: Vec<f32> = forward.iter().zip(&backward).map(|(f, b)| (f + b) / 2.0).collect();
    let mean = |v: &[f32]| v.iter().sum::<f32>() / v.len() as f32;
    let shift = mean(wanted) - mean(&placed);
    placed.into_iter().map(|c| c + shift).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rects(sizes: &[(f32, f32)], centers: &[Pos]) -> Vec<crate::geom::Rect> {
        sizes
            .iter()
            .zip(centers)
            .map(|(&(w, h), &c)| crate::geom::Rect::from_center(c, w, h))
            .collect()
    }

    fn assert_separated(sizes: &[(f32, f32)], centers: &[Pos]) {
        let r = rects(sizes, centers);
        for (i, a) in r.iter().enumerate() {
            for b in &r[i + 1..] {
                assert!(
                    a.right() <= b.x || b.right() <= a.x || a.bottom() <= b.y || b.bottom() <= a.y,
                    "{a:?} overlaps {b:?}"
                );
            }
        }
    }

    #[test]
    fn chain_follows_the_direction_and_stays_on_one_line() {
        let sizes = [(40.0, 40.0), (120.0, 60.0), (60.0, 60.0), (40.0, 40.0)];
        let edges = [(0, 1), (1, 2), (2, 3)];
        for direction in [Direction::LeftToRight, Direction::TopToBottom] {
            let at = layered(&sizes, &edges, Options::new(direction, None)).unwrap();
            for pair in at.windows(2) {
                match direction {
                    Direction::LeftToRight => assert!(pair[0].x < pair[1].x && pair[0].y == pair[1].y),
                    Direction::TopToBottom => assert!(pair[0].y < pair[1].y && pair[0].x == pair[1].x),
                }
            }
            assert_separated(&sizes, &at);
        }
    }

    #[test]
    fn branches_share_a_layer_without_overlap_and_join_after_them() {
        // Split into three branches of different sizes, then join.
        let sizes = [(30.0, 30.0), (100.0, 80.0), (100.0, 20.0), (100.0, 50.0), (30.0, 30.0)];
        let edges = [(0, 1), (0, 2), (0, 3), (1, 4), (2, 4), (3, 4)];
        let at = layered(&sizes, &edges, Options::new(Direction::LeftToRight, None)).unwrap();
        assert!(at[1].x == at[2].x && at[2].x == at[3].x);
        assert!(at[0].x < at[1].x && at[1].x < at[4].x);
        assert!((at[0].y - at[4].y).abs() < 0.01, "split and join stay aligned");
        assert_separated(&sizes, &at);
        let r = rects(&sizes, &at);
        assert!(r.iter().all(|r| r.x >= 40.0 && r.y >= 40.0), "margin is kept");
    }

    #[test]
    fn cycles_terminate_and_back_edges_return_left() {
        // start → a → b → c → a (rework loop) and c → end.
        let sizes = [(20.0, 20.0); 5];
        let edges = [(0, 1), (1, 2), (2, 3), (3, 1), (3, 4)];
        let at = layered(&sizes, &edges, Options::new(Direction::LeftToRight, None)).unwrap();
        assert!(at[0].x < at[1].x && at[1].x < at[2].x && at[2].x < at[3].x && at[3].x < at[4].x);
        // A pure cycle has no source and still terminates.
        let ring = layered(&sizes[..3], &[(0, 1), (1, 2), (2, 0)], Options::new(Direction::TopToBottom, None)).unwrap();
        assert_separated(&sizes[..3], &ring);
    }

    #[test]
    fn grid_alignment_rounds_outward_and_isolated_nodes_are_packed() {
        let sizes = [(33.0, 17.0), (71.0, 29.0), (45.0, 45.0), (45.0, 45.0), (45.0, 45.0), (45.0, 45.0)];
        let edges = [(0, 1), (1, 0), (0, 0), (9, 1)];
        for grid in [10.0, 25.0] {
            let at = layered(&sizes, &edges, Options::new(Direction::LeftToRight, Some(grid))).unwrap();
            assert!(at.iter().all(|p| p == &p.snapped(grid)));
            assert_separated(&sizes, &at);
            let connected = at[0].x.max(at[1].x);
            assert!(at[2..].iter().all(|p| p.x > connected), "isolated nodes trail the flow");
        }
    }

    #[test]
    fn crossings_are_reduced_by_ordering() {
        // Two parallel chains given in crossed index order: a0 → b1, a1 → b0.
        let sizes = [(40.0, 40.0); 4];
        let at = layered(&sizes, &[(0, 3), (1, 2)], Options::new(Direction::LeftToRight, None)).unwrap();
        assert_eq!(at[0].y < at[1].y, at[3].y < at[2].y, "edges do not cross");
    }

    #[test]
    fn invalid_geometry_is_rejected_and_empty_graphs_are_fine() {
        let opts = Options::new(Direction::LeftToRight, None);
        assert_eq!(layered(&[(f32::NAN, 1.0)], &[], opts), Err(LayoutError::InvalidGeometry));
        assert_eq!(layered(&[(0.0, 1.0)], &[], opts), Err(LayoutError::InvalidGeometry));
        assert_eq!(
            layered(&[(1.0, 1.0)], &[], Options::new(Direction::LeftToRight, Some(0.0))),
            Err(LayoutError::InvalidGeometry)
        );
        assert_eq!(layered(&[], &[], opts), Ok(vec![]));
        let single = layered(&[(10.0, 20.0)], &[], opts).unwrap();
        assert_eq!(single, vec![Pos::new(45.0, 50.0)]);
    }
}
