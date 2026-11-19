//! Default selection actions for graphical builders. Models and fragments stay typed.
use super::{CommandDescriptor, CommandGroup};
use egui::{Context, Event, Key, Response, Ui};
use op_core::{
    geom::Pos,
    i18n::{key, tr},
};
use serde::{Serialize, de::DeserializeOwned};
use std::hash::{Hash, Hasher};

pub const COPY: &str = "selection.copy";
pub const CUT: &str = "selection.cut";
pub const PASTE: &str = "selection.paste";
pub const DUPLICATE: &str = "selection.duplicate";
pub const SELECT_ALL: &str = "selection.all";
pub const COPY_COMMAND: CommandDescriptor = CommandDescriptor {
    id: COPY,
    label: key("common.copy"),
    group: CommandGroup::Edit,
};
pub const CUT_COMMAND: CommandDescriptor = CommandDescriptor {
    id: CUT,
    label: key("common.cut"),
    group: CommandGroup::Edit,
};
pub const PASTE_COMMAND: CommandDescriptor = CommandDescriptor {
    id: PASTE,
    label: key("common.paste"),
    group: CommandGroup::Edit,
};
pub const DUPLICATE_COMMAND: CommandDescriptor = CommandDescriptor {
    id: DUPLICATE,
    label: key("common.duplicate"),
    group: CommandGroup::Edit,
};
pub const SELECT_ALL_COMMAND: CommandDescriptor = CommandDescriptor {
    id: SELECT_ALL,
    label: key("common.select_all"),
    group: CommandGroup::Edit,
};
const LOCAL: &str = "open-process-selection-clipboard";
const PREFIX: &str = "open-process/selection/1/";

#[derive(Clone, Default)]
pub struct ClipboardState {
    pending: Option<&'static str>,
    paste_deadline: Option<f64>,
    paste_count: u32,
    previous: Option<u64>,
}

/// Consume canvas clipboard events once, while leaving focused text fields alone.
pub fn take_events(ui: &Ui, response: &Response, pending: bool) -> Vec<Event> {
    if ui.ctx().egui_wants_keyboard_input() || !(response.hovered() || pending) {
        return vec![];
    }
    ui.input_mut(|i| {
        let (clipboard, remaining) = std::mem::take(&mut i.events)
            .into_iter()
            .partition(|e| matches!(e, Event::Copy | Event::Cut | Event::Paste(_)));
        i.events = remaining;
        clipboard
    })
}

pub trait CanvasClipboard {
    type Fragment: Clone + Serialize + DeserializeOwned;
    fn clipboard_state(&mut self) -> &mut ClipboardState;
    fn clipboard_kind(&self) -> &'static str;
    fn has_selection(&self) -> bool;
    fn fragment(&self) -> Self::Fragment;
    /// `commit = false` participates in the current drag transaction.
    fn paste_fragment(&mut self, fragment: &Self::Fragment, delta: Pos, commit: bool);
    fn cut_selection(&mut self);
    fn select_everything(&mut self);
    fn editing_allowed(&self) -> bool {
        true
    }

    fn selection_command_enabled(&self, command: &str) -> Option<bool> {
        match command {
            COPY | CUT | DUPLICATE => Some(self.editing_allowed() && self.has_selection()),
            PASTE | SELECT_ALL => Some(self.editing_allowed()),
            _ => None,
        }
    }
    fn selection_command(&mut self, command: &str) -> bool {
        if let Some(enabled) = self.selection_command_enabled(command) {
            if enabled {
                match command {
                    DUPLICATE => self.duplicate_selection(Pos::new(30.0, 30.0), true),
                    SELECT_ALL => self.select_everything(),
                    COPY => self.clipboard_state().pending = Some(COPY),
                    CUT => self.clipboard_state().pending = Some(CUT),
                    PASTE => self.clipboard_state().pending = Some(PASTE),
                    _ => {}
                }
            }
            true
        } else {
            false
        }
    }
    fn duplicate_selection(&mut self, delta: Pos, commit: bool) {
        if self.editing_allowed() && self.has_selection() {
            self.paste_fragment(&self.fragment(), delta, commit);
        }
    }
    fn copy_selection(&mut self, ctx: &Context) -> bool {
        if !self.has_selection() {
            return false;
        }
        let Ok(json) = serde_json::to_string(&self.fragment()) else {
            return false;
        };
        let text = format!("{PREFIX}{}\n{json}", self.clipboard_kind());
        ctx.copy_text(text.clone());
        ctx.data_mut(|d| d.insert_temp(egui::Id::unique(LOCAL), text));
        self.clipboard_state().paste_count = 0;
        true
    }
    fn paste_text(&mut self, ctx: &Context, text: String) {
        let Some((format, json)) = text.split_once('\n') else {
            return;
        };
        if format != format!("{PREFIX}{}", self.clipboard_kind()) || json.len() > 16 * 1024 * 1024 {
            return;
        }
        let Ok(fragment) = serde_json::from_str::<Self::Fragment>(json) else {
            return;
        };
        let state = self.clipboard_state();
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        text.hash(&mut hash);
        let hash = hash.finish();
        if state.previous != Some(hash) {
            state.paste_count = 0;
            state.previous = Some(hash);
        }
        state.paste_count = state.paste_count.saturating_add(1).min(1000);
        let offset = 30.0 * state.paste_count as f32;
        ctx.data_mut(|d| d.insert_temp(egui::Id::unique(LOCAL), text));
        self.paste_fragment(&fragment, Pos::new(offset, offset), true);
    }
    fn clipboard_action(&mut self, ctx: &Context, action: &str) {
        if !self.editing_allowed() {
            return;
        }
        if let Some(id) = ctx.memory(|m| m.focused()) {
            ctx.memory_mut(|m| m.surrender_focus(id));
        }
        match action {
            COPY => {
                self.copy_selection(ctx);
            }
            CUT => {
                if self.copy_selection(ctx) {
                    self.cut_selection();
                }
            }
            PASTE => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.clipboard_state().paste_deadline = Some(ctx.input(|i| i.time) + 2.0);
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                }
                #[cfg(target_arch = "wasm32")]
                if let Some(text) = ctx.data(|d| d.get_temp::<String>(egui::Id::unique(LOCAL))) {
                    self.paste_text(ctx, text);
                }
            }
            DUPLICATE => self.duplicate_selection(Pos::new(30.0, 30.0), true),
            SELECT_ALL => self.select_everything(),
            _ => {}
        }
    }
    fn clipboard_ui(&mut self, ui: &Ui, response: &Response) {
        if !self.editing_allowed() {
            return;
        }
        if let Some(action) = self.clipboard_state().pending.take() {
            if let Some(id) = ui.ctx().memory(|m| m.focused()) {
                ui.ctx().memory_mut(|m| m.surrender_focus(id));
            }
            self.clipboard_action(ui.ctx(), action);
        }
        if self
            .clipboard_state()
            .paste_deadline
            .is_some_and(|deadline| ui.input(|i| i.time) > deadline)
        {
            self.clipboard_state().paste_deadline = None;
        }
        let pending = self.clipboard_state().paste_deadline.is_some();
        if ui.ctx().egui_wants_keyboard_input() || !(response.hovered() || pending) {
            return;
        }
        let events = take_events(ui, response, pending);
        let command = ui.input(|i| i.modifiers.command);
        let copy = events.iter().any(|e| matches!(e, Event::Copy)) || command && ui.input(|i| i.key_pressed(Key::C));
        let cut = events.iter().any(|e| matches!(e, Event::Cut)) || command && ui.input(|i| i.key_pressed(Key::X));
        if cut {
            self.clipboard_action(ui.ctx(), CUT);
        } else if copy {
            self.clipboard_action(ui.ctx(), COPY);
        }
        let mut pasted = false;
        for event in events {
            if let Event::Paste(text) = event {
                pasted = true;
                self.clipboard_state().paste_deadline = None;
                self.paste_text(ui.ctx(), text);
            }
        }
        if !pasted && command && ui.input(|i| i.key_pressed(Key::V)) {
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.clipboard_action(ui.ctx(), PASTE);
            }
        }
        if command && ui.input(|i| i.key_pressed(Key::D)) {
            self.clipboard_action(ui.ctx(), DUPLICATE);
        }
        if command && ui.input(|i| i.key_pressed(Key::A)) {
            self.select_everything();
        }
    }
    fn selection_menu(&mut self, ui: &mut Ui) {
        for descriptor in [COPY_COMMAND, CUT_COMMAND, PASTE_COMMAND, DUPLICATE_COMMAND, SELECT_ALL_COMMAND] {
            let mut button = egui::Button::new(tr(descriptor.label));
            let key = match descriptor.id {
                COPY => Some(Key::C),
                CUT => Some(Key::X),
                PASTE => Some(Key::V),
                DUPLICATE => Some(Key::D),
                SELECT_ALL => Some(Key::A),
                _ => None,
            };
            if let Some(key) = key {
                button = button.shortcut_text(
                    ui.ctx()
                        .format_shortcut(&egui::KeyboardShortcut::new(egui::Modifiers::COMMAND, key)),
                );
            }
            let response = ui.add_enabled(self.selection_command_enabled(descriptor.id).unwrap_or(false), button);
            response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, response.enabled(), tr(descriptor.label)));
            if response.clicked() {
                self.clipboard_action(ui.ctx(), descriptor.id);
                ui.close();
            }
        }
        ui.separator();
    }
}
