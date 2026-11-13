//! One registry-backed creation dialog, opened from File, tab bars and shortcuts.
use super::*;

pub(super) struct Picker {
    pub near: Option<egui_tiles::TileId>,
    query: String,
    focus: bool,
}
impl OpenProcessApp {
    pub(super) fn open_new_picker(&mut self, near: Option<egui_tiles::TileId>) {
        self.new_picker = Some(Picker {
            near,
            query: String::new(),
            focus: true,
        });
    }
    pub(super) fn new_document_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut picker) = self.new_picker.take() else {
            return;
        };
        let mut chosen = None;
        let mut chosen_example = None;
        let mut cancel = false;
        let response = egui::Modal::new(egui::Id::unique("new_document_picker")).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 48.0).clamp(160.0, 1040.0));
            ui.set_max_height((ctx.content_rect().height() - 48.0).max(160.0));
            ui.heading(tr("file.new_document"));
            ui.label(tr("app.choose_diagram_type_get_started"));
            if !self.registry.examples.is_empty() {
                ui.menu_button(tr("menu.examples"), |ui| {
                    for example in &self.registry.examples {
                        if ui.button(tr(example.label)).clicked() {
                            chosen_example = Some(example.id);
                            ui.close();
                        }
                    }
                });
            }
            ui.add_space(8.0);
            let label = ui.label(tr("app.search_diagram_types"));
            let search = ui
                .add(egui::TextEdit::singleline(&mut picker.query).desired_width(f32::INFINITY))
                .labelled_by(label.id);
            if picker.focus {
                search.request_focus();
                picker.focus = false;
            }
            ui.add_space(12.0);
            let footer_height = ui.spacing().interact_size.y + 3.0 * ui.spacing().item_spacing.y + 1.0;
            egui::ScrollArea::vertical()
                .id_salt("new_document_scroll")
                .max_height((ui.available_height() - footer_height).max(100.0))
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    chosen = crate::home::diagram_grid(ui, self.registry, &picker.query);
                });
            ui.separator();
            if ui.button(tr("common.cancel")).clicked() {
                cancel = true;
            }
        });
        if let Some(kind) = chosen {
            self.new_document(kind, picker.near);
        } else if let Some(id) = chosen_example {
            self.open_example_near(id, picker.near);
        } else if !cancel && !response.should_close() {
            self.new_picker = Some(picker);
        }
    }
}
