use super::*;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn harness(dark: bool) -> Harness<'static, OpenProcessApp> {
    harness_sized(dark, [1400.0, 860.0])
}

fn harness_sized(dark: bool, size: [f32; 2]) -> Harness<'static, OpenProcessApp> {
    let mut h = Harness::builder().with_size(size).build_eframe(|cc| {
        let mut app = OpenProcessApp::new(cc, Vec::new(), crate::composition::registry());
        app.open_model(Model::Bpmn(examples::order_process()), None, false);
        app
    });
    h.ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
    h.run_steps(4);
    h
}

#[test]
fn home_navigation() {
    let mut h = Harness::builder()
        .with_size([1400.0, 1000.0])
        .build_eframe(|cc| OpenProcessApp::new(cc, Vec::new(), crate::composition::registry()));
    h.ctx.set_theme(egui::Theme::Light);
    h.run_steps(4);
    assert!(h.state().show_home);
    assert!(h.state().docs.is_empty());
    shot(&mut h, "home_light");

    h.get_by_label("Model a process").click();
    h.run_steps(4);
    assert!(!h.state().show_home);
    assert_eq!(h.state().docs[0].editor.kind(), "bpmn");
    let uid = h.state().docs[0].uid;

    h.get_by_label("Home").click();
    h.run_steps(4);
    assert_eq!(h.state().docs[0].uid, uid);
    let name = format!("{} *", h.state().docs[0].name());
    h.get_by_label(&name).scroll_to_me();
    h.run_steps(12);
    h.get_by_label(&name).click();
    h.run_steps(4);
    assert!(!h.state().show_home);
    assert_eq!(h.state().docs[0].uid, uid);

    h.state_mut().close(0);
    h.run_steps(4);
    assert!(h.state().show_home);
    assert!(h.state().docs.is_empty());

    h.get_by_label("Examples for Model a process").click();
    h.run_steps(4);
    assert!(h.state().show_home);
    assert!(h.state().docs.is_empty());
    assert!(h.query_by_label(tr("petri.petri_mutual_exclusion")).is_none());
    h.get_by_label(tr("bpmn.bpmn_order_handling")).click();
    h.run_steps(4);
    let expected = crate::composition::registry()
        .open(&crate::composition::registry().example("bpmn.order").unwrap())
        .unwrap();
    assert_eq!(h.state().docs.len(), 1);
    assert_eq!(h.state().docs[0].snapshot(), expected.snapshot());
    h.state_mut().close(0);
    h.run_steps(4);

    for (label, kind) in [
        ("Simulate and analyze", "petri"),
        ("Discover a process", "log"),
        ("Design a class diagram", "class"),
        ("Map use cases", "usecase"),
        ("Define a decision table", "dmn.table"),
        ("Model an EPC", "epc"),
        ("Design an activity diagram", "uml.activity"),
        ("Design a database", "database.schema"),
        ("Sketch freely", "whiteboard"),
        ("Create with Mermaid", "mermaid"),
    ] {
        h.get_by_label(label).scroll_to_me();
        h.run_steps(12);
        h.get_by_label(label).click();
        h.run_steps(4);
        assert!(!h.state().show_home);
        assert_eq!(
            h.state().docs[0].editor.kind(),
            crate::composition::registry().find(kind).unwrap().id
        );
        h.state_mut().close(0);
        h.run_steps(4);
    }
    h.get_by_label("Evaluate costs and durations").scroll_to_me();
    h.run_steps(12);
    h.get_by_label("Evaluate costs and durations").click();
    h.run_steps(4);
    assert!(h.query_by_label("Process evaluation").is_some());

    h.get_by_label("Home").click();
    h.run_steps(4);
    h.ctx.set_theme(egui::Theme::Dark);
    shot(&mut h, "home_dark");
    h.get_by_label("Language").click();
    h.run_steps(4);
    assert!(h.query_by_label("Model a process").is_some());
    for language in ["English", "Français", "Español"] {
        assert!(h.query_by_label(language).is_some());
    }
    shot(&mut h, "language_dropdown");
    h.get_by_label("Español").click();
    h.run_steps(4);
    assert!(h.query_by_label("Modelar un proceso").is_some());
    assert_eq!(op_core::i18n::lang(), op_core::i18n::Lang::Es);
    assert!(h.query_by_label("Français").is_none());
    shot(&mut h, "home_spanish");
    h.get_by_label("Idioma").click();
    h.run_steps(4);
    h.get_by_label("Français").click();
    h.run_steps(4);
    assert!(h.query_by_label("Modéliser un processus").is_some());
    h.ctx.set_theme(egui::Theme::Light);
    shot(&mut h, "home_french");
    h.get_by_label("Langue").click();
    h.run_steps(4);
    h.get_by_label("English").click();
    h.run_steps(4);
    assert!(h.query_by_label("Model a process").is_some());
}

#[test]
fn mermaid_creator_opens_from_home_edits_templates_and_renders_at_responsive_widths() {
    for width in [1400.0, 420.0, 280.0] {
        let mut h = Harness::builder()
            .with_size([width, 860.0])
            .build_eframe(|cc| OpenProcessApp::new(cc, Vec::new(), crate::composition::registry()));
        h.ctx.set_theme(egui::Theme::Light);
        h.run_steps(4);
        h.get_by_label("Create with Mermaid").scroll_to_me();
        h.run_steps(12);
        h.get_by_label("Create with Mermaid").click();
        h.run_steps(5);
        assert_eq!(h.state().docs.len(), 1);
        assert_eq!(h.state().docs[0].editor.kind(), "mermaid");
        let initial = h.state().docs[0].snapshot();
        if width < 760.0 {
            if width < 360.0 {
                h.get_by_label("Options").click();
                h.run_steps(3);
            }
            h.get_by_label("Inspector").click();
            h.run_steps(4);
            h.key_press(Key::Escape);
            h.run_steps(3);
        }
        assert!(h.query_by_label("Mermaid code").is_some());
        h.get_by_label("Templates").click();
        h.run_steps(3);
        h.get_by_label("Sequence diagram").click();
        h.run_steps(4);
        let expected = op_core::mermaid::TEMPLATES[1].source.as_bytes();
        assert_eq!(h.state().docs[0].editor.export().unwrap().bytes, expected);
        h.state_mut().docs[0].editor.execute(UNDO, 0.0);
        h.run_steps(3);
        assert_eq!(h.state().docs[0].snapshot(), initial);
        h.state_mut().docs[0].editor.execute(REDO, 0.0);
        h.run_steps(3);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while h.state().docs[0].editor.export_as("svg").is_err() {
            assert!(std::time::Instant::now() < deadline, "Mermaid preview timed out");
            std::thread::sleep(std::time::Duration::from_millis(5));
            h.run_steps(3);
        }
        assert!(
            String::from_utf8(h.state().docs[0].editor.export_as("svg").unwrap().bytes)
                .unwrap()
                .contains("Submit request")
        );
        shot(&mut h, &format!("mermaid_creator_{width}"));
    }
}

#[test]
fn uml_mermaid_conversion_buttons_open_independent_editable_documents() {
    for width in [1400.0, 420.0] {
        let mut h = harness_sized(false, [width, 860.0]);
        h.state_mut().open_example("uml.classes");
        h.run_steps(5);
        let source_index = h.state().active;
        let source = h.state().docs[source_index].snapshot();
        let source_uid = h.state().docs[source_index].uid;
        let original = uml(&h).diagram.clone();
        let count = h.state().docs.len();
        h.get_by_label("Convert to Mermaid").click();
        h.run_steps(6);
        assert_eq!(h.state().docs.len(), count + 1);
        assert_eq!(h.state().docs[source_index].snapshot(), source);
        assert_eq!(h.state().docs[source_index].uid, source_uid);
        assert_eq!(h.state().docs[h.state().active].editor.kind(), "mermaid");
        let mermaid_index = h.state().active;
        let mermaid = h.state().docs[mermaid_index].snapshot();
        assert!(
            String::from_utf8(h.state().docs[mermaid_index].editor.export().unwrap().bytes)
                .unwrap()
                .contains("classDiagram")
        );
        h.get_by_label("Convert to UML classes").click();
        h.run_steps(6);
        assert_eq!(h.state().docs.len(), count + 2);
        assert_eq!(h.state().docs[mermaid_index].snapshot(), mermaid);
        assert_eq!(h.state().docs[source_index].snapshot(), source);
        let restored = uml(&h).diagram.clone();
        assert_eq!(restored.nodes.len(), original.nodes.len());
        for (a, b) in original.nodes.iter().zip(&restored.nodes) {
            assert_eq!(
                (&a.name, &a.attributes, &a.operations, a.kind),
                (&b.name, &b.attributes, &b.operations, b.kind)
            );
        }
        assert_eq!(restored.relations, original.relations);
        assert!(restored.validate().is_empty());
        let index = h.state().active;
        assert!(h.state().docs[index].dirty());
        h.state_mut().docs[index].editor.rename("Converted copy".into());
        h.state_mut().docs[index].editor.execute(UNDO, 0.0);
        assert_eq!(h.state().docs[index].name(), original.name);
        assert_eq!(h.state().docs[source_index].snapshot(), source);
        assert_eq!(h.state().docs[mermaid_index].snapshot(), mermaid);
        shot(&mut h, &format!("uml_mermaid_roundtrip_{width}"));
    }
}

#[test]
fn mermaid_conversion_commands_respect_source_kinds_and_fail_without_opening_partial_documents() {
    use op_core::document::DocumentCodec;
    let mut h = harness(false);
    h.state_mut().open_example("uml.use-cases");
    h.run_steps(4);
    assert!(!h.state().docs[h.state().active].editor.enabled("uml.convert-to-mermaid"));
    assert!(h.query_by_label("Convert to Mermaid").is_none());
    h.state_mut().open_example("mermaid.flowchart");
    h.run_steps(4);
    assert!(!h.state().docs[h.state().active].editor.enabled("mermaid.convert-to-uml"));
    let source = op_core::mermaid::Diagram {
        name: "Draft".into(),
        source: "classDiagram\nclass A {".into(),
    };
    h.state_mut()
        .open_document(op_core::document::JsonCodec::new("mermaid").encode(&source), None, false);
    h.run_steps(4);
    let count = h.state().docs.len();
    let original = h.state().docs[h.state().active].snapshot();
    h.get_by_label("Convert to UML classes").click();
    h.run_steps(4);
    assert_eq!(h.state().docs.len(), count);
    assert_eq!(h.state().docs[h.state().active].snapshot(), original);
    assert!(h.state().message.as_ref().unwrap().0.contains("Close all"));
    h.state_mut().open_example("mermaid.class");
    h.run_steps(4);
    h.get_by_label("Tools").click();
    h.run_steps(3);
    h.get_all_by_role_and_label(egui::accesskit::Role::Button, "Convert to UML classes")
        .last()
        .unwrap()
        .click();
    h.run_steps(5);
    assert_eq!(h.state().docs[h.state().active].editor.kind(), "uml.class");
}

#[test]
fn home_responsive_grid() {
    for (width, columns) in [(1400.0, 3), (720.0, 2), (420.0, 1)] {
        let mut h = Harness::builder()
            .with_size([width, 1800.0])
            .build_eframe(|cc| OpenProcessApp::new(cc, Vec::new(), crate::composition::registry()));
        h.run_steps(4);
        let a = h.get_by_label("Sketch freely").rect();
        let b = h.get_by_label("Design a database").rect();
        let c = h.get_by_label("Design a class diagram").rect();
        assert!(a.left() >= 0.0 && a.right() <= width);
        assert!(b.right() <= width && c.right() <= width);
        if columns == 1 {
            assert!(b.top() > a.bottom());
        } else {
            assert_eq!(a.top(), b.top());
            assert!(b.left() > a.right());
            assert_eq!(a.top() == c.top(), columns == 3);
        }
        if width == 720.0 {
            op_core::i18n::set_lang(op_core::i18n::Lang::Fr);
            h.run_steps(4);
            shot(&mut h, "home_narrow");
            op_core::i18n::set_lang(op_core::i18n::Lang::En);
        }
    }
}

#[test]
fn home_document_cards_capture_pngs_reflow_and_resume_the_same_document() {
    let mut h = harness(false);
    for example in ["petri.mutex", "dmn.discount", "database.shop", "whiteboard.ideas", "log.first"] {
        h.state_mut().open_example(example);
        let before = h.state().docs.last().unwrap().snapshot();
        h.run_steps(8);
        assert_eq!(
            h.state().docs.last().unwrap().snapshot(),
            before,
            "capturing must not edit the document"
        );
    }
    let snapshots: Vec<_> = h.state().docs.iter().map(|d| d.snapshot()).collect();
    let names: Vec<_> = h.state().docs.iter().map(|d| format!("{} *", d.name())).collect();
    let uids: Vec<_> = h.state().docs.iter().map(|d| d.uid).collect();
    let mut images = std::collections::HashSet::new();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
    std::fs::create_dir_all(&dir).unwrap();
    for doc in &h.state().docs {
        let png = h
            .state()
            .thumbnails
            .png(doc.uid)
            .expect("each displayed document has a generated PNG");
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(image::load_from_memory(png).unwrap().into_rgba8().dimensions(), (320, 180));
        assert!(images.insert(png.to_vec()), "previews show the content of each document");
        std::fs::write(dir.join(format!("home_thumbnail_{}.png", doc.editor.kind().replace('.', "_"))), png).unwrap();
    }
    h.get_by_label("Home").click();
    h.run_steps(5);
    for width in [1400.0, 720.0, 280.0] {
        h.set_size(egui::vec2(width, 1000.0));
        h.ctx.set_theme(if width == 720.0 { egui::Theme::Dark } else { egui::Theme::Light });
        h.run_steps(6);
        let first = h.get_by_label(&names[0]).rect();
        let second = h.get_by_label(&names[1]).rect();
        assert!(first.left() >= 0.0 && first.right() <= width);
        if width >= 720.0 {
            assert!((first.top() - second.top()).abs() < 1.0);
        } else {
            assert!(second.top() > first.bottom());
        }
        shot(&mut h, &format!("home_resume_cards_{width}"));
    }
    assert_eq!(h.state().docs.iter().map(|d| d.snapshot()).collect::<Vec<_>>(), snapshots);
    h.get_by_label(&names[0]).click();
    h.run_steps(5);
    assert!(!h.state().show_home);
    assert_eq!(h.state().docs[h.state().active].uid, uids[0]);
    assert_eq!(h.state().docs.len(), uids.len(), "resuming must not duplicate a document");
    let old = h.state().thumbnails.png(uids[0]).unwrap().to_vec();
    h.state_mut().docs[0]
        .editor
        .as_any_mut()
        .downcast_mut::<BpmnEditor>()
        .unwrap()
        .diagram
        .nodes[0]
        .name = "Changed thumbnail content".into();
    h.run_steps(8);
    assert_ne!(h.state().thumbnails.png(uids[0]).unwrap(), old, "edits refresh the PNG");
}

fn shot(h: &mut Harness<'static, OpenProcessApp>, name: &str) {
    h.run_steps(if name.contains("animation") { 0 } else { 4 });
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
    std::fs::create_dir_all(&dir).unwrap();
    match h.render() {
        Ok(img) => img.save(dir.join(format!("{name}.png"))).unwrap(),
        Err(e) => eprintln!("rendering unavailable: {e}"),
    }
}

#[test]
fn bpmn_evaluation_ui() {
    let mut h = harness(false);
    h.state_mut().open_model(Model::Bpmn(examples::evaluation_process()), None, false);
    h.run_steps(4);
    h.get_by_label("Evaluation").click();
    h.run_steps(4);
    assert!(h.query_by_label("Process evaluation").is_some());
    assert!(h.query_by_label("Cost CAi").is_some());
    assert!(h.query_by_label("Duration TAi").is_some());
    assert!(
        h.get_by_label("A2").rect().top() > h.get_by_label("A1").rect().bottom(),
        "one activity per table row"
    );
    assert!(
        h.get_by_label("Process evaluation").rect().left() > 900.0,
        "the inspector leaves room for the diagram"
    );
    shot(&mut h, "bpmn_evaluation");
}

#[test]
fn screenshots() {
    for (dark, theme) in [(false, "light"), (true, "dark")] {
        let mut h = harness(dark);
        shot(&mut h, &format!("bpmn_{theme}"));
        for (label, name) in [("Home", "home"), ("File", "file"), ("Style ▾", "style"), ("Inspector", "inspector")] {
            h.hover_at(h.get_by_label(label).rect().center());
            shot(&mut h, &format!("toolbar_hover_{name}_{theme}"));
        }
    }

    let mut h = harness(false);
    h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
    if let Some(e) = h
        .state_mut()
        .docs
        .last_mut()
        .and_then(|d| d.editor.as_any_mut().downcast_mut::<PetriEditor>())
    {
        e.show_matrices();
    }
    shot(&mut h, "petri_matrices");

    if let Some(e) = h
        .state_mut()
        .docs
        .last_mut()
        .and_then(|d| d.editor.as_any_mut().downcast_mut::<PetriEditor>())
    {
        e.show_analysis();
    }
    shot(&mut h, "petri_analysis");

    h.state_mut().open_model(Model::Petri(examples::dining_philosophers()), None, false);
    if let Some(e) = h
        .state_mut()
        .docs
        .last_mut()
        .and_then(|d| d.editor.as_any_mut().downcast_mut::<PetriEditor>())
    {
        e.toggle_simulation(0.0);
        e.show_reachability_graph();
    }
    shot(&mut h, "petri_simulation_graph");

    h.state_mut().set_active(0);
    h.state_mut().execute("bpmn.convert-to-petri", 0.0);
    if let Some(e) = h
        .state_mut()
        .docs
        .last_mut()
        .and_then(|d| d.editor.as_any_mut().downcast_mut::<PetriEditor>())
    {
        e.show_analysis();
    }
    shot(&mut h, "bpmn_converted");
}

/// Every non-ASCII character used in the UI must be drawable with the
/// embedded fonts (there are no system fonts on the web).
#[test]
fn glyph_coverage() {
    let ctx = egui::Context::default();
    crate::theme::setup_fonts(&ctx);
    let mut out = ctx.run_ui(Default::default(), |_| {});
    let mut chars: Vec<char> = Vec::new();
    for (file, src) in crate::test_sources::all() {
        // Parser input aliases are not necessarily displayed. Keep the original
        // core formatting sources, plus every UI module including extracted files.
        if file.contains("op-core/src/")
            && ![
                "i18n/en.rs",
                "i18n/fr.rs",
                "i18n/es.rs",
                "mining.rs",
                "analysis.rs",
                "convert.rs",
                "bpmn.rs",
            ]
            .iter()
            .any(|name| file.ends_with(name))
        {
            continue;
        }
        let src = src.split("\n#[cfg(test)]\nmod tests").next().unwrap_or(&src);
        for line in src.lines().filter(|l| !l.trim_start().starts_with("//")) {
            chars.extend(line.chars().filter(|c| !c.is_ascii()));
        }
    }
    chars.sort();
    chars.dedup();
    let missing: String = ctx.fonts_mut(|f| {
        chars
            .into_iter()
            .filter(|&c| !f.has_glyph(&egui::FontId::proportional(14.0), c))
            .collect()
    });
    let mut out2 = ctx.run_ui(Default::default(), |_| {});
    out.textures_delta.clear();
    out2.textures_delta.clear();
    assert!(missing.is_empty(), "missing glyphs: {missing}");
}

fn press(h: &mut Harness<'static, OpenProcessApp>, pos: egui::Pos2, down: bool) {
    h.event(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed: down,
        modifiers: Modifiers::NONE,
    });
    h.run_steps(2);
}

fn click(h: &mut Harness<'static, OpenProcessApp>, pos: egui::Pos2) {
    h.hover_at(pos);
    h.run_steps(2);
    press(h, pos, true);
    press(h, pos, false);
    // Let the double-click window pass so consecutive clicks stay single clicks.
    h.run_steps(30);
}

fn drag(h: &mut Harness<'static, OpenProcessApp>, from: egui::Pos2, to: egui::Pos2) {
    h.hover_at(from);
    h.run_steps(2);
    press(h, from, true);
    for k in 1..=10 {
        h.hover_at(from.lerp(to, k as f32 / 10.0));
        h.run_steps(1);
    }
    press(h, to, false);
}

fn key(h: &mut Harness<'static, OpenProcessApp>, k: Key) {
    h.key_press(k);
    h.run_steps(2);
}

fn uml<'a>(h: &'a Harness<'static, OpenProcessApp>) -> &'a UmlEditor {
    h.state().docs[h.state().active]
        .editor
        .as_any()
        .downcast_ref::<UmlEditor>()
        .unwrap()
}

#[test]
fn uml_editors_by_mouse() {
    use op_core::uml::{NodeKind, RelationKind};
    for (label, kind, first_tool, second_tool, first_kind, second_kind) in [
        (
            "New class diagram",
            DiagramKind::Class,
            "Class",
            "Interface",
            NodeKind::Class,
            NodeKind::Interface,
        ),
        (
            "New use-case diagram",
            DiagramKind::UseCase,
            "Actor",
            "Use case",
            NodeKind::Actor,
            NodeKind::UseCase,
        ),
    ] {
        let mut h = harness(false);
        h.get_by_label("File").click();
        h.run_steps(2);
        h.get_by_label(&format!(
            "New… {}",
            h.ctx.format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::N))
        ))
        .click();
        h.run_steps(4);
        h.get_by_label(label).scroll_to_me();
        h.run_steps(12);
        h.get_by_label(label).click();
        h.run_steps(4);
        assert_eq!(uml(&h).diagram.kind, kind);
        h.get_by_label(first_tool).click();
        h.run_steps(2);
        click(&mut h, egui::pos2(250.0, 300.0));
        assert_eq!(uml(&h).diagram.nodes[0].kind, first_kind);
        h.get_by_label(second_tool).click();
        h.run_steps(2);
        click(&mut h, egui::pos2(650.0, 300.0));
        assert_eq!(uml(&h).diagram.nodes[1].kind, second_kind);
        let a = uml(&h).node_screen_center(uml(&h).diagram.nodes[0].id);
        let b = uml(&h).node_screen_center(uml(&h).diagram.nodes[1].id);
        h.get_by_label("Connect").click();
        h.run_steps(2);
        click(&mut h, a);
        click(&mut h, b);
        assert_eq!(uml(&h).diagram.relations.len(), 1);
        assert_eq!(uml(&h).diagram.relations[0].kind, RelationKind::Association);
        h.get_by_label("Select / move").click();
        h.run_steps(2);
        let old = uml(&h).diagram.nodes[0].bounds;
        drag(&mut h, a, a + egui::vec2(0.0, 100.0));
        assert!(uml(&h).diagram.nodes[0].bounds.y > old.y + 70.0);
        h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
        h.run_steps(3);
        assert_eq!(uml(&h).diagram.nodes[0].bounds, old);
        click(&mut h, a);
        key(&mut h, Key::Delete);
        assert_eq!(uml(&h).diagram.nodes.len(), 1);
        assert!(uml(&h).diagram.relations.is_empty());
        h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
        h.run_steps(3);
        assert_eq!(uml(&h).diagram.nodes.len(), 2);
        assert_eq!(uml(&h).diagram.relations.len(), 1);
        if kind == DiagramKind::UseCase {
            h.get_by_label("Use case").click();
            h.run_steps(2);
            click(&mut h, egui::pos2(650.0, 550.0));
            h.get_by_role(egui::accesskit::Role::ComboBox).click();
            h.run_steps(2);
            h.get_by_label("Include").click();
            h.run_steps(2);
            click(&mut h, b);
            let c = uml(&h).node_screen_center(uml(&h).diagram.nodes[2].id);
            click(&mut h, c);
            let d = &uml(&h).diagram;
            assert_eq!(d.relations.len(), 2);
            assert_eq!(d.relations[1].kind, RelationKind::Include);
            assert_eq!((d.relations[1].from, d.relations[1].to), (d.nodes[1].id, d.nodes[2].id));
        }
        let model = h.state().docs[h.state().active].model();
        assert_eq!(document::load_any(document::to_json(&model).as_bytes()).unwrap(), model);
        shot(&mut h, if kind == DiagramKind::Class { "uml_class" } else { "uml_use_case" });
    }
}

#[test]
fn uml_group_gestures_clipboard_and_inline_rename() {
    use op_core::uml::{NodeKind, RelationKind};
    let mut diagram = UmlDiagram::new("Gestures", DiagramKind::Class);
    let a = diagram
        .add_node(NodeKind::Class, op_core::geom::Pos::new(100.0, 100.0), "A")
        .unwrap();
    let b = diagram
        .add_node(NodeKind::Class, op_core::geom::Pos::new(480.0, 100.0), "B")
        .unwrap();
    let mut h = harness(false);
    h.state_mut().open_model(Model::Uml(diagram), None, false);
    h.run_steps(5);
    let ac = uml(&h).node_screen_center(a);
    let bc = uml(&h).node_screen_center(b);
    h.get_by_label("Connect").click();
    h.run_steps(2);
    drag(&mut h, ac, bc);
    assert_eq!(uml(&h).diagram.relations.len(), 1, "connect with a drag");
    assert_eq!(uml(&h).diagram.relations[0].kind, RelationKind::Association);
    h.get_by_label("Select / move").click();
    h.run_steps(2);
    let union = uml(&h).node_screen_rect(a).union(uml(&h).node_screen_rect(b));
    drag(&mut h, union.min - egui::vec2(15.0, 15.0), union.max + egui::vec2(15.0, 15.0));
    assert_eq!(uml(&h).selected_nodes().len(), 2, "rubber-band selection");
    let before = uml(&h).diagram.nodes.clone();
    drag(&mut h, ac, ac + egui::vec2(70.0, 80.0));
    let after = &uml(&h).diagram.nodes;
    let delta = op_core::geom::Pos::new(after[0].bounds.x - before[0].bounds.x, after[0].bounds.y - before[0].bounds.y);
    assert!(delta.x > 20.0 && delta.y > 20.0);
    assert_eq!(after[1].bounds.x - before[1].bounds.x, delta.x);
    assert_eq!(after[1].bounds.y - before[1].bounds.y, delta.y);
    h.key_press_modifiers(Modifiers::COMMAND, Key::D);
    h.run_steps(3);
    assert_eq!(uml(&h).diagram.nodes.len(), 4);
    assert_eq!(uml(&h).diagram.relations.len(), 2);
    let copied = &uml(&h).diagram.relations[1];
    assert!(copied.from != a && copied.from != b && copied.to != a && copied.to != b);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(uml(&h).diagram.nodes.len(), 2, "duplicate is one undo step");
    assert_eq!(uml(&h).diagram.relations.len(), 1);
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::C);
    h.run_steps(2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::V);
    h.run_steps(3);
    assert_eq!(uml(&h).diagram.nodes.len(), 4, "clipboard preserves the group");
    assert_eq!(uml(&h).diagram.relations.len(), 2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    let center = uml(&h).node_screen_center(a);
    click(&mut h, center);
    key(&mut h, Key::F2);
    assert!(h.query_by_role_and_label(egui::accesskit::Role::TextInput, "Name").is_some());
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.event(egui::Event::Text("Cancelled name".into()));
    h.run_steps(2);
    key(&mut h, Key::Escape);
    assert_eq!(uml(&h).diagram.node(a).unwrap().name, "A");
    h.hover_at(center);
    h.run_steps(2);
    key(&mut h, Key::F2);
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.event(egui::Event::Text("Customer".into()));
    h.run_steps(2);
    key(&mut h, Key::Enter);
    assert_eq!(uml(&h).diagram.node(a).unwrap().name, "Customer");
    h.hover_at(center);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(uml(&h).diagram.node(a).unwrap().name, "A", "renaming is one undo step");
}

#[test]
fn uml_examples_themes_and_narrow_panes() {
    for dark in [false, true] {
        for (name, diagram) in [("classes", examples::uml_classes()), ("use_cases", examples::uml_use_cases())] {
            let mut h = harness(dark);
            let model = Model::Uml(diagram);
            h.state_mut().open_model(model.clone(), None, true);
            h.run_steps(4);
            let doc = &h.state().docs[h.state().active];
            assert!(!doc.dirty());
            assert_eq!(doc.model(), model);
            let text = document::to_json(&model);
            assert_eq!(document::load_any(text.as_bytes()).unwrap(), model);
            shot(&mut h, &format!("uml_{name}_{}", if dark { "dark" } else { "light" }));
        }
    }
    let mut h = harness_sized(false, [720.0, 480.0]);
    h.state_mut().open_model(Model::Uml(examples::uml_use_cases()), None, false);
    h.run_steps(4);
    assert!(h.query_by_label("Connect").is_some());
    shot(&mut h, "uml_narrow");
}

fn petri<'a>(h: &'a Harness<'static, OpenProcessApp>) -> &'a PetriEditor {
    h.state().docs[h.state().active]
        .editor
        .as_any()
        .downcast_ref::<PetriEditor>()
        .unwrap()
}

/// Drives the real UI: tools, clicks, arc drag, tokens, undo, token game.
#[test]
fn petri_editing_by_mouse() {
    let mut h = harness(false);
    h.state_mut().open_model(Model::Petri(PetriNet::new("t")), None, false);
    h.run_steps(4);
    let (a, b) = (egui::pos2(350.0, 400.0), egui::pos2(600.0, 400.0));

    h.hover_at(a);
    h.run_steps(2);
    key(&mut h, Key::P);
    click(&mut h, a);
    assert_eq!(petri(&h).net.places.len(), 1);

    key(&mut h, Key::T);
    click(&mut h, b);
    assert_eq!(petri(&h).net.transitions.len(), 1);

    key(&mut h, Key::A);
    drag(&mut h, a, b);
    assert_eq!(petri(&h).net.arcs.len(), 1, "arc created by dragging");

    key(&mut h, Key::K);
    click(&mut h, a);
    click(&mut h, a);
    assert_eq!(petri(&h).net.places[0].tokens, 2);

    // Undo the last token
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(petri(&h).net.places[0].tokens, 1);

    // Token game: fire the transition
    key(&mut h, Key::Escape);
    key(&mut h, Key::S);
    assert!(petri(&h).sim.is_some());
    click(&mut h, b);
    assert_eq!(petri(&h).sim.as_ref().unwrap().marking, vec![0]);
    assert_eq!(petri(&h).net.places[0].tokens, 1, "simulation does not change M0");

    // Move the place with the select tool
    key(&mut h, Key::S);
    key(&mut h, Key::V);
    let before = petri(&h).net.places[0].pos;
    drag(&mut h, a, a + egui::vec2(0.0, 100.0));
    let after = petri(&h).net.places[0].pos;
    assert!(after.y > before.y + 30.0, "place moved: {before:?} -> {after:?}");
}

fn bpmn<'a>(h: &'a Harness<'static, OpenProcessApp>) -> &'a BpmnEditor {
    h.state().docs[h.state().active]
        .editor
        .as_any()
        .downcast_ref::<BpmnEditor>()
        .unwrap()
}

#[test]
fn bpmn_editing_by_mouse() {
    let mut h = harness(false);
    h.state_mut().open_model(Model::Bpmn(BpmnDiagram::new("t")), None, false);
    h.run_steps(4);
    let (a, b, c) = (egui::pos2(250.0, 400.0), egui::pos2(450.0, 400.0), egui::pos2(700.0, 400.0));
    h.hover_at(a);
    h.run_steps(2);
    key(&mut h, Key::S);
    click(&mut h, a);
    key(&mut h, Key::T);
    click(&mut h, b);
    key(&mut h, Key::E);
    click(&mut h, c);
    assert_eq!(bpmn(&h).diagram.nodes.len(), 3);
    key(&mut h, Key::C);
    drag(&mut h, a, b);
    key(&mut h, Key::C);
    drag(&mut h, b, c);
    assert_eq!(bpmn(&h).diagram.flows.len(), 2);
    assert!(
        bpmn(&h)
            .diagram
            .validate()
            .iter()
            .all(|i| i.severity != op_core::bpmn::Severity::Error)
    );

    // Drop a gateway on the second flow: it gets inserted
    key(&mut h, Key::X);
    click(&mut h, egui::pos2(580.0, 400.0));
    assert_eq!(bpmn(&h).diagram.nodes.len(), 4);
    assert_eq!(bpmn(&h).diagram.flows.len(), 3);
    shot(&mut h, "bpmn_edited");

    // The icon + label rows remain clickable and preserve a single undo step.
    use op_core::bpmn::{GatewayType, NodeKind, TaskType};
    for dark in [false, true] {
        h.ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
        h.run_steps(4);
        for (point, menu, options, chosen, label) in [
            (
                b,
                "Task type",
                TaskType::ALL.map(|t| t.label()).to_vec(),
                NodeKind::Task(TaskType::User),
                "User task",
            ),
            (
                egui::pos2(580.0, 400.0),
                "Gateway type",
                GatewayType::ALL.map(|g| g.label()).to_vec(),
                NodeKind::Gateway(GatewayType::Parallel),
                "Parallel (AND)",
            ),
        ] {
            let before = h.state().docs.last().unwrap().snapshot();
            h.hover_at(point);
            h.run_steps(2);
            for pressed in [true, false] {
                h.event(egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: Modifiers::NONE,
                });
            }
            h.run_steps(4);
            h.get_by_role_and_label(egui::accesskit::Role::Button, menu).click();
            h.run_steps(4);
            for option in options {
                assert!(
                    h.query_by_role_and_label(egui::accesskit::Role::Button, option).is_some(),
                    "{menu}: {option}"
                );
            }
            shot(
                &mut h,
                &format!(
                    "bpmn_{}_menu_{}",
                    if menu == "Task type" { "task" } else { "gateway" },
                    if dark { "dark" } else { "light" }
                ),
            );
            h.get_by_role_and_label(egui::accesskit::Role::Button, label).click();
            h.run_steps(5);
            assert!(bpmn(&h).diagram.nodes.iter().any(|n| n.kind == chosen));
            let after = h.state().docs.last().unwrap().snapshot();
            assert_ne!(before, after);
            h.state_mut().docs.last_mut().unwrap().editor.execute(UNDO, 0.0);
            h.run_steps(3);
            assert_eq!(h.state().docs.last().unwrap().snapshot(), before);
            h.state_mut().docs.last_mut().unwrap().editor.execute(REDO, 0.0);
            h.run_steps(3);
            assert_eq!(h.state().docs.last().unwrap().snapshot(), after);
            h.state_mut().docs.last_mut().unwrap().editor.execute(UNDO, 0.0);
            h.run_steps(3);
        }
    }
}

#[test]
fn dark_token_game_animation() {
    let mut h = harness(true);
    h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
    h.run_steps(4);
    if let Some(e) = h
        .state_mut()
        .docs
        .last_mut()
        .unwrap()
        .editor
        .as_any_mut()
        .downcast_mut::<PetriEditor>()
    {
        e.toggle_simulation(0.0);
    }
    h.run_steps(4);
    if let Some(e) = h
        .state_mut()
        .docs
        .last_mut()
        .unwrap()
        .editor
        .as_any_mut()
        .downcast_mut::<PetriEditor>()
    {
        assert!(e.fire_by_name("enter1"));
    }
    h.run_steps(1);
    shot(&mut h, "petri_dark_animation");
    h.run_steps(60);
    shot(&mut h, "petri_dark_after");
    if let Some(e) = h.state().docs.last().unwrap().editor.as_any().downcast_ref::<PetriEditor>() {
        assert_eq!(e.sim.as_ref().unwrap().marking, vec![0, 0, 1, 1, 0]);
    }
}

/// Process discovery tab: course example 2, then open the net and the BPMN.
fn wait_for_discovery(h: &mut Harness<'static, OpenProcessApp>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        h.run_steps(2);
        if h.state().docs[h.state().active].editor.export().is_ok() {
            break;
        }
        assert!(std::time::Instant::now() < deadline, "discovery timed out");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
fn process_discovery() {
    let mut h = harness(false);
    h.state_mut().open_model(
        Model::Log(LogDocument {
            name: "Example 2".into(),
            text: crate::mining_editor::EXAMPLE_2.into(),
            ..Default::default()
        }),
        None,
        false,
    );
    h.run_steps(4);
    wait_for_discovery(&mut h);
    shot(&mut h, "mining_top");
    let n = h.state().docs.len();
    h.get_by_label("Open Petri net").click();
    h.run_steps(4);
    assert_eq!(h.state().docs.len(), n + 1);
    assert!(h.state().docs.last().unwrap().editor.as_any().is::<PetriEditor>());
    shot(&mut h, "mining_petri");
    h.state_mut().set_active(n - 1);
    h.run_steps(4);
    h.get_by_label("Deduce BPMN").click();
    h.run_steps(4);
    assert!(h.state().docs.last().unwrap().editor.as_any().is::<BpmnEditor>());
    if let Some(e) = h
        .state_mut()
        .docs
        .last_mut()
        .unwrap()
        .editor
        .as_any_mut()
        .downcast_mut::<BpmnEditor>()
    {
        e.request_fit();
    }
    shot(&mut h, "mining_bpmn");
}

#[test]
fn rubber_band_and_group_move() {
    let mut h = harness(false);
    h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
    h.run_steps(4);
    // Select everything with a rubber band drawn on empty space
    drag(&mut h, egui::pos2(1000.0, 800.0), egui::pos2(90.0, 95.0));
    let e = petri(&h);
    let (n, before, place) = (
        e.selected_count(),
        e.net.places.iter().map(|p| p.pos).collect::<Vec<_>>(),
        e.screen_pos(e.net.places[0].pos),
    );
    assert_eq!(n, 9, "all places and transitions selected");
    // Dragging one node moves the whole selection
    drag(&mut h, place, place + egui::vec2(60.0, 40.0));
    let after: Vec<op_core::geom::Pos> = petri(&h).net.places.iter().map(|p| p.pos).collect();
    let d0 = (after[0].x - before[0].x, after[0].y - before[0].y);
    assert!(d0.0 > 20.0 && d0.1 > 10.0, "moved: {d0:?}");
    for (a, b) in after.iter().zip(&before) {
        assert!(
            (a.x - b.x - d0.0).abs() < 11.0 && (a.y - b.y - d0.1).abs() < 11.0,
            "group moved together"
        );
    }
}

#[test]
fn palette_drag_and_drop() {
    let mut h = harness(false);
    h.state_mut().open_model(Model::Bpmn(BpmnDiagram::new("dnd")), None, false);
    h.run_steps(4);
    let task = h.get_by_label("Task (T)").rect().center();
    drag(&mut h, task, egui::pos2(600.0, 400.0));
    let start = h.get_by_label("Start event (S)").rect().center();
    drag(&mut h, start, egui::pos2(400.0, 400.0));
    h.run_steps(2);
    let d = &bpmn(&h).diagram;
    assert_eq!(d.nodes.len(), 2, "two elements dropped");
    assert!(d.nodes.iter().any(|n| matches!(n.kind, op_core::bpmn::NodeKind::Task(_))));
    shot(&mut h, "bpmn_dropped");
}

#[test]
fn link_styles_and_sketch() {
    use op_core::style::{DiagramStyle, EdgeStyle};
    let mut h = harness(false);
    let mut net = examples::mutual_exclusion();
    net.style = DiagramStyle {
        edges: EdgeStyle::Orthogonal,
        sketch: false,
        ..Default::default()
    };
    h.state_mut().open_model(Model::Petri(net.clone()), None, false);
    shot(&mut h, "style_petri_orthogonal");
    net.style = DiagramStyle {
        edges: EdgeStyle::Curved,
        sketch: true,
        ..Default::default()
    };
    h.state_mut().open_model(Model::Petri(net), None, false);
    shot(&mut h, "style_petri_sketch_curved");
    let mut bar = examples::mutual_exclusion();
    bar.style.transitions = op_core::style::TransitionShape::Bar;
    h.state_mut().open_model(Model::Petri(bar), None, false);
    shot(&mut h, "style_petri_bars");
    let mut d = examples::order_process();
    d.style = DiagramStyle {
        edges: EdgeStyle::Auto,
        sketch: true,
        ..Default::default()
    };
    h.state_mut().open_model(Model::Bpmn(d), None, false);
    shot(&mut h, "style_bpmn_sketch");
}

#[test]
fn manual_alpha_mode() {
    let mut h = harness_sized(false, [1400.0, 2600.0]);
    let mut answers = std::collections::BTreeMap::new();
    answers.insert("step:TI".to_string(), "{A}".to_string());
    answers.insert("step:TO".to_string(), "{D,E}".to_string()); // wrong on purpose
    answers.insert("step:YW".to_string(), "(A,BE), (A,CE), (BE,D), (CE,D)".to_string());
    answers.insert("m:A:B".to_string(), "→".to_string());
    answers.insert("m:B:C".to_string(), "#".to_string()); // wrong
    h.state_mut().open_model(
        Model::Log(LogDocument {
            name: "Exercise".into(),
            text: "ABCD\nACBD\nAED".into(),
            manual: true,
            answers,
        }),
        None,
        false,
    );
    h.run_steps(4);
    wait_for_discovery(&mut h);
    h.get_by_label("Check everything").click();
    h.run_steps(4);
    shot(&mut h, "mining_manual");
    let Some(e) = h
        .state()
        .docs
        .last()
        .unwrap()
        .editor
        .as_any()
        .downcast_ref::<crate::mining_editor::MiningEditor>()
    else {
        unreachable!()
    };
    assert!(e.manual);
    // the answers are saved with the document
    let Model::Log(l) = h.state().docs.last().unwrap().model() else {
        unreachable!()
    };
    assert_eq!(l.answers.get("step:TI").map(String::as_str), Some("{A}"));
}

#[test]
fn png_encoding() {
    let img = egui::ColorImage::new([20, 10], vec![egui::Color32::RED; 200]);
    let png = io::encode_png(&img, egui::Rect::from_min_size(egui::pos2(2.0, 2.0), egui::vec2(8.0, 4.0)), 1.0).unwrap();
    assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
}

/// BPMN on the left, Petri net matrices on the right.
#[test]
fn split_view() {
    let mut h = harness(false);
    h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
    let uid = h.state().docs.last().unwrap().uid;
    h.state_mut().set_active(0);
    h.state_mut().open_beside(
        Pane {
            doc: uid,
            view: View::Matrices,
        },
        false,
    );
    h.run_steps(6);
    let root = h.state().tree.root.unwrap();
    assert!(matches!(
        h.state().tree.tiles.get(root),
        Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(_)))
    ));
    shot(&mut h, "split_bpmn_matrices");
    // Closing the Petri editor removes its matrices pane too
    let i = h.state().docs.len() - 1;
    h.state_mut().close(i);
    h.run_steps(2);
    assert!(dock::panes_of(&h.state().tree, uid).is_empty());
}

#[test]
fn persistence_roundtrip() {
    let p = Persisted {
        docs: vec![
            (Model::Bpmn(examples::order_process()), None),
            (Model::Petri(examples::mutual_exclusion()), Some("x.opx".into())),
        ],
        active: 1,
    };
    let s = serde_json::to_string(&p).unwrap();
    let back: Persisted = serde_json::from_str(&s).unwrap();
    assert_eq!(back.docs.len(), 2);
    assert_eq!(back.docs[0].0, p.docs[0].0);
}

#[test]
fn independent_views_share_document_history_and_keep_their_cameras() {
    let mut h = harness(false);
    h.state_mut()
        .open_model(Model::Uml(examples::uml_classes()), Some("classes.opx".into()), true);
    let index = h.state().active;
    let uid = h.state().docs[index].uid;
    h.state_mut().open_beside(
        Pane {
            doc: uid,
            view: View::Editor,
        },
        false,
    );
    h.run_steps(6);
    let panes = dock::panes_of(&h.state().tree, uid);
    assert_eq!(panes.len(), 2);
    let cameras = [
        crate::canvas::View {
            offset: egui::vec2(20.0, 40.0),
            zoom: 0.7,
        },
        crate::canvas::View {
            offset: egui::vec2(100.0, 60.0),
            zoom: 1.3,
        },
    ];
    let state: Vec<_> = panes
        .iter()
        .zip(cameras)
        .map(|(id, camera)| (id.0, serde_json::json!({"camera":camera,"inspector":false})))
        .collect();
    h.state_mut().docs[index]
        .editor
        .restore_workspace(&serde_json::to_value(&state).unwrap());
    h.run_steps(4);
    let workspace = h.state().docs[index].editor.workspace();
    let views: Vec<(u64, serde_json::Value)> = serde_json::from_value(workspace).unwrap();
    for (id, camera) in &state {
        assert_eq!(views.iter().find(|(n, _)| n == id).unwrap().1["camera"], camera["camera"]);
    }
    assert!(!h.state().docs[index].dirty(), "camera changes do not modify the document");
    let original = h.state().docs[index].snapshot();
    h.state_mut().docs[index].editor.rename("Edited name".into());
    assert!(h.state().docs[index].dirty());
    h.state_mut().execute(UNDO, 0.0);
    assert_eq!(h.state().docs[index].snapshot(), original);
    assert!(!h.state().docs[index].dirty(), "undo to saved state clears the dirty flag");
    h.state_mut().handle_dock_event(DockEvent::ClosePane(uid, panes[0]));
    assert_eq!(dock::panes_of(&h.state().tree, uid).len(), 1);
    assert!(h.state().docs.iter().any(|d| d.uid == uid));
}

#[test]
fn empty_documents_close_from_tabs_and_file_menu_without_confirmation() {
    use op_core::document::{DocumentCodec, JsonCodec};

    let registry = crate::composition::registry();
    let mut h = Harness::builder()
        .with_size([1400.0, 860.0])
        .build_eframe(|cc| OpenProcessApp::new(cc, Vec::new(), registry));
    for descriptor in &registry.diagrams {
        let document = if descriptor.id == "mermaid" {
            JsonCodec::new("mermaid").encode(&op_core::mermaid::Diagram {
                name: "Empty source".into(),
                source: " \n\t ".into(),
            })
        } else {
            (descriptor.create)(1)
        };
        for file_menu in [false, true] {
            h.state_mut().open_document(document.clone(), None, false);
            h.state_mut().docs[0].editor.rename("Renamed empty document".into());
            h.run_steps(3);
            assert!(h.state().docs[0].dirty(), "{} is still unsaved", descriptor.id);
            if file_menu {
                h.get_by_label("File").click();
                h.run_steps(2);
                h.get_by_label("Close tab").click();
            } else {
                let uid = h.state().docs[0].uid;
                let pane = dock::panes_of(&h.state().tree, uid)[0];
                h.state_mut().handle_dock_event(DockEvent::ClosePane(uid, pane));
            }
            h.run_steps(3);
            assert!(h.state().docs.is_empty(), "{} closes directly", descriptor.id);
            assert!(h.state().confirm_close.is_none());
            assert!(h.state().show_home);
        }
    }
}

#[test]
fn documents_with_content_and_recovery_data_keep_close_confirmation() {
    use op_core::document::{DocumentCodec, JsonCodec};

    let registry = crate::composition::registry();
    let mut documents: Vec<_> = registry
        .examples
        .iter()
        .map(|example| StoredDocument::Editable((example.create)()))
        .collect();
    let mut configured_table = op_core::dmn::DecisionTable::new("Configured clauses");
    configured_table.inputs[0].name = "age".into();
    let mut pool_only = BpmnDiagram::new("Pool only");
    pool_only.add_pool(op_core::geom::Pos::default());
    documents.extend([
        StoredDocument::Editable(JsonCodec::new("dmn.table").encode(&configured_table)),
        StoredDocument::Editable(JsonCodec::new("bpmn").encode(&pool_only)),
        StoredDocument::Editable(JsonCodec::new("event-log").encode(&LogDocument {
            answers: [("step:TI".into(), "{A}".into())].into(),
            ..Default::default()
        })),
        StoredDocument::Editable(JsonCodec::new("mermaid").encode(&op_core::mermaid::Diagram {
            name: "Invalid draft".into(),
            source: "flowchart TD\n A[Unfinished".into(),
        })),
        StoredDocument::Original {
            bytes: b"unknown original data".to_vec(),
            reason: "Unrecognized format".into(),
        },
    ]);
    let mut h = Harness::builder().build_eframe(|cc| OpenProcessApp::new(cc, Vec::new(), registry));
    for document in documents {
        h.state_mut().open_stored(document, None, false, None);
        let uid = h.state().docs[0].uid;
        let before = h.state().docs[0].snapshot();
        h.state_mut().handle_dock_event(DockEvent::Close(uid));
        assert_eq!(h.state().confirm_close, Some(0));
        assert_eq!(h.state().docs[0].snapshot(), before);
        h.state_mut().confirm_close = None;
        h.state_mut().close(0);
    }

    h.state_mut().open_model(Model::Bpmn(examples::order_process()), None, false);
    h.run_steps(3);
    h.get_by_label("File").click();
    h.run_steps(2);
    h.get_by_label("Close tab").click();
    h.run_steps(3);
    assert_eq!(h.state().confirm_close, Some(0));
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert_eq!(h.state().docs.len(), 1);
}

#[test]
fn deleting_all_saved_diagram_content_allows_closing_without_confirmation() {
    let mut h = harness(false);
    let mut diagram = examples::order_process();
    diagram.pools.clear();
    h.state_mut().open_model(Model::Bpmn(diagram), Some("saved.opx".into()), true);
    let index = h.state().active;
    let uid = h.state().docs[index].uid;
    assert!(!h.state().docs[index].dirty());
    h.state_mut().docs[index]
        .editor
        .as_any_mut()
        .downcast_mut::<BpmnEditor>()
        .unwrap()
        .select_all();
    h.state_mut().execute(DELETE, 0.0);
    h.run_steps(3);
    assert!(h.state().docs[index].dirty());
    assert!(!h.state().docs[index].editor.has_content());
    assert!(h.state().docs[index].editor.enabled(UNDO));
    h.state_mut().handle_dock_event(DockEvent::Close(uid));
    assert!(h.state().confirm_close.is_none());
    assert!(!h.state().docs.iter().any(|doc| doc.uid == uid));
}

#[test]
fn unsupported_files_remain_recoverable_byte_for_byte() {
    let mut h = harness(false);
    for bytes in [
        br#"{ "format":"open-process", "version":999, "secret-data":[1,2,3] }"#.as_slice(),
        br#"{"format":"open-process","version":2,"document":{"kind":"future.diagram","model_version":9,"model":{"content":"keep me"}}}"#
            .as_slice(),
    ] {
        h.state_mut().load_bytes("future.opx".into(), bytes);
        let doc = h.state().docs.last().unwrap();
        assert_eq!(doc.snapshot().bytes(), bytes);
        assert_eq!(doc.editor.export().unwrap().bytes, bytes);
        assert!(!doc.dirty());
        let restored: StoredDocument = serde_json::from_str(&serde_json::to_string(&doc.snapshot()).unwrap()).unwrap();
        assert_eq!(restored.bytes(), bytes);
    }
}

#[test]
fn escape_rolls_back_a_drag_without_adding_an_undo_step() {
    let mut h = harness(false);
    h.state_mut().open_model(Model::Uml(examples::uml_classes()), None, true);
    h.run_steps(5);
    let id = uml(&h).diagram.nodes[0].id;
    let from = uml(&h).node_screen_center(id);
    let original = h.state().docs.last().unwrap().snapshot();
    let revision = h.state().docs.last().unwrap().editor.revisions();
    h.hover_at(from);
    h.run_steps(2);
    press(&mut h, from, true);
    let to = from + egui::vec2(80.0, 40.0);
    h.hover_at(to);
    h.run_steps(3);
    assert_ne!(h.state().docs.last().unwrap().snapshot(), original);
    assert_eq!(h.state().docs.last().unwrap().editor.revisions().semantic, revision.semantic);
    assert!(h.state().docs.last().unwrap().editor.revisions().presentation > revision.presentation);
    key(&mut h, Key::Escape);
    press(&mut h, to, false);
    assert_eq!(h.state().docs.last().unwrap().snapshot(), original);
    assert!(!h.state().docs.last().unwrap().dirty());
    assert!(!h.state().docs.last().unwrap().editor.enabled(UNDO));
}

#[test]
fn failed_or_cancelled_saves_keep_the_unsaved_document() {
    let mut h = harness(false);
    h.state_mut().open_model(Model::Uml(examples::uml_classes()), None, true);
    let index = h.state().active;
    h.state_mut().docs[index].editor.rename("Unsaved edit".into());
    let edited = h.state().docs[index].snapshot();
    assert_eq!(h.state_mut().docs[index].complete_save(edited.clone(), Ok(None)).unwrap(), None);
    assert!(h.state().docs[index].dirty());
    let path = format!("/tmp/open-process-absent-parent-{}/model.opx", h.state().docs[index].uid);
    assert!(!std::path::Path::new(&path).parent().unwrap().exists());
    h.state_mut().docs[index].path = Some(path.clone());
    assert!(!h.state_mut().save(false));
    assert!(h.state().docs[index].dirty());
    assert_eq!(h.state().docs[index].snapshot(), edited);
    assert_eq!(h.state().docs[index].path.as_deref(), Some(path.as_str()));
    h.state_mut().execute(UNDO, 0.0);
    assert!(
        !h.state().docs[index].dirty(),
        "failed save must not replace the last saved reference"
    );
}

#[test]
fn browser_download_keeps_unsaved_content_and_previous_destination() {
    use crate::{
        editor::{Doc, StoredDocument},
        io::SaveOutcome,
    };
    let document = Model::Bpmn(examples::order_process()).document();
    let mut doc = Doc::new(
        StoredDocument::Editable(document),
        Some("original.opx".into()),
        true,
        crate::composition::registry(),
    );
    doc.editor.rename("Edited".into());
    let snapshot = doc.snapshot();
    doc.complete_save(snapshot.clone(), Ok(Some(SaveOutcome::DownloadStarted("download.opx".into()))))
        .unwrap();
    assert!(doc.dirty());
    assert_eq!(doc.path.as_deref(), Some("original.opx"));
    doc.complete_save(snapshot, Ok(Some(SaveOutcome::Written("confirmed.opx".into()))))
        .unwrap();
    assert!(!doc.dirty());
    assert_eq!(doc.path.as_deref(), Some("confirmed.opx"));
}

#[test]
fn invalid_known_documents_remain_recoverable_without_opening_an_editor() {
    let mut h = harness(false);
    let mut document = Model::Bpmn(examples::order_process()).document();
    document.model["flows"][0]["to"] = u32::MAX.into();
    let original = document.to_json().into_bytes();
    h.state_mut().load_bytes("invalid.opx".into(), &original);
    let doc = h.state().docs.last().unwrap();
    assert_eq!(doc.editor.kind(), "unavailable");
    assert_eq!(doc.snapshot().bytes(), original);
}

#[test]
fn oversized_files_are_rejected_before_sniffing_or_creating_a_document() {
    let mut h = harness(false);
    let count = h.state().docs.len();
    let bytes = vec![b' '; op_core::document::MAX_DOCUMENT_BYTES + 1];
    h.state_mut().load_bytes("oversized.data".into(), &bytes);
    assert_eq!(h.state().docs.len(), count);
    assert!(h.state().message.as_ref().unwrap().1);
}

#[test]
fn invalid_storage_cannot_overwrite_a_previously_saved_document() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("original.opx");
    let mut h = harness(false);
    let index = h.state().active;
    let snapshot = h.state().docs[index].snapshot();
    let original = snapshot.bytes();
    std::fs::write(&path, &original).unwrap();
    h.state_mut().docs[index].mark_saved(snapshot);
    h.state_mut().docs[index].path = Some(path.display().to_string());
    h.state_mut().docs[index]
        .editor
        .as_any_mut()
        .downcast_mut::<BpmnEditor>()
        .unwrap()
        .diagram
        .flows[0]
        .to = u32::MAX;
    assert!(!h.state_mut().save(false));
    assert!(h.state().docs[index].dirty());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

fn active_view_settings(h: &Harness<'static, OpenProcessApp>) -> serde_json::Value {
    let doc = &h.state().docs[h.state().active];
    let pane = dock::panes_of(&h.state().tree, doc.uid)[0];
    let views: Vec<(u64, serde_json::Value)> = serde_json::from_value(doc.editor.workspace()).unwrap();
    views.into_iter().find(|(id, _)| *id == pane.0).unwrap().1
}

fn whiteboard<'a>(h: &'a Harness<'static, OpenProcessApp>) -> &'a crate::whiteboard_editor::WhiteboardEditor {
    h.state().docs[h.state().active]
        .editor
        .as_any()
        .downcast_ref::<crate::whiteboard_editor::WhiteboardEditor>()
        .unwrap()
}

#[test]
fn whiteboard_connections_snap_follow_resize_retarget_and_restore() {
    use op_core::{
        document::DocumentCodec,
        geom::{Port, Pos},
        whiteboard::{Board, Element, Kind, Paint},
    };
    let mut board = Board::new("Attached drawing");
    let source = board.add(Element::new(
        Kind::Rectangle,
        Pos::new(60.0, 80.0),
        Pos::new(220.0, 180.0),
        Paint::default(),
    ));
    let target = board.add(Element::new(
        Kind::Diamond,
        Pos::new(420.0, 80.0),
        Pos::new(520.0, 180.0),
        Paint::default(),
    ));
    let third = board.add(Element::new(
        Kind::Rectangle,
        Pos::new(350.0, 350.0),
        Pos::new(510.0, 450.0),
        Paint::default(),
    ));
    let mut h = harness(false);
    h.state_mut().open_document(Board::CODEC.encode(&board), None, false);
    h.run_steps(5);
    h.get_by_label(tr(Kind::Arrow.label())).click();
    h.run_steps(3);
    let start = whiteboard(&h).screen_point(Pos::new(220.0, 130.0));
    let end = whiteboard(&h).screen_point(Pos::new(420.0, 130.0));
    drag(&mut h, start, end);
    h.run_steps(4);
    let arrow = whiteboard(&h).board.elements.last().unwrap().id;
    assert_eq!(
        whiteboard(&h).board.element(arrow).unwrap().start_attachment.unwrap().element,
        source
    );
    assert_eq!(whiteboard(&h).board.element(arrow).unwrap().end_attachment.unwrap().element, target);
    let start = whiteboard(&h).screen_point(Pos::new(140.0, 130.0));
    let end = whiteboard(&h).screen_point(Pos::new(190.0, 200.0));
    drag(&mut h, start, end);
    h.run_steps(4);
    let board = &whiteboard(&h).board;
    assert_eq!(
        board.geometry().points(board.element(arrow).unwrap())[0],
        Port::Right.point(board.element(source).unwrap().bounds)
    );
    let bounds = board.element(source).unwrap().bounds;
    let from = whiteboard(&h).screen_point(Pos::new(bounds.right(), bounds.bottom()));
    let to = whiteboard(&h).screen_point(Pos::new(bounds.right() + 60.0, bounds.bottom() + 30.0));
    drag(&mut h, from, to);
    h.run_steps(4);
    let resized = whiteboard(&h).board.clone();
    let points = resized.geometry().points(resized.element(arrow).unwrap());
    assert_eq!(points[0], Port::Right.point(resized.element(source).unwrap().bounds));
    h.state_mut().undo();
    h.run_steps(3);
    assert_ne!(
        whiteboard(&h).board.element(source).unwrap().bounds,
        resized.element(source).unwrap().bounds
    );
    h.state_mut().redo();
    h.run_steps(3);
    let center = whiteboard(&h).screen_point(points[0].lerp(points[1], 0.5));
    click(&mut h, center);
    h.run_steps(3);
    let from = whiteboard(&h).screen_point(points[1]);
    let to = whiteboard(&h).screen_point(Pos::new(430.0, 350.0));
    drag(&mut h, from, to);
    h.run_steps(4);
    let attachment = whiteboard(&h).board.element(arrow).unwrap().end_attachment.unwrap();
    assert_eq!((attachment.element, attachment.port), (third, Port::Top));
    let from = whiteboard(&h).screen_point(Pos::new(430.0, 350.0));
    let to = whiteboard(&h).screen_point(Pos::new(630.0, 470.0));
    drag(&mut h, from, to);
    h.run_steps(4);
    assert!(whiteboard(&h).board.element(arrow).unwrap().end_attachment.is_none());
    h.state_mut().undo();
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board.element(arrow).unwrap().end_attachment.unwrap(), attachment);
    let snapshot = h.state().docs[h.state().active].snapshot();
    h.state_mut().load_bytes("attached.opx".into(), &snapshot.bytes());
    h.run_steps(4);
    assert_eq!(h.state().docs[h.state().active].snapshot(), snapshot);
    assert_eq!(h.state().docs[h.state().active].editor.kind(), "whiteboard");
    shot(&mut h, "whiteboard_attached_connectors");
}

#[test]
fn whiteboard_creation_tools_text_and_vector_export_roundtrip() {
    use op_core::geom::Pos;
    use op_core::whiteboard::Kind;
    let mut h = harness(false);
    h.get_by_label("New document").click();
    h.run_steps(5);
    h.get_by_label("Search diagram types").type_text("whiteboard");
    h.run_steps(4);
    h.get_by_label("New free drawing").click();
    h.run_steps(5);
    assert_eq!(h.state().docs.last().unwrap().editor.kind(), "whiteboard");
    for (i, kind) in Kind::ALL.into_iter().enumerate() {
        h.get_by_label(tr(kind.label())).click();
        h.run_steps(3);
        let x = 80.0 + (i % 4) as f32 * 220.0;
        let y = 80.0 + (i / 4) as f32 * 240.0;
        let from = whiteboard(&h).screen_point(Pos::new(x, y));
        let to = whiteboard(&h).screen_point(Pos::new(x + 160.0, y + 100.0));
        drag(&mut h, from, to);
        h.run_steps(4);
        assert_eq!(whiteboard(&h).board.elements.len(), i + 1, "{kind:?}");
        assert_eq!(whiteboard(&h).board.elements.last().unwrap().kind, kind);
    }
    h.get_by_label("Element text").focus();
    h.run_steps(2);
    h.get_by_label("Element text").type_text("Review <idea> & discuss\nNext step");
    h.run_steps(3);
    h.get_by_label("Select").click();
    h.run_steps(4);
    let original = h.state().docs.last().unwrap().snapshot();
    let svg = h.state().docs.last().unwrap().editor.export().unwrap();
    assert_eq!(svg.suffix, "svg");
    assert!(
        std::str::from_utf8(&svg.bytes)
            .unwrap()
            .contains("Review &lt;idea&gt; &amp; discuss")
    );
    assert!(
        whiteboard(&h)
            .board
            .elements
            .iter()
            .find(|e| e.kind == Kind::Freehand)
            .unwrap()
            .points
            .len()
            > 2
    );
    whiteboard(&h).board.validate().unwrap();
    h.state_mut().docs.last_mut().unwrap().editor.execute(UNDO, 0.0);
    h.run_steps(4);
    assert_ne!(h.state().docs.last().unwrap().snapshot(), original);
    h.state_mut().docs.last_mut().unwrap().editor.execute(REDO, 0.0);
    h.run_steps(4);
    assert_eq!(h.state().docs.last().unwrap().snapshot(), original);
    shot(&mut h, "whiteboard_all_tools_light");
    let bytes = original.bytes();
    h.state_mut().load_bytes("drawing.opx".into(), &bytes);
    h.run_steps(4);
    assert_eq!(h.state().docs.last().unwrap().snapshot(), original);
    assert!(!h.state().docs.last().unwrap().dirty());
}

#[test]
fn whiteboard_moving_resizing_duplication_and_cancel_are_undoable() {
    use op_core::geom::Pos;
    let mut h = harness(true);
    h.state_mut().open_example("whiteboard.ideas");
    h.run_steps(5);
    let index = h.state().active;
    let id = whiteboard(&h).board.elements[0].id;
    let original = h.state().docs[index].snapshot();
    let from = whiteboard(&h).element_screen_center(id);
    drag(&mut h, from, from + egui::vec2(40.0, 30.0));
    h.run_steps(4);
    let moved = h.state().docs[index].snapshot();
    assert_ne!(moved, original);
    let b = whiteboard(&h).board.element(id).unwrap().bounds;
    let corner = whiteboard(&h).screen_point(Pos::new(b.right(), b.bottom()));
    drag(&mut h, corner, corner + egui::vec2(45.0, 25.0));
    h.run_steps(4);
    assert!(whiteboard(&h).board.element(id).unwrap().bounds.w > b.w);
    h.state_mut().docs[index].editor.execute(UNDO, 0.0);
    h.run_steps(4);
    assert_eq!(h.state().docs[index].snapshot(), moved);
    h.state_mut().docs[index].editor.execute("whiteboard.duplicate", 0.0);
    h.run_steps(4);
    assert_eq!(whiteboard(&h).board.elements.len(), 5);
    h.state_mut().docs[index].editor.execute(DELETE, 0.0);
    h.run_steps(4);
    assert_eq!(whiteboard(&h).board.elements.len(), 4);
    let deleted = h.state().docs[index].snapshot();
    let from = whiteboard(&h).element_screen_center(id);
    click(&mut h, from);
    press(&mut h, from, true);
    h.hover_at(from + egui::vec2(90.0, 45.0));
    h.run_steps(3);
    assert_ne!(h.state().docs[index].snapshot(), deleted);
    key(&mut h, Key::Escape);
    press(&mut h, from + egui::vec2(90.0, 45.0), false);
    h.run_steps(4);
    assert_eq!(h.state().docs[index].snapshot(), deleted);
    h.state_mut().docs[index].editor.execute(UNDO, 0.0);
    h.run_steps(4);
    // The last committed operation was deletion; cancelling the drag added no history step.
    assert_eq!(whiteboard(&h).board.elements.len(), 5);
    h.state_mut().docs[index].editor.execute(REDO, 0.0);
    h.run_steps(4);
    assert_eq!(h.state().docs[index].snapshot(), deleted);
    shot(&mut h, "whiteboard_example_dark");
}

#[test]
fn whiteboard_group_selection_keeps_mixed_fills_and_pencil_click_draws_a_dot() {
    use op_core::geom::Pos;
    use op_core::whiteboard::Kind;
    let mut h = harness(false);
    h.state_mut().open_example("whiteboard.ideas");
    h.run_steps(5);
    let fills: Vec<_> = whiteboard(&h).board.elements.iter().map(|e| e.paint.fill).collect();
    let bounds = whiteboard(&h).board.bounds().unwrap();
    // Begin beside the floating palette, while still enclosing the leftmost element.
    let from = whiteboard(&h).screen_point(Pos::new(bounds.x - 8.0, bounds.y - 20.0));
    let to = whiteboard(&h).screen_point(Pos::new(bounds.right() + 20.0, bounds.bottom() + 20.0));
    drag(&mut h, from, to);
    h.run_steps(4);
    assert!(h.query_by_label("4 selected").is_some());
    h.get_by_label("Hand-drawn (sketch)").click();
    h.run_steps(4);
    assert!(whiteboard(&h).board.elements.iter().all(|e| !e.paint.sketch));
    assert_eq!(
        whiteboard(&h).board.elements.iter().map(|e| e.paint.fill).collect::<Vec<_>>(),
        fills
    );
    h.get_by_label("Freehand").click();
    h.run_steps(3);
    let point = whiteboard(&h).screen_point(Pos::new(300.0, 320.0));
    click(&mut h, point);
    let dot = whiteboard(&h).board.elements.last().unwrap();
    assert_eq!(dot.kind, Kind::Freehand);
    assert!(dot.bounds.w <= 1.0 && dot.bounds.h <= 1.0);
    assert_eq!(whiteboard(&h).board.elements.len(), 5);
    whiteboard(&h).board.validate().unwrap();
}

#[test]
fn new_document_picker_is_a_responsive_registry_grid() {
    for width in [1400.0, 420.0, 280.0] {
        let mut h = harness_sized(false, [width, 860.0]);
        let original = h.state().docs[0].snapshot();
        h.get_by_label("File").click();
        h.run_steps(3);
        assert!(h.query_by_label("New BPMN diagram").is_none(), "File contains one New entry");
        assert!(h.query_by_label("Examples").is_none(), "Examples belongs to the selection grid");
        h.get_by_label(&format!(
            "New… {}",
            h.ctx.format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::N))
        ))
        .click();
        h.run_steps(5);
        assert!(h.state().new_picker.is_some());
        assert_eq!(h.state().docs.len(), 1);
        let a = h
            .get_by_label(tr(crate::composition::registry().find("whiteboard").unwrap().new_label))
            .rect();
        let b = h
            .get_by_label(tr(crate::composition::registry().find("database.schema").unwrap().new_label))
            .rect();
        assert!(a.left() >= 0.0 && a.right() <= width, "picker cards fit at {width}px: {a:?}");
        if width > 1000.0 {
            assert!((a.top() - b.top()).abs() < 1.0);
            assert!(b.left() > a.right());
        } else {
            assert!(b.top() > a.bottom());
        }
        shot(&mut h, &format!("new_document_grid_{width}"));
        h.get_by_label("New BPMN diagram").scroll_to_me();
        h.run_steps(12);
        let bpmn = h.get_by_label("New BPMN diagram").rect();
        let examples = h.get_by_label("Examples for New BPMN diagram").rect();
        assert!(bpmn.contains_rect(examples), "the examples button belongs to its card");
        let examples = examples.center();
        click(&mut h, examples);
        h.run_steps(4);
        assert!(h.state().new_picker.is_some());
        assert_eq!(h.state().docs.len(), 1, "opening examples must not create a blank document");
        shot(&mut h, &format!("new_document_examples_{width}"));
        for example in &crate::composition::registry().examples {
            assert_eq!(
                h.query_by_label(tr(example.label)).is_some(),
                example.kind == "bpmn",
                "{} at {width}px",
                tr(example.label)
            );
        }
        key(&mut h, Key::Escape);
        h.run_steps(4);
        assert!(h.state().new_picker.is_some(), "Escape closes the examples menu first");
        let cancel = h.get_by_label("Cancel").rect().center();
        click(&mut h, cancel);
        h.run_steps(4);
        assert!(h.state().new_picker.is_none());
        assert_eq!(h.state().docs[0].snapshot(), original);
    }
}

#[test]
fn new_document_picker_filters_cancels_and_opens_from_tabs_and_shortcut() {
    let mut h = harness(false);
    let original = h.state().docs[0].snapshot();
    h.get_by_label("New document").click();
    h.run_steps(5);
    h.get_by_label("Search diagram types").type_text("no such notation");
    h.run_steps(4);
    assert!(h.query_by_label("No matching diagram types.").is_some());
    key(&mut h, Key::Escape);
    h.run_steps(4);
    assert!(h.state().new_picker.is_none());
    assert_eq!(h.state().docs.len(), 1);
    h.key_press_modifiers(Modifiers::COMMAND, Key::N);
    h.run_steps(5);
    assert!(h.state().new_picker.is_some());
    h.get_by_label("Search diagram types").type_text("database");
    h.run_steps(4);
    assert!(h.query_by_label("New database schema").is_some());
    assert!(h.query_by_label("New BPMN diagram").is_none());
    shot(&mut h, "new_document_grid_filtered");
    h.get_by_label("New database schema").click();
    h.run_steps(4);
    assert!(h.state().new_picker.is_none());
    assert_eq!(h.state().docs.last().unwrap().editor.kind(), "database.schema");
    assert_eq!(h.state().docs[0].snapshot(), original);
}

#[test]
fn new_document_from_a_split_tab_bar_stays_in_that_group() {
    for from_example in [false, true] {
        let mut h = harness(true);
        let original = h.state().docs[0].snapshot();
        let uid = h.state().docs[0].uid;
        h.state_mut().open_beside(
            Pane {
                doc: uid,
                view: View::Editor,
            },
            false,
        );
        h.run_steps(5);
        let position = h
            .get_all_by_label("New document")
            .map(|n| n.rect().center())
            .max_by(|a, b| a.x.total_cmp(&b.x))
            .unwrap();
        click(&mut h, position);
        let near = h.state().new_picker.as_ref().unwrap().near.unwrap();
        let group = h.state().tree.tiles.parent_of(near);
        if from_example {
            h.get_by_label("Search diagram types").type_text("database");
            h.run_steps(4);
            h.get_by_label("Examples for New database schema").click();
            h.run_steps(4);
            h.get_by_label(tr("database.database_shop_schema")).click();
        } else {
            h.get_by_label("Search diagram types").type_text("epc");
            h.run_steps(4);
            h.get_by_label("New EPC diagram").click();
        }
        h.run_steps(4);
        assert!(h.state().new_picker.is_none());
        assert_eq!(h.state().docs.len(), 2);
        assert_eq!(h.state().docs[0].snapshot(), original);
        let doc = h.state().docs.last().unwrap();
        assert_eq!(doc.editor.kind(), if from_example { "database.schema" } else { "epc" });
        if from_example {
            let expected = crate::composition::registry()
                .open(&crate::composition::registry().example("database.shop").unwrap())
                .unwrap();
            assert_eq!(doc.snapshot(), expected.snapshot());
        }
        let pane = dock::panes_of(&h.state().tree, doc.uid)[0];
        assert_eq!(h.state().tree.tiles.parent_of(pane), group);
    }
}

#[test]
fn file_exports_are_grouped_and_follow_the_active_document() {
    for (kind, formats, png) in [
        ("bpmn", vec!["Export BPMN 2.0 XML (.bpmn)…"], true),
        ("dmn.table", vec!["Export DMN XML (.dmn)…", "Export decision table (.csv)…"], false),
        ("epc", vec!["Export SVG (.svg)…", "Export Graphviz (.dot)…"], true),
        ("database.schema", vec!["Export SQL DDL (.sql)…"], true),
        ("whiteboard", vec!["Export SVG (.svg)…"], true),
    ] {
        let mut h = harness(false);
        h.state_mut().new_document(kind, None);
        h.run_steps(4);
        h.get_by_label("File").click();
        h.run_steps(3);
        for label in &formats {
            assert!(h.query_by_label(label).is_none(), "exports live in the submenu");
        }
        h.get_by_label("Export").click();
        h.run_steps(4);
        for label in formats {
            assert!(h.query_by_label(label).is_some(), "{kind}: {label}");
        }
        assert_eq!(h.query_by_label("Export image (PNG)…").is_some(), png);
        if kind == "bpmn" {
            shot(&mut h, "file_export_submenu");
        }
    }
}

#[test]
fn bpmn_auto_arrange_respects_grid_in_wide_and_compact_views_and_undoes() {
    for (width, snap) in [(1400.0, true), (420.0, true), (1400.0, false), (420.0, false)] {
        let mut h = harness_sized(false, [width, 860.0]);
        let original = h.state().docs[0].snapshot();
        let semantic = h.state().docs[0].editor.revisions().semantic;
        let business = EditorBackend::semantic_document(h.state().docs[0].editor.as_any().downcast_ref::<BpmnEditor>().unwrap());
        if width < 720.0 {
            assert!(h.query_by_label("Auto arrange").is_none());
            h.get_by_label("Options").click();
            h.run_steps(3);
        }
        if !snap {
            h.get_by_label("Snap to grid").click();
            h.run_steps(3);
            if width < 720.0 {
                h.get_by_label("Options").click();
                h.run_steps(3);
            }
        }
        h.get_by_label("Auto arrange").click();
        h.run_steps(5);
        let arranged = h.state().docs[0].snapshot();
        assert_ne!(arranged, original);
        let editor = h.state().docs[0].editor.as_any().downcast_ref::<BpmnEditor>().unwrap();
        assert_eq!(editor.diagram.nodes.iter().all(|n| n.pos == n.pos.snapped(10.0)), snap);
        assert_eq!(h.state().docs[0].editor.revisions().semantic, semantic);
        assert_eq!(
            EditorBackend::semantic_document(h.state().docs[0].editor.as_any().downcast_ref::<BpmnEditor>().unwrap()),
            business
        );
        assert!(h.state().docs[0].editor.enabled(UNDO));
        shot(&mut h, &format!("bpmn_auto_arranged_{width}_{snap}"));
        h.state_mut().docs[0].editor.execute(UNDO, 0.0);
        h.run_steps(4);
        assert_eq!(h.state().docs[0].snapshot(), original);
        h.state_mut().docs[0].editor.execute(REDO, 0.0);
        h.run_steps(4);
        assert_eq!(h.state().docs[0].snapshot(), arranged);
    }
}

#[test]
fn database_tables_relationships_history_and_ddl_views() {
    use crate::database_editor::DatabaseEditor;
    use op_core::database::Dialect;
    let mut h = harness(false);
    h.state_mut().new_document("database.schema", None);
    h.run_steps(4);
    assert!(
        h.state().docs.last().unwrap().editor.export().is_err(),
        "an empty schema cannot produce DDL"
    );
    h.get_by_label("Add table").click();
    h.run_steps(4);
    h.get_by_label("Add table").click();
    h.run_steps(4);
    let e = h
        .state()
        .docs
        .last()
        .unwrap()
        .editor
        .as_any()
        .downcast_ref::<DatabaseEditor>()
        .unwrap();
    assert_eq!(e.schema.tables.len(), 2);
    assert!(e.schema.tables[0].columns[0].primary_key);
    let a = e.table_screen_center(e.schema.tables[0].id);
    let b = e.table_screen_center(e.schema.tables[1].id);
    h.get_by_label("Connect").click();
    h.run_steps(3);
    click(&mut h, b);
    click(&mut h, a);
    let e = h
        .state()
        .docs
        .last()
        .unwrap()
        .editor
        .as_any()
        .downcast_ref::<DatabaseEditor>()
        .unwrap();
    assert_eq!(e.schema.foreign_keys.len(), 1);
    assert_eq!(e.schema.tables[1].columns.len(), 2);
    let original = h.state().docs.last().unwrap().snapshot();
    let sql = h.state().docs.last().unwrap().editor.export().unwrap().bytes;
    assert!(std::str::from_utf8(&sql).unwrap().contains("FOREIGN KEY"));
    let semantic = h.state().docs.last().unwrap().editor.revisions().semantic;
    h.get_by_label("Select").click();
    h.run_steps(3);
    drag(&mut h, b, b + egui::vec2(80.0, 40.0));
    assert_ne!(h.state().docs.last().unwrap().snapshot(), original);
    assert_eq!(h.state().docs.last().unwrap().editor.revisions().semantic, semantic);
    assert_eq!(
        h.state().docs.last().unwrap().editor.export().unwrap().bytes,
        sql,
        "moving tables does not change DDL"
    );
    h.state_mut().docs.last_mut().unwrap().editor.execute(crate::editor::UNDO, 0.0);
    h.run_steps(4);
    assert_eq!(h.state().docs.last().unwrap().snapshot(), original);
    h.get_by_label("Target database").click();
    h.run_steps(3);
    h.get_by_label("SQLite").click();
    h.run_steps(4);
    assert_eq!(
        h.state()
            .docs
            .last()
            .unwrap()
            .editor
            .as_any()
            .downcast_ref::<DatabaseEditor>()
            .unwrap()
            .schema
            .dialect,
        Dialect::Sqlite
    );
    let StoredDocument::Editable(document) = h.state().docs.last().unwrap().snapshot() else {
        panic!("editable schema")
    };
    let reopened = crate::composition::registry().open(&document).unwrap();
    assert_eq!(reopened.snapshot(), StoredDocument::Editable(document));
    assert!(
        std::str::from_utf8(&reopened.export().unwrap().bytes)
            .unwrap()
            .contains("PRAGMA foreign_keys = ON")
    );
    h.get_by_label("SQL / DDL").click();
    h.run_steps(5);
    assert!(h.query_by_label("Copy DDL").is_some());
    let before_copy = h.state().docs.last().unwrap().snapshot();
    h.get_by_label("Copy DDL").click();
    h.run_steps(4);
    assert_eq!(h.state().docs.last().unwrap().snapshot(), before_copy);
    let uid = h.state().docs.last().unwrap().uid;
    assert_eq!(dock::panes_of(&h.state().tree, uid).len(), 2);
    shot(&mut h, "database_ddl_split");
}

#[test]
fn database_example_renders_in_both_themes_and_columns_undo_atomically() {
    use crate::database_editor::DatabaseEditor;
    for dark in [false, true] {
        let mut h = harness(dark);
        h.state_mut().open_example("database.shop");
        h.run_steps(5);
        let original = h.state().docs.last().unwrap().snapshot();
        let total = h
            .state()
            .docs
            .last()
            .unwrap()
            .editor
            .as_any()
            .downcast_ref::<DatabaseEditor>()
            .unwrap()
            .schema
            .tables[0]
            .columns
            .len();
        h.get_by_label("Add column").click();
        h.run_steps(4);
        assert_eq!(
            h.state()
                .docs
                .last()
                .unwrap()
                .editor
                .as_any()
                .downcast_ref::<DatabaseEditor>()
                .unwrap()
                .schema
                .tables[0]
                .columns
                .len(),
            total + 1
        );
        h.state_mut().docs.last_mut().unwrap().editor.execute(crate::editor::UNDO, 0.0);
        h.run_steps(3);
        assert_eq!(h.state().docs.last().unwrap().snapshot(), original);
        shot(&mut h, if dark { "database_example_dark" } else { "database_example_light" });
    }
}

#[test]
fn new_graph_notations_create_connect_move_undo_and_reopen() {
    fn exercise<K: crate::flow_editor::Notation>(kind: &str, first: &str, second: &str, dark: bool) {
        let mut h = harness(dark);
        h.state_mut().new_document(kind, None);
        h.run_steps(5);
        h.get_by_label(first).click();
        h.run_steps(3);
        click(&mut h, egui::pos2(330.0, 300.0));
        h.get_by_label(second).click();
        h.run_steps(3);
        click(&mut h, egui::pos2(650.0, 400.0));
        let editor = h
            .state()
            .docs
            .last()
            .unwrap()
            .editor
            .as_any()
            .downcast_ref::<crate::flow_editor::FlowEditor<K>>()
            .unwrap();
        assert_eq!(editor.diagram.nodes.len(), 2);
        let a = editor.node_screen_center(editor.diagram.nodes[0].id);
        let b = editor.node_screen_center(editor.diagram.nodes[1].id);
        h.get_by_label("Connect").click();
        h.run_steps(3);
        click(&mut h, a);
        click(&mut h, b);
        let original = h.state().docs.last().unwrap().snapshot();
        let editor = h
            .state()
            .docs
            .last()
            .unwrap()
            .editor
            .as_any()
            .downcast_ref::<crate::flow_editor::FlowEditor<K>>()
            .unwrap();
        assert_eq!(editor.diagram.flows.len(), 1);
        h.get_by_label("Select").click();
        h.run_steps(3);
        drag(&mut h, b, b + egui::vec2(100.0, 30.0));
        assert_ne!(h.state().docs.last().unwrap().snapshot(), original);
        h.state_mut().docs.last_mut().unwrap().editor.execute(crate::editor::UNDO, 0.0);
        h.run_steps(3);
        assert_eq!(h.state().docs.last().unwrap().snapshot(), original);
        let stored = h.state().docs.last().unwrap().editor.snapshot();
        let StoredDocument::Editable(document) = stored else {
            panic!("editable diagram")
        };
        let reopened = crate::composition::registry().open(&document).unwrap();
        assert_eq!(reopened.snapshot(), StoredDocument::Editable(document));
        assert!(std::str::from_utf8(&reopened.export().unwrap().bytes).unwrap().contains("<svg"));
        shot(&mut h, &format!("new_{kind}"));
    }
    exercise::<op_core::epc::EpcKind>("epc", "Event", "Function", false);
    exercise::<op_core::activity::ActivityKind>("uml.activity", "Initial node", "Action", true);
}

#[test]
fn decision_table_edits_evaluates_exports_and_undoes() {
    let mut h = harness(false);
    h.state_mut().open_example("dmn.discount");
    h.run_steps(5);
    h.get_by_label("Evaluate decision").click();
    h.run_steps(4);
    assert!(h.query_by_label("Matched rules: 1").is_some());
    let original = h.state().docs.last().unwrap().snapshot();
    h.get_by_label("Add input").click();
    h.run_steps(4);
    let editor = h
        .state()
        .docs
        .last()
        .unwrap()
        .editor
        .as_any()
        .downcast_ref::<crate::dmn_editor::DmnEditor>()
        .unwrap();
    assert_eq!(editor.table.inputs.len(), 2);
    assert_eq!(editor.table.rules[0].inputs.len(), 2);
    assert!(
        h.query_by_label("Matched rules: 1").is_none(),
        "editing invalidates a previous result"
    );
    h.state_mut().docs.last_mut().unwrap().editor.execute(crate::editor::UNDO, 0.0);
    h.run_steps(3);
    assert_eq!(h.state().docs.last().unwrap().snapshot(), original);
    let export = h.state().docs.last().unwrap().editor.export().unwrap();
    let document = crate::composition::registry().load(&export.bytes).unwrap();
    let reopened = crate::composition::registry().open(&document).unwrap();
    assert_eq!(reopened.snapshot(), original);
    assert!(
        h.state()
            .docs
            .last()
            .unwrap()
            .editor
            .export_as("csv")
            .unwrap()
            .bytes
            .starts_with(b"\"input:amount\"")
    );
    shot(&mut h, "new_dmn_table");
    h.state_mut().docs.last_mut().unwrap().editor.rename("Discount model".into());
    h.state_mut().docs.last_mut().unwrap().editor.execute(crate::editor::UNDO, 0.0);
    assert_eq!(h.state().docs.last().unwrap().snapshot(), original);
}

#[test]
fn new_notation_workspace_restores_multiple_independent_views() {
    for (kind, example) in [
        ("epc", "epc.order"),
        ("uml.activity", "uml.activity.order"),
        ("dmn.table", "dmn.discount"),
        ("whiteboard", "whiteboard.ideas"),
    ] {
        let mut h = harness(false);
        h.state_mut().open_example(example);
        h.run_steps(4);
        shot(&mut h, &format!("example_{kind}"));
        let uid = h.state().docs.last().unwrap().uid;
        h.state_mut().open_beside(
            Pane {
                doc: uid,
                view: View::Editor,
            },
            false,
        );
        h.run_steps(4);
        let before = h.state().docs.last().unwrap().snapshot();
        let workspace = h.state().docs.last().unwrap().editor.workspace();
        let mut states: Vec<(u64, serde_json::Value)> = serde_json::from_value(workspace).unwrap();
        assert_eq!(states.len(), 2);
        states[0].1["inspector"] = false.into();
        states[1].1["inspector"] = true.into();
        if kind != "dmn.table" {
            states[0].1["snap"] = false.into();
            states[1].1["snap"] = true.into();
        }
        h.state_mut()
            .docs
            .last_mut()
            .unwrap()
            .editor
            .restore_workspace(&serde_json::to_value(&states).unwrap());
        h.run_steps(4);
        let restored: Vec<(u64, serde_json::Value)> = serde_json::from_value(h.state().docs.last().unwrap().editor.workspace()).unwrap();
        for (id, state) in states {
            assert_eq!(restored.iter().find(|(i, _)| *i == id).unwrap().1["inspector"], state["inspector"]);
        }
        assert_eq!(h.state().docs.last().unwrap().snapshot(), before);
    }
}

#[test]
fn diagram_editors_share_configuration_controls_in_wide_and_narrow_panes() {
    let mut wide_positions = Vec::new();
    for (kind, dark) in [
        ("bpmn", false),
        ("petri", true),
        ("uml.class", false),
        ("uml.use-case", true),
        ("epc", false),
        ("uml.activity", true),
        ("database.schema", false),
        ("whiteboard", true),
    ] {
        for width in [1400.0, 420.0, 280.0] {
            let mut h = harness_sized(dark, [width, 860.0]);
            h.state_mut().new_document(kind, None);
            h.run_steps(5);
            let original = h.state().docs.last().unwrap().snapshot();
            let settings = active_view_settings(&h);
            let initially_snapped = kind != "whiteboard";
            assert_eq!(settings["snap"], initially_snapped);
            let zoom_in = h.get_by_label("Zoom in").rect();
            let zoom_out = h.get_by_label("Zoom out").rect();
            assert!(
                zoom_in.left() >= 0.0 && zoom_out.right() <= width,
                "{kind} controls must fit at {width}px"
            );
            assert!((zoom_in.center().y - zoom_out.center().y).abs() < 1.0);
            if width > 1000.0 {
                assert!(h.query_by_label("Options").is_none());
                assert!(h.query_by_label("Snap to grid").is_some());
                assert!(h.query_by_label("Fit").is_some());
                let inspector = h.get_by_label("Inspector").rect();
                wide_positions.push((zoom_in.center().y, inspector.right()));
                if kind == "bpmn" {
                    h.get_by_label("Show issues").click();
                    h.run_steps(3);
                    assert_eq!(active_view_settings(&h)["issues"], true);
                }
            } else {
                assert!(
                    h.query_by_label("Snap to grid").is_none(),
                    "compact options are tucked into the shared menu"
                );
                h.get_by_label("Options").click();
                h.run_steps(3);
                assert!(h.query_by_label("Snap to grid").is_some());
                if kind == "bpmn" {
                    assert!(h.query_by_label("Show issues").is_some());
                    if width == 420.0 {
                        shot(&mut h, "common_config_options_bpmn");
                    }
                }
            }
            h.get_by_label("Snap to grid").click();
            h.run_steps(3);
            assert_eq!(active_view_settings(&h)["snap"], !initially_snapped, "{kind} at {width}px");
            key(&mut h, Key::Escape);
            if width < 360.0 {
                h.get_by_label("Options").click();
                h.run_steps(2);
            }
            h.get_by_label("Inspector").click();
            h.run_steps(4);
            key(&mut h, Key::Escape);
            assert_eq!(active_view_settings(&h)["inspector"], width < 1000.0);
            let zoom = active_view_settings(&h)["camera"]["zoom"].as_f64().unwrap();
            h.get_by_label("Zoom in").click();
            h.run_steps(3);
            assert!(active_view_settings(&h)["camera"]["zoom"].as_f64().unwrap() > zoom);
            h.get_by_label("Zoom out").click();
            h.run_steps(3);
            assert!((active_view_settings(&h)["camera"]["zoom"].as_f64().unwrap() - zoom).abs() < 0.0001);
            assert_eq!(
                h.state().docs.last().unwrap().snapshot(),
                original,
                "display settings must not modify the model"
            );
            if width > 1000.0 || (kind == "uml.class" && width == 420.0) {
                shot(&mut h, &format!("common_config_{}_{}", kind, width));
            }
        }
    }
    for p in &wide_positions[1..] {
        assert!(
            (p.0 - wide_positions[0].0).abs() < 1.0,
            "configuration bars use the same vertical position"
        );
        assert!(
            (p.1 - wide_positions[0].1).abs() < 1.0,
            "configuration bars use the same alignment: {wide_positions:?}"
        );
    }
}

#[test]
fn canvas_options_are_restored_independently_for_each_view() {
    let mut h = harness(false);
    let uid = h.state().docs[0].uid;
    h.state_mut().open_beside(
        Pane {
            doc: uid,
            view: View::Editor,
        },
        false,
    );
    h.run_steps(5);
    let original = h.state().docs[0].snapshot();
    let mut views: Vec<(u64, serde_json::Value)> = serde_json::from_value(h.state().docs[0].editor.workspace()).unwrap();
    assert_eq!(views.len(), 2);
    views[0].1["snap"] = false.into();
    views[0].1["issues"] = true.into();
    views[1].1["snap"] = true.into();
    views[1].1["issues"] = false.into();
    h.state_mut().docs[0]
        .editor
        .restore_workspace(&serde_json::to_value(&views).unwrap());
    h.run_steps(4);
    let restored: Vec<(u64, serde_json::Value)> = serde_json::from_value(h.state().docs[0].editor.workspace()).unwrap();
    for (id, state) in &views {
        let actual = &restored.iter().find(|(n, _)| n == id).unwrap().1;
        assert_eq!(actual["snap"], state["snap"]);
        assert_eq!(actual["issues"], state["issues"]);
    }
    assert_eq!(h.state().docs[0].snapshot(), original);
}
