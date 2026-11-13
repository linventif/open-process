//! Application start page and its process launchers.

use crate::theme;
use egui::{Color32, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use op_core::i18n::tr;

#[derive(Clone, Copy)]
pub enum Action {
    New(&'static str),
    Example(&'static str, Option<&'static str>),
    Open,
    Resume(u64),
}

pub struct Document {
    pub uid: u64,
    pub name: String,
    pub kind: String,
    pub color: Color32,
    pub dirty: bool,
}

struct Process {
    order: usize,
    title: &'static str,
    description: &'static str,
    badge: &'static str,
    color: Color32,
    icon: crate::icons::Icon,
    action: Action,
}

pub fn show(ui: &mut Ui, docs: &[Document], registry: &crate::editor::DiagramRegistry) -> Option<Action> {
    let mut processes: Vec<_> = registry
        .diagrams
        .iter()
        .map(|d| Process {
            order: d.home_order,
            title: d.title,
            description: d.description,
            badge: d.badge,
            color: d.color,
            icon: d.icon,
            action: Action::New(d.id),
        })
        .collect();
    processes.extend(registry.workflows.iter().map(|w| Process {
        order: w.home_order,
        title: w.title,
        description: w.description,
        badge: w.badge,
        color: w.color,
        icon: w.icon,
        action: Action::Example(w.example, w.command),
    }));
    processes.sort_by_key(|p| p.order);
    let mut action = None;
    egui::ScrollArea::vertical().id_salt("home_scroll").show(ui, |ui| {
        let width = (ui.available_width() - 40.0).clamp(0.0, 1120.0);
        let inset = ((ui.available_width() - width) / 2.0).max(0.0);
        ui.horizontal(|ui| {
            ui.add_space(inset);
            ui.vertical(|ui| {
                ui.set_width(width);
                ui.add_space(30.0);
                crate::brand::show(ui, 64.0);
                ui.add_space(8.0);
                ui.label(
                    RichText::new(tr("YOUR WORKSPACE"))
                        .font(theme::semibold(10.5))
                        .color(theme::accent(ui.visuals().dark_mode))
                        .extra_letter_spacing(1.5),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(tr("What would you like to do?")).font(theme::semibold(30.0)));
                ui.label(
                    RichText::new(tr("Model, discover and understand your processes."))
                        .size(15.0)
                        .weak(),
                );
                ui.add_space(14.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.add(theme::primary_button(ui, tr("Open a file…"))).clicked() {
                        action = Some(Action::Open);
                    }
                    ui.label(RichText::new(".opx · .bpmn · .pnml · .xes · .csv · .txt").small().weak());
                });
                ui.add_space(26.0);

                let columns = if width >= 960.0 {
                    3
                } else if width >= 600.0 {
                    2
                } else {
                    1
                };
                let gap = 16.0;
                let card_width = (width - gap * (columns - 1) as f32) / columns as f32;
                egui::Grid::new("home_processes")
                    .num_columns(columns)
                    .spacing(vec2(gap, gap))
                    .show(ui, |ui| {
                        for (i, process) in processes.iter().enumerate() {
                            if process_card(ui, process, card_width) {
                                action = Some(process.action);
                            }
                            if (i + 1) % columns == 0 {
                                ui.end_row();
                            }
                        }
                    });

                if !docs.is_empty() {
                    ui.add_space(24.0);
                    ui.heading(tr("Continue your work"));
                    ui.add_space(6.0);
                    theme::card(ui).show(ui, |ui| {
                        ui.set_width((width - 32.0).max(0.0));
                        for doc in docs {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&doc.kind).font(theme::semibold(10.5)).color(doc.color));
                                let title = if doc.dirty { format!("{} *", doc.name) } else { doc.name.clone() };
                                if ui.add(egui::Button::new(title).frame(false).wrap()).clicked() {
                                    action = Some(Action::Resume(doc.uid));
                                }
                            });
                        }
                    });
                }

                ui.add_space(24.0);
                ui.heading(tr("Start with an example"));
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    for e in &registry.examples {
                        let label = tr(e.label);
                        let example = Action::Example(e.id, None);
                        if ui.button(label).clicked() {
                            action = Some(example);
                        }
                    }
                });
                ui.add_space(30.0);
            });
        });
    });
    action
}

fn process_card(ui: &mut Ui, process: &Process, width: f32) -> bool {
    let (rect, response) = ui.allocate_exact_size(vec2(width, 222.0), Sense::click());
    let response = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tr(process.title)));
    if ui.is_rect_visible(rect) {
        let emphasized = response.hovered() || response.has_focus();
        let p = ui.painter().with_clip_rect(rect);
        p.rect_filled(rect, 12.0, ui.visuals().window_fill);
        p.rect_stroke(
            rect,
            12.0,
            if emphasized {
                Stroke::new(1.5, process.color)
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke
            },
            StrokeKind::Inside,
        );
        let icon = Rect::from_min_size(rect.min + vec2(18.0, 18.0), vec2(44.0, 40.0));
        p.rect_filled(
            icon,
            9.0,
            process.color.gamma_multiply(if ui.visuals().dark_mode { 0.22 } else { 0.1 }),
        );
        (process.icon)(&p, icon.shrink(4.0), process.color);
        p.text(
            pos2(icon.right() + 10.0, icon.center().y),
            egui::Align2::LEFT_CENTER,
            process.badge,
            theme::semibold(10.5),
            process.color,
        );

        let text_width = width - 36.0;
        let title = p.layout(
            tr(process.title).into(),
            theme::semibold(17.0),
            ui.visuals().text_color(),
            text_width,
        );
        let title_height = title.size().y;
        p.galley(rect.min + vec2(18.0, 74.0), title, ui.visuals().text_color());
        let description = p.layout(
            tr(process.description).into(),
            egui::FontId::proportional(12.5),
            ui.visuals().weak_text_color(),
            text_width,
        );
        p.galley(
            rect.min + vec2(18.0, 82.0 + title_height),
            description,
            ui.visuals().weak_text_color(),
        );
        p.text(
            rect.left_bottom() + vec2(18.0, -22.0),
            egui::Align2::LEFT_CENTER,
            tr("Get started"),
            theme::medium(12.5),
            process.color,
        );
        p.text(
            rect.right_bottom() + vec2(-20.0, -22.0),
            egui::Align2::RIGHT_CENTER,
            "→",
            egui::FontId::proportional(19.0),
            process.color,
        );
    }
    response.clicked()
}
