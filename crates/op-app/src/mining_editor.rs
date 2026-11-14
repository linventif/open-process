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
use op_core::answers::{self as ans, RelationList, Step, Verdict};
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
    result: Result<(EventLog, AlphaResult), String>,
    soundness: Option<Soundness>,
}

/// UI state of the step-by-step mode (not saved).
#[derive(Default)]
struct ManualUi {
    /// Answers whose verdict is displayed.
    checked: std::collections::HashSet<String>,
    /// Net built from the student's answers, keyed by the answers it was built from.
    preview: Option<(String, Result<PetriEditor, String>)>,
}

pub struct MiningEditor {
    history: crate::history::History<op_core::document::LogDocument>,
    pub name: String,
    pub text: String,
    /// Step-by-step mode: the student fills every step by hand.
    pub manual: bool,
    pub answers: BTreeMap<String, String>,
    manual_ui: ManualUi,
    computed: Option<Computed>,
    pending: Option<(String, op_core::i18n::Lang, crate::platform::jobs::Job)>,
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
            manual: doc.manual,
            answers: doc.answers,
            manual_ui: ManualUi::default(),
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
        if self.computed.as_ref().is_some_and(|c| c.text == self.text && c.lang == language) {
            return;
        }
        if self
            .pending
            .as_ref()
            .is_none_or(|(text, lang, _)| *text != self.text || *lang != language)
        {
            self.pending = None;
            self.computed = None;
            self.preview = None;
            let input = serde_json::to_string(&analysis_job::Input {
                task: "mining.discovery".into(),
                text: self.text.clone(),
                lang: language,
            })
            .unwrap();
            match crate::platform::jobs::Job::spawn(input, analysis_job::compute_json, ui.ctx()) {
                Ok(job) => self.pending = Some((self.text.clone(), language, job)),
                Err(error) => {
                    self.computed = Some(Computed {
                        text: self.text.clone(),
                        lang: language,
                        result: Err(error),
                        soundness: None,
                    })
                }
            }
        }
        if let Some(result) = self.pending.as_mut().and_then(|(_, _, job)| job.poll()) {
            self.pending = None;
            self.computed = Some(
                result
                    .and_then(|raw| serde_json::from_str::<Computed>(&raw).map_err(|e| e.to_string()))
                    .unwrap_or_else(|error| Computed {
                        text: self.text.clone(),
                        lang: language,
                        result: Err(error),
                        soundness: None,
                    }),
            );
            self.preview = self
                .computed
                .as_ref()
                .and_then(|c| c.result.as_ref().ok())
                .map(|(_, r)| PetriEditor::new(r.net.clone()));
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
            manual: self.manual,
            answers: self.answers.clone(),
        }
    }
    fn restore_document(&mut self, doc: op_core::document::LogDocument) {
        self.name = doc.name;
        self.text = doc.text;
        self.manual = doc.manual;
        self.answers = doc.answers;
        self.computed = None;
        self.pending = None;
        self.preview = None;
        self.manual_ui = ManualUi::default();
    }

    fn input_panel(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("mining.event_log")).font(theme::semibold(16.0)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button(tr("mining.examples"), |ui| {
                    if ui.button(tr("mining.abcd_acbd_ef_case_task_lines")).clicked() {
                        self.text = EXAMPLE_1.into();
                    }
                    if ui.button("ABCD, ACBD, AED").clicked() {
                        self.text = EXAMPLE_2.into();
                    }
                });
            });
        });
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
                    .hint_text("ABCD\nACBD\nEF"),
            )
            .labelled_by(lbl.id);
        });
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
// Step-by-step mode

fn verdict_ui(ui: &mut Ui, v: &Verdict, pal: &Palette) {
    match v {
        Verdict::Correct => {
            chip(ui, tr("mining.answer.correct"), pal.ok);
        }
        Verdict::Empty => {
            chip(ui, tr("mining.nothing_written_yet"), pal.muted);
        }
        Verdict::Invalid(e) => {
            chip(ui, &trf("mining.can_t_read", &[&e]), pal.warn);
        }
        Verdict::Wrong { missing, extra } => {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                chip(ui, "✖", pal.bad);
                if !missing.is_empty() {
                    ui.label(RichText::new(trf("mining.missing", &[&missing.len()])).color(pal.bad));
                    for m in missing {
                        chip(ui, m, pal.bad);
                    }
                }
                if !extra.is_empty() {
                    ui.label(RichText::new(trf("mining.wrong_extra", &[&extra.len()])).color(pal.warn));
                    for e in extra {
                        chip(ui, e, pal.warn);
                    }
                }
            });
        }
    }
}

/// Text answer + Check / Fill for me buttons. Returns whether it is correct once checked.
#[allow(clippy::too_many_arguments)]
fn answer_row(
    ui: &mut Ui,
    key: &str,
    hint: &str,
    multiline: bool,
    answers: &mut BTreeMap<String, String>,
    st: &mut ManualUi,
    solution: impl FnOnce() -> String,
    check: impl FnOnce(&str) -> Verdict,
    pal: &Palette,
) -> Option<bool> {
    let text = answers.entry(key.to_string()).or_default();
    let w = ui.available_width() - 190.0;
    ui.horizontal(|ui| {
        let te = if multiline {
            egui::TextEdit::multiline(text).desired_rows(2)
        } else {
            egui::TextEdit::singleline(text)
        };
        let r = ui.add(te.desired_width(w.max(160.0)).font(egui::FontId::monospace(13.0)).hint_text(hint));
        r.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, key));
        if ui.button(tr("common.check")).clicked() {
            st.checked.insert(key.to_string());
        }
        if ui
            .button(tr("mining.fill_me"))
            .on_hover_text(tr("mining.write_model_answer"))
            .clicked()
        {
            *text = solution();
            st.checked.insert(key.to_string());
        }
    });
    if st.checked.contains(key) {
        let v = check(text);
        verdict_ui(ui, &v, pal);
        Some(v.is_correct())
    } else {
        None
    }
}

const CYCLE: [&str; 5] = ["", "→", "←", "||", "#"];

#[allow(clippy::too_many_arguments)]
fn manual_sections(
    ui: &mut Ui,
    log: &EventLog,
    r: &AlphaResult,
    pal: &Palette,
    answers: &mut BTreeMap<String, String>,
    st: &mut ManualUi,
    action: &mut Option<MiningAction>,
    name: &str,
) {
    let f = &r.footprint;
    let mut score = (0usize, 0usize);
    let mut tally = |v: Option<bool>| {
        if let Some(ok) = v {
            score.1 += 1;
            score.0 += usize::from(ok);
        }
    };

    // ---- 2. relations
    card_section(
        ui,
        "2",
        tr("mining.relations_between_activities"),
        tr("mining.e_g_b_c_b_b_b"),
        |ui| {
            for list in RelationList::ALL {
                let label = match list {
                    RelationList::Succession => tr("mining.direct_succession_x_y"),
                    RelationList::Causality => tr("mining.causality_x_y"),
                    RelationList::Parallel => tr("mining.parallelism_x_y"),
                    RelationList::Independence => tr("mining.independence_x_y"),
                };
                ui.label(RichText::new(label).font(theme::medium(13.0)));
                let key = format!("rel:{}", list.key());
                let hint = format!("e.g. A{0}B, A{0}C", list.symbol());
                tally(answer_row(
                    ui,
                    &key,
                    &hint,
                    false,
                    answers,
                    st,
                    || ans::relations_solution(r, list),
                    |t| ans::check_relations(r, list, t),
                    pal,
                ));
                ui.add_space(6.0);
            }
        },
    );

    // ---- 3. matrix
    card_section(
        ui,
        "3",
        tr("mining.relation_matrix"),
        tr("mining.click_cell_cycle_right_click_goes_back"),
        |ui| {
            let mut check = st.checked.contains("matrix");
            ui.horizontal(|ui| {
                if ui.button(tr("common.check")).clicked() {
                    st.checked.insert("matrix".into());
                    check = true;
                }
                if ui.button(tr("mining.fill_me")).clicked() {
                    for a in 0..f.len() {
                        for b in 0..f.len() {
                            answers.insert(format!("m:{}:{}", f.activities[a], f.activities[b]), f.cell(a, b).to_string());
                        }
                    }
                    st.checked.insert("matrix".into());
                    check = true;
                }
                if ui.button(tr("common.clear")).clicked() {
                    answers.retain(|k, _| !k.starts_with("m:"));
                    st.checked.remove("matrix");
                    check = false;
                }
            });
            ui.add_space(4.0);
            let wrong = editable_relation_table(ui, f, answers, check, pal);
            if check {
                ui.add_space(6.0);
                if wrong == 0 {
                    chip(ui, tr("mining.matrix_correct"), pal.ok);
                } else {
                    chip(ui, &trf("mining.wrong_cell_s_red_hover_them_hint", &[&wrong]), pal.bad);
                }
                tally(Some(wrong == 0));
            }
        },
    );

    // ---- 4. algorithm steps
    card_section(ui, "4", tr("mining.alpha_algorithm"), tr("mining.notations_b_b_e_p_iw_ow"), |ui| {
        for step in Step::ALL {
            let (title, hint, multi) = match step {
                Step::Tw => (tr("mining.1_tw_all_activities"), "{A,B,C,…}", false),
                Step::Ti => (tr("mining.2_ti_initial_activities"), "{A}", false),
                Step::To => (tr("mining.3_final_activities"), "{D}", false),
                Step::Xw => (tr("mining.4_xw_causal_relations_between_sets"), "({A},{B}), ({A},{B,E}), …", true),
                Step::Yw => (tr("mining.5_yw_maximal_pairs_xw"), "({A},{B,E}), …", true),
                Step::Pw => (tr("mining.6_pw_places"), "P(A,BE), …, iw, ow", true),
                Step::Fw => (tr("mining.7_fw_arcs"), "(iw,A), (A,P(A,BE)), (P(A,BE),B), …", true),
            };
            ui.label(RichText::new(title).font(theme::medium(13.0)));
            tally(answer_row(
                ui,
                &format!("step:{}", step.key()),
                hint,
                multi,
                answers,
                st,
                || ans::solution(r, step),
                |t| ans::check(r, step, t),
                pal,
            ));
            ui.add_space(6.0);
        }
        ui.label(RichText::new(tr("mining.8_alpha_w_pw_tw_fw_net")).color(ui.visuals().weak_text_color()));
    });

    // ---- 5. the student's net
    let get = |k: &str| answers.get(k).cloned().unwrap_or_default();
    let key = format!("{}|{}|{}", get("step:TI"), get("step:TO"), get("step:YW"));
    if st.preview.as_ref().is_none_or(|(k, _)| *k != key) {
        let net = ans::student_net(r, &get("step:TI"), &get("step:TO"), &get("step:YW")).map(PetriEditor::new);
        st.preview = Some((key, net));
    }
    let yw_empty = get("step:YW").trim().is_empty();
    card_section(
        ui,
        "5",
        tr("mining.petri_net"),
        tr("mining.built_from_ti_yw_steps_6_8"),
        |ui| match &mut st.preview {
            Some((_, Ok(p))) if !p.net.transitions.is_empty() && !yw_empty => {
                if ui.button(tr("mining.open_my_net_petri_editor")).clicked() {
                    let mut net = p.net.clone();
                    net.name = trf("mining.custom_net_name", &[&name]);
                    *action = Some(MiningAction::OpenPetri(net));
                }
                let w = ui.available_width();
                p.preview_ui(ui, egui::vec2(w, 360.0));
            }
            Some((_, Err(e))) if !yw_empty => {
                chip(ui, &format!("⚠ {e}"), pal.warn);
            }
            _ => {
                ui.label(RichText::new(tr("mining.fill_ti_yw_step_4_see_net")).color(ui.visuals().weak_text_color()));
            }
        },
    );

    // ---- 6. word check on the student's net
    card_section(
        ui,
        "6",
        tr("mining.word_check"),
        tr("mining.replay_every_trace_net"),
        |ui| match &st.preview {
            Some((_, Ok(p))) if !p.net.transitions.is_empty() && !yw_empty => {
                egui::Grid::new("replay_manual").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                    for (t, _) in log.variants() {
                        ui.label(RichText::new(log.format_trace(&t)).monospace());
                        match replay(&p.net, &t) {
                            Ok(()) => chip(ui, tr("mining.answer.ok"), pal.ok),
                            Err(e) => chip(ui, &format!("✖ {e}"), pal.bad),
                        };
                        ui.end_row();
                    }
                });
            }
            _ => {
                ui.label(RichText::new(tr("mining.net_not_built_yet")).color(ui.visuals().weak_text_color()));
            }
        },
    );

    // ---- progress
    ui.add_space(12.0);
    theme::card(ui).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("mining.progress")).font(theme::semibold(15.0)));
            if score.1 == 0 {
                ui.label(RichText::new(tr("mining.check_answers_see_score")).color(ui.visuals().weak_text_color()));
            } else {
                let color = if score.0 == score.1 { pal.ok } else { pal.warn };
                chip(ui, &trf("mining.checked_answers_correct", &[&score.0, &score.1]), color);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr("mining.clear_all_my_answers")).clicked() {
                    answers.clear();
                    st.checked.clear();
                }
                if ui.button(tr("mining.check_all")).clicked() {
                    st.checked.insert("matrix".into());
                    for l in RelationList::ALL {
                        st.checked.insert(format!("rel:{}", l.key()));
                    }
                    for s in Step::ALL {
                        st.checked.insert(format!("step:{}", s.key()));
                    }
                }
            });
        });
    });
}

/// Relation matrix filled by clicking. Returns the number of wrong cells.
fn editable_relation_table(
    ui: &mut Ui,
    f: &op_core::mining::Footprint,
    answers: &mut BTreeMap<String, String>,
    check: bool,
    pal: &Palette,
) -> usize {
    let n = f.len();
    let cell = egui::vec2(44.0, 36.0);
    let size = egui::vec2(cell.x * (n + 1) as f32, cell.y * (n + 1) as f32);
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let at = |i: usize, j: usize| egui::Rect::from_min_size(rect.min + egui::vec2(cell.x * j as f32, cell.y * i as f32), cell);
    let hover = resp.hover_pos().map(|h| {
        let k = h - rect.min;
        ((k.y / cell.y) as usize, (k.x / cell.x) as usize)
    });
    let key = |a: usize, b: usize| format!("m:{}:{}", f.activities[a], f.activities[b]);
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
    let mut wrong = 0;
    let mut hint = None;
    for a in 0..n {
        for b in 0..n {
            let v = answers.get(&key(a, b)).map(String::as_str).unwrap_or("");
            let expected = f.cell(a, b);
            let ok = v == expected || (a == b && expected.is_empty() && v == "#");
            let r = at(a + 1, b + 1);
            if check {
                if !ok {
                    wrong += 1;
                }
                p.rect_filled(
                    r.shrink(1.0),
                    0.0,
                    if ok {
                        pal.ok.gamma_multiply(0.10)
                    } else {
                        pal.bad.gamma_multiply(0.18)
                    },
                );
            }
            if hover == Some((a + 1, b + 1)) {
                p.rect_filled(r.shrink(1.0), 0.0, pal.select.gamma_multiply(0.15));
                if check && !ok {
                    let (x, y) = (&f.activities[a], &f.activities[b]);
                    let (xy, yx) = (f.succ[a][b], f.succ[b][a]);
                    hint = Some(trf(
                        "mining.relation.explanation",
                        &[
                            &x,
                            &y,
                            &(if xy { tr("common.true") } else { tr("common.false") }),
                            &y,
                            &x,
                            &(if yx { tr("common.true") } else { tr("common.false") }),
                            &x,
                            &(if expected.is_empty() { "#" } else { expected }),
                            &y,
                        ],
                    ));
                }
            }
            let color = match v {
                "→" => pal.ok,
                "←" => pal.end,
                "||" => pal.select,
                _ => pal.muted,
            };
            p.text(r.center(), egui::Align2::CENTER_CENTER, v, theme::semibold(18.0), color);
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
    if let Some(h) = hint {
        resp.on_hover_text_at_pointer(h);
    } else {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    wrong
}
