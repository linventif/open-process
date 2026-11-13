//! Free drawing document, independent of its renderer. Path points are normalized
//! inside each element's bounds, so moving/resizing preserves their direction.
use crate::geom::{Pos, Rect, dist_to_polyline};
use crate::i18n::{key, tr};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub mod svg;
pub type Id = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Rectangle,
    Ellipse,
    Diamond,
    Line,
    Arrow,
    Freehand,
    Text,
    Note,
}
impl Kind {
    pub const ALL: [Self; 8] = [
        Self::Rectangle,
        Self::Ellipse,
        Self::Diamond,
        Self::Line,
        Self::Arrow,
        Self::Freehand,
        Self::Text,
        Self::Note,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => key("Rectangle"),
            Self::Ellipse => key("Ellipse"),
            Self::Diamond => key("Diamond"),
            Self::Line => key("Line"),
            Self::Arrow => key("Arrow"),
            Self::Freehand => key("Freehand"),
            Self::Text => key("Text"),
            Self::Note => key("Sticky note"),
        }
    }
    pub fn is_path(self) -> bool {
        matches!(self, Self::Line | Self::Arrow | Self::Freehand)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Paint {
    pub stroke: [u8; 3],
    pub fill: Option<[u8; 3]>,
    pub width: f32,
    pub font_size: f32,
    pub sketch: bool,
}
impl Default for Paint {
    fn default() -> Self {
        Self {
            stroke: [67, 76, 94],
            fill: None,
            width: 2.0,
            font_size: 20.0,
            sketch: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Element {
    pub id: Id,
    pub kind: Kind,
    pub bounds: Rect,
    pub points: Vec<Pos>,
    pub text: String,
    pub paint: Paint,
}
impl Element {
    pub fn new(kind: Kind, from: Pos, to: Pos, mut paint: Paint) -> Self {
        if kind == Kind::Note && paint.fill.is_none() {
            paint.fill = Some([255, 236, 153]);
        }
        let bounds = Rect::new(
            from.x.min(to.x),
            from.y.min(to.y),
            (to.x - from.x).abs().max(1.0),
            (to.y - from.y).abs().max(1.0),
        );
        let points = if kind.is_path() {
            [from, to]
                .into_iter()
                .map(|p| Pos::new((p.x - bounds.x) / bounds.w, (p.y - bounds.y) / bounds.h))
                .collect()
        } else {
            vec![]
        };
        Self {
            id: 0,
            kind,
            bounds,
            points,
            text: String::new(),
            paint,
        }
    }
    pub fn freehand(points: &[Pos], paint: Paint) -> Option<Self> {
        if points.len() < 2 {
            return None;
        }
        let min = Pos::new(
            points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
            points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
        );
        let max = Pos::new(
            points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
            points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max),
        );
        let mut e = Self::new(Kind::Freehand, min, max, paint);
        e.points = points
            .iter()
            .map(|p| Pos::new((p.x - min.x) / e.bounds.w, (p.y - min.y) / e.bounds.h))
            .collect();
        Some(e)
    }
    pub fn world_points(&self) -> Vec<Pos> {
        self.points
            .iter()
            .map(|p| Pos::new(self.bounds.x + p.x * self.bounds.w, self.bounds.y + p.y * self.bounds.h))
            .collect()
    }
    pub fn hit(&self, p: Pos, tolerance: f32) -> bool {
        if self.kind.is_path() {
            return dist_to_polyline(p, &self.world_points()) <= tolerance + self.paint.width / 2.0;
        }
        let b = self.bounds;
        let c = b.center();
        match self.kind {
            Kind::Ellipse => ((p.x - c.x) / (b.w / 2.0 + tolerance)).powi(2) + ((p.y - c.y) / (b.h / 2.0 + tolerance)).powi(2) <= 1.0,
            Kind::Diamond => ((p.x - c.x) / (b.w / 2.0 + tolerance)).abs() + ((p.y - c.y) / (b.h / 2.0 + tolerance)).abs() <= 1.0,
            _ => Rect::new(b.x - tolerance, b.y - tolerance, b.w + tolerance * 2.0, b.h + tolerance * 2.0).contains(p),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Board {
    pub name: String,
    pub elements: Vec<Element>,
    next_id: Id,
}
impl Board {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            elements: vec![],
            next_id: 1,
        }
    }
    pub fn add(&mut self, mut e: Element) -> Id {
        e.id = self.next_id;
        self.next_id += 1;
        let id = e.id;
        self.elements.push(e);
        id
    }
    pub fn element(&self, id: Id) -> Option<&Element> {
        self.elements.iter().find(|e| e.id == id)
    }
    pub fn hit(&self, p: Pos, tolerance: f32) -> Option<Id> {
        self.elements.iter().rev().find(|e| e.hit(p, tolerance)).map(|e| e.id)
    }
    pub fn delete(&mut self, ids: &BTreeSet<Id>) {
        self.elements.retain(|e| !ids.contains(&e.id));
    }
    pub fn duplicate(&mut self, ids: &BTreeSet<Id>) -> BTreeSet<Id> {
        let copies: Vec<_> = self.elements.iter().filter(|e| ids.contains(&e.id)).cloned().collect();
        copies
            .into_iter()
            .map(|mut e| {
                e.bounds.x += 20.0;
                e.bounds.y += 20.0;
                self.add(e)
            })
            .collect()
    }
    pub fn bounds(&self) -> Option<Rect> {
        let first = self.elements.first()?.bounds;
        Some(self.elements.iter().skip(1).fold(first, |a, e| {
            let b = e.bounds;
            let x = a.x.min(b.x);
            let y = a.y.min(b.y);
            Rect::new(x, y, a.right().max(b.right()) - x, a.bottom().max(b.bottom()) - y)
        }))
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut ids = BTreeSet::new();
        let mut points = 0usize;
        for e in &self.elements {
            points = points.saturating_add(e.points.len());
            if !ids.insert(e.id)
                || e.id == 0
                || e.id >= self.next_id
                || self.next_id == Id::MAX
                || ![e.bounds.x, e.bounds.y, e.bounds.w, e.bounds.h]
                    .iter()
                    .all(|n| n.is_finite() && n.abs() <= 1_000_000.0)
                || e.bounds.w <= 0.0
                || e.bounds.h <= 0.0
                || !e.paint.width.is_finite()
                || !(0.5..=12.0).contains(&e.paint.width)
                || !e.paint.font_size.is_finite()
                || !(8.0..=96.0).contains(&e.paint.font_size)
                || e.points
                    .iter()
                    .any(|p| !p.x.is_finite() || !p.y.is_finite() || !(0.0..=1.0).contains(&p.x) || !(0.0..=1.0).contains(&p.y))
                || (e.kind.is_path() && e.points.len() < 2)
                || (!e.kind.is_path() && !e.points.is_empty())
                || (matches!(e.kind, Kind::Line | Kind::Arrow) && e.points.len() != 2)
                || e.text.len() > 65_536
            {
                return Err(tr("Invalid free drawing element.").into());
            }
        }
        if self.next_id == 0 || self.next_id == Id::MAX || self.elements.len() > 20_000 || points > 200_000 {
            return Err(tr("Free drawing exceeds the supported size.").into());
        }
        Ok(())
    }
}

pub fn example() -> Board {
    let mut b = Board::new(tr("A shared idea"));
    let blue = Paint {
        fill: Some([219, 234, 254]),
        stroke: [37, 99, 235],
        ..Default::default()
    };
    let mut idea = Element::new(Kind::Rectangle, Pos::new(50.0, 100.0), Pos::new(270.0, 220.0), blue.clone());
    idea.text = tr("An idea").into();
    b.add(idea);
    b.add(Element::new(Kind::Arrow, Pos::new(290.0, 160.0), Pos::new(390.0, 160.0), blue));
    let mut note = Element::new(Kind::Note, Pos::new(410.0, 80.0), Pos::new(650.0, 260.0), Paint::default());
    note.text = tr("Sketch, discuss, improve.").into();
    b.add(note);
    let mut title = Element::new(
        Kind::Text,
        Pos::new(55.0, 10.0),
        Pos::new(630.0, 65.0),
        Paint {
            font_size: 32.0,
            ..Default::default()
        },
    );
    title.text = tr("A shared idea").into();
    b.add(title);
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn paths_keep_direction_when_moved_resized_and_duplicated() {
        let mut b = Board::new("test");
        let id = b.add(Element::new(
            Kind::Arrow,
            Pos::new(200.0, 90.0),
            Pos::new(20.0, 10.0),
            Paint::default(),
        ));
        assert_eq!(b.elements[0].world_points(), [Pos::new(200.0, 90.0), Pos::new(20.0, 10.0)]);
        assert_eq!(b.hit(Pos::new(110.0, 50.0), 3.0), Some(id));
        assert_eq!(b.hit(Pos::new(20.0, 80.0), 3.0), None);
        b.elements[0].bounds.w = 360.0;
        assert_eq!(b.elements[0].world_points()[0], Pos::new(380.0, 90.0));
        let copied = b.duplicate(&BTreeSet::from([id]));
        assert_eq!(copied.len(), 1);
        assert_eq!(b.elements[1].points, b.elements[0].points);
        assert_eq!(b.elements[1].world_points()[0], Pos::new(400.0, 110.0));
        b.delete(&BTreeSet::from([id]));
        b.validate().unwrap();
        assert_eq!(b.elements.len(), 1);
    }
    #[test]
    fn freehand_shape_hits_and_validation() {
        let mut b = example();
        let p = [Pos::new(0.0, 0.0), Pos::new(30.0, 60.0), Pos::new(80.0, 40.0)];
        let id = b.add(Element::freehand(&p, Paint::default()).unwrap());
        assert_eq!(b.element(id).unwrap().world_points(), p);
        assert!(b.element(id).unwrap().hit(p[1], 2.0));
        let e = Element::new(Kind::Ellipse, Pos::default(), Pos::new(100.0, 100.0), Paint::default());
        assert!(!e.hit(Pos::new(0.0, 0.0), 1.0));
        b.validate().unwrap();
        let restored: Board = serde_json::from_str(&serde_json::to_string(&b).unwrap()).unwrap();
        assert_eq!(b, restored);
        b.elements[0].bounds.w = f32::NAN;
        assert!(b.validate().is_err());
        b = example();
        b.elements[1].points[0].x = 2.0;
        assert!(b.validate().is_err());
        b = example();
        b.elements[1].id = b.elements[0].id;
        assert!(b.validate().is_err());
    }
}
