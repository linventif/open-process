//! Reproduce the TakeAway BPMN deliverables with Open Process domain APIs.
//!
//! Run from any directory with `cargo run --locked -p op-core --example takeaway_tp`.
//! `--check` validates the sources/models and compares existing generated outputs.
//! This example never rewrites supplied CSV/XLSX files or existing log/Petri work.

#[path = "takeaway_tp/support.rs"]
mod support;

use op_core::analysis::soundness;
use op_core::bpmn::{BpmnDiagram, EventTrigger, FlowKind, GatewayType, Id, NodeKind};
use op_core::document::{Document, DocumentCodec};
use op_core::geom::Pos;
use op_core::mining::{AlphaResult, DiscoveryOptions, discover_with_options_cancellable, is_routing_marker, replay};
use op_core::sales::{SalesFilter, SalesStudy};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use support::{Corpus, Event, Result};

const ROLE_LANES: [&str; 4] = [
    "Order handling* | William",
    "Production* | Patrick, Robert",
    "Delivery* | Jack, Paul, Richard, Harry",
    "Settlement* | William or delivery actor",
];

fn main() -> Result<()> {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if arguments.iter().any(|argument| argument != "--check") {
        return Err("Usage: takeaway_tp [--check]".into());
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").canonicalize()?;
    op_core::i18n::with_lang(op_core::i18n::Lang::En, || {
        run(&root, arguments.iter().any(|argument| argument == "--check"))
    })
}

fn role(activity: &str) -> usize {
    match activity {
        "Prepare" | "Bake" | "Pack" => 1,
        "Plan" | "Deliver" => 2,
        "Cash" | "Give order" => 3,
        _ => 0,
    }
}

fn first_tasks(diagram: &BpmnDiagram, from: Id) -> BTreeSet<String> {
    let mut queue = VecDeque::from([from]);
    let mut seen = BTreeSet::new();
    let mut tasks = BTreeSet::new();
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        let Some(node) = diagram.node(id) else { continue };
        if matches!(node.kind, NodeKind::Task(_)) {
            tasks.insert(node.name.clone());
        } else {
            queue.extend(diagram.outgoing(id).map(|flow| flow.to));
        }
    }
    tasks
}

fn annotate_choices(diagram: &mut BpmnDiagram) {
    let gateways: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Gateway(GatewayType::Exclusive)) && diagram.outgoing(node.id).count() > 1)
        .map(|node| node.id)
        .collect();
    let mut labels = Vec::new();
    for gateway in gateways {
        for flow in diagram.outgoing(gateway) {
            let tasks = first_tasks(diagram, flow.to);
            let label = if tasks.len() == 1 {
                match tasks.first().unwrap().as_str() {
                    "Prepare" => "Repeat preparation".to_owned(),
                    "Plan" | "Deliver" => "Continue to delivery".to_owned(),
                    "Contact customer" => "Contact needed".to_owned(),
                    "Modify order" => "Order modified".to_owned(),
                    "Cancel order" => "Order cancelled".to_owned(),
                    activity => format!("Continue: {activity}"),
                }
            } else if tasks.contains("Cash") && tasks.contains("Give order") {
                "Payment and handover".into()
            } else {
                tasks.into_iter().collect::<Vec<_>>().join(" / ")
            };
            labels.push((flow.id, label));
        }
    }
    for (id, label) in labels {
        diagram.flow_mut(id).unwrap().name = label;
    }
}

fn lane_position(lane: usize) -> Pos {
    Pos::new(160.0, 160.0 + lane as f32 * 240.0)
}

fn artifact(diagram: &mut BpmnDiagram, kind: NodeKind, name: &str, documentation: &str, lane: usize) -> Id {
    let id = diagram.add_node(kind, lane_position(lane));
    let node = diagram.node_mut(id).unwrap();
    node.name = name.into();
    node.documentation = documentation.into();
    if kind == NodeKind::TextAnnotation {
        node.w = 340.0;
        node.h = 150.0;
    } else {
        node.w = 190.0;
        node.h = 100.0;
    }
    id
}

fn associate(diagram: &mut BpmnDiagram, from: Id, to: Id, _name: &str) -> Result<()> {
    let id = diagram.add_flow(from, to)?;
    let flow = diagram.flow_mut(id).unwrap();
    assert_eq!(flow.kind, FlowKind::Association);
    // Dotted links identify authored information interfaces. Their meaning is
    // documented on the artifacts; repeated edge labels obscure the process.
    flow.name.clear();
    Ok(())
}

fn arrange_information_bands(diagram: &mut BpmnDiagram) -> Result<()> {
    let pool_id = diagram.pools[0].id;
    let lane_ids: Vec<_> = diagram.pools[0].lanes.iter().map(|lane| lane.id).collect();
    let owners: BTreeMap<_, _> = diagram
        .nodes
        .iter()
        .map(|node| {
            let (_, lane) = diagram.lane_of(node.id).expect("All generated elements have a role lane");
            (node.id, lane_ids.iter().position(|id| *id == lane).unwrap())
        })
        .collect();
    let mut control = diagram.clone();
    control.nodes.retain(|node| node.kind.is_flow_node());
    control.flows.retain(|flow| flow.kind == FlowKind::Sequence);
    let mut queue: VecDeque<_> = control
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::StartEvent(_)))
        .map(|node| node.id)
        .collect();
    let mut order = BTreeMap::new();
    while let Some(id) = queue.pop_front() {
        if order.contains_key(&id) {
            continue;
        }
        order.insert(id, order.len());
        queue.extend(control.outgoing(id).map(|flow| flow.to));
    }
    // A logical traversal places a cycle's entry before its repeated work when
    // the shared layout engine must break that cycle for layering.
    control.nodes.sort_by_key(|node| order.get(&node.id).copied().unwrap_or(usize::MAX));
    op_core::bpmn::layout::auto_arrange(&mut control, Some(10.0))?;
    let mut channel_captions = Vec::new();
    if control.nodes.iter().any(|node| node.documentation.starts_with("Channel branch: ")) {
        let mut pool = control.pool(pool_id).unwrap().clone();
        let mut origin_y = pool.pos.y;
        for (lane_index, lane) in pool.lanes.iter_mut().enumerate() {
            // These are visual subrows inside the four shared role lanes, not
            // additional organizational roles or independent participants.
            let mut cursor = if lane_index == 3 { 80.0 } else { 40.0 };
            for (branch, channel) in ["Counter", "Phone", "Internet"].into_iter().enumerate() {
                let prefix = format!("Channel branch: {channel}.");
                let members: Vec<_> = control
                    .nodes
                    .iter()
                    .filter(|node| owners[&node.id] == lane_index && node.documentation.starts_with(&prefix))
                    .map(|node| (node.id, node.pos.y, node.h))
                    .collect();
                if members.is_empty() {
                    continue;
                }
                let top = members.iter().map(|(_, y, h)| y - h / 2.0).fold(f32::INFINITY, f32::min);
                let bottom = members.iter().map(|(_, y, h)| y + h / 2.0).fold(f32::NEG_INFINITY, f32::max);
                let height = ((bottom - top + 60.0) / 10.0).ceil() * 10.0;
                for (id, y, _) in members {
                    let node = control.node_mut(id).unwrap();
                    node.pos.y = origin_y + cursor + 30.0 + y - top;
                    // Distinct entry/return corridors prevent collinear links
                    // from making separate channel branches appear merged.
                    node.pos.x += branch as f32 * 100.0;
                }
                if lane_index > 0 {
                    channel_captions.push((lane_index, channel, Pos::new(pool.pos.x + 130.0, origin_y + cursor + height / 2.0)));
                }
                cursor += height;
            }
            lane.height = (cursor + 40.0).max(240.0);
            origin_y += lane.height;
        }
        let intake_y = control.nodes.iter().find(|node| node.name == "Take order by phone").unwrap().pos.y;
        for node in &mut control.nodes {
            if matches!(node.kind, NodeKind::StartEvent(_)) || node.name == "Order channel" {
                node.pos.y = intake_y;
            }
        }
        let end = control
            .nodes
            .iter()
            .find(|node| matches!(node.kind, NodeKind::EndEvent(_)))
            .unwrap()
            .id;
        let completion = control.incoming(end).next().unwrap().from;
        let right = control
            .nodes
            .iter()
            .filter(|node| node.documentation.starts_with("Channel branch: "))
            .map(|node| node.pos.x + node.w / 2.0)
            .fold(0.0f32, f32::max);
        let corridor_y = pool.pos.y + pool.lanes.iter().take(3).map(|lane| lane.height).sum::<f32>() + 30.0;
        control.node_mut(completion).unwrap().pos = Pos::new(right + 150.0, corridor_y);
        control.node_mut(end).unwrap().pos = Pos::new(right + 280.0, corridor_y);
        pool.width = pool.width.max(right + 360.0 - pool.pos.x);
        *control.pool_mut(pool_id).unwrap() = pool;
    }
    let mut pool = control.pool(pool_id).unwrap().clone();
    let mut artifacts: Vec<Vec<Id>> = vec![Vec::new(); lane_ids.len()];
    for node in diagram.nodes.iter().filter(|node| !node.kind.is_flow_node()) {
        artifacts[owners[&node.id]].push(node.id);
    }
    let mut offsets = Vec::new();
    let mut artifact_y = Vec::new();
    let mut extra_above = 0.0;
    let mut old_y = pool.pos.y;
    for (lane, ids) in pool.lanes.iter_mut().zip(&artifacts) {
        offsets.push(extra_above);
        let extra = if ids.is_empty() {
            0.0
        } else {
            ids.iter().map(|id| diagram.node(*id).unwrap().h).fold(0.0f32, f32::max) + 100.0
        };
        artifact_y.push(old_y + extra_above + lane.height + extra / 2.0);
        old_y += lane.height;
        lane.height += extra;
        extra_above += extra;
    }
    for node in diagram.nodes.iter_mut().filter(|node| node.kind.is_flow_node()) {
        node.pos = control.node(node.id).unwrap().pos;
        node.pos.y += offsets[owners[&node.id]];
    }
    let cash_positions: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Task(_)) && node.name == "Cash")
        .map(|node| node.pos.x)
        .collect();
    let cash_center = cash_positions.iter().sum::<f32>() / cash_positions.len().max(1) as f32;
    for (lane, ids) in artifacts.iter_mut().enumerate() {
        ids.sort_by_key(|id| (diagram.node(*id).unwrap().kind != NodeKind::TextAnnotation, *id));
        for (column, id) in ids.iter().enumerate() {
            let node = diagram.node_mut(*id).unwrap();
            let x = if node.name.starts_with("Price catalogue") {
                cash_center - 250.0
            } else if node.name.starts_with("Recorded sales") {
                cash_center + 250.0
            } else {
                pool.pos.x + 230.0 + column as f32 * 450.0
            };
            node.pos = Pos::new(x.max(pool.pos.x + 230.0), artifact_y[lane]);
            pool.width = pool.width.max(node.pos.x - pool.pos.x + node.w / 2.0 + 80.0);
        }
    }
    *diagram.pool_mut(pool_id).unwrap() = pool;
    for (lane, channel, mut position) in channel_captions {
        position.y += offsets[lane];
        let id = diagram.add_node(NodeKind::TextAnnotation, position);
        let caption = diagram.node_mut(id).unwrap();
        caption.name = channel.into();
        caption.w = 110.0;
        caption.h = 26.0;
        caption.documentation="Visual channel subrow within a shared role lane. Control-flow branches remain separate; this caption is not an executable activity or an additional participant.".into();
    }
    Ok(())
}

fn enrich(diagram: &mut BpmnDiagram, events: &[Event], channel: &str) -> Result<()> {
    let task_ids: Vec<_> = diagram
        .nodes
        .iter()
        .filter(|node| matches!(node.kind, NodeKind::Task(_)))
        .map(|node| node.id)
        .collect();
    let mut memberships = BTreeMap::new();
    for node in &diagram.nodes {
        let lane = if matches!(node.kind, NodeKind::Task(_)) {
            role(&node.name)
        } else if matches!(node.kind, NodeKind::EndEvent(_)) {
            3
        } else {
            first_tasks(diagram, node.id).iter().map(|task| role(task)).min().unwrap_or(3)
        };
        memberships.insert(node.id, lane);
    }
    for node in &mut diagram.nodes {
        node.pos = lane_position(memberships[&node.id]);
        if matches!(node.kind, NodeKind::Task(_)) {
            node.w = 150.0;
            node.h = 76.0;
            if node.documentation.starts_with("Channel branch: ") {
                node.documentation
                    .push_str(" The integrated model retains this branch-specific assignment evidence.");
                continue;
            }
            let observed = support::actor_counts(events, &node.name);
            node.documentation = format!(
                "Source: corrected {channel} event log(s). Observed assignments (event counts): {}. Role lane titles marked * are inferred groupings. No task duration or execution cost is recorded; zero-valued model defaults are not measurements.",
                observed
                    .iter()
                    .map(|(actor, count)| format!("{actor}: {count}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    let pool = diagram.add_pool(Pos::new(40.0, 40.0));
    for _ in 0..3 {
        diagram.add_lane(pool);
    }
    let name = diagram.name.clone();
    let participant = diagram.pool_mut(pool).unwrap();
    participant.name = name;
    participant.width = 600.0;
    for (lane, label) in participant.lanes.iter_mut().zip(ROLE_LANES) {
        lane.name = label.into();
        lane.height = 240.0;
    }
    let case = artifact(
        diagram,
        NodeKind::DataObject,
        "Case and channel [observed]",
        "Corrected event CSV: CaseId, Activity, Actor; order channel comes from the source file. No customer identity or timestamp is available. Sales IDs are not case IDs.",
        0,
    );
    let specification = artifact(
        diagram,
        NodeKind::DataObject,
        "Pizza type / size [source fields]",
        "The Sales worksheets record Pizza Type and Pizza Size. Using those fields as a preparation specification is an authored information-flow assumption; there is no row-level sales-to-case join.",
        1,
    );
    let catalogue = artifact(
        diagram,
        NodeKind::DataStore,
        "Price catalogue [source]",
        "PizzasInfo: 18 type/size entries, recorded cost and revenue. Monetary unit is unspecified. The association with settlement describes a proposed information interface, not a recorded data exchange.",
        3,
    );
    let sales = artifact(
        diagram,
        NodeKind::DataStore,
        "Recorded sales [source]",
        "Sales fields: Id, StructureId, Pizza Type, Pizza Size, Revenue, Cost, Satisfaction, Weekday, Customer Location, Customer Type. This is a sales record, not proof of a distinct customer or an event-log transaction. Cached Excel values are used; formulas are never executed.",
        3,
    );
    let delivery = artifact(
        diagram,
        NodeKind::DataObject,
        "Customer location [source field]",
        "Customer Location is a geographic category in Sales, not a street address or a case-linked delivery instruction. Its proposed use in planning is an explicit modeling assumption.",
        2,
    );
    let global_choice = diagram
        .nodes
        .iter()
        .find(|node| node.name == "Order channel" && node.kind.is_gateway())
        .map(|node| node.id);
    if let Some(choice) = global_choice {
        associate(diagram, case, choice, "shared channel input")?;
    }
    let mut illustrated = BTreeSet::new();
    for id in &task_ids {
        let name = diagram.node(*id).unwrap().name.clone();
        if !illustrated.insert(name.clone()) {
            continue;
        }
        match name.as_str() {
            "Take order at counter" | "Take order by phone" | "Receive order by internet" => {
                if global_choice.is_none() {
                    associate(diagram, *id, case, "recorded case key")?;
                }
            }
            "Prepare" => associate(diagram, specification, *id, "proposed input")?,
            "Cash" => {
                associate(diagram, catalogue, *id, "proposed pricing input")?;
                associate(diagram, *id, sales, "proposed recording")?;
            }
            "Plan" => associate(diagram, delivery, *id, "proposed area input")?,
            _ => {}
        }
    }
    // A channel without delivery does not need an unconnected delivery artifact.
    if !task_ids.iter().any(|id| diagram.node(*id).unwrap().name == "Plan") {
        diagram.remove_node(delivery);
        artifact(
            diagram,
            NodeKind::TextAnnotation,
            "No delivery is observed in the Counter cases.",
            "This role vocabulary is shared with the integrated model. No Counter task is assigned to the delivery lane.",
            2,
        );
    }
    let missing_actors = events
        .iter()
        .filter(|event| event.actor.is_none() && !is_routing_marker(&event.activity))
        .count();
    let role_note = artifact(
        diagram,
        NodeKind::TextAnnotation,
        &format!("* Roles inferred from tasks. People are observed. {missing_actors} business events have no actor."),
        "William handles intake and Internet approval/contact/modification/cancellation. Patrick and Robert prepare, bake and pack. Jack, Paul, Richard and Harry plan/deliver and settle delivered orders. Counter and pickup settlement is assigned to William. Unknown Counter Prepare actors are retained, not filled from adjacent events.",
        0,
    );
    if let Some(id) = task_ids.iter().find(|id| role(&diagram.node(**id).unwrap().name) == 0) {
        associate(diagram, role_note, *id, "interpretation")?;
    }
    let assumptions = artifact(
        diagram,
        NodeKind::TextAnnotation,
        "Loops and order flexibility are observed; their business cause and timing are unknown.",
        "Prepare -> Bake -> Pack repeats one to three times in the logs. The model generalizes this as a loop; no defect/rework cause or one-cycle/one-sale equivalence is asserted. Cash and Give order occur in both orders. Logical parallel routing expresses that flexibility, not simultaneous human work. Branch labels are explanatory, not an executable decision engine.",
        1,
    );
    if let Some(id) = task_ids.iter().find(|id| diagram.node(**id).unwrap().name == "Prepare") {
        associate(diagram, assumptions, *id, "interpretation")?;
    }
    if global_choice.is_some() {
        for node in diagram.nodes.iter_mut().filter(|node| !node.kind.is_flow_node()) {
            node.documentation.push_str(" In the global view this is a shared information concept for every applicable branch; a representative association is drawn to keep the diagram readable.");
        }
    }
    arrange_information_bands(diagram)?;
    diagram.check_storage()?;
    Ok(())
}

fn global_union(channels: &[(&Corpus, BpmnDiagram)]) -> Result<BpmnDiagram> {
    let mut global = BpmnDiagram::new("TakeAway - integrated order management");
    let start = global.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::default());
    let choice = global.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::default());
    global.node_mut(choice).unwrap().name = "Order channel".into();
    let merge = global.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::default());
    let end = global.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::default());
    global.node_mut(end).unwrap().documentation =
        "A selected channel completes, including the explicitly observed Internet cancellation route.".into();
    global.add_flow(start, choice)?;
    global.add_flow(merge, end)?;
    for (corpus, source) in channels {
        let mut ids = BTreeMap::new();
        for node in &source.nodes {
            if !node.kind.is_flow_node() || matches!(node.kind, NodeKind::StartEvent(_) | NodeKind::EndEvent(_)) {
                continue;
            }
            let id = global.add_node(node.kind, node.pos);
            let target = global.node_mut(id).unwrap();
            target.name = node.name.clone();
            target.documentation = format!("Channel branch: {}. {}", corpus.channel, node.documentation);
            ids.insert(node.id, id);
        }
        for flow in source.flows.iter().filter(|flow| flow.kind == FlowKind::Sequence) {
            let from = source.node(flow.from).unwrap();
            let to = source.node(flow.to).unwrap();
            let source_id = if matches!(from.kind, NodeKind::StartEvent(_)) {
                choice
            } else {
                ids[&from.id]
            };
            let target_id = if matches!(to.kind, NodeKind::EndEvent(_)) {
                merge
            } else {
                ids[&to.id]
            };
            let id = global.add_flow(source_id, target_id)?;
            global.flow_mut(id).unwrap().name = if source_id == choice {
                corpus.channel.clone()
            } else {
                flow.name.clone()
            };
        }
    }
    global.node_mut(choice).unwrap().documentation = "One channel is selected for a case. Branch control-flow graphs remain distinct to retain channel-dependent delivery and cancellation choices; role lanes, information objects, entry and completion are integrated. Reusing a task label does not merge its behavior.".into();
    Ok(global)
}

fn segment_crosses_node(from: Pos, to: Pos, node: &op_core::bpmn::Node) -> bool {
    let rect = node.rect();
    let mut lower = 0.0f32;
    let mut upper = 1.0f32;
    for (origin, delta, minimum, maximum) in [
        (from.x, to.x - from.x, rect.x + 3.0, rect.right() - 3.0),
        (from.y, to.y - from.y, rect.y + 3.0, rect.bottom() - 3.0),
    ] {
        if delta.abs() < 0.0001 {
            if origin <= minimum || origin >= maximum {
                return false;
            }
        } else {
            let a = (minimum - origin) / delta;
            let b = (maximum - origin) / delta;
            lower = lower.max(a.min(b));
            upper = upper.min(a.max(b));
        }
    }
    upper - lower > 0.0001 && upper > 0.0 && lower < 1.0
}

fn validate_geometry(diagram: &BpmnDiagram) -> Result<Value> {
    let mut overlaps = Vec::new();
    for (index, a) in diagram.nodes.iter().enumerate() {
        for b in diagram.nodes.iter().skip(index + 1) {
            let (ra, rb) = (a.rect(), b.rect());
            if ra.x + 3.0 < rb.right() - 3.0
                && rb.x + 3.0 < ra.right() - 3.0
                && ra.y + 3.0 < rb.bottom() - 3.0
                && rb.y + 3.0 < ra.bottom() - 3.0
            {
                overlaps.push(json!({"first_node":a.id,"second_node":b.id}));
            }
        }
    }
    let mut intersections: BTreeMap<&str, Vec<Value>> = [("Sequence", Vec::new()), ("Association", Vec::new()), ("Message", Vec::new())]
        .into_iter()
        .collect();
    for flow in &diagram.flows {
        let points = op_core::bpmn::route(diagram, flow);
        let kind = match flow.kind {
            FlowKind::Sequence => "Sequence",
            FlowKind::Association => "Association",
            FlowKind::Message => "Message",
        };
        for node in diagram.nodes.iter().filter(|node| node.id != flow.from && node.id != flow.to) {
            if points.windows(2).any(|pair| segment_crosses_node(pair[0], pair[1], node)) {
                intersections
                    .get_mut(kind)
                    .unwrap()
                    .push(json!({"flow_id":flow.id,"node_id":node.id,"node_name":node.name}));
            }
        }
    }
    if !overlaps.is_empty() || !intersections["Sequence"].is_empty() {
        return Err(format!(
            "Presentation geometry failed for {}: node overlaps={overlaps:?}, sequence crossings={:?}",
            diagram.name, intersections["Sequence"]
        )
        .into());
    }
    Ok(
        json!({"node_body_overlaps":overlaps.len(),"sequence_route_node_crossings":intersections["Sequence"].len(),"association_route_node_crossings":intersections["Association"].len(),"message_route_node_crossings":intersections["Message"].len(),"route_node_intersection_details":intersections,"scope":"Node interiors use a 3 px inset; a flow's own endpoints are excluded. Text-label collisions are assessed separately by visual inspection."}),
    )
}

fn validate_model(diagram: &BpmnDiagram, corpora: &[&Corpus], discoveries: &[&AlphaResult]) -> Result<Value> {
    diagram.check_storage()?;
    let geometry = validate_geometry(diagram)?;
    assert!(
        !diagram
            .nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::Task(_)) && (node.name.is_empty() || is_routing_marker(&node.name)))
    );
    let issues = diagram.validate();
    let errors: Vec<_> = issues
        .iter()
        .filter(|issue| issue.severity == op_core::bpmn::Severity::Error)
        .collect();
    if !errors.is_empty() {
        return Err(format!("BPMN validation errors for {}: {errors:?}", diagram.name).into());
    }
    let conversion = op_core::convert::bpmn_to_petri(diagram);
    conversion.net.check_storage()?;
    let xml = op_core::bpmn_xml::export(diagram);
    let imported = op_core::bpmn_xml::import(&xml)?;
    imported.check_storage()?;
    assert_eq!(diagram.nodes.len(), imported.nodes.len());
    assert_eq!(diagram.flows.len(), imported.flows.len());
    assert_eq!(
        diagram.pools.iter().map(|pool| pool.lanes.len()).sum::<usize>(),
        imported.pools.iter().map(|pool| pool.lanes.len()).sum::<usize>()
    );
    let xml_conversion = op_core::convert::bpmn_to_petri(&imported);
    let native = BpmnDiagram::CODEC.encode(diagram).to_json();
    let reopened = BpmnDiagram::CODEC.decode(&Document::read(native.as_bytes())?)?;
    assert_eq!(&reopened, diagram);
    let mut cases = Vec::new();
    for (corpus, discovery) in corpora.iter().zip(discoveries) {
        for (index, raw) in corpus.log.traces.iter().enumerate() {
            let trace = discovery.project_trace(raw);
            replay(&discovery.net, &trace)
                .map_err(|error| format!("{} source Petri case {}: {error}", corpus.channel, corpus.case_ids[index]))?;
            replay(&conversion.net, &trace).map_err(|error| format!("{} BPMN case {}: {error}", corpus.channel, corpus.case_ids[index]))?;
            replay(&xml_conversion.net, &trace)
                .map_err(|error| format!("{} XML roundtrip case {}: {error}", corpus.channel, corpus.case_ids[index]))?;
            cases.push(json!({"channel":corpus.channel,"case_id":corpus.case_ids[index],"recorded_events":raw.len(),"observable_trace":trace,"source_petri":"accepted","bpmn_to_petri":"accepted","bpmn_xml_roundtrip":"accepted"}));
        }
    }
    Ok(json!({
        "model":diagram.name,"nodes":diagram.nodes.len(),"flows":diagram.flows.len(),"pools":diagram.pools.len(),"lanes":diagram.pools.iter().map(|pool|pool.lanes.len()).sum::<usize>(),
        "recorded_case_replays":cases.len(),"all_recorded_cases_accepted":true,"cases":cases,"geometry":geometry,
        "native_roundtrip":"equal","bpmn_xml_roundtrip":"all observed traces accepted",
        "validation_warnings":issues.iter().map(|issue|format!("{issue:?}")).collect::<Vec<_>>(),
        "bpmn_to_petri_warnings":conversion.warnings,"xml_roundtrip_conversion_warnings":xml_conversion.warnings,
        "bounded_soundness":format!("{:?}",soundness(&conversion.net,20_000)),
        "proof_scope":"Bounded checks and acceptance of observed traces; not a general conformance/equivalence proof. Human resource availability and information associations are not executable constraints."
    }))
}

fn negative_checks(global: &BpmnDiagram, corpora: &[Corpus], discoveries: &[AlphaResult]) -> Result<Value> {
    let net = op_core::convert::bpmn_to_petri(global).net;
    let mut checks = Vec::new();
    for (index, corpus) in corpora.iter().enumerate() {
        let trace = corpus
            .log
            .traces
            .iter()
            .map(|trace| discoveries[index].project_trace(trace))
            .find(|trace| trace.iter().any(|activity| activity == "Cash"))
            .unwrap();
        for missing in ["Bake", "Cash", "Give order"] {
            let mut invalid = trace.clone();
            invalid.remove(invalid.iter().position(|activity| activity == missing).unwrap());
            checks.push((format!("{}: missing {missing}", corpus.channel), invalid));
        }
        let mut invalid = trace.clone();
        invalid.insert(0, "Deliver".into());
        checks.push((format!("{}: delivery before intake", corpus.channel), invalid));
    }
    let mut counter = discoveries[0].project_trace(&corpora[0].log.traces[0]);
    let insert = counter
        .iter()
        .position(|activity| activity == "Cash" || activity == "Give order")
        .unwrap();
    counter.splice(insert..insert, ["Plan".into(), "Deliver".into()]);
    checks.push(("Counter must not use a delivery branch".into(), counter));
    let mut internet = discoveries[2].project_trace(&corpora[2].log.traces[0]);
    internet.retain(|activity| activity != "Plan" && activity != "Deliver");
    checks.push(("Completed Internet orders require delivery".into(), internet));
    let phone = vec!["Take order by phone".into(), "Contact customer".into(), "Cancel order".into()];
    checks.push(("Phone must not use Internet contact/cancellation".into(), phone));
    let mut results = Vec::new();
    for (name, trace) in checks {
        let error = replay(&net, &trace)
            .err()
            .ok_or_else(|| format!("Invalid cross-channel/control trace was accepted: {name}"))?;
        if error == op_core::i18n::tr("mining.replay_limit") {
            return Err(format!("Negative check was inconclusive: {name}").into());
        }
        results.push(json!({"check":name,"trace":trace,"result":"rejected","diagnostic":error}));
    }
    Ok(Value::Array(results))
}

fn add_model_outputs(outputs: &mut BTreeMap<PathBuf, Vec<u8>>, stem: &str, diagram: &BpmnDiagram) {
    outputs.insert(
        PathBuf::from(format!("tp/results/bpmn/takeaway-{stem}.opx")),
        BpmnDiagram::CODEC.encode(diagram).to_json().into_bytes(),
    );
    outputs.insert(
        PathBuf::from(format!("tp/results/bpmn/takeaway-{stem}.bpmn")),
        op_core::bpmn_xml::export(diagram).into_bytes(),
    );
    outputs.insert(
        PathBuf::from(format!("tp/results/bpmn/takeaway-{stem}.svg")),
        diagram.to_svg().into_bytes(),
    );
}

fn run(root: &Path, check_only: bool) -> Result<()> {
    let corpora = ["Counter", "Phone", "Internet"]
        .into_iter()
        .map(|channel| support::load(root, channel))
        .collect::<Result<Vec<_>>>()?;
    assert_eq!(corpora.iter().map(|corpus| corpus.log.traces.len()).sum::<usize>(), 327);
    let discoveries = corpora
        .iter()
        .map(|corpus| {
            discover_with_options_cancellable(
                &corpus.log,
                &DiscoveryOptions {
                    normalize_boundaries: true,
                },
                &|| false,
            )
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut channels = Vec::new();
    let mut validations = Vec::new();
    let mut outputs = BTreeMap::new();
    for (corpus, discovery) in corpora.iter().zip(&discoveries) {
        let (mut diagram, warnings) = op_core::convert::petri_to_bpmn(&discovery.net);
        diagram.name = format!("TakeAway - {} order management", corpus.channel);
        annotate_choices(&mut diagram);
        enrich(&mut diagram, &corpus.events, &corpus.channel)?;
        let mut validation = validate_model(&diagram, &[corpus], &[discovery])?;
        validation["petri_to_bpmn_warnings"] = json!(warnings);
        validations.push(validation);
        add_model_outputs(&mut outputs, &corpus.channel.to_lowercase(), &diagram);
        channels.push((corpus, diagram));
    }
    let mut global = global_union(&channels)?;
    let events: Vec<_> = corpora.iter().flat_map(|corpus| corpus.events.iter().cloned()).collect();
    enrich(&mut global, &events, "Counter / Phone / Internet")?;
    let corpus_refs: Vec<_> = corpora.iter().collect();
    let discovery_refs: Vec<_> = discoveries.iter().collect();
    let mut global_validation = validate_model(&global, &corpus_refs, &discovery_refs)?;
    global_validation["negative_control_and_cross_channel_checks"] = negative_checks(&global, &corpora, &discoveries)?;
    add_model_outputs(&mut outputs, "global", &global);
    let audits = corpora.iter().map(support::audit).collect::<Result<Vec<_>>>()?;
    let evidence = json!({"schema_version":1,"generator":"cargo run --locked -p op-core --example takeaway_tp", "inputs":audits,"integration":"One participant and shared inferred-role lanes/data interfaces, one channel XOR and common completion. Separate branch control-flow graphs prevent unsupported cross-channel hybrids; task labels alone are never used to merge behavior.","monetary_unit":"unspecified source unit","customer_identity":"not present","calendar_dates":"not present","loop_cause":"not recorded; repeated production is not asserted to be defect rework or one pizza per sale","information_associations":"explicit authored interpretations from complementary source fields, not observed data exchange","task_cost_and_duration":"not observed; model defaults of zero are not measurements","sources_unchanged":true});
    let validation = json!({"schema_version":1,"channels":validations,"global":global_validation,"all_327_cases_accepted_by_channel_and_global_bpmn":true,"checks":"native storage/roundtrip, standard BPMN XML roundtrip, projected trace replay through BPMN-to-Petri, selected invalid control/cross-channel traces, bounded soundness","not_claimed":"complete behavioral equivalence, executable decision conditions, resource-capacity scheduling, or correctness outside observed behavior and bounded checks"});
    outputs.insert("tp/results/bpmn/evidence.json".into(), serde_json::to_vec_pretty(&evidence)?);
    outputs.insert("tp/results/bpmn/validation.json".into(), serde_json::to_vec_pretty(&validation)?);
    let mut sales = SalesStudy::new("TakeAway recorded sales");
    for corpus in &corpora {
        let mut dataset = corpus.sales.clone();
        dataset.channel = corpus.channel.clone();
        sales.add_dataset(dataset)?;
    }
    let analysis = sales.analyze(&SalesFilter::default())?;
    outputs.insert(
        "tp/results/sales/weekday-series.csv".into(),
        analysis.weekday_series_csv().into_bytes(),
    );
    outputs.insert("tp/results/sales/kpis.csv".into(), analysis.kpis_csv().into_bytes());
    let cancellation_scenario = sales.analyze(&SalesFilter {
        channel: Some("Internet".into()),
        structure_id: Some("7".into()),
        ..Default::default()
    })?;
    outputs.insert(
        "tp/results/sales/internet-structure-7.csv".into(),
        cancellation_scenario.to_csv(op_core::sales::SalesGroupBy::Channel).into_bytes(),
    );
    for corpus in &corpora {
        assert_eq!(
            support::fingerprint(&fs::read(root.join(&corpus.csv_path))?),
            corpus.csv_fingerprint
        );
        assert_eq!(
            support::fingerprint(&fs::read(root.join(&corpus.workbook_path))?),
            corpus.workbook_fingerprint
        );
    }
    for (path, bytes) in &outputs {
        let destination = root.join(path);
        if check_only {
            if fs::read(&destination)? != *bytes {
                return Err(format!("Generated artifact differs: {}", path.display()).into());
            }
        } else {
            fs::create_dir_all(destination.parent().unwrap())?;
            let temporary = destination.with_extension(format!("{}.tmp", destination.extension().unwrap().to_string_lossy()));
            fs::write(&temporary, bytes)?;
            fs::rename(temporary, &destination)?;
        }
    }
    println!(
        "{} {} outputs; 327/327 cases accepted by channel BPMNs, global BPMN and XML roundtrips. Supplied inputs and existing log/Petri artifacts were not rewritten.",
        if check_only { "Verified" } else { "Generated" },
        outputs.len()
    );
    Ok(())
}
