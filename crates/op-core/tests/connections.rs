//! Moving connection ends and merging flows around deleted elements.
use op_core::bpmn::{BpmnDiagram, EventTrigger, FlowKind, NodeKind, TaskType};
use op_core::epc::{EpcDiagram, EpcKind};
use op_core::geom::{Endpoint, Pos};
use op_core::petri::{ArcKind, NodeRef, PetriNet};
use op_core::uml::{DiagramKind, NodeKind as UmlKind, RelationKind, UmlDiagram};
use std::collections::BTreeSet;

fn task(d: &mut BpmnDiagram, x: f32) -> op_core::bpmn::Id {
    d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(x, 100.0))
}

#[test]
fn bpmn_deletion_merges_a_single_incoming_and_outgoing_sequence_flow() {
    let mut d = BpmnDiagram::new("chain");
    let start = d.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::new(0.0, 100.0));
    let (a, b, c) = (task(&mut d, 150.0), task(&mut d, 300.0), task(&mut d, 450.0));
    let first = d.add_flow(start, a).unwrap();
    let kept = d.add_flow(a, b).unwrap();
    d.add_flow(b, c).unwrap();
    d.flow_mut(kept).unwrap().name = "approved".into();
    let note = d.add_node(NodeKind::TextAnnotation, Pos::new(300.0, 0.0));
    d.add_flow(b, note).unwrap();

    assert_eq!(d.remove_nodes_bridging(&[b]), [kept]);
    let merged = d.flow(kept).unwrap();
    assert_eq!((merged.from, merged.to, merged.name.as_str()), (a, c, "approved"));
    assert_eq!(d.flows.len(), 2, "the outgoing flow and the association are gone");
    assert_eq!(d.flows[0].id, first, "flow order is kept");

    // Two incoming flows: nothing is merged.
    let d2 = d.clone();
    let mut d = d2;
    let x = task(&mut d, 150.0);
    d.add_flow(x, c).unwrap();
    let end = d.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::new(600.0, 100.0));
    d.add_flow(c, end).unwrap();
    assert!(d.remove_nodes_bridging(&[c]).is_empty());
    assert!(d.flows.iter().all(|f| f.from != c && f.to != c));

    // A merge that would duplicate an existing flow is skipped.
    let mut d = BpmnDiagram::new("triangle");
    let (p, q, r) = (task(&mut d, 0.0), task(&mut d, 150.0), task(&mut d, 300.0));
    d.add_flow(p, q).unwrap();
    d.add_flow(q, r).unwrap();
    d.add_flow(p, r).unwrap();
    assert!(d.remove_nodes_bridging(&[q]).is_empty());
    assert_eq!(d.flows.len(), 1);

    // Linked elements removed together merge as one block: p → (q → r) → s.
    let mut d = BpmnDiagram::new("block");
    let ids: Vec<_> = (0..4).map(|i| task(&mut d, i as f32 * 150.0)).collect();
    let entering = d.add_flow(ids[0], ids[1]).unwrap();
    d.add_flow(ids[1], ids[2]).unwrap();
    d.add_flow(ids[2], ids[3]).unwrap();
    assert_eq!(d.remove_nodes_bridging(&[ids[2], ids[1]]), [entering]);
    assert_eq!((d.flows.len(), d.flow(entering).unwrap().to), (1, ids[3]));
}

#[test]
fn bpmn_reconnect_validates_like_a_new_flow_and_may_change_its_kind() {
    let mut d = BpmnDiagram::new("pools");
    let one = d.add_pool(Pos::new(0.0, 0.0));
    let two = d.add_pool(Pos::new(0.0, 400.0));
    d.add_lane(one);
    d.add_lane(two);
    let a = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(200.0, 70.0));
    let b = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(400.0, 70.0));
    let other = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(400.0, 470.0));
    let start = d.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::new(100.0, 70.0));
    let flow = d.add_flow(a, b).unwrap();
    d.flow_mut(flow).unwrap().is_default = true;
    assert_eq!(d.flow(flow).unwrap().kind, FlowKind::Sequence);

    assert!(
        d.reconnect(flow, Endpoint::Target, start).is_err(),
        "start events take no incoming flow"
    );
    assert!(d.reconnect(flow, Endpoint::Target, a).is_err(), "no self connection");
    assert_eq!(d.flow(flow).unwrap().to, b, "a refused change leaves the flow unchanged");

    d.reconnect(flow, Endpoint::Target, other).unwrap();
    let moved = d.flow(flow).unwrap();
    assert_eq!(
        (moved.from, moved.to, moved.kind, moved.is_default),
        (a, other, FlowKind::Message, false)
    );
    let x = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(600.0, 470.0));
    d.reconnect(flow, Endpoint::Target, x).unwrap();
    d.reconnect(flow, Endpoint::Source, other).unwrap();
    assert_eq!(d.flow(flow).unwrap().kind, FlowKind::Sequence, "same pool again");
    assert_eq!(d.flows.len(), 1);
}

#[test]
fn flow_diagrams_merge_chains_only_where_the_notation_allows_it() {
    let mut d = EpcDiagram::new("epc");
    let ids: Vec<_> = [EpcKind::Event, EpcKind::Function, EpcKind::Event, EpcKind::Function, EpcKind::Event]
        .into_iter()
        .enumerate()
        .map(|(i, kind)| d.add_node(kind, Pos::new(i as f32 * 200.0, 0.0), format!("n{i}")))
        .collect();
    for pair in ids.windows(2) {
        d.connect(pair[0], pair[1], "").unwrap();
    }
    // Event → (function) → event would join two events: EPC forbids it.
    let mut single = d.clone();
    single.delete_bridging(&BTreeSet::from([ids[1]]), None);
    assert!(!single.flows.iter().any(|f| f.from == ids[0] && f.to == ids[2]));
    // Removing a function and the following event joins event → function.
    d.delete_bridging(&BTreeSet::from([ids[1], ids[2]]), None);
    assert!(d.flows.iter().any(|f| f.from == ids[0] && f.to == ids[3]));
    assert_eq!(d.flows.len(), 2);
    assert_eq!(d.nodes.len(), 3);

    // Reconnecting follows the same rules.
    let first = d.flows.iter().find(|f| f.from == ids[0]).unwrap().id;
    assert!(!d.reconnect(first, Endpoint::Target, ids[4]), "event → event");
    assert!(!d.reconnect(first, Endpoint::Target, ids[0]), "self connection");
    assert!(d.reconnect(first, Endpoint::Source, ids[4]));
    let moved = d.flows.iter().find(|f| f.id == first).unwrap();
    assert_eq!((moved.from, moved.to), (ids[4], ids[3]));
}

#[test]
fn petri_arcs_keep_alternating_and_inhibitors_keep_leaving_places() {
    let mut net = PetriNet::default();
    let (p1, p2) = (net.add_place(Pos::new(0.0, 0.0), 1), net.add_place(Pos::new(200.0, 0.0), 0));
    let (t1, t2) = (net.add_transition(Pos::new(100.0, 0.0)), net.add_transition(Pos::new(100.0, 100.0)));
    let arc = net.add_arc(NodeRef::Place(p1), NodeRef::Transition(t1)).unwrap();
    net.add_arc(NodeRef::Place(p2), NodeRef::Transition(t2)).unwrap();
    assert!(net.reconnect_arc(arc, Endpoint::Target, NodeRef::Place(p2)).is_err());
    assert!(net.reconnect_arc(arc, Endpoint::Source, NodeRef::Place(p2)).is_ok());
    assert!(
        net.reconnect_arc(arc, Endpoint::Target, NodeRef::Transition(t2)).is_err(),
        "duplicate arc"
    );
    net.arc_mut(arc).unwrap().kind = ArcKind::Inhibitor;
    assert!(net.reconnect_arc(arc, Endpoint::Source, NodeRef::Transition(t2)).is_err());
    net.reconnect_arc(arc, Endpoint::Source, NodeRef::Place(p1)).unwrap();
    let moved = net.arc(arc).unwrap();
    assert_eq!(
        (moved.from, moved.to, moved.kind),
        (NodeRef::Place(p1), NodeRef::Transition(t1), ArcKind::Inhibitor)
    );
}

#[test]
fn uml_relations_reconnect_within_their_kind_rules() {
    let mut d = UmlDiagram::new("classes", DiagramKind::Class);
    let a = d.add_node(UmlKind::Class, Pos::default(), "A").unwrap();
    let b = d.add_node(UmlKind::Class, Pos::default(), "B").unwrap();
    let c = d.add_node(UmlKind::Class, Pos::default(), "C").unwrap();
    let generalization = d.connect(a, b, RelationKind::Generalization).unwrap();
    d.connect(b, c, RelationKind::Generalization).unwrap();
    assert!(!d.reconnect(generalization, Endpoint::Source, c), "inheritance cycle");
    assert!(d.reconnect(generalization, Endpoint::Target, c));
    let r = d.relations.iter().find(|r| r.id == generalization).unwrap();
    assert_eq!((r.from, r.to), (a, c));
}
