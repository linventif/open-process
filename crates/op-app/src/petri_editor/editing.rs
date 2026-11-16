use super::*;
use crate::editor::clipboard::{CanvasClipboard, ClipboardState};

impl CanvasClipboard for PetriEditor {
    type Fragment = op_core::petri::editing::Fragment;
    fn clipboard_state(&mut self) -> &mut ClipboardState {
        &mut self.clipboard
    }
    fn clipboard_kind(&self) -> &'static str {
        "petri"
    }
    fn editing_allowed(&self) -> bool {
        self.sim.is_none()
    }
    fn has_selection(&self) -> bool {
        !self.group.is_empty() || matches!(self.selection, Some(Sel::Node(_)))
    }
    fn fragment(&self) -> Self::Fragment {
        let mut nodes = self.group.clone();
        if let Some(Sel::Node(node)) = self.selection
            && !nodes.contains(&node)
        {
            nodes.push(node);
        }
        self.net.fragment(&nodes)
    }
    fn paste_fragment(&mut self, fragment: &Self::Fragment, delta: Pos, commit: bool) {
        if commit {
            self.history.sync(&self.net, false);
        } else {
            self.history.begin(&self.net);
        }
        match self.net.paste(fragment, delta) {
            Ok(nodes) => {
                self.group = nodes;
                self.selection = None;
                self.normalize_group();
                self.tool = Tool::Select;
                self.arc_pending = None;
                self.status.clear();
            }
            Err(error) => self.status = error,
        }
        if commit {
            self.history.sync(&self.net, false);
        }
    }
    fn cut_selection(&mut self) {
        self.history.sync(&self.net, false);
        self.delete_selection();
        self.history.sync(&self.net, false);
    }
    fn select_everything(&mut self) {
        self.select_all();
    }
}
