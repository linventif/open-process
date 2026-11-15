//! Shared file packaging; each notation supplies its own typed Mermaid projection.
use super::{ExportData, ExportDescriptor};
use op_core::i18n::key;
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
