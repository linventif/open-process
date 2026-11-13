//! Shared interactions for two typed notations: EPC and UML activity.
//! Models and rules stay in op-core; all graphical chrome stays in DiagramLayout.
use crate::canvas::{self, Palette, View};
use crate::editor::layout::{CanvasControls, DiagramLayout, FooterStatus};
use crate::editor::*;
use crate::history::History;
use egui::{Align2, Color32, FontId, Key, PointerButton, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use op_core::activity::ActivityKind;
use op_core::document::{Document, DocumentCodec, JsonCodec, ReadError};
use op_core::epc::EpcKind;
use op_core::flow::{Diagram, FlowKind, Id, Shape};
use op_core::geom::{Pos, dist_to_segment};
use op_core::i18n::{key, tr, trf};
use serde::{Serialize, de::DeserializeOwned};
use std::collections::BTreeSet;

pub trait Notation: FlowKind + Serialize + DeserializeOwned {
    const ID: &'static str;
    const NEW: &'static str;
    const TITLE: &'static str;
    const DESCRIPTION: &'static str;
    const NAME: &'static str;
    const BADGE: &'static str;
    const COLOR: Color32;
    const ICON: crate::icons::Icon;
    const EXAMPLE: &'static str;
    const EXAMPLE_LABEL: &'static str;
    fn example() -> Diagram<Self>;
}
impl Notation for EpcKind {
    const ID: &'static str = "epc";
    const NEW: &'static str = key("flow.new_epc_diagram");
    const TITLE: &'static str = key("flow.model_epc");
    const DESCRIPTION: &'static str = key("flow.describe_event_driven_processes_functions_logical_connectors");
    const NAME: &'static str = key("flow.epc_diagram");
    const BADGE: &'static str = "EPC";
    const COLOR: Color32 = Color32::from_rgb(37, 139, 102);
    const ICON: crate::icons::Icon = crate::icons::epc;
    const EXAMPLE: &'static str = "epc.order";
    const EXAMPLE_LABEL: &'static str = key("flow.epc_order_processing");
    fn example() -> Diagram<Self> {
        op_core::epc::example()
    }
}
impl Notation for ActivityKind {
    const ID: &'static str = "uml.activity";
    const NEW: &'static str = key("flow.new_activity_diagram");
    const TITLE: &'static str = key("flow.design_activity_diagram");
    const DESCRIPTION: &'static str = key("flow.describe_actions_decisions_parallel_branches_uml");
    const NAME: &'static str = key("flow.activity_diagram");
    const BADGE: &'static str = "UML · ACTIVITY";
    const COLOR: Color32 = Color32::from_rgb(167, 100, 42);
    const ICON: crate::icons::Icon = crate::icons::activity;
    const EXAMPLE: &'static str = "uml.activity.order";
    const EXAMPLE_LABEL: &'static str = key("flow.uml_activity_order_approval");
    fn example() -> Diagram<Self> {
        op_core::activity::example()
    }
}

#[derive(Clone)]
enum Drag {
    Move { start: Pos, origins: Vec<(Id, Pos)> },
    Resize(Id),
    Connect(Id),
    Rubber { start: Pos, base: BTreeSet<Id> },
}

#[derive(Clone)]
pub struct FlowView<K> {
    uid: u64,
    camera: View,
    canvas: Rect,
    fit: bool,
    snap: bool,
    inspector: Option<bool>,
    tool: Option<K>,
    connecting: bool,
    pending: Option<Id>,
    selected: BTreeSet<Id>,
    flow: Option<Id>,
    drag: Option<Drag>,
    export_png: bool,
    tab: bool,
    status: String,
}
impl<K> Default for FlowView<K> {
    fn default() -> Self {
        Self {
            uid: canvas::next_uid(),
            camera: View::default(),
            canvas: Rect::NOTHING,
            fit: true,
            snap: true,
            inspector: None,
            tool: None,
            connecting: false,
            pending: None,
            selected: BTreeSet::new(),
            flow: None,
            drag: None,
            export_png: false,
            tab: false,
            status: String::new(),
        }
    }
}
pub struct FlowEditor<K: Notation> {
    pub diagram: Diagram<K>,
    history: History<Diagram<K>>,
    state: FlowView<K>,
}
impl<K: Notation> FlowEditor<K> {
    pub fn new(diagram: Diagram<K>) -> Self {
        Self {
            history: History::new(&diagram),
            diagram,
            state: Default::default(),
        }
    }
    fn clear_selection(&mut self) {
        self.state.selected.clear();
        self.state.flow = None;
        self.state.pending = None;
        self.state.drag = None;
    }
    fn delete(&mut self) {
        self.history.sync(&self.diagram, false);
        self.diagram.delete(&self.state.selected, self.state.flow);
        self.clear_selection();
        self.history.sync(&self.diagram, false);
    }
    fn hit(&self, p: Pos) -> Option<Id> {
        self.diagram.nodes.iter().rev().find(|n| n.contains(p)).map(|n| n.id)
    }
    fn connect(&mut self, from: Id, to: Id) {
        self.history.sync(&self.diagram, false);
        if let Some(id) = self.diagram.connect(from, to, "") {
            self.state.flow = Some(id);
            self.state.selected.clear();
            self.state.status.clear();
        } else {
            self.state.status = tr("uml.relation_not_valid_these_elements").into();
        }
        self.state.pending = None;
        self.history.sync(&self.diagram, false);
    }
    fn properties(&mut self, ui: &mut Ui) {
        DiagramLayout::inspector_tabs(
            ui,
            &mut self.state.tab,
            &[(false, tr("common.properties")), (true, tr("common.validation"))],
            |_, _| {},
        );
        DiagramLayout::inspector_body(ui, |ui| {
            if self.state.tab {
                let issues = self.diagram.issues();
                if issues.is_empty() {
                    ui.label(tr("database.no_structural_issues"));
                }
                for issue in issues {
                    ui.colored_label(ui.visuals().warn_fg_color, issue);
                }
            } else if let Some(n) = self.diagram.nodes.iter_mut().find(|n| self.state.selected.contains(&n.id)) {
                ui.heading(tr(n.kind.label()));
                ui.label(tr("common.name"));
                ui.text_edit_singleline(&mut n.name);
                ui.add_space(8.0);
                ui.label(tr("common.size"));
                ui.horizontal(|ui| {
                    ui.add(egui::DragValue::new(&mut n.bounds.w).range(20.0..=2000.0));
                    ui.add(egui::DragValue::new(&mut n.bounds.h).range(8.0..=2000.0));
                });
                ui.label(tr("flow.drag_bottom_right_handle_resize"));
            } else if let Some(f) = self.diagram.flows.iter_mut().find(|f| Some(f.id) == self.state.flow) {
                ui.heading(tr("flow.flow"));
                ui.label(tr("flow.label_guard"));
                ui.text_edit_singleline(&mut f.label);
            } else {
                ui.heading(tr("document.diagram"));
                ui.label(tr("common.name"));
                ui.text_edit_singleline(&mut self.diagram.name);
                ui.add_space(12.0);
                ui.label(tr("flow.select_element_edit_properties"));
                ui.label(tr("flow.drag_between_elements_click_source_then_target"));
            }
        });
    }
    fn canvas(&mut self, ui: &mut Ui) {
        let (rect, response) = ui.allocate_exact_size(ui.available_size().max(vec2(1.0, 1.0)), Sense::click_and_drag());
        self.state.canvas = rect;
        self.state.camera.handle_navigation(ui, &response, rect);
        if self.state.fit {
            if let Some((min, max)) = self.diagram.bounds() {
                self.state.camera.fit(rect, min, max);
            }
            self.state.fit = false;
        }
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let world = pointer.map(|p| self.state.camera.to_world(rect, p));
        let shift = ui.input(|i| i.modifiers.shift);
        if response.hovered() && !ui.ctx().egui_wants_keyboard_input() {
            if ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
                self.delete();
            }
            if ui.input(|i| i.key_pressed(Key::F)) {
                self.state.fit = true;
            }
            if ui.input(|i| i.modifiers.command && i.key_pressed(Key::A)) {
                self.state.selected = self.diagram.nodes.iter().map(|n| n.id).collect();
            }
            if ui.input(|i| i.modifiers.command && i.key_pressed(Key::D)) {
                self.history.sync(&self.diagram, false);
                self.state.selected = self.diagram.duplicate(&self.state.selected);
                self.history.sync(&self.diagram, false);
            }
        }
        if response.drag_started_by(PointerButton::Primary)
            && let Some(screen) = ui.input(|i| i.pointer.press_origin())
        {
            let start = self.state.camera.to_world(rect, screen);
            self.history.sync(&self.diagram, false);
            if let Some(id) = self.hit(start) {
                if self.state.connecting {
                    self.state.drag = Some(Drag::Connect(id));
                    self.state.pending = Some(id);
                } else if self.state.tool.is_none() {
                    let n = self.diagram.node(id).unwrap();
                    let r = self.state.camera.rect_to_screen(rect, n.bounds);
                    if self.state.selected.len() == 1 && self.state.selected.contains(&id) && r.right_bottom().distance(screen) < 12.0 {
                        self.state.drag = Some(Drag::Resize(id));
                    } else {
                        if !self.state.selected.contains(&id) {
                            if !shift {
                                self.state.selected.clear();
                            }
                            self.state.selected.insert(id);
                        }
                        let origins = self
                            .diagram
                            .nodes
                            .iter()
                            .filter(|n| self.state.selected.contains(&n.id))
                            .map(|n| (n.id, Pos::new(n.bounds.x, n.bounds.y)))
                            .collect();
                        self.state.drag = Some(Drag::Move { start, origins });
                    }
                }
            } else if !self.state.connecting && self.state.tool.is_none() {
                self.state.drag = Some(Drag::Rubber {
                    start,
                    base: if shift { self.state.selected.clone() } else { BTreeSet::new() },
                });
            }
        }
        if response.dragged_by(PointerButton::Primary)
            && let Some(world) = world
        {
            match &self.state.drag {
                Some(Drag::Move { start, origins }) => {
                    let mut delta = world - *start;
                    if self.state.snap {
                        delta = delta.snapped(10.0);
                    }
                    for (id, p) in origins {
                        if let Some(n) = self.diagram.nodes.iter_mut().find(|n| n.id == *id) {
                            n.bounds.x = p.x + delta.x;
                            n.bounds.y = p.y + delta.y;
                        }
                    }
                }
                Some(Drag::Resize(id)) => {
                    if let Some(n) = self.diagram.nodes.iter_mut().find(|n| n.id == *id) {
                        let p = if self.state.snap { world.snapped(10.0) } else { world };
                        n.bounds.w = (p.x - n.bounds.x).clamp(20.0, 2000.0);
                        n.bounds.h = (p.y - n.bounds.y).clamp(8.0, 2000.0);
                    }
                }
                Some(Drag::Rubber { start, base }) => {
                    let area = Rect::from_two_pos(self.state.camera.to_screen(rect, *start), self.state.camera.to_screen(rect, world));
                    self.state.selected = base.clone();
                    self.state.selected.extend(
                        self.diagram
                            .nodes
                            .iter()
                            .filter(|n| area.contains_rect(self.state.camera.rect_to_screen(rect, n.bounds)))
                            .map(|n| n.id),
                    );
                    self.state.flow = None;
                }
                _ => {}
            }
        }
        if ui.input(|i| i.pointer.primary_released()) {
            if let Some(Drag::Connect(from)) = self.state.drag.take() {
                if let Some(to) = world.and_then(|p| self.hit(p)) {
                    self.connect(from, to);
                } else {
                    self.state.pending = None;
                }
            } else {
                self.state.drag = None;
            }
        }
        if response.clicked_by(PointerButton::Primary)
            && let Some(p) = world
        {
            if let Some(kind) = self.state.tool {
                self.history.sync(&self.diagram, false);
                let p = if self.state.snap { p.snapped(10.0) } else { p };
                let id = self.diagram.add_node(kind, p, tr(kind.label()));
                self.clear_selection();
                self.state.selected.insert(id);
                self.state.tool = None;
                self.history.sync(&self.diagram, false);
            } else if let Some(id) = self.hit(p) {
                if self.state.connecting {
                    if let Some(from) = self.state.pending {
                        self.connect(from, id);
                    } else {
                        self.state.pending = Some(id);
                    }
                } else {
                    if !shift {
                        self.state.selected.clear();
                    }
                    if !self.state.selected.insert(id) {
                        self.state.selected.remove(&id);
                    }
                    self.state.flow = None;
                }
            } else if !self.state.connecting {
                self.state.selected.clear();
                self.state.flow = self
                    .diagram
                    .flows
                    .iter()
                    .rev()
                    .find(|f| {
                        self.diagram
                            .points(f)
                            .is_some_and(|(a, b)| dist_to_segment(p, a, b) < 7.0 / self.state.camera.zoom)
                    })
                    .map(|f| f.id);
            } else {
                self.state.pending = None;
            }
        }
        let pal = Palette::for_ui(ui);
        let p = ui.painter().with_clip_rect(rect);
        let exporting = std::mem::take(&mut self.state.export_png);
        canvas::draw_background(&p, rect, &self.state.camera, &pal);
        for f in &self.diagram.flows {
            if let Some((a, b)) = self.diagram.points(f) {
                let a = self.state.camera.to_screen(rect, a);
                let b = self.state.camera.to_screen(rect, b);
                let color = if !exporting && Some(f.id) == self.state.flow {
                    pal.select
                } else {
                    pal.arc
                };
                p.line_segment([a, b], Stroke::new(1.5, color));
                canvas::arrow_head(&p, b, b - a, 9.0 * self.state.camera.zoom, color, true);
                if !f.label.is_empty() {
                    canvas::badge(&p, a.lerp(b, 0.5), &f.label, 12.0 * self.state.camera.zoom, pal.text, &pal);
                }
            }
        }
        for n in &self.diagram.nodes {
            let r = self.state.camera.rect_to_screen(rect, n.bounds);
            let z = self.state.camera.zoom;
            let selected = !exporting && self.state.selected.contains(&n.id);
            let stroke = Stroke::new(if selected { 2.0 } else { 1.5 }, if selected { pal.select } else { pal.ink });
            paint_shape(&p, r, n.kind.shape(), stroke, pal.task_fill, pal.ink);
            let inside = matches!(n.kind.shape(), Shape::Action | Shape::Event);
            let galley = p.layout(
                n.name.clone(),
                FontId::proportional(13.0 * z),
                pal.text,
                if inside { (r.width() - 12.0 * z).max(20.0) } else { 160.0 * z },
            );
            let text_pos = if inside {
                r.center() - galley.size() / 2.0
            } else {
                pos2(r.center().x - galley.size().x / 2.0, r.bottom() + 7.0 * z)
            };
            p.galley(text_pos, galley, pal.text);
            if selected {
                p.rect_filled(Rect::from_center_size(r.right_bottom(), vec2(8.0, 8.0)), 1.0, pal.select);
            }
        }
        if !exporting {
            if let Some(from) = self.state.pending.and_then(|id| self.diagram.node(id))
                && let Some(to) = pointer.filter(|p| rect.contains(*p))
            {
                p.line_segment(
                    [
                        self.state.camera.to_screen(rect, from.anchor(self.state.camera.to_world(rect, to))),
                        to,
                    ],
                    Stroke::new(1.5, pal.select),
                );
            }
            if let Some(Drag::Rubber { start, .. }) = &self.state.drag
                && let Some(end) = pointer
            {
                p.rect_stroke(
                    Rect::from_two_pos(self.state.camera.to_screen(rect, *start), end),
                    0.0,
                    Stroke::new(1.0, pal.select),
                    StrokeKind::Inside,
                );
            }
            self.palette(ui, rect);
        } else {
            crate::io::request_png_export(ui.ctx(), rect, self.diagram.name.clone());
        }
    }
    fn palette(&mut self, ui: &Ui, rect: Rect) {
        canvas::floating(
            ui.ctx(),
            egui::Id::unique(("flow_palette", self.state.uid)),
            rect.left_top() + vec2(14.0, 14.0),
            Align2::LEFT_TOP,
            |ui| {
                if canvas::icon_button(
                    ui,
                    self.state.tool.is_none() && !self.state.connecting,
                    tr("common.select"),
                    crate::icons::select,
                )
                .clicked()
                {
                    self.state.tool = None;
                    self.state.connecting = false;
                    self.state.pending = None;
                }
                if canvas::icon_button(ui, self.state.connecting, tr("database.connect"), |p, r, c| {
                    p.line_segment([r.left_bottom(), r.right_top()], Stroke::new(1.5, c));
                    canvas::arrow_head(p, r.right_top(), vec2(1.0, -1.0), 7.0, c, true);
                })
                .clicked()
                {
                    self.state.tool = None;
                    self.state.connecting = true;
                    self.state.pending = None;
                }
                canvas::toolbar_separator(ui);
                for &kind in K::all() {
                    if canvas::icon_button(ui, self.state.tool == Some(kind), tr(kind.label()), |p, r, c| {
                        let icon = match kind.shape() {
                            Shape::Bar => Rect::from_center_size(r.center(), vec2(r.width(), 4.0)),
                            Shape::Action | Shape::Event => r.shrink2(vec2(0.0, 3.0)),
                            _ => r,
                        };
                        paint_shape(p, icon, kind.shape(), Stroke::new(1.4, c), Color32::TRANSPARENT, c)
                    })
                    .clicked()
                    {
                        self.state.tool = Some(kind);
                        self.state.connecting = false;
                        self.state.pending = None;
                    }
                }
            },
        );
    }
    #[cfg(test)]
    pub fn node_screen_center(&self, id: Id) -> egui::Pos2 {
        self.state
            .camera
            .to_screen(self.state.canvas, self.diagram.node(id).unwrap().bounds.center())
    }
}

fn paint_shape(p: &egui::Painter, r: Rect, shape: Shape, stroke: Stroke, fill: Color32, ink: Color32) {
    let c = r.center();
    match shape {
        Shape::Initial => {
            p.add(egui::epaint::EllipseShape {
                center: c,
                angle: 0.0,
                radius: r.size() / 2.0,
                fill: ink,
                stroke: Stroke::NONE,
            });
        }
        Shape::Final | Shape::FlowFinal | Shape::Connector(_) => {
            p.add(egui::epaint::EllipseShape {
                center: c,
                angle: 0.0,
                radius: r.size() / 2.0,
                fill,
                stroke,
            });
            match shape {
                Shape::Final => {
                    p.circle_filled(c, r.width().min(r.height()) * 0.30, ink);
                }
                Shape::FlowFinal => {
                    p.line_segment([r.lerp_inside(vec2(0.25, 0.25)), r.lerp_inside(vec2(0.75, 0.75))], stroke);
                    p.line_segment([r.lerp_inside(vec2(0.25, 0.75)), r.lerp_inside(vec2(0.75, 0.25))], stroke);
                }
                Shape::Connector(symbol) => {
                    p.text(
                        c,
                        Align2::CENTER_CENTER,
                        symbol,
                        FontId::proportional((r.height() * 0.55).clamp(10.0, 32.0)),
                        ink,
                    );
                }
                _ => {}
            }
        }
        Shape::Diamond => {
            p.add(egui::Shape::convex_polygon(
                vec![pos2(c.x, r.top()), pos2(r.right(), c.y), pos2(c.x, r.bottom()), pos2(r.left(), c.y)],
                fill,
                stroke,
            ));
        }
        Shape::Event => {
            let d = r.width() * 0.12;
            p.add(egui::Shape::convex_polygon(
                vec![
                    r.left_top() + vec2(d, 0.0),
                    r.right_top() - vec2(d, 0.0),
                    pos2(r.right(), c.y),
                    r.right_bottom() - vec2(d, 0.0),
                    r.left_bottom() + vec2(d, 0.0),
                    pos2(r.left(), c.y),
                ],
                fill,
                stroke,
            ));
        }
        Shape::Bar => {
            p.rect_filled(r, 0.0, ink);
        }
        Shape::Action => {
            p.rect(r, (r.height() * 0.14).clamp(2.0, 8.0), fill, stroke, StrokeKind::Inside);
        }
    }
}

const COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        id: UNDO,
        label: key("common.undo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: REDO,
        label: key("common.redo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: DELETE,
        label: key("common.delete_selection"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: FIT,
        label: key("canvas.zoom_fit"),
        group: CommandGroup::View,
    },
];
impl<K: Notation> EditorBackend for FlowEditor<K> {
    type ViewState = FlowView<K>;
    fn capture_view(&self) -> Self::ViewState {
        self.state.clone()
    }
    fn restore_view(&mut self, s: &Self::ViewState) {
        self.state = s.clone();
    }
    fn fresh_view(&self) -> Self::ViewState {
        FlowView::default()
    }
    fn save_view(s: &Self::ViewState) -> serde_json::Value {
        serde_json::json!({"camera":s.camera,"snap":s.snap,"inspector":s.inspector})
    }
    fn load_view(s: &mut Self::ViewState, v: &serde_json::Value) {
        s.inspector = v["inspector"].as_bool();
        if let Some(snap) = v["snap"].as_bool() {
            s.snap = snap;
        }
        if let Ok(c) = serde_json::from_value::<View>(v["camera"].clone())
            && c.zoom.is_finite()
            && c.zoom > 0.0
            && c.offset.is_finite()
        {
            s.camera = c;
            s.fit = false;
        }
    }
    fn kind(&self) -> &'static str {
        K::ID
    }
    fn name(&self) -> &str {
        &self.diagram.name
    }
    fn rename(&mut self, name: String) {
        self.history.sync(&self.diagram, false);
        self.diagram.name = name;
        self.history.sync(&self.diagram, false);
    }
    fn document(&self) -> Document {
        JsonCodec::new(K::ID).encode(&self.diagram)
    }
    fn semantic_document(&self) -> Document {
        let mut d = self.diagram.clone();
        for n in &mut d.nodes {
            n.bounds = Default::default();
        }
        JsonCodec::new(K::ID).encode(&d)
    }
    fn presentation_key(&self) -> serde_json::Value {
        serde_json::json!(self.diagram.nodes.iter().map(|n| (n.id, n.bounds)).collect::<Vec<_>>())
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.diagram);
        self.state.drag = None;
        self.state.pending = None;
        self.state.tool = None;
        self.state.connecting = false;
    }
    fn ui(&mut self, ui: &mut Ui, _: &str) -> Vec<HostEvent> {
        let layout = DiagramLayout::new(ui, self.state.uid, self.state.inspector);
        layout.inspector(ui, |ui| self.properties(ui));
        layout.configuration(
            ui,
            CanvasControls {
                view: &mut self.state.camera,
                canvas: self.state.canvas,
                fit: &mut self.state.fit,
                snap: &mut self.state.snap,
                inspector: &mut self.state.inspector,
                issues: None,
            },
            FooterStatus {
                summary: trf("common.element_count", &[&self.diagram.nodes.len()]),
                detail: self.state.status.clone(),
                ..Default::default()
            },
            |_| {},
        );
        layout.canvas(ui, |ui| self.canvas(ui));
        self.history.sync(
            &self.diagram,
            self.state.drag.is_some() || ui.ctx().egui_wants_keyboard_input() || ui.input(|i| i.pointer.any_down()),
        );
        vec![]
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
        COMMANDS
    }
    fn enabled(&self, c: &str) -> bool {
        match c {
            UNDO => self.history.can_undo(),
            REDO => self.history.can_redo(),
            DELETE => !self.state.selected.is_empty() || self.state.flow.is_some(),
            FIT | PNG => true,
            _ => false,
        }
    }
    fn execute(&mut self, c: &str, _: f64) -> Vec<HostEvent> {
        match c {
            UNDO => {
                self.history.undo(&mut self.diagram);
                self.clear_selection();
            }
            REDO => {
                self.history.redo(&mut self.diagram);
                self.clear_selection();
            }
            DELETE => self.delete(),
            FIT => self.state.fit = true,
            PNG => self.state.export_png = true,
            _ => {}
        }
        vec![]
    }
    fn export(&self) -> Result<ExportData, String> {
        Ok(ExportData {
            suffix: "svg",
            bytes: self.diagram.to_svg().into_bytes(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("flow.export_svg_svg")
    }
    fn exports(&self) -> Vec<ExportDescriptor> {
        vec![
            ExportDescriptor {
                id: "standard",
                label: self.export_label(),
            },
            ExportDescriptor {
                id: "dot",
                label: key("flow.export_graphviz_dot"),
            },
        ]
    }
    fn export_as(&self, id: &str) -> Result<ExportData, String> {
        match id {
            "standard" => self.export(),
            "dot" => Ok(ExportData {
                suffix: "dot",
                bytes: self.diagram.to_dot().into_bytes(),
            }),
            _ => Err("Unknown graph export".into()),
        }
    }
}
fn register_one<K: Notation>(registry: &mut DiagramRegistry, order: usize) {
    registry.diagrams.push(DiagramDescriptor {
        id: K::ID,
        aliases: &[],
        home_order: order,
        new_label: K::NEW,
        title: K::TITLE,
        description: K::DESCRIPTION,
        badge: K::BADGE,
        color: K::COLOR,
        icon: K::ICON,
        create: |n| JsonCodec::new(K::ID).encode(&Diagram::<K>::new(trf(K::NAME, &[&n]))),
        open: |d| {
            let model: Diagram<K> = JsonCodec::new(K::ID).decode(d)?;
            model.check_storage().map_err(ReadError::Invalid)?;
            Ok(Session::boxed(FlowEditor::new(model)))
        },
    });
    registry.examples.push(ExampleDescriptor {
        id: K::EXAMPLE,
        kind: K::ID,
        label: K::EXAMPLE_LABEL,
        create: || JsonCodec::new(K::ID).encode(&K::example()),
    });
}
pub fn register(registry: &mut DiagramRegistry) {
    register_one::<EpcKind>(registry, 7);
    register_one::<ActivityKind>(registry, 8);
}
