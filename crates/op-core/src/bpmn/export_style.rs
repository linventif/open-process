//! Presentation-only colors shared by vector and BPMN DI exports.
//!
//! The palette does not add document properties or change process semantics.
use super::NodeKind;

pub(crate) const INK: &str = "#243342";
pub(crate) const POOL_HEADER: &str = "#e2e8f0";

pub(crate) struct Colors {
    pub fill: &'static str,
    pub stroke: &'static str,
}

pub(crate) fn node(kind: NodeKind) -> Colors {
    let (fill, stroke) = match kind {
        NodeKind::StartEvent(_) => ("#dcfce7", "#166534"),
        NodeKind::EndEvent(_) => ("#ffe4e6", "#9f1239"),
        NodeKind::IntermediateEvent { .. } | NodeKind::Gateway(_) => ("#fef3c7", "#92400e"),
        NodeKind::Task(_) | NodeKind::SubProcess => ("#dbeafe", "#1e40af"),
        NodeKind::DataObject | NodeKind::DataStore => ("#ede9fe", "#6b21a8"),
        NodeKind::TextAnnotation => ("#ffffff", INK),
    };
    Colors { fill, stroke }
}

pub(crate) fn lane(index: usize) -> &'static str {
    if index.is_multiple_of(2) { "#f8fafc" } else { "#f0f4f8" }
}
