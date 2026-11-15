use super::*;
use crate::geom::Pos;

fn graph(converted: &Conversion<Diagram>) -> mermaid_svg::ast::FlowchartDiagram {
    let raw = source(&converted.diagram, &converted.warnings);
    assert!(super::super::accepts(&raw));
    let mermaid_svg::Diagram::Flowchart(graph) = mermaid_svg::parse(&raw).unwrap_or_else(|e| panic!("{e}\n{raw}")) else {
        panic!("expected flowchart")
    };
    assert!(super::super::render_svg(&raw).unwrap().contains("<svg"));
    graph
}

#[test]
fn bpmn_projection_preserves_nodes_edges_pools_lanes_and_directions() {
    let mut d = crate::examples::order_process();
    let n = d.add_node(bpmn::NodeKind::DataObject, Pos::new(1200.0, 500.0));
    let target = d.nodes[0].id;
    d.add_flow(n, target).unwrap();
    let store = d.add_node(bpmn::NodeKind::DataStore, Pos::new(1400.0, 500.0));
    d.add_flow(n, store).unwrap();
    d.add_pool(Pos::new(0.0, 600.0));
    let other = d.add_node(bpmn::NodeKind::Task(bpmn::TaskType::Receive), Pos::new(150.0, 650.0));
    d.add_flow(target, other).unwrap();
    let before = d.clone();
    let converted = bpmn(&d).unwrap();
    let g = graph(&converted);
    assert_eq!(g.nodes.len(), d.nodes.len());
    assert_eq!(g.edges.len(), d.flows.len());
    for f in &d.flows {
        let e = g
            .edges
            .iter()
            .find(|e| e.from == format!("n{}", f.from) && e.to == format!("n{}", f.to))
            .unwrap();
        assert_eq!(
            e.head,
            if f.kind == bpmn::FlowKind::Association {
                mermaid_svg::ast::EdgeHead::None
            } else {
                mermaid_svg::ast::EdgeHead::Arrow
            }
        );
        assert_eq!(
            e.line,
            if f.kind == bpmn::FlowKind::Sequence {
                mermaid_svg::ast::EdgeLine::Solid
            } else {
                mermaid_svg::ast::EdgeLine::Dotted
            }
        );
    }
    for n in &d.nodes {
        if let Some((_, lane)) = d.lane_of(n.id) {
            let group = g.subgraphs.iter().find(|g| g.id == format!("lane{lane}")).unwrap();
            assert!(group.node_ids.contains(&format!("n{}", n.id)));
        }
    }
    for p in &d.pools {
        assert!(converted.diagram.source.contains(&format!("subgraph pool{}", p.id)));
        for lane in &p.lanes {
            assert!(converted.diagram.source.contains(&format!("subgraph lane{}", lane.id)));
        }
    }
    assert_eq!(d, before);
    assert!(!converted.warnings.is_empty());
    assert_eq!(converted.diagram.source, bpmn(&d).unwrap().diagram.source);
}

#[test]
fn all_bpmn_shapes_and_untrusted_labels_export_as_data() {
    let mut d = bpmn::BpmnDiagram::new("Escaping");
    let label = "end\" ] --> injected[\"<script> %%{init: x} | \\ `é\nline";
    for kind in [
        bpmn::NodeKind::StartEvent(bpmn::EventTrigger::Timer),
        bpmn::NodeKind::IntermediateEvent {
            trigger: bpmn::EventTrigger::Message,
            throwing: true,
        },
        bpmn::NodeKind::EndEvent(bpmn::EventTrigger::Terminate),
        bpmn::NodeKind::Task(bpmn::TaskType::User),
        bpmn::NodeKind::SubProcess,
        bpmn::NodeKind::Gateway(bpmn::GatewayType::Inclusive),
        bpmn::NodeKind::DataObject,
        bpmn::NodeKind::DataStore,
        bpmn::NodeKind::TextAnnotation,
    ] {
        let id = d.add_node(kind, Pos::default());
        d.node_mut(id).unwrap().name = label.into();
    }
    let a = d.nodes[0].id;
    let b = d.nodes[1].id;
    d.add_flow(a, b).unwrap();
    d.flows[0].name = label.into();
    d.flows[0].is_default = true;
    let g = graph(&bpmn(&d).unwrap());
    assert_eq!(g.nodes.len(), d.nodes.len());
    assert_eq!(g.edges.len(), 1);
    assert!(g.nodes.iter().all(|n| n.id != "injected" && n.click.is_none()));
    assert!(!bpmn(&d).unwrap().diagram.source.contains("<script>"));
    d.flows[0].to = u32::MAX;
    assert!(bpmn(&d).is_err(), "dangling endpoints cannot create phantom nodes");
}

#[test]
fn petri_projection_keeps_initial_marking_capacity_silent_transitions_and_inhibitors() {
    let mut d = crate::examples::mutual_exclusion();
    d.places[0].tokens = 2;
    d.places[0].capacity = Some(4);
    d.transitions[0].silent = true;
    d.arcs[0].weight = 3;
    // An inhibitor and normal arc may share endpoints without sharing semantics.
    let a = d.arcs.iter().find(|a| matches!(a.from, petri::NodeRef::Place(_))).unwrap().clone();
    d.arcs.push(petri::Arc {
        id: d.arcs.iter().map(|a| a.id).max().unwrap() + 1,
        weight: 2,
        kind: petri::ArcKind::Inhibitor,
        ..a
    });
    let before = d.clone();
    let converted = petri(&d).unwrap();
    let g = graph(&converted);
    assert_eq!(g.nodes.len(), d.places.len() + d.transitions.len());
    assert_eq!(g.edges.len(), d.arcs.len());
    assert!(converted.diagram.source.contains("--o"));
    assert!(converted.diagram.source.contains("τ"));
    let inhibitor = g.edges.iter().find(|e| e.head == mermaid_svg::ast::EdgeHead::Circle).unwrap();
    assert!(inhibitor.label.as_ref().unwrap().ends_with("2"));
    assert!(g.edges.iter().any(|e| e.label.as_deref() == Some("3")));
    assert_eq!(d, before);
}

#[test]
fn empty_diagrams_export_without_inventing_elements() {
    for converted in [
        bpmn(&bpmn::BpmnDiagram::new("Empty")).unwrap(),
        petri(&petri::PetriNet::new("Empty")).unwrap(),
        flow(&crate::epc::EpcDiagram::new("Empty")).unwrap(),
        flow(&crate::activity::ActivityDiagram::new("Empty")).unwrap(),
        uml(&uml::UmlDiagram::new("Empty", uml::DiagramKind::UseCase)).unwrap(),
        uml(&uml::UmlDiagram::new("Empty", uml::DiagramKind::Class)).unwrap(),
    ] {
        assert!(mermaid_svg::parse(&converted.diagram.source).is_ok());
        assert!(!converted.diagram.source.contains("n1"));
    }
}

#[test]
fn typed_flow_and_uml_projections_render_and_do_not_change_sources() {
    let epc = crate::epc::example();
    let activity = crate::activity::example();
    assert_eq!(graph(&flow(&epc).unwrap()).nodes.len(), epc.nodes.len());
    assert_eq!(graph(&flow(&activity).unwrap()).edges.len(), activity.flows.len());
    let cases = crate::examples::uml_use_cases();
    let g = graph(&uml(&cases).unwrap());
    assert_eq!(
        g.nodes.len(),
        cases.nodes.iter().filter(|n| n.kind != uml::NodeKind::System).count()
    );
    assert_eq!(g.edges.len(), cases.relations.len());
    let classes = crate::examples::uml_classes();
    let converted = uml(&classes).unwrap();
    assert!(super::super::is_class_source(&converted.diagram.source));
    assert!(super::super::render_svg(&converted.diagram.source).is_ok());
    assert_eq!(classes, crate::examples::uml_classes());
}

#[test]
fn markdown_roundtrips_exact_source_even_invalid_drafts_and_embedded_fences() {
    for code in [
        "",
        "broken draft",
        "flowchart LR\nA-->B\n",
        "flowchart LR\r\nA-->B\r\n",
        "flowchart LR\n```\n````\n",
        "raw ending\r",
    ] {
        let d = Diagram {
            name: "Name\n<script> & `# [link](https://example.com)".into(),
            source: code.into(),
        };
        let md = super::super::markdown::export(&d, &[]);
        assert!(!md.contains("<script>"));
        assert_eq!(super::super::markdown::source(&md).as_deref(), Some(code));
    }
    let converted = bpmn(&crate::examples::order_process()).unwrap();
    let md = super::super::markdown::export(&converted.diagram, &converted.warnings);
    assert_eq!(
        super::super::markdown::source(&md).unwrap(),
        source(&converted.diagram, &converted.warnings)
    );
}

#[test]
fn markdown_import_requires_one_complete_explicit_block() {
    let extract = super::super::markdown::source;
    assert_eq!(
        extract("Prose\r\n~~~mermaid\r\nflowchart LR\r\nA-->B\r\n~~~\r\n").unwrap(),
        "flowchart LR\r\nA-->B"
    );
    assert_eq!(
        extract("```text\nexample\n```\n```mermaid\ninvalid draft\n```\n").unwrap(),
        "invalid draft"
    );
    for invalid in [
        "# Normal Markdown",
        "```mermaid\nflowchart LR",
        "```mermaid\nA\n```\n```mermaid\nB\n```",
        "````text\n```mermaid\nA\n```\n````",
    ] {
        assert!(extract(invalid).is_none(), "{invalid}");
    }
}
