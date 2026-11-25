//! Exercise mode of the event-log editor: generation, step-by-step answers,
//! undo, saved view state and examination isolation.
use super::*;
use op_core::document::{DocumentCodec, JsonCodec};
use op_core::mining::{DiscoveryOptions, EventLog};

fn saved_log(h: &Harness<'static, OpenProcessApp>) -> LogDocument {
    let StoredDocument::Editable(document) = h.state().docs[h.state().active].snapshot() else {
        panic!("expected an editable log")
    };
    JsonCodec::new("event-log").decode(&document).unwrap()
}

fn open_log(h: &mut Harness<'static, OpenProcessApp>, log: LogDocument) {
    h.state_mut().open_model(Model::Log(log), Some("practice.opx".into()), true);
    h.run_steps(4);
    wait_for_discovery(h);
}

fn shown(h: &Harness<'static, OpenProcessApp>, revealed: usize) -> usize {
    h.query_all_by_label(&format!("{revealed} of 11 steps shown")).count()
}

/// Scrolls the control into view first, as a user would on a long walkthrough.
fn press_label(h: &mut Harness<'static, OpenProcessApp>, label: &str) {
    h.get_by_label(label).scroll_to_me();
    h.run_steps(3);
    h.get_by_label(label).click();
    h.run_steps(4);
}

#[test]
fn generated_exercises_reveal_alpha_step_by_step_and_undo_restores_the_log() {
    let mut h = harness_sized(false, [1400.0, 1000.0]);
    let original = LogDocument {
        name: "Practice".into(),
        text: crate::mining_editor::EXAMPLE_2.into(),
        answers: std::collections::BTreeMap::from([("step:TI".into(), "{A}".into())]),
        discovery: DiscoveryOptions {
            normalize_boundaries: true,
        },
        ..Default::default()
    };
    open_log(&mut h, original.clone());
    assert_eq!(h.query_all_by_label("Open Petri net").count(), 2, "full results by default");
    assert!(h.query_by_label("Show answer").is_none());

    h.get_by_role(egui::accesskit::Role::ComboBox).click();
    h.run_steps(3);
    press_label(&mut h, "Medium");
    press_label(&mut h, "New exercise");
    wait_for_discovery(&mut h);

    let generated = saved_log(&h);
    assert_ne!(generated.text, original.text);
    assert!(generated.answers.is_empty(), "answers of the previous log are cleared");
    assert_eq!(generated.discovery, DiscoveryOptions::default(), "exercises use classic Alpha");
    assert_eq!(generated.name, "Practice");
    let log = EventLog::parse(&generated.text).unwrap();
    let r = op_core::mining::alpha(&log);
    assert!((6..=8).contains(&r.footprint.len()), "a medium exercise: {}", generated.text);
    assert!(log.traces.iter().all(|t| op_core::mining::replay(&r.net, t).is_ok()));

    // Only the statement and the first question are visible.
    assert_eq!(shown(&h, 0), 2, "progress in the header and the status bar");
    assert!(h.query_by_label("Statement").is_some());
    assert!(h.query_by_label("Direct successions  x>y").is_some());
    assert!(h.query_by_label("Relations between activities").is_none());
    assert!(
        h.query_by_label("Open Petri net").is_none(),
        "no solution shortcut while answers are hidden"
    );
    assert!(h.query_by_label("Explanation").is_none());
    shot(&mut h, "mining_exercise_question");

    press_label(&mut h, "Show answer");
    assert_eq!(shown(&h, 1), 2);
    assert!(h.query_by_label("Explanation").is_some(), "per-trace working of the successions");
    assert!(h.query_by_label("Relations between activities").is_some(), "next question");
    press_label(&mut h, "Next step ▶");
    assert_eq!(shown(&h, 2), 2);
    press_label(&mut h, "◀ Previous");
    assert_eq!(shown(&h, 1), 2);

    press_label(&mut h, "Show all");
    assert_eq!(shown(&h, 11), 2);
    assert!(h.query_by_label("Show answer").is_none());
    assert!(h.query_by_label("✔ All steps revealed").is_some());
    assert!(h.query_by_label("Open Petri net").is_some(), "the last step opens α(W)");
    shot(&mut h, "mining_exercise_solution");
    h.get_by_label("YW — maximal pairs").scroll_to_me();
    shot(&mut h, "mining_exercise_maximal_pairs");
    h.get_by_label("Open Petri net").scroll_to_me();
    shot(&mut h, "mining_exercise_net");
    let documents = h.state().docs.len();
    press_label(&mut h, "Open Petri net");
    assert_eq!(h.state().docs.len(), documents + 1);
    h.state_mut().set_active(documents - 1);
    h.run_steps(4);

    press_label(&mut h, "Hide answers");
    assert_eq!(shown(&h, 0), 2);
    assert_eq!(saved_log(&h), generated, "revealing answers does not edit the document");

    h.state_mut().execute(UNDO, 0.0);
    h.run_steps(4);
    assert_eq!(saved_log(&h), original, "one undo restores the log, answers and options");

    press_label(&mut h, "Reveal answers step by step");
    wait_for_discovery(&mut h);
    assert_eq!(
        h.query_all_by_label("Open Petri net").count(),
        2,
        "unchecking shows every result again, with the commands in the header and under the word check"
    );
    // The commands under the word check work like those of the exercise's last step.
    h.query_all_by_label("Deduce BPMN").nth(1).unwrap().scroll_to_me();
    shot(&mut h, "mining_full_results_bottom");
    let documents = h.state().docs.len();
    h.query_all_by_label("Deduce BPMN").nth(1).unwrap().click();
    h.run_steps(4);
    assert_eq!(h.state().docs.len(), documents + 1);
    assert!(h.state().docs.last().unwrap().editor.as_any().is::<BpmnEditor>());
}

#[test]
fn normalized_log_walkthrough_explains_generated_boundaries_without_changing_the_source() {
    for (text, trace, start, end, successions) in [
        (
            "ABC\nADC",
            "⟨[start], A, B, C, [end]⟩",
            "[start]",
            "[end]",
            "[start]>A, A>B, B>C, C>[end]",
        ),
        (
            "[start] -> A -> [end]",
            "⟨[start] (2), [start], A, [end], [end] (2)⟩",
            "[start] (2)",
            "[end] (2)",
            "[start] (2)>[start], [start]>A, A>[end], [end]>[end] (2)",
        ),
    ] {
        let mut h = harness(false);
        let original = LogDocument {
            name: "Normalized practice".into(),
            text: text.into(),
            discovery: DiscoveryOptions {
                normalize_boundaries: true,
            },
            ..Default::default()
        };
        open_log(&mut h, original.clone());
        press_label(&mut h, "Reveal answers step by step");
        press_label(&mut h, "Show all");

        assert!(h.query_by_label(tr("mining.report.normalized_boundaries")).is_some());
        for explanation in [
            format!("• {trace}: {successions}"),
            format!("• {trace} → {start}"),
            format!("• {trace} → {end}"),
        ] {
            assert!(
                h.query_by_label(&explanation).is_some(),
                "missing normalized explanation: {explanation}"
            );
        }
        assert!(
            h.query_by_label(&format!("{{{start}}}")).is_some(),
            "TI uses the same generated start"
        );
        assert!(h.query_by_label(&format!("{{{end}}}")).is_some(), "TO uses the same generated end");
        assert_eq!(saved_log(&h), original, "the walkthrough preserves the original log and options");
        assert!(!h.state().docs[h.state().active].dirty());
    }
}

#[test]
fn exercise_view_state_is_saved_with_the_workspace_and_older_workspaces_open_unchanged() {
    let mut h = harness(false);
    open_log(
        &mut h,
        LogDocument {
            name: "Saved view".into(),
            text: crate::mining_editor::EXAMPLE_2.into(),
            ..Default::default()
        },
    );
    let index = h.state().active;
    assert!(!h.state().docs[index].dirty());
    press_label(&mut h, "Reveal answers step by step");
    press_label(&mut h, "Show answer");
    let workspace = h.state().docs[index].editor.workspace();
    let views: Vec<(u64, serde_json::Value)> = serde_json::from_value(workspace.clone()).unwrap();
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].1["active"], true);
    assert_eq!(views[0].1["revealed"], 1);
    assert_eq!(views[0].1["difficulty"], "Easy");
    assert!(!h.state().docs[index].dirty(), "view state is not document content");

    let older = serde_json::json!([[views[0].0, null]]);
    h.state_mut().docs[index].editor.restore_workspace(&older);
    h.run_steps(4);
    assert_eq!(
        h.query_all_by_label("Open Petri net").count(),
        2,
        "a null view opens the full results"
    );
    h.state_mut().docs[index].editor.restore_workspace(&workspace);
    h.run_steps(4);
    assert_eq!(shown(&h, 1), 2, "the saved view resumes the exercise");
}

#[test]
fn exam_mode_hides_exercise_controls_and_answers() {
    let mut h = harness(false);
    open_log(
        &mut h,
        LogDocument {
            name: "Exam".into(),
            text: crate::mining_editor::EXAMPLE_2.into(),
            ..Default::default()
        },
    );
    press_label(&mut h, "New exercise");
    wait_for_discovery(&mut h);
    press_label(&mut h, "Show all");
    let generated = saved_log(&h);
    toggle_exam(&mut h);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while h.query_by_label("step:TI").is_none() {
        h.run_steps(2);
        assert!(std::time::Instant::now() < deadline, "exam inputs timed out");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    for label in [
        "New exercise",
        "Reveal answers step by step",
        "Statement",
        "Show answer",
        "Show all",
        "Hide answers",
        "Next step ▶",
        "◀ Previous",
        "11 of 11 steps shown",
        "Explanation",
        "Open Petri net",
    ] {
        assert!(h.query_by_label(label).is_none(), "exam leaked {label}");
    }
    assert_eq!(saved_log(&h), generated, "the exercise log stays available as the exam subject");
    toggle_exam(&mut h);
    wait_for_discovery(&mut h);
    assert_eq!(shown(&h, 11), 2, "the exercise resumes after the exam");
}
