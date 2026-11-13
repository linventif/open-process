//! Native Open Process file format (`.opx`, JSON).

use crate::bpmn::BpmnDiagram;
use crate::petri::PetriNet;
use serde::{Deserialize, Serialize};

pub const EXTENSION: &str = "opx";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "model")]
pub enum Model {
    Bpmn(BpmnDiagram),
    Petri(PetriNet),
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
    pub fn name(&self) -> &str {
        match self {
            Model::Bpmn(d) => &d.name,
            Model::Petri(n) => &n.name,
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
    let text = String::from_utf8_lossy(bytes);
    let trimmed = text.trim_start_matches('\u{feff}').trim_start();
    if trimmed.starts_with('{') {
        let f: FileFormat = serde_json::from_str(trimmed).map_err(|e| format!("Invalid Open Process file: {e}"))?;
        return Ok(f.model);
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
