//! Relational schema editor. UI chrome is shared; database rules and DDL live in op-core.
mod editing;
mod inspector;
mod render;
mod session;
pub(crate) use session::register;

use crate::canvas::{self, View};
use crate::editor::clipboard::{CanvasClipboard, ClipboardState};
use crate::editor::layout::{ArrangeControl, CanvasControls, DiagramLayout, FooterStatus};
use crate::editor::*;
use crate::history::History;
use egui::{Rect, Ui};
use op_core::database::{self, Diagnostic, Dialect, Id, Schema};
use op_core::geom::Pos;
use op_core::i18n::{tr, trf};
use std::hash::{Hash, Hasher};

#[derive(Clone)]
enum Drag {
    Move { start: Pos, origins: Vec<(Id, Pos)> },
    Resize { id: Id, gesture: canvas::resize::ResizeGesture },
    Pan,
    Connect(Id),
}
#[derive(Clone)]
pub struct DatabaseView {
    uid: u64,
    camera: View,
    canvas: Rect,
    fit: bool,
    snap: bool,
    inspector: Option<bool>,
    selected: Option<Id>,
    group: std::collections::BTreeSet<Id>,
    validation: bool,
    connecting: bool,
    pending: Option<Id>,
    drag: Option<Drag>,
    clipboard: ClipboardState,
    status: String,
    export_png: bool,
    sql_view: bool,
}
impl Default for DatabaseView {
    fn default() -> Self {
        Self {
            uid: canvas::next_uid(),
            camera: View::default(),
            canvas: Rect::NOTHING,
            fit: true,
            snap: true,
            inspector: None,
            selected: None,
            group: Default::default(),
            validation: false,
            connecting: false,
            pending: None,
            drag: None,
            clipboard: ClipboardState::default(),
            status: String::new(),
            export_png: false,
            sql_view: false,
        }
    }
}
pub struct DatabaseEditor {
    pub schema: Schema,
    history: History<Schema>,
    state: DatabaseView,
    sql_cache: Option<(u64, Result<String, Vec<Diagnostic>>)>,
    events: Vec<HostEvent>,
    png_export: Option<canvas::export::ImageExport>,
}
impl DatabaseEditor {
    pub fn new(schema: Schema) -> Self {
        Self {
            history: History::new(&schema),
            schema,
            state: Default::default(),
            sql_cache: None,
            events: vec![],
            png_export: None,
        }
    }
    fn sql(&mut self) -> &Result<String, Vec<Diagnostic>> {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_vec(&(
            op_core::i18n::lang(),
            &self.schema.dialect,
            self.schema
                .tables
                .iter()
                .map(|t| (t.id, &t.name, &t.columns, &t.indexes))
                .collect::<Vec<_>>(),
            &self.schema.foreign_keys,
        ))
        .unwrap()
        .hash(&mut hash);
        let key = hash.finish();
        if self.sql_cache.as_ref().is_none_or(|(old, _)| *old != key) {
            self.sql_cache = Some((key, database::generate(&self.schema, self.schema.dialect)));
        }
        &self.sql_cache.as_ref().unwrap().1
    }
    fn add_table(&mut self, pos: Pos) {
        self.history.sync(&self.schema, false);
        let id = self.schema.add_table(pos);
        self.state.selected = Some(id);
        self.state.group.clear();
        self.state.status.clear();
        self.history.sync(&self.schema, false);
    }
    /// One undoable layout step: referenced tables precede referencing ones.
    fn auto_arrange(&mut self) {
        self.history.sync(&self.schema, false);
        match self.schema.auto_arrange(self.state.snap.then_some(10.0)) {
            Ok(()) => {
                self.state.drag = None;
                self.state.pending = None;
                self.state.fit = true;
                self.state.status.clear();
                self.history.sync(&self.schema, false);
            }
            Err(e) => self.state.status = e,
        }
    }
    fn link(&mut self, from: Id, to: Id) {
        self.history.sync(&self.schema, false);
        match self.schema.add_foreign_key(from, to) {
            Ok(_) => {
                self.state.status.clear();
                self.state.selected = Some(from);
            }
            Err(e) => self.state.status = e,
        }
        self.state.pending = None;
        self.history.sync(&self.schema, false);
    }
    fn delete_selected(&mut self) {
        let mut selected = std::mem::take(&mut self.state.group);
        selected.extend(self.state.selected.take());
        if !selected.is_empty() {
            self.history.sync(&self.schema, false);
            for id in selected {
                self.schema.delete_table(id);
            }
            self.state.pending = None;
            self.history.sync(&self.schema, false);
        }
    }
    fn toolbar(&mut self, ui: &mut Ui) {
        let dialect_label = ui.label(tr("database.target_database"));
        egui::ComboBox::from_id_salt("sql_dialect")
            .selected_text(self.schema.dialect.label())
            .show_ui(ui, |ui| {
                for d in Dialect::ALL {
                    ui.selectable_value(&mut self.schema.dialect, d, d.label());
                }
            })
            .response
            .labelled_by(dialect_label.id);
        if self.state.sql_view {
            let sql = self.sql().clone();
            if ui.add_enabled(sql.is_ok(), egui::Button::new(tr("database.copy_ddl"))).clicked()
                && let Ok(sql) = sql
            {
                ui.ctx().copy_text(sql);
            }
            return;
        }
        if ui.button(tr("database.add_table")).clicked() {
            let count = self.schema.tables.len();
            self.add_table(Pos::new(80.0 + (count % 2) as f32 * 360.0, 80.0 + (count / 2) as f32 * 240.0));
        }
        let selected = self.state.selected.filter(|id| self.schema.table(*id).is_some());
        if ui
            .add_enabled(selected.is_some(), egui::Button::new(tr("database.add_column")))
            .clicked()
        {
            self.schema.add_column(selected.unwrap());
        }
        let relationship_button = ui.add_enabled(selected.is_some(), egui::Button::new(tr("database.add_foreign_key")));
        egui::Popup::menu(&relationship_button).style(crate::theme::menu_style).show(|ui| {
            let tables: Vec<_> = self.schema.tables.iter().map(|t| (t.id, t.name.clone())).collect();
            for (id, name) in tables {
                if ui.button(name).clicked()
                    && let Some(selected) = selected
                {
                    self.link(selected, id);
                    ui.close();
                }
            }
        });
        if !crate::editor::exam::active(ui.ctx()) && ui.button(tr("database.sql_ddl")).clicked() {
            self.events.push(HostEvent::OpenView(crate::dock::View::from("database.ddl")));
        }
    }
    fn show(&mut self, ui: &mut Ui, sql_view: bool) {
        self.state.sql_view = sql_view;
        let layout = DiagramLayout::new(ui, self.state.uid, self.state.inspector);
        layout.tools(ui, |ui| self.toolbar(ui));
        if sql_view {
            let sql = self.sql().clone();
            egui::CentralPanel::default().show(ui, |ui| {
                egui::ScrollArea::both().show(ui, |ui| match &sql {
                    Ok(sql) => {
                        let mut code = sql.as_str();
                        ui.add(
                            egui::TextEdit::multiline(&mut code)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(30),
                        );
                    }
                    Err(issues) => {
                        ui.heading(tr("database.fix_schema_issues_before_generating_ddl"));
                        for issue in issues {
                            ui.colored_label(ui.visuals().error_fg_color, &issue.message);
                        }
                    }
                });
            });
        } else {
            layout.inspector(ui, |ui| self.properties(ui));
            let mut arrange = false;
            let can_arrange = !self.schema.tables.is_empty();
            layout.configuration(
                ui,
                CanvasControls {
                    view: &mut self.state.camera,
                    canvas: self.state.canvas,
                    fit: &mut self.state.fit,
                    snap: Some(&mut self.state.snap),
                    inspector: &mut self.state.inspector,
                    issues: None,
                    arrange: Some(ArrangeControl {
                        requested: &mut arrange,
                        enabled: can_arrange,
                    }),
                },
                FooterStatus {
                    summary: trf(
                        "database.tables_relationships",
                        &[&self.schema.tables.len(), &self.schema.foreign_keys.len()],
                    ),
                    detail: self.state.status.clone(),
                    ..Default::default()
                },
                |_| {},
            );
            if arrange {
                self.auto_arrange();
            }
            layout.canvas(ui, |ui| self.canvas(ui));
        }
        self.history.sync(
            &self.schema,
            self.state.drag.is_some() || ui.ctx().egui_wants_keyboard_input() || ui.input(|i| i.pointer.any_down()),
        );
    }
    #[cfg(test)]
    pub fn table_screen_center(&self, id: Id) -> egui::Pos2 {
        self.state
            .camera
            .to_screen(self.state.canvas, self.schema.table(id).unwrap().rect().center())
    }
}
