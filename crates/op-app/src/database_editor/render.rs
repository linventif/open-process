use super::*;
use crate::canvas::Palette;
use egui::{Align2, FontId, Key, PointerButton, Sense, Stroke, StrokeKind, pos2, vec2};
use op_core::database::{ForeignKey, Table};

impl DatabaseEditor {
    fn hit_table(&self, p: Pos) -> Option<Id> {
        self.schema.tables.iter().rev().find(|t| t.rect().contains(p)).map(|t| t.id)
    }
    pub(super) fn canvas(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size().max(vec2(1.0, 1.0)), Sense::click_and_drag());
        self.state.canvas = rect;
        self.state.camera.handle_navigation(ui, &response, rect);
        if self.state.fit {
            if !self.schema.tables.is_empty() {
                let min = Pos::new(
                    self.schema.tables.iter().map(|t| t.rect().x).fold(f32::INFINITY, f32::min),
                    self.schema.tables.iter().map(|t| t.rect().y).fold(f32::INFINITY, f32::min),
                );
                let max = Pos::new(
                    self.schema
                        .tables
                        .iter()
                        .map(|t| t.rect().right())
                        .fold(f32::NEG_INFINITY, f32::max),
                    self.schema
                        .tables
                        .iter()
                        .map(|t| t.rect().bottom())
                        .fold(f32::NEG_INFINITY, f32::max),
                );
                self.state.camera.fit(rect, min, max);
            }
            self.state.fit = false;
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let world = pointer.map(|p| self.state.camera.to_world(rect, p));
        if response.hovered() && !ui.ctx().egui_wants_keyboard_input() {
            if ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
                self.delete_selected();
            }
            if ui.input(|i| i.key_pressed(Key::F)) {
                self.state.fit = true;
            }
        }
        if response.drag_started_by(PointerButton::Primary)
            && let Some(p) = ui.input(|i| i.pointer.press_origin())
        {
            let start = self.state.camera.to_world(rect, p);
            if let Some(id) = self.hit_table(start) {
                self.history.sync(&self.schema, false);
                self.state.selected = Some(id);
                if self.state.connecting {
                    self.state.drag = Some(Drag::Connect(id));
                    self.state.pending = Some(id);
                } else {
                    let t = self.schema.table(id).unwrap();
                    let r = self.state.camera.rect_to_screen(rect, t.rect());
                    self.state.drag = if r.right_bottom().distance(p) < 12.0 {
                        Some(Drag::Resize(id))
                    } else {
                        Some(Drag::Move {
                            table: id,
                            start,
                            origin: Pos::new(t.bounds.x, t.bounds.y),
                        })
                    };
                }
            }
        }
        if response.dragged_by(PointerButton::Primary)
            && let Some(p) = world
        {
            match &self.state.drag {
                Some(Drag::Move { table, start, origin }) => {
                    let mut delta = p - *start;
                    if self.state.snap {
                        delta = delta.snapped(10.0);
                    }
                    if let Some(t) = self.schema.tables.iter_mut().find(|t| t.id == *table) {
                        t.bounds.x = origin.x + delta.x;
                        t.bounds.y = origin.y + delta.y;
                    }
                }
                Some(Drag::Resize(id)) => {
                    if let Some(t) = self.schema.tables.iter_mut().find(|t| t.id == *id) {
                        let p = if self.state.snap { p.snapped(10.0) } else { p };
                        t.bounds.w = (p.x - t.bounds.x).clamp(190.0, 1600.0);
                        t.bounds.h = (p.y - t.bounds.y).clamp(65.0, 3000.0);
                    }
                }
                _ => {}
            }
        }
        if ui.input(|i| i.pointer.primary_released()) {
            if let Some(Drag::Connect(from)) = self.state.drag.take() {
                if let Some(to) = world.and_then(|p| self.hit_table(p)) {
                    self.link(from, to);
                } else {
                    self.state.pending = None;
                }
            } else {
                self.state.drag = None;
            }
        }
        if response.clicked_by(PointerButton::Primary)
            && let Some(p) = world
        {
            let hit = self.hit_table(p);
            if self.state.connecting {
                if let Some(to) = hit {
                    if let Some(from) = self.state.pending {
                        self.link(from, to);
                    } else {
                        self.state.pending = Some(to);
                        self.state.selected = Some(to);
                    }
                } else {
                    self.state.pending = None;
                }
            } else {
                self.state.selected = hit;
            }
        }
        if response.double_clicked()
            && !self.state.connecting
            && let Some(p) = world
            && self.hit_table(p).is_none()
        {
            self.add_table(if self.state.snap { p.snapped(10.0) } else { p });
        }
        let exporting = std::mem::take(&mut self.state.export_png);
        let pal = Palette::for_ui(ui);
        let painter = ui.painter().with_clip_rect(rect);
        canvas::draw_background(&painter, rect, &self.state.camera, &pal);
        for f in &self.schema.foreign_keys {
            self.draw_relationship(&painter, rect, f, &pal);
        }
        for t in &self.schema.tables {
            self.draw_table(&painter, rect, t, &pal, exporting);
        }
        if exporting {
            crate::io::request_png_export(ui.ctx(), rect, self.schema.name.clone());
        } else {
            if self.schema.tables.is_empty() {
                painter.text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    tr("Add a table to start designing your database."),
                    FontId::proportional(16.0),
                    pal.muted,
                );
            }
            if let Some(from) = self.state.pending.and_then(|id| self.schema.table(id))
                && let Some(p) = pointer
            {
                painter.line_segment(
                    [self.state.camera.to_screen(rect, from.rect().center()), p],
                    Stroke::new(1.5, pal.select),
                );
            }
            canvas::floating(
                ui.ctx(),
                egui::Id::unique(("database_palette", self.state.uid)),
                rect.left_top() + vec2(14.0, 14.0),
                Align2::LEFT_TOP,
                |ui| {
                    if canvas::icon_button(ui, !self.state.connecting, tr("Select"), crate::icons::select).clicked() {
                        self.state.connecting = false;
                        self.state.pending = None;
                    }
                    if canvas::icon_button(ui, self.state.connecting, tr("Connect"), |p, r, c| {
                        p.line_segment([r.left_bottom(), r.right_top()], Stroke::new(1.5, c));
                        canvas::arrow_head(p, r.right_top(), vec2(1.0, -1.0), 7.0, c, true);
                    })
                    .clicked()
                    {
                        self.state.connecting = true;
                        self.state.pending = None;
                    }
                },
            );
        }
    }
    fn draw_table(&self, p: &egui::Painter, canvas: Rect, t: &Table, pal: &Palette, exporting: bool) {
        let r = self.state.camera.rect_to_screen(canvas, t.rect());
        let z = self.state.camera.zoom;
        let selected = !exporting && self.state.selected == Some(t.id);
        canvas::shadow_rect(p, r, 8.0, pal);
        p.rect_filled(r, 8.0, pal.fill);
        let header = Rect::from_min_size(r.min, vec2(r.width(), 36.0 * z));
        p.rect_filled(
            header,
            egui::CornerRadius {
                nw: 8,
                ne: 8,
                sw: 0,
                se: 0,
            },
            pal.task_fill,
        );
        p.rect_stroke(
            r,
            8.0,
            Stroke::new(if selected { 2.0 } else { 1.2 }, if selected { pal.select } else { pal.ink }),
            StrokeKind::Inside,
        );
        let text_p = p.with_clip_rect(r.shrink(4.0));
        text_p.text(
            header.center(),
            Align2::CENTER_CENTER,
            &t.name,
            crate::theme::semibold(14.0 * z),
            pal.text,
        );
        p.hline(r.x_range(), header.bottom(), Stroke::new(1.0, pal.border));
        for (i, c) in t.columns.iter().enumerate() {
            let y = r.top() + (49.0 + i as f32 * 25.0) * z;
            let fk = self
                .schema
                .foreign_keys
                .iter()
                .any(|f| f.table == t.id && f.columns.contains(&c.id));
            let marker = if c.primary_key {
                "PK"
            } else if fk {
                "FK"
            } else if c.unique {
                "UQ"
            } else {
                ""
            };
            text_p.text(
                pos2(r.left() + 9.0 * z, y),
                Align2::LEFT_CENTER,
                marker,
                crate::theme::semibold(9.0 * z),
                if c.primary_key { pal.warn } else { pal.muted },
            );
            let name_rect = Rect::from_min_max(
                pos2(r.left() + 36.0 * z, y - 12.0 * z),
                pos2(r.left() + r.width() * 0.62, y + 12.0 * z),
            );
            text_p.with_clip_rect(name_rect.intersect(text_p.clip_rect())).text(
                pos2(name_rect.left(), y),
                Align2::LEFT_CENTER,
                &c.name,
                FontId::proportional(12.5 * z),
                pal.text,
            );
            text_p.text(
                pos2(r.right() - 9.0 * z, y),
                Align2::RIGHT_CENTER,
                c.data_type.ddl(self.schema.dialect),
                FontId::monospace(10.0 * z),
                pal.muted,
            );
        }
        if selected {
            p.rect_filled(Rect::from_center_size(r.right_bottom(), vec2(8.0, 8.0)), 1.0, pal.select);
        }
    }
    fn draw_relationship(&self, p: &egui::Painter, canvas: Rect, f: &ForeignKey, pal: &Palette) {
        let Some(points) = relationship_points(&self.schema, f) else {
            return;
        };
        let pts: Vec<_> = points.iter().map(|p| self.state.camera.to_screen(canvas, *p)).collect();
        canvas::polyline(p, &pts, Stroke::new(1.5, pal.arc));
        let child = self.schema.table(f.table).unwrap();
        let optional = f.columns.iter().any(|id| child.column(*id).is_some_and(|c| c.nullable));
        let child_cardinality = if child.is_candidate_key(&f.columns) { "0..1" } else { "0..*" };
        for (pos, toward, text) in [
            (pts[0], pts[1], child_cardinality),
            (*pts.last().unwrap(), pts[pts.len() - 2], if optional { "0..1" } else { "1" }),
        ] {
            let x = (toward.x - pos.x).signum();
            p.text(
                pos + vec2(x * 8.0, -9.0),
                if x < 0.0 { Align2::RIGHT_BOTTOM } else { Align2::LEFT_BOTTOM },
                text,
                FontId::proportional(11.0 * self.state.camera.zoom),
                pal.arc,
            );
        }
        canvas::badge(
            p,
            canvas::polyline_midpoint(&pts),
            &f.name,
            10.0 * self.state.camera.zoom,
            pal.text,
            pal,
        );
    }
}
fn relationship_points(s: &Schema, f: &ForeignKey) -> Option<Vec<Pos>> {
    let a = s.table(f.table)?;
    let b = s.table(f.referenced_table)?;
    let ar = a.rect();
    let br = b.rect();
    let row = |t: &Table, id: Option<&Id>| {
        t.columns
            .iter()
            .position(|c| Some(&c.id) == id)
            .map(|i| t.bounds.y + 49.0 + i as f32 * 25.0)
            .unwrap_or(t.bounds.y + 18.0)
    };
    let ay = row(a, f.columns.first());
    let by = row(b, f.referenced_columns.first());
    let (ax, bx, mid) = if ar.right() + 20.0 < br.x {
        (ar.right(), br.x, (ar.right() + br.x) / 2.0)
    } else if br.right() + 20.0 < ar.x {
        (ar.x, br.right(), (ar.x + br.right()) / 2.0)
    } else {
        (ar.x, br.x, ar.x.min(br.x) - 45.0)
    };
    Some(vec![Pos::new(ax, ay), Pos::new(mid, ay), Pos::new(mid, by), Pos::new(bx, by)])
}
