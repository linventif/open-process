//! Application shell: documents/tabs, menus, files, persistence.

use crate::dock::{self, Pane, View};
use crate::editor::*;
use crate::io::{self, FileIo};
use crate::theme;
use egui::{Key, KeyboardShortcut, Modifiers, RichText, Ui};
use op_core::document::Document;
use op_core::i18n::{tr, trf};

mod new_document;
mod thumbnails;

#[cfg(test)]
use crate::{bpmn_editor::BpmnEditor, petri_editor::PetriEditor, uml_editor::UmlEditor};
#[cfg(test)]
use op_core::{
    bpmn::BpmnDiagram,
    document::{self, LogDocument, Model},
    examples,
    petri::PetriNet,
    uml::{DiagramKind, UmlDiagram},
};

#[cfg(test)]
use crate::platform::persistence::LegacyV1 as Persisted;
use crate::platform::persistence::{self, Workspace};

/// Things the panes ask the application to do (handled after the layout pass).
enum DockEvent {
    Activate(u64),
    Close(u64),
    ClosePane(u64, egui_tiles::TileId),
    Host(u64, HostEvent),
    New(Option<egui_tiles::TileId>),
    Beside(Pane, bool),
}

pub struct OpenProcessApp {
    registry: &'static DiagramRegistry,
    docs: Vec<Doc>,
    active: usize,
    tree: egui_tiles::Tree<Pane>,
    io: FileIo,
    message: Option<(String, bool)>,
    show_about: bool,
    show_shortcuts: bool,
    confirm_close: Option<usize>,
    show_home: bool,
    new_picker: Option<new_document::Picker>,
    thumbnails: thumbnails::Thumbnails,
    persistence_ready: bool,
}

impl OpenProcessApp {
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<String>, registry: &'static DiagramRegistry) -> Self {
        crate::theme::apply(&cc.egui_ctx);
        crate::i18n::load(cc.storage);
        let mut app = Self {
            registry,
            docs: Vec::new(),
            active: 0,
            tree: dock::new_tree(),
            io: FileIo::default(),
            message: None,
            show_about: false,
            show_shortcuts: false,
            confirm_close: None,
            show_home: true,
            new_picker: None,
            thumbnails: thumbnails::Thumbnails::default(),
            persistence_ready: true,
        };
        if let Some(storage) = cc.storage {
            let restored = match persistence::load(storage) {
                Ok(p) => p,
                Err(e) => {
                    app.message = Some((e, true));
                    app.persistence_ready = false;
                    None
                }
            };
            if let Some(p) = restored {
                // Restore documents, then remap the pane ids of the saved layout.
                let mut map = std::collections::HashMap::new();
                for (old, m, path, views) in p.docs {
                    let mut d = Doc::new(m, path, false, registry);
                    d.editor.restore_workspace(&views);
                    map.insert(old, d.uid);
                    app.docs.push(d);
                }
                app.tree = p.tree;
                let mut orphans = Vec::new();
                for (id, t) in app.tree.tiles.iter_mut() {
                    if let egui_tiles::Tile::Pane(pane) = t {
                        match map.get(&pane.doc) {
                            Some(&n) => pane.doc = n,
                            None => orphans.push(*id),
                        }
                    }
                }
                for id in orphans {
                    dock::remove_tile(&mut app.tree, id);
                }
                // Documents without a pane get a tab.
                let uids: Vec<u64> = app.docs.iter().map(|d| d.uid).collect();
                for uid in uids {
                    if dock::find(
                        &app.tree,
                        Pane {
                            doc: uid,
                            view: View::Editor,
                        },
                    )
                    .is_none()
                    {
                        dock::add_tab(
                            &mut app.tree,
                            Pane {
                                doc: uid,
                                view: View::Editor,
                            },
                            None,
                        );
                    }
                }
                app.active = p.active;
                app.thumbnails.restore(storage, &cc.egui_ctx, &map, &app.docs);
            }
        }
        for f in files {
            #[cfg(not(target_arch = "wasm32"))]
            match io::read_file(&f) {
                Ok(bytes) => app.io.push((f, bytes)),
                Err(e) => app.message = Some((trf("file.open_failed", &[&f, &e]), true)),
            }
            #[cfg(target_arch = "wasm32")]
            let _ = f;
        }
        app.active = app.active.min(app.docs.len().saturating_sub(1));
        app.set_active(app.active);
        app.show_home = true;
        app
    }

    #[cfg(test)]
    fn open_model(&mut self, model: Model, path: Option<String>, saved: bool) {
        self.open_document(model.document(), path, saved);
    }

    fn open_document(&mut self, model: Document, path: Option<String>, saved: bool) {
        let near = self.docs.get(self.active).and_then(|d| {
            dock::find(
                &self.tree,
                Pane {
                    doc: d.uid,
                    view: View::Editor,
                },
            )
        });
        self.open_document_near(model, path, saved, near);
    }

    fn open_document_near(&mut self, model: Document, path: Option<String>, saved: bool, near: Option<egui_tiles::TileId>) {
        self.open_stored(StoredDocument::Editable(model), path, saved, near);
    }

    fn open_stored(&mut self, state: StoredDocument, path: Option<String>, saved: bool, near: Option<egui_tiles::TileId>) {
        let doc = Doc::new(state, path, saved, self.registry);
        dock::add_tab(
            &mut self.tree,
            Pane {
                doc: doc.uid,
                view: View::Editor,
            },
            near,
        );
        self.docs.push(doc);
        self.active = self.docs.len() - 1;
        self.show_home = false;
    }

    /// Makes document `i` the active one and brings its editor tab to the front.
    pub fn set_active(&mut self, i: usize) {
        let Some(d) = self.docs.get(i) else { return };
        self.active = i;
        self.show_home = false;
        let pane = Pane {
            doc: d.uid,
            view: View::Editor,
        };
        let tile = dock::find(&self.tree, pane.clone()).unwrap_or_else(|| dock::add_tab(&mut self.tree, pane, None));
        dock::activate(&mut self.tree, tile);
    }

    /// Shows a view of the active document in a new pane beside the current one.
    fn open_beside(&mut self, pane: Pane, vertical: bool) {
        self.show_home = false;
        let near = self.docs.get(self.active).and_then(|d| {
            dock::find(
                &self.tree,
                Pane {
                    doc: d.uid,
                    view: View::Editor,
                },
            )
        });
        dock::add_beside(&mut self.tree, pane, near, vertical);
    }

    fn load_bytes(&mut self, name: String, bytes: &[u8]) {
        if bytes.len() > op_core::document::MAX_DOCUMENT_BYTES {
            self.message = Some((trf("file.open_failed", &[&io::file_name(&name), &tr("file.input_limit")]), true));
            return;
        }
        let native = name.to_lowercase().ends_with(".opx")
            || name.to_lowercase().ends_with(".json")
            || serde_json::from_slice::<serde_json::Value>(bytes).is_ok_and(|v| v["format"] == "open-process");
        let state = match self.registry.load(bytes) {
            Ok(d) => match self.registry.open(&d) {
                Ok(mut editor) => {
                    if ["Imported process", "Imported net", "net1", "Imported log"].contains(&editor.name()) {
                        editor.rename(io::file_name(&name).rsplit_once('.').map(|x| x.0).unwrap_or(&name).into());
                    }
                    editor.snapshot()
                }
                Err(e) => StoredDocument::Original {
                    bytes: bytes.to_vec(),
                    reason: e.to_string(),
                },
            },
            Err(e) => {
                if !native {
                    self.message = Some((format!("{}: {e}", io::file_name(&name)), true));
                    return;
                }
                StoredDocument::Original {
                    bytes: bytes.to_vec(),
                    reason: e.to_string(),
                }
            }
        };
        self.open_stored(state, native.then_some(name.clone()), native, None);
        self.message = Some((trf("app.opened", &[&io::file_name(&name)]), false));
    }

    fn save(&mut self, save_as: bool) -> bool {
        let Some(doc) = self.docs.get_mut(self.active) else { return false };
        if let Err(error) = doc.editor.validate_storage() {
            self.message = Some((trf("file.save_failed", &[&error]), true));
            return false;
        }
        let state = doc.snapshot();
        let data = state.bytes();
        if matches!(&state, StoredDocument::Editable(_)) && data.len() > op_core::document::MAX_DOCUMENT_BYTES {
            self.message = Some((trf("file.save_failed", &[&tr("file.input_limit")]), true));
            return false;
        }
        let result = match (&doc.path, save_as, cfg!(target_arch = "wasm32")) {
            (Some(path), false, false) => io::write_path(path, &data).map(|_| Some(io::SaveOutcome::Written(path.clone()))),
            _ => io::save_file(&format!("{}.opx", doc.file_stem()), &data),
        };
        match doc.complete_save(state, result) {
            Ok(Some(outcome)) => {
                self.message = Some((outcome.message(op_core::i18n::key("app.saved")), false));
                matches!(outcome, io::SaveOutcome::Written(_))
            }
            Ok(None) => false,
            Err(e) => {
                self.message = Some((trf("file.save_failed", &[&e]), true));
                false
            }
        }
    }

    fn export(&mut self, format: &str) {
        let Some(doc) = self.docs.get(self.active) else { return };
        let data = match doc.editor.export_as(format) {
            Ok(data) => data,
            Err(e) => {
                self.message = Some((e, true));
                return;
            }
        };
        let name = format!("{}.{}", doc.file_stem(), data.suffix);
        match io::save_file(&name, &data.bytes) {
            Ok(Some(outcome)) => self.message = Some((outcome.message(op_core::i18n::key("app.exported")), false)),
            Ok(None) => {}
            Err(e) => self.message = Some((trf("file.export_failed", &[&e]), true)),
        }
    }

    fn execute(&mut self, command: &str, time: f64) {
        let Some(doc) = self.docs.get_mut(self.active) else { return };
        let uid = doc.uid;
        let events = doc.editor.execute(command, time);
        for event in events {
            self.handle_dock_event(DockEvent::Host(uid, event));
        }
    }

    fn new_document(&mut self, kind: &str, near: Option<egui_tiles::TileId>) {
        if let Some(d) = self.registry.find(kind) {
            self.open_document_near((d.create)(self.docs.len() + 1), None, false, near);
        }
    }

    fn open_example(&mut self, id: &str) {
        self.open_example_near(id, None);
    }

    fn open_example_near(&mut self, id: &str, near: Option<egui_tiles::TileId>) {
        if let Some(d) = self.registry.example(id) {
            self.open_document_near(d, None, false, near);
        }
    }

    fn undo(&mut self) {
        self.execute(UNDO, 0.0);
    }

    fn redo(&mut self) {
        self.execute(REDO, 0.0);
    }

    fn close(&mut self, i: usize) {
        if i < self.docs.len() {
            let d = self.docs.remove(i);
            dock::remove_doc(&mut self.tree, d.uid);
        }
        if self.docs.is_empty() {
            self.show_home = true;
        }
        self.active = self.active.min(self.docs.len().saturating_sub(1));
    }

    fn home(&mut self, ui: &mut Ui) {
        use crate::home::{self, Action};
        let docs = self
            .docs
            .iter()
            .map(|doc| {
                let (kind, color) = doc.badge();
                home::Document {
                    uid: doc.uid,
                    name: doc.name(),
                    kind: kind.into(),
                    color,
                    dirty: doc.dirty(),
                    icon: self.registry.find(doc.editor.kind()).map(|d| d.icon).unwrap_or(crate::icons::note),
                    thumbnail: self.thumbnails.texture(doc.uid),
                }
            })
            .collect::<Vec<_>>();
        match home::show(ui, &docs, self.registry) {
            Some(Action::New(kind)) => self.new_document(kind, None),
            Some(Action::Example(id, command)) => {
                self.open_example(id);
                if let Some(command) = command {
                    self.execute(command, 0.0);
                }
            }
            Some(Action::Open) => self.io.open_dialog(ui.ctx(), self.registry.extensions()),
            Some(Action::Resume(uid)) => {
                if let Some(i) = self.docs.iter().position(|doc| doc.uid == uid) {
                    self.set_active(i);
                }
            }
            None => {}
        }
    }

    fn thumbnail_capture_blocked(&self, ctx: &egui::Context) -> bool {
        self.show_home
            || self.new_picker.is_some()
            || self.show_about
            || self.show_shortcuts
            || self.confirm_close.is_some()
            || egui::Popup::is_any_open(ctx)
            || ctx.memory(|m| m.top_modal_layer().is_some())
            || ctx.input(|i| i.pointer.any_down())
    }

    fn handle_dock_event(&mut self, ev: DockEvent) {
        match ev {
            DockEvent::Activate(uid) => {
                if let Some(i) = self.docs.iter().position(|d| d.uid == uid) {
                    self.active = i;
                }
            }
            DockEvent::ClosePane(uid, tile) => {
                if dock::panes_of(&self.tree, uid).len() > 1 {
                    dock::remove_tile(&mut self.tree, tile);
                } else {
                    self.handle_dock_event(DockEvent::Close(uid));
                }
            }
            DockEvent::Close(uid) => {
                if let Some(i) = self.docs.iter().position(|d| d.uid == uid) {
                    if self.docs[i].dirty() {
                        self.confirm_close = Some(i);
                    } else {
                        self.close(i);
                    }
                }
            }
            DockEvent::Host(uid, event) => match event {
                HostEvent::Open(d) => self.open_document(d, None, false),
                HostEvent::OpenView(view) => {
                    if let Some(i) = self.docs.iter().position(|d| d.uid == uid) {
                        self.active = i;
                    }
                    self.open_beside(Pane { doc: uid, view }, false);
                }
                HostEvent::Message(m) => self.message = Some((m, false)),
            },
            DockEvent::New(near) => self.open_new_picker(near),
            DockEvent::Beside(pane, vertical) => {
                if let Some(i) = self.docs.iter().position(|d| d.uid == pane.doc) {
                    self.active = i;
                }
                self.open_beside(pane, vertical);
            }
        }
    }

    fn shortcuts(&mut self, ui: &mut Ui) {
        if self.new_picker.is_some() {
            return;
        }
        let sc = |m: Modifiers, k: Key| KeyboardShortcut::new(m, k);
        let cmd = Modifiers::COMMAND;
        let cmd_shift = Modifiers::COMMAND | Modifiers::SHIFT;
        let (undo, redo, redo2, save, save_as, open, new) = ui.ctx().input_mut(|i| {
            (
                i.consume_shortcut(&sc(cmd, Key::Z)),
                i.consume_shortcut(&sc(cmd_shift, Key::Z)),
                i.consume_shortcut(&sc(cmd, Key::Y)),
                i.consume_shortcut(&sc(cmd, Key::S)),
                i.consume_shortcut(&sc(cmd_shift, Key::S)),
                i.consume_shortcut(&sc(cmd, Key::O)),
                i.consume_shortcut(&sc(cmd, Key::N)),
            )
        });
        if ui.ctx().egui_wants_keyboard_input() {
            // Let text fields handle their own undo.
        } else if undo && !self.show_home {
            self.undo();
        } else if (redo || redo2) && !self.show_home {
            self.redo();
        }
        if (save || save_as) && !self.show_home {
            self.save(save_as);
        }
        if open {
            self.io.open_dialog(ui.ctx(), self.registry.extensions());
        }
        if new {
            self.open_new_picker(None);
        }
    }

    fn handle_dropped_files(&mut self, ui: &Ui) {
        let dropped = ui.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let name = f.path().display().to_string();
            #[cfg(not(target_arch = "wasm32"))]
            match io::read_file(f.path()) {
                Ok(b) => self.io.push((name, b)),
                Err(e) => self.message = Some((trf("file.read_failed", &[&name, &e]), true)),
            }
            #[cfg(target_arch = "wasm32")]
            {
                let io = self.io.clone();
                let ctx = ui.ctx().clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Ok(b) = f.bytes_async().await {
                        io.push((name, b));
                        ctx.request_repaint();
                    }
                });
            }
        }
    }

    fn menu_bar(&mut self, ui: &mut Ui) {
        let has_doc = !self.show_home && !self.docs.is_empty();
        egui::MenuBar::new().style(theme::toolbar_style).ui(ui, |ui| {
            crate::brand::show(ui, 26.0);
            ui.label(RichText::new("Open Process").font(theme::semibold(14.0)));
            ui.add_space(10.0);
            if ui.selectable_label(self.show_home, tr("workspace.home")).clicked() {
                self.show_home = true;
            }
            ui.separator();
            ui.menu_button(tr("menu.file"), |ui| {
                if ui
                    .add(
                        egui::Button::new(tr("file.new"))
                            .shortcut_text(ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::N))),
                    )
                    .clicked()
                {
                    self.open_new_picker(None);
                    ui.close();
                }
                ui.separator();
                if ui
                    .add(
                        egui::Button::new(tr("file.open"))
                            .shortcut_text(ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O))),
                    )
                    .on_hover_text(tr("app.open_process_opx_bpmn_2_0_bpmn"))
                    .clicked()
                {
                    self.io.open_dialog(ui.ctx(), self.registry.extensions());
                }
                if ui
                    .add_enabled(
                        has_doc,
                        egui::Button::new(tr("file.save"))
                            .shortcut_text(ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S))),
                    )
                    .clicked()
                {
                    self.save(false);
                }
                if ui.add_enabled(has_doc, egui::Button::new(tr("file.save_as"))).clicked() {
                    self.save(true);
                }
                ui.add_enabled_ui(has_doc, |ui| {
                    ui.menu_button(tr("file.export"), |ui| {
                        let exports = self.docs.get(self.active).map(|d| d.editor.exports()).unwrap_or_default();
                        for export in exports {
                            if ui.button(tr(export.label)).clicked() {
                                self.export(export.id);
                                ui.close();
                            }
                        }
                        let png = self.docs.get(self.active).is_some_and(|d| d.editor.enabled(PNG));
                        if png {
                            ui.separator();
                            if ui
                                .button(tr("app.export_image_png"))
                                .on_hover_text(tr("app.picture_visible_diagram"))
                                .clicked()
                            {
                                self.execute(PNG, 0.0);
                                ui.close();
                            }
                        }
                    });
                });
                ui.separator();
                if ui
                    .add_enabled(!self.docs.is_empty() && !self.show_home, egui::Button::new(tr("app.close_tab")))
                    .clicked()
                {
                    self.confirm_close = Some(self.active);
                }
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button(tr("app.quit")).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(tr("menu.edit"), |ui| {
                if !has_doc {
                    ui.disable();
                }
                let can_undo = self.docs.get(self.active).is_some_and(|d| d.editor.enabled(UNDO));
                let can_redo = self.docs.get(self.active).is_some_and(|d| d.editor.enabled(REDO));
                if ui
                    .add_enabled(
                        can_undo,
                        egui::Button::new(tr("common.undo"))
                            .shortcut_text(ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Z))),
                    )
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(
                        can_redo,
                        egui::Button::new(tr("common.redo")).shortcut_text(
                            ui.ctx()
                                .format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)),
                        ),
                    )
                    .clicked()
                {
                    self.redo();
                }
                if ui
                    .add(egui::Button::new(tr("common.delete_selection")).shortcut_text("Del"))
                    .clicked()
                {
                    self.execute(DELETE, 0.0);
                }
            });
            ui.menu_button(tr("menu.view"), |ui| {
                if ui
                    .add_enabled(has_doc, egui::Button::new(tr("canvas.zoom_fit")).shortcut_text("F"))
                    .clicked()
                {
                    self.execute(FIT, 0.0);
                }
                ui.separator();
                let active = self.docs.get(self.active).map(|d| (d.uid, d.editor.views()));
                if let Some((uid, views)) = active
                    && !views.is_empty()
                {
                    ui.menu_button(tr("app.open_beside"), |ui| {
                        for view in views {
                            if ui.button(tr(view.label)).clicked() {
                                self.open_beside(
                                    Pane {
                                        doc: uid,
                                        view: View::from(view.id),
                                    },
                                    false,
                                );
                            }
                        }
                    });
                }
                ui.menu_button(tr("app.show_document_beside"), |ui| {
                    let others: Vec<(u64, String)> = self
                        .docs
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != self.active)
                        .map(|(_, d)| (d.uid, d.name()))
                        .collect();
                    if others.is_empty() {
                        ui.label(RichText::new(tr("app.open_another_document_first")).weak());
                    }
                    for (uid, name) in others {
                        if ui.button(&name).clicked() {
                            self.open_beside(
                                Pane {
                                    doc: uid,
                                    view: View::Editor,
                                },
                                false,
                            );
                        }
                    }
                });
                ui.label(RichText::new(tr("app.tip_drag_tab_onto_edge_pane_split")).small().weak());
                ui.separator();
                ui.label(tr("ui.theme"));
                egui::widgets::global_theme_preference_buttons(ui);
            });
            ui.menu_button(tr("menu.tools"), |ui| {
                let commands = self.docs.get(self.active).map(|d| d.editor.commands()).unwrap_or(&[]);
                for c in commands.iter().filter(|c| c.group == CommandGroup::Tools) {
                    let enabled = has_doc && self.docs.get(self.active).is_some_and(|d| d.editor.enabled(c.id));
                    if ui.add_enabled(enabled, egui::Button::new(tr(c.label))).clicked() {
                        self.execute(c.id, ui.input(|i| i.time));
                    }
                }
                ui.separator();
                if ui.button(tr("app.process_discovery_alpha_algorithm")).clicked() {
                    self.new_document("event-log", None);
                }
            });
            ui.menu_button(tr("menu.help"), |ui| {
                if ui.button(tr("app.keyboard_shortcuts")).clicked() {
                    self.show_shortcuts = true;
                }
                if ui.button(tr("app.about_open_process")).clicked() {
                    self.show_about = true;
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                theme_toggle(ui);
                crate::i18n::language_menu(ui);
            });
        });
    }

    fn dialogs(&mut self, ui: &Ui) {
        let ctx = ui.ctx().clone();
        self.new_document_dialog(&ctx);
        if self.show_about {
            egui::Window::new(tr("app.about_open_process"))
                .collapsible(false)
                .resizable(false)
                .open(&mut self.show_about)
                .show(&ctx, |ui| {
                    ui.horizontal(|ui| {
                        crate::brand::show(ui, 56.0);
                        ui.heading("Open Process");
                    });
                    ui.label(trf("app.version", &[&(env!("CARGO_PKG_VERSION"))]));
                    ui.label(trf("app.build", &[&env!("OP_BUILD_ID")]));
                    ui.label(tr("app.free_open_source_process_modelling_analysis_bpmn"));
                    ui.label(tr("app.written_rust_egui_runs_natively_windows_macos"));
                    ui.label(tr("app.license_mit_apache_2_0"));
                });
        }
        if self.show_shortcuts {
            egui::Window::new(tr("app.keyboard_shortcuts"))
                .collapsible(false)
                .open(&mut self.show_shortcuts)
                .show(&ctx, |ui| {
                    egui::Grid::new("shortcuts").striped(true).show(ui, |ui| {
                        for (k, v) in [
                            ("Ctrl+N / Ctrl+O / Ctrl+S", tr("app.new_document_open_save")),
                            ("Ctrl+Z / Ctrl+Shift+Z", tr("app.undo_redo")),
                            (tr("app.del_backspace"), tr("common.delete_selection")),
                            ("Esc", tr("app.back_select_tool")),
                            ("F", tr("canvas.zoom_fit")),
                            (tr("app.wheel_ctrl_wheel"), tr("app.scroll_zoom")),
                            (tr("app.right_middle_drag"), tr("app.pan")),
                            (tr("app.double_click_f2"), tr("app.rename")),
                            ("BPMN: V C S I E T X + O", tr("app.select_connect_start_intermediate_end_task_xor")),
                            ("Petri: V P T A K", tr("app.select_place_transition_arc_tokens")),
                            (tr("app.petri_s_space"), tr("app.token_game_random_step")),
                        ] {
                            ui.label(RichText::new(k).monospace());
                            ui.label(v);
                            ui.end_row();
                        }
                    });
                });
        }
        if let Some(i) = self.confirm_close {
            let mut keep = true;
            egui::Window::new(tr("app.confirm_close_tab"))
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    let name = self.docs.get(i).map(|d| d.name()).unwrap_or_default();
                    ui.label(trf("app.has_unsaved_changes", &[&name]));
                    ui.horizontal(|ui| {
                        if ui.button(tr("file.save")).clicked() {
                            self.active = i;
                            if self.save(false) {
                                self.close(i);
                                keep = false;
                            }
                        }
                        if ui.button(tr("app.close_without_saving")).clicked() {
                            self.close(i);
                            keep = false;
                        }
                        if ui.button(tr("common.cancel")).clicked() {
                            keep = false;
                        }
                    });
                });
            if !keep {
                self.confirm_close = None;
            }
        }
    }
}

/// Light / dark switch with a painted sun or moon.
fn theme_toggle(ui: &mut Ui) {
    let dark = ui.visuals().dark_mode;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 7.0, ui.visuals().widgets.hovered.bg_fill);
    }
    let c = rect.center();
    let col = ui.visuals().text_color();
    let p = ui.painter();
    if dark {
        // sun
        p.circle_filled(c, 4.0, col);
        for k in 0..8 {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            let d = egui::vec2(a.cos(), a.sin());
            p.line_segment([c + d * 6.5, c + d * 8.5], egui::Stroke::new(1.4, col));
        }
    } else {
        // moon
        p.circle_filled(c, 7.0, col);
        p.circle_filled(c + egui::vec2(3.5, -3.0), 6.0, ui.visuals().panel_fill);
    }
    let resp = resp.on_hover_text(if dark {
        tr("app.switch_light_theme")
    } else {
        tr("app.switch_dark_theme")
    });
    if resp.clicked() {
        ui.ctx().set_theme(if dark { egui::Theme::Light } else { egui::Theme::Dark });
    }
}

/// Bridges the split-view tree and the documents.
struct DockBehavior<'a> {
    docs: &'a mut Vec<Doc>,
    active_uid: Option<u64>,
    events: Vec<DockEvent>,
    previews: Vec<thumbnails::Source>,
}

impl DockBehavior<'_> {
    fn doc(&self, uid: u64) -> Option<&Doc> {
        self.docs.iter().find(|d| d.uid == uid)
    }
}

impl egui_tiles::Behavior<Pane> for DockBehavior<'_> {
    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: egui_tiles::TileId, pane: &mut Pane) -> egui_tiles::UiResponse {
        let pane_rect = ui.max_rect().intersect(ui.clip_rect());
        if ui.rect_contains_pointer(ui.max_rect()) && ui.input(|i| i.pointer.any_pressed()) && self.active_uid != Some(pane.doc) {
            self.events.push(DockEvent::Activate(pane.doc));
            self.active_uid = Some(pane.doc);
        }
        let Some(doc) = self.docs.iter_mut().find(|d| d.uid == pane.doc) else {
            return egui_tiles::UiResponse::None;
        };
        let view = if pane.view == View::Editor || doc.editor.views().iter().any(|v| v.id == pane.view.id()) {
            pane.view.id()
        } else {
            "editor"
        };
        for event in doc.editor.render(ui, _tile_id.0, view) {
            self.events.push(DockEvent::Host(pane.doc, event));
        }
        let rect = if view == "editor" {
            doc.editor.thumbnail_rect().unwrap_or(pane_rect).intersect(pane_rect)
        } else {
            pane_rect
        };
        self.previews.push(thumbnails::Source {
            uid: pane.doc,
            rect,
            view: view.to_string(),
        });
        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        let name = self.doc(pane.doc).map(|d| d.name()).unwrap_or_default();
        if pane.view == View::Editor {
            name.into()
        } else {
            let label = self.doc(pane.doc).map(|d| d.view_label(pane.view.id())).unwrap_or("");
            format!("{name} · {label}").into()
        }
    }

    fn retain_pane(&mut self, pane: &Pane) -> bool {
        self.docs.iter().any(|d| d.uid == pane.doc)
    }

    fn is_tab_closable(&self, _tiles: &egui_tiles::Tiles<Pane>, _tile_id: egui_tiles::TileId) -> bool {
        true
    }

    fn tab_bar_height(&self, _style: &egui::Style) -> f32 {
        36.0
    }

    fn gap_width(&self, _style: &egui::Style) -> f32 {
        3.0
    }

    fn tab_bar_color(&self, visuals: &egui::Visuals) -> egui::Color32 {
        visuals.panel_fill
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }

    fn tab_ui(
        &mut self,
        tiles: &mut egui_tiles::Tiles<Pane>,
        ui: &mut Ui,
        id: egui::Id,
        tile_id: egui_tiles::TileId,
        state: &egui_tiles::TabState,
    ) -> egui::Response {
        let pane = match tiles.get(tile_id) {
            Some(egui_tiles::Tile::Pane(p)) => Some(p.clone()),
            _ => None,
        };
        let (kind, color, title, dirty) = match pane.as_ref().and_then(|p| self.doc(p.doc).map(|d| (p, d))) {
            Some((p, d)) => {
                let (k, c) = d.badge();
                let t = if p.view == View::Editor {
                    d.name()
                } else {
                    format!("{} · {}", d.name(), d.view_label(p.view.id()))
                };
                (k, c, t, d.dirty() && p.view == View::Editor)
            }
            None => (
                "",
                ui.visuals().weak_text_color(),
                self.tab_title_for_tile(tiles, tile_id).text().to_string(),
                false,
            ),
        };
        let v = ui.visuals().clone();
        let active = state.active;
        let title_color = if active { v.text_color() } else { v.weak_text_color() };
        let title_g = ui.painter().layout_no_wrap(title.clone(), theme::medium(13.0), title_color);
        let badge_g = ui.painter().layout_no_wrap(kind.to_owned(), theme::semibold(9.5), color);
        let badge_w = if kind.is_empty() { 0.0 } else { badge_g.size().x + 10.0 };
        let title_w = title_g.size().x.min(240.0);
        let w = 12.0 + badge_w + 8.0 + title_w + 10.0 + 18.0 + 8.0;
        let (_, rect) = ui.allocate_space(egui::vec2(w, ui.available_height()));
        let resp = ui
            .interact(rect, id, egui::Sense::click_and_drag())
            .on_hover_cursor(egui::CursorIcon::Grab);
        resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, title.clone()));
        let hovered = resp.hovered();
        let close_rect = egui::Rect::from_center_size(egui::pos2(rect.right() - 17.0, rect.center().y + 1.0), egui::vec2(18.0, 18.0));
        let over_close = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| close_rect.contains(p));
        if ui.is_rect_visible(rect) && !state.is_being_dragged {
            let p = ui.painter();
            let rect = rect.with_min_y(rect.top() + 3.0);
            let top_round = egui::CornerRadius {
                nw: 9,
                ne: 9,
                sw: 0,
                se: 0,
            };
            if active {
                p.rect_filled(rect, top_round, v.window_fill);
                let b = egui::Stroke::new(1.0, v.widgets.noninteractive.bg_stroke.color);
                p.line_segment([rect.left_bottom(), rect.left_top() + egui::vec2(0.0, 8.0)], b);
                p.line_segment([rect.right_bottom(), rect.right_top() + egui::vec2(0.0, 8.0)], b);
                p.line_segment([rect.left_top() + egui::vec2(8.0, 0.0), rect.right_top() - egui::vec2(8.0, 0.0)], b);
                p.line_segment(
                    [rect.left_top() + egui::vec2(9.0, 1.0), rect.right_top() + egui::vec2(-9.0, 1.0)],
                    egui::Stroke::new(2.0, color),
                );
                p.hline(rect.x_range(), rect.bottom(), egui::Stroke::new(2.0, v.window_fill));
            } else if hovered {
                p.rect_filled(rect.shrink2(egui::vec2(0.0, 3.0)), 8.0, v.widgets.hovered.bg_fill);
            }
            let cy = rect.center().y;
            let mut x = rect.left() + 12.0;
            if !kind.is_empty() {
                let badge = egui::Rect::from_min_size(egui::pos2(x, cy - 8.0), egui::vec2(badge_w, 16.0));
                p.rect_filled(badge, 5.0, color.gamma_multiply(if v.dark_mode { 0.28 } else { 0.14 }));
                p.galley(badge.center() - badge_g.size() / 2.0, badge_g, color);
                x = badge.right() + 8.0;
            }
            let title_pos = egui::pos2(x, cy - title_g.size().y / 2.0);
            p.with_clip_rect(egui::Rect::from_min_size(title_pos, egui::vec2(title_w, 24.0)))
                .galley(title_pos, title_g, title_color);
            if hovered || active {
                if over_close {
                    p.rect_filled(close_rect, 5.0, v.widgets.hovered.bg_fill);
                }
                let s = egui::Stroke::new(1.3, if over_close { v.text_color() } else { v.weak_text_color() });
                let c = close_rect.center();
                p.line_segment([c + egui::vec2(-3.5, -3.5), c + egui::vec2(3.5, 3.5)], s);
                p.line_segment([c + egui::vec2(-3.5, 3.5), c + egui::vec2(3.5, -3.5)], s);
            } else if dirty {
                p.circle_filled(close_rect.center(), 3.5, v.weak_text_color());
            }
        }
        let close = (resp.clicked() && over_close) || resp.middle_clicked();
        if let Some(p) = pane {
            if resp.clicked() && !over_close {
                self.events.push(DockEvent::Activate(p.doc));
            }
            if close {
                self.events.push(DockEvent::ClosePane(p.doc, tile_id));
            }
            resp.context_menu(|ui| {
                if ui.button(tr("app.split_right")).clicked() {
                    self.events.push(DockEvent::Beside(p.clone(), false));
                    ui.close();
                }
                if ui.button(tr("app.split_down")).clicked() {
                    self.events.push(DockEvent::Beside(p.clone(), true));
                    ui.close();
                }
                let views = self.doc(p.doc).map(|d| d.editor.views()).unwrap_or(&[]);
                if !views.is_empty() {
                    ui.separator();
                    for view in views {
                        if ui
                            .button(trf("app.open_document_beside", &[&tr(view.label).to_lowercase()]))
                            .clicked()
                        {
                            self.events.push(DockEvent::Beside(
                                Pane {
                                    doc: p.doc,
                                    view: View::from(view.id),
                                },
                                false,
                            ));
                            ui.close();
                        }
                    }
                }
                ui.separator();
                if ui.button(tr("common.close")).clicked() {
                    self.events.push(DockEvent::ClosePane(p.doc, tile_id));
                    ui.close();
                }
            });
        }
        let hint = if dirty {
            trf("app.unsaved_changes_drag_move_split", &[&title])
        } else {
            trf("app.drag_move_split", &[&title])
        };
        resp.on_hover_text(hint)
    }

    fn tab_bar_trailing_ui(&mut self, tiles: &egui_tiles::Tiles<Pane>, ui: &mut Ui, _tile_id: egui_tiles::TileId, tabs: &egui_tiles::Tabs) {
        ui.add_space(6.0);
        let near = tabs.children.first().copied().filter(|c| tiles.get(*c).is_some());
        let response = ui
            .button(RichText::new("+").font(theme::semibold(16.0)))
            .on_hover_text(tr("file.new_document"));
        response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tr("file.new_document")));
        if response.clicked() {
            self.events.push(DockEvent::New(near));
        }
    }
}

impl eframe::App for OpenProcessApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.thumbnails.poll(ui.ctx(), &self.docs);
        let preview_was_blocked = self.thumbnail_capture_blocked(ui.ctx());
        for (name, bytes) in self.io.poll() {
            self.load_bytes(name, &bytes);
        }
        for error in self.io.poll_errors() {
            self.message = Some((error, true));
        }
        match io::poll_png_export(ui.ctx()) {
            Some(Ok(msg)) if !msg.is_empty() => self.message = Some((msg, false)),
            Some(Err(e)) => self.message = Some((trf("file.image_export_failed", &[&e]), true)),
            _ => {}
        }
        self.handle_dropped_files(ui);
        self.shortcuts(ui);

        let panel = ui.visuals().panel_fill;
        egui::Panel::top("menu")
            .frame(egui::Frame::new().fill(panel).inner_margin(egui::Margin {
                left: 12,
                right: 12,
                top: 6,
                bottom: 2,
            }))
            .show_separator_line(false)
            .show(ui, |ui| self.menu_bar(ui));
        if let Some((msg, is_error)) = self.message.clone() {
            egui::Panel::bottom("message").show(ui, |ui| {
                ui.horizontal(|ui| {
                    let color = if is_error {
                        ui.visuals().error_fg_color
                    } else {
                        ui.visuals().text_color()
                    };
                    ui.label(RichText::new(msg).color(color));
                    if ui.small_button("×").clicked() {
                        self.message = None;
                    }
                });
            });
        }

        if self.show_home {
            self.thumbnails.invalidate_scene();
            egui::CentralPanel::default()
                .frame(egui::Frame::new().fill(panel))
                .show(ui, |ui| self.home(ui));
            self.dialogs(ui);
            return;
        }

        let active_uid = self.docs.get(self.active).map(|d| d.uid);
        let mut behavior = DockBehavior {
            docs: &mut self.docs,
            active_uid,
            events: Vec::new(),
            previews: Vec::new(),
        };
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(panel).inner_margin(egui::Margin {
                left: 6,
                right: 6,
                top: 2,
                bottom: 0,
            }))
            .show(ui, |ui| self.tree.ui(&mut behavior, ui));
        let events = std::mem::take(&mut behavior.events);
        let previews = std::mem::take(&mut behavior.previews);
        for ev in events {
            self.handle_dock_event(ev);
        }
        // Documents whose last pane was closed by dragging are closed too.
        let orphan: Vec<usize> = (0..self.docs.len())
            .filter(|&i| dock::panes_of(&self.tree, self.docs[i].uid).is_empty())
            .collect();
        for i in orphan.into_iter().rev() {
            self.close(i);
        }
        for doc in &mut self.docs {
            let instances: Vec<_> = dock::panes_of(&self.tree, doc.uid).into_iter().map(|id| id.0).collect();
            doc.editor.retain_views(&instances);
        }
        self.dialogs(ui);
        let blocked = preview_was_blocked || self.thumbnail_capture_blocked(ui.ctx());
        self.thumbnails.capture(ui.ctx(), &self.docs, previews, blocked);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let p = Workspace {
            version: 3,
            docs: self
                .docs
                .iter()
                .map(|d| (d.uid, d.snapshot(), d.path.clone(), d.editor.workspace()))
                .collect(),
            active: self.active,
            tree: self.tree.clone(),
        };
        if self.persistence_ready {
            storage.set_string(persistence::KEY, serde_json::to_string(&p).expect("workspace is serializable"));
            self.thumbnails.save(storage, &self.docs);
        }
        crate::i18n::save(storage);
    }
}

#[cfg(test)]
mod tests;
