use super::*;
use crate::editor::{BinaryImportDescriptor, DiagramRegistry, EditorBackend, ImportDescriptor};
use op_core::document::{DocumentCodec, JsonCodec, LogDocument, ReadError};
use std::sync::{Arc, atomic::AtomicBool};

#[test]
fn binary_imports_do_not_require_utf8_or_change_text_importers() {
    let mut registry = DiagramRegistry::new(
        vec![],
        vec![],
        vec![ImportDescriptor {
            extensions: &["txt"],
            accepts: |text| text == "AB",
            decode: |text| {
                Ok(JsonCodec::new("event-log").encode(&LogDocument {
                    text: text.into(),
                    ..Default::default()
                }))
            },
        }],
    )
    .unwrap();
    let binary = || BinaryImportDescriptor {
        extensions: &["xlsx"],
        accepts: |bytes| bytes.starts_with(&[0xff]),
        decode: |_, source_name| {
            Ok(JsonCodec::new("event-log").encode(&LogDocument {
                name: source_name.unwrap_or_default().into(),
                text: "CD".into(),
                ..Default::default()
            }))
        },
    };
    registry.binary_imports.push(binary());
    assert_eq!(registry.extensions(), ["opx", "json", "txt", "xlsx"]);
    let codec = JsonCodec::<LogDocument>::new("event-log");
    assert_eq!(codec.decode(&registry.load(&[0xff, 0x00]).unwrap()).unwrap().text, "CD");
    assert_eq!(
        codec
            .decode(&registry.load_named("source.xlsx", &[0xff, 0x00]).unwrap())
            .unwrap()
            .name,
        "source.xlsx"
    );
    assert_eq!(codec.decode(&registry.load(b"AB").unwrap()).unwrap().text, "AB");
    registry.binary_imports.push(binary());
    assert!(matches!(registry.load(&[0xff]), Err(ReadError::Invalid(message)) if message.contains("Ambiguous")));
}

#[test]
fn export_rejects_results_computed_with_different_discovery_options() {
    let mut editor = MiningEditor::new(LogDocument {
        text: EXAMPLE_2.into(),
        ..Default::default()
    });
    let input = serde_json::to_string(&analysis_job::Input {
        task: "mining.discovery".into(),
        text: editor.text.clone(),
        lang: op_core::i18n::lang(),
        exam: false,
        discovery: DiscoveryOptions::default(),
    })
    .unwrap();
    let output = analysis_job::compute_json(input, Arc::new(AtomicBool::new(false))).unwrap();
    editor.computed = Some(serde_json::from_str(&output).unwrap());
    editor.computed.as_mut().unwrap().report = "Cached worker write-up".into();
    assert_eq!(
        editor.export().unwrap().bytes,
        b"Cached worker write-up",
        "export uses the worker's cached text"
    );
    editor.discovery.normalize_boundaries = true;
    assert!(editor.export().is_err(), "results from another discovery mode are stale");
}

#[test]
fn workbook_import_selects_the_event_sheet_and_preserves_its_scope_on_reopen() {
    let registry = crate::composition::registry();
    assert!(registry.extensions().contains(&"xlsx"));
    let bytes = include_bytes!("../../../op-core/tests/fixtures/mining/tiny-event-log.xlsx");
    let document = registry.load(bytes).unwrap();
    let codec = JsonCodec::<LogDocument>::new("event-log");
    let log = codec.decode(&document).unwrap();
    assert_eq!(log.import_note, Some(op_core::document::LogImportNote::WorkbookEventLog));
    assert!(log.discovery.normalize_boundaries);
    assert!(log.text.contains("Pat\nRick"), "quoted resources survive the workbook conversion");
    assert_eq!(
        EventLog::parse(&log.text).unwrap().traces,
        [vec!["Register", "Check"], vec!["Ship"]]
    );

    let reopened = registry.load(document.to_json().as_bytes()).unwrap();
    assert_eq!(codec.decode(&reopened).unwrap(), log);
    assert!(
        registry
            .load(include_bytes!("../../../op-core/tests/fixtures/mining/ambiguous-event-logs.xlsx"))
            .is_err()
    );
}

#[test]
fn explicit_mixed_workbook_log_import_opens_a_separate_document() {
    let bytes = include_bytes!("../../../op-core/tests/fixtures/sales/tiny-sales.xlsx");
    assert_eq!(
        crate::composition::registry().load_named("Counter.xlsx", bytes).unwrap().kind,
        "process.sales"
    );
    let original = LogDocument {
        name: "Authored log".into(),
        text: EXAMPLE_2.into(),
        answers: BTreeMap::from([("step:TW".into(), "My answer".into())]),
        ..Default::default()
    };
    let mut editor = MiningEditor::new(original.clone());
    editor.workbooks.push(("/examples/Counter.xlsx".into(), bytes.to_vec()));
    let context = egui::Context::default();
    let mut events = Vec::new();
    let mut output = context.run_ui(Default::default(), |ui| events.extend(EditorBackend::ui(&mut editor, ui, "editor")));
    output.textures_delta.clear();
    let [crate::editor::HostEvent::Open(document)] = events.as_slice() else {
        panic!("the explicit action must open a new document")
    };
    let imported: LogDocument = JsonCodec::new("event-log").decode(document).unwrap();
    assert_eq!(imported.name, "Counter");
    assert_eq!(EventLog::parse(&imported.text).unwrap().traces, [vec!["Check"]]);
    assert!(imported.text.contains("Pat"));
    assert!(imported.discovery.normalize_boundaries);
    assert_eq!(imported.import_note, Some(op_core::document::LogImportNote::WorkbookEventLog));
    assert_eq!(
        editor.log_document(),
        original,
        "the current source and authored answers are preserved"
    );

    editor.workbooks.push(("invalid.xlsx".into(), b"not a workbook".to_vec()));
    editor.poll_workbooks();
    assert!(editor.action.is_none());
    assert!(editor.status.contains("invalid.xlsx"));
    assert_eq!(editor.log_document(), original);
}

#[test]
fn explicit_workbook_selection_opens_every_valid_log_once_and_keeps_authored_work() {
    let bytes = include_bytes!("../../../op-core/tests/fixtures/sales/tiny-sales.xlsx");
    let original = LogDocument {
        name: "Authored log".into(),
        text: EXAMPLE_2.into(),
        answers: BTreeMap::from([("step:TW".into(), "My answer".into())]),
        ..Default::default()
    };
    let mut editor = MiningEditor::new(original.clone());
    editor.workbooks.push(("Counter.xlsx".into(), bytes.to_vec()));
    editor.workbooks.push(("invalid.xlsx".into(), b"not a workbook".to_vec()));
    editor.workbooks.push(("Phone.xlsx".into(), bytes.to_vec()));
    let context = egui::Context::default();
    let mut events = Vec::new();
    let mut output = context.run_ui(Default::default(), |ui| events.extend(EditorBackend::ui(&mut editor, ui, "editor")));
    output.textures_delta.clear();
    let documents: Vec<LogDocument> = events
        .iter()
        .map(|event| match event {
            crate::editor::HostEvent::Open(document) => JsonCodec::new("event-log").decode(document).unwrap(),
            _ => panic!("expected a document-open request"),
        })
        .collect();
    assert_eq!(
        documents.iter().map(|document| document.name.as_str()).collect::<Vec<_>>(),
        ["Counter", "Phone"]
    );
    for document in documents {
        assert_eq!(EventLog::parse(&document.text).unwrap().traces, [vec!["Check"]]);
        assert!(document.discovery.normalize_boundaries);
    }
    assert!(editor.status.contains("invalid.xlsx"));
    assert_eq!(editor.log_document(), original);
    let mut repeated = Vec::new();
    let mut output = context.run_ui(Default::default(), |ui| {
        repeated.extend(EditorBackend::ui(&mut editor, ui, "editor"))
    });
    output.textures_delta.clear();
    assert!(repeated.is_empty(), "import requests are drained after they are emitted");
    assert_eq!(editor.log_document(), original);
}
