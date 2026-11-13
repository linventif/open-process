use super::*;
use egui::{Align2, Color32, FontId, Sense, Shape, Stroke, StrokeKind, vec2};

impl WhiteboardEditor {
    pub(super) fn canvas(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size().max(vec2(1.0, 1.0)), Sense::click_and_drag());
        self.state.canvas = rect;
        self.state.camera.handle_navigation(ui, &response, rect);
        if self.state.fit {
            if let Some(b) = self.board.bounds() {
                self.state.camera.fit(rect, Pos::new(b.x, b.y), Pos::new(b.right(), b.bottom()));
            }
            self.state.fit = false;
        }
        self.shortcuts(ui, &response);
        let world = ui.input(|i| i.pointer.interact_pos()).map(|p| self.state.camera.to_world(rect, p));
        self.interact(ui, &response, world);
        let exporting = std::mem::take(&mut self.state.export_png);
        let pal = canvas::Palette::for_ui(ui);
        let p = ui.painter().with_clip_rect(rect);
        if self.state.grid {
            canvas::draw_background(&p, rect, &self.state.camera, &pal);
        } else {
            p.rect_filled(rect, 0.0, pal.bg);
        }
        for e in &self.board.elements {
            self.draw(&p, e, pal.dark);
        }
        if !exporting {
            if let Some(e) = self.preview() {
                self.draw(&p, &e, pal.dark);
            }
            for e in self.board.elements.iter().filter(|e| self.state.selection.contains(&e.id)) {
                let r = self.state.camera.rect_to_screen(rect, e.bounds);
                p.rect_stroke(r.expand(4.0), 3.0, Stroke::new(1.0, pal.select), StrokeKind::Outside);
                if self.state.selection.len() == 1 {
                    p.rect_filled(Rect::from_center_size(r.right_bottom(), vec2(8.0, 8.0)), 2.0, pal.select);
                }
            }
            if let Some(Drag::Rubber { start, current, .. }) = self.state.drag {
                let r = Rect::from_two_pos(self.state.camera.to_screen(rect, start), self.state.camera.to_screen(rect, current));
                p.rect_filled(r, 0.0, pal.select.gamma_multiply(0.08));
                p.rect_stroke(r, 0.0, Stroke::new(1.0, pal.select), StrokeKind::Inside);
            }
            if self.board.elements.is_empty() && self.state.drag.is_none() {
                p.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    tr("Choose a drawing tool, then drag on the canvas."),
                    FontId::proportional(15.0),
                    pal.muted,
                );
            }
            canvas::floating(
                ui.ctx(),
                egui::Id::unique(("whiteboard_palette", self.state.uid)),
                rect.left_top() + vec2(14.0, 14.0),
                Align2::LEFT_TOP,
                |ui| {
                    ui.set_width(36.0);
                    egui::ScrollArea::vertical()
                        .max_height((rect.height() - 40.0).max(40.0))
                        .show(ui, |ui| ui.vertical_centered(|ui| self.toolbar(ui)));
                },
            );
            if matches!(self.state.tool, Tool::Draw(_)) {
                response.on_hover_cursor(egui::CursorIcon::Crosshair);
            } else if self.state.tool == Tool::Hand {
                response.on_hover_cursor(egui::CursorIcon::Grab);
            }
        } else {
            crate::io::request_png_export(ui.ctx(), rect, self.board.name.clone());
        }
    }
    fn draw(&self, p: &egui::Painter, e: &Element, dark: bool) {
        let r = self.state.camera.rect_to_screen(self.state.canvas, e.bounds);
        let z = self.state.camera.zoom;
        // Saved colors stay theme-independent. Lift dark ink only for the dark canvas.
        let [red, green, blue] = e.paint.stroke;
        let ink = if dark && red.max(green).max(blue) < 120 {
            Color32::from_rgb(218, 224, 235)
        } else {
            Color32::from_rgb(red, green, blue)
        };
        let fill = e
            .paint
            .fill
            .map(|[r, g, b]| Color32::from_rgb(r, g, b))
            .unwrap_or(Color32::TRANSPARENT);
        let stroke = Stroke::new(e.paint.width * z, ink);
        let diamond = vec![r.center_top(), r.right_center(), r.center_bottom(), r.left_center()];
        match e.kind {
            Kind::Rectangle | Kind::Note => {
                p.rect_filled(r, 4.0, fill);
                if e.paint.sketch {
                    canvas::sketch_rect(p, r, stroke, e.id);
                } else {
                    p.rect_stroke(r, 4.0, stroke, StrokeKind::Inside);
                }
            }
            Kind::Ellipse => {
                p.add(egui::epaint::EllipseShape::filled(r.center(), r.size() / 2.0, fill));
                if e.paint.sketch {
                    canvas::sketch_ellipse(p, r.center(), r.size() / 2.0, stroke, e.id);
                } else {
                    p.add(egui::epaint::EllipseShape::stroke(r.center(), r.size() / 2.0, stroke));
                }
            }
            Kind::Diamond => {
                p.add(Shape::convex_polygon(diamond.clone(), fill, Stroke::NONE));
                if e.paint.sketch {
                    canvas::sketch_polyline(p, &diamond, stroke, e.id, true);
                } else {
                    p.add(Shape::closed_line(diamond, stroke));
                }
            }
            Kind::Line | Kind::Arrow | Kind::Freehand => {
                let points: Vec<_> = e
                    .world_points()
                    .into_iter()
                    .map(|q| self.state.camera.to_screen(self.state.canvas, q))
                    .collect();
                if e.kind == Kind::Freehand && points.len() == 2 && points[0].distance(points[1]) < z {
                    p.circle_filled(points[0], stroke.width / 2.0, ink);
                } else if e.paint.sketch && e.kind != Kind::Freehand {
                    canvas::sketch_polyline(p, &points, stroke, e.id, false);
                } else {
                    canvas::polyline(p, &points, stroke);
                }
                if e.kind == Kind::Arrow && points.len() >= 2 {
                    let tip = points[points.len() - 1];
                    let from = points[points.len() - 2];
                    let size = (tip.distance(from) / 3.0).min(16.0 * z);
                    if e.paint.sketch {
                        canvas::sketch_arrow_head(p, tip, tip - from, size, stroke, e.id);
                    } else {
                        canvas::arrow_head(p, tip, tip - from, size, ink, false);
                    }
                }
            }
            Kind::Text => {}
        }
        if !e.text.is_empty() {
            let text_ink = if e.paint.fill.is_some() && !e.kind.is_path() && e.kind != Kind::Text {
                Color32::from_rgb(red, green, blue)
            } else {
                ink
            };
            let font = if e.paint.sketch {
                crate::theme::hand(e.paint.font_size * z)
            } else {
                FontId::proportional(e.paint.font_size * z)
            };
            let mut job = egui::text::LayoutJob::simple(e.text.clone(), font, text_ink, (r.width() - 16.0 * z).max(10.0));
            job.halign = egui::Align::Center;
            let galley = p.layout_job(job);
            let pos = r.center() - galley.rect.center().to_vec2();
            p.with_clip_rect(r).galley(pos, galley, text_ink);
        }
    }
}
