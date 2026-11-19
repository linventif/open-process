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
fn historical_logs_keep_classic_discovery_and_new_import_settings_roundtrip() {
    use op_core::document::{LogDocument, LogImportNote};
    let old = Document::read(include_bytes!("fixtures/v1/event-log.opx")).unwrap();
    let codec = JsonCodec::<LogDocument>::new("event-log");
    let mut log = codec.decode(&old).unwrap();
    assert!(!log.discovery.normalize_boundaries);
    assert_eq!(log.import_note, None);
    let classic = codec.encode(&log);
    assert!(classic.model.get("discovery").is_none());
    assert!(classic.model.get("import_note").is_none());

    log.discovery.normalize_boundaries = true;
    log.import_note = Some(LogImportNote::WorkbookEventLog);
    let saved = codec.encode(&log).to_json();
    let reopened = codec.decode(&Document::read(saved.as_bytes()).unwrap()).unwrap();
    assert_eq!(reopened, log);
    assert_eq!(reopened.text, old.model["text"].as_str().unwrap());
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

#[test]
fn built_in_codecs_reject_unsafe_storage_but_keep_editable_drafts() {
    for (kind, fixture) in FIXTURES {
        if *kind == "event-log" {
            continue;
        }
        let original = Document::read(fixture.as_bytes()).unwrap();
        let (nodes, links, endpoint) = match *kind {
            "bpmn" => ("nodes", "flows", "to"),
            "petri" => ("places", "arcs", "to"),
            _ => ("nodes", "relations", "to"),
        };
        let mut broken = original.clone();
        broken.model[nodes][0]["id"] = 0.into();
        assert!(matches!(Model::from_document(&broken), Err(ReadError::Invalid(_))), "{kind}");
        broken = original.clone();
        broken.model[nodes][0]["id"] = if *kind == "bpmn" || *kind == "petri" {
            u32::MAX as u64
        } else {
            u64::MAX
        }
        .into();
        assert!(Model::from_document(&broken).is_err(), "{kind}");
        broken = original.clone();
        if *kind == "petri" {
            broken.model[nodes][0]["pos"]["x"] = 1e30.into();
        } else if *kind == "bpmn" {
            broken.model[nodes][0]["w"] = 0.into();
        } else {
            broken.model[nodes][0]["bounds"]["w"] = 0.into();
        }
        assert!(Model::from_document(&broken).is_err(), "{kind}");
        broken = original.clone();
        if *kind == "petri" {
            broken.model[links][0][endpoint] = serde_json::json!({"Place": 4_000_000_000u32});
        } else {
            broken.model[links][0][endpoint] = 4_000_000_000u32.into();
        }
        assert!(Model::from_document(&broken).is_err(), "{kind}");

        let mut draft = original;
        draft.model[nodes][0]["name"] = "".into();
        assert!(Model::from_document(&draft).is_ok(), "unfinished names remain editable: {kind}");
    }
}

#[test]
fn imported_counters_near_exhaustion_allocate_unused_identities() {
    use op_core::{
        bpmn::{BpmnDiagram, NodeKind, TaskType},
        geom::Pos,
        petri::PetriNet,
    };
    let mut net = PetriNet::new("counter");
    net.add_place(Pos::default(), 0);
    let mut doc = PetriNet::CODEC.encode(&net);
    doc.model["places"][0]["id"] = (u32::MAX - 1).into();
    doc.model["next_id"] = u32::MAX.into();
    let mut net = PetriNet::CODEC.decode(&doc).unwrap();
    let a = net.add_place(Pos::default(), 0);
    let b = net.add_transition(Pos::default());
    assert_ne!(a, b);
    assert!(net.check_storage().is_ok());

    let mut diagram = BpmnDiagram::new("counter");
    diagram.add_node(NodeKind::Task(TaskType::Abstract), Pos::default());
    let mut doc = BpmnDiagram::CODEC.encode(&diagram);
    doc.model["nodes"][0]["id"] = (u32::MAX - 1).into();
    doc.model["next_id"] = u32::MAX.into();
    let mut diagram = BpmnDiagram::CODEC.decode(&doc).unwrap();
    diagram.add_pool(Pos::default());
    assert!(
        diagram.check_storage().is_ok(),
        "a pool and its lane need distinct reserved identities"
    );
}

#[test]
fn files_record_their_writer_and_newer_content_is_detected() {
    use op_core::document::{CONTENT_REVISION, Generator, unsupported_content};
    let encoded = JsonCodec::<Vec<String>>::new("generator.test").encode(&vec!["a".into()]).to_json();
    let generator = Generator::of(encoded.as_bytes()).unwrap();
    assert_eq!(generator, Generator::current());
    assert_eq!(
        (generator.application.as_str(), generator.revision),
        ("Open Process", CONTENT_REVISION)
    );
    assert!(!generator.is_newer());
    // Historical files have no writer.
    for (_, fixture) in FIXTURES {
        assert_eq!(Generator::of(fixture.as_bytes()), None);
    }
    // A newer writer is recognised; unknown envelope and generator fields stay readable.
    let future = r#"{"format":"open-process","version":2,"generator":{"application":"Open Process","version":"v9.0.0","revision":999,"channel":"beta"},"extra":true,"document":{"kind":"bpmn","model_version":1,"model":{}}}"#;
    let newer = Generator::of(future.as_bytes()).unwrap();
    assert!(newer.is_newer());
    assert_eq!(newer.version, "v9.0.0");
    assert_eq!(Document::read(future.as_bytes()).unwrap().kind, "bpmn");
    assert_eq!(Generator::of(b"not json"), None);
    // Content this version drops is listed once per path; null placeholders are not content.
    let original = serde_json::json!({
        "name": "n",
        "nodes": [{"id": 1, "halo": {"r": 2}}, {"id": 2, "halo": null, "glow": 1}],
        "lanes": {"x": 1},
        "hint": null
    });
    let kept = serde_json::json!({"name": "n", "nodes": [{"id": 1}, {"id": 2}], "lanes": {}});
    assert_eq!(unsupported_content(&original, &kept), ["lanes › x", "nodes › glow", "nodes › halo"]);
    assert!(unsupported_content(&kept, &kept).is_empty());
}
