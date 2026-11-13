//! Application shell: documents/tabs, menus, files, persistence.

use crate::bpmn_editor::{BpmnAction, BpmnEditor};
use crate::dock::{self, Pane, View};
use crate::io::{self, FileIo};
use crate::mining_editor::{MiningAction, MiningEditor};
use crate::petri_editor::PetriEditor;
use crate::theme;
use egui::{Key, KeyboardShortcut, Modifiers, RichText, Ui};
use op_core::bpmn::BpmnDiagram;
use op_core::document::{self, LogDocument, Model};
use op_core::i18n::{tr, trf};
use op_core::petri::PetriNet;
use op_core::{bpmn_xml, convert, examples, pnml};
use serde::{Deserialize, Serialize};

pub enum Editor {
    Bpmn(Box<BpmnEditor>),
    Petri(Box<PetriEditor>),
    Mining(Box<MiningEditor>),
}

pub struct Doc {
    /// Stable identifier used by the panes of the split view.
    pub uid: u64,
    pub editor: Editor,
    /// Native: full path of the `.opx` file. Web: last downloaded name.
    pub path: Option<String>,
    saved: Option<Model>,
}

impl Doc {
    pub fn new(model: Model, path: Option<String>, saved: bool) -> Self {
        let snapshot = saved.then(|| model.clone());
        let editor = match model {
            Model::Bpmn(d) => Editor::Bpmn(Box::new(BpmnEditor::new(d))),
            Model::Petri(n) => Editor::Petri(Box::new(PetriEditor::new(n))),
            Model::Log(l) => Editor::Mining(Box::new(MiningEditor::new(l))),
        };
        Self {
            uid: crate::canvas::next_uid(),
            editor,
            path,
            saved: snapshot,
        }
    }

    pub fn model(&self) -> Model {
        match &self.editor {
            Editor::Bpmn(e) => Model::Bpmn(e.diagram.clone()),
            Editor::Petri(e) => Model::Petri(e.net.clone()),
            Editor::Mining(e) => Model::Log(LogDocument {
                name: e.name.clone(),
                text: e.text.clone(),
                manual: e.manual,
                answers: e.answers.clone(),
            }),
        }
    }

    fn name(&self) -> String {
        let n = match &self.editor {
            Editor::Bpmn(e) => e.diagram.name.clone(),
            Editor::Petri(e) => e.net.name.clone(),
            Editor::Mining(e) => e.name.clone(),
        };
        if n.trim().is_empty() { tr("Untitled").into() } else { n }
    }

    fn dirty(&self) -> bool {
        match (&self.saved, &self.editor) {
            (None, _) => true,
            (Some(Model::Bpmn(d)), Editor::Bpmn(e)) => *d != e.diagram,
            (Some(Model::Petri(n)), Editor::Petri(e)) => *n != e.net,
            (Some(Model::Log(l)), Editor::Mining(e)) => {
                l.name != e.name || l.text != e.text || l.manual != e.manual || l.answers != e.answers
            }
            _ => true,
        }
    }

    fn file_stem(&self) -> String {
        let s: String = self
            .name()
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect();
        s.trim_matches('_').to_string()
    }
}

#[derive(Serialize, Deserialize, Default)]
struct Persisted {
    docs: Vec<(Model, Option<String>)>,
    active: usize,
}

/// v2: documents with their pane identifiers + the split-view layout.
#[derive(Serialize, Deserialize)]
struct PersistedV2 {
    docs: Vec<(u64, Model, Option<String>)>,
    active: usize,
    tree: egui_tiles::Tree<Pane>,
}

const STORAGE_KEY: &str = "open-process-v1";
const STORAGE_KEY_V2: &str = "open-process-v2";

/// Things the panes ask the application to do (handled after the layout pass).
enum DockEvent {
    Activate(u64),
    Close(u64),
    Bpmn(u64, BpmnAction),
    Mining(MiningAction),
    New(&'static str, Option<egui_tiles::TileId>),
    Beside(Pane, bool),
}

pub struct OpenProcessApp {
    docs: Vec<Doc>,
    active: usize,
    tree: egui_tiles::Tree<Pane>,
    io: FileIo,
    message: Option<(String, bool)>,
    show_about: bool,
    show_shortcuts: bool,
    confirm_close: Option<usize>,
}

impl OpenProcessApp {
    pub fn new(cc: &eframe::CreationContext<'_>, files: Vec<String>) -> Self {
        crate::theme::apply(&cc.egui_ctx);
        crate::i18n::load(cc.storage);
        let mut app = Self {
            docs: Vec::new(),
            active: 0,
            tree: dock::new_tree(),
            io: FileIo::default(),
            message: None,
            show_about: false,
            show_shortcuts: false,
            confirm_close: None,
        };
        if let Some(storage) = cc.storage {
            if let Some(p) = eframe::get_value::<PersistedV2>(storage, STORAGE_KEY_V2) {
                // Restore documents, then remap the pane ids of the saved layout.
                let mut map = std::collections::HashMap::new();
                for (old, m, path) in p.docs {
                    let d = Doc::new(m, path, false);
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
            } else if let Some(p) = eframe::get_value::<Persisted>(storage, STORAGE_KEY) {
                for (m, path) in p.docs {
                    app.open_model(m, path, false);
                }
                app.active = p.active;
            }
        }
        if app.docs.is_empty() {
            app.open_model(Model::Bpmn(examples::order_process()), None, false);
        }
        for f in files {
            #[cfg(not(target_arch = "wasm32"))]
            match std::fs::read(&f) {
                Ok(bytes) => app.io.push((f, bytes)),
                Err(e) => app.message = Some((trf("Cannot open {}: {}", &[&f, &e]), true)),
            }
            #[cfg(target_arch = "wasm32")]
            let _ = f;
        }
        app.active = app.active.min(app.docs.len() - 1);
        app.set_active(app.active);
        app
    }

    fn open_model(&mut self, model: Model, path: Option<String>, saved: bool) {
        let near = self.docs.get(self.active).and_then(|d| {
            dock::find(
                &self.tree,
                Pane {
                    doc: d.uid,
                    view: View::Editor,
                },
            )
        });
        self.open_model_near(model, path, saved, near);
    }

    fn open_model_near(&mut self, model: Model, path: Option<String>, saved: bool, near: Option<egui_tiles::TileId>) {
        let doc = Doc::new(model, path, saved);
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
    }

    /// Makes document `i` the active one and brings its editor tab to the front.
    pub fn set_active(&mut self, i: usize) {
        let Some(d) = self.docs.get(i) else { return };
        self.active = i;
        let pane = Pane {
            doc: d.uid,
            view: View::Editor,
        };
        let tile = dock::find(&self.tree, pane).unwrap_or_else(|| dock::add_tab(&mut self.tree, pane, None));
        dock::activate(&mut self.tree, tile);
    }

    /// Shows a view of the active document in a new pane beside the current one.
    fn open_beside(&mut self, pane: Pane, vertical: bool) {
        if let Some(existing) = dock::find(&self.tree, pane) {
            dock::remove_tile(&mut self.tree, existing);
        }
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
        match document::load_any(bytes) {
            Ok(model) => {
                let is_opx = name.to_lowercase().ends_with(".opx") || name.to_lowercase().ends_with(".json");
                let mut model = model;
                // Use the file name as model name when the format has none
                let stem = io::file_name(&name).rsplit_once('.').map(|x| x.0).unwrap_or(&name).to_string();
                match &mut model {
                    Model::Bpmn(d) if d.name == "Imported process" => d.name = stem,
                    Model::Petri(n) if n.name == "Imported net" || n.name == "net1" => n.name = stem,
                    Model::Log(l) if l.name == "Imported log" => l.name = stem,
                    _ => {}
                }
                self.open_model(model, is_opx.then_some(name.clone()), is_opx);
                self.message = Some((trf("Opened {}", &[&(io::file_name(&name))]), false));
            }
            Err(e) => self.message = Some((format!("{}: {e}", io::file_name(&name)), true)),
        }
    }

    fn active_doc(&mut self) -> Option<&mut Doc> {
        self.docs.get_mut(self.active)
    }

    fn save(&mut self, save_as: bool) {
        let Some(doc) = self.docs.get_mut(self.active) else { return };
        let model = doc.model();
        let data = document::to_json(&model);
        let result = match (&doc.path, save_as, cfg!(target_arch = "wasm32")) {
            (Some(path), false, false) => io::write_path(path, data.as_bytes()).map(|_| Some(path.clone())),
            _ => io::save_file(&format!("{}.{}", doc.file_stem(), document::EXTENSION), data.as_bytes()),
        };
        match result {
            Ok(Some(path)) => {
                self.message = Some((trf("Saved {}", &[&(io::file_name(&path))]), false));
                doc.path = Some(path);
                doc.saved = Some(model);
            }
            Ok(None) => {}
            Err(e) => self.message = Some((trf("Save failed: {}", &[&e]), true)),
        }
    }

    fn export(&mut self) {
        let Some(doc) = self.docs.get(self.active) else { return };
        let (name, data) = match doc.model() {
            Model::Bpmn(d) => (format!("{}.bpmn", doc.file_stem()), bpmn_xml::export(&d)),
            Model::Petri(n) => (format!("{}.pnml", doc.file_stem()), pnml::export(&n)),
            Model::Log(l) => match op_core::mining::EventLog::parse(&l.text) {
                Ok(log) => (format!("{}-alpha.txt", doc.file_stem()), op_core::mining::alpha(&log).report(&log)),
                Err(e) => {
                    self.message = Some((e, true));
                    return;
                }
            },
        };
        match io::save_file(&name, data.as_bytes()) {
            Ok(Some(p)) => self.message = Some((trf("Exported {}", &[&(io::file_name(&p))]), false)),
            Ok(None) => {}
            Err(e) => self.message = Some((trf("Export failed: {}", &[&e]), true)),
        }
    }

    fn convert_active(&mut self) {
        let Some(Doc {
            editor: Editor::Bpmn(e), ..
        }) = self.docs.get(self.active)
        else {
            return;
        };
        let conv = convert::bpmn_to_petri(&e.diagram);
        let warnings = conv.warnings.clone();
        self.open_model(Model::Petri(conv.net), None, false);
        self.message = Some(if warnings.is_empty() {
            (
                tr("BPMN converted to a Petri net (see the Analysis tab for soundness)").into(),
                false,
            )
        } else {
            (trf("Converted with warnings: {}", &[&(warnings.join(" · "))]), false)
        });
    }

    fn new_log(&mut self) {
        let n = self.docs.iter().filter(|d| matches!(d.editor, Editor::Mining(_))).count() + 1;
        self.open_model(
            Model::Log(LogDocument {
                name: trf("Log {}", &[&n]),
                text: String::new(),
                ..Default::default()
            }),
            None,
            false,
        );
    }

    fn petri_to_bpmn(&mut self) {
        let Some(Doc {
            editor: Editor::Petri(e), ..
        }) = self.docs.get(self.active)
        else {
            return;
        };
        let (d, warnings) = convert::petri_to_bpmn(&e.net);
        self.open_model(Model::Bpmn(d), None, false);
        self.message = Some(if warnings.is_empty() {
            (tr("Petri net converted to BPMN").into(), false)
        } else {
            (trf("Converted with warnings: {}", &[&(warnings.join(" · "))]), false)
        });
    }

    fn undo(&mut self) {
        match self.active_doc().map(|d| &mut d.editor) {
            Some(Editor::Bpmn(e)) => e.undo(),
            Some(Editor::Petri(e)) => e.undo(),
            Some(Editor::Mining(_)) | None => {}
        }
    }

    fn redo(&mut self) {
        match self.active_doc().map(|d| &mut d.editor) {
            Some(Editor::Bpmn(e)) => e.redo(),
            Some(Editor::Petri(e)) => e.redo(),
            Some(Editor::Mining(_)) | None => {}
        }
    }

    fn close(&mut self, i: usize) {
        if i < self.docs.len() {
            let d = self.docs.remove(i);
            dock::remove_doc(&mut self.tree, d.uid);
        }
        if self.docs.is_empty() {
            self.open_model(Model::Bpmn(BpmnDiagram::new(trf("Process {}", &[&1]))), None, false);
        }
        self.active = self.active.min(self.docs.len() - 1);
    }

    fn handle_dock_event(&mut self, ev: DockEvent) {
        let index_of = |docs: &Vec<Doc>, uid: u64| docs.iter().position(|d| d.uid == uid);
        match ev {
            DockEvent::Activate(uid) => {
                if let Some(i) = index_of(&self.docs, uid) {
                    self.active = i;
                }
            }
            DockEvent::Close(uid) => {
                if let Some(i) = index_of(&self.docs, uid) {
                    if self.docs[i].dirty() && self.docs[i].saved.is_some() {
                        self.confirm_close = Some(i);
                    } else {
                        self.close(i);
                    }
                }
            }
            DockEvent::Bpmn(uid, BpmnAction::ConvertToPetri) => {
                if let Some(i) = index_of(&self.docs, uid) {
                    self.active = i;
                    self.convert_active();
                }
            }
            DockEvent::Mining(MiningAction::OpenPetri(net)) => self.open_model(Model::Petri(net), None, false),
            DockEvent::Mining(MiningAction::OpenBpmn(d)) => self.open_model(Model::Bpmn(d), None, false),
            DockEvent::New(kind, near) => {
                let n = self.docs.len() + 1;
                let model = match kind {
                    "bpmn" => Model::Bpmn(BpmnDiagram::new(trf("Process {}", &[&n]))),
                    "petri" => Model::Petri(PetriNet::new(trf("Net {}", &[&n]))),
                    _ => Model::Log(LogDocument {
                        name: trf("Log {}", &[&n]),
                        text: String::new(),
                        ..Default::default()
                    }),
                };
                self.open_model_near(model, None, false, near);
            }
            DockEvent::Beside(pane, vertical) => {
                if let Some(i) = index_of(&self.docs, pane.doc) {
                    self.active = i;
                }
                self.open_beside(pane, vertical);
            }
        }
    }

    fn shortcuts(&mut self, ui: &mut Ui) {
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
        } else if undo {
            self.undo();
        } else if redo || redo2 {
            self.redo();
        }
        if save || save_as {
            self.save(save_as);
        }
        if open {
            self.io.open_dialog(ui.ctx());
        }
        if new {
            let n = self.docs.len() + 1;
            self.open_model(Model::Bpmn(BpmnDiagram::new(trf("Process {}", &[&n]))), None, false);
        }
    }

    fn handle_dropped_files(&mut self, ui: &Ui) {
        let dropped = ui.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            let name = f.path().display().to_string();
            #[cfg(not(target_arch = "wasm32"))]
            match f.bytes() {
                Ok(b) => self.io.push((name, b)),
                Err(e) => self.message = Some((trf("Cannot read {}: {}", &[&name, &e]), true)),
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
        egui::MenuBar::new().ui(ui, |ui| {
            logo(ui);
            ui.label(RichText::new("Open Process").font(theme::semibold(14.0)));
            ui.add_space(10.0);
            ui.menu_button(tr("File"), |ui| {
                if ui
                    .add(
                        egui::Button::new(tr("New BPMN diagram"))
                            .shortcut_text(ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::N))),
                    )
                    .clicked()
                {
                    let n = self.docs.len() + 1;
                    self.open_model(Model::Bpmn(BpmnDiagram::new(trf("Process {}", &[&n]))), None, false);
                }
                if ui.button(tr("New Petri net")).clicked() {
                    let n = self.docs.len() + 1;
                    self.open_model(Model::Petri(PetriNet::new(trf("Net {}", &[&n]))), None, false);
                }
                if ui.button(tr("New event log (process discovery)")).clicked() {
                    self.new_log();
                }
                ui.separator();
                if ui
                    .add(
                        egui::Button::new(tr("Open…"))
                            .shortcut_text(ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::O))),
                    )
                    .on_hover_text(tr(
                        "Open Process (.opx), BPMN 2.0 (.bpmn/.xml), PNML (.pnml), event logs (.xes, .csv, .txt)",
                    ))
                    .clicked()
                {
                    self.io.open_dialog(ui.ctx());
                }
                if ui
                    .add(
                        egui::Button::new(tr("Save"))
                            .shortcut_text(ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::S))),
                    )
                    .clicked()
                {
                    self.save(false);
                }
                if ui.button(tr("Save as…")).clicked() {
                    self.save(true);
                }
                let export_label = match self.docs.get(self.active).map(|d| &d.editor) {
                    Some(Editor::Bpmn(_)) => tr("Export BPMN 2.0 XML (.bpmn)…"),
                    Some(Editor::Mining(_)) => tr("Export Alpha write-up (.txt)…"),
                    _ => tr("Export PNML (.pnml)…"),
                };
                if ui
                    .button(export_label)
                    .on_hover_text(tr(
                        "Standard format readable by Bizagi, Camunda, bpmn.io, Signavio / WoPeD, PIPE, ProM, TINA…",
                    ))
                    .clicked()
                {
                    self.export();
                }
                let is_diagram = matches!(
                    self.docs.get(self.active).map(|d| &d.editor),
                    Some(Editor::Bpmn(_) | Editor::Petri(_))
                );
                if ui
                    .add_enabled(is_diagram, egui::Button::new(tr("Export image (PNG)…")))
                    .on_hover_text(tr("Picture of the visible diagram"))
                    .clicked()
                {
                    match self.active_doc().map(|d| &mut d.editor) {
                        Some(Editor::Bpmn(e)) => e.export_png = true,
                        Some(Editor::Petri(e)) => e.export_png = true,
                        _ => {}
                    }
                }
                ui.separator();
                ui.menu_button(tr("Examples"), |ui| {
                    if ui.button(tr("BPMN – Process P evaluation")).clicked() {
                        self.open_model(Model::Bpmn(examples::evaluation_process()), None, false);
                        ui.close();
                    }
                    if ui.button(tr("BPMN – Order handling")).clicked() {
                        self.open_model(Model::Bpmn(examples::order_process()), None, false);
                    }
                    if ui.button(tr("Petri – Mutual exclusion")).clicked() {
                        self.open_model(Model::Petri(examples::mutual_exclusion()), None, false);
                    }
                    if ui.button(tr("Petri – Producer / consumer")).clicked() {
                        self.open_model(Model::Petri(examples::producer_consumer()), None, false);
                    }
                    if ui.button(tr("Petri – Dining philosophers")).clicked() {
                        self.open_model(Model::Petri(examples::dining_philosophers()), None, false);
                    }
                    if ui.button(tr("Log – ABCD, ACBD, EF")).clicked() {
                        self.open_model(
                            Model::Log(LogDocument {
                                name: tr("Alpha example 1").into(),
                                text: crate::mining_editor::EXAMPLE_1.into(),
                                ..Default::default()
                            }),
                            None,
                            false,
                        );
                    }
                    if ui.button(tr("Log – ABCD, ACBD, AED")).clicked() {
                        self.open_model(
                            Model::Log(LogDocument {
                                name: tr("Alpha example 2").into(),
                                text: crate::mining_editor::EXAMPLE_2.into(),
                                ..Default::default()
                            }),
                            None,
                            false,
                        );
                    }
                });
                ui.separator();
                if ui.button(tr("Close tab")).clicked() {
                    self.confirm_close = Some(self.active);
                }
                #[cfg(not(target_arch = "wasm32"))]
                if ui.button(tr("Quit")).clicked() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button(tr("Edit"), |ui| {
                let (can_undo, can_redo) = match self.docs.get(self.active).map(|d| &d.editor) {
                    Some(Editor::Bpmn(e)) => (e.can_undo(), e.can_redo()),
                    Some(Editor::Petri(e)) => (e.can_undo(), e.can_redo()),
                    Some(Editor::Mining(_)) | None => (false, false),
                };
                if ui
                    .add_enabled(
                        can_undo,
                        egui::Button::new(tr("Undo"))
                            .shortcut_text(ui.ctx().format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::Z))),
                    )
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(
                        can_redo,
                        egui::Button::new(tr("Redo")).shortcut_text(
                            ui.ctx()
                                .format_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z)),
                        ),
                    )
                    .clicked()
                {
                    self.redo();
                }
                if ui.add(egui::Button::new(tr("Delete selection")).shortcut_text("Del")).clicked() {
                    match self.active_doc().map(|d| &mut d.editor) {
                        Some(Editor::Bpmn(e)) => e.delete_selection(),
                        Some(Editor::Petri(e)) => e.delete_selection(),
                        Some(Editor::Mining(_)) | None => {}
                    }
                }
            });
            ui.menu_button(tr("View"), |ui| {
                if ui.add(egui::Button::new(tr("Zoom to fit")).shortcut_text("F")).clicked() {
                    match self.active_doc().map(|d| &mut d.editor) {
                        Some(Editor::Bpmn(e)) => e.request_fit(),
                        Some(Editor::Petri(e)) => e.request_fit(),
                        Some(Editor::Mining(_)) | None => {}
                    }
                }
                ui.separator();
                let active = self.docs.get(self.active).map(|d| (d.uid, matches!(d.editor, Editor::Petri(_))));
                if let Some((uid, true)) = active {
                    ui.menu_button(tr("Open beside"), |ui| {
                        for view in [View::Matrices, View::Analysis, View::States, View::Graph] {
                            if ui.button(view.label()).clicked() {
                                self.open_beside(Pane { doc: uid, view }, false);
                            }
                        }
                    });
                }
                ui.menu_button(tr("Show document beside"), |ui| {
                    let others: Vec<(u64, String)> = self
                        .docs
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| *i != self.active)
                        .map(|(_, d)| (d.uid, d.name()))
                        .collect();
                    if others.is_empty() {
                        ui.label(RichText::new(tr("Open another document first")).weak());
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
                ui.label(
                    RichText::new(tr("Tip: drag a tab onto the edge of a pane to split it"))
                        .small()
                        .weak(),
                );
                ui.separator();
                ui.label(tr("Theme"));
                egui::widgets::global_theme_preference_buttons(ui);
            });
            ui.menu_button(tr("Tools"), |ui| {
                let is_bpmn = matches!(self.docs.get(self.active).map(|d| &d.editor), Some(Editor::Bpmn(_)));
                if ui
                    .add_enabled(is_bpmn, egui::Button::new(tr("Convert BPMN → Petri net")))
                    .on_hover_text(tr("Workflow-net translation for soundness analysis"))
                    .clicked()
                {
                    self.convert_active();
                }
                let is_petri = matches!(self.docs.get(self.active).map(|d| &d.editor), Some(Editor::Petri(_)));
                if ui
                    .add_enabled(is_petri, egui::Button::new(tr("Start / stop token game")).shortcut_text("S"))
                    .clicked()
                    && let Some(Editor::Petri(e)) = self.active_doc().map(|d| &mut d.editor)
                {
                    e.toggle_simulation(ui.input(|i| i.time));
                }
                if ui.add_enabled(is_petri, egui::Button::new(tr("Matrices (Pre, Post, C)"))).clicked()
                    && let Some(Editor::Petri(e)) = self.active_doc().map(|d| &mut d.editor)
                {
                    e.show_matrices();
                }
                if ui.add_enabled(is_petri, egui::Button::new(tr("Analysis & invariants"))).clicked()
                    && let Some(Editor::Petri(e)) = self.active_doc().map(|d| &mut d.editor)
                {
                    e.show_analysis();
                }
                if ui.add_enabled(is_petri, egui::Button::new(tr("Reachability graph"))).clicked()
                    && let Some(Editor::Petri(e)) = self.active_doc().map(|d| &mut d.editor)
                {
                    e.show_reachability_graph();
                }
                if ui
                    .add_enabled(is_petri, egui::Button::new(tr("Convert Petri net → BPMN")))
                    .on_hover_text(tr("Deduce a BPMN diagram (AND/XOR gateways) from a workflow net"))
                    .clicked()
                {
                    self.petri_to_bpmn();
                }
                ui.separator();
                if ui
                    .button(tr("Process discovery (Alpha algorithm)…"))
                    .on_hover_text(tr("Event log → relation matrix → Alpha → Petri net → BPMN"))
                    .clicked()
                {
                    self.new_log();
                }
            });
            ui.menu_button(tr("Help"), |ui| {
                if ui.button(tr("Keyboard shortcuts")).clicked() {
                    self.show_shortcuts = true;
                }
                if ui.button(tr("About Open Process")).clicked() {
                    self.show_about = true;
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                theme_toggle(ui);
                crate::i18n::lang_toggle(ui);
            });
        });
    }

    fn dialogs(&mut self, ui: &Ui) {
        let ctx = ui.ctx().clone();
        if self.show_about {
            egui::Window::new(tr("About Open Process")).collapsible(false).resizable(false).open(&mut self.show_about).show(&ctx, |ui| {
                ui.heading("Open Process");
                ui.label(trf("Version {}", &[&(env!("CARGO_PKG_VERSION"))]));
                ui.label(tr("Free and open-source process modelling & analysis: BPMN 2.0, Petri nets, matrices, invariants, reachability graphs, soundness."));
                ui.label(tr("Written in Rust with egui — runs natively on Windows, macOS, Linux and in the browser (WebAssembly)."));
                ui.label(tr("License: MIT OR Apache-2.0"));
            });
        }
        if self.show_shortcuts {
            egui::Window::new(tr("Keyboard shortcuts"))
                .collapsible(false)
                .open(&mut self.show_shortcuts)
                .show(&ctx, |ui| {
                    egui::Grid::new("shortcuts").striped(true).show(ui, |ui| {
                        for (k, v) in [
                            ("Ctrl+N / Ctrl+O / Ctrl+S", tr("New BPMN / open / save")),
                            ("Ctrl+Z / Ctrl+Shift+Z", tr("Undo / redo")),
                            (tr("Del / Backspace"), tr("Delete selection")),
                            ("Esc", tr("Back to the select tool")),
                            ("F", tr("Zoom to fit")),
                            (tr("Wheel / Ctrl+wheel"), tr("Scroll / zoom")),
                            (tr("Right or middle drag"), tr("Pan")),
                            (tr("Double-click / F2"), tr("Rename")),
                            (
                                "BPMN: V C S I E T X + O",
                                tr("Select, connect, start, intermediate, end, task, XOR, AND, OR"),
                            ),
                            ("Petri: V P T A K", tr("Select, place, transition, arc, tokens")),
                            (tr("Petri: S / Space"), tr("Token game / random step")),
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
            egui::Window::new(tr("Close tab?"))
                .collapsible(false)
                .resizable(false)
                .show(&ctx, |ui| {
                    let name = self.docs.get(i).map(|d| d.name()).unwrap_or_default();
                    ui.label(trf("'{}' has unsaved changes.", &[&name]));
                    ui.horizontal(|ui| {
                        if ui.button(tr("Save")).clicked() {
                            self.active = i;
                            self.save(false);
                            keep = false;
                        }
                        if ui.button(tr("Close without saving")).clicked() {
                            self.close(i);
                            keep = false;
                        }
                        if ui.button(tr("Cancel")).clicked() {
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
        tr("Switch to light theme")
    } else {
        tr("Switch to dark theme")
    });
    if resp.clicked() {
        ui.ctx().set_theme(if dark { egui::Theme::Light } else { egui::Theme::Dark });
    }
}

/// Small painted logo: place → transition → place.
fn logo(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(30.0, 20.0), egui::Sense::hover());
    let accent = theme::accent(ui.visuals().dark_mode);
    let p = ui.painter();
    let c1 = rect.left_center() + egui::vec2(6.0, 0.0);
    let c2 = rect.right_center() - egui::vec2(6.0, 0.0);
    let stroke = egui::Stroke::new(1.6, accent);
    p.circle_stroke(c1, 5.0, stroke);
    p.circle_filled(c1, 1.8, accent);
    p.circle_stroke(c2, 5.0, stroke);
    p.line_segment([c1 + egui::vec2(5.0, 0.0), c2 - egui::vec2(5.0, 0.0)], stroke);
    p.rect_filled(egui::Rect::from_center_size(rect.center(), egui::vec2(3.5, 12.0)), 1.0, accent);
}

/// Bridges the split-view tree and the documents.
struct DockBehavior<'a> {
    docs: &'a mut Vec<Doc>,
    active_uid: Option<u64>,
    events: Vec<DockEvent>,
}

impl DockBehavior<'_> {
    fn doc(&self, uid: u64) -> Option<&Doc> {
        self.docs.iter().find(|d| d.uid == uid)
    }
}

fn kind_badge(editor: &Editor) -> (&'static str, egui::Color32) {
    match editor {
        Editor::Bpmn(_) => ("BPMN", theme::KIND_BPMN),
        Editor::Petri(_) => ("PETRI", theme::KIND_PETRI),
        Editor::Mining(_) => ("LOG", theme::KIND_LOG),
    }
}

impl egui_tiles::Behavior<Pane> for DockBehavior<'_> {
    fn pane_ui(&mut self, ui: &mut Ui, _tile_id: egui_tiles::TileId, pane: &mut Pane) -> egui_tiles::UiResponse {
        if ui.rect_contains_pointer(ui.max_rect()) && ui.input(|i| i.pointer.any_pressed()) && self.active_uid != Some(pane.doc) {
            self.events.push(DockEvent::Activate(pane.doc));
            self.active_uid = Some(pane.doc);
        }
        let Some(doc) = self.docs.iter_mut().find(|d| d.uid == pane.doc) else {
            return egui_tiles::UiResponse::None;
        };
        match (&mut doc.editor, pane.view) {
            (Editor::Bpmn(e), _) => {
                e.ui(ui);
                if let Some(a) = e.action.take() {
                    self.events.push(DockEvent::Bpmn(pane.doc, a));
                }
            }
            (Editor::Petri(e), View::Editor) => {
                e.ui(ui);
                if let Some(v) = e.request_pane.take() {
                    self.events.push(DockEvent::Beside(Pane { doc: pane.doc, view: v }, false));
                }
            }
            (Editor::Petri(e), v) => e.view_ui(ui, v),
            (Editor::Mining(e), _) => {
                e.ui(ui);
                if let Some(a) = e.action.take() {
                    self.events.push(DockEvent::Mining(a));
                }
            }
        }
        egui_tiles::UiResponse::None
    }

    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        let name = self.doc(pane.doc).map(|d| d.name()).unwrap_or_default();
        match pane.view {
            View::Editor => name.into(),
            v => format!("{name} · {}", v.label()).into(),
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
            Some(egui_tiles::Tile::Pane(p)) => Some(*p),
            _ => None,
        };
        let (kind, color, title, dirty) = match pane.and_then(|p| self.doc(p.doc).map(|d| (p, d))) {
            Some((p, d)) => {
                let (k, c) = kind_badge(&d.editor);
                let t = match p.view {
                    View::Editor => d.name(),
                    v => format!("{} · {}", d.name(), v.label()),
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
                if p.view == View::Editor {
                    self.events.push(DockEvent::Close(p.doc));
                } else {
                    tiles.remove(tile_id);
                }
            }
            resp.context_menu(|ui| {
                if ui.button(tr("Split right")).clicked() {
                    self.events.push(DockEvent::Beside(p, false));
                    ui.close();
                }
                if ui.button(tr("Split down")).clicked() {
                    self.events.push(DockEvent::Beside(p, true));
                    ui.close();
                }
                if let Some(Editor::Petri(_)) = self.doc(p.doc).map(|d| &d.editor) {
                    ui.separator();
                    for view in [View::Matrices, View::Analysis, View::States, View::Graph] {
                        if ui.button(trf("Open {} beside", &[&(view.label().to_lowercase())])).clicked() {
                            self.events.push(DockEvent::Beside(Pane { doc: p.doc, view }, false));
                            ui.close();
                        }
                    }
                }
                ui.separator();
                if ui.button(tr("Close")).clicked() {
                    if p.view == View::Editor {
                        self.events.push(DockEvent::Close(p.doc));
                    } else {
                        tiles.remove(tile_id);
                    }
                    ui.close();
                }
            });
        }
        let hint = if dirty {
            trf("{} (unsaved changes) — drag to move or split", &[&title])
        } else {
            trf("{} — drag to move or split", &[&title])
        };
        resp.on_hover_text(hint)
    }

    fn tab_bar_trailing_ui(&mut self, tiles: &egui_tiles::Tiles<Pane>, ui: &mut Ui, _tile_id: egui_tiles::TileId, tabs: &egui_tiles::Tabs) {
        ui.add_space(6.0);
        let near = tabs.children.first().copied().filter(|c| tiles.get(*c).is_some());
        ui.menu_button(RichText::new("+").font(theme::semibold(16.0)), |ui| {
            if ui.button(tr("New BPMN diagram")).clicked() {
                self.events.push(DockEvent::New("bpmn", near));
            }
            if ui.button(tr("New Petri net")).clicked() {
                self.events.push(DockEvent::New("petri", near));
            }
            if ui.button(tr("New event log (process discovery)")).clicked() {
                self.events.push(DockEvent::New("log", near));
            }
        })
        .response
        .on_hover_text(tr("New document"));
    }
}

impl eframe::App for OpenProcessApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        for (name, bytes) in self.io.poll() {
            self.load_bytes(name, &bytes);
        }
        match io::poll_png_export(ui.ctx()) {
            Some(Ok(msg)) if !msg.is_empty() => self.message = Some((msg, false)),
            Some(Err(e)) => self.message = Some((trf("Image export failed: {}", &[&e]), true)),
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

        let active_uid = self.docs.get(self.active).map(|d| d.uid);
        let mut behavior = DockBehavior {
            docs: &mut self.docs,
            active_uid,
            events: Vec::new(),
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
        self.dialogs(ui);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let p = PersistedV2 {
            docs: self.docs.iter().map(|d| (d.uid, d.model(), d.path.clone())).collect(),
            active: self.active,
            tree: self.tree.clone(),
        };
        eframe::set_value(storage, STORAGE_KEY_V2, &p);
        crate::i18n::save(storage);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    fn harness(dark: bool) -> Harness<'static, OpenProcessApp> {
        harness_sized(dark, [1400.0, 860.0])
    }

    fn harness_sized(dark: bool, size: [f32; 2]) -> Harness<'static, OpenProcessApp> {
        let mut h = Harness::builder()
            .with_size(size)
            .build_eframe(|cc| OpenProcessApp::new(cc, Vec::new()));
        h.ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
        h.run_steps(4);
        h
    }

    fn shot(h: &mut Harness<'static, OpenProcessApp>, name: &str) {
        h.run_steps(if name.contains("animation") { 0 } else { 4 });
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/screenshots");
        std::fs::create_dir_all(&dir).unwrap();
        match h.render() {
            Ok(img) => img.save(dir.join(format!("{name}.png"))).unwrap(),
            Err(e) => eprintln!("rendering unavailable: {e}"),
        }
    }

    #[test]
    fn bpmn_evaluation_ui() {
        let mut h = harness(false);
        h.state_mut().open_model(Model::Bpmn(examples::evaluation_process()), None, false);
        h.run_steps(4);
        h.get_by_label("Evaluation").click();
        h.run_steps(4);
        assert!(h.query_by_label("Process evaluation").is_some());
        assert!(h.query_by_label("Cost CAi").is_some());
        assert!(h.query_by_label("Duration TAi").is_some());
        assert!(
            h.get_by_label("A2").rect().top() > h.get_by_label("A1").rect().bottom(),
            "one activity per table row"
        );
        assert!(
            h.get_by_label("Process evaluation").rect().left() > 900.0,
            "the inspector leaves room for the diagram"
        );
        shot(&mut h, "bpmn_evaluation");
    }

    #[test]
    fn screenshots() {
        let mut h = harness(false);
        shot(&mut h, "bpmn_light");

        let mut h = harness(true);
        shot(&mut h, "bpmn_dark");

        let mut h = harness(false);
        h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
        if let Some(Editor::Petri(e)) = h.state_mut().docs.last_mut().map(|d| &mut d.editor) {
            e.show_matrices();
        }
        shot(&mut h, "petri_matrices");

        if let Some(Editor::Petri(e)) = h.state_mut().docs.last_mut().map(|d| &mut d.editor) {
            e.show_analysis();
        }
        shot(&mut h, "petri_analysis");

        h.state_mut().open_model(Model::Petri(examples::dining_philosophers()), None, false);
        if let Some(Editor::Petri(e)) = h.state_mut().docs.last_mut().map(|d| &mut d.editor) {
            e.toggle_simulation(0.0);
            e.show_reachability_graph();
        }
        shot(&mut h, "petri_simulation_graph");

        h.state_mut().set_active(0);
        h.state_mut().convert_active();
        if let Some(Editor::Petri(e)) = h.state_mut().docs.last_mut().map(|d| &mut d.editor) {
            e.show_analysis();
        }
        shot(&mut h, "bpmn_converted");
    }

    /// Every non-ASCII character used in the UI must be drawable with the
    /// embedded fonts (there are no system fonts on the web).
    #[test]
    fn glyph_coverage() {
        let ctx = egui::Context::default();
        crate::theme::setup_fonts(&ctx);
        let mut out = ctx.run_ui(Default::default(), |_| {});
        let mut chars: Vec<char> = Vec::new();
        for src in [
            include_str!("app.rs"),
            include_str!("bpmn_editor.rs"),
            include_str!("petri_editor.rs"),
            include_str!("canvas.rs"),
            include_str!("mining_editor.rs"),
            include_str!("../../op-core/src/i18n/fr.rs"),
            include_str!("../../op-core/src/mining.rs"),
            include_str!("../../op-core/src/analysis.rs"),
            include_str!("../../op-core/src/convert.rs"),
            include_str!("../../op-core/src/bpmn.rs"),
        ] {
            for line in src.lines().filter(|l| !l.trim_start().starts_with("//")) {
                chars.extend(line.chars().filter(|c| !c.is_ascii()));
            }
        }
        chars.sort();
        chars.dedup();
        let missing: String = ctx.fonts_mut(|f| {
            chars
                .into_iter()
                .filter(|&c| !f.has_glyph(&egui::FontId::proportional(14.0), c))
                .collect()
        });
        let mut out2 = ctx.run_ui(Default::default(), |_| {});
        out.textures_delta.clear();
        out2.textures_delta.clear();
        assert!(missing.is_empty(), "missing glyphs: {missing}");
    }

    fn press(h: &mut Harness<'static, OpenProcessApp>, pos: egui::Pos2, down: bool) {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: down,
            modifiers: Modifiers::NONE,
        });
        h.run_steps(2);
    }

    fn click(h: &mut Harness<'static, OpenProcessApp>, pos: egui::Pos2) {
        h.hover_at(pos);
        h.run_steps(2);
        press(h, pos, true);
        press(h, pos, false);
        // Let the double-click window pass so consecutive clicks stay single clicks.
        h.run_steps(30);
    }

    fn drag(h: &mut Harness<'static, OpenProcessApp>, from: egui::Pos2, to: egui::Pos2) {
        h.hover_at(from);
        h.run_steps(2);
        press(h, from, true);
        for k in 1..=10 {
            h.hover_at(from.lerp(to, k as f32 / 10.0));
            h.run_steps(1);
        }
        press(h, to, false);
    }

    fn key(h: &mut Harness<'static, OpenProcessApp>, k: Key) {
        h.key_press(k);
        h.run_steps(2);
    }

    fn petri<'a>(h: &'a Harness<'static, OpenProcessApp>) -> &'a PetriEditor {
        match &h.state().docs[h.state().active].editor {
            Editor::Petri(e) => e,
            _ => panic!("not a Petri net"),
        }
    }

    /// Drives the real UI: tools, clicks, arc drag, tokens, undo, token game.
    #[test]
    fn petri_editing_by_mouse() {
        let mut h = harness(false);
        h.state_mut().open_model(Model::Petri(PetriNet::new("t")), None, false);
        h.run_steps(4);
        let (a, b) = (egui::pos2(350.0, 400.0), egui::pos2(600.0, 400.0));

        h.hover_at(a);
        h.run_steps(2);
        key(&mut h, Key::P);
        click(&mut h, a);
        assert_eq!(petri(&h).net.places.len(), 1);

        key(&mut h, Key::T);
        click(&mut h, b);
        assert_eq!(petri(&h).net.transitions.len(), 1);

        key(&mut h, Key::A);
        drag(&mut h, a, b);
        assert_eq!(petri(&h).net.arcs.len(), 1, "arc created by dragging");

        key(&mut h, Key::K);
        click(&mut h, a);
        click(&mut h, a);
        assert_eq!(petri(&h).net.places[0].tokens, 2);

        // Undo the last token
        h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
        h.run_steps(3);
        assert_eq!(petri(&h).net.places[0].tokens, 1);

        // Token game: fire the transition
        key(&mut h, Key::Escape);
        key(&mut h, Key::S);
        assert!(petri(&h).sim.is_some());
        click(&mut h, b);
        assert_eq!(petri(&h).sim.as_ref().unwrap().marking, vec![0]);
        assert_eq!(petri(&h).net.places[0].tokens, 1, "simulation does not change M0");

        // Move the place with the select tool
        key(&mut h, Key::S);
        key(&mut h, Key::V);
        let before = petri(&h).net.places[0].pos;
        drag(&mut h, a, a + egui::vec2(0.0, 100.0));
        let after = petri(&h).net.places[0].pos;
        assert!(after.y > before.y + 30.0, "place moved: {before:?} -> {after:?}");
    }

    fn bpmn<'a>(h: &'a Harness<'static, OpenProcessApp>) -> &'a BpmnEditor {
        match &h.state().docs[h.state().active].editor {
            Editor::Bpmn(e) => e,
            _ => panic!("not BPMN"),
        }
    }

    #[test]
    fn bpmn_editing_by_mouse() {
        let mut h = harness(false);
        h.state_mut().open_model(Model::Bpmn(BpmnDiagram::new("t")), None, false);
        h.run_steps(4);
        let (a, b, c) = (egui::pos2(250.0, 400.0), egui::pos2(450.0, 400.0), egui::pos2(700.0, 400.0));
        h.hover_at(a);
        h.run_steps(2);
        key(&mut h, Key::S);
        click(&mut h, a);
        key(&mut h, Key::T);
        click(&mut h, b);
        key(&mut h, Key::E);
        click(&mut h, c);
        assert_eq!(bpmn(&h).diagram.nodes.len(), 3);
        key(&mut h, Key::C);
        drag(&mut h, a, b);
        key(&mut h, Key::C);
        drag(&mut h, b, c);
        assert_eq!(bpmn(&h).diagram.flows.len(), 2);
        assert!(
            bpmn(&h)
                .diagram
                .validate()
                .iter()
                .all(|i| i.severity != op_core::bpmn::Severity::Error)
        );

        // Drop a gateway on the second flow: it gets inserted
        key(&mut h, Key::X);
        click(&mut h, egui::pos2(580.0, 400.0));
        assert_eq!(bpmn(&h).diagram.nodes.len(), 4);
        assert_eq!(bpmn(&h).diagram.flows.len(), 3);
        shot(&mut h, "bpmn_edited");
    }

    #[test]
    fn dark_token_game_animation() {
        let mut h = harness(true);
        h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
        h.run_steps(4);
        if let Editor::Petri(e) = &mut h.state_mut().docs.last_mut().unwrap().editor {
            e.toggle_simulation(0.0);
        }
        h.run_steps(4);
        if let Editor::Petri(e) = &mut h.state_mut().docs.last_mut().unwrap().editor {
            assert!(e.fire_by_name("enter1"));
        }
        h.run_steps(1);
        shot(&mut h, "petri_dark_animation");
        h.run_steps(60);
        shot(&mut h, "petri_dark_after");
        if let Editor::Petri(e) = &h.state().docs.last().unwrap().editor {
            assert_eq!(e.sim.as_ref().unwrap().marking, vec![0, 0, 1, 1, 0]);
        }
    }

    /// Process discovery tab: course example 2, then open the net and the BPMN.
    #[test]
    fn process_discovery() {
        let mut h = harness(false);
        h.state_mut().open_model(
            Model::Log(LogDocument {
                name: "Example 2".into(),
                text: crate::mining_editor::EXAMPLE_2.into(),
                ..Default::default()
            }),
            None,
            false,
        );
        h.run_steps(4);
        shot(&mut h, "mining_top");
        let n = h.state().docs.len();
        h.get_by_label("Open Petri net").click();
        h.run_steps(4);
        assert_eq!(h.state().docs.len(), n + 1);
        assert!(matches!(h.state().docs.last().unwrap().editor, Editor::Petri(_)));
        shot(&mut h, "mining_petri");
        h.state_mut().set_active(n - 1);
        h.run_steps(4);
        h.get_by_label("Deduce BPMN").click();
        h.run_steps(4);
        assert!(matches!(h.state().docs.last().unwrap().editor, Editor::Bpmn(_)));
        if let Editor::Bpmn(e) = &mut h.state_mut().docs.last_mut().unwrap().editor {
            e.request_fit();
        }
        shot(&mut h, "mining_bpmn");
    }

    #[test]
    fn rubber_band_and_group_move() {
        let mut h = harness(false);
        h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
        h.run_steps(4);
        // Select everything with a rubber band drawn on empty space
        drag(&mut h, egui::pos2(1000.0, 800.0), egui::pos2(90.0, 95.0));
        let (n, before, place) = match &h.state().docs.last().unwrap().editor {
            Editor::Petri(e) => (
                e.selected_count(),
                e.net.places.iter().map(|p| p.pos).collect::<Vec<_>>(),
                e.screen_pos(e.net.places[0].pos),
            ),
            _ => unreachable!(),
        };
        assert_eq!(n, 9, "all places and transitions selected");
        // Dragging one node moves the whole selection
        drag(&mut h, place, place + egui::vec2(60.0, 40.0));
        let after: Vec<op_core::geom::Pos> = match &h.state().docs.last().unwrap().editor {
            Editor::Petri(e) => e.net.places.iter().map(|p| p.pos).collect(),
            _ => unreachable!(),
        };
        let d0 = (after[0].x - before[0].x, after[0].y - before[0].y);
        assert!(d0.0 > 20.0 && d0.1 > 10.0, "moved: {d0:?}");
        for (a, b) in after.iter().zip(&before) {
            assert!(
                (a.x - b.x - d0.0).abs() < 11.0 && (a.y - b.y - d0.1).abs() < 11.0,
                "group moved together"
            );
        }
    }

    #[test]
    fn palette_drag_and_drop() {
        let mut h = harness(false);
        h.state_mut().open_model(Model::Bpmn(BpmnDiagram::new("dnd")), None, false);
        h.run_steps(4);
        let task = h.get_by_label("Task (T)").rect().center();
        drag(&mut h, task, egui::pos2(600.0, 400.0));
        let start = h.get_by_label("Start event (S)").rect().center();
        drag(&mut h, start, egui::pos2(400.0, 400.0));
        h.run_steps(2);
        let d = &bpmn(&h).diagram;
        assert_eq!(d.nodes.len(), 2, "two elements dropped");
        assert!(d.nodes.iter().any(|n| matches!(n.kind, op_core::bpmn::NodeKind::Task(_))));
        shot(&mut h, "bpmn_dropped");
    }

    #[test]
    fn link_styles_and_sketch() {
        use op_core::style::{DiagramStyle, EdgeStyle};
        let mut h = harness(false);
        let mut net = examples::mutual_exclusion();
        net.style = DiagramStyle {
            edges: EdgeStyle::Orthogonal,
            sketch: false,
            ..Default::default()
        };
        h.state_mut().open_model(Model::Petri(net.clone()), None, false);
        shot(&mut h, "style_petri_orthogonal");
        net.style = DiagramStyle {
            edges: EdgeStyle::Curved,
            sketch: true,
            ..Default::default()
        };
        h.state_mut().open_model(Model::Petri(net), None, false);
        shot(&mut h, "style_petri_sketch_curved");
        let mut bar = examples::mutual_exclusion();
        bar.style.transitions = op_core::style::TransitionShape::Bar;
        h.state_mut().open_model(Model::Petri(bar), None, false);
        shot(&mut h, "style_petri_bars");
        let mut d = examples::order_process();
        d.style = DiagramStyle {
            edges: EdgeStyle::Auto,
            sketch: true,
            ..Default::default()
        };
        h.state_mut().open_model(Model::Bpmn(d), None, false);
        shot(&mut h, "style_bpmn_sketch");
    }

    #[test]
    fn manual_alpha_mode() {
        let mut h = harness_sized(false, [1400.0, 2600.0]);
        let mut answers = std::collections::BTreeMap::new();
        answers.insert("step:TI".to_string(), "{A}".to_string());
        answers.insert("step:TO".to_string(), "{D,E}".to_string()); // wrong on purpose
        answers.insert("step:YW".to_string(), "(A,BE), (A,CE), (BE,D), (CE,D)".to_string());
        answers.insert("m:A:B".to_string(), "→".to_string());
        answers.insert("m:B:C".to_string(), "#".to_string()); // wrong
        h.state_mut().open_model(
            Model::Log(LogDocument {
                name: "Exercise".into(),
                text: "ABCD\nACBD\nAED".into(),
                manual: true,
                answers,
            }),
            None,
            false,
        );
        h.run_steps(4);
        h.get_by_label("Check everything").click();
        h.run_steps(4);
        shot(&mut h, "mining_manual");
        let Editor::Mining(e) = &h.state().docs.last().unwrap().editor else {
            unreachable!()
        };
        assert!(e.manual);
        // the answers are saved with the document
        let Model::Log(l) = h.state().docs.last().unwrap().model() else {
            unreachable!()
        };
        assert_eq!(l.answers.get("step:TI").map(String::as_str), Some("{A}"));
    }

    #[test]
    fn png_encoding() {
        let img = egui::ColorImage::new([20, 10], vec![egui::Color32::RED; 200]);
        let png = io::encode_png(&img, egui::Rect::from_min_size(egui::pos2(2.0, 2.0), egui::vec2(8.0, 4.0)), 1.0).unwrap();
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    /// BPMN on the left, Petri net matrices on the right.
    #[test]
    fn split_view() {
        let mut h = harness(false);
        h.state_mut().open_model(Model::Petri(examples::mutual_exclusion()), None, false);
        let uid = h.state().docs.last().unwrap().uid;
        h.state_mut().set_active(0);
        h.state_mut().open_beside(
            Pane {
                doc: uid,
                view: View::Matrices,
            },
            false,
        );
        h.run_steps(6);
        let root = h.state().tree.root.unwrap();
        assert!(matches!(
            h.state().tree.tiles.get(root),
            Some(egui_tiles::Tile::Container(egui_tiles::Container::Linear(_)))
        ));
        shot(&mut h, "split_bpmn_matrices");
        // Closing the Petri editor removes its matrices pane too
        let i = h.state().docs.len() - 1;
        h.state_mut().close(i);
        h.run_steps(2);
        assert!(dock::panes_of(&h.state().tree, uid).is_empty());
    }

    #[test]
    fn persistence_roundtrip() {
        let p = Persisted {
            docs: vec![
                (Model::Bpmn(examples::order_process()), None),
                (Model::Petri(examples::mutual_exclusion()), Some("x.opx".into())),
            ],
            active: 1,
        };
        let s = serde_json::to_string(&p).unwrap();
        let back: Persisted = serde_json::from_str(&s).unwrap();
        assert_eq!(back.docs.len(), 2);
        assert_eq!(back.docs[0].0, p.docs[0].0);
    }
}
