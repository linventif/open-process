//! Common chrome for graphical editors: inspector, configuration, tools and canvas.
//! Notations supply content/options; spacing, placement and responsive behavior live here.
use crate::{
    canvas::{self, View},
    theme,
};
use egui::{Align, Color32, Frame, Id, Layout, Margin, Rect, RichText, Ui};
use op_core::i18n::tr;

const INSPECTOR_BREAKPOINT: f32 = 760.0;
const CONFIG_BREAKPOINT: f32 = 720.0;
const COMPACT_INSPECTOR_BREAKPOINT: f32 = 360.0;

#[derive(Clone, Copy)]
pub struct DiagramLayout {
    id: Id,
    wide: bool,
    pub inspector_visible: bool,
}

pub struct CanvasControls<'a> {
    pub view: &'a mut View,
    pub canvas: Rect,
    pub fit: &'a mut bool,
    pub snap: &'a mut bool,
    pub inspector: &'a mut Option<bool>,
    pub issues: Option<&'a mut bool>,
}

#[derive(Default)]
pub struct FooterStatus {
    pub summary: String,
    pub detail: String,
    pub indicator: Option<(String, Color32)>,
    pub detail_color: Option<Color32>,
}

impl DiagramLayout {
    pub fn new(ui: &Ui, uid: u64, inspector: Option<bool>) -> Self {
        let wide = ui.available_width() > INSPECTOR_BREAKPOINT;
        Self {
            id: Id::unique(("diagram_layout", uid)),
            wide,
            inspector_visible: inspector.unwrap_or(wide),
        }
    }

    /// Wide panes use a side inspector; an explicitly opened narrow inspector is bounded above the canvas.
    pub fn inspector(self, ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
        if !self.inspector_visible {
            return;
        }
        let frame = Frame::new().fill(ui.visuals().panel_fill).inner_margin(Margin {
            left: 14,
            right: 12,
            top: 10,
            bottom: 6,
        });
        if self.wide {
            egui::Panel::right(self.id.with("inspector"))
                .resizable(true)
                .default_size(320.0)
                .size_range(240.0..=450.0)
                .frame(frame)
                .show(ui, contents);
        } else {
            let height = (ui.available_height() * 0.4).clamp(0.0, 220.0);
            egui::Panel::top(self.id.with("compact_inspector"))
                .resizable(true)
                .default_size(height.min(180.0))
                .size_range(height.min(64.0)..=height)
                .frame(frame)
                .show(ui, contents);
        }
    }

    pub fn inspector_tabs<T: Copy + PartialEq>(ui: &mut Ui, value: &mut T, tabs: &[(T, &str)], extra: impl FnOnce(&mut Ui, T)) {
        ui.scope(|ui| {
            theme::toolbar_style(ui.style_mut());
            ui.horizontal_wrapped(|ui| {
                for &(tab, label) in tabs {
                    ui.selectable_value(value, tab, label);
                }
                extra(ui, *value);
            });
        });
        ui.separator();
    }

    pub fn inspector_body(ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, contents);
    }

    /// Optional notation-specific actions, kept separate from the shared canvas configuration.
    pub fn tools(self, ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
        egui::Panel::top(self.id.with("tools"))
            .frame(Frame::new().fill(ui.visuals().panel_fill).inner_margin(Margin::symmetric(12, 4)))
            .show(ui, |ui| {
                theme::toolbar_style(ui.style_mut());
                ui.horizontal_wrapped(contents);
            });
    }

    pub fn configuration(self, ui: &mut Ui, mut controls: CanvasControls<'_>, status: FooterStatus, extra_options: impl FnOnce(&mut Ui)) {
        egui::Panel::bottom(self.id.with("configuration"))
            .exact_size(34.0)
            .frame(Frame::new().fill(ui.visuals().panel_fill).inner_margin(Margin::symmetric(12, 4)))
            .show(ui, |ui| {
                theme::toolbar_style(ui.style_mut());
                let width = ui.available_width();
                let compact = width < CONFIG_BREAKPOINT;
                let inspector_in_menu = width < COMPACT_INSPECTOR_BREAKPOINT;
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if !inspector_in_menu {
                        self.inspector_toggle(ui, controls.inspector);
                    }
                    ui.scope(|ui| {
                        if compact {
                            canvas::zoom_buttons(ui, controls.view, controls.canvas);
                        } else {
                            canvas::zoom_controls(ui, controls.view, controls.canvas, controls.fit);
                        }
                    });
                    if compact {
                        ui.menu_button(tr("common.options"), |ui| {
                            if inspector_in_menu {
                                self.inspector_toggle(ui, controls.inspector);
                            }
                            if ui.button(tr("canvas.zoom_fit")).clicked() {
                                *controls.fit = true;
                                ui.close();
                            }
                            ui.separator();
                            view_options(ui, &mut controls, extra_options);
                            if let Some((label, color)) = &status.indicator {
                                ui.colored_label(*color, label);
                            }
                            if !status.detail.is_empty() {
                                ui.label(&status.detail);
                            }
                        });
                    } else {
                        view_options(ui, &mut controls, extra_options);
                    }
                    if let Some((label, color)) = &status.indicator {
                        if ui.available_width() > 140.0 {
                            ui.label(RichText::new(label).color(*color));
                        }
                    }
                    if ui.available_width() > 48.0 {
                        let text = if status.detail.is_empty() {
                            &status.summary
                        } else {
                            &status.detail
                        };
                        let tooltip = if status.detail.is_empty() {
                            status.summary.clone()
                        } else {
                            format!("{}\n{}", status.summary, status.detail)
                        };
                        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                            ui.add(
                                egui::Label::new(RichText::new(text).color(status.detail_color.unwrap_or(ui.visuals().weak_text_color())))
                                    .truncate(),
                            )
                            .on_hover_text(tooltip);
                        });
                    }
                });
            });
    }

    fn inspector_toggle(self, ui: &mut Ui, inspector: &mut Option<bool>) {
        if ui
            .selectable_label(self.inspector_visible, tr("common.inspector"))
            .on_hover_text(tr("editor.show_hide_side_panel"))
            .clicked()
        {
            *inspector = Some(!self.inspector_visible);
        }
    }

    pub fn canvas(self, ui: &mut Ui, contents: impl FnOnce(&mut Ui)) {
        egui::CentralPanel::default().frame(Frame::NONE).show(ui, contents);
    }
}

fn view_options(ui: &mut Ui, controls: &mut CanvasControls<'_>, extra_options: impl FnOnce(&mut Ui)) {
    ui.checkbox(controls.snap, tr("canvas.snap_grid"));
    if let Some(issues) = controls.issues.as_deref_mut() {
        ui.checkbox(issues, tr("editor.show_issues"));
    }
    extra_options(ui);
}
