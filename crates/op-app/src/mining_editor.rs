//! Process discovery: event log → relation matrix → Alpha algorithm → Petri net → BPMN.

use crate::canvas::Palette;
use crate::petri_editor::PetriEditor;
use crate::theme;
use egui::{Color32, RichText, Ui};
use op_core::analysis::{Soundness, soundness};
use op_core::answers::{self as ans, RelationList, Step, Verdict};
use op_core::bpmn::BpmnDiagram;
use op_core::convert;
use op_core::i18n::{key, tr, trf};
use op_core::mining::{AlphaResult, EventLog, Relation, alpha, replay};
use op_core::petri::PetriNet;
use std::collections::BTreeMap;

pub enum MiningAction {
    OpenPetri(PetriNet),
    OpenBpmn(BpmnDiagram),
}

struct Computed {
    text: String,
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
    pub name: String,
    pub text: String,
    /// Step-by-step mode: the student fills every step by hand.
    pub manual: bool,
    pub answers: BTreeMap<String, String>,
    manual_ui: ManualUi,
    computed: Option<Computed>,
    preview: Option<PetriEditor>,
    pub action: Option<MiningAction>,
    matrix_rect: egui::Rect,
    pub status: String,
}

pub const EXAMPLE_1: &str = "case 1 : task A\ncase 2 : task A\ncase 3 : task A\ncase 3 : task B\ncase 1 : task B\n\
case 1 : task C\ncase 2 : task C\ncase 4 : task A\ncase 2 : task B\ncase 2 : task D\ncase 5 : task E\ncase 4 : task C\n\
case 1 : task D\ncase 3 : task C\ncase 3 : task D\ncase 4 : task B\ncase 5 : task F\ncase 4 : task D\n";

pub const EXAMPLE_2: &str = "ABCD\nACBD\nAED\n";

const FORMATS: &str = key(
    "Accepted formats:\n• Case 1, Task A   (one event per line, or several columns per line)\n• words of one-letter activities: ABCD  ACBD  EF\n• one trace per line: register -> check -> pay\n• CSV with a header (case, activity), or an XES file (ProM)\nLines starting with # are comments.",
);

impl MiningEditor {
    pub fn new(doc: op_core::document::LogDocument) -> Self {
        Self {
            name: doc.name,
            text: doc.text,
            manual: doc.manual,
            answers: doc.answers,
            manual_ui: ManualUi::default(),
            computed: None,
            preview: None,
            action: None,
            matrix_rect: egui::Rect::NOTHING,
            status: String::new(),
        }
    }

    fn recompute(&mut self) {
        if self.computed.as_ref().is_some_and(|c| c.text == self.text) {
            return;
        }
        let result = EventLog::parse(&self.text).and_then(|log| {
            if log.traces.is_empty() {
                return Err(tr("No trace found").to_string());
            }
            let r = alpha(&log);
            Ok((log, r))
        });
        self.preview = result.as_ref().ok().map(|(_, r)| PetriEditor::new(r.net.clone()));
        let sound = result.as_ref().ok().map(|(_, r)| soundness(&r.net, 20_000));
        self.computed = Some(Computed {
            text: self.text.clone(),
            result,
            soundness: sound,
        });
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        self.recompute();
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
    }

    fn input_panel(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Event log")).font(theme::semibold(16.0)));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.menu_button(tr("Examples ▾"), |ui| {
                    if ui.button(tr("ABCD, ACBD, EF  (case / task lines)")).clicked() {
                        self.text = EXAMPLE_1.into();
                    }
                    if ui.button("ABCD, ACBD, AED").clicked() {
                        self.text = EXAMPLE_2.into();
                    }
                });
            });
        });
        theme::section_title(ui, tr("Name"));
        let lbl = ui.label(RichText::new(tr("Name of the log")).small().weak());
        ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(f32::INFINITY))
            .labelled_by(lbl.id);
        theme::section_title(ui, tr("Traces"));
        let lbl = ui.label(RichText::new(tr("One event or trace per line")).small().weak());
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
        egui::CollapsingHeader::new(RichText::new(tr("Accepted formats")).small())
            .default_open(false)
            .show(ui, |ui| {
                ui.label(RichText::new(tr(FORMATS)).small().weak());
            });
    }

    fn results(&mut self, ui: &mut Ui) {
        let pal = Palette::for_ui(ui);
        let Some(Computed {
            result, soundness: sound, ..
        }) = &self.computed
        else {
            return;
        };
        let (log, r) = match result {
            Ok(x) => x,
            Err(e) => {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(tr("Paste an event log on the left")).font(theme::semibold(17.0)));
                    ui.add_space(4.0);
                    ui.label(RichText::new(e).color(ui.visuals().weak_text_color()));
                    ui.add_space(10.0);
                    ui.label(RichText::new(tr(FORMATS)).color(ui.visuals().weak_text_color()));
                });
                return;
            }
        };
        let f = &r.footprint;
        let n = |a: usize| f.activities[a].as_str();
        let mut action = None;
        let mut copy = None;
        let mut export: Option<&str> = None;
        let mut matrix_rect = self.matrix_rect;
        let max_w = 980.0f32.min(ui.available_width());

        ui.vertical_centered(|ui| {
            ui.set_max_width(max_w);
            ui.add_space(14.0);
            // ---- header
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(RichText::new(&self.name).font(theme::semibold(20.0)));
                    ui.label(
                        RichText::new(trf(
                            "{} traces · {} variants · {} activities",
                            &[&log.traces.len(), &(log.variants().len()), &f.len()],
                        ))
                        .color(ui.visuals().weak_text_color()),
                    );
                });
                ui.add_space(16.0);
                theme::segmented(
                    ui,
                    &mut self.manual,
                    &[(false, tr("Automatic")), (true, tr("Step by step (by hand)"))],
                );
                if self.manual
                    && ui
                        .button(tr("Check everything"))
                        .on_hover_text(tr("Check all your answers"))
                        .clicked()
                {
                    self.manual_ui.checked.insert("matrix".into());
                    for l in RelationList::ALL {
                        self.manual_ui.checked.insert(format!("rel:{}", l.key()));
                    }
                    for s in Step::ALL {
                        self.manual_ui.checked.insert(format!("step:{}", s.key()));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(theme::primary_button(ui, tr("Deduce BPMN")))
                        .on_hover_text(tr("Deduce the BPMN diagram from α(W)"))
                        .clicked()
                    {
                        let (mut d, _) = convert::petri_to_bpmn(&r.net);
                        d.name = trf("{} – BPMN", &[&self.name]);
                        action = Some(MiningAction::OpenBpmn(d));
                    }
                    if ui
                        .add(theme::primary_button(ui, tr("Open Petri net")))
                        .on_hover_text(tr("Open α(W) in a Petri net tab (simulation, analysis…)"))
                        .clicked()
                    {
                        let mut net = r.net.clone();
                        net.name = trf("{} – α(W)", &[&self.name]);
                        action = Some(MiningAction::OpenPetri(net));
                    }
                    ui.menu_button(tr("Copy ▾"), |ui| {
                        if ui.button(tr("Full write-up (text)")).clicked() {
                            copy = Some((r.report(log), tr("Write-up copied")));
                        }
                        if ui.button(tr("Relation matrix (LaTeX)")).clicked() {
                            copy = Some((f.to_latex(), tr("Relation matrix copied as LaTeX")));
                        }
                        if ui.button(tr("Relation matrix (text)")).clicked() {
                            copy = Some((f.to_text(), tr("Relation matrix copied")));
                        }
                    });
                });
            });

            // ---- 1. traces
            card_section(ui, "1", tr("Traces"), tr("distinct words of the log"), |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (t, k) in log.variants() {
                        let txt = if k > 1 {
                            format!("{}  ×{k}", log.format_trace(&t))
                        } else {
                            log.format_trace(&t)
                        };
                        chip(ui, &txt, ui.visuals().text_color());
                    }
                });
            });

            if self.manual {
                manual_sections(ui, log, r, &pal, &mut self.answers, &mut self.manual_ui, &mut action, &self.name);
                ui.add_space(24.0);
                return;
            }
            // ---- 2. relations
            let list = |pairs: &[(usize, usize)], sym: &str| -> Vec<String> {
                pairs.iter().map(|&(a, b)| format!("{}{sym}{}", n(a), n(b))).collect()
            };
            let par: Vec<(usize, usize)> = f
                .pairs(Relation::Parallel)
                .into_iter()
                .flat_map(|(a, b)| if a == b { vec![(a, b)] } else { vec![(a, b), (b, a)] })
                .collect();
            card_section(
                ui,
                "2",
                tr("Relations between activities"),
                tr("identified from the direct successions"),
                |ui| {
                    egui::Grid::new("relations").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
                        for (label, hint, items, color) in [
                            (
                                tr("Succession  x>y"),
                                tr("x is directly followed by y in at least one trace"),
                                list(&f.successions(), ">"),
                                pal.muted,
                            ),
                            (
                                tr("Causality  x→y"),
                                tr("x>y and not y>x"),
                                list(&f.pairs(Relation::Causal), "→"),
                                pal.ok,
                            ),
                            (tr("Parallelism  x||y"), tr("x>y and y>x"), list(&par, "||"), pal.select),
                            (
                                tr("Independence  x#y"),
                                tr("not x>y and not y>x"),
                                list(&f.pairs(Relation::Independent), "#"),
                                pal.muted,
                            ),
                        ] {
                            ui.label(RichText::new(label).font(theme::medium(13.0))).on_hover_text(hint);
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                                if items.is_empty() {
                                    ui.label(RichText::new("—").weak());
                                }
                                for it in items {
                                    chip(ui, &it, color);
                                }
                            });
                            ui.end_row();
                        }
                    });
                },
            );

            // ---- 3. matrix
            card_section(
                ui,
                "3",
                tr("Relation matrix"),
                tr("row x, column y: relation between x and y"),
                |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .small_button(tr("Save PNG…"))
                            .on_hover_text(tr("Export the matrix as an image"))
                            .clicked()
                        {
                            export = Some("png");
                        }
                        if ui
                            .small_button(tr("Save CSV…"))
                            .on_hover_text(tr("Export the matrix as CSV (Excel, LibreOffice…)"))
                            .clicked()
                        {
                            export = Some("csv");
                        }
                        if ui.small_button(tr("Copy LaTeX")).clicked() {
                            copy = Some((f.to_latex(), tr("Relation matrix copied as LaTeX")));
                        }
                    });
                    ui.add_space(4.0);
                    egui::ScrollArea::horizontal().id_salt("matrix").show(ui, |ui| {
                        matrix_rect = relation_table(ui, f, &pal);
                    });
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for (sym, txt, c) in [
                            ("→", tr("causality"), pal.ok),
                            ("←", tr("reverse causality"), pal.end),
                            ("||", tr("parallel"), pal.select),
                            ("#", tr("independent"), pal.muted),
                        ] {
                            ui.label(RichText::new(sym).font(theme::semibold(14.0)).color(c));
                            ui.label(RichText::new(txt).small().weak());
                            ui.add_space(8.0);
                        }
                    });
                },
            );

            // ---- 4. alpha
            let pairs = |v: &[op_core::mining::Pair]| v.iter().map(|p| r.pair(p)).collect::<Vec<_>>().join(", ");
            let mut places: Vec<String> = r.y_w.iter().map(|p| r.place_name(p)).collect();
            places.extend(["iw".into(), "ow".into()]);
            let fw: Vec<String> = r.flow_relation().iter().map(|(a, b)| format!("({a},{b})")).collect();
            card_section(ui, "4", tr("Alpha algorithm"), tr("the 8 steps"), |ui| {
                egui::Grid::new("alpha").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                    for (label, hint, value) in [
                        ("TW", tr("all activities"), r.set(&r.t_w)),
                        ("TI", tr("initial activities (first of a trace)"), r.set(&r.t_i)),
                        ("TO", tr("final activities (last of a trace)"), r.set(&r.t_o)),
                        (
                            "XW",
                            tr("pairs (A,B): every a∈A causes every b∈B; A and B internally independent (#)"),
                            format!("{{{}}}", pairs(&r.x_w)),
                        ),
                        ("YW", tr("maximal pairs of XW"), format!("{{{}}}", pairs(&r.y_w))),
                        (
                            "PW",
                            tr("one place per YW pair, plus iw and ow"),
                            format!("{{{}}}", places.join(", ")),
                        ),
                        ("FW", tr("arcs of the net"), format!("{{{}}}", fw.join(", "))),
                        ("α(W)", tr("the discovered workflow net"), "(PW, TW, FW)".into()),
                    ] {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 22.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 6.0, pal.select.gamma_multiply(if pal.dark { 0.3 } else { 0.12 }));
                        ui.painter()
                            .text(rect.center(), egui::Align2::CENTER_CENTER, label, theme::semibold(12.5), pal.select);
                        ui.interact(rect, ui.scope_id().with(label), egui::Sense::hover())
                            .on_hover_text(hint);
                        ui.add(egui::Label::new(RichText::new(value).monospace()).wrap());
                        ui.end_row();
                    }
                });
                if r.x_truncated {
                    ui.label(RichText::new(tr("XW was too large and has been truncated.")).color(pal.warn));
                }
            });

            // ---- 5. net
            card_section(
                ui,
                "5",
                tr("Discovered Petri net α(W)"),
                tr("drag to pan · Ctrl+wheel to zoom · double-click to fit"),
                |ui| {
                    if let Some(p) = &mut self.preview {
                        let w = ui.available_width();
                        p.preview_ui(ui, egui::vec2(w, 400.0));
                    }
                },
            );

            // ---- 6. word check
            card_section(ui, "6", tr("Word check"), tr("replay of every trace on α(W)"), |ui| {
                egui::Grid::new("replay").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                    for (t, _) in log.variants() {
                        ui.label(RichText::new(log.format_trace(&t)).monospace());
                        match replay(&r.net, &t) {
                            Ok(()) => chip(ui, tr("✔ ok"), pal.ok),
                            Err(e) => chip(ui, &format!("✖ {e}"), pal.bad),
                        };
                        ui.end_row();
                    }
                });
                ui.add_space(8.0);
                match sound {
                    Some(Soundness::Sound) => {
                        chip(ui, tr("✔ α(W) is a sound workflow net"), pal.ok);
                    }
                    Some(Soundness::Unsound(why)) => {
                        chip(ui, tr("✖ α(W) is not a sound workflow net"), pal.bad);
                        for w in why {
                            ui.label(RichText::new(format!("• {w}")).weak());
                        }
                    }
                    Some(Soundness::Unknown) => {
                        chip(ui, tr("? soundness undecided (state space too large)"), pal.warn);
                    }
                    Some(Soundness::NotWorkflowNet) => {
                        chip(ui, tr("α(W) is not a workflow net"), pal.warn);
                    }
                    None => {}
                }
            });
            ui.add_space(24.0);
        });

        self.matrix_rect = matrix_rect;
        let stem: String = self.name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
        match export {
            Some("png") => crate::io::request_png_export(ui.ctx(), matrix_rect.expand(6.0), format!("{stem}-relation-matrix.png")),
            Some(_) => match crate::io::save_file(&format!("{stem}-relation-matrix.csv"), f.to_csv().as_bytes()) {
                Ok(Some(p)) => self.status = trf("Exported {}", &[&(crate::io::file_name(&p))]),
                Ok(None) => {}
                Err(e) => self.status = trf("Export failed: {}", &[&e]),
            },
            None => {}
        }
        if let Some((text, msg)) = copy {
            ui.ctx().copy_text(text);
            self.status = msg.into();
        }
        if action.is_some() {
            self.action = action;
        }
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
            Relation::Causal => trf("{}→{}: {}>{} and not {}>{}", &[&x, &y, &x, &y, &y, &x]),
            Relation::Reverse => trf("{}←{}: {}>{} and not {}>{}", &[&x, &y, &y, &x, &x, &y]),
            Relation::Parallel => trf("{}||{}: {}>{} and {}>{}", &[&x, &y, &x, &y, &y, &x]),
            Relation::Independent => trf("{}#{}: neither {}>{} nor {}>{}", &[&x, &y, &x, &y, &y, &x]),
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
            chip(ui, tr("✔ correct"), pal.ok);
        }
        Verdict::Empty => {
            chip(ui, tr("nothing written yet"), pal.muted);
        }
        Verdict::Invalid(e) => {
            chip(ui, &trf("⚠ can't read: {}", &[&e]), pal.warn);
        }
        Verdict::Wrong { missing, extra } => {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                chip(ui, "✖", pal.bad);
                if !missing.is_empty() {
                    ui.label(RichText::new(trf("missing ({}):", &[&missing.len()])).color(pal.bad));
                    for m in missing {
                        chip(ui, m, pal.bad);
                    }
                }
                if !extra.is_empty() {
                    ui.label(RichText::new(trf("wrong / extra ({}):", &[&extra.len()])).color(pal.warn));
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
        if ui.button(tr("Check")).clicked() {
            st.checked.insert(key.to_string());
        }
        if ui.button(tr("Fill for me")).on_hover_text(tr("Write the model answer")).clicked() {
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
        tr("Relations between activities"),
        tr("e.g.  A>B, A>C   ·   A→B (or A->B)   ·   B||C   ·   A#D"),
        |ui| {
            for list in RelationList::ALL {
                let label = match list {
                    RelationList::Succession => tr("Direct succession  x>y"),
                    RelationList::Causality => tr("Causality  x→y"),
                    RelationList::Parallel => tr("Parallelism  x||y"),
                    RelationList::Independence => tr("Independence  x#y"),
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
        tr("Relation matrix"),
        tr("click a cell to cycle  →  ←  ||  #   (right-click goes back)"),
        |ui| {
            let mut check = st.checked.contains("matrix");
            ui.horizontal(|ui| {
                if ui.button(tr("Check")).clicked() {
                    st.checked.insert("matrix".into());
                    check = true;
                }
                if ui.button(tr("Fill for me")).clicked() {
                    for a in 0..f.len() {
                        for b in 0..f.len() {
                            answers.insert(format!("m:{}:{}", f.activities[a], f.activities[b]), f.cell(a, b).to_string());
                        }
                    }
                    st.checked.insert("matrix".into());
                    check = true;
                }
                if ui.button(tr("Clear")).clicked() {
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
                    chip(ui, tr("✔ the matrix is correct"), pal.ok);
                } else {
                    chip(ui, &trf("✖ {} wrong cell(s), in red (hover them for a hint)", &[&wrong]), pal.bad);
                }
                tally(Some(wrong == 0));
            }
        },
    );

    // ---- 4. algorithm steps
    card_section(
        ui,
        "4",
        tr("Alpha algorithm"),
        tr("notations: {A,B}   ({A},{B,E}) or (A,BE)   P(A,BE)   iw ow   (iw,A)"),
        |ui| {
            for step in Step::ALL {
                let (title, hint, multi) = match step {
                    Step::Tw => (tr("1. TW — all activities"), "{A,B,C,…}", false),
                    Step::Ti => (tr("2. TI — initial activities"), "{A}", false),
                    Step::To => (tr("3. TO — final activities"), "{D}", false),
                    Step::Xw => (tr("4. XW — causal relations between sets"), "({A},{B}), ({A},{B,E}), …", true),
                    Step::Yw => (tr("5. YW — maximal pairs of XW"), "({A},{B,E}), …", true),
                    Step::Pw => (tr("6. PW — places"), "P(A,BE), …, iw, ow", true),
                    Step::Fw => (tr("7. FW — arcs"), "(iw,A), (A,P(A,BE)), (P(A,BE),B), …", true),
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
            ui.label(
                RichText::new(tr("8. α(W) = (PW, TW, FW): your net is drawn below from your TI, TO and YW."))
                    .color(ui.visuals().weak_text_color()),
            );
        },
    );

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
        tr("Your Petri net"),
        tr("built from your TI, TO and YW (steps 6–8)"),
        |ui| match &mut st.preview {
            Some((_, Ok(p))) if !p.net.transitions.is_empty() && !yw_empty => {
                if ui.button(tr("Open my net in the Petri editor")).clicked() {
                    let mut net = p.net.clone();
                    net.name = trf("{} – my α(W)", &[&name]);
                    *action = Some(MiningAction::OpenPetri(net));
                }
                let w = ui.available_width();
                p.preview_ui(ui, egui::vec2(w, 360.0));
            }
            Some((_, Err(e))) if !yw_empty => {
                chip(ui, &format!("⚠ {e}"), pal.warn);
            }
            _ => {
                ui.label(RichText::new(tr("Fill TI, TO and YW (step 4) to see your net.")).color(ui.visuals().weak_text_color()));
            }
        },
    );

    // ---- 6. word check on the student's net
    card_section(ui, "6", tr("Word check"), tr("replay of every trace on your net"), |ui| {
        match &st.preview {
            Some((_, Ok(p))) if !p.net.transitions.is_empty() && !yw_empty => {
                egui::Grid::new("replay_manual").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
                    for (t, _) in log.variants() {
                        ui.label(RichText::new(log.format_trace(&t)).monospace());
                        match replay(&p.net, &t) {
                            Ok(()) => chip(ui, tr("✔ ok"), pal.ok),
                            Err(e) => chip(ui, &format!("✖ {e}"), pal.bad),
                        };
                        ui.end_row();
                    }
                });
            }
            _ => {
                ui.label(RichText::new(tr("Your net is not built yet.")).color(ui.visuals().weak_text_color()));
            }
        }
    });

    // ---- progress
    ui.add_space(12.0);
    theme::card(ui).show(ui, |ui| {
        ui.set_width(ui.available_width());
        ui.horizontal(|ui| {
            ui.label(RichText::new(tr("Progress")).font(theme::semibold(15.0)));
            if score.1 == 0 {
                ui.label(RichText::new(tr("check your answers to see your score")).color(ui.visuals().weak_text_color()));
            } else {
                let color = if score.0 == score.1 { pal.ok } else { pal.warn };
                chip(ui, &trf("{} / {} checked answers correct", &[&score.0, &score.1]), color);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr("Clear all my answers")).clicked() {
                    answers.clear();
                    st.checked.clear();
                }
                if ui.button(tr("Check all")).clicked() {
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
                        "{}>{} is {}, {}>{} is {} — so {} {} {}",
                        &[
                            &x,
                            &y,
                            &(if xy { tr("true") } else { tr("false") }),
                            &y,
                            &x,
                            &(if yx { tr("true") } else { tr("false") }),
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
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Grid, true, tr("Relation matrix (editable)")));
    if let Some(h) = hint {
        resp.on_hover_text_at_pointer(h);
    } else {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    wrong
}
