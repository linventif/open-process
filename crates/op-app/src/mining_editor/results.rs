//! Results for the MiningEditor notation.

use super::exercise::{Solved, exercise_results};
use super::*;

impl MiningEditor {
    pub(super) fn results(&mut self, ui: &mut Ui) {
        let pal = Palette::for_ui(ui);
        let exam = crate::editor::exam::active(ui.ctx());
        let Some(Computed {
            result,
            soundness: sound,
            replays,
            report,
            ..
        }) = &self.computed
        else {
            ui.spinner();
            ui.label(tr("mining.analysis_pending"));
            return;
        };
        let (log, r) = match result {
            Ok(MiningOutput::Discovery(log, r)) if !exam => (log, r),
            Ok(MiningOutput::Exam(activities)) if exam => {
                ui.label(RichText::new(&self.name).font(theme::semibold(20.0)));
                exam_sections(ui, activities, &mut self.answers, &pal);
                return;
            }
            Ok(_) => return,
            Err(e) => {
                ui.add_space(40.0);
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new(tr("mining.paste_event_log_left")).font(theme::semibold(17.0)));
                    ui.add_space(4.0);
                    ui.label(RichText::new(e).color(ui.visuals().weak_text_color()));
                    ui.add_space(10.0);
                    if !exam {
                        ui.label(RichText::new(tr(FORMATS)).color(ui.visuals().weak_text_color()));
                    }
                });
                if ui.button(tr("petri.recompute")).clicked() {
                    self.computed = None;
                }
                return;
            }
        };
        if self.exercise.active {
            let solved = Solved {
                name: &self.name,
                log,
                r,
                replays,
                soundness: sound.as_ref(),
                report,
            };
            let requests = exercise_results(ui, &mut self.exercise, &solved, &mut self.preview);
            if let Some((text, msg)) = requests.copy {
                ui.ctx().copy_text(text);
                self.status = msg.into();
            }
            if requests.action.is_some() {
                self.action = requests.action;
            }
            return;
        }
        let f = &r.footprint;
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
                            "mining.traces_variants_activities",
                            &[&log.traces.len(), &(log.variants().len()), &f.len()],
                        ))
                        .color(ui.visuals().weak_text_color()),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    action = open_actions(ui, r, &self.name);
                    copy = copy_menu(ui, f, report);
                });
            });

            if !r.projected_activities.is_empty() {
                ui.label(RichText::new(tr("mining.report.projected_markers")).small().weak());
            }

            // ---- 1. traces
            card_section(ui, "1", tr("mining.traces"), tr("mining.distinct_words_log"), |ui| {
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

            // ---- 2. relations
            card_section(
                ui,
                "2",
                tr("mining.relations_between_activities"),
                tr("mining.identified_from_direct_successions"),
                |ui| relation_lists(ui, f, &pal),
            );

            // ---- 3. matrix
            card_section(
                ui,
                "3",
                tr("mining.relation_matrix"),
                tr("mining.row_x_column_y_relation_between_x"),
                |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .small_button(tr("mining.save_png"))
                            .on_hover_text(tr("mining.export_matrix_image"))
                            .clicked()
                        {
                            export = Some("png");
                        }
                        if ui
                            .small_button(tr("mining.save_csv"))
                            .on_hover_text(tr("mining.export_matrix_csv_excel_libreoffice"))
                            .clicked()
                        {
                            export = Some("csv");
                        }
                        if ui.small_button(tr("mining.copy_latex")).clicked() {
                            copy = Some((f.to_latex(), tr("mining.relation_matrix_copied_latex")));
                        }
                    });
                    ui.add_space(4.0);
                    egui::ScrollArea::horizontal().id_salt("matrix").show(ui, |ui| {
                        matrix_rect = relation_table(ui, f, &pal);
                    });
                    ui.add_space(6.0);
                    matrix_legend(ui, &pal);
                },
            );

            // ---- 4. alpha
            let pairs = |v: &[op_core::mining::Pair]| v.iter().map(|p| r.pair(p)).collect::<Vec<_>>().join(", ");
            let mut places: Vec<String> = r.y_w.iter().map(|p| r.place_name(p)).collect();
            places.extend(["iw".into(), "ow".into()]);
            let fw: Vec<String> = r.flow_relation().iter().map(|(a, b)| format!("({a},{b})")).collect();
            card_section(ui, "4", tr("mining.alpha_algorithm"), tr("mining.8_steps"), |ui| {
                egui::Grid::new("alpha").num_columns(2).spacing([14.0, 10.0]).show(ui, |ui| {
                    for (label, hint, value) in [
                        ("TW", tr("mining.all_activities"), r.set(&r.t_w)),
                        ("TI", tr("mining.initial_activities_first_trace"), r.set(&r.t_i)),
                        ("TO", tr("mining.final_activities_last_trace"), r.set(&r.t_o)),
                        ("XW", tr("mining.pairs_b_every_causes_every_b_b"), format!("{{{}}}", pairs(&r.x_w))),
                        ("YW", tr("mining.maximal_pairs_xw"), format!("{{{}}}", pairs(&r.y_w))),
                        (
                            "PW",
                            tr("mining.one_place_per_yw_pair_plus_iw"),
                            format!("{{{}}}", places.join(", ")),
                        ),
                        ("FW", tr("mining.arcs_net"), format!("{{{}}}", fw.join(", "))),
                        ("α(W)", tr("mining.discovered_workflow_net"), "(PW, TW, FW)".into()),
                    ] {
                        step_badge(ui, label, hint, &pal);
                        ui.add(egui::Label::new(RichText::new(value).monospace()).wrap());
                        ui.end_row();
                    }
                });
                if r.x_truncated {
                    ui.label(RichText::new(tr("mining.xw_was_too_large_has_been_truncated")).color(pal.warn));
                }
            });

            // ---- 5. net
            card_section(
                ui,
                "5",
                tr("mining.discovered_petri_net_alpha_w"),
                tr("mining.drag_pan_ctrl_wheel_zoom_double_click"),
                |ui| {
                    if let Some(p) = &mut self.preview {
                        let w = ui.available_width();
                        p.preview_ui(ui, egui::vec2(w, 400.0));
                    }
                },
            );

            // ---- 6. word check
            card_section(ui, "6", tr("mining.word_check"), tr("mining.replay_every_trace_alpha_w"), |ui| {
                word_check(ui, log, replays, sound.as_ref(), &pal);
                // Same commands as the header, next to the checked net, as in the exercise's last step.
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if let Some(requested) = open_actions(ui, r, &self.name) {
                        action = Some(requested);
                    }
                    if let Some(requested) = copy_menu(ui, f, report) {
                        copy = Some(requested);
                    }
                });
            });
            ui.add_space(24.0);
        });

        self.matrix_rect = matrix_rect;
        let stem: String = self.name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
        match export {
            Some("png") => crate::io::request_png_export(ui.ctx(), matrix_rect.expand(6.0), format!("{stem}-relation-matrix.png")),
            Some(_) => match crate::io::save_file(&format!("{stem}-relation-matrix.csv"), f.to_csv().as_bytes()) {
                Ok(Some(outcome)) => self.status = outcome.message(op_core::i18n::key("app.exported")),
                Ok(None) => {}
                Err(e) => self.status = trf("file.export_failed", &[&e]),
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

/// Succession, causality, parallelism and independence lists (step 2).
pub(super) fn relation_lists(ui: &mut Ui, f: &op_core::mining::Footprint, pal: &Palette) {
    let list = |pairs: &[(usize, usize)], sym: &str| -> Vec<String> {
        pairs
            .iter()
            .map(|&(a, b)| format!("{}{sym}{}", f.activities[a], f.activities[b]))
            .collect()
    };
    let par: Vec<(usize, usize)> = f
        .pairs(Relation::Parallel)
        .into_iter()
        .flat_map(|(a, b)| if a == b { vec![(a, b)] } else { vec![(a, b), (b, a)] })
        .collect();
    egui::Grid::new("relations").num_columns(2).spacing([16.0, 10.0]).show(ui, |ui| {
        for (label, hint, items, color) in [
            (
                tr("mining.succession_x_y"),
                tr("mining.x_directly_followed_y_at_least_one"),
                list(&f.successions(), ">"),
                pal.muted,
            ),
            (
                tr("mining.causality_x_y"),
                tr("mining.x_y_not_y_x"),
                list(&f.pairs(Relation::Causal), "→"),
                pal.ok,
            ),
            (tr("mining.parallelism_x_y"), tr("mining.x_y_y_x"), list(&par, "||"), pal.select),
            (
                tr("mining.independence_x_y"),
                tr("mining.not_x_y_not_y_x"),
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
}

pub(super) fn matrix_legend(ui: &mut Ui, pal: &Palette) {
    ui.horizontal_wrapped(|ui| {
        for (sym, txt, c) in [
            ("→", tr("mining.causality"), pal.ok),
            ("←", tr("mining.reverse_causality"), pal.end),
            ("||", tr("mining.parallel"), pal.select),
            ("#", tr("mining.independent"), pal.muted),
        ] {
            ui.label(RichText::new(sym).font(theme::semibold(14.0)).color(c));
            ui.label(RichText::new(txt).small().weak());
            ui.add_space(8.0);
        }
    });
}

/// Rounded label of an Alpha step, such as `TW`, explained on hover.
pub(super) fn step_badge(ui: &mut Ui, label: &str, hint: &str, pal: &Palette) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 22.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 6.0, pal.select.gamma_multiply(if pal.dark { 0.3 } else { 0.12 }));
    ui.painter()
        .text(rect.center(), egui::Align2::CENTER_CENTER, label, theme::semibold(12.5), pal.select);
    ui.interact(rect, ui.scope_id().with(label), egui::Sense::hover())
        .on_hover_text(hint);
}

/// Replay outcome of each variant on α(W), followed by the soundness verdict.
pub(super) fn word_check(ui: &mut Ui, log: &EventLog, replays: &[Result<(), String>], sound: Option<&Soundness>, pal: &Palette) {
    egui::Grid::new("replay").num_columns(2).spacing([14.0, 8.0]).show(ui, |ui| {
        for (index, (t, _)) in log.variants().into_iter().enumerate() {
            ui.label(RichText::new(log.format_trace(&t)).monospace());
            match replays.get(index) {
                Some(Ok(())) => chip(ui, tr("mining.answer.ok"), pal.ok),
                Some(Err(e)) => chip(ui, &format!("✖ {e}"), pal.bad),
                None => chip(ui, tr("mining.analysis_pending"), pal.warn),
            };
            ui.end_row();
        }
    });
    ui.add_space(8.0);
    match sound {
        Some(Soundness::Sound) => {
            chip(ui, tr("mining.sound_workflow"), pal.ok);
        }
        Some(Soundness::Unsound(why)) => {
            chip(ui, tr("mining.unsound_workflow"), pal.bad);
            for w in why {
                ui.label(RichText::new(format!("• {w}")).weak());
            }
        }
        Some(Soundness::Unknown) => {
            chip(ui, tr("mining.soundness_undecided_state_space_too_large"), pal.warn);
        }
        Some(Soundness::NotWorkflowNet) => {
            chip(ui, tr("mining.alpha_w_not_workflow_net"), pal.warn);
        }
        None => {}
    }
}

/// Primary buttons opening α(W) as a Petri net or its BPMN projection.
pub(super) fn open_actions(ui: &mut Ui, r: &AlphaResult, name: &str) -> Option<MiningAction> {
    let mut action = None;
    if ui
        .add(theme::primary_button(ui, tr("mining.deduce_bpmn")))
        .on_hover_text(tr("mining.deduce_bpmn_diagram_from_alpha_w"))
        .clicked()
    {
        let (mut d, _) = convert::petri_to_bpmn(&r.net);
        d.name = trf("mining.bpmn_name", &[&name]);
        action = Some(MiningAction::OpenBpmn(d));
    }
    if ui
        .add(theme::primary_button(ui, tr("mining.open_petri_net")))
        .on_hover_text(tr("mining.open_alpha_w_petri_net_tab_simulation"))
        .clicked()
    {
        let mut net = r.net.clone();
        net.name = trf("mining.net_name", &[&name]);
        action = Some(MiningAction::OpenPetri(net));
    }
    action
}

/// Copy menu: full write-up, then the relation matrix as LaTeX or text.
pub(super) fn copy_menu(ui: &mut Ui, f: &op_core::mining::Footprint, report: &str) -> Option<(String, &'static str)> {
    let mut copy = None;
    ui.menu_button(tr("mining.copy"), |ui| {
        theme::menu_style(ui.style_mut());
        if ui.button(tr("mining.full_write_up_text")).clicked() {
            copy = Some((report.to_owned(), tr("mining.write_up_copied")));
        }
        if ui.button(tr("mining.relation_matrix_latex")).clicked() {
            copy = Some((f.to_latex(), tr("mining.relation_matrix_copied_latex")));
        }
        if ui.button(tr("mining.relation_matrix_text")).clicked() {
            copy = Some((f.to_text(), tr("mining.relation_matrix_copied")));
        }
    });
    copy
}
