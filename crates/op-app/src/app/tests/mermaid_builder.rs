use super::*;
use op_core::document::{DocumentCodec, JsonCodec};
use op_core::mermaid::{Diagram, TEMPLATES};

fn show_builder(h: &mut Harness<'static, OpenProcessApp>, width: f32) {
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
    h.get_by_label("Builder").click();
    h.run_steps(4);
}

#[test]
fn builder_and_source_share_exact_content_history_and_native_storage_at_responsive_widths() {
    for width in [1400.0, 420.0, 280.0] {
        let mut h = harness_sized(false, [width, 860.0]);
        let original = Diagram {
            name: "Source preservation".into(),
            source: "---\ntitle: Example\n---\n%% keep comment\nflowchart LR\n    A[\"Original\"]\n    A --> B\n    classDef special fill:#def\n".into(),
        };
        let document = JsonCodec::new("mermaid").encode(&original);
        h.state_mut().open_document(document.clone(), None, false);
        h.run_steps(4);
        let active = h.state().active;
        let before = h.state().docs[active].snapshot();
        let revisions = h.state().docs[active].editor.revisions();
        show_builder(&mut h, width);
        assert_eq!(h.state().docs[active].snapshot(), before);
        assert_eq!(h.state().docs[active].editor.revisions(), revisions);
        h.get_by_label("Source").click();
        h.run_steps(4);
        assert_eq!(h.state().docs[active].editor.export().unwrap().bytes, original.source.as_bytes());
        h.get_by_label("Builder").click();
        h.run_steps(4);
        h.get_by_label("Add element").scroll_to_me();
        h.run_steps(12);
        h.get_by_label("Add element").click();
        h.run_steps(4);
        let source = String::from_utf8(h.state().docs[active].editor.export().unwrap().bytes).unwrap();
        assert!(source.starts_with(&original.source));
        assert!(source.contains("N1[\"New node\"]"));
        let edited = h.state().docs[active].snapshot();
        h.state_mut().docs[active].editor.execute(UNDO, 0.0);
        h.run_steps(4);
        assert_eq!(h.state().docs[active].snapshot(), before);
        h.state_mut().docs[active].editor.execute(REDO, 0.0);
        h.run_steps(4);
        assert_eq!(h.state().docs[active].snapshot(), edited);
        let StoredDocument::Editable(saved) = edited else { unreachable!() };
        let saved = op_core::document::Document::read(saved.to_json().as_bytes()).unwrap();
        let mut reopened = crate::composition::registry().open(&saved).unwrap();
        reopened.restore_workspace(&h.state().docs[active].editor.workspace());
        assert_eq!(reopened.export().unwrap().bytes, source.as_bytes());
        shot(&mut h, &format!("mermaid_builder_{width}"));
    }
}

#[test]
fn every_family_opens_with_its_builder_and_switching_preserves_source_and_exports() {
    let mut h = harness(false);
    for template in TEMPLATES {
        h.state_mut().open_example(template.id);
        h.run_steps(4);
        let active = h.state().active;
        let original = h.state().docs[active].snapshot();
        let bytes = h.state().docs[active].editor.export().unwrap().bytes;
        h.get_by_label("Builder").click();
        h.run_steps(4);
        assert!(h.query_by_label("Add element").is_some(), "{}", template.id);
        h.get_by_label("Source").click();
        h.run_steps(4);
        assert_eq!(h.state().docs[active].snapshot(), original);
        assert_eq!(h.state().docs[active].editor.export().unwrap().bytes, bytes);
        let markdown = h.state().docs[active].editor.export_as("mermaid-markdown").unwrap();
        let document = crate::composition::registry().load(&markdown.bytes).unwrap();
        let diagram: Diagram = JsonCodec::new("mermaid").decode(&document).unwrap();
        assert_eq!(diagram.source.as_bytes(), bytes);
    }
}

#[test]
fn applying_and_removing_an_element_are_explicit_single_history_steps() {
    let mut h = harness(false);
    let diagram = Diagram {
        name: "Edit".into(),
        source: "flowchart LR\n    %% keep\n    A[\"Original\"]\n    A --> B\n".into(),
    };
    h.state_mut().open_document(JsonCodec::new("mermaid").encode(&diagram), None, false);
    h.run_steps(4);
    let active = h.state().active;
    let original = h.state().docs[active].snapshot();
    show_builder(&mut h, 1400.0);
    h.get_by_label("Line 3 · Node").click();
    h.run_steps(3);
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Text").click();
    h.run_steps(3);
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    h.run_steps(3);
    h.event(egui::Event::Paste("Revised <idea>".into()));
    h.run_steps(4);
    assert_eq!(
        h.state().docs[active].snapshot(),
        original,
        "unapplied form input must not change the document"
    );
    h.get_by_label("Apply changes").click();
    h.run_steps(4);
    let edited = h.state().docs[active].snapshot();
    assert_ne!(edited, original);
    assert_eq!(
        String::from_utf8(h.state().docs[active].editor.export().unwrap().bytes).unwrap(),
        "flowchart LR\n    %% keep\n    A[\"Revised #60;idea#62;\"]\n    A --> B\n"
    );
    h.state_mut().docs[active].editor.execute(UNDO, 0.0);
    h.run_steps(4);
    assert_eq!(h.state().docs[active].snapshot(), original);
    h.state_mut().docs[active].editor.execute(REDO, 0.0);
    h.run_steps(4);
    assert_eq!(h.state().docs[active].snapshot(), edited);
    h.get_by_label("Line 3 · Node").click();
    h.run_steps(3);
    h.get_by_label("Remove statement").click();
    h.run_steps(4);
    assert_eq!(
        h.state().docs[active].editor.export().unwrap().bytes,
        b"flowchart LR\n    %% keep\n    A --> B\n"
    );
    h.state_mut().docs[active].editor.execute(UNDO, 0.0);
    h.run_steps(4);
    assert_eq!(h.state().docs[active].snapshot(), edited);
}

#[test]
fn native_builder_tools_open_separate_mermaid_documents_with_form_editing() {
    let mut h = harness(false);
    for example in [
        "bpmn.order",
        "petri.mutex",
        "epc.order",
        "uml.activity.order",
        "uml.classes",
        "uml.use-cases",
    ] {
        h.state_mut().open_example(example);
        h.run_steps(4);
        let native = h.state().active;
        let original = h.state().docs[native].snapshot();
        let count = h.state().docs.len();
        h.get_by_label("Tools").click();
        h.run_steps(3);
        h.get_by_label("Convert to Mermaid").click();
        h.run_steps(4);
        assert_eq!(h.state().docs.len(), count + 1);
        assert_eq!(h.state().docs[native].snapshot(), original);
        assert_eq!(h.state().docs[h.state().active].editor.kind(), "mermaid");
        h.get_by_label("Builder").click();
        h.run_steps(4);
        assert!(h.query_by_label("Add element").is_some());
    }
}

#[test]
fn the_last_mermaid_family_is_reachable_from_the_home_example_menu() {
    let mut h = Harness::builder()
        .with_size([1400.0, 860.0])
        .build_eframe(|cc| OpenProcessApp::new(cc, Vec::new(), crate::composition::registry()));
    h.run_steps(4);
    h.get_by_label("Examples for Create with Mermaid").scroll_to_me();
    h.run_steps(12);
    h.get_by_label("Examples for Create with Mermaid").click();
    h.run_steps(4);
    h.get_by_label("Tree view").scroll_to_me();
    h.run_steps(12);
    h.get_by_label("Tree view").click();
    h.run_steps(4);
    assert_eq!(h.state().docs.len(), 1);
    let editor = &h.state().docs[0].editor;
    assert_eq!(editor.kind(), "mermaid");
    assert_eq!(editor.export().unwrap().bytes, TEMPLATES.last().unwrap().source.as_bytes());
    assert!(editor.export_as("svg").is_err());
    assert!(h.query_by_label("Preview unavailable for this family").is_some());
}

#[test]
fn mermaid_flowcharts_open_a_real_native_builder_and_convert_back_without_mutating_the_source() {
    let mut h = harness(false);
    h.state_mut().open_example("mermaid.flowchart");
    h.run_steps(4);
    let source_index = h.state().active;
    let source = h.state().docs[source_index].snapshot();
    h.get_by_label("Convert to flowchart builder").click();
    h.run_steps(4);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while h.state().docs[h.state().active].editor.kind() == "mermaid" {
        assert!(std::time::Instant::now() < deadline, "flowchart conversion timed out");
        std::thread::sleep(std::time::Duration::from_millis(1));
        h.run_steps(3);
    }
    assert_eq!(h.state().docs[h.state().active].editor.kind(), "flowchart");
    assert_eq!(h.state().docs[source_index].snapshot(), source);
    let native_index = h.state().active;
    let native = h.state().docs[native_index].snapshot();
    let StoredDocument::Editable(document) = &native else {
        unreachable!()
    };
    let model: op_core::flowchart::FlowchartDiagram = JsonCodec::new("flowchart").decode(document).unwrap();
    assert_eq!(model.nodes.len(), 5);
    assert_eq!(model.flows.len(), 5);
    h.get_by_label("Tools").click();
    h.run_steps(3);
    h.get_by_label("Convert to Mermaid").click();
    h.run_steps(4);
    assert_eq!(h.state().docs[h.state().active].editor.kind(), "mermaid");
    assert_eq!(h.state().docs[native_index].snapshot(), native);
    assert_eq!(h.state().docs[source_index].snapshot(), source);
    let source = Diagram {
        name: "Restored".into(),
        source: String::from_utf8(h.state().docs[h.state().active].editor.export().unwrap().bytes).unwrap(),
    };
    let restored = op_core::flowchart::from_mermaid(&source).unwrap().diagram;
    assert_eq!(
        model.nodes.iter().map(|n| (&n.name, n.kind)).collect::<Vec<_>>(),
        restored.nodes.iter().map(|n| (&n.name, n.kind)).collect::<Vec<_>>()
    );
    assert_eq!(model.flows, restored.flows);
}
