//! UML authoring tools belong to the floating palette, not a second action bar.

use super::*;

impl UmlEditor {
    pub(super) fn palette(&mut self, ui: &Ui, rect: Rect) {
        let tools = std::iter::once((Tool::Select, tr("common.select_move")))
            .chain(std::iter::once((Tool::Connect, tr("database.connect"))))
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
                                        Tool::Select => crate::icons::select(p, r, color),
                                        Tool::Connect => {
                                            relation_icon(p, r, color, self.relation_kind);
                                            // A disclosure mark identifies variants without another button.
                                            let corner = r.right_bottom() + vec2(4.0, 4.0);
                                            p.add(Shape::convex_polygon(
                                                vec![corner - vec2(4.0, 0.0), corner, corner - vec2(0.0, 4.0)],
                                                color,
                                                Stroke::NONE,
                                            ));
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
                                if *tool == Tool::Connect {
                                    self.relation_menu(ui, &response);
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

    fn relation_menu(&mut self, ui: &Ui, response: &egui::Response) {
        let popup_id = egui::Popup::default_response_id(response);
        let hover_id = response.id.with("relation_hover");
        let hovered = response.hovered();
        let entered = ui.ctx().data_mut(|data| {
            let previous = data.get_temp::<bool>(hover_id).unwrap_or(false);
            data.insert_temp(hover_id, hovered);
            hovered && !previous
        });
        let pointer = ui.ctx().pointer_hover_pos();
        let old_rect = ui.ctx().read_response(popup_id).map(|r| r.rect);
        let moving = ui.input(|i| i.pointer.delta() != egui::Vec2::ZERO);
        if egui::Popup::is_id_open(ui.ctx(), popup_id)
            && moving
            && pointer.is_some_and(|p| !response.rect.expand(5.0).contains(p) && old_rect.is_none_or(|r| !r.expand(5.0).contains(p)))
        {
            egui::Popup::close_id(ui.ctx(), popup_id);
        }
        let hover_open = entered
            && self.drag.is_none()
            && self.editing.is_none()
            && !ui.input(|i| i.pointer.any_down())
            && (!egui::Popup::is_any_open(ui.ctx()) || egui::Popup::is_id_open(ui.ctx(), popup_id));
        let activated = response.clicked() || response.secondary_clicked();
        let action = (hover_open || activated).then_some(egui::SetOpenCommand::Bool(true));
        egui::Popup::menu(response)
            .open_memory(action)
            // Clicking a tool whose menu already opened on hover keeps that menu open.
            .close_behavior(if activated {
                egui::PopupCloseBehavior::IgnoreClicks
            } else {
                egui::PopupCloseBehavior::CloseOnClickOutside
            })
            .align(egui::RectAlign::RIGHT_START)
            .gap(4.0)
            .width(220.0)
            .show(|ui| {
                crate::theme::menu_style(ui.style_mut());
                ui.label(tr("uml.relation_types"));
                ui.separator();
                for &kind in self.diagram.kind.relation_kinds() {
                    let icon = egui::Atom::paint(vec2(34.0, 18.0), move |ui, args| {
                        relation_icon(ui.painter(), args.rect.shrink(2.0), args.fallback_text_color, kind)
                    });
                    if ui
                        .add(egui::Button::selectable(self.relation_kind == kind, (icon, tr(kind.label()))))
                        .clicked()
                    {
                        self.relation_kind = kind;
                        self.tool = Tool::Connect;
                        self.pending = None;
                        ui.close();
                    }
                }
            });
    }

    pub(super) fn arrange_actions(&mut self, ui: &mut Ui) {
        crate::theme::menu_style(ui.style_mut());
        for (label, horizontal, edge) in [
            (tr("common.left"), true, -1),
            (tr("canvas.center_horizontal"), true, 0),
            (tr("common.right"), true, 1),
            (tr("common.top"), false, -1),
            (tr("canvas.center_vertical"), false, 0),
            (tr("common.bottom"), false, 1),
        ] {
            if ui.button(label).clicked() {
                self.align(horizontal, edge);
                ui.close();
            }
        }
        for (label, horizontal) in [
            (tr("canvas.distribute_horizontal"), true),
            (tr("canvas.distribute_vertical"), false),
        ] {
            if ui.add_enabled(self.group.len() > 2, egui::Button::new(label)).clicked() {
                self.distribute(horizontal);
                ui.close();
            }
        }
    }
}

fn relation_icon(p: &Painter, rect: Rect, color: egui::Color32, kind: RelationKind) {
    let start = rect.left_center();
    let end = rect.right_center();
    let size = rect.height().min(rect.width() * 0.4).min(9.0);
    let stroke = Stroke::new(1.4, color);
    let diamond = matches!(kind, RelationKind::Aggregation | RelationKind::Composition);
    let triangle = matches!(kind, RelationKind::Generalization | RelationKind::Realization);
    let from = start + vec2(if diamond { size * 1.4 } else { 0.0 }, 0.0);
    let to = end - vec2(if triangle { size } else { 0.0 }, 0.0);
    if kind.dashed() {
        let mut x = from.x;
        while x < to.x {
            p.line_segment([pos2(x, from.y), pos2((x + 3.0).min(to.x), to.y)], stroke);
            x += 5.0;
        }
    } else {
        p.line_segment([from, to], stroke);
    }
    if diamond {
        p.add(Shape::convex_polygon(
            vec![
                start,
                start + vec2(size * 0.7, -size * 0.4),
                start + vec2(size * 1.4, 0.0),
                start + vec2(size * 0.7, size * 0.4),
            ],
            if kind == RelationKind::Composition {
                color
            } else {
                egui::Color32::TRANSPARENT
            },
            stroke,
        ));
    } else if triangle {
        p.add(Shape::convex_polygon(
            vec![end, end - vec2(size, size * 0.5), end + vec2(-size, size * 0.5)],
            egui::Color32::TRANSPARENT,
            stroke,
        ));
    } else if kind != RelationKind::Association {
        canvas::arrow_head(p, end, vec2(1.0, 0.0), size, color, false);
    }
}
