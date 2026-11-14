use super::*;
use crate::theme::{self, PresetId};

#[test]
fn theme_picker_preserves_documents_views_history_and_language() {
    let mut h = harness(false);
    let snapshot = h.state().docs[0].snapshot();
    let workspace = h.state().docs[0].editor.workspace();
    let revisions = h.state().docs[0].editor.revisions();
    for preset in theme::all() {
        h.get_by_label("Themes").click();
        h.run_steps(2);
        h.get_by_label(preset.id.label()).click();
        h.run_steps(4);
        assert_eq!(theme::active(&h.ctx), preset.id);
        assert_eq!(h.state().docs[0].snapshot(), snapshot);
        assert_eq!(h.state().docs[0].editor.workspace(), workspace);
        assert_eq!(h.state().docs[0].editor.revisions(), revisions);
        assert_eq!(op_core::i18n::lang(), op_core::i18n::Lang::En);
        shot(&mut h, &format!("appearance_{}", preset.id.slug()));
    }
    theme::set(&h.ctx, PresetId::Classic);
    h.ctx.set_theme(egui::Theme::Light);
    h.get_by_label("Home").click();
    h.run_steps(4);
    for width in [1400.0, 720.0, 320.0] {
        h.set_size(egui::vec2(width, 1000.0));
        for preset in theme::all() {
            theme::set(&h.ctx, preset.id);
            h.run_steps(4);
            let card = h.get_by_label("Sketch freely").rect();
            assert!(card.left() >= 0.0 && card.right() <= width);
        }
    }
}

/// Generate real gallery captures for every appearance and both color modes.
#[test]
fn appearance_gallery_captures_keep_document_snapshots_unchanged() {
    let mut h = harness(false);
    for id in [
        "whiteboard.ideas",
        "database.shop",
        "uml.classes",
        "petri.mutex",
        "log.second",
        "mermaid.sequence",
    ] {
        h.state_mut().open_example(id);
        h.run_steps(6);
        if id == "log.second" {
            wait_for_discovery(&mut h);
        }
    }
    let before: Vec<_> = h.state().docs.iter().map(|doc| doc.snapshot()).collect();
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/themes");
    // The default order process plus the six registry examples above.
    for (view, index) in [
        ("draw", 1),
        ("database", 2),
        ("uml", 3),
        ("bpmn", 0),
        ("petri", 4),
        ("mining", 5),
        ("mermaid", 6),
        ("split", 0),
    ] {
        h.state_mut().set_active(index);
        if view == "petri" {
            h.state_mut().docs[index]
                .editor
                .as_any_mut()
                .downcast_mut::<PetriEditor>()
                .unwrap()
                .show_matrices();
        }
        if view == "split" {
            let uid = h.state().docs[4].uid;
            h.state_mut().open_beside(
                Pane {
                    doc: uid,
                    view: View::Matrices,
                },
                false,
            );
        }
        h.run_steps(8);
        for preset in theme::all() {
            theme::set(&h.ctx, preset.id);
            for (mode, dark) in [("light", false), ("dark", true)] {
                h.ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
                h.run_steps(4);
                let dir = base.join(preset.id.slug()).join(mode);
                std::fs::create_dir_all(&dir).unwrap();
                match h.render() {
                    Ok(image) => image.save(dir.join(format!("{view}.png"))).unwrap(),
                    Err(error) => eprintln!("gallery rendering unavailable: {error}"),
                }
                assert_eq!(h.state().docs.iter().map(|doc| doc.snapshot()).collect::<Vec<_>>(), before);
            }
        }
    }
}
