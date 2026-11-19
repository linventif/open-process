use super::*;
use op_core::document::{DocumentCodec, JsonCodec};
use op_core::geom::{Rect as Bounds, ResizeEdge};

fn document(kind: &'static str) -> Document {
    let bounds = Bounds::new(200.0, 180.0, 200.0, 160.0);
    match kind {
        "bpmn" => {
            let mut d = BpmnDiagram::new("Resize");
            let id = d.add_node(op_core::bpmn::NodeKind::Task(op_core::bpmn::TaskType::User), bounds.center());
            let n = d.node_mut(id).unwrap();
            n.w = bounds.w;
            n.h = bounds.h;
            JsonCodec::new(kind).encode(&d)
        }
        "uml.class" | "uml.use-case" => {
            use op_core::uml::{DiagramKind, NodeKind, UmlDiagram};
            let mut d = UmlDiagram::new(
                "Resize",
                if kind == "uml.class" {
                    DiagramKind::Class
                } else {
                    DiagramKind::UseCase
                },
            );
            let id = d
                .add_node(
                    if kind == "uml.class" { NodeKind::Class } else { NodeKind::UseCase },
                    bounds.center(),
                    "Element",
                )
                .unwrap();
            d.nodes.iter_mut().find(|n| n.id == id).unwrap().bounds = bounds;
            JsonCodec::new(kind).encode(&d)
        }
        "epc" => {
            let mut d = op_core::epc::EpcDiagram::new("Resize");
            let id = d.add_node(op_core::epc::EpcKind::Function, bounds.center(), "Element");
            d.nodes.iter_mut().find(|n| n.id == id).unwrap().bounds = bounds;
            JsonCodec::new(kind).encode(&d)
        }
        "uml.activity" => {
            let mut d = op_core::activity::ActivityDiagram::new("Resize");
            let id = d.add_node(op_core::activity::ActivityKind::Action, bounds.center(), "Element");
            d.nodes.iter_mut().find(|n| n.id == id).unwrap().bounds = bounds;
            JsonCodec::new(kind).encode(&d)
        }
        "database.schema" => {
            let mut d = op_core::database::Schema::new("Resize");
            let id = d.add_table(Pos::new(bounds.x, bounds.y));
            d.tables.iter_mut().find(|t| t.id == id).unwrap().bounds = bounds;
            JsonCodec::new(kind).encode(&d)
        }
        "whiteboard" => {
            use op_core::whiteboard::{Board, Element, Kind, Paint};
            let mut d = Board::new("Resize");
            d.add(Element::new(
                Kind::Rectangle,
                Pos::new(bounds.x, bounds.y),
                Pos::new(bounds.right(), bounds.bottom()),
                Paint::default(),
            ));
            Board::CODEC.encode(&d)
        }
        _ => unreachable!(),
    }
}

fn bounds(h: &Harness<'static, OpenProcessApp>) -> Bounds {
    let StoredDocument::Editable(d) = h.state().docs[h.state().active].snapshot() else {
        unreachable!()
    };
    match d.kind.as_str() {
        "bpmn" => JsonCodec::<BpmnDiagram>::new("bpmn").decode(&d).unwrap().nodes[0].rect(),
        "uml.class" | "uml.use-case" => {
            JsonCodec::<op_core::uml::UmlDiagram>::new(if d.kind == "uml.class" { "uml.class" } else { "uml.use-case" })
                .decode(&d)
                .unwrap()
                .nodes[0]
                .rect()
        }
        "epc" => JsonCodec::<op_core::epc::EpcDiagram>::new("epc").decode(&d).unwrap().nodes[0].bounds,
        "uml.activity" => {
            JsonCodec::<op_core::activity::ActivityDiagram>::new("uml.activity")
                .decode(&d)
                .unwrap()
                .nodes[0]
                .bounds
        }
        "database.schema" => JsonCodec::<op_core::database::Schema>::new("database.schema")
            .decode(&d)
            .unwrap()
            .tables[0]
            .rect(),
        "whiteboard" => op_core::whiteboard::Board::CODEC.decode(&d).unwrap().elements[0].bounds,
        _ => unreachable!(),
    }
}

fn setup(kind: &'static str, dark: bool, zoom: f32) -> Harness<'static, OpenProcessApp> {
    let mut h = harness(dark);
    h.state_mut().open_document(document(kind), None, true);
    h.run_steps(5);
    let active = h.state().active;
    let camera = serde_json::to_value(crate::canvas::View {
        zoom,
        offset: egui::vec2(100.0, 80.0),
    })
    .unwrap();
    let mut views: Vec<(u64, serde_json::Value)> = serde_json::from_value(h.state().docs[active].editor.workspace()).unwrap();
    for (_, state) in &mut views {
        state["camera"] = camera.clone();
        state["snap"] = true.into();
    }
    h.state_mut().docs[active]
        .editor
        .restore_workspace(&serde_json::to_value(views).unwrap());
    h.run_steps(3);
    h
}

fn screen(h: &Harness<'static, OpenProcessApp>, position: Pos, zoom: f32) -> egui::Pos2 {
    let rect = h.state().docs[h.state().active].editor.thumbnail_rect().unwrap();
    crate::canvas::View {
        zoom,
        offset: egui::vec2(100.0, 80.0),
    }
    .to_screen(rect, position)
}

#[test]
fn every_resizable_editor_shares_border_cursors_and_atomic_resize_gestures() {
    use ResizeEdge::*;
    for (index, kind) in [
        "bpmn",
        "uml.class",
        "uml.use-case",
        "epc",
        "uml.activity",
        "database.schema",
        "whiteboard",
    ]
    .iter()
    .enumerate()
    {
        let zoom = if index % 2 == 0 { 1.0 } else { 0.75 };
        let mut h = setup(kind, index % 2 == 0, zoom);
        let original = h.state().docs[h.state().active].snapshot();
        let initial = bounds(&h);
        for edge in ResizeEdge::ALL {
            let anchor = edge.anchor(initial);
            let offset = egui::vec2(2.0, 2.0);
            let from = screen(&h, anchor, zoom) + offset;
            h.hover_at(from);
            h.run_steps(3);
            let expected_cursor = match edge {
                Left | Right => egui::CursorIcon::ResizeHorizontal,
                Top | Bottom => egui::CursorIcon::ResizeVertical,
                TopLeft | BottomRight => egui::CursorIcon::ResizeNwSe,
                TopRight | BottomLeft => egui::CursorIcon::ResizeNeSw,
            };
            assert_eq!(h.output().platform_output.cursor_icon, expected_cursor, "{kind}: {edge:?}");
            assert_eq!(h.state().docs[h.state().active].snapshot(), original);
            let delta = Pos::new(
                if matches!(edge, Left | TopLeft | BottomLeft) { -40.0 } else { 40.0 },
                if matches!(edge, Top | TopLeft | TopRight) { -40.0 } else { 40.0 },
            );
            let to = screen(&h, anchor + delta, zoom) + offset;
            drag(&mut h, from, to);
            let actual = bounds(&h);
            let expected = Bounds::new(
                if matches!(edge, Left | TopLeft | BottomLeft) {
                    initial.x - 40.0
                } else {
                    initial.x
                },
                if matches!(edge, Top | TopLeft | TopRight) {
                    initial.y - 40.0
                } else {
                    initial.y
                },
                if matches!(edge, Left | Right | TopLeft | TopRight | BottomLeft | BottomRight) {
                    initial.w + 40.0
                } else {
                    initial.w
                },
                if matches!(edge, Top | Bottom | TopLeft | TopRight | BottomLeft | BottomRight) {
                    initial.h + 40.0
                } else {
                    initial.h
                },
            );
            for (a, b) in [
                (actual.x, expected.x),
                (actual.y, expected.y),
                (actual.w, expected.w),
                (actual.h, expected.h),
            ] {
                assert!((a - b).abs() < 0.01, "{kind}: {edge:?}, actual {actual:?}, expected {expected:?}");
            }
            let active = h.state().active;
            let changed = h.state().docs[active].snapshot();
            if edge == TopLeft {
                shot(&mut h, &format!("resize_{}", kind.replace('.', "_")));
            }
            h.state_mut().docs[active].editor.execute(UNDO, 0.0);
            h.run_steps(3);
            assert_eq!(h.state().docs[active].snapshot(), original, "{kind}: undo");
            assert!(!h.state().docs[active].editor.enabled(UNDO));
            h.state_mut().docs[active].editor.execute(REDO, 0.0);
            h.run_steps(3);
            assert_eq!(h.state().docs[active].snapshot(), changed, "{kind}: redo");
            h.state_mut().docs[active].editor.execute(UNDO, 0.0);
            h.run_steps(3);
        }
        let active = h.state().active;
        let from = screen(&h, ResizeEdge::Top.anchor(initial), zoom);
        let to = screen(&h, Pos::new(initial.center().x, initial.bottom() - 2.0), zoom);
        drag(&mut h, from, to);
        let resized = bounds(&h);
        let minimum = match *kind {
            "bpmn" => 30.0,
            "uml.class" => 102.0,
            "uml.use-case" => 60.0,
            "database.schema" => 67.0,
            "whiteboard" => 1.0,
            _ => 8.0,
        };
        assert!(resized.h >= minimum, "{kind}: respects content/minimum height");
        assert!(
            (resized.bottom() - initial.bottom()).abs() < 0.01,
            "{kind}: top resizing keeps bottom fixed"
        );
        h.state_mut().docs[active].editor.execute(UNDO, 0.0);
        h.run_steps(3);
        assert_eq!(h.state().docs[active].snapshot(), original);

        h.get_by_label("Snap to grid").click();
        h.run_steps(3);
        let from = screen(&h, ResizeEdge::Right.anchor(initial), zoom);
        let to = screen(&h, ResizeEdge::Right.anchor(initial) + Pos::new(17.25, 0.0), zoom);
        drag(&mut h, from, to);
        assert!(
            (bounds(&h).w - initial.w - 17.25).abs() < 0.01,
            "{kind}: free resizing retains fractional sizes"
        );
        h.state_mut().docs[active].editor.execute(UNDO, 0.0);
        h.run_steps(3);
        assert_eq!(h.state().docs[active].snapshot(), original);

        let from = screen(&h, initial.center(), zoom);
        h.hover_at(from);
        h.run_steps(3);
        assert_eq!(h.output().platform_output.cursor_icon, egui::CursorIcon::Default);
        let from = screen(&h, Pos::new(initial.x, initial.y), zoom);
        h.hover_at(from);
        h.run_steps(2);
        press(&mut h, from, true);
        h.hover_at(from - egui::vec2(40.0, 40.0));
        h.run_steps(3);
        assert_ne!(h.state().docs[h.state().active].snapshot(), original);
        key(&mut h, Key::Escape);
        press(&mut h, from - egui::vec2(40.0, 40.0), false);
        assert_eq!(h.state().docs[h.state().active].snapshot(), original, "{kind}: Escape");
    }
}
