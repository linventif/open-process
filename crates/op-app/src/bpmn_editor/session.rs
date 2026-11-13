//! Session adapter for the bpmn notation.
use super::*;
use crate::editor::*;
use op_core::document::{Document, DocumentCodec, JsonCodec};
use op_core::i18n::key;

view_state!(BpmnEditor, BpmnEditorView, uid: u64, view: View, last_canvas: Rect, snap: bool, show_issues: bool, fit_request: bool, show_inspector: Option<bool>, tab: SideTab, tool: Tool, drag: Option<Drag>, editing: Option<(Sel, String, bool)>, connect_pending: Option<Id>, dnd: Option<Tool>);

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
    CommandDescriptor {
        id: "bpmn.convert-to-petri",
        label: key("Convert BPMN → Petri net"),
        group: CommandGroup::Tools,
    },
];
const VIEWS: &[ViewDescriptor] = &[];

impl EditorBackend for BpmnEditor {
    type ViewState = BpmnEditorView;
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
        state.fit_request = true;
        state.drag = None;
        state.editing = None;
        state.dnd = None;
        state
    }
    fn save_view(state: &Self::ViewState) -> serde_json::Value {
        serde_json::json!({"camera": state.view, "snap": state.snap, "issues": state.show_issues, "inspector": state.show_inspector, "tab": match state.tab { SideTab::Properties => 0, SideTab::Validation => 1, SideTab::Evaluation => 2 } })
    }
    fn load_view(state: &mut Self::ViewState, value: &serde_json::Value) {
        if let Some(issues) = value["issues"].as_bool() {
            state.show_issues = issues;
        }
        if let Some(snap) = value["snap"].as_bool() {
            state.snap = snap;
        }
        state.show_inspector = value["inspector"].as_bool();
        if let Some(tab) = value["tab"].as_u64() {
            state.tab = match tab {
                0 => SideTab::Properties,
                1 => SideTab::Validation,
                2 => SideTab::Evaluation,
                _ => SideTab::Properties,
            };
        }
        if let Ok(camera) = serde_json::from_value::<View>(value["camera"].clone()) {
            if camera.zoom.is_finite() && camera.zoom > 0.0 && camera.offset.is_finite() {
                state.view = camera;
                state.fit_request = false;
            }
        }
    }
    fn kind(&self) -> &'static str {
        "bpmn"
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
        let membership: Vec<_> = d.nodes.iter().map(|n| (n.id, d.pool_of(n.id), d.lane_of(n.id))).collect();
        for n in &mut d.nodes {
            n.pos = Pos::default();
            n.w = 0.0;
            n.h = 0.0;
        }
        for p in &mut d.pools {
            p.pos = Pos::default();
            p.width = 0.0;
            for l in &mut p.lanes {
                l.height = 0.0;
            }
        }
        d.style = Default::default();
        let mut key = JsonCodec::new("bpmn").encode(&d);
        key.model["membership"] = serde_json::to_value(membership).unwrap();
        key
    }
    fn presentation_key(&self) -> serde_json::Value {
        serde_json::json!({"nodes":self.diagram.nodes.iter().map(|n| (n.id,n.pos,n.w,n.h)).collect::<Vec<_>>(), "pools":self.diagram.pools.iter().map(|p| (p.id,p.pos,p.width,p.lanes.iter().map(|l| (l.id,l.height)).collect::<Vec<_>>())).collect::<Vec<_>>(), "style":self.diagram.style})
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.diagram);
        self.drag = None;
        self.dnd = None;
    }
    fn ui(&mut self, ui: &mut Ui, view: &str) -> Vec<HostEvent> {
        let _ = view;
        BpmnEditor::ui(self, ui);
        let mut events = Vec::new();
        if self.action.take().is_some() {
            events.extend(self.execute("bpmn.convert-to-petri", 0.0));
        }
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
        let mut events = Vec::new();
        if !self.enabled(command) && command != "bpmn.evaluation" {
            return events;
        }
        match command {
            UNDO => self.undo(),
            REDO => self.redo(),
            DELETE => self.delete_selection(),
            FIT => self.request_fit(),
            PNG => self.export_png = true,
            "bpmn.convert-to-petri" => {
                let c = op_core::convert::bpmn_to_petri(&self.diagram);
                events.push(HostEvent::Open(JsonCodec::new("petri").encode(&c.net)));
                if c.warnings.is_empty() {
                    events.push(HostEvent::Message(
                        tr("BPMN converted to a Petri net (see the Analysis tab for soundness)").into(),
                    ));
                } else {
                    events.push(HostEvent::Message(trf("Converted with warnings: {}", &[&c.warnings.join(" · ")])));
                }
            }
            "bpmn.evaluation" => self.show_evaluation(),
            _ => {}
        }
        events
    }
    fn views(&self) -> &'static [ViewDescriptor] {
        VIEWS
    }
    fn export(&self) -> Result<ExportData, String> {
        Ok(ExportData {
            suffix: "bpmn",
            bytes: op_core::bpmn_xml::export(&self.diagram).into_bytes(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("Export BPMN 2.0 XML (.bpmn)…")
    }
}

use crate::theme;
use op_core::examples;

pub(crate) fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.extend([DiagramDescriptor {
        id: "bpmn",
        home_order: 0,
        aliases: &[],
        new_label: key("New BPMN diagram"),
        title: key("Model a process"),
        description: key("Draw BPMN diagrams with activities, events, gateways, pools and lanes."),
        badge: "BPMN 2.0",
        color: theme::KIND_BPMN,
        create: |n| JsonCodec::new("bpmn").encode(&BpmnDiagram::new(trf("Process {}", &[&n]))),
        open: |d| Ok(Session::boxed(BpmnEditor::new(JsonCodec::new("bpmn").decode(d)?))),
    }]);
    registry.examples.extend([
        ExampleDescriptor {
            id: "bpmn.evaluation",
            label: key("BPMN – Process P evaluation"),
            create: || JsonCodec::new("bpmn").encode(&examples::evaluation_process()),
        },
        ExampleDescriptor {
            id: "bpmn.order",
            label: key("BPMN – Order handling"),
            create: || JsonCodec::new("bpmn").encode(&examples::order_process()),
        },
    ]);
    registry.imports.extend([ImportDescriptor {
        extensions: &["bpmn", "xml"],
        accepts: |s| s.starts_with('<') && op_core::xml::root_name(s).as_deref() == Some("definitions"),
        decode: |s| op_core::bpmn_xml::import(s).map(|d| JsonCodec::new("bpmn").encode(&d)),
    }]);
    registry.workflows.push(WorkflowDescriptor {
        home_order: 3,
        title: key("Evaluate costs and durations"),
        description: key("Explore Process P to calculate traces, activity costs and mean execution time."),
        badge: "BPMN · CA / TA",
        color: egui::Color32::from_rgb(13, 148, 136),
        example: "bpmn.evaluation",
        command: Some("bpmn.evaluation"),
    });
}
