//! Automatic copies are acknowledged separately from manual destinations.
use super::*;

impl OpenProcessApp {
    pub(super) fn restore_automatic_reference(&self, doc: &mut Doc, entry: autosave::Entry) {
        match self.automatic.load(&entry.id) {
            Ok(record) => {
                let matches = doc.snapshot() == record.state;
                if record.entry.revision != entry.revision && !matches {
                    // Keep pending workspace edits as an independent copy of an advanced record.
                    return;
                }
                doc.mark_saved(record.state);
                doc.archive = Some(record.entry);
                if matches {
                    doc.autosaved_revision = Some(doc.editor.revisions());
                }
            }
            Err(_) => doc.archive = Some(entry),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn save_browser_now(&mut self) {
        self.flush_automatic_documents();
        if !self.persistence_ready {
            return;
        }
        let result = (|| -> Result<(), String> {
            let storage = web_sys::window()
                .ok_or("No browser window")?
                .local_storage()
                .map_err(|error| format!("{error:?}"))?
                .ok_or("Local storage is unavailable")?;
            let raw = serde_json::to_string(&self.workspace_snapshot()).map_err(|error| error.to_string())?;
            storage.set_item(persistence::KEY, &raw).map_err(|error| format!("{error:?}"))
        })();
        if let Err(error) = result {
            self.automatic_error = Some(error);
        }
    }

    pub(super) fn complete_automatic(&mut self, results: Vec<autosave::Completion>) {
        for completion in results {
            match completion.result {
                Ok(()) => {
                    let id = &completion.record.entry.id;
                    if let Some(doc) = self
                        .docs
                        .iter_mut()
                        .find(|doc| doc.archive.as_ref().is_some_and(|entry| entry.id == *id))
                    {
                        doc.mark_saved(completion.record.state);
                        doc.autosaved_revision = Some(completion.revisions);
                        doc.archive = Some(completion.record.entry.clone());
                    }
                    self.automatic.remember(completion.record.entry);
                    if self.docs.iter().all(|doc| {
                        (!doc.editor.has_content() && doc.archive.is_none()) || doc.autosaved_revision == Some(doc.editor.revisions())
                    }) {
                        self.automatic_error = None;
                    }
                }
                Err(error) => self.automatic_error = Some(error),
            }
        }
    }

    fn automatic_record(&mut self, index: usize) -> Result<Option<(autosave::Record, Revisions)>, String> {
        let Some(doc) = self.docs.get_mut(index) else { return Ok(None) };
        if !doc.editor.has_content() && doc.archive.is_none() {
            return Ok(None);
        }
        if doc.autosaved_revision == Some(doc.editor.revisions()) && doc.archive.as_ref().is_some_and(|entry| entry.manual_path == doc.path)
        {
            return Ok(None);
        }
        doc.editor.validate_storage()?;
        let mut entry = doc
            .archive
            .clone()
            .unwrap_or_else(|| autosave::Entry::new(doc.name(), doc.editor.kind().into(), doc.path.clone()));
        entry.name = doc.name();
        entry.kind = doc.editor.kind().into();
        entry.manual_path = doc.path.clone();
        doc.archive = Some(entry.clone());
        Ok(Some((
            autosave::Record {
                entry,
                state: doc.snapshot(),
            },
            doc.editor.revisions(),
        )))
    }

    pub(super) fn save_automatically(&mut self, ctx: &egui::Context) {
        let completed = self.automatic.poll();
        self.complete_automatic(completed);
        if !self.automatic.enabled() {
            return;
        }
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        if self.automatic.pending() {
            return;
        }
        let now = ctx.input(|input| input.time);
        for index in 0..self.docs.len() {
            let uid = self.docs[index].uid;
            if self.automatic_attempts.get(&uid).is_some_and(|last| now - last < 1.0) {
                continue;
            }
            match self.automatic_record(index) {
                Ok(Some((record, revision))) => {
                    self.automatic_attempts.insert(uid, now);
                    match self.automatic.submit(record, revision) {
                        Ok(results) => self.complete_automatic(results),
                        Err(error) => self.automatic_error = Some(error),
                    }
                    ctx.request_repaint_after(std::time::Duration::from_millis(100));
                    break;
                }
                Ok(None) => {}
                Err(error) => {
                    self.automatic_attempts.insert(uid, now);
                    self.automatic_error = Some(error);
                }
            }
        }
    }

    pub(super) fn flush_automatic_document(&mut self, index: usize) {
        if !self.automatic.enabled() {
            return;
        }
        match self.automatic.wait() {
            Ok(results) => self.complete_automatic(results),
            Err(error) => {
                self.automatic_error = Some(error);
                return;
            }
        }
        match self.automatic_record(index) {
            Ok(Some((record, revision))) => match self.automatic.save_now(record, revision) {
                Ok(results) => self.complete_automatic(results),
                Err(error) => self.automatic_error = Some(error),
            },
            Ok(None) => {}
            Err(error) => self.automatic_error = Some(error),
        }
    }

    pub(super) fn flush_automatic_documents(&mut self) {
        for index in 0..self.docs.len() {
            self.flush_automatic_document(index);
        }
    }

    pub(super) fn automatic_status(&self, ui: &mut Ui) {
        let Some(error) = &self.automatic_error else { return };
        egui::Panel::bottom("automatic_save_status").show(ui, |ui| {
            ui.colored_label(ui.visuals().error_fg_color, trf("autosave.failed", &[&error]))
                .on_hover_text(&self.automatic.location);
        });
    }

    pub(super) fn open_automatic_document(&mut self, index: usize) {
        let Some(entry) = self.automatic.entries.get(index) else { return };
        let id = entry.id.clone();
        if let Some(index) = self
            .docs
            .iter()
            .position(|doc| doc.archive.as_ref().is_some_and(|entry| entry.id == id))
        {
            self.set_active(index);
            return;
        }
        match self.automatic.load(&id) {
            Ok(record) => {
                self.open_stored(record.state, record.entry.manual_path.clone(), true, None);
                let doc = &mut self.docs[self.active];
                doc.archive = Some(record.entry);
                doc.autosaved_revision = Some(doc.editor.revisions());
            }
            Err(error) => self.automatic_error = Some(error),
        }
    }

    pub(super) fn workspace_snapshot(&self) -> Workspace {
        let mut workspace = Workspace {
            version: 4,
            docs: vec![],
            active: self.active,
            tree: self.tree.clone(),
            autosave: Default::default(),
            archived: vec![],
        };
        for (index, doc) in self.docs.iter().enumerate() {
            let views = doc.editor.workspace();
            if let Some(entry) = &doc.archive {
                if doc.autosaved_revision == Some(doc.editor.revisions()) && !entry.revision.is_empty() {
                    workspace.archived.push((index, doc.uid, entry.clone(), views));
                    continue;
                }
                workspace.autosave.insert(doc.uid, entry.clone());
            }
            workspace.docs.push((doc.uid, doc.snapshot(), doc.path.clone(), views));
        }
        workspace
    }

    pub(super) fn resolve_automatic_workspace(&self, workspace: &mut Workspace) -> Result<(), String> {
        let mut resolved = Vec::new();
        for (index, uid, entry, views) in &workspace.archived {
            let record = self.automatic.load(&entry.id)?;
            resolved.push((*index, *uid, record, views.clone()));
        }
        resolved.sort_by_key(|(index, ..)| *index);
        for (index, uid, record, views) in resolved {
            workspace.autosave.insert(uid, record.entry.clone());
            workspace.docs.insert(
                index.min(workspace.docs.len()),
                (uid, record.state, record.entry.manual_path, views),
            );
        }
        workspace.archived.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
