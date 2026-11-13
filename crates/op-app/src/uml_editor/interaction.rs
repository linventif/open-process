//! Canvas gestures and selection commands. Model mutations form complete undo steps.

use super::*;
use canvas::InlineOutcome;

const CLIPBOARD: &str = "open_process_uml_clipboard";

impl UmlEditor {
    pub(super) fn select_node(&mut self, id: Id, additive: bool) {
        if !additive {
            self.group.clear();
        }
        if additive && self.group.contains(&id) {
            self.group.remove(&id);
        } else {
            self.group.insert(id);
        }
        self.selection = self.group.iter().next_back().copied().map(Selection::Node);
    }

    pub(super) fn paste(&mut self, fragment: &DiagramFragment) {
        self.history.sync(&self.diagram, false);
        self.paste_count += 1;
        let offset = 30.0 * self.paste_count as f32;
        let ids = self.diagram.paste(fragment, Pos::new(offset, offset));
        if !ids.is_empty() {
            self.group = ids;
            self.selection = self.group.iter().next().copied().map(Selection::Node);
            self.tool = Tool::Select;
        }
        self.history.sync(&self.diagram, false);
    }

    pub(super) fn align(&mut self, horizontal: bool, edge: i32) {
        let roots = self.selection_roots();
        let rects: Vec<_> = self
            .diagram
            .nodes
            .iter()
            .filter(|n| roots.contains(&n.id))
            .map(|n| (n.id, n.rect()))
            .collect();
        if rects.len() < 2 {
            return;
        }
        let min = rects
            .iter()
            .map(|(_, r)| if horizontal { r.x } else { r.y })
            .fold(f32::INFINITY, f32::min);
        let max = rects
            .iter()
            .map(|(_, r)| if horizontal { r.right() } else { r.bottom() })
            .fold(f32::NEG_INFINITY, f32::max);
        self.history.sync(&self.diagram, false);
        for (id, r) in rects {
            let size = if horizontal { r.w } else { r.h };
            let target = match edge {
                -1 => min,
                1 => max - size,
                _ => (min + max - size) / 2.0,
            };
            let pos = if horizontal { Pos::new(target, r.y) } else { Pos::new(r.x, target) };
            self.diagram.move_node(id, pos);
        }
        self.history.sync(&self.diagram, false);
    }

    fn selection_roots(&self) -> BTreeSet<Id> {
        self.group
            .iter()
            .copied()
            .filter(|id| {
                self.diagram
                    .node(*id)
                    .is_some_and(|n| self.diagram.system_of(n).is_none_or(|system| !self.group.contains(&system)))
            })
            .collect()
    }

    pub(super) fn distribute(&mut self, horizontal: bool) {
        let roots = self.selection_roots();
        let mut rects: Vec<_> = self
            .diagram
            .nodes
            .iter()
            .filter(|n| roots.contains(&n.id))
            .map(|n| (n.id, n.rect()))
            .collect();
        if rects.len() < 3 {
            return;
        }
        rects.sort_by(|a, b| {
            let a = if horizontal { a.1.x } else { a.1.y };
            let b = if horizontal { b.1.x } else { b.1.y };
            a.total_cmp(&b)
        });
        let first = rects[0].1;
        let last = rects.last().unwrap().1;
        let min = if horizontal { first.x } else { first.y };
        let max = if horizontal { last.right() } else { last.bottom() };
        let sizes: f32 = rects.iter().map(|(_, r)| if horizontal { r.w } else { r.h }).sum();
        let gap = (max - min - sizes) / (rects.len() - 1) as f32;
        self.history.sync(&self.diagram, false);
        let mut cursor = min;
        for (id, r) in rects {
            let pos = if horizontal { Pos::new(cursor, r.y) } else { Pos::new(r.x, cursor) };
            self.diagram.move_node(id, pos);
            cursor += gap + if horizontal { r.w } else { r.h };
        }
        self.history.sync(&self.diagram, false);
    }

    fn hit(&self, world: Pos, screen: egui::Pos2) -> Option<Selection> {
        self.hit_node(world)
            .map(Selection::Node)
            .or_else(|| {
                self.label_hits
                    .iter()
                    .rev()
                    .find(|(_, rect)| rect.expand(3.0).contains(screen))
                    .map(|(id, _)| Selection::Relation(*id))
            })
            .or_else(|| {
                self.diagram
                    .relations
                    .iter()
                    .rev()
                    .find(|r| dist_to_polyline(world, &self.relation_points(r)) < 7.0 / self.view.zoom)
                    .map(|r| Selection::Relation(r.id))
            })
    }

    fn begin_rename(&mut self) {
        if self.group.len() > 1 {
            return;
        }
        if let Some(target) = self.selection {
            let name = match target {
                Selection::Node(id) => self.diagram.node(id).map(|n| n.name.clone()),
                Selection::Relation(id) => self.diagram.relations.iter().find(|r| r.id == id).map(|r| r.label.clone()),
            };
            if let Some(text) = name {
                self.history.sync(&self.diagram, false);
                self.editing = Some((target, text, true));
                self.pending = None;
            }
        }
    }

    fn connect(&mut self, from: Id, to: Id) {
        self.history.sync(&self.diagram, false);
        if let Some(id) = self.diagram.connect(from, to, self.relation_kind) {
            self.group.clear();
            self.selection = Some(Selection::Relation(id));
            self.pending = None;
            self.status.clear();
            self.history.sync(&self.diagram, false);
        } else {
            self.status = tr("This relation is not valid for these elements.").into();
        }
    }

    fn create_at(&mut self, kind: NodeKind, world: Pos) {
        self.history.sync(&self.diagram, false);
        let Some(id) = self.diagram.add_node(kind, world, tr(kind.label())) else {
            return;
        };
        let node = self.diagram.nodes.iter_mut().find(|n| n.id == id).unwrap();
        let p = Pos::new(world.x - node.bounds.w / 2.0, world.y - node.bounds.h / 2.0);
        let p = if self.snap { p.snapped(10.0) } else { p };
        node.bounds.x = p.x;
        node.bounds.y = p.y;
        self.select_node(id, false);
        self.tool = Tool::Select;
        self.status.clear();
        self.history.sync(&self.diagram, false);
    }

    fn shortcuts(&mut self, ui: &Ui, response: &egui::Response) {
        if self.editing.is_some() || !response.hovered() || ui.ctx().egui_wants_keyboard_input() {
            return;
        }
        let command = ui.input(|i| i.modifiers.command);
        if command {
            if ui.input(|i| i.key_pressed(Key::A)) {
                self.group = self.diagram.nodes.iter().map(|n| n.id).collect();
                self.selection = self.group.iter().next().copied().map(Selection::Node);
            }
            if ui.input(|i| i.key_pressed(Key::C)) && !self.group.is_empty() {
                let fragment = self.diagram.fragment(&self.group);
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique(CLIPBOARD), fragment));
                self.paste_count = 0;
            }
            if ui.input(|i| i.key_pressed(Key::V)) {
                let fragment = ui.ctx().data(|d| d.get_temp::<DiagramFragment>(egui::Id::unique(CLIPBOARD)));
                if let Some(fragment) = fragment {
                    self.paste(&fragment);
                }
            }
            if ui.input(|i| i.key_pressed(Key::D)) {
                let fragment = self.diagram.fragment(&self.group);
                self.paste_count = 0;
                self.paste(&fragment);
            }
            return;
        }
        if ui.input(|i| i.key_pressed(Key::Escape)) {
            self.history.cancel(&mut self.diagram);
            self.drag = None;
            self.editing = None;
            self.tool = Tool::Select;
            self.clear_interaction();
            self.status.clear();
        }
        if ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
            self.delete_selection();
        }
        if ui.input(|i| i.key_pressed(Key::F)) {
            self.request_fit();
        }
        if ui.input(|i| i.key_pressed(Key::F2)) {
            self.begin_rename();
        }
        if ui.input(|i| i.key_pressed(Key::V)) {
            self.tool = Tool::Select;
            self.pending = None;
        }
        if ui.input(|i| i.key_pressed(Key::C)) {
            self.tool = Tool::Connect;
        }
        let step = if ui.input(|i| i.modifiers.shift) { 10.0 } else { 1.0 };
        let mut delta = Pos::default();
        for (key, dx, dy) in [
            (Key::ArrowLeft, -step, 0.0),
            (Key::ArrowRight, step, 0.0),
            (Key::ArrowUp, 0.0, -step),
            (Key::ArrowDown, 0.0, step),
        ] {
            if ui.input(|i| i.key_pressed(key)) {
                delta = delta + Pos::new(dx, dy);
            }
        }
        if delta != Pos::default() && !self.group.is_empty() {
            self.history.sync(&self.diagram, false);
            let ids = self.diagram.selection_closure(&self.group);
            for n in &mut self.diagram.nodes {
                if ids.contains(&n.id) {
                    n.bounds.x += delta.x;
                    n.bounds.y += delta.y;
                }
            }
            self.history.sync(&self.diagram, false);
        }
    }

    pub(super) fn canvas(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size().max(vec2(1.0, 1.0)), Sense::click_and_drag());
        self.last_canvas = rect;
        self.update_routes();
        self.view.handle_navigation(ui, &response, rect);
        self.shortcuts(ui, &response);
        if self.fit {
            self.fit = false;
            let mut bounds: Vec<_> = self
                .diagram
                .nodes
                .iter()
                .flat_map(|n| {
                    let r = n.rect();
                    [Pos::new(r.x, r.y), Pos::new(r.right(), r.bottom())]
                })
                .collect();
            bounds.extend(self.routes.values().flatten().copied());
            if !bounds.is_empty() {
                let min = Pos::new(
                    bounds.iter().map(|p| p.x).fold(f32::INFINITY, f32::min),
                    bounds.iter().map(|p| p.y).fold(f32::INFINITY, f32::min),
                );
                let max = Pos::new(
                    bounds.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max),
                    bounds.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max),
                );
                self.view.fit(rect, min, max);
            } else {
                self.view = View::default();
            }
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let world = pointer.map(|p| self.view.to_world(rect, p));
        let shift = ui.input(|i| i.modifiers.shift);
        if response.drag_started_by(PointerButton::Primary)
            && self.editing.is_none()
            && let Some(screen) = ui.input(|i| i.pointer.press_origin())
        {
            let start = self.view.to_world(rect, screen);
            self.history.sync(&self.diagram, false);
            if ui.input(|i| i.key_down(Key::Space)) {
                self.drag = Some(Drag::Pan);
            } else {
                match self.tool {
                    Tool::Connect => {
                        if let Some(id) = self
                            .hit_node(start)
                            .filter(|id| self.diagram.node(*id).unwrap().kind != NodeKind::System)
                        {
                            self.drag = Some(Drag::Connect(id));
                            self.pending = Some(id);
                        }
                    }
                    Tool::Select => {
                        let resize = if let Some(Selection::Node(id)) = self.selection {
                            self.diagram
                                .node(id)
                                .filter(|n| {
                                    let r = self.view.rect_to_screen(rect, n.rect());
                                    self.group.len() == 1 && r.right_bottom().distance(screen) < 12.0
                                })
                                .map(|n| n.id)
                        } else {
                            None
                        };
                        if let Some(id) = resize {
                            self.drag = Some(Drag::Resize(id));
                        } else if let Some(Selection::Node(id)) = self.hit(start, screen) {
                            if !self.group.contains(&id) {
                                self.select_node(id, shift);
                            }
                            let ids = self.diagram.selection_closure(&self.group);
                            let origins = self
                                .diagram
                                .nodes
                                .iter()
                                .filter(|n| ids.contains(&n.id))
                                .map(|n| (n.id, Pos::new(n.bounds.x, n.bounds.y)))
                                .collect();
                            self.drag = Some(Drag::Move { start, origins });
                        } else {
                            let base = if shift { self.group.clone() } else { BTreeSet::new() };
                            self.drag = Some(Drag::Rubber { start, base });
                        }
                    }
                    Tool::Create(_) => {}
                }
            }
        }
        if response.dragged_by(PointerButton::Primary)
            && let Some(world) = world
        {
            match self.drag.as_ref() {
                Some(Drag::Pan) => self.view.offset += response.drag_delta(),
                Some(Drag::Move { start, origins }) => {
                    let mut delta = world - *start;
                    if self.snap {
                        delta = delta.snapped(10.0);
                    }
                    for (id, origin) in origins {
                        if let Some(n) = self.diagram.nodes.iter_mut().find(|n| n.id == *id) {
                            n.bounds.x = origin.x + delta.x;
                            n.bounds.y = origin.y + delta.y;
                        }
                    }
                }
                Some(Drag::Resize(id)) => {
                    if let Some(n) = self.diagram.nodes.iter_mut().find(|n| n.id == *id) {
                        let p = if self.snap { world.snapped(10.0) } else { world };
                        n.bounds.w = (p.x - n.bounds.x).clamp(80.0, 2000.0);
                        n.bounds.h = (p.y - n.bounds.y).clamp(if n.kind == NodeKind::Actor { 130.0 } else { 60.0 }, 2000.0);
                    }
                }
                Some(Drag::Rubber { start, base }) => {
                    let area = Rect::from_two_pos(self.view.to_screen(rect, *start), self.view.to_screen(rect, world));
                    self.group = base.clone();
                    self.group.extend(
                        self.diagram
                            .nodes
                            .iter()
                            .filter(|n| area.contains_rect(self.view.rect_to_screen(rect, n.rect())))
                            .map(|n| n.id),
                    );
                    self.selection = self.group.iter().next().copied().map(Selection::Node);
                }
                _ => {}
            }
        }
        if ui.input(|i| i.pointer.primary_released()) {
            if let Some(Drag::Connect(from)) = self.drag.take() {
                if let Some(to) = world.and_then(|p| self.hit_node(p)) {
                    self.connect(from, to);
                } else {
                    self.pending = None;
                }
            } else {
                self.drag = None;
            }
            if let Some(kind) = self.dnd.take()
                && let Some(p) = pointer.filter(|p| rect.contains(*p))
            {
                self.create_at(kind, self.view.to_world(rect, p));
            }
        }
        if response.clicked_by(PointerButton::Primary)
            && !ui.input(|i| i.key_down(Key::Space))
            && self.editing.is_none()
            && let (Some(world), Some(screen)) = (world, pointer)
        {
            self.status.clear();
            match self.tool {
                Tool::Create(kind) => self.create_at(kind, world),
                Tool::Select => match self.hit(world, screen) {
                    Some(Selection::Node(id)) => self.select_node(id, shift),
                    Some(Selection::Relation(id)) => {
                        self.group.clear();
                        self.selection = Some(Selection::Relation(id));
                    }
                    None if !shift => {
                        self.group.clear();
                        self.selection = None;
                    }
                    None => {}
                },
                Tool::Connect => {
                    if let Some(id) = self.hit_node(world) {
                        if let Some(from) = self.pending {
                            self.connect(from, id);
                        } else if self.diagram.node(id).unwrap().kind != NodeKind::System {
                            self.pending = Some(id);
                            self.select_node(id, false);
                        }
                    } else {
                        self.pending = None;
                    }
                }
            }
            if response.double_clicked() && self.tool == Tool::Select {
                self.begin_rename();
            }
        }
        // A right-click targets the element under the pointer, rather than a stale selection.
        if response.secondary_clicked()
            && let (Some(world), Some(screen)) = (world, pointer)
        {
            match self.hit(world, screen) {
                Some(Selection::Node(id)) if !self.group.contains(&id) => self.select_node(id, false),
                Some(target @ Selection::Relation(_)) => {
                    self.group.clear();
                    self.selection = Some(target);
                }
                _ => {}
            }
        }
        response.context_menu(|ui| {
            if ui
                .add_enabled(self.selection.is_some() && self.group.len() <= 1, egui::Button::new(tr("Rename")))
                .clicked()
            {
                self.begin_rename();
                ui.close();
            }
            if ui.add_enabled(!self.group.is_empty(), egui::Button::new(tr("Duplicate"))).clicked() {
                let fragment = self.diagram.fragment(&self.group);
                self.paste_count = 0;
                self.paste(&fragment);
                ui.close();
            }
            if ui
                .add_enabled(self.selection.is_some(), egui::Button::new(tr("Delete selection")))
                .clicked()
            {
                self.delete_selection();
                ui.close();
            }
            ui.separator();
            if ui.button(tr("Zoom to fit")).clicked() {
                self.request_fit();
                ui.close();
            }
        });
        self.update_routes();
        let pal = Palette::for_ui(ui);
        let painter = ui.painter().with_clip_rect(rect);
        let exporting = std::mem::take(&mut self.export_png);
        canvas::draw_background(&painter, rect, &self.view, &pal);
        for n in self.diagram.nodes.iter().filter(|n| n.kind == NodeKind::System) {
            self.draw_node(&painter, rect, n, &pal, exporting);
        }
        self.label_hits.clear();
        for r in &self.diagram.relations {
            let hits = self.draw_relation(&painter, rect, r, &pal, exporting);
            self.label_hits.extend(hits.into_iter().map(|rect| (r.id, rect)));
        }
        for n in self.diagram.nodes.iter().filter(|n| n.kind != NodeKind::System) {
            self.draw_node(&painter, rect, n, &pal, exporting);
        }
        if !exporting {
            self.draw_overlays(ui, &painter, rect, &pal);
            self.palette(ui, rect);
            self.inline_editor(ui, rect);
        } else {
            crate::io::request_png_export(ui.ctx(), rect, format!("{}.png", self.diagram.name));
        }
    }

    fn inline_editor(&mut self, ui: &Ui, rect: Rect) {
        let Some((target, mut text, first)) = self.editing.take() else {
            return;
        };
        let (pos, width) = match target {
            Selection::Node(id) => {
                let Some(n) = self.diagram.node(id) else {
                    return;
                };
                let r = self.view.rect_to_screen(rect, n.rect());
                (r.left_top() + vec2(0.0, 12.0 * self.view.zoom), r.width())
            }
            Selection::Relation(id) => {
                let Some(r) = self.diagram.relations.iter().find(|r| r.id == id) else {
                    return;
                };
                let points: Vec<_> = self.relation_points(r).iter().map(|p| self.view.to_screen(rect, *p)).collect();
                (canvas::point_along(&points, 0.5) - vec2(80.0, 15.0), 160.0)
            }
        };
        let outcome = canvas::inline_editor(ui.ctx(), egui::Id::unique(("uml_inline", self.uid)), pos, width, &mut text, first);
        match outcome {
            InlineOutcome::Editing => self.editing = Some((target, text, false)),
            InlineOutcome::Cancelled => {}
            InlineOutcome::Done => {
                match target {
                    Selection::Node(id) => {
                        if let Some(n) = self.diagram.nodes.iter_mut().find(|n| n.id == id) {
                            n.name = text;
                            Self::fit_text(ui.painter(), n, false);
                        }
                    }
                    Selection::Relation(id) => {
                        if let Some(r) = self.diagram.relations.iter_mut().find(|r| r.id == id) {
                            r.label = text;
                        }
                    }
                }
                self.history.sync(&self.diagram, false);
            }
        }
    }
}
