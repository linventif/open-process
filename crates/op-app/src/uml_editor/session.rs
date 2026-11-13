//! Session adapter for the uml notation.
use super::*;
use crate::editor::*;
use op_core::document::{Document, DocumentCodec, JsonCodec, ReadError};
use op_core::i18n::key;

view_state!(UmlEditor, UmlEditorView, uid: u64, view: View, last_canvas: Rect, snap: bool, fit: bool, show_inspector: Option<bool>, tool: Tool, drag: Option<Drag>, editing: Option<(Selection, String, bool)>, pending: Option<Id>, dnd: Option<NodeKind>);

const COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        id: UNDO,
        label: key("Undo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: REDO,
        label: key("Redo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: DELETE,
        label: key("Delete selection"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: FIT,
        label: key("Zoom to fit"),
        group: CommandGroup::View,
    },
];
const VIEWS: &[ViewDescriptor] = &[];

impl EditorBackend for UmlEditor {
    type ViewState = UmlEditorView;
    fn capture_view(&self) -> Self::ViewState {
        self.capture_state()
    }
    fn restore_view(&mut self, state: &Self::ViewState) {
        self.restore_state(state);
    }
    fn fresh_view(&self) -> Self::ViewState {
        let mut state = self.capture_state();
        state.uid = canvas::next_uid();
        state.view = View::default();
        state.fit = true;
        state.drag = None;
        state.editing = None;
        state.dnd = None;
        state
    }
    fn save_view(state: &Self::ViewState) -> serde_json::Value {
        serde_json::json!({"camera": state.view, "snap": state.snap, "inspector": state.show_inspector})
    }
    fn load_view(state: &mut Self::ViewState, value: &serde_json::Value) {
        if let Some(snap) = value["snap"].as_bool() {
            state.snap = snap;
        }
        state.show_inspector = value["inspector"].as_bool();
        if let Ok(camera) = serde_json::from_value::<View>(value["camera"].clone()) {
            if camera.zoom.is_finite() && camera.zoom > 0.0 && camera.offset.is_finite() {
                state.view = camera;
                state.fit = false;
            }
        }
    }
    fn kind(&self) -> &'static str {
        if self.diagram.kind == DiagramKind::Class {
            "uml.class"
        } else {
            "uml.use-case"
        }
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
        JsonCodec::new(EditorBackend::kind(self)).encode(&self.diagram)
    }
    fn semantic_document(&self) -> Document {
        let mut d = self.diagram.clone();
        let containment: Vec<_> = d.nodes.iter().map(|n| (n.id, d.system_of(n))).collect();
        for n in &mut d.nodes {
            n.bounds = Default::default();
        }
        let mut key = JsonCodec::new(EditorBackend::kind(self)).encode(&d);
        key.model["containment"] = serde_json::to_value(containment).unwrap();
        key
    }
    fn presentation_key(&self) -> serde_json::Value {
        serde_json::json!(self.diagram.nodes.iter().map(|n| (n.id, n.bounds)).collect::<Vec<_>>())
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.diagram);
        self.drag = None;
        self.dnd = None;
    }
    fn ui(&mut self, ui: &mut Ui, view: &str) -> Vec<HostEvent> {
        let _ = view;
        UmlEditor::ui(self, ui);
        let events = Vec::new();

        events
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
        COMMANDS
    }
    fn enabled(&self, command: &str) -> bool {
        match command {
            UNDO => self.can_undo(),
            REDO => self.can_redo(),
            DELETE => self.selection.is_some() || !self.group.is_empty(),
            FIT | PNG => true,
            _ => COMMANDS.iter().any(|c| c.id == command),
        }
    }
    fn execute(&mut self, command: &str, time: f64) -> Vec<HostEvent> {
        let _ = time;
        let events = Vec::new();
        if !self.enabled(command) && command != "bpmn.evaluation" {
            return events;
        }
        match command {
            UNDO => self.undo(),
            REDO => self.redo(),
            DELETE => self.delete_selection(),
            FIT => self.request_fit(),
            PNG => self.export_png = true,

            _ => {}
        }
        events
    }
    fn views(&self) -> &'static [ViewDescriptor] {
        VIEWS
    }
    fn export(&self) -> Result<ExportData, String> {
        Ok(ExportData {
            suffix: "puml",
            bytes: self.diagram.to_plantuml().into_bytes(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("Export PlantUML (.puml)…")
    }
}

use op_core::examples;

pub(crate) fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.extend([
        DiagramDescriptor {
            id: "uml.class",
            home_order: 4,
            icon: crate::icons::class,
            aliases: &["class"],
            new_label: key("New class diagram"),
            title: key("Design a class diagram"),
            description: key("Describe classes, attributes, operations and their relationships in a UML model."),
            badge: "UML · CLASS",
            color: egui::Color32::from_rgb(79, 110, 190),
            create: |n| JsonCodec::new("uml.class").encode(&UmlDiagram::new(trf("Class diagram {}", &[&n]), DiagramKind::Class)),
            open: |d| open_uml(d, DiagramKind::Class),
        },
        DiagramDescriptor {
            id: "uml.use-case",
            home_order: 5,
            icon: crate::icons::use_case,
            aliases: &["usecase"],
            new_label: key("New use-case diagram"),
            title: key("Map use cases"),
            description: key("Identify actors, use cases and interactions within a system."),
            badge: "UML · USE CASE",
            color: egui::Color32::from_rgb(192, 80, 131),
            create: |n| JsonCodec::new("uml.use-case").encode(&UmlDiagram::new(trf("Use-case diagram {}", &[&n]), DiagramKind::UseCase)),
            open: |d| open_uml(d, DiagramKind::UseCase),
        },
    ]);
    registry.examples.extend([
        ExampleDescriptor {
            id: "uml.classes",
            label: key("UML – Order classes"),
            create: || JsonCodec::new("uml.class").encode(&examples::uml_classes()),
        },
        ExampleDescriptor {
            id: "uml.use-cases",
            label: key("UML – Online shop use cases"),
            create: || JsonCodec::new("uml.use-case").encode(&examples::uml_use_cases()),
        },
    ]);
}
fn open_uml(d: &Document, kind: DiagramKind) -> Result<Box<dyn EditorSession>, ReadError> {
    let codec = JsonCodec::new(if kind == DiagramKind::Class { "uml.class" } else { "uml.use-case" });
    let model: UmlDiagram = codec.decode(d)?;
    if model.kind != kind {
        return Err(ReadError::Invalid("UML kind does not match envelope".into()));
    }
    Ok(Session::boxed(UmlEditor::new(model)))
}
