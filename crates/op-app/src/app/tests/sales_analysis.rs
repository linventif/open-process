use super::*;
use op_core::document::DocumentCodec;
use op_core::sales::{Money, SalesStudy};

const CSV: &str = "Id;StructureId;Pizza Type;Pizza Size;Revenue;Cost;Satisfaction;Weekday;Customer Location;Customer Type\n1;1;Margherita;Medium;15;6;4;Monday;Centre;Regular\n2;1;Pepperoni;Large;20;8;;Tuesday;North;New\n";

fn wait_for_sales(h: &mut Harness<'static, OpenProcessApp>) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        h.run_steps(2);
        if h.state().docs[h.state().active].editor.export().is_ok() {
            return;
        }
        assert!(std::time::Instant::now() < deadline, "sales analysis timed out");
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
}

#[test]
fn sales_csv_import_analysis_reopen_and_exam_mode() {
    let mut h = harness(false);
    h.state().io.push(("Counter.csv".into(), CSV.as_bytes().to_vec()));
    h.run_steps(4);
    assert_eq!(h.state().docs[h.state().active].editor.kind(), "process.sales");
    wait_for_sales(&mut h);
    assert!(h.query_by_label(tr("sales.average_revenue")).is_some());
    assert!(h.query_by_label(&trf("sales.satisfaction_note", &[&1, &1])).is_some());
    let active = h.state().active;
    let snapshot = h.state().docs[active].snapshot();
    let StoredDocument::Editable(document) = &snapshot else {
        panic!("expected an editable study")
    };
    let study = SalesStudy::CODEC.decode(document).unwrap();
    assert_eq!(study.datasets[0].source_name, "Counter.csv");
    assert_eq!(study.analyze(&Default::default()).unwrap().totals.revenue, Money(3500));
    shot(&mut h, "sales_summary");
    h.get_by_label(tr("sales.breakdown")).click();
    h.run_steps(4);
    assert!(h.query_by_label(tr("sales.group_by")).is_some());
    shot(&mut h, "sales_breakdown");
    h.state_mut().load_bytes("sales.opx".into(), &snapshot.bytes());
    h.run_steps(4);
    assert_eq!(h.state().docs[h.state().active].snapshot(), snapshot);
    wait_for_sales(&mut h);
    toggle_exam(&mut h);
    assert!(h.state().docs[h.state().active].editor.export().is_err());
    assert!(h.query_by_label(tr("sales.average_revenue")).is_none());
}

#[test]
fn sales_location_filter_accepts_spaces_typed_in_separate_frames() {
    let mut h = harness(false);
    let source = CSV.replace("Centre", "London District One");
    h.state().io.push(("Counter.csv".into(), source.into_bytes()));
    h.run_steps(4);
    wait_for_sales(&mut h);
    let original = h.state().docs[h.state().active].snapshot();
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, tr("sales.customer_location"))
        .click();
    h.run_steps(2);
    for character in "London District One".chars() {
        h.event(egui::Event::Text(character.to_string()));
        h.run_steps(2);
    }
    wait_for_sales(&mut h);
    assert!(h.query_by_label(tr("sales.no_matches")).is_none());
    let csv = String::from_utf8(h.state().docs[h.state().active].editor.export().unwrap().bytes).unwrap();
    assert!(
        csv.contains("Counter;1;15.00;6.00;9.00;"),
        "the multiword location selects its one recorded sale: {csv}"
    );
    assert_eq!(
        h.state().docs[h.state().active].snapshot(),
        original,
        "typing a filter does not modify the study"
    );
}
