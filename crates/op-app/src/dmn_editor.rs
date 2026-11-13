//! Table-based decision editor. No artificial canvas or zoom for tabular content.
use crate::editor::layout::DiagramLayout;
use crate::editor::*;
use crate::history::History;
use egui::Ui;
use op_core::dmn::{self, DataType, DecisionTable, HitPolicy};
use op_core::document::{Document, DocumentCodec, JsonCodec, ReadError};
use op_core::i18n::{key, tr, trf};
use serde_json::Value;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

#[derive(Clone)]
pub struct TableView {
    uid: u64,
    inspector: Option<bool>,
    sample: String,
    result: Option<Result<dmn::Evaluation, String>>,
    result_key: u64,
    tab: bool,
    selected: Option<usize>,
}
impl Default for TableView {
    fn default() -> Self {
        Self {
            uid: crate::canvas::next_uid(),
            inspector: None,
            sample: "{}".into(),
            result: None,
            result_key: 0,
            tab: false,
            selected: None,
        }
    }
}
pub struct DmnEditor {
    pub table: DecisionTable,
    history: History<DecisionTable>,
    state: TableView,
}
impl DmnEditor {
    fn new(table: DecisionTable) -> Self {
        Self {
            history: History::new(&table),
            table,
            state: Default::default(),
        }
    }
    fn run(&mut self) {
        self.state.result_key = self.evaluation_key();
        self.state.result = Some(
            serde_json::from_str::<BTreeMap<String, Value>>(&self.state.sample)
                .map_err(|e| e.to_string())
                .and_then(|v| self.table.evaluate(&v)),
        );
    }
    fn evaluation_key(&self) -> u64 {
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_vec(&self.table).unwrap().hash(&mut hash);
        self.state.sample.hash(&mut hash);
        hash.finish()
    }
    fn properties(&mut self, ui: &mut Ui) {
        DiagramLayout::inspector_tabs(
            ui,
            &mut self.state.tab,
            &[(false, tr("common.properties")), (true, tr("common.validation"))],
            |_, _| {},
        );
        DiagramLayout::inspector_body(ui, |ui| {
            if self.state.tab {
                let issues = self.table.validate();
                if issues.is_empty() {
                    ui.label(tr("database.no_structural_issues"));
                }
                for issue in issues {
                    ui.colored_label(ui.visuals().warn_fg_color, issue);
                }
                return;
            }
            ui.heading(tr("dmn.decision_table"));
            ui.label(tr("common.name"));
            ui.text_edit_singleline(&mut self.table.name);
            ui.add_space(12.0);
            ui.label(tr("dmn.input_values_json"));
            ui.add(
                egui::TextEdit::multiline(&mut self.state.sample)
                    .code_editor()
                    .desired_rows(4)
                    .desired_width(f32::INFINITY),
            );
            if ui.button(tr("dmn.evaluate_decision")).clicked() {
                self.run();
            }
            if let Some(result) = &self.state.result {
                match result {
                    Ok(result) => {
                        ui.label(trf(
                            "dmn.matched_rules",
                            &[&result.matched_rules.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")],
                        ));
                        ui.label(egui::RichText::new(serde_json::to_string_pretty(&result.results).unwrap()).monospace());
                    }
                    Err(e) => {
                        ui.colored_label(ui.visuals().error_fg_color, e);
                    }
                }
            }
            ui.add_space(12.0);
            ui.label(tr("dmn.supported_tests_literals_comparisons_intervals_lists_not"));
            ui.label(tr("dmn.outputs_must_string_number_boolean_null_literals"));
        });
    }
    fn grid(&mut self, ui: &mut Ui) {
        let mut remove_input = None;
        let mut remove_output = None;
        let mut remove_rule = None;
        let can_remove_input = self.table.inputs.len() > 1;
        let can_remove_output = self.table.outputs.len() > 1;
        egui::ScrollArea::both().id_salt("decision_table_scroll").show(ui, |ui| {
            egui::Grid::new("decision_rules")
                .striped(true)
                .min_col_width(130.0)
                .spacing(egui::vec2(10.0, 8.0))
                .show(ui, |ui| {
                    ui.label(tr("dmn.rule"));
                    for (i, c) in self.table.inputs.iter_mut().enumerate() {
                        ui.push_id(("input", i), |ui| {
                            ui.vertical(|ui| {
                                ui.label(tr("dmn.input"));
                                ui.add(
                                    egui::TextEdit::singleline(&mut c.name)
                                        .desired_width(140.0)
                                        .hint_text(tr("dmn.input_name")),
                                );
                                type_selector(ui, &mut c.data_type);
                                if ui
                                    .add_enabled(can_remove_input, egui::Button::new(tr("dmn.remove_column")))
                                    .clicked()
                                {
                                    remove_input = Some(i);
                                }
                            });
                        });
                    }
                    for (i, c) in self.table.outputs.iter_mut().enumerate() {
                        ui.push_id(("output", i), |ui| {
                            ui.vertical(|ui| {
                                ui.label(tr("dmn.output"));
                                ui.add(
                                    egui::TextEdit::singleline(&mut c.name)
                                        .desired_width(140.0)
                                        .hint_text(tr("dmn.output_name")),
                                );
                                type_selector(ui, &mut c.data_type);
                                if ui
                                    .add_enabled(can_remove_output, egui::Button::new(tr("dmn.remove_column")))
                                    .clicked()
                                {
                                    remove_output = Some(i);
                                }
                            });
                        });
                    }
                    ui.label(tr("dmn.annotation"));
                    ui.end_row();
                    for (i, r) in self.table.rules.iter_mut().enumerate() {
                        if ui.selectable_label(self.state.selected == Some(i), (i + 1).to_string()).clicked() {
                            self.state.selected = Some(i);
                        }
                        for (j, cell) in r.inputs.iter_mut().enumerate() {
                            ui.add(
                                egui::TextEdit::singleline(cell)
                                    .id(egui::Id::unique((self.state.uid, "input_cell", i, j)))
                                    .desired_width(140.0),
                            );
                        }
                        for (j, cell) in r.outputs.iter_mut().enumerate() {
                            ui.add(
                                egui::TextEdit::singleline(cell)
                                    .id(egui::Id::unique((self.state.uid, "output_cell", i, j)))
                                    .desired_width(140.0),
                            );
                        }
                        ui.horizontal(|ui| {
                            ui.add(egui::TextEdit::singleline(&mut r.annotation).desired_width(170.0));
                            if ui.small_button("×").on_hover_text(tr("dmn.remove_rule")).clicked() {
                                remove_rule = Some(i);
                            }
                        });
                        ui.end_row();
                    }
                });
            if self.table.rules.is_empty() {
                ui.add_space(20.0);
                ui.label(tr("dmn.add_rule_define_decision"));
            }
        });
        if let Some(i) = remove_input {
            self.table.remove_input(i);
        }
        if let Some(i) = remove_output {
            self.table.remove_output(i);
        }
        if let Some(i) = remove_rule {
            self.table.rules.remove(i);
            self.state.selected = None;
        }
    }
}
fn type_selector(ui: &mut Ui, value: &mut DataType) {
    egui::ComboBox::from_id_salt("type")
        .width(140.0)
        .selected_text(value.type_ref())
        .show_ui(ui, |ui| {
            for t in DataType::ALL {
                ui.selectable_value(value, t, t.type_ref());
            }
        });
}
const COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        id: UNDO,
        label: key("common.undo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: REDO,
        label: key("common.redo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: DELETE,
        label: key("common.delete_selection"),
        group: CommandGroup::Edit,
    },
];
impl EditorBackend for DmnEditor {
    type ViewState = TableView;
    fn capture_view(&self) -> Self::ViewState {
        self.state.clone()
    }
    fn restore_view(&mut self, s: &Self::ViewState) {
        self.state = s.clone();
    }
    fn fresh_view(&self) -> Self::ViewState {
        let values: BTreeMap<_, _> = self
            .table
            .inputs
            .iter()
            .map(|c| {
                let value = match c.data_type {
                    DataType::String => Value::from(""),
                    DataType::Number => Value::from(0),
                    DataType::Boolean => Value::from(false),
                };
                (c.name.clone(), value)
            })
            .collect();
        TableView {
            sample: serde_json::to_string_pretty(&values).unwrap(),
            ..Default::default()
        }
    }
    fn save_view(s: &Self::ViewState) -> Value {
        serde_json::json!({"inspector":s.inspector,"sample":s.sample})
    }
    fn load_view(s: &mut Self::ViewState, v: &Value) {
        s.inspector = v["inspector"].as_bool();
        if let Some(sample) = v["sample"].as_str() {
            s.sample = sample.into();
        }
    }
    fn kind(&self) -> &'static str {
        "dmn.table"
    }
    fn name(&self) -> &str {
        &self.table.name
    }
    fn rename(&mut self, name: String) {
        self.history.sync(&self.table, false);
        self.table.name = name;
        self.history.sync(&self.table, false);
    }
    fn document(&self) -> Document {
        JsonCodec::new("dmn.table").encode(&self.table)
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.table);
    }
    fn ui(&mut self, ui: &mut Ui, _: &str) -> Vec<HostEvent> {
        if self.state.result.is_some() && self.state.result_key != self.evaluation_key() {
            self.state.result = None;
        }
        let before = self.table.clone();
        let layout = DiagramLayout::new(ui, self.state.uid, self.state.inspector);
        layout.tools(ui, |ui| {
            if ui.button(tr("dmn.add_input")).clicked() {
                self.table.add_input();
            }
            if ui.button(tr("dmn.add_output")).clicked() {
                self.table.add_output();
            }
            if ui.button(tr("dmn.add_rule")).clicked() {
                self.table.add_rule();
            }
            ui.label(tr("dmn.hit_policy"));
            egui::ComboBox::from_id_salt("hit_policy")
                .selected_text(self.table.hit_policy.code())
                .show_ui(ui, |ui| {
                    for p in HitPolicy::ALL {
                        ui.selectable_value(&mut self.table.hit_policy, p, p.code());
                    }
                });
            if ui.selectable_label(layout.inspector_visible, tr("common.inspector")).clicked() {
                self.state.inspector = Some(!layout.inspector_visible);
            }
        });
        layout.inspector(ui, |ui| self.properties(ui));
        egui::CentralPanel::default().show(ui, |ui| self.grid(ui));
        if self.table != before {
            self.state.result = None;
        }
        self.history.sync(
            &self.table,
            ui.ctx().egui_wants_keyboard_input() || ui.input(|i| i.pointer.any_down()),
        );
        vec![]
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
        COMMANDS
    }
    fn enabled(&self, c: &str) -> bool {
        match c {
            UNDO => self.history.can_undo(),
            REDO => self.history.can_redo(),
            DELETE => self.state.selected.is_some_and(|i| i < self.table.rules.len()),
            _ => false,
        }
    }
    fn execute(&mut self, c: &str, _: f64) -> Vec<HostEvent> {
        match c {
            UNDO => self.history.undo(&mut self.table),
            REDO => self.history.redo(&mut self.table),
            DELETE => {
                if let Some(i) = self.state.selected.take().filter(|i| *i < self.table.rules.len()) {
                    self.history.sync(&self.table, false);
                    self.table.rules.remove(i);
                    self.history.sync(&self.table, false);
                }
            }
            _ => {}
        }
        self.state.result = None;
        vec![]
    }
    fn export(&self) -> Result<ExportData, String> {
        self.table.check_storage()?;
        Ok(ExportData {
            suffix: "dmn",
            bytes: self.table.to_xml().into_bytes(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("dmn.export_dmn_xml_dmn")
    }
    fn exports(&self) -> Vec<ExportDescriptor> {
        vec![
            ExportDescriptor {
                id: "standard",
                label: self.export_label(),
            },
            ExportDescriptor {
                id: "csv",
                label: key("dmn.export_decision_table_csv"),
            },
        ]
    }
    fn export_as(&self, id: &str) -> Result<ExportData, String> {
        match id {
            "standard" => self.export(),
            "csv" => Ok(ExportData {
                suffix: "csv",
                bytes: self.table.to_csv().into_bytes(),
            }),
            _ => Err("Unknown decision export".into()),
        }
    }
}
pub fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.push(DiagramDescriptor {
        id: "dmn.table",
        aliases: &["dmn"],
        home_order: 6,
        icon: crate::icons::decision_table,
        new_label: key("dmn.new_decision_table"),
        title: key("dmn.define_decision_table"),
        description: key("dmn.edit_evaluate_decision_rules_typed_inputs_outputs"),
        badge: "DMN",
        color: egui::Color32::from_rgb(126, 92, 171),
        create: |n| JsonCodec::new("dmn.table").encode(&DecisionTable::new(trf("dmn.document_name", &[&n]))),
        open: |d| {
            let table: DecisionTable = JsonCodec::new("dmn.table").decode(d)?;
            table.check_storage().map_err(ReadError::Invalid)?;
            Ok(Session::boxed(DmnEditor::new(table)))
        },
    });
    registry.examples.push(ExampleDescriptor {
        id: "dmn.discount",
        kind: "dmn.table",
        label: key("dmn.dmn_discount_decision"),
        create: || JsonCodec::new("dmn.table").encode(&dmn::example()),
    });
    registry.imports.push(ImportDescriptor {
        extensions: &["dmn", "xml"],
        accepts: dmn::accepts_xml,
        decode: |s| dmn::from_xml(s).map(|d| JsonCodec::new("dmn.table").encode(&d)),
    });
}
