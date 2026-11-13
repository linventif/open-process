//! Inspector for the BpmnEditor notation.

use super::*;

impl BpmnEditor {
    pub(super) fn side_panel(&mut self, ui: &mut Ui) {
        ui.add_space(4.0);
        let n_issues = self.diagram.validate().len();
        let vlabel = if n_issues == 0 {
            tr("Validation ✔").to_string()
        } else {
            trf("Validation ({})", &[&n_issues])
        };
        DiagramLayout::inspector_tabs(
            ui,
            &mut self.tab,
            &[
                (SideTab::Properties, tr("Properties")),
                (SideTab::Validation, vlabel.as_str()),
                (SideTab::Evaluation, tr("Evaluation")),
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
        ui.heading(tr("Process evaluation"));
        ui.label(tr(
            "Equiprobable choices, no waiting between activities. Use the same time and cost units throughout.",
        ));
        ui.label(tr("Values default to zero. Sub-processes are treated as single activities."));
        let activities: Vec<_> = evaluation::activities(&self.diagram).collect();
        crate::theme::section_title(ui, tr("Activity values"));
        egui::Grid::new("activity_metrics").num_columns(3).striped(true).show(ui, |ui| {
            ui.strong(tr("Activity"));
            ui.strong(tr("Cost CAi"));
            ui.strong(tr("Duration TAi"));
            ui.end_row();
            for (id, symbol) in &activities {
                let n = self.diagram.node_mut(*id).unwrap();
                ui.label(symbol).on_hover_text(&n.name);
                ui.add(egui::DragValue::new(&mut n.cost).range(0.0..=f64::MAX).speed(0.1));
                ui.add(egui::DragValue::new(&mut n.duration).range(0.0..=f64::MAX).speed(0.1));
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
        crate::theme::section_title(ui, tr("Equations"));
        ui.label("C(σ) = Σi ni(σ) × CAi");
        ui.small(tr("ni(σ) counts the occurrences of activity Ai in trace σ."));
        ui.label(tr("Sequence: T = T1 + … + Tk"));
        ui.label(tr("Parallel (AND): T = max(T1, …, Tk)"));
        ui.label(tr("Choice (XOR): E[T] = (T1 + … + Tk) / k"));
        ui.small(tr(
            "For choices inside parallel branches, compute each scenario's maximum before averaging.",
        ));
        let report = match evaluation::evaluate(&self.diagram) {
            Ok(r) => r,
            Err(message) => {
                ui.colored_label(Palette::for_ui(ui).bad, message);
                return;
            }
        };
        crate::theme::section_title(ui, tr("Mean execution time"));
        ui.add(egui::Label::new(&report.duration_formula).wrap().selectable(true));
        ui.strong(format!("T(P) = {}", report.mean_duration));
        let symbols: std::collections::HashMap<_, _> = activities.into_iter().collect();
        let mut writeup = format!("{}\nT(P) = {}\n", report.duration_formula, report.mean_duration);
        crate::theme::section_title(ui, tr("Traces and costs"));
        ui.small(tr("A scenario fixes the XOR choices. All its parallel interleavings have the same cost and elapsed time; their count does not change its probability."));
        for (i, s) in report.scenarios.iter().enumerate() {
            let title = trf("Scenario {} · probability 1/{}", &[&(i + 1), &s.denominator]);
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
                writeup.push_str(tr("Trace display limit reached; cost and time calculations remain exact."));
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
                        ui.label(tr("Trace display limit reached; cost and time calculations remain exact."));
                    }
                });
        }
        if ui.button(tr("Copy equations and traces")).clicked() {
            ui.ctx().copy_text(writeup);
        }
    }

    pub(super) fn group_panel(&mut self, ui: &mut Ui) {
        ui.label(RichText::new(trf("{} elements selected", &[&self.group.len()])).font(crate::theme::semibold(16.0)));
        ui.label(RichText::new(tr("Drag one of them to move the whole selection")).color(ui.visuals().weak_text_color()));
        crate::theme::section_title(ui, tr("Arrange"));
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
                .button(tr("Align horizontally"))
                .on_hover_text(tr("Same vertical position (centres)"))
                .clicked()
            {
                for (id, p) in &pos {
                    set(&mut self.diagram, *id, Pos::new(p.x, my));
                }
            }
            if ui
                .button(tr("Align vertically"))
                .on_hover_text(tr("Same horizontal position (centres)"))
                .clicked()
            {
                for (id, p) in &pos {
                    set(&mut self.diagram, *id, Pos::new(mx, p.y));
                }
            }
            if ui.button(tr("Distribute horizontally")).clicked() && pos.len() > 2 {
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
            if ui.button(tr("Distribute vertically")).clicked() && pos.len() > 2 {
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
            .button(RichText::new(tr("Delete selection")).color(Palette::for_ui(ui).bad))
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
                    let lbl = ui.label(tr("Name"));
                    ui.add(egui::TextEdit::multiline(&mut n.name).desired_rows(2).desired_width(f32::INFINITY))
                        .labelled_by(lbl.id);
                    ui.end_row();
                    match &mut n.kind {
                        NodeKind::StartEvent(t) => {
                            ui.label(tr("Trigger"));
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
                            ui.label(tr("Result"));
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
                            ui.label(tr("Trigger"));
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
                            ui.label(tr("Throwing"));
                            ui.checkbox(throwing, "");
                            ui.end_row();
                        }
                        NodeKind::Task(t) => {
                            ui.label(tr("Task type"));
                            egui::ComboBox::from_id_salt("tt").selected_text(t.label()).show_ui(ui, |ui| {
                                for x in TaskType::ALL {
                                    ui.selectable_value(t, x, x.label());
                                }
                            });
                            ui.end_row();
                        }
                        NodeKind::Gateway(g) => {
                            ui.label(tr("Gateway type"));
                            egui::ComboBox::from_id_salt("gt").selected_text(g.label()).show_ui(ui, |ui| {
                                for x in GatewayType::ALL {
                                    ui.selectable_value(g, x, x.label());
                                }
                            });
                            ui.end_row();
                        }
                        _ => {}
                    }
                    if matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess | NodeKind::TextAnnotation) {
                        ui.label(tr("Size"));
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut n.w).range(40.0..=600.0).prefix("w "));
                            ui.add(egui::DragValue::new(&mut n.h).range(30.0..=400.0).prefix("h "));
                        });
                        ui.end_row();
                    }
                    if let Some(l) = &lane {
                        ui.label(tr("Lane"));
                        ui.label(l);
                        ui.end_row();
                    }
                    if matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess) {
                        ui.label(tr("Cost CAi"));
                        ui.add(egui::DragValue::new(&mut n.cost).range(0.0..=f64::MAX).speed(0.1));
                        ui.end_row();
                        ui.label(tr("Duration TAi"));
                        ui.add(egui::DragValue::new(&mut n.duration).range(0.0..=f64::MAX).speed(0.1));
                        ui.end_row();
                    }
                });
                if matches!(n.kind, NodeKind::Task(_)) && ui.button(tr("Convert to sub-process")).clicked() {
                    n.kind = NodeKind::SubProcess;
                }
                if n.kind == NodeKind::SubProcess && ui.button(tr("Convert to task")).clicked() {
                    n.kind = NodeKind::Task(TaskType::Abstract);
                }
                ui.add_space(6.0);
                let lbl = ui.label(tr("Documentation"));
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
                    FlowKind::Sequence => tr("Sequence flow"),
                    FlowKind::Message => tr("Message flow"),
                    FlowKind::Association => tr("Association"),
                });
                if let Some((a, b)) = ends {
                    ui.label(format!("{a} → {b}"));
                }
                ui.horizontal(|ui| {
                    let lbl = ui.label(if matches!(src_kind, Some(NodeKind::Gateway(_))) {
                        tr("Condition")
                    } else {
                        tr("Label")
                    });
                    ui.text_edit_singleline(&mut f.name).labelled_by(lbl.id);
                });
                if matches!(src_kind, Some(NodeKind::Gateway(GatewayType::Exclusive | GatewayType::Inclusive))) {
                    ui.checkbox(&mut f.is_default, tr("Default flow"));
                }
                if f.kind == FlowKind::Sequence && ui.button(tr("Reverse direction")).clicked() {
                    std::mem::swap(&mut f.from, &mut f.to);
                }
            }
            Some(Sel::Pool(id)) => {
                let Some(p) = self.diagram.pool_mut(id) else { return };
                ui.heading(tr("Pool"));
                egui::Grid::new("pool").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut p.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("Width"));
                    ui.add(egui::DragValue::new(&mut p.width).range(200.0..=5000.0));
                    ui.end_row();
                });
                ui.add_space(6.0);
                crate::theme::section_title(ui, tr("Lanes"));
                let mut remove = None;
                let many = p.lanes.len() > 1;
                for (i, l) in p.lanes.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        let lbl = ui.label(format!("{}.", i + 1));
                        ui.add(egui::TextEdit::singleline(&mut l.name).desired_width(120.0))
                            .labelled_by(lbl.id);
                        ui.add(egui::DragValue::new(&mut l.height).range(60.0..=2000.0).prefix("h "));
                        if many && ui.small_button("✖").on_hover_text(tr("Remove lane")).clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    p.lanes.remove(i);
                }
                if ui.button(tr("Add lane")).clicked() {
                    self.diagram.add_lane(id);
                }
            }
            Some(Sel::Lane(pid, i)) => {
                let Some(l) = self.diagram.pool_mut(pid).and_then(|p| p.lanes.get_mut(i)) else {
                    return;
                };
                ui.heading(tr("Lane"));
                egui::Grid::new("lane").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut l.name).labelled_by(lbl.id);
                    ui.end_row();
                    ui.label(tr("Height"));
                    ui.add(egui::DragValue::new(&mut l.height).range(60.0..=2000.0));
                    ui.end_row();
                });
            }
            None => {
                ui.heading(tr("Process"));
                ui.horizontal(|ui| {
                    let lbl = ui.label(tr("Name"));
                    ui.text_edit_singleline(&mut self.diagram.name).labelled_by(lbl.id);
                });
                ui.add_space(8.0);
                if ui
                    .button(tr("⇄ Convert to Petri net"))
                    .on_hover_text(tr("Translate this process into a workflow net and analyse it"))
                    .clicked()
                {
                    self.action = Some(BpmnAction::ConvertToPetri);
                }
                ui.add_space(8.0);
                crate::theme::section_title(ui, tr("Tips"));
                ui.label(tr(
                    "• Pick an element in the palette, then click on the canvas. Dropping it on a flow inserts it.",
                ));
                ui.label(tr("• Select an element to get the quick-append pad (task, gateway, event…)."));
                ui.label(tr("• Connect (C): drag from an element to another."));
                ui.label(tr("• Double-click or F2 to rename; right-click for more actions."));
                ui.label(tr("• Pools: drag by the title band; resize with the bottom-right handle."));
                ui.label(tr("• Wheel = scroll, Ctrl+wheel = zoom, F = zoom to fit."));
            }
        }
    }

    pub(super) fn validation(&mut self, ui: &mut Ui) {
        let pal = Palette::for_ui(ui);
        let issues = self.diagram.validate();
        ui.checkbox(&mut self.show_issues, tr("Highlight on the diagram"));
        ui.add_space(4.0);
        if issues.is_empty() {
            ui.label(RichText::new(tr("✔ No problem found")).color(pal.ok));
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
