//! Inspector for the BpmnEditor notation.

use super::*;

impl BpmnEditor {
    pub(super) fn side_panel(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        let n_issues = self.diagram.validate().len();
        let vlabel = if n_issues == 0 {
            tr("bpmn.validation.valid").to_string()
        } else {
            trf("bpmn.validation.issue_count", &[&n_issues])
        };
        DiagramLayout::inspector_tabs(
            ui,
            &mut self.tab,
            &[
                (SideTab::Properties, tr("common.properties")),
                (SideTab::Validation, vlabel.as_str()),
                (SideTab::Evaluation, tr("view.evaluation")),
            ],
            |_, _| {},
        );
        DiagramLayout::inspector_body(ui, |ui| match self.tab {
            SideTab::Properties => self.properties(ui),
            SideTab::Validation => self.validation(ui),
            SideTab::Evaluation => self.evaluation(ui),
        });
    }

    pub(super) fn evaluation(&mut self, ui: &mut Ui) {
        use op_core::evaluation;
        ui.heading(tr("bpmn.process_evaluation"));
        ui.label(tr("bpmn.equiprobable_choices_no_waiting_between_activities_use"));
        ui.label(tr("bpmn.values_default_zero_sub_processes_treated_single"));
        let activities: Vec<_> = evaluation::activities(&self.diagram).collect();
        let mut metrics_changed = false;
        crate::theme::section_title(ui, tr("bpmn.activity_values"));
        egui::Grid::new("activity_metrics").num_columns(3).striped(true).show(ui, |ui| {
            ui.strong(tr("bpmn.activity"));
            ui.strong(tr("bpmn.cost_cai"));
            ui.strong(tr("bpmn.duration_tai"));
            ui.end_row();
            for (id, symbol) in &activities {
                let n = self.diagram.node_mut(*id).unwrap();
                ui.label(symbol).on_hover_text(&n.name);
                metrics_changed |= ui.add(egui::DragValue::new(&mut n.cost).range(0.0..=f64::MAX).speed(0.1)).changed();
                metrics_changed |= ui
                    .add(egui::DragValue::new(&mut n.duration).range(0.0..=f64::MAX).speed(0.1))
                    .changed();
                ui.end_row();
            }
        });
        for (id, symbol) in &activities {
            let n = self.diagram.node(*id).unwrap();
            if n.name == *symbol {
                continue;
            }
            ui.small(format!(
                "{symbol} = {}",
                if n.name.is_empty() { n.kind.label() } else { n.name.clone() }
            ));
        }
        crate::theme::section_title(ui, tr("bpmn.equations"));
        ui.label("C(σ) = Σi ni(σ) × CAi");
        ui.small(tr("bpmn.ni_sigma_counts_occurrences_activity_ai_trace"));
        ui.label(tr("bpmn.sequence_t_t1_tk"));
        ui.label(tr("bpmn.parallel_t_max_t1_tk"));
        ui.label(tr("bpmn.choice_xor_e_t_t1_tk_k"));
        ui.small(tr("bpmn.choices_inside_parallel_branches_compute_each_scenario"));
        if metrics_changed {
            self.evaluation_cache.invalidate();
            ui.ctx().request_repaint();
            return;
        }
        self.evaluation_cache.update(&self.diagram, self.history.generation(), ui.ctx());
        let Some(result) = self.evaluation_cache.result.as_ref() else {
            ui.spinner();
            ui.label(tr("petri.computing_analysis"));
            return;
        };
        let report = match result {
            Ok(r) => r,
            Err(message) => {
                ui.colored_label(Palette::for_ui(ui).bad, message);
                if ui.button(tr("petri.recompute")).clicked() {
                    self.evaluation_cache.invalidate();
                }
                return;
            }
        };
        crate::theme::section_title(ui, tr("bpmn.mean_execution_time"));
        ui.add(egui::Label::new(&report.duration_formula).wrap().selectable(true));
        ui.strong(format!("T(P) = {}", report.mean_duration));
        let symbols: std::collections::HashMap<_, _> = activities.into_iter().collect();
        let mut writeup = format!("{}\nT(P) = {}\n", report.duration_formula, report.mean_duration);
        crate::theme::section_title(ui, tr("bpmn.traces_costs"));
        ui.small(tr("bpmn.scenario_fixes_xor_choices_all_parallel_interleavings"));
        for (i, s) in report.scenarios.iter().enumerate() {
            let title = trf("bpmn.scenario_probability_1", &[&(i + 1), &s.denominator]);
            let cost = format!("C(σ) = {} = {}", s.cost_formula, s.cost);
            let duration = format!("T = {} = {}", s.duration_formula, s.duration);
            writeup.push_str(&format!("\n{title}\n{cost}\n{duration}\n"));
            let traces: Vec<_> = s
                .traces
                .iter()
                .map(|t| {
                    if t.is_empty() {
                        "ε".into()
                    } else {
                        t.iter().map(|id| symbols[id].as_str()).collect::<Vec<_>>().join(" → ")
                    }
                })
                .collect();
            for trace in &traces {
                writeup.push_str(trace);
                writeup.push('\n');
            }
            if s.traces_truncated {
                writeup.push_str(tr("bpmn.trace_display_limit_reached_cost_time_calculations"));
                writeup.push('\n');
            }
            egui::CollapsingHeader::new(title)
                .id_salt(("scenario", i))
                .default_open(report.scenarios.len() <= 4)
                .show(ui, |ui| {
                    ui.add(egui::Label::new(cost).wrap().selectable(true));
                    ui.add(egui::Label::new(duration).wrap().selectable(true));
                    for trace in &traces {
                        ui.add(egui::Label::new(trace).wrap().selectable(true));
                    }
                    if s.traces_truncated {
                        ui.label(tr("bpmn.trace_display_limit_reached_cost_time_calculations"));
                    }
                });
        }
        if ui.button(tr("bpmn.copy_equations_traces")).clicked() {
            ui.ctx().copy_text(writeup);
        }
    }

    pub(super) fn group_panel(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(trf("uml.elements_selected", &[&self.group.len()])).font(crate::theme::semibold(16.0)));
        ui.label(RichText::new(tr("bpmn.drag_one_them_move_whole_selection")).color(ui.visuals().weak_text_color()));
        crate::theme::section_title(ui, tr("bpmn.arrange"));
        let pos: Vec<(Id, Pos)> = self
            .group
            .iter()
            .filter_map(|&id| self.diagram.node(id).map(|n| (id, n.pos)))
            .collect();
        let k = pos.len().max(1) as f32;
        let (mx, my) = (
            pos.iter().map(|p| p.1.x).sum::<f32>() / k,
            pos.iter().map(|p| p.1.y).sum::<f32>() / k,
        );
        let set = |d: &mut BpmnDiagram, id: Id, p: Pos| {
            if let Some(n) = d.node_mut(id) {
                n.pos = p;
            }
        };
        ui.horizontal_wrapped(|ui| {
            if ui
                .button(tr("canvas.align_horizontal"))
                .on_hover_text(tr("bpmn.same_vertical_position_centres"))
                .clicked()
            {
                for (id, p) in &pos {
                    set(&mut self.diagram, *id, Pos::new(p.x, my));
                }
            }
            if ui
                .button(tr("canvas.align_vertical"))
                .on_hover_text(tr("bpmn.same_horizontal_position_centres"))
                .clicked()
            {
                for (id, p) in &pos {
                    set(&mut self.diagram, *id, Pos::new(mx, p.y));
                }
            }
            if ui.button(tr("canvas.distribute_horizontal")).clicked() && pos.len() > 2 {
                let mut v = pos.clone();
                v.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
                let (x0, x1) = (v[0].1.x, v[v.len() - 1].1.x);
                for (i, (id, p)) in v.iter().enumerate() {
                    set(
                        &mut self.diagram,
                        *id,
                        Pos::new(x0 + (x1 - x0) * i as f32 / (v.len() - 1) as f32, p.y),
                    );
                }
            }
            if ui.button(tr("canvas.distribute_vertical")).clicked() && pos.len() > 2 {
                let mut v = pos.clone();
                v.sort_by(|a, b| a.1.y.total_cmp(&b.1.y));
                let (y0, y1) = (v[0].1.y, v[v.len() - 1].1.y);
                for (i, (id, p)) in v.iter().enumerate() {
                    set(
                        &mut self.diagram,
                        *id,
                        Pos::new(p.x, y0 + (y1 - y0) * i as f32 / (v.len() - 1) as f32),
                    );
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

    pub(super) fn properties(&mut self, ui: &mut Ui) {
        if self.group.len() > 1 {
            self.group_panel(ui);
            return;
        }
        match self.selection {
            Some(Sel::Node(id)) => {
                let lane = self.diagram.lane_of(id).and_then(|(pid, lid)| {
                    let p = self.diagram.pool(pid)?;
                    let l = p.lanes.iter().find(|l| l.id == lid)?;
                    Some(if l.name.is_empty() {
                        p.name.clone()
                    } else {
                        format!("{} / {}", p.name, l.name)
                    })
                });
                let Some(n) = self.diagram.node_mut(id) else { return };
                ui.heading(n.kind.label());
                egui::Grid::new("np").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("common.name"));
                    ui.add(egui::TextEdit::multiline(&mut n.name).desired_rows(2).desired_width(f32::INFINITY))
                        .labelled_by(lbl.id);
                    ui.end_row();
                    match &mut n.kind {
                        NodeKind::StartEvent(t) => {
                            ui.label(tr("bpmn.trigger"));
                            trigger_combo(
                                ui,
                                t,
                                &[
                                    EventTrigger::None,
                                    EventTrigger::Message,
                                    EventTrigger::Timer,
                                    EventTrigger::Signal,
                                    EventTrigger::Conditional,
                                ],
                            );
                            ui.end_row();
                        }
                        NodeKind::EndEvent(t) => {
                            ui.label(tr("common.result"));
                            trigger_combo(
                                ui,
                                t,
                                &[
                                    EventTrigger::None,
                                    EventTrigger::Message,
                                    EventTrigger::Error,
                                    EventTrigger::Signal,
                                    EventTrigger::Terminate,
                                ],
                            );
                            ui.end_row();
                        }
                        NodeKind::IntermediateEvent { trigger, throwing } => {
                            ui.label(tr("bpmn.trigger"));
                            trigger_combo(
                                ui,
                                trigger,
                                &[
                                    EventTrigger::None,
                                    EventTrigger::Message,
                                    EventTrigger::Timer,
                                    EventTrigger::Signal,
                                    EventTrigger::Conditional,
                                    EventTrigger::Error,
                                ],
                            );
                            ui.end_row();
                            ui.label(tr("bpmn.throwing"));
                            ui.checkbox(throwing, "");
                            ui.end_row();
                        }
                        NodeKind::Task(t) => {
                            ui.label(tr("bpmn.task_type"));
                            egui::ComboBox::from_id_salt("tt").selected_text(t.label()).show_ui(ui, |ui| {
                                for x in TaskType::ALL {
                                    if node_option(ui, x.label(), NodeKind::Task(x), Some(*t == x)).clicked() {
                                        *t = x;
                                        ui.close();
                                    }
                                }
                            });
                            ui.end_row();
                        }
                        NodeKind::Gateway(g) => {
                            ui.label(tr("bpmn.gateway_type"));
                            egui::ComboBox::from_id_salt("gt").selected_text(g.label()).show_ui(ui, |ui| {
                                for x in GatewayType::ALL {
                                    if node_option(ui, x.label(), NodeKind::Gateway(x), Some(*g == x)).clicked() {
                                        *g = x;
                                        ui.close();
                                    }
                                }
                            });
                            ui.end_row();
                        }
                        _ => {}
                    }
                    if matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess | NodeKind::TextAnnotation) {
                        ui.label(tr("common.size"));
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut n.w).range(40.0..=600.0).prefix("w "));
                            ui.add(egui::DragValue::new(&mut n.h).range(30.0..=400.0).prefix("h "));
                        });
                        ui.end_row();
                    }
                    if let Some(l) = &lane {
                        ui.label(tr("bpmn.lane"));
                        ui.label(l);
                        ui.end_row();
                    }
                    if matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess) {
                        ui.label(tr("bpmn.cost_cai"));
                        ui.add(egui::DragValue::new(&mut n.cost).range(0.0..=f64::MAX).speed(0.1));
                        ui.end_row();
                        ui.label(tr("bpmn.duration_tai"));
                        ui.add(egui::DragValue::new(&mut n.duration).range(0.0..=f64::MAX).speed(0.1));
                        ui.end_row();
                    }
                });
                if matches!(n.kind, NodeKind::Task(_)) && ui.button(tr("bpmn.convert_sub_process")).clicked() {
                    n.kind = NodeKind::SubProcess;
                }
                if n.kind == NodeKind::SubProcess && ui.button(tr("bpmn.convert_task")).clicked() {
                    n.kind = NodeKind::Task(TaskType::Abstract);
                }
                ui.add_space(6.0);
                let lbl = ui.label(tr("bpmn.documentation"));
                ui.add(
                    egui::TextEdit::multiline(&mut n.documentation)
                        .desired_rows(4)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(lbl.id);
            }
            Some(Sel::Flow(id)) => {
                let src_kind = self.diagram.flow(id).and_then(|f| self.diagram.node(f.from)).map(|n| n.kind);
                let ends = self.diagram.flow(id).map(|f| {
                    let name = |i| {
                        self.diagram
                            .node(i)
                            .map(|n| if n.name.is_empty() { n.kind.label() } else { n.name.clone() })
                            .unwrap_or_default()
                    };
                    (name(f.from), name(f.to))
                });
                let Some(f) = self.diagram.flow_mut(id) else { return };
                ui.heading(match f.kind {
                    FlowKind::Sequence => tr("bpmn.sequence_flow"),
                    FlowKind::Message => tr("bpmn.message_flow"),
                    FlowKind::Association => tr("bpmn.association"),
                });
                if let Some((a, b)) = ends {
                    ui.label(format!("{a} → {b}"));
                }
                ui.horizontal(|ui| {
                    let lbl = ui.label(if matches!(src_kind, Some(NodeKind::Gateway(_))) {
                        tr("bpmn.condition")
                    } else {
                        tr("common.label")
                    });
                    ui.text_edit_singleline(&mut f.name).labelled_by(lbl.id);
                });
                if matches!(src_kind, Some(NodeKind::Gateway(GatewayType::Exclusive | GatewayType::Inclusive))) {
                    ui.checkbox(&mut f.is_default, tr("bpmn.default_flow"));
                }
                if f.kind == FlowKind::Sequence && ui.button(tr("bpmn.reverse_direction")).clicked() {
                    std::mem::swap(&mut f.from, &mut f.to);
                }
            }
            Some(Sel::Pool(id)) => {
                let Some(p) = self.diagram.pool_mut(id) else { return };
                ui.heading(tr("bpmn.pool"));
                egui::Grid::new("pool").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("common.name"));
                    ui.text_edit_singleline(&mut p.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("common.width"));
                    ui.add(egui::DragValue::new(&mut p.width).range(200.0..=5000.0));
                    ui.end_row();
                });
                ui.add_space(6.0);
                crate::theme::section_title(ui, tr("bpmn.lanes"));
                let mut remove = None;
                let many = p.lanes.len() > 1;
                for (i, l) in p.lanes.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let lbl = ui.label(format!("{}.", i + 1));
                        ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(120.0))
                            .labelled_by(lbl.id);
                        ui.add(egui::DragValue::new(&mut l.height).range(60.0..=2000.0).prefix("h "));
                        if many && ui.small_button("✖").on_hover_text(tr("bpmn.remove_lane")).clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    p.lanes.remove(i);
                }
                if ui.button(tr("bpmn.add_lane")).clicked() {
                    self.diagram.add_lane(id);
                }
            }
            Some(Sel::Lane(pid, i)) => {
                let Some(l) = self.diagram.pool_mut(pid).and_then(|p| p.lanes.get_mut(i)) else {
                    return;
                };
                ui.heading(tr("bpmn.lane"));
                egui::Grid::new("lane").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("common.name"));
                    ui.text_edit_singleline(&mut l.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("common.height"));
                    ui.add(egui::DragValue::new(&mut l.height).range(60.0..=2000.0));
                    ui.end_row();
                });
            }
            None => {
                ui.heading(tr("bpmn.process"));
                ui.horizontal(|ui| {
                    let lbl = ui.label(tr("common.name"));
                    ui.text_edit_singleline(&mut self.diagram.name).labelled_by(lbl.id);
                });
                ui.add_space(8.0);
                if ui
                    .button(tr("bpmn.convert_petri_net"))
                    .on_hover_text(tr("bpmn.translate_process_into_workflow_net_analyse"))
                    .clicked()
                {
                    self.action = Some(BpmnAction::ConvertToPetri);
                }
                ui.add_space(8.0);
                crate::theme::section_title(ui, tr("bpmn.tips"));
                ui.label(tr("bpmn.pick_element_palette_then_click_canvas_dropping"));
                ui.label(tr("bpmn.select_element_get_quick_append_pad_task"));
                ui.label(tr("bpmn.connect_c_drag_from_element_another"));
                ui.label(tr("bpmn.double_click_f2_rename_right_click_more"));
                ui.label(tr("bpmn.pools_drag_title_band_resize_bottom_right"));
                ui.label(tr("bpmn.wheel_scroll_ctrl_wheel_zoom_f_zoom"));
            }
        }
    }

    pub(super) fn validation(&mut self, ui: &mut Ui) {
        let pal = Palette::for_ui(ui);
        let issues = self.diagram.validate();
        ui.checkbox(&mut self.show_issues, tr("bpmn.highlight_diagram"));
        ui.add_space(4.0);
        if issues.is_empty() {
            ui.label(RichText::new(tr("bpmn.no_problem_found")).color(pal.ok));
            return;
        }
        for issue in issues {
            let (sym, color) = match issue.severity {
                Severity::Error => ("✖", pal.bad),
                Severity::Warning => ("⚠", pal.warn),
            };
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(sym).color(color));
                let r = ui.add(egui::Label::new(&issue.message).sense(Sense::click()));
                if r.clicked()
                    && let Some(id) = issue.node
                {
                    self.selection = Some(Sel::Node(id));
                }
            });
        }
    }
}
