use super::{Pos, Rect};

/// The moving sides of a rectangular element. Opposite sides remain fixed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeEdge {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeEdge {
    pub const ALL: [Self; 8] = [
        Self::Left,
        Self::Right,
        Self::Top,
        Self::Bottom,
        Self::TopLeft,
        Self::TopRight,
        Self::BottomLeft,
        Self::BottomRight,
    ];

    pub fn anchor(self, r: Rect) -> Pos {
        use ResizeEdge::*;
        Pos::new(
            match self {
                Left | TopLeft | BottomLeft => r.x,
                Right | TopRight | BottomRight => r.right(),
                _ => r.center().x,
            },
            match self {
                Top | TopLeft | TopRight => r.y,
                Bottom | BottomLeft | BottomRight => r.bottom(),
                _ => r.center().y,
            },
        )
    }

    /// Apply a border position with notation-supplied size limits and optional snapping.
    pub fn resize(self, r: Rect, position: Pos, min: Pos, max: Pos, grid: Option<f32>) -> Rect {
        if ![r.x, r.y, r.w, r.h, position.x, position.y, min.x, min.y]
            .iter()
            .all(|v| v.is_finite())
            || min.x <= 0.0
            || min.y <= 0.0
            || max.x < min.x
            || max.y < min.y
            || max.x.is_nan()
            || max.y.is_nan()
            || grid.is_some_and(|v| !v.is_finite() || v <= 0.0)
        {
            return r;
        }
        let border = |value: f32, lo: f32, hi: f32| {
            if let Some(step) = grid {
                let low = (lo / step).ceil();
                let high = (hi / step).floor();
                if low <= high {
                    return (value / step).round().clamp(low, high) * step;
                }
            }
            value.clamp(lo, hi)
        };
        let mut next = r;
        use ResizeEdge::*;
        if matches!(self, Left | TopLeft | BottomLeft) {
            next.x = border(position.x, r.right() - max.x, r.right() - min.x);
            next.w = r.right() - next.x;
        } else if matches!(self, Right | TopRight | BottomRight) {
            next.w = border(position.x, r.x + min.x, r.x + max.x) - r.x;
        }
        if matches!(self, Top | TopLeft | TopRight) {
            next.y = border(position.y, r.bottom() - max.y, r.bottom() - min.y);
            next.h = r.bottom() - next.y;
        } else if matches!(self, Bottom | BottomLeft | BottomRight) {
            next.h = border(position.y, r.y + min.y, r.y + max.y) - r.y;
        }
        if [next.x, next.y, next.w, next.h].iter().all(|v| v.is_finite()) {
            next
        } else {
            r
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edges_keep_opposites_and_unaffected_axes_fixed() {
        let r = Rect::new(-100.0, 50.0, 300.0, 200.0);
        use ResizeEdge::*;
        for edge in ResizeEdge::ALL {
            let delta = Pos::new(
                if matches!(edge, Left | TopLeft | BottomLeft) { -23.0 } else { 23.0 },
                if matches!(edge, Top | TopLeft | TopRight) { -17.0 } else { 17.0 },
            );
            let n = edge.resize(
                r,
                edge.anchor(r) + delta,
                Pos::new(40.0, 30.0),
                Pos::new(2000.0, 2000.0),
                Some(10.0),
            );
            assert_eq!(n.x, if matches!(edge, Left | TopLeft | BottomLeft) { -120.0 } else { r.x });
            assert_eq!(
                n.right(),
                if matches!(edge, Right | TopRight | BottomRight) {
                    220.0
                } else {
                    r.right()
                }
            );
            assert_eq!(n.y, if matches!(edge, Top | TopLeft | TopRight) { 30.0 } else { r.y });
            assert_eq!(
                n.bottom(),
                if matches!(edge, Bottom | BottomLeft | BottomRight) {
                    270.0
                } else {
                    r.bottom()
                }
            );
            assert_eq!(
                edge.resize(
                    n,
                    edge.anchor(r) + delta,
                    Pos::new(40.0, 30.0),
                    Pos::new(2000.0, 2000.0),
                    Some(10.0)
                ),
                n
            );
        }
    }

    #[test]
    fn limits_free_coordinates_and_invalid_inputs() {
        let r = Rect::new(10.0, 20.0, 200.0, 100.0);
        let min = Pos::new(40.0, 30.0);
        let max = Pos::new(300.0, 200.0);
        let n = ResizeEdge::TopLeft.resize(r, Pos::new(1000.0, 1000.0), min, max, None);
        assert_eq!(n, Rect::new(170.0, 90.0, 40.0, 30.0));
        assert_eq!(
            ResizeEdge::BottomRight.resize(r, Pos::new(1000.0, 1000.0), min, max, None),
            Rect::new(10.0, 20.0, 300.0, 200.0)
        );
        assert_eq!(
            ResizeEdge::Left.resize(r, Pos::new(13.25, 0.0), min, max, None),
            Rect::new(13.25, 20.0, 196.75, 100.0)
        );
        assert_eq!(ResizeEdge::Top.resize(r, Pos::new(0.0, f32::NAN), min, max, None), r);
        assert_eq!(ResizeEdge::Top.resize(r, Pos::new(0.0, 0.0), min, max, Some(0.0)), r);
    }
}
