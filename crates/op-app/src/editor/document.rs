use super::*;
use op_core::i18n::{key, tr};

pub struct Doc {
    pub uid: u64,
    pub editor: Box<dyn EditorSession>,
    pub path: Option<String>,
    saved: Option<StoredDocument>,
    descriptor: Option<&'static DiagramDescriptor>,
    dirty_cache: std::cell::Cell<Option<(Revisions, bool)>>,
}

impl Doc {
    pub fn new(state: StoredDocument, path: Option<String>, saved: bool, registry: &'static DiagramRegistry) -> Self {
        let editor = match &state {
            StoredDocument::Editable(d) => registry.open(d).unwrap_or_else(|e| {
                Box::new(RecoverySession {
                    bytes: d.to_json().into_bytes(),
                    reason: e.to_string(),
                    name: String::new(),
                })
            }),
            StoredDocument::Original { bytes, reason } => Box::new(RecoverySession {
                bytes: bytes.clone(),
                reason: reason.clone(),
                name: String::new(),
            }),
        };
        let descriptor = registry.find(editor.kind());
        let snapshot = saved.then(|| editor.snapshot());
        Self {
            uid: crate::canvas::next_uid(),
            editor,
            path,
            saved: snapshot,
            descriptor,
            dirty_cache: Default::default(),
        }
    }
    pub fn snapshot(&self) -> StoredDocument {
        self.editor.snapshot()
    }
    pub fn name(&self) -> String {
        let name = self.editor.name();
        if name.trim().is_empty() {
            tr("Untitled").into()
        } else {
            name.into()
        }
    }
    pub fn mark_saved(&mut self, snapshot: StoredDocument) {
        self.saved = Some(snapshot);
        self.dirty_cache.set(None);
    }
    /// Only a completed write/download changes the saved reference.
    pub fn complete_save(&mut self, snapshot: StoredDocument, result: Result<Option<String>, String>) -> Result<Option<String>, String> {
        if let Ok(Some(path)) = &result {
            self.path = Some(path.clone());
            self.mark_saved(snapshot);
        }
        result
    }
    pub fn dirty(&self) -> bool {
        let revision = self.editor.revisions();
        if let Some((previous, dirty)) = self.dirty_cache.get() {
            if previous == revision {
                return dirty;
            }
        }
        let dirty = self.saved.as_ref() != Some(&self.snapshot());
        self.dirty_cache.set(Some((revision, dirty)));
        dirty
    }
    pub fn file_stem(&self) -> String {
        let s: String = self
            .name()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        let stem = s.trim_matches('_');
        if stem.is_empty() { "document".into() } else { stem.into() }
    }
    pub fn view_label(&self, view: &str) -> &str {
        self.editor.views().iter().find(|v| v.id == view).map(|v| tr(v.label)).unwrap_or("")
    }
    pub fn badge(&self) -> (&str, egui::Color32) {
        self.descriptor.map(|d| (d.badge, d.color)).unwrap_or(("?", egui::Color32::GRAY))
    }
    #[cfg(test)]
    pub fn model(&self) -> op_core::document::Model {
        let StoredDocument::Editable(d) = self.snapshot() else {
            panic!("original document has no typed model")
        };
        op_core::document::Model::from_document(&d).unwrap()
    }
}

struct RecoverySession {
    bytes: Vec<u8>,
    reason: String,
    name: String,
}
impl EditorSession for RecoverySession {
    fn kind(&self) -> &str {
        "unavailable"
    }
    fn name(&self) -> &str {
        &self.name
    }
    fn rename(&mut self, name: String) {
        self.name = name;
    }
    fn snapshot(&self) -> StoredDocument {
        StoredDocument::Original {
            bytes: self.bytes.clone(),
            reason: self.reason.clone(),
        }
    }
    fn revisions(&self) -> Revisions {
        Revisions::default()
    }
    fn render(&mut self, ui: &mut egui::Ui, _: u64, _: &str) -> Vec<HostEvent> {
        ui.colored_label(ui.visuals().warn_fg_color, &self.reason);
        ui.label(tr("Original data preserved. Use Save as to recover the file."));
        vec![]
    }
    fn retain_views(&mut self, _: &[u64]) {}
    fn workspace(&self) -> serde_json::Value {
        serde_json::Value::Null
    }
    fn restore_workspace(&mut self, _: &serde_json::Value) {}
    fn commands(&self) -> &'static [CommandDescriptor] {
        &[]
    }
    fn enabled(&self, _: &str) -> bool {
        false
    }
    fn execute(&mut self, _: &str, _: f64) -> Vec<HostEvent> {
        vec![]
    }
    fn views(&self) -> &'static [ViewDescriptor] {
        &[]
    }
    fn export(&self) -> Result<ExportData, String> {
        Ok(ExportData {
            suffix: "opx",
            bytes: self.bytes.clone(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("Save as…")
    }
    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
