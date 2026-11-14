//! Workspace migration is separate from the native document file format.
use crate::{
    dock::{self, Pane},
    editor::StoredDocument,
};
use op_core::document::Model;
use serde::{Deserialize, Serialize};

pub const KEY: &str = "open-process-v4";
pub const LEGACY_KEY: &str = "open-process-v1";
pub const LEGACY_KEY_V2: &str = "open-process-v2";
pub const LEGACY_KEY_V3: &str = "open-process-v3";

#[derive(Serialize, Deserialize, Default)]
pub struct LegacyV1 {
    pub docs: Vec<(Model, Option<String>)>,
    pub active: usize,
}
#[derive(Serialize, Deserialize)]
pub struct LegacyV2 {
    pub docs: Vec<(u64, Model, Option<String>)>,
    pub active: usize,
    pub tree: egui_tiles::Tree<LegacyPane>,
}
#[derive(Serialize, Deserialize)]
pub struct LegacyPane {
    pub doc: u64,
    pub view: LegacyView,
}
#[derive(Serialize, Deserialize)]
pub enum LegacyView {
    Editor,
    Matrices,
    Analysis,
    States,
    Graph,
}
#[derive(Serialize, Deserialize)]
pub struct Workspace {
    pub version: u32,
    pub docs: Vec<(u64, StoredDocument, Option<String>, serde_json::Value)>,
    pub active: usize,
    pub tree: egui_tiles::Tree<Pane>,
    #[serde(default)]
    pub autosave: std::collections::HashMap<u64, super::autosave::Entry>,
    /// Confirmed automatic copies need only references, avoiding duplicate browser payloads.
    #[serde(default)]
    pub archived: Vec<(usize, u64, super::autosave::Entry, serde_json::Value)>,
}

pub fn load(storage: &dyn eframe::Storage) -> Result<Option<Workspace>, String> {
    if let Some(raw) = storage.get_string(KEY) {
        let w: Workspace = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if w.version != 4 {
            return Err(format!("Unsupported workspace version: {}", w.version));
        }
        return Ok(Some(w));
    }
    if let Some(raw) = storage.get_string(LEGACY_KEY_V3) {
        let mut workspace: Workspace = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if workspace.version != 3 {
            return Err(format!("Unsupported workspace version: {}", workspace.version));
        }
        workspace.version = 4;
        return Ok(Some(workspace));
    }
    if storage.get_string(LEGACY_KEY_V2).is_some() {
        let p: LegacyV2 = eframe::get_value(storage, LEGACY_KEY_V2).ok_or("Invalid legacy v2 workspace")?;
        let tree = serde_json::from_value(serde_json::to_value(p.tree).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        return Ok(Some(Workspace {
            version: 4,
            docs: p
                .docs
                .into_iter()
                .map(|(id, m, path)| (id, StoredDocument::Editable(m.document()), path, serde_json::Value::Null))
                .collect(),
            active: p.active,
            tree,
            autosave: Default::default(),
            archived: vec![],
        }));
    }
    if storage.get_string(LEGACY_KEY).is_some() {
        let p: LegacyV1 = eframe::get_value(storage, LEGACY_KEY).ok_or("Invalid legacy v1 workspace")?;
        return Ok(Some(Workspace {
            version: 4,
            docs: p
                .docs
                .into_iter()
                .enumerate()
                .map(|(id, (m, path))| (id as u64, StoredDocument::Editable(m.document()), path, serde_json::Value::Null))
                .collect(),
            active: p.active,
            tree: dock::new_tree(),
            autosave: Default::default(),
            archived: vec![],
        }));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::Storage as _;
    #[derive(Default)]
    struct Storage(std::collections::HashMap<String, String>);
    impl eframe::Storage for Storage {
        fn get_string(&self, key: &str) -> Option<String> {
            self.0.get(key).cloned()
        }
        fn set_string(&mut self, key: &str, value: String) {
            self.0.insert(key.into(), value);
        }
        fn remove_string(&mut self, key: &str) {
            self.0.remove(key);
        }
        fn flush(&mut self) {}
    }
    #[test]
    fn legacy_sessions_migrate_without_modifying_old_keys() {
        let mut storage = Storage::default();
        let model = Model::Bpmn(op_core::examples::order_process());
        let old = LegacyV1 {
            docs: vec![(model.clone(), Some("saved.opx".into()))],
            active: 0,
        };
        eframe::set_value(&mut storage, LEGACY_KEY, &old);
        let original = storage.0.clone();
        let migrated = load(&storage).unwrap().unwrap();
        assert_eq!(migrated.docs[0].1, StoredDocument::Editable(model.document()));
        assert_eq!(storage.0, original);
        let old = LegacyV2 {
            docs: vec![(44, model, Some("saved.opx".into()))],
            active: 0,
            tree: egui_tiles::Tree::new_tabs(
                egui::Id::unique("legacy"),
                vec![
                    LegacyPane {
                        doc: 44,
                        view: LegacyView::Editor,
                    },
                    LegacyPane {
                        doc: 44,
                        view: LegacyView::Matrices,
                    },
                ],
            ),
        };
        eframe::set_value(&mut storage, LEGACY_KEY_V2, &old);
        let migrated = load(&storage).unwrap().unwrap();
        assert_eq!(migrated.docs[0].0, 44);
        assert_eq!(dock::panes_of(&migrated.tree, 44).len(), 2);
        storage.set_string(KEY, serde_json::to_string(&migrated).unwrap());
        let back = load(&storage).unwrap().unwrap();
        assert_eq!(back.docs[0].1, migrated.docs[0].1);
        assert!(storage.get_string(LEGACY_KEY).is_some());
        assert!(storage.get_string(LEGACY_KEY_V2).is_some());
    }
    #[test]
    fn v3_workspace_migrates_without_overwriting_its_source() {
        let mut storage = Storage::default();
        let old = serde_json::json!({
            "version": 3,
            "docs": [[77, StoredDocument::Editable(Model::Bpmn(op_core::examples::order_process()).document()), "chosen.opx", null]],
            "active": 0,
            "tree": dock::new_tree(),
        })
        .to_string();
        storage.set_string(LEGACY_KEY_V3, old.clone());
        let migrated = load(&storage).unwrap().unwrap();
        assert_eq!(migrated.version, 4);
        assert_eq!(migrated.docs[0].0, 77);
        assert_eq!(migrated.docs[0].2.as_deref(), Some("chosen.opx"));
        assert!(migrated.autosave.is_empty() && migrated.archived.is_empty());
        assert_eq!(storage.get_string(LEGACY_KEY_V3), Some(old));
        storage.set_string(KEY, "{malformed".into());
        assert!(load(&storage).is_err());
    }

    #[test]
    fn malformed_new_workspace_does_not_silently_fall_back() {
        let mut storage = Storage::default();
        eframe::set_value(&mut storage, LEGACY_KEY, &LegacyV1::default());
        storage.set_string(KEY, "{broken".into());
        let original = storage.0.clone();
        assert!(load(&storage).is_err());
        assert_eq!(storage.0, original);
    }
}
