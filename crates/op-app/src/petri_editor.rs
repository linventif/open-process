//! Petri net editor: modelling, token game, matrices, analysis, reachability graph.

use crate::canvas::{self, InlineOutcome, Palette, Rng, View};
use crate::history::History;
use egui::{Color32, CornerRadius, FontId, Key, Painter, PointerButton, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use op_core::analysis::{self, BehaviouralReport, Soundness, StateSpace, StructuralReport, format_marking};
use op_core::geom::{Pos, dist_to_polyline};
use op_core::i18n::{tr, trf};
use op_core::petri::{ArcKind, Id, Marking, NodeRef, OMEGA, PetriNet};

const PLACE_R: f32 = 22.0;
const TRANS_W: f32 = 14.0;
const TRANS_H: f32 = 44.0;
const GRID: f32 = 10.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    Place,
    Transition,
    Arc,
    Token,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Sel {
    Node(NodeRef),
    Arc(Id),
}

#[derive(Clone, Copy, Debug)]
enum Drag {
    Move {
        node: NodeRef,
        grab: Pos,
    },
    /// Moves every node of the multi-selection.
    MoveGroup {
        last: Pos,
    },
    /// Rubber-band selection rectangle started at this world point.
    Rubber {
        start: Pos,
    },
    Arc {
        from: NodeRef,
    },
    Pan,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SideTab {
    Properties,
    Matrices,
    Analysis,
    States,
}

pub struct Simulation {
    pub marking: Marking,
    pub trace: Vec<usize>,
    stack: Vec<Marking>,
    auto: bool,
    interval: f32,
    last: f64,
    rng: Rng,
    anim: Option<FireAnim>,
}

/// Tokens travelling along the arcs of the last fired transition.
#[derive(Clone)]
struct FireAnim {
    t: usize,
    start: f64,
    duration: f32,
    before: Marking,
}

struct AnalysisCache {
    net: PetriNet,
    limit: usize,
    ss: StateSpace,
    beh: BehaviouralReport,
    st: StructuralReport,
    pinv: Option<Vec<Vec<i64>>>,
    tinv: Option<Vec<Vec<i64>>>,
    sound: Soundness,
}

pub struct PetriEditor {
    pub uid: u64,
    /// Export the canvas as PNG on the next frame (without palette and selection).
    pub export_png: bool,
    /// Hand-filled matrices (exercise mode): Pre, Post, C.
    exercise: Option<[Vec<Vec<String>>; 3]>,
    exercise_checked: bool,
    /// Multi-selection (rubber band, Shift+click, Ctrl+A).
    group: Vec<NodeRef>,
    /// Tool being dragged from the palette onto the canvas.
    dnd: Option<Tool>,
    toolbar_rect: Rect,
    /// None = automatic (hidden when the pane is narrow).
    show_inspector: Option<bool>,
    inspector_visible: bool,
    /// Set when the user asks to show a view in its own pane.
    pub request_pane: Option<crate::dock::View>,
    pub net: PetriNet,
    pub view: View,
    last_canvas: Rect,
    tool: Tool,
    selection: Option<Sel>,
    drag: Option<Drag>,
    arc_pending: Option<NodeRef>,
    history: History<PetriNet>,
    pub sim: Option<Simulation>,
    analysis: Option<AnalysisCache>,
    tab: SideTab,
    pub status: String,
    snap: bool,
    fit_request: bool,
    ctx_target: (Option<Sel>, Pos),
    editing: Option<(NodeRef, String, bool)>,
    state_limit: usize,
    graph_open: bool,
    graph_view: View,
    graph_fit: bool,
    selected_state: Option<usize>,
    /// Time of the current frame (for animations).
    now: f64,
}

impl PetriEditor {
    pub fn new(net: PetriNet) -> Self {
        let history = History::new(&net);
        Self {
            uid: canvas::next_uid(),
            export_png: false,
            exercise: None,
            exercise_checked: false,
            group: Vec::new(),
            dnd: None,
            toolbar_rect: Rect::NOTHING,
            show_inspector: None,
            inspector_visible: true,
            request_pane: None,
            net,
            view: View::default(),
            last_canvas: Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0)),
            tool: Tool::Select,
            selection: None,
            drag: None,
            arc_pending: None,
            history,
            sim: None,
            analysis: None,
            tab: SideTab::Properties,
            status: String::new(),
            snap: true,
            fit_request: true,
            ctx_target: (None, Pos::default()),
            editing: None,
            state_limit: 5000,
            graph_open: false,
            graph_view: View::default(),
            graph_fit: true,
            selected_state: None,
            now: 0.0,
        }
    }

    pub fn undo(&mut self) {
        if self.sim.is_none() {
            self.history.undo(&mut self.net);
            self.selection = None;
        }
    }

    pub fn redo(&mut self) {
        if self.sim.is_none() {
            self.history.redo(&mut self.net);
            self.selection = None;
        }
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

    #[cfg(test)]
    pub fn selected_count(&self) -> usize {
        self.group.len() + usize::from(self.selection.is_some())
    }

    #[cfg(test)]
    pub fn screen_pos(&self, p: Pos) -> Pos2 {
        self.view.to_screen(self.last_canvas, p)
    }

    /// Fires the first transition named `name` in the token game (tests, scripting).
    #[cfg(test)]
    pub fn fire_by_name(&mut self, name: &str) -> bool {
        match self.net.transitions.iter().position(|t| t.name == name) {
            Some(t) => self.fire(t),
            None => false,
        }
    }

    pub fn show_matrices(&mut self) {
        self.tab = SideTab::Matrices;
    }

    pub fn show_analysis(&mut self) {
        self.tab = SideTab::Analysis;
    }

    pub fn show_reachability_graph(&mut self) {
        self.tab = SideTab::States;
        self.graph_open = true;
        self.graph_fit = true;
    }

    pub fn delete_selection(&mut self) {
        if self.sim.is_some() {
            return;
        }
        for n in std::mem::take(&mut self.group) {
            self.net.remove_node(n);
        }
        match self.selection.take() {
            Some(Sel::Node(n)) => self.net.remove_node(n),
            Some(Sel::Arc(a)) => self.net.remove_arc(a),
            None => {}
        }
    }

    // ------------------------------------------------------------------
    // Geometry & hit testing

    fn hit_node(&self, w: Pos) -> Option<NodeRef> {
        for t in self.net.transitions.iter().rev() {
            let (tw, th) = self.trans_size(&t.name);
            if (w.x - t.pos.x).abs() <= tw / 2.0 + 3.0 && (w.y - t.pos.y).abs() <= th / 2.0 + 3.0 {
                return Some(NodeRef::Transition(t.id));
            }
        }
        for p in self.net.places.iter().rev() {
            if p.pos.dist(w) <= PLACE_R + 2.0 {
                return Some(NodeRef::Place(p.id));
            }
        }
        None
    }

    /// World size of a transition: a box sized to its name (course notation) or a thin bar.
    fn trans_size(&self, name: &str) -> (f32, f32) {
        match self.net.style.transitions {
            op_core::style::TransitionShape::Bar => (TRANS_W, TRANS_H),
            op_core::style::TransitionShape::Box => ((name.chars().count() as f32 * 7.4 + 20.0).max(44.0), 30.0),
        }
    }

    fn trans_size_of(&self, id: Id) -> (f32, f32) {
        self.trans_size(self.net.transition(id).map(|t| t.name.as_str()).unwrap_or(""))
    }

    fn boundary(&self, n: NodeRef, center: Pos, dir: (f32, f32)) -> Pos {
        match n {
            NodeRef::Place(_) => Pos::new(center.x + dir.0 * PLACE_R, center.y + dir.1 * PLACE_R),
            NodeRef::Transition(id) => {
                let (tw, th) = self.trans_size_of(id);
                let tx = if dir.0.abs() > 1e-6 {
                    tw / 2.0 / dir.0.abs()
                } else {
                    f32::INFINITY
                };
                let ty = if dir.1.abs() > 1e-6 {
                    th / 2.0 / dir.1.abs()
                } else {
                    f32::INFINITY
                };
                let t = tx.min(ty);
                Pos::new(center.x + dir.0 * t, center.y + dir.1 * t)
            }
        }
    }

    /// World-space path of an arc, following the link style of the net.
    fn arc_path(&self, from: NodeRef, to: NodeRef) -> Option<Vec<Pos>> {
        use op_core::style::EdgeStyle;
        let (a, b) = (self.net.node_pos(from)?, self.net.node_pos(to)?);
        let len = a.dist(b);
        if len < 1.0 {
            return None;
        }
        let d = ((b.x - a.x) / len, (b.y - a.y) / len);
        let reverse = self.net.arcs.iter().any(|x| x.from == to && x.to == from);
        let half = |n: NodeRef| match n {
            NodeRef::Place(_) => (PLACE_R, PLACE_R),
            NodeRef::Transition(id) => {
                let (tw, th) = self.trans_size_of(id);
                (tw / 2.0, th / 2.0)
            }
        };
        let dir = |p: Pos, q: Pos| {
            let l = p.dist(q).max(1e-3);
            ((q.x - p.x) / l, (q.y - p.y) / l)
        };
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let off = if reverse { 9.0 } else { 0.0 };
        match self.net.style.edges.or(EdgeStyle::Straight) {
            EdgeStyle::Orthogonal => {
                let (ha, hb) = (half(from), half(to));
                if dx.abs() >= dy.abs() {
                    let sx = dx.signum();
                    let (ay, by) = (a.y - off * sx, b.y - off * sx);
                    let s = Pos::new(a.x + sx * ha.0, ay);
                    let e = Pos::new(b.x - sx * hb.0, by);
                    if (ay - by).abs() < 2.0 {
                        return Some(vec![s, e]);
                    }
                    let mx = (s.x + e.x) / 2.0;
                    Some(vec![s, Pos::new(mx, ay), Pos::new(mx, by), e])
                } else {
                    let sy = dy.signum();
                    let (ax, bx) = (a.x + off * sy, b.x + off * sy);
                    let s = Pos::new(ax, a.y + sy * ha.1);
                    let e = Pos::new(bx, b.y - sy * hb.1);
                    if (ax - bx).abs() < 2.0 {
                        return Some(vec![s, e]);
                    }
                    let my = (s.y + e.y) / 2.0;
                    Some(vec![s, Pos::new(ax, my), Pos::new(bx, my), e])
                }
            }
            EdgeStyle::Curved => {
                let n = Pos::new(-d.1 * off * 2.0, d.0 * off * 2.0);
                let (c1, c2) = if dx.abs() >= dy.abs() {
                    (Pos::new(a.x + dx * 0.5, a.y) + n, Pos::new(b.x - dx * 0.5, b.y) + n)
                } else {
                    (Pos::new(a.x, a.y + dy * 0.5) + n, Pos::new(b.x, b.y - dy * 0.5) + n)
                };
                let s = self.boundary(from, a, dir(a, c1));
                let e = self.boundary(to, b, dir(b, c2));
                Some(
                    (0..=20)
                        .map(|i| {
                            let t = i as f32 / 20.0;
                            let u = 1.0 - t;
                            let (k1, k2, k3, k4) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                            Pos::new(
                                k1 * s.x + k2 * c1.x + k3 * c2.x + k4 * e.x,
                                k1 * s.y + k2 * c1.y + k3 * c2.y + k4 * e.y,
                            )
                        })
                        .collect(),
                )
            }
            _ => {
                if !reverse {
                    return Some(vec![self.boundary(from, a, d), self.boundary(to, b, (-d.0, -d.1))]);
                }
                let bend = (len * 0.2).min(30.0);
                let ctrl = Pos::new((a.x + b.x) / 2.0 - d.1 * bend, (a.y + b.y) / 2.0 + d.0 * bend);
                let s = self.boundary(from, a, dir(a, ctrl));
                let e = self.boundary(to, b, dir(b, ctrl));
                Some(
                    (0..=20)
                        .map(|i| {
                            let t = i as f32 / 20.0;
                            let u = 1.0 - t;
                            Pos::new(
                                u * u * s.x + 2.0 * u * t * ctrl.x + t * t * e.x,
                                u * u * s.y + 2.0 * u * t * ctrl.y + t * t * e.y,
                            )
                        })
                        .collect(),
                )
            }
        }
    }

    fn hit_arc(&self, w: Pos, tol: f32) -> Option<Id> {
        self.net
            .arcs
            .iter()
            .rev()
            .find(|a| self.arc_path(a.from, a.to).is_some_and(|pts| dist_to_polyline(w, &pts) <= tol))
            .map(|a| a.id)
    }

    fn hit(&self, w: Pos) -> Option<Sel> {
        self.hit_node(w)
            .map(Sel::Node)
            .or_else(|| self.hit_arc(w, 6.0 / self.view.zoom.max(0.3)).map(Sel::Arc))
    }

    fn snap(&self, p: Pos) -> Pos {
        if self.snap { p.snapped(GRID) } else { p }
    }

    fn fit(&mut self, canvas: Rect) {
        let pts: Vec<Pos> = self
            .net
            .places
            .iter()
            .map(|p| p.pos)
            .chain(self.net.transitions.iter().map(|t| t.pos))
            .collect();
        if pts.is_empty() {
            self.view = View::default();
            self.view.offset = canvas.size() / 2.0;
            return;
        }
        let min = Pos::new(
            pts.iter().map(|p| p.x).fold(f32::MAX, f32::min) - 40.0,
            pts.iter().map(|p| p.y).fold(f32::MAX, f32::min) - 40.0,
        );
        let max = Pos::new(
            pts.iter().map(|p| p.x).fold(f32::MIN, f32::max) + 40.0,
            pts.iter().map(|p| p.y).fold(f32::MIN, f32::max) + 50.0,
        );
        self.view.fit(canvas, min, max);
    }

    /// Marking currently displayed (simulation or initial marking).
    fn current_marking(&self) -> Marking {
        match &self.sim {
            Some(s) => s.marking.clone(),
            None => self.net.initial_marking(),
        }
    }

    // ------------------------------------------------------------------
    // UI

    pub fn ui(&mut self, ui: &mut Ui) {
        self.now = ui.input(|i| i.time);
        let panel = ui.visuals().panel_fill;
        let inspector = self.show_inspector.unwrap_or(ui.available_width() > 760.0);
        self.inspector_visible = inspector;
        if inspector {
            egui::Panel::right("petri_side")
                .resizable(true)
                .default_size(340.0)
                .min_size(260.0)
                .frame(egui::Frame::new().fill(panel).inner_margin(egui::Margin {
                    left: 14,
                    right: 12,
                    top: 10,
                    bottom: 6,
                }))
                .show(ui, |ui| self.side_panel(ui));
        }
        egui::Panel::bottom("petri_status")
            .exact_size(30.0)
            .frame(egui::Frame::new().fill(panel).inner_margin(egui::Margin::symmetric(12, 3)))
            .show(ui, |ui| self.status_bar(ui));
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| self.canvas(ui));

        if self.graph_open {
            let mut open = true;
            egui::Window::new(tr("Reachability graph"))
                .id(egui::Id::unique(("graph_window", self.uid)))
                .open(&mut open)
                .default_size([900.0, 600.0])
                .show(ui.ctx(), |ui| self.graph_window(ui));
            self.graph_open = open;
        }

        self.tick_simulation(ui);
        let interacting = ui.input(|i| i.pointer.any_down()) || ui.ctx().egui_wants_keyboard_input();
        self.history.sync(&self.net, interacting);
    }

    fn palette(&mut self, ui: &mut Ui) {
        let boxed = self.net.style.transitions == op_core::style::TransitionShape::Box;
        ui.vertical_centered(|ui| {
            let editing = self.sim.is_none();
            let tools: [(Tool, &str); 5] = [
                (
                    Tool::Select,
                    tr("Select / move (V)\nDrag empty space to pan, wheel to scroll, Ctrl+wheel to zoom"),
                ),
                (Tool::Place, tr("Place (P)")),
                (Tool::Transition, tr("Transition (T)")),
                (Tool::Arc, tr("Arc (A): drag from a node to another, or click both")),
                (Tool::Token, tr("Tokens (K): left click +1, right click −1")),
            ];
            for (tool, tip) in tools {
                let r = ui.add_enabled_ui(editing || tool == Tool::Select, |ui| {
                    canvas::icon_button(ui, self.tool == tool, tip, |p, r, c| paint_tool_icon(p, r, c, tool, boxed))
                });
                if r.inner.clicked() {
                    self.tool = tool;
                    self.arc_pending = None;
                }
                if r.inner.drag_started() && matches!(tool, Tool::Place | Tool::Transition) && editing {
                    self.dnd = Some(tool);
                }
            }
            canvas::toolbar_separator(ui);
            let sim_on = self.sim.is_some();
            let r = canvas::icon_button(ui, sim_on, tr("Token game / simulation (S)"), |p, r, c| {
                let c2 = r.center();
                p.add(egui::Shape::convex_polygon(
                    vec![c2 + vec2(-7.0, -10.0), c2 + vec2(10.0, 0.0), c2 + vec2(-7.0, 10.0)],
                    if sim_on { c } else { Color32::TRANSPARENT },
                    Stroke::new(1.6, c),
                ));
            });
            if r.clicked() {
                self.toggle_simulation(ui.input(|i| i.time));
            }
        });
    }

    pub fn toggle_simulation(&mut self, time: f64) {
        if self.sim.is_some() {
            self.sim = None;
        } else {
            self.sim = Some(Simulation {
                marking: self.net.initial_marking(),
                trace: Vec::new(),
                stack: Vec::new(),
                auto: false,
                interval: 0.6,
                last: time,
                rng: Rng((time * 1e6) as u64 ^ 0x9E37_79B9_7F4A_7C15),
                anim: None,
            });
            self.tool = Tool::Select;
            self.arc_pending = None;
            self.tab = SideTab::Properties;
        }
    }

    fn fire(&mut self, t: usize) -> bool {
        let compiled = self.net.compile();
        let now = self.now;
        let Some(sim) = &mut self.sim else { return false };
        match compiled.fire(t, &sim.marking) {
            Some(m) => {
                let before = std::mem::replace(&mut sim.marking, m);
                sim.anim = Some(FireAnim {
                    t,
                    start: now,
                    duration: (sim.interval * 0.85).min(0.55),
                    before: before.clone(),
                });
                sim.stack.push(before);
                sim.trace.push(t);
                true
            }
            None => false,
        }
    }

    fn random_step(&mut self) -> bool {
        let enabled = match &self.sim {
            Some(s) => self.net.enabled_transitions(&s.marking),
            None => return false,
        };
        if enabled.is_empty() {
            return false;
        }
        let k = self.sim.as_mut().unwrap().rng.pick(enabled.len());
        self.fire(enabled[k])
    }

    fn tick_simulation(&mut self, ui: &Ui) {
        let now = ui.input(|i| i.time);
        let Some(sim) = &mut self.sim else { return };
        if let Some(a) = &sim.anim {
            if now - a.start >= a.duration as f64 {
                sim.anim = None;
            } else {
                ui.ctx().request_repaint();
            }
        }
        if !sim.auto {
            return;
        }
        if now - sim.last >= sim.interval as f64 {
            sim.last = now;
            if !self.random_step() {
                if let Some(s) = &mut self.sim {
                    s.auto = false;
                }
                self.status = tr("Dead marking reached: no transition is enabled").into();
            }
        }
        if let Some(s) = &self.sim {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f32(s.interval.min(0.1)));
        }
    }

    fn status_bar(&mut self, ui: &mut Ui) {
        ui.horizontal_centered(|ui| {
            let weak = ui.visuals().weak_text_color();
            let narrow = ui.available_width() < 560.0;
            if !narrow {
                ui.label(
                    RichText::new(trf(
                        "{} places · {} transitions · {} arcs",
                        &[&self.net.places.len(), &self.net.transitions.len(), &self.net.arcs.len()],
                    ))
                    .color(weak),
                );
            }
            if self.sim.is_some() {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(tr("● Simulation"))
                        .color(Palette::for_ui(ui).ok)
                        .font(crate::theme::semibold(12.0)),
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
                canvas::style_menu(ui, &mut self.net.style, true);
            });
        });
    }

    // ------------------------------------------------------------------
    // Canvas

    /// Renders one of the analysis views on its own (for a docked pane).
    pub fn view_ui(&mut self, ui: &mut Ui, view: crate::dock::View) {
        use crate::dock::View;
        self.now = ui.input(|i| i.time);
        match view {
            View::Graph => self.graph_window(ui),
            View::Editor => self.ui(ui),
            _ => {
                egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| match view {
                        View::Matrices => self.matrices(ui),
                        View::Analysis => self.analysis_panel(ui),
                        View::States => self.states_panel(ui),
                        _ => {}
                    });
                });
            }
        }
    }

    /// Read-only, embeddable view of the net (drag to pan, Ctrl+wheel to zoom).
    pub fn preview_ui(&mut self, ui: &mut Ui, size: egui::Vec2) {
        let (response, painter) = ui.allocate_painter(size, Sense::click_and_drag());
        let canvas = response.rect;
        let painter = painter.with_clip_rect(canvas);
        let pal = Palette::for_ui(ui);
        if self.fit_request {
            self.fit(canvas);
            self.fit_request = false;
        }
        if response.hovered() {
            let (zoom, pointer) = ui.input(|i| (i.zoom_delta(), i.pointer.hover_pos()));
            if zoom != 1.0
                && let Some(p) = pointer
            {
                self.view.zoom_around(canvas, p, zoom);
            }
        }
        if response.dragged() {
            self.view.offset += response.drag_delta();
        }
        if response.double_clicked() {
            self.fit_request = true;
        }
        canvas::draw_background(&painter, canvas, &self.view, &pal);
        self.draw_net(&painter, canvas, &pal, None);
        painter.rect_stroke(canvas, 0.0, Stroke::new(1.0, pal.grid), StrokeKind::Inside);
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

        if self.sim.is_some() {
            self.sim_interaction(&response, wp);
        } else {
            self.edit_interaction(ui, &response, canvas, wp);
        }

        if response.hovered() && !ui.ctx().egui_wants_keyboard_input() {
            self.shortcuts(ui);
        }

        // Image export: render this frame without selection, hover and palette.
        let exporting = std::mem::take(&mut self.export_png);
        let saved = exporting.then(|| (self.selection.take(), std::mem::take(&mut self.group)));
        canvas::draw_background(&painter, canvas, &self.view, &pal);
        self.draw_net(&painter, canvas, &pal, if exporting { None } else { wp });
        if let Some((sel, group)) = saved {
            self.selection = sel;
            self.group = group;
            crate::io::request_png_export(ui.ctx(), canvas, format!("{}.png", self.file_stem()));
        }

        // Rubber-band selection
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

        // Rubber band while creating an arc
        let from = match self.drag {
            Some(Drag::Arc { from }) => Some(from),
            _ => self.arc_pending,
        };
        if let (Some(from), Some(p)) = (from, pointer)
            && let Some(a) = self.net.node_pos(from)
        {
            canvas::dashed(
                &painter,
                &[self.view.to_screen(canvas, a), p],
                Stroke::new(1.5, pal.select),
                6.0,
                4.0,
            );
        }

        if self.net.places.is_empty() && self.net.transitions.is_empty() {
            painter.text(
                canvas.center() - vec2(0.0, 12.0),
                egui::Align2::CENTER_CENTER,
                tr("Empty Petri net"),
                crate::theme::semibold(17.0),
                pal.text,
            );
            painter.text(
                canvas.center() + vec2(0.0, 14.0),
                egui::Align2::CENTER_CENTER,
                tr("Pick the Place (P) or Transition (T) tool and click on the canvas"),
                FontId::proportional(13.5),
                pal.muted,
            );
        }

        // Floating tool palette
        if !exporting {
            self.toolbar_rect = canvas::floating(
                ui.ctx(),
                egui::Id::unique(("petri_toolbar", self.uid)),
                canvas.left_top() + vec2(14.0, 14.0),
                egui::Align2::LEFT_TOP,
                |ui| self.palette(ui),
            );
        }
        // Simulation banner
        if self.sim.is_some() && !exporting {
            let enabled = self.net.enabled_transitions(&self.current_marking()).len();
            let fired = self.sim.as_ref().map(|s| s.trace.len()).unwrap_or(0);
            let mut stop = false;
            canvas::floating(
                ui.ctx(),
                egui::Id::unique(("petri_sim_banner", self.uid)),
                pos2(canvas.center().x, canvas.top() + 14.0),
                egui::Align2::CENTER_TOP,
                |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(6.0);
                        let (r, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
                        ui.painter().circle_filled(r.center(), 4.5, pal.ok);
                        ui.label(RichText::new(tr("Token game")).font(crate::theme::semibold(13.0)));
                        ui.label(
                            RichText::new(if enabled == 0 {
                                trf("dead marking · {} firing(s)", &[&fired])
                            } else {
                                trf("click a green transition · {} enabled · {} firing(s)", &[&enabled, &fired])
                            })
                            .color(ui.visuals().weak_text_color()),
                        );
                        if ui.button(tr("Stop")).clicked() {
                            stop = true;
                        }
                    });
                },
            );
            if stop {
                self.sim = None;
            }
        }

        // Inline renaming
        if let Some((node, mut text, first)) = self.editing.take() {
            let pos = self.net.node_pos(node).unwrap_or_default();
            let sp = self.view.to_screen(canvas, pos) + vec2(-60.0, 30.0 * self.view.zoom);
            match canvas::inline_editor(ui.ctx(), egui::Id::unique(("petri_inline", self.uid)), sp, 120.0, &mut text, first) {
                InlineOutcome::Editing => self.editing = Some((node, text, false)),
                InlineOutcome::Done => match node {
                    NodeRef::Place(id) => self.net.place_mut(id).into_iter().for_each(|p| p.name = text.clone()),
                    NodeRef::Transition(id) => self.net.transition_mut(id).into_iter().for_each(|t| t.name = text.clone()),
                },
                InlineOutcome::Cancelled => {}
            }
        }
    }

    fn shortcuts(&mut self, ui: &Ui) {
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

    fn sim_interaction(&mut self, response: &egui::Response, wp: Option<Pos>) {
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
                        self.status = trf("Fired {}", &[&name]);
                    } else {
                        self.status = trf("{} is not enabled", &[&name]);
                    }
                }
                Some(n) => self.selection = Some(Sel::Node(n)),
                None => self.selection = None,
            }
        }
    }

    fn edit_interaction(&mut self, ui: &Ui, response: &egui::Response, canvas: Rect, wp: Option<Pos>) {
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
    fn normalize_group(&mut self) {
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

    fn in_selection(&self, n: NodeRef) -> bool {
        self.selection == Some(Sel::Node(n)) || self.group.contains(&n)
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

    fn connect(&mut self, from: NodeRef, to: NodeRef) {
        match self.net.add_arc(from, to) {
            Ok(id) => {
                self.selection = Some(Sel::Arc(id));
                self.status.clear();
            }
            Err(e) => self.status = e.to_string(),
        }
    }

    fn context_menu(&mut self, ui: &mut Ui) {
        let (target, w) = self.ctx_target;
        match target {
            Some(Sel::Node(NodeRef::Place(id))) => {
                if ui.button(tr("Add token")).clicked() {
                    if let Some(p) = self.net.place_mut(id) {
                        p.tokens += 1;
                    }
                    ui.close();
                }
                if ui.button(tr("Remove token")).clicked() {
                    if let Some(p) = self.net.place_mut(id) {
                        p.tokens = p.tokens.saturating_sub(1);
                    }
                    ui.close();
                }
                if ui.button(tr("Rename…")).clicked() {
                    let n = NodeRef::Place(id);
                    self.editing = Some((n, self.net.node_name(n).unwrap_or("").into(), true));
                    ui.close();
                }
                if ui.button(tr("Delete")).clicked() {
                    self.net.remove_node(NodeRef::Place(id));
                    ui.close();
                }
            }
            Some(Sel::Node(NodeRef::Transition(id))) => {
                if let Some(t) = self.net.transition_mut(id) {
                    ui.checkbox(&mut t.silent, tr("Silent (τ)"));
                }
                if ui.button(tr("Rename…")).clicked() {
                    let n = NodeRef::Transition(id);
                    self.editing = Some((n, self.net.node_name(n).unwrap_or("").into(), true));
                    ui.close();
                }
                if ui.button(tr("Delete")).clicked() {
                    self.net.remove_node(NodeRef::Transition(id));
                    ui.close();
                }
            }
            Some(Sel::Arc(id)) => {
                if let Some(a) = self.net.arc_mut(id) {
                    ui.horizontal(|ui| {
                        ui.label(tr("Weight"));
                        ui.add(egui::DragValue::new(&mut a.weight).range(1..=1000));
                    });
                    if matches!(a.from, NodeRef::Place(_)) {
                        let mut inh = a.kind == ArcKind::Inhibitor;
                        if ui.checkbox(&mut inh, tr("Inhibitor arc")).changed() {
                            a.kind = if inh { ArcKind::Inhibitor } else { ArcKind::Normal };
                        }
                    }
                }
                if ui.button(tr("Delete")).clicked() {
                    self.net.remove_arc(id);
                    ui.close();
                }
            }
            None => {
                if ui.button(tr("Add place here")).clicked() {
                    let id = self.net.add_place(self.snap(w), 0);
                    self.selection = Some(Sel::Node(NodeRef::Place(id)));
                    ui.close();
                }
                if ui.button(tr("Add transition here")).clicked() {
                    let id = self.net.add_transition(self.snap(w));
                    self.selection = Some(Sel::Node(NodeRef::Transition(id)));
                    ui.close();
                }
                if ui.button(tr("Zoom to fit")).clicked() {
                    self.fit_request = true;
                    ui.close();
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Rendering

    fn draw_net(&self, painter: &Painter, canvas: Rect, pal: &Palette, wp: Option<Pos>) {
        let sketch = self.net.style.sketch;
        canvas::set_sketch(sketch);
        self.draw_net_inner(painter, canvas, pal, wp, sketch);
        canvas::set_sketch(false);
    }

    fn draw_net_inner(&self, painter: &Painter, canvas: Rect, pal: &Palette, wp: Option<Pos>, sketch: bool) {
        let v = &self.view;
        let z = v.zoom;
        let compiled = self.net.compile();
        let sim_on = self.sim.is_some();

        // Animation state of the last firing
        let anim = self
            .sim
            .as_ref()
            .and_then(|s| s.anim.clone())
            .filter(|a| self.now - a.start < a.duration as f64);
        let progress = anim
            .as_ref()
            .map(|a| (((self.now - a.start) / a.duration as f64) as f32).clamp(0.0, 1.0));
        let marking = match (&anim, &self.sim) {
            // during the animation, consumed tokens have left and produced ones have not arrived yet
            (Some(a), _) => {
                let mut m = a.before.clone();
                for &(p, w) in &compiled.inputs[a.t] {
                    m[p] = m[p].saturating_sub(w);
                }
                m
            }
            _ => self.current_marking(),
        };
        let enabled: Vec<bool> = match &self.sim {
            Some(s) => (0..self.net.transitions.len())
                .map(|t| compiled.is_enabled(t, &s.marking))
                .collect(),
            None => vec![false; self.net.transitions.len()],
        };
        let hover = wp.and_then(|w| self.hit(w));
        let focus = match (hover, self.selection) {
            (Some(Sel::Node(n)), _) => Some(n),
            (_, Some(Sel::Node(n))) => Some(n),
            _ => None,
        };
        let label_size = (12.0 * z).clamp(7.0, 22.0);

        // ---- arcs
        for a in &self.net.arcs {
            let Some(path) = self.arc_path(a.from, a.to) else { continue };
            let pts: Vec<Pos2> = path.iter().map(|&p| v.to_screen(canvas, p)).collect();
            let selected = self.selection == Some(Sel::Arc(a.id));
            let hovered = hover == Some(Sel::Arc(a.id));
            let related = focus.is_some_and(|n| a.from == n || a.to == n);
            let firing = anim.as_ref().is_some_and(|f| {
                a.from == NodeRef::Transition(self.net.transitions[f.t].id) || a.to == NodeRef::Transition(self.net.transitions[f.t].id)
            });
            let color = if selected || hovered || related {
                pal.select
            } else if firing {
                pal.ok
            } else {
                pal.arc
            };
            let width = if selected {
                2.4
            } else if hovered || related {
                1.9
            } else {
                1.4
            } * z.clamp(0.6, 1.6);
            let n = pts.len();
            let dir = pts[n - 1] - pts[n - 2];
            let head = 10.0 * z.clamp(0.5, 1.6);
            let mut body = pts.clone();
            match a.kind {
                ArcKind::Normal if sketch => {
                    canvas::sketch_polyline(painter, &body, Stroke::new(width, color), a.id as u64, false);
                    canvas::sketch_arrow_head(painter, pts[n - 1], dir, head, Stroke::new(width, color), a.id as u64);
                }
                ArcKind::Normal => {
                    body[n - 1] = pts[n - 1] - dir.normalized() * head * 0.75;
                    canvas::polyline(painter, &body, Stroke::new(width, color));
                    canvas::arrow_head_slim(painter, pts[n - 1], dir, head, color);
                }
                ArcKind::Inhibitor => {
                    let r = 4.5 * z.clamp(0.5, 1.6);
                    body[n - 1] = pts[n - 1] - dir.normalized() * 2.0 * r;
                    if sketch {
                        canvas::sketch_polyline(painter, &body, Stroke::new(width, color), a.id as u64, false);
                    } else {
                        canvas::polyline(painter, &body, Stroke::new(width, color));
                    }
                    painter.circle(pts[n - 1] - dir.normalized() * r, r, pal.fill, Stroke::new(width, color));
                }
            }
            if a.weight != 1 {
                let mid = canvas::point_along(&pts, 0.5);
                let nrm = vec2(-dir.y, dir.x).normalized() * 11.0 * z;
                canvas::badge(painter, mid + nrm, &a.weight.to_string(), (11.0 * z).clamp(8.0, 18.0), color, pal);
            }
        }

        // ---- places
        for (i, p) in self.net.places.iter().enumerate() {
            let c = v.to_screen(canvas, p.pos);
            let r = PLACE_R * z;
            let node = NodeRef::Place(p.id);
            let sel = self.in_selection(node);
            let hov = hover == Some(Sel::Node(node));
            if !sketch {
                canvas::shadow_circle(painter, c, r, pal);
            }
            if sel {
                painter.circle_stroke(c, r + 4.0 * z, Stroke::new(4.0 * z, pal.select.gamma_multiply(0.25)));
            }
            let stroke_c = if sel || hov { pal.select } else { pal.ink };
            let stroke = Stroke::new(if sel { 2.2 } else { 1.6 } * z.clamp(0.6, 1.6), stroke_c);
            if sketch {
                painter.circle_filled(c, r, pal.fill);
                canvas::sketch_ellipse(painter, c, vec2(r, r), stroke, p.id as u64);
            } else {
                painter.circle(c, r, pal.fill, stroke);
            }
            draw_tokens(painter, c, z, marking[i], pal.token);
            canvas::text_below(
                painter,
                c + vec2(0.0, r + 4.0 * z),
                150.0 * z,
                &p.name,
                label_size,
                pal.text,
                Some(pal.bg),
            );
            if let Some(cap) = p.capacity {
                canvas::badge(
                    painter,
                    c + vec2(r * 0.85, -r * 0.85),
                    &format!("≤{cap}"),
                    label_size * 0.8,
                    pal.muted,
                    pal,
                );
            }
        }

        // ---- transitions
        for (i, t) in self.net.transitions.iter().enumerate() {
            let c = v.to_screen(canvas, t.pos);
            let boxed = self.net.style.transitions == op_core::style::TransitionShape::Box;
            let (tw, th) = self.trans_size(&t.name);
            let rect = Rect::from_center_size(c, vec2(tw, th) * z);
            let node = NodeRef::Transition(t.id);
            let sel = self.in_selection(node);
            let hov = hover == Some(Sel::Node(node));
            let is_firing = anim.as_ref().is_some_and(|a| a.t == i);
            let radius = (3.0 * z).min(255.0) as u8;
            if sim_on && (enabled[i] || is_firing) {
                let glow = if is_firing { pal.select } else { pal.ok };
                for k in [9.0f32, 6.0, 3.0] {
                    painter.rect_filled(
                        rect.expand(k * z),
                        CornerRadius::same(radius + (k * z) as u8),
                        glow.gamma_multiply(0.07 + if hov { 0.05 } else { 0.0 }),
                    );
                }
            } else if !sketch {
                canvas::shadow_rect(painter, rect, radius as f32, pal);
            }
            let fill = if is_firing {
                pal.select
            } else if sim_on && enabled[i] {
                pal.ok
            } else if t.silent {
                pal.silent
            } else {
                pal.transition
            };
            if boxed {
                // Course notation: rectangle with the name inside.
                let (bg, border) = if is_firing {
                    (pal.select.gamma_multiply(0.25), pal.select)
                } else if sim_on && enabled[i] {
                    (pal.ok.gamma_multiply(0.22), pal.ok)
                } else if t.silent {
                    (pal.silent.gamma_multiply(0.35), pal.ink)
                } else {
                    (pal.task_fill, pal.ink)
                };
                let stroke = Stroke::new(1.5 * z.clamp(0.6, 1.6), border);
                if sketch {
                    painter.rect_filled(rect, 0.0, pal.fill);
                    painter.rect_filled(rect, 0.0, bg);
                    canvas::sketch_rect(painter, rect, stroke, t.id as u64);
                } else {
                    painter.rect(rect, CornerRadius::same(radius), bg, stroke, StrokeKind::Middle);
                }
                let label = if t.name.is_empty() && t.silent { "τ" } else { t.name.as_str() };
                canvas::text_centered(painter, c, rect.width() + 40.0 * z, label, label_size, pal.text);
            } else if sketch {
                painter.rect_filled(rect, 0.0, pal.fill);
                canvas::hachure_rect(painter, rect, fill, (3.5 * z).max(2.5), t.id as u64);
                canvas::sketch_rect(painter, rect, Stroke::new(1.5 * z.clamp(0.6, 1.6), fill), t.id as u64);
            } else {
                painter.rect_filled(rect, CornerRadius::same(radius), fill);
            }
            if sel || hov {
                painter.rect_stroke(
                    rect.expand(3.0 * z),
                    CornerRadius::same(radius + 3),
                    Stroke::new(2.0, pal.select),
                    StrokeKind::Outside,
                );
            }
            if !boxed {
                canvas::text_below(
                    painter,
                    pos2(c.x, rect.max.y + 4.0 * z),
                    150.0 * z,
                    &t.name,
                    label_size,
                    pal.text,
                    Some(pal.bg),
                );
            }
        }

        // ---- travelling tokens
        if let (Some(a), Some(p)) = (&anim, progress) {
            let tid = NodeRef::Transition(self.net.transitions[a.t].id);
            let ease = |x: f32| {
                let x = x.clamp(0.0, 1.0);
                x * x * (3.0 - 2.0 * x)
            };
            let (phase_in, phase_out) = (ease(p * 2.0), ease(p * 2.0 - 1.0));
            for arc in &self.net.arcs {
                let (incoming, frac) = if arc.to == tid && arc.kind == ArcKind::Normal {
                    (true, phase_in)
                } else if arc.from == tid {
                    (false, phase_out)
                } else {
                    continue;
                };
                if (incoming && p >= 0.5) || (!incoming && p < 0.5) {
                    continue;
                }
                let Some(path) = self.arc_path(arc.from, arc.to) else { continue };
                let pts: Vec<Pos2> = path.iter().map(|&q| v.to_screen(canvas, q)).collect();
                let pos = canvas::point_along(&pts, frac);
                let r = 4.2 * z.clamp(0.6, 1.6);
                painter.circle_filled(pos, r + 3.0, pal.select.gamma_multiply(0.18));
                painter.circle_filled(pos, r, pal.token);
                if arc.weight > 1 {
                    canvas::badge(painter, pos + vec2(9.0, -9.0) * z, &arc.weight.to_string(), 9.0, pal.select, pal);
                }
            }
        }
    }

    // ------------------------------------------------------------------
    // Side panel

    fn side_panel(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            crate::theme::segmented(
                ui,
                &mut self.tab,
                &[
                    (SideTab::Properties, tr("Properties")),
                    (SideTab::Matrices, tr("Matrices")),
                    (SideTab::Analysis, tr("Analysis")),
                    (SideTab::States, tr("States")),
                ],
            );
            let view = match self.tab {
                SideTab::Properties => None,
                SideTab::Matrices => Some(crate::dock::View::Matrices),
                SideTab::Analysis => Some(crate::dock::View::Analysis),
                SideTab::States => Some(crate::dock::View::States),
            };
            if let Some(v) = view
                && ui
                    .small_button("↗")
                    .on_hover_text(tr("Open this view in its own pane (side by side)"))
                    .clicked()
            {
                self.request_pane = Some(v);
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| match self.tab {
                SideTab::Properties => {
                    if self.sim.is_some() {
                        self.simulation_panel(ui);
                    } else {
                        self.properties(ui);
                    }
                }
                SideTab::Matrices => self.matrices(ui),
                SideTab::Analysis => self.analysis_panel(ui),
                SideTab::States => self.states_panel(ui),
            });
    }

    fn properties(&mut self, ui: &mut Ui) {
        if self.group.len() > 1 {
            self.group_panel(ui);
            return;
        }
        let editable = self.sim.is_none();
        ui.add_enabled_ui(editable, |ui| match self.selection {
            Some(Sel::Node(NodeRef::Place(id))) => {
                let Some(p) = self.net.place_mut(id) else { return };
                ui.heading(tr("Place"));
                egui::Grid::new("pp").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut p.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("Tokens (M0)"));
                    ui.add(egui::DragValue::new(&mut p.tokens).range(0..=100_000));
                    ui.end_row();
                    ui.label(tr("Capacity"));
                    ui.horizontal(|ui| {
                        let mut has = p.capacity.is_some();
                        if ui.checkbox(&mut has, "").changed() {
                            p.capacity = if has { Some(p.tokens.max(1)) } else { None };
                        }
                        if let Some(c) = &mut p.capacity {
                            ui.add(egui::DragValue::new(c).range(1..=100_000));
                        } else {
                            ui.label(RichText::new("∞").weak());
                        }
                    });
                    ui.end_row();
                });
            }
            Some(Sel::Node(NodeRef::Transition(id))) => {
                let Some(t) = self.net.transition_mut(id) else { return };
                ui.heading(tr("Transition"));
                egui::Grid::new("tp").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut t.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("Silent (τ)"));
                    ui.checkbox(&mut t.silent, "");
                    ui.end_row();
                });
                let n = NodeRef::Transition(id);
                let pre: Vec<String> = self
                    .net
                    .arcs
                    .iter()
                    .filter(|a| a.to == n)
                    .map(|a| weighted(self.net.node_name(a.from).unwrap_or("?"), a.weight))
                    .collect();
                let post: Vec<String> = self
                    .net
                    .arcs
                    .iter()
                    .filter(|a| a.from == n)
                    .map(|a| weighted(self.net.node_name(a.to).unwrap_or("?"), a.weight))
                    .collect();
                ui.add_space(6.0);
                ui.label(format!("•t = {{{}}}", pre.join(", ")));
                ui.label(format!("t• = {{{}}}", post.join(", ")));
            }
            Some(Sel::Arc(id)) => {
                let (from, to) = match self.net.arc(id) {
                    Some(a) => (
                        self.net.node_name(a.from).unwrap_or("?").to_string(),
                        self.net.node_name(a.to).unwrap_or("?").to_string(),
                    ),
                    None => return,
                };
                let Some(a) = self.net.arc_mut(id) else { return };
                ui.heading(tr("Arc"));
                ui.label(format!("{from} → {to}"));
                egui::Grid::new("ap").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("Weight"));
                    ui.add(egui::DragValue::new(&mut a.weight).range(1..=100_000));
                    ui.end_row();
                    if matches!(a.from, NodeRef::Place(_)) {
                        ui.label(tr("Kind"));
                        egui::ComboBox::from_id_salt("arck")
                            .selected_text(if a.kind == ArcKind::Normal { tr("Normal") } else { tr("Inhibitor") })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut a.kind, ArcKind::Normal, tr("Normal"));
                                ui.selectable_value(&mut a.kind, ArcKind::Inhibitor, tr("Inhibitor"));
                            });
                        ui.end_row();
                    }
                });
            }
            None => {
                ui.heading(tr("Petri net"));
                ui.horizontal(|ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut self.net.name).labelled_by(lbl.id);
                });
                ui.add_space(8.0);
                crate::theme::section_title(ui, tr("Tips"));
                ui.label(tr("• P / T: add places / transitions by clicking."));
                ui.label(tr(
                    "• A: drag from a node to another to add an arc (click again on an existing arc's ends to increase its weight).",
                ));
                ui.label(tr("• K: left-click a place to add a token, right-click to remove one."));
                ui.label(tr("• Double-click a node to rename it. Right-click for more actions."));
                ui.label(tr("• S: start the token game; click green transitions to fire them."));
            }
        });
    }

    fn group_panel(&mut self, ui: &mut Ui) {
        let places = self.group.iter().filter(|n| matches!(n, NodeRef::Place(_))).count();
        ui.label(RichText::new(trf("{} elements selected", &[&self.group.len()])).font(crate::theme::semibold(16.0)));
        ui.label(
            RichText::new(trf("{} place(s), {} transition(s)", &[&places, &(self.group.len() - places)]))
                .color(ui.visuals().weak_text_color()),
        );
        crate::theme::section_title(ui, tr("Arrange"));
        let pos: Vec<(NodeRef, Pos)> = self.group.iter().filter_map(|&n| self.net.node_pos(n).map(|p| (n, p))).collect();
        let k = pos.len().max(1) as f32;
        let (mx, my) = (
            pos.iter().map(|p| p.1.x).sum::<f32>() / k,
            pos.iter().map(|p| p.1.y).sum::<f32>() / k,
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr("Align horizontally"))
                .on_hover_text(tr("Same vertical position"))
                .clicked()
            {
                for (n, p) in &pos {
                    self.net.set_node_pos(*n, self.snap(Pos::new(p.x, my)));
                }
            }
            if ui
                .button(tr("Align vertically"))
                .on_hover_text(tr("Same horizontal position"))
                .clicked()
            {
                for (n, p) in &pos {
                    self.net.set_node_pos(*n, self.snap(Pos::new(mx, p.y)));
                }
            }
            if ui.button(tr("Distribute horizontally")).clicked() && pos.len() > 2 {
                let mut v = pos.clone();
                v.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
                let (x0, x1) = (v[0].1.x, v[v.len() - 1].1.x);
                for (i, (n, p)) in v.iter().enumerate() {
                    let x = x0 + (x1 - x0) * i as f32 / (v.len() - 1) as f32;
                    self.net.set_node_pos(*n, Pos::new(x, p.y));
                }
            }
            if ui.button(tr("Distribute vertically")).clicked() && pos.len() > 2 {
                let mut v = pos.clone();
                v.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
                let (y0, y1) = (v[0].1.y, v[v.len() - 1].1.y);
                for (i, (n, p)) in v.iter().enumerate() {
                    let y = y0 + (y1 - y0) * i as f32 / (v.len() - 1) as f32;
                    self.net.set_node_pos(*n, Pos::new(p.x, y));
                }
            }
        });
        crate::theme::section_title(ui, tr("Tokens"));
        ui.horizontal(|ui| {
            if ui.button(tr("+1 token")).clicked() {
                for n in self.group.clone() {
                    if let NodeRef::Place(id) = n
                        && let Some(p) = self.net.place_mut(id)
                    {
                        p.tokens += 1;
                    }
                }
            }
            if ui.button(tr("Clear tokens")).clicked() {
                for n in self.group.clone() {
                    if let NodeRef::Place(id) = n
                        && let Some(p) = self.net.place_mut(id)
                    {
                        p.tokens = 0;
                    }
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

    fn simulation_panel(&mut self, ui: &mut Ui) {
        let pal = Palette::for_ui(ui);
        ui.label(RichText::new(tr("Token game")).font(crate::theme::semibold(16.0)));
        ui.add_space(4.0);
        let marking = self.current_marking();
        let enabled = self.net.enabled_transitions(&marking);
        ui.horizontal(|ui| {
            let can_back = self.sim.as_ref().is_some_and(|s| !s.stack.is_empty());
            if ui
                .add_enabled(can_back, egui::Button::new(tr("⏴ Back")))
                .on_hover_text(tr("Undo the last firing"))
                .clicked()
                && let Some(s) = &mut self.sim
                && let Some(m) = s.stack.pop()
            {
                s.marking = m;
                s.trace.pop();
                s.anim = None;
            }
            if ui
                .add_enabled(!enabled.is_empty(), egui::Button::new(tr("Step ⏵")))
                .on_hover_text(tr("Fire a random enabled transition (Space)"))
                .clicked()
            {
                self.random_step();
            }
            if let Some(s) = &mut self.sim {
                let label = if s.auto { tr("⏸ Pause") } else { tr("⏵ Play") };
                if ui
                    .add(crate::theme::primary_button(ui, label))
                    .on_hover_text(tr("Fire random transitions automatically"))
                    .clicked()
                {
                    s.auto = !s.auto;
                }
                if ui.button("⟲").on_hover_text(tr("Back to M0")).clicked() {
                    s.marking = self.net.initial_marking();
                    s.trace.clear();
                    s.stack.clear();
                    s.anim = None;
                }
            }
        });
        if let Some(s) = &mut self.sim {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Speed")).color(ui.visuals().weak_text_color()));
                ui.add(egui::Slider::new(&mut s.interval, 0.05..=2.0).suffix(" s").logarithmic(true));
            });
        }

        crate::theme::section_title(ui, tr("Current marking"));
        ui.label(RichText::new(format!("M = {}", fmt_u32(&marking))).font(FontId::monospace(13.5)));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (i, p) in self.net.places.iter().enumerate() {
                if marking[i] > 0 {
                    pill(ui, &format!("{} · {}", p.name, marking[i]), pal.text, pal.muted);
                }
            }
        });

        crate::theme::section_title(ui, tr("Enabled transitions"));
        if enabled.is_empty() {
            pill(ui, tr("Dead marking: nothing can fire"), pal.bad, pal.bad);
        } else {
            let mut fire = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for &t in &enabled {
                    let b = egui::Button::new(
                        RichText::new(&self.net.transitions[t].name)
                            .color(pal.ok)
                            .font(crate::theme::medium(13.0)),
                    )
                    .fill(pal.ok.gamma_multiply(if pal.dark { 0.2 } else { 0.1 }))
                    .corner_radius(6.0);
                    if ui.add(b).on_hover_text(tr("Fire")).clicked() {
                        fire = Some(t);
                    }
                }
            });
            if let Some(t) = fire {
                self.fire(t);
            }
        }

        if let Some(s) = &self.sim {
            crate::theme::section_title(ui, &trf("Firing sequence σ ({})", &[&s.trace.len()]));
            if s.trace.is_empty() {
                ui.label(RichText::new(tr("ε (nothing fired yet)")).color(ui.visuals().weak_text_color()));
            } else {
                let trace: Vec<&str> = s.trace.iter().map(|&t| self.net.transitions[t].name.as_str()).collect();
                ui.add(egui::Label::new(RichText::new(trace.join(" → ")).font(FontId::monospace(13.0))).wrap());
            }
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui
                .button(tr("Use as M0"))
                .on_hover_text(tr("Make the current marking the initial marking"))
                .clicked()
                && let Some(s) = &self.sim
            {
                let m = s.marking.clone();
                self.net.set_initial_marking(&m);
            }
            if ui.button(tr("Stop")).clicked() {
                self.sim = None;
            }
        });
    }

    fn matrices(&mut self, ui: &mut Ui) {
        let places: Vec<String> = self.net.places.iter().map(|p| p.name.clone()).collect();
        let trans: Vec<String> = self.net.transitions.iter().map(|t| t.name.clone()).collect();
        if places.is_empty() || trans.is_empty() {
            ui.label(tr("Add places and transitions to see the matrices."));
            return;
        }
        let to_i = |m: Vec<Vec<u32>>| {
            m.into_iter()
                .map(|r| r.into_iter().map(i64::from).collect())
                .collect::<Vec<Vec<i64>>>()
        };
        let solutions: [Vec<Vec<i64>>; 3] = [
            to_i(self.net.pre_matrix()),
            to_i(self.net.post_matrix()),
            self.net.incidence_matrix(),
        ];
        let titles = [
            tr("Pre (W⁻): arcs place → transition"),
            tr("Post (W⁺): arcs transition → place"),
            tr("Incidence C = Post − Pre"),
        ];
        let short = ["pre", "post", "incidence"];
        let s_of = |m: &Vec<Vec<i64>>| {
            m.iter()
                .map(|r| r.iter().map(|x| x.to_string()).collect())
                .collect::<Vec<Vec<String>>>()
        };

        let mut exercise = self.exercise.is_some();
        ui.horizontal_wrapped(|ui| {
            crate::theme::segmented(ui, &mut exercise, &[(false, tr("Show")), (true, tr("Exercise: fill by hand"))]);
        });
        if exercise != self.exercise.is_some() {
            self.exercise = exercise.then(|| [0, 1, 2].map(|_| vec![vec![String::new(); trans.len()]; places.len()]));
            self.exercise_checked = false;
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr("Copy LaTeX"))
                .on_hover_text(tr("Pre, Post, C and M0 as LaTeX pmatrix"))
                .clicked()
            {
                let mut s = String::new();
                for (k, name) in ["Pre", "Post", "C"].iter().enumerate() {
                    s += &latex(name, &s_of(&solutions[k]));
                }
                let m0: Vec<Vec<String>> = self.net.initial_marking().iter().map(|x| vec![x.to_string()]).collect();
                s += &latex("M_0", &m0);
                ui.ctx().copy_text(s);
                self.status = tr("Matrices copied as LaTeX").into();
            }
            if ui
                .button(tr("Save CSV…"))
                .on_hover_text(tr("Pre, Post and C in one CSV file"))
                .clicked()
            {
                let mut out = String::new();
                for k in 0..3 {
                    out += &csv(titles[k], &places, &trans, &s_of(&solutions[k]));
                    out += "\n";
                }
                self.save_text(&format!("{}-matrices.csv", self.file_stem()), &out);
            }
        });

        let mut export: Option<(usize, bool, Rect)> = None;
        if let Some(answers) = &mut self.exercise {
            // ---- exercise mode
            for a in answers.iter_mut() {
                a.resize(places.len(), Vec::new());
                for row in a.iter_mut() {
                    row.resize(trans.len(), String::new());
                }
            }
            ui.label(RichText::new(tr("Type the values, then check. Empty cells count as 0.")).color(ui.visuals().weak_text_color()));
            let mut action = None;
            for k in 0..3 {
                let r = editable_matrix(
                    ui,
                    titles[k],
                    &places,
                    &trans,
                    &mut answers[k],
                    &solutions[k],
                    self.exercise_checked,
                );
                if let Some(a) = r {
                    action = Some((k, a));
                }
            }
            match action {
                Some((k, "fill")) => answers[k] = s_of(&solutions[k]),
                Some((k, "clear")) => {
                    for row in answers[k].iter_mut() {
                        row.iter_mut().for_each(|c| c.clear());
                    }
                }
                _ => {}
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(crate::theme::primary_button(ui, tr("Check my matrices"))).clicked() {
                    self.exercise_checked = true;
                }
                if self.exercise_checked && ui.button(tr("Hide corrections")).clicked() {
                    self.exercise_checked = false;
                }
            });
            if self.exercise_checked {
                let wrong: usize = (0..3)
                    .map(|k| {
                        answers[k]
                            .iter()
                            .enumerate()
                            .flat_map(|(i, r)| r.iter().enumerate().map(move |(j, v)| (i, j, v)))
                            .filter(|(i, j, v)| parse_cell(v) != Some(solutions[k][*i][*j]))
                            .count()
                    })
                    .sum();
                let pal = Palette::for_ui(ui);
                if wrong == 0 {
                    ui.label(
                        RichText::new(tr("✔ All three matrices are correct"))
                            .color(pal.ok)
                            .font(crate::theme::semibold(14.0)),
                    );
                } else {
                    ui.label(
                        RichText::new(trf("✖ {} wrong cell(s), highlighted in red", &[&wrong]))
                            .color(pal.bad)
                            .font(crate::theme::semibold(14.0)),
                    );
                }
            }
        } else {
            // ---- display mode
            for k in 0..3 {
                let (rect, png, csv_clicked) = matrix_grid(
                    ui,
                    titles[k],
                    &places,
                    &trans,
                    &s_of(&solutions[k]),
                    (k == 2).then_some(&solutions[2]),
                );
                if png {
                    export = Some((k, true, rect));
                }
                if csv_clicked {
                    export = Some((k, false, rect));
                }
            }
            let m0: Vec<Vec<String>> = self.net.initial_marking().iter().map(|x| vec![x.to_string()]).collect();
            let mut cols = vec!["M0".to_string()];
            let mut vals = m0.clone();
            if let Some(s) = &self.sim {
                cols.push("M".into());
                for (i, row) in vals.iter_mut().enumerate() {
                    row.push(s.marking[i].to_string());
                }
            }
            matrix_grid(ui, tr("Markings"), &places, &cols, &vals, None);
            ui.label(RichText::new(tr("State equation: M' = M + C·σ  (σ = Parikh vector of the firing sequence)")).weak());
        }
        if let Some((k, png, rect)) = export {
            let name = format!("{}-{}", self.file_stem(), short[k]);
            if png {
                crate::io::request_png_export(ui.ctx(), rect.expand(4.0), format!("{name}.png"));
            } else {
                let text = csv(titles[k], &places, &trans, &s_of(&solutions[k]));
                self.save_text(&format!("{name}.csv"), &text);
            }
        }
    }

    fn file_stem(&self) -> String {
        let s: String = self
            .net
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
            .collect();
        if s.trim_matches('_').is_empty() {
            "petri-net".into()
        } else {
            s.trim_matches('_').to_string()
        }
    }

    fn save_text(&mut self, name: &str, text: &str) {
        match crate::io::save_file(name, text.as_bytes()) {
            Ok(Some(p)) => self.status = trf("Exported {}", &[&(crate::io::file_name(&p))]),
            Ok(None) => {}
            Err(e) => self.status = trf("Export failed: {}", &[&e]),
        }
    }

    fn ensure_analysis(&mut self) {
        let stale = match &self.analysis {
            Some(a) => a.net != self.net || a.limit != self.state_limit,
            None => true,
        };
        if stale {
            let ss = StateSpace::explore(&self.net, self.state_limit);
            let beh = BehaviouralReport::compute(&self.net, &ss);
            self.analysis = Some(AnalysisCache {
                net: self.net.clone(),
                limit: self.state_limit,
                st: StructuralReport::compute(&self.net),
                pinv: analysis::p_invariants(&self.net),
                tinv: analysis::t_invariants(&self.net),
                sound: analysis::soundness(&self.net, self.state_limit),
                ss,
                beh,
            });
            self.graph_fit = true;
        }
    }

    fn analysis_panel(&mut self, ui: &mut Ui) {
        if self.net.places.is_empty() {
            ui.label(tr("Nothing to analyse yet."));
            return;
        }
        self.ensure_analysis();
        let a = self.analysis.as_ref().unwrap();
        let net = &self.net;
        let pal = Palette::for_ui(ui);
        let pname = |i: usize| net.places[i].name.as_str();
        let tname = |i: usize| net.transitions[i].name.as_str();

        ui.horizontal(|ui| {
            ui.label(tr("State limit"));
            ui.add(egui::DragValue::new(&mut self.state_limit).range(10..=1_000_000).speed(100));
        });

        ui.add_space(4.0);
        crate::theme::section_title(ui, tr("Behaviour"));
        let b = &a.beh;
        let kind = if a.ss.has_omega() {
            tr("coverability graph")
        } else {
            tr("reachability graph")
        };
        ui.label(trf(
            "{} states, {} arcs in the {}{}",
            &[
                &b.state_count,
                &b.edge_count,
                &kind,
                &(if b.truncated { tr(" (truncated!)") } else { "" }),
            ],
        ));
        egui::Grid::new("beh").num_columns(2).min_col_width(14.0).show(ui, |ui| {
            let k = b.place_bounds.iter().map(|x| x.unwrap_or(u64::MAX)).max().unwrap_or(0);
            let bounded_label = match b.bounded {
                Some(true) => trf("Bounded ({}-bounded)", &[&k]),
                _ => tr("Bounded").into(),
            };
            canvas::property_row(
                ui,
                &bounded_label,
                b.bounded,
                tr("Every place has a finite maximum number of tokens"),
            );
            canvas::property_row(ui, tr("Safe (1-bounded)"), b.safe, tr("No place ever holds more than one token"));
            canvas::property_row(
                ui,
                tr("Deadlock-free"),
                b.deadlock_free,
                tr("No reachable marking without enabled transitions"),
            );
            canvas::property_row(
                ui,
                tr("Live"),
                b.live,
                tr("Every transition can always fire again eventually (L4-live)"),
            );
            canvas::property_row(
                ui,
                tr("Reversible"),
                b.reversible,
                tr("M0 can be reached again from every reachable marking"),
            );
            let all_quasi = b.quasi_live.iter().all(|&x| x);
            canvas::property_row(
                ui,
                tr("Quasi-live (no dead transition)"),
                Some(all_quasi),
                tr("Every transition can fire at least once"),
            );
        });
        if b.truncated {
            ui.label(
                RichText::new(tr(
                    "The state space was truncated: some properties are undecided (?). Increase the limit.",
                ))
                .color(pal.warn),
            );
        }

        if !b.deadlocks.is_empty() {
            ui.add_space(4.0);
            crate::theme::section_title(ui, tr("Deadlocks"));
            // In a workflow net, [o] (one token in the sink place) is the expected final marking.
            let is_final = |m: &[u64]| a.st.workflow_net && m.iter().sum::<u64>() == 1 && m[a.st.sink_places[0]] == 1;
            for &d in b.deadlocks.iter().take(10) {
                let path = a.ss.path_to(d).unwrap_or_default();
                let seq: Vec<&str> = path.iter().map(|&t| tname(t)).collect();
                let m = &a.ss.states[d];
                ui.label(format!(
                    "M{d} = {}  via σ = {}",
                    format_marking(m),
                    if seq.is_empty() { "ε".into() } else { seq.join(" ") }
                ));
                if is_final(m) {
                    ui.label(RichText::new(tr("   ↳ final marking [o]: proper completion of the workflow (expected)")).color(pal.ok));
                }
            }
            if a.st.workflow_net {
                ui.label(
                    RichText::new(tr(
                        "For a workflow net, liveness/reversibility are checked on the short-circuited net: see Soundness below.",
                    ))
                    .weak(),
                );
            }
        }

        ui.add_space(4.0);
        egui::CollapsingHeader::new(tr("Place bounds & transitions"))
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("bounds").striped(true).show(ui, |ui| {
                    ui.label(RichText::new(tr("Place")).strong());
                    ui.label(RichText::new(tr("Bound")).strong());
                    ui.end_row();
                    for (i, bd) in b.place_bounds.iter().enumerate() {
                        ui.label(pname(i));
                        ui.label(bd.map(|x| x.to_string()).unwrap_or("ω".into()));
                        ui.end_row();
                    }
                });
                ui.add_space(4.0);
                egui::Grid::new("tlive").striped(true).show(ui, |ui| {
                    ui.label(RichText::new(tr("Transition")).strong());
                    ui.label(RichText::new(tr("Quasi-live")).strong());
                    ui.label(RichText::new(tr("Live")).strong());
                    ui.end_row();
                    for (i, q) in b.quasi_live.iter().enumerate() {
                        ui.label(tname(i));
                        ui.label(if *q { "✔" } else { tr("✖ dead") });
                        ui.label(match &b.live_transitions {
                            Some(l) => {
                                if l[i] {
                                    "✔"
                                } else {
                                    "✖"
                                }
                            }
                            None => "?",
                        });
                        ui.end_row();
                    }
                });
            });

        ui.add_space(6.0);
        crate::theme::section_title(ui, tr("Invariants"));
        let m0 = net.initial_marking();
        match &a.pinv {
            Some(v) if !v.is_empty() => {
                ui.label(trf(
                    "P-invariants ({}){}",
                    &[
                        &v.len(),
                        &(if analysis::covered(v, net.places.len()) {
                            tr(" — net covered: structurally bounded")
                        } else {
                            ""
                        }),
                    ],
                ));
                for inv in v {
                    let terms: Vec<String> = inv
                        .iter()
                        .enumerate()
                        .filter(|(_, x)| **x != 0)
                        .map(|(i, &x)| {
                            if x == 1 {
                                format!("M({})", pname(i))
                            } else {
                                format!("{x}·M({})", pname(i))
                            }
                        })
                        .collect();
                    let k: i64 = inv.iter().zip(&m0).map(|(a, &b)| a * b as i64).sum();
                    ui.label(RichText::new(format!("{} = {k}", terms.join(" + "))).monospace());
                }
            }
            Some(_) => {
                ui.label(tr("No semi-positive P-invariant."));
            }
            None => {
                ui.label(tr("P-invariants: too many to compute."));
            }
        }
        match &a.tinv {
            Some(v) if !v.is_empty() => {
                ui.label(trf(
                    "T-invariants ({}){}",
                    &[
                        &v.len(),
                        &(if analysis::covered(v, net.transitions.len()) {
                            tr(" — net covered")
                        } else {
                            ""
                        }),
                    ],
                ));
                for inv in v {
                    let terms: Vec<String> = inv
                        .iter()
                        .enumerate()
                        .filter(|(_, x)| **x != 0)
                        .map(|(i, &x)| {
                            if x == 1 {
                                tname(i).to_string()
                            } else {
                                format!("{x}·{}", tname(i))
                            }
                        })
                        .collect();
                    ui.label(RichText::new(terms.join(" + ")).monospace());
                }
            }
            Some(_) => {
                ui.label(tr("No semi-positive T-invariant (the net cannot return to a marking)."));
            }
            None => {
                ui.label(tr("T-invariants: too many to compute."));
            }
        }

        ui.add_space(6.0);
        crate::theme::section_title(ui, tr("Structure"));
        let s = &a.st;
        egui::Grid::new("struct").num_columns(2).min_col_width(14.0).show(ui, |ui| {
            canvas::property_row(ui, tr("Ordinary (all weights = 1)"), Some(s.ordinary), "");
            canvas::property_row(ui, tr("Pure (no self-loop)"), Some(s.pure), "");
            canvas::property_row(
                ui,
                tr("State machine"),
                Some(s.state_machine),
                tr("Each transition has exactly one input and one output place"),
            );
            canvas::property_row(
                ui,
                tr("Marked graph"),
                Some(s.marked_graph),
                tr("Each place has exactly one input and one output transition"),
            );
            canvas::property_row(
                ui,
                tr("Free-choice"),
                Some(s.free_choice),
                tr("Conflicting transitions have a single input place"),
            );
            canvas::property_row(ui, tr("Extended free-choice"), Some(s.extended_free_choice), "");
            canvas::property_row(
                ui,
                tr("Workflow net"),
                Some(s.workflow_net),
                tr("One source place i, one sink place o, every node on a path from i to o"),
            );
        });
        if s.has_inhibitor_arcs {
            ui.label(RichText::new(tr("Inhibitor arcs are ignored by matrices and invariants.")).weak());
        }
        if s.workflow_net {
            ui.add_space(4.0);
            match &a.sound {
                Soundness::Sound => {
                    ui.label(RichText::new(tr("✔ Sound workflow net")).color(pal.ok).strong());
                }
                Soundness::Unsound(why) => {
                    ui.label(RichText::new(tr("✖ Not sound")).color(pal.bad).strong());
                    for w in why {
                        ui.label(format!("• {w}"));
                    }
                }
                Soundness::Unknown => {
                    ui.label(RichText::new(tr("? Soundness undecided (increase the state limit)")).color(pal.warn));
                }
                Soundness::NotWorkflowNet => {}
            }
        }
    }

    fn states_panel(&mut self, ui: &mut Ui) {
        if self.net.places.is_empty() {
            ui.label(tr("Nothing to explore yet."));
            return;
        }
        self.ensure_analysis();
        ui.horizontal(|ui| {
            if ui.button(tr("Open reachability graph")).clicked() {
                self.graph_open = true;
                self.graph_fit = true;
            }
            if ui
                .button(tr("↗ In a pane"))
                .on_hover_text(tr("Show the graph in its own pane"))
                .clicked()
            {
                self.request_pane = Some(crate::dock::View::Graph);
            }
        });
        let a = self.analysis.as_ref().unwrap();
        ui.label(trf(
            "{} states, {} transitions{}",
            &[
                &a.ss.states.len(),
                &a.ss.edges.len(),
                &(if a.ss.truncated { tr(" (truncated)") } else { "" }),
            ],
        ));
        ui.add_space(4.0);
        let tname = |i: usize| self.net.transitions[i].name.as_str();
        let mut clicked = None;
        egui::Grid::new("states").striped(true).show(ui, |ui| {
            ui.label(RichText::new(tr("State")).strong());
            ui.label(RichText::new(tr("Marking")).strong());
            ui.label(RichText::new(tr("Successors")).strong());
            ui.end_row();
            for (i, m) in a.ss.states.iter().enumerate().take(500) {
                if ui.selectable_label(self.selected_state == Some(i), format!("M{i}")).clicked() {
                    clicked = Some(i);
                }
                ui.label(RichText::new(format_marking(m)).monospace());
                let succ: Vec<String> =
                    a.ss.edges
                        .iter()
                        .filter(|e| e.0 == i)
                        .map(|e| format!("{}→M{}", tname(e.1), e.2))
                        .collect();
                ui.label(if succ.is_empty() {
                    tr("— dead —").to_string()
                } else {
                    succ.join(", ")
                });
                ui.end_row();
            }
        });
        if a.ss.states.len() > 500 {
            ui.label(tr("(only the first 500 states are listed)"));
        }
        if let Some(i) = clicked {
            self.select_state(i);
        }
    }

    /// Selects a state; in simulation mode, jumps to it (replaying the shortest path).
    fn select_state(&mut self, i: usize) {
        self.selected_state = Some(i);
        let Some(a) = &self.analysis else { return };
        if a.ss.states[i].contains(&OMEGA) {
            return;
        }
        if let (Some(path), Some(sim)) = (a.ss.path_to(i), &mut self.sim) {
            sim.marking = self.net.initial_marking();
            sim.trace.clear();
            sim.stack.clear();
            for t in path {
                self.fire(t);
            }
        }
    }

    fn graph_window(&mut self, ui: &mut Ui) {
        self.ensure_analysis();
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let canvas = response.rect;
        let painter = painter.with_clip_rect(canvas);
        let pal = Palette::for_ui(ui);
        let a = self.analysis.as_ref().unwrap();
        let ss = &a.ss;

        // Layered layout by BFS depth
        let maxd = ss.depth.iter().copied().max().unwrap_or(0);
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); maxd + 1];
        for (i, &d) in ss.depth.iter().enumerate() {
            layers[d].push(i);
        }
        let mut pos = vec![Pos::default(); ss.states.len()];
        // Box width: widest label at zoom 1 (monospace 11 px)
        let label_w = ss
            .states
            .iter()
            .enumerate()
            .take(2000)
            .map(|(i, m)| {
                painter
                    .layout_no_wrap(format!("M{i} {}", format_marking(m)), FontId::monospace(11.0), pal.text)
                    .size()
                    .x
            })
            .fold(0.0f32, f32::max)
            + 16.0;
        let dx = label_w.max(70.0) + 30.0;
        let widest = layers.iter().map(|l| l.len()).max().unwrap_or(1) as f32;
        for (d, layer) in layers.iter().enumerate() {
            let offset = (widest - layer.len() as f32) * dx / 2.0;
            for (k, &i) in layer.iter().enumerate() {
                pos[i] = Pos::new(offset + k as f32 * dx, d as f32 * 110.0);
            }
        }
        if self.graph_fit && !pos.is_empty() {
            let min = Pos::new(-dx / 2.0, -30.0);
            let max = Pos::new(widest * dx, maxd as f32 * 110.0 + 30.0);
            self.graph_view.fit(canvas, min, max);
            self.graph_fit = false;
        }
        self.graph_view.handle_navigation(ui, &response, canvas);
        if response.dragged_by(PointerButton::Primary) {
            self.graph_view.offset += response.drag_delta();
        }
        let v = self.graph_view;
        let z = v.zoom;
        painter.rect_filled(canvas, 0.0, pal.bg);
        // Text scales exactly with the boxes; when too small, only the state name is shown.
        let font = FontId::monospace(11.0 * z);
        let full_labels = 11.0 * z >= 5.5;
        let edge_labels = 11.0 * z >= 6.0;
        let box_size = vec2(label_w.max(70.0), 26.0) * z;
        let current = self.sim.as_ref().map(|s| s.marking.iter().map(|&x| x as u64).collect::<Vec<u64>>());

        for &(from, t, to) in &ss.edges {
            let (a2, b2) = (v.to_screen(canvas, pos[from]), v.to_screen(canvas, pos[to]));
            let tn = &self.net.transitions[t].name;
            if from == to {
                let c = a2 + vec2(box_size.x / 2.0 + 8.0 * z, 0.0);
                painter.circle_stroke(c, 9.0 * z, Stroke::new(1.2, pal.muted));
                if edge_labels {
                    canvas::label_with_halo(&painter, c + vec2(14.0 * z, 0.0), tn, 11.0 * z, &pal);
                }
                continue;
            }
            let rf = Rect::from_center_size(a2, box_size);
            let rt = Rect::from_center_size(b2, box_size);
            let s = clip_rect(rf, b2);
            let e = clip_rect(rt, a2);
            let dir = e - s;
            let back = ss.depth[to] <= ss.depth[from];
            let n = vec2(-dir.y, dir.x).normalized() * if back { 25.0 * z } else { 0.0 };
            let ctrl1 = s + dir * 0.33 + n;
            let ctrl2 = s + dir * 0.66 + n;
            canvas::bezier(&painter, [s, ctrl1, ctrl2, e], Stroke::new(1.3, pal.muted));
            canvas::arrow_head(&painter, e, e - ctrl2, 8.0 * z.max(0.6), pal.muted, true);
            let mid = s.lerp(e, 0.5) + n * 0.75;
            if edge_labels {
                canvas::label_with_halo(&painter, mid, tn, 11.0 * z, &pal);
            }
        }
        let mut clicked = None;
        let hover = response.hover_pos();
        for (i, m) in ss.states.iter().enumerate() {
            let c = v.to_screen(canvas, pos[i]);
            let r = Rect::from_center_size(c, box_size);
            let is_dead = a.beh.deadlocks.contains(&i);
            let is_current = current.as_ref() == Some(m);
            let fill = if is_current { pal.ok.gamma_multiply(0.35) } else { pal.fill };
            let stroke = if self.selected_state == Some(i) {
                Stroke::new(2.4, pal.select)
            } else if is_dead {
                Stroke::new(2.0, pal.bad)
            } else if i == 0 {
                Stroke::new(2.0, pal.ink)
            } else {
                Stroke::new(1.0, pal.ink)
            };
            painter.rect(r, CornerRadius::same(6), fill, stroke, StrokeKind::Middle);
            let text = if full_labels {
                format!("M{i} {}", format_marking(m))
            } else {
                format!("M{i}")
            };
            let size = if full_labels {
                font.clone()
            } else {
                FontId::monospace((11.0 * z).max(7.0).min(box_size.y * 0.8))
            };
            painter.text(c, egui::Align2::CENTER_CENTER, text, size, pal.text);
            if response.clicked() && hover.is_some_and(|h| r.contains(h)) {
                clicked = Some(i);
            }
        }
        painter.text(
            canvas.left_bottom() + vec2(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            tr("Thick border: M0 · red: deadlock · green: current marking · click a state to select (jumps there in simulation)"),
            FontId::proportional(11.0),
            pal.muted,
        );
        if let Some(i) = clicked {
            self.select_state(i);
        }
    }
}

fn clip_rect(r: Rect, toward: Pos2) -> Pos2 {
    let c = r.center();
    let d = toward - c;
    if d.length() < 1e-3 {
        return c;
    }
    let tx = if d.x.abs() > 1e-6 {
        r.width() / 2.0 / d.x.abs()
    } else {
        f32::INFINITY
    };
    let ty = if d.y.abs() > 1e-6 {
        r.height() / 2.0 / d.y.abs()
    } else {
        f32::INFINITY
    };
    c + d * tx.min(ty)
}

fn weighted(name: &str, w: u32) -> String {
    if w == 1 { name.to_string() } else { format!("{w}·{name}") }
}

fn fmt_u32(m: &[u32]) -> String {
    format!("({})", m.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(", "))
}

fn latex(name: &str, m: &[Vec<String>]) -> String {
    let rows: Vec<String> = m.iter().map(|r| r.join(" & ")).collect();
    format!("{name} = \\begin{{pmatrix}}\n{}\n\\end{{pmatrix}}\n\n", rows.join(" \\\\\n"))
}

/// Read-only matrix table. Returns (frame rect, PNG clicked, CSV clicked).
fn matrix_grid(
    ui: &mut Ui,
    title: &str,
    rows: &[String],
    cols: &[String],
    vals: &[Vec<String>],
    signed: Option<&Vec<Vec<i64>>>,
) -> (Rect, bool, bool) {
    let pal = Palette::for_ui(ui);
    let (mut png, mut csv_clicked) = (false, false);
    ui.horizontal(|ui| {
        crate::theme::section_title(ui, title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
            csv_clicked = ui.small_button("CSV").on_hover_text(tr("Save this matrix as CSV")).clicked();
            png = ui.small_button("PNG").on_hover_text(tr("Save this matrix as an image")).clicked();
        });
    });
    let resp = egui::Frame::new()
        .fill(ui.visuals().window_fill)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal().id_salt(title).show(ui, |ui| {
                egui::Grid::new(title)
                    .striped(true)
                    .min_col_width(30.0)
                    .spacing([10.0, 5.0])
                    .show(ui, |ui| {
                        ui.label("");
                        for c in cols {
                            ui.label(RichText::new(c).font(crate::theme::medium(12.0)).color(pal.muted));
                        }
                        ui.end_row();
                        for (i, r) in rows.iter().enumerate() {
                            ui.label(RichText::new(r).font(crate::theme::medium(12.0)).color(pal.muted));
                            for (j, v) in vals[i].iter().enumerate() {
                                let mut t = RichText::new(v).font(FontId::monospace(13.5));
                                if let Some(s) = signed {
                                    t = match s[i][j].signum() {
                                        1 => t.color(pal.ok),
                                        -1 => t.color(pal.bad),
                                        _ => t.color(pal.muted.gamma_multiply(0.6)),
                                    };
                                } else if v == "0" {
                                    t = t.color(pal.muted.gamma_multiply(0.6));
                                }
                                ui.label(t);
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    (resp.response.rect, png, csv_clicked)
}

fn parse_cell(v: &str) -> Option<i64> {
    let t = v.trim().replace('−', "-");
    if t.is_empty() { Some(0) } else { t.parse().ok() }
}

/// Matrix whose cells are typed by the student. Returns "fill" / "clear" requests.
fn editable_matrix(
    ui: &mut Ui,
    title: &str,
    rows: &[String],
    cols: &[String],
    vals: &mut [Vec<String>],
    solution: &[Vec<i64>],
    checked: bool,
) -> Option<&'static str> {
    let pal = Palette::for_ui(ui);
    let mut action = None;
    ui.horizontal(|ui| {
        crate::theme::section_title(ui, title);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Max), |ui| {
            if ui.small_button(tr("Clear")).clicked() {
                action = Some("clear");
            }
            if ui
                .small_button(tr("Fill for me"))
                .on_hover_text(tr("Let the app fill this matrix"))
                .clicked()
            {
                action = Some("fill");
            }
        });
    });
    egui::Frame::new()
        .fill(ui.visuals().window_fill)
        .stroke(Stroke::new(1.0, pal.border))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal().id_salt(("edit", title)).show(ui, |ui| {
                egui::Grid::new(("edit", title))
                    .min_col_width(34.0)
                    .spacing([6.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("");
                        for c in cols {
                            ui.label(RichText::new(c).font(crate::theme::medium(12.0)).color(pal.muted));
                        }
                        ui.end_row();
                        for (i, r) in rows.iter().enumerate() {
                            let lbl = ui.label(RichText::new(r).font(crate::theme::medium(12.0)).color(pal.muted));
                            for j in 0..cols.len() {
                                let ok = parse_cell(&vals[i][j]) == Some(solution[i][j]);
                                let mut te = egui::TextEdit::singleline(&mut vals[i][j])
                                    .desired_width(30.0)
                                    .font(FontId::monospace(13.5))
                                    .align(egui::Align2::CENTER_CENTER);
                                if checked {
                                    te = te.background_color(if ok {
                                        pal.ok.gamma_multiply(0.18)
                                    } else {
                                        pal.bad.gamma_multiply(0.22)
                                    });
                                }
                                let r = ui.add(te).labelled_by(lbl.id);
                                if checked && !ok {
                                    r.on_hover_text(trf("expected {}", &[&(solution[i][j])]));
                                }
                            }
                            ui.end_row();
                        }
                    });
            });
        });
    action
}

fn csv(title: &str, rows: &[String], cols: &[String], vals: &[Vec<String>]) -> String {
    let esc = |v: &str| {
        if v.contains([',', '"', ';']) {
            format!("\"{}\"", v.replace('"', "\"\""))
        } else {
            v.to_string()
        }
    };
    let mut out = format!("{}\n,{}\n", esc(title), cols.iter().map(|c| esc(c)).collect::<Vec<_>>().join(","));
    for (i, r) in rows.iter().enumerate() {
        out += &format!("{},{}\n", esc(r), vals[i].join(","));
    }
    out
}

fn draw_tokens(painter: &Painter, c: Pos2, z: f32, k: u32, color: Color32) {
    let r = 3.6 * z;
    let o = 6.8 * z;
    let pts: &[(f32, f32)] = match k {
        0 => &[],
        1 => &[(0.0, 0.0)],
        2 => &[(-0.8, 0.0), (0.8, 0.0)],
        3 => &[(0.0, -0.9), (-0.85, 0.6), (0.85, 0.6)],
        4 => &[(-0.8, -0.8), (0.8, -0.8), (-0.8, 0.8), (0.8, 0.8)],
        5 => &[(-0.95, -0.95), (0.95, -0.95), (0.0, 0.0), (-0.95, 0.95), (0.95, 0.95)],
        _ => {
            painter.text(
                c,
                egui::Align2::CENTER_CENTER,
                k.to_string(),
                crate::theme::semibold(15.0 * z),
                color,
            );
            return;
        }
    };
    for (x, y) in pts {
        painter.circle_filled(c + vec2(*x, *y) * o, r, color);
    }
}

fn paint_tool_icon(p: &Painter, r: Rect, c: Color32, tool: Tool, boxed: bool) {
    let s = Stroke::new(1.6, c);
    let ctr = r.center();
    match tool {
        Tool::Select => {
            let pts = vec![
                ctr + vec2(-6.0, -10.0),
                ctr + vec2(7.0, 2.0),
                ctr + vec2(1.0, 3.0),
                ctr + vec2(4.0, 10.0),
                ctr + vec2(1.0, 11.0),
                ctr + vec2(-2.0, 4.0),
                ctr + vec2(-6.0, 8.0),
            ];
            p.add(egui::Shape::closed_line(pts, s));
        }
        Tool::Place => {
            p.circle_stroke(ctr, 10.0, s);
        }
        Tool::Transition if boxed => {
            p.rect_stroke(
                Rect::from_center_size(ctr, vec2(20.0, 12.0)),
                2.0,
                Stroke::new(1.6, c),
                StrokeKind::Middle,
            );
        }
        Tool::Transition => {
            p.rect_filled(Rect::from_center_size(ctr, vec2(6.0, 20.0)), 0.0, c);
        }
        Tool::Arc => {
            let a = ctr + vec2(-10.0, 8.0);
            let b = ctr + vec2(10.0, -8.0);
            p.line_segment([a, b - (b - a).normalized() * 5.0], s);
            canvas::arrow_head(p, b, b - a, 8.0, c, true);
        }
        Tool::Token => {
            p.circle_stroke(ctr, 10.0, s);
            p.circle_filled(ctr, 3.5, c);
        }
    }
}

/// Rounded tag with a tinted background.
fn pill(ui: &mut Ui, text: &str, text_color: Color32, tint: Color32) {
    egui::Frame::new()
        .fill(tint.gamma_multiply(if ui.visuals().dark_mode { 0.22 } else { 0.10 }))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).color(text_color));
        });
}
