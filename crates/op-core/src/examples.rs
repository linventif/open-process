//! Built-in example models (also used in tests).

use crate::bpmn::*;
use crate::geom::Pos;
use crate::i18n::tr;
use crate::petri::{NodeRef, PetriNet};

/// Process P from the process-evaluation exercise: A1; AND(A2; XOR(A5,A6), A3; A4, A7); A8.
pub fn evaluation_process() -> BpmnDiagram {
    let mut d = BpmnDiagram::new(tr("example.process_p_evaluation"));
    let positions = [
        (160.0, 180.0),
        (400.0, 100.0),
        (400.0, 240.0),
        (560.0, 240.0),
        (660.0, 40.0),
        (660.0, 140.0),
        (560.0, 360.0),
        (1000.0, 180.0),
    ];
    let a: Vec<_> = positions
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            let id = d.add_node(NodeKind::Task(TaskType::Abstract), Pos::new(x, y));
            d.node_mut(id).unwrap().name = format!("A{}", i + 1);
            id
        })
        .collect();
    let start = d.add_node(NodeKind::StartEvent(EventTrigger::None), Pos::new(50.0, 180.0));
    let split = d.add_node(NodeKind::Gateway(GatewayType::Parallel), Pos::new(280.0, 180.0));
    let choice = d.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::new(520.0, 100.0));
    let merge = d.add_node(NodeKind::Gateway(GatewayType::Exclusive), Pos::new(790.0, 100.0));
    let join = d.add_node(NodeKind::Gateway(GatewayType::Parallel), Pos::new(880.0, 180.0));
    let end = d.add_node(NodeKind::EndEvent(EventTrigger::None), Pos::new(1120.0, 180.0));
    for (from, to) in [
        (start, a[0]),
        (a[0], split),
        (split, a[1]),
        (split, a[2]),
        (split, a[6]),
        (a[1], choice),
        (choice, a[4]),
        (choice, a[5]),
        (a[4], merge),
        (a[5], merge),
        (merge, join),
        (a[2], a[3]),
        (a[3], join),
        (a[6], join),
        (join, a[7]),
        (a[7], end),
    ] {
        d.add_flow(from, to).unwrap();
    }
    for flow in &mut d.flows {
        if flow.from == choice {
            flow.name = "1/2".into();
        }
    }
    d
}

/// Order handling process with a pool, two lanes, XOR and AND gateways.
pub fn order_process() -> BpmnDiagram {
    let mut d = BpmnDiagram::new(tr("example.order_handling"));
    let pool = d.add_pool(Pos::new(0.0, 0.0));
    d.add_lane(pool);
    {
        let p = d.pool_mut(pool).unwrap();
        p.name = tr("example.online_shop").into();
        p.width = 1140.0;
        p.lanes[0].name = tr("example.sales").into();
        p.lanes[0].height = 200.0;
        p.lanes[1].name = tr("example.warehouse").into();
        p.lanes[1].height = 200.0;
    }
    let node = |d: &mut BpmnDiagram, kind, x, y, name: &str| {
        let id = d.add_node(kind, Pos::new(x, y));
        d.node_mut(id).unwrap().name = name.into();
        id
    };
    let s = node(
        &mut d,
        NodeKind::StartEvent(EventTrigger::Message),
        90.0,
        100.0,
        tr("epc.order_received"),
    );
    let check = node(&mut d, NodeKind::Task(TaskType::User), 200.0, 100.0, tr("activity.check_order"));
    let ok = node(
        &mut d,
        NodeKind::Gateway(GatewayType::Exclusive),
        330.0,
        100.0,
        tr("example.order_valid"),
    );
    let reject = node(&mut d, NodeKind::Task(TaskType::Send), 460.0, 40.0, tr("example.notify_rejection"));
    let e_rej = node(
        &mut d,
        NodeKind::EndEvent(EventTrigger::None),
        600.0,
        40.0,
        tr("example.order_rejected"),
    );
    let split = node(&mut d, NodeKind::Gateway(GatewayType::Parallel), 460.0, 300.0, "");
    let invoice = node(&mut d, NodeKind::Task(TaskType::Service), 600.0, 140.0, tr("example.send_invoice"));
    let pack = node(&mut d, NodeKind::Task(TaskType::Manual), 600.0, 300.0, tr("example.pack_goods"));
    let ship = node(&mut d, NodeKind::Task(TaskType::Manual), 760.0, 300.0, tr("example.ship_goods"));
    let join = node(&mut d, NodeKind::Gateway(GatewayType::Parallel), 900.0, 300.0, "");
    let e_ok = node(
        &mut d,
        NodeKind::EndEvent(EventTrigger::None),
        1040.0,
        300.0,
        tr("example.order_fulfilled"),
    );
    for (a, b) in [
        (s, check),
        (check, ok),
        (ok, reject),
        (reject, e_rej),
        (ok, split),
        (split, invoice),
        (split, pack),
        (pack, ship),
        (ship, join),
        (invoice, join),
        (join, e_ok),
    ] {
        d.add_flow(a, b).unwrap();
    }
    for f in &mut d.flows {
        if f.from == ok && f.to == reject {
            f.name = tr("common.no").into();
        }
        if f.from == ok && f.to == split {
            f.name = tr("common.yes").into();
        }
    }
    d
}

fn connect(n: &mut PetriNet, arcs: &[(NodeRef, NodeRef)]) {
    for &(a, b) in arcs {
        n.add_arc(a, b).unwrap();
    }
}

fn named_place(n: &mut PetriNet, name: &str, x: f32, y: f32, tokens: u32) -> NodeRef {
    let id = n.add_place(Pos::new(x, y), tokens);
    n.place_mut(id).unwrap().name = name.into();
    NodeRef::Place(id)
}

fn named_transition(n: &mut PetriNet, name: &str, x: f32, y: f32) -> NodeRef {
    let id = n.add_transition(Pos::new(x, y));
    n.transition_mut(id).unwrap().name = name.into();
    NodeRef::Transition(id)
}

/// Two processes sharing a critical section protected by a mutex.
pub fn mutual_exclusion() -> PetriNet {
    let mut n = PetriNet::new(tr("example.mutual_exclusion"));
    let mtx = named_place(&mut n, "mutex", 420.0, 200.0, 1);
    // Each process is a cycle drawn as a diamond: idle → enter → critical → leave → idle,
    // with the critical section on the side of the shared mutex.
    for (k, cx, side) in [(1, 200.0f32, 1.0f32), (2, 640.0, -1.0)] {
        let idle = named_place(&mut n, &format!("idle{k}"), cx - 100.0 * side, 200.0, 1);
        let enter = named_transition(&mut n, &format!("enter{k}"), cx, 110.0);
        let crit = named_place(&mut n, &format!("critical{k}"), cx + 100.0 * side, 200.0, 0);
        let leave = named_transition(&mut n, &format!("leave{k}"), cx, 290.0);
        connect(
            &mut n,
            &[
                (idle, enter),
                (mtx, enter),
                (enter, crit),
                (crit, leave),
                (leave, idle),
                (leave, mtx),
            ],
        );
    }
    n
}

/// Producer / consumer with a bounded buffer of size 3 (complementary place).
pub fn producer_consumer() -> PetriNet {
    let mut n = PetriNet::new(tr("example.producer_consumer"));
    let ready_p = named_place(&mut n, "ready to produce", 80.0, 100.0, 1);
    let produce = named_transition(&mut n, "produce", 180.0, 40.0);
    let ready_d = named_place(&mut n, "ready to deliver", 80.0, 260.0, 0);
    let deliver = named_transition(&mut n, "deliver", 180.0, 320.0);
    let buffer = named_place(&mut n, "buffer", 320.0, 260.0, 0);
    let free = named_place(&mut n, "free slots", 320.0, 100.0, 3);
    let remove = named_transition(&mut n, "remove", 460.0, 320.0);
    let ready_c = named_place(&mut n, "ready to consume", 560.0, 260.0, 0);
    let consume = named_transition(&mut n, "consume", 460.0, 40.0);
    let ready_r = named_place(&mut n, "ready to remove", 560.0, 100.0, 1);
    connect(
        &mut n,
        &[
            (ready_p, produce),
            (produce, ready_d),
            (ready_d, deliver),
            (deliver, ready_p),
            (free, deliver),
            (deliver, buffer),
            (buffer, remove),
            (remove, free),
            (ready_r, remove),
            (remove, ready_c),
            (ready_c, consume),
            (consume, ready_r),
        ],
    );
    n
}

/// Classic dining philosophers (3 philosophers): has a deadlock.
pub fn dining_philosophers() -> PetriNet {
    let mut n = PetriNet::new(tr("example.dining_philosophers"));
    let k = 3;
    let (cx, cy) = (420.0f32, 420.0f32);
    let angle = |i: usize| -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / k as f32;
    // Forks sit between two philosophers.
    let forks: Vec<NodeRef> = (0..k)
        .map(|i| {
            let a = angle(i) + std::f32::consts::PI / k as f32;
            named_place(&mut n, &format!("fork{}", i + 1), cx + 150.0 * a.cos(), cy + 150.0 * a.sin(), 1)
        })
        .collect();
    for i in 0..k {
        let a = angle(i);
        let (u, v) = ((a.cos(), a.sin()), (-a.sin(), a.cos()));
        // r: distance from the center along u, t: offset along v
        let at = |r: f32, t: f32| (cx + r * u.0 + t * v.0, cy + r * u.1 + t * v.1);
        let p = |n: &mut PetriNet, name: &str, (x, y): (f32, f32), tok| named_place(n, &format!("{name}{}", i + 1), x, y, tok);
        let t = |n: &mut PetriNet, name: &str, (x, y): (f32, f32)| named_transition(n, &format!("{name}{}", i + 1), x, y);
        let think = p(&mut n, "thinking", at(360.0, 0.0), 1);
        let take_l = t(&mut n, "take left", at(300.0, 80.0));
        let has_l = p(&mut n, "has left", at(220.0, 80.0), 0);
        let take_r = t(&mut n, "take right", at(170.0, 0.0));
        let eat = p(&mut n, "eating", at(220.0, -80.0), 0);
        let release = t(&mut n, "release", at(300.0, -80.0));
        let left = forks[i];
        let right = forks[(i + k - 1) % k];
        connect(
            &mut n,
            &[
                (think, take_l),
                (left, take_l),
                (take_l, has_l),
                (has_l, take_r),
                (right, take_r),
                (take_r, eat),
                (eat, release),
                (release, think),
                (release, left),
                (release, right),
            ],
        );
    }
    n
}

/// UML class diagram illustrating interfaces, composition and multiplicities.
pub fn uml_classes() -> crate::uml::UmlDiagram {
    use crate::uml::{DiagramKind, NodeKind, RelationKind, UmlDiagram};
    let mut d = UmlDiagram::new(tr("uml.uml_order_classes"), DiagramKind::Class);
    let customer = d.add_node(NodeKind::Class, Pos::new(20.0, 210.0), tr("example.customer")).unwrap();
    let order = d.add_node(NodeKind::Class, Pos::new(380.0, 210.0), tr("example.order")).unwrap();
    let item = d
        .add_node(NodeKind::Class, Pos::new(760.0, 210.0), tr("example.order_item"))
        .unwrap();
    let payable = d.add_node(NodeKind::Interface, Pos::new(380.0, -40.0), "Payable").unwrap();
    d.nodes[0].attributes = "- name: String\n- email: String".into();
    d.nodes[0].operations = "+ placeOrder(): Order".into();
    d.nodes[1].attributes = "- number: String\n- date: Date".into();
    d.nodes[1].operations = "+ total(): Decimal\n+ pay(): bool".into();
    d.nodes[2].attributes = "- quantity: int\n- unitPrice: Decimal".into();
    d.nodes[2].operations = "+ subtotal(): Decimal".into();
    d.nodes[3].operations = "+ pay(): bool".into();
    d.connect(customer, order, RelationKind::Association);
    d.relations[0].source_multiplicity = "1".into();
    d.relations[0].target_multiplicity = "0..*".into();
    d.connect(order, item, RelationKind::Composition);
    d.relations[1].source_multiplicity = "1".into();
    d.relations[1].target_multiplicity = "1..*".into();
    d.connect(order, payable, RelationKind::Realization);
    d
}

/// UML use cases with a system boundary, mandatory inclusion and optional extension.
pub fn uml_use_cases() -> crate::uml::UmlDiagram {
    use crate::uml::{DiagramKind, NodeKind, RelationKind, UmlDiagram};
    let mut d = UmlDiagram::new(tr("uml.uml_online_shop_use_cases"), DiagramKind::UseCase);
    d.add_node(NodeKind::System, Pos::new(260.0, 0.0), tr("example.online_shop"));
    d.nodes[0].bounds.w = 650.0;
    d.nodes[0].bounds.h = 450.0;
    let customer = d.add_node(NodeKind::Actor, Pos::new(30.0, 150.0), tr("example.customer")).unwrap();
    let order = d
        .add_node(NodeKind::UseCase, Pos::new(320.0, 170.0), tr("example.place_order"))
        .unwrap();
    let pay = d
        .add_node(NodeKind::UseCase, Pos::new(670.0, 75.0), tr("example.pay_order"))
        .unwrap();
    let discount = d
        .add_node(NodeKind::UseCase, Pos::new(670.0, 300.0), tr("example.apply_discount"))
        .unwrap();
    d.connect(customer, order, RelationKind::Association);
    d.connect(order, pay, RelationKind::Include);
    d.connect(discount, order, RelationKind::Extend);
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::{BehaviouralReport, StateSpace};

    #[test]
    fn examples_are_valid() {
        let d = order_process();
        assert!(d.validate().is_empty(), "{:?}", d.validate());
        let conv = crate::convert::bpmn_to_petri(&d);
        assert!(matches!(
            crate::analysis::soundness(&conv.net, 10_000),
            crate::analysis::Soundness::Sound
        ));

        let pc = producer_consumer();
        let r = BehaviouralReport::compute(&pc, &StateSpace::explore(&pc, 10_000));
        assert_eq!(r.bounded, Some(true));
        assert_eq!(r.live, Some(true));

        let dp = dining_philosophers();
        let r = BehaviouralReport::compute(&dp, &StateSpace::explore(&dp, 10_000));
        assert_eq!(r.deadlock_free, Some(false));
    }
}
