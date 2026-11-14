use super::*;
use op_core::document::{Document, ReadError};

pub struct DiagramDescriptor {
    pub id: &'static str,
    pub aliases: &'static [&'static str],
    pub home_order: usize,
    pub new_label: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub badge: &'static str,
    pub color: egui::Color32,
    pub icon: crate::icons::Icon,
    pub create: fn(usize) -> Document,
    pub open: fn(&Document) -> Result<Box<dyn EditorSession>, ReadError>,
}

pub struct ExampleDescriptor {
    pub id: &'static str,
    /// Canonical diagram identifier used to group examples on their type card.
    pub kind: &'static str,
    pub label: &'static str,
    pub create: fn() -> Document,
}

pub struct WorkflowDescriptor {
    pub home_order: usize,
    pub title: &'static str,
    pub description: &'static str,
    pub badge: &'static str,
    pub color: egui::Color32,
    pub icon: crate::icons::Icon,
    pub example: &'static str,
    pub command: Option<&'static str>,
}

pub struct ImportDescriptor {
    pub extensions: &'static [&'static str],
    pub accepts: fn(&str) -> bool,
    pub decode: fn(&str) -> Result<Document, String>,
}

pub struct DiagramRegistry {
    pub diagrams: Vec<DiagramDescriptor>,
    pub examples: Vec<ExampleDescriptor>,
    pub imports: Vec<ImportDescriptor>,
    pub workflows: Vec<WorkflowDescriptor>,
}

impl DiagramRegistry {
    pub fn new(diagrams: Vec<DiagramDescriptor>, examples: Vec<ExampleDescriptor>, imports: Vec<ImportDescriptor>) -> Result<Self, String> {
        let registry = Self {
            diagrams,
            examples,
            imports,
            workflows: vec![],
        };
        registry.validate()?;
        Ok(registry)
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut names = std::collections::HashSet::new();
        for d in &self.diagrams {
            for id in std::iter::once(d.id).chain(d.aliases.iter().copied()) {
                if !names.insert(id) {
                    return Err(format!("Duplicate document identifier: {id}"));
                }
            }
        }
        let mut ids = std::collections::HashSet::new();
        for e in &self.examples {
            if !ids.insert(e.id) {
                return Err(format!("Duplicate example identifier: {}", e.id));
            }
            if !self.diagrams.iter().any(|d| d.id == e.kind) {
                return Err(format!("Unknown diagram kind for example {}: {}", e.id, e.kind));
            }
        }
        Ok(())
    }
    pub fn find(&self, kind: &str) -> Option<&DiagramDescriptor> {
        self.diagrams.iter().find(|d| d.id == kind || d.aliases.contains(&kind))
    }
    pub fn open(&self, document: &Document) -> Result<Box<dyn EditorSession>, ReadError> {
        let d = self
            .find(&document.kind)
            .ok_or_else(|| ReadError::Unsupported(format!("Unknown document kind: {}", document.kind)))?;
        (d.open)(document)
    }
    pub fn example(&self, id: &str) -> Option<Document> {
        self.examples.iter().find(|e| e.id == id).map(|e| (e.create)())
    }
    pub fn load(&self, bytes: &[u8]) -> Result<Document, ReadError> {
        if bytes.len() > op_core::document::MAX_DOCUMENT_BYTES {
            return Err(ReadError::Invalid(op_core::i18n::tr("file.input_limit").into()));
        }
        let text = std::str::from_utf8(bytes).map_err(|e| ReadError::Invalid(e.to_string()))?;
        let text = text.trim_start_matches('\u{feff}').trim_start();
        if text.starts_with('{') {
            return Document::read(bytes);
        }
        let candidates: Vec<_> = self.imports.iter().filter(|i| (i.accepts)(text)).collect();
        match candidates.as_slice() {
            [import] => (import.decode)(text).map_err(ReadError::Invalid),
            [] => Err(ReadError::Invalid("Unrecognized file format".into())),
            _ => Err(ReadError::Invalid("Ambiguous file format".into())),
        }
    }
    pub fn extensions(&self) -> Vec<&'static str> {
        let mut result = vec!["opx", "json"];
        for ext in self.imports.iter().flat_map(|i| i.extensions) {
            if !result.contains(ext) {
                result.push(ext);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use op_core::document::{DocumentCodec, JsonCodec};
    struct Table(Vec<Vec<String>>);
    impl EditorBackend for Table {
        type ViewState = ();
        fn capture_view(&self) {}
        fn restore_view(&mut self, _: &()) {}
        fn fresh_view(&self) {}
        fn kind(&self) -> &'static str {
            "test.decision-table"
        }
        fn name(&self) -> &str {
            "Decision table"
        }
        fn rename(&mut self, _: String) {}
        fn document(&self) -> Document {
            JsonCodec::new("test.decision-table").encode(&self.0)
        }
        fn ui(&mut self, ui: &mut egui::Ui, _: &str) -> Vec<HostEvent> {
            ui.label("Decision table");
            vec![]
        }
        fn commands(&self) -> &'static [CommandDescriptor] {
            &[]
        }
        fn enabled(&self, _: &str) -> bool {
            false
        }
        fn execute(&mut self, _: &str, _: f64) -> Vec<HostEvent> {
            vec![]
        }
        fn export(&self) -> Result<ExportData, String> {
            Ok(ExportData {
                suffix: "json",
                bytes: self.document().to_json().into_bytes(),
            })
        }
        fn export_label(&self) -> &'static str {
            "Export"
        }
        fn exports(&self) -> Vec<ExportDescriptor> {
            vec![
                ExportDescriptor { id: "json", label: "JSON" },
                ExportDescriptor { id: "csv", label: "CSV" },
            ]
        }
        fn export_as(&self, format: &str) -> Result<ExportData, String> {
            match format {
                "json" => self.export(),
                "csv" => Ok(ExportData {
                    suffix: "csv",
                    bytes: self.0.iter().map(|r| r.join(",")).collect::<Vec<_>>().join("\n").into_bytes(),
                }),
                _ => Err("unknown format".into()),
            }
        }
    }
    fn descriptor() -> DiagramDescriptor {
        DiagramDescriptor {
            id: "test.decision-table",
            aliases: &["table"],
            home_order: 0,
            new_label: "New table",
            title: "Table",
            description: "Decision table",
            badge: "TEST",
            color: egui::Color32::GRAY,
            icon: crate::icons::decision_table,
            create: |_| JsonCodec::new("test.decision-table").encode(&vec![vec!["true".to_string(), "approve".to_string()]]),
            open: |d| Ok(Session::boxed(Table(JsonCodec::new("test.decision-table").decode(d)?))),
        }
    }
    #[test]
    fn extension_without_canvas_uses_the_same_document_shell() {
        let registry = Box::leak(Box::new(DiagramRegistry::new(vec![descriptor()], vec![], vec![]).unwrap()));
        let document = (registry.find("table").unwrap().create)(1);
        let doc = Doc::new(StoredDocument::Editable(document.clone()), None, true, registry);
        assert_eq!(doc.editor.kind(), "test.decision-table");
        assert_eq!(doc.snapshot(), StoredDocument::Editable(document));
        assert!(doc.editor.views().is_empty());
        assert_eq!(doc.editor.exports().len(), 2);
        assert_eq!(doc.editor.export_as("csv").unwrap().bytes, b"true,approve");
        assert!(!doc.editor.enabled(PNG));
        assert!(!doc.dirty());
    }
    #[test]
    fn duplicate_identifiers_and_ambiguous_imports_are_rejected() {
        assert!(DiagramRegistry::new(vec![descriptor(), descriptor()], vec![], vec![]).is_err());
        let example = ExampleDescriptor {
            id: "test.unknown-example",
            kind: "unknown-kind",
            label: "Example",
            create: || unreachable!(),
        };
        assert!(DiagramRegistry::new(vec![descriptor()], vec![example], vec![]).is_err());
        let importer = || ImportDescriptor {
            extensions: &["txt"],
            accepts: |_| true,
            decode: |_| unreachable!(),
        };
        let registry = DiagramRegistry::new(vec![], vec![], vec![importer(), importer()]).unwrap();
        assert!(matches!(registry.load(b"text"), Err(ReadError::Invalid(_))));
    }
    #[test]
    fn registered_examples_create_their_declared_diagram_kind() {
        let registry = crate::composition::registry();
        for example in &registry.examples {
            let document = (example.create)();
            assert_eq!(document.kind, example.kind, "{} must belong to its declared card", example.id);
            assert!(registry.open(&document).is_ok());
        }
    }
    #[test]
    fn xml_import_detection_does_not_depend_on_namespace_prefixes() {
        let registry = crate::composition::registry();
        let bpmn = op_core::bpmn_xml::export(&op_core::examples::order_process())
            .replace("bpmn:", "third:")
            .replace("xmlns:bpmn=", "xmlns:third=");
        assert_eq!(registry.load(bpmn.as_bytes()).unwrap().kind, "bpmn");
        let pnml = op_core::pnml::export(&op_core::examples::mutual_exclusion())
            .replace("<pnml ", "<p:pnml xmlns:p=\"urn:pnml\" ")
            .replace("</pnml>", "</p:pnml>");
        assert_eq!(registry.load(pnml.as_bytes()).unwrap().kind, "petri");
        let xes = br#"<x:log xmlns:x="urn:xes"><x:trace><x:event><x:string key="concept:name" value="A"/></x:event></x:trace></x:log>"#;
        assert_eq!(registry.load(xes).unwrap().kind, "event-log");
        assert!(registry.load(b"<unknown/>").is_err());
    }
}
