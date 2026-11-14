//! Native Open Process file format (`.opx`, JSON).

use crate::bpmn::BpmnDiagram;
use crate::petri::PetriNet;
use serde::{Deserialize, Serialize};

mod envelope;
pub use envelope::{Document, DocumentCodec, JsonCodec, ReadError};

pub const EXTENSION: &str = "opx";
pub const MAX_DOCUMENT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "model")]
pub enum Model {
    Bpmn(BpmnDiagram),
    Petri(PetriNet),
    Uml(crate::uml::UmlDiagram),
    /// Event log for process discovery (raw text, parsed by [`crate::mining::EventLog::parse`]).
    Log(LogDocument),
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LogDocument {
    pub name: String,
    pub text: String,
    /// Step-by-step (manual) mode of the Alpha exercise.
    #[serde(default)]
    pub manual: bool,
    /// The student's answers (relations, matrix cells, algorithm steps).
    #[serde(default)]
    pub answers: std::collections::BTreeMap<String, String>,
}

impl Model {
    /// Compatibility bridge for callers of the original closed model API.
    pub fn document(&self) -> Document {
        match self {
            Self::Bpmn(m) => JsonCodec::new("bpmn").encode(m),
            Self::Petri(m) => JsonCodec::new("petri").encode(m),
            Self::Log(m) => JsonCodec::new("event-log").encode(m),
            Self::Uml(m) => JsonCodec::new(match m.kind {
                crate::uml::DiagramKind::Class => "uml.class",
                crate::uml::DiagramKind::UseCase => "uml.use-case",
            })
            .encode(m),
        }
    }

    pub fn from_document(d: &Document) -> Result<Self, ReadError> {
        match d.kind.as_str() {
            "bpmn" => BpmnDiagram::CODEC.decode(d).map(Self::Bpmn),
            "petri" => PetriNet::CODEC.decode(d).map(Self::Petri),
            "event-log" => JsonCodec::new("event-log").decode(d).map(Self::Log),
            "uml.class" | "uml.use-case" => crate::uml::UmlDiagram::codec(if d.kind == "uml.class" {
                crate::uml::DiagramKind::Class
            } else {
                crate::uml::DiagramKind::UseCase
            })
            .decode(d)
            .map(Self::Uml),
            kind => Err(ReadError::Unsupported(format!("Unknown document kind: {kind}"))),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Model::Bpmn(d) => &d.name,
            Model::Petri(n) => &n.name,
            Model::Uml(d) => &d.name,
            Model::Log(l) => &l.name,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileFormat {
    pub format: String,
    pub version: u32,
    pub model: Model,
}

pub fn to_json(model: &Model) -> String {
    serde_json::to_string_pretty(&FileFormat {
        format: "open-process".into(),
        version: 1,
        model: model.clone(),
    })
    .expect("serializable")
}

/// Loads any supported file by sniffing its content:
/// Open Process JSON, BPMN 2.0 XML, PNML, XES event logs, or plain-text / CSV event logs.
pub fn load_any(bytes: &[u8]) -> Result<Model, String> {
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(crate::i18n::tr("file.input_limit").into());
    }
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('{') {
        return Document::read(bytes)
            .and_then(|d| Model::from_document(&d))
            .map_err(|e| e.to_string());
    }
    if trimmed.starts_with('<') {
        let head: String = trimmed.chars().take(4000).collect();
        if head.contains("<log") && crate::mining::EventLog::from_xes(trimmed).is_ok() {
            return Ok(Model::Log(LogDocument {
                name: "Imported log".into(),
                text: trimmed.to_string(),
                ..Default::default()
            }));
        }
        if head.contains("<pnml") || head.contains("<net ") {
            return crate::pnml::import(trimmed).map(Model::Petri);
        }
        return crate::bpmn_xml::import(trimmed).map(Model::Bpmn);
    }
    // Anything else that parses as an event log (plain text, "Case 1, Task A", CSV)
    match crate::mining::EventLog::parse(trimmed) {
        Ok(log) if !log.traces.is_empty() => Ok(Model::Log(LogDocument {
            name: "Imported log".into(),
            text: trimmed.to_string(),
            ..Default::default()
        })),
        _ => Err("Unrecognized file format (expected .opx, .bpmn, .pnml, .xes, .csv or a text event log)".into()),
    }
}
