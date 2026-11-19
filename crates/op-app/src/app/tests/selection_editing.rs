use super::*;
use crate::editor::clipboard::{COPY, CUT, DUPLICATE, PASTE, SELECT_ALL};
use op_core::document::{DocumentCodec, JsonCodec};

const KINDS: &[&str] = &[
    "bpmn",
    "petri",
    "uml.class",
    "uml.use-case",
    "epc",
    "uml.activity",
    "flowchart",
    "database.schema",
];

fn document(kind: &'static str) -> Document {
    let a = Pos::new(250.0, 220.0);
    let b = Pos::new(550.0, 220.0);
    match kind {
        "bpmn" => {
            use op_core::bpmn::*;
            let mut d = BpmnDiagram::new("Clipboard");
            let a = d.add_node(NodeKind::Task(TaskType::User), a);
            let b = d.add_node(NodeKind::Task(TaskType::Service), b);
            d.add_flow(a, b).unwrap();
            BpmnDiagram::CODEC.encode(&d)
        }
        "petri" => {
            use op_core::petri::*;
            let mut d = PetriNet::new("Clipboard");
            let a = NodeRef::Place(d.add_place(a, 3));
            let b = NodeRef::Transition(d.add_transition(b));
            d.add_arc(a, b).unwrap();
            PetriNet::CODEC.encode(&d)
        }
        "uml.class" | "uml.use-case" => {
            use op_core::uml::*;
            let mut d = UmlDiagram::new(
                "Clipboard",
                if kind == "uml.class" {
                    DiagramKind::Class
                } else {
                    DiagramKind::UseCase
                },
            );
            let a = d
                .add_node(if kind == "uml.class" { NodeKind::Class } else { NodeKind::Actor }, a, "Source")
                .unwrap();
            let b = d
                .add_node(if kind == "uml.class" { NodeKind::Class } else { NodeKind::UseCase }, b, "Target")
                .unwrap();
            d.connect(a, b, RelationKind::Association).unwrap();
            JsonCodec::new(kind).encode(&d)
        }
        "epc" => {
            use op_core::epc::*;
            let mut d = EpcDiagram::new("Clipboard");
            let a = d.add_node(EpcKind::Event, a, "Ready");
            let b = d.add_node(EpcKind::Function, b, "Act");
            d.connect(a, b, "go").unwrap();
            JsonCodec::new(kind).encode(&d)
        }
        "uml.activity" => {
            use op_core::activity::*;
            let mut d = ActivityDiagram::new("Clipboard");
            let a = d.add_node(ActivityKind::Action, a, "A");
            let b = d.add_node(ActivityKind::Action, b, "B");
            d.connect(a, b, "go").unwrap();
            JsonCodec::new(kind).encode(&d)
        }
        "flowchart" => {
            use op_core::flowchart::*;
            let mut d = FlowchartDiagram::new("Clipboard");
            let a = d.add_node(FlowchartKind::Process, a, "A");
            let b = d.add_node(FlowchartKind::Decision, b, "B");
            d.connect(a, b, "go").unwrap();
            JsonCodec::new(kind).encode(&d)
        }
        "database.schema" => {
            let mut d = op_core::database::Schema::new("Clipboard");
            let a = d.add_table(Pos::new(180.0, 160.0));
            let b = d.add_table(Pos::new(500.0, 160.0));
            d.add_foreign_key(a, b).unwrap();
            JsonCodec::new(kind).encode(&d)
        }
        _ => unreachable!(),
    }
}
fn snapshot(h: &Harness<'static, OpenProcessApp>) -> Document {
    let StoredDocument::Editable(d) = h.state().docs[h.state().active].snapshot() else {
        unreachable!()
    };
    d
}
fn nodes(d: &Document) -> Vec<serde_json::Value> {
    ["nodes", "places", "transitions", "tables"]
        .into_iter()
        .flat_map(|field| d.model[field].as_array().cloned().unwrap_or_default())
        .collect()
}
fn edges(d: &Document) -> Vec<serde_json::Value> {
    ["flows", "arcs", "relations", "foreign_keys"]
        .into_iter()
        .flat_map(|field| d.model[field].as_array().cloned().unwrap_or_default())
        .collect()
}
fn setup(kind: &'static str) -> Harness<'static, OpenProcessApp> {
    let mut h = harness(false);
    h.state_mut().open_document(document(kind), None, true);
    h.run_steps(5);
    let active = h.state().active;
    let mut views: Vec<(u64, serde_json::Value)> = serde_json::from_value(h.state().docs[active].editor.workspace()).unwrap();
    for (_, state) in &mut views {
        state["camera"] = serde_json::to_value(crate::canvas::View {
            zoom: 1.0,
            offset: egui::vec2(80.0, 80.0),
        })
        .unwrap();
    }
    h.state_mut().docs[active]
        .editor
        .restore_workspace(&serde_json::to_value(views).unwrap());
    h.run_steps(3);
    let p = screen(&h, first_center(&snapshot(&h)));
    h.hover_at(p);
    h.run_steps(2);
    h
}
fn first_center(d: &Document) -> Pos {
    let n = &nodes(d)[0];
    if n["pos"].is_object() {
        serde_json::from_value(n["pos"].clone()).unwrap()
    } else {
        serde_json::from_value::<op_core::geom::Rect>(n["bounds"].clone()).unwrap().center()
    }
}
fn screen(h: &Harness<'static, OpenProcessApp>, p: Pos) -> egui::Pos2 {
    crate::canvas::View {
        zoom: 1.0,
        offset: egui::vec2(80.0, 80.0),
    }
    .to_screen(h.state().docs[h.state().active].editor.thumbnail_rect().unwrap(), p)
}
fn shortcut(h: &mut Harness<'static, OpenProcessApp>, k: Key) {
    h.key_press_modifiers(Modifiers::COMMAND, k);
    h.run_steps(3);
}
fn copied(h: &mut Harness<'static, OpenProcessApp>, event: egui::Event) -> String {
    h.event(event);
    h.run_steps(1);
    h.output()
        .platform_output
        .commands
        .iter()
        .find_map(|c| match c {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .expect("selection writes system clipboard")
}
fn pointer(h: &mut Harness<'static, OpenProcessApp>, p: egui::Pos2, down: bool, modifiers: Modifiers) {
    h.event(egui::Event::ModifiersChanged(modifiers));
    h.event(egui::Event::PointerButton {
        pos: p,
        button: egui::PointerButton::Primary,
        pressed: down,
        modifiers,
    });
    h.run_steps(2);
}

#[test]
fn every_graphical_builder_copies_cuts_pastes_across_documents_and_has_edit_commands() {
    for kind in KINDS {
        let mut h = setup(kind);
        let original = snapshot(&h);
        let source = h.state().active;
        let commands = h.state().docs[source].editor.commands();
        for id in [COPY, CUT, PASTE, DUPLICATE, SELECT_ALL] {
            assert!(commands.iter().any(|c| c.id == id), "{kind}: {id}");
        }
        shortcut(&mut h, Key::A);
        let text = copied(&mut h, egui::Event::Copy);
        assert_eq!(snapshot(&h), original);
        h.event(egui::Event::Paste(text.clone()));
        h.run_steps(3);
        assert_eq!(nodes(&snapshot(&h)).len(), 4, "{kind}: paste group");
        assert_eq!(edges(&snapshot(&h)).len(), 2, "{kind}: internal links");
        h.state().docs[source].editor.validate_storage().unwrap();
        shortcut(&mut h, Key::Z);
        assert_eq!(snapshot(&h), original, "{kind}: one-step undo");
        shortcut(&mut h, Key::A);
        let cut = copied(&mut h, egui::Event::Cut);
        assert_eq!(nodes(&snapshot(&h)).len(), 0, "{kind}: cut");
        shortcut(&mut h, Key::Z);
        assert_eq!(snapshot(&h), original);
        h.state_mut().open_document(document(kind), None, true);
        h.run_steps(5);
        h.hover_at(screen(&h, first_center(&snapshot(&h))));
        h.run_steps(2);
        h.event(egui::Event::Paste(cut));
        h.run_steps(3);
        assert_eq!(nodes(&snapshot(&h)).len(), 4, "{kind}: cross-document paste");
        assert_eq!(edges(&snapshot(&h)).len(), 2);
        assert_eq!(h.state().docs[source].snapshot(), StoredDocument::Editable(original));
        h.event(egui::Event::Paste("ordinary text".into()));
        h.run_steps(3);
        assert_eq!(nodes(&snapshot(&h)).len(), 4, "{kind}: unrelated clipboard text");
    }
}

#[test]
fn every_graphical_builder_alt_drag_preserves_originals_and_is_one_cancelable_transaction() {
    for kind in KINDS {
        let mut h = setup(kind);
        let original = snapshot(&h);
        shortcut(&mut h, Key::A);
        let from = screen(&h, first_center(&original));
        let to = from + egui::vec2(70.0, 90.0);
        pointer(&mut h, from, true, Modifiers::ALT);
        for i in 1..=6 {
            h.hover_at(from.lerp(to, i as f32 / 6.0));
            h.run_steps(1);
        }
        pointer(&mut h, to, false, Modifiers::ALT);
        h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
        h.run_steps(3);
        let duplicated = snapshot(&h);
        assert_eq!(nodes(&duplicated).len(), 4, "{kind}: Alt-drag duplicates group once");
        assert_eq!(edges(&duplicated).len(), 2, "{kind}: keeps internal links");
        for node in nodes(&original) {
            assert!(nodes(&duplicated).contains(&node), "{kind}: original unchanged");
        }
        shortcut(&mut h, Key::Z);
        assert_eq!(snapshot(&h), original, "{kind}: duplication and movement undo together");
        shortcut(&mut h, Key::A);
        pointer(&mut h, from, true, Modifiers::ALT);
        h.hover_at(to);
        h.run_steps(3);
        assert_eq!(nodes(&snapshot(&h)).len(), 4, "{kind}: live duplication");
        key(&mut h, Key::Escape);
        pointer(&mut h, to, false, Modifiers::NONE);
        h.run_steps(3);
        assert_eq!(snapshot(&h), original, "{kind}: Escape cancels duplication and movement");
    }
}

#[test]
fn graphical_clipboard_leaves_focused_text_fields_in_control() {
    for kind in KINDS {
        let mut h = setup(kind);
        key(&mut h, Key::Escape);
        h.get_by_role(egui::accesskit::Role::TextInput).click();
        h.run_steps(3);
        shortcut(&mut h, Key::A);
        h.hover_at(screen(&h, first_center(&snapshot(&h))));
        h.run_steps(2);
        assert_eq!(copied(&mut h, egui::Event::Copy), "Clipboard", "{kind}: copy focused text");
        h.event(egui::Event::Paste("Renamed".into()));
        h.run_steps(3);
        assert_eq!(snapshot(&h).model["name"], "Renamed", "{kind}: paste into focused text");
        assert_eq!(nodes(&snapshot(&h)).len(), 2);
    }
}

#[test]
fn graphical_edit_and_context_menus_offer_the_shared_selection_actions() {
    for kind in KINDS {
        let mut h = setup(kind);
        let from = screen(&h, first_center(&snapshot(&h)));
        click(&mut h, from);
        h.get_by_label("Edit").click();
        h.run_steps(3);
        for label in ["Copy", "Cut", "Paste", "Select all", "Duplicate"] {
            assert!(h.query_all_by_label(label).next().is_some(), "{kind}: Edit → {label}");
        }
        key(&mut h, Key::Escape);
        h.hover_at(from);
        h.run_steps(2);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: from,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        h.run_steps(3);
        for label in ["Copy", "Cut", "Paste", "Select all", "Duplicate"] {
            assert!(h.query_all_by_label(label).next().is_some(), "{kind}: context menu → {label}");
        }
    }
}

#[test]
fn bpmn_alt_dragging_a_pool_only_moves_the_copy_and_its_contents() {
    use op_core::bpmn::*;
    let mut h = setup("bpmn");
    let mut diagram = BpmnDiagram::new("Pool copy");
    diagram.add_pool(Pos::new(100.0, 100.0));
    let a = diagram.add_node(NodeKind::Task(TaskType::User), Pos::new(260.0, 160.0));
    let b = diagram.add_node(NodeKind::Task(TaskType::Service), Pos::new(460.0, 160.0));
    diagram.add_flow(a, b).unwrap();
    h.state_mut().open_document(BpmnDiagram::CODEC.encode(&diagram), None, true);
    h.run_steps(5);
    // Use the fitted camera rather than changing it while a gesture is pending.
    let editor = bpmn(&h);
    let canvas = h.state().docs[h.state().active].editor.thumbnail_rect().unwrap();
    let from = editor.view.to_screen(canvas, Pos::new(115.0, 145.0));
    let to = from + egui::vec2(100.0, 60.0);
    pointer(&mut h, from, true, Modifiers::ALT);
    for i in 1..=6 {
        h.hover_at(from.lerp(to, i as f32 / 6.0));
        h.run_steps(1);
    }
    pointer(&mut h, to, false, Modifiers::ALT);
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(3);
    let result = &bpmn(&h).diagram;
    assert_eq!((result.pools.len(), result.nodes.len(), result.flows.len()), (2, 4, 2));
    assert_eq!(result.pools[0], diagram.pools[0]);
    assert_eq!(result.nodes[..2], diagram.nodes);
    assert_ne!(result.pools[1].pos, result.pools[0].pos);
    shortcut(&mut h, Key::Z);
    assert_eq!(bpmn(&h).diagram, diagram);
}
