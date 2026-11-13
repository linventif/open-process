//! Inspector for the UmlEditor notation.

use super::*;

impl UmlEditor {
    pub(super) fn properties(&mut self, ui: &mut Ui) {
        ui.label(tr("document.name"));
        ui.text_edit_singleline(&mut self.diagram.name);
        let issues = self.diagram.validate();
        if !issues.is_empty() {
            egui::CollapsingHeader::new(trf("uml.model_issues", &[&issues.len()])).show(ui, |ui| {
                for (id, message) in &issues {
                    if ui.selectable_label(false, message).clicked()
                        && let Some(id) = id
                    {
                        if self.diagram.node(*id).is_some() {
                            self.select_node(*id, false);
                        } else {
                            self.group.clear();
                            self.selection = Some(Selection::Relation(*id));
                        }
                    }
                }
            });
        }
        ui.separator();
        match self.selection {
            Some(Selection::Node(_)) if self.group.len() > 1 => {
                ui.heading(trf("uml.elements_selected", &[&self.group.len()]));
                ui.weak(tr("uml.use_arrange_align_distribute_selection"));
                if ui.button(tr("common.delete_selection")).clicked() {
                    self.delete_selection();
                }
            }
            Some(Selection::Node(id)) => {
                let Some(n) = self.diagram.nodes.iter_mut().find(|n| n.id == id) else {
                    return;
                };
                ui.heading(tr(n.kind.label()));
                ui.label(tr("common.name"));
                let name_edit = ui.text_edit_singleline(&mut n.name);
                name_edit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::TextInput, true, tr("common.element_name")));
                let mut content_changed = name_edit.changed();
                if n.kind.is_classifier() {
                    if n.kind == NodeKind::Class {
                        ui.checkbox(&mut n.abstract_class, tr("uml.abstract_class"));
                    }
                    ui.label(if n.kind == NodeKind::Enumeration {
                        tr("uml.literals")
                    } else {
                        tr("uml.attributes")
                    });
                    let attributes_edit = ui.add(
                        egui::TextEdit::multiline(&mut n.attributes)
                            .desired_rows(5)
                            .desired_width(f32::INFINITY)
                            .hint_text("- name: String"),
                    );
                    content_changed |= attributes_edit.changed();
                    ui.label(tr("uml.operations"));
                    let operations_edit = ui.add(
                        egui::TextEdit::multiline(&mut n.operations)
                            .desired_rows(5)
                            .desired_width(f32::INFINITY)
                            .hint_text("+ submit(): bool"),
                    );
                    content_changed |= operations_edit.changed();
                    ui.weak(tr("uml.one_member_per_line_visibility_public_private"));
                }
                if content_changed {
                    Self::fit_text(ui.painter(), n, false);
                }
                if n.kind != NodeKind::System && ui.button(tr("canvas.fit_content")).clicked() {
                    Self::fit_text(ui.painter(), n, true);
                }
                ui.horizontal(|ui| {
                    ui.label(tr("common.width"));
                    ui.add(egui::DragValue::new(&mut n.bounds.w).range(80.0..=2000.0));
                });
                ui.horizontal(|ui| {
                    ui.label(tr("common.height"));
                    let min = if n.kind == NodeKind::Actor { 130.0 } else { 60.0 };
                    ui.add(egui::DragValue::new(&mut n.bounds.h).range(min..=2000.0));
                });
                if ui.button(tr("common.delete_selection")).clicked() {
                    self.delete_selection();
                }
            }
            Some(Selection::Relation(id)) => {
                let Some(r) = self.diagram.relations.iter().find(|r| r.id == id) else {
                    return;
                };
                let (from, to, current) = (r.from, r.to, r.kind);
                let allowed: Vec<_> = self
                    .diagram
                    .kind
                    .relation_kinds()
                    .iter()
                    .copied()
                    .filter(|&k| self.diagram.can_connect_excluding(from, to, k, Some(id)))
                    .collect();
                let mut selected_kind = current;
                egui::ComboBox::from_id_salt("selected_relation")
                    .selected_text(tr(current.label()))
                    .show_ui(ui, |ui| {
                        for kind in allowed {
                            ui.selectable_value(&mut selected_kind, kind, tr(kind.label()));
                        }
                    });
                if ui.button(tr("bpmn.reverse_direction")).clicked() {
                    if self.diagram.can_connect_excluding(to, from, selected_kind, Some(id)) {
                        let r = self.diagram.relations.iter_mut().find(|r| r.id == id).unwrap();
                        std::mem::swap(&mut r.from, &mut r.to);
                        std::mem::swap(&mut r.source_multiplicity, &mut r.target_multiplicity);
                        std::mem::swap(&mut r.source_role, &mut r.target_role);
                    } else {
                        self.status = tr("uml.relation_not_valid_these_elements").into();
                    }
                }
                let r = self.diagram.relations.iter_mut().find(|r| r.id == id).unwrap();
                r.kind = selected_kind;
                ui.label(tr("common.label"));
                ui.text_edit_singleline(&mut r.label);
                if self.diagram.kind == DiagramKind::Class {
                    ui.label(tr("uml.source_multiplicity"));
                    ui.text_edit_singleline(&mut r.source_multiplicity);
                    ui.label(tr("uml.target_multiplicity"));
                    ui.text_edit_singleline(&mut r.target_multiplicity);
                    ui.label(tr("uml.source_role"));
                    ui.text_edit_singleline(&mut r.source_role);
                    ui.label(tr("uml.target_role"));
                    ui.text_edit_singleline(&mut r.target_role);
                }
                if ui.button(tr("common.delete_selection")).clicked() {
                    self.delete_selection();
                }
            }
            None => {
                ui.weak(tr("uml.select_element_relation_edit_properties"));
            }
        }
    }

    pub(super) fn fit_text(painter: &Painter, node: &mut Node, shrink: bool) {
        let title = painter
            .layout_no_wrap(node.name.clone(), crate::theme::semibold(14.0), egui::Color32::WHITE)
            .size()
            .x;
        let members = node
            .attributes
            .lines()
            .chain(node.operations.lines())
            .map(|line| {
                painter
                    .layout_no_wrap(line.into(), FontId::monospace(12.0), egui::Color32::WHITE)
                    .size()
                    .x
            })
            .fold(0.0, f32::max);
        let min = if node.kind == NodeKind::Actor {
            90.0
        } else if node.kind == NodeKind::UseCase {
            180.0
        } else {
            220.0
        };
        let width = (title.max(members) + 32.0).clamp(min, 2000.0);
        node.bounds.w = if shrink { width } else { node.bounds.w.max(width) };
        if shrink && node.kind.is_classifier() {
            node.bounds.h = 60.0;
            node.bounds.h = node.rect().h;
        }
    }
}
