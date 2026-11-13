//! BPMN 2.0 editor.

use crate::canvas::{self, InlineOutcome, Palette, View, to_pos2};
use crate::history::History;
use egui::{
    Color32, CornerRadius, FontId, Key, Painter, PointerButton, Pos2, Rect, RichText, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2,
};
use op_core::bpmn::*;
use op_core::geom::{Pos, dist_to_polyline};
use op_core::i18n::{key, tr, trf};

const GRID: f32 = 10.0;
const LANE_HEADER: f32 = 26.0;
const PAD_BTN: f32 = 26.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Tool {
    Select,
    Connect,
    Create(NodeKind),
    Pool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sel {
    Node(Id),
    Flow(Id),
    Pool(Id),
    Lane(Id, usize),
}

#[derive(Clone, Copy, Debug)]
enum Drag {
    Move { node: Id, grab: Pos },
    MovePool { pool: Id, last: Pos },
    ResizeNode { node: Id },
    ResizePool { pool: Id },
    Connect { from: Id },
    MoveGroup { last: Pos },
    Rubber { start: Pos },
    Pan,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideTab {
    Properties,
    Validation,
    Evaluation,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum PadAction {
    Append(NodeKind),
    Connect,
    Delete,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BpmnAction {
    ConvertToPetri,
}

pub struct BpmnEditor {
    pub uid: u64,
    /// Export the canvas as PNG on the next frame (without palette and selection).
    pub export_png: bool,
    /// Multi-selection of nodes (rubber band, Shift+click, Ctrl+A).
    group: Vec<Id>,
    dnd: Option<Tool>,
    toolbar_rect: Rect,
    /// None = automatic (hidden when the pane is narrow).
    show_inspector: Option<bool>,
    inspector_visible: bool,
    pub diagram: BpmnDiagram,
    pub view: View,
    last_canvas: Rect,
    tool: Tool,
    selection: Option<Sel>,
    drag: Option<Drag>,
    connect_pending: Option<Id>,
    history: History<BpmnDiagram>,
    pub status: String,
    snap: bool,
    fit_request: bool,
    ctx_target: (Option<Sel>, Pos),
    editing: Option<(Sel, String, bool)>,
    tab: SideTab,
    show_issues: bool,
    pub action: Option<BpmnAction>,
}

const PALETTE: &[(Tool, &str)] = &[
    (
        Tool::Select,
        key("Select / move (V)\nDrag empty space to pan, wheel to scroll, Ctrl+wheel to zoom"),
    ),
    (
        Tool::Connect,
        key("Connect (C): sequence flow, message flow between pools, association with data/annotations"),
    ),
    (Tool::Create(NodeKind::StartEvent(EventTrigger::None)), key("Start event (S)")),
    (
        Tool::Create(NodeKind::IntermediateEvent {
            trigger: EventTrigger::None,
            throwing: false,
        }),
        key("Intermediate event (I)"),
    ),
    (Tool::Create(NodeKind::EndEvent(EventTrigger::None)), key("End event (E)")),
    (Tool::Create(NodeKind::Task(TaskType::Abstract)), key("Task (T)")),
    (Tool::Create(NodeKind::SubProcess), key("Sub-process")),
    (
        Tool::Create(NodeKind::Gateway(GatewayType::Exclusive)),
        key("Exclusive gateway – XOR (X)"),
    ),
    (
        Tool::Create(NodeKind::Gateway(GatewayType::Parallel)),
        key("Parallel gateway – AND (+)"),
    ),
    (
        Tool::Create(NodeKind::Gateway(GatewayType::Inclusive)),
        key("Inclusive gateway – OR (O)"),
    ),
    (Tool::Create(NodeKind::Gateway(GatewayType::EventBased)), key("Event-based gateway")),
    (Tool::Create(NodeKind::DataObject), key("Data object")),
    (Tool::Create(NodeKind::DataStore), key("Data store")),
    (Tool::Create(NodeKind::TextAnnotation), key("Text annotation")),
    (Tool::Pool, key("Pool / participant (with lanes)")),
];

const PAD: &[(PadAction, &str)] = &[
    (PadAction::Append(NodeKind::Task(TaskType::Abstract)), key("Append task")),
    (
        PadAction::Append(NodeKind::Gateway(GatewayType::Exclusive)),
        key("Append exclusive gateway"),
    ),
    (
        PadAction::Append(NodeKind::Gateway(GatewayType::Parallel)),
        key("Append parallel gateway"),
    ),
    (
        PadAction::Append(NodeKind::IntermediateEvent {
            trigger: EventTrigger::None,
            throwing: false,
        }),
        key("Append intermediate event"),
    ),
    (PadAction::Append(NodeKind::EndEvent(EventTrigger::None)), key("Append end event")),
    (PadAction::Connect, key("Connect to…")),
    (PadAction::Delete, key("Delete")),
];

impl BpmnEditor {
    pub fn new(diagram: BpmnDiagram) -> Self {
        let history = History::new(&diagram);
        Self {
            uid: canvas::next_uid(),
            export_png: false,
            group: Vec::new(),
            dnd: None,
            toolbar_rect: Rect::NOTHING,
            show_inspector: None,
            inspector_visible: true,
            diagram,
            view: View::default(),
            last_canvas: Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)),
            tool: Tool::Select,
            selection: None,
            drag: None,
            connect_pending: None,
            history,
            status: String::new(),
            snap: true,
            fit_request: true,
            ctx_target: (None, Pos::default()),
            editing: None,
            tab: SideTab::Properties,
            show_issues: false,
            action: None,
        }
    }

    pub fn undo(&mut self) {
        self.history.undo(&mut self.diagram);
        self.selection = None;
    }

    pub fn redo(&mut self) {
        self.history.redo(&mut self.diagram);
        self.selection = None;
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    pub fn request_fit(&mut self) {
        self.fit_request = true;
    }

    pub fn delete_selection(&mut self) {
        for id in std::mem::take(&mut self.group) {
            self.diagram.remove_node(id);
        }
        match self.selection.take() {
            Some(Sel::Node(id)) => self.diagram.remove_node(id),
            Some(Sel::Flow(id)) => self.diagram.remove_flow(id),
            Some(Sel::Pool(id)) => self.diagram.remove_pool(id),
            Some(Sel::Lane(pool, i)) => {
                if let Some(p) = self.diagram.pool_mut(pool)
                    && p.lanes.len() > 1
                {
                    p.lanes.remove(i);
                }
            }
            None => {}
        }
    }

    fn snap(&self, p: Pos) -> Pos {
        if self.snap { p.snapped(GRID) } else { p }
    }

    // ------------------------------------------------------------------
    // Hit testing

    fn hit_node(&self, w: Pos) -> Option<Id> {
        self.diagram.nodes.iter().rev().find(|n| hit_shape(n, w)).map(|n| n.id)
    }

    fn hit_flow(&self, w: Pos) -> Option<Id> {
        let tol = 6.0 / self.view.zoom.max(0.3);
        self.diagram
            .flows
            .iter()
            .rev()
            .find(|f| dist_to_polyline(w, &route(&self.diagram, f)) <= tol)
            .map(|f| f.id)
    }

    /// Pool header / lane header / pool border.
    fn hit_pool(&self, w: Pos) -> Option<Sel> {
        let tol = 5.0 / self.view.zoom.max(0.3);
        for p in self.diagram.pools.iter().rev() {
            let r = p.rect();
            if !r.contains(w) && !op_core::geom::Rect::new(r.x - tol, r.y - tol, r.w + 2.0 * tol, r.h + 2.0 * tol).contains(w) {
                continue;
            }
            if w.x <= r.x + POOL_HEADER {
                return Some(Sel::Pool(p.id));
            }
            if p.lanes.len() > 1
                && w.x <= r.x + POOL_HEADER + LANE_HEADER
                && let Some(i) = p.lane_at(w)
            {
                return Some(Sel::Lane(p.id, i));
            }
            let near_border =
                (w.x - r.x).abs() < tol || (w.x - r.right()).abs() < tol || (w.y - r.y).abs() < tol || (w.y - r.bottom()).abs() < tol;
            if near_border {
                return Some(Sel::Pool(p.id));
            }
        }
        None
    }

    fn hit(&self, w: Pos) -> Option<Sel> {
        self.hit_node(w)
            .map(Sel::Node)
            .or_else(|| self.hit_flow(w).map(Sel::Flow))
            .or_else(|| self.hit_pool(w))
    }

    /// Screen rect of the resize handle of the selection, if any.
    fn resize_handle(&self, canvas: Rect) -> Option<(Rect, Sel)> {
        let sel = self.selection?;
        let corner = match sel {
            Sel::Node(id) => {
                let n = self.diagram.node(id)?;
                if !matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess | NodeKind::TextAnnotation) {
                    return None;
                }
                let r = n.rect();
                Pos::new(r.right(), r.bottom())
            }
            Sel::Pool(id) => {
                let r = self.diagram.pool(id)?.rect();
                Pos::new(r.right(), r.bottom())
            }
            _ => return None,
        };
        Some((Rect::from_center_size(self.view.to_screen(canvas, corner), vec2(10.0, 10.0)), sel))
    }

    /// Context pad buttons around the selected flow node (screen space).
    fn pad_buttons(&self, canvas: Rect) -> Vec<(Rect, PadAction, &'static str)> {
        if self.tool != Tool::Select || self.drag.is_some() || self.editing.is_some() {
            return Vec::new();
        }
        let Some(Sel::Node(id)) = self.selection else { return Vec::new() };
        let Some(n) = self.diagram.node(id) else { return Vec::new() };
        let r = n.rect();
        let origin = self.view.to_screen(canvas, Pos::new(r.right(), r.y)) + vec2(14.0, -4.0);
        let is_end = matches!(n.kind, NodeKind::EndEvent(_));
        let flow_node = n.kind.is_flow_node();
        let mut out = Vec::new();
        let mut k = 0;
        for &(a, tip) in PAD {
            let tip = tr(tip);
            let show = match a {
                PadAction::Append(_) => flow_node && !is_end,
                PadAction::Connect => !is_end || !flow_node,
                PadAction::Delete => true,
            };
            if !show {
                continue;
            }
            let col = (k % 2) as f32;
            let row = (k / 2) as f32;
            out.push((
                Rect::from_min_size(origin + vec2(col * (PAD_BTN + 3.0), row * (PAD_BTN + 3.0)), vec2(PAD_BTN, PAD_BTN)),
                a,
                tip,
            ));
            k += 1;
        }
        out
    }

    fn fit(&mut self, canvas: Rect) {
        let mut rects: Vec<op_core::geom::Rect> = self.diagram.nodes.iter().map(|n| n.rect()).collect();
        rects.extend(self.diagram.pools.iter().map(|p| p.rect()));
        if rects.is_empty() {
            self.view = View::default();
            return;
        }
        let min = Pos::new(
            rects.iter().map(|r| r.x).fold(f32::MAX, f32::min),
            rects.iter().map(|r| r.y).fold(f32::MAX, f32::min),
        );
        let max = Pos::new(
            rects.iter().map(|r| r.right()).fold(f32::MIN, f32::max),
            rects.iter().map(|r| r.bottom()).fold(f32::MIN, f32::max) + 20.0,
        );
        self.view.fit(canvas, min, max);
    }

    // ------------------------------------------------------------------
    // UI

    pub fn ui(&mut self, ui: &mut Ui) {
        let panel = ui.visuals().panel_fill;
        let inspector = self.show_inspector.unwrap_or(ui.available_width() > 760.0);
        self.inspector_visible = inspector;
        if inspector {
            egui::Panel::right("bpmn_side")
                .resizable(true)
                .default_size(320.0)
                .min_size(240.0)
                .frame(egui::Frame::new().fill(panel).inner_margin(egui::Margin {
                    left: 14,
                    right: 12,
                    top: 10,
                    bottom: 6,
                }))
                .show(ui, |ui| self.side_panel(ui));
        }
        egui::Panel::bottom("bpmn_status")
            .exact_size(30.0)
            .frame(egui::Frame::new().fill(panel).inner_margin(egui::Margin::symmetric(12, 3)))
            .show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.canvas(ui));
        let interacting = ui.input(|i| i.pointer.any_down()) || ui.ctx().egui_wants_keyboard_input();
        self.history.sync(&self.diagram, interacting);
    }

    fn palette(&mut self, ui: &mut Ui, max_height: f32) {
        ui.spacing_mut().item_spacing.y = 2.0;
        egui::ScrollArea::vertical().max_height(max_height).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                for (i, &(tool, tip)) in PALETTE.iter().enumerate() {
                    let tip = tr(tip);
                    if i == 2 || i == 7 || i == 11 || i == 14 {
                        canvas::toolbar_separator(ui);
                    }
                    let r = canvas::icon_button(ui, self.tool == tool, tip, |p, r, c| paint_tool_icon(p, r, c, tool));
                    if r.clicked() {
                        self.tool = tool;
                        self.connect_pending = None;
                    }
                    if r.drag_started() && matches!(tool, Tool::Create(_) | Tool::Pool) {
                        self.dnd = Some(tool);
                    }
                }
            });
        });
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            let narrow = ui.available_width() < 560.0;
            let seq = self.diagram.flows.iter().filter(|f| f.kind == FlowKind::Sequence).count();
            if !narrow {
                ui.label(
                    RichText::new(trf(
                        "{} elements · {} flows · {} pools",
                        &[&self.diagram.nodes.len(), &seq, &self.diagram.pools.len()],
                    ))
                    .color(weak),
                );
            }
            if !self.status.is_empty() {
                ui.add_space(8.0);
                ui.label(RichText::new(&self.status).color(weak));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .selectable_label(self.inspector_visible, tr("Inspector"))
                    .on_hover_text(tr("Show / hide the side panel"))
                    .clicked()
                {
                    self.show_inspector = Some(!self.inspector_visible);
                }
                canvas::zoom_controls(ui, &mut self.view, self.last_canvas, &mut self.fit_request);
                ui.add_space(10.0);
                ui.checkbox(&mut self.snap, tr("Snap to grid"));
                canvas::style_menu(ui, &mut self.diagram.style, false);
                ui.checkbox(&mut self.show_issues, tr("Show issues"));
            });
        });
    }

    fn canvas(&mut self, ui: &mut Ui) {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let canvas = response.rect;
        let painter = painter.with_clip_rect(canvas);
        let pal = Palette::for_ui(ui);
        self.last_canvas = canvas;
        if self.fit_request {
            self.fit(canvas);
            self.fit_request = false;
        }
        self.view.handle_navigation(ui, &response, canvas);
        let pointer = response.hover_pos().or(response.interact_pointer_pos());
        let wp = pointer.map(|p| self.view.to_world(canvas, p));

        self.interaction(ui, &response, canvas, pointer, wp);
        if response.hovered() && !ui.ctx().egui_wants_keyboard_input() {
            self.shortcuts(ui);
        }

        // Image export: render this frame without selection, hover and palette.
        let exporting = std::mem::take(&mut self.export_png);
        let saved = exporting.then(|| (self.selection.take(), std::mem::take(&mut self.group)));
        canvas::draw_background(&painter, canvas, &self.view, &pal);
        self.draw(
            &painter,
            canvas,
            &pal,
            if exporting { None } else { wp },
            if exporting { None } else { pointer },
        );
        if let Some((sel, group)) = saved {
            self.selection = sel;
            self.group = group;
            crate::io::request_png_export(
                ui.ctx(),
                canvas,
                format!(
                    "{}.png",
                    self.diagram
                        .name
                        .chars()
                        .map(|c| if c.is_alphanumeric() { c } else { '_' })
                        .collect::<String>()
                ),
            );
        }
        if let (Some(Drag::Rubber { start }), Some(p)) = (self.drag, pointer) {
            let r = Rect::from_two_pos(self.view.to_screen(canvas, start), p);
            painter.rect(
                r,
                2.0,
                pal.select.gamma_multiply(0.08),
                Stroke::new(1.0, pal.select),
                StrokeKind::Inside,
            );
        }
        self.palette_drop(ui, canvas, &pal);

        if self.diagram.nodes.is_empty() && self.diagram.pools.is_empty() {
            painter.text(
                canvas.center() - vec2(0.0, 12.0),
                egui::Align2::CENTER_CENTER,
                tr("Empty BPMN diagram"),
                crate::theme::semibold(17.0),
                pal.text,
            );
            painter.text(
                canvas.center() + vec2(0.0, 14.0),
                egui::Align2::CENTER_CENTER,
                tr("Pick an element in the palette and click on the canvas"),
                FontId::proportional(13.5),
                pal.muted,
            );
        }
        let max_h = canvas.height() - 40.0;
        if !exporting {
            self.toolbar_rect = canvas::floating(
                ui.ctx(),
                egui::Id::unique(("bpmn_toolbar", self.uid)),
                canvas.left_top() + vec2(14.0, 14.0),
                egui::Align2::LEFT_TOP,
                |ui| self.palette(ui, max_h),
            );
        }

        // Inline renaming
        if let Some((target, mut text, first)) = self.editing.take() {
            let (pos, width) = match target {
                Sel::Node(id) => match self.diagram.node(id) {
                    Some(n) => (Pos::new(n.pos.x - n.w.max(120.0) / 2.0, n.pos.y - 12.0), n.w.max(120.0)),
                    None => return,
                },
                Sel::Flow(id) => {
                    let pts: Vec<Pos2> = self
                        .diagram
                        .flow(id)
                        .map(|f| route(&self.diagram, f))
                        .unwrap_or_default()
                        .into_iter()
                        .map(to_pos2)
                        .collect();
                    let m = canvas::polyline_midpoint(&pts);
                    (Pos::new(m.x - 60.0, m.y - 12.0), 120.0)
                }
                Sel::Pool(id) => match self.diagram.pool(id) {
                    Some(p) => (Pos::new(p.pos.x, p.pos.y + 10.0), 160.0),
                    None => return,
                },
                Sel::Lane(pid, i) => match self.diagram.pool(pid) {
                    Some(p) if i < p.lanes.len() => {
                        let r = p.lane_rect(i);
                        (Pos::new(r.x, r.y + 10.0), 160.0)
                    }
                    _ => return,
                },
            };
            let sp = self.view.to_screen(canvas, pos);
            match canvas::inline_editor(
                ui.ctx(),
                egui::Id::unique(("bpmn_inline", self.uid)),
                sp,
                width * self.view.zoom,
                &mut text,
                first,
            ) {
                InlineOutcome::Editing => self.editing = Some((target, text, false)),
                InlineOutcome::Done => self.set_name(target, text),
                InlineOutcome::Cancelled => {}
            }
        }
    }

    fn set_name(&mut self, target: Sel, text: String) {
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

    fn name_of(&self, target: Sel) -> String {
        match target {
            Sel::Node(id) => self.diagram.node(id).map(|n| n.name.clone()),
            Sel::Flow(id) => self.diagram.flow(id).map(|f| f.name.clone()),
            Sel::Pool(id) => self.diagram.pool(id).map(|p| p.name.clone()),
            Sel::Lane(pid, i) => self.diagram.pool(pid).and_then(|p| p.lanes.get(i)).map(|l| l.name.clone()),
        }
        .unwrap_or_default()
    }

    fn shortcuts(&mut self, ui: &Ui) {
        let pressed = |k: Key| ui.input(|i| i.key_pressed(k) && !i.modifiers.command && !i.modifiers.ctrl);
        if pressed(Key::Delete) || pressed(Key::Backspace) {
            self.delete_selection();
        }
        if pressed(Key::Escape) {
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

    fn interaction(&mut self, ui: &Ui, response: &egui::Response, canvas: Rect, pointer: Option<Pos2>, wp: Option<Pos>) {
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
    fn create_at(&mut self, tool: Tool, w: Pos) {
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

    fn normalize_group(&mut self) {
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
    fn palette_drop(&mut self, ui: &Ui, canvas: Rect, pal: &Palette) {
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

    fn connect(&mut self, from: Id, to: Id) {
        match self.diagram.add_flow(from, to) {
            Ok(id) => {
                self.selection = Some(Sel::Flow(id));
                self.status.clear();
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    /// Inserts node `mid` in sequence flow `fid` (a → b becomes a → mid → b).
    fn split_flow(&mut self, fid: Id, mid: Id) {
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

    fn pad_action(&mut self, action: PadAction) {
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

    fn context_menu(&mut self, ui: &mut Ui) {
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

    fn draw(&self, painter: &Painter, canvas: Rect, pal: &Palette, wp: Option<Pos>, pointer: Option<Pos2>) {
        canvas::set_sketch(self.diagram.style.sketch);
        self.draw_inner(painter, canvas, pal, wp, pointer);
        canvas::set_sketch(false);
    }

    fn draw_inner(&self, painter: &Painter, canvas: Rect, pal: &Palette, wp: Option<Pos>, pointer: Option<Pos2>) {
        let sketch = self.diagram.style.sketch;
        let v = &self.view;
        let z = v.zoom;
        let hover = wp.and_then(|w| self.hit(w));
        let font_size = (12.0 * z).clamp(6.0, 26.0);

        // Pools & lanes
        for p in &self.diagram.pools {
            let r = v.rect_to_screen(canvas, p.rect());
            let sel = self.selection == Some(Sel::Pool(p.id));
            painter.rect_filled(r, 0.0, pal.fill.gamma_multiply(0.6));
            let header = Rect::from_min_size(r.min, vec2(POOL_HEADER * z, r.height()));
            painter.rect_filled(header, 0.0, pal.pool_header);
            vertical_text(painter, header.center(), &p.name, font_size * 1.05, pal.text);
            if p.lanes.len() > 1 {
                for i in 0..p.lanes.len() {
                    let lr = v.rect_to_screen(canvas, p.lane_rect(i));
                    let lh = Rect::from_min_size(lr.min, vec2(LANE_HEADER * z, lr.height()));
                    let lsel = self.selection == Some(Sel::Lane(p.id, i));
                    painter.rect_filled(
                        lh,
                        0.0,
                        if lsel {
                            pal.select.gamma_multiply(0.2)
                        } else {
                            pal.pool_header.gamma_multiply(0.7)
                        },
                    );
                    painter.line_segment([lh.right_top(), lh.right_bottom()], Stroke::new(1.0, pal.ink));
                    vertical_text(painter, lh.center(), &p.lanes[i].name, font_size, pal.text);
                    if i > 0 {
                        painter.line_segment([lr.left_top(), lr.right_top()], Stroke::new(1.0, pal.ink));
                    }
                }
            }
            painter.line_segment([header.right_top(), header.right_bottom()], Stroke::new(1.2, pal.ink));
            let stroke = if sel {
                Stroke::new(2.2, pal.select)
            } else {
                Stroke::new(1.4, pal.ink)
            };
            if sketch {
                canvas::sketch_rect(painter, r, stroke, p.id as u64);
            } else {
                painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            }
        }

        // Flows
        for f in &self.diagram.flows {
            let pts: Vec<Pos2> = route(&self.diagram, f).into_iter().map(|p| v.to_screen(canvas, p)).collect();
            if pts.len() < 2 {
                continue;
            }
            let sel = self.selection == Some(Sel::Flow(f.id));
            let hov = hover == Some(Sel::Flow(f.id));
            let color = if sel {
                pal.select
            } else if hov {
                pal.select.gamma_multiply(0.7)
            } else {
                pal.ink
            };
            let width = if sel { 2.2 } else { 1.4 } * z.max(0.6);
            let stroke = Stroke::new(width, color);
            let n = pts.len();
            let dir = pts[n - 1] - pts[n - 2];
            let head = 10.0 * z.max(0.5);
            match f.kind {
                FlowKind::Sequence => {
                    let mut body = pts.clone();
                    if sketch {
                        canvas::sketch_polyline(painter, &body, stroke, f.id as u64, false);
                        canvas::sketch_arrow_head(painter, pts[n - 1], dir, head, stroke, f.id as u64);
                    } else {
                        body[n - 1] = pts[n - 1] - dir.normalized() * head * 0.8;
                        canvas::polyline(painter, &body, stroke);
                        canvas::arrow_head(painter, pts[n - 1], dir, head, color, true);
                    }
                    if f.is_default {
                        let d0 = (pts[1] - pts[0]).normalized();
                        let c = pts[0] + d0 * 10.0 * z;
                        let nrm = egui::vec2(-d0.y, d0.x);
                        painter.line_segment([c + (nrm - d0) * 5.0 * z, c - (nrm - d0) * 5.0 * z], stroke);
                    }
                }
                FlowKind::Message => {
                    canvas::dashed(painter, &pts, stroke, 7.0 * z, 4.0 * z);
                    painter.circle(pts[0], 3.5 * z, pal.fill, stroke);
                    let tip = pts[n - 1];
                    let d = dir.normalized();
                    let nrm = egui::vec2(-d.y, d.x);
                    let base = tip - d * head;
                    painter.add(Shape::convex_polygon(
                        vec![tip, base + nrm * head * 0.45, base - nrm * head * 0.45],
                        pal.fill,
                        stroke,
                    ));
                }
                FlowKind::Association => {
                    canvas::dashed(painter, &pts, Stroke::new(width, color.gamma_multiply(0.8)), 2.0 * z, 3.0 * z);
                    let to_data = self
                        .diagram
                        .node(f.to)
                        .is_some_and(|n| matches!(n.kind, NodeKind::DataObject | NodeKind::DataStore));
                    let from_data = self
                        .diagram
                        .node(f.from)
                        .is_some_and(|n| matches!(n.kind, NodeKind::DataObject | NodeKind::DataStore));
                    if to_data || from_data {
                        canvas::arrow_head(painter, pts[n - 1], dir, head * 0.8, color, false);
                    }
                }
            }
            if !f.name.is_empty() {
                let mid = canvas::polyline_midpoint(&pts);
                canvas::label_with_halo(painter, mid + vec2(0.0, -10.0 * z), &f.name, font_size * 0.95, pal);
            }
        }

        // Nodes
        let issues = if self.show_issues { self.diagram.validate() } else { Vec::new() };
        for n in &self.diagram.nodes {
            let r = v.rect_to_screen(canvas, n.rect());
            let sel = self.selection == Some(Sel::Node(n.id)) || self.group.contains(&n.id);
            let hov = hover == Some(Sel::Node(n.id)) || self.connect_pending == Some(n.id);
            if let Some(sev) = issues
                .iter()
                .filter(|i| i.node == Some(n.id))
                .map(|i| i.severity)
                .max_by_key(|s| *s == Severity::Error)
            {
                let c = if sev == Severity::Error { pal.bad } else { pal.warn };
                painter.rect_stroke(r.expand(6.0 * z), CornerRadius::same(6), Stroke::new(2.0, c), StrokeKind::Outside);
            }
            paint_node(painter, r, n.kind, &n.name, z, pal, None);
            if sel || hov {
                let c = if sel { pal.select } else { pal.select.gamma_multiply(0.6) };
                painter.rect_stroke(r.expand(4.0), CornerRadius::same(4), Stroke::new(1.5, c), StrokeKind::Outside);
            }
        }

        // Rubber band for connections
        let from = match self.drag {
            Some(Drag::Connect { from }) => Some(from),
            _ => self.connect_pending,
        };
        if let (Some(from), Some(p)) = (from, pointer)
            && let Some(n) = self.diagram.node(from)
        {
            canvas::dashed(painter, &[v.to_screen(canvas, n.pos), p], Stroke::new(1.5, pal.select), 6.0, 4.0);
        }

        // Resize handle
        if let Some((h, _)) = self.resize_handle(canvas) {
            painter.rect(h, CornerRadius::same(2), pal.fill, Stroke::new(1.5, pal.select), StrokeKind::Middle);
        }

        // Context pad (card with shadow behind the buttons)
        let pad = self.pad_buttons(canvas);
        if let Some(bg) = pad.iter().map(|(r, _, _)| *r).reduce(|a, b| a.union(b)) {
            let bg = bg.expand(5.0);
            canvas::shadow_rect(painter, bg, 9.0, pal);
            painter.rect(
                bg,
                CornerRadius::same(9),
                pal.fill,
                Stroke::new(1.0, pal.border),
                StrokeKind::Inside,
            );
        }
        for (r, action, _) in pad {
            let hovered = pointer.is_some_and(|p| r.contains(p));
            if hovered {
                painter.rect_filled(r, CornerRadius::same(6), pal.select.gamma_multiply(0.18));
            }
            let inner = r.shrink(5.0);
            match action {
                PadAction::Append(kind) => paint_node(painter, inner, kind, "", 0.35, pal, Some(pal.ink)),
                PadAction::Connect => {
                    let a = inner.left_bottom();
                    let b = inner.right_top();
                    painter.line_segment([a, b - (b - a).normalized() * 4.0], Stroke::new(1.3, pal.ink));
                    canvas::arrow_head(painter, b, b - a, 6.0, pal.ink, true);
                }
                PadAction::Delete => {
                    painter.text(r.center(), egui::Align2::CENTER_CENTER, "🗑", FontId::proportional(13.0), pal.bad);
                }
            }
        }
        if let Some(p) = pointer
            && let Some((_, _, tip)) = self.pad_buttons(canvas).into_iter().find(|(r, _, _)| r.contains(p))
        {
            canvas::badge(painter, p + vec2(0.0, -20.0), tip, 11.5, pal.text, pal);
        }
    }

    // ------------------------------------------------------------------
    // Side panel

    fn side_panel(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        let n_issues = self.diagram.validate().len();
        ui.horizontal_wrapped(|ui| {
            let vlabel = if n_issues == 0 {
                tr("Validation ✔").to_string()
            } else {
                trf("Validation ({})", &[&n_issues])
            };
            for (tab, label) in [
                (SideTab::Properties, tr("Properties")),
                (SideTab::Validation, vlabel.as_str()),
                (SideTab::Evaluation, tr("Evaluation")),
            ] {
                ui.selectable_value(&mut self.tab, tab, label);
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.tab {
                SideTab::Properties => self.properties(ui),
                SideTab::Validation => self.validation(ui),
                SideTab::Evaluation => self.evaluation(ui),
            });
    }

    fn evaluation(&mut self, ui: &mut Ui) {
        use op_core::evaluation;
        ui.heading(tr("Process evaluation"));
        ui.label(tr(
            "Equiprobable choices, no waiting between activities. Use the same time and cost units throughout.",
        ));
        ui.label(tr("Values default to zero. Sub-processes are treated as single activities."));
        let activities: Vec<_> = evaluation::activities(&self.diagram).collect();
        crate::theme::section_title(ui, tr("Activity values"));
        egui::Grid::new("activity_metrics").num_columns(3).striped(true).show(ui, |ui| {
            ui.strong(tr("Activity"));
            ui.strong(tr("Cost CAi"));
            ui.strong(tr("Duration TAi"));
            ui.end_row();
            for (id, symbol) in &activities {
                let n = self.diagram.node_mut(*id).unwrap();
                ui.label(symbol).on_hover_text(&n.name);
                ui.add(egui::DragValue::new(&mut n.cost).range(0.0..=f64::MAX).speed(0.1));
                ui.add(egui::DragValue::new(&mut n.duration).range(0.0..=f64::MAX).speed(0.1));
                ui.end_row();
            }
        });
        for (id, symbol) in &activities {
            let n = self.diagram.node(*id).unwrap();
            if n.name == *symbol {
                continue;
            }
            ui.small(format!(
                "{symbol} = {}",
                if n.name.is_empty() { n.kind.label() } else { n.name.clone() }
            ));
        }
        crate::theme::section_title(ui, tr("Equations"));
        ui.label("C(σ) = Σi ni(σ) × CAi");
        ui.small(tr("ni(σ) counts the occurrences of activity Ai in trace σ."));
        ui.label(tr("Sequence: T = T1 + … + Tk"));
        ui.label(tr("Parallel (AND): T = max(T1, …, Tk)"));
        ui.label(tr("Choice (XOR): E[T] = (T1 + … + Tk) / k"));
        ui.small(tr(
            "For choices inside parallel branches, compute each scenario's maximum before averaging.",
        ));
        let report = match evaluation::evaluate(&self.diagram) {
            Ok(r) => r,
            Err(message) => {
                ui.colored_label(Palette::for_ui(ui).bad, message);
                return;
            }
        };
        crate::theme::section_title(ui, tr("Mean execution time"));
        ui.add(egui::Label::new(&report.duration_formula).wrap().selectable(true));
        ui.strong(format!("T(P) = {}", report.mean_duration));
        let symbols: std::collections::HashMap<_, _> = activities.into_iter().collect();
        let mut writeup = format!("{}\nT(P) = {}\n", report.duration_formula, report.mean_duration);
        crate::theme::section_title(ui, tr("Traces and costs"));
        ui.small(tr("A scenario fixes the XOR choices. All its parallel interleavings have the same cost and elapsed time; their count does not change its probability."));
        for (i, s) in report.scenarios.iter().enumerate() {
            let title = trf("Scenario {} · probability 1/{}", &[&(i + 1), &s.denominator]);
            let cost = format!("C(σ) = {} = {}", s.cost_formula, s.cost);
            let duration = format!("T = {} = {}", s.duration_formula, s.duration);
            writeup.push_str(&format!("\n{title}\n{cost}\n{duration}\n"));
            let traces: Vec<_> = s
                .traces
                .iter()
                .map(|t| {
                    if t.is_empty() {
                        "ε".into()
                    } else {
                        t.iter().map(|id| symbols[id].as_str()).collect::<Vec<_>>().join(" → ")
                    }
                })
                .collect();
            for trace in &traces {
                writeup.push_str(trace);
                writeup.push('\n');
            }
            if s.traces_truncated {
                writeup.push_str(tr("Trace display limit reached; cost and time calculations remain exact."));
                writeup.push('\n');
            }
            egui::CollapsingHeader::new(title)
                .id_salt(("scenario", i))
                .default_open(report.scenarios.len() <= 4)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(cost).wrap().selectable(true));
                    ui.add(egui::Label::new(duration).wrap().selectable(true));
                    for trace in &traces {
                        ui.add(egui::Label::new(trace).wrap().selectable(true));
                    }
                    if s.traces_truncated {
                        ui.label(tr("Trace display limit reached; cost and time calculations remain exact."));
                    }
                });
        }
        if ui.button(tr("Copy equations and traces")).clicked() {
            ui.ctx().copy_text(writeup);
        }
    }

    fn group_panel(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(trf("{} elements selected", &[&self.group.len()])).font(crate::theme::semibold(16.0)));
        ui.label(RichText::new(tr("Drag one of them to move the whole selection")).color(ui.visuals().weak_text_color()));
        crate::theme::section_title(ui, tr("Arrange"));
        let pos: Vec<(Id, Pos)> = self
            .group
            .iter()
            .filter_map(|&id| self.diagram.node(id).map(|n| (id, n.pos)))
            .collect();
        let k = pos.len().max(1) as f32;
        let (mx, my) = (
            pos.iter().map(|p| p.1.x).sum::<f32>() / k,
            pos.iter().map(|p| p.1.y).sum::<f32>() / k,
        );
        let set = |d: &mut BpmnDiagram, id: Id, p: Pos| {
            if let Some(n) = d.node_mut(id) {
                n.pos = p;
            }
        };
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr("Align horizontally"))
                .on_hover_text(tr("Same vertical position (centres)"))
                .clicked()
            {
                for (id, p) in &pos {
                    set(&mut self.diagram, *id, Pos::new(p.x, my));
                }
            }
            if ui
                .button(tr("Align vertically"))
                .on_hover_text(tr("Same horizontal position (centres)"))
                .clicked()
            {
                for (id, p) in &pos {
                    set(&mut self.diagram, *id, Pos::new(mx, p.y));
                }
            }
            if ui.button(tr("Distribute horizontally")).clicked() && pos.len() > 2 {
                let mut v = pos.clone();
                v.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
                let (x0, x1) = (v[0].1.x, v[v.len() - 1].1.x);
                for (i, (id, p)) in v.iter().enumerate() {
                    set(
                        &mut self.diagram,
                        *id,
                        Pos::new(x0 + (x1 - x0) * i as f32 / (v.len() - 1) as f32, p.y),
                    );
                }
            }
            if ui.button(tr("Distribute vertically")).clicked() && pos.len() > 2 {
                let mut v = pos.clone();
                v.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
                let (y0, y1) = (v[0].1.y, v[v.len() - 1].1.y);
                for (i, (id, p)) in v.iter().enumerate() {
                    set(
                        &mut self.diagram,
                        *id,
                        Pos::new(p.x, y0 + (y1 - y0) * i as f32 / (v.len() - 1) as f32),
                    );
                }
            }
        });
        ui.add_space(10.0);
        if ui
            .button(RichText::new(tr("Delete selection")).color(Palette::for_ui(ui).bad))
            .clicked()
        {
            self.delete_selection();
        }
    }

    fn properties(&mut self, ui: &mut Ui) {
        if self.group.len() > 1 {
            self.group_panel(ui);
            return;
        }
        match self.selection {
            Some(Sel::Node(id)) => {
                let lane = self.diagram.lane_of(id).and_then(|(pid, lid)| {
                    let p = self.diagram.pool(pid)?;
                    let l = p.lanes.iter().find(|l| l.id == lid)?;
                    Some(if l.name.is_empty() {
                        p.name.clone()
                    } else {
                        format!("{} / {}", p.name, l.name)
                    })
                });
                let Some(n) = self.diagram.node_mut(id) else { return };
                ui.heading(n.kind.label());
                egui::Grid::new("np").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.add(egui::TextEdit::multiline(&mut n.name).desired_rows(2).desired_width(f32::INFINITY))
                        .labelled_by(lbl.id);
                    ui.end_row();
                    match &mut n.kind {
                        NodeKind::StartEvent(t) => {
                            ui.label(tr("Trigger"));
                            trigger_combo(
                                ui,
                                t,
                                &[
                                    EventTrigger::None,
                                    EventTrigger::Message,
                                    EventTrigger::Timer,
                                    EventTrigger::Signal,
                                    EventTrigger::Conditional,
                                ],
                            );
                            ui.end_row();
                        }
                        NodeKind::EndEvent(t) => {
                            ui.label(tr("Result"));
                            trigger_combo(
                                ui,
                                t,
                                &[
                                    EventTrigger::None,
                                    EventTrigger::Message,
                                    EventTrigger::Error,
                                    EventTrigger::Signal,
                                    EventTrigger::Terminate,
                                ],
                            );
                            ui.end_row();
                        }
                        NodeKind::IntermediateEvent { trigger, throwing } => {
                            ui.label(tr("Trigger"));
                            trigger_combo(
                                ui,
                                trigger,
                                &[
                                    EventTrigger::None,
                                    EventTrigger::Message,
                                    EventTrigger::Timer,
                                    EventTrigger::Signal,
                                    EventTrigger::Conditional,
                                    EventTrigger::Error,
                                ],
                            );
                            ui.end_row();
                            ui.label(tr("Throwing"));
                            ui.checkbox(throwing, "");
                            ui.end_row();
                        }
                        NodeKind::Task(t) => {
                            ui.label(tr("Task type"));
                            egui::ComboBox::from_id_salt("tt").selected_text(t.label()).show_ui(ui, |ui| {
                                for x in TaskType::ALL {
                                    ui.selectable_value(t, x, x.label());
                                }
                            });
                            ui.end_row();
                        }
                        NodeKind::Gateway(g) => {
                            ui.label(tr("Gateway type"));
                            egui::ComboBox::from_id_salt("gt").selected_text(g.label()).show_ui(ui, |ui| {
                                for x in GatewayType::ALL {
                                    ui.selectable_value(g, x, x.label());
                                }
                            });
                            ui.end_row();
                        }
                        _ => {}
                    }
                    if matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess | NodeKind::TextAnnotation) {
                        ui.label(tr("Size"));
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut n.w).range(40.0..=600.0).prefix("w "));
                            ui.add(egui::DragValue::new(&mut n.h).range(30.0..=400.0).prefix("h "));
                        });
                        ui.end_row();
                    }
                    if let Some(l) = &lane {
                        ui.label(tr("Lane"));
                        ui.label(l);
                        ui.end_row();
                    }
                    if matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess) {
                        ui.label(tr("Cost CAi"));
                        ui.add(egui::DragValue::new(&mut n.cost).range(0.0..=f64::MAX).speed(0.1));
                        ui.end_row();
                        ui.label(tr("Duration TAi"));
                        ui.add(egui::DragValue::new(&mut n.duration).range(0.0..=f64::MAX).speed(0.1));
                        ui.end_row();
                    }
                });
                if matches!(n.kind, NodeKind::Task(_)) && ui.button(tr("Convert to sub-process")).clicked() {
                    n.kind = NodeKind::SubProcess;
                }
                if n.kind == NodeKind::SubProcess && ui.button(tr("Convert to task")).clicked() {
                    n.kind = NodeKind::Task(TaskType::Abstract);
                }
                ui.add_space(6.0);
                let lbl = ui.label(tr("Documentation"));
                ui.add(
                    egui::TextEdit::multiline(&mut n.documentation)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(lbl.id);
            }
            Some(Sel::Flow(id)) => {
                let src_kind = self.diagram.flow(id).and_then(|f| self.diagram.node(f.from)).map(|n| n.kind);
                let ends = self.diagram.flow(id).map(|f| {
                    let name = |i| {
                        self.diagram
                            .node(i)
                            .map(|n| if n.name.is_empty() { n.kind.label() } else { n.name.clone() })
                            .unwrap_or_default()
                    };
                    (name(f.from), name(f.to))
                });
                let Some(f) = self.diagram.flow_mut(id) else { return };
                ui.heading(match f.kind {
                    FlowKind::Sequence => tr("Sequence flow"),
                    FlowKind::Message => tr("Message flow"),
                    FlowKind::Association => tr("Association"),
                });
                if let Some((a, b)) = ends {
                    ui.label(format!("{a} → {b}"));
                }
                ui.horizontal(|ui| {
                    let lbl = ui.label(if matches!(src_kind, Some(NodeKind::Gateway(_))) {
                        tr("Condition")
                    } else {
                        tr("Label")
                    });
                    ui.text_edit_singleline(&mut f.name).labelled_by(lbl.id);
                });
                if matches!(src_kind, Some(NodeKind::Gateway(GatewayType::Exclusive | GatewayType::Inclusive))) {
                    ui.checkbox(&mut f.is_default, tr("Default flow"));
                }
                if f.kind == FlowKind::Sequence && ui.button(tr("Reverse direction")).clicked() {
                    std::mem::swap(&mut f.from, &mut f.to);
                }
            }
            Some(Sel::Pool(id)) => {
                let Some(p) = self.diagram.pool_mut(id) else { return };
                ui.heading(tr("Pool"));
                egui::Grid::new("pool").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut p.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("Width"));
                    ui.add(egui::DragValue::new(&mut p.width).range(200.0..=5000.0));
                    ui.end_row();
                });
                ui.add_space(6.0);
                crate::theme::section_title(ui, tr("Lanes"));
                let mut remove = None;
                let many = p.lanes.len() > 1;
                for (i, l) in p.lanes.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let lbl = ui.label(format!("{}.", i + 1));
                        ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(120.0))
                            .labelled_by(lbl.id);
                        ui.add(egui::DragValue::new(&mut l.height).range(60.0..=2000.0).prefix("h "));
                        if many && ui.small_button("✖").on_hover_text(tr("Remove lane")).clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    p.lanes.remove(i);
                }
                if ui.button(tr("Add lane")).clicked() {
                    self.diagram.add_lane(id);
                }
            }
            Some(Sel::Lane(pid, i)) => {
                let Some(l) = self.diagram.pool_mut(pid).and_then(|p| p.lanes.get_mut(i)) else {
                    return;
                };
                ui.heading(tr("Lane"));
                egui::Grid::new("lane").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut l.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("Height"));
                    ui.add(egui::DragValue::new(&mut l.height).range(60.0..=2000.0));
                    ui.end_row();
                });
            }
            None => {
                ui.heading(tr("Process"));
                ui.horizontal(|ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut self.diagram.name).labelled_by(lbl.id);
                });
                ui.add_space(8.0);
                if ui
                    .button(tr("⇄ Convert to Petri net"))
                    .on_hover_text(tr("Translate this process into a workflow net and analyse it"))
                    .clicked()
                {
                    self.action = Some(BpmnAction::ConvertToPetri);
                }
                ui.add_space(8.0);
                crate::theme::section_title(ui, tr("Tips"));
                ui.label(tr(
                    "• Pick an element in the palette, then click on the canvas. Dropping it on a flow inserts it.",
                ));
                ui.label(tr("• Select an element to get the quick-append pad (task, gateway, event…)."));
                ui.label(tr("• Connect (C): drag from an element to another."));
                ui.label(tr("• Double-click or F2 to rename; right-click for more actions."));
                ui.label(tr("• Pools: drag by the title band; resize with the bottom-right handle."));
                ui.label(tr("• Wheel = scroll, Ctrl+wheel = zoom, F = zoom to fit."));
            }
        }
    }

    fn validation(&mut self, ui: &mut Ui) {
        let pal = Palette::for_ui(ui);
        let issues = self.diagram.validate();
        ui.checkbox(&mut self.show_issues, tr("Highlight on the diagram"));
        ui.add_space(4.0);
        if issues.is_empty() {
            ui.label(RichText::new(tr("✔ No problem found")).color(pal.ok));
            return;
        }
        for issue in issues {
            let (sym, color) = match issue.severity {
                Severity::Error => ("✖", pal.bad),
                Severity::Warning => ("⚠", pal.warn),
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(sym).color(color));
                let r = ui.add(egui::Label::new(&issue.message).sense(Sense::click()));
                if r.clicked()
                    && let Some(id) = issue.node
                {
                    self.selection = Some(Sel::Node(id));
                }
            });
        }
    }
}

// ----------------------------------------------------------------------
// Shapes

fn hit_shape(n: &Node, w: Pos) -> bool {
    let r = n.rect();
    match n.kind {
        k if k.is_event() => n.pos.dist(w) <= n.w / 2.0 + 3.0,
        NodeKind::Gateway(_) => (w.x - n.pos.x).abs() / (n.w / 2.0 + 3.0) + (w.y - n.pos.y).abs() / (n.h / 2.0 + 3.0) <= 1.0,
        _ => r.contains(w),
    }
}

fn trigger_combo(ui: &mut Ui, t: &mut EventTrigger, options: &[EventTrigger]) {
    egui::ComboBox::from_id_salt("trig").selected_text(t.label()).show_ui(ui, |ui| {
        for &x in options {
            ui.selectable_value(t, x, x.label());
        }
    });
}

fn vertical_text(painter: &Painter, center: Pos2, text: &str, size: f32, color: Color32) {
    if text.is_empty() {
        return;
    }
    let galley = painter.layout_no_wrap(text.to_owned(), FontId::proportional(size), color);
    let s = galley.size();
    // Rotated by -90°: the text runs bottom → top.
    let pos = center + vec2(-s.y / 2.0, s.x / 2.0);
    painter.add(egui::epaint::TextShape::new(pos, galley, color).with_angle(-std::f32::consts::FRAC_PI_2));
}

/// Paints a BPMN node into screen rect `r`. With `mono`, everything uses that color (palette icons).
fn kind_seed(kind: NodeKind) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(&kind, &mut h);
    std::hash::Hasher::finish(&h)
}

pub fn paint_node(painter: &Painter, r: Rect, kind: NodeKind, name: &str, z: f32, pal: &Palette, mono: Option<Color32>) {
    let zz = z.max(0.35);
    let col = |c: Color32| mono.unwrap_or(c);
    let fill = if mono.is_some() { Color32::TRANSPARENT } else { pal.fill };
    let c = r.center();
    if mono.is_none() && !canvas::sketch() {
        match kind {
            k if k.is_event() => {
                canvas::shadow_circle(painter, c, r.width() / 2.0, pal);
                painter.circle_filled(c, r.width() / 2.0, pal.fill);
            }
            NodeKind::Task(_) | NodeKind::SubProcess => canvas::shadow_rect(painter, r, 9.0 * zz, pal),
            NodeKind::Gateway(_) => {
                for (k, a) in [(1.5f32, 1.0f32), (3.5, 0.45)] {
                    let o = vec2(0.0, k * 0.8);
                    let e = k * 0.7;
                    let pts = vec![
                        pos2(c.x, r.min.y - e) + o,
                        pos2(r.max.x + e, c.y) + o,
                        pos2(c.x, r.max.y + e) + o,
                        pos2(r.min.x - e, c.y) + o,
                    ];
                    painter.add(Shape::convex_polygon(pts, pal.shadow.gamma_multiply(a * 0.6), Stroke::NONE));
                }
                let base = vec![pos2(c.x, r.min.y), pos2(r.max.x, c.y), pos2(c.x, r.max.y), pos2(r.min.x, c.y)];
                painter.add(Shape::convex_polygon(base, pal.fill, Stroke::NONE));
            }
            _ => {}
        }
    }
    // Sketch mode: rough outlines, seeded per element so they don't flicker.
    let sk = mono.is_none() && canvas::sketch();
    let seed = name.bytes().fold(kind_seed(kind), |h, b| h.wrapping_mul(31).wrapping_add(b as u64));
    let circle = |p: &Painter, c: Pos2, rad: f32, fill: Color32, stroke: Stroke| {
        if sk {
            p.circle_filled(c, rad, fill);
            canvas::sketch_ellipse(p, c, vec2(rad, rad), stroke, seed);
        } else {
            p.circle(c, rad, fill, stroke);
        }
    };
    let rrect = |p: &Painter, r: Rect, radius: f32, fill: Color32, stroke: Stroke| {
        if sk {
            p.rect_filled(r, radius, fill);
            canvas::sketch_rect(p, r, stroke, seed);
        } else {
            p.rect(r, CornerRadius::same(radius.min(255.0) as u8), fill, stroke, StrokeKind::Middle);
        }
    };
    let label_size = (12.0 * z).clamp(6.0, 26.0);
    let label_below = |painter: &Painter| {
        if mono.is_none() {
            canvas::text_below(
                painter,
                pos2(c.x, r.max.y + 4.0 * z),
                130.0 * z,
                name,
                label_size,
                pal.text,
                Some(pal.bg),
            );
        }
    };
    match kind {
        NodeKind::StartEvent(t) => {
            let rad = r.width() / 2.0;
            circle(
                painter,
                c,
                rad,
                if mono.is_some() { fill } else { pal.start.gamma_multiply(0.12) },
                Stroke::new(1.6 * zz, col(pal.start)),
            );
            paint_trigger(painter, c, rad * 0.6, t, col(pal.start), false, fill);
            label_below(painter);
        }
        NodeKind::IntermediateEvent { trigger, throwing } => {
            let rad = r.width() / 2.0;
            let color = col(pal.warn);
            circle(
                painter,
                c,
                rad,
                if mono.is_some() { fill } else { pal.warn.gamma_multiply(0.10) },
                Stroke::new(1.4 * zz, color),
            );
            painter.circle_stroke(c, rad - 3.0 * zz, Stroke::new(1.2 * zz, color));
            paint_trigger(painter, c, rad * 0.55, trigger, color, throwing, fill);
            label_below(painter);
        }
        NodeKind::EndEvent(t) => {
            let rad = r.width() / 2.0;
            circle(
                painter,
                c,
                rad - 1.5 * zz,
                if mono.is_some() { fill } else { pal.end.gamma_multiply(0.12) },
                Stroke::new(3.5 * zz, col(pal.end)),
            );
            paint_trigger(painter, c, rad * 0.55, t, col(pal.end), true, fill);
            label_below(painter);
        }
        NodeKind::Task(t) => {
            rrect(
                painter,
                r,
                9.0 * zz,
                if mono.is_some() { fill } else { pal.task_fill },
                Stroke::new(1.4 * zz, col(pal.ink)),
            );
            let icon = match t {
                TaskType::Abstract => "",
                TaskType::User => "👤",
                TaskType::Manual => "✋",
                TaskType::Service => "⚙",
                TaskType::Script => "📜",
                TaskType::Send => "✉",
                TaskType::Receive => "📨",
                TaskType::BusinessRule => "▦",
            };
            if !icon.is_empty() {
                painter.text(
                    r.min + vec2(5.0, 4.0) * zz,
                    egui::Align2::LEFT_TOP,
                    icon,
                    FontId::proportional(13.0 * zz),
                    col(pal.muted),
                );
            }
            if mono.is_none() {
                canvas::text_centered(painter, c, r.width() - 12.0 * z, name, label_size, pal.text);
            }
        }
        NodeKind::SubProcess => {
            rrect(
                painter,
                r,
                9.0 * zz,
                if mono.is_some() { fill } else { pal.task_fill },
                Stroke::new(1.4 * zz, col(pal.ink)),
            );
            let m = Rect::from_center_size(pos2(c.x, r.max.y - 9.0 * zz), vec2(12.0, 12.0) * zz);
            painter.rect_stroke(m, 0.0, Stroke::new(1.0 * zz, col(pal.ink)), StrokeKind::Middle);
            painter.line_segment(
                [m.center() - vec2(3.5, 0.0) * zz, m.center() + vec2(3.5, 0.0) * zz],
                Stroke::new(1.0 * zz, col(pal.ink)),
            );
            painter.line_segment(
                [m.center() - vec2(0.0, 3.5) * zz, m.center() + vec2(0.0, 3.5) * zz],
                Stroke::new(1.0 * zz, col(pal.ink)),
            );
            if mono.is_none() {
                canvas::text_centered(painter, c - vec2(0.0, 6.0 * z), r.width() - 12.0 * z, name, label_size, pal.text);
            }
        }
        NodeKind::Gateway(g) => {
            let pts = vec![pos2(c.x, r.min.y), pos2(r.max.x, c.y), pos2(c.x, r.max.y), pos2(r.min.x, c.y)];
            let color = col(pal.gateway);
            if sk {
                painter.add(Shape::convex_polygon(pts.clone(), pal.gateway.gamma_multiply(0.12), Stroke::NONE));
                canvas::sketch_polyline(painter, &pts, Stroke::new(1.6 * zz, color), seed, true);
            } else {
                painter.add(Shape::convex_polygon(
                    pts,
                    if mono.is_some() { fill } else { pal.gateway.gamma_multiply(0.12) },
                    Stroke::new(1.6 * zz, color),
                ));
            }
            let k = r.width() * 0.2;
            let s = Stroke::new(3.0 * zz, col(pal.ink));
            match g {
                GatewayType::Exclusive => {
                    painter.line_segment([c - vec2(k, k), c + vec2(k, k)], s);
                    painter.line_segment([c + vec2(-k, k), c + vec2(k, -k)], s);
                }
                GatewayType::Parallel => {
                    painter.line_segment([c - vec2(k * 1.2, 0.0), c + vec2(k * 1.2, 0.0)], s);
                    painter.line_segment([c - vec2(0.0, k * 1.2), c + vec2(0.0, k * 1.2)], s);
                }
                GatewayType::Inclusive => {
                    painter.circle_stroke(c, k * 1.1, Stroke::new(2.5 * zz, col(pal.ink)));
                }
                GatewayType::EventBased => {
                    let t = Stroke::new(1.0 * zz, col(pal.ink));
                    painter.circle_stroke(c, k * 1.3, t);
                    painter.circle_stroke(c, k * 1.05, t);
                    let pent: Vec<Pos2> = (0..5)
                        .map(|i| {
                            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / 5.0;
                            c + vec2(a.cos(), a.sin()) * k * 0.7
                        })
                        .collect();
                    painter.add(Shape::closed_line(pent, t));
                }
                GatewayType::Complex => {
                    for i in 0..4 {
                        let a = i as f32 * std::f32::consts::FRAC_PI_4;
                        let d = vec2(a.cos(), a.sin()) * k * 1.2;
                        painter.line_segment([c - d, c + d], s);
                    }
                }
            }
            label_below(painter);
        }
        NodeKind::DataObject => {
            let f = r.width() * 0.3;
            let pts = vec![
                r.min,
                pos2(r.max.x - f, r.min.y),
                pos2(r.max.x, r.min.y + f),
                r.max,
                pos2(r.min.x, r.max.y),
            ];
            painter.add(Shape::convex_polygon(pts, fill, Stroke::new(1.3 * zz, col(pal.ink))));
            painter.add(Shape::line(
                vec![
                    pos2(r.max.x - f, r.min.y),
                    pos2(r.max.x - f, r.min.y + f),
                    pos2(r.max.x, r.min.y + f),
                ],
                Stroke::new(1.0 * zz, col(pal.ink)),
            ));
            label_below(painter);
        }
        NodeKind::DataStore => {
            let s = Stroke::new(1.3 * zz, col(pal.ink));
            let ry = r.height() * 0.12;
            let body = Rect::from_min_max(pos2(r.min.x, r.min.y + ry), pos2(r.max.x, r.max.y - ry));
            painter.rect_filled(body, 0.0, fill);
            painter.add(Shape::ellipse_filled(pos2(c.x, r.max.y - ry), vec2(r.width() / 2.0, ry), fill));
            painter.add(Shape::ellipse_stroke(pos2(c.x, r.max.y - ry), vec2(r.width() / 2.0, ry), s));
            painter.rect_filled(body.shrink2(vec2(0.5, 0.0)), 0.0, fill);
            painter.line_segment([body.left_top(), body.left_bottom()], s);
            painter.line_segment([body.right_top(), body.right_bottom()], s);
            painter.add(Shape::ellipse_filled(pos2(c.x, r.min.y + ry), vec2(r.width() / 2.0, ry), fill));
            painter.add(Shape::ellipse_stroke(pos2(c.x, r.min.y + ry), vec2(r.width() / 2.0, ry), s));
            label_below(painter);
        }
        NodeKind::TextAnnotation => {
            let s = Stroke::new(1.3 * zz, col(pal.muted));
            let t = 10.0 * zz;
            painter.add(Shape::line(
                vec![r.min + vec2(t, 0.0), r.min, r.left_bottom(), r.left_bottom() + vec2(t, 0.0)],
                s,
            ));
            if mono.is_none() {
                canvas::text_centered(painter, c, r.width() - 10.0 * z, name, label_size, pal.text);
            } else {
                for i in 0..3 {
                    let y = r.min.y + r.height() * (0.3 + 0.2 * i as f32);
                    painter.line_segment(
                        [pos2(r.min.x + t * 0.6, y), pos2(r.max.x - t * 0.3, y)],
                        Stroke::new(1.0, col(pal.muted)),
                    );
                }
            }
        }
    }
}

fn paint_trigger(painter: &Painter, c: Pos2, s: f32, t: EventTrigger, color: Color32, filled: bool, bg: Color32) {
    let stroke = Stroke::new((s * 0.12).clamp(0.8, 2.0), color);
    let fill = if filled { color } else { Color32::TRANSPARENT };
    match t {
        EventTrigger::None => {}
        EventTrigger::Message => {
            let r = Rect::from_center_size(c, vec2(s * 1.4, s * 0.95));
            painter.rect(r, 0.0, fill, stroke, StrokeKind::Middle);
            let flap = if filled { Stroke::new(stroke.width, bg) } else { stroke };
            painter.add(Shape::line(vec![r.min, pos2(c.x, c.y + s * 0.05), pos2(r.max.x, r.min.y)], flap));
        }
        EventTrigger::Timer => {
            painter.circle_stroke(c, s * 0.8, stroke);
            painter.line_segment([c, c + vec2(0.0, -s * 0.6)], stroke);
            painter.line_segment([c, c + vec2(s * 0.45, 0.0)], stroke);
        }
        EventTrigger::Error => {
            let pts = vec![
                c + vec2(-s * 0.7, s * 0.7),
                c + vec2(-s * 0.3, -s * 0.6),
                c + vec2(s * 0.15, s * 0.15),
                c + vec2(s * 0.7, -s * 0.7),
                c + vec2(s * 0.3, s * 0.6),
                c + vec2(-s * 0.15, -s * 0.15),
            ];
            if filled {
                painter.add(Shape::closed_line(pts, Stroke::new(stroke.width * 1.5, color)));
            } else {
                painter.add(Shape::closed_line(pts, stroke));
            }
        }
        EventTrigger::Signal => {
            let pts = vec![c + vec2(0.0, -s * 0.8), c + vec2(s * 0.75, s * 0.55), c + vec2(-s * 0.75, s * 0.55)];
            painter.add(Shape::convex_polygon(pts, fill, stroke));
        }
        EventTrigger::Conditional => {
            let r = Rect::from_center_size(c, vec2(s * 1.0, s * 1.3));
            painter.rect_stroke(r, 0.0, stroke, StrokeKind::Middle);
            for i in 0..4 {
                let y = r.min.y + r.height() * (0.2 + 0.2 * i as f32);
                painter.line_segment([pos2(r.min.x + s * 0.15, y), pos2(r.max.x - s * 0.15, y)], stroke);
            }
        }
        EventTrigger::Terminate => {
            painter.circle_filled(c, s * 0.75, color);
        }
    }
}

fn paint_tool_icon(p: &Painter, r: Rect, c: Color32, tool: Tool) {
    let pal = Palette::new(false);
    match tool {
        Tool::Select => {
            let ctr = r.center();
            let pts = vec![
                ctr + vec2(-6.0, -10.0),
                ctr + vec2(7.0, 2.0),
                ctr + vec2(1.0, 3.0),
                ctr + vec2(4.0, 10.0),
                ctr + vec2(1.0, 11.0),
                ctr + vec2(-2.0, 4.0),
                ctr + vec2(-6.0, 8.0),
            ];
            p.add(Shape::closed_line(pts, Stroke::new(1.6, c)));
        }
        Tool::Connect => {
            let a = r.left_bottom() + vec2(1.0, -1.0);
            let b = r.right_top() + vec2(-1.0, 1.0);
            p.line_segment([a, b - (b - a).normalized() * 6.0], Stroke::new(1.6, c));
            canvas::arrow_head(p, b, b - a, 8.0, c, true);
        }
        Tool::Create(kind) => {
            let (w, h) = kind.default_size();
            let scale = (r.width() / w).min(r.height() / h).min(1.0) * if kind.is_event() { 0.95 } else { 1.0 };
            let rr = Rect::from_center_size(r.center(), vec2(w, h) * scale);
            paint_node(p, rr, kind, "", scale * 1.4, &pal, Some(c));
        }
        Tool::Pool => {
            let rr = r.shrink2(vec2(0.0, 3.0));
            p.rect_stroke(rr, 0.0, Stroke::new(1.4, c), StrokeKind::Middle);
            p.line_segment(
                [pos2(rr.min.x + 6.0, rr.min.y), pos2(rr.min.x + 6.0, rr.max.y)],
                Stroke::new(1.2, c),
            );
            p.line_segment(
                [pos2(rr.min.x + 6.0, rr.center().y), pos2(rr.max.x, rr.center().y)],
                Stroke::new(1.0, c),
            );
        }
    }
}
