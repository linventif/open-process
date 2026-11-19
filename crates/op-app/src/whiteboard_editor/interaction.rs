use super::*;
use egui::{Key, PointerButton, Response};

impl WhiteboardEditor {
    pub(super) fn resize_at(&self, pointer: egui::Pos2) -> Option<(Id, canvas::resize::ResizeEdge)> {
        if self.state.tool != Tool::Select {
            return None;
        }
        canvas::resize::target(
            self.state.camera,
            self.state.canvas,
            pointer,
            self.board
                .elements
                .iter()
                .filter(|e| !e.kind.is_connector())
                .map(|e| (e.id, e.bounds)),
        )
    }

    pub(super) fn snap_point(&self, p: Pos) -> Pos {
        if self.state.snap { p.snapped(20.0) } else { p }
    }
    pub(super) fn shortcuts(&mut self, ui: &Ui, response: &Response) {
        if self.state.drag.is_some() {
            return;
        }
        self.clipboard_events(ui, response);
        if !response.hovered() || ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let command = ui.input(|i| i.modifiers.command);
        let pressed = |k| ui.input(|i| i.key_pressed(k));
        if pressed(Key::Delete) || pressed(Key::Backspace) {
            self.delete();
        }
        if command {
            if pressed(Key::A) {
                self.state.selection = self.board.elements.iter().map(|e| e.id).collect();
                self.state.tool = Tool::Select;
            }
            if pressed(Key::D) {
                self.duplicate();
            }
            let shift = ui.input(|i| i.modifiers.shift);
            if pressed(Key::OpenBracket) {
                self.reorder(if shift { StackOrder::ToBack } else { StackOrder::Backward });
            }
            if pressed(Key::CloseBracket) {
                self.reorder(if shift { StackOrder::ToFront } else { StackOrder::Forward });
            }
            return;
        }
        if (pressed(Key::F2) || pressed(Key::Enter)) && self.state.selection.len() == 1 {
            self.state.inspector = Some(true);
            self.state.focus_text = true;
        }
        if pressed(Key::F) {
            self.state.fit = true;
        }
        for (key, tool) in [
            (Key::V, Tool::Select),
            (Key::H, Tool::Hand),
            (Key::R, Tool::Draw(Kind::Rectangle)),
            (Key::E, Tool::Draw(Kind::Ellipse)),
            (Key::D, Tool::Draw(Kind::Diamond)),
            (Key::L, Tool::Draw(Kind::Line)),
            (Key::A, Tool::Draw(Kind::Arrow)),
            (Key::P, Tool::Draw(Kind::Freehand)),
            (Key::T, Tool::Draw(Kind::Text)),
            (Key::N, Tool::Draw(Kind::Note)),
        ] {
            if pressed(key) {
                self.state.tool = tool;
            }
        }
        let step = if ui.input(|i| i.modifiers.shift) { 10.0 } else { 1.0 };
        let delta = Pos::new(
            (if pressed(Key::ArrowRight) { step } else { 0.0 }) - (if pressed(Key::ArrowLeft) { step } else { 0.0 }),
            (if pressed(Key::ArrowDown) { step } else { 0.0 }) - (if pressed(Key::ArrowUp) { step } else { 0.0 }),
        );
        if delta != Pos::default() {
            self.history.sync(&self.board, false);
            self.board.prepare_move(&self.state.selection);
            for e in &mut self.board.elements {
                if self.state.selection.contains(&e.id) {
                    e.bounds.x += delta.x;
                    e.bounds.y += delta.y;
                }
            }
            self.history.sync(&self.board, false);
        }
    }
    pub(super) fn interact(&mut self, ui: &Ui, response: &Response, world: Option<Pos>) {
        if response.drag_started_by(PointerButton::Primary)
            && let Some(screen) = ui.input(|i| i.pointer.press_origin())
        {
            let start = self.state.camera.to_world(self.state.canvas, screen);
            self.history.sync(&self.board, false);
            self.history.begin(&self.board);
            if self.state.tool == Tool::Hand || ui.input(|i| i.key_down(Key::Space)) {
                self.state.drag = Some(Drag::Pan);
            } else if let Tool::Draw(kind) = self.state.tool {
                self.state.drag = Some(Drag::Draw {
                    kind,
                    start,
                    current: start,
                    points: vec![start],
                });
            } else {
                let alt_hit = if ui.input(|i| i.modifiers.alt) {
                    self.board
                        .hit(start, 5.0 / self.state.camera.zoom)
                        .or_else(|| self.resize_at(screen).map(|(id, _)| id))
                } else {
                    None
                };
                if let Some(id) = alt_hit {
                    if !self.state.selection.contains(&id) {
                        self.state.selection = BTreeSet::from([id]);
                    }
                    if !self.begin_move(start, true) {
                        return;
                    }
                } else {
                    let endpoint = self
                        .state
                        .selection
                        .first()
                        .filter(|_| self.state.selection.len() == 1)
                        .and_then(|id| self.board.element(*id))
                        .filter(|e| e.kind.is_connector())
                        .and_then(|e| {
                            self.board
                                .geometry()
                                .points(e)
                                .iter()
                                .enumerate()
                                .find(|(_, point)| self.state.camera.to_screen(self.state.canvas, **point).distance(screen) < 12.0)
                                .map(|(index, _)| (e.id, index == 0))
                        });
                    let handle = self.resize_at(screen);
                    if let Some((id, start)) = endpoint {
                        self.state.drag = Some(Drag::Endpoint { id, start });
                    } else if let Some((id, edge)) = handle {
                        self.state.selection.clear();
                        self.state.selection.insert(id);
                        self.state.drag = Some(Drag::Resize {
                            id,
                            gesture: canvas::resize::ResizeGesture::new(self.board.element(id).unwrap().bounds, edge, start),
                        });
                    } else if let Some(id) = self.board.hit(start, 5.0 / self.state.camera.zoom) {
                        if !self.state.selection.contains(&id) {
                            if !ui.input(|i| i.modifiers.shift) {
                                self.state.selection.clear();
                            }
                            self.state.selection.insert(id);
                        }
                        self.begin_move(start, false);
                    } else {
                        let initial = if ui.input(|i| i.modifiers.shift) {
                            self.state.selection.clone()
                        } else {
                            BTreeSet::new()
                        };
                        self.state.drag = Some(Drag::Rubber {
                            start,
                            current: start,
                            initial,
                        });
                    }
                }
            }
        }
        if response.dragged_by(PointerButton::Primary)
            && let Some(p) = world
        {
            if ui.input(|i| i.modifiers.alt)
                && let Some(Drag::Move {
                    start,
                    original_selection: None,
                    ..
                }) = self.state.drag
            {
                self.history.cancel(&mut self.board);
                self.history.begin(&self.board);
                if !self.begin_move(start, true) {
                    self.state.drag = None;
                    return;
                }
            }
            let snapped = self.snap_point(p);
            let shift = ui.input(|i| i.modifiers.shift);
            let endpoint = if let Some(Drag::Endpoint { id, start }) = self.state.drag {
                self.board.element(id).map(|e| {
                    let geometry = self.board.geometry();
                    let points = geometry.points(e);
                    let attachment = geometry.attachment_at(p, points[usize::from(start)], 12.0 / self.state.camera.zoom);
                    (id, start, attachment)
                })
            } else {
                None
            };
            match &mut self.state.drag {
                Some(Drag::Pan) => self.state.camera.offset += response.drag_delta(),
                Some(Drag::Draw {
                    kind,
                    start,
                    current,
                    points,
                }) => {
                    let origin = if self.state.snap { start.snapped(20.0) } else { *start };
                    *current = if shift && *kind != Kind::Freehand {
                        kind.constrained_endpoint(origin, snapped)
                    } else {
                        p
                    };
                    if *kind == Kind::Freehand
                        && points.len() < 20_000
                        && points.last().is_none_or(|last| last.dist(p) * self.state.camera.zoom >= 2.0)
                    {
                        points.push(p);
                    }
                }
                Some(Drag::Move { start, origins, .. }) => {
                    let mut delta = p - *start;
                    if shift {
                        delta = delta.axis_locked();
                    }
                    if self.state.snap {
                        delta = delta.snapped(20.0);
                    }
                    for e in &mut self.board.elements {
                        if let Some(origin) = origins.get(&e.id) {
                            e.bounds.x = origin.x + delta.x;
                            e.bounds.y = origin.y + delta.y;
                        }
                    }
                }
                Some(Drag::Resize { id, gesture }) => {
                    if let Some(e) = self.board.elements.iter_mut().find(|e| e.id == *id) {
                        e.bounds = gesture.apply(
                            e.bounds,
                            p,
                            Pos::new(1.0, 1.0),
                            Pos::new(10_000.0, 10_000.0),
                            self.state.snap.then_some(20.0),
                        );
                    }
                }
                Some(Drag::Endpoint { .. }) => {
                    if let Some((id, start, attachment)) = endpoint {
                        self.board.set_endpoint(id, start, snapped, attachment);
                    }
                }
                Some(Drag::Rubber { start, current, initial }) => {
                    *current = p;
                    let b = Element::new(Kind::Rectangle, *start, p, Paint::default()).bounds;
                    self.state.selection = initial.clone();
                    let geometry = self.board.geometry();
                    self.state.selection.extend(
                        self.board
                            .elements
                            .iter()
                            .filter(|e| {
                                let bounds = geometry.element_bounds(e);
                                b.contains(Pos::new(bounds.x, bounds.y)) && b.contains(Pos::new(bounds.right(), bounds.bottom()))
                            })
                            .map(|e| e.id),
                    );
                }
                None => {}
            }
        }
        if ui.input(|i| i.pointer.primary_released())
            && let Some(drag) = self.state.drag.take()
        {
            if let Drag::Draw {
                kind,
                start,
                current,
                mut points,
            } = drag
            {
                if kind == Kind::Freehand {
                    if points.last() != Some(&current) {
                        if points.len() < 20_000 {
                            points.push(current);
                        } else if let Some(last) = points.last_mut() {
                            *last = current;
                        }
                    }
                    if let Some(e) = Element::freehand(&points, self.state.paint.clone()) {
                        self.add(e);
                    }
                } else {
                    let mut e = self.drawing_element(kind, start, current);
                    if kind == Kind::Text {
                        e.text = tr("drawing.text_placeholder").into();
                    }
                    self.add(e);
                }
            }
            self.history.sync(&self.board, false);
        }
        if response.clicked_by(PointerButton::Primary)
            && let Some(p) = world
        {
            if ui.input(|i| i.key_down(Key::Space)) || self.state.tool == Tool::Hand {
                return;
            }
            if let Tool::Draw(kind) = self.state.tool {
                if kind == Kind::Freehand {
                    let e = Element::freehand(&[p, p + Pos::new(0.5, 0.0)], self.state.paint.clone()).unwrap();
                    self.history.sync(&self.board, false);
                    self.add(e);
                    self.history.sync(&self.board, false);
                    return;
                }
                let (w, h) = match kind {
                    Kind::Text => (220.0, 80.0),
                    Kind::Note => (220.0, 160.0),
                    Kind::Line | Kind::Arrow => (140.0, 1.0),
                    _ => (160.0, 100.0),
                };
                let mut e = self.drawing_element(kind, p, p + Pos::new(w, h));
                if kind == Kind::Text {
                    e.text = tr("drawing.text_placeholder").into();
                }
                self.history.sync(&self.board, false);
                self.add(e);
                self.history.sync(&self.board, false);
            } else {
                let hit = self.board.hit(p, 5.0 / self.state.camera.zoom);
                if ui.input(|i| i.modifiers.shift) {
                    if let Some(id) = hit
                        && !self.state.selection.remove(&id)
                    {
                        self.state.selection.insert(id);
                    }
                } else {
                    self.state.selection = hit.into_iter().collect();
                }
                if response.double_clicked() && hit.is_some() {
                    self.state.inspector = Some(true);
                    self.state.focus_text = true;
                }
            }
        }
        if response.secondary_clicked()
            && let Some(p) = world
        {
            if let Some(id) = self.board.hit(p, 5.0 / self.state.camera.zoom) {
                if !self.state.selection.contains(&id) {
                    self.state.selection = BTreeSet::from([id]);
                }
            } else {
                self.state.selection.clear();
            }
            self.state.context_at = p;
            if let Some(id) = ui.memory(|m| m.focused()) {
                ui.memory_mut(|m| m.surrender_focus(id));
            }
        }
        response.context_menu(|ui| self.context_menu(ui, self.state.context_at));
    }
    pub(super) fn preview(&self) -> Option<Element> {
        if let Some(Drag::Draw {
            kind,
            start,
            current,
            points,
        }) = &self.state.drag
        {
            if *kind == Kind::Freehand {
                Element::freehand(points, self.state.paint.clone())
            } else {
                Some(self.drawing_element(*kind, *start, *current))
            }
        } else {
            None
        }
    }
}
