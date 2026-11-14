//! Shared graphical editor for UML class and use-case diagrams.

mod inspector;

mod session;
pub(crate) use session::register;

use crate::editor::layout::{CanvasControls, DiagramLayout, FooterStatus};

use crate::canvas::{self, Palette, View};
use crate::history::History;
use egui::{Align2, FontId, Key, Painter, PointerButton, Rect, Sense, Shape, Stroke, StrokeKind, Ui, pos2, vec2};
use op_core::geom::{Pos, dist_to_polyline};
use op_core::i18n::{tr, trf};
use std::collections::{BTreeSet, HashMap};

mod draw;
mod interaction;
use op_core::uml::*;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Tool {
    Select,
    Create(NodeKind),
    Connect,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Selection {
    Node(Id),
    Relation(Id),
}

#[derive(Clone, Debug)]
enum Drag {
    Move { start: Pos, origins: Vec<(Id, Pos)> },
    Resize(Id),
    Rubber { start: Pos, base: BTreeSet<Id> },
    Connect(Id),
    Pan,
}

pub struct UmlEditor {
    pub diagram: UmlDiagram,
    pub export_png: bool,
    pub view: View,
    uid: u64,
    tool: Tool,
    selection: Option<Selection>,
    relation_kind: RelationKind,
    pending: Option<Id>,
    group: BTreeSet<Id>,
    drag: Option<Drag>,
    editing: Option<(Selection, String, bool)>,
    dnd: Option<NodeKind>,
    last_canvas: Rect,
    routes: HashMap<Id, Vec<Pos>>,
    routed_model: Option<UmlDiagram>,
    label_hits: Vec<(Id, Rect)>,
    paste_count: u32,
    history: History<UmlDiagram>,
    fit: bool,
    snap: bool,
    show_inspector: Option<bool>,
    status: String,
    convert_mermaid: bool,
}

impl UmlEditor {
    pub fn new(diagram: UmlDiagram) -> Self {
        Self {
            history: History::new(&diagram),
            diagram,
            export_png: false,
            view: View::default(),
            uid: canvas::next_uid(),
            tool: Tool::Select,
            selection: None,
            relation_kind: RelationKind::Association,
            pending: None,
            group: BTreeSet::new(),
            drag: None,
            editing: None,
            dnd: None,
            last_canvas: Rect::NOTHING,
            routes: HashMap::new(),
            routed_model: None,
            label_hits: Vec::new(),
            paste_count: 0,
            fit: true,
            snap: true,
            show_inspector: None,
            status: String::new(),
            convert_mermaid: false,
        }
    }

    pub fn undo(&mut self) {
        self.history.undo(&mut self.diagram);
        self.clear_interaction();
    }

    pub fn redo(&mut self) {
        self.history.redo(&mut self.diagram);
        self.clear_interaction();
    }

    fn clear_interaction(&mut self) {
        self.selection = None;
        self.pending = None;
        self.group.clear();
        self.drag = None;
        self.editing = None;
        self.dnd = None;
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }
    pub fn can_redo(&self) -> bool {
        self.history.can_redo()
    }
    pub fn request_fit(&mut self) {
        self.fit = true;
    }

    pub fn delete_selection(&mut self) {
        self.history.sync(&self.diagram, false);
        let ids = self.group.clone();
        self.diagram.nodes.retain(|n| !ids.contains(&n.id));
        self.diagram
            .relations
            .retain(|r| !ids.contains(&r.from) && !ids.contains(&r.to) && self.selection != Some(Selection::Relation(r.id)));
        self.clear_interaction();
        self.history.sync(&self.diagram, false);
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.push_id(self.uid, |ui| {
            let layout = DiagramLayout::new(ui, self.uid, self.show_inspector);
            layout.tools(ui, |ui| {
                egui::ComboBox::from_id_salt("relation_type")
                    .selected_text(tr(self.relation_kind.label()))
                    .show_ui(ui, |ui| {
                        for &kind in self.diagram.kind.relation_kinds() {
                            if ui.selectable_label(self.relation_kind == kind, tr(kind.label())).clicked() {
                                self.relation_kind = kind;
                                self.tool = Tool::Connect;
                                self.pending = None;
                            }
                        }
                    });
                ui.menu_button(tr("bpmn.arrange"), |ui| {
                    if self.group.len() < 2 {
                        ui.disable();
                    }
                    for (label, horizontal, edge) in [
                        (tr("common.left"), true, -1),
                        (tr("canvas.center_horizontal"), true, 0),
                        (tr("common.right"), true, 1),
                        (tr("common.top"), false, -1),
                        (tr("canvas.center_vertical"), false, 0),
                        (tr("common.bottom"), false, 1),
                    ] {
                        if ui.button(label).clicked() {
                            self.align(horizontal, edge);
                            ui.close();
                        }
                    }
                    if ui
                        .add_enabled(self.group.len() > 2, egui::Button::new(tr("canvas.distribute_horizontal")))
                        .clicked()
                    {
                        self.distribute(true);
                        ui.close();
                    }
                    if ui
                        .add_enabled(self.group.len() > 2, egui::Button::new(tr("canvas.distribute_vertical")))
                        .clicked()
                    {
                        self.distribute(false);
                        ui.close();
                    }
                });
                if ui
                    .add_enabled(!self.group.is_empty(), egui::Button::new(tr("common.duplicate")))
                    .clicked()
                {
                    let fragment = self.diagram.fragment(&self.group);
                    self.paste(&fragment);
                }
                if self.diagram.kind == DiagramKind::Class && ui.button(tr("conversion.to_mermaid")).clicked() {
                    self.convert_mermaid = true;
                }
            });
            layout.inspector(ui, |ui| DiagramLayout::inspector_body(ui, |ui| self.properties(ui)));
            self.status_bar(ui, layout);
            layout.canvas(ui, |ui| self.canvas(ui));
            self.history.sync(
                &self.diagram,
                self.drag.is_some() || self.editing.is_some() || ui.input(|i| i.pointer.any_down()) || ui.ctx().egui_wants_keyboard_input(),
            );
        });
    }

    fn status_bar(&mut self, ui: &mut Ui, layout: DiagramLayout) {
        let detail = if !self.status.is_empty() {
            self.status.clone()
        } else if self.group.len() > 1 {
            trf("uml.elements_selected", &[&self.group.len()])
        } else if self.tool == Tool::Connect {
            tr("flow.drag_between_elements_click_source_then_target").into()
        } else {
            tr("uml.double_click_rename_shift_select_several_drag").into()
        };
        let status = FooterStatus {
            detail,
            detail_color: (!self.status.is_empty()).then(|| ui.visuals().warn_fg_color),
            ..Default::default()
        };
        layout.configuration(
            ui,
            CanvasControls {
                view: &mut self.view,
                canvas: self.last_canvas,
                fit: &mut self.fit,
                snap: Some(&mut self.snap),
                inspector: &mut self.show_inspector,
                issues: None,
            },
            status,
            |_| {},
        );
    }

    fn hit_node(&self, p: Pos) -> Option<Id> {
        self.diagram
            .nodes
            .iter()
            .rev()
            .find(|n| n.kind != NodeKind::System && n.contains(p))
            .or_else(|| {
                self.diagram
                    .nodes
                    .iter()
                    .rev()
                    .find(|n| n.kind == NodeKind::System && n.contains(p))
            })
            .map(|n| n.id)
    }

    fn relation_points(&self, r: &Relation) -> Vec<Pos> {
        self.routes.get(&r.id).cloned().unwrap_or_else(|| routing::route(&self.diagram, r))
    }

    fn update_routes(&mut self) {
        if self.routed_model.as_ref() != Some(&self.diagram) {
            self.routes = routing::route_all(&self.diagram);
            self.routed_model = Some(self.diagram.clone());
        }
    }

    #[cfg(test)]
    pub fn selected_nodes(&self) -> &BTreeSet<Id> {
        &self.group
    }

    #[cfg(test)]
    pub fn node_screen_rect(&self, id: Id) -> Rect {
        self.view.rect_to_screen(self.last_canvas, self.diagram.node(id).unwrap().rect())
    }

    #[cfg(test)]
    pub fn node_screen_center(&self, id: Id) -> egui::Pos2 {
        self.view
            .to_screen(self.last_canvas, self.diagram.node(id).unwrap().rect().center())
    }
}
