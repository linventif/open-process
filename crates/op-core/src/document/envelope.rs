//! Open, versioned document envelope. No dependency on the notation catalogue or UI.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::marker::PhantomData;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Document {
    pub kind: String,
    pub model_version: u32,
    pub model: Value,
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    format: String,
    version: u32,
    document: Document,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReadError {
    Invalid(String),
    Unsupported(String),
}

impl std::fmt::Display for ReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(s) | Self::Unsupported(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for ReadError {}

impl Document {
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(&Envelope {
            format: "open-process".into(),
            version: 2,
            document: self.clone(),
        })
        .expect("JSON document is serializable")
    }

    /// Reads the envelope before handing the payload to a notation's typed codec.
    pub fn read(bytes: &[u8]) -> Result<Self, ReadError> {
        if bytes.len() > super::MAX_DOCUMENT_BYTES {
            return Err(ReadError::Invalid(crate::i18n::tr("file.input_limit").into()));
        }
        let text = std::str::from_utf8(bytes).map_err(|e| ReadError::Invalid(e.to_string()))?;
        let text = text.trim_start_matches('\u{feff}').trim_start();
        let value: Value = serde_json::from_str(text).map_err(|e| ReadError::Invalid(e.to_string()))?;
        if value.get("format").and_then(Value::as_str) != Some("open-process") {
            return Err(ReadError::Invalid("Invalid Open Process format identifier".into()));
        }
        match value.get("version").and_then(Value::as_u64) {
            Some(2) => serde_json::from_value::<Envelope>(value)
                .map(|f| f.document)
                .map_err(|e| ReadError::Invalid(e.to_string())),
            Some(1) => {
                let tagged = &value["model"];
                let model = tagged
                    .get("model")
                    .cloned()
                    .ok_or_else(|| ReadError::Invalid("Missing legacy model".into()))?;
                let kind = match tagged.get("type").and_then(Value::as_str) {
                    Some("Bpmn") => "bpmn",
                    Some("Petri") => "petri",
                    Some("Log") => "event-log",
                    Some("Uml") => match model.get("kind").and_then(Value::as_str) {
                        Some("Class") => "uml.class",
                        Some("UseCase") => "uml.use-case",
                        _ => return Err(ReadError::Unsupported("Unknown legacy UML kind".into())),
                    },
                    Some(kind) => return Err(ReadError::Unsupported(format!("Unknown legacy document type: {kind}"))),
                    None => return Err(ReadError::Invalid("Missing legacy model type".into())),
                };
                Ok(Self {
                    kind: kind.into(),
                    model_version: 1,
                    model,
                })
            }
            Some(v) => Err(ReadError::Unsupported(format!("Unsupported Open Process envelope version: {v}"))),
            None => Err(ReadError::Invalid("Missing or invalid envelope version".into())),
        }
    }
}

/// Notation-specific code can replace this with a codec implementing migrations.
pub trait DocumentCodec<M> {
    fn encode(&self, model: &M) -> Document;
    fn decode(&self, document: &Document) -> Result<M, ReadError>;
}

type StorageValidator<M> = fn(&M) -> Result<(), String>;

pub struct JsonCodec<M> {
    kind: &'static str,
    validate: Option<StorageValidator<M>>,
    _model: PhantomData<fn() -> M>,
}

impl<M> JsonCodec<M> {
    pub const fn new(kind: &'static str) -> Self {
        Self {
            kind,
            validate: None,
            _model: PhantomData,
        }
    }

    /// A notation owns the storage checks applied to every supported payload version.
    pub const fn checked(kind: &'static str, validate: fn(&M) -> Result<(), String>) -> Self {
        Self {
            kind,
            validate: Some(validate),
            _model: PhantomData,
        }
    }
}

impl<M: Serialize + DeserializeOwned> DocumentCodec<M> for JsonCodec<M> {
    fn encode(&self, model: &M) -> Document {
        Document {
            kind: self.kind.into(),
            model_version: 1,
            model: serde_json::to_value(model).expect("model is serializable"),
        }
    }

    fn decode(&self, document: &Document) -> Result<M, ReadError> {
        if document.kind != self.kind {
            return Err(ReadError::Invalid(format!("Expected {}, got {}", self.kind, document.kind)));
        }
        if document.model_version != 1 {
            return Err(ReadError::Unsupported(format!(
                "Unsupported {} model version: {}",
                self.kind, document.model_version
            )));
        }
        let model = serde_json::from_value(document.model.clone()).map_err(|e| ReadError::Invalid(e.to_string()))?;
        if let Some(validate) = self.validate {
            validate(&model).map_err(ReadError::Invalid)?;
        }
        Ok(model)
    }
}
