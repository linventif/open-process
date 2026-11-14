//! Composition root: static registration is the only knowledge of concrete modules.
use crate::editor::DiagramRegistry;

pub fn registry() -> &'static DiagramRegistry {
    static REGISTRY: std::sync::OnceLock<DiagramRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = DiagramRegistry::new(vec![], vec![], vec![]).unwrap();
        crate::bpmn_editor::register(&mut registry);
        crate::petri_editor::register(&mut registry);
        crate::uml_editor::register(&mut registry);
        crate::flow_editor::register(&mut registry);
        crate::dmn_editor::register(&mut registry);
        crate::database_editor::register(&mut registry);
        crate::whiteboard_editor::register(&mut registry);
        crate::mermaid_editor::register(&mut registry);
        crate::mining_editor::register(&mut registry);
        registry.validate().expect("built-in identifiers are unique");
        registry
    })
}

#[cfg(target_arch = "wasm32")]
pub fn run_background_job(input: &str) -> Result<String, String> {
    if input.len() > 8 * 1024 * 1024 {
        return Err(op_core::i18n::tr("file.input_limit").into());
    }
    let task: serde_json::Value = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let result = match task["task"].as_str() {
        Some("bpmn.evaluation") => crate::bpmn_editor::evaluation_job::compute_json(input.into(), stop),
        Some("mining.discovery") => crate::mining_editor::analysis_job::compute_json(input.into(), stop),
        Some("mermaid.preview") => crate::mermaid_editor::preview::compute_json(input.into(), stop),
        None | Some("petri.analysis") => crate::petri_editor::analysis_job::compute_json(input.into(), stop),
        Some(_) => return Err("Unknown background task".into()),
    };
    result.and_then(crate::platform::jobs::bounded_result)
}
