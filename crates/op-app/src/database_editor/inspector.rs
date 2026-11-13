use super::*;
use op_core::database::{Column, DefaultValue, ReferentialAction, SqlType, Table};

impl DatabaseEditor {
    pub(super) fn properties(&mut self, ui: &mut Ui) {
        DiagramLayout::inspector_tabs(
            ui,
            &mut self.state.validation,
            &[(false, tr("common.properties")), (true, tr("common.validation"))],
            |_, _| {},
        );
        DiagramLayout::inspector_body(ui, |ui| {
            if self.state.validation {
                let issues = match self.sql().clone() {
                    Ok(_) => vec![],
                    Err(issues) => issues,
                };
                if issues.is_empty() {
                    ui.label(tr("database.no_structural_issues"));
                }
                for issue in issues {
                    ui.colored_label(ui.visuals().warn_fg_color, issue.message);
                }
                return;
            }
            let selected_name = self
                .state
                .selected
                .and_then(|id| self.schema.table(id))
                .map(|t| t.name.clone())
                .unwrap_or_else(|| tr("database.select_table").into());
            egui::ComboBox::from_id_salt("selected_table")
                .selected_text(selected_name)
                .show_ui(ui, |ui| {
                    for t in &self.schema.tables {
                        ui.selectable_value(&mut self.state.selected, Some(t.id), &t.name);
                    }
                });
            let Some(table_id) = self.state.selected.filter(|id| self.schema.table(*id).is_some()) else {
                ui.label(tr("database.schema_name"));
                ui.text_edit_singleline(&mut self.schema.name);
                ui.label(tr("database.add_tables_define_columns_then_connect_their"));
                return;
            };
            let (remove_column, remove_index, add_index) =
                table_properties(ui, self.schema.tables.iter_mut().find(|t| t.id == table_id).unwrap());
            if let Some(id) = remove_column {
                self.schema.delete_column(table_id, id);
            }
            if let Some(id) = remove_index {
                self.schema
                    .tables
                    .iter_mut()
                    .find(|t| t.id == table_id)
                    .unwrap()
                    .indexes
                    .retain(|i| i.id != id);
            }
            if add_index {
                self.schema.add_index(table_id);
            }
            self.foreign_keys(ui, table_id);
        });
    }
    fn foreign_keys(&mut self, ui: &mut Ui, table_id: Id) {
        let tables = self.schema.tables.clone();
        let Some(child) = tables.iter().find(|t| t.id == table_id) else {
            return;
        };
        let mut remove = None;
        ui.add_space(12.0);
        ui.heading(tr("database.foreign_keys"));
        for f in self.schema.foreign_keys.iter_mut().filter(|f| f.table == table_id) {
            egui::CollapsingHeader::new(&f.name).id_salt(f.id).show(ui, |ui| {
                ui.label(tr("database.constraint_name"));
                ui.text_edit_singleline(&mut f.name);
                ui.label(tr("database.referenced_table"));
                egui::ComboBox::from_id_salt("target_table")
                    .selected_text(
                        tables
                            .iter()
                            .find(|t| t.id == f.referenced_table)
                            .map(|t| t.name.as_str())
                            .unwrap_or("?"),
                    )
                    .show_ui(ui, |ui| {
                        for t in &tables {
                            ui.selectable_value(&mut f.referenced_table, t.id, &t.name);
                        }
                    });
                let target = tables.iter().find(|t| t.id == f.referenced_table);
                let mut remove_mapping = None;
                for (i, (local, remote)) in f.columns.iter_mut().zip(&mut f.referenced_columns).enumerate() {
                    ui.push_id(i, |ui| {
                        ui.horizontal_wrapped(|ui| {
                            column_choice(ui, "local", local, child);
                            if let Some(target) = target {
                                column_choice(ui, "remote", remote, target);
                            }
                            if ui.small_button("×").on_hover_text(tr("database.remove_mapping")).clicked() {
                                remove_mapping = Some(i);
                            }
                        });
                    });
                }
                if let Some(i) = remove_mapping {
                    f.columns.remove(i);
                    f.referenced_columns.remove(i);
                }
                if ui.button(tr("database.add_column_mapping")).clicked()
                    && let (Some(local), Some(target)) = (
                        child.columns.iter().find(|c| !f.columns.contains(&c.id)),
                        target.and_then(|t| t.columns.iter().find(|c| !f.referenced_columns.contains(&c.id))),
                    )
                {
                    f.columns.push(local.id);
                    f.referenced_columns.push(target.id);
                }
                for (label, value) in [("ON DELETE", &mut f.on_delete), ("ON UPDATE", &mut f.on_update)] {
                    ui.label(label);
                    egui::ComboBox::from_id_salt(label).selected_text(value.sql()).show_ui(ui, |ui| {
                        for action in ReferentialAction::ALL {
                            ui.selectable_value(value, action, action.sql());
                        }
                    });
                }
                if ui.button(tr("database.remove_relationship")).clicked() {
                    remove = Some(f.id);
                }
            });
        }
        if let Some(id) = remove {
            self.schema.foreign_keys.retain(|f| f.id != id);
        }
    }
}
fn column_choice(ui: &mut Ui, id: &str, value: &mut Id, table: &Table) {
    egui::ComboBox::from_id_salt(id)
        .width(100.0)
        .selected_text(table.column(*value).map(|c| c.name.as_str()).unwrap_or("?"))
        .show_ui(ui, |ui| {
            for c in &table.columns {
                ui.selectable_value(value, c.id, &c.name);
            }
        });
}
fn table_properties(ui: &mut Ui, table: &mut Table) -> (Option<Id>, Option<Id>, bool) {
    ui.add_space(8.0);
    ui.label(tr("database.table_name"));
    ui.text_edit_singleline(&mut table.name);
    ui.label(tr("common.width"));
    ui.add(egui::DragValue::new(&mut table.bounds.w).range(190.0..=1600.0));
    ui.heading(tr("database.columns"));
    let mut remove_column = None;
    let single_column = table.columns.len() == 1;
    for c in &mut table.columns {
        egui::CollapsingHeader::new(format!("{} · {}", c.name, c.data_type.label()))
            .id_salt(c.id)
            .default_open(single_column)
            .show(ui, |ui| {
                column_properties(ui, c);
                if ui.button(tr("dmn.remove_column")).clicked() {
                    remove_column = Some(c.id);
                }
            });
    }
    ui.add_space(12.0);
    ui.heading(tr("database.indexes"));
    let mut remove_index = None;
    for index in &mut table.indexes {
        egui::CollapsingHeader::new(&index.name).id_salt(index.id).show(ui, |ui| {
            ui.label(tr("database.index_name"));
            ui.text_edit_singleline(&mut index.name);
            ui.checkbox(&mut index.unique, tr("database.unique"));
            for c in &table.columns {
                let mut checked = index.columns.contains(&c.id);
                if ui.checkbox(&mut checked, &c.name).changed() {
                    if checked {
                        index.columns.push(c.id);
                    } else {
                        index.columns.retain(|id| *id != c.id);
                    }
                }
            }
            if ui.button(tr("database.remove_index")).clicked() {
                remove_index = Some(index.id);
            }
        });
    }
    let add_index = ui
        .add_enabled(!table.columns.is_empty(), egui::Button::new(tr("database.add_index")))
        .clicked();
    (remove_column, remove_index, add_index)
}
fn column_properties(ui: &mut Ui, c: &mut Column) {
    ui.label(tr("database.column_name"));
    ui.text_edit_singleline(&mut c.name);
    ui.label(tr("common.type"));
    egui::ComboBox::from_id_salt("column_type")
        .selected_text(c.data_type.label())
        .show_ui(ui, |ui| {
            for kind in SqlType::ALL {
                if ui
                    .selectable_label(std::mem::discriminant(&c.data_type) == std::mem::discriminant(&kind), kind.label())
                    .clicked()
                {
                    c.data_type = kind;
                }
            }
        });
    match &mut c.data_type {
        SqlType::VarChar { length } => {
            ui.label(tr("database.length"));
            ui.add(egui::DragValue::new(length).range(1..=65_535));
        }
        SqlType::Decimal { precision, scale } => {
            ui.horizontal(|ui| {
                ui.label(tr("database.precision"));
                ui.add(egui::DragValue::new(precision).range(1..=38));
                ui.label(tr("database.scale"));
                ui.add(egui::DragValue::new(scale).range(0..=38));
            });
        }
        _ => {}
    }
    ui.horizontal_wrapped(|ui| {
        if ui.checkbox(&mut c.primary_key, tr("database.primary_key")).changed() && c.primary_key {
            c.nullable = false;
        }
        ui.add_enabled(!c.primary_key, egui::Checkbox::new(&mut c.nullable, tr("database.nullable")));
        ui.checkbox(&mut c.unique, tr("database.unique"));
        if ui.checkbox(&mut c.auto_increment, tr("database.auto_increment")).changed() && c.auto_increment {
            c.primary_key = true;
            c.nullable = false;
            c.default = DefaultValue::None;
            if !c.data_type.is_integer() {
                c.data_type = SqlType::Integer;
            }
        }
    });
    ui.label(tr("database.default_value"));
    egui::ComboBox::from_id_salt("column_default")
        .selected_text(tr(c.default.label()))
        .show_ui(ui, |ui| {
            for value in DefaultValue::choices() {
                if ui
                    .selectable_label(
                        std::mem::discriminant(&c.default) == std::mem::discriminant(&value),
                        tr(value.label()),
                    )
                    .clicked()
                {
                    c.default = value;
                }
            }
        });
    match &mut c.default {
        DefaultValue::Text(s) | DefaultValue::Number(s) => {
            ui.text_edit_singleline(s);
        }
        DefaultValue::Boolean(v) => {
            ui.checkbox(v, tr("common.value"));
        }
        _ => {}
    }
}
