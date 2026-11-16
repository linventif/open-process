use op_core::{bpmn::*, geom::Pos};
use std::collections::BTreeSet;

#[test]
fn bpmn_pool_copy_preserves_lanes_metadata_and_internal_flow_with_fresh_ids() {
    let mut diagram = BpmnDiagram::new("Source");
    let pool = diagram.add_pool(Pos::new(100.0, 100.0));
    diagram.add_lane(pool);
    let a = diagram.add_node(NodeKind::Task(TaskType::User), Pos::new(260.0, 150.0));
    let b = diagram.add_node(NodeKind::Task(TaskType::Service), Pos::new(460.0, 150.0));
    diagram.node_mut(a).unwrap().documentation = "Keep this documentation".into();
    diagram.node_mut(a).unwrap().cost = 12.5;
    let flow = diagram.add_flow(a, b).unwrap();
    diagram.flow_mut(flow).unwrap().name = "Approved".into();
    diagram.flow_mut(flow).unwrap().is_default = true;
    let outside = diagram.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(1000.0, 150.0));
    diagram.add_flow(b, outside).unwrap();
    let original = diagram.clone();
    let fragment = diagram.fragment(&[], &[pool]);
    let (nodes, pools) = diagram.paste(&fragment, Pos::new(0.0, 400.0)).unwrap();
    assert_eq!((nodes.len(), pools.len()), (2, 1));
    assert_eq!(diagram.flows.len(), 3);
    assert_eq!(diagram.node(nodes[0]).unwrap().documentation, "Keep this documentation");
    assert_eq!(diagram.node(nodes[0]).unwrap().cost, 12.5);
    assert_eq!(diagram.flows.last().unwrap().name, "Approved");
    assert!(diagram.flows.last().unwrap().is_default);
    assert_eq!(diagram.pool(pools[0]).unwrap().lanes.len(), 2);
    for node in &original.nodes {
        assert_eq!(diagram.node(node.id), Some(node));
    }
    diagram.check_storage().unwrap();
    // Invalid external clipboard data fails atomically.
    let before = diagram.clone();
    let mut invalid = fragment;
    invalid.flows[0].from = u32::MAX;
    assert!(diagram.paste(&invalid, Pos::default()).is_err());
    assert_eq!(diagram, before);
}

#[test]
fn petri_copy_preserves_marking_capacity_and_inhibitor_weight() {
    use op_core::petri::*;
    let mut net = PetriNet::new("Net");
    let p = net.add_place(Pos::new(100.0, 100.0), 4);
    net.place_mut(p).unwrap().capacity = Some(8);
    let t = net.add_transition(Pos::new(300.0, 100.0));
    net.transition_mut(t).unwrap().silent = true;
    let a = net.add_arc(NodeRef::Place(p), NodeRef::Transition(t)).unwrap();
    let arc = net.arc_mut(a).unwrap();
    arc.kind = ArcKind::Inhibitor;
    arc.weight = 7;
    let fragment = net.fragment(&[NodeRef::Place(p), NodeRef::Transition(t)]);
    let mut destination = PetriNet::new("Other net");
    destination.add_place(Pos::default(), 0);
    let nodes = destination.paste(&fragment, Pos::new(50.0, 60.0)).unwrap();
    assert_eq!(destination.place(nodes[0].id()).unwrap().capacity, Some(8));
    assert_eq!(destination.place(nodes[0].id()).unwrap().tokens, 4);
    assert!(destination.transition(nodes[1].id()).unwrap().silent);
    assert_eq!(destination.arcs[0].kind, ArcKind::Inhibitor);
    assert_eq!(destination.arcs[0].weight, 7);
    assert_eq!(destination.arcs[0].from, nodes[0]);
    assert_eq!(destination.arcs[0].to, nodes[1]);
    destination.check_storage().unwrap();
}

#[test]
fn flow_copy_keeps_labels_and_omits_external_edges() {
    use op_core::activity::*;
    let mut diagram = ActivityDiagram::new("Activities");
    let a = diagram.add_node(ActivityKind::Action, Pos::new(100.0, 100.0), "A");
    let b = diagram.add_node(ActivityKind::Action, Pos::new(300.0, 100.0), "B");
    let c = diagram.add_node(ActivityKind::Action, Pos::new(500.0, 100.0), "C");
    diagram.connect(a, b, "ready").unwrap();
    diagram.connect(b, c, "next").unwrap();
    let fragment = diagram.fragment(&BTreeSet::from([a, b]));
    let ids = diagram.paste(&fragment, Pos::new(0.0, 100.0)).unwrap();
    assert_eq!(diagram.flows.len(), 3);
    let flow = diagram.flows.last().unwrap();
    assert_eq!(flow.label, "ready");
    assert!(ids.contains(&flow.from) && ids.contains(&flow.to));
    diagram.check_storage().unwrap();
}

#[test]
fn database_copy_remaps_columns_indexes_and_foreign_keys_and_rejects_dangling_references() {
    use op_core::database::*;
    let mut schema = example();
    let ids = schema.tables.iter().map(|t| t.id).collect();
    let fragment = schema.fragment(&ids);
    let mut destination = Schema::new("Destination");
    destination.add_table(Pos::default());
    let copied = destination.paste(&fragment, Pos::new(30.0, 40.0)).unwrap();
    for key in &destination.foreign_keys {
        assert!(copied.contains(&key.table) && copied.contains(&key.referenced_table));
        assert!(
            key.columns
                .iter()
                .all(|id| destination.table(key.table).unwrap().column(*id).is_some())
        );
        assert!(
            key.referenced_columns
                .iter()
                .all(|id| destination.table(key.referenced_table).unwrap().column(*id).is_some())
        );
    }
    destination.check_storage().unwrap();
    let before = destination.clone();
    let mut invalid = fragment;
    invalid.foreign_keys[0].columns = vec![u64::MAX];
    assert!(destination.paste(&invalid, Pos::default()).is_err());
    assert_eq!(destination, before);
    assert!(generate(&schema, Dialect::PostgreSql).is_ok());
    let copied = schema.paste(&schema.fragment(&ids), Pos::new(30.0, 30.0)).unwrap();
    assert_eq!(copied.len(), ids.len());
    let names: BTreeSet<_> = schema.tables.iter().map(|t| &t.name).collect();
    assert_eq!(names.len(), schema.tables.len());
    assert!(generate(&schema, Dialect::PostgreSql).is_ok());
}

#[test]
fn malformed_uml_clipboard_data_does_not_mutate_the_destination() {
    use op_core::uml::*;
    let mut diagram = UmlDiagram::new("Classes", DiagramKind::Class);
    let a = diagram.add_node(NodeKind::Class, Pos::new(100.0, 100.0), "A").unwrap();
    let b = diagram.add_node(NodeKind::Class, Pos::new(400.0, 100.0), "B").unwrap();
    diagram.connect(a, b, RelationKind::Association).unwrap();
    let mut fragment = diagram.fragment(&BTreeSet::from([a, b]));
    fragment.relations[0].to = u64::MAX;
    let before = diagram.clone();
    assert!(diagram.paste(&fragment, Pos::new(30.0, 30.0)).is_empty());
    assert_eq!(diagram, before);
}
