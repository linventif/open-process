//! Application start page and its process launchers.

use crate::theme;
use egui::{Color32, Rect, RichText, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use op_core::i18n::{tr, trf};

#[derive(Clone, Copy)]
pub enum Action {
    New(&'static str),
    Example(&'static str, Option<&'static str>),
    Open,
    Resume(u64),
    Saved(usize),
}

pub struct Document {
    pub uid: u64,
    pub name: String,
    pub kind: String,
    pub color: Color32,
    pub dirty: bool,
    pub icon: crate::icons::Icon,
    pub thumbnail: Option<egui::TextureId>,
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

pub fn show(ui: &mut Ui, docs: &[Document], saved: &[Document], registry: &crate::editor::DiagramRegistry) -> Option<Action> {
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
                    RichText::new(tr("workspace.workspace"))
                        .font(theme::semibold(10.5))
                        .color(theme::accent(ui.visuals().dark_mode))
                        .extra_letter_spacing(1.5),
                );
                ui.add_space(6.0);
                ui.label(RichText::new(tr("workspace.what_would_you_like_do")).font(theme::semibold(30.0)));
                ui.label(RichText::new(tr("workspace.model_discover_understand_processes")).size(15.0).weak());
                ui.add_space(14.0);
                ui.horizontal_wrapped(|ui| {
                    if ui.add(theme::primary_button(ui, tr("file.open_file"))).clicked() {
                        action = Some(Action::Open);
                    }
                    ui.label(RichText::new(".opx · .bpmn · .pnml · .xes · .csv · .txt").small().weak());
                });
                ui.add_space(26.0);

                if !docs.is_empty() {
                    ui.heading(tr("workspace.continue_work"));
                    ui.add_space(10.0);
                    if let Some(uid) = document_grid(ui, docs, "resume_document_cards") {
                        action = Some(Action::Resume(uid));
                    }
                    ui.add_space(26.0);
                }

                if !saved.is_empty() {
                    ui.heading(tr("autosave.library"));
                    ui.add_space(10.0);
                    if let Some(index) = document_grid(ui, saved, "saved_document_cards") {
                        action = Some(Action::Saved(index as usize));
                    }
                    ui.add_space(26.0);
                }
                if !docs.is_empty() || !saved.is_empty() {
                    ui.heading(tr("file.new_document"));
                    ui.add_space(10.0);
                }

                if let Some(chosen) = process_grid(ui, &processes, registry) {
                    action = Some(chosen);
                }

                ui.add_space(30.0);
            });
        });
    });
    action
}

fn document_grid(ui: &mut Ui, docs: &[Document], id: &str) -> Option<u64> {
    let gap = 16.0;
    let columns = ((ui.available_width() + gap) / (240.0 + gap)).floor().clamp(1.0, 4.0) as usize;
    let width = ((ui.available_width() - gap * (columns - 1) as f32) / columns as f32).max(1.0);
    let mut chosen = None;
    egui::Grid::new(id).num_columns(columns).spacing(vec2(gap, gap)).show(ui, |ui| {
        for (i, doc) in docs.iter().enumerate() {
            if document_card(ui, doc, width) {
                chosen = Some(doc.uid);
            }
            if (i + 1) % columns == 0 {
                ui.end_row();
            }
        }
    });
    chosen
}

fn document_card(ui: &mut Ui, doc: &Document, width: f32) -> bool {
    let preview_height = (width - 16.0).max(1.0) * 9.0 / 16.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, preview_height + 76.0), Sense::click());
    let label = if doc.dirty { format!("{} *", doc.name) } else { doc.name.clone() };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    if ui.is_rect_visible(rect) {
        let p = ui.painter().with_clip_rect(rect);
        let emphasized = response.hovered() || response.has_focus();
        let accent = theme::accent(ui.visuals().dark_mode);
        p.rect_filled(
            rect,
            12.0,
            if emphasized {
                ui.visuals().selection.bg_fill.gamma_multiply(0.12)
            } else {
                ui.visuals().window_fill
            },
        );
        p.rect_stroke(
            rect,
            12.0,
            if emphasized {
                Stroke::new(1.5, accent)
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke
            },
            StrokeKind::Inside,
        );
        let preview = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2((width - 16.0).max(1.0), preview_height));
        if let Some(texture) = doc.thumbnail {
            egui::Image::new((texture, preview.size())).corner_radius(6).paint_at(ui, preview);
        } else {
            p.rect_filled(preview, 6.0, doc.color.gamma_multiply(0.09));
            (doc.icon)(&p, Rect::from_center_size(preview.center(), vec2(64.0, 64.0)), doc.color);
        }
        let title_y = preview.bottom() + 20.0;
        (doc.icon)(
            &p,
            Rect::from_center_size(pos2(rect.left() + 23.0, title_y), vec2(20.0, 20.0)),
            doc.color,
        );
        let mut title = egui::text::LayoutJob::simple(
            doc.name.clone(),
            theme::semibold(13.5),
            ui.visuals().text_color(),
            (width - 58.0).max(1.0),
        );
        title.wrap.max_rows = 1;
        title.wrap.break_anywhere = true;
        let galley = p.layout_job(title);
        p.galley(
            pos2(rect.left() + 40.0, title_y - galley.size().y / 2.0),
            galley,
            ui.visuals().text_color(),
        );
        p.text(
            rect.left_bottom() + vec2(14.0, -18.0),
            egui::Align2::LEFT_CENTER,
            &doc.kind,
            theme::medium(10.5),
            doc.color,
        );
        if doc.dirty {
            p.text(
                rect.right_bottom() - vec2(14.0, 18.0),
                egui::Align2::RIGHT_CENTER,
                tr("workspace.unsaved"),
                theme::medium(10.5),
                ui.visuals().weak_text_color(),
            );
        }
    }
    let tooltip = if doc.dirty {
        format!("{}\n{} · {}", doc.name, doc.kind, tr("workspace.unsaved"))
    } else {
        format!("{}\n{}", doc.name, doc.kind)
    };
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip)
        .clicked()
}

/// The creation picker and home page share their cards, sizing and breakpoints.
pub fn diagram_grid(ui: &mut Ui, registry: &crate::editor::DiagramRegistry, query: &str) -> Option<Action> {
    let query = query.trim().to_lowercase();
    let mut processes: Vec<_> = registry
        .diagrams
        .iter()
        .filter(|d| {
            query.is_empty()
                || format!("{} {} {} {} {}", tr(d.new_label), tr(d.title), tr(d.description), d.badge, d.id)
                    .to_lowercase()
                    .contains(&query)
        })
        .map(|d| Process {
            order: d.home_order,
            title: d.new_label,
            description: d.description,
            badge: d.badge,
            color: d.color,
            icon: d.icon,
            action: Action::New(d.id),
        })
        .collect();
    processes.sort_by_key(|p| p.order);
    if processes.is_empty() {
        ui.label(tr("workspace.no_matching_diagram_types"));
        return None;
    }
    process_grid(ui, &processes, registry)
}

fn process_grid(ui: &mut Ui, processes: &[Process], registry: &crate::editor::DiagramRegistry) -> Option<Action> {
    let width = ui.available_width();
    let columns = if width >= 960.0 {
        3
    } else if width >= 600.0 {
        2
    } else {
        1
    };
    let gap = 16.0;
    let card_width = ((width - gap * (columns - 1) as f32) / columns as f32).max(1.0);
    let mut chosen = None;
    egui::Grid::new("document_type_cards")
        .num_columns(columns)
        .spacing(vec2(gap, gap))
        .show(ui, |ui| {
            for (i, process) in processes.iter().enumerate() {
                let examples: Vec<_> = registry
                    .examples
                    .iter()
                    .filter(|e| matches!(process.action, Action::New(kind) if kind == e.kind))
                    .collect();
                if let Some(action) = process_card(ui, process, card_width, &examples) {
                    chosen = Some(action);
                }
                if (i + 1) % columns == 0 {
                    ui.end_row();
                }
            }
        });
    chosen
}

fn process_card(ui: &mut Ui, process: &Process, width: f32, examples: &[&crate::editor::ExampleDescriptor]) -> Option<Action> {
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
            tr("workspace.get_started"),
            theme::medium(12.5),
            process.color,
        );
        if examples.is_empty() {
            p.text(
                rect.right_bottom() + vec2(-20.0, -22.0),
                egui::Align2::RIGHT_CENTER,
                "→",
                egui::FontId::proportional(19.0),
                process.color,
            );
        }
    }
    let mut chosen = None;
    let mut examples_clicked = false;
    if !examples.is_empty() {
        // The card already occupies its grid cell; this child places its button
        // inside that rectangle without advancing the grid a second time.
        let mut footer = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("card_examples", examples[0].kind))
                .max_rect(Rect::from_min_max(
                    rect.left_bottom() + vec2(18.0, -38.0),
                    rect.right_bottom() - vec2(12.0, 10.0),
                ))
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
        );
        let examples_response = footer
            .menu_button(tr("menu.examples"), |ui| {
                for example in examples {
                    if ui.button(tr(example.label)).clicked() {
                        chosen = Some(Action::Example(example.id, None));
                        ui.close();
                    }
                }
            })
            .response
            .on_hover_text(tr("workspace.start_example"));
        examples_clicked = examples_response.clicked();
        examples_response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, trf("workspace.examples_for", &[&tr(process.title)])));
    }
    chosen.or_else(|| (response.clicked() && !examples_clicked).then_some(process.action))
}
