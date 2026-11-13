//! Session adapter for the petri notation.
use super::*;
use crate::editor::*;
use op_core::document::{Document, DocumentCodec, JsonCodec};
use op_core::i18n::key;

view_state!(PetriEditor, PetriEditorView, uid: u64, view: View, last_canvas: Rect, snap: bool, fit_request: bool, show_inspector: Option<bool>, tab: SideTab, tool: Tool, drag: Option<Drag>, editing: Option<(NodeRef, String, bool)>, arc_pending: Option<NodeRef>, dnd: Option<Tool>, graph_view: View, graph_fit: bool, graph_key: Option<u64>, graph_open: bool);

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
        id: "petri.simulation",
        label: key("Start / stop token game"),
        group: CommandGroup::Tools,
    },
    CommandDescriptor {
        id: "petri.matrices",
        label: key("Matrices (Pre, Post, C)"),
        group: CommandGroup::Tools,
    },
    CommandDescriptor {
        id: "petri.analysis",
        label: key("Analysis & invariants"),
        group: CommandGroup::Tools,
    },
    CommandDescriptor {
        id: "petri.graph",
        label: key("Reachability graph"),
        group: CommandGroup::Tools,
    },
    CommandDescriptor {
        id: "petri.convert-to-bpmn",
        label: key("Convert Petri net → BPMN"),
        group: CommandGroup::Tools,
    },
];
const VIEWS: &[ViewDescriptor] = &[
    ViewDescriptor {
        id: "petri.matrices",
        label: key("Matrices"),
    },
    ViewDescriptor {
        id: "petri.analysis",
        label: key("Analysis"),
    },
    ViewDescriptor {
        id: "petri.states",
        label: key("States"),
    },
    ViewDescriptor {
        id: "petri.graph",
        label: key("Reachability graph"),
    },
];

impl EditorBackend for PetriEditor {
    type ViewState = PetriEditorView;
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
        serde_json::json!({"camera": state.view, "snap": state.snap, "graph_camera": state.graph_view, "graph_key": state.graph_key, "graph_open": state.graph_open, "inspector": state.show_inspector, "tab": match state.tab { SideTab::Properties => 0, SideTab::Matrices => 1, SideTab::Analysis => 2, SideTab::States => 3 } })
    }
    fn load_view(state: &mut Self::ViewState, value: &serde_json::Value) {
        if let Some(snap) = value["snap"].as_bool() {
            state.snap = snap;
        }
        state.graph_key = value["graph_key"].as_u64();
        if let Some(open) = value["graph_open"].as_bool() {
            state.graph_open = open;
        }
        if let Ok(camera) = serde_json::from_value::<View>(value["graph_camera"].clone()) {
            if camera.zoom.is_finite() && camera.zoom > 0.0 && camera.offset.is_finite() {
                state.graph_view = camera;
                state.graph_fit = false;
            }
        }
        state.show_inspector = value["inspector"].as_bool();
        if let Some(tab) = value["tab"].as_u64() {
            state.tab = match tab {
                0 => SideTab::Properties,
                1 => SideTab::Matrices,
                2 => SideTab::Analysis,
                3 => SideTab::States,
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
        "petri"
    }
    fn name(&self) -> &str {
        &self.net.name
    }
    fn rename(&mut self, name: String) {
        self.history.sync(&self.net, false);
        self.net.name = name;
        self.history.sync(&self.net, false);
    }
    fn document(&self) -> Document {
        JsonCodec::new(EditorBackend::kind(self)).encode(&self.net)
    }
    fn semantic_document(&self) -> Document {
        JsonCodec::new("petri").encode(&self.net.analysis_model())
    }
    fn presentation_key(&self) -> serde_json::Value {
        serde_json::json!({"places":self.net.places.iter().map(|n| (n.id,n.pos)).collect::<Vec<_>>(), "transitions":self.net.transitions.iter().map(|n| (n.id,n.pos)).collect::<Vec<_>>(), "style":self.net.style})
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.net);
        self.drag = None;
        self.dnd = None;
    }
    fn ui(&mut self, ui: &mut Ui, view: &str) -> Vec<HostEvent> {
        let _ = view;
        self.view_ui(ui, crate::dock::View::from(view));
        let mut events = Vec::new();
        if let Some(v) = self.request_pane.take() {
            events.push(HostEvent::OpenView(v));
        }
        events
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
        COMMANDS
    }
    fn enabled(&self, command: &str) -> bool {
        match command {
            UNDO => self.sim.is_none() && self.can_undo(),
            REDO => self.sim.is_none() && self.can_redo(),
            DELETE => self.sim.is_none() && (self.selection.is_some() || !self.group.is_empty()),
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
            "petri.simulation" => self.toggle_simulation(time),
            "petri.matrices" => self.show_matrices(),
            "petri.analysis" => self.show_analysis(),
            "petri.graph" => self.show_reachability_graph(),
            "petri.convert-to-bpmn" => {
                let (d, warnings) = op_core::convert::petri_to_bpmn(&self.net);
                events.push(HostEvent::Open(JsonCodec::new("bpmn").encode(&d)));
                if warnings.is_empty() {
                    events.push(HostEvent::Message(tr("Petri net converted to BPMN").into()));
                } else {
                    events.push(HostEvent::Message(trf("Converted with warnings: {}", &[&warnings.join(" · ")])));
                }
            }
            _ => {}
        }
        events
    }
    fn views(&self) -> &'static [ViewDescriptor] {
        VIEWS
    }
    fn export(&self) -> Result<ExportData, String> {
        Ok(ExportData {
            suffix: "pnml",
            bytes: op_core::pnml::export(&self.net).into_bytes(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("Export PNML (.pnml)…")
    }
}

use crate::theme;
use op_core::examples;

pub(crate) fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.extend([DiagramDescriptor {
        id: "petri",
        home_order: 1,
        aliases: &[],
        new_label: key("New Petri net"),
        title: key("Simulate and analyze"),
        description: key("Build a Petri net, play tokens and explore matrices, invariants and reachability."),
        badge: "PETRI",
        color: theme::KIND_PETRI,
        create: |n| JsonCodec::new("petri").encode(&PetriNet::new(trf("Net {}", &[&n]))),
        open: |d| Ok(Session::boxed(PetriEditor::new(JsonCodec::new("petri").decode(d)?))),
    }]);
    registry.examples.extend([
        ExampleDescriptor {
            id: "petri.mutex",
            label: key("Petri – Mutual exclusion"),
            create: || JsonCodec::new("petri").encode(&examples::mutual_exclusion()),
        },
        ExampleDescriptor {
            id: "petri.producer",
            label: key("Petri – Producer / consumer"),
            create: || JsonCodec::new("petri").encode(&examples::producer_consumer()),
        },
        ExampleDescriptor {
            id: "petri.philosophers",
            label: key("Petri – Dining philosophers"),
            create: || JsonCodec::new("petri").encode(&examples::dining_philosophers()),
        },
    ]);
    registry.imports.extend([ImportDescriptor {
        extensions: &["pnml", "xml"],
        accepts: |s| s.starts_with('<') && matches!(op_core::xml::root_name(s).as_deref(), Some("pnml" | "net")),
        decode: |s| op_core::pnml::import(s).map(|d| JsonCodec::new("petri").encode(&d)),
    }]);
}
