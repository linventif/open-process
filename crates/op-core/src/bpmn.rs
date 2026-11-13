//! BPMN 2.0 process model (the subset most used in teaching):
//! events, tasks, gateways, sub-processes, data objects, annotations,
//! sequence flows, message flows, pools and lanes.

use crate::geom::{Pos, Rect};
use crate::i18n::{tr, trf};
use serde::{Deserialize, Serialize};

pub mod layout;

pub type Id = u32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventTrigger {
    None,
    Message,
    Timer,
    Error,
    Signal,
    Conditional,
    Terminate,
}

impl EventTrigger {
    pub const ALL: [EventTrigger; 7] = [
        EventTrigger::None,
        EventTrigger::Message,
        EventTrigger::Timer,
        EventTrigger::Error,
        EventTrigger::Signal,
        EventTrigger::Conditional,
        EventTrigger::Terminate,
    ];

    pub fn label(self) -> &'static str {
        match self {
            EventTrigger::None => tr("common.none"),
            EventTrigger::Message => tr("bpmn.message"),
            EventTrigger::Timer => tr("bpmn.timer"),
            EventTrigger::Error => tr("bpmn.error"),
            EventTrigger::Signal => tr("bpmn.signal"),
            EventTrigger::Conditional => tr("bpmn.conditional"),
            EventTrigger::Terminate => tr("bpmn.terminate"),
        }
    }

    /// BPMN XML element name of the event definition, if any.
    pub fn xml_definition(self) -> Option<&'static str> {
        match self {
            EventTrigger::None => None,
            EventTrigger::Message => Some("messageEventDefinition"),
            EventTrigger::Timer => Some("timerEventDefinition"),
            EventTrigger::Error => Some("errorEventDefinition"),
            EventTrigger::Signal => Some("signalEventDefinition"),
            EventTrigger::Conditional => Some("conditionalEventDefinition"),
            EventTrigger::Terminate => Some("terminateEventDefinition"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    Abstract,
    User,
    Manual,
    Service,
    Script,
    Send,
    Receive,
    BusinessRule,
}

impl TaskType {
    pub const ALL: [TaskType; 8] = [
        TaskType::Abstract,
        TaskType::User,
        TaskType::Manual,
        TaskType::Service,
        TaskType::Script,
        TaskType::Send,
        TaskType::Receive,
        TaskType::BusinessRule,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TaskType::Abstract => tr("bpmn.task"),
            TaskType::User => tr("bpmn.user_task"),
            TaskType::Manual => tr("bpmn.manual_task"),
            TaskType::Service => tr("bpmn.service_task"),
            TaskType::Script => tr("bpmn.script_task"),
            TaskType::Send => tr("bpmn.send_task"),
            TaskType::Receive => tr("bpmn.receive_task"),
            TaskType::BusinessRule => tr("bpmn.business_rule_task"),
        }
    }

    pub fn xml_tag(self) -> &'static str {
        match self {
            TaskType::Abstract => "task",
            TaskType::User => "userTask",
            TaskType::Manual => "manualTask",
            TaskType::Service => "serviceTask",
            TaskType::Script => "scriptTask",
            TaskType::Send => "sendTask",
            TaskType::Receive => "receiveTask",
            TaskType::BusinessRule => "businessRuleTask",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GatewayType {
    Exclusive,
    Parallel,
    Inclusive,
    EventBased,
    Complex,
}

impl GatewayType {
    pub const ALL: [GatewayType; 5] = [
        GatewayType::Exclusive,
        GatewayType::Parallel,
        GatewayType::Inclusive,
        GatewayType::EventBased,
        GatewayType::Complex,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GatewayType::Exclusive => tr("bpmn.exclusive_xor"),
            GatewayType::Parallel => tr("bpmn.parallel"),
            GatewayType::Inclusive => tr("bpmn.inclusive"),
            GatewayType::EventBased => tr("bpmn.event_based"),
            GatewayType::Complex => tr("bpmn.complex"),
        }
    }

    pub fn xml_tag(self) -> &'static str {
        match self {
            GatewayType::Exclusive => "exclusiveGateway",
            GatewayType::Parallel => "parallelGateway",
            GatewayType::Inclusive => "inclusiveGateway",
            GatewayType::EventBased => "eventBasedGateway",
            GatewayType::Complex => "complexGateway",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    StartEvent(EventTrigger),
    /// Intermediate event: `throwing = true` for throw events.
    IntermediateEvent {
        trigger: EventTrigger,
        throwing: bool,
    },
    EndEvent(EventTrigger),
    Task(TaskType),
    SubProcess,
    Gateway(GatewayType),
    DataObject,
    DataStore,
    TextAnnotation,
}

impl NodeKind {
    pub fn default_size(self) -> (f32, f32) {
        match self {
            NodeKind::StartEvent(_) | NodeKind::IntermediateEvent { .. } | NodeKind::EndEvent(_) => (36.0, 36.0),
            NodeKind::Task(_) => (110.0, 70.0),
            NodeKind::SubProcess => (130.0, 80.0),
            NodeKind::Gateway(_) => (50.0, 50.0),
            NodeKind::DataObject => (36.0, 50.0),
            NodeKind::DataStore => (50.0, 50.0),
            NodeKind::TextAnnotation => (110.0, 50.0),
        }
    }

    pub fn is_event(self) -> bool {
        matches!(
            self,
            NodeKind::StartEvent(_) | NodeKind::IntermediateEvent { .. } | NodeKind::EndEvent(_)
        )
    }

    pub fn is_gateway(self) -> bool {
        matches!(self, NodeKind::Gateway(_))
    }

    /// Nodes that participate in the sequence flow (as opposed to artifacts / data).
    pub fn is_flow_node(self) -> bool {
        !matches!(self, NodeKind::DataObject | NodeKind::DataStore | NodeKind::TextAnnotation)
    }

    pub fn label(self) -> String {
        match self {
            NodeKind::StartEvent(EventTrigger::None) => tr("bpmn.start_event").into(),
            NodeKind::StartEvent(t) => trf("bpmn.validation.start_event", &[&t.label()]),
            NodeKind::IntermediateEvent { trigger, throwing } => {
                if throwing {
                    trf("bpmn.validation.throw_event", &[&trigger.label()])
                } else {
                    trf("bpmn.validation.catch_event", &[&trigger.label()])
                }
            }
            NodeKind::EndEvent(EventTrigger::None) => tr("bpmn.end_event").into(),
            NodeKind::EndEvent(t) => trf("bpmn.validation.end_event", &[&t.label()]),
            NodeKind::Task(t) => t.label().into(),
            NodeKind::SubProcess => tr("bpmn.sub_process").into(),
            NodeKind::Gateway(g) => trf("bpmn.validation.gateway", &[&g.label()]),
            NodeKind::DataObject => tr("bpmn.data_object").into(),
            NodeKind::DataStore => tr("bpmn.data_store").into(),
            NodeKind::TextAnnotation => tr("bpmn.text_annotation").into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub kind: NodeKind,
    pub name: String,
    /// Center position.
    pub pos: Pos,
    pub w: f32,
    pub h: f32,
    /// Free text documentation.
    #[serde(default)]
    pub documentation: String,
    /// Processing cost and execution duration (in user-chosen, consistent units).
    #[serde(default)]
    pub cost: f64,
    #[serde(default)]
    pub duration: f64,
}

impl Node {
    pub fn rect(&self) -> Rect {
        Rect::from_center(self.pos, self.w, self.h)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlowKind {
    Sequence,
    /// Between pools.
    Message,
    /// Data / annotation association (dotted).
    Association,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flow {
    pub id: Id,
    pub kind: FlowKind,
    pub from: Id,
    pub to: Id,
    /// Label (e.g. the condition of an outgoing XOR branch).
    pub name: String,
    /// Default flow of an exclusive/inclusive gateway.
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    pub id: Id,
    pub name: String,
    /// Height of the lane; lanes are stacked vertically inside the pool.
    pub height: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pool {
    pub id: Id,
    pub name: String,
    /// Top-left corner.
    pub pos: Pos,
    pub width: f32,
    pub lanes: Vec<Lane>,
}

/// Width of the vertical title band of pools and lanes.
pub const POOL_HEADER: f32 = 30.0;

impl Pool {
    pub fn height(&self) -> f32 {
        self.lanes.iter().map(|l| l.height).sum::<f32>().max(60.0)
    }

    pub fn rect(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, self.width, self.height())
    }

    /// Rect of lane `i` (excluding the pool header band).
    pub fn lane_rect(&self, i: usize) -> Rect {
        let y = self.pos.y + self.lanes[..i].iter().map(|l| l.height).sum::<f32>();
        Rect::new(self.pos.x + POOL_HEADER, y, self.width - POOL_HEADER, self.lanes[i].height)
    }

    /// Index of the lane containing point `p`.
    pub fn lane_at(&self, p: Pos) -> Option<usize> {
        (0..self.lanes.len()).find(|&i| self.lane_rect(i).contains(p))
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BpmnDiagram {
    pub name: String,
    pub nodes: Vec<Node>,
    pub flows: Vec<Flow>,
    pub pools: Vec<Pool>,
    #[serde(default)]
    pub style: crate::style::DiagramStyle,
    next_id: Id,
}

impl BpmnDiagram {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            next_id: 1,
            ..Default::default()
        }
    }

    pub fn fresh_id(&mut self) -> Id {
        let max = self
            .nodes
            .iter()
            .map(|n| n.id)
            .chain(self.flows.iter().map(|f| f.id))
            .chain(self.pools.iter().map(|p| p.id))
            .chain(self.pools.iter().flat_map(|p| p.lanes.iter().map(|l| l.id)))
            .max()
            .unwrap_or(0);
        self.next_id = self.next_id.max(max + 1);
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn add_node(&mut self, kind: NodeKind, pos: Pos) -> Id {
        let id = self.fresh_id();
        let (w, h) = kind.default_size();
        let name = match kind {
            NodeKind::Task(_) => trf(
                "bpmn.task_name",
                &[&(self.nodes.iter().filter(|n| matches!(n.kind, NodeKind::Task(_))).count() + 1)],
            ),
            NodeKind::SubProcess => tr("bpmn.sub_process").into(),
            NodeKind::TextAnnotation => tr("bpmn.comment").into(),
            _ => String::new(),
        };
        self.nodes.push(Node {
            id,
            kind,
            name,
            pos,
            w,
            h,
            documentation: String::new(),
            cost: 0.0,
            duration: 0.0,
        });
        id
    }

    pub fn add_pool(&mut self, pos: Pos) -> Id {
        let id = self.fresh_id();
        let lane_id = self.fresh_id();
        let n = self.pools.len() + 1;
        self.pools.push(Pool {
            id,
            name: trf("bpmn.pool_name", &[&n]),
            pos,
            width: 700.0,
            lanes: vec![Lane {
                id: lane_id,
                name: tr("bpmn.lane_1").into(),
                height: 180.0,
            }],
        });
        id
    }

    pub fn add_lane(&mut self, pool: Id) {
        let lane_id = self.fresh_id();
        if let Some(p) = self.pool_mut(pool) {
            let n = p.lanes.len() + 1;
            p.lanes.push(Lane {
                id: lane_id,
                name: trf("bpmn.lane_name", &[&n]),
                height: 150.0,
            });
        }
    }

    /// Chooses the right flow kind and validates the connection.
    pub fn add_flow(&mut self, from: Id, to: Id) -> Result<Id, &'static str> {
        if from == to {
            return Err(tr("bpmn.cannot_connect_element_itself"));
        }
        let (Some(a), Some(b)) = (self.node(from), self.node(to)) else {
            return Err(tr("bpmn.unknown_element"));
        };
        let kind = if !a.kind.is_flow_node() || !b.kind.is_flow_node() {
            FlowKind::Association
        } else if self.pool_of(from) != self.pool_of(to) && self.pool_of(from).is_some() && self.pool_of(to).is_some() {
            FlowKind::Message
        } else {
            if matches!(a.kind, NodeKind::EndEvent(_)) {
                return Err(tr("bpmn.end_event_cannot_have_outgoing_sequence_flows"));
            }
            if matches!(b.kind, NodeKind::StartEvent(_)) {
                return Err(tr("bpmn.start_event_cannot_have_incoming_sequence_flows"));
            }
            FlowKind::Sequence
        };
        if self.flows.iter().any(|f| f.from == from && f.to == to) {
            return Err(tr("bpmn.these_elements_already_connected"));
        }
        let id = self.fresh_id();
        self.flows.push(Flow {
            id,
            kind,
            from,
            to,
            name: String::new(),
            is_default: false,
        });
        Ok(id)
    }

    pub fn node(&self, id: Id) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: Id) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn flow(&self, id: Id) -> Option<&Flow> {
        self.flows.iter().find(|f| f.id == id)
    }

    pub fn flow_mut(&mut self, id: Id) -> Option<&mut Flow> {
        self.flows.iter_mut().find(|f| f.id == id)
    }

    pub fn pool(&self, id: Id) -> Option<&Pool> {
        self.pools.iter().find(|p| p.id == id)
    }

    pub fn pool_mut(&mut self, id: Id) -> Option<&mut Pool> {
        self.pools.iter_mut().find(|p| p.id == id)
    }

    pub fn remove_node(&mut self, id: Id) {
        self.nodes.retain(|n| n.id != id);
        self.flows.retain(|f| f.from != id && f.to != id);
    }

    pub fn remove_flow(&mut self, id: Id) {
        self.flows.retain(|f| f.id != id);
    }

    pub fn remove_pool(&mut self, id: Id) {
        self.pools.retain(|p| p.id != id);
    }

    /// Pool (id) whose rectangle contains the center of node `id`.
    pub fn pool_of(&self, id: Id) -> Option<Id> {
        let n = self.node(id)?;
        self.pools.iter().find(|p| p.rect().contains(n.pos)).map(|p| p.id)
    }

    /// (pool id, lane id) containing node `id`.
    pub fn lane_of(&self, id: Id) -> Option<(Id, Id)> {
        let n = self.node(id)?;
        self.pools.iter().find_map(|p| p.lane_at(n.pos).map(|i| (p.id, p.lanes[i].id)))
    }

    pub fn incoming(&self, id: Id) -> impl Iterator<Item = &Flow> {
        self.flows.iter().filter(move |f| f.to == id && f.kind == FlowKind::Sequence)
    }

    pub fn outgoing(&self, id: Id) -> impl Iterator<Item = &Flow> {
        self.flows.iter().filter(move |f| f.from == id && f.kind == FlowKind::Sequence)
    }

    /// Simple modelling checks (the kind of feedback a teacher gives).
    pub fn validate(&self) -> Vec<Issue> {
        let mut issues = Vec::new();
        let starts = self.nodes.iter().filter(|n| matches!(n.kind, NodeKind::StartEvent(_))).count();
        let ends = self.nodes.iter().filter(|n| matches!(n.kind, NodeKind::EndEvent(_))).count();
        if starts == 0 {
            issues.push(Issue::warn(None, tr("bpmn.process_has_no_start_event")));
        }
        if ends == 0 {
            issues.push(Issue::warn(None, tr("bpmn.process_has_no_end_event")));
        }
        for n in &self.nodes {
            if !n.kind.is_flow_node() {
                continue;
            }
            let ins = self.incoming(n.id).count();
            let outs = self.outgoing(n.id).count();
            let label = if n.name.is_empty() {
                n.kind.label()
            } else {
                format!("'{}'", n.name)
            };
            match n.kind {
                NodeKind::StartEvent(_) if outs == 0 => issues.push(Issue::error(Some(n.id), trf("bpmn.has_no_outgoing_flow", &[&label]))),
                NodeKind::EndEvent(_) if ins == 0 => issues.push(Issue::error(Some(n.id), trf("bpmn.has_no_incoming_flow", &[&label]))),
                NodeKind::Task(_) | NodeKind::SubProcess | NodeKind::IntermediateEvent { .. } => {
                    if ins == 0 {
                        issues.push(Issue::error(Some(n.id), trf("bpmn.has_no_incoming_flow", &[&label])));
                    }
                    if outs == 0 {
                        issues.push(Issue::error(Some(n.id), trf("bpmn.has_no_outgoing_flow", &[&label])));
                    }
                    if outs > 1 {
                        issues.push(Issue::warn(
                            Some(n.id),
                            trf("bpmn.has_outgoing_flows_implicit_parallel_split_use", &[&label, &outs]),
                        ));
                    }
                    if ins > 1 {
                        issues.push(Issue::warn(
                            Some(n.id),
                            trf("bpmn.has_incoming_flows_implicit_merge_use_gateway", &[&label, &ins]),
                        ));
                    }
                    if matches!(n.kind, NodeKind::Task(_)) && n.name.trim().is_empty() {
                        issues.push(Issue::warn(Some(n.id), tr("bpmn.task_has_no_name")));
                    }
                }
                NodeKind::Gateway(g) => {
                    if ins == 0 || outs == 0 {
                        issues.push(Issue::error(Some(n.id), trf("bpmn.must_have_incoming_outgoing_flows", &[&label])));
                    } else if ins == 1 && outs == 1 {
                        issues.push(Issue::warn(Some(n.id), trf("bpmn.validation.no_split_or_join", &[&label])));
                    } else if ins > 1 && outs > 1 {
                        issues.push(Issue::warn(Some(n.id), trf("bpmn.both_joins_splits_mixed_gateway", &[&label])));
                    }
                    if matches!(g, GatewayType::Exclusive | GatewayType::Inclusive) && outs > 1 {
                        let unlabeled = self.outgoing(n.id).filter(|f| f.name.trim().is_empty() && !f.is_default).count();
                        if unlabeled > 0 {
                            issues.push(Issue::warn(
                                Some(n.id),
                                trf("bpmn.outgoing_branch_es_have_no_condition", &[&label, &unlabeled]),
                            ));
                        }
                    }
                }
                _ => {}
            }
        }
        issues
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct Issue {
    pub node: Option<Id>,
    pub severity: Severity,
    pub message: String,
}

impl Issue {
    fn warn(node: Option<Id>, message: impl Into<String>) -> Self {
        Self {
            node,
            severity: Severity::Warning,
            message: message.into(),
        }
    }
    fn error(node: Option<Id>, message: impl Into<String>) -> Self {
        Self {
            node,
            severity: Severity::Error,
            message: message.into(),
        }
    }
}

// -------------------------------------------------------------------------
// Flow routing (shared by the editor and the BPMN DI export)

/// Point where the ray from the node center towards `toward` leaves the node shape.
pub fn boundary_point(n: &Node, toward: Pos) -> Pos {
    let c = n.pos;
    let (dx, dy) = (toward.x - c.x, toward.y - c.y);
    if dx.abs() < 1e-3 && dy.abs() < 1e-3 {
        return c;
    }
    let (hw, hh) = (n.w / 2.0, n.h / 2.0);
    let t = match n.kind {
        k if k.is_event() => {
            let len = (dx * dx + dy * dy).sqrt();
            hw / len
        }
        NodeKind::Gateway(_) => 1.0 / (dx.abs() / hw + dy.abs() / hh),
        _ => {
            let tx = if dx.abs() > 1e-6 { hw / dx.abs() } else { f32::INFINITY };
            let ty = if dy.abs() > 1e-6 { hh / dy.abs() } else { f32::INFINITY };
            tx.min(ty)
        }
    };
    Pos::new(c.x + dx * t, c.y + dy * t)
}

/// Waypoints of a flow according to the diagram's link style.
pub fn route(diagram: &BpmnDiagram, f: &Flow) -> Vec<Pos> {
    use crate::style::EdgeStyle;
    let (Some(a), Some(b)) = (diagram.node(f.from), diagram.node(f.to)) else {
        return Vec::new();
    };
    match diagram.style.edges.or(EdgeStyle::Orthogonal) {
        EdgeStyle::Straight => vec![boundary_point(a, b.pos), boundary_point(b, a.pos)],
        EdgeStyle::Curved => {
            let (s, e) = (boundary_point(a, b.pos), boundary_point(b, a.pos));
            let (dx, dy) = (e.x - s.x, e.y - s.y);
            // tangents follow the dominant direction (like node editors)
            let (c1, c2) = if dx.abs() >= dy.abs() {
                (Pos::new(s.x + dx * 0.5, s.y), Pos::new(e.x - dx * 0.5, e.y))
            } else {
                (Pos::new(s.x, s.y + dy * 0.5), Pos::new(e.x, e.y - dy * 0.5))
            };
            (0..=20)
                .map(|i| {
                    let t = i as f32 / 20.0;
                    let u = 1.0 - t;
                    let (a1, a2, a3, a4) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    Pos::new(
                        a1 * s.x + a2 * c1.x + a3 * c2.x + a4 * e.x,
                        a1 * s.y + a2 * c1.y + a3 * c2.y + a4 * e.y,
                    )
                })
                .collect()
        }
        _ => route_orthogonal(diagram, f),
    }
}

/// Orthogonal routing similar to common BPMN tools.
pub fn route_orthogonal(diagram: &BpmnDiagram, f: &Flow) -> Vec<Pos> {
    let (Some(a), Some(b)) = (diagram.node(f.from), diagram.node(f.to)) else {
        return Vec::new();
    };
    if f.kind != FlowKind::Sequence {
        return vec![boundary_point(a, b.pos), boundary_point(b, a.pos)];
    }
    let (ra, rb) = (a.rect(), b.rect());
    let (dx, dy) = (b.pos.x - a.pos.x, b.pos.y - a.pos.y);

    // Aligned → straight line
    if dy.abs() < 4.0 || dx.abs() < 4.0 {
        return vec![boundary_point(a, b.pos), boundary_point(b, a.pos)];
    }
    let forward = rb.x > ra.right() + 10.0;
    if forward {
        if a.kind.is_gateway() {
            // Leave the gateway by its top/bottom vertex, then go right.
            let start = Pos::new(a.pos.x, if dy > 0.0 { ra.bottom() } else { ra.y });
            let corner = Pos::new(a.pos.x, b.pos.y);
            return vec![start, corner, Pos::new(rb.x, b.pos.y)];
        }
        if b.kind.is_gateway() {
            // Go right, then enter the gateway by its top/bottom vertex.
            let start = Pos::new(ra.right(), a.pos.y);
            let corner = Pos::new(b.pos.x, a.pos.y);
            return vec![start, corner, Pos::new(b.pos.x, if dy > 0.0 { rb.y } else { rb.bottom() })];
        }
        let mid = (ra.right() + rb.x) / 2.0;
        return vec![
            Pos::new(ra.right(), a.pos.y),
            Pos::new(mid, a.pos.y),
            Pos::new(mid, b.pos.y),
            Pos::new(rb.x, b.pos.y),
        ];
    }
    // Vertical layout (target above/below without horizontal room)
    let vertical_room = rb.y > ra.bottom() + 10.0 || rb.bottom() < ra.y - 10.0;
    if vertical_room {
        let (sy, ey) = if dy > 0.0 { (ra.bottom(), rb.y) } else { (ra.y, rb.bottom()) };
        let mid = (sy + ey) / 2.0;
        return vec![
            Pos::new(a.pos.x, sy),
            Pos::new(a.pos.x, mid),
            Pos::new(b.pos.x, mid),
            Pos::new(b.pos.x, ey),
        ];
    }
    vec![boundary_point(a, b.pos), boundary_point(b, a.pos)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flows_and_validation() {
        let mut d = BpmnDiagram::new("t");
        let s = d.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::new(0.0, 0.0));
        let t = d.add_node(NodeKind::Task(TaskType::User), Pos::new(150.0, 0.0));
        let e = d.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::new(300.0, 0.0));
        d.add_flow(s, t).unwrap();
        d.add_flow(t, e).unwrap();
        assert!(d.add_flow(e, s).is_err());
        assert!(d.add_flow(s, t).is_err());
        assert!(d.validate().is_empty(), "{:?}", d.validate());
    }

    #[test]
    fn message_flow_between_pools() {
        let mut d = BpmnDiagram::new("t");
        d.add_pool(Pos::new(0.0, 0.0));
        d.add_pool(Pos::new(0.0, 300.0));
        let a = d.add_node(NodeKind::Task(TaskType::Send), Pos::new(200.0, 90.0));
        let b = d.add_node(NodeKind::Task(TaskType::Receive), Pos::new(200.0, 390.0));
        let f = d.add_flow(a, b).unwrap();
        assert_eq!(d.flow(f).unwrap().kind, FlowKind::Message);
    }

    #[test]
    fn routing_gateway() {
        let mut d = BpmnDiagram::new("t");
        let g = d.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::new(0.0, 0.0));
        let t = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(200.0, 100.0));
        let f = d.add_flow(g, t).unwrap();
        let pts = route(&d, d.flow(f).unwrap());
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], Pos::new(0.0, 25.0));
    }
}
