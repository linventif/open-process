//! UML rendering and the label hit regions share the same geometry.

use super::*;

impl UmlEditor {
    pub(super) fn draw_node(&self, p: &Painter, canvas: Rect, n: &Node, pal: &Palette, exporting: bool) {
        let r = self.view.rect_to_screen(canvas, n.rect());
        let z = self.view.zoom;
        let selected = !exporting && self.group.contains(&n.id);
        let stroke = Stroke::new(if selected { 2.0 } else { 1.3 }, if selected { pal.select } else { pal.ink });
        let text_painter = p.with_clip_rect(r.shrink(3.0 * z).intersect(p.clip_rect()));
        let font = FontId::proportional(14.0 * z);
        match n.kind {
            NodeKind::System => {
                p.rect_stroke(r, 0.0, stroke, StrokeKind::Inside);
                text_painter.text(r.left_top() + vec2(12.0, 8.0) * z, Align2::LEFT_TOP, &n.name, font, pal.text);
            }
            NodeKind::UseCase => {
                p.add(egui::epaint::EllipseShape {
                    center: r.center(),
                    angle: 0.0,
                    radius: r.size() / 2.0,
                    fill: pal.task_fill,
                    stroke,
                });
                let galley = text_painter.layout(n.name.clone(), font, pal.text, r.width() * 0.75);
                text_painter.galley(r.center() - galley.size() / 2.0, galley, pal.text);
            }
            NodeKind::Actor => {
                let cx = r.center().x;
                p.circle_stroke(pos2(cx, r.top() + 15.0 * z), 12.0 * z, stroke);
                let neck = pos2(cx, r.top() + 27.0 * z);
                let hip = pos2(cx, r.top() + 66.0 * z);
                p.line_segment([neck, hip], stroke);
                p.line_segment(
                    [pos2(cx - 25.0 * z, r.top() + 40.0 * z), pos2(cx + 25.0 * z, r.top() + 40.0 * z)],
                    stroke,
                );
                p.line_segment([hip, pos2(cx - 22.0 * z, r.top() + 95.0 * z)], stroke);
                p.line_segment([hip, pos2(cx + 22.0 * z, r.top() + 95.0 * z)], stroke);
                text_painter.text(pos2(cx, r.top() + 103.0 * z), Align2::CENTER_TOP, &n.name, font, pal.text);
                if selected {
                    p.rect_stroke(r.expand(3.0), 4.0, Stroke::new(1.0, pal.select), StrokeKind::Outside);
                }
            }
            _ => {
                p.rect_filled(r, 0.0, pal.fill);
                p.rect_filled(Rect::from_min_size(r.min, vec2(r.width(), 44.0 * z)), 0.0, pal.task_fill);
                p.rect_stroke(r, 0.0, stroke, StrokeKind::Inside);
                let stereotype = match n.kind {
                    NodeKind::Interface => "«interface»",
                    NodeKind::Enumeration => "«enumeration»",
                    _ if n.abstract_class => "{abstract}",
                    _ => "",
                };
                if !stereotype.is_empty() {
                    text_painter.text(
                        pos2(r.center().x, r.top() + 4.0 * z),
                        Align2::CENTER_TOP,
                        stereotype,
                        FontId::proportional(11.0 * z),
                        pal.muted,
                    );
                }
                text_painter.text(
                    pos2(r.center().x, r.top() + 24.0 * z),
                    Align2::CENTER_CENTER,
                    &n.name,
                    crate::theme::semibold(14.0 * z),
                    pal.text,
                );
                let y = r.top() + 44.0 * z;
                let sep = y + (n.attributes.lines().count().max(1) as f32 * 20.0 + 8.0) * z;
                p.hline(r.x_range(), y, stroke);
                p.hline(r.x_range(), sep, stroke);
                for (text, top) in [(&n.attributes, y), (&n.operations, sep)] {
                    for (i, line) in text.lines().enumerate() {
                        text_painter.text(
                            pos2(r.left() + 8.0 * z, top + (5.0 + i as f32 * 20.0) * z),
                            Align2::LEFT_TOP,
                            line,
                            FontId::monospace(12.0 * z),
                            pal.text,
                        );
                    }
                }
            }
        }
        if selected && self.group.len() == 1 {
            let handle = Rect::from_center_size(r.right_bottom(), vec2(8.0, 8.0));
            p.rect_filled(handle, 1.0, pal.select);
        }
    }

    pub(super) fn draw_relation(&self, p: &Painter, canvas: Rect, r: &Relation, pal: &Palette, exporting: bool) -> Vec<Rect> {
        let points: Vec<_> = self
            .relation_points(r)
            .into_iter()
            .map(|v| self.view.to_screen(canvas, v))
            .collect();
        if points.len() < 2 {
            return Vec::new();
        }
        let selected = !exporting && self.selection == Some(Selection::Relation(r.id));
        let color = if selected { pal.select } else { pal.ink };
        let stroke = Stroke::new(if selected { 2.0 } else { 1.4 }, color);
        let z = self.view.zoom;
        for segment in points.windows(2) {
            if r.kind.dashed() {
                let len = segment[0].distance(segment[1]);
                let dir = (segment[1] - segment[0]).normalized();
                if len > 0.01 {
                    let mut d = 0.0;
                    while d < len {
                        p.line_segment([segment[0] + dir * d, segment[0] + dir * (d + 7.0 * z).min(len)], stroke);
                        d += 12.0 * z;
                    }
                }
            } else {
                p.line_segment([segment[0], segment[1]], stroke);
            }
        }
        let start = points[0];
        let end = *points.last().unwrap();
        let direction = (end - points[points.len() - 2]).normalized();
        let size = 14.0 * z;
        if direction.is_finite() {
            match r.kind {
                RelationKind::Generalization | RelationKind::Realization => {
                    let normal = vec2(-direction.y, direction.x);
                    p.add(Shape::convex_polygon(
                        vec![
                            end,
                            end - direction * size + normal * size * 0.5,
                            end - direction * size - normal * size * 0.5,
                        ],
                        pal.bg,
                        stroke,
                    ));
                }
                RelationKind::Association | RelationKind::Aggregation | RelationKind::Composition => {}
                _ => canvas::arrow_head(p, end, direction, size, color, false),
            }
        }
        if matches!(r.kind, RelationKind::Aggregation | RelationKind::Composition) {
            let dir = (points[1] - start).normalized();
            if dir.is_finite() {
                let normal = vec2(-dir.y, dir.x);
                p.add(Shape::convex_polygon(
                    vec![
                        start,
                        start + dir * size * 0.7 + normal * size * 0.4,
                        start + dir * size * 1.4,
                        start + dir * size * 0.7 - normal * size * 0.4,
                    ],
                    if r.kind == RelationKind::Composition { color } else { pal.bg },
                    stroke,
                ));
            }
        }
        let label = match r.kind {
            RelationKind::Include => format!("«include»\n{}", r.label),
            RelationKind::Extend => format!("«extend»\n{}", r.label),
            _ => r.label.clone(),
        };
        let mut hits = Vec::new();
        let mid = canvas::point_along(&points, 0.5);
        let tangent = (canvas::point_along(&points, 0.51) - canvas::point_along(&points, 0.49)).normalized();
        let normal = if tangent.is_finite() {
            vec2(-tangent.y, tangent.x)
        } else {
            vec2(0.0, -1.0)
        };
        let label_pos = mid + normal * 16.0 * z;
        if let Some(rect) = self.relation_text(p, label_pos, &label, pal) {
            hits.push(rect);
        }
        // Each end follows its own segment, including on routes with several bends.
        for (anchor, direction, text) in [
            (
                start,
                (points[1] - start).normalized(),
                format!("{} {}", r.source_role, r.source_multiplicity),
            ),
            (
                end,
                (points[points.len() - 2] - end).normalized(),
                format!("{} {}", r.target_role, r.target_multiplicity),
            ),
        ] {
            if direction.is_finite() {
                let normal = vec2(-direction.y, direction.x);
                if let Some(rect) = self.relation_text(p, anchor + direction * 30.0 * z + normal * 14.0 * z, text.trim(), pal) {
                    hits.push(rect);
                }
            }
        }
        hits
    }

    fn relation_text(&self, p: &Painter, pos: egui::Pos2, text: &str, pal: &Palette) -> Option<Rect> {
        if text.trim().is_empty() {
            return None;
        }
        let galley = p.layout_no_wrap(text.into(), FontId::proportional(12.0 * self.view.zoom), pal.text);
        let rect = Rect::from_center_size(pos, galley.size() + vec2(6.0, 3.0));
        p.rect_filled(rect, 2.0, pal.bg);
        p.galley(pos - galley.size() / 2.0, galley, pal.text);
        Some(rect)
    }
}

impl UmlEditor {
    pub(super) fn palette(&mut self, ui: &Ui, rect: Rect) {
        let tools = std::iter::once((Tool::Select, tr("Select / move")))
            .chain(std::iter::once((Tool::Connect, tr("Connect"))))
            .chain(
                self.diagram
                    .kind
                    .node_kinds()
                    .iter()
                    .map(|&kind| (Tool::Create(kind), tr(kind.label()))),
            )
            .collect::<Vec<_>>();
        canvas::floating(
            ui.ctx(),
            egui::Id::unique(("uml_palette", self.uid)),
            rect.left_top() + vec2(14.0, 14.0),
            Align2::LEFT_TOP,
            |ui| {
                egui::ScrollArea::vertical()
                    .max_height((rect.height() - 40.0).max(40.0))
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            for (i, (tool, label)) in tools.iter().enumerate() {
                                if i == 2 {
                                    canvas::toolbar_separator(ui);
                                }
                                let response = canvas::icon_button(ui, self.tool == *tool, label, |p, r, color| {
                                    let stroke = Stroke::new(1.4, color);
                                    match tool {
                                        Tool::Select => {
                                            p.add(Shape::closed_line(
                                                vec![r.left_top(), r.center_bottom(), r.center(), r.right_center()],
                                                stroke,
                                            ));
                                        }
                                        Tool::Connect => {
                                            p.line_segment([r.left_bottom(), r.right_top()], stroke);
                                            canvas::arrow_head(p, r.right_top(), vec2(1.0, -1.0), 7.0, color, false);
                                        }
                                        Tool::Create(NodeKind::UseCase) => {
                                            p.add(egui::epaint::EllipseShape {
                                                center: r.center(),
                                                radius: r.size() / 2.0,
                                                angle: 0.0,
                                                fill: egui::Color32::TRANSPARENT,
                                                stroke,
                                            });
                                        }
                                        Tool::Create(NodeKind::Actor) => {
                                            let c = r.center();
                                            p.circle_stroke(pos2(c.x, r.top() + 3.0), 3.0, stroke);
                                            p.line_segment([pos2(c.x, r.top() + 6.0), pos2(c.x, r.bottom() - 5.0)], stroke);
                                            p.line_segment([pos2(r.left(), c.y), pos2(r.right(), c.y)], stroke);
                                            p.line_segment([pos2(c.x, r.bottom() - 5.0), r.left_bottom()], stroke);
                                            p.line_segment([pos2(c.x, r.bottom() - 5.0), r.right_bottom()], stroke);
                                        }
                                        Tool::Create(kind) => {
                                            p.rect_stroke(r, 1.0, stroke, StrokeKind::Inside);
                                            p.hline(r.x_range(), r.top() + 5.0, stroke);
                                            if kind.is_classifier() {
                                                p.hline(r.x_range(), r.top() + 11.0, stroke);
                                            }
                                            if *kind == NodeKind::Interface {
                                                p.circle_stroke(r.center(), 2.0, stroke);
                                            }
                                            if *kind == NodeKind::Enumeration {
                                                p.text(r.center(), Align2::CENTER_CENTER, "E", FontId::proportional(9.0), color);
                                            }
                                        }
                                    }
                                });
                                if response.clicked() {
                                    self.tool = *tool;
                                    self.pending = None;
                                }
                                if response.drag_started()
                                    && let Tool::Create(kind) = tool
                                {
                                    self.dnd = Some(*kind);
                                }
                            }
                        });
                    });
            },
        );
    }

    pub(super) fn draw_overlays(&self, ui: &Ui, p: &Painter, rect: Rect, pal: &Palette) {
        if self.diagram.nodes.is_empty() {
            p.text(
                rect.center() - vec2(0.0, 14.0),
                Align2::CENTER_CENTER,
                if self.diagram.kind == DiagramKind::Class {
                    tr("Empty class diagram")
                } else {
                    tr("Empty use-case diagram")
                },
                crate::theme::semibold(18.0),
                pal.text,
            );
            p.text(
                rect.center() + vec2(0.0, 14.0),
                Align2::CENTER_CENTER,
                tr("Pick an element in the palette and click on the canvas"),
                FontId::proportional(13.0),
                pal.muted,
            );
        }
        if let Some(Drag::Rubber { start, .. }) = &self.drag
            && let Some(end) = ui.input(|i| i.pointer.hover_pos())
        {
            let area = Rect::from_two_pos(self.view.to_screen(rect, *start), end);
            p.rect(
                area,
                2.0,
                pal.select.gamma_multiply(0.08),
                Stroke::new(1.0, pal.select),
                StrokeKind::Inside,
            );
        }
        if let Some(id) = self.pending
            && let Some(n) = self.diagram.node(id)
            && let Some(end) = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p))
        {
            p.line_segment(
                [self.view.to_screen(rect, n.anchor(self.view.to_world(rect, end))), end],
                Stroke::new(1.5, pal.select),
            );
            let hover = self.hit_node(self.view.to_world(rect, end));
            if let Some(target) = hover.and_then(|id| self.diagram.node(id)) {
                let valid = self.diagram.can_connect(id, target.id, self.relation_kind);
                p.rect_stroke(
                    self.view.rect_to_screen(rect, target.rect()).expand(4.0),
                    5.0,
                    Stroke::new(2.0, if valid { pal.ok } else { pal.bad }),
                    StrokeKind::Outside,
                );
            }
        }
        if let Some(kind) = self.dnd
            && let Some(pos) = ui.input(|i| i.pointer.hover_pos()).filter(|p| rect.contains(*p))
        {
            p.rect(
                Rect::from_center_size(pos, vec2(100.0, 55.0)),
                4.0,
                pal.select.gamma_multiply(0.1),
                Stroke::new(1.5, pal.select),
                StrokeKind::Inside,
            );
            p.text(pos, Align2::CENTER_CENTER, tr(kind.label()), FontId::proportional(13.0), pal.select);
        }
    }
}
