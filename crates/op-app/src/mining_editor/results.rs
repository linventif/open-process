//! Results for the MiningEditor notation.

use super::*;

impl MiningEditor {
    pub(super) fn results(&mut self, ui: &mut Ui) {
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
