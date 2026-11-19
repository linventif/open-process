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

/// Open Process build that wrote a file. Older files have none; readers that
/// predate this field ignore it, so it needs no new envelope version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Generator {
    pub application: String,
    /// Shown to people: a release tag such as `v0.4.0` or `main-812-1a2b3c4`.
    pub version: String,
    /// Compared by readers: see [`CONTENT_REVISION`]. Missing means 0.
    #[serde(default)]
    pub revision: u32,
}

/// Increases whenever any notation can store content that older readers
/// would not keep (a new optional field, kind or model version). A file with a
/// higher revision comes from a newer Open Process.
pub const CONTENT_REVISION: u32 = 1;

static WRITER_VERSION: std::sync::OnceLock<&'static str> = std::sync::OnceLock::new();

/// Called once at startup with the application's release tag.
pub fn set_writer_version(version: &'static str) {
    let _ = WRITER_VERSION.set(version);
}

impl Generator {
    /// Identity of this build, written into every saved envelope.
    pub fn current() -> Self {
        Self {
            application: "Open Process".into(),
            version: WRITER_VERSION
                .get()
                .copied()
                .unwrap_or(concat!("v", env!("CARGO_PKG_VERSION")))
                .into(),
            revision: CONTENT_REVISION,
        }
    }

    /// Reads the writer of an `.opx` file without decoding its document.
    pub fn of(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > super::MAX_DOCUMENT_BYTES {
            return None;
        }
        let text = std::str::from_utf8(bytes).ok()?.trim_start_matches('\u{feff}').trim_start();
        if !text.starts_with('{') {
            return None;
        }
        let value: Value = serde_json::from_str(text).ok()?;
        if value.get("format").and_then(Value::as_str) != Some("open-process") {
            return None;
        }
        serde_json::from_value(value.get("generator")?.clone()).ok()
    }

    pub fn is_newer(&self) -> bool {
        self.revision > CONTENT_REVISION
    }
}

/// Fields present in `original` that this version dropped from `kept` (the same
/// document decoded and encoded again), as readable paths such as
/// `nodes › attachment`. Saving from this version would lose them.
pub fn unsupported_content(original: &Value, kept: &Value) -> Vec<String> {
    fn walk(original: &Value, kept: &Value, path: &mut Vec<String>, out: &mut std::collections::BTreeSet<String>) {
        match (original, kept) {
            (Value::Object(a), Value::Object(b)) => {
                for (key, value) in a {
                    path.push(key.clone());
                    match b.get(key) {
                        Some(other) => walk(value, other, path, out),
                        None if !value.is_null() => {
                            out.insert(path.join(" › "));
                        }
                        None => {}
                    }
                    path.pop();
                }
            }
            (Value::Array(a), Value::Array(b)) => {
                for (value, other) in a.iter().zip(b) {
                    walk(value, other, path, out);
                }
            }
            _ => {}
        }
    }
    let mut out = std::collections::BTreeSet::new();
    walk(original, kept, &mut vec![], &mut out);
    out.into_iter().collect()
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    format: String,
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    generator: Option<Generator>,
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
            generator: Some(Generator::current()),
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
