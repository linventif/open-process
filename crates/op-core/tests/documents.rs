//! Captured v1 fixtures predate the migration: never regenerate them with the new writer.
use op_core::document::{Document, DocumentCodec, JsonCodec, Model, ReadError};

const FIXTURES: &[(&str, &str)] = &[
    ("bpmn", include_str!("fixtures/v1/bpmn.opx")),
    ("petri", include_str!("fixtures/v1/petri.opx")),
    ("uml.class", include_str!("fixtures/v1/uml-class.opx")),
    ("uml.use-case", include_str!("fixtures/v1/uml-use-case.opx")),
    ("event-log", include_str!("fixtures/v1/event-log.opx")),
];

#[test]
fn historical_documents_migrate_and_roundtrip_without_losing_payloads() {
    for (kind, fixture) in FIXTURES {
        let original = fixture.as_bytes().to_vec();
        let migrated = Document::read(&original).unwrap();
        assert_eq!(migrated.kind, *kind);
        let old: serde_json::Value = serde_json::from_slice(&original).unwrap();
        assert_eq!(migrated.model, old["model"]["model"]);
        let encoded = migrated.to_json();
        assert_eq!(serde_json::from_str::<serde_json::Value>(&encoded).unwrap()["version"], 2);
        assert_eq!(Document::read(encoded.as_bytes()).unwrap(), migrated);
        assert_eq!(
            op_core::document::load_any(encoded.as_bytes()).unwrap(),
            Model::from_document(&migrated).unwrap()
        );
        assert_eq!(original, fixture.as_bytes());
    }
}

#[test]
fn typed_codec_supports_a_new_kind_without_changing_the_catalogue() {
    let codec = JsonCodec::<Vec<Vec<String>>>::new("decision.table.test");
    let table = vec![vec!["input".into(), "decision".into()], vec!["true".into(), "approve".into()]];
    let document = codec.encode(&table);
    assert_eq!(
        codec.decode(&Document::read(document.to_json().as_bytes()).unwrap()).unwrap(),
        table
    );
    let mut future = document.clone();
    future.model_version = 2;
    assert!(matches!(codec.decode(&future), Err(ReadError::Unsupported(_))));
    future = document;
    future.kind = "another.kind".into();
    assert!(matches!(codec.decode(&future), Err(ReadError::Invalid(_))));
}

#[test]
fn invalid_and_future_envelopes_are_distinguished() {
    for text in [
        r#"{"format":"other","version":1}"#,
        r#"{"format":"open-process"}"#,
        r#"{"format":"open-process","version":2}"#,
        "{broken",
    ] {
        assert!(matches!(Document::read(text.as_bytes()), Err(ReadError::Invalid(_))));
    }
    assert!(matches!(
        Document::read(br#"{"format":"open-process","version":999}"#),
        Err(ReadError::Unsupported(_))
    ));
}

#[test]
fn geometric_changes_preserve_petri_analysis_inputs() {
    let mut net = op_core::examples::mutual_exclusion();
    let semantic = net.analysis_model();
    net.places[0].pos.x += 200.0;
    net.transitions[0].pos.y += 50.0;
    net.style.sketch = true;
    assert_eq!(net.analysis_model(), semantic);
    net.places[0].tokens += 1;
    assert_ne!(net.analysis_model(), semantic);
}

#[test]
fn cancelled_exploration_never_claims_complete_properties() {
    let net = op_core::examples::mutual_exclusion();
    let ss = op_core::analysis::StateSpace::explore_cancellable(&net, 5000, &|| true);
    assert!(ss.truncated);
    assert!(!ss.is_complete());
    assert!(ss.expanded.iter().all(|expanded| !expanded));
}
