//! Vector pictograms. Modules register their card icon explicitly; navigation
//! never infers a notation from its position in a list.
use egui::{Color32, Painter, Rect, Shape, Stroke, StrokeKind, vec2};

pub type Icon = fn(&Painter, Rect, Color32);

/// The original BPMN selection cursor, shared by all graphical editors.
pub fn select(p: &Painter, r: Rect, color: Color32) {
    let c = r.center();
    let points = [
        (-6.0, -10.0),
        (7.0, 2.0),
        (1.0, 3.0),
        (4.0, 10.0),
        (1.0, 11.0),
        (-2.0, 4.0),
        (-6.0, 8.0),
    ];
    p.add(Shape::closed_line(
        points.into_iter().map(|(x, y)| c + vec2(x, y)).collect(),
        Stroke::new(1.6, color),
    ));
}

/// A small 32 × 32 drawing space that fits both cards and tool buttons.
struct Glyph<'a> {
    p: &'a Painter,
    origin: egui::Pos2,
    scale: f32,
    stroke: Stroke,
    color: Color32,
}
impl<'a> Glyph<'a> {
    fn new(p: &'a Painter, r: Rect, color: Color32) -> Self {
        let size = r.width().min(r.height());
        let scale = size / 32.0;
        Self {
            p,
            origin: r.center() - vec2(size, size) / 2.0,
            scale,
            stroke: Stroke::new((1.6 * scale).clamp(1.1, 2.0), color),
            color,
        }
    }
    fn at(&self, x: f32, y: f32) -> egui::Pos2 {
        self.origin + vec2(x, y) * self.scale
    }
    fn rect(&self, x: f32, y: f32, w: f32, h: f32, round: f32) {
        self.p.rect_stroke(
            Rect::from_min_size(self.at(x, y), vec2(w, h) * self.scale),
            round * self.scale,
            self.stroke,
            StrokeKind::Middle,
        );
    }
    fn line(&self, points: &[(f32, f32)]) {
        self.p
            .add(Shape::line(points.iter().map(|&(x, y)| self.at(x, y)).collect(), self.stroke));
    }
    fn polygon(&self, points: &[(f32, f32)]) {
        self.p.add(Shape::closed_line(
            points.iter().map(|&(x, y)| self.at(x, y)).collect(),
            self.stroke,
        ));
    }
    fn circle(&self, x: f32, y: f32, r: f32, filled: bool) {
        if filled {
            self.p.circle_filled(self.at(x, y), r * self.scale, self.color);
        } else {
            self.p.circle_stroke(self.at(x, y), r * self.scale, self.stroke);
        }
    }
    fn arrow(&self, from: (f32, f32), to: (f32, f32)) {
        self.line(&[from, to]);
        let a = self.at(from.0, from.1);
        let b = self.at(to.0, to.1);
        let d = (b - a).normalized();
        let n = vec2(-d.y, d.x);
        let size = 3.5 * self.scale;
        self.p.add(Shape::line(
            vec![b - d * size + n * size * 0.6, b, b - d * size - n * size * 0.6],
            self.stroke,
        ));
    }
}

pub fn bpmn(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.circle(4.0, 16.0, 3.0, false);
    g.line(&[(7.0, 16.0), (10.0, 16.0)]);
    g.rect(10.0, 10.0, 12.0, 12.0, 2.0);
    g.line(&[(22.0, 16.0), (25.0, 16.0)]);
    g.polygon(&[(25.0, 16.0), (28.0, 12.0), (31.0, 16.0), (28.0, 20.0)]);
    g.line(&[(27.0, 15.0), (29.0, 17.0)]);
    g.line(&[(29.0, 15.0), (27.0, 17.0)]);
}
pub fn petri(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.circle(5.0, 16.0, 4.0, false);
    g.circle(5.0, 16.0, 1.3, true);
    g.arrow((9.0, 16.0), (14.0, 16.0));
    g.p.rect_filled(Rect::from_min_size(g.at(15.0, 8.0), vec2(3.0, 16.0) * g.scale), 0.0, c);
    g.arrow((19.0, 16.0), (23.0, 16.0));
    g.circle(27.0, 16.0, 4.0, false);
}
pub fn mining(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    for y in [6.0, 14.0, 22.0] {
        g.circle(3.0, y, 1.3, true);
        g.line(&[(7.0, y), (13.0, y)]);
    }
    g.arrow((16.0, 14.0), (21.0, 14.0));
    g.rect(22.0, 10.0, 8.0, 7.0, 1.0);
    g.line(&[(26.0, 17.0), (26.0, 23.0)]);
    g.circle(26.0, 27.0, 3.0, false);
}
pub fn mermaid(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.line(&[(8.0, 6.0), (2.0, 16.0), (8.0, 26.0)]);
    g.line(&[(24.0, 6.0), (30.0, 16.0), (24.0, 26.0)]);
    g.rect(12.0, 4.0, 8.0, 6.0, 1.0);
    g.line(&[(16.0, 10.0), (16.0, 17.0), (11.0, 17.0), (11.0, 23.0)]);
    g.line(&[(16.0, 17.0), (21.0, 17.0), (21.0, 23.0)]);
    g.rect(8.0, 23.0, 6.0, 5.0, 1.0);
    g.rect(18.0, 23.0, 6.0, 5.0, 1.0);
}
pub fn evaluation(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.circle(22.0, 10.0, 7.0, false);
    g.line(&[(22.0, 6.0), (22.0, 10.0), (26.0, 12.0)]);
    for (x, h) in [(3.0, 12.0), (11.0, 7.0), (19.0, 10.0)] {
        g.p.rect_filled(Rect::from_min_size(g.at(x, 29.0 - h), vec2(5.0, h) * g.scale), g.scale, c);
    }
    g.line(&[(1.0, 31.0), (30.0, 31.0)]);
}
pub fn class(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    for (x, y, w, h) in [(1.0, 2.0, 13.0, 17.0), (19.0, 13.0, 12.0, 17.0)] {
        g.rect(x, y, w, h, 1.0);
        g.line(&[(x, y + 5.0), (x + w, y + 5.0)]);
        g.line(&[(x, y + 10.0), (x + w, y + 10.0)]);
    }
    g.line(&[(14.0, 8.0), (25.0, 8.0), (25.0, 13.0)]);
}
pub fn use_case(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.circle(6.0, 6.0, 3.0, false);
    g.line(&[(6.0, 9.0), (6.0, 20.0)]);
    g.line(&[(1.0, 14.0), (11.0, 14.0)]);
    g.line(&[(1.0, 28.0), (6.0, 20.0), (11.0, 28.0)]);
    g.line(&[(11.0, 14.0), (17.0, 14.0)]);
    g.p.add(egui::epaint::EllipseShape::stroke(
        g.at(24.0, 14.0),
        vec2(7.0, 5.0) * g.scale,
        g.stroke,
    ));
}
pub fn decision_table(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.rect(2.0, 4.0, 28.0, 24.0, 1.0);
    for y in [11.0, 19.0] {
        g.line(&[(2.0, y), (30.0, y)]);
    }
    for x in [10.0, 19.0] {
        g.line(&[(x, 4.0), (x, 28.0)]);
    }
    g.line(&[(22.0, 23.0), (24.0, 25.0), (28.0, 21.0)]);
    g.line(&[(22.0, 7.0), (27.0, 7.0)]);
}
pub fn epc(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.polygon(&[(9.0, 2.0), (23.0, 2.0), (26.0, 6.0), (23.0, 10.0), (9.0, 10.0), (6.0, 6.0)]);
    g.arrow((16.0, 10.0), (16.0, 14.0));
    g.rect(6.0, 14.0, 20.0, 8.0, 3.0);
    g.line(&[(16.0, 22.0), (16.0, 25.0)]);
    g.circle(16.0, 28.5, 3.0, false);
    g.line(&[(14.8, 27.3), (17.2, 29.7)]);
    g.line(&[(17.2, 27.3), (14.8, 29.7)]);
}
pub fn activity(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.circle(16.0, 3.0, 2.3, true);
    g.line(&[(16.0, 5.3), (16.0, 8.0)]);
    g.rect(10.0, 8.0, 12.0, 7.0, 3.0);
    g.line(&[(16.0, 15.0), (16.0, 18.0)]);
    g.polygon(&[(16.0, 18.0), (20.0, 22.0), (16.0, 26.0), (12.0, 22.0)]);
    for x in [5.0, 27.0] {
        g.line(&[(if x < 16.0 { 12.0 } else { 20.0 }, 22.0), (x, 22.0), (x, 26.0)]);
        g.circle(x, 29.0, 2.5, false);
        g.circle(x, 29.0, 1.0, true);
    }
}
pub fn database(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    for (x, y) in [(1.0, 3.0), (20.0, 13.0)] {
        g.rect(x, y, 11.0, 16.0, 1.0);
        g.line(&[(x, y + 5.0), (x + 11.0, y + 5.0)]);
        g.circle(x + 3.0, y + 9.0, 1.1, false);
        g.line(&[(x + 5.0, y + 9.0), (x + 8.0, y + 9.0)]);
        g.line(&[(x + 3.0, y + 13.0), (x + 8.0, y + 13.0)]);
    }
    g.line(&[(12.0, 12.0), (16.0, 12.0), (16.0, 23.0), (20.0, 23.0)]);
    g.line(&[(17.0, 23.0), (20.0, 20.5)]);
    g.line(&[(17.0, 23.0), (20.0, 25.5)]);
}

pub fn rectangle(p: &Painter, r: Rect, c: Color32) {
    Glyph::new(p, r, c).rect(3.0, 6.0, 26.0, 20.0, 2.0);
}
pub fn ellipse(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    p.add(egui::epaint::EllipseShape::stroke(
        g.at(16.0, 16.0),
        vec2(13.0, 10.0) * g.scale,
        g.stroke,
    ));
}
pub fn diamond(p: &Painter, r: Rect, c: Color32) {
    Glyph::new(p, r, c).polygon(&[(16.0, 2.0), (30.0, 16.0), (16.0, 30.0), (2.0, 16.0)]);
}
pub fn line(p: &Painter, r: Rect, c: Color32) {
    Glyph::new(p, r, c).line(&[(4.0, 28.0), (28.0, 4.0)]);
}
pub fn arrow(p: &Painter, r: Rect, c: Color32) {
    Glyph::new(p, r, c).arrow((4.0, 28.0), (28.0, 4.0));
}
pub fn freehand(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.polygon(&[(11.0, 20.0), (24.0, 3.0), (29.0, 7.0), (16.0, 24.0), (9.0, 27.0)]);
    g.line(&[(21.5, 6.0), (26.5, 10.0)]);
    g.line(&[(11.0, 20.0), (16.0, 24.0)]);
    g.line(&[(3.0, 29.0), (5.0, 26.0), (7.0, 29.0), (10.0, 30.0), (14.0, 28.0)]);
}
pub fn text(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.line(&[(4.0, 8.0), (4.0, 4.0), (28.0, 4.0), (28.0, 8.0)]);
    g.line(&[(16.0, 4.0), (16.0, 28.0)]);
    g.line(&[(10.0, 28.0), (22.0, 28.0)]);
}
pub fn note(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.polygon(&[(3.0, 3.0), (29.0, 3.0), (29.0, 21.0), (21.0, 29.0), (3.0, 29.0)]);
    g.line(&[(21.0, 29.0), (21.0, 21.0), (29.0, 21.0)]);
    g.line(&[(8.0, 10.0), (23.0, 10.0)]);
    g.line(&[(8.0, 15.0), (19.0, 15.0)]);
}
pub fn hand(p: &Painter, r: Rect, c: Color32) {
    let g = Glyph::new(p, r, c);
    g.polygon(&[
        (10.0, 29.0),
        (5.0, 22.0),
        (2.0, 17.0),
        (3.0, 14.0),
        (5.0, 14.0),
        (9.0, 18.0),
        (9.0, 7.0),
        (10.0, 5.0),
        (12.0, 5.0),
        (13.0, 7.0),
        (13.0, 15.0),
        (14.0, 15.0),
        (14.0, 3.0),
        (15.0, 1.0),
        (17.0, 1.0),
        (18.0, 3.0),
        (18.0, 15.0),
        (19.0, 15.0),
        (19.0, 6.0),
        (20.0, 4.0),
        (22.0, 4.0),
        (23.0, 6.0),
        (23.0, 16.0),
        (24.0, 16.0),
        (24.0, 10.0),
        (25.0, 8.0),
        (27.0, 8.0),
        (28.0, 10.0),
        (28.0, 22.0),
        (24.0, 29.0),
    ]);
}
