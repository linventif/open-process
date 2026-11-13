//! Interaction for the PetriEditor notation.

use super::*;

impl PetriEditor {
    pub(super) fn shortcuts(&mut self, ui: &Ui) {
        let (del, esc, keys) = ui.input(|i| {
            (
                i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace),
                i.key_pressed(Key::Escape),
                [Key::V, Key::P, Key::T, Key::A, Key::K, Key::S, Key::F, Key::Space]
                    .map(|k| i.key_pressed(k) && !i.modifiers.command && !i.modifiers.ctrl),
            )
        });
        if del {
            self.delete_selection();
        }
        if esc {
            self.history.cancel(&mut self.net);
            self.drag = None;
            self.editing = None;
            self.tool = Tool::Select;
            self.arc_pending = None;
            self.selection = None;
            self.group.clear();
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(Key::A)) && self.sim.is_none() {
            self.select_all();
        }
        let editing = self.sim.is_none();
        let tools = [Tool::Select, Tool::Place, Tool::Transition, Tool::Arc, Tool::Token];
        for (k, tool) in tools.into_iter().enumerate() {
            if keys[k] && (editing || tool == Tool::Select) {
                self.tool = tool;
                self.arc_pending = None;
            }
        }
        if keys[5] {
            self.toggle_simulation(ui.input(|i| i.time));
        }
        if keys[6] {
            self.fit_request = true;
        }
        if keys[7] && self.sim.is_some() {
            self.random_step();
        }
    }

    pub(super) fn sim_interaction(&mut self, response: &egui::Response, wp: Option<Pos>) {
        if response.dragged_by(PointerButton::Primary) {
            self.view.offset += response.drag_delta();
        }
        if response.clicked()
            && let Some(w) = wp
        {
            match self.hit_node(w) {
                Some(NodeRef::Transition(id)) => {
                    let ti = self.net.transition_index(id).unwrap();
                    let name = self.net.transitions[ti].name.clone();
                    if self.fire(ti) {
                        self.status = trf("petri.fired", &[&name]);
                    } else {
                        self.status = trf("petri.not_enabled", &[&name]);
                    }
                }
                Some(n) => self.selection = Some(Sel::Node(n)),
                None => self.selection = None,
            }
        }
    }

    pub(super) fn edit_interaction(&mut self, ui: &Ui, response: &egui::Response, canvas: Rect, wp: Option<Pos>) {
        let press = ui.input(|i| i.pointer.press_origin()).map(|p| self.view.to_world(canvas, p));
        let (shift, space) = ui.input(|i| (i.modifiers.shift, i.key_down(Key::Space)));

        // ---- drag start
        if response.drag_started_by(PointerButton::Primary)
            && let Some(w) = press
        {
            let hit = self.hit_node(w);
            self.drag = match (self.tool, hit) {
                _ if space => Some(Drag::Pan),
                (Tool::Select | Tool::Token, Some(n)) if self.group.contains(&n) => Some(Drag::MoveGroup { last: w }),
                (Tool::Select | Tool::Token, Some(n)) => {
                    self.group.clear();
                    self.selection = Some(Sel::Node(n));
                    let p = self.net.node_pos(n).unwrap_or(w);
                    Some(Drag::Move { node: n, grab: w - p })
                }
                (Tool::Select, None) => {
                    if !shift {
                        self.group.clear();
                        self.selection = None;
                    }
                    Some(Drag::Rubber { start: w })
                }
                (Tool::Arc, Some(n)) => Some(Drag::Arc { from: n }),
                _ => Some(Drag::Pan),
            };
        }
        // ---- dragging
        if response.dragged_by(PointerButton::Primary) {
            match self.drag {
                Some(Drag::Move { node, grab }) => {
                    if let Some(w) = wp {
                        let p = self.snap(w - grab);
                        self.net.set_node_pos(node, p);
                    }
                }
                Some(Drag::MoveGroup { last }) => {
                    if let Some(w) = wp {
                        let d = w - last;
                        for n in self.group.clone() {
                            if let Some(p) = self.net.node_pos(n) {
                                self.net.set_node_pos(n, p + d);
                            }
                        }
                        self.drag = Some(Drag::MoveGroup { last: w });
                    }
                }
                Some(Drag::Pan) => self.view.offset += response.drag_delta(),
                _ => {}
            }
        }
        // ---- drag end
        if response.drag_stopped_by(PointerButton::Primary) {
            match (self.drag, wp) {
                (Some(Drag::Arc { from }), Some(w)) => {
                    if let Some(to) = self.hit_node(w) {
                        self.connect(from, to);
                    }
                }
                (Some(Drag::Rubber { start }), Some(w)) => {
                    let r = op_core::geom::Rect::new(start.x.min(w.x), start.y.min(w.y), (start.x - w.x).abs(), (start.y - w.y).abs());
                    let inside: Vec<NodeRef> = self
                        .net
                        .places
                        .iter()
                        .map(|p| (NodeRef::Place(p.id), p.pos))
                        .chain(self.net.transitions.iter().map(|t| (NodeRef::Transition(t.id), t.pos)))
                        .filter(|(_, p)| r.contains(*p))
                        .map(|(n, _)| n)
                        .collect();
                    for n in inside {
                        if !self.group.contains(&n) {
                            self.group.push(n);
                        }
                    }
                    self.normalize_group();
                }
                (Some(Drag::MoveGroup { .. }), _) if self.snap => {
                    for n in self.group.clone() {
                        if let Some(p) = self.net.node_pos(n) {
                            self.net.set_node_pos(n, p.snapped(GRID));
                        }
                    }
                }
                _ => {}
            }
            self.drag = None;
        }

        // ---- clicks
        if response.clicked()
            && let Some(w) = wp
        {
            let hit = self.hit(w);
            match self.tool {
                Tool::Select => {
                    if shift && let Some(Sel::Node(n)) = hit {
                        // Shift+click toggles a node in the multi-selection
                        if let Some(Sel::Node(prev)) = self.selection.take()
                            && !self.group.contains(&prev)
                        {
                            self.group.push(prev);
                        }
                        if let Some(i) = self.group.iter().position(|x| *x == n) {
                            self.group.remove(i);
                        } else {
                            self.group.push(n);
                        }
                        self.normalize_group();
                    } else {
                        self.group.clear();
                        self.selection = hit;
                    }
                }
                Tool::Place => {
                    let id = self.net.add_place(self.snap(w), 0);
                    self.selection = Some(Sel::Node(NodeRef::Place(id)));
                }
                Tool::Transition => {
                    let id = self.net.add_transition(self.snap(w));
                    self.selection = Some(Sel::Node(NodeRef::Transition(id)));
                }
                Tool::Arc => match (self.arc_pending, self.hit_node(w)) {
                    (None, Some(n)) => self.arc_pending = Some(n),
                    (Some(from), Some(to)) => {
                        self.connect(from, to);
                        self.arc_pending = None;
                    }
                    (_, None) => self.arc_pending = None,
                },
                Tool::Token => {
                    if let Some(NodeRef::Place(id)) = self.hit_node(w)
                        && let Some(p) = self.net.place_mut(id)
                    {
                        p.tokens += 1;
                    }
                }
            }
        }
        if response.double_clicked()
            && let Some(w) = wp
            && let Some(n) = self.hit_node(w)
            && self.tool == Tool::Select
        {
            let name = self.net.node_name(n).unwrap_or("").to_string();
            self.group.clear();
            self.selection = Some(Sel::Node(n));
            self.editing = Some((n, name, true));
        }
        if response.secondary_clicked()
            && let Some(w) = wp
        {
            if self.tool == Tool::Token {
                if let Some(NodeRef::Place(id)) = self.hit_node(w)
                    && let Some(p) = self.net.place_mut(id)
                {
                    p.tokens = p.tokens.saturating_sub(1);
                }
            } else {
                let hit = self.hit(w);
                if hit.is_some() && !matches!(hit, Some(Sel::Node(n)) if self.group.contains(&n)) {
                    self.group.clear();
                    self.selection = hit;
                }
                self.ctx_target = (hit, w);
            }
        }
        if self.tool != Tool::Token {
            response.context_menu(|ui| self.context_menu(ui));
        }
    }

    /// A one-element group is just a normal selection.
    pub(super) fn normalize_group(&mut self) {
        self.group.retain(|n| self.net.contains(*n));
        match self.group.len() {
            0 => {}
            1 => self.selection = Some(Sel::Node(self.group.pop().unwrap())),
            _ => self.selection = None,
        }
    }

    pub fn select_all(&mut self) {
        self.group = self
            .net
            .places
            .iter()
            .map(|p| NodeRef::Place(p.id))
            .chain(self.net.transitions.iter().map(|t| NodeRef::Transition(t.id)))
            .collect();
        self.normalize_group();
    }

    pub(super) fn in_selection(&self, n: NodeRef) -> bool {
        self.selection == Some(Sel::Node(n)) || self.group.contains(&n)
    }

    /// Drag & drop from the palette: ghost while dragging, element on release.
    pub(super) fn palette_drop(&mut self, ui: &Ui, canvas: Rect, pal: &Palette) {
        let Some(tool) = self.dnd else { return };
        let (pos, released) = ui.input(|i| (i.pointer.latest_pos(), i.pointer.any_released() || !i.pointer.any_down()));
        let Some(p) = pos else { return };
        let over_canvas = canvas.contains(p) && !self.toolbar_rect.contains(p);
        if over_canvas {
            let painter = ui
                .ctx()
                .layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique(("petri_dnd", self.uid))));
            let z = self.view.zoom;
            let c = if self.snap {
                self.view.to_screen(canvas, self.view.to_world(canvas, p).snapped(GRID))
            } else {
                p
            };
            match tool {
                Tool::Place => {
                    painter.circle(c, PLACE_R * z, pal.select.gamma_multiply(0.12), Stroke::new(1.6, pal.select));
                }
                Tool::Transition => {
                    painter.rect(
                        Rect::from_center_size(c, {
                            let (tw, th) = self.trans_size("T");
                            vec2(tw, th) * z
                        }),
                        3.0,
                        pal.select.gamma_multiply(0.5),
                        Stroke::new(1.6, pal.select),
                        StrokeKind::Middle,
                    );
                }
                _ => {}
            }
        }
        if released {
            if over_canvas {
                let w = self.snap(self.view.to_world(canvas, p));
                let id = match tool {
                    Tool::Place => Some(NodeRef::Place(self.net.add_place(w, 0))),
                    Tool::Transition => Some(NodeRef::Transition(self.net.add_transition(w))),
                    _ => None,
                };
                if let Some(n) = id {
                    self.group.clear();
                    self.selection = Some(Sel::Node(n));
                }
                self.tool = Tool::Select;
            }
            self.dnd = None;
        }
    }

    pub(super) fn connect(&mut self, from: NodeRef, to: NodeRef) {
        match self.net.add_arc(from, to) {
            Ok(id) => {
                self.selection = Some(Sel::Arc(id));
                self.status.clear();
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    pub(super) fn context_menu(&mut self, ui: &mut Ui) {
        let (target, w) = self.ctx_target;
        match target {
            Some(Sel::Node(NodeRef::Place(id))) => {
                if ui.button(tr("petri.add_token")).clicked() {
                    if let Some(p) = self.net.place_mut(id) {
                        p.tokens += 1;
                    }
                    ui.close();
                }
                if ui.button(tr("petri.remove_token")).clicked() {
                    if let Some(p) = self.net.place_mut(id) {
                        p.tokens = p.tokens.saturating_sub(1);
                    }
                    ui.close();
                }
                if ui.button(tr("petri.rename")).clicked() {
                    let n = NodeRef::Place(id);
                    self.editing = Some((n, self.net.node_name(n).unwrap_or("").into(), true));
                    ui.close();
                }
                if ui.button(tr("common.delete")).clicked() {
                    self.net.remove_node(NodeRef::Place(id));
                    ui.close();
                }
            }
            Some(Sel::Node(NodeRef::Transition(id))) => {
                if let Some(t) = self.net.transition_mut(id) {
                    ui.checkbox(&mut t.silent, tr("petri.silent_tau"));
                }
                if ui.button(tr("petri.rename")).clicked() {
                    let n = NodeRef::Transition(id);
                    self.editing = Some((n, self.net.node_name(n).unwrap_or("").into(), true));
                    ui.close();
                }
                if ui.button(tr("common.delete")).clicked() {
                    self.net.remove_node(NodeRef::Transition(id));
                    ui.close();
                }
            }
            Some(Sel::Arc(id)) => {
                if let Some(a) = self.net.arc_mut(id) {
                    ui.horizontal(|ui| {
                        ui.label(tr("petri.weight"));
                        ui.add(egui::DragValue::new(&mut a.weight).range(1..=1000));
                    });
                    if matches!(a.from, NodeRef::Place(_)) {
                        let mut inh = a.kind == ArcKind::Inhibitor;
                        if ui.checkbox(&mut inh, tr("petri.inhibitor_arc")).changed() {
                            a.kind = if inh { ArcKind::Inhibitor } else { ArcKind::Normal };
                        }
                    }
                }
                if ui.button(tr("common.delete")).clicked() {
                    self.net.remove_arc(id);
                    ui.close();
                }
            }
            None => {
                if ui.button(tr("petri.add_place_here")).clicked() {
                    let id = self.net.add_place(self.snap(w), 0);
                    self.selection = Some(Sel::Node(NodeRef::Place(id)));
                    ui.close();
                }
                if ui.button(tr("petri.add_transition_here")).clicked() {
                    let id = self.net.add_transition(self.snap(w));
                    self.selection = Some(Sel::Node(NodeRef::Transition(id)));
                    ui.close();
                }
                if ui.button(tr("canvas.zoom_fit")).clicked() {
                    self.fit_request = true;
                    ui.close();
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Rendering
}
