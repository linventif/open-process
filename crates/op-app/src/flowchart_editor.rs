//! Native flowchart registration; shared interactions stay in FlowEditor.
use crate::editor::DiagramRegistry;
use crate::flow_editor::Notation;
use op_core::flow::Diagram;
use op_core::flowchart::FlowchartKind;
use op_core::i18n::key;

impl Notation for FlowchartKind {
    const ID: &'static str = "flowchart";
    const NEW: &'static str = key("flowchart.new");
    const TITLE: &'static str = key("flowchart.title");
    const DESCRIPTION: &'static str = key("flowchart.description");
    const NAME: &'static str = key("flowchart.name");
    const BADGE: &'static str = "FLOWCHART";
    const COLOR: egui::Color32 = egui::Color32::from_rgb(45, 115, 174);
    const ICON: crate::icons::Icon = crate::icons::activity;
    const EXAMPLE: &'static str = "flowchart.order";
    const EXAMPLE_LABEL: &'static str = key("flowchart.example");
    fn example() -> Diagram<Self> {
        op_core::flowchart::example()
    }
    fn to_mermaid(diagram: &Diagram<Self>) -> Result<op_core::mermaid::conversion::Conversion<op_core::mermaid::Diagram>, String> {
        op_core::flowchart::to_mermaid(diagram)
    }
}

pub(crate) fn register(registry: &mut DiagramRegistry) {
    crate::flow_editor::register_one::<FlowchartKind>(registry, 12);
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct ImportInput {
    pub task: String,
    pub source: op_core::mermaid::Diagram,
    pub lang: op_core::i18n::Lang,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct ImportOutput {
    pub diagram: op_core::flowchart::FlowchartDiagram,
    pub warnings: Vec<String>,
}

pub(crate) fn import_json(input: String, stop: std::sync::Arc<std::sync::atomic::AtomicBool>) -> Result<String, String> {
    let input: ImportInput = serde_json::from_str(&input).map_err(|error| error.to_string())?;
    op_core::i18n::with_lang(input.lang, || {
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        let converted = op_core::flowchart::from_mermaid(&input.source)?;
        if stop.load(std::sync::atomic::Ordering::Relaxed) {
            return Err("Cancelled".into());
        }
        serde_json::to_string(&ImportOutput {
            diagram: converted.diagram,
            warnings: converted.warnings,
        })
        .map_err(|error| error.to_string())
    })
}
