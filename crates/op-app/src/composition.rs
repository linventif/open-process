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
        crate::mining_editor::register(&mut registry);
        registry.validate().expect("built-in identifiers are unique");
        registry
    })
}

#[cfg(target_arch = "wasm32")]
pub fn run_background_job(input: &str) -> Result<String, String> {
    crate::petri_editor::analysis_job::compute_json(input.into(), std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)))
}
