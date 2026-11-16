use super::*;
use crate::editor::clipboard::{CanvasClipboard, ClipboardState};

impl CanvasClipboard for DatabaseEditor {
    type Fragment = database::editing::Fragment;
    fn clipboard_state(&mut self) -> &mut ClipboardState {
        &mut self.state.clipboard
    }
    fn clipboard_kind(&self) -> &'static str {
        "database.schema"
    }
    fn editing_allowed(&self) -> bool {
        !self.state.sql_view
    }
    fn has_selection(&self) -> bool {
        self.state.selected.is_some() || !self.state.group.is_empty()
    }
    fn fragment(&self) -> Self::Fragment {
        let mut selected = self.state.group.clone();
        selected.extend(self.state.selected);
        self.schema.fragment(&selected)
    }
    fn paste_fragment(&mut self, fragment: &Self::Fragment, delta: Pos, commit: bool) {
        if commit {
            self.history.sync(&self.schema, false);
        } else {
            self.history.begin(&self.schema);
        }
        match self.schema.paste(fragment, delta) {
            Ok(ids) => {
                self.state.selected = ids.iter().next().copied();
                self.state.group = ids;
                self.state.connecting = false;
                self.state.pending = None;
                self.state.status.clear();
            }
            Err(error) => self.state.status = error,
        }
        if commit {
            self.history.sync(&self.schema, false);
        }
    }
    fn cut_selection(&mut self) {
        self.delete_selected();
    }
    fn select_everything(&mut self) {
        self.state.group = self.schema.tables.iter().map(|t| t.id).collect();
        self.state.selected = self.state.group.iter().next().copied();
    }
}
