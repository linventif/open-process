//! Shared resize picking, pointer offsets, cursors and visual feedback.

use super::*;
use op_core::geom::Rect as Bounds;
pub use op_core::geom::ResizeEdge;

pub const EDGE_TOLERANCE: f32 = 6.0;
pub const CORNER_TOLERANCE: f32 = 8.0;

#[derive(Clone, Copy, Debug)]
pub struct ResizeGesture {
    pub edge: ResizeEdge,
    grab: Pos,
}

impl ResizeGesture {
    pub fn new(bounds: Bounds, edge: ResizeEdge, pointer: Pos) -> Self {
        Self {
            edge,
            grab: pointer - edge.anchor(bounds),
        }
    }

    pub fn apply(self, bounds: Bounds, pointer: Pos, min: Pos, max: Pos, grid: Option<f32>) -> Bounds {
        self.edge.resize(bounds, pointer - self.grab, min, max, grid)
    }
}

/// Pick the topmost visible element; interiors occlude elements behind them.
pub fn target<I: Copy>(
    view: View,
    canvas: Rect,
    pointer: Pos2,
    items: impl DoubleEndedIterator<Item = (I, Bounds)>,
) -> Option<(I, ResizeEdge)> {
    for (id, bounds) in items.rev() {
        let r = view.rect_to_screen(canvas, bounds);
        if let Some(edge) = hit(r, pointer) {
            return Some((id, edge));
        }
        if r.contains(pointer) {
            return None;
        }
    }
    None
}

pub fn hit(r: Rect, p: Pos2) -> Option<ResizeEdge> {
    let tolerance = EDGE_TOLERANCE;
    let corner = CORNER_TOLERANCE;
    if !r.expand(corner).contains(p) {
        return None;
    }
    use ResizeEdge::*;
    for (point, edge) in [
        (r.left_top(), TopLeft),
        (r.right_top(), TopRight),
        (r.left_bottom(), BottomLeft),
        (r.right_bottom(), BottomRight),
    ] {
        if (p.x - point.x).abs() <= corner && (p.y - point.y).abs() <= corner {
            return Some(edge);
        }
    }
    if (p.x - r.left()).abs() <= tolerance {
        Some(Left)
    } else if (p.x - r.right()).abs() <= tolerance {
        Some(Right)
    } else if (p.y - r.top()).abs() <= tolerance {
        Some(Top)
    } else if (p.y - r.bottom()).abs() <= tolerance {
        Some(Bottom)
    } else {
        None
    }
}

pub fn cursor(edge: ResizeEdge) -> egui::CursorIcon {
    use ResizeEdge::*;
    match edge {
        Left | Right => egui::CursorIcon::ResizeHorizontal,
        Top | Bottom => egui::CursorIcon::ResizeVertical,
        TopLeft | BottomRight => egui::CursorIcon::ResizeNwSe,
        TopRight | BottomLeft => egui::CursorIcon::ResizeNeSw,
    }
}

pub fn draw(painter: &Painter, r: Rect, edge: ResizeEdge, color: Color32) {
    use ResizeEdge::*;
    let (a, b, c) = match edge {
        Left => (r.left_top(), r.left_bottom(), None),
        Right => (r.right_top(), r.right_bottom(), None),
        Top => (r.left_top(), r.right_top(), None),
        Bottom => (r.left_bottom(), r.right_bottom(), None),
        _ => {
            let corner = match edge {
                TopLeft => r.left_top(),
                TopRight => r.right_top(),
                BottomLeft => r.left_bottom(),
                _ => r.right_bottom(),
            };
            let len = 14.0_f32.min(r.width() / 2.0).min(r.height() / 2.0);
            let x = if matches!(edge, TopLeft | BottomLeft) { len } else { -len };
            let y = if matches!(edge, TopLeft | TopRight) { len } else { -len };
            (corner + vec2(x, 0.0), corner, Some(corner + vec2(0.0, y)))
        }
    };
    painter.line_segment([a, b], Stroke::new(2.5, color));
    if let Some(c) = c {
        painter.line_segment([b, c], Stroke::new(2.5, color));
    }
}

pub fn feedback(ui: &Ui, painter: &Painter, view: View, canvas: Rect, bounds: Bounds, edge: ResizeEdge, color: Color32) {
    ui.ctx().set_cursor_icon(cursor(edge));
    draw(painter, view.rect_to_screen(canvas, bounds), edge, color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_picking_uses_screen_tolerance_and_respects_occlusion() {
        let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(2000.0, 2000.0));
        let bounds = Bounds::new(100.0, 100.0, 300.0, 200.0);
        for zoom in [0.2, 1.0, 3.0] {
            let view = View {
                offset: vec2(40.0, 40.0),
                zoom,
            };
            for edge in ResizeEdge::ALL {
                let point = view.to_screen(canvas, edge.anchor(bounds)) + vec2(3.0, 2.0);
                assert_eq!(target(view, canvas, point, [(1, bounds)].into_iter()), Some((1, edge)));
            }
            assert!(target(view, canvas, view.to_screen(canvas, bounds.center()), [(1, bounds)].into_iter()).is_none());
        }
        let view = View {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
        };
        let covering = Bounds::new(50.0, 150.0, 150.0, 100.0);
        let point = pos2(100.0, 200.0);
        assert!(target(view, canvas, point, [(1, bounds), (2, covering)].into_iter()).is_none());
        assert_eq!(
            target(view, canvas, pos2(50.0, 200.0), [(1, bounds), (2, covering)].into_iter()),
            Some((2, ResizeEdge::Left))
        );
    }
}
