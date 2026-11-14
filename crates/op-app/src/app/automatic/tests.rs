use super::*;
use egui_kittest::{Harness, kittest::Queryable};
use std::{fs, path::Path};

fn isolated(root: &Path) -> Harness<'static, OpenProcessApp> {
    let root = root.to_path_buf();
    Harness::builder().with_size([1400.0, 1000.0]).build_eframe(move |cc| {
        let mut app = OpenProcessApp::new(cc, vec![], crate::composition::registry());
        app.automatic = autosave::Library::at(root, cc.egui_ctx.clone()).unwrap();
        app
    })
}

#[test]
fn automatic_copies_preserve_latest_edits_on_close_and_manual_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Open Process");
    let manual = temporary.path().join("chosen.opx");
    fs::write(&manual, b"original manual copy").unwrap();
    let mut h = isolated(&root);
    h.state_mut()
        .open_model(Model::Bpmn(examples::order_process()), Some(manual.display().to_string()), true);
    h.state_mut().docs[0].editor.rename("First edit".into());
    let ctx = h.ctx.clone();
    h.state_mut().save_automatically(&ctx);
    assert!(h.state().automatic.pending());
    h.state_mut().docs[0].editor.rename("Latest edit".into());
    h.state_mut().request_close(0);
    assert!(h.state().confirm_close.is_none());
    assert!(h.state().docs.is_empty());
    assert_eq!(h.state().automatic.entries.len(), 1);
    assert_eq!(fs::read(&manual).unwrap(), b"original manual copy");
    h.run_steps(4);
    assert!(h.query_by_label("Saved diagrams").is_some());
    let screenshots = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
    fs::create_dir_all(&screenshots).unwrap();
    h.render().unwrap().save(screenshots.join("automatic_library.png")).unwrap();
    h.get_by_label("Latest edit").click();
    h.run_steps(4);
    assert_eq!(h.state().docs.len(), 1, "library open failed: {:?}", h.state().automatic_error);
    assert_eq!(h.state().docs[0].name(), "Latest edit");
    assert!(!h.state().docs[0].dirty());
    assert!(h.state_mut().save(false));
    assert_eq!(fs::read(&manual).unwrap(), h.state().docs[0].snapshot().bytes());

    let mut restarted = isolated(&root);
    assert_eq!(restarted.state().automatic.entries.len(), 1);
    restarted.state_mut().open_automatic_document(0);
    assert_eq!(restarted.state().docs[0].name(), "Latest edit");
}

#[test]
fn automatic_failure_keeps_content_dirty_and_close_confirmation() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("Open Process");
    let mut h = isolated(&root);
    fs::remove_dir(&root).unwrap();
    fs::write(&root, b"blocked destination").unwrap();
    h.state_mut().open_model(Model::Bpmn(examples::order_process()), None, false);
    h.state_mut().request_close(0);
    assert_eq!(h.state().confirm_close, Some(0));
    assert!(h.state().docs[0].dirty());
    assert!(h.state().automatic_error.is_some());
    assert!(h.state().automatic.entries.is_empty());
    assert_eq!(fs::read(&root).unwrap(), b"blocked destination");
    fs::remove_file(&root).unwrap();
    fs::create_dir(&root).unwrap();
    h.state_mut().confirm_close = None;
    h.state_mut().request_close(0);
    assert!(h.state().confirm_close.is_none());
    assert!(h.state().docs.is_empty());
    assert_eq!(h.state().automatic.entries.len(), 1);
}

#[test]
fn automatic_workspace_references_preserve_order_and_unfinished_changes() {
    let temporary = tempfile::tempdir().unwrap();
    let mut h = isolated(temporary.path());
    h.state_mut().open_model(Model::Bpmn(examples::order_process()), None, false);
    h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
    h.state_mut().open_model(Model::Uml(examples::uml_classes()), None, false);
    h.state_mut().flush_automatic_documents();
    let compact = h.state().workspace_snapshot();
    assert!(compact.docs.is_empty());
    assert_eq!(compact.archived.len(), 3);
    h.state_mut().docs[1].editor.rename("Unfinished edit".into());
    let mut workspace = h.state().workspace_snapshot();
    assert_eq!(workspace.docs.len(), 1);
    assert_eq!(workspace.archived.len(), 2);
    h.state().resolve_automatic_workspace(&mut workspace).unwrap();
    assert_eq!(
        workspace.docs.iter().map(|doc| doc.0).collect::<Vec<_>>(),
        h.state().docs.iter().map(|doc| doc.uid).collect::<Vec<_>>()
    );
    assert_eq!(workspace.docs[1].1, h.state().docs[1].snapshot());
    assert_eq!(workspace.active, 2);
    let missing_id = &compact.archived[0].2.id;
    fs::remove_file(temporary.path().join(format!("{missing_id}.opx"))).unwrap();
    let mut missing = compact;
    let before = serde_json::to_string(&missing).unwrap();
    assert!(h.state().resolve_automatic_workspace(&mut missing).is_err());
    assert_eq!(serde_json::to_string(&missing).unwrap(), before);
}

#[test]
fn automatic_restore_keeps_pending_changes_when_another_writer_advanced() {
    let temporary = tempfile::tempdir().unwrap();
    let mut h = isolated(temporary.path());
    h.state_mut().open_model(Model::Bpmn(examples::order_process()), None, false);
    h.state_mut().flush_automatic_documents();
    let entry = h.state().docs[0].archive.clone().unwrap();
    h.state_mut().docs[0].editor.rename("Pending own edit".into());
    let workspace = h.state().workspace_snapshot();
    let mut other = autosave::Library::at(temporary.path().into(), egui::Context::default()).unwrap();
    let mut changed_elsewhere = examples::order_process();
    changed_elsewhere.name = "Changed elsewhere".into();
    let record = autosave::Record {
        entry: entry.clone(),
        state: StoredDocument::Editable(Model::Bpmn(changed_elsewhere).document()),
    };
    assert!(other.save_now(record, Revisions::default()).unwrap()[0].result.is_ok());
    let mut pending = Doc::new(workspace.docs[0].1.clone(), None, false, crate::composition::registry());
    h.state().restore_automatic_reference(&mut pending, entry.clone());
    assert_eq!(pending.name(), "Pending own edit");
    assert!(pending.archive.is_none());
    assert!(pending.dirty());
    h.state_mut().docs[0] = pending;
    h.state_mut().flush_automatic_documents();
    assert_ne!(h.state().docs[0].archive.as_ref().unwrap().id, entry.id);
    assert_eq!(h.state().automatic.entries.len(), 2);
    let original = h.state().automatic.load(&entry.id).unwrap();
    let StoredDocument::Editable(original) = original.state else {
        panic!("editable copy expected")
    };
    assert_eq!(original.model["name"], "Changed elsewhere");
}

#[test]
fn automatic_identities_conflicts_and_original_bytes_survive_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let mut library = autosave::Library::at(temporary.path().into(), egui::Context::default()).unwrap();
    let entry = autosave::Entry::new("Same name".into(), "bpmn".into(), None);
    let original = Model::Bpmn(examples::order_process()).document();
    let record = autosave::Record {
        entry,
        state: StoredDocument::Editable(original),
    };
    let completion = library.save_now(record, Revisions::default()).unwrap().pop().unwrap();
    assert!(completion.result.is_ok());
    let first = completion.record.clone();
    library.remember(first.entry.clone());
    let mut newer = first.clone();
    newer.entry.name = "Newer name".into();
    let completion = library.save_now(newer, Revisions::default()).unwrap().pop().unwrap();
    assert!(completion.result.is_ok());
    library.remember(completion.record.entry.clone());
    let stale = library.save_now(first, Revisions::default()).unwrap().pop().unwrap();
    assert!(stale.result.is_err());
    let entry = autosave::Entry::new("Same name".into(), "unavailable".into(), None);
    let bytes = br#" { "format": "open-process", "version": 2, "document": { "kind": "future.kind", "model_version": 1, "model": { "keep": true } }, "unknown-field": 42 } "#.to_vec();
    let record = autosave::Record {
        entry,
        state: StoredDocument::Original {
            bytes: bytes.clone(),
            reason: "Future format".into(),
        },
    };
    let completed = library.save_now(record, Revisions::default()).unwrap().pop().unwrap();
    assert!(completed.result.is_ok());
    let original_id = completed.record.entry.id;
    let restarted = autosave::Library::at(temporary.path().into(), egui::Context::default()).unwrap();
    assert_eq!(restarted.entries.len(), 2);
    assert_eq!(restarted.load(&original_id).unwrap().state.bytes(), bytes);
    assert!(restarted.entries.iter().any(|entry| entry.name == "Newer name"));
}
