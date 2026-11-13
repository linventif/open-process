//! BPMN 2.0 editor.

mod render;

mod inspector;

mod interaction;

mod session;
pub(crate) use session::register;

use crate::editor::layout::{CanvasControls, DiagramLayout, FooterStatus};

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
    (Tool::Select, key("bpmn.select_move_v_drag_empty_space_pan")),
    (Tool::Connect, key("bpmn.connect_c_sequence_flow_message_flow_between")),
    (Tool::Create(NodeKind::StartEvent(EventTrigger::None)), key("bpmn.start_event_s")),
    (
        Tool::Create(NodeKind::IntermediateEvent {
            trigger: EventTrigger::None,
            throwing: false,
        }),
        key("bpmn.intermediate_event_i"),
    ),
    (Tool::Create(NodeKind::EndEvent(EventTrigger::None)), key("bpmn.end_event_e")),
    (Tool::Create(NodeKind::Task(TaskType::Abstract)), key("bpmn.task_t")),
    (Tool::Create(NodeKind::SubProcess), key("bpmn.sub_process")),
    (
        Tool::Create(NodeKind::Gateway(GatewayType::Exclusive)),
        key("bpmn.exclusive_gateway_xor_x"),
    ),
    (Tool::Create(NodeKind::Gateway(GatewayType::Parallel)), key("bpmn.parallel_gateway")),
    (
        Tool::Create(NodeKind::Gateway(GatewayType::Inclusive)),
        key("bpmn.inclusive_gateway_o"),
    ),
    (
        Tool::Create(NodeKind::Gateway(GatewayType::EventBased)),
        key("bpmn.event_based_gateway"),
    ),
    (Tool::Create(NodeKind::DataObject), key("bpmn.data_object")),
    (Tool::Create(NodeKind::DataStore), key("bpmn.data_store")),
    (Tool::Create(NodeKind::TextAnnotation), key("bpmn.text_annotation")),
    (Tool::Pool, key("bpmn.pool_participant_lanes")),
];

const PAD: &[(PadAction, &str)] = &[
    (PadAction::Append(NodeKind::Task(TaskType::Abstract)), key("bpmn.append_task")),
    (
        PadAction::Append(NodeKind::Gateway(GatewayType::Exclusive)),
        key("bpmn.append_exclusive_gateway"),
    ),
    (
        PadAction::Append(NodeKind::Gateway(GatewayType::Parallel)),
        key("bpmn.append_parallel_gateway"),
    ),
    (
        PadAction::Append(NodeKind::IntermediateEvent {
            trigger: EventTrigger::None,
            throwing: false,
        }),
        key("bpmn.append_intermediate_event"),
    ),
    (
        PadAction::Append(NodeKind::EndEvent(EventTrigger::None)),
        key("bpmn.append_end_event"),
    ),
    (PadAction::Connect, key("bpmn.connect")),
    (PadAction::Delete, key("common.delete")),
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

    pub fn show_evaluation(&mut self) {
        self.tab = SideTab::Evaluation;
        self.show_inspector = Some(true);
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
        let layout = DiagramLayout::new(ui, self.uid, self.show_inspector);
        layout.inspector(ui, |ui| self.side_panel(ui));
        self.status_bar(ui, layout);
        layout.canvas(ui, |ui| self.canvas(ui));
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

    fn status_bar(&mut self, ui: &mut Ui, layout: DiagramLayout) {
        let mut arrange = false;
        let seq = self.diagram.flows.iter().filter(|f| f.kind == FlowKind::Sequence).count();
        let status = FooterStatus {
            summary: trf(
                "bpmn.elements_flows_pools",
                &[&self.diagram.nodes.len(), &seq, &self.diagram.pools.len()],
            ),
            detail: self.status.clone(),
            ..Default::default()
        };
        layout.configuration(
            ui,
            CanvasControls {
                view: &mut self.view,
                canvas: self.last_canvas,
                fit: &mut self.fit_request,
                snap: &mut self.snap,
                inspector: &mut self.show_inspector,
                issues: Some(&mut self.show_issues),
            },
            status,
            |ui| {
                canvas::style_menu(ui, &mut self.diagram.style, false);
                if ui
                    .add_enabled(
                        !self.diagram.nodes.is_empty() || !self.diagram.pools.is_empty(),
                        egui::Button::new(tr("bpmn.auto_arrange")),
                    )
                    .clicked()
                {
                    arrange = true;
                    if ui.stack().iter().any(|stack| stack.kind() == Some(egui::UiKind::Menu)) {
                        ui.close_kind(egui::UiKind::Menu);
                    }
                }
            },
        );
        if arrange {
            self.auto_arrange();
        }
    }

    fn auto_arrange(&mut self) {
        self.history.sync(&self.diagram, false);
        match op_core::bpmn::layout::auto_arrange(&mut self.diagram) {
            Ok(()) => {
                self.drag = None;
                self.editing = None;
                self.connect_pending = None;
                self.dnd = None;
                self.fit_request = true;
                self.status.clear();
                self.history.sync(&self.diagram, false);
            }
            Err(e) => self.status = e,
        }
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
                tr("bpmn.empty_bpmn_diagram"),
                crate::theme::semibold(17.0),
                pal.text,
            );
            painter.text(
                canvas.center() + vec2(0.0, 14.0),
                egui::Align2::CENTER_CENTER,
                tr("bpmn.pick_element_palette_click_canvas"),
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

/// Decorative icon inside a standard egui menu button: the label, focus,
/// selection and hit area remain those of the button.
fn node_icon(kind: NodeKind) -> egui::Atom<'static> {
    egui::Atom::paint(vec2(26.0, 26.0), move |ui, args| {
        let color = args.fallback_text_color;
        let rect = args.rect.shrink(2.0);
        match kind {
            NodeKind::Task(TaskType::Abstract) => {
                ui.painter()
                    .rect_stroke(rect.shrink2(vec2(0.0, 4.0)), 3.0, Stroke::new(1.4, color), StrokeKind::Inside);
            }
            NodeKind::Task(task) => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    task_marker(task),
                    FontId::proportional(20.0),
                    color,
                );
            }
            _ => paint_tool_icon(ui.painter(), rect, color, Tool::Create(kind)),
        }
    })
}

fn node_option(ui: &mut Ui, label: &str, kind: NodeKind, selected: Option<bool>) -> egui::Response {
    let mut button = egui::Button::new((node_icon(kind), label));
    if let Some(selected) = selected {
        button = button.selected(selected);
    }
    ui.add(button)
}

fn task_marker(task: TaskType) -> &'static str {
    match task {
        TaskType::Abstract => "",
        TaskType::User => "👤",
        TaskType::Manual => "✋",
        TaskType::Service => "⚙",
        TaskType::Script => "📜",
        TaskType::Send => "✉",
        TaskType::Receive => "📨",
        TaskType::BusinessRule => "▦",
    }
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
            let icon = task_marker(t);
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
        Tool::Select => crate::icons::select(p, r, c),
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
