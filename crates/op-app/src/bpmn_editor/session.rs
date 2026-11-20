//! Session adapter for the bpmn notation.
use super::*;
use crate::editor::*;
use op_core::document::{Document, DocumentCodec, JsonCodec};
use op_core::i18n::key;

view_state!(BpmnEditor, BpmnEditorView, uid: u64, clipboard: ClipboardState, group: Vec<Id>, pool_group: Vec<Id>, pool_move_nodes: Vec<Id>, selection: Option<Sel>, view: View, last_canvas: Rect, snap: bool, show_issues: bool, fit_request: bool, show_inspector: Option<bool>, tab: SideTab, tool: Tool, drag: Option<Drag>, editing: Option<(Sel, String, bool)>, connect_pending: Option<Id>, dnd: Option<Tool>);

const COMMANDS: &[CommandDescriptor] = &[
    clipboard::COPY_COMMAND,
    clipboard::CUT_COMMAND,
    clipboard::PASTE_COMMAND,
    clipboard::DUPLICATE_COMMAND,
    clipboard::SELECT_ALL_COMMAND,
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
    CommandDescriptor {
        id: "bpmn.convert-to-petri",
        label: key("bpmn.convert_bpmn_petri_net"),
        group: CommandGroup::Tools,
    },
    CommandDescriptor {
        id: ARRANGE,
        label: key("canvas.auto_arrange"),
        group: CommandGroup::Tools,
    },
    CommandDescriptor {
        id: "bpmn.convert-to-mermaid",
        label: key("conversion.to_mermaid"),
        group: CommandGroup::Tools,
    },
];
const VIEWS: &[ViewDescriptor] = &[];

impl EditorBackend for BpmnEditor {
    fn set_exam_mode(&mut self, active: bool) {
        if active {
            self.evaluation_cache.invalidate();
            self.action = None;
        }
    }
    fn thumbnail_rect(&self) -> Option<egui::Rect> {
        Some(self.last_canvas)
    }
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
        state.clipboard = ClipboardState::default();
        state.group.clear();
        state.pool_group.clear();
        state.pool_move_nodes.clear();
        state.selection = None;
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
        if let Ok(camera) = serde_json::from_value::<View>(value["camera"].clone())
            && camera.zoom.is_finite()
            && camera.zoom > 0.0
            && camera.offset.is_finite()
        {
            state.view = camera;
            state.fit_request = false;
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
    fn content_generation(&self) -> Option<u64> {
        Some(self.history.generation())
    }
    fn has_content(&self) -> bool {
        !self.diagram.nodes.is_empty() || !self.diagram.flows.is_empty() || !self.diagram.pools.is_empty()
    }
    fn validate_storage(&self) -> Result<(), String> {
        self.diagram.check_storage()
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
        if let Some(enabled) = self.selection_command_enabled(command) {
            return enabled;
        }
        match command {
            UNDO => self.can_undo(),
            REDO => self.can_redo(),
            DELETE => self.selection.is_some() || !self.group.is_empty() || !self.pool_group.is_empty(),
            FIT | PNG => true,
            ARRANGE => !self.diagram.nodes.is_empty() || !self.diagram.pools.is_empty(),
            _ => COMMANDS.iter().any(|c| c.id == command),
        }
    }
    fn execute(&mut self, command: &str, time: f64) -> Vec<HostEvent> {
        if self.selection_command(command) {
            return vec![];
        }
        let _ = time;
        let mut events = Vec::new();
        if !self.enabled(command) && command != "bpmn.evaluation" {
            return events;
        }
        match command {
            "bpmn.convert-to-mermaid" => return mermaid_exports::open(op_core::mermaid::export::bpmn(&self.diagram)),
            UNDO => self.undo(),
            REDO => self.redo(),
            DELETE => self.delete_selection(),
            FIT => self.request_fit(),
            PNG => self.export_png = true,
            ARRANGE => self.auto_arrange(),
            "bpmn.convert-to-petri" => {
                let c = op_core::convert::bpmn_to_petri(&self.diagram);
                events.push(HostEvent::Open(JsonCodec::new("petri").encode(&c.net)));
                if c.warnings.is_empty() {
                    events.push(HostEvent::Message(tr("bpmn.bpmn_converted_petri_net_see_analysis_tab").into()));
                } else {
                    events.push(HostEvent::Message(trf("bpmn.converted_warnings", &[&c.warnings.join(" · ")])));
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
        key("bpmn.export_bpmn_2_0_xml_bpmn")
    }
    fn exports(&self) -> Vec<ExportDescriptor> {
        let mut formats = vec![ExportDescriptor {
            id: "standard",
            label: self.export_label(),
        }];
        formats.push(ExportDescriptor {
            id: "svg",
            label: key("bpmn.export_svg"),
        });
        formats.extend(mermaid_exports::FORMATS);
        formats
    }
    fn export_as(&self, format: &str) -> Result<ExportData, String> {
        match format {
            "standard" => self.export(),
            "svg" => Ok(ExportData {
                suffix: "svg",
                bytes: self.diagram.to_svg().into_bytes(),
            }),
            "mermaid" | "mermaid-markdown" => mermaid_exports::data(format, op_core::mermaid::export::bpmn(&self.diagram)?),
            _ => Err(format!("Unknown BPMN export: {format}")),
        }
    }
    fn export_format_requires_assistance(&self, format: &str) -> bool {
        mermaid_exports::is_projection(format)
    }
}

use crate::theme;
use op_core::examples;

pub(crate) fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.extend([DiagramDescriptor {
        id: "bpmn",
        home_order: 3,
        icon: crate::icons::bpmn,
        aliases: &[],
        new_label: key("bpmn.new_bpmn_diagram"),
        title: key("bpmn.model_process"),
        description: key("bpmn.draw_bpmn_diagrams_activities_events_gateways_pools"),
        badge: "BPMN 2.0",
        color: theme::KIND_BPMN,
        create: |n| JsonCodec::new("bpmn").encode(&BpmnDiagram::new(trf("bpmn.document_name", &[&n]))),
        open: |d| Ok(Session::boxed(BpmnEditor::new(BpmnDiagram::CODEC.decode(d)?))),
    }]);
    registry.examples.extend([
        ExampleDescriptor {
            id: "bpmn.evaluation",
            kind: "bpmn",
            label: key("bpmn.bpmn_process_p_evaluation"),
            create: || JsonCodec::new("bpmn").encode(&examples::evaluation_process()),
        },
        ExampleDescriptor {
            id: "bpmn.order",
            kind: "bpmn",
            label: key("bpmn.bpmn_order_handling"),
            create: || JsonCodec::new("bpmn").encode(&examples::order_process()),
        },
    ]);
    registry.imports.extend([ImportDescriptor {
        extensions: &["bpmn", "xml"],
        accepts: |s| {
            s.starts_with('<')
                && op_core::xml::root_name(s).as_deref() == Some("definitions")
                && op_core::xml::parse(s).is_ok_and(|root| root.children.iter().any(|c| c.name == "process"))
        },
        decode: |s| op_core::bpmn_xml::import(s).map(|d| JsonCodec::new("bpmn").encode(&d)),
    }]);
    registry.workflows.push(WorkflowDescriptor {
        home_order: 7,
        icon: crate::icons::evaluation,
        title: key("bpmn.evaluate_costs_durations"),
        description: key("bpmn.explore_process_p_calculate_traces_activity_costs"),
        badge: "BPMN · CA / TA",
        color: egui::Color32::from_rgb(13, 148, 136),
        example: "bpmn.evaluation",
        command: Some("bpmn.evaluation"),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vector_export_is_renderable_and_remains_available_in_exam_mode() {
        let mut session = Session::boxed(BpmnEditor::new(op_core::examples::order_process()));
        let original = session.snapshot();
        session.set_exam_mode(true);
        assert!(session.exports().iter().any(|format| format.id == "svg"));
        let exported = session.export_as("svg").unwrap();
        assert_eq!(exported.suffix, "svg");
        let mut options = resvg::usvg::Options::default();
        options
            .fontdb_mut()
            .load_font_data(include_bytes!("../../assets/fonts/Inter-Regular.ttf").to_vec());
        options.fontdb_mut().set_sans_serif_family("Inter");
        let tree = resvg::usvg::Tree::from_data(&exported.bytes, &options).unwrap();
        let scale = (1200.0 / tree.size().width().max(tree.size().height())).min(1.0);
        let mut image = resvg::tiny_skia::Pixmap::new(
            (tree.size().width() * scale).ceil() as u32,
            (tree.size().height() * scale).ceil() as u32,
        )
        .unwrap();
        resvg::render(&tree, resvg::tiny_skia::Transform::from_scale(scale, scale), &mut image.as_mut());
        assert!(
            image
                .data()
                .chunks_exact(4)
                .any(|pixel| pixel[0] < 100 && pixel[1] < 100 && pixel[2] < 100 && pixel[3] > 0)
        );
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
        std::fs::create_dir_all(&directory).unwrap();
        image.save_png(directory.join("bpmn_vector_export.png")).unwrap();
        assert_eq!(session.snapshot(), original);
    }
}
