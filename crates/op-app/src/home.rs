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
    /// Moves a saved diagram to the trash.
    Trash(usize),
    Restore(usize),
    /// Erases a trashed diagram now.
    Erase(usize),
}

/// A saved diagram in the trash, erased once `days_left` reaches zero.
pub struct Trashed {
    pub index: usize,
    pub name: String,
    pub kind: String,
    pub color: Color32,
    pub days_left: u64,
}

enum CardEvent {
    Open,
    Trash,
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

pub fn show(
    ui: &mut Ui,
    docs: &[Document],
    saved: &[Document],
    trashed: &[Trashed],
    registry: &crate::editor::DiagramRegistry,
) -> Option<Action> {
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
    processes.extend(
        registry
            .workflows
            .iter()
            .filter(|_| !crate::editor::exam::active(ui.ctx()))
            .map(|w| Process {
                order: w.home_order,
                title: w.title,
                description: w.description,
                badge: w.badge,
                color: w.color,
                icon: w.icon,
                action: Action::Example(w.example, w.command),
            }),
    );
    processes.sort_by_key(|p| p.order);
    let mut action = None;
    let scroll = egui::ScrollArea::vertical().id_salt("home_scroll").auto_shrink([false, true]);
    scroll.show(ui, |ui| {
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
                        .color(theme::accent(ui))
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
                    if let Some((uid, _)) = document_grid(ui, docs, "resume_document_cards", false) {
                        action = Some(Action::Resume(uid));
                    }
                    ui.add_space(26.0);
                }

                if !saved.is_empty() {
                    ui.heading(tr("autosave.library"));
                    ui.add_space(10.0);
                    match document_grid(ui, saved, "saved_document_cards", true) {
                        Some((index, CardEvent::Open)) => action = Some(Action::Saved(index as usize)),
                        Some((index, CardEvent::Trash)) => action = Some(Action::Trash(index as usize)),
                        None => {}
                    }
                    ui.add_space(26.0);
                }
                if !trashed.is_empty() {
                    if let Some(chosen) = trash_section(ui, trashed) {
                        action = Some(chosen);
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

/// Trashed diagrams with their remaining days, to restore or erase now.
fn trash_section(ui: &mut Ui, trashed: &[Trashed]) -> Option<Action> {
    let mut action = None;
    egui::CollapsingHeader::new(RichText::new(trf("autosave.trash", &[&trashed.len()])).heading())
        .id_salt("saved_diagram_trash")
        .show(ui, |ui| {
            ui.label(RichText::new(trf("autosave.trash_notice", &[&crate::platform::autosave::TRASH_DAYS])).weak());
            ui.add_space(6.0);
            egui::Grid::new("saved_diagram_trash_rows")
                .num_columns(4)
                .spacing(vec2(14.0, 8.0))
                .show(ui, |ui| {
                    for item in trashed {
                        ui.label(RichText::new(&item.kind).font(theme::medium(10.5)).color(item.color));
                        ui.label(RichText::new(&item.name).strong());
                        ui.label(RichText::new(trf("autosave.days_left", &[&item.days_left])).weak());
                        ui.horizontal(|ui| {
                            if ui.button(tr("autosave.restore")).clicked() {
                                action = Some(Action::Restore(item.index));
                            }
                            if ui.button(tr("autosave.erase")).clicked() {
                                action = Some(Action::Erase(item.index));
                            }
                        });
                        ui.end_row();
                    }
                });
        });
    action
}

fn document_grid(ui: &mut Ui, docs: &[Document], id: &str, removable: bool) -> Option<(u64, CardEvent)> {
    let gap = 16.0;
    let columns = ((ui.available_width() + gap) / (240.0 + gap)).floor().clamp(1.0, 4.0) as usize;
    let width = ((ui.available_width() - gap * (columns - 1) as f32) / columns as f32).max(1.0);
    let mut chosen = None;
    egui::Grid::new(id).num_columns(columns).spacing(vec2(gap, gap)).show(ui, |ui| {
        for (i, doc) in docs.iter().enumerate() {
            if let Some(event) = document_card(ui, doc, width, removable) {
                chosen = Some((doc.uid, event));
            }
            if (i + 1) % columns == 0 {
                ui.end_row();
            }
        }
    });
    chosen
}

fn document_card(ui: &mut Ui, doc: &Document, width: f32, removable: bool) -> Option<CardEvent> {
    let preview_height = (width - 16.0).max(1.0) * 9.0 / 16.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, preview_height + 76.0), Sense::click());
    let label = if doc.dirty { format!("{} *", doc.name) } else { doc.name.clone() };
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
    if ui.is_rect_visible(rect) {
        let p = ui.painter().with_clip_rect(rect);
        let emphasized = response.hovered() || response.has_focus();
        let accent = theme::accent(ui);
        let radius = theme::CARD_RADIUS;
        p.rect_filled(
            rect,
            radius,
            if emphasized {
                ui.visuals().selection.bg_fill.gamma_multiply(0.12)
            } else {
                ui.visuals().window_fill
            },
        );
        p.rect_stroke(
            rect,
            radius,
            if emphasized {
                Stroke::new(1.5, accent)
            } else {
                ui.visuals().widgets.noninteractive.bg_stroke
            },
            StrokeKind::Inside,
        );
        let preview = Rect::from_min_size(rect.min + vec2(8.0, 8.0), vec2((width - 16.0).max(1.0), preview_height));
        if let Some(texture) = doc.thumbnail.filter(|_| !crate::editor::exam::active(ui.ctx())) {
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
    // Saved diagrams: a trash button in the bottom-right corner, above the card.
    let mut trash = false;
    if removable {
        let button = Rect::from_center_size(rect.right_bottom() - vec2(24.0, 18.0), vec2(26.0, 24.0));
        let r = ui.interact(button, response.id.with("trash"), Sense::click());
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, tr("autosave.move_to_trash")));
        let color = if r.hovered() {
            ui.visuals().error_fg_color
        } else {
            ui.visuals().weak_text_color()
        };
        if r.hovered() {
            ui.painter().rect_filled(button, 5.0, ui.visuals().widgets.hovered.bg_fill);
        }
        paint_trash(ui.painter(), button.center(), color);
        trash = r.on_hover_text(tr("autosave.move_to_trash")).clicked();
        response.context_menu(|ui| {
            theme::menu_style(ui.style_mut());
            if ui.button(tr("autosave.move_to_trash")).clicked() {
                trash = true;
                ui.close();
            }
        });
    }
    let tooltip = if doc.dirty {
        format!("{}\n{} · {}", doc.name, doc.kind, tr("workspace.unsaved"))
    } else {
        format!("{}\n{}", doc.name, doc.kind)
    };
    let open = response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tooltip)
        .clicked();
    if trash {
        Some(CardEvent::Trash)
    } else {
        open.then_some(CardEvent::Open)
    }
}

/// A small bin: lid, handle and body with two ribs.
fn paint_trash(p: &egui::Painter, c: egui::Pos2, color: Color32) {
    let s = Stroke::new(1.4, color);
    p.line_segment([c + vec2(-6.0, -5.0), c + vec2(6.0, -5.0)], s);
    p.line_segment([c + vec2(-2.0, -7.0), c + vec2(2.0, -7.0)], s);
    let body = [c + vec2(-4.5, -3.5), c + vec2(-3.5, 7.0), c + vec2(3.5, 7.0), c + vec2(4.5, -3.5)];
    p.add(egui::Shape::line(body.to_vec(), s));
    for x in [-1.3, 1.3] {
        p.line_segment([c + vec2(x, -1.5), c + vec2(x, 5.0)], Stroke::new(1.0, color));
    }
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
                    .filter(|e| !crate::editor::exam::active(ui.ctx()) && matches!(process.action, Action::New(kind) if kind == e.kind))
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
        let radius = theme::CARD_RADIUS;
        p.rect_filled(rect, radius, ui.visuals().window_fill);
        p.rect_stroke(
            rect,
            radius,
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
            theme::CONTROL_RADIUS,
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
                theme::menu_style(ui.style_mut());
                egui::ScrollArea::vertical().max_height(ui.available_height()).show(ui, |ui| {
                    for example in examples {
                        if ui.button(tr(example.label)).clicked() {
                            chosen = Some(Action::Example(example.id, None));
                            ui.close();
                        }
                    }
                });
            })
            .response
            .on_hover_text(tr("workspace.start_example"));
        examples_clicked = examples_response.clicked();
        examples_response
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, trf("workspace.examples_for", &[&tr(process.title)])));
    }
    chosen.or_else(|| (response.clicked() && !examples_clicked).then_some(process.action))
}
