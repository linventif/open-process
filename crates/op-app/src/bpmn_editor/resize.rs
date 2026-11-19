//! Screen-space pool/lane resize affordances. Geometry rules live in op-core.

use super::*;

pub(super) fn resize_cursor(target: PoolResize) -> egui::CursorIcon {
    target
        .edge()
        .map(canvas::resize::cursor)
        .unwrap_or(egui::CursorIcon::ResizeVertical)
}

impl BpmnEditor {
    pub(super) fn node_resize_at(&self, canvas: Rect, pointer: Pos2) -> Option<(Id, canvas::resize::ResizeEdge)> {
        if self.tool != Tool::Select
            || self.editing.is_some()
            || self.dnd.is_some()
            || self.toolbar_rect.contains(pointer)
            || self.pad_buttons(canvas).iter().any(|(r, _, _)| r.contains(pointer))
        {
            return None;
        }
        canvas::resize::target(self.view, canvas, pointer, self.diagram.nodes.iter().map(|n| (n.id, n.rect()))).filter(|(id, _)| {
            self.diagram
                .node(*id)
                .is_some_and(|n| matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess | NodeKind::TextAnnotation))
        })
    }

    pub(super) fn pool_resize_at(&self, canvas: Rect, pointer: Pos2) -> Option<(Id, PoolResize)> {
        if self.tool != Tool::Select
            || self.editing.is_some()
            || self.dnd.is_some()
            || self.toolbar_rect.contains(pointer)
            || self.pad_buttons(canvas).iter().any(|(r, _, _)| r.contains(pointer))
        {
            return None;
        }
        // Elements and their own handles remain reachable on top of pool borders.
        if self.hit_node(self.view.to_world(canvas, pointer)).is_some() {
            return None;
        }
        let tolerance = canvas::resize::EDGE_TOLERANCE;
        for pool in self.diagram.pools.iter().rev().filter(|p| !p.lanes.is_empty()) {
            let r = self.view.rect_to_screen(canvas, pool.rect());
            if !r.expand(canvas::resize::CORNER_TOLERANCE).contains(pointer) {
                continue;
            }
            let edge = canvas::resize::hit(r, pointer);
            if edge.is_some_and(|e| {
                matches!(
                    e,
                    canvas::resize::ResizeEdge::TopLeft
                        | canvas::resize::ResizeEdge::TopRight
                        | canvas::resize::ResizeEdge::BottomLeft
                        | canvas::resize::ResizeEdge::BottomRight
                )
            }) {
                return edge.map(|e| (pool.id, e.into()));
            }
            let lane_left = r.left() + POOL_HEADER * self.view.zoom;
            let mut separator = pool.pos.y;
            for (i, lane) in pool.lanes[..pool.lanes.len() - 1].iter().enumerate() {
                separator += lane.height;
                let y = self.view.to_screen(canvas, Pos::new(pool.pos.x, separator)).y;
                if pointer.x >= lane_left - tolerance && pointer.x <= r.right() + tolerance && (pointer.y - y).abs() <= tolerance {
                    return Some((pool.id, PoolResize::LaneBoundary(i)));
                }
            }
            if let Some(edge) = edge {
                return Some((pool.id, edge.into()));
            }
        }
        None
    }

    pub(super) fn draw_pool_resize(&self, painter: &Painter, canvas: Rect, pointer: Option<Pos2>, color: Color32) {
        let target = match self.drag {
            Some(Drag::ResizePool { pool, target, .. }) => Some((pool, target)),
            None => pointer.and_then(|p| self.pool_resize_at(canvas, p)),
            _ => None,
        };
        let Some((id, target)) = target else { return };
        let Some(pool) = self.diagram.pool(id) else { return };
        if let Some(edge) = target.edge() {
            canvas::resize::draw(painter, self.view.rect_to_screen(canvas, pool.rect()), edge, color);
        } else if let PoolResize::LaneBoundary(i) = target
            && i < pool.lanes.len().saturating_sub(1)
        {
            let lane = self.view.rect_to_screen(canvas, pool.lane_rect(i));
            painter.line_segment([lane.left_bottom(), lane.right_bottom()], Stroke::new(2.5, color));
        }
    }
}
