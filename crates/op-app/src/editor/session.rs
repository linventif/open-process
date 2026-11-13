use egui::Ui;
use op_core::document::Document;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum StoredDocument {
    Editable(Document),
    Original { bytes: Vec<u8>, reason: String },
}

impl StoredDocument {
    pub fn bytes(&self) -> Vec<u8> {
        match self {
            Self::Editable(d) => d.to_json().into_bytes(),
            Self::Original { bytes, .. } => bytes.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Revisions {
    pub semantic: u64,
    pub presentation: u64,
}

pub struct ExportDescriptor {
    pub id: &'static str,
    pub label: &'static str,
}

pub struct ExportData {
    pub suffix: &'static str,
    pub bytes: Vec<u8>,
}

pub enum HostEvent {
    Open(Document),
    OpenView(crate::dock::View),
    Message(String),
}

#[derive(Clone, Copy, PartialEq)]
pub enum CommandGroup {
    Edit,
    View,
    Tools,
}

pub struct CommandDescriptor {
    pub id: &'static str,
    pub label: &'static str,
    pub group: CommandGroup,
}

pub const UNDO: &str = "edit.undo";
pub const REDO: &str = "edit.redo";
pub const DELETE: &str = "edit.delete";
pub const FIT: &str = "view.fit";
pub const PNG: &str = "export.png";

pub struct ViewDescriptor {
    pub id: &'static str,
    pub label: &'static str,
}

/// One mutable document and history, independently rendered in multiple panes.
pub trait EditorSession {
    fn kind(&self) -> &str;
    fn name(&self) -> &str;
    fn rename(&mut self, name: String);
    fn snapshot(&self) -> StoredDocument;
    fn revisions(&self) -> Revisions;
    fn render(&mut self, ui: &mut Ui, instance: u64, view: &str) -> Vec<HostEvent>;
    fn retain_views(&mut self, instances: &[u64]);
    fn workspace(&self) -> serde_json::Value;
    fn restore_workspace(&mut self, state: &serde_json::Value);
    fn commands(&self) -> &'static [CommandDescriptor];
    fn enabled(&self, command: &str) -> bool;
    fn execute(&mut self, command: &str, time: f64) -> Vec<HostEvent>;
    fn views(&self) -> &'static [ViewDescriptor];
    fn export(&self) -> Result<ExportData, String>;
    fn export_label(&self) -> &'static str;
    fn exports(&self) -> Vec<ExportDescriptor> {
        vec![ExportDescriptor {
            id: "standard",
            label: self.export_label(),
        }]
    }
    fn export_as(&self, format: &str) -> Result<ExportData, String> {
        if format == "standard" {
            self.export()
        } else {
            Err(format!("Unknown export format: {format}"))
        }
    }
    // Test-only probes keep UI regression tests on their concrete typed editors.
    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any;
    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}

/// Implemented next to each notation, where private view state remains accessible.
pub trait EditorBackend: 'static {
    type ViewState: Clone;
    fn capture_view(&self) -> Self::ViewState;
    fn restore_view(&mut self, state: &Self::ViewState);
    fn fresh_view(&self) -> Self::ViewState;
    fn save_view(_state: &Self::ViewState) -> serde_json::Value {
        serde_json::Value::Null
    }
    fn load_view(_state: &mut Self::ViewState, _value: &serde_json::Value) {}
    fn kind(&self) -> &'static str;
    fn name(&self) -> &str;
    fn rename(&mut self, name: String);
    fn document(&self) -> Document;
    fn semantic_document(&self) -> Document {
        self.document()
    }
    fn presentation_key(&self) -> serde_json::Value {
        serde_json::Value::Null
    }
    fn cancel_edit(&mut self) {}
    fn ui(&mut self, ui: &mut Ui, view: &str) -> Vec<HostEvent>;
    fn commands(&self) -> &'static [CommandDescriptor];
    fn enabled(&self, command: &str) -> bool;
    fn execute(&mut self, command: &str, time: f64) -> Vec<HostEvent>;
    fn views(&self) -> &'static [ViewDescriptor] {
        &[]
    }
    fn export(&self) -> Result<ExportData, String>;
    fn export_label(&self) -> &'static str;
    fn exports(&self) -> Vec<ExportDescriptor> {
        vec![ExportDescriptor {
            id: "standard",
            label: self.export_label(),
        }]
    }
    fn export_as(&self, format: &str) -> Result<ExportData, String> {
        if format == "standard" {
            self.export()
        } else {
            Err(format!("Unknown export format: {format}"))
        }
    }
}

pub struct Session<E: EditorBackend> {
    editor: E,
    views: HashMap<u64, E::ViewState>,
    active: Option<u64>,
    revisions: Revisions,
    previous: Document,
    semantic: Document,
    presentation: serde_json::Value,
}

impl<E: EditorBackend> Session<E> {
    fn observe(&mut self) {
        let current = self.editor.document();
        if current != self.previous {
            let semantic = self.editor.semantic_document();
            if semantic != self.semantic {
                self.revisions.semantic += 1;
                self.semantic = semantic;
            }
            let presentation = self.editor.presentation_key();
            if presentation != self.presentation {
                self.revisions.presentation += 1;
                self.presentation = presentation;
            }
            self.previous = current;
        }
    }

    pub fn boxed(editor: E) -> Box<dyn EditorSession> {
        Box::new(Self {
            previous: editor.document(),
            semantic: editor.semantic_document(),
            presentation: editor.presentation_key(),
            editor,
            views: HashMap::new(),
            active: None,
            revisions: Revisions::default(),
        })
    }
}

impl<E: EditorBackend> EditorSession for Session<E> {
    fn kind(&self) -> &str {
        self.editor.kind()
    }
    fn name(&self) -> &str {
        self.editor.name()
    }
    fn rename(&mut self, name: String) {
        self.editor.rename(name);
        self.observe();
    }
    fn snapshot(&self) -> StoredDocument {
        StoredDocument::Editable(self.editor.document())
    }
    fn revisions(&self) -> Revisions {
        self.revisions
    }
    fn render(&mut self, ui: &mut Ui, instance: u64, view: &str) -> Vec<HostEvent> {
        if self.active.is_none() || (ui.rect_contains_pointer(ui.max_rect()) && ui.input(|i| i.pointer.any_pressed())) {
            self.active = Some(instance);
        }
        let state = self.views.entry(instance).or_insert_with(|| self.editor.fresh_view());
        self.editor.restore_view(state);
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.editor.cancel_edit();
        }
        let events = ui.push_id(instance, |ui| self.editor.ui(ui, view)).inner;
        self.views.insert(instance, self.editor.capture_view());
        self.observe();
        events
    }
    fn retain_views(&mut self, instances: &[u64]) {
        self.views.retain(|id, _| instances.contains(id));
        if self.active.is_some_and(|id| !instances.contains(&id)) {
            self.active = instances.first().copied();
        }
    }
    fn workspace(&self) -> serde_json::Value {
        serde_json::to_value(self.views.iter().map(|(id, state)| (*id, E::save_view(state))).collect::<Vec<_>>()).unwrap()
    }
    fn restore_workspace(&mut self, value: &serde_json::Value) {
        if let Ok(views) = serde_json::from_value::<Vec<(u64, serde_json::Value)>>(value.clone()) {
            for (id, value) in views {
                let mut state = self.editor.fresh_view();
                E::load_view(&mut state, &value);
                self.views.insert(id, state);
            }
        }
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
        self.editor.commands()
    }
    fn enabled(&self, command: &str) -> bool {
        self.editor.enabled(command)
    }
    fn execute(&mut self, command: &str, time: f64) -> Vec<HostEvent> {
        // Commands affecting the viewport must also update the stored view states.
        if command == FIT {
            for state in self.views.values_mut() {
                self.editor.restore_view(state);
                self.editor.execute(command, time);
                *state = self.editor.capture_view();
            }
        }
        if let Some(id) = self.active {
            if let Some(state) = self.views.get(&id) {
                self.editor.restore_view(state);
            }
            let events = self.editor.execute(command, time);
            self.views.insert(id, self.editor.capture_view());
            self.observe();
            events
        } else {
            let events = self.editor.execute(command, time);
            self.observe();
            events
        }
    }
    fn views(&self) -> &'static [ViewDescriptor] {
        self.editor.views()
    }
    fn export(&self) -> Result<ExportData, String> {
        self.editor.export()
    }
    fn export_label(&self) -> &'static str {
        self.editor.export_label()
    }
    fn exports(&self) -> Vec<ExportDescriptor> {
        self.editor.exports()
    }
    fn export_as(&self, format: &str) -> Result<ExportData, String> {
        self.editor.export_as(format)
    }
    #[cfg(test)]
    fn as_any(&self) -> &dyn std::any::Any {
        &self.editor
    }
    #[cfg(test)]
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self.views.clear();
        self.revisions.semantic += 1;
        self.revisions.presentation += 1;
        &mut self.editor
    }
}

/// Snapshot/view boilerplate; the model and history are deliberately excluded.
macro_rules! view_state {
    ($editor:ty, $state:ident, $($field:ident: $ty:ty),+ $(,)?) => {
        #[derive(Clone)]
        pub struct $state { $( $field: $ty, )+ }
        impl $editor {
            fn capture_state(&self) -> $state { $state { $( $field: self.$field.clone(), )+ } }
            fn restore_state(&mut self, state: &$state) { $( self.$field = state.$field.clone(); )+ }
        }
    };
}
pub(crate) use view_state;
