use super::*;
use op_core::document::{DocumentCodec, JsonCodec};

fn future_file() -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_str(&JsonCodec::new("bpmn").encode(&op_core::examples::order_process()).to_json()).unwrap();
    value["generator"]["version"] = "v9.0.0".into();
    value["generator"]["revision"] = 999.into();
    value["document"]["model"]["nodes"][0]["halo"] = serde_json::json!({"radius": 3});
    serde_json::to_vec(&value).unwrap()
}

#[test]
fn newer_files_list_unsupported_content_before_opening() {
    let mut h = harness_sized(false, [1200.0, 700.0]);
    let open = h.state().docs.len();

    h.state_mut().load_bytes("future.opx".into(), &future_file());
    h.run_steps(3);
    assert_eq!(h.state().docs.len(), open, "nothing opens before the person chooses");
    assert!(h.query_by_label_contains("v9.0.0").is_some());
    assert!(h.query_by_label("• nodes › halo").is_some());
    shot(&mut h, "newer_file_prompt");
    h.get_by_label("Cancel").click();
    h.run_steps(3);
    assert_eq!(h.state().docs.len(), open);
    assert!(h.state().newer_files.is_empty());

    // Several files queue one prompt each; opening keeps what this version reads.
    h.state_mut().load_bytes("first.opx".into(), &future_file());
    h.state_mut().load_bytes("second.opx".into(), &future_file());
    h.run_steps(3);
    h.get_by_label("Open anyway").click();
    h.run_steps(3);
    assert_eq!(h.state().docs.len(), open + 1);
    assert_eq!(h.state().newer_files.len(), 1);
    h.get_by_label("Open anyway").click();
    h.run_steps(3);
    assert_eq!(h.state().docs.len(), open + 2);
    assert!(h.state().docs.last().unwrap().editor.as_any().is::<BpmnEditor>());

    // Files from this version open directly and record their writer.
    let current = JsonCodec::new("bpmn").encode(&op_core::examples::order_process()).to_json();
    assert_eq!(
        op_core::document::Generator::of(current.as_bytes()),
        Some(op_core::document::Generator::current())
    );
    h.state_mut().load_bytes("current.opx".into(), current.as_bytes());
    h.run_steps(3);
    assert_eq!(h.state().docs.len(), open + 3);
    assert!(h.state().newer_files.is_empty());
}
