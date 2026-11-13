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
            action: Action::New(d.id),
        })
        .collect();
    processes.extend(registry.workflows.iter().map(|w| Process {
        order: w.home_order,
        title: w.title,
        description: w.description,
        badge: w.badge,
        color: w.color,
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
                            if process_card(ui, process, card_width, i) {
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

fn process_card(ui: &mut Ui, process: &Process, width: f32, index: usize) -> bool {
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
        paint_icon(&p, icon, process.color, index);
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

fn paint_icon(p: &egui::Painter, r: Rect, color: Color32, index: usize) {
    let c = r.center();
    let stroke = Stroke::new(1.6, color);
    match index {
        0 => {
            p.line_segment([c - vec2(14.0, 0.0), c + vec2(14.0, 0.0)], stroke);
            p.rect_filled(Rect::from_center_size(c, vec2(16.0, 12.0)), 3.0, color);
            p.circle_stroke(c - vec2(14.0, 0.0), 3.0, stroke);
            p.circle_stroke(c + vec2(14.0, 0.0), 3.0, stroke);
        }
        1 => {
            p.line_segment([c - vec2(10.0, 0.0), c + vec2(10.0, 0.0)], stroke);
            p.circle_stroke(c - vec2(11.0, 0.0), 5.0, stroke);
            p.circle_filled(c - vec2(11.0, 0.0), 1.8, color);
            p.rect_filled(Rect::from_center_size(c, vec2(3.0, 16.0)), 0.0, color);
            p.circle_stroke(c + vec2(11.0, 0.0), 5.0, stroke);
        }
        2 => {
            for i in 0..3 {
                let y = c.y - 9.0 + i as f32 * 9.0;
                p.circle_filled(pos2(c.x - 11.0, y), 2.0, color);
                p.line_segment([pos2(c.x - 4.0, y), pos2(c.x + 12.0 - i as f32 * 3.0, y)], stroke);
            }
        }
        3 => {
            for (i, height) in [10.0, 23.0, 17.0].into_iter().enumerate() {
                p.rect_filled(
                    Rect::from_min_size(c + vec2(-13.0 + i as f32 * 10.0, 12.0 - height), vec2(6.0, height)),
                    2.0,
                    color,
                );
            }
        }
        4 => {
            let class = Rect::from_center_size(c, vec2(26.0, 26.0));
            p.rect_stroke(class, 3.0, stroke, StrokeKind::Inside);
            for y in [-4.0, 4.0] {
                p.line_segment([pos2(class.left(), c.y + y), pos2(class.right(), c.y + y)], stroke);
            }
        }
        _ => {
            let actor = c - vec2(10.0, 0.0);
            p.circle_stroke(actor - vec2(0.0, 9.0), 3.0, stroke);
            p.line_segment([actor - vec2(0.0, 6.0), actor + vec2(0.0, 4.0)], stroke);
            p.line_segment([actor - vec2(5.0, 2.0), actor + vec2(5.0, -2.0)], stroke);
            for x in [-4.0, 4.0] {
                p.line_segment([actor + vec2(0.0, 4.0), actor + vec2(x, 11.0)], stroke);
            }
            p.line_segment([actor + vec2(5.0, 0.0), c + vec2(5.0, 0.0)], stroke);
            p.add(egui::epaint::EllipseShape::stroke(c + vec2(11.0, 0.0), vec2(7.0, 5.0), stroke));
        }
    }
}
