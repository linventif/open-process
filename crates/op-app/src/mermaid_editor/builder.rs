//! Structured authoring on the Mermaid document's source, without a second model.
use egui::Ui;
use op_core::i18n::{tr, trf};
use op_core::mermaid::{Diagram, builder};

#[derive(Clone)]
pub(super) struct BuilderState {
    family: Option<&'static str>,
    recipe: usize,
    // Transient, unapplied form input only; never serialized into a document or workspace.
    values: Vec<String>,
    selected: Option<builder::Statement>,
    indent: usize,
    before_line: usize,
    error: String,
}

impl Default for BuilderState {
    fn default() -> Self {
        Self {
            family: None,
            recipe: 0,
            values: vec![],
            selected: None,
            indent: 2,
            before_line: 0,
            error: String::new(),
        }
    }
}

impl BuilderState {
    pub(super) fn show(&mut self, ui: &mut Ui, diagram: &mut Diagram) -> bool {
        let Some(family) = builder::family(&diagram.source) else {
            ui.label(tr("mermaid.builder.unknown_family"));
            return false;
        };
        if self.family != Some(family.id) {
            *self = Self {
                family: Some(family.id),
                ..Default::default()
            };
        }
        ui.heading(tr(family.label));
        ui.label(tr("mermaid.builder.hint"));
        if !family.preview {
            ui.label(tr("mermaid.preview_unsupported"));
        }
        if diagram.source.len() > builder::MAX_BUILDER_BYTES {
            ui.label(tr("mermaid.builder.limit"));
            return false;
        }
        let recipes: Vec<_> = builder::recipes(&diagram.source).collect();
        self.recipe = self.recipe.min(recipes.len().saturating_sub(1));
        let Some(mut recipe) = recipes.get(self.recipe).copied() else {
            return false;
        };
        let mut chosen = self.recipe;
        egui::ComboBox::from_id_salt("mermaid-builder-recipe")
            .selected_text(tr(recipe.label))
            .show_ui(ui, |ui| {
                for (index, recipe) in recipes.iter().enumerate() {
                    ui.selectable_value(&mut chosen, index, tr(recipe.label));
                }
            });
        if chosen != self.recipe {
            self.recipe = chosen;
            self.selected = None;
            self.values.clear();
            self.error.clear();
        }
        recipe = recipes[self.recipe];
        if self.values.len() != recipe.fields.len() {
            self.values = recipe.defaults();
        }
        ui.push_id(recipe.id, |ui| {
            for (index, (field, value)) in recipe.fields.iter().zip(&mut self.values).enumerate() {
                ui.push_id(index, |ui| {
                    ui.label(tr(field.label));
                    if let builder::FieldKind::Choice(options) = field.kind {
                        egui::ComboBox::from_id_salt("choice")
                            .selected_text(value.as_str())
                            .show_ui(ui, |ui| {
                                for &option in options {
                                    ui.selectable_value(value, option.into(), option);
                                }
                            });
                    } else {
                        ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY))
                            .accessible_name(tr(field.label));
                    }
                });
            }
        });
        let mut changed = false;
        if self.selected.is_some() {
            ui.label(tr("mermaid.builder.replace_hint"));
            if ui.button(tr("mermaid.builder.apply")).clicked() {
                match builder::replace(diagram, self.selected.as_ref().unwrap(), &self.values) {
                    Ok(()) => {
                        changed = true;
                        self.selected = None;
                        self.error.clear();
                    }
                    Err(error) => self.error = error,
                }
            }
            if self.selected.is_some() && ui.button(tr("mermaid.builder.remove")).clicked() {
                match builder::remove(diagram, self.selected.as_ref().unwrap()) {
                    Ok(()) => {
                        changed = true;
                        self.selected = None;
                        self.error.clear();
                    }
                    Err(error) => self.error = error,
                }
            }
            if ui.button(tr("mermaid.builder.new_element")).clicked() {
                self.selected = None;
                self.values = recipe.defaults();
                self.error.clear();
            }
        } else {
            ui.horizontal(|ui| {
                ui.label(tr("mermaid.builder.indent"));
                ui.add(egui::DragValue::new(&mut self.indent).range(0..=16));
            });
            let lines = diagram.source.split_inclusive('\n').count();
            self.before_line = self.before_line.min(lines);
            ui.horizontal(|ui| {
                ui.label(tr("mermaid.builder.before_line"));
                ui.add(egui::DragValue::new(&mut self.before_line).range(0..=lines));
            });
            ui.small(tr("mermaid.builder.insertion_hint"));
            if ui.button(tr("mermaid.builder.add")).clicked() {
                let before =
                    (self.before_line > 0).then(|| diagram.source.split_inclusive('\n').take(self.before_line - 1).map(str::len).sum());
                match builder::insert(diagram, recipe, &self.values, before, self.indent) {
                    Ok(()) => {
                        changed = true;
                        self.error.clear();
                    }
                    Err(error) => self.error = error,
                }
            }
        }
        if !self.error.is_empty() {
            ui.colored_label(ui.visuals().error_fg_color, &self.error);
        }
        ui.separator();
        ui.label(tr("mermaid.builder.elements"));
        let statements = builder::statements(&diagram.source);
        if statements.is_empty() {
            ui.label(tr("mermaid.builder.no_elements"));
        }
        let row_height = ui.spacing().interact_size.y;
        egui::ScrollArea::vertical()
            .id_salt("mermaid-builder-statements")
            .max_height(row_height * 10.0)
            .show_rows(ui, row_height, statements.len(), |ui, rows| {
                for statement in &statements[rows] {
                    let label = trf("mermaid.builder.element_line", &[&statement.line, &tr(statement.recipe.label)]);
                    let selected = self.selected.as_ref().is_some_and(|s| s.range == statement.range);
                    if ui
                        .selectable_label(selected, label)
                        .on_hover_text(&diagram.source[statement.range.clone()])
                        .clicked()
                    {
                        self.recipe = recipes.iter().position(|r| r.id == statement.recipe.id).unwrap();
                        self.values = statement.values.clone();
                        self.selected = Some(statement.clone());
                        self.error.clear();
                    }
                }
            });
        ui.small(tr("mermaid.builder.preserve_hint"));
        changed
    }
}
