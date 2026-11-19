//! Shared file packaging; each notation supplies its own typed Mermaid projection.
use super::{ExportData, ExportDescriptor, HostEvent};
use op_core::document::{DocumentCodec, JsonCodec};
use op_core::i18n::{key, trf};
use op_core::mermaid::{Diagram, conversion::Conversion};

pub const MARKDOWN: ExportDescriptor = ExportDescriptor {
    id: "mermaid-markdown",
    label: key("mermaid.export_markdown"),
};
pub const FORMATS: [ExportDescriptor; 2] = [
    ExportDescriptor {
        id: "mermaid",
        label: key("mermaid.export_source"),
    },
    MARKDOWN,
];

pub fn is_projection(format: &str) -> bool {
    matches!(format, "mermaid" | "mermaid-markdown")
}

/// Each adapter supplies its typed projection; the host opens a separate document.
pub fn open(converted: Result<Conversion<Diagram>, String>) -> Vec<HostEvent> {
    match converted {
        Ok(mut converted) => {
            converted.diagram.source = op_core::mermaid::export::source(&converted.diagram, &converted.warnings);
            let mut events = vec![HostEvent::Open(JsonCodec::new("mermaid").encode(&converted.diagram))];
            if !converted.warnings.is_empty() {
                events.push(HostEvent::Message(trf(
                    "conversion.mermaid_warnings",
                    &[&converted.warnings.join(" · ")],
                )));
            }
            events
        }
        Err(error) => vec![HostEvent::Message(error)],
    }
}

pub fn data(format: &str, converted: Conversion<Diagram>) -> Result<ExportData, String> {
    let (suffix, text) = match format {
        "mermaid" => ("mmd", op_core::mermaid::export::source(&converted.diagram, &converted.warnings)),
        "mermaid-markdown" => (
            "mermaid.md",
            op_core::mermaid::markdown::export(&converted.diagram, &converted.warnings),
        ),
        _ => return Err(format!("Unknown Mermaid export: {format}")),
    };
    Ok(ExportData {
        suffix,
        bytes: text.into_bytes(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_core::document::{DocumentCodec, JsonCodec};

    #[test]
    fn registered_mermaid_exports_reopen_without_mutating_models_histories_or_views() {
        let registry = crate::composition::registry();
        for example in [
            "bpmn.order",
            "petri.mutex",
            "epc.order",
            "uml.activity.order",
            "uml.classes",
            "uml.use-cases",
        ] {
            let document = registry.example(example).unwrap();
            let mut editor = registry.open(&document).unwrap();
            let original = editor.snapshot();
            let revisions = editor.revisions();
            let workspace = editor.workspace();
            let standard = editor.export().unwrap();
            let raw = editor.export_as("mermaid").unwrap();
            let md = editor.export_as("mermaid-markdown").unwrap();
            assert_eq!(raw.suffix, "mmd");
            assert_eq!(md.suffix, "mermaid.md");
            let raw = String::from_utf8(raw.bytes).unwrap();
            let markdown = String::from_utf8(md.bytes).unwrap();
            assert_eq!(op_core::mermaid::markdown::source(&markdown).unwrap(), raw);
            for bytes in [raw.as_bytes(), markdown.as_bytes()] {
                let imported = registry.load(bytes).unwrap();
                assert_eq!(imported.kind, "mermaid");
                let imported: Diagram = JsonCodec::new("mermaid").decode(&imported).unwrap();
                assert_eq!(imported.source, raw);
            }
            assert_eq!(editor.snapshot(), original);
            assert_eq!(editor.revisions(), revisions);
            assert_eq!(editor.workspace(), workspace);
            assert_eq!(editor.export().unwrap().bytes, standard.bytes);
            // Generated notation conversions follow the same exam policy as conversion commands.
            editor.set_exam_mode(true);
            assert!(!editor.exports().iter().any(|f| is_projection(f.id)));
            assert!(editor.export_as("mermaid").is_err());
            assert!(editor.export_as("mermaid-markdown").is_err());
            assert_eq!(editor.export().unwrap().bytes, standard.bytes);
            editor.set_exam_mode(false);
            assert!(editor.exports().iter().any(|f| f.id == "mermaid-markdown"));
        }
    }

    #[test]
    fn direct_conversion_commands_preserve_native_documents_and_follow_exam_policy() {
        let registry = crate::composition::registry();
        for (example, command) in [
            ("bpmn.order", "bpmn.convert-to-mermaid"),
            ("petri.mutex", "petri.convert-to-mermaid"),
            ("epc.order", "flow.convert-to-mermaid"),
            ("uml.activity.order", "flow.convert-to-mermaid"),
            ("uml.classes", "uml.convert-to-mermaid"),
            ("uml.use-cases", "uml.convert-to-mermaid"),
        ] {
            let mut editor = registry.open(&registry.example(example).unwrap()).unwrap();
            let original = editor.snapshot();
            let revisions = editor.revisions();
            let workspace = editor.workspace();
            let expected = editor.export_as("mermaid").unwrap().bytes;
            assert!(editor.enabled(command));
            let events = editor.execute(command, 0.0);
            assert_eq!(events.len(), 2, "open a document and report its conversion notes");
            let HostEvent::Open(document) = &events[0] else {
                panic!("missing Mermaid document")
            };
            assert_eq!(document.kind, "mermaid");
            let opened = registry.open(document).unwrap();
            assert_eq!(opened.export().unwrap().bytes, expected);
            assert_eq!(editor.snapshot(), original);
            assert_eq!(editor.revisions(), revisions);
            assert_eq!(editor.workspace(), workspace);
            editor.set_exam_mode(true);
            assert!(!editor.enabled(command));
            assert!(editor.execute(command, 0.0).is_empty());
            assert_eq!(editor.snapshot(), original);
        }
    }

    #[test]
    fn markdown_preserves_authored_mermaid_drafts_even_without_a_valid_header() {
        let registry = crate::composition::registry();
        let draft = Diagram {
            name: "An unfinished draft".into(),
            source: "not valid\n```\n".into(),
        };
        let document = JsonCodec::new("mermaid").encode(&draft);
        let mut editor = registry.open(&document).unwrap();
        editor.set_exam_mode(true);
        let md = editor.export_as("mermaid-markdown").unwrap();
        let restored = registry.load(&md.bytes).unwrap();
        let restored: Diagram = JsonCodec::new("mermaid").decode(&restored).unwrap();
        assert_eq!(restored.source, draft.source);
        assert_eq!(editor.snapshot(), crate::editor::StoredDocument::Editable(document));
        assert!(registry.load(b"```mermaid\nA\n```\n```mermaid\nB\n```\n").is_err());
        assert!(registry.extensions().contains(&"md"));
    }
}
