use super::*;
use op_core::bpmn::{NodeKind, TaskType};
use op_core::petri::{NodeRef, PetriNet};

fn canvas_of(h: &Harness<'static, OpenProcessApp>) -> egui::Rect {
    h.state().docs[h.state().active].editor.thumbnail_rect().unwrap()
}

#[test]
fn bpmn_flow_ends_move_by_dragging_and_deletion_merges_flows() {
    let mut d = BpmnDiagram::new("Reconnect");
    let task = |d: &mut BpmnDiagram, x, y| d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(x, y));
    let (a, b, c) = (task(&mut d, 100.0, 100.0), task(&mut d, 400.0, 100.0), task(&mut d, 400.0, 320.0));
    let flow = d.add_flow(a, b).unwrap();
    let mut h = harness_sized(false, [1200.0, 700.0]);
    h.state_mut().open_model(Model::Bpmn(d), None, false);
    h.run_steps(4);

    let middle = bpmn_screen(&h, Pos::new(250.0, 100.0));
    click(&mut h, middle);
    let (_, path) = bpmn(&h).selected_flow_path(canvas_of(&h)).expect("the flow is selected");
    shot(&mut h, "reconnect_handles");
    let target = bpmn_screen(&h, Pos::new(400.0, 320.0));
    drag(&mut h, *path.last().unwrap(), target);
    h.run_steps(3);
    assert_eq!(bpmn(&h).diagram.flow(flow).unwrap().to, c);
    let i = h.state().active;
    h.state_mut().docs[i].editor.execute(UNDO, 0.0);
    h.run_steps(2);
    assert_eq!(bpmn(&h).diagram.flow(flow).unwrap().to, b, "one undoable step");
    h.state_mut().docs[i].editor.execute(REDO, 0.0);
    h.run_steps(2);

    // a → c → d: deleting c leaves a single flow a → d.
    let mut d2 = bpmn(&h).diagram.clone();
    let e = task(&mut d2, 700.0, 320.0);
    d2.add_flow(c, e).unwrap();
    h.state_mut().open_model(Model::Bpmn(d2), None, false);
    h.run_steps(4);
    let removed = bpmn_screen(&h, Pos::new(400.0, 320.0));
    click(&mut h, removed);
    key(&mut h, Key::Delete);
    h.run_steps(2);
    let flows = &bpmn(&h).diagram.flows;
    assert_eq!(flows.len(), 1);
    assert_eq!((flows[0].id, flows[0].from, flows[0].to), (flow, a, e));
}

#[test]
fn petri_arc_ends_move_by_dragging_and_stay_bipartite() {
    let mut net = PetriNet::default();
    let p = net.add_place(Pos::new(100.0, 100.0), 1);
    let t1 = net.add_transition(Pos::new(400.0, 100.0));
    let t2 = net.add_transition(Pos::new(400.0, 320.0));
    let p2 = net.add_place(Pos::new(700.0, 320.0), 0);
    let arc = net.add_arc(NodeRef::Place(p), NodeRef::Transition(t1)).unwrap();
    let mut h = harness_sized(false, [1200.0, 700.0]);
    h.state_mut().open_model(Model::Petri(net), None, false);
    h.run_steps(4);
    let screen = |h: &Harness<'static, OpenProcessApp>, x, y| petri(h).screen_pos(Pos::new(x, y));

    let middle = screen(&h, 250.0, 100.0);
    click(&mut h, middle);
    let (_, path) = petri(&h).selected_arc_path(canvas_of(&h)).expect("the arc is selected");
    // A place cannot follow a place: the arc is unchanged and the reason is shown.
    let place = screen(&h, 700.0, 320.0);
    drag(&mut h, *path.last().unwrap(), place);
    h.run_steps(3);
    assert_eq!(petri(&h).net.arc(arc).unwrap().to, NodeRef::Transition(t1));
    assert!(!petri(&h).status.is_empty());
    let (_, path) = petri(&h).selected_arc_path(canvas_of(&h)).unwrap();
    let transition = screen(&h, 400.0, 320.0);
    drag(&mut h, *path.last().unwrap(), transition);
    h.run_steps(3);
    assert_eq!(petri(&h).net.arc(arc).unwrap().to, NodeRef::Transition(t2));
    let _ = p2;
}
