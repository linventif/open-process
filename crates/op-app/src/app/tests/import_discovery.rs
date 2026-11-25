use super::*;
use op_core::document::{DocumentCodec, JsonCodec, LogImportNote};

fn saved_log(h: &Harness<'static, OpenProcessApp>) -> LogDocument {
    let StoredDocument::Editable(document) = h.state().docs[h.state().active].snapshot() else {
        panic!("expected an editable log")
    };
    JsonCodec::new("event-log").decode(&document).unwrap()
}

#[test]
fn padded_csv_import_discovers_an_editable_net_and_round_trips() {
    let mut source = String::from("CaseId;Activity;Actor");
    let padding = ";".repeat(16_381);
    source.push_str(&padding);
    source.push('\n');
    for case in 1..=100 {
        for activity in ["Receive", "Prepare"] {
            source.push_str(&format!("{case};{activity};Operator{padding}\n"));
        }
    }
    assert!(
        source.len() > 2 * 1024 * 1024,
        "exercise preparation before the bounded worker transport"
    );

    let mut h = harness(false);
    h.state().io.push(("orders.csv".into(), source.into_bytes()));
    h.run_steps(4);
    wait_for_discovery(&mut h);
    let log = saved_log(&h);
    assert!(log.text.len() < 10_000);
    assert!(log.text.contains("Operator"), "nonempty resource data is preserved");
    assert_eq!(log.import_note, Some(LogImportNote::EmptyColumnsRemoved));
    assert!(log.discovery.normalize_boundaries);
    assert_eq!(h.state().docs[h.state().active].name(), "orders");

    h.get_by_label(tr("mining.normalize_boundaries")).click();
    h.run_steps(4);
    assert!(!saved_log(&h).discovery.normalize_boundaries);
    let active = h.state().active;
    h.state_mut().docs[active].editor.execute(UNDO, 0.0);
    h.run_steps(4);
    assert!(
        saved_log(&h).discovery.normalize_boundaries,
        "the mode participates in document history"
    );
    wait_for_discovery(&mut h);

    let log_bytes = h.state().docs[active].snapshot().bytes();
    h.state_mut().load_bytes("orders.opx".into(), &log_bytes);
    h.run_steps(4);
    assert_eq!(saved_log(&h), log);
    wait_for_discovery(&mut h);
    h.query_all_by_label("Open Petri net").next().unwrap().click();
    h.run_steps(4);
    let active = h.state().active;
    let editor = h.state_mut().docs[active]
        .editor
        .as_any_mut()
        .downcast_mut::<PetriEditor>()
        .unwrap();
    assert!(editor.net.transitions.iter().any(|transition| transition.silent));
    for trace in op_core::mining::EventLog::parse(&log.text).unwrap().traces {
        op_core::mining::replay(&editor.net, &trace).unwrap();
    }
    editor
        .net
        .transitions
        .iter_mut()
        .find(|transition| !transition.silent)
        .unwrap()
        .name = "Edited activity".into();
    h.run_steps(4);
    let edited = h.state().docs[active].snapshot();
    h.state_mut().load_bytes("discovered.opx".into(), &edited.bytes());
    h.run_steps(4);
    assert_eq!(h.state().docs[h.state().active].snapshot(), edited);
    shot(&mut h, "imported_csv_petri");
}
