use super::*;
use egui::{Key, PointerButton, Response};

impl WhiteboardEditor {
    fn snap(&self, p: Pos) -> Pos {
        if self.state.snap { p.snapped(20.0) } else { p }
    }
    pub(super) fn shortcuts(&mut self, ui: &Ui, response: &Response) {
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
            }
            if pressed(Key::D) {
                self.duplicate();
            }
            return;
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
                let start = if kind == Kind::Freehand { start } else { self.snap(start) };
                self.state.drag = Some(Drag::Draw {
                    kind,
                    start,
                    current: start,
                    points: vec![start],
                });
            } else {
                let handle = self
                    .state
                    .selection
                    .first()
                    .filter(|_| self.state.selection.len() == 1)
                    .and_then(|id| self.board.element(*id))
                    .filter(|e| {
                        self.state
                            .camera
                            .rect_to_screen(self.state.canvas, e.bounds)
                            .right_bottom()
                            .distance(screen)
                            < 12.0
                    });
                if let Some(e) = handle {
                    self.state.drag = Some(Drag::Resize {
                        id: e.id,
                        original: e.bounds,
                    });
                } else if let Some(id) = self.board.hit(start, 5.0 / self.state.camera.zoom) {
                    if !self.state.selection.contains(&id) {
                        if !ui.input(|i| i.modifiers.shift) {
                            self.state.selection.clear();
                        }
                        self.state.selection.insert(id);
                    }
                    let origins = self
                        .board
                        .elements
                        .iter()
                        .filter(|e| self.state.selection.contains(&e.id))
                        .map(|e| (e.id, Pos::new(e.bounds.x, e.bounds.y)))
                        .collect();
                    self.state.drag = Some(Drag::Move { start, origins });
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
        if response.dragged_by(PointerButton::Primary)
            && let Some(p) = world
        {
            let snapped = self.snap(p);
            match &mut self.state.drag {
                Some(Drag::Pan) => self.state.camera.offset += response.drag_delta(),
                Some(Drag::Draw { kind, current, points, .. }) => {
                    *current = if *kind == Kind::Freehand { p } else { snapped };
                    if *kind == Kind::Freehand
                        && points.len() < 20_000
                        && points.last().is_none_or(|last| last.dist(p) * self.state.camera.zoom >= 2.0)
                    {
                        points.push(p);
                    }
                }
                Some(Drag::Move { start, origins }) => {
                    let mut delta = p - *start;
                    if self.state.snap {
                        delta = delta.snapped(20.0);
                    }
                    for (id, origin) in origins {
                        if let Some(e) = self.board.elements.iter_mut().find(|e| e.id == *id) {
                            e.bounds.x = origin.x + delta.x;
                            e.bounds.y = origin.y + delta.y;
                        }
                    }
                }
                Some(Drag::Resize { id, original }) => {
                    if let Some(e) = self.board.elements.iter_mut().find(|e| e.id == *id) {
                        e.bounds.w = (snapped.x - original.x).clamp(1.0, 10_000.0);
                        e.bounds.h = (snapped.y - original.y).clamp(1.0, 10_000.0);
                    }
                }
                Some(Drag::Rubber { start, current, initial }) => {
                    *current = p;
                    let b = Element::new(Kind::Rectangle, *start, p, Paint::default()).bounds;
                    self.state.selection = initial.clone();
                    self.state.selection.extend(
                        self.board
                            .elements
                            .iter()
                            .filter(|e| {
                                b.contains(Pos::new(e.bounds.x, e.bounds.y)) && b.contains(Pos::new(e.bounds.right(), e.bounds.bottom()))
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
                    let mut e = Element::new(kind, start, current, self.state.paint.clone());
                    if kind == Kind::Text {
                        e.text = tr("Your text").into();
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
                let p = self.snap(p);
                let (w, h) = match kind {
                    Kind::Text => (220.0, 80.0),
                    Kind::Note => (220.0, 160.0),
                    Kind::Line | Kind::Arrow => (140.0, 1.0),
                    _ => (160.0, 100.0),
                };
                let mut e = Element::new(kind, p, p + Pos::new(w, h), self.state.paint.clone());
                if kind == Kind::Text {
                    e.text = tr("Your text").into();
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
                Some(Element::new(*kind, *start, *current, self.state.paint.clone()))
            }
        } else {
            None
        }
    }
}
