//! Automatic document copies, separate from manually selected files and the open workspace.
use crate::editor::StoredDocument;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
const PREFIX: &str = "open-process-document-";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub revision: String,
    pub name: String,
    pub kind: String,
    pub manual_path: Option<String>,
    #[serde(default)]
    pub original_reason: Option<String>,
}

impl Entry {
    pub fn new(name: String, kind: String, manual_path: Option<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            revision: String::new(),
            name,
            kind,
            manual_path,
            original_reason: None,
        }
    }
    fn validate(&self) -> Result<(), String> {
        if uuid::Uuid::parse_str(&self.id).is_err() {
            return Err("Invalid automatic document identifier".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Record {
    pub entry: Entry,
    pub state: StoredDocument,
}

struct Request {
    ticket: u64,
    record: Record,
    previous: String,
    revisions: crate::editor::Revisions,
}

pub struct Completion {
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    ticket: u64,
    pub record: Record,
    pub result: Result<(), String>,
    pub revisions: crate::editor::Revisions,
}

enum Backend {
    Disabled,
    #[cfg(not(target_arch = "wasm32"))]
    Native(native::Worker),
    #[cfg(target_arch = "wasm32")]
    Browser(web_sys::Storage),
}

pub struct Library {
    backend: Backend,
    pub entries: Vec<Entry>,
    ticket: u64,
    pending: bool,
    pub location: String,
}

impl Library {
    pub fn disabled() -> Self {
        Self {
            backend: Backend::Disabled,
            entries: vec![],
            ticket: 0,
            pending: false,
            location: String::new(),
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn open(ctx: egui::Context) -> Result<Self, String> {
        let dirs = directories::UserDirs::new().ok_or("Cannot locate your Documents folder")?;
        let documents = dirs
            .document_dir()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| dirs.home_dir().join("Documents"));
        Self::at(documents.join("Open Process"), ctx)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn at(root: std::path::PathBuf, ctx: egui::Context) -> Result<Self, String> {
        let store = native::Store::new(root)?;
        let entries = store.entries()?;
        let location = store.root.display().to_string();
        Ok(Self {
            backend: Backend::Native(native::Worker::start(store, ctx)?),
            entries,
            ticket: 0,
            pending: false,
            location,
        })
    }

    #[cfg(target_arch = "wasm32")]
    pub fn open(_: egui::Context) -> Result<Self, String> {
        let storage = web_sys::window()
            .ok_or("No browser window")?
            .local_storage()
            .map_err(js_error)?
            .ok_or("Local storage is unavailable")?;
        let mut entries = Vec::new();
        for index in 0..storage.length().map_err(js_error)? {
            let Some(key) = storage.key(index).map_err(js_error)? else {
                continue;
            };
            if key.starts_with(PREFIX) {
                let raw = storage.get_item(&key).map_err(js_error)?.ok_or("Missing automatic document")?;
                let record: Record = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                record.entry.validate()?;
                if key != format!("{PREFIX}{}", record.entry.id) {
                    return Err("Mismatched automatic document identity".into());
                }
                entries.push(record.entry);
            }
        }
        Ok(Self {
            backend: Backend::Browser(storage),
            entries,
            ticket: 0,
            pending: false,
            location: op_core::i18n::tr("autosave.browser_location").into(),
        })
    }

    pub fn enabled(&self) -> bool {
        !matches!(self.backend, Backend::Disabled)
    }
    pub fn pending(&self) -> bool {
        self.pending
    }

    pub fn load(&self, id: &str) -> Result<Record, String> {
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or("Unknown automatic document")?;
        entry.validate()?;
        match &self.backend {
            Backend::Disabled => Err("Automatic saving is unavailable".into()),
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Native(worker) => worker.store.load(entry),
            #[cfg(target_arch = "wasm32")]
            Backend::Browser(storage) => {
                let raw = storage
                    .get_item(&format!("{PREFIX}{id}"))
                    .map_err(js_error)?
                    .ok_or("Missing automatic document")?;
                serde_json::from_str(&raw).map_err(|e| e.to_string())
            }
        }
    }

    fn request(&mut self, mut record: Record, revisions: crate::editor::Revisions) -> Request {
        self.ticket += 1;
        record.entry.original_reason = match &record.state {
            StoredDocument::Original { reason, .. } => Some(reason.clone()),
            StoredDocument::Editable(_) => None,
        };
        let previous = std::mem::replace(&mut record.entry.revision, uuid::Uuid::new_v4().to_string());
        Request {
            ticket: self.ticket,
            record,
            previous,
            revisions,
        }
    }

    pub fn submit(&mut self, record: Record, revisions: crate::editor::Revisions) -> Result<Vec<Completion>, String> {
        let request = self.request(record, revisions);
        match &mut self.backend {
            Backend::Disabled => Err("Automatic saving is unavailable".into()),
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Native(worker) => {
                worker.sender.send(request).map_err(|e| e.to_string())?;
                self.pending = true;
                Ok(vec![])
            }
            #[cfg(target_arch = "wasm32")]
            Backend::Browser(storage) => Ok(vec![browser_write(storage, request)]),
        }
    }

    /// A close/shutdown barrier follows older writes on the same single-writer queue.
    pub fn save_now(&mut self, record: Record, revisions: crate::editor::Revisions) -> Result<Vec<Completion>, String> {
        let request = self.request(record, revisions);
        let target = request.ticket;
        match &mut self.backend {
            Backend::Disabled => Err("Automatic saving is unavailable".into()),
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Native(worker) => {
                worker.sender.send(request).map_err(|e| e.to_string())?;
                let mut completed = Vec::new();
                loop {
                    let result = worker.receiver.recv().map_err(|e| e.to_string())?;
                    let finished = result.ticket == target;
                    completed.push(result);
                    if finished {
                        break;
                    }
                }
                self.pending = false;
                Ok(completed)
            }
            #[cfg(target_arch = "wasm32")]
            Backend::Browser(storage) => {
                let _ = target;
                Ok(vec![browser_write(storage, request)])
            }
        }
    }

    pub fn poll(&mut self) -> Vec<Completion> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Backend::Native(worker) = &mut self.backend {
            let completed: Vec<_> = worker.receiver.try_iter().collect();
            if !completed.is_empty() {
                self.pending = false;
            }
            return completed;
        }
        vec![]
    }

    pub fn wait(&mut self) -> Result<Vec<Completion>, String> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.pending
            && let Backend::Native(worker) = &mut self.backend
        {
            let result = worker.receiver.recv().map_err(|e| e.to_string())?;
            self.pending = false;
            return Ok(vec![result]);
        }
        Ok(self.poll())
    }

    pub fn remember(&mut self, entry: Entry) {
        self.entries.retain(|old| old.id != entry.id);
        self.entries.insert(0, entry);
    }
}

fn check_record(record: &Record) -> Result<Vec<u8>, String> {
    record.entry.validate()?;
    let bytes = record.state.bytes();
    if bytes.len() > op_core::document::MAX_DOCUMENT_BYTES {
        return Err(op_core::i18n::tr("file.input_limit").into());
    }
    Ok(bytes)
}

fn check_revision(previous: &str, current: Option<&Entry>) -> Result<(), String> {
    if current.map(|entry| entry.revision.as_str()).unwrap_or("") != previous {
        return Err(op_core::i18n::tr("autosave.conflict").into());
    }
    Ok(())
}

#[cfg(target_arch = "wasm32")]
fn js_error(error: wasm_bindgen::JsValue) -> String {
    format!("{error:?}")
}

#[cfg(target_arch = "wasm32")]
fn browser_write(storage: &web_sys::Storage, request: Request) -> Completion {
    let result = (|| {
        check_record(&request.record)?;
        let key = format!("{PREFIX}{}", request.record.entry.id);
        let previous: Option<Record> = storage
            .get_item(&key)
            .map_err(js_error)?
            .map(|raw| serde_json::from_str(&raw))
            .transpose()
            .map_err(|e| e.to_string())?;
        check_revision(&request.previous, previous.as_ref().map(|record| &record.entry))?;
        let raw = serde_json::to_string(&request.record).map_err(|e| e.to_string())?;
        storage.set_item(&key, &raw).map_err(js_error)
    })();
    Completion {
        ticket: request.ticket,
        record: request.record,
        result,
        revisions: request.revisions,
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::{fs, path::PathBuf, sync::mpsc};

    #[derive(Clone)]
    pub(super) struct Store {
        pub root: PathBuf,
    }

    impl Store {
        pub fn new(root: PathBuf) -> Result<Self, String> {
            fs::create_dir_all(&root).map_err(|e| e.to_string())?;
            Ok(Self { root })
        }
        fn metadata(&self, id: &str) -> PathBuf {
            self.root.join(format!("{id}.json"))
        }
        fn document(&self, id: &str) -> PathBuf {
            self.root.join(format!("{id}.opx"))
        }
        pub fn entries(&self) -> Result<Vec<Entry>, String> {
            let mut entries = vec![];
            for item in fs::read_dir(&self.root).map_err(|e| e.to_string())? {
                let path = item.map_err(|e| e.to_string())?.path();
                if path.extension().and_then(|s| s.to_str()) != Some("json") {
                    continue;
                }
                let Some(id) = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .filter(|s| uuid::Uuid::parse_str(s).is_ok())
                else {
                    continue;
                };
                let entry: Entry = serde_json::from_slice(&crate::io::read_file(&path)?).map_err(|e| e.to_string())?;
                entry.validate()?;
                if entry.id != id {
                    return Err("Mismatched automatic document identity".into());
                }
                if self.document(id).is_file() {
                    entries.push(entry);
                }
            }
            entries.sort_by(|a, b| a.name.cmp(&b.name));
            Ok(entries)
        }
        pub fn load(&self, entry: &Entry) -> Result<Record, String> {
            let bytes = crate::io::read_file(self.document(&entry.id))?;
            let current: Entry = serde_json::from_slice(&crate::io::read_file(self.metadata(&entry.id))?).map_err(|e| e.to_string())?;
            current.validate()?;
            if current.id != entry.id {
                return Err("Mismatched automatic document identity".into());
            }
            let state = if let Some(reason) = &current.original_reason {
                StoredDocument::Original {
                    bytes,
                    reason: reason.clone(),
                }
            } else {
                match op_core::document::Document::read(&bytes) {
                    Ok(document) => StoredDocument::Editable(document),
                    Err(reason) => StoredDocument::Original {
                        bytes,
                        reason: reason.to_string(),
                    },
                }
            };
            Ok(Record { entry: current, state })
        }
        fn write(&self, request: &Request) -> Result<(), String> {
            let bytes = check_record(&request.record)?;
            let metadata = self.metadata(&request.record.entry.id);
            let current: Option<Entry> = match crate::io::read_file(&metadata) {
                Ok(raw) => Some(serde_json::from_slice(&raw).map_err(|e| e.to_string())?),
                Err(_) if !metadata.exists() => None,
                Err(error) => return Err(error),
            };
            check_revision(&request.previous, current.as_ref())?;
            crate::io::atomic::write(&self.document(&request.record.entry.id), &bytes).map_err(|e| e.to_string())?;
            let metadata_bytes = serde_json::to_vec(&request.record.entry).map_err(|e| e.to_string())?;
            crate::io::atomic::write(&metadata, &metadata_bytes).map_err(|e| e.to_string())
        }
    }

    pub(super) struct Worker {
        pub store: Store,
        pub sender: mpsc::SyncSender<Request>,
        pub receiver: mpsc::Receiver<Completion>,
    }
    impl Worker {
        pub fn start(store: Store, ctx: egui::Context) -> Result<Self, String> {
            let (sender, input) = mpsc::sync_channel::<Request>(1);
            let (output, receiver) = mpsc::channel();
            let target = store.clone();
            std::thread::Builder::new()
                .name("open-process-autosave".into())
                .spawn(move || {
                    while let Ok(request) = input.recv() {
                        let result = target.write(&request);
                        if output
                            .send(Completion {
                                ticket: request.ticket,
                                record: request.record,
                                result,
                                revisions: request.revisions,
                            })
                            .is_err()
                        {
                            break;
                        }
                        ctx.request_repaint();
                    }
                })
                .map_err(|e| e.to_string())?;
            Ok(Self { store, sender, receiver })
        }
    }
}
