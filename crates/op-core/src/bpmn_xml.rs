//! BPMN 2.0 XML import / export (with BPMN DI so diagrams keep their layout).
//! Interoperable with bpmn.io / Camunda Modeler, Bizagi Modeler (BPMN export),
//! Signavio, Visual Paradigm, etc.

use crate::bpmn::*;
use crate::geom::Pos;
use crate::xml::{self, Element, XmlWriter};
use std::collections::HashMap;

const NS: &[(&str, &str)] = &[
    ("xmlns:bpmn", "http://www.omg.org/spec/BPMN/20100524/MODEL"),
    ("xmlns:bpmndi", "http://www.omg.org/spec/BPMN/20100524/DI"),
    ("xmlns:dc", "http://www.omg.org/spec/DD/20100524/DC"),
    ("xmlns:di", "http://www.omg.org/spec/DD/20100524/DI"),
    ("xmlns:op", "https://open-process.app/schema/evaluation"),
];

fn node_xml_id(n: &Node) -> String {
    let prefix = match n.kind {
        k if k.is_event() => "Event",
        NodeKind::Gateway(_) => "Gateway",
        NodeKind::DataObject => "DataObjectReference",
        NodeKind::DataStore => "DataStoreReference",
        NodeKind::TextAnnotation => "TextAnnotation",
        _ => "Activity",
    };
    format!("{prefix}_{}", n.id)
}

fn f(v: f32) -> String {
    format!("{}", (v * 100.0).round() / 100.0)
}

/// Serializes the diagram to BPMN 2.0 XML.
pub fn export(d: &BpmnDiagram) -> String {
    let ids: HashMap<Id, String> = d.nodes.iter().map(|n| (n.id, node_xml_id(n))).collect();
    let flow_id = |fl: &Flow| match fl.kind {
        FlowKind::Sequence => format!("Flow_{}", fl.id),
        FlowKind::Message => format!("MessageFlow_{}", fl.id),
        FlowKind::Association => format!("Association_{}", fl.id),
    };
    let has_collab = !d.pools.is_empty();

    // Which process does each node belong to?
    let default_pool = d.pools.first().map(|p| p.id);
    let process_of = |n: &Node| d.pool_of(n.id).or(default_pool);

    let mut w = XmlWriter::new();
    let mut attrs: Vec<(&str, String)> = NS.iter().map(|(k, v)| (*k, v.to_string())).collect();
    attrs.extend([
        ("id", "Definitions_1".to_string()),
        ("targetNamespace", "http://bpmn.io/schema/bpmn".to_string()),
        ("exporter", "Open Process".to_string()),
        ("exporterVersion", env!("CARGO_PKG_VERSION").to_string()),
    ]);
    w.open("bpmn:definitions", &attrs);

    if has_collab {
        w.open("bpmn:collaboration", &[("id", "Collaboration_1".into())]);
        for p in &d.pools {
            w.empty(
                "bpmn:participant",
                &[
                    ("id", format!("Participant_{}", p.id)),
                    ("name", p.name.clone()),
                    ("processRef", format!("Process_{}", p.id)),
                ],
            );
        }
        for fl in d.flows.iter().filter(|f| f.kind == FlowKind::Message) {
            let mut a = vec![
                ("id", flow_id(fl)),
                ("sourceRef", ids[&fl.from].clone()),
                ("targetRef", ids[&fl.to].clone()),
            ];
            if !fl.name.is_empty() {
                a.push(("name", fl.name.clone()));
            }
            w.empty("bpmn:messageFlow", &a);
        }
        w.close("bpmn:collaboration");
    }

    let processes: Vec<Option<Id>> = if has_collab {
        d.pools.iter().map(|p| Some(p.id)).collect()
    } else {
        vec![None]
    };
    for proc in processes {
        let pid = proc.map(|p| format!("Process_{p}")).unwrap_or_else(|| "Process_1".into());
        let mut pattrs = vec![("id", pid.clone()), ("isExecutable", "false".to_string())];
        if !has_collab && !d.name.is_empty() {
            pattrs.push(("name", d.name.clone()));
        }
        w.open("bpmn:process", &pattrs);
        let in_proc = |n: &&Node| !has_collab || process_of(n) == proc;

        // Lanes
        if let Some(pool) = proc.and_then(|p| d.pool(p))
            && pool.lanes.len() > 1
        {
            w.open("bpmn:laneSet", &[("id", format!("LaneSet_{}", pool.id))]);
            for l in &pool.lanes {
                w.open("bpmn:lane", &[("id", format!("Lane_{}", l.id)), ("name", l.name.clone())]);
                for n in d
                    .nodes
                    .iter()
                    .filter(|n| n.kind.is_flow_node() && d.lane_of(n.id).map(|x| x.1) == Some(l.id))
                {
                    w.text_element("bpmn:flowNodeRef", &[], &ids[&n.id]);
                }
                w.close("bpmn:lane");
            }
            w.close("bpmn:laneSet");
        }

        for n in d.nodes.iter().filter(in_proc) {
            let id = ids[&n.id].clone();
            let mut a = vec![("id", id.clone())];
            if !n.name.is_empty() && n.kind != NodeKind::TextAnnotation {
                a.push(("name", n.name.clone()));
            }
            let (tag, trigger): (String, Option<EventTrigger>) = match n.kind {
                NodeKind::StartEvent(t) => ("bpmn:startEvent".into(), Some(t)),
                NodeKind::EndEvent(t) => ("bpmn:endEvent".into(), Some(t)),
                NodeKind::IntermediateEvent { trigger, throwing } => (
                    if throwing {
                        "bpmn:intermediateThrowEvent"
                    } else {
                        "bpmn:intermediateCatchEvent"
                    }
                    .into(),
                    Some(trigger),
                ),
                NodeKind::Task(t) => (format!("bpmn:{}", t.xml_tag()), None),
                NodeKind::SubProcess => ("bpmn:subProcess".into(), None),
                NodeKind::Gateway(g) => {
                    if let Some(def) = d.outgoing(n.id).find(|f| f.is_default) {
                        a.push(("default", flow_id(def)));
                    }
                    (format!("bpmn:{}", g.xml_tag()), None)
                }
                NodeKind::DataObject => {
                    w.empty("bpmn:dataObject", &[("id", format!("DataObject_{}", n.id))]);
                    a.push(("dataObjectRef", format!("DataObject_{}", n.id)));
                    ("bpmn:dataObjectReference".into(), None)
                }
                NodeKind::DataStore => ("bpmn:dataStoreReference".into(), None),
                NodeKind::TextAnnotation => {
                    w.open("bpmn:textAnnotation", &a);
                    w.text_element("bpmn:text", &[], &n.name);
                    w.close("bpmn:textAnnotation");
                    continue;
                }
            };
            let incoming: Vec<&Flow> = d.incoming(n.id).collect();
            let outgoing: Vec<&Flow> = d.outgoing(n.id).collect();
            let def = trigger.and_then(|t| t.xml_definition());
            if incoming.is_empty()
                && outgoing.is_empty()
                && def.is_none()
                && n.documentation.is_empty()
                && !matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess)
            {
                w.empty(&tag, &a);
                continue;
            }
            w.open(&tag, &a);
            if !n.documentation.is_empty() {
                w.text_element("bpmn:documentation", &[], &n.documentation);
            }
            if matches!(n.kind, NodeKind::Task(_) | NodeKind::SubProcess) {
                w.open("bpmn:extensionElements", &[]);
                w.empty(
                    "op:activityMetrics",
                    &[("cost", n.cost.to_string()), ("duration", n.duration.to_string())],
                );
                w.close("bpmn:extensionElements");
            }
            for fl in incoming {
                w.text_element("bpmn:incoming", &[], &flow_id(fl));
            }
            for fl in outgoing {
                w.text_element("bpmn:outgoing", &[], &flow_id(fl));
            }
            if let Some(def) = def {
                w.empty(&format!("bpmn:{def}"), &[("id", format!("{}Def_{}", def, n.id))]);
            }
            w.close(&tag);
        }

        for fl in d.flows.iter() {
            let Some(src) = d.node(fl.from) else { continue };
            if has_collab && process_of(src) != proc {
                continue;
            }
            match fl.kind {
                FlowKind::Sequence => {
                    let mut a = vec![
                        ("id", flow_id(fl)),
                        ("sourceRef", ids[&fl.from].clone()),
                        ("targetRef", ids[&fl.to].clone()),
                    ];
                    if !fl.name.is_empty() {
                        a.push(("name", fl.name.clone()));
                    }
                    w.empty("bpmn:sequenceFlow", &a);
                }
                FlowKind::Association => {
                    w.empty(
                        "bpmn:association",
                        &[
                            ("id", flow_id(fl)),
                            ("sourceRef", ids[&fl.from].clone()),
                            ("targetRef", ids[&fl.to].clone()),
                        ],
                    );
                }
                FlowKind::Message if !has_collab => {
                    w.empty(
                        "bpmn:sequenceFlow",
                        &[
                            ("id", flow_id(fl)),
                            ("sourceRef", ids[&fl.from].clone()),
                            ("targetRef", ids[&fl.to].clone()),
                        ],
                    );
                }
                FlowKind::Message => {}
            }
        }
        w.close("bpmn:process");
    }

    // ---- Diagram interchange
    w.open("bpmndi:BPMNDiagram", &[("id", "BPMNDiagram_1".into())]);
    let plane_ref = if has_collab {
        "Collaboration_1".to_string()
    } else {
        "Process_1".to_string()
    };
    w.open("bpmndi:BPMNPlane", &[("id", "BPMNPlane_1".into()), ("bpmnElement", plane_ref)]);
    let bounds = |w: &mut XmlWriter, x: f32, y: f32, wd: f32, h: f32| {
        w.empty("dc:Bounds", &[("x", f(x)), ("y", f(y)), ("width", f(wd)), ("height", f(h))]);
    };
    for p in &d.pools {
        let r = p.rect();
        w.open(
            "bpmndi:BPMNShape",
            &[
                ("id", format!("Participant_{}_di", p.id)),
                ("bpmnElement", format!("Participant_{}", p.id)),
                ("isHorizontal", "true".into()),
            ],
        );
        bounds(&mut w, r.x, r.y, r.w, r.h);
        w.close("bpmndi:BPMNShape");
        if p.lanes.len() > 1 {
            for (i, l) in p.lanes.iter().enumerate() {
                let r = p.lane_rect(i);
                w.open(
                    "bpmndi:BPMNShape",
                    &[
                        ("id", format!("Lane_{}_di", l.id)),
                        ("bpmnElement", format!("Lane_{}", l.id)),
                        ("isHorizontal", "true".into()),
                    ],
                );
                bounds(&mut w, r.x, r.y, r.w, r.h);
                w.close("bpmndi:BPMNShape");
            }
        }
    }
    for n in &d.nodes {
        let r = n.rect();
        let mut a = vec![("id", format!("{}_di", ids[&n.id])), ("bpmnElement", ids[&n.id].clone())];
        if n.kind == NodeKind::Gateway(GatewayType::Exclusive) {
            a.push(("isMarkerVisible", "true".into()));
        }
        if n.kind == NodeKind::SubProcess {
            a.push(("isExpanded", "false".into()));
        }
        w.open("bpmndi:BPMNShape", &a);
        bounds(&mut w, r.x, r.y, r.w, r.h);
        w.close("bpmndi:BPMNShape");
    }
    for fl in &d.flows {
        let pts = route(d, fl);
        w.open(
            "bpmndi:BPMNEdge",
            &[("id", format!("{}_di", flow_id(fl))), ("bpmnElement", flow_id(fl))],
        );
        for p in pts {
            w.empty("di:waypoint", &[("x", f(p.x)), ("y", f(p.y))]);
        }
        w.close("bpmndi:BPMNEdge");
    }
    w.close("bpmndi:BPMNPlane");
    w.close("bpmndi:BPMNDiagram");
    w.close("bpmn:definitions");
    w.out
}

// -------------------------------------------------------------------------
// Import

fn trigger_of(e: &Element) -> EventTrigger {
    for c in &e.children {
        let t = match c.name.as_str() {
            "messageEventDefinition" => EventTrigger::Message,
            "timerEventDefinition" => EventTrigger::Timer,
            "errorEventDefinition" => EventTrigger::Error,
            "signalEventDefinition" => EventTrigger::Signal,
            "conditionalEventDefinition" => EventTrigger::Conditional,
            "terminateEventDefinition" => EventTrigger::Terminate,
            _ => continue,
        };
        return t;
    }
    EventTrigger::None
}

fn kind_of(e: &Element) -> Option<NodeKind> {
    Some(match e.name.as_str() {
        "startEvent" => NodeKind::StartEvent(trigger_of(e)),
        "endEvent" => NodeKind::EndEvent(trigger_of(e)),
        "intermediateCatchEvent" | "boundaryEvent" => NodeKind::IntermediateEvent {
            trigger: trigger_of(e),
            throwing: false,
        },
        "intermediateThrowEvent" => NodeKind::IntermediateEvent {
            trigger: trigger_of(e),
            throwing: true,
        },
        "task" => NodeKind::Task(TaskType::Abstract),
        "userTask" => NodeKind::Task(TaskType::User),
        "manualTask" => NodeKind::Task(TaskType::Manual),
        "serviceTask" => NodeKind::Task(TaskType::Service),
        "scriptTask" => NodeKind::Task(TaskType::Script),
        "sendTask" => NodeKind::Task(TaskType::Send),
        "receiveTask" => NodeKind::Task(TaskType::Receive),
        "businessRuleTask" => NodeKind::Task(TaskType::BusinessRule),
        "subProcess" | "callActivity" | "transaction" | "adHocSubProcess" => NodeKind::SubProcess,
        "exclusiveGateway" => NodeKind::Gateway(GatewayType::Exclusive),
        "parallelGateway" => NodeKind::Gateway(GatewayType::Parallel),
        "inclusiveGateway" => NodeKind::Gateway(GatewayType::Inclusive),
        "eventBasedGateway" => NodeKind::Gateway(GatewayType::EventBased),
        "complexGateway" => NodeKind::Gateway(GatewayType::Complex),
        "dataObjectReference" => NodeKind::DataObject,
        "dataStoreReference" => NodeKind::DataStore,
        "textAnnotation" => NodeKind::TextAnnotation,
        _ => return None,
    })
}

/// Parses BPMN 2.0 XML. Elements without diagram information are laid out on a grid.
pub fn import(src: &str) -> Result<BpmnDiagram, String> {
    let root = xml::parse(src)?;
    if root.name != "definitions" {
        return Err(format!("Not a BPMN file (root element is <{}>)", root.name));
    }
    let mut d = BpmnDiagram::new("Imported process");

    // DI bounds & waypoints
    let mut shapes: HashMap<String, (f32, f32, f32, f32)> = HashMap::new();
    for s in root.descendants().into_iter().filter(|e| e.name == "BPMNShape") {
        if let (Some(el), Some(b)) = (s.attr("bpmnElement"), s.child("Bounds")) {
            let v = (
                b.attr_f32("x").unwrap_or(0.0),
                b.attr_f32("y").unwrap_or(0.0),
                b.attr_f32("width").unwrap_or(100.0),
                b.attr_f32("height").unwrap_or(80.0),
            );
            shapes.entry(el.to_string()).or_insert(v);
        }
    }

    let processes: Vec<&Element> = root.children_named("process").collect();
    if processes.is_empty() {
        return Err("The file contains no <process>".into());
    }
    if let Some(name) = processes[0].attr("name") {
        d.name = name.to_string();
    }

    // Pools (participants) and lanes
    let mut participants: Vec<&Element> = Vec::new();
    if let Some(collab) = root.child("collaboration") {
        participants = collab.children_named("participant").collect();
    }
    for part in &participants {
        let Some(pid) = part.attr("id") else { continue };
        let Some(&(x, y, w, h)) = shapes.get(pid) else { continue };
        let pool_id = d.add_pool(Pos::new(x, y));
        let process = part
            .attr("processRef")
            .and_then(|r| processes.iter().find(|p| p.attr("id") == Some(r)));
        let mut lanes: Vec<(f32, String, f32)> = Vec::new();
        if let Some(proc) = process {
            for lane in proc.children_named("laneSet").flat_map(|ls| ls.children_named("lane")) {
                if let Some(&(_, ly, _, lh)) = lane.attr("id").and_then(|id| shapes.get(id)) {
                    lanes.push((ly, lane.attr("name").unwrap_or("").to_string(), lh));
                }
            }
        }
        lanes.sort_by(|a, b| a.0.total_cmp(&b.0));
        let pool = d.pool_mut(pool_id).unwrap();
        pool.name = part.attr("name").unwrap_or("Pool").to_string();
        pool.width = w;
        if lanes.is_empty() {
            pool.lanes[0].height = h;
            pool.lanes[0].name = String::new();
        } else {
            let first = pool.lanes[0].id;
            pool.lanes.clear();
            for (i, (_, name, lh)) in lanes.into_iter().enumerate() {
                pool.lanes.push(Lane {
                    id: if i == 0 { first } else { 0 },
                    name,
                    height: lh,
                });
            }
            // Assign fresh ids to the extra lanes
            let n = d.pool(pool_id).unwrap().lanes.len();
            for i in 1..n {
                let id = d.fresh_id();
                d.pool_mut(pool_id).unwrap().lanes[i].id = id;
            }
        }
    }

    // Flow nodes
    let mut idmap: HashMap<String, Id> = HashMap::new();
    let mut defaults: Vec<String> = Vec::new();
    let mut seq: Vec<&Element> = Vec::new();
    let mut assoc: Vec<(String, String)> = Vec::new();
    let mut auto_layout = 0usize;
    for proc in &processes {
        for e in &proc.children {
            if e.name == "sequenceFlow" {
                seq.push(e);
                continue;
            }
            if e.name == "association" {
                if let (Some(s), Some(t)) = (e.attr("sourceRef"), e.attr("targetRef")) {
                    assoc.push((s.into(), t.into()));
                }
                continue;
            }
            let Some(kind) = kind_of(e) else { continue };
            let Some(xid) = e.attr("id") else { continue };
            let (pos, size) = match shapes.get(xid) {
                Some(&(x, y, w, h)) => (Pos::new(x + w / 2.0, y + h / 2.0), Some((w, h))),
                None => {
                    auto_layout += 1;
                    (
                        Pos::new(
                            100.0 + 160.0 * ((auto_layout - 1) % 8) as f32,
                            100.0 + 140.0 * ((auto_layout - 1) / 8) as f32,
                        ),
                        None,
                    )
                }
            };
            let id = d.add_node(kind, pos);
            let n = d.node_mut(id).unwrap();
            n.name = if kind == NodeKind::TextAnnotation {
                e.text_at(&["text"]).unwrap_or("").to_string()
            } else {
                e.attr("name").unwrap_or("").to_string()
            };
            n.documentation = e.text_at(&["documentation"]).unwrap_or("").to_string();
            if let Some(metrics) = e.child("extensionElements").and_then(|e| e.child("activityMetrics")) {
                for (name, target) in [("cost", &mut n.cost), ("duration", &mut n.duration)] {
                    if let Some(raw) = metrics.attr(name) {
                        let value: f64 = raw.parse().map_err(|_| format!("Invalid activity {name}: {raw}"))?;
                        if !value.is_finite() || value < 0.0 {
                            return Err(format!("Invalid activity {name}: {raw}"));
                        }
                        *target = value;
                    }
                }
            }
            if let Some((w, h)) = size {
                n.w = w;
                n.h = h;
            }
            if let Some(def) = e.attr("default") {
                defaults.push(def.to_string());
            }
            // Data associations nested in activities
            for da in e.children.iter().filter(|c| c.name == "dataInputAssociation") {
                if let Some(s) = da.text_at(&["sourceRef"]) {
                    assoc.push((s.into(), xid.into()));
                }
            }
            for da in e.children.iter().filter(|c| c.name == "dataOutputAssociation") {
                if let Some(t) = da.text_at(&["targetRef"]) {
                    assoc.push((xid.into(), t.into()));
                }
            }
            idmap.insert(xid.to_string(), id);
        }
    }

    let push_flow = |d: &mut BpmnDiagram, kind: FlowKind, s: &str, t: &str, name: &str, is_default: bool| {
        if let (Some(&a), Some(&b)) = (idmap.get(s), idmap.get(t)) {
            let id = d.fresh_id();
            d.flows.push(Flow {
                id,
                kind,
                from: a,
                to: b,
                name: name.to_string(),
                is_default,
            });
        }
    };
    for e in seq {
        let (Some(s), Some(t)) = (e.attr("sourceRef"), e.attr("targetRef")) else {
            continue;
        };
        let is_default = e.attr("id").is_some_and(|id| defaults.iter().any(|x| x == id));
        push_flow(&mut d, FlowKind::Sequence, s, t, e.attr("name").unwrap_or(""), is_default);
    }
    for (s, t) in &assoc {
        push_flow(&mut d, FlowKind::Association, s, t, "", false);
    }
    if let Some(collab) = root.child("collaboration") {
        for e in collab.children_named("messageFlow") {
            if let (Some(s), Some(t)) = (e.attr("sourceRef"), e.attr("targetRef")) {
                push_flow(&mut d, FlowKind::Message, s, t, e.attr("name").unwrap_or(""), false);
            }
        }
        for e in collab.children_named("association") {
            if let (Some(s), Some(t)) = (e.attr("sourceRef"), e.attr("targetRef")) {
                push_flow(&mut d, FlowKind::Association, s, t, "", false);
            }
        }
    }
    Ok(d)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> BpmnDiagram {
        let mut d = BpmnDiagram::new("Order");
        d.add_pool(Pos::new(0.0, 0.0));
        let pool = d.pools[0].id;
        d.add_lane(pool);
        let s = d.add_node(NodeKind::StartEvent(EventTrigger::Message), Pos::new(80.0, 90.0));
        let t = d.add_node(NodeKind::Task(TaskType::User), Pos::new(200.0, 90.0));
        let g = d.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::new(330.0, 90.0));
        let t2 = d.add_node(NodeKind::Task(TaskType::Service), Pos::new(450.0, 250.0));
        let e = d.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::new(600.0, 90.0));
        d.node_mut(t).unwrap().name = "Check <order> & stock".into();
        d.add_flow(s, t).unwrap();
        d.add_flow(t, g).unwrap();
        let f1 = d.add_flow(g, e).unwrap();
        d.add_flow(g, t2).unwrap();
        d.add_flow(t2, e).unwrap();
        d.flow_mut(f1).unwrap().is_default = true;
        d
    }

    #[test]
    fn roundtrip() {
        let d = sample();
        let xml = export(&d);
        let back = import(&xml).unwrap();
        assert_eq!(back.nodes.len(), d.nodes.len());
        assert_eq!(back.flows.len(), d.flows.len());
        assert_eq!(back.pools.len(), 1);
        assert_eq!(back.pools[0].lanes.len(), 2);
        assert!(
            back.nodes
                .iter()
                .any(|n| n.name == "Check <order> & stock" && n.kind == NodeKind::Task(TaskType::User))
        );
        assert!(back.nodes.iter().any(|n| n.kind == NodeKind::StartEvent(EventTrigger::Message)));
        assert_eq!(back.flows.iter().filter(|f| f.is_default).count(), 1);
        let t = back
            .nodes
            .iter()
            .find(|n| matches!(n.kind, NodeKind::Task(TaskType::Service)))
            .unwrap();
        assert_eq!(t.pos, Pos::new(450.0, 250.0));
    }

    #[test]
    fn import_without_di() {
        let src = r#"<?xml version="1.0"?>
        <definitions xmlns="http://www.omg.org/spec/BPMN/20100524/MODEL">
          <process id="p">
            <startEvent id="s"/>
            <task id="t" name="Do it"/>
            <endEvent id="e"/>
            <sequenceFlow id="f1" sourceRef="s" targetRef="t"/>
            <sequenceFlow id="f2" sourceRef="t" targetRef="e"/>
          </process>
        </definitions>"#;
        let d = import(src).unwrap();
        assert_eq!(d.nodes.len(), 3);
        assert_eq!(d.flows.len(), 2);
        assert!(d.validate().is_empty());
    }
}
