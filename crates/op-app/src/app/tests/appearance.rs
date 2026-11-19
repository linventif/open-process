use super::*;

/// Generate real gallery captures for Classic in both color modes.
#[test]
fn classic_gallery_captures_keep_document_snapshots_unchanged() {
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
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots/classic");
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
        for (mode, dark) in [("light", false), ("dark", true)] {
            h.ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
            h.run_steps(4);
            let dir = base.join(mode);
            std::fs::create_dir_all(&dir).unwrap();
            match h.render() {
                Ok(image) => image.save(dir.join(format!("{view}.png"))).unwrap(),
                Err(error) => eprintln!("gallery rendering unavailable: {error}"),
            }
            assert_eq!(h.state().docs.iter().map(|doc| doc.snapshot()).collect::<Vec<_>>(), before);
        }
    }
}

#[test]
fn choosing_the_system_theme_resumes_following_it() {
    use egui::{Theme, ThemePreference};
    assert_eq!(super::super::toggled_preference(Theme::Dark, Theme::Dark), ThemePreference::System);
    assert_eq!(
        super::super::toggled_preference(Theme::Light, Theme::Light),
        ThemePreference::System
    );
    assert_eq!(super::super::toggled_preference(Theme::Light, Theme::Dark), ThemePreference::Light);
    assert_eq!(super::super::toggled_preference(Theme::Dark, Theme::Light), ThemePreference::Dark);
    // While following, a new fallback (the Linux desktop preference) applies at once.
    let ctx = egui::Context::default();
    ctx.set_theme(ThemePreference::System);
    for theme in [Theme::Light, Theme::Dark] {
        ctx.options_mut(|o| o.fallback_theme = theme);
        assert_eq!(ctx.theme(), theme);
        assert_eq!(super::super::system_theme(&ctx), theme);
    }
}
