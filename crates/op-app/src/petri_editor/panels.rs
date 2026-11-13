//! Panels for the PetriEditor notation.

use super::*;

impl PetriEditor {
    pub(super) fn side_panel(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        DiagramLayout::inspector_tabs(
            ui,
            &mut self.tab,
            &[
                (SideTab::Properties, tr("common.properties")),
                (SideTab::Matrices, tr("view.matrices")),
                (SideTab::Analysis, tr("view.analysis")),
                (SideTab::States, tr("view.states")),
            ],
            |ui, tab| {
                let view = match tab {
                    SideTab::Properties => None,
                    SideTab::Matrices => Some(crate::dock::View::Matrices),
                    SideTab::Analysis => Some(crate::dock::View::Analysis),
                    SideTab::States => Some(crate::dock::View::States),
                };
                if let Some(v) = view
                    && ui
                        .small_button("↗")
                        .on_hover_text(tr("petri.open_view_own_pane_side_side"))
                        .clicked()
                {
                    self.request_pane = Some(v);
                }
            },
        );
        DiagramLayout::inspector_body(ui, |ui| match self.tab {
            SideTab::Properties => {
                if self.sim.is_some() {
                    self.simulation_panel(ui);
                } else {
                    self.properties(ui);
                }
            }
            SideTab::Matrices => self.matrices(ui),
            SideTab::Analysis => self.analysis_panel(ui),
            SideTab::States => self.states_panel(ui),
        });
    }

    pub(super) fn properties(&mut self, ui: &mut Ui) {
        if self.group.len() > 1 {
            self.group_panel(ui);
            return;
        }
        let editable = self.sim.is_none();
        ui.add_enabled_ui(editable, |ui| match self.selection {
            Some(Sel::Node(NodeRef::Place(id))) => {
                let Some(p) = self.net.place_mut(id) else { return };
                ui.heading(tr("petri.place"));
                egui::Grid::new("pp").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("common.name"));
                    ui.text_edit_singleline(&mut p.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("petri.tokens_m0"));
                    ui.add(egui::DragValue::new(&mut p.tokens).range(0..=100_000));
                    ui.end_row();
                    ui.label(tr("petri.capacity"));
                    ui.horizontal(|ui| {
                        let mut has = p.capacity.is_some();
                        if ui.checkbox(&mut has, "").changed() {
                            p.capacity = if has { Some(p.tokens.max(1)) } else { None };
                        }
                        if let Some(c) = &mut p.capacity {
                            ui.add(egui::DragValue::new(c).range(1..=100_000));
                        } else {
                            ui.label(RichText::new("∞").weak());
                        }
                    });
                    ui.end_row();
                });
            }
            Some(Sel::Node(NodeRef::Transition(id))) => {
                let Some(t) = self.net.transition_mut(id) else { return };
                ui.heading(tr("petri.transition"));
                egui::Grid::new("tp").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("common.name"));
                    ui.text_edit_singleline(&mut t.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("petri.silent_tau"));
                    ui.checkbox(&mut t.silent, "");
                    ui.end_row();
                });
                let n = NodeRef::Transition(id);
                let pre: Vec<String> = self
                    .net
                    .arcs
                    .iter()
                    .filter(|a| a.to == n)
                    .map(|a| weighted(self.net.node_name(a.from).unwrap_or("?"), a.weight))
                    .collect();
                let post: Vec<String> = self
                    .net
                    .arcs
                    .iter()
                    .filter(|a| a.from == n)
                    .map(|a| weighted(self.net.node_name(a.to).unwrap_or("?"), a.weight))
                    .collect();
                ui.add_space(6.0);
                ui.label(format!("•t = {{{}}}", pre.join(", ")));
                ui.label(format!("t• = {{{}}}", post.join(", ")));
            }
            Some(Sel::Arc(id)) => {
                let (from, to) = match self.net.arc(id) {
                    Some(a) => (
                        self.net.node_name(a.from).unwrap_or("?").to_string(),
                        self.net.node_name(a.to).unwrap_or("?").to_string(),
                    ),
                    None => return,
                };
                let Some(a) = self.net.arc_mut(id) else { return };
                ui.heading(tr("petri.arc"));
                ui.label(format!("{from} → {to}"));
                egui::Grid::new("ap").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("petri.weight"));
                    ui.add(egui::DragValue::new(&mut a.weight).range(1..=100_000));
                    ui.end_row();
                    if matches!(a.from, NodeRef::Place(_)) {
                        ui.label(tr("petri.kind"));
                        egui::ComboBox::from_id_salt("arck")
                            .selected_text(if a.kind == ArcKind::Normal {
                                tr("petri.normal")
                            } else {
                                tr("petri.inhibitor")
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut a.kind, ArcKind::Normal, tr("petri.normal"));
                                ui.selectable_value(&mut a.kind, ArcKind::Inhibitor, tr("petri.inhibitor"));
                            });
                        ui.end_row();
                    }
                });
            }
            None => {
                ui.heading(tr("petri.petri_net"));
                ui.horizontal(|ui| {
                    let lbl = ui.label(tr("common.name"));
                    ui.text_edit_singleline(&mut self.net.name).labelled_by(lbl.id);
                });
                ui.add_space(8.0);
                crate::theme::section_title(ui, tr("bpmn.tips"));
                ui.label(tr("petri.p_t_add_places_transitions_clicking"));
                ui.label(tr("petri.drag_from_node_another_add_arc_click"));
                ui.label(tr("petri.k_left_click_place_add_token_right"));
                ui.label(tr("petri.double_click_node_rename_right_click_more"));
                ui.label(tr("petri.s_start_token_game_click_green_transitions"));
            }
        });
    }

    pub(super) fn group_panel(&mut self, ui: &mut Ui) {
        let places = self.group.iter().filter(|n| matches!(n, NodeRef::Place(_))).count();
        ui.label(RichText::new(trf("uml.elements_selected", &[&self.group.len()])).font(crate::theme::semibold(16.0)));
        ui.label(
            RichText::new(trf("petri.place_s_transition_s", &[&places, &(self.group.len() - places)]))
                .color(ui.visuals().weak_text_color()),
        );
        crate::theme::section_title(ui, tr("bpmn.arrange"));
        let pos: Vec<(NodeRef, Pos)> = self.group.iter().filter_map(|&n| self.net.node_pos(n).map(|p| (n, p))).collect();
        let k = pos.len().max(1) as f32;
        let (mx, my) = (
            pos.iter().map(|p| p.1.x).sum::<f32>() / k,
            pos.iter().map(|p| p.1.y).sum::<f32>() / k,
        );
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr("canvas.align_horizontal"))
                .on_hover_text(tr("petri.same_vertical_position"))
                .clicked()
            {
                for (n, p) in &pos {
                    self.net.set_node_pos(*n, self.snap(Pos::new(p.x, my)));
                }
            }
            if ui
                .button(tr("canvas.align_vertical"))
                .on_hover_text(tr("petri.same_horizontal_position"))
                .clicked()
            {
                for (n, p) in &pos {
                    self.net.set_node_pos(*n, self.snap(Pos::new(mx, p.y)));
                }
            }
            if ui.button(tr("canvas.distribute_horizontal")).clicked() && pos.len() > 2 {
                let mut v = pos.clone();
                v.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
                let (x0, x1) = (v[0].1.x, v[v.len() - 1].1.x);
                for (i, (n, p)) in v.iter().enumerate() {
                    let x = x0 + (x1 - x0) * i as f32 / (v.len() - 1) as f32;
                    self.net.set_node_pos(*n, Pos::new(x, p.y));
                }
            }
            if ui.button(tr("canvas.distribute_vertical")).clicked() && pos.len() > 2 {
                let mut v = pos.clone();
                v.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
                let (y0, y1) = (v[0].1.y, v[v.len() - 1].1.y);
                for (i, (n, p)) in v.iter().enumerate() {
                    let y = y0 + (y1 - y0) * i as f32 / (v.len() - 1) as f32;
                    self.net.set_node_pos(*n, Pos::new(p.x, y));
                }
            }
        });
        crate::theme::section_title(ui, tr("petri.tokens"));
        ui.horizontal(|ui| {
            if ui.button(tr("petri.1_token")).clicked() {
                for n in self.group.clone() {
                    if let NodeRef::Place(id) = n
                        && let Some(p) = self.net.place_mut(id)
                    {
                        p.tokens += 1;
                    }
                }
            }
            if ui.button(tr("petri.clear_tokens")).clicked() {
                for n in self.group.clone() {
                    if let NodeRef::Place(id) = n
                        && let Some(p) = self.net.place_mut(id)
                    {
                        p.tokens = 0;
                    }
                }
            }
        });
        ui.add_space(10.0);
        if ui
            .button(RichText::new(tr("common.delete_selection")).color(Palette::for_ui(ui).bad))
            .clicked()
        {
            self.delete_selection();
        }
    }

    pub(super) fn simulation_panel(&mut self, ui: &mut Ui) {
        let pal = Palette::for_ui(ui);
        ui.label(RichText::new(tr("petri.token_game")).font(crate::theme::semibold(16.0)));
        ui.add_space(4.0);
        let marking = self.current_marking();
        let enabled = self.net.enabled_transitions(&marking);
        ui.horizontal(|ui| {
            let can_back = self.sim.as_ref().is_some_and(|s| !s.stack.is_empty());
            if ui
                .add_enabled(can_back, egui::Button::new(tr("petri.back")))
                .on_hover_text(tr("petri.undo_last_firing"))
                .clicked()
                && let Some(s) = &mut self.sim
                && let Some(m) = s.stack.pop()
            {
                s.marking = m;
                s.trace.pop();
                s.anim = None;
            }
            if ui
                .add_enabled(!enabled.is_empty(), egui::Button::new(tr("petri.step")))
                .on_hover_text(tr("petri.fire_random_enabled_transition_space"))
                .clicked()
            {
                self.random_step();
            }
            if let Some(s) = &mut self.sim {
                let label = if s.auto { tr("petri.pause") } else { tr("petri.play") };
                if ui
                    .add(crate::theme::primary_button(ui, label))
                    .on_hover_text(tr("petri.fire_random_transitions_automatically"))
                    .clicked()
                {
                    s.auto = !s.auto;
                }
                if ui.button("⟲").on_hover_text(tr("petri.back_m0")).clicked() {
                    s.marking = self.net.initial_marking();
                    s.trace.clear();
                    s.stack.clear();
                    s.anim = None;
                }
            }
        });
        if let Some(s) = &mut self.sim {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("petri.speed")).color(ui.visuals().weak_text_color()));
                ui.add(egui::Slider::new(&mut s.interval, 0.05..=2.0).suffix(" s").logarithmic(true));
            });
        }

        crate::theme::section_title(ui, tr("petri.current_marking"));
        ui.label(RichText::new(format!("M = {}", fmt_u32(&marking))).font(FontId::monospace(13.5)));
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
            for (i, p) in self.net.places.iter().enumerate() {
                if marking[i] > 0 {
                    pill(ui, &format!("{} · {}", p.name, marking[i]), pal.text, pal.muted);
                }
            }
        });

        crate::theme::section_title(ui, tr("petri.enabled_transitions"));
        if enabled.is_empty() {
            pill(ui, tr("petri.dead_marking_nothing_can_fire"), pal.bad, pal.bad);
        } else {
            let mut fire = None;
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(4.0, 4.0);
                for &t in &enabled {
                    let b = egui::Button::new(
                        RichText::new(&self.net.transitions[t].name)
                            .color(pal.ok)
                            .font(crate::theme::medium(13.0)),
                    )
                    .fill(pal.ok.gamma_multiply(if pal.dark { 0.2 } else { 0.1 }))
                    .corner_radius(6.0);
                    if ui.add(b).on_hover_text(tr("petri.fire")).clicked() {
                        fire = Some(t);
                    }
                }
            });
            if let Some(t) = fire {
                self.fire(t);
            }
        }

        if let Some(s) = &self.sim {
            crate::theme::section_title(ui, &trf("petri.firing_sequence_sigma", &[&s.trace.len()]));
            if s.trace.is_empty() {
                ui.label(RichText::new(tr("petri.no_firing_yet")).color(ui.visuals().weak_text_color()));
            } else {
                let trace: Vec<&str> = s.trace.iter().map(|&t| self.net.transitions[t].name.as_str()).collect();
                ui.add(egui::Label::new(RichText::new(trace.join(" → ")).font(FontId::monospace(13.0))).wrap());
            }
        }
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui
                .button(tr("petri.use_m0"))
                .on_hover_text(tr("petri.make_current_marking_initial_marking"))
                .clicked()
                && let Some(s) = &self.sim
            {
                let m = s.marking.clone();
                self.net.set_initial_marking(&m);
            }
            if ui.button(tr("petri.stop")).clicked() {
                self.sim = None;
            }
        });
    }

    pub(super) fn matrices(&mut self, ui: &mut Ui) {
        let places: Vec<String> = self.net.places.iter().map(|p| p.name.clone()).collect();
        let trans: Vec<String> = self.net.transitions.iter().map(|t| t.name.clone()).collect();
        if places.is_empty() || trans.is_empty() {
            ui.label(tr("petri.add_places_transitions_see_matrices"));
            return;
        }
        let to_i = |m: Vec<Vec<u32>>| {
            m.into_iter()
                .map(|r| r.into_iter().map(i64::from).collect())
                .collect::<Vec<Vec<i64>>>()
        };
        let solutions: [Vec<Vec<i64>>; 3] = [
            to_i(self.net.pre_matrix()),
            to_i(self.net.post_matrix()),
            self.net.incidence_matrix(),
        ];
        let titles = [
            tr("petri.pre_w_arcs_place_transition"),
            tr("petri.post_w_arcs_transition_place"),
            tr("petri.incidence_c_post_pre"),
        ];
        let short = ["pre", "post", "incidence"];
        let s_of = |m: &Vec<Vec<i64>>| {
            m.iter()
                .map(|r| r.iter().map(|x| x.to_string()).collect())
                .collect::<Vec<Vec<String>>>()
        };

        let mut exercise = self.exercise.is_some();
        ui.horizontal_wrapped(|ui| {
            crate::theme::segmented(
                ui,
                &mut exercise,
                &[(false, tr("common.show")), (true, tr("petri.exercise_fill_hand"))],
            );
        });
        if exercise != self.exercise.is_some() {
            self.exercise = exercise.then(|| [0, 1, 2].map(|_| vec![vec![String::new(); trans.len()]; places.len()]));
            self.exercise_checked = false;
        }
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr("mining.copy_latex"))
                .on_hover_text(tr("petri.pre_post_c_m0_latex_pmatrix"))
                .clicked()
            {
                let mut s = String::new();
                for (k, name) in ["Pre", "Post", "C"].iter().enumerate() {
                    s += &latex(name, &s_of(&solutions[k]));
                }
                let m0: Vec<Vec<String>> = self.net.initial_marking().iter().map(|x| vec![x.to_string()]).collect();
                s += &latex("M_0", &m0);
                ui.ctx().copy_text(s);
                self.status = tr("petri.matrices_copied_latex").into();
            }
            if ui
                .button(tr("mining.save_csv"))
                .on_hover_text(tr("petri.pre_post_c_one_csv_file"))
                .clicked()
            {
                let mut out = String::new();
                for k in 0..3 {
                    out += &csv(titles[k], &places, &trans, &s_of(&solutions[k]));
                    out += "\n";
                }
                self.save_text(&format!("{}-matrices.csv", self.file_stem()), &out);
            }
        });

        let mut export: Option<(usize, bool, Rect)> = None;
        if let Some(answers) = &mut self.exercise {
            // ---- exercise mode
            for a in answers.iter_mut() {
                a.resize(places.len(), Vec::new());
                for row in a.iter_mut() {
                    row.resize(trans.len(), String::new());
                }
            }
            ui.label(RichText::new(tr("petri.type_values_then_check_empty_cells_count")).color(ui.visuals().weak_text_color()));
            let mut action = None;
            for k in 0..3 {
                let r = editable_matrix(
                    ui,
                    titles[k],
                    &places,
                    &trans,
                    &mut answers[k],
                    &solutions[k],
                    self.exercise_checked,
                );
                if let Some(a) = r {
                    action = Some((k, a));
                }
            }
            match action {
                Some((k, "fill")) => answers[k] = s_of(&solutions[k]),
                Some((k, "clear")) => {
                    for row in answers[k].iter_mut() {
                        row.iter_mut().for_each(|c| c.clear());
                    }
                }
                _ => {}
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.add(crate::theme::primary_button(ui, tr("petri.check_my_matrices"))).clicked() {
                    self.exercise_checked = true;
                }
                if self.exercise_checked && ui.button(tr("petri.hide_corrections")).clicked() {
                    self.exercise_checked = false;
                }
            });
            if self.exercise_checked {
                let wrong: usize = (0..3)
                    .map(|k| {
                        answers[k]
                            .iter()
                            .enumerate()
                            .flat_map(|(i, r)| r.iter().enumerate().map(move |(j, v)| (i, j, v)))
                            .filter(|(i, j, v)| parse_cell(v) != Some(solutions[k][*i][*j]))
                            .count()
                    })
                    .sum();
                let pal = Palette::for_ui(ui);
                if wrong == 0 {
                    ui.label(
                        RichText::new(tr("petri.all_three_matrices_correct"))
                            .color(pal.ok)
                            .font(crate::theme::semibold(14.0)),
                    );
                } else {
                    ui.label(
                        RichText::new(trf("petri.wrong_cell_s_highlighted_red", &[&wrong]))
                            .color(pal.bad)
                            .font(crate::theme::semibold(14.0)),
                    );
                }
            }
        } else {
            // ---- display mode
            for k in 0..3 {
                let (rect, png, csv_clicked) = matrix_grid(
                    ui,
                    titles[k],
                    &places,
                    &trans,
                    &s_of(&solutions[k]),
                    (k == 2).then_some(&solutions[2]),
                );
                if png {
                    export = Some((k, true, rect));
                }
                if csv_clicked {
                    export = Some((k, false, rect));
                }
            }
            let m0: Vec<Vec<String>> = self.net.initial_marking().iter().map(|x| vec![x.to_string()]).collect();
            let mut cols = vec!["M0".to_string()];
            let mut vals = m0.clone();
            if let Some(s) = &self.sim {
                cols.push("M".into());
                for (i, row) in vals.iter_mut().enumerate() {
                    row.push(s.marking[i].to_string());
                }
            }
            matrix_grid(ui, tr("petri.markings"), &places, &cols, &vals, None);
            ui.label(RichText::new(tr("petri.state_equation_m_m_c_sigma_sigma")).weak());
        }
        if let Some((k, png, rect)) = export {
            let name = format!("{}-{}", self.file_stem(), short[k]);
            if png {
                crate::io::request_png_export(ui.ctx(), rect.expand(4.0), format!("{name}.png"));
            } else {
                let text = csv(titles[k], &places, &trans, &s_of(&solutions[k]));
                self.save_text(&format!("{name}.csv"), &text);
            }
        }
    }

    pub(super) fn file_stem(&self) -> String {
        let s: String = self
            .net
            .name
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '_' })
            .collect();
        if s.trim_matches('_').is_empty() {
            "petri-net".into()
        } else {
            s.trim_matches('_').to_string()
        }
    }

    pub(super) fn save_text(&mut self, name: &str, text: &str) {
        match crate::io::save_file(name, text.as_bytes()) {
            Ok(Some(p)) => self.status = trf("app.exported", &[&(crate::io::file_name(&p))]),
            Ok(None) => {}
            Err(e) => self.status = trf("file.export_failed", &[&e]),
        }
    }

    pub(super) fn analysis_panel(&mut self, ui: &mut Ui) {
        if self.net.places.is_empty() {
            ui.label(tr("petri.nothing_analyse_yet"));
            return;
        }
        if !self.ensure_analysis(ui) {
            return;
        }
        let a = self.analysis.as_ref().unwrap();
        let net = &self.net;
        let pal = Palette::for_ui(ui);
        let pname = |i: usize| net.places[i].name.as_str();
        let tname = |i: usize| net.transitions[i].name.as_str();

        ui.horizontal(|ui| {
            ui.label(tr("petri.state_limit"));
            ui.add(egui::DragValue::new(&mut self.state_limit).range(10..=1_000_000).speed(100));
        });

        ui.add_space(4.0);
        crate::theme::section_title(ui, tr("petri.behaviour"));
        let b = &a.beh;
        let kind = if a.ss.has_omega() {
            tr("petri.coverability_graph")
        } else {
            tr("analysis.reachability_graph_name")
        };
        ui.label(trf(
            "petri.states_arcs",
            &[
                &b.state_count,
                &b.edge_count,
                &kind,
                &(if b.truncated { tr("analysis.truncated_warning") } else { "" }),
            ],
        ));
        egui::Grid::new("beh").num_columns(2).min_col_width(14.0).show(ui, |ui| {
            let k = b.place_bounds.iter().map(|x| x.unwrap_or(u64::MAX)).max().unwrap_or(0);
            let bounded_label = match b.bounded {
                Some(true) => trf("petri.bounded_bounded", &[&k]),
                _ => tr("petri.bounded").into(),
            };
            canvas::property_row(
                ui,
                &bounded_label,
                b.bounded,
                tr("petri.every_place_has_finite_maximum_number_tokens"),
            );
            canvas::property_row(
                ui,
                tr("petri.safe_1_bounded"),
                b.safe,
                tr("petri.no_place_ever_holds_more_than_one"),
            );
            canvas::property_row(
                ui,
                tr("petri.deadlock_free"),
                b.deadlock_free,
                tr("petri.no_reachable_marking_without_enabled_transitions"),
            );
            canvas::property_row(
                ui,
                tr("petri.live"),
                b.live,
                tr("petri.every_transition_can_always_fire_again_eventually"),
            );
            canvas::property_row(
                ui,
                tr("petri.reversible"),
                b.reversible,
                tr("petri.m0_can_reached_again_from_every_reachable"),
            );
            let all_quasi = b.quasi_live.iter().all(|&x| x);
            canvas::property_row(
                ui,
                tr("petri.quasi_live_no_dead_transition"),
                Some(all_quasi),
                tr("petri.every_transition_can_fire_at_least_once"),
            );
        });
        if b.truncated {
            ui.label(RichText::new(tr("petri.state_space_was_truncated_some_properties_undecided")).color(pal.warn));
        }

        if !b.deadlocks.is_empty() {
            ui.add_space(4.0);
            crate::theme::section_title(ui, tr("petri.deadlocks"));
            // In a workflow net, [o] (one token in the sink place) is the expected final marking.
            let is_final = |m: &[u64]| a.st.workflow_net && m.iter().sum::<u64>() == 1 && m[a.st.sink_places[0]] == 1;
            for &d in b.deadlocks.iter().take(10) {
                let path = a.ss.path_to(d).unwrap_or_default();
                let seq: Vec<&str> = path.iter().map(|&t| tname(t)).collect();
                let m = &a.ss.states[d];
                ui.label(format!(
                    "M{d} = {}  via σ = {}",
                    format_marking(m),
                    if seq.is_empty() { "ε".into() } else { seq.join(" ") }
                ));
                if is_final(m) {
                    ui.label(RichText::new(tr("petri.final_marking_o_proper_completion_workflow_expected")).color(pal.ok));
                }
            }
            if a.st.workflow_net {
                ui.label(RichText::new(tr("petri.workflow_net_liveness_reversibility_checked_short_circuited")).weak());
            }
        }

        ui.add_space(4.0);
        egui::CollapsingHeader::new(tr("petri.place_bounds_transitions"))
            .default_open(false)
            .show(ui, |ui| {
                egui::Grid::new("bounds").striped(true).show(ui, |ui| {
                    ui.label(RichText::new(tr("petri.place")).strong());
                    ui.label(RichText::new(tr("petri.bound")).strong());
                    ui.end_row();
                    for (i, bd) in b.place_bounds.iter().enumerate() {
                        ui.label(pname(i));
                        ui.label(bd.map(|x| x.to_string()).unwrap_or("ω".into()));
                        ui.end_row();
                    }
                });
                ui.add_space(4.0);
                egui::Grid::new("tlive").striped(true).show(ui, |ui| {
                    ui.label(RichText::new(tr("petri.transition")).strong());
                    ui.label(RichText::new(tr("petri.quasi_live")).strong());
                    ui.label(RichText::new(tr("petri.live")).strong());
                    ui.end_row();
                    for (i, q) in b.quasi_live.iter().enumerate() {
                        ui.label(tname(i));
                        ui.label(if *q { "✔" } else { tr("analysis.dead") });
                        ui.label(match &b.live_transitions {
                            Some(l) => {
                                if l[i] {
                                    "✔"
                                } else {
                                    "✖"
                                }
                            }
                            None => "?",
                        });
                        ui.end_row();
                    }
                });
            });

        ui.add_space(6.0);
        crate::theme::section_title(ui, tr("petri.invariants"));
        let m0 = net.initial_marking();
        match &a.pinv {
            Some(v) if !v.is_empty() => {
                ui.label(trf(
                    "petri.p_invariants",
                    &[
                        &v.len(),
                        &(if analysis::covered(v, net.places.len()) {
                            tr("analysis.net_covered_bounded")
                        } else {
                            ""
                        }),
                    ],
                ));
                for inv in v {
                    let terms: Vec<String> = inv
                        .iter()
                        .enumerate()
                        .filter(|(_, x)| **x != 0)
                        .map(|(i, &x)| {
                            if x == 1 {
                                format!("M({})", pname(i))
                            } else {
                                format!("{x}·M({})", pname(i))
                            }
                        })
                        .collect();
                    let k: i64 = inv.iter().zip(&m0).map(|(a, &b)| a * b as i64).sum();
                    ui.label(RichText::new(format!("{} = {k}", terms.join(" + "))).monospace());
                }
            }
            Some(_) => {
                ui.label(tr("petri.no_semi_positive_p_invariant"));
            }
            None => {
                ui.label(tr("petri.p_invariants_too_many_compute"));
            }
        }
        match &a.tinv {
            Some(v) if !v.is_empty() => {
                ui.label(trf(
                    "petri.t_invariants",
                    &[
                        &v.len(),
                        &(if analysis::covered(v, net.transitions.len()) {
                            tr("analysis.net_covered")
                        } else {
                            ""
                        }),
                    ],
                ));
                for inv in v {
                    let terms: Vec<String> = inv
                        .iter()
                        .enumerate()
                        .filter(|(_, x)| **x != 0)
                        .map(|(i, &x)| {
                            if x == 1 {
                                tname(i).to_string()
                            } else {
                                format!("{x}·{}", tname(i))
                            }
                        })
                        .collect();
                    ui.label(RichText::new(terms.join(" + ")).monospace());
                }
            }
            Some(_) => {
                ui.label(tr("petri.no_semi_positive_t_invariant_net_cannot"));
            }
            None => {
                ui.label(tr("petri.t_invariants_too_many_compute"));
            }
        }

        ui.add_space(6.0);
        crate::theme::section_title(ui, tr("petri.structure"));
        let s = &a.st;
        egui::Grid::new("struct").num_columns(2).min_col_width(14.0).show(ui, |ui| {
            canvas::property_row(ui, tr("petri.ordinary_all_weights_1"), Some(s.ordinary), "");
            canvas::property_row(ui, tr("petri.pure_no_self_loop"), Some(s.pure), "");
            canvas::property_row(
                ui,
                tr("petri.state_machine"),
                Some(s.state_machine),
                tr("petri.each_transition_has_exactly_one_input_one"),
            );
            canvas::property_row(
                ui,
                tr("petri.marked_graph"),
                Some(s.marked_graph),
                tr("petri.each_place_has_exactly_one_input_one"),
            );
            canvas::property_row(
                ui,
                tr("petri.free_choice"),
                Some(s.free_choice),
                tr("petri.conflicting_transitions_have_single_input_place"),
            );
            canvas::property_row(ui, tr("petri.extended_free_choice"), Some(s.extended_free_choice), "");
            canvas::property_row(
                ui,
                tr("petri.workflow_net"),
                Some(s.workflow_net),
                tr("petri.one_source_place_i_one_sink_place"),
            );
        });
        if s.has_inhibitor_arcs {
            ui.label(RichText::new(tr("petri.inhibitor_arcs_ignored_matrices_invariants")).weak());
        }
        if s.workflow_net {
            ui.add_space(4.0);
            match &a.sound {
                Soundness::Sound => {
                    ui.label(RichText::new(tr("analysis.sound_workflow")).color(pal.ok).strong());
                }
                Soundness::Unsound(why) => {
                    ui.label(RichText::new(tr("analysis.unsound_workflow")).color(pal.bad).strong());
                    for w in why {
                        ui.label(format!("• {w}"));
                    }
                }
                Soundness::Unknown => {
                    ui.label(RichText::new(tr("petri.soundness_undecided_increase_state_limit")).color(pal.warn));
                }
                Soundness::NotWorkflowNet => {}
            }
        }
    }

    pub(super) fn states_panel(&mut self, ui: &mut Ui) {
        if self.net.places.is_empty() {
            ui.label(tr("petri.nothing_explore_yet"));
            return;
        }
        if !self.ensure_analysis(ui) {
            return;
        }
        ui.horizontal(|ui| {
            if ui.button(tr("petri.open_reachability_graph")).clicked() {
                self.graph_open = true;
                self.graph_fit = true;
            }
            if ui.button(tr("petri.pane")).on_hover_text(tr("petri.show_graph_own_pane")).clicked() {
                self.request_pane = Some(crate::dock::View::Graph);
            }
        });
        let a = self.analysis.as_ref().unwrap();
        ui.label(trf(
            "petri.states_transitions",
            &[
                &a.ss.states.len(),
                &a.ss.edges.len(),
                &(if a.ss.truncated { tr("analysis.truncated") } else { "" }),
            ],
        ));
        ui.add_space(4.0);
        let tname = |i: usize| self.net.transitions[i].name.as_str();
        let mut clicked = None;
        egui::Grid::new("states").striped(true).show(ui, |ui| {
            ui.label(RichText::new(tr("petri.state")).strong());
            ui.label(RichText::new(tr("petri.marking")).strong());
            ui.label(RichText::new(tr("petri.successors")).strong());
            ui.end_row();
            for (i, m) in a.ss.states.iter().enumerate().take(500) {
                if ui.selectable_label(self.selected_state == Some(i), format!("M{i}")).clicked() {
                    clicked = Some(i);
                }
                ui.label(RichText::new(format_marking(m)).monospace());
                let succ: Vec<String> =
                    a.ss.edges
                        .iter()
                        .filter(|e| e.0 == i)
                        .map(|e| format!("{}→M{}", tname(e.1), e.2))
                        .collect();
                ui.label(if succ.is_empty() {
                    tr("petri.dead_state").to_string()
                } else {
                    succ.join(", ")
                });
                ui.end_row();
            }
        });
        if a.ss.states.len() > 500 {
            ui.label(tr("petri.only_first_500_states_listed"));
        }
        if let Some(i) = clicked {
            self.select_state(i);
        }
    }

    /// Selects a state; in simulation mode, jumps to it (replaying the shortest path).
    pub(super) fn select_state(&mut self, i: usize) {
        self.selected_state = Some(i);
        let Some(a) = &self.analysis else { return };
        if a.ss.states[i].contains(&OMEGA) {
            return;
        }
        if let (Some(path), Some(sim)) = (a.ss.path_to(i), &mut self.sim) {
            sim.marking = self.net.initial_marking();
            sim.trace.clear();
            sim.stack.clear();
            for t in path {
                self.fire(t);
            }
        }
    }

    pub(super) fn graph_window(&mut self, ui: &mut Ui) {
        if !self.ensure_analysis(ui) {
            return;
        }
        // Compare the graph's actual input, including after workspace restoration.
        // A rebuilt cache for identical data must preserve the restored camera.
        use std::hash::{Hash, Hasher};
        let a = self.analysis.as_ref().unwrap();
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&(&a.net, a.limit)).unwrap().hash(&mut hash);
        let key = hash.finish();
        if self.graph_key != Some(key) {
            self.graph_key = Some(key);
            self.graph_fit = true;
        }
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let canvas = response.rect;
        let painter = painter.with_clip_rect(canvas);
        let pal = Palette::for_ui(ui);
        let a = self.analysis.as_ref().unwrap();
        let ss = &a.ss;

        // Layered layout by BFS depth
        let maxd = ss.depth.iter().copied().max().unwrap_or(0);
        let mut layers: Vec<Vec<usize>> = vec![Vec::new(); maxd + 1];
        for (i, &d) in ss.depth.iter().enumerate() {
            layers[d].push(i);
        }
        let mut pos = vec![Pos::default(); ss.states.len()];
        // Box width: widest label at zoom 1 (monospace 11 px)
        let label_w = ss
            .states
            .iter()
            .enumerate()
            .take(2000)
            .map(|(i, m)| {
                painter
                    .layout_no_wrap(format!("M{i} {}", format_marking(m)), FontId::monospace(11.0), pal.text)
                    .size()
                    .x
            })
            .fold(0.0f32, f32::max)
            + 16.0;
        let dx = label_w.max(70.0) + 30.0;
        let widest = layers.iter().map(|l| l.len()).max().unwrap_or(1) as f32;
        for (d, layer) in layers.iter().enumerate() {
            let offset = (widest - layer.len() as f32) * dx / 2.0;
            for (k, &i) in layer.iter().enumerate() {
                pos[i] = Pos::new(offset + k as f32 * dx, d as f32 * 110.0);
            }
        }
        if self.graph_fit && !pos.is_empty() {
            let min = Pos::new(-dx / 2.0, -30.0);
            let max = Pos::new(widest * dx, maxd as f32 * 110.0 + 30.0);
            self.graph_view.fit(canvas, min, max);
            self.graph_fit = false;
        }
        self.graph_view.handle_navigation(ui, &response, canvas);
        if response.dragged_by(PointerButton::Primary) {
            self.graph_view.offset += response.drag_delta();
        }
        let v = self.graph_view;
        let z = v.zoom;
        painter.rect_filled(canvas, 0.0, pal.bg);
        // Text scales exactly with the boxes; when too small, only the state name is shown.
        let font = FontId::monospace(11.0 * z);
        let full_labels = 11.0 * z >= 5.5;
        let edge_labels = 11.0 * z >= 6.0;
        let box_size = vec2(label_w.max(70.0), 26.0) * z;
        let current = self.sim.as_ref().map(|s| s.marking.iter().map(|&x| x as u64).collect::<Vec<u64>>());

        for &(from, t, to) in &ss.edges {
            let (a2, b2) = (v.to_screen(canvas, pos[from]), v.to_screen(canvas, pos[to]));
            let tn = &self.net.transitions[t].name;
            if from == to {
                let c = a2 + vec2(box_size.x / 2.0 + 8.0 * z, 0.0);
                painter.circle_stroke(c, 9.0 * z, Stroke::new(1.2, pal.muted));
                if edge_labels {
                    canvas::label_with_halo(&painter, c + vec2(14.0 * z, 0.0), tn, 11.0 * z, &pal);
                }
                continue;
            }
            let rf = Rect::from_center_size(a2, box_size);
            let rt = Rect::from_center_size(b2, box_size);
            let s = clip_rect(rf, b2);
            let e = clip_rect(rt, a2);
            let dir = e - s;
            let back = ss.depth[to] <= ss.depth[from];
            let n = vec2(-dir.y, dir.x).normalized() * if back { 25.0 * z } else { 0.0 };
            let ctrl1 = s + dir * 0.33 + n;
            let ctrl2 = s + dir * 0.66 + n;
            canvas::bezier(&painter, [s, ctrl1, ctrl2, e], Stroke::new(1.3, pal.muted));
            canvas::arrow_head(&painter, e, e - ctrl2, 8.0 * z.max(0.6), pal.muted, true);
            let mid = s.lerp(e, 0.5) + n * 0.75;
            if edge_labels {
                canvas::label_with_halo(&painter, mid, tn, 11.0 * z, &pal);
            }
        }
        let mut clicked = None;
        let hover = response.hover_pos();
        for (i, m) in ss.states.iter().enumerate() {
            let c = v.to_screen(canvas, pos[i]);
            let r = Rect::from_center_size(c, box_size);
            let is_dead = a.beh.deadlocks.contains(&i);
            let is_current = current.as_ref() == Some(m);
            let fill = if is_current { pal.ok.gamma_multiply(0.35) } else { pal.fill };
            let stroke = if self.selected_state == Some(i) {
                Stroke::new(2.4, pal.select)
            } else if is_dead {
                Stroke::new(2.0, pal.bad)
            } else if i == 0 {
                Stroke::new(2.0, pal.ink)
            } else {
                Stroke::new(1.0, pal.ink)
            };
            painter.rect(r, CornerRadius::same(6), fill, stroke, StrokeKind::Middle);
            let text = if full_labels {
                format!("M{i} {}", format_marking(m))
            } else {
                format!("M{i}")
            };
            let size = if full_labels {
                font.clone()
            } else {
                FontId::monospace((11.0 * z).max(7.0).min(box_size.y * 0.8))
            };
            painter.text(c, egui::Align2::CENTER_CENTER, text, size, pal.text);
            if response.clicked() && hover.is_some_and(|h| r.contains(h)) {
                clicked = Some(i);
            }
        }
        painter.text(
            canvas.left_bottom() + vec2(8.0, -8.0),
            egui::Align2::LEFT_BOTTOM,
            tr("petri.thick_border_m0_red_deadlock_green_current"),
            FontId::proportional(11.0),
            pal.muted,
        );
        if let Some(i) = clicked {
            self.select_state(i);
        }
    }
}
