//! Petri net editor: modelling, token game, matrices, analysis, reachability graph.

mod render;

mod panels;

mod interaction;

mod session;
pub(crate) use session::register;
pub(crate) mod analysis_job;

use crate::editor::layout::{CanvasControls, DiagramLayout, FooterStatus};

use crate::canvas::{self, InlineOutcome, Palette, Rng, View};
use crate::history::History;
use egui::{Color32, CornerRadius, FontId, Key, Painter, PointerButton, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use op_core::analysis::{self, BehaviouralReport, Soundness, StateSpace, StructuralReport, format_marking};
use op_core::geom::{Pos, dist_to_polyline};
use op_core::i18n::{tr, trf};
use op_core::petri::{ArcKind, Id, Marking, NodeRef, OMEGA, PetriNet};

const PLACE_R: f32 = 8.25;
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

#[derive(serde::Serialize, serde::Deserialize)]
struct AnalysisCache {
    lang: op_core::i18n::Lang,
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
    analysis_job: Option<analysis_job::PendingAnalysis>,
    analysis_cancelled: Option<(PetriNet, usize)>,
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
    graph_key: Option<u64>,
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
            analysis_job: None,
            analysis_cancelled: None,
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
            graph_key: None,
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
        let layout = DiagramLayout::new(ui, self.uid, self.show_inspector);
        layout.inspector(ui, |ui| self.side_panel(ui));
        self.status_bar(ui, layout);
        layout.canvas(ui, |ui| self.canvas(ui));
        if self.graph_open {
            let mut open = true;
            egui::Window::new(tr("petri.reachability_graph"))
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
                (Tool::Select, tr("bpmn.select_move_v_drag_empty_space_pan")),
                (Tool::Place, tr("petri.place_p")),
                (Tool::Transition, tr("petri.transition_t")),
                (Tool::Arc, tr("petri.arc_drag_from_node_another_click_both")),
                (Tool::Token, tr("petri.tokens_k_left_click_1_right_click")),
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
            let r = canvas::icon_button(ui, sim_on, tr("petri.token_game_simulation_s"), |p, r, c| {
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
                self.status = tr("petri.dead_marking_reached_no_transition_enabled").into();
            }
        }
        if let Some(s) = &self.sim {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f32(s.interval.min(0.1)));
        }
    }

    fn status_bar(&mut self, ui: &mut Ui, layout: DiagramLayout) {
        let status = FooterStatus {
            summary: trf(
                "petri.places_transitions_arcs",
                &[&self.net.places.len(), &self.net.transitions.len(), &self.net.arcs.len()],
            ),
            detail: self.status.clone(),
            indicator: self.sim.as_ref().map(|_| (tr("petri.simulation").into(), Palette::for_ui(ui).ok)),
            ..Default::default()
        };
        layout.configuration(
            ui,
            CanvasControls {
                view: &mut self.view,
                canvas: self.last_canvas,
                fit: &mut self.fit_request,
                snap: Some(&mut self.snap),
                inspector: &mut self.show_inspector,
                issues: None,
            },
            status,
            |ui| canvas::style_menu(ui, &mut self.net.style, true),
        );
    }

    // ------------------------------------------------------------------
    // Canvas

    /// Renders one of the analysis views on its own (for a docked pane).
    pub fn view_ui(&mut self, ui: &mut Ui, view: crate::dock::View) {
        self.now = ui.input(|i| i.time);
        match view.id() {
            "petri.graph" => self.graph_window(ui),
            "editor" => self.ui(ui),
            _ => {
                egui::Frame::new().inner_margin(egui::Margin::symmetric(16, 10)).show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| match view.id() {
                            "petri.matrices" => self.matrices(ui),
                            "petri.analysis" => self.analysis_panel(ui),
                            "petri.states" => self.states_panel(ui),
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
                tr("petri.empty_petri_net"),
                crate::theme::semibold(17.0),
                pal.text,
            );
            painter.text(
                canvas.center() + vec2(0.0, 14.0),
                egui::Align2::CENTER_CENTER,
                tr("petri.pick_place_p_transition_t_tool_click"),
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
                        ui.label(RichText::new(tr("petri.token_game")).font(crate::theme::semibold(13.0)));
                        ui.label(
                            RichText::new(if enabled == 0 {
                                trf("petri.dead_marking_firing_s", &[&fired])
                            } else {
                                trf("petri.click_green_transition_enabled_firing_s", &[&enabled, &fired])
                            })
                            .color(ui.visuals().weak_text_color()),
                        );
                        if ui.button(tr("petri.stop")).clicked() {
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
            csv_clicked = ui.small_button("CSV").on_hover_text(tr("petri.save_matrix_csv")).clicked();
            png = ui.small_button("PNG").on_hover_text(tr("petri.save_matrix_image")).clicked();
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
            if ui.small_button(tr("common.clear")).clicked() {
                action = Some("clear");
            }
            if ui
                .small_button(tr("mining.fill_me"))
                .on_hover_text(tr("petri.let_app_fill_matrix"))
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
                                    r.on_hover_text(trf("petri.expected", &[&(solution[i][j])]));
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

fn draw_tokens(painter: &Painter, c: Pos2, place_radius: f32, k: u32, color: Color32) {
    // Keep token positions and sizes proportional when the place size changes.
    let z = place_radius / 22.0;
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
        Tool::Select => crate::icons::select(p, r, c),
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
