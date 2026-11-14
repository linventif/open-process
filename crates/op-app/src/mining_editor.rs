//! Process discovery: event log → relation matrix → Alpha algorithm → Petri net → BPMN.

pub(crate) mod analysis_job;
mod results;
mod session;
pub(crate) use session::register;

use crate::canvas::Palette;
use crate::petri_editor::PetriEditor;
use crate::theme;
use egui::{Color32, RichText, Ui};
use op_core::analysis::Soundness;
use op_core::answers::{RelationList, Step};
use op_core::bpmn::BpmnDiagram;
use op_core::convert;
use op_core::i18n::{key, tr, trf};
use op_core::mining::{AlphaResult, EventLog, Relation, replay};
use op_core::petri::PetriNet;
use std::collections::BTreeMap;

pub enum MiningAction {
    OpenPetri(PetriNet),
    OpenBpmn(BpmnDiagram),
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Computed {
    text: String,
    lang: op_core::i18n::Lang,
    exam: bool,
    result: Result<MiningOutput, String>,
    soundness: Option<Soundness>,
}

#[derive(serde::Serialize, serde::Deserialize)]
enum MiningOutput {
    Discovery(EventLog, AlphaResult),
    Exam(Vec<String>),
}

pub struct MiningEditor {
    history: crate::history::History<op_core::document::LogDocument>,
    pub name: String,
    pub text: String,
    /// Retained for compatibility with existing event-log documents.
    legacy_manual: bool,
    pub answers: BTreeMap<String, String>,
    computed: Option<Computed>,
    pending: Option<(String, op_core::i18n::Lang, bool, crate::platform::jobs::Job)>,
    preview: Option<PetriEditor>,
    pub action: Option<MiningAction>,
    matrix_rect: egui::Rect,
    pub status: String,
}

pub const EXAMPLE_1: &str = "case 1 : task A\ncase 2 : task A\ncase 3 : task A\ncase 3 : task B\ncase 1 : task B\n\
case 1 : task C\ncase 2 : task C\ncase 4 : task A\ncase 2 : task B\ncase 2 : task D\ncase 5 : task E\ncase 4 : task C\n\
case 1 : task D\ncase 3 : task C\ncase 3 : task D\ncase 4 : task B\ncase 5 : task F\ncase 4 : task D\n";

pub const EXAMPLE_2: &str = "ABCD\nACBD\nAED\n";

const FORMATS: &str = key("mining.accepted_formats_case_1_task_one_event");

impl MiningEditor {
    pub fn new(doc: op_core::document::LogDocument) -> Self {
        let history = crate::history::History::new(&doc);
        Self {
            history,
            name: doc.name,
            text: doc.text,
            legacy_manual: doc.manual,
            answers: doc.answers,
            computed: None,
            pending: None,
            preview: None,
            action: None,
            matrix_rect: egui::Rect::NOTHING,
            status: String::new(),
        }
    }

    fn recompute(&mut self, ui: &Ui) {
        let language = op_core::i18n::lang();
        let exam = crate::editor::exam::active(ui.ctx());
        if self
            .computed
            .as_ref()
            .is_some_and(|c| c.text == self.text && c.lang == language && c.exam == exam)
        {
            return;
        }
        if self
            .pending
            .as_ref()
            .is_none_or(|(text, lang, mode, _)| *text != self.text || *lang != language || *mode != exam)
        {
            self.pending = None;
            self.computed = None;
            self.preview = None;
            let input = serde_json::to_string(&analysis_job::Input {
                task: "mining.discovery".into(),
                text: self.text.clone(),
                lang: language,
                exam,
            })
            .unwrap();
            match crate::platform::jobs::Job::spawn(input, analysis_job::compute_json, ui.ctx()) {
                Ok(job) => self.pending = Some((self.text.clone(), language, exam, job)),
                Err(error) => {
                    self.computed = Some(Computed {
                        text: self.text.clone(),
                        lang: language,
                        exam,
                        result: Err(error),
                        soundness: None,
                    })
                }
            }
        }
        if let Some(result) = self.pending.as_mut().and_then(|(_, _, _, job)| job.poll()) {
            self.pending = None;
            self.computed = Some(
                result
                    .and_then(|raw| serde_json::from_str::<Computed>(&raw).map_err(|e| e.to_string()))
                    .unwrap_or_else(|error| Computed {
                        text: self.text.clone(),
                        lang: language,
                        exam,
                        result: Err(error),
                        soundness: None,
                    }),
            );
            self.preview = self
                .computed
                .as_ref()
                .and_then(|c| c.result.as_ref().ok())
                .and_then(|result| match result {
                    MiningOutput::Discovery(_, r) => Some(PetriEditor::new(r.net.clone())),
                    MiningOutput::Exam(_) => None,
                });
        }
        if self.pending.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        self.recompute(ui);
        let panel = ui.visuals().panel_fill;
        egui::Panel::left("log_input")
            .resizable(true)
            .default_size(320.0)
            .min_size(240.0)
            .frame(egui::Frame::new().fill(panel).inner_margin(egui::Margin {
                left: 14,
                right: 12,
                top: 12,
                bottom: 10,
            }))
            .show(ui, |ui| self.input_panel(ui));
        egui::Panel::bottom("log_status")
            .exact_size(30.0)
            .frame(egui::Frame::new().fill(panel).inner_margin(egui::Margin::symmetric(12, 3)))
            .show(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    if let Some(Err(e)) = self.computed.as_ref().map(|c| &c.result) {
                        ui.label(RichText::new(e).color(ui.visuals().error_fg_color));
                    }
                    ui.label(RichText::new(&self.status).color(ui.visuals().weak_text_color()));
                });
            });
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Palette::for_ui(ui).bg)
                    .inner_margin(egui::Margin::symmetric(20, 0)),
            )
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| self.results(ui));
            });
        self.history.sync(
            &self.log_document(),
            ui.input(|i| i.pointer.any_down()) || ui.ctx().egui_wants_keyboard_input(),
        );
    }

    fn log_document(&self) -> op_core::document::LogDocument {
        op_core::document::LogDocument {
            name: self.name.clone(),
            text: self.text.clone(),
            manual: self.legacy_manual,
            answers: self.answers.clone(),
        }
    }
    fn restore_document(&mut self, doc: op_core::document::LogDocument) {
        self.name = doc.name;
        self.text = doc.text;
        self.legacy_manual = doc.manual;
        self.answers = doc.answers;
        self.computed = None;
        self.pending = None;
        self.preview = None;
    }

    fn input_panel(&mut self, ui: &mut Ui) {
        let exam = crate::editor::exam::active(ui.ctx());
        ui.label(RichText::new(tr("mining.event_log")).font(theme::semibold(16.0)));
        theme::section_title(ui, tr("common.name"));
        let lbl = ui.label(RichText::new(tr("mining.name_log")).small().weak());
        ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(f32::INFINITY))
            .labelled_by(lbl.id);
        theme::section_title(ui, tr("mining.traces"));
        let lbl = ui.label(RichText::new(tr("mining.one_event_trace_per_line")).small().weak());
        let help_h = 120.0;
        let h = (ui.available_height() - help_h).max(120.0);
        egui::ScrollArea::vertical().id_salt("log_text").max_height(h).show(ui, |ui| {
            ui.add_sized(
                egui::vec2(ui.available_width(), h),
                egui::TextEdit::multiline(&mut self.text)
                    .code_editor()
                    .margin(egui::Margin::same(10))
                    .hint_text(if exam { "" } else { "ABCD\nACBD\nEF" }),
            )
            .labelled_by(lbl.id);
        });
        if exam {
            return;
        }
        ui.add_space(6.0);
        egui::CollapsingHeader::new(RichText::new(tr("mining.accepted_formats")).small())
            .default_open(false)
            .show(ui, |ui| {
                ui.label(RichText::new(tr(FORMATS)).small().weak());
            });
    }
}

/// Numbered card section.
fn card_section(ui: &mut Ui, number: &str, title: &str, subtitle: &str, add: impl FnOnce(&mut Ui)) {
    ui.add_space(12.0);
    theme::card(ui).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.horizontal(|ui| {
                let accent = theme::accent(ui.visuals().dark_mode);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
                ui.painter().circle_filled(rect.center(), 11.0, accent);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    number,
                    theme::semibold(12.0),
                    Color32::WHITE,
                );
                ui.label(RichText::new(title).font(theme::semibold(15.0)));
                if !subtitle.is_empty() {
                    ui.label(RichText::new(subtitle).color(ui.visuals().weak_text_color()));
                }
            });
            ui.add_space(10.0);
            add(ui);
        });
    });
}

/// Rounded tag.
fn chip(ui: &mut Ui, text: &str, color: Color32) -> egui::Response {
    let dark = ui.visuals().dark_mode;
    egui::Frame::new()
        .fill(color.gamma_multiply(if dark { 0.22 } else { 0.10 }))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            let text_color = if color == Palette::for_ui(ui).muted {
                ui.visuals().text_color()
            } else {
                color
            };
            ui.label(RichText::new(text).font(egui::FontId::monospace(13.0)).color(text_color));
        })
        .response
}

/// The relation matrix drawn as a bordered table, as in the course slides.
fn relation_table(ui: &mut Ui, f: &op_core::mining::Footprint, pal: &Palette) -> egui::Rect {
    let n = f.len();
    let cell = egui::vec2(44.0, 36.0);
    let size = egui::vec2(cell.x * (n + 1) as f32, cell.y * (n + 1) as f32);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter();
    let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let header_bg = ui.visuals().faint_bg_color;
    p.rect_filled(rect, 8.0, ui.visuals().window_fill);
    p.rect_filled(
        egui::Rect::from_min_size(rect.min, egui::vec2(size.x, cell.y)),
        egui::CornerRadius {
            nw: 8,
            ne: 8,
            sw: 0,
            se: 0,
        },
        header_bg,
    );
    p.rect_filled(
        egui::Rect::from_min_size(rect.min + egui::vec2(0.0, cell.y), egui::vec2(cell.x, size.y - cell.y)),
        egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: 8,
            se: 0,
        },
        header_bg,
    );
    let at = |i: usize, j: usize| egui::Rect::from_min_size(rect.min + egui::vec2(cell.x * j as f32, cell.y * i as f32), cell);
    let hover = resp.hover_pos().map(|h| {
        let k = h - rect.min;
        ((k.y / cell.y) as usize, (k.x / cell.x) as usize)
    });
    if let Some((i, j)) = hover
        && i >= 1
        && j >= 1
        && i <= n
        && j <= n
    {
        p.rect_filled(at(i, 0), 0.0, pal.select.gamma_multiply(0.12));
        p.rect_filled(at(0, j), 0.0, pal.select.gamma_multiply(0.12));
        p.rect_filled(at(i, j), 0.0, pal.select.gamma_multiply(0.12));
    }
    for k in 1..=n {
        p.text(
            at(0, k).center(),
            egui::Align2::CENTER_CENTER,
            &f.activities[k - 1],
            theme::semibold(14.0),
            pal.text,
        );
        p.text(
            at(k, 0).center(),
            egui::Align2::CENTER_CENTER,
            &f.activities[k - 1],
            theme::semibold(14.0),
            pal.text,
        );
    }
    for a in 0..n {
        for b in 0..n {
            let color = match f.rel[a][b] {
                Relation::Causal => pal.ok,
                Relation::Reverse => pal.end,
                Relation::Parallel => pal.select,
                Relation::Independent => pal.muted,
            };
            p.text(
                at(a + 1, b + 1).center(),
                egui::Align2::CENTER_CENTER,
                f.cell(a, b),
                theme::semibold(18.0),
                color,
            );
        }
    }
    for k in 1..=n {
        let s = egui::Stroke::new(if k == 1 { 1.4 } else { 1.0 }, border);
        p.line_segment(
            [
                rect.left_top() + egui::vec2(0.0, cell.y * k as f32),
                rect.right_top() + egui::vec2(0.0, cell.y * k as f32),
            ],
            s,
        );
        p.line_segment(
            [
                rect.left_top() + egui::vec2(cell.x * k as f32, 0.0),
                rect.left_bottom() + egui::vec2(cell.x * k as f32, 0.0),
            ],
            s,
        );
    }
    p.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
    if let Some((i, j)) = hover
        && (1..=n).contains(&i)
        && (1..=n).contains(&j)
    {
        let (a, b) = (i - 1, j - 1);
        let (x, y) = (&f.activities[a], &f.activities[b]);
        let hint = match f.rel[a][b] {
            Relation::Causal => trf("mining.relation.causal_reason", &[&x, &y, &x, &y, &y, &x]),
            Relation::Reverse => trf("mining.relation.reverse_causal_reason", &[&x, &y, &y, &x, &x, &y]),
            Relation::Parallel => trf("mining.relation.parallel_reason", &[&x, &y, &x, &y, &y, &x]),
            Relation::Independent => trf("mining.relation.independence_reason", &[&x, &y, &x, &y, &y, &x]),
        };
        resp.on_hover_text_at_pointer(hint);
    }
    rect
}

// ---------------------------------------------------------------------------
// Examination answers contain no corrections, examples or generated previews.

fn exam_sections(ui: &mut Ui, activities: &[String], answers: &mut BTreeMap<String, String>, pal: &Palette) {
    card_section(ui, "2", tr("mining.relations_between_activities"), "", |ui| {
        for list in RelationList::ALL {
            let label = match list {
                RelationList::Succession => tr("mining.direct_succession_x_y"),
                RelationList::Causality => tr("mining.causality_x_y"),
                RelationList::Parallel => tr("mining.parallelism_x_y"),
                RelationList::Independence => tr("mining.independence_x_y"),
            };
            exam_answer(ui, &format!("rel:{}", list.key()), label, false, answers);
        }
    });
    card_section(
        ui,
        "3",
        tr("mining.relation_matrix"),
        tr("mining.click_cell_cycle_right_click_goes_back"),
        |ui| {
            if ui.button(tr("common.clear")).clicked() {
                answers.retain(|key, _| !key.starts_with("m:"));
            }
            egui::ScrollArea::horizontal().id_salt("exam_matrix").show(ui, |ui| {
                editable_relation_table(ui, activities, answers, pal);
            });
        },
    );
    card_section(ui, "4", tr("mining.alpha_algorithm"), "", |ui| {
        for step in Step::ALL {
            exam_answer(
                ui,
                &format!("step:{}", step.key()),
                step.key(),
                matches!(step, Step::Xw | Step::Yw | Step::Pw | Step::Fw),
                answers,
            );
        }
    });
    if ui.button(tr("mining.clear_all_my_answers")).clicked() {
        answers.clear();
    }
    ui.add_space(24.0);
}

fn exam_answer(ui: &mut Ui, key: &str, label: &str, multiline: bool, answers: &mut BTreeMap<String, String>) {
    let label = ui.label(RichText::new(label).font(theme::medium(13.0)));
    let mut text = answers.get(key).cloned().unwrap_or_default();
    let edit = if multiline {
        egui::TextEdit::multiline(&mut text).desired_rows(2)
    } else {
        egui::TextEdit::singleline(&mut text)
    };
    let response = ui
        .add(
            edit.desired_width(f32::INFINITY)
                .font(egui::FontId::monospace(13.0))
                .background_color(ui.visuals().faint_bg_color),
        )
        .labelled_by(label.id);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, key));
    if response.changed() {
        answers.insert(key.into(), text);
    }
    ui.add_space(6.0);
}

const CYCLE: [&str; 5] = ["", "→", "←", "||", "#"];

/// Relation matrix filled by clicking, without access to expected relations.
fn editable_relation_table(ui: &mut Ui, activities: &[String], answers: &mut BTreeMap<String, String>, pal: &Palette) {
    let n = activities.len();
    let cell = egui::vec2(44.0, 36.0);
    let size = egui::vec2(cell.x * (n + 1) as f32, cell.y * (n + 1) as f32);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let at = |i: usize, j: usize| egui::Rect::from_min_size(rect.min + egui::vec2(cell.x * j as f32, cell.y * i as f32), cell);
    let hover = resp.hover_pos().map(|h| {
        let k = h - rect.min;
        ((k.y / cell.y) as usize, (k.x / cell.x) as usize)
    });
    let key = |a: usize, b: usize| format!("m:{}:{}", activities[a], activities[b]);
    if let Some((i, j)) = hover
        && (1..=n).contains(&i)
        && (1..=n).contains(&j)
    {
        let step: i32 = if resp.clicked() {
            1
        } else if resp.secondary_clicked() {
            -1
        } else {
            0
        };
        if step != 0 {
            let k = key(i - 1, j - 1);
            let cur = answers.get(&k).map(String::as_str).unwrap_or("");
            let idx = CYCLE.iter().position(|c| *c == cur).unwrap_or(0) as i32;
            let next = CYCLE[(idx + step).rem_euclid(CYCLE.len() as i32) as usize];
            answers.insert(k, next.to_string());
        }
    }
    let p = ui.painter();
    let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
    let header_bg = ui.visuals().faint_bg_color;
    p.rect_filled(rect, 8.0, ui.visuals().window_fill);
    p.rect_filled(
        egui::Rect::from_min_size(rect.min, egui::vec2(size.x, cell.y)),
        egui::CornerRadius {
            nw: 8,
            ne: 8,
            sw: 0,
            se: 0,
        },
        header_bg,
    );
    p.rect_filled(
        egui::Rect::from_min_size(rect.min + egui::vec2(0.0, cell.y), egui::vec2(cell.x, size.y - cell.y)),
        egui::CornerRadius {
            nw: 0,
            ne: 0,
            sw: 8,
            se: 0,
        },
        header_bg,
    );
    for k in 1..=n {
        p.text(
            at(0, k).center(),
            egui::Align2::CENTER_CENTER,
            &activities[k - 1],
            theme::semibold(14.0),
            pal.text,
        );
        p.text(
            at(k, 0).center(),
            egui::Align2::CENTER_CENTER,
            &activities[k - 1],
            theme::semibold(14.0),
            pal.text,
        );
    }
    for a in 0..n {
        for b in 0..n {
            let v = answers.get(&key(a, b)).map(String::as_str).unwrap_or("");
            let r = at(a + 1, b + 1);
            if hover == Some((a + 1, b + 1)) {
                p.rect_filled(r.shrink(1.0), 0.0, pal.select.gamma_multiply(0.15));
            }
            p.text(r.center(), egui::Align2::CENTER_CENTER, v, theme::semibold(18.0), pal.text);
        }
    }
    for k in 1..=n {
        let s = egui::Stroke::new(if k == 1 { 1.4 } else { 1.0 }, border);
        p.line_segment(
            [
                rect.left_top() + egui::vec2(0.0, cell.y * k as f32),
                rect.right_top() + egui::vec2(0.0, cell.y * k as f32),
            ],
            s,
        );
        p.line_segment(
            [
                rect.left_top() + egui::vec2(cell.x * k as f32, 0.0),
                rect.left_bottom() + egui::vec2(cell.x * k as f32, 0.0),
            ],
            s,
        );
    }
    p.rect_stroke(rect, 8.0, egui::Stroke::new(1.0, border), egui::StrokeKind::Inside);
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Grid, true, tr("mining.relation_matrix_editable")));
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
}
