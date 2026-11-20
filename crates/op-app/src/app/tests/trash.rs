use super::*;
use crate::platform::autosave::{Entry, Library, Record, TRASH_DAYS};
use op_core::document::{DocumentCodec, JsonCodec};

fn trashed_at(h: &Harness<'static, OpenProcessApp>, id: &str) -> Option<Option<u64>> {
    h.state().automatic.entries.iter().find(|e| e.id == id).map(|e| e.trashed_at)
}

#[test]
fn saved_diagrams_move_to_the_trash_restore_and_erase() {
    let directory = tempfile::tempdir().unwrap();
    let mut h = harness_sized(false, [1200.0, 900.0]);
    let mut library = Library::at(directory.path().into(), h.ctx.clone()).unwrap();
    let record = Record {
        entry: Entry::new("Archived process".into(), "bpmn".into(), None),
        state: StoredDocument::Editable(JsonCodec::new("bpmn").encode(&BpmnDiagram::new("Archived process"))),
    };
    let entry = library.save_now(record, Default::default()).unwrap().remove(0).record.entry;
    library.remember(entry.clone());
    h.state_mut().automatic = library;
    h.state_mut().show_home = true;
    h.run_steps(4);
    assert!(h.query_by_label("Archived process").is_some());
    h.get_by_label("Move to trash").hover();
    h.run_steps(2);
    shot(&mut h, "saved_diagram_card_trash_button");
    h.hover_at(egui::pos2(5.0, 5.0));
    h.run_steps(3);

    h.get_by_label("Move to trash").click();
    h.run_steps(4);
    assert!(h.query_by_label("Archived process").is_none(), "no longer a saved card");
    assert!(matches!(trashed_at(&h, &entry.id), Some(Some(_))));
    assert_eq!(h.state().docs.len(), 1, "trashing does not open the diagram");
    h.get_by_label_contains("Trash (1)").click();
    h.run_steps(4);
    assert!(h.query_by_label_contains(&format!("{TRASH_DAYS} days")).is_some());
    assert!(h.query_by_label(&format!("Days left: {TRASH_DAYS}")).is_some());
    shot(&mut h, "saved_diagram_trash");

    h.get_by_label("Restore").click();
    h.run_steps(4);
    assert!(h.query_by_label("Archived process").is_some(), "back among saved diagrams");
    assert_eq!(trashed_at(&h, &entry.id), Some(None));

    h.get_by_label("Move to trash").click();
    h.run_steps(4);
    h.get_by_label("Delete permanently").click();
    h.run_steps(4);
    assert_eq!(trashed_at(&h, &entry.id), None, "erased");
    assert!(!directory.path().join(format!("{}.opx", entry.id)).exists());
    assert!(!directory.path().join(format!("{}.json", entry.id)).exists());
    assert!(h.query_by_label_contains("Trash (").is_none());
}
