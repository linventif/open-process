use super::*;
use op_core::uml::{NodeKind, RelationKind};

pub(super) fn right_click(h: &mut Harness<'static, OpenProcessApp>, pos: egui::Pos2) {
    h.hover_at(pos);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        });
        h.run_steps(2);
    }
}

fn pair(kind: DiagramKind, a: NodeKind, b: NodeKind, dark: bool, width: f32) -> (Harness<'static, OpenProcessApp>, u64, u64) {
    let mut diagram = UmlDiagram::new("Relation tools", kind);
    let a = diagram.add_node(a, Pos::new(120.0, 120.0), "Source").unwrap();
    let b = diagram.add_node(b, Pos::new(520.0, 180.0), "Target").unwrap();
    let mut h = harness_sized(dark, [width, 860.0]);
    h.state_mut().open_model(Model::Uml(diagram), None, true);
    h.run_steps(5);
    (h, a, b)
}

#[test]
fn uml_relation_tools_create_each_valid_type_and_keep_atomic_history() {
    for (i, &relation) in DiagramKind::Class.relation_kinds().iter().enumerate() {
        let target = if relation == RelationKind::Realization {
            NodeKind::Interface
        } else {
            NodeKind::Class
        };
        let (mut h, a, b) = pair(DiagramKind::Class, NodeKind::Class, target, i % 2 == 1, 1400.0);
        let original = h.state().docs[h.state().active].snapshot();
        assert!(h.query_by_role(egui::accesskit::Role::ComboBox).is_none());
        assert!(h.query_by_label("Arrange").is_none());
        assert!(h.query_by_label("Convert to Mermaid").is_none());
        let connect = h.get_by_label("Connect").rect();
        h.hover_at(connect.center());
        h.run_steps(4);
        assert!(h.query_by_label("Dependency").is_some());
        assert!(h.query_by_label("Include").is_none());
        assert_eq!(
            h.state().docs[h.state().active].snapshot(),
            original,
            "hovering cannot edit a diagram"
        );
        if relation == RelationKind::Composition {
            shot(&mut h, "uml_relation_tools_class");
        }
        h.get_by_label(tr(relation.label())).click();
        h.run_steps(3);
        assert_eq!(
            h.state().docs[h.state().active].snapshot(),
            original,
            "choosing a tool cannot edit a diagram"
        );
        let from = uml(&h).node_screen_center(a);
        let to = uml(&h).node_screen_center(b);
        if i % 2 == 0 {
            drag(&mut h, from, to);
        } else {
            click(&mut h, from);
            click(&mut h, to);
        }
        assert_eq!(uml(&h).diagram.relations.len(), 1, "{relation:?}");
        assert_eq!(uml(&h).diagram.relations[0].kind, relation);
        assert_eq!((uml(&h).diagram.relations[0].from, uml(&h).diagram.relations[0].to), (a, b));
        assert!(uml(&h).diagram.validate().is_empty());
        h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
        h.run_steps(3);
        assert_eq!(h.state().docs[h.state().active].snapshot(), original);
        h.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::Z);
        h.run_steps(3);
        assert_eq!(uml(&h).diagram.relations[0].kind, relation);
        // C reuses the chosen type after switching to the selection tool.
        h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
        h.run_steps(3);
        h.get_by_label("Select / move").click();
        h.run_steps(2);
        h.hover_at(from);
        h.key_press(Key::C);
        h.run_steps(2);
        click(&mut h, from);
        click(&mut h, to);
        assert_eq!(uml(&h).diagram.relations.len(), 1);
        assert_eq!(uml(&h).diagram.relations[0].kind, relation);
    }
}

#[test]
fn uml_relation_flyout_supports_hover_keyboard_escape_and_narrow_use_cases() {
    let (mut h, a, b) = pair(DiagramKind::UseCase, NodeKind::UseCase, NodeKind::UseCase, true, 420.0);
    let original = h.state().docs[h.state().active].snapshot();
    let connect = h.get_by_label("Connect").rect();
    h.hover_at(connect.center());
    h.run_steps(4);
    assert!(h.query_by_label("Include").is_some());
    assert!(h.query_by_label("Extend").is_some());
    assert!(h.query_by_label("Aggregation").is_none());
    assert!(h.query_by_label("Realization").is_none());
    shot(&mut h, "uml_relation_tools_use_case_narrow");
    h.key_press(Key::Escape);
    h.run_steps(4);
    assert!(h.query_by_label("Include").is_none(), "Escape must not immediately reopen on hover");
    // A focused palette button can open its picker without a mouse hover.
    h.hover_at(egui::pos2(350.0, 500.0));
    h.get_by_role_and_label(egui::accesskit::Role::Button, "Connect").focus();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(4);
    assert!(h.query_by_label("Include").is_some());
    h.get_by_label("Include").focus();
    h.run_steps(2);
    h.key_press(Key::Enter);
    h.run_steps(3);
    assert_eq!(h.state().docs[h.state().active].snapshot(), original);
    let from = uml(&h).node_screen_center(a);
    let to = uml(&h).node_screen_center(b);
    click(&mut h, from);
    click(&mut h, to);
    assert_eq!(uml(&h).diagram.relations[0].kind, RelationKind::Include);
    // Switching relation types abandons the previous unfinished source.
    click(&mut h, from);
    let connect = h.get_by_role_and_label(egui::accesskit::Role::Button, "Connect").rect();
    h.hover_at(connect.center());
    h.run_steps(4);
    h.get_by_label("Extend").click();
    h.run_steps(3);
    click(&mut h, to);
    assert_eq!(uml(&h).diagram.relations.len(), 1);
    click(&mut h, from);
    assert_eq!(uml(&h).diagram.relations.len(), 2);
    assert_eq!(uml(&h).diagram.relations[1].kind, RelationKind::Extend);
    assert_eq!((uml(&h).diagram.relations[1].from, uml(&h).diagram.relations[1].to), (b, a));
    h.get_by_label("Select / move").click();
    h.run_steps(2);
    let connect = h.get_by_label("Connect").rect();
    h.hover_at(connect.center());
    h.run_steps(4);
    assert!(h.query_by_label("Extend").is_some());
    h.hover_at(egui::pos2(350.0, 600.0));
    h.run_steps(4);
    assert!(
        h.query_by_label("Extend").is_none(),
        "leaving the tool/menu must dismiss the flyout"
    );
}

#[test]
fn uml_arrangement_stays_available_in_the_inspector_and_context_menu() {
    let (mut h, a, b) = pair(DiagramKind::Class, NodeKind::Class, NodeKind::Class, false, 1400.0);
    super::open_inspector(&mut h);
    let original = h.state().docs[h.state().active].snapshot();
    h.hover_at(uml(&h).node_screen_center(a));
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run_steps(3);
    assert_eq!(uml(&h).selected_nodes().len(), 2);
    let arrange = h.get_by_label("Arrange").rect();
    assert!(arrange.left() > 900.0, "selection actions belong to the inspector");
    h.get_by_label("Arrange").click();
    h.run_steps(3);
    h.get_by_label("Top").click();
    h.run_steps(3);
    assert_eq!(uml(&h).diagram.node(a).unwrap().bounds.y, uml(&h).diagram.node(b).unwrap().bounds.y);
    h.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    h.run_steps(3);
    assert_eq!(h.state().docs[h.state().active].snapshot(), original);
    h.hover_at(uml(&h).node_screen_center(a));
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run_steps(3);
    right_click(&mut h, egui::pos2(700.0, 500.0));
    assert!(h.query_all_by_label("Arrange").next().is_some());
    assert!(h.query_by_label("Duplicate").is_some());
    assert!(h.query_by_label("Convert to Mermaid").is_some());
    shot(&mut h, "uml_selection_context_actions");
}
