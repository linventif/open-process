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
            self.pool_group.clear();
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
            let handle = press_screen.and_then(|p| self.node_resize_at(canvas, p));
            let pool_resize = press_screen.and_then(|p| self.pool_resize_at(canvas, p));
            let flow_end = self
                .selected_flow_path(canvas)
                .and_then(|(flow, path)| Some((flow, canvas::end_at(&path, press_screen?)?)));
            self.drag = if space {
                Some(Drag::Pan)
            } else if let Some((flow, end)) = flow_end {
                Some(Drag::Reconnect { flow, end })
            } else if self.tool == Tool::Select
                && ui.input(|i| i.modifiers.alt)
                && matches!(self.hit(w), Some(Sel::Node(_) | Sel::Pool(_) | Sel::Lane(_, _)))
            {
                let hit = self.hit(w).unwrap();
                if !matches!(hit, Sel::Node(id) if self.group.contains(&id)) {
                    self.group.clear();
                    self.pool_group.clear();
                    self.selection = Some(hit);
                }
                self.duplicate_selection(Pos::default(), false);
                match self.selection {
                    Some(Sel::Pool(pool)) => Some(Drag::MovePool { pool, last: w }),
                    Some(Sel::Node(node)) => Some(Drag::Move {
                        node,
                        grab: w - self.diagram.node(node).unwrap().pos,
                    }),
                    _ => Some(Drag::MoveGroup { last: w }),
                }
            } else if let Some((node, edge)) = handle {
                self.group.clear();
                self.pool_group.clear();
                self.selection = Some(Sel::Node(node));
                Some(Drag::ResizeNode {
                    node,
                    gesture: canvas::resize::ResizeGesture::new(self.diagram.node(node).unwrap().rect(), edge, w),
                })
            } else if let Some((pool, target)) = pool_resize {
                self.group.clear();
                self.pool_group.clear();
                self.selection = Some(match target {
                    PoolResize::LaneBoundary(i) => Sel::Lane(pool, i),
                    _ => Sel::Pool(pool),
                });
                self.history.begin(&self.diagram);
                target.anchor(self.diagram.pool(pool).unwrap()).map(|anchor| Drag::ResizePool {
                    pool,
                    target,
                    grab: w - anchor,
                })
            } else {
                match self.tool {
                    Tool::Connect => self.hit_node(w).map(|n| Drag::Connect { from: n }).or(Some(Drag::Pan)),
                    _ => match self.hit(w) {
                        Some(Sel::Node(id)) if self.group.contains(&id) => Some(Drag::MoveGroup { last: w }),
                        Some(Sel::Node(id)) => {
                            self.group.clear();
                            self.pool_group.clear();
                            self.selection = Some(Sel::Node(id));
                            let p = self.diagram.node(id).map(|n| n.pos).unwrap_or(w);
                            Some(Drag::Move { node: id, grab: w - p })
                        }
                        Some(Sel::Pool(id)) => {
                            self.pool_move_nodes = self.diagram.fragment(&[], &[id]).nodes.iter().map(|n| n.id).collect();
                            self.group.clear();
                            self.pool_group.clear();
                            self.selection = Some(Sel::Pool(id));
                            Some(Drag::MovePool { pool: id, last: w })
                        }
                        _ if self.tool == Tool::Select => {
                            if !shift {
                                self.group.clear();
                                self.pool_group.clear();
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
                    for id in &self.pool_move_nodes {
                        if let Some(node) = self.diagram.node_mut(*id) {
                            node.pos = node.pos + delta;
                        }
                    }
                    if let Some(p) = self.diagram.pool_mut(pool) {
                        p.pos = p.pos + delta;
                    }
                    self.drag = Some(Drag::MovePool { pool, last: w });
                }
                Some(Drag::ResizeNode { node, gesture }) => {
                    if let Some(n) = self.diagram.node_mut(node) {
                        let bounds = gesture.apply(
                            n.rect(),
                            w,
                            Pos::new(40.0, 30.0),
                            Pos::new(f32::INFINITY, f32::INFINITY),
                            self.snap.then_some(GRID),
                        );
                        n.w = bounds.w;
                        n.h = bounds.h;
                        n.pos = bounds.center();
                    }
                }
                Some(Drag::ResizePool { pool, target, grab }) => {
                    self.diagram.resize_pool(pool, target, w - grab, self.snap.then_some(GRID));
                }
                Some(Drag::MoveGroup { last }) => {
                    let d = w - last;
                    for id in self.group.clone() {
                        if let Some(n) = self.diagram.node_mut(id) {
                            n.pos = n.pos + d;
                        }
                    }
                    for pool in &self.pool_group {
                        if let Some(pool) = self.diagram.pool_mut(*pool) {
                            pool.pos = pool.pos + d;
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
                (Some(Drag::Reconnect { flow, end }), Some(w)) => {
                    if let Some(node) = self.hit_node(w) {
                        match self.diagram.reconnect(flow, end, node) {
                            Ok(()) => self.status.clear(),
                            Err(e) => self.status = e.into(),
                        }
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
                            self.pool_group.clear();
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
                self.pool_group.clear();
                self.selection = hit;
            }
            self.ctx_target = (hit, w);
        }
        response.context_menu(|ui| self.context_menu(ui));

        let cursor = match self.drag {
            Some(Drag::ResizePool { target, .. }) => Some(resize::resize_cursor(target)),
            Some(Drag::ResizeNode { gesture, .. }) => Some(canvas::resize::cursor(gesture.edge)),
            None if response.hovered() && !space && on_pad(pointer).is_none() => pointer.and_then(|p| {
                if let Some((_, edge)) = self.node_resize_at(canvas, p) {
                    Some(canvas::resize::cursor(edge))
                } else {
                    self.pool_resize_at(canvas, p).map(|(_, target)| resize::resize_cursor(target))
                }
            }),
            _ => None,
        };
        if let Some(cursor) = cursor {
            ui.ctx().set_cursor_icon(cursor);
        }
    }

    /// Creates the element of `tool` at world position `w` (click or drop).
    pub(super) fn create_at(&mut self, tool: Tool, w: Pos) {
        let pos = self.snap(w);
        self.group.clear();
        self.pool_group.clear();
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
        self.pool_group = self.diagram.pools.iter().map(|p| p.id).collect();
        self.selection = None;
        if self.pool_group.is_empty() {
            self.normalize_group();
        }
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
                            tr("bpmn.insert_into_flow"),
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
        crate::theme::menu_style(ui.style_mut());
        self.selection_menu(ui);
        let (target, w) = self.ctx_target;
        match target {
            Some(Sel::Node(id)) => {
                if ui.button(tr("bpmn.rename_f2")).clicked() {
                    self.editing = Some((Sel::Node(id), self.name_of(Sel::Node(id)), true));
                    ui.close();
                }
                let kind = self.diagram.node(id).map(|n| n.kind);
                if let Some(NodeKind::Task(current)) = kind {
                    ui.menu_button((node_icon(NodeKind::Task(current)), tr("bpmn.task_type")), |ui| {
                        crate::theme::menu_style(ui.style_mut());
                        for t in TaskType::ALL {
                            if node_option(ui, t.label(), NodeKind::Task(t), Some(t == current)).clicked() {
                                if let Some(n) = self.diagram.node_mut(id) {
                                    n.kind = NodeKind::Task(t);
                                }
                                ui.close();
                            }
                        }
                    });
                }
                if let Some(NodeKind::Gateway(current)) = kind {
                    ui.menu_button((node_icon(NodeKind::Gateway(current)), tr("bpmn.gateway_type")), |ui| {
                        crate::theme::menu_style(ui.style_mut());
                        for g in GatewayType::ALL {
                            if node_option(ui, g.label(), NodeKind::Gateway(g), Some(g == current)).clicked() {
                                if let Some(n) = self.diagram.node_mut(id) {
                                    n.kind = NodeKind::Gateway(g);
                                }
                                ui.close();
                            }
                        }
                    });
                }
                if ui.button(tr("common.delete")).clicked() {
                    self.diagram.remove_node(id);
                    ui.close();
                }
            }
            Some(Sel::Flow(id)) => {
                if ui.button(tr("bpmn.edit_label_condition")).clicked() {
                    self.editing = Some((Sel::Flow(id), self.name_of(Sel::Flow(id)), true));
                    ui.close();
                }
                if ui.button(tr("common.delete")).clicked() {
                    self.diagram.remove_flow(id);
                    ui.close();
                }
            }
            Some(Sel::Pool(id)) => {
                if ui.button(tr("bpmn.add_lane")).clicked() {
                    self.diagram.add_lane(id);
                    ui.close();
                }
                if ui.button(tr("bpmn.delete_pool")).clicked() {
                    self.diagram.remove_pool(id);
                    ui.close();
                }
            }
            Some(Sel::Lane(pid, i)) => {
                if ui.button(tr("bpmn.rename_lane")).clicked() {
                    self.editing = Some((Sel::Lane(pid, i), self.name_of(Sel::Lane(pid, i)), true));
                    ui.close();
                }
                if ui.button(tr("bpmn.add_lane")).clicked() {
                    self.diagram.add_lane(pid);
                    ui.close();
                }
                if ui.button(tr("bpmn.delete_lane")).clicked() {
                    self.selection = Some(Sel::Lane(pid, i));
                    self.delete_selection();
                    ui.close();
                }
            }
            None => {
                for (kind, label) in [
                    (NodeKind::Task(TaskType::Abstract), tr("bpmn.add_task_here")),
                    (NodeKind::Gateway(GatewayType::Exclusive), tr("bpmn.add_exclusive_gateway_here")),
                    (NodeKind::StartEvent(EventTrigger::None), tr("bpmn.add_start_event_here")),
                    (NodeKind::EndEvent(EventTrigger::None), tr("bpmn.add_end_event_here")),
                ] {
                    if node_option(ui, label, kind, None).clicked() {
                        let id = self.diagram.add_node(kind, self.snap(w));
                        self.selection = Some(Sel::Node(id));
                        ui.close();
                    }
                }
                if ui.button(tr("canvas.zoom_fit_shortcut")).clicked() {
                    self.fit_request = true;
                    ui.close();
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Rendering
}
