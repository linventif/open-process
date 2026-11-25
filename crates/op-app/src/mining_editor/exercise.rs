//! Exercise mode: random Alpha exercises whose solution is revealed step by step.
//!
//! A generated log is ordinary document content: it replaces the source text in
//! one undoable edit and is saved like any other log. Hiding the results and the
//! number of revealed steps are view state, restored with the workspace.

use super::results::{copy_menu, matrix_legend, open_actions, relation_lists, step_badge, word_check};
use super::*;
use op_core::mining::exercise::{self, Difficulty};

/// Per-view exercise state.
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ExerciseView {
    /// Hide the results and reveal them one step at a time.
    pub active: bool,
    /// Number of revealed steps, from the start of [`Step::ALL`].
    pub revealed: usize,
    pub difficulty: Difficulty,
    /// Step brought into view on the next frame.
    #[serde(skip)]
    scroll: Option<usize>,
}

impl ExerciseView {
    fn reveal(&mut self, count: usize) {
        self.revealed = count.min(Step::ALL.len());
        self.scroll = self.revealed.checked_sub(1);
    }
}

/// Steps of the walkthrough, in the order of the course.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Successions,
    Relations,
    Matrix,
    Tw,
    Ti,
    To,
    Xw,
    Yw,
    Pw,
    Fw,
    Net,
}

impl Step {
    pub(super) const ALL: [Step; 11] = [
        Step::Successions,
        Step::Relations,
        Step::Matrix,
        Step::Tw,
        Step::Ti,
        Step::To,
        Step::Xw,
        Step::Yw,
        Step::Pw,
        Step::Fw,
        Step::Net,
    ];

    fn title(self) -> &'static str {
        match self {
            Step::Successions => key("mining.exercise.step.successions"),
            Step::Relations => key("mining.relations_between_activities"),
            Step::Matrix => key("mining.relation_matrix"),
            Step::Tw => key("mining.exercise.step.tw"),
            Step::Ti => key("mining.exercise.step.ti"),
            Step::To => key("mining.exercise.step.to"),
            Step::Xw => key("mining.exercise.step.xw"),
            Step::Yw => key("mining.exercise.step.yw"),
            Step::Pw => key("mining.exercise.step.pw"),
            Step::Fw => key("mining.exercise.step.fw"),
            Step::Net => key("mining.exercise.step.net"),
        }
    }

    /// What to compute, shown before the answer is revealed.
    fn rule(self) -> &'static str {
        match self {
            Step::Successions => key("mining.x_directly_followed_y_at_least_one"),
            Step::Relations => key("mining.exercise.rule.relations"),
            Step::Matrix => key("mining.row_x_column_y_relation_between_x"),
            Step::Tw => key("mining.exercise.rule.tw"),
            Step::Ti => key("mining.exercise.rule.ti"),
            Step::To => key("mining.exercise.rule.to"),
            Step::Xw => key("mining.pairs_b_every_causes_every_b_b"),
            Step::Yw => key("mining.exercise.rule.yw"),
            Step::Pw => key("mining.one_place_per_yw_pair_plus_iw"),
            Step::Fw => key("mining.exercise.rule.fw"),
            Step::Net => key("mining.exercise.rule.net"),
        }
    }
}

/// Discovery results shown by the walkthrough.
pub(super) struct Solved<'a> {
    pub name: &'a str,
    pub log: &'a EventLog,
    pub r: &'a AlphaResult,
    pub replays: &'a [Result<(), String>],
    pub soundness: Option<&'a Soundness>,
    pub report: &'a str,
}

/// Host requests from the last step.
#[derive(Default)]
pub(super) struct Requests {
    pub action: Option<MiningAction>,
    pub copy: Option<(String, &'static str)>,
}

/// Longest explanation shown for one step; the remainder is counted.
const DETAIL_LIMIT: usize = 12;

impl MiningEditor {
    /// Generator and reveal toggle in the input panel.
    pub(super) fn exercise_controls(&mut self, ui: &mut Ui) {
        theme::section_title(ui, tr("mining.exercise.title"));
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("exercise_difficulty")
                .selected_text(tr(self.exercise.difficulty.label()))
                .show_ui(ui, |ui| {
                    for difficulty in Difficulty::ALL {
                        ui.selectable_value(&mut self.exercise.difficulty, difficulty, tr(difficulty.label()));
                    }
                })
                .response
                .on_hover_text(tr("mining.exercise.difficulty"));
            if ui
                .button(tr("mining.exercise.generate"))
                .on_hover_text(tr("mining.exercise.generate_help"))
                .clicked()
            {
                self.generate_exercise(random_seed());
            }
        });
        let mut active = self.exercise.active;
        if ui
            .checkbox(&mut active, tr("mining.exercise.step_by_step"))
            .on_hover_text(tr("mining.exercise.step_by_step_help"))
            .changed()
        {
            self.exercise.active = active;
            self.exercise.reveal(0);
        }
    }

    /// Replaces the log with a new exercise in one undoable edit. Answers typed
    /// for the previous log and its discovery options belong to that log.
    pub(super) fn generate_exercise(&mut self, seed: u64) {
        let generated = exercise::generate(self.exercise.difficulty, seed);
        self.text = generated.source();
        self.answers.clear();
        self.discovery = DiscoveryOptions::default();
        self.import_note = None;
        self.exercise.active = true;
        self.exercise.reveal(0);
        self.status = trf("mining.exercise.generated", &[&tr(generated.difficulty.label())]);
    }

    /// Previous/next controls, kept visible in the status bar (right to left).
    pub(super) fn exercise_navigation(&mut self, ui: &mut Ui) {
        let total = Step::ALL.len();
        let revealed = self.exercise.revealed.min(total);
        if ui
            .add_enabled(revealed < total, egui::Button::new(tr("mining.exercise.next")))
            .clicked()
        {
            self.exercise.reveal(revealed + 1);
        }
        ui.label(RichText::new(trf("mining.exercise.progress", &[&revealed, &total])).weak());
        if ui
            .add_enabled(revealed > 0, egui::Button::new(tr("mining.exercise.previous")))
            .clicked()
        {
            self.exercise.reveal(revealed - 1);
        }
    }
}

fn random_seed() -> u64 {
    let (high, low) = uuid::Uuid::new_v4().as_u64_pair();
    high ^ low.rotate_left(32)
}

/// The statement, every revealed step, then the next step's question.
pub(super) fn exercise_results(ui: &mut Ui, view: &mut ExerciseView, solved: &Solved, preview: &mut Option<PetriEditor>) -> Requests {
    let pal = Palette::for_ui(ui);
    let total = Step::ALL.len();
    view.revealed = view.revealed.min(total);
    let mut requests = Requests::default();
    let mut reveal = None;
    let max_w = 980.0f32.min(ui.available_width());
    ui.vertical_centered(|ui| {
        ui.set_max_width(max_w);
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new(solved.name).font(theme::semibold(20.0)));
                ui.label(RichText::new(trf("mining.exercise.progress", &[&view.revealed, &total])).weak());
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add_enabled(view.revealed > 0, egui::Button::new(tr("mining.exercise.hide_all")))
                    .clicked()
                {
                    view.reveal(0);
                }
                if ui
                    .add_enabled(view.revealed < total, egui::Button::new(tr("mining.exercise.show_all")))
                    .clicked()
                {
                    view.revealed = total;
                }
            });
        });
        if solved.r.boundary_activities.is_some() {
            ui.label(RichText::new(tr("mining.report.normalized_boundaries")).small().weak());
        }
        if !solved.r.projected_activities.is_empty() {
            ui.label(RichText::new(tr("mining.report.projected_markers")).small().weak());
        }

        card_section(ui, "W", tr("mining.exercise.statement"), "", |ui| {
            ui.label(tr("mining.exercise.statement_text"));
            ui.add_space(8.0);
            ui.horizontal_wrapped(|ui| {
                for (trace, count) in solved.log.variants() {
                    let text = solved.log.format_trace(&trace);
                    let text = if count > 1 { format!("{text}  ×{count}") } else { text };
                    chip(ui, &text, ui.visuals().text_color());
                }
            });
        });

        for (index, step) in Step::ALL.into_iter().enumerate().take(view.revealed + 1) {
            let shown = index < view.revealed;
            let card = ui.scope(|ui| {
                card_section(ui, &(index + 1).to_string(), tr(step.title()), "", |ui| {
                    ui.add(egui::Label::new(RichText::new(tr(step.rule())).weak()).wrap());
                    ui.add_space(8.0);
                    if shown {
                        step_answer(ui, step, solved, preview, &pal, &mut requests);
                    } else {
                        ui.label(RichText::new(tr("mining.exercise.try_first")).italics().weak());
                        ui.add_space(6.0);
                        if ui.add(theme::primary_button(ui, tr("mining.exercise.show_answer"))).clicked() {
                            reveal = Some(index + 1);
                        }
                    }
                });
            });
            if view.scroll == Some(index) {
                card.response.scroll_to_me(Some(egui::Align::Min));
                view.scroll = None;
            }
        }
        if view.revealed == total {
            ui.add_space(12.0);
            ui.horizontal(|ui| chip(ui, tr("mining.exercise.complete"), pal.ok));
        }
        ui.add_space(24.0);
    });
    if let Some(count) = reveal {
        view.reveal(count);
        ui.ctx().request_repaint();
    }
    requests
}

fn step_answer(ui: &mut Ui, step: Step, solved: &Solved, preview: &mut Option<PetriEditor>, pal: &Palette, requests: &mut Requests) {
    let (log, r) = (solved.log, solved.r);
    let f = &r.footprint;
    let variants = log.variants();
    let per_trace = |ui: &mut Ui, separator: &str, describe: &dyn Fn(&[String]) -> String| {
        explanation(
            ui,
            variants.iter().map(|(trace, _)| {
                // Explain the same activities used to construct the footprint,
                // including collision-safe boundaries, without editing the source.
                let normalized = r.boundary_activities.as_ref().map(|[start, end]| {
                    std::iter::once(start.clone())
                        .chain(trace.iter().cloned())
                        .chain(std::iter::once(end.clone()))
                        .collect::<Vec<_>>()
                });
                let trace = normalized.as_deref().unwrap_or(trace);
                let text = if normalized.is_some() {
                    format!("⟨{}⟩", trace.join(", "))
                } else {
                    log.format_trace(trace)
                };
                format!("{text}{separator}{}", describe(trace))
            }),
            variants.len(),
        );
    };
    let pairs = |v: &[op_core::mining::Pair]| format!("{{{}}}", v.iter().map(|p| r.pair(p)).collect::<Vec<_>>().join(", "));
    match step {
        Step::Successions => {
            per_trace(ui, ": ", &|trace| {
                let mut seen = Vec::new();
                for w in trace.windows(2) {
                    let succession = format!("{}>{}", w[0], w[1]);
                    if !seen.contains(&succession) {
                        seen.push(succession);
                    }
                }
                seen.join(", ")
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for (a, b) in f.successions() {
                    chip(ui, &format!("{}>{}", f.activities[a], f.activities[b]), pal.muted);
                }
            });
        }
        Step::Relations => relation_lists(ui, f, pal),
        Step::Matrix => {
            egui::ScrollArea::horizontal().id_salt("exercise_matrix").show(ui, |ui| {
                relation_table(ui, f, pal);
            });
            ui.add_space(6.0);
            matrix_legend(ui, pal);
        }
        Step::Tw => answer(ui, "TW", step, &r.set(&r.t_w), pal),
        Step::Ti => {
            per_trace(ui, " → ", &|trace| trace.first().cloned().unwrap_or_default());
            answer(ui, "TI", step, &r.set(&r.t_i), pal);
        }
        Step::To => {
            per_trace(ui, " → ", &|trace| trace.last().cloned().unwrap_or_default());
            answer(ui, "TO", step, &r.set(&r.t_o), pal);
        }
        Step::Xw => answer(ui, "XW", step, &pairs(&r.x_w), pal),
        Step::Yw => {
            let removed = r.x_w.len().saturating_sub(r.y_w.len());
            if removed == 0 {
                ui.label(RichText::new(tr("mining.exercise.all_maximal")).weak());
                ui.add_space(6.0);
            } else {
                ui.label(RichText::new(trf("mining.exercise.removed_count", &[&removed])).weak());
                explanation(
                    ui,
                    exercise::removed_pairs(r, DETAIL_LIMIT)
                        .into_iter()
                        .map(|(pair, wider)| format!("{} ⊂ {}", r.pair(pair), r.pair(wider))),
                    removed,
                );
            }
            answer(ui, "YW", step, &pairs(&r.y_w), pal);
        }
        Step::Pw => {
            let mut places: Vec<String> = r.y_w.iter().map(|p| r.place_name(p)).collect();
            places.extend(["iw".into(), "ow".into()]);
            answer(ui, "PW", step, &format!("{{{}}}", places.join(", ")), pal);
        }
        Step::Fw => {
            let arcs: Vec<String> = r.flow_relation().iter().map(|(a, b)| format!("({a},{b})")).collect();
            answer(ui, "FW", step, &format!("{{{}}}", arcs.join(", ")), pal);
        }
        Step::Net => {
            answer(ui, "α(W)", step, "(PW, TW, FW)", pal);
            ui.add_space(6.0);
            if let Some(p) = preview {
                let w = ui.available_width();
                p.preview_ui(ui, egui::vec2(w, 400.0));
            }
            ui.add_space(10.0);
            ui.label(RichText::new(tr("mining.word_check")).font(theme::medium(13.0)));
            ui.add_space(4.0);
            word_check(ui, log, solved.replays, solved.soundness, pal);
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                requests.action = open_actions(ui, r, solved.name);
                requests.copy = copy_menu(ui, f, solved.report);
            });
        }
    }
}

/// The value of a step next to its badge.
fn answer(ui: &mut Ui, label: &str, step: Step, value: &str, pal: &Palette) {
    egui::Grid::new(("exercise_answer", label))
        .num_columns(2)
        .spacing([14.0, 8.0])
        .show(ui, |ui| {
            step_badge(ui, label, tr(step.rule()), pal);
            ui.add(egui::Label::new(RichText::new(value).monospace()).wrap());
            ui.end_row();
        });
}

/// Working lines, with the number of lines left out.
fn explanation(ui: &mut Ui, lines: impl Iterator<Item = String>, total: usize) {
    ui.label(RichText::new(tr("mining.exercise.explanation")).font(theme::medium(12.5)));
    for line in lines.take(DETAIL_LIMIT) {
        ui.add(egui::Label::new(RichText::new(format!("• {line}")).monospace().weak()).wrap());
    }
    if total > DETAIL_LIMIT {
        ui.label(RichText::new(trf("mining.exercise.more", &[&(total - DETAIL_LIMIT)])).weak());
    }
    ui.add_space(8.0);
}
