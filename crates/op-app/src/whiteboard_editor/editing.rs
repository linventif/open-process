//! Clipboard integration and discoverable drawing actions.
use super::*;
use egui::{Context, Key, KeyboardShortcut, Modifiers};
use op_core::i18n::key;

const CLIPBOARD: &str = "open-process-whiteboard-clipboard";

#[derive(Clone, Copy)]
pub(super) enum ClipboardAction {
    Copy,
    Cut,
    Paste,
}

impl WhiteboardEditor {
    fn copy(&mut self, ctx: &Context) -> bool {
        if self.state.selection.is_empty() {
            return false;
        }
        let fragment = self.board.fragment(&self.state.selection);
        match fragment.to_clipboard() {
            Ok(text) => {
                ctx.copy_text(text);
                ctx.data_mut(|d| d.insert_temp(egui::Id::unique(CLIPBOARD), fragment));
                self.state.paste_count = 0;
                self.state.status.clear();
                true
            }
            Err(error) => {
                self.state.status = error;
                false
            }
        }
    }

    pub(super) fn clipboard_action(&mut self, ctx: &Context, action: ClipboardAction, at: Option<Pos>) {
        if let Some(id) = ctx.memory(|m| m.focused()) {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
        match action {
            ClipboardAction::Copy => {
                self.copy(ctx);
            }
            ClipboardAction::Cut => {
                if self.copy(ctx) {
                    self.delete();
                }
            }
            ClipboardAction::Paste => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.state.pending_paste = Some((
                        at.unwrap_or_else(|| self.state.camera.to_world(self.state.canvas, self.state.canvas.center())),
                        ctx.input(|i| i.time) + 2.0,
                    ));
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                }
                #[cfg(target_arch = "wasm32")]
                {
                    if let Some(fragment) = ctx.data(|d| d.get_temp::<Fragment>(egui::Id::unique(CLIPBOARD))) {
                        self.paste_fragment(&fragment, at);
                    } else {
                        self.state.status = tr("drawing.paste_shortcut").into();
                    }
                }
            }
        }
    }

    fn paste_position(&mut self, at: Option<Pos>) -> Pos {
        if let Some(at) = at {
            return at;
        }
        self.state.paste_count = self.state.paste_count.saturating_add(1).min(1000);
        self.state.camera.to_world(self.state.canvas, self.state.canvas.center())
            + Pos::new(20.0 * self.state.paste_count as f32, 20.0 * self.state.paste_count as f32)
    }

    fn paste_fragment(&mut self, fragment: &Fragment, at: Option<Pos>) {
        let Some(bounds) = fragment.bounds() else { return };
        let offset = self.paste_position(at) - bounds.center();
        self.history.sync(&self.board, false);
        match self.board.paste(fragment, offset) {
            Ok(ids) => {
                self.state.selection = ids;
                self.state.tool = Tool::Select;
                self.state.status.clear();
            }
            Err(error) => self.state.status = error,
        }
        self.history.sync(&self.board, false);
    }

    pub(super) fn clipboard_events(&mut self, ui: &Ui, response: &egui::Response) {
        if self
            .state
            .pending_paste
            .is_some_and(|(_, deadline)| ui.input(|i| i.time) > deadline)
        {
            self.state.pending_paste = None;
        }
        if ui.ctx().egui_wants_keyboard_input() || !(response.hovered() || self.state.pending_paste.is_some()) {
            return;
        }
        let events = ui.input_mut(|i| {
            let (clipboard, remaining): (Vec<_>, Vec<_>) = std::mem::take(&mut i.events)
                .into_iter()
                .partition(|event| matches!(event, egui::Event::Copy | egui::Event::Cut | egui::Event::Paste(_)));
            i.events = remaining;
            clipboard
        });
        let command = ui.input(|i| i.modifiers.command);
        let copy = events.iter().any(|e| matches!(e, egui::Event::Copy)) || command && ui.input(|i| i.key_pressed(Key::C));
        let cut = events.iter().any(|e| matches!(e, egui::Event::Cut)) || command && ui.input(|i| i.key_pressed(Key::X));
        if cut {
            if self.copy(ui.ctx()) {
                self.delete();
            }
        } else if copy {
            self.copy(ui.ctx());
        }
        let mut pasted = false;
        for event in events {
            let egui::Event::Paste(text) = event else { continue };
            pasted = true;
            let at = self.state.pending_paste.take().map(|(at, _)| at);
            match Fragment::from_clipboard(&text) {
                Ok(Some(fragment)) => {
                    let previous = ui.ctx().data(|d| d.get_temp::<Fragment>(egui::Id::unique(CLIPBOARD)));
                    if previous.as_ref() != Some(&fragment) {
                        self.state.paste_count = 0;
                    }
                    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique(CLIPBOARD), fragment.clone()));
                    self.paste_fragment(&fragment, at);
                }
                Ok(None) if !text.is_empty() => {
                    let position = self.paste_position(at);
                    let height = (text.lines().count().max(1) as f32 * self.state.paint.font_size * 1.5).clamp(40.0, 10_000.0);
                    let mut element = Element::new(Kind::Text, position, position + Pos::new(300.0, height), self.state.paint.clone());
                    element.text = text;
                    self.history.sync(&self.board, false);
                    self.add(element);
                    self.state.focus_text = false;
                    self.history.sync(&self.board, false);
                }
                Ok(None) => {}
                Err(error) => self.state.status = error,
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if !pasted && command && ui.input(|i| i.key_pressed(Key::V)) {
            self.clipboard_action(ui.ctx(), ClipboardAction::Paste, None);
        }
        #[cfg(target_arch = "wasm32")]
        let _ = pasted;
    }

    pub(super) fn reorder(&mut self, order: StackOrder) {
        if self.state.selection.is_empty() {
            return;
        }
        self.history.sync(&self.board, false);
        self.board.reorder(&self.state.selection, order);
        self.history.sync(&self.board, false);
    }

    pub(super) fn begin_move(&mut self, start: Pos, duplicate: bool) -> bool {
        let original_selection = duplicate.then(|| self.state.selection.clone());
        if duplicate {
            match self.board.paste(&self.board.fragment(&self.state.selection), Pos::default()) {
                Ok(ids) => self.state.selection = ids,
                Err(error) => {
                    self.state.status = error;
                    return false;
                }
            }
        }
        self.board.prepare_move(&self.state.selection);
        let origins = self
            .board
            .elements
            .iter()
            .filter(|e| self.state.selection.contains(&e.id))
            .map(|e| (e.id, Pos::new(e.bounds.x, e.bounds.y)))
            .collect();
        self.state.drag = Some(Drag::Move {
            start,
            origins,
            original_selection,
        });
        true
    }

    pub(super) fn context_menu(&mut self, ui: &mut Ui, at: Pos) {
        crate::theme::menu_style(ui.style_mut());
        let selected = !self.state.selection.is_empty();
        for (label, key, action) in [
            (key("common.copy"), Key::C, ClipboardAction::Copy),
            (key("common.cut"), Key::X, ClipboardAction::Cut),
            (key("common.paste"), Key::V, ClipboardAction::Paste),
        ] {
            let enabled = selected || matches!(action, ClipboardAction::Paste);
            let shortcut = ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, key));
            if ui
                .add_enabled(enabled, egui::Button::new(tr(label)).shortcut_text(shortcut))
                .clicked()
            {
                self.clipboard_action(ui.ctx(), action, Some(at));
                ui.close();
            }
        }
        ui.separator();
        if ui
            .add_enabled(selected, egui::Button::new(tr("common.duplicate_selection")))
            .clicked()
        {
            self.duplicate();
            ui.close();
        }
        if ui.add_enabled(selected, egui::Button::new(tr("common.delete_selection"))).clicked() {
            self.delete();
            ui.close();
        }
        ui.menu_button(tr("drawing.order_selection"), |ui| {
            crate::theme::menu_style(ui.style_mut());
            for (label, order) in [
                (key("drawing.bring_to_front"), StackOrder::ToFront),
                (key("drawing.bring_forward"), StackOrder::Forward),
                (key("drawing.send_backward"), StackOrder::Backward),
                (key("drawing.send_to_back"), StackOrder::ToBack),
            ] {
                if ui.add_enabled(selected, egui::Button::new(tr(label))).clicked() {
                    self.reorder(order);
                    ui.close();
                }
            }
        });
        ui.separator();
        if ui.button(tr("common.select_all")).clicked() {
            self.state.selection = self.board.elements.iter().map(|e| e.id).collect();
            self.state.tool = Tool::Select;
            ui.close();
        }
    }
}
