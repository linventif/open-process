use super::*;
use op_core::document::DocumentCodec;
use op_core::geom::Port;
use op_core::whiteboard::{Attachment, Board, Element, Fragment, Kind, Paint};

fn setup() -> Harness<'static, OpenProcessApp> {
    let mut board = Board::new("Drawing shortcuts");
    let a = board.add(Element::new(
        Kind::Rectangle,
        Pos::new(200.0, 180.0),
        Pos::new(360.0, 280.0),
        Paint::default(),
    ));
    let b = board.add(Element::new(
        Kind::Ellipse,
        Pos::new(500.0, 180.0),
        Pos::new(660.0, 280.0),
        Paint::default(),
    ));
    let mut arrow = Element::new(Kind::Arrow, Pos::new(360.0, 230.0), Pos::new(500.0, 230.0), Paint::default());
    arrow.start_attachment = Some(Attachment {
        element: a,
        port: Port::Right,
    });
    arrow.end_attachment = Some(Attachment {
        element: b,
        port: Port::Left,
    });
    board.add(arrow);
    board.add(
        Element::freehand(
            &[Pos::new(300.0, 400.0), Pos::new(320.0, 440.0), Pos::new(350.0, 420.0)],
            Paint::default(),
        )
        .unwrap(),
    );
    setup_board(board)
}

fn setup_board(board: Board) -> Harness<'static, OpenProcessApp> {
    let mut h = harness(false);
    h.state_mut().open_document(Board::CODEC.encode(&board), None, true);
    h.run_steps(5);
    let active = h.state().active;
    let mut views: Vec<(u64, serde_json::Value)> = serde_json::from_value(h.state().docs[active].editor.workspace()).unwrap();
    for (_, view) in &mut views {
        view["camera"] = serde_json::to_value(crate::canvas::View {
            zoom: 1.0,
            offset: egui::vec2(100.0, 80.0),
        })
        .unwrap();
    }
    h.state_mut().docs[active]
        .editor
        .restore_workspace(&serde_json::to_value(views).unwrap());
    h.run_steps(3);
    h.hover_at(whiteboard(&h).element_screen_center(1));
    h.run_steps(2);
    h
}

#[test]
fn drawing_advanced_sizes_preserve_custom_values_and_edit_exact_geometry() {
    use egui::accesskit::Role;
    use egui_kittest::kittest::NodeT;

    let mut board = Board::new("Exact sizes");
    let id = board.add(Element::new(
        Kind::Rectangle,
        Pos::new(200.0, 180.0),
        Pos::new(360.25, 280.75),
        Paint {
            width: 2.750123,
            font_size: 23.512345,
            ..Paint::default()
        },
    ));
    let mut h = setup_board(board.clone());
    let center = whiteboard(&h).element_screen_center(id);
    click(&mut h, center);
    assert!(h.query_by_label("Stroke width: XS").is_some());
    assert!(h.query_by_label("Font size: XL").is_some());
    assert_eq!(h.query_all_by_label("Custom").count(), 2);
    assert!(h.query_by_role_and_label(Role::SpinButton, "Font size").is_none());
    assert!(h.query_by_label("Width").is_none());
    shot(&mut h, "whiteboard_sizes_simple");

    h.get_by_label("Advanced").click();
    h.run_steps(5);
    for (label, value) in [
        ("Stroke width", 2.750123_f32),
        ("Font size", 23.512345_f32),
        ("Width", 160.25),
        ("Height", 100.75),
    ] {
        assert_eq!(
            h.get_by_role_and_label(Role::SpinButton, label).accesskit_node().numeric_value(),
            Some(f64::from(value)),
            "{label} shows the stored value"
        );
        assert_eq!(h.get_by_role_and_label(Role::SpinButton, label).value(), Some(value.to_string()));
    }
    h.get_by_role_and_label(Role::SpinButton, "Font size").focus();
    h.run_steps(3);
    h.get_by_role_and_label(Role::TextInput, "Name").focus();
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board, board, "opening Advanced must preserve custom values");
    assert!(!h.state().docs[h.state().active].editor.enabled(UNDO));
    shot(&mut h, "whiteboard_sizes_advanced");
    h.get_by_role_and_label(Role::SpinButton, "Font size").set_value_accesskit(27.5);
    h.run_steps(3);
    h.get_by_role_and_label(Role::SpinButton, "Width").set_value_accesskit(225.5);
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board.element(id).unwrap().paint.font_size, 27.5);
    assert_eq!(whiteboard(&h).board.element(id).unwrap().bounds.w, 225.5);
    assert_eq!(whiteboard(&h).board.element(id).unwrap().bounds.h, 100.75);
    h.get_by_label("Advanced").click();
    h.run_steps(5);
    assert!(h.query_by_role_and_label(Role::SpinButton, "Font size").is_none());
    assert_eq!(h.query_all_by_label("Custom").count(), 2);
    h.get_by_label("Font size: XL").click();
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board.element(id).unwrap().paint.font_size, 48.0);
    assert_eq!(whiteboard(&h).board.element(id).unwrap().paint.width, 2.750123);
    let active = h.state().active;
    h.state_mut().docs[active].editor.execute(UNDO, 0.0);
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board.element(id).unwrap().paint.font_size, 27.5);
    whiteboard(&h).board.validate().unwrap();
}

#[test]
fn drawing_size_preset_unifies_mixed_selection_and_sets_the_next_element_style() {
    let mut board = Board::new("Mixed sizes");
    board.add(Element::new(
        Kind::Rectangle,
        Pos::new(200.0, 180.0),
        Pos::new(360.0, 280.0),
        Paint::default(),
    ));
    board.add(Element::new(
        Kind::Ellipse,
        Pos::new(500.0, 180.0),
        Pos::new(660.0, 280.0),
        Paint {
            width: 8.0,
            font_size: 48.0,
            fill: Some([255, 236, 153]),
            ..Paint::default()
        },
    ));
    let mut h = setup_board(board.clone());
    shortcut(&mut h, Key::A, Modifiers::COMMAND);
    assert_eq!(h.query_all_by_label("Mixed").count(), 2);
    // M is already the first element's size; choosing it still applies to the whole selection.
    h.get_by_label("Font size: M").click();
    h.run_steps(3);
    for (original, element) in board.elements.iter().zip(&whiteboard(&h).board.elements) {
        assert_eq!(element.paint.font_size, 20.0);
        let mut expected = original.clone();
        expected.paint.font_size = 20.0;
        assert_eq!(*element, expected, "other styles and geometry are preserved");
    }
    let active = h.state().active;
    h.state_mut().docs[active].editor.execute(UNDO, 0.0);
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board, board);
    h.get_by_label("Font size: S").click();
    h.run_steps(3);
    h.get_by_label("Stroke width: XS").click();
    h.run_steps(3);
    h.get_by_label("Rectangle").click();
    h.run_steps(3);
    let from = whiteboard(&h).screen_point(Pos::new(200.0, 400.0));
    let to = whiteboard(&h).screen_point(Pos::new(360.0, 500.0));
    drag(&mut h, from, to);
    let element = whiteboard(&h).board.elements.last().unwrap();
    assert_eq!(element.paint.width, 0.5);
    assert_eq!(element.paint.font_size, 16.0);
    whiteboard(&h).board.validate().unwrap();
}

fn shortcut(h: &mut Harness<'static, OpenProcessApp>, key: Key, modifiers: Modifiers) {
    h.key_press_modifiers(modifiers, key);
    h.run_steps(3);
}

fn copied_text(h: &mut Harness<'static, OpenProcessApp>, event: egui::Event) -> String {
    h.event(event);
    h.run_steps(1);
    h.output()
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .expect("copy must write the system clipboard")
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
fn drawing_clipboard_copy_cut_paste_cross_documents_and_undo_keep_connections() {
    let mut h = setup();
    let original = whiteboard(&h).board.clone();
    shortcut(&mut h, Key::A, Modifiers::COMMAND);
    let text = copied_text(&mut h, egui::Event::Copy);
    assert_eq!(whiteboard(&h).board, original);
    assert_eq!(Fragment::from_clipboard(&text).unwrap().unwrap().elements.len(), 4);
    // Native and browser integrations emit Paste, rather than a plain V key event.
    h.event(egui::Event::Paste(text.clone()));
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board.elements.len(), 8);
    whiteboard(&h).board.validate().unwrap();
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(whiteboard(&h).board, original);
    shortcut(&mut h, Key::Z, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(whiteboard(&h).board.elements.len(), 8);
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    shortcut(&mut h, Key::A, Modifiers::COMMAND);
    let cut = copied_text(&mut h, egui::Event::Cut);
    assert!(whiteboard(&h).board.elements.is_empty());
    h.event(egui::Event::Paste(cut));
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board.elements.len(), 4);
    whiteboard(&h).board.validate().unwrap();
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    assert!(whiteboard(&h).board.elements.is_empty());
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(whiteboard(&h).board, original);
    let source = h.state().active;
    h.state_mut()
        .open_document(Board::CODEC.encode(&Board::new("Destination")), None, true);
    h.run_steps(4);
    h.get_by_label("Edit").click();
    h.run_steps(3);
    h.get_by_label("Paste").click();
    h.run_steps(3);
    h.event(egui::Event::Paste(text));
    h.run_steps(4);
    assert_eq!(whiteboard(&h).board.elements.len(), 4);
    whiteboard(&h).board.validate().unwrap();
    assert_eq!(
        h.state().docs[source].snapshot(),
        StoredDocument::Editable(Board::CODEC.encode(&original))
    );
    let arrow = whiteboard(&h).board.elements.iter().find(|e| e.kind == Kind::Arrow).unwrap();
    assert!(arrow.start_attachment.is_some() && arrow.end_attachment.is_some());
    shot(&mut h, "whiteboard_clipboard_pasted");
}

#[test]
fn drawing_alt_drag_duplicates_once_constrains_with_shift_and_cancels_atomically() {
    let mut h = setup();
    let original = whiteboard(&h).board.clone();
    shortcut(&mut h, Key::A, Modifiers::COMMAND);
    let from = whiteboard(&h).element_screen_center(1);
    let to = from + egui::vec2(80.0, 30.0);
    let modifiers = Modifiers::ALT | Modifiers::SHIFT;
    pointer(&mut h, from, true, modifiers);
    for i in 1..=8 {
        h.hover_at(from.lerp(to, i as f32 / 8.0));
        h.run_steps(1);
    }
    assert_eq!(h.output().platform_output.cursor_icon, egui::CursorIcon::Copy);
    pointer(&mut h, to, false, modifiers);
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board.elements.len(), 8);
    for element in &original.elements {
        assert_eq!(whiteboard(&h).board.element(element.id), Some(element));
    }
    let copies: Vec<_> = whiteboard(&h)
        .board
        .elements
        .iter()
        .filter(|e| original.element(e.id).is_none())
        .collect();
    for (source, copy) in original.elements.iter().zip(copies) {
        assert_eq!(copy.bounds.x, source.bounds.x + 80.0);
        assert_eq!(copy.bounds.y, source.bounds.y);
    }
    whiteboard(&h).board.validate().unwrap();
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(whiteboard(&h).board, original);
    assert!(!h.state().docs[h.state().active].editor.enabled(UNDO));
    shortcut(&mut h, Key::A, Modifiers::COMMAND);
    let from = whiteboard(&h).element_screen_center(1);
    pointer(&mut h, from, true, Modifiers::ALT);
    h.hover_at(from + egui::vec2(45.0, 65.0));
    h.run_steps(3);
    assert_eq!(whiteboard(&h).board.elements.len(), 8);
    key(&mut h, Key::Escape);
    pointer(&mut h, from + egui::vec2(45.0, 65.0), false, Modifiers::NONE);
    assert_eq!(whiteboard(&h).board, original);
    assert!(!h.state().docs[h.state().active].dirty());
    assert!(!h.state().docs[h.state().active].editor.enabled(UNDO));
    assert!(h.query_by_label("4 selected").is_some(), "Escape restores the source selection");
    // Alt may also be pressed after a move has already started.
    pointer(&mut h, from, true, Modifiers::NONE);
    h.hover_at(from + egui::vec2(20.0, 10.0));
    h.run_steps(3);
    h.event(egui::Event::ModifiersChanged(Modifiers::ALT));
    h.hover_at(from + egui::vec2(100.0, 60.0));
    h.run_steps(3);
    pointer(&mut h, from + egui::vec2(100.0, 60.0), false, Modifiers::NONE);
    assert_eq!(whiteboard(&h).board.elements.len(), 8);
    for element in &original.elements {
        assert_eq!(whiteboard(&h).board.element(element.id), Some(element));
    }
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(whiteboard(&h).board, original);
}

#[test]
fn drawing_clipboard_respects_text_focus_and_plain_text_becomes_a_text_element() {
    let mut h = setup();
    let count = whiteboard(&h).board.elements.len();
    h.event(egui::Event::Paste("Hello\nworld".into()));
    h.run_steps(4);
    assert_eq!(whiteboard(&h).board.elements.len(), count + 1);
    assert_eq!(whiteboard(&h).board.elements.last().unwrap().text, "Hello\nworld");
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(whiteboard(&h).board.elements.len(), count);
    h.get_by_role_and_label(egui::accesskit::Role::TextInput, "Name").click();
    h.run_steps(3);
    shortcut(&mut h, Key::A, Modifiers::COMMAND);
    // Moving the pointer back over the canvas must not redirect the focused field's clipboard events.
    h.hover_at(whiteboard(&h).element_screen_center(1));
    h.run_steps(2);
    let text = copied_text(&mut h, egui::Event::Copy);
    assert_eq!(text, "Drawing shortcuts");
    h.event(egui::Event::Paste("Edited name".into()));
    h.run_steps(4);
    assert_eq!(whiteboard(&h).board.name, "Edited name");
    assert_eq!(whiteboard(&h).board.elements.len(), count);
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(whiteboard(&h).board.name, "Drawing shortcuts", "text fields retain their own undo");
}

#[test]
fn drawing_edit_menu_context_menu_and_layer_shortcuts_are_available() {
    let mut h = setup();
    let from = whiteboard(&h).element_screen_center(1);
    click(&mut h, from);
    h.get_by_label("Edit").click();
    h.run_steps(3);
    for label in ["Copy", "Cut", "Paste", "Select all", "Duplicate selection"] {
        assert!(h.query_all_by_label(label).next().is_some(), "Edit: {label}");
    }
    key(&mut h, Key::Escape);
    h.hover_at(from);
    h.run_steps(2);
    shortcut(&mut h, Key::CloseBracket, Modifiers::COMMAND | Modifiers::SHIFT);
    assert_eq!(whiteboard(&h).board.elements.last().unwrap().id, 1);
    shortcut(&mut h, Key::Z, Modifiers::COMMAND);
    assert_eq!(whiteboard(&h).board.elements.first().unwrap().id, 1);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: from,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    h.run_steps(3);
    assert!(h.query_by_label("Arrange selection").is_some());
    assert!(h.query_all_by_label("Duplicate selection").next().is_some());
    shot(&mut h, "whiteboard_context_editing");
    key(&mut h, Key::Escape);
    h.hover_at(from);
    h.run_steps(2);
    shortcut(&mut h, Key::D, Modifiers::COMMAND);
    assert_eq!(whiteboard(&h).board.elements.len(), 5);
    shortcut(&mut h, Key::R, Modifiers::NONE);
    let start = whiteboard(&h).screen_point(Pos::new(200.0, 500.0));
    let end = whiteboard(&h).screen_point(Pos::new(280.0, 530.0));
    h.hover_at(start);
    h.run_steps(2);
    pointer(&mut h, start, true, Modifiers::SHIFT);
    h.hover_at(end);
    h.run_steps(3);
    pointer(&mut h, end, false, Modifiers::SHIFT);
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    let shape = whiteboard(&h).board.elements.last().unwrap();
    assert_eq!(shape.kind, Kind::Rectangle);
    assert_eq!(shape.bounds.w, shape.bounds.h, "Shift draws equal-sided shapes");
    h.get_by_label("Snap to grid").click();
    h.run_steps(3);
    h.hover_at(start);
    shortcut(&mut h, Key::P, Modifiers::NONE);
    let start = whiteboard(&h).screen_point(Pos::new(420.0, 500.0));
    let end = whiteboard(&h).screen_point(Pos::new(431.0, 507.0));
    h.hover_at(start);
    h.run_steps(2);
    pointer(&mut h, start, true, Modifiers::SHIFT);
    h.hover_at(end);
    h.run_steps(3);
    pointer(&mut h, end, false, Modifiers::SHIFT);
    h.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    h.run_steps(2);
    let stroke = whiteboard(&h).board.elements.last().unwrap();
    assert_eq!(stroke.kind, Kind::Freehand);
    assert_eq!(
        *stroke.world_points().last().unwrap(),
        Pos::new(431.0, 507.0),
        "pencil samples remain free even with Shift and snapping"
    );
    shortcut(&mut h, Key::V, Modifiers::NONE);
    let center = whiteboard(&h).element_screen_center(1);
    click(&mut h, center);
    shortcut(&mut h, Key::F2, Modifiers::NONE);
    shot(&mut h, "whiteboard_text_editing");
    assert!(h.get_by_label("Element text").is_focused());
}
