use super::*;
use crate::editor::clipboard::{CanvasClipboard, ClipboardState};

impl CanvasClipboard for UmlEditor {
    type Fragment = DiagramFragment;
    fn clipboard_state(&mut self) -> &mut ClipboardState {
        &mut self.clipboard
    }
    fn clipboard_kind(&self) -> &'static str {
        match self.diagram.kind {
            DiagramKind::Class => "uml.class",
            DiagramKind::UseCase => "uml.use-case",
        }
    }
    fn has_selection(&self) -> bool {
        !self.group.is_empty()
    }
    fn fragment(&self) -> Self::Fragment {
        self.diagram.fragment(&self.group)
    }
    fn paste_fragment(&mut self, fragment: &Self::Fragment, delta: Pos, commit: bool) {
        if fragment.kind != self.diagram.kind || fragment.check_storage().is_err() {
            return;
        }
        if commit {
            self.history.sync(&self.diagram, false);
        } else {
            self.history.begin(&self.diagram);
        }
        let ids = self.diagram.paste(fragment, delta);
        if !ids.is_empty() {
            self.group = ids;
            self.selection = self.group.iter().next().copied().map(Selection::Node);
            self.tool = Tool::Select;
            self.pending = None;
        }
        if commit {
            self.history.sync(&self.diagram, false);
        }
    }
    fn cut_selection(&mut self) {
        self.delete_selection();
    }
    fn select_everything(&mut self) {
        self.group = self.diagram.nodes.iter().map(|n| n.id).collect();
        self.selection = self.group.iter().next().copied().map(Selection::Node);
    }
}
