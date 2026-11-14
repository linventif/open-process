//! Render for the BpmnEditor notation.

use super::*;

impl BpmnEditor {
    pub(super) fn draw(&self, painter: &Painter, canvas: Rect, pal: &Palette, wp: Option<Pos>, pointer: Option<Pos2>) {
        canvas::set_sketch(self.diagram.style.sketch);
        self.draw_inner(painter, canvas, pal, wp, pointer);
        canvas::set_sketch(false);
    }

    pub(super) fn draw_inner(&self, painter: &Painter, canvas: Rect, pal: &Palette, wp: Option<Pos>, pointer: Option<Pos2>) {
        let sketch = self.diagram.style.sketch;
        let v = &self.view;
        let z = v.zoom;
        let hover = wp.and_then(|w| self.hit(w));
        let font_size = (12.0 * z).clamp(6.0, 26.0);

        // Pools & lanes
        for p in &self.diagram.pools {
            let r = v.rect_to_screen(canvas, p.rect());
            let sel = self.selection == Some(Sel::Pool(p.id));
            painter.rect_filled(r, 0.0, pal.fill.gamma_multiply(0.6));
            let header = Rect::from_min_size(r.min, vec2(POOL_HEADER * z, r.height()));
            painter.rect_filled(header, 0.0, pal.pool_header);
            vertical_text(painter, header.center(), &p.name, font_size * 1.05, pal.text);
            if p.lanes.len() > 1 {
                for i in 0..p.lanes.len() {
                    let lr = v.rect_to_screen(canvas, p.lane_rect(i));
                    let lh = Rect::from_min_size(lr.min, vec2(LANE_HEADER * z, lr.height()));
                    let lsel = self.selection == Some(Sel::Lane(p.id, i));
                    painter.rect_filled(
                        lh,
                        0.0,
                        if lsel {
                            pal.select.gamma_multiply(0.2)
                        } else {
                            pal.pool_header.gamma_multiply(0.7)
                        },
                    );
                    painter.line_segment([lh.right_top(), lh.right_bottom()], Stroke::new(1.0, pal.ink));
                    vertical_text(painter, lh.center(), &p.lanes[i].name, font_size, pal.text);
                    if i > 0 {
                        painter.line_segment([lr.left_top(), lr.right_top()], Stroke::new(1.0, pal.ink));
                    }
                }
            }
            painter.line_segment([header.right_top(), header.right_bottom()], Stroke::new(1.2, pal.ink));
            let stroke = if sel {
                Stroke::new(2.2, pal.select)
            } else {
                Stroke::new(1.4, pal.ink)
            };
            if sketch {
                canvas::sketch_rect(painter, r, stroke, p.id as u64);
            } else {
                painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            }
        }

        // Flows
        for f in &self.diagram.flows {
            let pts: Vec<Pos2> = route(&self.diagram, f).into_iter().map(|p| v.to_screen(canvas, p)).collect();
            if pts.len() < 2 {
                continue;
            }
            let sel = self.selection == Some(Sel::Flow(f.id));
            let hov = hover == Some(Sel::Flow(f.id));
            let color = if sel {
                pal.select
            } else if hov {
                pal.select.gamma_multiply(0.7)
            } else {
                pal.ink
            };
            let width = if sel { 2.2 } else { 1.4 } * z.max(0.6);
            let stroke = Stroke::new(width, color);
            let n = pts.len();
            let dir = pts[n - 1] - pts[n - 2];
            let head = 10.0 * z.max(0.5);
            match f.kind {
                FlowKind::Sequence => {
                    let mut body = pts.clone();
                    if sketch {
                        canvas::sketch_polyline(painter, &body, stroke, f.id as u64, false);
                        canvas::sketch_arrow_head(painter, pts[n - 1], dir, head, stroke, f.id as u64);
                    } else {
                        body[n - 1] = pts[n - 1] - dir.normalized() * head * 0.8;
                        canvas::polyline(painter, &body, stroke);
                        canvas::arrow_head(painter, pts[n - 1], dir, head, color, true);
                    }
                    if f.is_default {
                        let d0 = (pts[1] - pts[0]).normalized();
                        let c = pts[0] + d0 * 10.0 * z;
                        let nrm = egui::vec2(-d0.y, d0.x);
                        painter.line_segment([c + (nrm - d0) * 5.0 * z, c - (nrm - d0) * 5.0 * z], stroke);
                    }
                }
                FlowKind::Message => {
                    canvas::dashed(painter, &pts, stroke, 7.0 * z, 4.0 * z);
                    painter.circle(pts[0], 3.5 * z, pal.fill, stroke);
                    let tip = pts[n - 1];
                    let d = dir.normalized();
                    let nrm = egui::vec2(-d.y, d.x);
                    let base = tip - d * head;
                    painter.add(Shape::convex_polygon(
                        vec![tip, base + nrm * head * 0.45, base - nrm * head * 0.45],
                        pal.fill,
                        stroke,
                    ));
                }
                FlowKind::Association => {
                    canvas::dashed(painter, &pts, Stroke::new(width, color.gamma_multiply(0.8)), 2.0 * z, 3.0 * z);
                    let to_data = self
                        .diagram
                        .node(f.to)
                        .is_some_and(|n| matches!(n.kind, NodeKind::DataObject | NodeKind::DataStore));
                    let from_data = self
                        .diagram
                        .node(f.from)
                        .is_some_and(|n| matches!(n.kind, NodeKind::DataObject | NodeKind::DataStore));
                    if to_data || from_data {
                        canvas::arrow_head(painter, pts[n - 1], dir, head * 0.8, color, false);
                    }
                }
            }
            if !f.name.is_empty() {
                let mid = canvas::polyline_midpoint(&pts);
                canvas::label_with_halo(painter, mid + vec2(0.0, -10.0 * z), &f.name, font_size * 0.95, pal);
            }
        }

        // Nodes
        let issues = if self.show_issues { self.diagram.validate() } else { Vec::new() };
        for n in &self.diagram.nodes {
            let r = v.rect_to_screen(canvas, n.rect());
            let sel = self.selection == Some(Sel::Node(n.id)) || self.group.contains(&n.id);
            let hov = hover == Some(Sel::Node(n.id)) || self.connect_pending == Some(n.id);
            if let Some(sev) = issues
                .iter()
                .filter(|i| i.node == Some(n.id))
                .map(|i| i.severity)
                .max_by_key(|s| *s == Severity::Error)
            {
                let c = if sev == Severity::Error { pal.bad } else { pal.warn };
                painter.rect_stroke(r.expand(6.0 * z), CornerRadius::same(6), Stroke::new(2.0, c), StrokeKind::Outside);
            }
            paint_node(painter, r, n.kind, &n.name, z, pal, None);
            if sel || hov {
                let c = if sel { pal.select } else { pal.select.gamma_multiply(0.6) };
                painter.rect_stroke(r.expand(4.0), CornerRadius::same(4), Stroke::new(1.5, c), StrokeKind::Outside);
            }
        }

        // Rubber band for connections
        let from = match self.drag {
            Some(Drag::Connect { from }) => Some(from),
            _ => self.connect_pending,
        };
        if let (Some(from), Some(p)) = (from, pointer)
            && let Some(n) = self.diagram.node(from)
        {
            let world = v.to_world(canvas, p);
            let target = self.hit_node(world).and_then(|id| self.diagram.node(id));
            let toward = target.map(|n| n.pos).unwrap_or(world);
            let end = target.map(|target| v.to_screen(canvas, boundary_point(target, n.pos))).unwrap_or(p);
            canvas::dashed(
                painter,
                &[v.to_screen(canvas, boundary_point(n, toward)), end],
                Stroke::new(1.5, pal.select),
                6.0,
                4.0,
            );
            canvas::connection_ports(painter, v.rect_to_screen(canvas, n.rect()), pointer, pal.select);
        }
        if (self.tool == Tool::Connect || from.is_some())
            && let Some(target) = wp.and_then(|w| self.hit_node(w)).and_then(|id| self.diagram.node(id))
        {
            canvas::connection_ports(painter, v.rect_to_screen(canvas, target.rect()), pointer, pal.select);
        }

        // Resize handle
        if let Some((h, _)) = self.resize_handle(canvas) {
            painter.rect(h, CornerRadius::same(2), pal.fill, Stroke::new(1.5, pal.select), StrokeKind::Middle);
        }

        // Context pad (card with shadow behind the buttons)
        let pad = self.pad_buttons(canvas);
        if let Some(bg) = pad.iter().map(|(r, _, _)| *r).reduce(|a, b| a.union(b)) {
            let bg = bg.expand(5.0);
            canvas::shadow_rect(painter, bg, 9.0, pal);
            painter.rect(
                bg,
                CornerRadius::same(9),
                pal.fill,
                Stroke::new(1.0, pal.border),
                StrokeKind::Inside,
            );
        }
        for (r, action, _) in pad {
            let hovered = pointer.is_some_and(|p| r.contains(p));
            if hovered {
                painter.rect_filled(r, CornerRadius::same(6), pal.select.gamma_multiply(0.18));
            }
            let inner = r.shrink(5.0);
            match action {
                PadAction::Append(kind) => paint_node(painter, inner, kind, "", 0.35, pal, Some(pal.ink)),
                PadAction::Connect => {
                    let a = inner.left_bottom();
                    let b = inner.right_top();
                    painter.line_segment([a, b - (b - a).normalized() * 4.0], Stroke::new(1.3, pal.ink));
                    canvas::arrow_head(painter, b, b - a, 6.0, pal.ink, true);
                }
                PadAction::Delete => {
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, "🗑", FontId::proportional(13.0), pal.bad);
                }
            }
        }
        if let Some(p) = pointer
            && let Some((_, _, tip)) = self.pad_buttons(canvas).into_iter().find(|(r, _, _)| r.contains(p))
        {
            canvas::badge(painter, p + vec2(0.0, -20.0), tip, 11.5, pal.text, pal);
        }
    }

    // ------------------------------------------------------------------
    // Side panel
}
