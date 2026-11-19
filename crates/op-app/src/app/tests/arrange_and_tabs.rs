use super::*;

#[test]
fn graph_notations_auto_arrange_in_one_undoable_step() {
    for example in [
        "bpmn.order",
        "petri.philosophers",
        "epc.order",
        "uml.activity.order",
        "flowchart.order",
        "uml.classes",
        "uml.use-cases",
        "database.shop",
    ] {
        let mut h = harness_sized(false, [1400.0, 860.0]);
        h.state_mut().open_example(example);
        h.run_steps(4);
        let i = h.state().active;
        assert!(h.state().docs[i].editor.enabled(ARRANGE), "{example}");
        let original = h.state().docs[i].snapshot();
        let semantic = h.state().docs[i].editor.revisions().semantic;
        h.get_by_label("Auto arrange").click();
        h.run_steps(5);
        let arranged = h.state().docs[i].snapshot();
        assert_ne!(arranged, original, "{example} changed no position");
        assert_eq!(h.state().docs[i].editor.revisions().semantic, semantic, "{example}");
        shot(&mut h, &format!("auto_arranged_{example}"));
        h.state_mut().docs[i].editor.execute(UNDO, 0.0);
        h.run_steps(4);
        assert_eq!(h.state().docs[i].snapshot(), original, "{example} undo");
        h.state_mut().docs[i].editor.execute(REDO, 0.0);
        h.run_steps(4);
        assert_eq!(h.state().docs[i].snapshot(), arranged, "{example} redo");
    }
}

fn names(h: &Harness<'static, OpenProcessApp>) -> Vec<String> {
    h.state().docs.iter().map(|d| d.name()).collect()
}

fn tab_menu(h: &mut Harness<'static, OpenProcessApp>, tab: &str, item: &str) {
    h.get_by_label_contains(tab).click_secondary();
    h.run_steps(3);
    h.get_by_label(item).click();
    h.run_steps(4);
}

#[test]
fn bulk_tab_close_follows_tab_order_and_pauses_on_unsaved_changes() {
    let mut h = harness_sized(false, [1200.0, 600.0]);
    for name in ["One", "Two", "Three"] {
        h.state_mut().open_model(Model::Bpmn(BpmnDiagram::new(name)), None, false);
    }
    h.run_steps(4);
    let initial = names(&h);
    assert_eq!(initial.len(), 4);
    assert!(h.state().docs[0].dirty() && h.state().docs[0].editor.has_content());

    h.get_by_label_contains("Two").click_secondary();
    h.run_steps(3);
    shot(&mut h, "tab_close_menu");
    h.get_by_label("Close tabs to the right").click();
    h.run_steps(4);
    assert_eq!(names(&h), initial[..3]);

    // The unsaved first tab asks first; cancelling keeps every remaining tab.
    tab_menu(&mut h, "Two", "Close tabs to the left");
    assert_eq!(h.state().confirm_close, Some(0));
    h.get_by_label("Cancel").click();
    h.run_steps(4);
    assert_eq!(h.state().confirm_close, None);
    assert_eq!(names(&h), initial[..3]);

    // Answering the prompt resumes the queue in tab order.
    tab_menu(&mut h, "Two", "Close tabs to the left");
    h.get_by_label("Close without saving").click();
    h.run_steps(4);
    assert_eq!(names(&h), [initial[2].clone()]);

    tab_menu(&mut h, "Two", "Close all tabs");
    assert!(h.state().docs.is_empty());
}
