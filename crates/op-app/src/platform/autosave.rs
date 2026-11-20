//! Automatic document copies, separate from manually selected files and the open workspace.
use crate::editor::StoredDocument;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
const PREFIX: &str = "open-process-document-";

/// Days a deleted automatic copy stays in the trash before it is erased.
pub const TRASH_DAYS: u64 = 15;
const DAY: u64 = 86_400;

/// Seconds since the Unix epoch, on native and web.
pub fn now() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs())
    }
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() / 1000.0) as u64
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub revision: String,
    pub name: String,
    pub kind: String,
    pub manual_path: Option<String>,
    #[serde(default)]
    pub original_reason: Option<String>,
    /// When the copy was moved to the trash (Unix seconds). Older versions
    /// ignore this field and list the copy as a saved diagram again.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trashed_at: Option<u64>,
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
            trashed_at: None,
        }
    }

    /// Whole days left before a trashed copy is erased (0 once due).
    pub fn days_left(&self, now: u64) -> Option<u64> {
        self.trashed_at.map(|at| (at + TRASH_DAYS * DAY).saturating_sub(now).div_ceil(DAY))
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

    /// Moves a closed copy to the trash (`Some(time)`) or restores it (`None`).
    /// Only its metadata changes; call [`Self::wait`] first so that no earlier
    /// write of the same copy can land afterwards.
    pub fn set_trashed(&mut self, id: &str, trashed_at: Option<u64>) -> Result<(), String> {
        let index = self.entries.iter().position(|e| e.id == id).ok_or("Unknown automatic document")?;
        let mut entry = self.entries[index].clone();
        entry.validate()?;
        entry.trashed_at = trashed_at;
        match &self.backend {
            Backend::Disabled => return Err("Automatic saving is unavailable".into()),
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Native(worker) => worker.store.write_metadata(&entry)?,
            #[cfg(target_arch = "wasm32")]
            Backend::Browser(storage) => {
                let key = format!("{PREFIX}{id}");
                let raw = storage.get_item(&key).map_err(js_error)?.ok_or("Missing automatic document")?;
                let mut record: Record = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
                record.entry.trashed_at = trashed_at;
                entry = record.entry.clone();
                let raw = serde_json::to_string(&record).map_err(|e| e.to_string())?;
                storage.set_item(&key, &raw).map_err(js_error)?;
            }
        }
        self.entries[index] = entry;
        Ok(())
    }

    /// Erases a copy for good.
    pub fn delete(&mut self, id: &str) -> Result<(), String> {
        let entry = self.entries.iter().find(|e| e.id == id).ok_or("Unknown automatic document")?;
        entry.validate()?;
        match &self.backend {
            Backend::Disabled => return Err("Automatic saving is unavailable".into()),
            #[cfg(not(target_arch = "wasm32"))]
            Backend::Native(worker) => worker.store.delete(id)?,
            #[cfg(target_arch = "wasm32")]
            Backend::Browser(storage) => storage.remove_item(&format!("{PREFIX}{id}")).map_err(js_error)?,
        }
        self.entries.retain(|e| e.id != id);
        Ok(())
    }

    /// Erases copies trashed more than [`TRASH_DAYS`] ago; returns how many.
    pub fn purge(&mut self, now: u64) -> Result<usize, String> {
        let due: Vec<String> = self
            .entries
            .iter()
            .filter(|e| e.days_left(now) == Some(0))
            .map(|e| e.id.clone())
            .collect();
        for id in &due {
            self.delete(id)?;
        }
        Ok(due.len())
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
        pub fn write_metadata(&self, entry: &Entry) -> Result<(), String> {
            let path = self.metadata(&entry.id);
            let current: Entry = serde_json::from_slice(&crate::io::read_file(&path)?).map_err(|e| e.to_string())?;
            if current.id != entry.id || current.revision != entry.revision {
                return Err(op_core::i18n::tr("autosave.conflict").into());
            }
            let bytes = serde_json::to_vec(entry).map_err(|e| e.to_string())?;
            crate::io::atomic::write(&path, &bytes).map_err(|e| e.to_string())
        }
        /// The document goes first: without it the copy is no longer listed.
        pub fn delete(&self, id: &str) -> Result<(), String> {
            for path in [self.document(id), self.metadata(id)] {
                match fs::remove_file(&path) {
                    Err(error) if error.kind() != std::io::ErrorKind::NotFound => return Err(error.to_string()),
                    _ => {}
                }
            }
            Ok(())
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use op_core::document::{DocumentCodec, JsonCodec};

    fn save(library: &mut Library, name: &str) -> String {
        let state = StoredDocument::Editable(JsonCodec::new("bpmn").encode(&op_core::bpmn::BpmnDiagram::new(name)));
        let record = Record {
            entry: Entry::new(name.into(), "bpmn".into(), None),
            state,
        };
        let done = library.save_now(record, Default::default()).unwrap();
        let entry = done.last().unwrap().record.entry.clone();
        assert!(done.iter().all(|c| c.result.is_ok()));
        library.remember(entry.clone());
        entry.id
    }

    #[test]
    fn trashed_copies_persist_restore_and_are_erased_after_fifteen_days() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut library = Library::at(directory.path().into(), ctx.clone()).unwrap();
        let (a, b) = (save(&mut library, "A"), save(&mut library, "B"));
        let reopen = || Library::at(directory.path().into(), ctx.clone()).unwrap();
        let trashed_at = |library: &Library, id: &str| library.entries.iter().find(|e| e.id == id).unwrap().trashed_at;

        library.set_trashed(&a, Some(1_000)).unwrap();
        assert_eq!(trashed_at(&reopen(), &a), Some(1_000), "the trash survives a restart");
        assert_eq!(trashed_at(&reopen(), &b), None);
        let entry = reopen().entries.into_iter().find(|e| e.id == a).unwrap();
        assert_eq!(entry.days_left(1_000), Some(TRASH_DAYS));
        assert_eq!(entry.days_left(1_000 + TRASH_DAYS * DAY - DAY / 2), Some(1));
        assert_eq!(entry.days_left(1_000 + TRASH_DAYS * DAY), Some(0));
        assert!(reopen().load(&a).is_ok(), "trashed content stays readable until erased");

        library.set_trashed(&a, None).unwrap();
        assert_eq!(trashed_at(&reopen(), &a), None, "restored");

        library.set_trashed(&a, Some(5_000)).unwrap();
        assert_eq!(library.purge(5_000 + TRASH_DAYS * DAY - 1).unwrap(), 0);
        assert_eq!(library.purge(5_000 + TRASH_DAYS * DAY).unwrap(), 1);
        let files = |id: &str| [format!("{id}.opx"), format!("{id}.json")].map(|f| directory.path().join(f).exists());
        assert_eq!(files(&a), [false, false]);
        assert_eq!(files(&b), [true, true]);
        assert_eq!(reopen().entries.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), [b.as_str()]);

        library.delete(&b).unwrap();
        assert!(reopen().entries.is_empty());
        assert!(library.set_trashed(&b, Some(1)).is_err(), "unknown copies are reported");
    }
}
