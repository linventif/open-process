//! Minimal geometry types, independent of any GUI toolkit.

use serde::{Deserialize, Serialize};

/// A point in diagram (world) coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
}

impl Pos {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dist(self, other: Pos) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn lerp(self, other: Pos, t: f32) -> Pos {
        Pos::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }

    pub fn snapped(self, grid: f32) -> Pos {
        Pos::new((self.x / grid).round() * grid, (self.y / grid).round() * grid)
    }
}

impl std::ops::Add for Pos {
    type Output = Pos;
    fn add(self, o: Pos) -> Pos {
        Pos::new(self.x + o.x, self.y + o.y)
    }
}

impl std::ops::Sub for Pos {
    type Output = Pos;
    fn sub(self, o: Pos) -> Pos {
        Pos::new(self.x - o.x, self.y - o.y)
    }
}

/// Axis-aligned rectangle in world coordinates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_center(c: Pos, w: f32, h: f32) -> Self {
        Self::new(c.x - w / 2.0, c.y - h / 2.0, w, h)
    }

    pub fn center(&self) -> Pos {
        Pos::new(self.x + self.w / 2.0, self.y + self.h / 2.0)
    }

    pub fn contains(&self, p: Pos) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }

    pub fn right(&self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.h
    }
}

/// Preferred connector ports: side midpoints on rectangles, vertices on diamonds.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Port {
    Top,
    Right,
    Bottom,
    Left,
}

impl Port {
    pub const ALL: [Self; 4] = [Self::Top, Self::Right, Self::Bottom, Self::Left];

    pub fn point(self, bounds: Rect) -> Pos {
        let center = bounds.center();
        match self {
            Self::Top => Pos::new(center.x, bounds.y),
            Self::Right => Pos::new(bounds.right(), center.y),
            Self::Bottom => Pos::new(center.x, bounds.bottom()),
            Self::Left => Pos::new(bounds.x, center.y),
        }
    }

    pub fn toward(bounds: Rect, target: Pos) -> Self {
        let delta = target - bounds.center();
        if delta.x.abs() / bounds.w.max(0.001) >= delta.y.abs() / bounds.h.max(0.001) {
            if delta.x < 0.0 { Self::Left } else { Self::Right }
        } else if delta.y < 0.0 {
            Self::Top
        } else {
            Self::Bottom
        }
    }
}

/// Distance from point `p` to segment `[a, b]`.
pub fn dist_to_segment(p: Pos, a: Pos, b: Pos) -> f32 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 <= f32::EPSILON {
        return p.dist(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    p.dist(Pos::new(a.x + t * dx, a.y + t * dy))
}

/// Distance from point `p` to a polyline.
pub fn dist_to_polyline(p: Pos, pts: &[Pos]) -> f32 {
    pts.windows(2).map(|w| dist_to_segment(p, w[0], w[1])).fold(f32::INFINITY, f32::min)
}
