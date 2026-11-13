//! Interaction for the BpmnEditor notation.

use super::*;

impl BpmnEditor {
    pub(super) fn set_name(&mut self, target: Sel, text: String) {
        match target {
            Sel::Node(id) => self.diagram.node_mut(id).into_iter().for_each(|n| n.name = text.clone()),
            Sel::Flow(id) => self.diagram.flow_mut(id).into_iter().for_each(|f| f.name = text.clone()),
            Sel::Pool(id) => self.diagram.pool_mut(id).into_iter().for_each(|p| p.name = text.clone()),
            Sel::Lane(pid, i) => {
                if let Some(l) = self.diagram.pool_mut(pid).and_then(|p| p.lanes.get_mut(i)) {
                    l.name = text;
                }
            }
        }
    }

    pub(super) fn name_of(&self, target: Sel) -> String {
        match target {
            Sel::Node(id) => self.diagram.node(id).map(|n| n.name.clone()),
            Sel::Flow(id) => self.diagram.flow(id).map(|f| f.name.clone()),
            Sel::Pool(id) => self.diagram.pool(id).map(|p| p.name.clone()),
            Sel::Lane(pid, i) => self.diagram.pool(pid).and_then(|p| p.lanes.get(i)).map(|l| l.name.clone()),
        }
        .unwrap_or_default()
    }

    pub(super) fn shortcuts(&mut self, ui: &Ui) {
        let pressed = |k: Key| ui.input(|i| i.key_pressed(k) && !i.modifiers.command && !i.modifiers.ctrl);
        if pressed(Key::Delete) || pressed(Key::Backspace) {
            self.delete_selection();
        }
        if pressed(Key::Escape) {
            self.history.cancel(&mut self.diagram);
            self.drag = None;
            self.editing = None;
            self.tool = Tool::Select;
            self.connect_pending = None;
            self.selection = None;
            self.group.clear();
        }
        if ui.input(|i| i.modifiers.command && i.key_pressed(Key::A)) {
            self.select_all();
        }
        if pressed(Key::F2)
            && let Some(sel) = self.selection
        {
            self.editing = Some((sel, self.name_of(sel), true));
        }
        let map: [(Key, Tool); 9] = [
            (Key::V, Tool::Select),
            (Key::C, Tool::Connect),
            (Key::S, Tool::Create(NodeKind::StartEvent(EventTrigger::None))),
            (
                Key::I,
                Tool::Create(NodeKind::IntermediateEvent {
                    trigger: EventTrigger::None,
                    throwing: false,
                }),
            ),
            (Key::E, Tool::Create(NodeKind::EndEvent(EventTrigger::None))),
            (Key::T, Tool::Create(NodeKind::Task(TaskType::Abstract))),
            (Key::X, Tool::Create(NodeKind::Gateway(GatewayType::Exclusive))),
            (Key::Plus, Tool::Create(NodeKind::Gateway(GatewayType::Parallel))),
            (Key::O, Tool::Create(NodeKind::Gateway(GatewayType::Inclusive))),
        ];
        for (k, t) in map {
            if pressed(k) {
                self.tool = t;
                self.connect_pending = None;
            }
        }
        if pressed(Key::F) {
            self.fit_request = true;
        }
    }

    pub(super) fn interaction(&mut self, ui: &Ui, response: &egui::Response, canvas: Rect, pointer: Option<Pos2>, wp: Option<Pos>) {
        let press_screen = ui.input(|i| i.pointer.press_origin());
        let press = press_screen.map(|p| self.view.to_world(canvas, p));
        let (shift, space) = ui.input(|i| (i.modifiers.shift, i.key_down(Key::Space)));
        let pad = self.pad_buttons(canvas);
        let on_pad = |p: Option<Pos2>| p.and_then(|p| pad.iter().find(|(r, _, _)| r.contains(p)).map(|(_, a, _)| *a));

        // ---- drag start
        if response.drag_started_by(PointerButton::Primary)
            && let Some(w) = press
            && on_pad(press_screen).is_none()
        {
            let handle = self
                .resize_handle(canvas)
                .filter(|(r, _)| press_screen.is_some_and(|p| r.expand(3.0).contains(p)));
            self.drag = if let Some((_, sel)) = handle {
                match sel {
                    Sel::Node(id) => Some(Drag::ResizeNode { node: id }),
                    Sel::Pool(id) => Some(Drag::ResizePool { pool: id }),
                    _ => None,
                }
            } else {
                match self.tool {
                    _ if space => Some(Drag::Pan),
                    Tool::Connect => self.hit_node(w).map(|n| Drag::Connect { from: n }).or(Some(Drag::Pan)),
                    _ => match self.hit(w) {
                        Some(Sel::Node(id)) if self.group.contains(&id) => Some(Drag::MoveGroup { last: w }),
                        Some(Sel::Node(id)) => {
                            self.group.clear();
                            self.selection = Some(Sel::Node(id));
                            let p = self.diagram.node(id).map(|n| n.pos).unwrap_or(w);
                            Some(Drag::Move { node: id, grab: w - p })
                        }
                        Some(Sel::Pool(id)) => {
                            self.group.clear();
                            self.selection = Some(Sel::Pool(id));
                            Some(Drag::MovePool { pool: id, last: w })
                        }
                        _ if self.tool == Tool::Select => {
                            if !shift {
                                self.group.clear();
                                self.selection = None;
                            }
                            Some(Drag::Rubber { start: w })
                        }
                        _ => Some(Drag::Pan),
                    },
                }
            };
        }

        // ---- dragging
        if response.dragged_by(PointerButton::Primary)
            && let Some(w) = wp
        {
            match self.drag {
                Some(Drag::Move { node, grab }) => {
                    let p = self.snap(w - grab);
                    if let Some(n) = self.diagram.node_mut(node) {
                        n.pos = p;
                    }
                }
                Some(Drag::MovePool { pool, last }) => {
                    let delta = w - last;
                    if let Some(p) = self.diagram.pool(pool) {
                        let r = p.rect();
                        let inside: Vec<Id> = self.diagram.nodes.iter().filter(|n| r.contains(n.pos)).map(|n| n.id).collect();
                        for id in inside {
                            if let Some(n) = self.diagram.node_mut(id) {
                                n.pos = n.pos + delta;
                            }
                        }
                    }
                    if let Some(p) = self.diagram.pool_mut(pool) {
                        p.pos = p.pos + delta;
                    }
                    self.drag = Some(Drag::MovePool { pool, last: w });
                }
                Some(Drag::ResizeNode { node }) => {
                    let snapped = self.snap(w);
                    if let Some(n) = self.diagram.node_mut(node) {
                        let left = n.pos.x - n.w / 2.0;
                        let top = n.pos.y - n.h / 2.0;
                        n.w = (snapped.x - left).max(40.0);
                        n.h = (snapped.y - top).max(30.0);
                        n.pos = Pos::new(left + n.w / 2.0, top + n.h / 2.0);
                    }
                }
                Some(Drag::ResizePool { pool }) => {
                    let snapped = self.snap(w);
                    if let Some(p) = self.diagram.pool_mut(pool) {
                        p.width = (snapped.x - p.pos.x).max(200.0);
                        let others: f32 = p.lanes[..p.lanes.len() - 1].iter().map(|l| l.height).sum();
                        let last = p.lanes.last_mut().unwrap();
                        last.height = (snapped.y - p.pos.y - others).max(60.0);
                    }
                }
                Some(Drag::MoveGroup { last }) => {
                    let d = w - last;
                    for id in self.group.clone() {
                        if let Some(n) = self.diagram.node_mut(id) {
                            n.pos = n.pos + d;
                        }
                    }
                    self.drag = Some(Drag::MoveGroup { last: w });
                }
                Some(Drag::Pan) => self.view.offset += response.drag_delta(),
                _ => {}
            }
        }

        // ---- drag end
        if response.drag_stopped_by(PointerButton::Primary) {
            match (self.drag, wp) {
                (Some(Drag::Connect { from }), Some(w)) => {
                    if let Some(to) = self.hit_node(w) {
                        self.connect(from, to);
                    }
                }
                (Some(Drag::Rubber { start }), Some(w)) => {
                    let r = op_core::geom::Rect::new(start.x.min(w.x), start.y.min(w.y), (start.x - w.x).abs(), (start.y - w.y).abs());
                    for n in &self.diagram.nodes {
                        if r.contains(n.pos) && !self.group.contains(&n.id) {
                            self.group.push(n.id);
                        }
                    }
                    self.normalize_group();
                }
                (Some(Drag::MoveGroup { .. }), _) if self.snap => {
                    for id in self.group.clone() {
                        if let Some(n) = self.diagram.node_mut(id) {
                            n.pos = n.pos.snapped(GRID);
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
            if let Some(action) = on_pad(pointer) {
                self.pad_action(action);
            } else {
                match self.tool {
                    Tool::Select => {
                        let hit = self.hit(w);
                        if shift && let Some(Sel::Node(n)) = hit {
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
                    Tool::Connect => match (self.connect_pending, self.hit_node(w)) {
                        (None, Some(n)) => self.connect_pending = Some(n),
                        (Some(from), Some(to)) => {
                            self.connect(from, to);
                            self.connect_pending = None;
                            self.tool = Tool::Select;
                        }
                        (_, None) => self.connect_pending = None,
                    },
                    tool @ (Tool::Create(_) | Tool::Pool) => self.create_at(tool, w),
                }
            }
        }
        if response.double_clicked()
            && let Some(w) = wp
            && self.tool == Tool::Select
            && let Some(sel) = self.hit(w)
        {
            self.selection = Some(sel);
            self.editing = Some((sel, self.name_of(sel), true));
        }
        if response.secondary_clicked()
            && let Some(w) = wp
        {
            let hit = self.hit(w);
            if hit.is_some() && !matches!(hit, Some(Sel::Node(n)) if self.group.contains(&n)) {
                self.group.clear();
                self.selection = hit;
            }
            self.ctx_target = (hit, w);
        }
        response.context_menu(|ui| self.context_menu(ui));
    }

    /// Creates the element of `tool` at world position `w` (click or drop).
    pub(super) fn create_at(&mut self, tool: Tool, w: Pos) {
        let pos = self.snap(w);
        self.group.clear();
        match tool {
            Tool::Create(kind) => {
                let id = self.diagram.add_node(kind, pos);
                // Dropping on a flow: insert the element in the flow
                if kind.is_flow_node()
                    && let Some(fid) = self.hit_flow(w)
                {
                    self.split_flow(fid, id);
                }
                self.selection = Some(Sel::Node(id));
            }
            Tool::Pool => {
                let id = self.diagram.add_pool(pos);
                self.selection = Some(Sel::Pool(id));
            }
            _ => {}
        }
        self.tool = Tool::Select;
    }

    pub(super) fn normalize_group(&mut self) {
        self.group.retain(|id| self.diagram.node(*id).is_some());
        match self.group.len() {
            0 => {}
            1 => self.selection = Some(Sel::Node(self.group.pop().unwrap())),
            _ => self.selection = None,
        }
    }

    pub fn select_all(&mut self) {
        self.group = self.diagram.nodes.iter().map(|n| n.id).collect();
        self.normalize_group();
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
                .layer_painter(egui::LayerId::new(egui::Order::Tooltip, egui::Id::unique(("bpmn_dnd", self.uid))));
            let z = self.view.zoom;
            let w = self.view.to_world(canvas, p);
            let c = self.view.to_screen(canvas, self.snap(w));
            match tool {
                Tool::Create(kind) => {
                    let (sw, sh) = kind.default_size();
                    let r = Rect::from_center_size(c, vec2(sw, sh) * z);
                    paint_node(&painter, r, kind, "", z, pal, Some(pal.select));
                    if kind.is_flow_node() && self.hit_flow(w).is_some() {
                        canvas::badge(
                            &painter,
                            c + vec2(0.0, -sh * z / 2.0 - 14.0),
                            tr("insert into flow"),
                            11.0,
                            pal.select,
                            pal,
                        );
                    }
                }
                Tool::Pool => {
                    painter.rect_stroke(
                        Rect::from_min_size(c, vec2(700.0, 180.0) * z),
                        0.0,
                        Stroke::new(1.5, pal.select),
                        StrokeKind::Middle,
                    );
                }
                _ => {}
            }
        }
        if released {
            if over_canvas {
                self.create_at(tool, self.view.to_world(canvas, p));
            }
            self.dnd = None;
        }
    }

    pub(super) fn connect(&mut self, from: Id, to: Id) {
        match self.diagram.add_flow(from, to) {
            Ok(id) => {
                self.selection = Some(Sel::Flow(id));
                self.status.clear();
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    /// Inserts node `mid` in sequence flow `fid` (a → b becomes a → mid → b).
    pub(super) fn split_flow(&mut self, fid: Id, mid: Id) {
        let Some(f) = self.diagram.flow(fid).cloned() else { return };
        if f.kind != FlowKind::Sequence {
            return;
        }
        self.diagram.remove_flow(fid);
        if let Ok(id) = self.diagram.add_flow(f.from, mid)
            && let Some(nf) = self.diagram.flow_mut(id)
        {
            nf.name = f.name.clone();
            nf.is_default = f.is_default;
        }
        let _ = self.diagram.add_flow(mid, f.to);
    }

    pub(super) fn pad_action(&mut self, action: PadAction) {
        let Some(Sel::Node(id)) = self.selection else { return };
        match action {
            PadAction::Delete => self.delete_selection(),
            PadAction::Connect => {
                self.tool = Tool::Connect;
                self.connect_pending = Some(id);
            }
            PadAction::Append(kind) => {
                let Some(n) = self.diagram.node(id).cloned() else { return };
                let (w, _) = kind.default_size();
                let mut pos = self.snap(Pos::new(n.pos.x + n.w / 2.0 + 70.0 + w / 2.0, n.pos.y));
                // avoid overlapping an existing element
                let mut guard = 0;
                while self.diagram.nodes.iter().any(|m| m.rect().contains(pos)) && guard < 20 {
                    pos.y += 110.0;
                    guard += 1;
                }
                let new = self.diagram.add_node(kind, pos);
                let _ = self.diagram.add_flow(id, new);
                self.selection = Some(Sel::Node(new));
                if matches!(kind, NodeKind::Task(_)) {
                    let name = self.name_of(Sel::Node(new));
                    self.editing = Some((Sel::Node(new), name, true));
                }
            }
        }
    }

    pub(super) fn context_menu(&mut self, ui: &mut Ui) {
        let (target, w) = self.ctx_target;
        match target {
            Some(Sel::Node(id)) => {
                if ui.button(tr("Rename…  (F2)")).clicked() {
                    self.editing = Some((Sel::Node(id), self.name_of(Sel::Node(id)), true));
                    ui.close();
                }
                let kind = self.diagram.node(id).map(|n| n.kind);
                if let Some(NodeKind::Task(_)) = kind {
                    ui.menu_button(tr("Task type"), |ui| {
                        for t in TaskType::ALL {
                            if ui.button(t.label()).clicked() {
                                if let Some(n) = self.diagram.node_mut(id) {
                                    n.kind = NodeKind::Task(t);
                                }
                                ui.close();
                            }
                        }
                    });
                }
                if let Some(NodeKind::Gateway(_)) = kind {
                    ui.menu_button(tr("Gateway type"), |ui| {
                        for g in GatewayType::ALL {
                            if ui.button(g.label()).clicked() {
                                if let Some(n) = self.diagram.node_mut(id) {
                                    n.kind = NodeKind::Gateway(g);
                                }
                                ui.close();
                            }
                        }
                    });
                }
                if ui.button(tr("Delete")).clicked() {
                    self.diagram.remove_node(id);
                    ui.close();
                }
            }
            Some(Sel::Flow(id)) => {
                if ui.button(tr("Edit label / condition…")).clicked() {
                    self.editing = Some((Sel::Flow(id), self.name_of(Sel::Flow(id)), true));
                    ui.close();
                }
                if ui.button(tr("Delete")).clicked() {
                    self.diagram.remove_flow(id);
                    ui.close();
                }
            }
            Some(Sel::Pool(id)) => {
                if ui.button(tr("Add lane")).clicked() {
                    self.diagram.add_lane(id);
                    ui.close();
                }
                if ui.button(tr("Delete pool")).clicked() {
                    self.diagram.remove_pool(id);
                    ui.close();
                }
            }
            Some(Sel::Lane(pid, i)) => {
                if ui.button(tr("Rename lane…")).clicked() {
                    self.editing = Some((Sel::Lane(pid, i), self.name_of(Sel::Lane(pid, i)), true));
                    ui.close();
                }
                if ui.button(tr("Add lane")).clicked() {
                    self.diagram.add_lane(pid);
                    ui.close();
                }
                if ui.button(tr("Delete lane")).clicked() {
                    self.selection = Some(Sel::Lane(pid, i));
                    self.delete_selection();
                    ui.close();
                }
            }
            None => {
                for (kind, label) in [
                    (NodeKind::Task(TaskType::Abstract), tr("Add task here")),
                    (NodeKind::Gateway(GatewayType::Exclusive), tr("Add exclusive gateway here")),
                    (NodeKind::StartEvent(EventTrigger::None), tr("Add start event here")),
                    (NodeKind::EndEvent(EventTrigger::None), tr("Add end event here")),
                ] {
                    if ui.button(label).clicked() {
                        let id = self.diagram.add_node(kind, self.snap(w));
                        self.selection = Some(Sel::Node(id));
                        ui.close();
                    }
                }
                if ui.button(tr("Zoom to fit  (F)")).clicked() {
                    self.fit_request = true;
                    ui.close();
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Rendering
}
